//! Request and response bodies of every op (`docs/NATIVE_MESSAGING.md` §5).
//!
//! Every struct has camelCase keys, every key present (`null` for no value)
//! and rejects unknown keys. The host parses only the two host-local bodies
//! ([`HelloBody`], [`PairBody`]); the desktop parses the others.

use core::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

use crate::hex::{Hex, HexKey};
use crate::protocol::{LockState, Secret};

/// An `Option` field whose key must be present (a missing key is an error,
/// unlike serde's default for `Option`).
fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Which browser family the extension runs in; set by the host from the
/// verified caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserKind {
    /// Chrome, Edge, Brave, Opera, Vivaldi, Chromium.
    Chromium,
    /// Firefox.
    Firefox,
}

/// The extension's guess at the browser, for display only.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hint {
    /// Google Chrome.
    Chrome,
    /// Microsoft Edge.
    Edge,
    /// Brave.
    Brave,
    /// Opera.
    Opera,
    /// Vivaldi.
    Vivaldi,
    /// Chromium.
    Chromium,
    /// Firefox.
    Firefox,
    /// Anything else (also what an unknown value is read as).
    #[serde(other)]
    Other,
}

/// What the host knows of the desktop.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopState {
    /// Nothing serves the endpoint.
    NotRunning,
    /// The desktop is locked.
    Locked,
    /// The desktop is unlocked.
    Unlocked,
}

impl From<LockState> for DesktopState {
    fn from(state: LockState) -> Self {
        match state {
            LockState::Locked => Self::Locked,
            LockState::Unlocked => Self::Unlocked,
        }
    }
}

/// An empty body: `{}`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// A pairing as the extension stores it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingBody {
    /// 32 lowercase hex digits.
    pub pairing_id: Hex<16>,
    /// 64 lowercase hex digits. A secret.
    pub key: HexKey,
}

/// `hello` request.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloBody {
    /// The stored pairing, if the extension has one.
    #[serde(deserialize_with = "required")]
    pub pairing: Option<PairingBody>,
    /// The browser guess, for display.
    pub hint: Hint,
}

/// `hello` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloResult {
    /// Whether the desktop runs, and its lock state.
    pub desktop: DesktopState,
    /// Whether the session with the stored pairing is established.
    pub paired: bool,
}

/// `pair` request.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairBody {
    /// The browser guess, for display.
    pub hint: Hint,
}

/// `pair` response: the new pairing.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairResult {
    /// 32 lowercase hex digits.
    pub pairing_id: Hex<16>,
    /// 64 lowercase hex digits. A secret.
    pub key: HexKey,
}

/// `status` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusResult {
    /// The desktop's lock state.
    pub state: LockState,
}

/// `listForOrigin` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListForOriginBody {
    /// The frame's origin (at most 2048 bytes).
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
}

/// How a saved URL matched.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchedBy {
    /// Scheme, host and port.
    ExactOrigin,
    /// Registrable domain.
    Etld1,
}

/// Why a match cannot be filled.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blocked {
    /// The page is plain HTTP and the URL does not allow it.
    Insecure,
}

/// One login that matches the page.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginMatch {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The title (at most 256 characters).
    #[serde(deserialize_with = "required")]
    pub title: Option<String>,
    /// The username.
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The host of the matching saved URL.
    #[serde(deserialize_with = "required")]
    pub host: Option<String>,
    /// Whether the item has a one-time password.
    pub has_otp: bool,
    /// Whether the item is a favorite.
    pub favorite: bool,
    /// How the URL matched.
    pub matched_by: MatchedBy,
    /// Why it cannot be filled, if so.
    #[serde(deserialize_with = "required")]
    pub blocked: Option<Blocked>,
}

/// A warning about the page.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OriginWarning {
    /// The host has punycode labels.
    Idn {
        /// The Unicode form.
        unicode_host: String,
    },
    /// The page is plain HTTP.
    Insecure {},
    /// The host imitates a saved one.
    Lookalike {
        /// The saved host.
        saved_host: String,
    },
}

/// `listForOrigin` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListForOriginResult {
    /// At most 50 matches.
    pub items: Vec<LoginMatch>,
    /// Warnings about the page.
    pub warnings: Vec<OriginWarning>,
}

