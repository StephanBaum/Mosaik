//! Browser extension endpoint (M4, ADR-0036, ADR-0037): the paired browsers,
//! the pairing prompt and the save prompt. The channel to the extension
//! itself speaks `mosaic-native-messaging`'s protocol and is not part of
//! this contract.
//!
//! A pairing request arrives as the `browserPairingRequested` event and is
//! answered with `browser_pairing_respond`; a save request arrives as
//! `browserSaveRequested` and is answered with `browser_save_respond`. Both
//! close without an answer on expiry or lock (`...Closed`). The captured
//! password never reaches the webview: the save request carries no secret,
//! and the dialog shows a fixed mask.
//!
//! This module imports only `serde` and contract types, because the desktop
//! fuzz crate includes `src/contract/` by path.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::macros::wire_enum;

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
    /// Why a pairing or save prompt closed.
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
                pairings: vec![],
            },
            BrowserStatus {
                endpoint: BrowserEndpointState::Unavailable,
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
