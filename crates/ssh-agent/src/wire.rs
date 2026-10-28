//! The SSH agent wire format (draft-miller-ssh-agent, sections 3 to 5).
//!
//! Every message is a `uint32` length followed by that many bytes; the first
//! byte is the message number. Strings are a `uint32` length followed by the
//! bytes (RFC 4251 §5). This module only encodes and decodes; what the agent
//! answers is decided in [`crate::AgentCore`].
//!
//! Parsing is strict: a request with missing or extra bytes is an error, and
//! the agent answers it with `SSH_AGENT_FAILURE`. A length prefix of 0 or
//! above [`MAX_MESSAGE_LEN`] ends the connection.

use thiserror::Error;

/// The largest message either side may send, as in OpenSSH (256 KiB).
pub const MAX_MESSAGE_LEN: usize = 256 * 1024;

/// Message numbers (draft-miller-ssh-agent §5.1).
pub mod msg {
    /// `SSH_AGENT_FAILURE`.
    pub const FAILURE: u8 = 5;
    /// `SSH_AGENT_SUCCESS`.
    pub const SUCCESS: u8 = 6;
    /// `SSH_AGENTC_REQUEST_IDENTITIES`.
    pub const REQUEST_IDENTITIES: u8 = 11;
    /// `SSH_AGENT_IDENTITIES_ANSWER`.
    pub const IDENTITIES_ANSWER: u8 = 12;
    /// `SSH_AGENTC_SIGN_REQUEST`.
    pub const SIGN_REQUEST: u8 = 13;
    /// `SSH_AGENT_SIGN_RESPONSE`.
    pub const SIGN_RESPONSE: u8 = 14;
    /// `SSH_AGENTC_ADD_IDENTITY`.
    pub const ADD_IDENTITY: u8 = 17;
    /// `SSH_AGENTC_REMOVE_IDENTITY`.
    pub const REMOVE_IDENTITY: u8 = 18;
    /// `SSH_AGENTC_REMOVE_ALL_IDENTITIES`.
    pub const REMOVE_ALL_IDENTITIES: u8 = 19;
    /// `SSH_AGENTC_ADD_SMARTCARD_KEY`.
    pub const ADD_SMARTCARD_KEY: u8 = 20;
    /// `SSH_AGENTC_REMOVE_SMARTCARD_KEY`.
    pub const REMOVE_SMARTCARD_KEY: u8 = 21;
    /// `SSH_AGENTC_LOCK`.
    pub const LOCK: u8 = 22;
    /// `SSH_AGENTC_UNLOCK`.
    pub const UNLOCK: u8 = 23;
    /// `SSH_AGENTC_ADD_ID_CONSTRAINED`.
    pub const ADD_ID_CONSTRAINED: u8 = 25;
    /// `SSH_AGENTC_ADD_SMARTCARD_KEY_CONSTRAINED`.
    pub const ADD_SMARTCARD_KEY_CONSTRAINED: u8 = 26;
    /// `SSH_AGENTC_EXTENSION`.
    pub const EXTENSION: u8 = 27;
    /// `SSH_AGENT_EXTENSION_FAILURE`.
    pub const EXTENSION_FAILURE: u8 = 28;
}

/// Sign request flag: an RSA key signs with `rsa-sha2-256` (RFC 8332).
pub const SSH_AGENT_RSA_SHA2_256: u32 = 0x02;
/// Sign request flag: an RSA key signs with `rsa-sha2-512` (RFC 8332).
pub const SSH_AGENT_RSA_SHA2_512: u32 = 0x04;

/// A malformed message. Carries no message bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum WireError {
    /// A message or field ends early.
    #[error("message truncated")]
    Truncated,
    /// Bytes left over after the last field.
    #[error("trailing data after the message")]
    TrailingData,
    /// A zero-length message (it has no message number).
    #[error("empty message")]
    Empty,
    /// Longer than [`MAX_MESSAGE_LEN`].
    #[error("message too long")]
    TooLong,
    /// A message number this side never receives.
    #[error("unexpected message number")]
    UnexpectedMessage,
}

