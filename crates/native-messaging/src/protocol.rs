//! The extension and desktop envelope (`docs/NATIVE_MESSAGING.md` §5).
//!
//! The same JSON travels between the extension and the host (stdio) and,
//! sealed in AEAD frames, between the host and the desktop.
//!
//! ```text
//! request  {"v":1,"id":<u32>,"op":<Op>,"body":{...}}
//! response {"v":1,"id":7,"ok":true,"body":{...}} | {"v":1,"id":7,"ok":false,"error":<ErrorCode>}
//! event    {"v":1,"event":"state","state":"locked"|"unlocked"} | {"v":1,"event":"unpaired"}
//!          | {"v":1,"event":"disconnected"}
//!          | {"v":1,"event":"pairCode","requestId":<id>,"code":"4F7K2Q","expiresInSeconds":120}
//! ```
//!
//! Rules (DESKTOP_IPC §1.4): camelCase, every key present, `null` for no
//! value, unknown keys rejected. Secrets are JSON strings of type [`Secret`].

use core::fmt;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use crate::error::Error;

/// The protocol version in every message.
pub const PROTOCOL_VERSION: u8 = 1;

/// The largest frame on either link, in bytes of payload.
pub const MAX_FRAME_BYTES: usize = 262_144;

/// Seconds to connect to the desktop endpoint.
pub const CONNECT_TIMEOUT_SECS: u64 = 2;
/// Seconds for each step of the handshake and of a pre-auth exchange.
pub const HANDSHAKE_STEP_TIMEOUT_SECS: u64 = 5;
/// Seconds the host waits for a response to a forwarded request.
pub const REQUEST_TIMEOUT_SECS: u64 = 15;
/// Seconds a pairing request stays open.
pub const PAIR_TIMEOUT_SECS: u64 = 120;
/// Seconds a save prompt stays open (the extension waits longer).
pub const SAVE_PROMPT_TIMEOUT_SECS: u64 = 300;
/// Connections the desktop serves at once (more are accepted and closed).
pub const MAX_CONNECTIONS: usize = 8;
/// Requests in flight per connection.
pub const MAX_IN_FLIGHT: usize = 4;
/// Pairings the desktop keeps.
pub const MAX_PAIRINGS: usize = 16;

/// A secret JSON string (a password, an OTP code, a generated value, a
/// captured password). Zeroized on drop, not `Clone`, `Debug` prints
/// `[REDACTED]`. Parsing leaves residue the parser owns (THREAT_MODEL R40).
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Wraps `value`.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Borrows the text. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for Secret {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// Every error code that can be on the wire.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    // Desktop.
    /// Malformed request, unknown or forbidden op, bad field.
    InvalidRequest,
    /// No pairing with this ID (or no pairing at all).
    NotPaired,
    /// The message has a protocol version other than 1.
    ProtocolVersion,
    /// Another prompt or too many requests are in flight.
    Busy,
    /// The vault is locked.
    Locked,
    /// A rate limit was hit.
    RateLimited,
    /// No saved URL of the item matches the page.
    OriginMismatch,
    /// The page is plain HTTP and the URL does not allow it.
    InsecureOrigin,
    /// A cross-origin frame asked for a fill.
    CrossOriginFrame,
    /// No such item (or not a login).
    ItemNotFound,
    /// The item's vault is not open.
    VaultUnavailable,
    /// The item has no one-time password.
    NoOtp,
    /// The item's one-time password is counter based.
    OtpNotFillable,
    /// The requested field is empty.
    FieldEmpty,
    /// The clipboard cannot be written.
    ClipboardUnavailable,
    /// The user cancelled.
    Cancelled,
    /// The user denied the pairing.
    PairingDenied,
    /// The pairing request timed out.
    PairingExpired,
    /// Another pairing request replaced this one.
    PairingConflict,
    /// Too many denials: pairing requests are blocked for a while.
    PairingBlocked,
    /// The desktop keeps no more pairings.
    PairingLimit,
    /// An unexpected failure.
    Internal,
    // Host.
    /// Nothing serves the endpoint.
    DesktopNotRunning,
    /// The desktop failed the handshake.
    DesktopNotAuthentic,
    /// A request took too long.
    Timeout,
    /// A message would exceed the frame cap.
    FrameTooLarge,
}

