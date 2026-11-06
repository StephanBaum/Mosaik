//! The transport trait (M6 spec §2.1, ADR-0042): everything the sync engine
//! needs from a backend, blocking and object-safe. The hosted transport
//! (M6 T4) and the folder transport (M6 T2) implement it; so do the test
//! transports behind the `test-util` feature.
//!
//! The engine never holds the session lock while it calls a transport.
//! Every method returns [`TransportError`], whose variants carry no paths,
//! bodies or IDs.
//!
//! A backend without a server sequence (the folder) answers `seq: None`; the
//! engine then allocates a device-local sequence from the store so
//! `records.server_seq` stays monotonic per record on this device. Folder
//! sequence numbers never leave the device.

use core::time::Duration;

use mosaic_core::{
    AttachmentId, InstallId, ItemId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey,
};
use mosaic_vault::storage::RecordBody;

use crate::wire::checkpoint::Checkpoint;

/// Which kind of backend a transport talks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    /// The Mosaic server.
    Hosted,
    /// A user-chosen folder.
    Folder,
}

/// An opaque per-vault pull cursor of at most 65,536 bytes. Hosted: the
/// 8-byte big-endian `server_seq`. Folder: `wire::cursor`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Cursor(pub Vec<u8>);

/// A vault the backend holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteVault {
    /// The vault.
    pub vault_id: VaultId,
    /// Its current key generation on the backend.
    pub generation: KeyGeneration,
}

/// A wrapped vault key pulled from the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledKey {
    /// The key's generation.
    pub generation: KeyGeneration,
    /// The key wrapped under the account key.
    pub wrapped: WrappedKey,
    /// The install that generated the key (the canonical-winner rule,
    /// CRYPTO_SPEC §10.1).
    pub device_id: InstallId,
    /// The backend's sequence number, if it has one.
    pub seq: Option<u64>,
}

/// Vault metadata pulled from the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledMeta {
    /// The sealed metadata.
    pub ciphertext: VaultMetaCiphertext,
    /// The backend's sequence number, if it has one.
    pub seq: Option<u64>,
}

/// A record pulled from the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledRecord {
    /// The record ID (the item ID).
    pub record_id: ItemId,
    /// The ciphertext.
    pub body: RecordBody,
    /// The backend's sequence number, if it has one.
    pub seq: Option<u64>,
}

/// A group of records that must be fully applied before the cursor may move
/// past them (folder: one segment, CRYPTO_SPEC §10.1): the records
/// `first_record..end_record` of the page, and the cursor to store once they
/// are applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullUnit {
    /// Index of the unit's first record in [`PullPage::records`].
    pub first_record: usize,
    /// Index one past the unit's last record.
    pub end_record: usize,
    /// The cursor after this unit.
    pub commit: Cursor,
}

/// One page of changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullPage {
    /// Wrapped keys, in the order the backend lists them.
    pub keys: Vec<PulledKey>,
    /// The vault metadata, if it changed.
    pub meta: Option<PulledMeta>,
    /// Records, oldest first.
    pub records: Vec<PulledRecord>,
    /// The cursor after the whole page.
    pub next: Cursor,
    /// More pages follow.
    pub more: bool,
    /// Folder only: the units of the page (see [`PullUnit`]); empty for the
    /// hosted backend.
    pub units: Vec<PullUnit>,
}

/// A record to upload.
#[derive(Clone, Copy, Debug)]
pub struct PushRecord<'a> {
    /// The record ID (the item ID).
    pub record_id: ItemId,
    /// The ciphertext.
    pub body: &'a RecordBody,
    /// The backend's sequence number of the version this one replaces
    /// (`None` for a record the backend does not have).
    pub base_seq: Option<u64>,
}

/// What the backend did with a record or metadata upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushResult {
    /// Stored.
    Accepted {
        /// The new sequence number, if the backend has one.
        seq: Option<u64>,
    },
    /// The backend holds another version than `base_seq`: pull first.
    Conflict {
        /// The sequence number the backend holds.
        current_seq: u64,
    },
}

/// What the backend did with a vault key upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyPushResult {
    /// Stored.
    Accepted {
        /// The key's sequence number, if the backend has one.
        seq: Option<u64>,
    },
    /// The backend already has a key of that generation.
    GenerationTaken {
        /// The backend's current generation.
        current: KeyGeneration,
    },
    /// The vault's head moved since `expected_head`: pull and retry.
    HeadMoved {
        /// The vault's head now.
        head: Cursor,
    },
}

/// What the backend did with a vault creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultCreateResult {
    /// The vault was created.
    Created,
    /// The vault exists already.
    Exists,
}

/// An attachment the backend holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteAttachment {
    /// The attachment.
    pub attachment_id: AttachmentId,
    /// The ciphertext size.
    pub size: u64,
    /// When it was soft-deleted (ms since the Unix epoch), if it was.
    pub deleted_at_ms: Option<u64>,
}

