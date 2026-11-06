//! Passkeys (M5, ADR-0039, ADR-0040): planning and
//! applying `passkeyCreate` and `passkeyGet`.
//!
//! The extension never signs or writes. A request is **planned** here (the
//! origin and the relying party ID are checked against each other with
//! `mosaic-origin` and the public suffix list, the inputs are decoded within
//! their limits, the candidates are looked up), then shown in the desktop
//! dialog (`passkey_prompt`), and only the user's Allow **applies** it: a
//! create writes a new `passkey` item before the response returns, an
//! assertion stores a raised sign count before the signature is released.
//! The private key lives in the vault item and in an `Es256Key` for one
//! signature; no op, event or `ItemDetail` field carries it.
//!
//! Activity policy: a request that only plans, opens a dialog, is cancelled
//! or answered "another device" is not user activity (a script that polls the
//! endpoint cannot keep the vault from auto-locking). A **completed** create
//! or assertion counts (`Backend::touch`), like a completed `fill`, and so
//! does the user's own answer in the dialog (`browser_passkey_respond`).

use std::time::{Duration, Instant, SystemTime};

use mosaic_core::model::{ItemType, PasskeyCredential, Text};
use mosaic_core::{ItemId, Timestamp, VaultId};
use mosaic_native_messaging::bodies::{
    AssertionBody, CreateOutcome, CreatedCredential, GetOutcome, PasskeyCreateBody,
    PasskeyCreateResult, PasskeyGetBody, PasskeyGetResult, UserVerification,
};
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use mosaic_origin::{RpIdError, webauthn_rp_id};
use mosaic_passkey::{
    COSE_ALG_ES256, CountPersisted, CreateInput, GetInput, MAX_ALLOW_LIST, MAX_CHALLENGE_LEN,
    MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN, MIN_CHALLENGE_LEN, b64url, from_b64url,
    sanitize_name,
};
use mosaic_vault::session::{ItemFilter, UnlockedSession, VaultProtection};

