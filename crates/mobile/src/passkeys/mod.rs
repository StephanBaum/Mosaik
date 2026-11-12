//! Passkeys on a phone (ADR-0093, CRYPTO_SPEC §16): Android Credential
//! Manager (API 34+) and iOS 17 credential provider, both through
//! `mosaic-passkey` in client-data mode.
//!
//! - **Android.** The caller is checked here: a request with a `platform_origin`
//!   must come from a browser on Rust's own allowlist (and may pass a hash of the
//!   client data); any other app gets the origin `android:apk-key-hash:…` and must
//!   be delegated by the relying party's `assetlinks.json`. A ceremony needs user
//!   verification and works in two calls: `begin` shows what can answer,
//!   `finish` signs. A session lives five minutes and is dropped on lock.
//! - **iOS.** The OS has already checked the caller; the request carries the
//!   `clientDataHash` and Rust signs in hash mode.
//!
//! The sign count is stored before an assertion leaves, and a new passkey is
//! stored before its response is built (ADR-0039 D5).

pub mod json;

use mosaic_core::model::{ItemType, PasskeyCredential, PasskeyPublic, Text, Timestamp};
use mosaic_core::{ItemId, VaultId};
use mosaic_origin::android_browsers::is_trusted_browser;
use mosaic_origin::app::is_valid_package_name;
use mosaic_origin::{PageOrigin, canonical_rp_id, saved_origin, webauthn_rp_id};
use mosaic_passkey::{
    COSE_ALG_ES256, ClientData, CountPersisted, CreateRequest, GetRequest, android_app_origin,
    assert_request, create_request, sanitize_name, select_algorithm,
};
use mosaic_vault::session::{ItemFilter as SessionFilter, UnlockedSession};

use crate::Core;
use crate::autofill::{AndroidCaller, single_signer, usable_vaults};
use crate::config::Platform;
use crate::error::{MobileError, Result};
use crate::items::{ItemKey, VaultSummary, parse_item_key, parse_vault_id};
use crate::state::{Phase, UnlockKind};

/// How long a ceremony waits for the user (CRYPTO_SPEC §16).
const SESSION_TTL_MS: u64 = 300_000;
/// Most ceremonies waiting at once; the oldest is dropped.
const MAX_SESSIONS: usize = 4;

/// An Android passkey request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPasskeyRequest {
    /// The `requestJson` of the Credential Manager request.
    pub request_json: String,
    /// The calling app.
    pub caller: AndroidCaller,
    /// `CallingAppInfo.getOrigin(...)` for a privileged browser.
    pub platform_origin: Option<String>,
    /// SHA-256 of the `clientDataJSON` the platform built (privileged
    /// browsers), 32 bytes.
    pub client_data_hash: Option<Vec<u8>>,
}

/// A passkey that can answer a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyCandidate {
    /// The item.
    pub key: ItemKey,
    /// The relying party.
    pub rp_id: String,
    /// The user name.
    pub user_name: String,
    /// The display name.
    pub display_name: String,
    /// When the passkey was created.
    pub created_at_ms: i64,
}

/// A started assertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeySession {
    /// The opaque ID of the ceremony.
    pub session_id: String,
    /// The validated relying party ID.
    pub rp_id: String,
    /// What can answer.
    pub candidates: Vec<PasskeyCandidate>,
}

/// A started registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyCreateSession {
    /// The opaque ID of the ceremony.
    pub session_id: String,
    /// The validated relying party ID.
    pub rp_id: String,
    /// The relying party's name.
    pub rp_name: String,
    /// The user name (sanitized).
    pub user_name: String,
    /// The display name (sanitized).
    pub user_display_name: String,
    /// The vaults that can hold the passkey.
    pub vaults: Vec<VaultSummary>,
}

/// An iOS assertion request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IosAssertRequest {
    /// The relying party.
    pub rp_id: String,
    /// SHA-256 of the `clientDataJSON` the OS built, 32 bytes.
    pub client_data_hash: Vec<u8>,
    /// The credential to sign with.
    pub credential_id: Vec<u8>,
    /// The user was verified (Secure Enclave biometrics in the same run).
    pub user_verified: bool,
}