/// `search` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchBody {
    /// 1 to 128 characters, trimmed.
    pub query: String,
    /// The page origin, to compute `fillable`.
    #[serde(deserialize_with = "required")]
    pub origin: Option<String>,
}

/// One search result.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchHit {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The title.
    pub title: String,
    /// The username.
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The host of the first saved URL.
    #[serde(deserialize_with = "required")]
    pub host: Option<String>,
    /// Whether the item has a one-time password.
    pub has_otp: bool,
    /// Whether the item is a favorite.
    pub favorite: bool,
    /// Whether it may be filled on the given origin.
    pub fillable: bool,
}

/// `search` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchResult {
    /// At most 20 hits.
    pub items: Vec<SearchHit>,
}

/// The frame a fill is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameKind {
    /// The top frame.
    Top,
    /// A same-origin frame with same-origin ancestry, after a confirmation.
    SameOriginFrame,
}

/// What a fill asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Want {
    /// Username and password.
    pub login: bool,
    /// A one-time code.
    pub otp: bool,
}

/// `fill` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// Which kind of frame.
    pub frame: FrameKind,
    /// What to return.
    pub want: Want,
}

/// `fill` response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillResult {
    /// The username.
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The password.
    #[serde(deserialize_with = "required")]
    pub password: Option<Secret>,
    /// The one-time code.
    #[serde(deserialize_with = "required")]
    pub otp: Option<Secret>,
}

/// A field to copy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CopyField {
    /// The username.
    Username,
    /// The password.
    Password,
    /// The current one-time code.
    Totp,
}

/// What to copy.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CopySource {
    /// A field of a login.
    Item {
        /// The item's vault.
        vault_id: String,
        /// The item.
        item_id: String,
        /// The field.
        field: CopyField,
    },
    /// The last generated password.
    Generated {},
}

/// `copy` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyBody {
    /// What to copy.
    pub source: CopySource,
}

/// `copy` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyResult {
    /// Seconds until the clipboard is cleared.
    #[serde(deserialize_with = "required")]
    pub clear_after_seconds: Option<u32>,
}

/// `generate` request. The inner request is the desktop's `GenerateRequest`
/// (its `forSite` gains `rules`, at most 4096 characters); the host does not
/// look into it.
///
/// Borrows the request from the buffer it was parsed from (a body is never
/// copied into unwiped memory); `Debug` does not print it.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateBody<'a> {
    /// The desktop's generator request.
    #[serde(borrow)]
    pub request: &'a RawValue,
}

impl fmt::Debug for GenerateBody<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenerateBody")
            .field("request", &"[REDACTED]")
            .finish()
    }
}

/// Where the generator's rules came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RulesSource {
    /// The page's `passwordrules` attribute.
    Page,
    /// The built-in quirks list.
    SiteQuirks,
}

/// `generate` response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateResult {
    /// The generated value.
    pub value: Secret,
    /// Estimated entropy in bits.
    pub entropy_bits: f64,
    /// The strength bucket (the desktop's `Strength`).
    pub strength: String,
    /// Where the rules came from.
    #[serde(deserialize_with = "required")]
    pub rules_source: Option<RulesSource>,
}

/// `checkSave` and `proposeSave` request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveBody {
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// The captured username (at most 256 characters).
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The captured password (at most 1024 characters).
    pub password: Secret,
}

/// What saving would do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveKind {
    /// A new login.
    New,
    /// A new password for an existing login.
    Update,
    /// Nothing changes.
    Unchanged,
}

/// `checkSave` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckSaveResult {
    /// What saving would do.
    pub kind: SaveKind,
    /// The title of the login that would be updated.
    #[serde(deserialize_with = "required")]
    pub item_title: Option<String>,
}

/// What the user decided.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveOutcome {
    /// A new login was written.
    Saved,
    /// An existing login got the new password.
    Updated,
    /// Nothing was written.
    Cancelled,
}

/// `proposeSave` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposeSaveResult {
    /// What happened.
    pub outcome: SaveOutcome,
    /// The vault of the written item.
    #[serde(deserialize_with = "required")]
    pub vault_id: Option<String>,
    /// The written item.
    #[serde(deserialize_with = "required")]
    pub item_id: Option<String>,
}

/// The relying party's user verification preference (WebAuthn L3 6.4.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserVerification {
    /// The relying party requires it.
    Required,
    /// The relying party prefers it.
    Preferred,
    /// The relying party does not want it.
    Discouraged,
}