use super::fill::parse_origin;
use super::parse_id;
use super::save::MAX_ORIGIN_BYTES;
use crate::contract::browser::{
    BrowserPasskeyCandidate, BrowserPasskeyKind, BrowserPasskeyRequest, PasskeyVerification,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::Backend;

/// How long a user verification (an unlock or a right master password)
/// counts for the UV flag of a passkey operation (ADR-0039 D6).
pub const PASSKEY_UV_WINDOW: Duration = Duration::from_secs(300);

/// Whether a verification made at `at` / `wall_at` still counts at `now` /
/// `wall_now`: within [`PASSKEY_UV_WINDOW`] on **both** clocks. The monotonic
/// clock may stand still while the machine sleeps; the wall clock jumps over
/// the sleep, so a verification from before a suspend expires. A wall clock
/// that went backwards counts as expired.
pub(crate) fn uv_fresh(
    at: Instant,
    wall_at: SystemTime,
    now: Instant,
    wall_now: SystemTime,
) -> bool {
    now.saturating_duration_since(at) < PASSKEY_UV_WINDOW
        && wall_now
            .duration_since(wall_at)
            .is_ok_and(|elapsed| elapsed < PASSKEY_UV_WINDOW)
}

/// The longest name the extension may send, in characters.
const MAX_NAME_CHARS: usize = 256;

/// The most `pubKeyCredParams` entries accepted.
const MAX_ALGORITHMS: usize = 16;

/// What a create request shows and, once allowed, writes.
pub(crate) struct CreatePlan {
    /// The extension's request ID (matched by `passkeyCancel` only; the
    /// dialog has its own desktop-generated handle).
    pub(crate) request_id: String,
    /// The name of the browser that asks.
    pub(crate) pairing_label: String,
    /// The page's origin in canonical form (it goes into `clientDataJSON`).
    pub(crate) origin: String,
    /// The validated relying party ID.
    pub(crate) rp_id: String,
    /// The challenge (not secret).
    pub(crate) challenge: Vec<u8>,
    /// The user handle (`user.id`).
    pub(crate) user_handle: Vec<u8>,
    /// The sanitized user name.
    pub(crate) user_name: String,
    /// The sanitized display name.
    pub(crate) user_display_name: String,
    /// The COSE algorithms the site offers.
    pub(crate) algorithms: Vec<i64>,
    /// The site's preference.
    pub(crate) user_verification: UserVerification,
    /// A passkey from `excludeCredentials` is already in Mosaic.
    pub(crate) excluded: bool,
}

/// What a sign-in request shows and, once allowed, signs.
pub(crate) struct GetPlan {
    /// The extension's request ID (matched by `passkeyCancel` only; the
    /// dialog has its own desktop-generated handle).
    pub(crate) request_id: String,
    /// The name of the browser that asks.
    pub(crate) pairing_label: String,
    /// The page's origin in canonical form.
    pub(crate) origin: String,
    /// The validated relying party ID.
    pub(crate) rp_id: String,
    /// The challenge (not secret).
    pub(crate) challenge: Vec<u8>,
    /// The site's preference.
    pub(crate) user_verification: UserVerification,
    /// The decoded `allowCredentials` (empty: discoverable credentials).
    pub(crate) allow: Vec<Vec<u8>>,
    /// The passkeys offered to the user; never empty.
    pub(crate) candidates: Vec<BrowserPasskeyCandidate>,
}

/// A planned passkey request.
pub(crate) enum PasskeyPlan {
    /// `passkeyCreate`.
    Create(CreatePlan),
    /// `passkeyGet`.
    Get(GetPlan),
}

const fn verification(uv: UserVerification) -> PasskeyVerification {
    match uv {
        UserVerification::Required => PasskeyVerification::Required,
        UserVerification::Preferred => PasskeyVerification::Preferred,
        UserVerification::Discouraged => PasskeyVerification::Discouraged,
    }
}

impl PasskeyPlan {
    /// The extension's request ID.
    pub(crate) fn request_id(&self) -> &str {
        match self {
            Self::Create(plan) => &plan.request_id,
            Self::Get(plan) => &plan.request_id,
        }
    }

    /// The site's user verification preference.
    pub(crate) const fn verification(&self) -> UserVerification {
        match self {
            Self::Create(plan) => plan.user_verification,
            Self::Get(plan) => plan.user_verification,
        }
    }

    /// An exclusion only informs; nothing is signed or written.
    pub(crate) const fn is_excluded(&self) -> bool {
        matches!(self, Self::Create(CreatePlan { excluded: true, .. }))
    }

    /// Whether the dialog asks for the master password: the site wants user
    /// verification, something is to be signed or written, and the user has
    /// not been verified within [`PASSKEY_UV_WINDOW`].
    pub(crate) fn needs_password(&self, uv_recent: bool) -> bool {
        !self.is_excluded() && self.verification() != UserVerification::Discouraged && !uv_recent
    }

    /// The event the webview shows. `handle` is the dialog's
    /// desktop-generated ID: it, not the extension's, is the event's
    /// `requestId`.
    pub(crate) fn to_request(
        &self,
        handle: &str,
        needs_password: bool,
        requested_at: i64,
        expires_at: i64,
    ) -> BrowserPasskeyRequest {
        match self {
            Self::Create(plan) => BrowserPasskeyRequest {
                request_id: handle.to_owned(),
                pairing_label: plan.pairing_label.clone(),
                kind: BrowserPasskeyKind::Create,
                origin: plan.origin.clone(),
                rp_id: plan.rp_id.clone(),
                user_name: Some(plan.user_name.clone()),
                user_display_name: Some(plan.user_display_name.clone()),
                candidates: Vec::new(),
                excluded: plan.excluded,
                needs_password,
                user_verification: verification(plan.user_verification),
                requested_at,
                expires_at,
            },
            Self::Get(plan) => BrowserPasskeyRequest {
                request_id: handle.to_owned(),
                pairing_label: plan.pairing_label.clone(),
                kind: BrowserPasskeyKind::Get,
                origin: plan.origin.clone(),
                rp_id: plan.rp_id.clone(),
                user_name: None,
                user_display_name: None,
                candidates: plan.candidates.clone(),
                excluded: false,
                needs_password,
                user_verification: verification(plan.user_verification),
                requested_at,
                expires_at,
            },
        }
    }
}

/// The origin, the top-level origin and the relying party ID of a request:
/// both origins parse and are equal (`crossOriginFrame`), the page is secure
/// (`insecureOrigin`), the rp ID is valid for the origin (`rpIdMismatch`).
fn check_origin(
    origin: &str,
    top_origin: &str,
    rp_id: Option<&str>,
) -> Result<(String, String), WireError> {
    if origin.len() > MAX_ORIGIN_BYTES || top_origin.len() > MAX_ORIGIN_BYTES {
        return Err(WireError::InvalidRequest);
    }
    let page = parse_origin(origin)?;
    if page != parse_origin(top_origin)? {
        return Err(WireError::CrossOriginFrame);
    }
    let rp_id = webauthn_rp_id(&page, rp_id).map_err(|e| match e {
        RpIdError::InsecureOrigin => WireError::InsecureOrigin,
        _ => WireError::RpIdMismatch,
    })?;
    Ok((page.serialize(), rp_id))
}

fn decode(text: &str, min: usize, max: usize) -> Result<Vec<u8>, WireError> {
    let bytes = from_b64url(text, max).map_err(|_| WireError::InvalidRequest)?;
    if bytes.len() < min {
        return Err(WireError::InvalidRequest);
    }
    Ok(bytes)
}

fn decode_challenge(text: &str) -> Result<Vec<u8>, WireError> {
    decode(text, MIN_CHALLENGE_LEN, MAX_CHALLENGE_LEN)
}

/// A list of credential IDs (`allowCredentials`, `excludeCredentials`).
fn decode_ids(list: &[String]) -> Result<Vec<Vec<u8>>, WireError> {
    if list.len() > MAX_ALLOW_LIST {
        return Err(WireError::InvalidRequest);
    }
    list.iter()
        .map(|id| decode(id, 1, MAX_CREDENTIAL_ID_LEN))
        .collect()
}

fn check_request_id(id: &str) -> Result<(), WireError> {
    parse_id(id).map(|_| ()).ok_or(WireError::InvalidRequest)
}

fn check_name(name: &str) -> Result<(), WireError> {
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(WireError::InvalidRequest);
    }
    Ok(())
}