/// An iOS assertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IosAssertion {
    /// The credential ID.
    pub credential_id: Vec<u8>,
    /// The authenticator data.
    pub authenticator_data: Vec<u8>,
    /// The DER signature over `authenticatorData || clientDataHash`.
    pub signature: Vec<u8>,
    /// The user handle.
    pub user_handle: Vec<u8>,
}

/// An iOS registration request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IosRegisterRequest {
    /// The relying party.
    pub rp_id: String,
    /// The user name.
    pub user_name: String,
    /// The user handle, 1 to 64 bytes.
    pub user_handle: Vec<u8>,
    /// SHA-256 of the `clientDataJSON` the OS built, 32 bytes.
    pub client_data_hash: Vec<u8>,
    /// The COSE algorithms offered (empty: any).
    pub algorithms: Vec<i64>,
    /// The credential IDs already registered.
    pub exclude_credential_ids: Vec<Vec<u8>>,
    /// The vault that takes the passkey.
    pub vault_id: String,
    /// The user was verified.
    pub user_verified: bool,
}

/// An iOS registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IosRegistration {
    /// The new credential ID.
    pub credential_id: Vec<u8>,
    /// The attestation object (format `none`).
    pub attestation_object: Vec<u8>,
}

/// A passkey for the iOS identity store. No key material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyIdentity {
    /// The relying party.
    pub rp_id: String,
    /// The user name.
    pub user_name: String,
    /// The credential ID.
    pub credential_id: Vec<u8>,
    /// The user handle.
    pub user_handle: Vec<u8>,
    /// The item (the record ID of the identity).
    pub record: ItemKey,
}

/// A password for the iOS identity store (only with the OD-M10-7 setting).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordIdentity {
    /// The host of the login's URL.
    pub domain: String,
    /// The user name.
    pub user_name: String,
    /// The item (the record ID of the identity).
    pub record: ItemKey,
}

/// Where the client data comes from, owned for the length of a ceremony.
#[derive(Clone)]
pub(crate) enum ClientSpec {
    Json { origin: String, challenge: Vec<u8> },
    Hash([u8; 32]),
}

impl ClientSpec {
    fn data(&self) -> ClientData<'_> {
        match self {
            Self::Json { origin, challenge } => ClientData::Json { origin, challenge },
            Self::Hash(hash) => ClientData::Hash(hash),
        }
    }
}

enum Ceremony {
    Get {
        rp_id: String,
        client: ClientSpec,
        candidates: Vec<ItemKey>,
    },
    Create {
        rp_id: String,
        client: ClientSpec,
        user_handle: Vec<u8>,
        user_name: String,
        display_name: String,
        algorithms: Vec<i64>,
        exclude: Vec<Vec<u8>>,
    },
}

struct Waiting {
    id: String,
    expires_at_ms: u64,
    ceremony: Ceremony,
}

/// The ceremonies waiting for the user.
#[derive(Default)]
pub(crate) struct PasskeySessions {
    waiting: Vec<Waiting>,
}

impl PasskeySessions {
    fn insert(&mut self, ceremony: Ceremony, now_ms: u64) -> Result<String> {
        let bytes = mosaic_core::random_public_bytes::<16>()?;
        let id = crate::storage::hex(&bytes);
        self.waiting.retain(|w| now_ms < w.expires_at_ms);
        while self.waiting.len() >= MAX_SESSIONS {
            self.waiting.remove(0);
        }
        self.waiting.push(Waiting {
            id: id.clone(),
            expires_at_ms: now_ms.saturating_add(SESSION_TTL_MS),
            ceremony,
        });
        Ok(id)
    }

    /// Removes and returns a ceremony unless it expired.
    fn take(&mut self, id: &str, now_ms: u64) -> Option<Ceremony> {
        self.waiting.retain(|w| now_ms < w.expires_at_ms);
        let at = self.waiting.iter().position(|w| w.id == id)?;
        Some(self.waiting.remove(at).ceremony)
    }

    /// Forgets every ceremony (lock).
    pub(crate) fn clear(&mut self) {
        self.waiting.clear();
    }
}

fn passkey_error(e: mosaic_passkey::Error) -> MobileError {
    match e {
        mosaic_passkey::Error::UnsupportedAlgorithm => MobileError::PasskeyUnsupportedAlgorithm,
        mosaic_passkey::Error::InvalidInput => MobileError::InvalidInput,
        _ => MobileError::Internal,
    }
}

