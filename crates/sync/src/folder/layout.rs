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

use mosaic_core::{AccountId, AttachmentId, InstallId, VaultId};

use super::store::RelPath;
use crate::wire::folder_names;

/// The marker file's name.
pub(crate) const MARKER_NAME: &str = "mosaic-sync.json";
/// The exact bytes of the marker file.
pub(crate) const MARKER_BYTES: &[u8] = br#"{"format":"mosaic-folder","version":1}"#;
/// The readme's name.
pub(crate) const README_NAME: &str = "README.txt";
/// The exact bytes of the readme.
pub(crate) const README_BYTES: &[u8] = b"Managed by Mosaic. Do not edit, move or delete files here.
";
/// The canonical checkpoint file name.
pub(crate) const HEAD_NAME: &str = "head.mck";
/// The device info file name.
pub(crate) const INFO_NAME: &str = "info.mdev";
/// The directory of all accounts.
pub(crate) const ACCOUNTS_DIR: &str = "a";

/// The paths of one account in a folder, relative to the folder's root.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    account_dir: RelPath,
}

impl Layout {
    pub(crate) fn new(account: &AccountId) -> Self {
        Self {
            account_dir: accounts_dir().known(&folder_names::dir_name(account.as_bytes())),
        }
    }

    /// The account's directory.
    pub(crate) fn dir(&self) -> RelPath {
        self.account_dir.clone()
    }

    pub(crate) fn account_files(&self) -> RelPath {
        self.account_dir.known("account")
    }

    pub(crate) fn devices_dir(&self) -> RelPath {
        self.account_dir.known("d")
    }

    pub(crate) fn device_dir(&self, device: &InstallId) -> RelPath {
        self.devices_dir()
            .known(&folder_names::dir_name(device.as_bytes()))
    }

    pub(crate) fn vaults_dir(&self) -> RelPath {
        self.account_dir.known("v")
    }

    pub(crate) fn vault_dir(&self, vault: &VaultId) -> RelPath {
        self.vaults_dir()
            .known(&folder_names::dir_name(vault.as_bytes()))
    }

    pub(crate) fn logs_dir(&self, vault: &VaultId) -> RelPath {
        self.vault_dir(vault).known("log")
    }

    pub(crate) fn log_dir(&self, vault: &VaultId, device: &InstallId) -> RelPath {
        self.logs_dir(vault)
            .known(&folder_names::dir_name(device.as_bytes()))
    }

    pub(crate) fn blobs_dir(&self, vault: &VaultId) -> RelPath {
        self.vault_dir(vault).known("blobs")
    }

    pub(crate) fn blob_path(&self, vault: &VaultId, id: &AttachmentId) -> RelPath {
        self.blobs_dir(vault)
            .known(&folder_names::blob_name(id.as_bytes()))
    }

    pub(crate) fn marker_path(
        &self,
        vault: &VaultId,
        id: &AttachmentId,
        device: &InstallId,
    ) -> RelPath {
        self.blobs_dir(vault).known(&folder_names::blob_delete_name(
            id.as_bytes(),
            device.as_bytes(),
        ))
    }
}

/// The directory that holds the account directories (`a`).
pub(crate) fn accounts_dir() -> RelPath {
    RelPath::root().known(ACCOUNTS_DIR)
}

/// The marker file.
pub(crate) fn marker_path() -> RelPath {
    RelPath::root().known(MARKER_NAME)
}