/// A stored passkey that could answer a request, without its key.
struct Hit {
    vault_id: VaultId,
    item_id: ItemId,
    vault_name: String,
    credential_id: Vec<u8>,
    discoverable: bool,
    user_name: String,
    user_display_name: String,
}

/// The vaults a passkey may be used in or written to: open (this session
/// holds the current key) and not hardware-protected (those hold only
/// one-time-code items). The vault's name goes with it.
fn usable_vaults(session: &UnlockedSession) -> Vec<(VaultId, String)> {
    session
        .vaults()
        .into_iter()
        .filter(|v| {
            v.available
                && !matches!(
                    session.vault_protection(v.vault_id),
                    Ok(VaultProtection::HardwareKey { .. })
                )
        })
        .map(|v| (v.vault_id, v.name.unwrap_or_default().to_owned()))
        .collect()
}

/// The live ES256 passkeys of `rp_id` in usable vaults, in a stable order.
fn passkeys_of(session: &UnlockedSession, rp_id: &str) -> Vec<Hit> {
    let vaults = usable_vaults(session);
    let mut hits: Vec<Hit> = session
        .items(&ItemFilter::default())
        .into_iter()
        .filter(|entry| entry.item.item_type() == Some(ItemType::Passkey))
        .filter_map(|entry| {
            let vault_name = vaults
                .iter()
                .find(|(id, _)| *id == entry.vault_id)
                .map(|(_, name)| name.clone())?;
            // Public fields only: the private key is not decoded here, so no
            // key is copied for a credential that is merely a candidate.
            let passkey = entry.item.passkey_public()?;
            (passkey.algorithm == COSE_ALG_ES256 && passkey.rp_id.as_str() == rp_id).then(|| Hit {
                vault_id: entry.vault_id,
                item_id: entry.item_id,
                vault_name,
                credential_id: passkey.credential_id.clone(),
                discoverable: passkey.discoverable,
                user_name: sanitize_name(passkey.user_name.as_str()),
                user_display_name: sanitize_name(passkey.user_display_name.as_str()),
            })
        })
        .collect();
    hits.sort_by_cached_key(|h| {
        (
            h.user_name.to_lowercase(),
            h.vault_name.to_lowercase(),
            h.item_id.to_string(),
        )
    });
    hits
}

/// Whether a stored passkey answers a request: one of the listed IDs, or,
/// with no list, a discoverable one.
fn answers(allow: &[Vec<u8>], hit: &Hit) -> bool {
    if allow.is_empty() {
        hit.discoverable
    } else {
        allow.contains(&hit.credential_id)
    }
}

