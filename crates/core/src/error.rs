//! The crate-wide error type.
//!
//! Variants never carry secret-derived data (CRYPTO_SPEC §4). In particular,
//! every authentication or padding failure of every object type collapses into
//! the single variant [`Error::Decryption`], so callers (and attackers) cannot
//! tell a bad tag from bad padding or a wrong key from a wrong context ID.

/// Errors returned by `mosaic-core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Authenticated decryption failed: wrong key, wrong context, tampered
    /// data, or invalid padding. Deliberately indistinguishable.
    #[error("decryption failed")]
    Decryption,

    /// The input is not a well-formed encoding of the expected object
    /// (wrong length, truncated, trailing bytes, non-canonical form).
    #[error("malformed encoding")]
    Malformed,

    /// The object carries a format version this build does not support.
    #[error("unsupported format version")]
    UnsupportedVersion,

    /// The object carries an algorithm ID (or algorithm parameter) this build
    /// does not support.
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,

    /// The Argon2id profile ID is not one of the profiles in CRYPTO_SPEC §5.3.
    #[error("unknown Argon2id profile")]
    UnknownProfile,

    /// A password change or Secret Key rotation asked for a weaker Argon2id
    /// profile than the account's current one. Profiles only ever increase
    /// (CRYPTO_SPEC §5.3).
    #[error("Argon2id profile downgrade")]
    ProfileDowngrade,

    /// The unlock method byte is not one of the methods in CRYPTO_SPEC §6.
    #[error("unknown unlock method")]
    UnknownMethod,

    /// An identifier is the reserved nil UUID or otherwise invalid.
    #[error("invalid identifier")]
    InvalidId,

    /// A key generation is invalid (zero) where generations start at 1.
    #[error("invalid key generation")]
    InvalidKeyGeneration,

    /// The object was encrypted under a different vault key generation than
    /// the key supplied for opening it.
    #[error("key generation mismatch")]
    KeyGenerationMismatch,

    /// The Secret Key is well-formed but its checksum does not match: it was
    /// almost certainly mistyped.
    #[error("Secret Key mistyped")]
    SecretKeyMistyped,

    /// The Secret Key string is not in the `M1-` format at all.
    #[error("Secret Key malformed")]
    SecretKeyMalformed,

    /// The master password is empty.
    #[error("empty password")]
    EmptyPassword,

    /// The operating system's CSPRNG failed. There is no fallback.
    #[error("randomness unavailable")]
    Randomness,

    /// Memory for a key-stretching or buffer allocation could not be reserved.
    #[error("memory allocation failed")]
    Allocation,

    /// An input exceeds a size limit (object plaintext, attachment, field).
    #[error("size limit exceeded")]
    SizeLimit,
}

/// Result alias with [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
