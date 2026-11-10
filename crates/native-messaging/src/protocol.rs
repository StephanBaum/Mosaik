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
use std::io;

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
/// Seconds a passkey prompt stays open (the extension waits longer).
pub const PASSKEY_PROMPT_TIMEOUT_SECS: u64 = 300;
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
    /// A cross-origin frame asked for a fill or a passkey.
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
    /// No usable passkey for this site (answered without a dialog).
    PasskeyNotFound,
    /// A passkey for this account already exists (`excludeCredentials` hit).
    PasskeyExcluded,
    /// The relying party ID is not valid for the page's origin.
    RpIdMismatch,
    /// No offered algorithm or option is supported.
    PasskeyUnsupported,
    /// No such open password-change attempt on that item.
    RotationNotFound,
    /// The origin is not the same registrable domain as the item's saved URL.
    OriginNotRelated,
    /// The item already has four open password-change attempts.
    RotationLimit,
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
    pub const ALL: [Self; 33] = [
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
        Self::PasskeyNotFound,
        Self::PasskeyExcluded,
        Self::RpIdMismatch,
        Self::PasskeyUnsupported,
        Self::RotationNotFound,
        Self::OriginNotRelated,
        Self::RotationLimit,
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
    /// Create a passkey after the user approves it on the desktop.
    PasskeyCreate,
    /// Sign in with a passkey after the user approves it on the desktop.
    PasskeyGet,
    /// Close an open passkey prompt of this pairing.
    PasskeyCancel,
    /// The open password-change attempts of the logins saved for an origin.
    RotationForOrigin,
    /// Start a password change: the desktop dialog generates and confirms it.
    RotationBegin,
    /// The current and the new password of one attempt, for a change form.
    FillChange,
    /// The new password of one attempt (and the user name), to sign in after a partial change.
    FillPending,
    /// Report how a change went; the desktop dialog commits it.
    RotationResult,
    /// Ask to save a related origin for the login; the desktop dialog decides.
    RotationAddOrigin,
}

impl Op {
    /// Every op.
    pub const ALL: [Self; 21] = [
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
        Self::PasskeyCreate,
        Self::PasskeyGet,
        Self::PasskeyCancel,
        Self::RotationForOrigin,
        Self::RotationBegin,
        Self::FillChange,
        Self::FillPending,
        Self::RotationResult,
        Self::RotationAddOrigin,
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
            Self::PasskeyCreate => "passkeyCreate",
            Self::PasskeyGet => "passkeyGet",
            Self::PasskeyCancel => "passkeyCancel",
            Self::RotationForOrigin => "rotationForOrigin",
            Self::RotationBegin => "rotationBegin",
            Self::FillChange => "fillChange",
            Self::FillPending => "fillPending",
            Self::RotationResult => "rotationResult",
            Self::RotationAddOrigin => "rotationAddOrigin",
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
pub const FORWARDED_OPS: [Op; 19] = [
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
    Op::PasskeyCreate,
    Op::PasskeyGet,
    Op::PasskeyCancel,
    Op::RotationForOrigin,
    Op::RotationBegin,
    Op::FillChange,
    Op::FillPending,
    Op::RotationResult,
    Op::RotationAddOrigin,
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
///
/// The body borrows the buffer the request was parsed from. Parse it with
/// `serde_json::from_slice` from a zeroizing frame buffer that outlives the
/// envelope: an owned copy of a body (`Box<RawValue>`) would sit on the heap
/// unwiped and can hold `K_pair`, passwords and OTP codes (THREAT_MODEL R40).
/// `Debug` never prints the body.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope<'a> {
    /// Protocol version.
    pub v: u8,
    /// Correlation ID, chosen by the extension.
    pub id: u32,
    /// The op's wire name.
    pub op: String,
    /// The op's body, not parsed by the host.
    #[serde(borrow)]
    pub body: &'a RawValue,
}

impl fmt::Debug for RequestEnvelope<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestEnvelope")
            .field("v", &self.v)
            .field("id", &self.id)
            .field("op", &self.op)
            .field("body", &REDACTED)
            .finish()
    }
}