impl Backend {
    /// What `passkeyCreate` shows the user. Checks the origins and the rp ID,
    /// decodes the inputs, and looks for a passkey that `excludeCredentials`
    /// names.
    ///
    /// # Errors
    /// `invalidRequest`, `crossOriginFrame`, `insecureOrigin`, `rpIdMismatch`,
    /// `passkeyUnsupported` (no offered algorithm is ES256), `locked`.
    pub(crate) fn browser_plan_passkey_create(
        &self,
        body: &PasskeyCreateBody,
        pairing_label: &str,
    ) -> Result<CreatePlan, WireError> {
        check_request_id(&body.request_id)?;
        let (origin, rp_id) = check_origin(&body.origin, &body.top_origin, body.rp_id.as_deref())?;
        let challenge = decode_challenge(&body.challenge)?;
        let user_handle = decode(&body.user_id, 1, MAX_USER_HANDLE_LEN)?;
        let exclude = decode_ids(&body.exclude_credentials)?;
        check_name(&body.user_name)?;
        check_name(&body.user_display_name)?;
        if body.algorithms.len() > MAX_ALGORITHMS {
            return Err(WireError::InvalidRequest);
        }
        mosaic_passkey::select_algorithm(&body.algorithms)
            .map_err(|_| WireError::PasskeyUnsupported)?;
        let excluded = {
            let slot = self.slot().map_err(super::fill::wire)?;
            let session = slot.unlocked().map_err(super::fill::wire)?;
            !exclude.is_empty()
                && passkeys_of(session, &rp_id)
                    .iter()
                    .any(|hit| exclude.contains(&hit.credential_id))
        };
        Ok(CreatePlan {
            request_id: body.request_id.clone(),
            pairing_label: pairing_label.to_owned(),
            origin,
            rp_id,
            challenge,
            user_handle,
            user_name: sanitize_name(&body.user_name),
            user_display_name: sanitize_name(&body.user_display_name),
            algorithms: body.algorithms.clone(),
            user_verification: body.user_verification,
            excluded,
        })
    }

    /// What `passkeyGet` shows the user.
    ///
    /// # Errors
    /// As [`Backend::browser_plan_passkey_create`], and `passkeyNotFound`
    /// when no stored passkey answers the request: then no dialog opens.
    pub(crate) fn browser_plan_passkey_get(
        &self,
        body: &PasskeyGetBody,
        pairing_label: &str,
    ) -> Result<GetPlan, WireError> {
        check_request_id(&body.request_id)?;
        let (origin, rp_id) = check_origin(&body.origin, &body.top_origin, body.rp_id.as_deref())?;
        let challenge = decode_challenge(&body.challenge)?;
        let allow = decode_ids(&body.allow_credentials)?;
        let candidates: Vec<BrowserPasskeyCandidate> = {
            let slot = self.slot().map_err(super::fill::wire)?;
            let session = slot.unlocked().map_err(super::fill::wire)?;
            passkeys_of(session, &rp_id)
                .into_iter()
                .filter(|hit| answers(&allow, hit))
                .map(|hit| BrowserPasskeyCandidate {
                    vault_id: hit.vault_id.into(),
                    item_id: hit.item_id.into(),
                    vault_name: hit.vault_name,
                    user_name: hit.user_name,
                    user_display_name: hit.user_display_name,
                })
                .collect()
        };
        if candidates.is_empty() {
            return Err(WireError::PasskeyNotFound);
        }
        Ok(GetPlan {
            request_id: body.request_id.clone(),
            pairing_label: pairing_label.to_owned(),
            origin,
            rp_id,
            challenge,
            user_verification: body.user_verification,
            allow,
            candidates,
        })
    }

    /// Notes that the user just proved it is them (an unlock or a right
    /// master password), on the monotonic and on the wall clock.
    pub(crate) fn note_user_verification(&self) {
        let now = self.browser.shared().now();
        if let Ok(mut last) = self.last_user_verification.lock() {
            *last = Some((now, SystemTime::now()));
        }
    }

    /// Forgets the verification (the vault locked).
    pub(crate) fn clear_user_verification(&self) {
        if let Ok(mut last) = self.last_user_verification.lock() {
            *last = None;
        }
    }