impl ErrorCode {
    /// Every code, in the order of the protocol document.
    pub const ALL: [Self; 26] = [
        Self::InvalidRequest,
        Self::NotPaired,
        Self::ProtocolVersion,
        Self::Busy,
        Self::Locked,
        Self::RateLimited,
        Self::OriginMismatch,
        Self::InsecureOrigin,
        Self::CrossOriginFrame,
        Self::ItemNotFound,
        Self::VaultUnavailable,
        Self::NoOtp,
        Self::OtpNotFillable,
        Self::FieldEmpty,
        Self::ClipboardUnavailable,
        Self::Cancelled,
        Self::PairingDenied,
        Self::PairingExpired,
        Self::PairingConflict,
        Self::PairingBlocked,
        Self::PairingLimit,
        Self::Internal,
        Self::DesktopNotRunning,
        Self::DesktopNotAuthentic,
        Self::Timeout,
        Self::FrameTooLarge,
    ];
}

/// An operation of the extension protocol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Op {
    /// Host-local: start (or restart) the session with a stored pairing.
    Hello,
    /// Host-local: start pairing.
    Pair,
    /// Lock state of the desktop.
    Status,
    /// Bring the desktop window forward.
    Focus,
    /// Logins matching an origin.
    ListForOrigin,
    /// Search logins by text.
    Search,
    /// One credential for a fill.
    Fill,
    /// Copy a field on the desktop's clipboard.
    Copy,
    /// Generate a password.
    Generate,
    /// Would a captured login be new, an update or unchanged.
    CheckSave,
    /// Ask the user to save a captured login.
    ProposeSave,
    /// Remove this pairing.
    Unpair,
}

impl Op {
    /// Every op.
    pub const ALL: [Self; 12] = [
        Self::Hello,
        Self::Pair,
        Self::Status,
        Self::Focus,
        Self::ListForOrigin,
        Self::Search,
        Self::Fill,
        Self::Copy,
        Self::Generate,
        Self::CheckSave,
        Self::ProposeSave,
        Self::Unpair,
    ];

    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Pair => "pair",
            Self::Status => "status",
            Self::Focus => "focus",
            Self::ListForOrigin => "listForOrigin",
            Self::Search => "search",
            Self::Fill => "fill",
            Self::Copy => "copy",
            Self::Generate => "generate",
            Self::CheckSave => "checkSave",
            Self::ProposeSave => "proposeSave",
            Self::Unpair => "unpair",
        }
    }

    /// The op with this wire name. This is the allowlist: nothing else (no
    /// `get_item`, `export`, `reveal_secret`, ...) is an op.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.as_str() == name)
    }

    /// Whether the host relays this op to the desktop (every op except the
    /// host-local `hello` and `pair`).
    #[must_use]
    pub fn is_forwarded(self) -> bool {
        FORWARDED_OPS.contains(&self)
    }
}

/// The ops the host forwards to the desktop.
pub const FORWARDED_OPS: [Op; 10] = [
    Op::Status,
    Op::Focus,
    Op::ListForOrigin,
    Op::Search,
    Op::Fill,
    Op::Copy,
    Op::Generate,
    Op::CheckSave,
    Op::ProposeSave,
    Op::Unpair,
];

/// The lock state the desktop reports.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LockState {
    /// No vault key in memory.
    Locked,
    /// The vault is open.
    Unlocked,
}

/// A request from the extension (or, sealed, from the host).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    /// Protocol version.
    pub v: u8,
    /// Correlation ID, chosen by the extension.
    pub id: u32,
    /// The op's wire name.
    pub op: String,
    /// The op's body, not parsed by the host.
    pub body: Box<RawValue>,
}

