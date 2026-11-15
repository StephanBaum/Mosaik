//! Pre-auth messages between the MCP relay and the desktop on the agent
//! channel (`docs/NATIVE_MESSAGING.md` section 9): JSON frames before the AEAD
//! session starts.
//!
//! The shapes follow the browser channel's [`preauth`](crate::preauth): every
//! message has `v: 1` and a `type`, unknown fields are rejected, and
//! identifiers, nonces, proofs and keys are lowercase hex of a fixed length.

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::hex::{Hex, HexKey};
use crate::protocol::PROTOCOL_VERSION;

/// The longest pairing name in characters.
pub const MAX_PAIRING_NAME_CHARS: usize = 64;

/// Whether `name` is a valid pairing name: 1 to 64 characters of ASCII
/// letters, digits, space, `.`, `_`, `-`, `(` and `)`, with no leading or
/// trailing space.
#[must_use]
pub fn pairing_name_is_valid(name: &str) -> bool {
    let len = name.chars().count();
    (1..=MAX_PAIRING_NAME_CHARS).contains(&len)
        && !name.starts_with(' ')
        && !name.ends_with(' ')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-' | '(' | ')'))
}

/// Whether the vault is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultState {
    /// No vault key in memory.
    Locked,
    /// The vault is open.
    Unlocked,
}

/// Whether the user turned agent access on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessState {
    /// Agent access is on.
    On,
    /// Agent access is off.
    Off,
}

/// Why the desktop refused a pre-auth message.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRefusal {
    /// No pairing with this ID, or the proof is wrong (the same answer).
    NotPaired,
    /// The vault is locked.
    Locked,
    /// Agent access is off.
    AgentAccessOff,
    /// The user denied the pairing.
    PairingDenied,
    /// The pairing request timed out.
    PairingExpired,
    /// 16 agent pairings exist already.
    PairingLimit,
    /// A pairing with this name exists already (compared without case).
    PairingNameTaken,
    /// Too many denials: pairing dialogs are blocked for a while.
    PairingBlocked,
    /// A rate limit was hit.
    RateLimited,
    /// Too many connections or another dialog is open.
    Busy,
    /// The message has a protocol version other than 1.
    ProtocolVersion,
    /// Malformed message or a bad field.
    InvalidRequest,
    /// A failure on the desktop.
    Internal,
}

/// A message from the relay to the desktop.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AgentClientMessage {
    /// Starts a handshake for a stored pairing.
    Hello {
        /// Protocol version.
        v: u8,
        /// The pairing the relay wants to use.
        pairing_id: Hex<16>,
        /// The relay's fresh nonce.
        nonce: Hex<32>,
    },
    /// The relay's proof, answering the desktop's `challenge`.
    Finish {
        /// Protocol version.
        v: u8,
        /// `proof_h`.
        proof: Hex<32>,
    },
    /// Asks the user to approve a new pairing under `name`.
    PairRequest {
        /// Protocol version.
        v: u8,
        /// The client name the user typed (display only).
        name: String,
    },
    /// Asks for the vault and access state, unauthenticated.
    Status {
        /// Protocol version.
        v: u8,
    },
}

/// A message from the desktop to the relay.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AgentServerMessage {
    /// The desktop's nonce and its proof, sent first.
    Challenge {
        /// Protocol version.
        v: u8,
        /// The desktop's fresh nonce.
        nonce: Hex<32>,
        /// `proof_d`.
        proof: Hex<32>,
    },
    /// The request is refused.
    Refused {
        /// Protocol version.
        v: u8,
        /// Why.
        reason: AgentRefusal,
    },
    /// The vault and access state.
    Status {
        /// Protocol version.
        v: u8,
        /// Locked or unlocked.
        vault: VaultState,
        /// Agent access on or off.
        agent_access: AccessState,
    },
    /// The pairing request waits for the user.
    PairPending {
        /// Protocol version.
        v: u8,
        /// The code the user compares, six Crockford base32 symbols.
        code: String,
        /// Seconds until the request expires.
        expires_in_seconds: u32,
    },
    /// The user allowed the pairing.
    PairApproved {
        /// Protocol version.
        v: u8,
        /// The new pairing's ID.
        pairing_id: Hex<16>,
        /// `K_pair`.
        key: HexKey,
    },
}

impl AgentClientMessage {
    /// Parses a frame from the relay.
    ///
    /// # Errors
    /// [`Error::ProtocolVersion`] for a `v` other than 1, [`Error::Malformed`]
    /// for anything else that is not a message of this protocol, and for a
    /// `pairRequest` whose name fails [`pairing_name_is_valid`].
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let message: Self = crate::preauth::parse(bytes)?;
        if let Self::PairRequest { name, .. } = &message
            && !pairing_name_is_valid(name)
        {
            return Err(Error::Malformed);
        }
        Ok(message)
    }
}

