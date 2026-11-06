//! Choosing and inspecting a folder (ADR-0043 "folder states").

use std::path::Path;

use mosaic_core::AccountId;

use super::fsutil;
use super::layout::{MARKER_BYTES, MARKER_NAME, README_BYTES, README_NAME};
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
        /// The account directories, ascending.
        accounts: Vec<AccountId>,
    },
    /// A Mosaic marker of another version.
    OtherVersion,
    /// Files that are not Mosaic's. Nothing may be written.
    Foreign,
}

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
    if !root.is_dir() {
        return FolderContents::Missing;
    }
    let Ok(entries) = fsutil::list(root) else {
        return FolderContents::Foreign;
    };
    match fsutil::read_capped(&root.join(MARKER_NAME), 256) {
        Ok(Some(bytes)) if bytes == MARKER_BYTES => FolderContents::Mosaic {
            accounts: list_accounts(root),
        },
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
    match inspect(root) {
        FolderContents::Missing => Err(FolderError::Unavailable),
        FolderContents::Foreign => Err(FolderError::NotEmpty),
        FolderContents::OtherVersion => Err(FolderError::Unsupported),
        FolderContents::Mosaic { .. } => Ok(()),
        FolderContents::Empty => {
            fsutil::write_atomic(root, MARKER_NAME, MARKER_BYTES, false)
                .map_err(|_| FolderError::Io)?;
            fsutil::write_atomic(root, README_NAME, README_BYTES, false)
                .map_err(|_| FolderError::Io)
        }
    }
}

/// The account directories of a folder, ascending (empty if there are none
/// or the folder is unreadable).
#[must_use]
pub fn list_accounts(root: &Path) -> Vec<AccountId> {
    let Ok(entries) = fsutil::list(&root.join("a")) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|e| e.is_dir)
        .filter_map(|e| match folder_names::parse(&e.name) {
            Some(FolderName::Dir(id)) => AccountId::from_bytes(id).ok(),
            _ => None,
        })
        .collect()
}
