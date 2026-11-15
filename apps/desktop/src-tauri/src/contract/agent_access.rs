//! Agent access, the app side (ADR-0159): the paired agent clients, the pairing and access
//! dialogs, grants, the kill switch and the activity log. The bindings of items are in
//! [`super::agent`].
//!
//! No value crosses this contract. The only text from outside that reaches the webview is the
//! client's name and the agent's reason, and the UI shows both as plain, labelled text.
//!
//! This module imports only `serde` and contract types, because the desktop fuzz crate includes
//! `src/contract/` by path.

use serde::{Deserialize, Serialize};

use super::agent::AgentInjectKind;
use super::browser::PairingDecision;
use super::ids::{UnixMillis, WireItemId};
use super::macros::wire_enum;

wire_enum!(
    /// Whether the endpoint agents connect to is up.
    pub enum AgentEndpointState {
        /// Listening.
        Running,
        /// Agent access is off, so nothing listens.
        Stopped,
        /// It could not be created (another copy runs, no private directory).
        Unavailable,
    }
);

wire_enum!(
    /// What a grant allows.
    pub enum AgentGrantState {
        /// One request.
        Once,
        /// Until `expiresAt`.
        Until,
        /// Until the vault locks.
        UntilLock,
    }
);

wire_enum!(
    /// The user's answer to an access request.
    pub enum AgentAccessDecision {
        /// Refuse.
        Deny,
        /// Allow one request.
        AllowOnce,
        /// Allow for 15 minutes.
        Allow15m,
        /// Allow for 1 hour.
        Allow1h,
        /// Allow until the vault locks.
        AllowUntilLock,
    }
);

wire_enum!(
    /// Why an agent dialog closed.
    pub enum AgentCloseReason {
        /// The user answered (possibly in another window).
        Answered,
        /// Nobody answered in time.
        Expired,
        /// The vault locked.
        Locked,
        /// The agent went away.
        Disconnected,
        /// Agent access was switched off, or the pairing was removed.
        Off,
    }
);

wire_enum!(
    /// What an entry of the activity log records.
    pub enum AgentLogKind {
        /// A client was paired.
        Paired,
        /// A pairing was removed.
        PairingRemoved,
        /// The user allowed access to a secret.
        AccessGranted,
        /// The user denied access, or the dialog timed out.
        AccessDenied,
        /// A request was sent and answered.
        Request,
        /// A request was refused or failed.
        Refused,
        /// A grant was revoked.
        Revoked,
        /// All agent access was stopped.
        StoppedAll,
        /// The user confirmed a binding on this device.
        BindingConfirmed,
    }
);

wire_enum!(
    /// Why a request was refused or failed (the agent error codes, ADR-0159).
    pub enum AgentRefusalCode {
        /// The desktop app is not running.
        DesktopNotRunning,
        /// The vault is locked.
        Locked,
        /// Agent access is off.
        AgentAccessOff,
        /// The client is not paired.
        NotPaired,
        /// The reference names nothing an agent may use.
        NotAgentUsable,
        /// The binding is not confirmed on this device.
        NeedsReview,
        /// The reference matches more than one item.
        AmbiguousRef,
        /// No live grant.
        NoGrant,
        /// The user denied the request.
        Denied,
        /// The destination is not one of the binding's.
        DestinationNotAllowed,
        /// The method is not allowed.
        MethodNotAllowed,
        /// A header is not allowed.
        HeaderNotAllowed,
        /// The request is over a size limit.
        RequestTooLarge,
        /// The response is over a size limit.
        ResponseTooLarge,
        /// The request timed out.
        Timeout,
        /// The TLS handshake or certificate check failed.
        TlsFailed,
        /// Too many requests.
        RateLimited,
        /// Another request or dialog is in progress.
        Busy,
        /// The reference is not well formed.
        InvalidRef,
        /// The request is not well formed.
        InvalidRequest,
        /// The request could not be sent.
        RequestFailed,
        /// An internal error.
        Internal,
    }
);

/// A paired agent client.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPairingView {
    /// The pairing, 32 lowercase hex digits.
    pub pairing_id: String,
    /// The name the client gave at pairing. Not verified.
    pub name: String,
    /// When it was paired.
    pub paired_at: UnixMillis,
    /// When it last made a request (`null` if never).
    pub last_used_at: Option<UnixMillis>,
}