/// A response from the host or the desktop. Borrows its body from the buffer
/// it was parsed from, like [`RequestEnvelope`]; `Debug` never prints it.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope<'a> {
    /// Protocol version.
    pub v: u8,
    /// The request's ID.
    pub id: u32,
    /// Whether the request succeeded.
    pub ok: bool,
    /// The result, if `ok`.
    #[serde(default, borrow, skip_serializing_if = "Option::is_none")]
    pub body: Option<&'a RawValue>,
    /// The failure, if not `ok`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorCode>,
}

impl fmt::Debug for ResponseEnvelope<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("ResponseEnvelope");
        out.field("v", &self.v)
            .field("id", &self.id)
            .field("ok", &self.ok);
        if self.body.is_some() {
            out.field("body", &REDACTED);
        }
        out.field("error", &self.error).finish()
    }
}

/// What `Debug` prints in place of a body or a secret.
struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

const REDACTED: Redacted = Redacted;

impl ResponseEnvelope<'_> {
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

/// What a desktop-to-host plaintext is. A response borrows its body from the
/// plaintext it was parsed from.
pub enum DesktopMessage<'a> {
    /// An answer to a request.
    Response(ResponseEnvelope<'a>),
    /// A push.
    Event(Event),
}

impl fmt::Debug for DesktopMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Response(response) => f.debug_tuple("Response").field(response).finish(),
            Self::Event(event) => f.debug_tuple("Event").field(event).finish(),
        }
    }
}

impl<'a> DesktopMessage<'a> {
    /// Parses and validates a plaintext from the desktop.
    ///
    /// # Errors
    /// [`Error::ProtocolVersion`] for a version other than 1,
    /// [`Error::Malformed`] for anything else that is not a response or
    /// event of this protocol.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
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
            let response: ResponseEnvelope<'a> =
                serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
            response.validate()?;
            Ok(Self::Response(response))
        }
    }
}

/// Counts the bytes a serialization writes, and keeps none of them.
struct CountingWriter(usize);

impl io::Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_add(buf.len())
            .ok_or_else(|| io::Error::from(io::ErrorKind::OutOfMemory))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Appends to a vector that never grows: a write that does not fit fails, so
/// no reallocation can leave an unwiped copy of the bytes behind.
struct FixedWriter<'a>(&'a mut Vec<u8>);

