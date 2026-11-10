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

    /// The quick-unlock enrollment was past its maximum age (CRYPTO_SPEC
    /// §6.2); it was deleted.
    #[error("quick unlock expired")]
    QuickUnlockExpired,

    /// The platform secret did not open the quick-unlock wrapping; the
    /// enrollment was deleted.
    #[error("quick unlock invalidated")]
    QuickUnlockInvalidated,

    /// No quick-unlock enrollment exists.
    #[error("no quick unlock")]
    NoQuickUnlock,

    /// The master password must be used first: no password unlock since the
    /// store was opened (quick unlock), or too many rejected attempts in a
    /// row (quick unlock and hardware keys).
    #[error("master password required")]
    PasswordRequired,

    /// Enrolling a quick-unlock or hardware-key method needs a fresh proof
    /// of the master password from this session
    /// ([`super::UnlockedSession::verify_master_password`]): the proof is
    /// older than [`super::REAUTH_GRANT`] or comes from another session.
    #[error("master password re-authentication required")]
    ReauthenticationRequired,

    /// The credential is not registered (for unlock, or for this vault).
    #[error("unknown hardware key")]
    UnknownHardwareKey,

    /// The security key's PRF output did not open the wrapping.
    #[error("hardware key rejected")]
    HardwareKeyRejected,

    /// The maximum number of hardware keys is registered.
    #[error("too many hardware keys")]
    HardwareKeyLimit,

    /// The vault's last security key cannot be removed.
    #[error("last hardware key")]
    LastHardwareKey,

    /// Credential ID or RP ID outside the accepted lengths.
    #[error("invalid hardware key")]
    InvalidHardwareKey,

    /// The operation needs a hardware-protected vault (or one that is not).
    #[error("wrong vault protection")]
    WrongVaultProtection,

    /// A prepared key rotation or master password change belongs to an
    /// unlocked session that has ended: it was prepared before a lock, so it
    /// is dropped and has to be prepared again
    /// ([`super::UnlockedSession::commit_vault_rotation`],
    /// [`super::UnlockedSession::commit_master_password_change`]).
    #[error("prepared change belongs to an earlier session")]
    PreparedStale,

    /// A backend offered a master password this device retired earlier (its
    /// salt was replaced by a later change): a rollback to an old password
    /// ([`super::UnlockedSession::adopt_remote_password`], CRYPTO_SPEC 9.4).
    #[error("the backend offered a retired master password")]
    PasswordRollback,

    /// A hardware-protected vault holds only one-time-code items
    /// (CRYPTO_SPEC §7.4).
    #[error("item type not allowed in this vault")]
    ItemTypeNotAllowed,

    /// The operation needs the account's identity key, which is not set up
    /// yet ([`super::UnlockedSession::ensure_identity_key`]).
    #[error("no identity key")]
    NoIdentity,

    /// The identity key this device holds is not the account's: the stored
    /// row cannot be read, the server's key does not open under the account
    /// key, or this device's key was already published. Sharing stays off
    /// until the user acknowledges (CRYPTO_SPEC §8.1).
    #[error("identity key mismatch")]
    IdentityMismatch,

    /// A shared vault never adopts a key (CRYPTO_SPEC §10.2).
    #[error("a shared vault never adopts a key")]
    AdoptionForbidden,

    /// The vault is shared: its key changes only through the shared
    /// rotation, never through the M6 rotation.
    #[error("the vault is shared")]
    VaultShared,

    /// The contact is not pinned, or its pin was not verified by the user
    /// (CRYPTO_SPEC §8.2).
    #[error("contact not verified")]
    ContactUnverified,

    /// The served key of the contact differs from its pin, or devices pinned
    /// different keys: nothing is sealed to or opened from the account until
    /// the user accepts the new key (CRYPTO_SPEC §8.5).
    #[error("contact key changed")]
    ContactKeyChanged,

    /// This account's role does not allow the operation.
    #[error("role does not allow this")]
    RoleForbidden,

    /// The vault is shared and this account only reads it: no item change
    /// (a viewer of a shared vault cannot start or close a rotation).
    #[error("the vault is read-only for this account")]
    ReadOnly,

    /// The item has no login or Wi-Fi password to track (a snooze or
    /// dismissal binds to the password's mark).
    #[error("item has no password")]
    NoPassword,

    /// Only the vault owner may do this (a rotation, emergency seals).
    #[error("only the vault owner may do this")]
    NotOwner,

    /// The invite's seal or the vault it carries could not be opened.
    #[error("invite cannot be read")]
    InviteUnreadable,

    /// The roster does not list this account (or the sender) as the invite
    /// says (CRYPTO_SPEC §8.2).
    #[error("invite does not match the roster")]
    InviteNotInRoster,

    /// The invite was not previewed in this unlocked session, or the preview
    /// is older than five minutes.
    #[error("invite not opened")]
    InviteNotOpened,

    /// The owner the server names is not the owner the roster names.
    #[error("owner mismatch")]
    OwnerMismatch,

    /// Nothing on this device or in the account's contact book says who
    /// owns the vault: it is held until an `owned:` or `member:` entry
    /// arrives (M6b verify round 3).
    #[error("vault owner unknown")]
    OwnerUnknown,

    /// The vault is already known on this device (open, a generation floor
    /// or a sharing row): an invite or an emergency release may not hand it
    /// a key (M6b review H1).
    #[error("vault already known")]
    VaultKnown,

    /// A roster that does not list this account.
    #[error("not in the roster")]
    NotInRoster,

    /// The vault has no roster.
    #[error("the vault has no roster")]
    NoRoster,

    /// A member's key does not match the roster fingerprint or the contact
    /// pin: no seal is made (CRYPTO_SPEC §8.3).
    #[error("member key does not match")]
    MemberKeyMismatch,

    /// The rotation plan does not cover the roster's members.
    #[error("rotation plan does not match the roster")]
    PlanMismatch,

    /// The account is a member of the vault already.
    #[error("already a member")]
    AlreadyMember,

    /// A removed member can be invited again only after the next rotation
    /// (their removal is written at the current generation, and ties go to
    /// the removal).
    #[error("re-invite has to wait for the next rotation")]
    ReinviteBlocked,

    /// The owner cannot be revoked.
    #[error("the owner cannot be removed")]
    OwnerCannotBeRevoked,

    /// A key share failed the claim rule (CRYPTO_SPEC §8.3): the vault stays
    /// at its generation.
    #[error("untrusted key share for generation {generation}")]
    ShareUntrusted {
        /// The generation of the refused share.
        generation: u32,
    },

    /// There is no key share above the held generation.
    #[error("no key share to claim")]
    NothingToClaim,

    /// No item of the vault references the attachment, so a downloaded
    /// ciphertext has nothing to belong to
    /// ([`super::UnlockedSession::store_remote_attachment`]).
    #[error("no item references the attachment")]
    UnknownAttachmentReference,

    /// The backend offered an account-key epoch below this device's: a
    /// rollback of a re-key (CRYPTO_SPEC §6.7).
    #[error("the backend offered account-key epoch {offered}, below {stored}")]
    EpochRollback {
        /// This device's epoch.
        stored: u32,
        /// The epoch offered.
        offered: u32,
    },

    /// The epoch is above the stored one by more than one: nothing moves the
    /// device to it (CRYPTO_SPEC §6.7).
    #[error("account-key epoch {offered} does not follow {stored}")]
    EpochNotNext {
        /// This device's epoch.
        stored: u32,
        /// The epoch offered.
        offered: u32,
    },

    /// The epoch is more than `MAX_EPOCH_JUMP` (16) above the stored one
    /// (CRYPTO_SPEC §6.6, §6.7, ADR-0064): a re-key never targets it and a
    /// folder adoption refuses it. A device that lagged that far behind is
    /// set up again (join).
    #[error("account-key epoch {offered} is too far above {stored}")]
    EpochJumpTooLarge {
        /// This device's epoch.
        stored: u32,
        /// The epoch offered or targeted.
        offered: u32,
    },

    /// The epoch cannot be raised any more.
    #[error("account-key epoch exhausted")]
    EpochExhausted,

    /// A re-key needs a new master password; this one opens the current
    /// wrapping (CRYPTO_SPEC §6.6).
    #[error("the new master password is the current one")]
    PasswordUnchanged,

    /// The new Secret Key is the current one (a re-key and a Secret Key
    /// rotation need a new one), or the one typed for an adoption is the
    /// current or a retired one (CRYPTO_SPEC §6.7).
    #[error("the Secret Key is not new")]
    SecretKeyUnchanged,

    /// A key rotation job is open (CRYPTO_SPEC §6.6).
    #[error("a key rotation is in progress")]
    RotationInProgress,

    /// A re-key needs a clean sync: no halt (other than a re-key marker), no
    /// finding against the server or about sharing, no pending revocation,
    /// and every owned vault created on a hosted backend.
    #[error("sync is not clean")]
    RekeyNotClean,

    /// The backend offered an account version below the highest this device
    /// has seen (CRYPTO_SPEC §9.4).
    #[error("the backend offered an older account version")]
    AccountVersionRollback,
}

impl From<mosaic_core::Error> for Error {
    fn from(e: mosaic_core::Error) -> Self {
        Self::Core(e)
    }
}

impl From<crate::storage::Error> for Error {
    fn from(e: crate::storage::Error) -> Self {
        match e {
            crate::storage::Error::RotationInProgress => Self::RotationInProgress,
            other => Self::Store(other),
        }
    }
}

impl From<mosaic_totp::Error> for Error {
    fn from(e: mosaic_totp::Error) -> Self {
        Self::Otp(e)
    }
}

/// Result type of the session layer.
pub type Result<T> = core::result::Result<T, Error>;
