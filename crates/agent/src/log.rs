//! Agent log entries (agent access design §9.4, CRYPTO_SPEC §11, ADR-0160).
//!
//! The engine produces one [`LogEntry`] per decision; the host seals it into the local agent log
//! (`UnlockedSession::append_agent_log`). An entry holds facts about a use, never a value, a
//! header, a body or a query string: the `path` is the URL path only.

use serde::{Deserialize, Serialize};

use crate::binding::InjectKind;
use crate::error::AgentErrorCode;
use crate::grants::GrantState;

/// What happened.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogKind {
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
    /// "Stop all agent access now", lock or switching agent access off.
    StoppedAll,
    /// The user confirmed a binding on this device.
    BindingConfirmed,
}

/// One log entry. Everything except `at_ms` is sealed in the store.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Wall-clock time in milliseconds.
    pub at_ms: u64,
    /// What happened.
    pub kind: LogKind,
    /// The pairing, as 32 hex digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing_id: Option<String>,
    /// The pairing's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing_name: Option<String>,
    /// The vault.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_id: Option<String>,
    /// The item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    /// The item's title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_title: Option<String>,
    /// The field's label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_label: Option<String>,
    /// The method of a request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// The origin of a request (`https://host[:port]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// The URL path of a request (no query, no fragment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The response status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// The size of the response body read, in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// How many replacements the masker made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masked: Option<u32>,
    /// The refusal or failure code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<AgentErrorCode>,
    /// The grant that was given or revoked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant: Option<GrantState>,
    /// The destinations of the binding at the time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origins: Option<Vec<String>>,
    /// How the value is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inject: Option<InjectKind>,
}

impl LogEntry {
    /// An entry of `kind` at `at_ms` with nothing else set.
    #[must_use]
    pub const fn new(kind: LogKind, at_ms: u64) -> Self {
        Self {
            at_ms,
            kind,
            pairing_id: None,
            pairing_name: None,
            vault_id: None,
            item_id: None,
            item_title: None,
            field_label: None,
            method: None,
            origin: None,
            path: None,
            status: None,
            bytes: None,
            masked: None,
            code: None,
            grant: None,
            origins: None,
            inject: None,
        }
    }

    /// Whether this is a refusal row (any `refused` entry, including the `rateLimited` summary).
    /// These are kept under their own, smaller cap in the store and can never evict other kinds.
    #[must_use]
    pub fn is_noisy(&self) -> bool {
        self.kind == LogKind::Refused
    }

    /// The JSON bytes that are sealed into the log.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Reads an entry back; `None` for anything that is not an entry.
    #[must_use]
    pub fn from_json(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// 32 lower-case hex digits for a pairing ID.
#[must_use]
pub fn hex16(id: &[u8; 16]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(32);
    for byte in id {
        out.push(char::from(
            DIGITS.get(usize::from(byte >> 4)).copied().unwrap_or(b'0'),
        ));
        out.push(char::from(
            DIGITS.get(usize::from(byte & 15)).copied().unwrap_or(b'0'),
        ));
    }
    out
}
