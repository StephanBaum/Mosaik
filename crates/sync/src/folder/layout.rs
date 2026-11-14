//! The directory layout of the folder backend (CRYPTO_SPEC §10.1, ADR-0043).
//!
//! ```text
//! F/mosaic-sync.json
//! F/README.txt
//! F/a/<account>/account/<version:016x>-<device>.macct
//! F/a/<account>/d/<device>/head.mck
//! F/a/<account>/d/<device>/info.mdev
//! F/a/<account>/v/<vault>/log/<device>/<seq:016x>.mseg
//! F/a/<account>/v/<vault>/blobs/<attachment>.matt
//! F/a/<account>/v/<vault>/blobs/<attachment>.<device>.mdel
//! ```
//!
//! `F/s/` is reserved for shared vaults (M6b); this crate never touches it.

use std::path::{Path, PathBuf};

use mosaic_core::{AccountId, AttachmentId, InstallId, VaultId};

use crate::wire::folder_names;

/// The marker file's name.
pub(crate) const MARKER_NAME: &str = "mosaic-sync.json";
/// The exact bytes of the marker file.
pub(crate) const MARKER_BYTES: &[u8] = br#"{"format":"mosaic-folder","version":1}"#;
/// The readme's name.
pub(crate) const README_NAME: &str = "README.txt";
/// The exact bytes of the readme.
pub(crate) const README_BYTES: &[u8] =
    b"Managed by Mosaic. Do not edit, move or delete files here.\n";
/// The canonical checkpoint file name.
pub(crate) const HEAD_NAME: &str = "head.mck";
/// The device info file name.
pub(crate) const INFO_NAME: &str = "info.mdev";

/// The paths of one account in a folder.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    account_dir: PathBuf,
}

impl Layout {
    pub(crate) fn new(root: &Path, account: &AccountId) -> Self {
        Self {
            account_dir: root
                .join("a")
                .join(folder_names::dir_name(account.as_bytes())),
        }
    }

    /// The account's directory.
    pub(crate) fn dir(&self) -> PathBuf {
        self.account_dir.clone()
    }

    pub(crate) fn account_files(&self) -> PathBuf {
        self.account_dir.join("account")
    }

    pub(crate) fn devices_dir(&self) -> PathBuf {
        self.account_dir.join("d")
    }

    pub(crate) fn device_dir(&self, device: &InstallId) -> PathBuf {
        self.devices_dir()
            .join(folder_names::dir_name(device.as_bytes()))
    }

    pub(crate) fn vaults_dir(&self) -> PathBuf {
        self.account_dir.join("v")
    }

    pub(crate) fn vault_dir(&self, vault: &VaultId) -> PathBuf {
        self.vaults_dir()
            .join(folder_names::dir_name(vault.as_bytes()))
    }

    pub(crate) fn logs_dir(&self, vault: &VaultId) -> PathBuf {
        self.vault_dir(vault).join("log")
    }

    pub(crate) fn log_dir(&self, vault: &VaultId, device: &InstallId) -> PathBuf {
        self.logs_dir(vault)
            .join(folder_names::dir_name(device.as_bytes()))
    }

    pub(crate) fn blobs_dir(&self, vault: &VaultId) -> PathBuf {
        self.vault_dir(vault).join("blobs")
    }

    pub(crate) fn blob_path(&self, vault: &VaultId, id: &AttachmentId) -> PathBuf {
        self.blobs_dir(vault)
            .join(folder_names::blob_name(id.as_bytes()))
    }

    pub(crate) fn marker_path(
        &self,
        vault: &VaultId,
        id: &AttachmentId,
        device: &InstallId,
    ) -> PathBuf {
        self.blobs_dir(vault).join(folder_names::blob_delete_name(
            id.as_bytes(),
            device.as_bytes(),
        ))
    }
}