/// A request that changes the agent's keys or lock state. The agent refuses
/// all of them: its keys come from the vault only (brief §5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefusedRequest {
    /// `SSH_AGENTC_ADD_IDENTITY` or `SSH_AGENTC_ADD_ID_CONSTRAINED`.
    AddIdentity,
    /// `SSH_AGENTC_REMOVE_IDENTITY`.
    RemoveIdentity,
    /// `SSH_AGENTC_REMOVE_ALL_IDENTITIES`.
    RemoveAllIdentities,
    /// `SSH_AGENTC_ADD_SMARTCARD_KEY` or its constrained form.
    AddSmartcardKey,
    /// `SSH_AGENTC_REMOVE_SMARTCARD_KEY`.
    RemoveSmartcardKey,
    /// `SSH_AGENTC_LOCK`. Its passphrase is never parsed.
    Lock,
    /// `SSH_AGENTC_UNLOCK`. Its passphrase is never parsed.
    Unlock,
}

impl RefusedRequest {
    const fn from_number(n: u8) -> Option<Self> {
        Some(match n {
            msg::ADD_IDENTITY | msg::ADD_ID_CONSTRAINED => Self::AddIdentity,
            msg::REMOVE_IDENTITY => Self::RemoveIdentity,
            msg::REMOVE_ALL_IDENTITIES => Self::RemoveAllIdentities,
            msg::ADD_SMARTCARD_KEY | msg::ADD_SMARTCARD_KEY_CONSTRAINED => Self::AddSmartcardKey,
            msg::REMOVE_SMARTCARD_KEY => Self::RemoveSmartcardKey,
            msg::LOCK => Self::Lock,
            msg::UNLOCK => Self::Unlock,
            _ => return None,
        })
    }

    const fn number(self) -> u8 {
        match self {
            Self::AddIdentity => msg::ADD_IDENTITY,
            Self::RemoveIdentity => msg::REMOVE_IDENTITY,
            Self::RemoveAllIdentities => msg::REMOVE_ALL_IDENTITIES,
            Self::AddSmartcardKey => msg::ADD_SMARTCARD_KEY,
            Self::RemoveSmartcardKey => msg::REMOVE_SMARTCARD_KEY,
            Self::Lock => msg::LOCK,
            Self::Unlock => msg::UNLOCK,
        }
    }
}

/// `SSH_AGENTC_SIGN_REQUEST` (draft-miller-ssh-agent §4.5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SignRequest<'a> {
    /// The public key blob, as listed by the identities answer.
    pub key_blob: &'a [u8],
    /// The data to sign (for SSH user authentication, the session
    /// identifier and the userauth request, RFC 4252 §7).
    pub data: &'a [u8],
    /// `SSH_AGENT_RSA_SHA2_256`, `SSH_AGENT_RSA_SHA2_512`; other bits are
    /// ignored, as in OpenSSH.
    pub flags: u32,
}

/// A client request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Request<'a> {
    /// `SSH_AGENTC_REQUEST_IDENTITIES`.
    RequestIdentities,
    /// `SSH_AGENTC_SIGN_REQUEST`.
    Sign(SignRequest<'a>),
    /// A key or lock management request; its body is not parsed.
    Refused(RefusedRequest),
    /// `SSH_AGENTC_EXTENSION` with its extension name; the contents after
    /// the name are not parsed.
    Extension {
        /// The extension name, for example `session-bind@openssh.com`.
        name: &'a [u8],
    },
    /// Any other message number.
    Unknown(u8),
}