/// `passkeyCreate` request. Binary values are base64url without padding.
///
/// The `origin` and `top_origin` come from the browser, never from the page;
/// `rp_id` is the page's request and is checked on the desktop.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasskeyCreateBody {
    /// 32 lowercase hex digits, chosen by the extension.
    pub request_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// The requested relying party ID (`rp.id`), if the page gave one.
    #[serde(deserialize_with = "required")]
    pub rp_id: Option<String>,
    /// The user handle (`user.id`), 1 to 64 bytes.
    pub user_id: String,
    /// The user name (at most 256 characters).
    pub user_name: String,
    /// The user's display name (at most 256 characters).
    pub user_display_name: String,
    /// The challenge, 16 to 1024 bytes.
    pub challenge: String,
    /// The COSE algorithms of `pubKeyCredParams` (at most 16).
    pub algorithms: Vec<i64>,
    /// The credential IDs of `excludeCredentials` (at most 64, each 1 to 1023 bytes).
    pub exclude_credentials: Vec<String>,
    /// The user verification preference.
    pub user_verification: UserVerification,
}

/// The credential a `passkeyCreate` made. The only key material is the
/// public key (`public_key_spki`); the private key never leaves the desktop.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatedCredential {
    /// The credential ID (base64url).
    pub credential_id: String,
    /// The `clientDataJSON` bytes (base64url).
    pub client_data_json: String,
    /// The `none` attestation object (base64url).
    pub attestation_object: String,
    /// The authenticator data (base64url).
    pub authenticator_data: String,
    /// The public key as an SPKI (base64url).
    pub public_key_spki: String,
    /// The COSE algorithm of the key (-7).
    pub public_key_algorithm: i64,
    /// The vault of the new `passkey` item.
    pub vault_id: String,
    /// The new `passkey` item.
    pub item_id: String,
}

/// What came of a `passkeyCreate`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreateOutcome {
    /// The passkey was created and stored.
    Created,
    /// The user chose another device or a security key.
    UseAnotherDevice,
}

/// `passkeyCreate` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasskeyCreateResult {
    /// What happened.
    pub outcome: CreateOutcome,
    /// The credential, if `created`.
    #[serde(deserialize_with = "required")]
    pub credential: Option<CreatedCredential>,
}

/// `passkeyGet` request. Binary values are base64url without padding.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasskeyGetBody {
    /// 32 lowercase hex digits, chosen by the extension.
    pub request_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// The requested relying party ID (`rpId`), if the page gave one.
    #[serde(deserialize_with = "required")]
    pub rp_id: Option<String>,
    /// The challenge, 16 to 1024 bytes.
    pub challenge: String,
    /// The credential IDs of `allowCredentials` (at most 64).
    pub allow_credentials: Vec<String>,
    /// The user verification preference.
    pub user_verification: UserVerification,
}

/// The signed assertion of a `passkeyGet`, all base64url.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssertionBody {
    /// The credential ID that signed.
    pub credential_id: String,
    /// The `clientDataJSON` bytes.
    pub client_data_json: String,
    /// The authenticator data.
    pub authenticator_data: String,
    /// The DER signature.
    pub signature: String,
    /// The user handle.
    pub user_handle: String,
}

/// What came of a `passkeyGet`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GetOutcome {
    /// The assertion was signed.
    Asserted,
    /// The user chose another device or a security key.
    UseAnotherDevice,
}

/// `passkeyGet` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasskeyGetResult {
    /// What happened.
    pub outcome: GetOutcome,
    /// The assertion, if `asserted`.
    #[serde(deserialize_with = "required")]
    pub assertion: Option<AssertionBody>,
}

/// `passkeyCancel` request; the response is [`Empty`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasskeyCancelBody {
    /// The `requestId` of the request to close.
    pub request_id: String,
}

/// How far a password-change attempt got.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttemptStage {
    /// The new password is stored; no page has seen it.
    Prepared,
    /// The new password was handed to a page.
    Delivered,
}

