//! Browser extension endpoint (M4, ADR-0036, ADR-0037; passkeys M5, ADR-0040):
//! the paired browsers, the pairing prompt, the save prompt and the passkey
//! prompt. The channel to the extension
//! itself speaks `mosaic-native-messaging`'s protocol and is not part of
//! this contract.
//!
//! A pairing request arrives as the `browserPairingRequested` event and is
//! answered with `browser_pairing_respond`; a save request arrives as
//! `browserSaveRequested` and is answered with `browser_save_respond`. Both
//! close without an answer on expiry or lock (`...Closed`). The captured
//! password never reaches the webview: the save request carries no secret,
//! and the dialog shows a fixed mask. A passkey request arrives as
//! `browserPasskeyRequested` and is answered with `browser_passkey_respond`;
//! it carries no key material, and the only secret that crosses is the
//! master password the user types to confirm it is them (`SecretIn`).
//!
//! This module imports only `serde` and contract types, because the desktop
//! fuzz crate includes `src/contract/` by path.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// Whether the local endpoint for the browser host is up.
    pub enum BrowserEndpointState {
        /// Listening.
        Running,
        /// Another program (or another desktop instance) uses the endpoint.
        InUse,
        /// It could not be created (no private directory, no runtime).
        Unavailable,
    }
);

wire_enum!(
    /// Whether the saved list of paired browsers could be read at start.
    pub enum BrowserRegistryState {
        /// Read (or there was none yet).
        Ok,
        /// The file exists but could not be read. The list is empty and
        /// read-only: nothing is added or removed until it is fixed.
        Unreadable,
    }
);

wire_enum!(
    /// The browser family the host reported for a pairing.
    pub enum BrowserFamily {
        /// Chrome, Edge, Brave, ...
        Chromium,
        /// Firefox.
        Firefox,
    }
);

wire_enum!(
    /// Which browser the extension says it runs in. Display only: the user
    /// confirms with the code, and the label is theirs to change.
    pub enum BrowserHint {
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
        /// Anything else.
        Other,
    }
);

wire_enum!(
    /// The user's answer to a pairing request.
    pub enum PairingDecision {
        /// Pair this browser.
        Allow,
        /// Refuse.
        Deny,
    }
);

wire_enum!(
    /// Why a pairing, save or passkey prompt closed.
    pub enum BrowserCloseReason {
        /// The user answered (possibly in another window).
        Answered,
        /// Nobody answered in time.
        Expired,
        /// A second request replaced it (pairing only).
        Conflict,
        /// The vault locked.
        Locked,
        /// The browser went away.
        Disconnected,
        /// The page gave up (the request was aborted; passkey only).
        AbortedByPage,
    }
);

wire_enum!(
    /// What saving would do.
    pub enum BrowserSaveKind {
        /// No saved login matches: add one.
        New,
        /// A login with this user name exists with another password.
        Update,
    }
);

wire_enum!(
    /// What a passkey request asks for.
    pub enum BrowserPasskeyKind {
        /// Create a passkey (`navigator.credentials.create`).
        Create,
        /// Sign in with a passkey (`navigator.credentials.get`).
        Get,
    }
);

wire_enum!(
    /// The relying party's user verification preference.
    pub enum PasskeyVerification {
        /// The site requires it.
        Required,
        /// The site prefers it.
        Preferred,
        /// The site does not want it.
        Discouraged,
    }
);

/// A paired browser.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPairing {
    /// 32 lowercase hex digits.
    pub pairing_id: String,
    /// The name the user gave it.
    pub label: String,
    /// The browser family.
    pub browser: BrowserFamily,
    /// When it was paired.
    pub created_at: UnixMillis,
    /// When it last used the vault (`null` if never; saved at most every
    /// ten minutes).
    pub last_used_at: Option<UnixMillis>,
    /// Whether a session is open right now.
    pub connected: bool,
}

/// The `browser_status` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserStatus {
    /// The endpoint.
    pub endpoint: BrowserEndpointState,
    /// Whether the saved pairings could be read.
    pub registry: BrowserRegistryState,
    /// Paired browsers, oldest first.
    pub pairings: Vec<BrowserPairing>,
}

/// `browser_remove_pairing`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserRemovePairingRequest {
    /// From `browser_status`.
    pub pairing_id: String,
}

/// A pairing request waiting for the user.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPairingRequest {
    /// Handle for `browser_pairing_respond`: 32 lowercase hex digits.
    pub request_id: String,
    /// The browser family the host verified.
    pub browser: BrowserFamily,
    /// What the extension says it runs in.
    pub hint: BrowserHint,
    /// Six symbols of Crockford base32; the extension shows the same code.
    pub code: String,
    /// When the request arrived.
    pub requested_at: UnixMillis,
    /// When it is refused without an answer.
    pub expires_at: UnixMillis,
}

