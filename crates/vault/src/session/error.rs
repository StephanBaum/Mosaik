//! Errors of the session layer.
//!
//! No variant carries a key, a plaintext or anything derived from item
//! content. They carry only public values: IDs and key generations, which
//! the server sees as well (CRYPTO_SPEC §12).

/// Errors returned by [`super::UnlockedSession`] and
/// [`super::LockedSession`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A `mosaic-core` operation failed (decoding, sealing, the model).
    #[error("core error")]
    Core(#[source] mosaic_core::Error),

    /// The local store failed.
    #[error("local store error")]
    Store(#[source] crate::storage::Error),

    /// A one-time-password operation failed.
    #[error("one-time password error")]
    Otp(#[source] mosaic_totp::Error),

    /// The master password or the Secret Key is wrong. The two are
    /// indistinguishable by design (CRYPTO_SPEC §5.4).
    #[error("wrong master password or Secret Key")]
    UnlockFailed,

    /// The store holds no account yet.
    #[error("no account")]
    NoAccount,

    /// The vault is not in the store.
    #[error("unknown vault")]
    UnknownVault,

    /// The vault's key is not available in this session: a hardware-protected
    /// TOTP vault (CRYPTO_SPEC §7.4) or a vault whose wrapped key does not
    /// open under the account key.
    #[error("vault not available")]
    VaultUnavailable,

    /// The item is not in the vault.
    #[error("unknown item")]
    UnknownItem,

    /// The item is deleted (a tombstone) or in a state the operation does
    /// not accept.
    #[error("item is deleted")]
    ItemDeleted,

    /// Source and target vault of a move are the same.
    #[error("item is already in that vault")]
    SameVault,

    /// The item has no attachment with this ID.
    #[error("unknown attachment")]
    UnknownAttachment,

    /// The file is larger than the per-attachment limit (CRYPTO_SPEC §7.3).
    #[error("attachment too large")]
    AttachmentTooLarge,

    /// The account's attachments would exceed the per-account limit
    /// (CRYPTO_SPEC §7.3).
    #[error("attachment quota exceeded")]
    AttachmentQuota,

    /// The item has no one-time-password configuration.
    #[error("item has no one-time password")]
    NoOtp,

    /// The object carries a key generation below the vault's highest
    /// accepted one (CRYPTO_SPEC §7.1): a rollback, rejected.
    #[error("key generation {offered} is below the accepted generation {highest_accepted}")]
    KeyGenerationRollback {
        /// The highest generation accepted for the vault.
        highest_accepted: u32,
        /// The generation of the rejected object.
        offered: u32,
    },

    /// A different vault key was received for a generation this device
    /// already holds: two devices rotated the vault concurrently
    /// (CRYPTO_SPEC §7.1, ADR-0013). Nothing changed; the key this device
    /// holds stays, so nothing it sealed becomes unreadable. Records sealed
    /// under the other key fail to authenticate here until one side rotates
    /// again (the M6 server accepts one wrapping per generation).
    #[error("a different vault key of generation {generation} is already held")]
    VaultKeyConflict {
        /// The generation received twice.
        generation: u32,
    },

    /// The object carries a key generation above the highest accepted one;
    /// apply that generation's wrapped vault key first
    /// ([`super::UnlockedSession::apply_remote_vault_key`]).
    #[error("key generation {offered} is above the accepted generation {highest_accepted}")]
    KeyGenerationNotAccepted {
        /// The highest generation accepted for the vault.
        highest_accepted: u32,
        /// The generation of the rejected object.
        offered: u32,
    },
}

impl From<mosaic_core::Error> for Error {
    fn from(e: mosaic_core::Error) -> Self {
        Self::Core(e)
    }
}

impl From<crate::storage::Error> for Error {
    fn from(e: crate::storage::Error) -> Self {
        Self::Store(e)
    }
}

impl From<mosaic_totp::Error> for Error {
    fn from(e: mosaic_totp::Error) -> Self {
        Self::Otp(e)
    }
}

/// Result type of the session layer.
pub type Result<T> = core::result::Result<T, Error>;