/// One open password-change attempt of a login saved for the page's exact origin. No secret.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationMatch {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The attempt: the key suffix, at most 64 ASCII characters.
    pub attempt_id: String,
    /// The title (at most 256 characters).
    #[serde(deserialize_with = "required")]
    pub title: Option<String>,
    /// The username.
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The host of the matching saved URL.
    pub host: String,
    /// How far the attempt got.
    pub stage: AttemptStage,
    /// When it started, in Unix milliseconds.
    pub started_at: i64,
    /// The change page the desktop validated (https, the item's registrable domain), if any.
    #[serde(deserialize_with = "required")]
    pub change_url: Option<String>,
}

/// `rotationForOrigin` request: [`ListForOriginBody`] plus the origin to judge.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationForOriginBody {
    /// The frame's origin (at most 2048 bytes).
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// The origin a change page was opened from. With it the desktop also says, without a
    /// dialog, whether `origin` may be offered as a related address of it (the desktop's public
    /// suffix list decides, not the extension's).
    #[serde(deserialize_with = "required")]
    pub related_to: Option<String>,
}

/// `rotationForOrigin` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationForOriginResult {
    /// At most 5 attempts, newest first.
    pub attempts: Vec<RotationMatch>,
    /// With `relatedTo`: `origin` is https, not loopback, on the same port and the same
    /// registrable domain as `relatedTo` (private suffixes such as `github.io` count as public
    /// suffixes). `false` without it; the desktop's dialog decides again.
    pub related: bool,
}

/// `rotationBegin` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationBeginBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The page's origin (top frame only).
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// The page's `passwordrules` (at most 4096 bytes), if it sets one.
    #[serde(deserialize_with = "required")]
    pub rules: Option<String>,
}

/// What came of a `rotationBegin`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BeginOutcome {
    /// The user allowed it: the attempt is stored.
    Started,
    /// The user declined, or the dialog ended.
    Cancelled,
}

/// `rotationBegin` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationBeginResult {
    /// What happened.
    pub outcome: BeginOutcome,
    /// The new attempt, if `started`.
    #[serde(deserialize_with = "required")]
    pub attempt_id: Option<String>,
}

/// Which fields a change form has.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeShape {
    /// A current-password field and one or two new-password fields.
    CurrentAndNew,
    /// One or two new-password fields and nothing else.
    NewOnly,
}

/// `fillChange` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillChangeBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The attempt.
    pub attempt_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// Which kind of frame.
    pub frame: FrameKind,
    /// The form's shape, as the extension classified it.
    pub shape: ChangeShape,
}

/// `fillChange` response: the two values of one item.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillChangeResult {
    /// The current password (`null` for `newOnly`).
    #[serde(deserialize_with = "required")]
    pub current: Option<Secret>,
    /// The attempt's new password.
    pub new: Secret,
}

/// `fillPending` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillPendingBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The attempt.
    pub attempt_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// Which kind of frame.
    pub frame: FrameKind,
}

/// `fillPending` response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FillPendingResult {
    /// The username.
    #[serde(deserialize_with = "required")]
    pub username: Option<String>,
    /// The attempt's new password.
    pub password: Secret,
}

/// How the user says the change went.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReportedOutcome {
    /// The site accepted the new password.
    Succeeded,
    /// It did not.
    Failed,
}

/// `rotationResult` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationResultBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The attempt.
    pub attempt_id: String,
    /// The frame's origin.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
    /// What the user said in the popup.
    pub outcome: ReportedOutcome,
}

/// What the desktop dialog did with a `rotationResult`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultOutcome {
    /// The new password became the item's password.
    Confirmed,
    /// The password stayed; the attempt's value went to the history.
    KeptOld,
    /// Nothing changed.
    Cancelled,
}

/// `rotationResult` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationResultResult {
    /// What happened.
    pub outcome: ResultOutcome,
}

/// `rotationAddOrigin` request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationAddOriginBody {
    /// The item's vault.
    pub vault_id: String,
    /// The item.
    pub item_id: String,
    /// The attempt.
    pub attempt_id: String,
    /// The origin the extension opened the change page on.
    pub from_origin: String,
    /// The origin the tab is on now.
    pub origin: String,
    /// The tab's top-level origin.
    pub top_origin: String,
}

/// What came of a `rotationAddOrigin`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AddOriginOutcome {
    /// The URL was added to the login.
    Added,
    /// The user declined, or the dialog ended.
    Cancelled,
}

