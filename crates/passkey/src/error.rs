//! The crate's error type. No variant carries key material or input data.

/// Why an authenticator operation failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Error {
    /// The OS random source failed, or no valid key scalar came out of it.
    #[error("random source failed")]
    Rng,
    /// No offered algorithm is supported (only ES256, -7).
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,
    /// The private key is not a P-256 key in PKCS#8, or could not be encoded.
    #[error("invalid private key")]
    InvalidKey,
    /// An input is out of range (length limits, mismatched relying party).
    #[error("invalid input")]
    InvalidInput,
}
