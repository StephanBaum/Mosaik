//! The folder backend (CRYPTO_SPEC §10.1, ADR-0043): a directory that a
//! file-sync client (Dropbox, `OneDrive`, iCloud Drive, Nextcloud, Syncthing,
//! a NAS share) replicates between the user's devices. [`FolderTransport`]
//! implements [`Transport`](crate::transport::Transport) over `std::fs`
//! only.
//!
//! Every path is written by exactly one install (its own `d/<device>/`, its
//! own `log/<device>/`, its own `.macct` and `.mdel`; blobs are write-once),
//! so a sync client's conflict copies can only appear for `head.mck`,
//! `info.mdev`, `mosaic-sync.json` and `README.txt`. Readers accept only
//! the exact names Mosaic writes ([`crate::wire::folder_names`]), verify
//! every file's MAC before they parse it, and retry a file that fails (it
//! may be a partial upload) on the next sync instead of skipping it.
//!
//! The transport holds no key: its [`SyncCrypto`] is
//! [`SessionCrypto`](crate::wire::SessionCrypto) over the shared session
//! handle, which derives the keys per operation inside the session, so
//! locking the session ends every operation of the transport
//! ([`TransportError::Locked`]) and nothing needs to be dropped. The same
//! handle persists this install's own sequence numbers
//! ([`OwnLogState`]) before a segment is written.

mod account;
mod blobs;
mod devices;
pub(crate) mod fsutil;
mod init;
mod layout;
mod maintain;
mod reader;
mod rekey;
mod writer;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use mosaic_core::{AccountId, InstallId, KeyGeneration, VaultId};

pub use account::{JoinError, JoinFailure, current_account_file, join_from_folder};
pub use init::{FolderContents, FolderError, MAX_LISTED_ACCOUNTS, init, inspect, list_accounts};

use self::layout::{HEAD_NAME, INFO_NAME, Layout};
use crate::clock::Clock;
use crate::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RekeyCandidate, RekeyPublish, RekeySignals, RemoteAttachment, RemoteDevice, RemoteVault,
    Transport, TransportError, TransportKind, VaultCreateResult,
};
use crate::wire::device_info::DeviceInfo;
use crate::wire::segment::SnapshotInfo;
use crate::wire::{OwnLogState, SyncCrypto, WireError};
use core::time::Duration;
use mosaic_core::{AttachmentId, VaultMetaCiphertext, WrappedKey};

/// What the folder transport needs from its crypto provider: the keyed
/// operations, the persisted own-log state, and to be sendable.
pub trait FolderCrypto: SyncCrypto + OwnLogState + Send {}

impl<T: SyncCrypto + OwnLogState + Send> FolderCrypto for T {}

/// The transport error of a failed wire operation: a locked session is
/// [`TransportError::Locked`], anything else `other`.
pub(crate) fn lock_or(e: WireError, other: TransportError) -> TransportError {
    if e == WireError::Locked {
        TransportError::Locked
    } else {
        other
    }
}

/// `Ok(Some(v))` for a decoded value, `Ok(None)` for bytes that do not
/// verify or parse, and `Err(Locked)` if the session is locked (nothing may
/// be concluded about the bytes then).
pub(crate) fn decoded<T>(r: Result<T, WireError>) -> Result<Option<T>, TransportError> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(WireError::Locked) => Err(TransportError::Locked),
        Err(_) => Ok(None),
    }
}

/// What [`FolderTransport::open`] needs.
pub struct FolderConfig<C> {
    /// The folder the user chose.
    pub root: PathBuf,
    /// The account.
    pub account_id: AccountId,
    /// This install.
    pub install_id: InstallId,
    /// The keyed operations (the unlocked session's keys).
    pub crypto: C,
    /// The clock for file contents and age checks.
    pub clock: Arc<dyn Clock>,
    /// This device's name and platform, published as `info.mdev`.
    pub device: DeviceInfo,
}