impl AgentServerMessage {
    /// Parses a frame from the desktop.
    ///
    /// # Errors
    /// As [`AgentClientMessage::parse`].
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        crate::preauth::parse(bytes)
    }

    /// A `refused` message.
    #[must_use]
    pub const fn refused(reason: AgentRefusal) -> Self {
        Self::Refused {
            v: PROTOCOL_VERSION,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_request_name_rules() {
        assert!(pairing_name_is_valid("Claude Code"));
        assert!(pairing_name_is_valid("Cursor (work)"));
        assert!(pairing_name_is_valid("a"));
        assert!(pairing_name_is_valid(&"a".repeat(64)));
        assert!(!pairing_name_is_valid(&"a".repeat(65)));
        assert!(!pairing_name_is_valid(""));
        assert!(!pairing_name_is_valid(" lead"));
        assert!(!pairing_name_is_valid("trail "));
        assert!(!pairing_name_is_valid("tab\there"));
        assert!(!pairing_name_is_valid("new\nline"));
        assert!(!pairing_name_is_valid("nul\0"));
        assert!(!pairing_name_is_valid("umlaut \u{e4}"));
        assert!(!pairing_name_is_valid("a/b"));
        let ok = r#"{"type":"pairRequest","v":1,"name":"Claude Code"}"#;
        assert!(matches!(
            AgentClientMessage::parse(ok.as_bytes()),
            Ok(AgentClientMessage::PairRequest { .. })
        ));
        let long = format!(
            r#"{{"type":"pairRequest","v":1,"name":"{}"}}"#,
            "a".repeat(65)
        );
        assert_eq!(
            AgentClientMessage::parse(long.as_bytes()).unwrap_err(),
            Error::Malformed
        );
        let ctl = r#"{"type":"pairRequest","v":1,"name":"a\u0001b"}"#;
        assert_eq!(
            AgentClientMessage::parse(ctl.as_bytes()).unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    fn preauth_rejects_unknown_fields_and_versions() {
        let id = "0a".repeat(16);
        let nonce = "0b".repeat(32);
        let hello = format!(r#"{{"type":"hello","v":1,"pairingId":"{id}","nonce":"{nonce}"}}"#);
        let msg = AgentClientMessage::parse(hello.as_bytes()).unwrap();
        assert_eq!(serde_json::to_string(&msg).unwrap(), hello);
        assert_eq!(
            AgentClientMessage::parse(
                format!(r#"{{"type":"hello","v":2,"pairingId":"{id}","nonce":"{nonce}"}}"#)
                    .as_bytes()
            )
            .unwrap_err(),
            Error::ProtocolVersion
        );
        for bad in [
            r#"{"type":"status","v":1,"x":1}"#,
            r#"{"type":"status"}"#,
            r#"{"type":"focus","v":1}"#,
            r#"{"type":"pairRequest","v":1}"#,
            r#"{"type":"pairRequest","v":1,"name":"a","browser":"firefox"}"#,
            r#"{"type":"hello","v":1,"pairingId":"00","nonce":"00"}"#,
            "[]",
            "",
        ] {
            assert_eq!(
                AgentClientMessage::parse(bad.as_bytes()).unwrap_err(),
                Error::Malformed,
                "{bad}"
            );
        }
        for bad in [
            r#"{"type":"status","v":1,"vault":"locked"}"#,
            r#"{"type":"refused","v":1,"reason":"hostMissing"}"#,
            r#"{"type":"focused","v":1}"#,
        ] {
            assert_eq!(
                AgentServerMessage::parse(bad.as_bytes()).unwrap_err(),
                Error::Malformed,
                "{bad}"
            );
        }
        assert_eq!(
            AgentServerMessage::parse(br#"{"type":"status","v":3}"#).unwrap_err(),
            Error::ProtocolVersion
        );
    }

    #[test]
    fn server_messages_round_trip() {
        let json =
            serde_json::to_string(&AgentServerMessage::refused(AgentRefusal::AgentAccessOff))
                .unwrap();
        assert_eq!(
            json,
            r#"{"type":"refused","v":1,"reason":"agentAccessOff"}"#
        );
        assert!(matches!(
            AgentServerMessage::parse(
                br#"{"type":"status","v":1,"vault":"unlocked","agentAccess":"on"}"#
            ),
            Ok(AgentServerMessage::Status {
                vault: VaultState::Unlocked,
                agent_access: AccessState::On,
                ..
            })
        ));
        assert!(matches!(
            AgentServerMessage::parse(
                br#"{"type":"pairPending","v":1,"code":"4F7K2Q","expiresInSeconds":120}"#
            ),
            Ok(AgentServerMessage::PairPending {
                expires_in_seconds: 120,
                ..
            })
        ));
        let approved = format!(
            r#"{{"type":"pairApproved","v":1,"pairingId":"{}","key":"{}"}}"#,
            "01".repeat(16),
            "02".repeat(32)
        );
        assert!(matches!(
            AgentServerMessage::parse(approved.as_bytes()),
            Ok(AgentServerMessage::PairApproved { .. })
        ));
    }
}
