//! Errors. No variant carries a secret, an item field or an ID.

/// Why a report could not be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The operating system's random number generator failed (the per-call
    /// reuse key could not be drawn).
    #[error("the random number generator failed")]
    Random,
    /// A secret could not be copied into zeroizing memory.
    #[error("a secret could not be copied into protected memory")]
    Allocation,
    /// A cryptographic primitive refused its input (HMAC key setup; not
    /// reachable with the fixed key length used here).
    #[error("a cryptographic primitive failed")]
    Crypto,
}

/// `Result` with this crate's [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