/// A device on the backend's device list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteDevice {
    /// The install.
    pub device_id: InstallId,
    /// Its name, if known.
    pub name: Option<String>,
    /// When it first appeared (ms since the Unix epoch), if known.
    pub created_at_ms: Option<u64>,
    /// When it was last seen (ms since the Unix epoch), if known.
    pub last_seen_ms: Option<u64>,
}

/// An opaque token that changes when the backend changes
/// ([`Transport::wait_for_change`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeToken(pub Vec<u8>);

/// Why a transport call failed. No variant carries a path, a body or an ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The backend cannot be reached.
    #[error("backend unreachable")]
    Unreachable,
    /// The backend did not answer in time.
    #[error("backend timed out")]
    Timeout,
    /// The session is not valid (hosted: signed out or removed).
    #[error("not authorized")]
    Unauthorized,
    /// The account needs its second factor.
    #[error("two-factor code required")]
    TwoFactorRequired,
    /// The action needs a fresh login.
    #[error("fresh login required")]
    FreshLoginRequired,
    /// Too many requests.
    #[error("rate limited, retry in {retry_after_s} s")]
    RateLimited {
        /// How long to wait, in seconds.
        retry_after_s: u32,
    },
    /// The account's quota is used up.
    #[error("quota exceeded")]
    QuotaExceeded,
    /// An upload carried another key generation than the vault's current one.
    #[error("key generation mismatch")]
    GenerationMismatch {
        /// The vault's current generation on the backend.
        current: KeyGeneration,
    },
    /// The account may not do this.
    #[error("forbidden")]
    Forbidden,
    /// The backend does not have it.
    #[error("not found")]
    NotFound,
    /// The server is not the one the account was set up with.
    #[error("server identity mismatch")]
    ServerMismatch,
    /// The backend speaks another protocol version.
    #[error("incompatible protocol")]
    Incompatible,
    /// The backend answered something that is not the protocol.
    #[error("protocol error")]
    Protocol,
    /// The folder is not available (unplugged, not shared, not empty).
    #[error("folder unavailable")]
    FolderUnavailable,
    /// A local or network I/O failure.
    #[error("I/O error")]
    Io,
}

/// A sync backend. See the [module documentation](self).
pub trait Transport: Send {
    /// Which kind of backend this is.
    fn kind(&self) -> TransportKind;

    /// The vaults the backend holds.
    ///
    /// # Errors
    /// [`TransportError`].
    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError>;

    /// Creates a vault with its current key and metadata.
    ///
    /// # Errors
    /// [`TransportError`].
    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        device: InstallId,
    ) -> Result<VaultCreateResult, TransportError>;

    /// Pulls up to `limit` objects after `cursor`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError>;

    /// Uploads records sealed under `generation`. The results have the
    /// order and length of `records`.
    ///
    /// # Errors
    /// [`TransportError`] (`GenerationMismatch` fails the whole batch).
    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError>;

    /// Uploads the vault's metadata on top of `base_seq`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError>;

    /// Uploads a wrapped vault key. With `expected_head`, the upload only
    /// succeeds if the vault's head is still there (a two-phase rotation);
    /// `None` is the generation compare-and-swap alone (a rotation that was
    /// already committed locally).
    ///
    /// # Errors
    /// [`TransportError`].
    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
        expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError>;

    /// Uploads an attachment ciphertext.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError>;

    /// Downloads an attachment ciphertext, `None` if the backend has none.
    ///
    /// # Errors
    /// [`TransportError`].
    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError>;

    /// Soft-deletes an attachment.
    ///
    /// # Errors
    /// [`TransportError`].
    fn delete_attachment(&mut self, vault: VaultId, id: AttachmentId)
    -> Result<(), TransportError>;

    /// Undoes a soft delete. `false` if the attachment was purged or never
    /// existed.
    ///
    /// # Errors
    /// [`TransportError`].
    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError>;

    /// The attachments the backend holds for a vault.
    ///
    /// # Errors
    /// [`TransportError`].
    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError>;

    /// Publishes this device's checkpoint.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), TransportError>;

    /// Every device's latest checkpoint, this device's included. The caller
    /// verifies each MAC.
    ///
    /// # Errors
    /// [`TransportError`].
    fn checkpoints(&mut self) -> Result<Vec<Checkpoint>, TransportError>;

    /// The backend's device list.
    ///
    /// # Errors
    /// [`TransportError`].
    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError>;

    /// Waits until the backend differs from `since` (or `timeout` passes)
    /// and returns the token to pass next time.
    ///
    /// # Errors
    /// [`TransportError`].
    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trait is object-safe: the desktop holds a `Box<dyn Transport>`.
    #[test]
    fn transport_is_object_safe() {
        fn takes(_: Option<Box<dyn Transport>>) {}
        takes(None);
    }

    #[test]
    fn errors_display_without_data() {
        assert_eq!(
            TransportError::Unreachable.to_string(),
            "backend unreachable"
        );
        assert_eq!(
            TransportError::RateLimited { retry_after_s: 7 }.to_string(),
            "rate limited, retry in 7 s"
        );
        assert_eq!(Cursor::default(), Cursor(Vec::new()));
    }
}