impl<'a> Request<'a> {
    /// Parses one message body (without its length prefix).
    ///
    /// # Errors
    /// [`WireError::Empty`] for an empty body, [`WireError::TooLong`] above
    /// [`MAX_MESSAGE_LEN`], [`WireError::Truncated`] or
    /// [`WireError::TrailingData`] for a request identities or sign request
    /// whose fields do not fill the body exactly, and
    /// [`WireError::Truncated`] for an extension without a name.
    pub fn parse(payload: &'a [u8]) -> Result<Self, WireError> {
        if payload.len() > MAX_MESSAGE_LEN {
            return Err(WireError::TooLong);
        }
        let mut r = Reader::new(payload);
        let number = r.u8().map_err(|_| WireError::Empty)?;
        match number {
            msg::REQUEST_IDENTITIES => {
                r.finish()?;
                Ok(Self::RequestIdentities)
            }
            msg::SIGN_REQUEST => {
                let key_blob = r.string()?;
                let data = r.string()?;
                let flags = r.u32()?;
                r.finish()?;
                Ok(Self::Sign(SignRequest {
                    key_blob,
                    data,
                    flags,
                }))
            }
            msg::EXTENSION => Ok(Self::Extension { name: r.string()? }),
            n => Ok(RefusedRequest::from_number(n).map_or(Self::Unknown(n), Self::Refused)),
        }
    }

    /// Encodes the message body, as a client sends it. Refused requests are
    /// encoded with an empty body (the agent never reads it), extensions
    /// with the name only.
    ///
    /// # Errors
    /// [`WireError::TooLong`] if the body would exceed [`MAX_MESSAGE_LEN`].
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let mut w = Writer::default();
        match *self {
            Self::RequestIdentities => w.u8(msg::REQUEST_IDENTITIES),
            Self::Sign(s) => {
                w.u8(msg::SIGN_REQUEST);
                w.string(s.key_blob)?;
                w.string(s.data)?;
                w.u32(s.flags);
            }
            Self::Refused(r) => w.u8(r.number()),
            Self::Extension { name } => {
                w.u8(msg::EXTENSION);
                w.string(name)?;
            }
            Self::Unknown(n) => w.u8(n),
        }
        w.finish()
    }
}

/// One entry of the identities answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Identity<'a> {
    /// The public key blob (RFC 4253 §6.6).
    pub key_blob: &'a [u8],
    /// The key comment, UTF-8 by convention.
    pub comment: &'a [u8],
}

/// An agent response.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Response<'a> {
    /// `SSH_AGENT_FAILURE`.
    Failure,
    /// `SSH_AGENT_SUCCESS`.
    Success,
    /// `SSH_AGENT_EXTENSION_FAILURE`.
    ExtensionFailure,
    /// `SSH_AGENT_IDENTITIES_ANSWER`.
    Identities(Vec<Identity<'a>>),
    /// `SSH_AGENT_SIGN_RESPONSE` with the signature blob (string algorithm
    /// name, string signature).
    Signature(&'a [u8]),
}

impl<'a> Response<'a> {
    /// Encodes the message body (without the length prefix).
    ///
    /// # Errors
    /// [`WireError::TooLong`] if the body would exceed [`MAX_MESSAGE_LEN`].
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let mut w = Writer::default();
        match self {
            Self::Failure => w.u8(msg::FAILURE),
            Self::Success => w.u8(msg::SUCCESS),
            Self::ExtensionFailure => w.u8(msg::EXTENSION_FAILURE),
            Self::Identities(ids) => {
                w.u8(msg::IDENTITIES_ANSWER);
                w.u32(u32::try_from(ids.len()).map_err(|_| WireError::TooLong)?);
                for id in ids {
                    w.string(id.key_blob)?;
                    w.string(id.comment)?;
                }
            }
            Self::Signature(sig) => {
                w.u8(msg::SIGN_RESPONSE);
                w.string(sig)?;
            }
        }
        w.finish()
    }

    /// Parses a response body, as a client reads it.
    ///
    /// # Errors
    /// As [`Request::parse`]; [`WireError::UnexpectedMessage`] for a
    /// message number that is not a response.
    pub fn parse(payload: &'a [u8]) -> Result<Self, WireError> {
        if payload.len() > MAX_MESSAGE_LEN {
            return Err(WireError::TooLong);
        }
        let mut r = Reader::new(payload);
        let number = r.u8().map_err(|_| WireError::Empty)?;
        let response = match number {
            msg::FAILURE => Self::Failure,
            msg::SUCCESS => Self::Success,
            msg::EXTENSION_FAILURE => Self::ExtensionFailure,
            msg::IDENTITIES_ANSWER => {
                let n = r.u32()?;
                // Each entry needs at least two length prefixes; never
                // reserve more than the body can hold.
                let cap = usize::try_from(n)
                    .unwrap_or(usize::MAX)
                    .min(r.remaining() / 8);
                let mut ids = Vec::with_capacity(cap);
                for _ in 0..n {
                    let key_blob = r.string()?;
                    let comment = r.string()?;
                    ids.push(Identity { key_blob, comment });
                }
                Self::Identities(ids)
            }
            msg::SIGN_RESPONSE => Self::Signature(r.string()?),
            _ => return Err(WireError::UnexpectedMessage),
        };
        r.finish()?;
        Ok(response)
    }
}

