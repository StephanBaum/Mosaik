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

use mosaic_core::model::Role;
use mosaic_core::{
    AccountId, AttachmentId, InstallId, ItemId, KeyGeneration, PasswordParams, ShareSeal, VaultId,
    VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use mosaic_vault::storage::{FolderRekeyFloor, RecordBody};

use crate::sharing::SharingTransport;
use crate::wire::account_file::AccountFile;
use crate::wire::rekey_marker::LEN as MARKER_LEN;
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

/// A key share addressed to this account, pulled from the hosted backend
/// (M6b: a vault key sealed to this account's identity key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledShare {
    /// The generation of the key it carries.
    pub generation: KeyGeneration,
    /// Who sealed it (the server's claim; the claim rule checks it).
    pub sender: AccountId,
    /// The seal.
    pub seal: ShareSeal,
    /// The backend's sequence number, if it has one.
    pub seq: Option<u64>,
}

/// A vault roster pulled from the hosted backend (the current one and the
/// history of earlier generations newer than the cursor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulledRoster {
    /// The generation it is sealed under.
    pub generation: KeyGeneration,
    /// The sealed roster.
    pub ciphertext: VaultRosterCiphertext,
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// Hosted only (M6b): every unclaimed key share addressed to this
    /// account, whatever the cursor. Empty for the folder.
    pub shares: Vec<PulledShare>,
    /// Hosted only (M6b): the rosters newer than the cursor. Empty for the
    /// folder.
    pub rosters: Vec<PulledRoster>,
    /// Hosted only (M6b): the role the server says this account has in the
    /// vault. `None` for the folder.
    pub role: Option<Role>,
    /// Hosted only (M7): a member that re-keyed asked the owner to rotate
    /// this vault (only the owner's page carries it; the request is advisory,
    /// THREAT_MODEL R86).
    pub rotation_requested: bool,
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
    /// The account's role does not allow the request (M6b).
    RoleForbidden,
    /// A plain key upload or a rotation that does not cover exactly the
    /// vault's members and open invites (M6b).
    MembersMismatch,
    /// The identity key is already set to another pair (M6b).
    IdentityExists,
    /// A compare-and-swap failed (M6b).
    Conflict {
        /// The sequence the server holds, if it said.
        current_seq: Option<u64>,
    },
    /// The recipient already has an open invite to the vault (M6b).
    InviteExists,
    /// The invite was sealed for an older key generation (M6b).
    InviteStale {
        /// The vault's generation now.
        current_generation: u32,
    },
    /// The invite ran out (M6b).
    InviteExpired,
    /// The vault is at its member limit or the account at its invite limit
    /// (M6b).
    InviteLimit,
    /// The invited account is a member already (M6b).
    AlreadyMember,
    /// A removed account cannot be invited again before the vault rotates
    /// past the generation of its removal (M6b).
    ReinviteBlocked,
    /// The owner cannot leave the vault (M6b).
    OwnerCannotLeave,
    /// No key share or emergency seal backs the claim (M6b).
    NoShare,
    /// The emergency wait is not over, or a request is open (M6b).
    EmergencyWaiting {
        /// When the wait ends (ms since the Unix epoch), if the server said.
        release_at: Option<u64>,
    },
    /// The grantee asked less than 24 hours after a refusal (M6b).
    EmergencyCooldown {
        /// When asking is allowed again (ms since the Unix epoch).
        until: u64,
    },
    /// The grant was approved or released already (M6b).
    EmergencyReleased,
    /// There is no open emergency request (M6b).
    EmergencyNotRequested,
    /// Emergency seals cover only vaults the grantor owns (M6b).
    EmergencyOwnerOnly,
    /// A re-key named an epoch other than the account's plus one (M7).
    EpochMismatch {
        /// The account's epoch on the server.
        current_epoch: u32,
    },
    /// A re-key with a weaker Argon2id profile than the account's (M7).
    ProfileDowngrade,
    /// A re-key request that does not cover exactly what the server holds
    /// for the account (M7).
    RekeyMismatch {
        /// Which list of the request.
        set: RekeySet,
    },
}

/// The list of a re-key request that did not match the server's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RekeySet {
    /// The owned vaults.
    Vaults,
    /// The vaults shared to the account.
    MemberRewraps,
    /// The emergency seals the account gave.
    EmergencySeals,
    /// A list this client does not know.
    Other,
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
    /// A pull page whose head is below the cursor the device already read:
    /// the vault was rolled back on the backend (hosted). The engine reports
    /// it for that vault and carries on with the others.
    #[error("backend head below the cursor")]
    HeadBelowCursor,
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
    /// A master password change was sent, its answer was lost (timeout,
    /// unreachable, 5xx) and the check with the new password could not
    /// confirm it either: the server may or may not hold the new password
    /// (CRYPTO_SPEC §9.4, "Lost answer").
    #[error("password change outcome unknown")]
    OutcomeUnknown,
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

    /// The pre-re-key own-segment floors (folder, M7): own segments of a vault
    /// up to the floor were sealed under the previous epoch and are kept
    /// until a snapshot set of the new epoch exists and the retention passed.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the session is locked or the store fails.
    fn rekey_floors(&mut self) -> Result<Vec<FolderRekeyFloor>, TransportError> {
        Ok(Vec::new())
    }

    /// Records that a complete snapshot set of the new epoch exists in the
    /// own log of `vault`.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the session is locked or the store fails.
    fn mark_rekey_snapshot(&mut self, vault: VaultId) -> Result<(), TransportError> {
        let _ = vault;
        Ok(())
    }
}

