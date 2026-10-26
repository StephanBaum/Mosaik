//! Errors of the local store.
//!
//! The store never holds a key or a plaintext, so no variant can carry one.
//! Variants carry only public values: key generations and server sequence
//! numbers, which the server sees as well (CRYPTO_SPEC §12). Every SQL
//! statement binds its values as parameters, so SQLite error messages name
//! tables and constraints but never contain stored data.

use mosaic_core::Error as CoreError;

/// Errors returned by [`super::Store`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// SQLite reported an error (I/O, locking, constraint, a non-database
    /// file).
    #[error("database error")]
    Database(#[source] rusqlite::Error),

    /// A file-system operation on the attachment directory failed.
    #[error("attachment file I/O error")]
    Io(#[source] std::io::Error),

    /// The database path has no file name, so the attachment directory next
    /// to it cannot be named.
    #[error("invalid database path")]
    InvalidPath,

    /// The file is an SQLite database, but not a Mosaic store.
    #[error("not a Mosaic store")]
    NotAMosaicStore,

    /// The store was written by a newer version of Mosaic. It is never
    /// downgraded or modified.
    #[error("store schema version {found} is newer than the supported version {supported}")]
    SchemaTooNew {
        /// The schema version found in the file.
        found: u32,
        /// The newest schema version this build knows.
        supported: u32,
    },

    /// Data read back from the database failed validation: a stored object
    /// does not decode, or a column holds an impossible value. The file was
    /// modified outside this crate or is damaged.
    #[error("store data is corrupt")]
    Corrupt,

    /// An object handed to the store does not decode (see
    /// [`super::RecordBody::decode`]).
    #[error("malformed object")]
    Format(#[source] CoreError),

    /// No account is stored yet.
    #[error("no account")]
    NoAccount,

    /// An account is already stored. A store holds exactly one account.
    #[error("account already exists")]
    AccountExists,

    /// The parameters belong to a different account than the stored one.
    #[error("account ID mismatch")]
    AccountMismatch,

    /// The Argon2id profile would get weaker. Profiles only ever increase
    /// (CRYPTO_SPEC §5.3).
    #[error("Argon2id profile downgrade")]
    ProfileDowngrade,

    /// The password wrapping of the account key is managed only through
    /// [`super::Store::init_account`] and [`super::Store::update_password`],
    /// so it always changes together with the salt and profile, and it is
    /// never removed.
    #[error("the password unlock method is managed with the account")]
    PasswordMethodReserved,

    /// Public unlock-method parameters exceed
    /// [`super::MAX_UNLOCK_PARAMS_LEN`].
    #[error("unlock-method parameters too large")]
    UnlockParamsTooLarge,

    /// The vault is not in the store.
    #[error("unknown vault")]
    UnknownVault,

    /// The vault is already in the store.
    #[error("vault already exists")]
    VaultExists,

    /// The object carries a lower key generation than the highest one
    /// accepted for its vault (CRYPTO_SPEC §7.1). Accepting it would let a
    /// server roll back a key rotation.
    #[error("key generation {offered} is below the accepted generation {highest_accepted}")]
    KeyGenerationRollback {
        /// The highest generation accepted for the vault.
        highest_accepted: u32,
        /// The generation of the rejected object.
        offered: u32,
    },

    /// The object carries a key generation above the highest accepted one.
    /// The caller cannot have authenticated it; accept the vault key of that
    /// generation first ([`super::Store::accept_vault_key`]).
    #[error("key generation {offered} is above the accepted generation {highest_accepted}")]
    KeyGenerationNotAccepted {
        /// The highest generation accepted for the vault.
        highest_accepted: u32,
        /// The generation of the rejected object.
        offered: u32,
    },

    /// Pruning would remove the vault key of the current generation, or a
    /// key that a stored record or the vault metadata still needs.
    #[error("vault key still in use")]
    VaultKeyInUse,

    /// The caller's accepted-state blob exceeds
    /// [`super::MAX_ACCEPTED_STATE_LEN`].
    #[error("accepted state too large")]
    AcceptedStateTooLarge,

    /// The record is not in the store.
    #[error("unknown record")]
    UnknownRecord,

    /// A remote record would replace a local change that is still waiting
    /// for upload. Resolve the conflict and store the result with
    /// [`super::Origin::Merged`], so the local change is never lost.
    #[error("record has a pending local change")]
    PendingLocalChange,

    /// The server sequence number is lower than the one already stored for
    /// the record or the vault's sync cursor.
    #[error("server sequence {offered} is below the stored {stored}")]
    StaleServerSeq {
        /// The stored sequence number.
        stored: u64,
        /// The rejected sequence number.
        offered: u64,
    },

    /// A sequence number does not fit SQLite's signed 64-bit integers.
    #[error("sequence number out of range")]
    SeqOutOfRange,

    /// The store was opened in memory, so it has no attachment directory.
    #[error("attachments are unavailable for an in-memory store")]
    AttachmentsUnavailable,

    /// The bytes are not an attachment ciphertext in the CRYPTO_SPEC §7.3
    /// layout (header, nonce prefix, canonical chunking).
    #[error("malformed attachment ciphertext")]
    AttachmentMalformed,

    /// The attachment ciphertext exceeds the per-attachment limit
    /// (`mosaic_core::MAX_ATTACHMENT_LEN` of plaintext).
    #[error("attachment too large")]
    AttachmentTooLarge,

    /// An attachment with this ID is already stored. Attachment ciphertexts
    /// are immutable; delete the old one first.
    #[error("attachment already exists")]
    AttachmentExists,

    /// The attachment is not in the store.
    #[error("unknown attachment")]
    UnknownAttachment,
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Result alias with [`Error`].
pub type Result<T> = core::result::Result<T, Error>;