/// A stored passkey in a vault this session fills from.
struct Stored {
    vault: VaultId,
    item: ItemId,
    public: PasskeyPublic,
}

/// The live ES256 passkeys of usable vaults, in a stable order.
fn stored_passkeys(session: &UnlockedSession) -> Vec<Stored> {
    let usable = usable_vaults(session);
    let mut list: Vec<Stored> = session
        .items(&SessionFilter::default())
        .into_iter()
        .filter(|r| usable.contains(&r.vault_id) && r.item.item_type() == Some(ItemType::Passkey))
        .filter_map(|r| {
            Some(Stored {
                vault: r.vault_id,
                item: r.item_id,
                public: r.item.passkey_public()?,
            })
        })
        .filter(|s| s.public.algorithm == COSE_ALG_ES256)
        .collect();
    list.sort_by_cached_key(|s| {
        (
            s.public.user_name.as_str().to_lowercase(),
            s.public.created_at.as_unix_millis(),
            s.item.to_string(),
        )
    });
    list
}

/// Whether a stored passkey answers a request: one of the listed IDs, or, with
/// no list, a discoverable one.
fn answers(allow: &[Vec<u8>], public: &PasskeyPublic) -> bool {
    if allow.is_empty() {
        public.discoverable
    } else {
        allow.contains(&public.credential_id)
    }
}

fn candidate_of(s: &Stored) -> PasskeyCandidate {
    PasskeyCandidate {
        key: ItemKey {
            vault_id: s.vault.to_string(),
            item_id: s.item.to_string(),
        },
        rp_id: s.public.rp_id.as_str().to_owned(),
        user_name: sanitize_name(s.public.user_name.as_str()),
        display_name: sanitize_name(s.public.user_display_name.as_str()),
        created_at_ms: s.public.created_at.as_unix_millis(),
    }
}

fn rp_matches(s: &Stored, rp_id: &str) -> bool {
    s.public.rp_id.as_str() == rp_id
}

/// Signs an assertion with the stored passkey and persists the count first.
fn sign(
    session: &mut UnlockedSession,
    vault: VaultId,
    item: ItemId,
    rp_id: &str,
    client: &ClientSpec,
    user_verified: bool,
) -> Result<mosaic_passkey::ClientAssertionResponse> {
    let stored = session.item(vault, item)?;
    if stored.is_deleted() || stored.trashed_at().is_some() {
        return Err(MobileError::PasskeyNotFound);
    }
    let credential = stored.passkey().ok_or(MobileError::PasskeyNotFound)?;
    let pending = assert_request(
        &GetRequest {
            rp_id,
            client_data: client.data(),
            user_verified,
        },
        &credential,
        stored.passkey_sign_count(),
    )
    .map_err(passkey_error)?;
    drop(credential);
    if let Some(count) = pending.count_to_persist() {
        session.update_item(vault, item, |e| e.set_passkey_sign_count(count))?;
        Ok(pending.into_client_response(CountPersisted::after_store_write()))
    } else {
        pending
            .into_client_response_without_count()
            .map_err(passkey_error)
    }
}

/// What a registration made, after the item is stored.
struct Registered {
    credential_id: Vec<u8>,
    client_data_json: Option<Vec<u8>>,
    attestation_object: Vec<u8>,
    authenticator_data: Vec<u8>,
    public_key_spki: Vec<u8>,
    algorithm: i64,
}

/// Creates the passkey and stores it **before** anything is returned.
fn register(
    session: &mut UnlockedSession,
    vault: VaultId,
    request: &CreateRequest<'_>,
    user_name: &str,
    display_name: &str,
    now_ms: i64,
) -> Result<Registered> {
    let made = create_request(request).map_err(passkey_error)?;
    let mosaic_passkey::ClientCreated {
        credential_id,
        private_key_pkcs8,
        client_data_json,
        authenticator_data,
        attestation_object,
        public_key_spki,
        algorithm,
    } = made;
    let mut credential = PasskeyCredential::new(
        credential_id.clone(),
        Text::new(request.rp_id),
        request.user_handle.to_vec(),
        private_key_pkcs8,
        COSE_ALG_ES256,
        Timestamp::from_unix_millis(now_ms),
    );
    credential.user_name = Text::new(&sanitize_name(user_name));
    credential.user_display_name = Text::new(&sanitize_name(display_name));
    session.add_item(vault, &ItemType::Passkey, |e| {
        e.set_title(request.rp_id)?;
        e.set_passkey(&credential)
    })?;
    Ok(Registered {
        credential_id,
        client_data_json,
        attestation_object,
        authenticator_data,
        public_key_spki,
        algorithm,
    })
}

