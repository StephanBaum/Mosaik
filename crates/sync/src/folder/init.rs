//! Choosing and inspecting a folder (ADR-0043 "folder states").

use std::path::Path;

use mosaic_core::AccountId;

use super::layout::{
    Layout, MARKER_BYTES, MARKER_NAME, README_BYTES, README_NAME, accounts_dir, marker_path,
};
use super::store::{Entry, FolderStore, LocalStore, RelPath};
use crate::transport::TransportError;
use crate::wire::folder_names::{self, FolderName};

/// Why a folder cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FolderError {
    /// The folder does not exist or cannot be read.
    #[error("folder unavailable")]
    Unavailable,
    /// The folder has files and no valid Mosaic marker: nothing is written
    /// (`syncFolderNotEmpty`).
    #[error("folder is not empty and not a Mosaic folder")]
    NotEmpty,
    /// A Mosaic folder of a version this build does not know.
    #[error("unsupported folder version")]
    Unsupported,
    /// An I/O failure.
    #[error("I/O error")]
    Io,
}

impl From<FolderError> for TransportError {
    fn from(e: FolderError) -> Self {
        match e {
            FolderError::Unavailable | FolderError::NotEmpty => Self::FolderUnavailable,
            FolderError::Unsupported => Self::Incompatible,
            FolderError::Io => Self::Io,
        }
    }
}

/// What a folder holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderContents {
    /// The path does not exist or is not a directory.
    Missing,
    /// Nothing in it (apart from OS metadata files): [`init`] may use it.
    Empty,
    /// A Mosaic folder with these accounts.
    Mosaic {
        /// The account directories, ascending, at most
        /// [`MAX_LISTED_ACCOUNTS`] (a hostile peer can plant any number).
        accounts: Vec<AccountId>,
        /// More account directories exist than `accounts` lists.
        truncated: bool,
    },
    /// A Mosaic marker of another version.
    OtherVersion,
    /// Files that are not Mosaic's. Nothing may be written.
    Foreign,
}

/// How many accounts [`inspect`] lists (RT-desktop info: a folder shared with
/// a hostile peer can hold any number of directories).
pub const MAX_LISTED_ACCOUNTS: usize = 64;

/// File names an OS or sync client puts into every folder; they do not make
/// a folder non-empty.
fn is_os_metadata(name: &str) -> bool {
    matches!(
        name,
        "desktop.ini" | ".DS_Store" | "Thumbs.db" | ".dropbox" | ".dropbox.attr"
    )
}

/// Looks into `root` without writing anything.
#[must_use]
pub fn inspect(root: &Path) -> FolderContents {
    inspect_in(&LocalStore::new(root))
}

/// As [`inspect`], over any [`FolderStore`].
#[must_use]
pub fn inspect_in(store: &dyn FolderStore) -> FolderContents {
    let root = RelPath::root();
    if !store.is_real_dir(&root) {
        return FolderContents::Missing;
    }
    let Ok(entries) = store.list(&root) else {
        return FolderContents::Foreign;
    };
    match store.read_capped(&marker_path(), 256) {
        Ok(Some(bytes)) if bytes == MARKER_BYTES => {
            let mut accounts = list_accounts_capped(store, MAX_LISTED_ACCOUNTS + 1);
            let truncated = accounts.len() > MAX_LISTED_ACCOUNTS;
            accounts.truncate(MAX_LISTED_ACCOUNTS);
            FolderContents::Mosaic {
                accounts,
                truncated,
            }
        }
        Ok(Some(bytes)) => {
            if String::from_utf8_lossy(&bytes).contains("\"mosaic-folder\"") {
                FolderContents::OtherVersion
            } else {
                FolderContents::Foreign
            }
        }
        Ok(None) => {
            let only_metadata = entries
                .iter()
                .all(|e| e.name.to_str().is_some_and(is_os_metadata));
            if only_metadata {
                FolderContents::Empty
            } else {
                FolderContents::Foreign
            }
        }
        Err(_) => FolderContents::Foreign,
    }
}

/// Prepares an empty folder (or accepts a Mosaic one): writes the marker and
/// the readme if the folder is empty, and nothing otherwise.
///
/// # Errors
/// [`FolderError::Unavailable`], [`FolderError::NotEmpty`],
/// [`FolderError::Unsupported`], [`FolderError::Io`].
pub fn init(root: &Path) -> Result<(), FolderError> {
    init_in(&LocalStore::new(root))
}