/// What is known of a segment file from reading it once.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Outcome {
    /// Did not verify or parse.
    Bad,
    /// Verified; the highest key generation it carries, whether it
    /// carries a vault key at all and its snapshot fields.
    Good {
        max_generation: Option<KeyGeneration>,
        has_key: bool,
        snapshot: Option<SnapshotInfo>,
    },
}

/// The cache entry of a segment file: valid for the length and modification
/// time it was read at.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScanEntry {
    pub(crate) len: u64,
    pub(crate) modified: Option<SystemTime>,
    pub(crate) outcome: Outcome,
}

/// The folder transport. See the [module documentation](self).
pub struct FolderTransport<C: FolderCrypto> {
    pub(crate) layout: Layout,
    pub(crate) account: AccountId,
    pub(crate) install: InstallId,
    pub(crate) crypto: C,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) scan: BTreeMap<(VaultId, InstallId, u64), ScanEntry>,
    /// The next sequence number of this install's log per vault, so it never
    /// goes back within a run even if the directory listing lags (the
    /// persisted reservation covers a restart).
    pub(crate) own_next: BTreeMap<VaultId, u64>,
    /// Vaults whose own log skipped a planted name: the next housekeeping
    /// compacts them, so the hole closes.
    pub(crate) force_compact: BTreeSet<VaultId>,
    /// The checkpoint candidates of this install's own directory that the
    /// last [`Transport::checkpoints`] call read (name, length, time): only
    /// conflict copies read in the run are deleted.
    pub(crate) own_checkpoints_read: BTreeMap<OsString, (u64, Option<SystemTime>)>,
    /// This device's name and platform, published again as `info.mdev` after
    /// a re-key (the sealed file is under the account key of its epoch).
    pub(crate) device: DeviceInfo,
    /// The account-key epoch the verification results in `scan` belong to
    /// (0 before the first look): a segment that verified under the previous
    /// epoch's keys does not verify under the new ones, so the cache is
    /// dropped when the epoch changes (M7).
    pub(crate) scan_epoch: u32,
}

impl<C: FolderCrypto> core::fmt::Debug for FolderTransport<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FolderTransport")
            .field("account", &self.account)
            .field("install", &self.install)
            .finish_non_exhaustive()
    }
}

impl<C: FolderCrypto> FolderTransport<C> {
    /// Opens a Mosaic folder (see [`init`]) for one account and publishes
    /// this device's sealed info.
    ///
    /// # Errors
    /// [`TransportError::FolderUnavailable`] if the folder is missing, not
    /// empty and not a Mosaic folder; [`TransportError::Incompatible`] for
    /// another folder version; [`TransportError::Io`].
    pub fn open(config: FolderConfig<C>) -> Result<Self, TransportError> {
        match inspect(&config.root) {
            FolderContents::Mosaic { .. } => {}
            FolderContents::Missing | FolderContents::Foreign | FolderContents::Empty => {
                return Err(TransportError::FolderUnavailable);
            }
            FolderContents::OtherVersion => return Err(TransportError::Incompatible),
        }
        let layout = Layout::new(&config.root, &config.account_id);
        let transport = Self {
            layout,
            account: config.account_id,
            install: config.install_id,
            crypto: config.crypto,
            clock: config.clock,
            scan: BTreeMap::new(),
            own_next: BTreeMap::new(),
            force_compact: BTreeSet::new(),
            own_checkpoints_read: BTreeMap::new(),
            device: config.device.clone(),
            scan_epoch: 0,
        };
        transport.write_device_info(&config.device)?;
        Ok(transport)
    }

    /// The account.
    #[must_use]
    pub const fn account_id(&self) -> AccountId {
        self.account
    }

    /// This install.
    #[must_use]
    pub const fn install_id(&self) -> InstallId {
        self.install
    }