/// A re-key marker that verified under this device's keys (folder only, M7):
/// the account was re-keyed away from this device's epoch. Anyone who holds
/// the old keys (a thief of an old device) can write one, so a device that
/// cannot follow it can dismiss it (CRYPTO_SPEC 10.1, review H1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RekeyMarkerSeen {
    /// The marker's file name in the account directory.
    pub name: String,
    /// SHA-256 of its bytes.
    pub sha256: [u8; 32],
    /// The epoch the re-key moved to (above this device's, not always by one).
    pub to_epoch: u32,
    /// The `account_version` of the new account file.
    pub to_account_version: u64,
    /// The device that re-keyed (it wrote the new account file).
    pub device: InstallId,
}

/// An account file that does not verify under this device's keys and names
/// this device's epoch or a higher one (folder only, M7): a re-key this
/// device cannot verify, a concurrent re-key, or a planted file. A finding,
/// never a halt (CRYPTO_SPEC §10.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnverifiedAccountFile {
    /// The file's name in the account directory.
    pub name: String,
    /// SHA-256 of its bytes.
    pub sha256: [u8; 32],
    /// The epoch its clear header states (unauthenticated).
    pub epoch: u32,
}

/// What a transport saw of a re-key (folder only, M7).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RekeySignals {
    /// The verified `.mrek` files for this device's epoch (at most
    /// `MAX_SCANNED`, newest epoch first): the only hard stop, unless the
    /// user dismissed them.
    pub markers: Vec<RekeyMarkerSeen>,
    /// Account files that do not verify and name this epoch or a higher one.
    pub unverified: Vec<UnverifiedAccountFile>,
}

/// An account file a device that lags behind a re-key may try to adopt
/// (folder only, M7), with its bytes for the MAC check after the typed keys
/// opened its wrapping.
#[derive(Clone, Debug)]
pub struct RekeyCandidate {
    /// The parsed header.
    pub file: AccountFile,
    /// The whole file.
    pub bytes: Vec<u8>,
}

/// What a device that re-keyed (or adopted a re-key) publishes to a folder:
/// the new account file, a full snapshot set of each vault under the new keys
/// and the marker for the devices of the old epoch (in that order).
#[derive(Clone, Copy, Debug)]
pub struct RekeyPublish<'a> {
    /// The new account file (parameters, wrapped `AK`, version); `None` for a
    /// device that adopted another device's re-key.
    pub account_file: Option<(&'a PasswordParams, &'a WrappedKey, u64)>,
    /// The vaults to write a snapshot set for.
    pub snapshot_vaults: &'a [VaultId],
    /// The marker, built under the old epoch's keys; `None` for an adopting
    /// device.
    pub marker: Option<&'a [u8; MARKER_LEN]>,
    /// The epoch the marker leaves (the name of the marker file).
    pub from_epoch: u32,
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
    /// [`TransportError`]; hosted: `Rejected(Rejection::CheckpointStale)`
    /// when the server already holds a checkpoint with this counter or a
    /// higher one (the engine does not count it as published).
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

    /// What the backend shows of an account re-key, for a device at account
    /// key epoch `epoch` (M7; the folder reads its account directory, the
    /// hosted backend signs the device out instead and answers nothing).
    ///
    /// # Errors
    /// [`TransportError`].
    fn rekey_signals(&mut self, epoch: u32) -> Result<RekeySignals, TransportError> {
        let _ = epoch;
        Ok(RekeySignals::default())
    }

    /// The account-key epoch the backend states for the account (hosted
    /// only, M7): unauthenticated, the server's claim. `None` for a backend
    /// that does not state one (the folder checks its account files instead).
    ///
    /// # Errors
    /// [`TransportError`].
    fn backend_epoch(&mut self) -> Result<Option<u32>, TransportError> {
        Ok(None)
    }

    /// The account files a device at `epoch` may try to adopt, best first:
    /// the files every verified marker names, then the others by version,
    /// only those whose header names an epoch above `epoch` (at most `limit`).
    /// Default none.
    ///
    /// # Errors
    /// [`TransportError`].
    fn rekey_candidates(
        &mut self,
        epoch: u32,
        limit: usize,
    ) -> Result<Vec<RekeyCandidate>, TransportError> {
        let _ = (epoch, limit);
        Ok(Vec::new())
    }

    /// The `account_version` the next account file of a folder takes: the
    /// highest verified one plus one. `None` for a backend without account
    /// files (hosted).
    ///
    /// # Errors
    /// [`TransportError`].
    fn next_folder_account_version(&mut self) -> Result<Option<u64>, TransportError> {
        Ok(None)
    }

    /// Publishes a re-key (see [`RekeyPublish`]). Every step is idempotent: a
    /// crashed publish is run again from the start. The default does nothing.
    ///
    /// # Errors
    /// [`TransportError`].
    fn publish_rekey(
        &mut self,
        cx: &mut dyn MaintainContext,
        publish: &RekeyPublish<'_>,
    ) -> Result<(), TransportError> {
        let _ = (cx, publish);
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

    /// The sharing and emergency-access calls of the hosted backend (M6b).
    /// `None` for a backend that cannot share (the folder, CRYPTO_SPEC
    /// §8.6): the engine then skips every sharing step. A wrapper passes the
    /// inner transport's answer on.
    fn sharing(&mut self) -> Option<&mut dyn SharingTransport> {
        None
    }
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