/// A vault that can hold a passkey: open here and not hardware-protected.
fn check_vault(session: &UnlockedSession, vault: VaultId) -> Result<()> {
    if !session.vaults().iter().any(|v| v.vault_id == vault) {
        return Err(MobileError::NotFound);
    }
    if usable_vaults(session).contains(&vault) {
        Ok(())
    } else {
        Err(MobileError::VaultUnavailable)
    }
}

fn excluded(list: &[Stored], rp_id: &str, exclude: &[Vec<u8>]) -> bool {
    list.iter()
        .any(|s| rp_matches(s, rp_id) && exclude.contains(&s.public.credential_id))
}

fn hash_of(bytes: &[u8]) -> Result<[u8; 32]> {
    <[u8; 32]>::try_from(bytes).map_err(|_| MobileError::InvalidInput)
}

impl Core {
    /// The relying party and the client data of an Android request, after the
    /// caller is checked.
    fn android_client(
        &self,
        request: &AndroidPasskeyRequest,
        rp_id: Option<&str>,
        challenge: &[u8],
    ) -> Result<(String, ClientSpec)> {
        let caller = &request.caller;
        if !is_valid_package_name(&caller.package) {
            return Err(MobileError::InvalidInput);
        }
        let certs: Vec<[u8; 32]> = caller
            .current_signers
            .iter()
            .map(|c| hash_of(c))
            .collect::<Result<_>>()?;
        if let Some(origin_text) = &request.platform_origin {
            // Rust checks the browser itself; the platform's word is not enough.
            if !is_trusted_browser(&caller.package, &certs) {
                return Err(MobileError::AppNotVerified);
            }
            let origin = PageOrigin::parse(origin_text).map_err(|_| MobileError::InvalidInput)?;
            let rp = webauthn_rp_id(&origin, rp_id).map_err(|_| MobileError::RpIdInvalid)?;
            let client = match &request.client_data_hash {
                Some(hash) => ClientSpec::Hash(hash_of(hash)?),
                None => ClientSpec::Json {
                    origin: origin.serialize(),
                    challenge: challenge.to_vec(),
                },
            };
            return Ok((rp, client));
        }
        if request.client_data_hash.is_some() {
            return Err(MobileError::InvalidInput);
        }
        // The app's identity is its one current signer (G1-9).
        let cert = single_signer(&caller.current_signers)?;
        let rp = rp_id
            .and_then(canonical_rp_id)
            .ok_or(MobileError::RpIdInvalid)?;
        self.check_asset_links(&rp, &caller.package, &cert)?;
        Ok((
            rp,
            ClientSpec::Json {
                origin: android_app_origin(&cert),
                challenge: challenge.to_vec(),
            },
        ))
    }