    /// Whether the user was verified within [`PASSKEY_UV_WINDOW`] on both
    /// clocks (see [`uv_fresh`]).
    pub(crate) fn uv_recent(&self) -> bool {
        let now = self.browser.shared().now();
        self.last_user_verification
            .lock()
            .ok()
            .and_then(|last| *last)
            .is_some_and(|(at, wall_at)| uv_fresh(at, wall_at, now, SystemTime::now()))
    }

    /// Creates the passkey in `vault` and stores it **before** the response
    /// is built: a new `passkey` item titled with the rp ID.
    ///
    /// # Errors
    /// `vaultNotFound`, `vaultUnavailable` (closed or hardware-protected),
    /// `locked`, `internal` (the key could not be made or the item not
    /// written).
    pub(crate) fn browser_apply_passkey_create(
        &self,
        plan: &CreatePlan,
        vault: VaultId,
        user_verified: bool,
    ) -> IpcResult<PasskeyCreateResult> {
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        check_vault(session, vault)?;
        let created = mosaic_passkey::create(&CreateInput {
            rp_id: &plan.rp_id,
            origin: &plan.origin,
            challenge: &plan.challenge,
            user_handle: &plan.user_handle,
            algorithms: &plan.algorithms,
            user_verified,
        })
        .map_err(|_| IpcError::INTERNAL)?;
        let mut credential = PasskeyCredential::new(
            created.credential_id.clone(),
            Text::new(&plan.rp_id),
            plan.user_handle.clone(),
            created.private_key_pkcs8,
            COSE_ALG_ES256,
            Timestamp::from_unix_millis(super::unix_millis()),
        );
        credential.user_name = Text::new(&plan.user_name);
        credential.user_display_name = Text::new(&plan.user_display_name);
        let item = session.add_item(vault, &ItemType::Passkey, |edit| {
            edit.set_title(&plan.rp_id)?;
            edit.set_passkey(&credential)
        })?;
        self.items_changed(session);
        self.touch();
        Ok(PasskeyCreateResult {
            outcome: CreateOutcome::Created,
            credential: Some(CreatedCredential {
                credential_id: b64url(&created.credential_id),
                client_data_json: b64url(&created.client_data_json),
                attestation_object: b64url(&created.attestation_object),
                authenticator_data: b64url(&created.authenticator_data),
                public_key_spki: b64url(&created.public_key_spki),
                public_key_algorithm: created.algorithm,
                vault_id: vault.to_string(),
                item_id: item.to_string(),
            }),
        })
    }

    /// Signs the assertion with the stored passkey. A counter above 0 is
    /// raised and stored **first**; the signature is released only after
    /// that write (ADR-0039 D5).
    ///
    /// # Errors
    /// `itemNotFound` (not one of the offered passkeys, or no longer an
    /// answer to the request), `vaultNotFound`, `vaultUnavailable`,
    /// `locked`, `internal`.
    pub(crate) fn browser_apply_passkey_get(
        &self,
        plan: &GetPlan,
        vault: VaultId,
        item: ItemId,
        user_verified: bool,
    ) -> IpcResult<PasskeyGetResult> {
        let not_found = || IpcError::new(ErrorCode::ItemNotFound);
        if !plan
            .candidates
            .iter()
            .any(|c| c.vault_id.get() == vault && c.item_id.get() == item)
        {
            return Err(not_found());
        }
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        check_vault(session, vault)?;
        // Read the item again: it may have been trashed, changed or removed
        // since the dialog opened.
        if !passkeys_of(session, &plan.rp_id)
            .iter()
            .any(|hit| hit.vault_id == vault && hit.item_id == item && answers(&plan.allow, hit))
        {
            return Err(not_found());
        }
        let stored = session.item(vault, item).map_err(|_| not_found())?;
        let credential = stored.passkey().ok_or_else(not_found)?;
        let pending = mosaic_passkey::assert(
            &GetInput {
                rp_id: &plan.rp_id,
                origin: &plan.origin,
                challenge: &plan.challenge,
                user_verified,
            },
            &credential,
            stored.passkey_sign_count(),
        )
        .map_err(|_| IpcError::INTERNAL)?;
        drop(credential);
        let response = if let Some(count) = pending.count_to_persist() {
            session.update_item(vault, item, |edit| edit.set_passkey_sign_count(count))?;
            pending.into_response(CountPersisted::after_store_write())
        } else {
            pending
                .into_response_without_count()
                .map_err(|_| IpcError::INTERNAL)?
        };
        self.touch();
        Ok(PasskeyGetResult {
            outcome: GetOutcome::Asserted,
            assertion: Some(AssertionBody {
                credential_id: b64url(&response.credential_id),
                client_data_json: b64url(&response.client_data_json),
                authenticator_data: b64url(&response.authenticator_data),
                signature: b64url(&response.signature),
                user_handle: b64url(&response.user_handle),
            }),
        })
    }
}