/// A live grant.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentGrantView {
    /// The grant, for `agent_grant_revoke`.
    pub grant_id: u64,
    /// The client's pairing.
    pub pairing_id: String,
    /// The client's name.
    pub pairing_name: String,
    /// The item's title.
    pub item_title: String,
    /// The vault's name.
    pub vault_name: String,
    /// The field's label.
    pub field_label: String,
    /// What it allows.
    pub state: AgentGrantState,
    /// When it ends (`null` unless `until`).
    pub expires_at: Option<UnixMillis>,
}

/// `agent_access_status`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAccessStatus {
    /// The setting.
    pub enabled: bool,
    /// Whether the endpoint is up.
    pub endpoint: AgentEndpointState,
    /// The paired clients.
    pub pairings: Vec<AgentPairingView>,
    /// The live grants.
    pub grants: Vec<AgentGrantView>,
    /// Requests being sent right now.
    pub in_flight: u32,
    /// Where the `mosaic` command line program is (`null` if this build has none next to it).
    pub cli_path: Option<String>,
}

/// `agent_access_set_enabled`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentSetEnabledRequest {
    /// On or off.
    pub enabled: bool,
}

/// A client asks to be paired (`agentPairingRequested`).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPairingRequest {
    /// The dialog's handle.
    pub request_id: String,
    /// The name the client gave. Not verified.
    pub name: String,
    /// The six-symbol code the terminal shows.
    pub code: String,
    /// When it appeared.
    pub requested_at: UnixMillis,
    /// When it is refused automatically.
    pub expires_at: UnixMillis,
}

/// `agent_pairing_respond`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentPairingRespondRequest {
    /// The dialog.
    pub request_id: String,
    /// The answer.
    pub decision: PairingDecision,
}

/// `agent_pairing_remove`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentPairingRemoveRequest {
    /// The pairing, 32 hex digits.
    pub pairing_id: String,
}

/// A client asks for access to a secret (`agentAccessRequested`). Facts first; the agent's own
/// words are `reason`, which the UI shows labelled and as plain text.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAccessRequest {
    /// The dialog's handle.
    pub request_id: String,
    /// The name the client gave at pairing. Not verified.
    pub pairing_name: String,
    /// The item's title.
    pub item_title: String,
    /// The vault's name.
    pub vault_name: String,
    /// The field's label.
    pub field_label: String,
    /// Where the value may be sent.
    pub origins: Vec<String>,
    /// How it is sent.
    pub inject: AgentInjectKind,
    /// What the agent says it is for (at most 200 characters).
    pub reason: Option<String>,
    /// Requests waiting behind this one.
    pub waiting: u32,
    /// When it was asked.
    pub requested_at: UnixMillis,
    /// When it is refused automatically (the queue time counts).
    pub expires_at: UnixMillis,
}

/// `agent_access_respond`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAccessRespondRequest {
    /// The dialog.
    pub request_id: String,
    /// The answer.
    pub decision: AgentAccessDecision,
}

/// `agent_grant_revoke`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentGrantRevokeRequest {
    /// The grant.
    pub grant_id: u64,
}

/// `agent_log`.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentLogRequest {
    /// Only this client.
    pub pairing_id: Option<String>,
    /// Only this item.
    pub item_id: Option<WireItemId>,
    /// Only entries before this time (`null`: the newest).
    pub before_ms: Option<UnixMillis>,
    /// At most this many (capped at 200).
    pub limit: u32,
}

/// One entry of the activity log. Facts only: never a value, header, body or query string.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLogView {
    /// When.
    pub at_ms: UnixMillis,
    /// What happened.
    pub kind: AgentLogKind,
    /// The client, 32 hex digits.
    pub pairing_id: Option<String>,
    /// The client's name then.
    pub pairing_name: Option<String>,
    /// The item.
    pub item_id: Option<String>,
    /// The item's title then.
    pub item_title: Option<String>,
    /// The field's label then.
    pub field_label: Option<String>,
    /// The method of a request.
    pub method: Option<String>,
    /// The origin of a request.
    pub origin: Option<String>,
    /// The URL path of a request (no query).
    pub path: Option<String>,
    /// The response status.
    pub status: Option<u16>,
    /// The response bytes read.
    pub bytes: Option<u64>,
    /// How many replacements the masker made.
    pub masked: Option<u32>,
    /// The refusal code.
    pub code: Option<AgentRefusalCode>,
    /// What a grant allowed.
    pub grant: Option<AgentGrantState>,
    /// The destinations of the binding then.
    pub origins: Option<Vec<String>>,
    /// How the value was sent.
    pub inject: Option<AgentInjectKind>,
}