/// Prefixes a message body with its `uint32` length.
///
/// # Errors
/// [`WireError::Empty`] for an empty body, [`WireError::TooLong`] above
/// [`MAX_MESSAGE_LEN`].
pub fn frame(payload: &[u8]) -> Result<Vec<u8>, WireError> {
    let len = check_len(payload.len())?;
    let mut out = Vec::with_capacity(payload.len().saturating_add(4));
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// The body length announced by a length prefix.
///
/// # Errors
/// [`WireError::Empty`] for 0, [`WireError::TooLong`] above
/// [`MAX_MESSAGE_LEN`].
pub fn body_len(prefix: [u8; 4]) -> Result<usize, WireError> {
    let len = usize::try_from(u32::from_be_bytes(prefix)).map_err(|_| WireError::TooLong)?;
    check_len(len)?;
    Ok(len)
}

fn check_len(len: usize) -> Result<u32, WireError> {
    if len == 0 {
        Err(WireError::Empty)
    } else if len > MAX_MESSAGE_LEN {
        Err(WireError::TooLong)
    } else {
        u32::try_from(len).map_err(|_| WireError::TooLong)
    }
}

/// A cursor over a message body.
struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    const fn remaining(&self) -> usize {
        self.buf.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        let (head, tail) = self.buf.split_at_checked(n).ok_or(WireError::Truncated)?;
        self.buf = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, WireError> {
        let (&b, tail) = self.buf.split_first().ok_or(WireError::Truncated)?;
        self.buf = tail;
        Ok(b)
    }

    fn u32(&mut self) -> Result<u32, WireError> {
        let (head, tail) = self
            .buf
            .split_first_chunk::<4>()
            .ok_or(WireError::Truncated)?;
        self.buf = tail;
        Ok(u32::from_be_bytes(*head))
    }

    fn string(&mut self) -> Result<&'a [u8], WireError> {
        let len = usize::try_from(self.u32()?).map_err(|_| WireError::Truncated)?;
        self.take(len)
    }

    const fn finish(&self) -> Result<(), WireError> {
        if self.buf.is_empty() {
            Ok(())
        } else {
            Err(WireError::TrailingData)
        }
    }
}

