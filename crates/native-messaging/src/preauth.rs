//! Pre-auth messages between the host and the desktop (`docs/NATIVE_MESSAGING.md`
//! §4.2): JSON frames before the AEAD session starts.
//!
//! Every message has `v: 1` and a `type`; unknown fields are rejected;
//! identifiers, nonces, proofs and keys are lowercase hex of a fixed length
//! (pairing ID 32 digits, nonce and proof 64, key 64).

use serde::{Deserialize, Serialize};

use crate::bodies::BrowserKind;
use crate::bodies::Hint;
use crate::error::Error;
use crate::hex::{Hex, HexKey};
use crate::protocol::{ErrorCode, LockState, PROTOCOL_VERSION};

/// A message from the host to the desktop.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ClientMessage {
    /// Starts a handshake for a stored pairing.
    Hello {
        /// Protocol version.
        v: u8,
        /// The pairing the host wants to use.
        pairing_id: Hex<16>,
        /// The host's fresh nonce.
        nonce: Hex<32>,
    },
    /// The host's proof, answering the desktop's `challenge`.
    Finish {
        /// Protocol version.
        v: u8,
        /// `proof_h`.
        proof: Hex<32>,
    },
    /// Asks the user to approve a new pairing.
    PairRequest {
        /// Protocol version.
        v: u8,
        /// Set by the host from the verified caller.
        browser: BrowserKind,
        /// The extension's guess, for display only.
        hint: Hint,
    },
    /// Asks for the lock state, unauthenticated.
    Status {
        /// Protocol version.
        v: u8,
    },
    /// Asks to bring the window forward, unauthenticated.
    Focus {
        /// Protocol version.
        v: u8,
    },
}

/// A message from the desktop to the host.
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ServerMessage {
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
        reason: ErrorCode,
    },
    /// The lock state.
    Status {
        /// Protocol version.
        v: u8,
        /// The state.
        state: LockState,
    },
    /// The window was brought forward.
    Focused {
        /// Protocol version.
        v: u8,
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

/// Reads the version, then the typed message.
fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, Error> {
    #[derive(Deserialize)]
    struct Probe {
        v: u8,
    }
    let probe: Probe = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
    if probe.v != PROTOCOL_VERSION {
        return Err(Error::ProtocolVersion);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::Malformed)
}

impl ClientMessage {
    /// Parses a frame from the host.
    ///
    /// # Errors
    /// [`Error::ProtocolVersion`] for a `v` other than 1, [`Error::Malformed`]
    /// for anything else that is not a message of this protocol.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        parse(bytes)
    }
}

impl ServerMessage {
    /// Parses a frame from the desktop.
    ///
    /// # Errors
    /// As [`ClientMessage::parse`].
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        parse(bytes)
    }

    /// A `refused` message.
    #[must_use]
    pub const fn refused(reason: ErrorCode) -> Self {
        Self::Refused {
            v: PROTOCOL_VERSION,
            reason,
        }
    }
}

/// Crockford base32 without `I`, `L`, `O` and `U`.
const CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Length of a pairing code in symbols.
pub const PAIRING_CODE_LEN: usize = 6;

/// A pairing code from 30 random bits: six symbols of Crockford base32.
///
/// # Errors
/// [`Error::Io`] if the OS CSPRNG fails.
pub fn new_pairing_code() -> Result<String, Error> {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).map_err(|_| Error::Io(std::io::ErrorKind::Other))?;
    Ok(pairing_code(u32::from_be_bytes(bytes)))
}

/// The six symbols of the low 30 bits of `bits`.
#[must_use]
pub fn pairing_code(bits: u32) -> String {
    (0..PAIRING_CODE_LEN)
        .rev()
        .filter_map(|i| {
            let index = usize::try_from((bits >> (5 * i)) & 0x1f).ok()?;
            CODE_ALPHABET.get(index).map(|c| char::from(*c))
        })
        .collect()
}

