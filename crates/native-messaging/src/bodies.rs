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
}