/// `rotationAddOrigin` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationAddOriginResult {
    /// What happened.
    pub outcome: AddOriginOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "ab";

    fn hello(json: &str) -> Result<HelloBody, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn hello_body_requires_every_key() {
        let key = KEY.repeat(32);
        let id = "01".repeat(16);
        let ok = format!(r#"{{"pairing":{{"pairingId":"{id}","key":"{key}"}},"hint":"edge"}}"#);
        let body = hello(&ok).unwrap();
        assert_eq!(body.hint, Hint::Edge);
        assert_eq!(body.pairing.unwrap().pairing_id, Hex([1; 16]));
        assert!(
            hello(r#"{"pairing":null,"hint":"chrome"}"#)
                .unwrap()
                .pairing
                .is_none()
        );
        assert!(
            hello(r#"{"hint":"chrome"}"#).is_err(),
            "pairing key missing"
        );
        assert!(hello(r#"{"pairing":null}"#).is_err(), "hint missing");
        assert!(hello(r#"{"pairing":null,"hint":"chrome","x":1}"#).is_err());
        let bad_key = format!(r#"{{"pairing":{{"pairingId":"{id}","key":"zz"}},"hint":"edge"}}"#);
        assert!(hello(&bad_key).is_err());
        let extra =
            format!(r#"{{"pairing":{{"pairingId":"{id}","key":"{key}","x":1}},"hint":"edge"}}"#);
        assert!(hello(&extra).is_err());
        assert_eq!(
            hello(r#"{"pairing":null,"hint":"netscape"}"#).unwrap().hint,
            Hint::Other
        );
    }

    #[test]
    fn optional_fields_must_be_present_as_null() {
        let ok = r#"{"query":"git","origin":null}"#;
        assert_eq!(serde_json::from_str::<SearchBody>(ok).unwrap().origin, None);
        assert!(serde_json::from_str::<SearchBody>(r#"{"query":"git"}"#).is_err());
        let fill = r#"{"username":null,"password":"pw","otp":null}"#;
        let fill: FillResult = serde_json::from_str(fill).unwrap();
        assert!(fill.username.is_none() && fill.otp.is_none());
        assert_eq!(fill.password.unwrap().expose_secret(), "pw");
        assert!(
            serde_json::from_str::<FillResult>(r#"{"username":null,"password":"pw"}"#).is_err()
        );
    }

    #[test]
    fn tagged_unions_and_enums() {
        let w: Vec<OriginWarning> = serde_json::from_str(
            r#"[{"kind":"idn","unicodeHost":"bücher.example"},{"kind":"insecure"},{"kind":"lookalike","savedHost":"github.com"}]"#,
        )
        .unwrap();
        assert_eq!(w.len(), 3);
        assert!(serde_json::from_str::<OriginWarning>(r#"{"kind":"insecure","x":1}"#).is_err());
        let src: CopySource = serde_json::from_str(r#"{"kind":"generated"}"#).unwrap();
        assert_eq!(src, CopySource::Generated {});
        let item: CopySource =
            serde_json::from_str(r#"{"kind":"item","vaultId":"v","itemId":"i","field":"totp"}"#)
                .unwrap();
        assert!(matches!(
            item,
            CopySource::Item {
                field: CopyField::Totp,
                ..
            }
        ));
        assert!(serde_json::from_str::<CopySource>(r#"{"kind":"item"}"#).is_err());
        assert!(serde_json::from_str::<Want>(r#"{"login":true}"#).is_err());
        assert_eq!(
            serde_json::to_string(&DesktopState::NotRunning).unwrap(),
            "\"notRunning\""
        );
    }

    #[test]
    fn generate_keeps_the_inner_request_verbatim() {
        let body: GenerateBody =
            serde_json::from_str(r#"{"request":{"kind":"random","length":20}}"#).unwrap();
        assert_eq!(body.request.get(), r#"{"kind":"random","length":20}"#);
        assert!(serde_json::from_str::<GenerateBody>(r#"{"request":1,"x":2}"#).is_err());
    }

    const CREATE: &str = r#"{"requestId":"00112233445566778899aabbccddeeff","origin":"https://a.example","topOrigin":"https://a.example","rpId":null,"userId":"AQID","userName":"ada","userDisplayName":"Ada","challenge":"AAECAwQFBgcICQoLDA0ODw","algorithms":[-7],"excludeCredentials":[],"userVerification":"preferred"}"#;
    const GET: &str = r#"{"requestId":"00112233445566778899aabbccddeeff","origin":"https://a.example","topOrigin":"https://a.example","rpId":"a.example","challenge":"AAECAwQFBgcICQoLDA0ODw","allowCredentials":["AQID"],"userVerification":"required"}"#;
    const CANCEL: &str = r#"{"requestId":"00112233445566778899aabbccddeeff"}"#;

    fn rejects<T: serde::de::DeserializeOwned>(json: &str) -> bool {
        serde_json::from_str::<T>(json).is_err()
    }

    #[test]
    fn passkey_create_body_is_strict() {
        let body: PasskeyCreateBody = serde_json::from_str(CREATE).unwrap();
        assert_eq!(body.rp_id, None);
        assert_eq!(body.user_verification, UserVerification::Preferred);
        assert_eq!(serde_json::to_string(&body).unwrap(), CREATE);
        let extra = CREATE.replace('}', r#","x":1}"#);
        assert!(rejects::<PasskeyCreateBody>(&extra));
        assert!(rejects::<PasskeyCreateBody>(
            &CREATE.replace(r#""rpId":null,"#, "")
        ));
        assert!(rejects::<PasskeyCreateBody>(
            &CREATE.replace(r#""algorithms":[-7],"#, "")
        ));
        assert!(rejects::<PasskeyCreateBody>(
            &CREATE.replace("preferred", "maybe")
        ));
        assert!(rejects::<PasskeyCreateBody>(
            &CREATE.replace("preferred", "Preferred")
        ));
    }

    #[test]
    fn passkey_get_and_cancel_bodies_are_strict() {
        let body: PasskeyGetBody = serde_json::from_str(GET).unwrap();
        assert_eq!(body.rp_id.as_deref(), Some("a.example"));
        assert_eq!(serde_json::to_string(&body).unwrap(), GET);
        assert!(rejects::<PasskeyGetBody>(
            &GET.replace(r#""allowCredentials":["AQID"],"#, "")
        ));
        assert!(rejects::<PasskeyGetBody>(&GET.replace('}', r#","x":1}"#)));
        assert!(rejects::<PasskeyGetBody>(
            &GET.replace("required", "always")
        ));
        assert!(serde_json::from_str::<PasskeyCancelBody>(CANCEL).is_ok());
        assert!(rejects::<PasskeyCancelBody>("{}"));
        assert!(rejects::<PasskeyCancelBody>(
            &CANCEL.replace('}', r#","x":1}"#)
        ));
    }

    #[test]
    fn passkey_results_are_strict() {
        let none = r#"{"outcome":"useAnotherDevice","credential":null}"#;
        let result: PasskeyCreateResult = serde_json::from_str(none).unwrap();
        assert_eq!(result.outcome, CreateOutcome::UseAnotherDevice);
        assert!(rejects::<PasskeyCreateResult>(r#"{"outcome":"created"}"#));
        assert!(rejects::<PasskeyCreateResult>(
            r#"{"outcome":"done","credential":null}"#
        ));
        let none = r#"{"outcome":"useAnotherDevice","assertion":null}"#;
        assert!(serde_json::from_str::<PasskeyGetResult>(none).is_ok());
        assert!(rejects::<PasskeyGetResult>(r#"{"outcome":"asserted"}"#));
        let assertion = r#"{"credentialId":"a","clientDataJson":"b","authenticatorData":"c","signature":"d","userHandle":"e"}"#;
        assert!(serde_json::from_str::<AssertionBody>(assertion).is_ok());
        assert!(rejects::<AssertionBody>(
            &assertion.replace(r#","userHandle":"e""#, "")
        ));
        assert!(rejects::<AssertionBody>(
            &assertion.replace('}', r#","privateKey":"k"}"#)
        ));
    }

    const BEGIN: &str = r#"{"vaultId":"v","itemId":"i","origin":"https://a.example","topOrigin":"https://a.example","rules":null}"#;

    #[test]
    fn rotation_bodies_exact_keys() {
        let body: RotationBeginBody = serde_json::from_str(BEGIN).unwrap();
        assert_eq!(body.rules, None);
        assert_eq!(serde_json::to_string(&body).unwrap(), BEGIN);
        assert!(rejects::<RotationBeginBody>(
            &BEGIN.replace(r#","rules":null"#, "")
        ));
        assert!(rejects::<RotationBeginBody>(
            &BEGIN.replace('}', r#","x":1}"#)
        ));
        let fill = r#"{"vaultId":"v","itemId":"i","attemptId":"a","origin":"o","topOrigin":"o","frame":"top","shape":"newOnly"}"#;
        assert!(serde_json::from_str::<FillChangeBody>(fill).is_ok());
        assert!(rejects::<FillChangeBody>(&fill.replace("newOnly", "none")));
        assert!(rejects::<FillChangeBody>(
            &fill.replace(r#","shape":"newOnly""#, "")
        ));
        assert!(rejects::<FillChangeBody>(
            &fill.replace('}', r#","want":{}}"#)
        ));
        let pending = r#"{"vaultId":"v","itemId":"i","attemptId":"a","origin":"o","topOrigin":"o","frame":"sameOriginFrame"}"#;
        assert!(serde_json::from_str::<FillPendingBody>(pending).is_ok());
        assert!(rejects::<FillPendingBody>(
            &pending.replace("sameOriginFrame", "frame")
        ));
        let result = r#"{"vaultId":"v","itemId":"i","attemptId":"a","origin":"o","topOrigin":"o","outcome":"failed"}"#;
        assert!(serde_json::from_str::<RotationResultBody>(result).is_ok());
        assert!(rejects::<RotationResultBody>(
            &result.replace("failed", "maybe")
        ));
        let add = r#"{"vaultId":"v","itemId":"i","attemptId":"a","fromOrigin":"o","origin":"p","topOrigin":"p"}"#;
        assert!(serde_json::from_str::<RotationAddOriginBody>(add).is_ok());
        assert!(rejects::<RotationAddOriginBody>(
            &add.replace(r#""fromOrigin":"o","#, "")
        ));
    }

    #[test]
    fn rotation_results_are_strict_and_hide_secrets() {
        let ok = r#"{"current":null,"new":"pw"}"#;
        let result: FillChangeResult = serde_json::from_str(ok).unwrap();
        assert!(result.current.is_none());
        assert_eq!(result.new.expose_secret(), "pw");
        assert!(!format!("{result:?}").contains("pw"));
        assert!(rejects::<FillChangeResult>(r#"{"new":"pw"}"#));
        assert!(rejects::<FillChangeResult>(
            r#"{"current":null,"new":"pw","x":1}"#
        ));
        let pending: FillPendingResult =
            serde_json::from_str(r#"{"username":"ada","password":"pw"}"#).unwrap();
        assert!(!format!("{pending:?}").contains("pw"));
        assert!(rejects::<FillPendingResult>(r#"{"password":"pw"}"#));
        assert!(rejects::<RotationResultResult>(r#"{"outcome":"done"}"#));
        assert!(rejects::<RotationAddOriginResult>(
            r#"{"outcome":"added","x":1}"#
        ));
        assert!(rejects::<RotationBeginResult>(r#"{"outcome":"started"}"#));
        assert!(rejects::<RotationForOriginResult>(
            r#"{"attempts":[],"x":1}"#
        ));
        // Every key is present: `related` is a boolean and `relatedTo` a string or null.
        assert!(rejects::<RotationForOriginResult>(r#"{"attempts":[]}"#));
        let related: RotationForOriginResult =
            serde_json::from_str(r#"{"attempts":[],"related":true}"#).unwrap();
        assert!(related.related);
        let ask: RotationForOriginBody = serde_json::from_str(
            r#"{"origin":"https://a.example","topOrigin":"https://a.example","relatedTo":"https://b.example"}"#,
        )
        .unwrap();
        assert_eq!(ask.related_to.as_deref(), Some("https://b.example"));
        let bare: RotationForOriginBody = serde_json::from_str(
            r#"{"origin":"https://a.example","topOrigin":"https://a.example","relatedTo":null}"#,
        )
        .unwrap();
        assert_eq!(bare.related_to, None);
        assert!(rejects::<RotationForOriginBody>(
            r#"{"origin":"a","topOrigin":"a"}"#
        ));
        assert!(rejects::<RotationForOriginBody>(
            r#"{"origin":"a","topOrigin":"a","relatedTo":null,"x":1}"#
        ));
    }
}