    fn write_device_info(&self, info: &DeviceInfo) -> Result<(), TransportError> {
        let sealed = info
            .seal(&self.crypto)
            .map_err(|e| lock_or(e, TransportError::Io))?;
        fsutil::write_atomic(
            &self.layout.device_dir(&self.install),
            INFO_NAME,
            &sealed,
            true,
        )
    }

    pub(crate) fn now_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    /// Drops the cached verification results when the account-key epoch of
    /// the session changed since they were made.
    pub(crate) fn refresh_epoch(&mut self) -> Result<(), TransportError> {
        let epoch = self
            .crypto
            .ak_epoch()
            .map_err(|e| lock_or(e, TransportError::Io))?;
        if epoch != self.scan_epoch {
            self.scan.clear();
            self.scan_epoch = epoch;
        }
        Ok(())
    }
}

impl<C: FolderCrypto> Transport for FolderTransport<C> {
    fn kind(&self) -> TransportKind {
        TransportKind::Folder
    }

    fn list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        self.folder_list_vaults()
    }

    fn create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
        _device: InstallId,
    ) -> Result<VaultCreateResult, TransportError> {
        self.folder_create_vault(vault, generation, wrapped, meta)
    }

    fn pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        self.folder_pull(vault, cursor, limit)
    }

    fn push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        self.folder_push_records(vault, generation, records)
    }

    fn push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
        _base_seq: Option<u64>,
    ) -> Result<PushResult, TransportError> {
        self.folder_push_meta(vault, meta)
    }

    fn push_vault_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        _device: InstallId,
        _expected_head: Option<&Cursor>,
    ) -> Result<KeyPushResult, TransportError> {
        self.folder_push_key(vault, generation, wrapped)
    }

    fn put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.folder_put_attachment(vault, id, ciphertext)
    }

    fn replace_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        self.folder_replace_attachment(vault, id, ciphertext)
    }

    fn get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        self.folder_get_attachment(vault, id)
    }

    fn delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        self.folder_delete_attachment(vault, id)
    }

    fn restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        self.folder_restore_attachment(vault, id)
    }

    fn attachments(&mut self, vault: VaultId) -> Result<Vec<RemoteAttachment>, TransportError> {
        self.folder_attachments(vault)
    }

    fn put_checkpoint(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
        if bytes.len() > crate::wire::checkpoint::MAX_LEN {
            return Err(TransportError::Protocol);
        }
        fsutil::write_atomic(
            &self.layout.device_dir(&self.install),
            HEAD_NAME,
            bytes,
            true,
        )
    }

    fn checkpoints(&mut self) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        self.folder_checkpoints()
    }

    fn devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        self.folder_devices()
    }

    fn checkpoint_high_seq(
        &mut self,
        vault: VaultId,
        _cursor: &Cursor,
    ) -> Result<u64, TransportError> {
        self.own_high_seq(vault)
    }

    fn own_logs_rolled_back(&mut self, vaults: &[VaultId]) -> Result<bool, TransportError> {
        self.folder_own_logs_rolled_back(vaults)
    }

    fn maintain(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        self.folder_maintain(cx)
    }

    fn rekey_signals(&mut self, epoch: u32) -> Result<RekeySignals, TransportError> {
        self.folder_rekey_signals(epoch)
    }

    fn rekey_candidates(
        &mut self,
        epoch: u32,
        limit: usize,
    ) -> Result<Vec<RekeyCandidate>, TransportError> {
        self.folder_rekey_candidates(epoch, limit)
    }

    fn next_folder_account_version(&mut self) -> Result<Option<u64>, TransportError> {
        self.next_account_version().map(Some)
    }

    fn publish_rekey(
        &mut self,
        cx: &mut dyn MaintainContext,
        publish: &RekeyPublish<'_>,
    ) -> Result<(), TransportError> {
        self.folder_publish_rekey(cx, publish)
    }

    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        self.folder_wait_for_change(since, timeout)
    }
}