/// One page of the activity log, newest first.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLogPage {
    /// The entries.
    pub entries: Vec<AgentLogView>,
    /// Entries that could not be opened (damaged), skipped.
    pub skipped: u32,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn pairing_id() -> String {
        "0123456789abcdef0123456789abcdef".to_owned()
    }

    pub(crate) fn request_id() -> String {
        "a1b2c3d4e5f60718293a4b5c6d7e8f90".to_owned()
    }

    fn grant(
        grant_id: u64,
        state: AgentGrantState,
        expires_at: Option<UnixMillis>,
    ) -> AgentGrantView {
        AgentGrantView {
            grant_id,
            pairing_id: pairing_id(),
            pairing_name: "Claude Code".into(),
            item_title: "Example API".into(),
            vault_name: "Personal".into(),
            field_label: "api_key.secret".into(),
            state,
            expires_at,
        }
    }

    pub(crate) fn statuses() -> Vec<AgentAccessStatus> {
        vec![
            AgentAccessStatus {
                enabled: true,
                endpoint: AgentEndpointState::Running,
                pairings: vec![AgentPairingView {
                    pairing_id: pairing_id(),
                    name: "Claude Code".into(),
                    paired_at: 1_761_480_000_000,
                    last_used_at: Some(1_761_480_035_000),
                }],
                grants: vec![
                    grant(1, AgentGrantState::Until, Some(1_761_480_900_000)),
                    grant(2, AgentGrantState::UntilLock, None),
                    grant(3, AgentGrantState::Once, None),
                ],
                in_flight: 1,
                cli_path: Some("/usr/local/bin/mosaic".into()),
            },
            AgentAccessStatus {
                enabled: false,
                endpoint: AgentEndpointState::Stopped,
                pairings: vec![],
                grants: vec![],
                in_flight: 0,
                cli_path: None,
            },
            AgentAccessStatus {
                enabled: true,
                endpoint: AgentEndpointState::Unavailable,
                pairings: vec![],
                grants: vec![],
                in_flight: 0,
                cli_path: None,
            },
        ]
    }

    pub(crate) fn pairing_request() -> AgentPairingRequest {
        AgentPairingRequest {
            request_id: request_id(),
            name: "Claude Code".into(),
            code: "K7M2QX".into(),
            requested_at: 1_761_480_000_000,
            expires_at: 1_761_480_120_000,
        }
    }

    pub(crate) fn access_request() -> AgentAccessRequest {
        AgentAccessRequest {
            request_id: request_id(),
            pairing_name: "Claude Code".into(),
            item_title: "Example API".into(),
            vault_name: "Personal".into(),
            field_label: "api_key.secret".into(),
            origins: vec!["https://api.example.com".into()],
            inject: AgentInjectKind::Bearer,
            reason: Some("List the open invoices".into()),
            waiting: 1,
            requested_at: 1_761_480_000_000,
            expires_at: 1_761_480_120_000,
        }
    }

    pub(crate) fn log_pages() -> Vec<AgentLogPage> {
        let base = AgentLogView {
            at_ms: 1_761_480_035_000,
            kind: AgentLogKind::Request,
            pairing_id: Some(pairing_id()),
            pairing_name: Some("Claude Code".into()),
            item_id: Some("019a2b3c-4d5e-7000-8000-000000000001".into()),
            item_title: Some("Example API".into()),
            field_label: Some("api_key.secret".into()),
            method: Some("GET".into()),
            origin: Some("https://api.example.com".into()),
            path: Some("/v1/invoices".into()),
            status: Some(200),
            bytes: Some(2048),
            masked: Some(0),
            code: None,
            grant: None,
            origins: None,
            inject: None,
        };
        let refused = AgentLogView {
            kind: AgentLogKind::Refused,
            status: None,
            bytes: None,
            masked: None,
            code: Some(AgentRefusalCode::DestinationNotAllowed),
            origin: Some("https://other.example.net".into()),
            ..base.clone()
        };
        let granted = AgentLogView {
            kind: AgentLogKind::AccessGranted,
            method: None,
            origin: None,
            path: None,
            status: None,
            bytes: None,
            masked: None,
            grant: Some(AgentGrantState::Until),
            origins: Some(vec!["https://api.example.com".into()]),
            inject: Some(AgentInjectKind::Bearer),
            ..base.clone()
        };
        vec![
            AgentLogPage {
                entries: vec![base, refused, granted],
                skipped: 0,
            },
            AgentLogPage {
                entries: vec![],
                skipped: 2,
            },
        ]
    }
}