impl io::Write for FixedWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.0.capacity() - self.0.len() < buf.len() {
            return Err(io::Error::from(io::ErrorKind::WriteZero));
        }
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serializes `value` into zeroizing memory, in a buffer of exactly the
/// serialized length. A first pass only counts the bytes; the second fills a
/// buffer reserved once, so the secret is never copied by a reallocation of
/// a growing buffer (THREAT_MODEL R40).
///
/// # Errors
/// [`Error::Malformed`] if serialization fails (it cannot for these types),
/// [`Error::Io`] if the buffer cannot be reserved, [`Error::Internal`] if
/// the second pass does not write what the first one counted.
pub fn to_zeroizing<T: Serialize + ?Sized>(value: &T) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut counter = CountingWriter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| Error::Malformed)?;
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(counter.0)
        .map_err(|_| Error::Io(io::ErrorKind::OutOfMemory))?;
    serde_json::to_writer(FixedWriter(&mut out), value).map_err(|_| Error::Internal)?;
    if out.len() == counter.0 {
        Ok(out)
    } else {
        Err(Error::Internal)
    }
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
        assert_eq!(
            serde_json::to_string(&ErrorCode::PasskeyNotFound).unwrap(),
            "\"passkeyNotFound\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::RpIdMismatch).unwrap(),
            "\"rpIdMismatch\""
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
            "listPasskeys",
            "getPasskey",
            "passkeyExport",
            "passkeyDelete",
            "rotationList",
            "rotationReveal",
            "rotationClose",
            "listRotations",
            "revealPending",
            "fillchange",
            "passkeyCreate ",
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
        let req: RequestEnvelope<'_> = serde_json::from_slice(ok).unwrap();
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
            assert!(serde_json::from_slice::<RequestEnvelope<'_>>(bad).is_err());
        }
    }

    #[test]
    fn responses_are_validated() {
        fn parse(s: &str) -> Result<DesktopMessage<'_>, Error> {
            DesktopMessage::parse(s.as_bytes())
        }
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

    #[test]
    fn envelopes_borrow_their_bodies_from_the_frame() {
        // Compile-level check: the body's lifetime is the buffer's. If the
        // envelopes owned their bodies (`Box<RawValue>`), these signatures
        // would not tie the result to `frame`.
        fn request_body<'a>(frame: &'a [u8]) -> &'a str {
            let request: RequestEnvelope<'a> = serde_json::from_slice(frame).unwrap();
            request.body.get()
        }
        fn response_body(frame: &[u8]) -> Option<&str> {
            match DesktopMessage::parse(frame) {
                Ok(DesktopMessage::Response(r)) => r.body.map(RawValue::get),
                _ => None,
            }
        }
        let frame = Zeroizing::new(br#"{"v":1,"id":1,"op":"fill","body":{"k":"v"}}"#.to_vec());
        let body = request_body(&frame);
        // The body is a slice of the frame buffer itself, not a copy.
        let range = frame.as_ptr_range();
        assert!(range.contains(&body.as_ptr()));
        let reply = Zeroizing::new(br#"{"v":1,"id":1,"ok":true,"body":[1]}"#.to_vec());
        let body = response_body(&reply).unwrap();
        assert!(reply.as_ptr_range().contains(&body.as_ptr()));
    }

    #[test]
    fn debug_never_prints_a_body() {
        let request = br#"{"v":1,"id":7,"op":"fill","body":{"password":"hunter2"}}"#;
        let request: RequestEnvelope<'_> = serde_json::from_slice(request).unwrap();
        let text = format!("{request:?}");
        assert!(
            text.contains("id: 7") && text.contains("\"fill\""),
            "{text}"
        );
        assert!(text.contains("body: [REDACTED]"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");

        let reply = br#"{"v":1,"id":7,"ok":true,"body":{"password":"hunter2"}}"#;
        let message = DesktopMessage::parse(reply).unwrap();
        let text = format!("{message:?}");
        assert!(text.contains("body: [REDACTED]"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");

        let failure = DesktopMessage::parse(br#"{"v":1,"id":7,"ok":false,"error":"locked"}"#);
        let text = format!("{:?}", failure.unwrap());
        assert!(text.contains("Locked") && !text.contains("body"), "{text}");
    }

    /// Serializes a longer string at every call, like a buggy `Serialize`.
    struct Unstable(core::cell::Cell<usize>);

    impl Serialize for Unstable {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.set(self.0.get() + 1);
            serializer.serialize_str(&"y".repeat(self.0.get() * 10))
        }
    }

    #[test]
    fn serialization_reserves_the_exact_size_once() {
        let body = "x".repeat(5000);
        let json = ok_json(1, &body).unwrap();
        assert!(json.len() > 4096);
        assert_eq!(json.capacity(), json.len(), "no reallocation, no slack");
        let small = error_json(3, ErrorCode::Locked).unwrap();
        assert_eq!(small.capacity(), small.len());
        // Escapes make the output longer than the input; still exact.
        let escaped = ok_json(2, &String::from('"').repeat(3000)).unwrap();
        assert_eq!(escaped.capacity(), escaped.len());
        // A serialization that differs between passes is refused without
        // growing the buffer.
        assert_eq!(
            to_zeroizing(&Unstable(core::cell::Cell::new(0))).unwrap_err(),
            Error::Internal
        );
    }
}
