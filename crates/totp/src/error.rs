//! The crate-wide error type.
//!
//! Variants never carry input data: a rejected `otpauth://` URI or secret may
//! contain the OTP secret, so errors name only the kind of problem.

/// Errors returned by `mosaic-totp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The secret is empty, is not valid base32, or has padding in the wrong
    /// place.
    #[error("invalid OTP secret")]
    InvalidSecret,

    /// The number of digits is outside 6 to 8 (5 for Steam Guard).
    #[error("invalid number of digits")]
    InvalidDigits,

    /// The period is outside 1 to 300 seconds.
    #[error("invalid period")]
    InvalidPeriod,

    /// The hash algorithm is not SHA-1, SHA-256 or SHA-512.
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,

    /// The `otpauth://` type is not `totp`, `hotp` or `steam`.
    #[error("unsupported OTP type")]
    UnsupportedType,

    /// The `encoder` parameter is not `steam`, or `steam` was combined with
    /// HOTP.
    #[error("unsupported encoder")]
    UnsupportedEncoder,

    /// The URI is not of the form `otpauth://TYPE/LABEL?PARAMETERS`, has a
    /// fragment, a malformed parameter, invalid percent-encoding or invalid
    /// UTF-8.
    #[error("malformed otpauth URI")]
    MalformedUri,

    /// The URI has no `secret` parameter.
    #[error("missing secret")]
    MissingSecret,

    /// An HOTP URI has no `counter` parameter.
    #[error("missing counter")]
    MissingCounter,

    /// A parameter appears more than once.
    #[error("duplicate parameter")]
    DuplicateParameter,

    /// A numeric parameter (`digits`, `period`, `counter`) is not a decimal
    /// number in range.
    #[error("invalid parameter value")]
    InvalidParameter,

    /// An issuer or account name is too long, contains control or
    /// bidirectional formatting characters, has surrounding whitespace, or
    /// cannot be represented unambiguously in a label.
    #[error("invalid label")]
    InvalidLabel,

    /// An input exceeds a length limit (URI, secret, parameter count).
    #[error("size limit exceeded")]
    SizeLimit,

    /// The system clock is set before 1970-01-01.
    #[error("system time before the Unix epoch")]
    TimeBeforeEpoch,
}

/// Result alias with [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