/// A response from the host or the desktop.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    /// Protocol version.
    pub v: u8,
    /// The request's ID.
    pub id: u32,
    /// Whether the request succeeded.
    pub ok: bool,
    /// The result, if `ok`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Box<RawValue>>,
    /// The failure, if not `ok`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorCode>,
}

impl ResponseEnvelope {
    /// Checks the shape: version 1, `ok` with a body and no error, or not
    /// `ok` with an error and no body.
    ///
    /// # Errors
    /// [`Error::ProtocolVersion`] or [`Error::Malformed`].
    pub fn validate(&self) -> Result<(), Error> {
        if self.v != PROTOCOL_VERSION {
            return Err(Error::ProtocolVersion);
        }
        let shaped = if self.ok {
            self.body.is_some() && self.error.is_none()
        } else {
            self.body.is_none() && self.error.is_some()
        };
        if shaped {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}

/// A message the desktop pushes without a request.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    /// The lock state changed (also the first message of a session).
    State {
        /// Protocol version.
        v: u8,
        /// The new state.
        state: LockState,
    },
    /// The pairing was removed on the desktop.
    Unpaired {
        /// Protocol version.
        v: u8,
    },
    /// The host lost the desktop.
    Disconnected {
        /// Protocol version.
        v: u8,
    },
    /// The code of a pending pairing request.
    PairCode {
        /// Protocol version.
        v: u8,
        /// The `pair` request's ID.
        request_id: u32,
        /// Six Crockford base32 symbols.
        code: String,
        /// Seconds until the request expires.
        expires_in_seconds: u32,
    },
}

impl Event {
    /// The protocol version of the event.
    #[must_use]
    pub const fn version(&self) -> u8 {
        match self {
            Self::State { v, .. }
            | Self::Unpaired { v }
            | Self::Disconnected { v }
            | Self::PairCode { v, .. } => *v,
        }
    }

    /// A `state` event.
    #[must_use]
    pub const fn state(state: LockState) -> Self {
        Self::State {
            v: PROTOCOL_VERSION,
            state,
        }
    }
}

/// What a desktop-to-host plaintext is.
#[derive(Debug)]
pub enum DesktopMessage {
    /// An answer to a request.
    Response(ResponseEnvelope),
    /// A push.
    Event(Event),
}

impl DesktopMessage {
    /// Parses and validates a plaintext from the desktop.
    ///
    /// # Errors
    /// [`Error::ProtocolVersion`] for a version other than 1,
    /// [`Error::Malformed`] for anything else that is not a response or
    /// event of this protocol.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        #[derive(Deserialize)]
        struct Probe<'a> {
            #[serde(borrow, default)]
            event: Option<&'a str>,
        }
        let probe: Probe<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
        if probe.event.is_some() {
            let event: Event = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
            if event.version() != PROTOCOL_VERSION {
                return Err(Error::ProtocolVersion);
            }
            Ok(Self::Event(event))
        } else {
            let response: ResponseEnvelope =
                serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
            response.validate()?;
            Ok(Self::Response(response))
        }
    }
}

/// Serializes `value` into zeroizing memory.
///
/// # Errors
/// [`Error::Malformed`] if serialization fails (it cannot for these types).
pub fn to_zeroizing<T: Serialize + ?Sized>(value: &T) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut out = Zeroizing::new(Vec::with_capacity(512));
    serde_json::to_writer(&mut *out, value).map_err(|_| Error::Malformed)?;
    Ok(out)
}

/// A request with a typed body, for senders (tests, fixtures, the host's
/// re-sealing).
#[derive(Serialize)]
struct RequestRef<'a, T: Serialize + ?Sized> {
    v: u8,
    id: u32,
    op: &'a str,
    body: &'a T,
}

/// An `ok` response with a typed body.
#[derive(Serialize)]
struct OkRef<'a, T: Serialize + ?Sized> {
    v: u8,
    id: u32,
    ok: bool,
    body: &'a T,
}