/// As [`init`], over any [`FolderStore`].
///
/// # Errors
/// As [`init`].
pub fn init_in(store: &dyn FolderStore) -> Result<(), FolderError> {
    match inspect_in(store) {
        FolderContents::Missing => Err(FolderError::Unavailable),
        FolderContents::Foreign => Err(FolderError::NotEmpty),
        FolderContents::OtherVersion => Err(FolderError::Unsupported),
        FolderContents::Mosaic { .. } => Ok(()),
        FolderContents::Empty => {
            let root = RelPath::root();
            store
                .write_atomic(&root, MARKER_NAME, MARKER_BYTES, false)
                .map_err(|_| FolderError::Io)?;
            store
                .write_atomic(&root, README_NAME, README_BYTES, false)
                .map_err(|_| FolderError::Io)
        }
    }
}

/// The account directories of a folder, ascending (empty if there are none
/// or the folder is unreadable). Not capped: reads the whole `a` directory.
/// [`inspect`] lists with [`list_accounts_capped`] instead.
#[must_use]
pub fn list_accounts(root: &Path) -> Vec<AccountId> {
    list_accounts_in(&LocalStore::new(root))
}

/// As [`list_accounts`], over any [`FolderStore`].
#[must_use]
pub fn list_accounts_in(store: &dyn FolderStore) -> Vec<AccountId> {
    let Ok(entries) = store.list(&accounts_dir()) else {
        return Vec::new();
    };
    entries.iter().filter_map(account_of).collect()
}

/// The account of a directory entry, if it is an account directory.
fn account_of(e: &Entry) -> Option<AccountId> {
    if !e.is_dir() {
        return None;
    }
    match folder_names::parse(&e.name) {
        Some(FolderName::Dir(id)) => AccountId::from_bytes(id).ok(),
        _ => None,
    }
}

/// Up to `limit` account directories, ascending among the ones found: the read
/// stops once `limit` are found (or after a bounded number of directory items),
/// so a hostile folder cannot make this read or sort them all. With more than
/// `limit` accounts present, which of them are listed is up to the file system.
fn list_accounts_capped(store: &dyn FolderStore, limit: usize) -> Vec<AccountId> {
    store
        .list_capped(&accounts_dir(), limit, &|e| account_of(e).is_some())
        .map(|entries| entries.iter().filter_map(account_of).collect())
        .unwrap_or_default()
}

/// Whether the folder has a real (not symlinked) directory for `account`:
/// one `stat`, no listing, so it holds however many accounts the folder has.
#[must_use]
pub fn has_account(root: &Path, account: &AccountId) -> bool {
    has_account_in(&LocalStore::new(root), account)
}

/// As [`has_account`], over any [`FolderStore`].
#[must_use]
pub fn has_account_in(store: &dyn FolderStore, account: &AccountId) -> bool {
    store.is_real_dir(&Layout::new(account).dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The account directory on disk.
    fn account_path(root: &Path, account: &AccountId) -> std::path::PathBuf {
        let mut path = root.to_path_buf();
        path.extend(Layout::new(account).dir().components());
        path
    }

    fn account(n: u8) -> AccountId {
        let mut bytes = [7u8; 16];
        bytes[15] = n;
        AccountId::from_bytes(bytes).unwrap()
    }

    fn mosaic_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        dir
    }

    #[test]
    fn inspect_lists_at_most_the_cap_and_says_so() {
        let dir = mosaic_folder();
        let count = u8::try_from(MAX_LISTED_ACCOUNTS + 6).unwrap();
        for n in 0..count {
            std::fs::create_dir_all(account_path(dir.path(), &account(n))).unwrap();
        }
        let FolderContents::Mosaic {
            accounts,
            truncated,
        } = inspect(dir.path())
        else {
            panic!("a Mosaic folder");
        };
        assert_eq!(accounts.len(), MAX_LISTED_ACCOUNTS);
        assert!(truncated);
        assert!(
            accounts
                .windows(2)
                .all(|w| w[0].as_bytes() < w[1].as_bytes())
        );
        // Exactly the cap is not truncated.
        let dir = mosaic_folder();
        for n in 0..u8::try_from(MAX_LISTED_ACCOUNTS).unwrap() {
            std::fs::create_dir_all(account_path(dir.path(), &account(n))).unwrap();
        }
        assert!(matches!(
            inspect(dir.path()),
            FolderContents::Mosaic { truncated: false, ref accounts } if accounts.len() == MAX_LISTED_ACCOUNTS
        ));
    }

    #[test]
    fn has_account_is_a_direct_check_beyond_the_cap() {
        let dir = mosaic_folder();
        for n in 0..=u8::try_from(MAX_LISTED_ACCOUNTS + 6).unwrap() {
            std::fs::create_dir_all(account_path(dir.path(), &account(n))).unwrap();
        }
        let last = account(u8::try_from(MAX_LISTED_ACCOUNTS + 6).unwrap());
        assert!(has_account(dir.path(), &last));
        assert!(!has_account(dir.path(), &account(200)));
        // A plain file under the account's name is not an account.
        std::fs::write(account_path(dir.path(), &account(201)), b"x").unwrap();
        assert!(!has_account(dir.path(), &account(201)));
    }
}