/// Whether `code` is six Crockford base32 symbols (upper case).
#[must_use]
pub fn is_pairing_code(code: &str) -> bool {
    code.len() == PAIRING_CODE_LEN && code.bytes().all(|b| CODE_ALPHABET.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(json: &str) -> Result<ClientMessage, Error> {
        ClientMessage::parse(json.as_bytes())
    }

    fn server(json: &str) -> Result<ServerMessage, Error> {
        ServerMessage::parse(json.as_bytes())
    }

    #[test]
    fn client_messages_parse_and_round_trip() {
        let id = "0a".repeat(16);
        let nonce = "0b".repeat(32);
        let hello = format!(r#"{{"type":"hello","v":1,"pairingId":"{id}","nonce":"{nonce}"}}"#);
        let msg = client(&hello).unwrap();
        assert!(
            matches!(&msg, ClientMessage::Hello { pairing_id, nonce, .. }
            if *pairing_id == Hex([0x0a; 16]) && *nonce == Hex([0x0b; 32]))
        );
        assert_eq!(
            serde_json::to_string(&msg).unwrap(),
            hello,
            "canonical key order: tag first"
        );
        assert!(matches!(
            client(&format!(r#"{{"type":"finish","v":1,"proof":"{nonce}"}}"#)),
            Ok(ClientMessage::Finish { .. })
        ));
        assert!(matches!(
            client(r#"{"type":"pairRequest","v":1,"browser":"firefox","hint":"firefox"}"#),
            Ok(ClientMessage::PairRequest {
                browser: BrowserKind::Firefox,
                ..
            })
        ));
        assert!(matches!(
            client(r#"{"type":"status","v":1}"#),
            Ok(ClientMessage::Status { .. })
        ));
        assert!(matches!(
            client(r#"{"type":"focus","v":1}"#),
            Ok(ClientMessage::Focus { .. })
        ));
    }

    #[test]
    fn client_messages_are_strict() {
        let id = "0a".repeat(16);
        let nonce = "0b".repeat(32);
        assert_eq!(
            client(&format!(
                r#"{{"type":"hello","v":2,"pairingId":"{id}","nonce":"{nonce}"}}"#
            ))
            .unwrap_err(),
            Error::ProtocolVersion
        );
        assert_eq!(
            client(r#"{"type":"nope","v":9}"#).unwrap_err(),
            Error::ProtocolVersion
        );
        for bad in [
            r#"{"type":"nope","v":1}"#,
            r#"{"type":"status","v":1,"x":1}"#,
            r#"{"type":"status"}"#,
            r#"{"v":1}"#,
            r#"{"type":"hello","v":1,"pairingId":"00","nonce":"00"}"#,
            r#"{"type":"pairRequest","v":1,"browser":"edge","hint":"edge"}"#,
            r#"{"type":"pairRequest","v":1,"browser":"firefox"}"#,
            r#"{"type":"challenge","v":1}"#,
            "[]",
            "",
        ] {
            assert_eq!(client(bad).unwrap_err(), Error::Malformed, "{bad}");
        }
        let upper = format!(
            r#"{{"type":"hello","v":1,"pairingId":"{}","nonce":"{nonce}"}}"#,
            "0A".repeat(16)
        );
        assert_eq!(
            client(&upper).unwrap_err(),
            Error::Malformed,
            "uppercase hex"
        );
    }

    #[test]
    fn server_messages_parse_and_round_trip() {
        let nonce = "0c".repeat(32);
        let challenge =
            format!(r#"{{"type":"challenge","v":1,"nonce":"{nonce}","proof":"{nonce}"}}"#);
        assert!(matches!(
            server(&challenge),
            Ok(ServerMessage::Challenge { .. })
        ));
        let refused = ServerMessage::refused(ErrorCode::NotPaired);
        let json = serde_json::to_string(&refused).unwrap();
        assert_eq!(json, r#"{"type":"refused","v":1,"reason":"notPaired"}"#);
        assert!(matches!(
            server(&json),
            Ok(ServerMessage::Refused {
                reason: ErrorCode::NotPaired,
                ..
            })
        ));
        assert!(matches!(
            server(r#"{"type":"status","v":1,"state":"unlocked"}"#),
            Ok(ServerMessage::Status {
                state: LockState::Unlocked,
                ..
            })
        ));
        assert!(matches!(
            server(r#"{"type":"focused","v":1}"#),
            Ok(ServerMessage::Focused { .. })
        ));
        assert!(matches!(
            server(r#"{"type":"pairPending","v":1,"code":"4F7K2Q","expiresInSeconds":120}"#),
            Ok(ServerMessage::PairPending {
                expires_in_seconds: 120,
                ..
            })
        ));
        let approved = format!(
            r#"{{"type":"pairApproved","v":1,"pairingId":"{}","key":"{}"}}"#,
            "01".repeat(16),
            "02".repeat(32)
        );
        let Ok(ServerMessage::PairApproved { key, .. }) = server(&approved) else {
            panic!("not approved");
        };
        assert_eq!(key.0.expose_secret(), &[2u8; 32]);
        assert_eq!(
            server(r#"{"type":"refused","v":1,"reason":"hostMissing"}"#).unwrap_err(),
            Error::Malformed
        );
        assert_eq!(
            server(r#"{"type":"focused","v":3}"#).unwrap_err(),
            Error::ProtocolVersion
        );
    }

    #[test]
    fn pairing_codes() {
        assert_eq!(pairing_code(0), "000000");
        assert_eq!(pairing_code((1 << 30) - 1), "ZZZZZZ");
        assert_eq!(
            pairing_code(0b00001_00010_00011_00100_00101_00110),
            "123456"
        );
        assert_eq!(pairing_code(u32::MAX), "ZZZZZZ", "only 30 bits are used");
        let code = new_pairing_code().unwrap();
        assert!(is_pairing_code(&code), "{code}");
        assert!(!is_pairing_code("4F7K2"));
        assert!(!is_pairing_code("4F7K2Q7"));
        assert!(!is_pairing_code("4f7k2q"));
        assert!(!is_pairing_code("4F7K2I"), "I is not in the alphabet");
        assert!(!is_pairing_code("4F7K2U"), "U is not in the alphabet");
    }
}