/// The vault exists, is open here and holds passkeys (not hardware-protected).
pub(crate) fn check_vault(session: &UnlockedSession, vault: VaultId) -> IpcResult<()> {
    if !session.vaults().iter().any(|v| v.vault_id == vault) {
        return Err(IpcError::new(ErrorCode::VaultNotFound));
    }
    if usable_vaults(session).iter().any(|(id, _)| *id == vault) {
        Ok(())
    } else {
        Err(IpcError::new(ErrorCode::VaultUnavailable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_lose_control_and_format_characters_and_are_cut_at_64_bytes() {
        assert_eq!(sanitize_name("alex@example.com"), "alex@example.com");
        assert_eq!(sanitize_name("a\u{202e}b\u{200b}c\u{0007}d\n"), "abcd");
        let long = "x".repeat(100);
        assert_eq!(sanitize_name(&long).len(), 64);
        // A character that would cross the limit is dropped whole.
        let wide = format!("{}é", "x".repeat(63));
        assert_eq!(sanitize_name(&wide), "x".repeat(63));
        assert_eq!(sanitize_name(""), "");
    }

    #[test]
    fn verification_window_is_five_minutes() {
        assert_eq!(PASSKEY_UV_WINDOW, Duration::from_secs(300));
    }

    #[test]
    fn a_verification_counts_only_within_the_window_on_both_clocks() {
        let at = Instant::now();
        let wall = SystemTime::now();
        let secs = Duration::from_secs;
        assert!(uv_fresh(at, wall, at + secs(299), wall + secs(299)));
        // The monotonic clock ran out.
        assert!(!uv_fresh(at, wall, at + secs(300), wall + secs(10)));
        // The monotonic clock stood still (suspend), the wall clock did not.
        assert!(!uv_fresh(at, wall, at + secs(10), wall + secs(301)));
        // The wall clock went backwards: expired, not fresh.
        assert!(!uv_fresh(at, wall + secs(100), at + secs(10), wall));
    }

    #[test]
    fn id_lists_and_challenges_are_checked_within_their_limits() {
        assert!(decode_challenge("AAECAwQFBgcICQoLDA0ODw").is_ok());
        assert_eq!(decode_challenge("AAEC"), Err(WireError::InvalidRequest));
        assert_eq!(decode_challenge("AA=="), Err(WireError::InvalidRequest));
        let ids: Vec<String> = (0..=MAX_ALLOW_LIST).map(|_| "AQID".to_owned()).collect();
        assert_eq!(decode_ids(&ids), Err(WireError::InvalidRequest));
        assert_eq!(
            decode_ids(&[String::new()]),
            Err(WireError::InvalidRequest),
            "an empty credential ID"
        );
        assert_eq!(decode_ids(&["AQID".to_owned()]), Ok(vec![vec![1, 2, 3]]));
    }

    #[test]
    fn origins_and_relying_party_ids_are_checked_against_each_other() {
        let ok = |o: &str, rp: Option<&str>| check_origin(o, o, rp);
        assert_eq!(
            ok("https://login.example.com", Some("example.com")),
            Ok((
                "https://login.example.com".to_owned(),
                "example.com".to_owned()
            ))
        );
        assert_eq!(
            ok("https://evil.test", Some("bank.test")),
            Err(WireError::RpIdMismatch)
        );
        assert_eq!(
            ok("http://example.com", None),
            Err(WireError::InsecureOrigin)
        );
        assert_eq!(
            check_origin("https://a.example.com", "https://example.com", None),
            Err(WireError::CrossOriginFrame)
        );
        assert_eq!(
            check_origin("not an origin", "not an origin", None),
            Err(WireError::InvalidRequest)
        );
        assert_eq!(
            ok("https://example.com", Some("com")),
            Err(WireError::RpIdMismatch)
        );
    }
}