/// `browser_pairing_respond`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserPairingRespondRequest {
    /// From the event.
    pub request_id: String,
    /// The answer.
    pub decision: PairingDecision,
    /// The pairing's name, 1 to 64 characters (`null`: from the hint).
    pub label: Option<String>,
}

/// A login that a save could update.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSaveCandidate {
    /// The item's vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Its title (`null` if empty).
    pub title: Option<String>,
    /// Its user name (`null` if empty).
    pub username: Option<String>,
}

/// A save request waiting for the user. Carries no password.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSaveRequest {
    /// Handle for `browser_save_respond`: 32 lowercase hex digits.
    pub request_id: String,
    /// The name of the browser that asks.
    pub pairing_label: String,
    /// The page's origin.
    pub origin: String,
    /// The page's host, as the URL bar shows it (punycode stays punycode).
    pub host: String,
    /// The captured user name (`null` if the form had none).
    pub username: Option<String>,
    /// New login or update.
    pub kind: BrowserSaveKind,
    /// Logins with this user name that an update could change.
    pub candidates: Vec<BrowserSaveCandidate>,
    /// The title offered for a new login.
    pub suggested_title: String,
    /// The page is plain HTTP (not loopback): offer the fill override.
    pub insecure: bool,
    /// When the request arrived.
    pub requested_at: UnixMillis,
    /// When it is dropped without an answer.
    pub expires_at: UnixMillis,
}

/// The user's answer to a save request.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BrowserSaveDecision {
    /// Add a login to a vault.
    SaveNew {
        /// The target vault.
        vault_id: WireVaultId,
        /// The title, 1 to 256 characters.
        title: String,
        /// Allow filling over plain HTTP for this URL (only honoured when
        /// the page is insecure).
        allow_http: bool,
    },
    /// Replace the password of an existing login.
    Update {
        /// Its vault.
        vault_id: WireVaultId,
        /// The login.
        item_id: WireItemId,
    },
    /// Save nothing.
    Cancel,
}

/// `browser_save_respond`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserSaveRespondRequest {
    /// From the event.
    pub request_id: String,
    /// The answer.
    pub decision: BrowserSaveDecision,
}

/// A passkey the user may sign in with.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPasskeyCandidate {
    /// The item's vault.
    pub vault_id: WireVaultId,
    /// The `passkey` item.
    pub item_id: WireItemId,
    /// The vault's name (empty if it has none).
    pub vault_name: String,
    /// The account's user name (may be empty).
    pub user_name: String,
    /// The account's display name (may be empty).
    pub user_display_name: String,
}

/// A passkey request waiting for the user. Carries no key material and no
/// challenge.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPasskeyRequest {
    /// The desktop's own handle for this prompt (`browser_passkey_respond`,
    /// `browserPasskeyClosed`): 32 lowercase hex digits, made for each
    /// prompt. It is never the extension's request ID, so a page that reuses
    /// its IDs cannot steer an answer meant for another prompt.
    pub request_id: String,
    /// The name of the browser that asks.
    pub pairing_label: String,
    /// Create or sign in.
    pub kind: BrowserPasskeyKind,
    /// The page's origin, as the browser reports it.
    pub origin: String,
    /// The relying party ID, checked against the origin on the desktop.
    pub rp_id: String,
    /// Create: the account's user name (control and format characters
    /// removed, at most 64 bytes); `null` for a sign-in.
    pub user_name: Option<String>,
    /// Create: the account's display name, likewise; `null` for a sign-in.
    pub user_display_name: Option<String>,
    /// Sign-in: the passkeys that fit (never empty); create: empty.
    pub candidates: Vec<BrowserPasskeyCandidate>,
    /// Create: the site lists a passkey that Mosaic already holds, so none
    /// is made. The dialog only informs.
    pub excluded: bool,
    /// The site asked for user verification and none happened in the last
    /// five minutes: the dialog asks for the master password.
    pub needs_password: bool,
    /// The site's preference.
    pub user_verification: PasskeyVerification,
    /// When the request arrived.
    pub requested_at: UnixMillis,
    /// When it is dropped without an answer.
    pub expires_at: UnixMillis,
}

/// The user's answer to a passkey request.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BrowserPasskeyDecision {
    /// Create the passkey in this vault.
    Create {
        /// The target vault.
        vault_id: WireVaultId,
    },
    /// Sign in with this passkey.
    Assert {
        /// Its vault.
        vault_id: WireVaultId,
        /// The `passkey` item.
        item_id: WireItemId,
    },
    /// Let the browser use another device or a security key.
    UseAnotherDevice,
    /// Do nothing; the site sees a cancelled request.
    Cancel,
    /// Acknowledge that a passkey already exists (`excluded`).
    Acknowledge,
}

