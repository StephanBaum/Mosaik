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
//! The transport holds the keys of [`SyncCrypto`]
//! ([`crate::wire::KeyedCrypto`] in production) and so must be dropped when
//! the session locks.

mod account;
mod blobs;
mod devices;
pub(crate) mod fsutil;
mod init;
mod layout;
mod maintain;
mod reader;
mod writer;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use mosaic_core::{AccountId, InstallId, KeyGeneration, VaultId};

pub use account::{JoinError, JoinFailure, current_account_file, join_from_folder};
pub use init::{FolderContents, FolderError, init, inspect, list_accounts};

use self::layout::{HEAD_NAME, INFO_NAME, Layout};
use crate::clock::Clock;
use crate::transport::{
    ChangeToken, Cursor, KeyPushResult, MaintainContext, PullPage, PushRecord, PushResult,
    RemoteAttachment, RemoteDevice, RemoteVault, Transport, TransportError, TransportKind,
    VaultCreateResult,
};
use crate::wire::SyncCrypto;
use crate::wire::device_info::DeviceInfo;
use crate::wire::segment::{SnapshotInfo, SnapshotTracker};
use core::time::Duration;
use mosaic_core::{AttachmentId, VaultMetaCiphertext, WrappedKey};

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
pub struct FolderTransport<C: SyncCrypto + Send> {
    pub(crate) layout: Layout,
    pub(crate) account: AccountId,
    pub(crate) install: InstallId,
    pub(crate) crypto: C,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) scan: BTreeMap<(VaultId, InstallId, u64), ScanEntry>,
    pub(crate) trackers: BTreeMap<(VaultId, InstallId), SnapshotTracker>,
    /// The next sequence number of this install's log per vault, so it never
    /// goes back within a run even if the directory listing lags.
    pub(crate) own_next: BTreeMap<VaultId, u64>,
}

impl<C: SyncCrypto + Send> core::fmt::Debug for FolderTransport<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FolderTransport")
            .field("account", &self.account)
            .field("install", &self.install)
            .finish_non_exhaustive()
    }
}

impl<C: SyncCrypto + Send> FolderTransport<C> {
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
            trackers: BTreeMap::new(),
            own_next: BTreeMap::new(),
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
        let sealed = info.seal(&self.crypto).map_err(|_| TransportError::Io)?;
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
}

impl<C: SyncCrypto + Send> Transport for FolderTransport<C> {
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

    fn maintain(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        self.folder_maintain(cx)
    }

    fn wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        self.folder_wait_for_change(since, timeout)
    }
}
