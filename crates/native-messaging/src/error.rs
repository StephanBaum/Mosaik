//! Errors of the host, the framing, the handshake and the installer.

use std::io;

use crate::protocol::ErrorCode;

/// Why an operation of this crate failed. Carries no key material, no paths
/// and no message content.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The endpoint does not fit this platform, is not absolute, is too
    /// long, or its discovery file is malformed.
    #[error("the browser endpoint is not valid")]
    EndpointInvalid,
    /// The socket's directory is not a private directory of the current
    /// user, or the socket belongs to someone else.
    #[error("the browser endpoint is not private")]
    EndpointInsecure,
    /// Another program serves the endpoint.
    #[error("another program uses the browser endpoint")]
    EndpointInUse,
    /// Nothing serves the endpoint: the desktop app is not running.
    #[error("the desktop app is not running")]
    DesktopNotRunning,
    /// An OS error.
    #[error("I/O error: {0}")]
    Io(io::ErrorKind),
    /// The peer closed the connection.
    #[error("connection closed")]
    Closed,
    /// A frame with length 0.
    #[error("empty frame")]
    FrameEmpty,
    /// A frame longer than `MAX_FRAME_BYTES`.
    #[error("frame too large")]
    FrameTooLarge,
    /// The stream ended inside a frame.
    #[error("truncated frame")]
    FrameTruncated,
    /// A message that is not valid JSON of the expected shape.
    #[error("malformed message")]
    Malformed,
    /// A message with a protocol version other than 1.
    #[error("unsupported protocol version")]
    ProtocolVersion,
    /// The peer refused with this code.
    #[error("the peer refused: {0:?}")]
    Refused(ErrorCode),
    /// The desktop did not prove knowledge of the pairing key.
    #[error("the desktop is not authentic")]
    NotAuthentic,
    /// A channel crypto failure ([`mosaic_core::Error`]).
    #[error("channel failure: {0}")]
    Channel(mosaic_core::Error),
    /// A step took too long.
    #[error("timed out")]
    Timeout,
    /// The caller is not an allowed extension.
    #[error("the caller is not an allowed extension")]
    CallerNotAllowed,
    /// Writing or removing a host manifest or registry entry failed.
    #[error("installing the native messaging manifest failed")]
    Install,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e.kind())
    }
}

impl From<mosaic_core::Error> for Error {
    fn from(e: mosaic_core::Error) -> Self {
        Self::Channel(e)
    }
}

impl Error {
    /// The code the host reports to the extension for this failure.
    #[must_use]
    pub const fn wire_code(self) -> ErrorCode {
        match self {
            Self::DesktopNotRunning
            | Self::Closed
            | Self::EndpointInvalid
            | Self::EndpointInsecure => ErrorCode::DesktopNotRunning,
            Self::NotAuthentic | Self::Channel(_) => ErrorCode::DesktopNotAuthentic,
            Self::Timeout => ErrorCode::Timeout,
            Self::FrameTooLarge => ErrorCode::FrameTooLarge,
            Self::ProtocolVersion => ErrorCode::ProtocolVersion,
            Self::Refused(code) => code,
            _ => ErrorCode::Internal,
        }
    }
}