/// `browser_passkey_respond`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserPasskeyRespondRequest {
    /// The prompt's handle, from the event.
    pub request_id: String,
    /// The answer.
    pub decision: BrowserPasskeyDecision,
    /// The master password, when the user typed one: the dialog asked for it
    /// (`needsPassword`) or the verification ran out while it was open
    /// (`reauthRequired`). `null` otherwise. Checked only when verification
    /// is needed at that moment.
    pub password: Option<SecretIn>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn pairing_id() -> String {
        "0123456789abcdef0123456789abcdef".to_owned()
    }

    pub(crate) fn request_id() -> String {
        "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_owned()
    }

    pub(crate) fn statuses() -> Vec<BrowserStatus> {
        vec![
            BrowserStatus {
                endpoint: BrowserEndpointState::Running,
                registry: BrowserRegistryState::Ok,
                pairings: vec![
                    BrowserPairing {
                        pairing_id: pairing_id(),
                        label: "Chrome".into(),
                        browser: BrowserFamily::Chromium,
                        created_at: 1_761_480_000_000,
                        last_used_at: Some(1_761_480_035_000),
                        connected: true,
                    },
                    BrowserPairing {
                        pairing_id: "fedcba9876543210fedcba9876543210".into(),
                        label: "Firefox".into(),
                        browser: BrowserFamily::Firefox,
                        created_at: 1_761_480_010_000,
                        last_used_at: None,
                        connected: false,
                    },
                ],
            },
            BrowserStatus {
                endpoint: BrowserEndpointState::InUse,
                registry: BrowserRegistryState::Ok,
                pairings: vec![],
            },
            BrowserStatus {
                endpoint: BrowserEndpointState::Unavailable,
                registry: BrowserRegistryState::Ok,
                pairings: vec![],
            },
            BrowserStatus {
                endpoint: BrowserEndpointState::Running,
                registry: BrowserRegistryState::Unreadable,
                pairings: vec![],
            },
        ]
    }

    pub(crate) fn pairing_request() -> BrowserPairingRequest {
        BrowserPairingRequest {
            request_id: request_id(),
            browser: BrowserFamily::Chromium,
            hint: BrowserHint::Edge,
            code: "4F7K2Q".into(),
            requested_at: 1_761_480_035_000,
            expires_at: 1_761_480_155_000,
        }
    }

    pub(crate) fn save_request() -> BrowserSaveRequest {
        BrowserSaveRequest {
            request_id: request_id(),
            pairing_label: "Chrome".into(),
            origin: "https://example.com".into(),
            host: "example.com".into(),
            username: Some("alex@example.com".into()),
            kind: BrowserSaveKind::Update,
            candidates: vec![BrowserSaveCandidate {
                vault_id: ids::vault(1),
                item_id: ids::item(2),
                title: Some("Example".into()),
                username: Some("alex@example.com".into()),
            }],
            suggested_title: "example.com".into(),
            insecure: false,
            requested_at: 1_761_480_035_000,
            expires_at: 1_761_480_335_000,
        }
    }

    pub(crate) fn passkey_get_request() -> BrowserPasskeyRequest {
        BrowserPasskeyRequest {
            request_id: request_id(),
            pairing_label: "Chrome".into(),
            kind: BrowserPasskeyKind::Get,
            origin: "https://example.com".into(),
            rp_id: "example.com".into(),
            user_name: None,
            user_display_name: None,
            candidates: vec![
                BrowserPasskeyCandidate {
                    vault_id: ids::vault(1),
                    item_id: ids::item(5),
                    vault_name: "Personal".into(),
                    user_name: "alex@example.com".into(),
                    user_display_name: "Alex Example".into(),
                },
                BrowserPasskeyCandidate {
                    vault_id: ids::vault(2),
                    item_id: ids::item(6),
                    vault_name: "Work".into(),
                    user_name: "alex.work".into(),
                    user_display_name: String::new(),
                },
            ],
            excluded: false,
            needs_password: true,
            user_verification: PasskeyVerification::Required,
            requested_at: 1_761_480_035_000,
            expires_at: 1_761_480_335_000,
        }
    }

    pub(crate) fn passkey_create_request() -> BrowserPasskeyRequest {
        BrowserPasskeyRequest {
            kind: BrowserPasskeyKind::Create,
            user_name: Some("alex@example.com".into()),
            user_display_name: Some("Alex Example".into()),
            candidates: vec![],
            needs_password: false,
            user_verification: PasskeyVerification::Preferred,
            ..passkey_get_request()
        }
    }

    pub(crate) fn passkey_excluded_request() -> BrowserPasskeyRequest {
        BrowserPasskeyRequest {
            excluded: true,
            user_verification: PasskeyVerification::Discouraged,
            ..passkey_create_request()
        }
    }

    pub(crate) fn save_request_new() -> BrowserSaveRequest {
        BrowserSaveRequest {
            kind: BrowserSaveKind::New,
            candidates: vec![],
            insecure: true,
            origin: "http://example.com".into(),
            username: None,
            ..save_request()
        }
    }
}