    fn open_ceremony(&self, ceremony: Ceremony) -> Result<String> {
        let now = self.now_ms();
        let mut guard = self.guard()?;
        if !matches!(guard.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        guard.slots.passkeys.insert(ceremony, now)
    }

    fn take_ceremony(&self, session_id: &str) -> Result<Ceremony> {
        let now = self.now_ms();
        let mut guard = self.guard()?;
        if !matches!(guard.phase, Phase::Unlocked(_)) {
            return Err(MobileError::Locked);
        }
        guard
            .slots
            .passkeys
            .take(session_id, now)
            .ok_or(MobileError::PasskeySessionExpired)
    }

    /// Starts an Android assertion: checks the caller and the relying party and
    /// lists what can answer.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `rpIdInvalid`, `appNotVerified`,
    /// `assetLinksUnavailable`.
    pub fn passkey_android_get_begin(
        &self,
        request: AndroidPasskeyRequest,
    ) -> Result<PasskeySession> {
        self.require_platform(Platform::Android)?;
        // The vault first: a locked phone does not make a request for a
        // relying party.
        drop(self.owned_session()?);
        let options = json::parse_get(request.request_json.as_bytes())?;
        let (rp_id, client) =
            self.android_client(&request, options.rp_id.as_deref(), &options.challenge)?;
        let candidates = self.with_session(|s| {
            Ok(stored_passkeys(s)
                .iter()
                .filter(|p| rp_matches(p, &rp_id) && answers(&options.allow, &p.public))
                .map(candidate_of)
                .collect::<Vec<_>>())
        })?;
        let session_id = self.open_ceremony(Ceremony::Get {
            rp_id: rp_id.clone(),
            client,
            candidates: candidates.iter().map(|c| c.key.clone()).collect(),
        })?;
        Ok(PasskeySession {
            session_id,
            rp_id,
            candidates,
        })
    }

    /// Finishes an Android assertion with the chosen passkey and returns the
    /// `AuthenticationResponseJSON`. The sign count is stored first.
    ///
    /// # Errors
    /// `locked`, `reauthRequired` (the user was not verified),
    /// `passkeySessionExpired`, `passkeyNotFound`.
    pub fn passkey_android_get_finish(
        &self,
        session_id: String,
        candidate: ItemKey,
        user_verified: bool,
    ) -> Result<String> {
        self.require_platform(Platform::Android)?;
        if !user_verified {
            return Err(MobileError::ReauthRequired);
        }
        let Ceremony::Get {
            rp_id,
            client,
            candidates,
        } = self.take_ceremony(&session_id)?
        else {
            return Err(MobileError::InvalidInput);
        };
        if !candidates.contains(&candidate) {
            return Err(MobileError::PasskeyNotFound);
        }
        let (vault, item) = parse_item_key(&candidate.vault_id, &candidate.item_id)?;
        let response = self.with_session(|s| {
            // Read again: the item may have changed since the list was made.
            let still = stored_passkeys(s)
                .iter()
                .any(|p| p.vault == vault && p.item == item && rp_matches(p, &rp_id));
            if !still {
                return Err(MobileError::PasskeyNotFound);
            }
            sign(s, vault, item, &rp_id, &client, true)
        })?;
        Ok(json::authentication_response(
            &response.credential_id,
            response.client_data_json.as_deref(),
            &response.authenticator_data,
            &response.signature,
            &response.user_handle,
        ))
    }

    /// Starts an Android registration.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `rpIdInvalid`, `appNotVerified`,
    /// `assetLinksUnavailable`, `passkeyUnsupportedAlgorithm`,
    /// `passkeyExcluded`.
    pub fn passkey_android_create_begin(
        &self,
        request: AndroidPasskeyRequest,
    ) -> Result<PasskeyCreateSession> {
        self.require_platform(Platform::Android)?;
        drop(self.owned_session()?);
        let options = json::parse_create(request.request_json.as_bytes())?;
        let (rp_id, client) =
            self.android_client(&request, options.rp_id.as_deref(), &options.challenge)?;
        select_algorithm(&options.algorithms).map_err(passkey_error)?;
        let vaults = self.with_session(|s| {
            if excluded(&stored_passkeys(s), &rp_id, &options.exclude) {
                return Err(MobileError::PasskeyExcluded);
            }
            let usable = usable_vaults(s);
            Ok(s.vaults()
                .into_iter()
                .filter(|v| usable.contains(&v.vault_id))
                .map(|v| VaultSummary {
                    vault_id: v.vault_id.to_string(),
                    name: v.name.map(str::to_owned),
                    item_count: u32::try_from(v.item_count).unwrap_or(u32::MAX),
                    available: v.available,
                })
                .collect::<Vec<_>>())
        })?;
        let user_name = sanitize_name(&options.user_name);
        let user_display_name = sanitize_name(&options.user_display_name);
        let session_id = self.open_ceremony(Ceremony::Create {
            rp_id: rp_id.clone(),
            client,
            user_handle: options.user_handle,
            user_name: user_name.clone(),
            display_name: user_display_name.clone(),
            algorithms: options.algorithms,
            exclude: options.exclude,
        })?;
        Ok(PasskeyCreateSession {
            session_id,
            rp_id,
            rp_name: options.rp_name,
            user_name,
            user_display_name,
            vaults,
        })
    }

    /// Finishes an Android registration in `vault_id` and returns the
    /// `RegistrationResponseJSON`. The passkey is stored before the response is
    /// built.
    ///
    /// # Errors
    /// `locked`, `reauthRequired` (the user was not verified),
    /// `passkeySessionExpired`, `notFound`, `vaultUnavailable`,
    /// `passkeyExcluded`.
    pub fn passkey_android_create_finish(
        &self,
        session_id: String,
        vault_id: String,
        user_verified: bool,
    ) -> Result<String> {
        self.require_platform(Platform::Android)?;
        if !user_verified {
            return Err(MobileError::ReauthRequired);
        }
        let Ceremony::Create {
            rp_id,
            client,
            user_handle,
            user_name,
            display_name,
            algorithms,
            exclude,
        } = self.take_ceremony(&session_id)?
        else {
            return Err(MobileError::InvalidInput);
        };
        let vault = parse_vault_id(&vault_id)?;
        let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
        let made = self.with_session(|s| {
            check_vault(s, vault)?;
            if excluded(&stored_passkeys(s), &rp_id, &exclude) {
                return Err(MobileError::PasskeyExcluded);
            }
            register(
                s,
                vault,
                &CreateRequest {
                    rp_id: &rp_id,
                    client_data: client.data(),
                    user_handle: &user_handle,
                    algorithms: &algorithms,
                    user_verified: true,
                },
                &user_name,
                &display_name,
                now,
            )
        })?;
        Ok(json::registration_response(
            &made.credential_id,
            made.client_data_json.as_deref(),
            &made.attestation_object,
            &made.authenticator_data,
            &made.public_key_spki,
            made.algorithm,
        ))
    }

    /// Whether this process's session was opened by a verified user: the
    /// master password or `W` from the platform's biometric key. The iOS
    /// passkey calls sign only then (G1-4); the extension never opens a session
    /// any other way.
    fn require_verified_session(&self) -> Result<()> {
        let guard = self.guard()?;
        match &guard.phase {
            Phase::Unlocked(u) => match u.info.unlock {
                UnlockKind::Password | UnlockKind::Quick => Ok(()),
            },
            Phase::Locked(_) => Err(MobileError::Locked),
            Phase::NoAccount(_) => Err(MobileError::NoAccount),
            Phase::Closed => Err(MobileError::StoreUnavailable),
            Phase::Unlocking | Phase::Busy => Err(MobileError::Busy),
        }
    }

    /// The passkeys of `rp_id` that can answer an iOS request (`allow` empty:
    /// any discoverable one).
    ///
    /// # Errors
    /// `locked`, `rpIdInvalid`, `invalidInput` (not an iOS core).
    pub fn passkey_ios_candidates(
        &self,
        rp_id: String,
        allow: Vec<Vec<u8>>,
    ) -> Result<Vec<PasskeyCandidate>> {
        self.require_platform(Platform::Ios)?;
        let rp_id = canonical_rp_id(&rp_id).ok_or(MobileError::RpIdInvalid)?;
        self.with_session(|s| {
            Ok(stored_passkeys(s)
                .iter()
                .filter(|p| rp_matches(p, &rp_id) && answers(&allow, &p.public))
                .map(candidate_of)
                .collect())
        })
    }

    /// Signs an iOS assertion in hash mode. The sign count is stored first.
    ///
    /// # Errors
    /// `locked`, `rpIdInvalid`, `invalidInput` (the hash is not 32 bytes, or
    /// not an iOS core), `reauthRequired` (the user was not verified),
    /// `passkeyNotFound`.
    pub fn passkey_ios_assert(&self, request: IosAssertRequest) -> Result<IosAssertion> {
        self.require_platform(Platform::Ios)?;
        if !request.user_verified {
            return Err(MobileError::ReauthRequired);
        }
        self.require_verified_session()?;
        let rp_id = canonical_rp_id(&request.rp_id).ok_or(MobileError::RpIdInvalid)?;
        let client = ClientSpec::Hash(hash_of(&request.client_data_hash)?);
        let response = self.with_session(|s| {
            let found = stored_passkeys(s)
                .into_iter()
                .find(|p| rp_matches(p, &rp_id) && p.public.credential_id == request.credential_id)
                .ok_or(MobileError::PasskeyNotFound)?;
            sign(s, found.vault, found.item, &rp_id, &client, true)
        })?;
        Ok(IosAssertion {
            credential_id: response.credential_id,
            authenticator_data: response.authenticator_data,
            signature: response.signature,
            user_handle: response.user_handle,
        })
    }

    /// Registers a passkey for iOS in hash mode. The passkey is stored before
    /// the attestation is returned.
    ///
    /// # Errors
    /// `locked`, `rpIdInvalid`, `invalidInput` (also not an iOS core),
    /// `reauthRequired` (the user was not verified),
    /// `passkeyUnsupportedAlgorithm`, `passkeyExcluded`, `notFound`,
    /// `vaultUnavailable`.
    pub fn passkey_ios_register(&self, request: IosRegisterRequest) -> Result<IosRegistration> {
        self.require_platform(Platform::Ios)?;
        if !request.user_verified {
            return Err(MobileError::ReauthRequired);
        }
        self.require_verified_session()?;
        let rp_id = canonical_rp_id(&request.rp_id).ok_or(MobileError::RpIdInvalid)?;
        let client = ClientSpec::Hash(hash_of(&request.client_data_hash)?);
        select_algorithm(&request.algorithms).map_err(passkey_error)?;
        let vault = parse_vault_id(&request.vault_id)?;
        let now = i64::try_from(self.now_ms()).unwrap_or(i64::MAX);
        let made = self.with_session(|s| {
            check_vault(s, vault)?;
            if excluded(&stored_passkeys(s), &rp_id, &request.exclude_credential_ids) {
                return Err(MobileError::PasskeyExcluded);
            }
            register(
                s,
                vault,
                &CreateRequest {
                    rp_id: &rp_id,
                    client_data: client.data(),
                    user_handle: &request.user_handle,
                    algorithms: &request.algorithms,
                    user_verified: true,
                },
                &request.user_name,
                &request.user_name,
                now,
            )
        })?;
        Ok(IosRegistration {
            credential_id: made.credential_id,
            attestation_object: made.attestation_object,
        })
    }

    /// The passkeys for the iOS identity store. No key material.
    ///
    /// # Errors
    /// `locked`.
    pub fn passkey_identities(&self) -> Result<Vec<PasskeyIdentity>> {
        self.with_session(|s| {
            Ok(stored_passkeys(s)
                .into_iter()
                .map(|p| PasskeyIdentity {
                    rp_id: p.public.rp_id.as_str().to_owned(),
                    user_name: sanitize_name(p.public.user_name.as_str()),
                    credential_id: p.public.credential_id.clone(),
                    user_handle: p.public.user_handle.clone(),
                    record: ItemKey {
                        vault_id: p.vault.to_string(),
                        item_id: p.item.to_string(),
                    },
                })
                .collect())
        })
    }

    /// The passwords for the iOS identity store: only with the OD-M10-7
    /// setting on, and only logins with a user name and a web URL.
    ///
    /// # Errors
    /// `locked`.
    pub fn password_identities(&self) -> Result<Vec<PasswordIdentity>> {
        let enabled = self.settings().password_identities;
        self.with_session(|s| {
            if !enabled {
                return Ok(Vec::new());
            }
            let usable = usable_vaults(s);
            let mut list = Vec::new();
            for entry in s.items(&SessionFilter::default()) {
                if !usable.contains(&entry.vault_id)
                    || entry.item.item_type() != Some(ItemType::Login)
                {
                    continue;
                }
                let Some(user_name) = entry
                    .item
                    .text(mosaic_core::model::TextField::Username)
                    .filter(|u| !u.is_empty())
                else {
                    continue;
                };
                let Some(domain) = entry
                    .item
                    .urls()
                    .iter()
                    .find_map(|(_, u)| saved_origin(u.url.as_str()))
                    .map(|o| o.host_str())
                else {
                    continue;
                };
                list.push(PasswordIdentity {
                    domain,
                    user_name: user_name.to_owned(),
                    record: ItemKey {
                        vault_id: entry.vault_id.to_string(),
                        item_id: entry.item_id.to_string(),
                    },
                });
            }
            Ok(list)
        })
    }
}