/// Builds a message body.
#[derive(Default)]
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, b: u8) {
        self.buf.push(b);
    }

    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    fn string(&mut self, s: &[u8]) -> Result<(), WireError> {
        if s.len() > MAX_MESSAGE_LEN {
            return Err(WireError::TooLong);
        }
        self.u32(u32::try_from(s.len()).map_err(|_| WireError::TooLong)?);
        self.buf.extend_from_slice(s);
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, WireError> {
        if self.buf.len() > MAX_MESSAGE_LEN {
            Err(WireError::TooLong)
        } else {
            Ok(self.buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// An SSH `string` (RFC 4251 §5).
    fn s(b: &[u8]) -> Vec<u8> {
        let mut v = u32::try_from(b.len()).unwrap().to_be_bytes().to_vec();
        v.extend_from_slice(b);
        v
    }

    #[test]
    fn request_identities_vector() {
        // draft-miller-ssh-agent §4.4: just the message number.
        assert_eq!(Request::parse(&[11]).unwrap(), Request::RequestIdentities);
        assert_eq!(Request::RequestIdentities.encode().unwrap(), vec![11]);
        assert_eq!(
            frame(&[11]).unwrap(),
            vec![0, 0, 0, 1, 11],
            "the complete frame ssh-add -l sends"
        );
        assert_eq!(Request::parse(&[11, 0]), Err(WireError::TrailingData));
    }

    #[test]
    fn sign_request_vector() {
        // byte 13, string key_blob, string data, uint32 flags
        let body = [
            &[13u8][..],
            &[0, 0, 0, 3],
            b"key",
            &[0, 0, 0, 4],
            b"data",
            &[0, 0, 0, 4],
        ]
        .concat();
        let req = Request::parse(&body).unwrap();
        assert_eq!(
            req,
            Request::Sign(SignRequest {
                key_blob: b"key",
                data: b"data",
                flags: SSH_AGENT_RSA_SHA2_512,
            })
        );
        assert_eq!(req.encode().unwrap(), body);
    }

    #[test]
    fn sign_request_must_be_exact() {
        let mut body = vec![13];
        body.extend(s(b"k"));
        body.extend(s(b"d"));
        assert_eq!(Request::parse(&body), Err(WireError::Truncated), "no flags");
        body.extend([0, 0, 0]);
        assert_eq!(
            Request::parse(&body),
            Err(WireError::Truncated),
            "short flags"
        );
        body.push(0);
        assert!(Request::parse(&body).is_ok());
        body.push(0);
        assert_eq!(Request::parse(&body), Err(WireError::TrailingData));
        // A string length running past the end.
        assert_eq!(
            Request::parse(&[13, 0, 0, 0, 9, 1, 2]),
            Err(WireError::Truncated)
        );
        assert_eq!(
            Request::parse(&[13, 0xff, 0xff, 0xff, 0xff]),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn management_requests_are_classified_without_parsing() {
        for (n, want) in [
            (17, RefusedRequest::AddIdentity),
            (25, RefusedRequest::AddIdentity),
            (18, RefusedRequest::RemoveIdentity),
            (19, RefusedRequest::RemoveAllIdentities),
            (20, RefusedRequest::AddSmartcardKey),
            (26, RefusedRequest::AddSmartcardKey),
            (21, RefusedRequest::RemoveSmartcardKey),
            (22, RefusedRequest::Lock),
            (23, RefusedRequest::Unlock),
        ] {
            // Bodies are ignored, even malformed ones.
            assert_eq!(
                Request::parse(&[n, 0xde, 0xad]).unwrap(),
                Request::Refused(want)
            );
            let encoded = Request::Refused(want).encode().unwrap();
            assert_eq!(
                Request::parse(&encoded).unwrap(),
                Request::Refused(want),
                "round trip for {n}"
            );
        }
    }

    #[test]
    fn extension_and_unknown() {
        let mut body = vec![27];
        body.extend(s(b"session-bind@openssh.com"));
        body.extend([1, 2, 3]);
        assert_eq!(
            Request::parse(&body).unwrap(),
            Request::Extension {
                name: b"session-bind@openssh.com"
            }
        );
        assert_eq!(Request::parse(&[27]), Err(WireError::Truncated));
        assert_eq!(Request::parse(&[1]).unwrap(), Request::Unknown(1));
        assert_eq!(Request::parse(&[255, 1]).unwrap(), Request::Unknown(255));
        assert_eq!(Request::Unknown(9).encode().unwrap(), vec![9]);
        assert_eq!(Request::parse(&[]), Err(WireError::Empty));
    }

    #[test]
    fn responses_round_trip() {
        let ids = vec![
            Identity {
                key_blob: b"blob-1",
                comment: b"one",
            },
            Identity {
                key_blob: b"blob-2",
                comment: b"",
            },
        ];
        for r in [
            Response::Failure,
            Response::Success,
            Response::ExtensionFailure,
            Response::Identities(vec![]),
            Response::Identities(ids),
            Response::Signature(b"sig"),
        ] {
            let body = r.encode().unwrap();
            assert_eq!(Response::parse(&body).unwrap(), r);
        }
        assert_eq!(Response::Failure.encode().unwrap(), vec![5]);
        assert_eq!(Response::parse(&[11]), Err(WireError::UnexpectedMessage));
        assert_eq!(Response::parse(&[]), Err(WireError::Empty));
        assert_eq!(Response::parse(&[5, 0]), Err(WireError::TrailingData));
        // An identity count far beyond the body fails without reserving it.
        assert_eq!(
            Response::parse(&[12, 0xff, 0xff, 0xff, 0xff]),
            Err(WireError::Truncated)
        );
    }

    #[test]
    fn length_limits() {
        assert_eq!(body_len([0, 0, 0, 0]), Err(WireError::Empty));
        assert_eq!(body_len([0, 0, 0, 1]), Ok(1));
        assert_eq!(body_len([0, 4, 0, 0]), Ok(MAX_MESSAGE_LEN));
        assert_eq!(body_len([0, 4, 0, 1]), Err(WireError::TooLong));
        assert_eq!(body_len([0xff; 4]), Err(WireError::TooLong));
        assert_eq!(frame(&[]), Err(WireError::Empty));
        let max = vec![0u8; MAX_MESSAGE_LEN];
        assert_eq!(frame(&max).unwrap().len(), MAX_MESSAGE_LEN + 4);
        let over = vec![0u8; MAX_MESSAGE_LEN + 1];
        assert_eq!(frame(&over), Err(WireError::TooLong));
        assert_eq!(Request::parse(&over), Err(WireError::TooLong));
        assert_eq!(Response::parse(&over), Err(WireError::TooLong));
        assert_eq!(Response::Signature(&over).encode(), Err(WireError::TooLong));
        // A body that is exactly at the limit after its header: the string
        // fits, the message does not.
        let near = vec![0u8; MAX_MESSAGE_LEN - 4];
        assert_eq!(Response::Signature(&near).encode(), Err(WireError::TooLong));
        let fits = vec![0u8; MAX_MESSAGE_LEN - 5];
        assert_eq!(
            Response::Signature(&fits).encode().unwrap().len(),
            MAX_MESSAGE_LEN
        );
    }

    proptest! {
        #[test]
        fn sign_requests_round_trip(
            key in proptest::collection::vec(any::<u8>(), 0..300),
            data in proptest::collection::vec(any::<u8>(), 0..600),
            flags in any::<u32>(),
        ) {
            let req = Request::Sign(SignRequest { key_blob: &key, data: &data, flags });
            let body = req.encode().unwrap();
            prop_assert_eq!(Request::parse(&body).unwrap(), req);
            // Every strict prefix is truncated, never accepted.
            for cut in 1..body.len() {
                prop_assert_eq!(Request::parse(&body[..cut]), Err(WireError::Truncated));
            }
        }

        #[test]
        fn parsing_never_panics_and_reencodes(body in proptest::collection::vec(any::<u8>(), 0..64)) {
            if let Ok(req) = Request::parse(&body) {
                let again = req.encode().unwrap();
                prop_assert_eq!(Request::parse(&again).unwrap(), req);
                if matches!(req, Request::RequestIdentities | Request::Sign(_)) {
                    prop_assert_eq!(&again, &body);
                }
            }
            if let Ok(resp) = Response::parse(&body) {
                prop_assert_eq!(resp.encode().unwrap(), body);
            }
        }
    }
}
