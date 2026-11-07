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

use crate::wire::segment::SegmentObject;

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
    /// Vault metadata objects, in the order the backend lists them (the
    /// hosted backend sends at most one; a folder page can hold one per
    /// segment, and the engine merges them).
    pub metas: Vec<PulledMeta>,
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

/// Why the hosted server refused a request (a code of the protocol that has
/// no variant of its own in [`TransportError`]). It carries no data of the
/// request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rejection {
    /// Registration: the email is taken.
    AccountExists,
    /// Registration is closed on this server.
    RegistrationClosed,
    /// The invite code is missing, wrong or used.
    InvalidInvite,
    /// The email address is not valid.
    InvalidEmail,
    /// Login: wrong credentials (the server does not say which).
    LoginFailed,
    /// This device was removed from the account.
    DeviceRevoked,
    /// The second-factor code is wrong or its attempts are used up.
    TwoFactorInvalid,
    /// The server did not accept the request body.
    BadRequest,
    /// The request or object is larger than the server accepts.
    TooLarge,
    /// An attachment with this ID and another size exists.
    AttachmentExists,
    /// The checkpoint is not newer than the stored one.
    CheckpointStale,
    /// A second long poll on one session.
    TooManyPolls,
    /// A device cannot remove itself.
    DeviceIsCurrent,
    /// A password change raced another one.
    AccountVersionMismatch {
        /// The account version on the server.
        account_version: u64,
    },
}

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
    /// The server refused the request for a reason of its own (hosted).
    #[error("request refused")]
    Rejected(Rejection),
    /// The server URL is `http` and not a loopback address, or uses another
    /// scheme than `https`.
    #[error("server URL is not https")]
    InsecureUrl,
    /// The server URL is not `https://host[:port]`.
    #[error("invalid server URL")]
    InvalidUrl,
    /// The session is locked, so the transport has no keys (the folder
    /// transport derives them per operation from the session). The engine
    /// reports it as `SyncError::Locked`.
    #[error("session locked")]
    Locked,
}

/// What a transport may ask the engine for while it maintains its storage
/// ([`Transport::maintain`]). The engine answers each call with a short
/// session step, so the session lock is never held across a transport call.
pub trait MaintainContext {
    /// The current time (ms since the Unix epoch).
    fn now_ms(&self) -> u64;

    /// The writer's current state of `vault` as segment objects: its current
    /// key, its metadata, every readable item at the current generation, then
    /// every tombstone (a snapshot, CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`TransportError::Io`] if the session is locked or the store fails.
    fn snapshot_objects(&mut self, vault: VaultId) -> Result<Vec<SegmentObject>, TransportError>;

    /// Whether an item of a synced vault, live or in the trash, refers to the
    /// attachment.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the session is locked.
    fn attachment_referenced(&mut self, id: AttachmentId) -> Result<bool, TransportError>;

    /// Whether this sync run saw no other device behind or missing files, no
    /// unresolved key conflict and no object it could not explain. Purging
    /// data other devices might still need waits for it.
    fn caught_up(&self) -> bool;

    /// When this device first saw each delete marker of `vault`, by its own
    /// clock, in the order of `markers`; a marker seen for the first time (or
    /// with another stamp) is first seen `now`. Markers not listed are
    /// forgotten. A marker's age counts from this time, never from the
    /// wall-clock time inside the marker, which anyone with the account key
    /// (or a device with a skewed clock) could set.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the session is locked or the store fails.
    fn marker_first_seen(
        &mut self,
        vault: VaultId,
        markers: &[mosaic_vault::storage::MarkerStamp],
        now: u64,
    ) -> Result<Vec<u64>, TransportError>;
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

    /// Replaces an attachment ciphertext that does not authenticate with the
    /// holder's authentic local copy. The default is [`Transport::put_attachment`];
    /// the folder (blobs are write-once) overwrites through a temp file and a
    /// rename, the one exception to the write-once rule (CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`TransportError`].
    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.put_attachment(vault, id, ciphertext)
    }

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

    /// Publishes this device's checkpoint, the bytes of
    /// `Checkpoint::encode`. The transport files them under this install's
    /// ID (the folder's `d/<device>/head.mck`, the hosted `deviceId`) and
    /// never decodes them.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError>;

    /// Every device's latest checkpoint as raw bytes, each with the
    /// install ID it is filed under (the folder path's device, the hosted
    /// `deviceId`), this device's included. The caller decodes each with
    /// `Checkpoint::decode(bytes, device, crypto)`, which verifies the MAC
    /// and that the body names the device it is filed under. A transport
    /// that sees several candidate files for one device (conflict copies)
    /// returns each of them.
    ///
    /// # Errors
    /// [`TransportError`].
    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError>;

    /// The backend's device list.
    ///
    /// # Errors
    /// [`TransportError`].
    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError>;

    /// The `high_seq` of this device's checkpoint entry for `vault`: the
    /// hosted default reads the 8-byte big-endian server sequence of the
    /// device's `cursor`; the folder answers the highest segment sequence
    /// this device wrote for the vault (0 if none).
    ///
    /// # Errors
    /// [`TransportError`].
    fn checkpoint_high_seq(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
    ) -> Result<u64, TransportError> {
        let _ = vault;
        match <[u8; 8]>::try_from(cursor.0.as_slice()) {
            Ok(bytes) => Ok(u64::from_be_bytes(bytes)),
            Err(_) if cursor.0.is_empty() => Ok(0),
            Err(_) => Err(TransportError::Protocol),
        }
    }

    /// Whether this install's own log of any of `vaults` lost segments it
    /// wrote (a folder that was rolled back or had files deleted): `true`
    /// means so. The engine then halts before it pushes anything. The default
    /// (hosted) is `false`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn own_logs_rolled_back(&mut self, vaults: &[VaultId]) -> Result<bool, TransportError> {
        let _ = vaults;
        Ok(false)
    }

    /// Housekeeping after a sync run: the folder compacts its own log and
    /// removes stale temp and conflict files and expired blobs. The default
    /// does nothing.
    ///
    /// # Errors
    /// [`TransportError`].
    fn maintain(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        let _ = cx;
        Ok(())
    }

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