/// An error response.
#[derive(Serialize)]
struct ErrRef {
    v: u8,
    id: u32,
    ok: bool,
    error: ErrorCode,
}

/// The JSON of a request.
///
/// # Errors
/// [`Error::Malformed`] if serialization fails.
pub fn request_json<T: Serialize + ?Sized>(
    id: u32,
    op: Op,
    body: &T,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    to_zeroizing(&RequestRef {
        v: PROTOCOL_VERSION,
        id,
        op: op.as_str(),
        body,
    })
}

/// The JSON of an `ok` response.
///
/// # Errors
/// [`Error::Malformed`] if serialization fails.
pub fn ok_json<T: Serialize + ?Sized>(id: u32, body: &T) -> Result<Zeroizing<Vec<u8>>, Error> {
    to_zeroizing(&OkRef {
        v: PROTOCOL_VERSION,
        id,
        ok: true,
        body,
    })
}

/// The JSON of an error response.
///
/// # Errors
/// [`Error::Malformed`] if serialization fails.
pub fn error_json(id: u32, error: ErrorCode) -> Result<Zeroizing<Vec<u8>>, Error> {
    to_zeroizing(&ErrRef {
        v: PROTOCOL_VERSION,
        id,
        ok: false,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_camel_case_and_complete() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::DesktopNotAuthentic).unwrap(),
            "\"desktopNotAuthentic\""
        );
        for code in ErrorCode::ALL {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(serde_json::from_str::<ErrorCode>(&json).unwrap(), code);
        }
        let mut seen = std::collections::HashSet::new();
        assert!(ErrorCode::ALL.iter().all(|c| seen.insert(*c)));
        assert!(serde_json::from_str::<ErrorCode>("\"hostMissing\"").is_err());
    }

    #[test]
    fn the_op_allowlist_is_closed() {
        for op in Op::ALL {
            assert_eq!(Op::from_wire(op.as_str()), Some(op));
        }
        for name in [
            "get_item",
            "getItem",
            "export",
            "reveal_secret",
            "revealSecret",
            "unlock",
            "lock",
            "add_item",
            "list_items",
            "",
            "Fill",
            "fill ",
        ] {
            assert_eq!(Op::from_wire(name), None, "{name:?}");
        }
        assert!(!Op::Hello.is_forwarded() && !Op::Pair.is_forwarded());
        assert_eq!(
            Op::ALL.iter().filter(|op| op.is_forwarded()).count(),
            FORWARDED_OPS.len()
        );
        assert_eq!(FORWARDED_OPS.len(), Op::ALL.len() - 2);
    }

    #[test]
    fn requests_reject_unknown_fields_and_keep_the_body_verbatim() {
        let ok = br#"{"v":1,"id":7,"op":"status","body":{ "a" : [1, 2] }}"#;
        let req: RequestEnvelope = serde_json::from_slice(ok).unwrap();
        assert_eq!((req.v, req.id, req.op.as_str()), (1, 7, "status"));
        assert_eq!(req.body.get(), r#"{ "a" : [1, 2] }"#);
        for bad in [
            &br#"{"v":1,"id":7,"op":"status","body":{},"extra":1}"#[..],
            br#"{"v":1,"id":7,"op":"status"}"#,
            br#"{"v":1,"id":-1,"op":"status","body":{}}"#,
            br#"{"v":1,"id":4294967296,"op":"status","body":{}}"#,
            br#"{"v":1,"id":7,"op":5,"body":{}}"#,
            br"[1]",
            b"",
            b"not json",
        ] {
            assert!(serde_json::from_slice::<RequestEnvelope>(bad).is_err());
        }
    }

    #[test]
    fn responses_are_validated() {
        let parse = |s: &str| DesktopMessage::parse(s.as_bytes());
        let is_response =
            |m: Result<DesktopMessage, Error>| matches!(m, Ok(DesktopMessage::Response(_)));
        assert!(is_response(parse(r#"{"v":1,"id":7,"ok":true,"body":{}}"#)));
        assert!(is_response(parse(
            r#"{"v":1,"id":7,"ok":false,"error":"locked"}"#
        )));
        assert!(matches!(
            parse(r#"{"v":2,"id":7,"ok":true,"body":{}}"#),
            Err(Error::ProtocolVersion)
        ));
        for bad in [
            r#"{"v":1,"id":7,"ok":true}"#,
            r#"{"v":1,"id":7,"ok":true,"body":{},"error":"locked"}"#,
            r#"{"v":1,"id":7,"ok":false}"#,
            r#"{"v":1,"id":7,"ok":false,"body":{},"error":"locked"}"#,
            r#"{"v":1,"id":7,"ok":false,"error":"hostMissing"}"#,
            r#"{"v":1,"id":7,"ok":true,"body":{},"x":1}"#,
            r#"{"v":1,"ok":true,"body":{}}"#,
            r#"{"v":1}"#,
            "null",
            "[]",
        ] {
            assert!(matches!(parse(bad), Err(Error::Malformed)), "{bad}");
        }
    }

    #[test]
    fn events_round_trip_and_reject_unknown_fields() {
        let cases = [
            (
                Event::state(LockState::Locked),
                r#"{"event":"state","v":1,"state":"locked"}"#,
            ),
            (Event::Unpaired { v: 1 }, r#"{"event":"unpaired","v":1}"#),
            (
                Event::Disconnected { v: 1 },
                r#"{"event":"disconnected","v":1}"#,
            ),
            (
                Event::PairCode {
                    v: 1,
                    request_id: 9,
                    code: "4F7K2Q".into(),
                    expires_in_seconds: 120,
                },
                r#"{"event":"pairCode","v":1,"requestId":9,"code":"4F7K2Q","expiresInSeconds":120}"#,
            ),
        ];
        for (event, json) in cases {
            assert_eq!(serde_json::to_string(&event).unwrap(), json);
            assert!(matches!(
                DesktopMessage::parse(json.as_bytes()),
                Ok(DesktopMessage::Event(e)) if e == event
            ));
        }
        for bad in [
            r#"{"event":"state","v":1,"state":"locked","x":1}"#,
            r#"{"event":"state","v":1}"#,
            r#"{"event":"nope","v":1}"#,
            r#"{"event":"unpaired"}"#,
            r#"{"event":5,"v":1}"#,
        ] {
            assert!(
                matches!(DesktopMessage::parse(bad.as_bytes()), Err(Error::Malformed)),
                "{bad}"
            );
        }
        assert!(matches!(
            DesktopMessage::parse(br#"{"event":"unpaired","v":2}"#),
            Err(Error::ProtocolVersion)
        ));
    }

    #[test]
    fn builders_produce_the_documented_json() {
        let req = request_json(7, Op::Status, &serde_json::json!({})).unwrap();
        assert_eq!(&**req, br#"{"v":1,"id":7,"op":"status","body":{}}"#);
        let ok = ok_json(7, &serde_json::json!({"state": "locked"})).unwrap();
        assert_eq!(
            &**ok,
            br#"{"v":1,"id":7,"ok":true,"body":{"state":"locked"}}"#
        );
        let err = error_json(7, ErrorCode::NotPaired).unwrap();
        assert_eq!(&**err, br#"{"v":1,"id":7,"ok":false,"error":"notPaired"}"#);
    }

    #[test]
    fn secrets_serialize_but_never_print() {
        let s = Secret::new("hunter2".to_owned());
        assert_eq!(serde_json::to_string(&s).unwrap(), "\"hunter2\"");
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        assert_eq!(s.expose_secret(), "hunter2");
        let back: Secret = serde_json::from_str("\"pw\"").unwrap();
        assert_eq!(back.expose_secret(), "pw");
        assert!(serde_json::from_str::<Secret>("5").is_err());
    }
}
