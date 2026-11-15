//! Removing the plaintext export after a successful import, and telling the
//! user when the file sits in a folder a cloud service syncs.
//!
//! The overwrite is best effort and is never described as secure erasure:
//! on an SSD, a journaling or copy-on-write file system, or a synced folder,
//! other copies of the bytes can survive (the UI says so). Only a path the
//! shell itself holds behind a token is ever deleted (`commands::import`).

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write as _};
use std::path::{Component, Path, PathBuf};

use crate::contract::import::ImportSyncedFolder;

const CHUNK: usize = 64 * 1024;

/// `O_NOFOLLOW` (std has no portable constant and `libc` is not a dependency
/// of this crate). Values from the platform headers: 0x100 on macOS and the
/// BSDs, 0o100000 on Linux arm and aarch64, 0o400000 on the other Linux
/// architectures.
#[cfg(unix)]
const O_NOFOLLOW: i32 = if cfg!(any(target_os = "linux", target_os = "android")) {
    if cfg!(any(target_arch = "arm", target_arch = "aarch64")) {
        0o100_000
    } else {
        0o400_000
    }
} else {
    0x100
};

/// `FILE_FLAG_OPEN_REPARSE_POINT`: open a link itself instead of its target.
#[cfg(windows)]
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
/// `FILE_ATTRIBUTE_REPARSE_POINT`.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

/// Opens `path` for writing without truncation and without following a
/// symlink or reparse point, so the check below runs on what was opened.
fn open_no_follow(path: &Path) -> io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path)
}

/// Whether the opened handle is a plain file (not a link or reparse point).
fn handle_is_regular(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return false;
        }
    }
    meta.file_type().is_file()
}

/// Overwrites the file once with zeros, flushes it to disk and deletes it.
///
/// The file is opened first, without following links, and checked on the
/// handle: there is no window between a check and the open in which the path
/// can be swapped for a link. The bytes are written through that handle.
///
/// Refuses (and leaves the file as it was) when `path` is not a regular
/// file (a symlink, a directory), when it cannot be opened for writing
/// (read-only, in use, a network share that refuses) or when it cannot be
/// removed afterwards.
///
/// # Errors
/// `InvalidInput` for a non-regular file; the underlying I/O error otherwise.
pub fn overwrite_and_delete(path: &Path) -> io::Result<()> {
    // A directory cannot be opened for writing on Windows: look first only to
    // report it as the wrong kind. Nothing is trusted from this look.
    if std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let mut file = open_no_follow(path).map_err(|e| {
        // A symlink under O_NOFOLLOW fails with ELOOP (Unix): a refusal.
        if is_symlink_refusal(&e) {
            io::Error::new(io::ErrorKind::InvalidInput, "not a regular file")
        } else {
            e
        }
    })?;
    let meta = file.metadata()?;
    if !handle_is_regular(&meta) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let zeros = vec![0_u8; CHUNK].into_boxed_slice();
    let mut left = meta.len();
    while left > 0 {
        let n = usize::try_from(left).map_or(CHUNK, |l| l.min(CHUNK));
        file.write_all(zeros.get(..n).unwrap_or(&zeros))?;
        left = left.saturating_sub(n as u64);
    }
    file.sync_all()?;
    drop(file);
    std::fs::remove_file(path)
}

/// `ELOOP` (40 on Linux, 62 on macOS and the BSDs): what `O_NOFOLLOW` reports
/// for a symlink.
fn is_symlink_refusal(error: &io::Error) -> bool {
    cfg!(unix)
        && error.raw_os_error().is_some_and(|code| {
            code == if cfg!(any(target_os = "linux", target_os = "android")) {
                40
            } else {
                62
            }
        })
}

/// The sync roots to compare a file against: the environment's `OneDrive`
/// folders (Windows Known Folder Move puts Downloads inside them), and the
/// usual folders of Dropbox, Google Drive, Box and `iCloud` under `home`.
#[must_use]
pub fn sync_roots(
    home: Option<&Path>,
    var: &dyn Fn(&str) -> Option<OsString>,
) -> Vec<(ImportSyncedFolder, PathBuf)> {
    let mut roots = Vec::new();
    for name in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(dir) = var(name).filter(|v| !v.is_empty()) {
            roots.push((ImportSyncedFolder::OneDrive, PathBuf::from(dir)));
        }
    }
    let Some(home) = home else {
        return roots;
    };
    for (service, dir) in [
        (ImportSyncedFolder::Dropbox, "Dropbox"),
        (ImportSyncedFolder::GoogleDrive, "Google Drive"),
        (ImportSyncedFolder::GoogleDrive, "My Drive"),
        (ImportSyncedFolder::Box, "Box"),
        (ImportSyncedFolder::Icloud, "Library/Mobile Documents"),
    ] {
        roots.push((service, home.join(dir)));
    }
    let storage = home.join("Library").join("CloudStorage");
    if let Ok(entries) = std::fs::read_dir(&storage) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(service) = name.to_str().and_then(cloud_storage_service) {
                roots.push((service, entry.path()));
            }
        }
    }
    roots
}

/// The service behind a `~/Library/CloudStorage/<name>` folder.
#[must_use]
pub fn cloud_storage_service(dir_name: &str) -> Option<ImportSyncedFolder> {
    [
        ("OneDrive", ImportSyncedFolder::OneDrive),
        ("Dropbox", ImportSyncedFolder::Dropbox),
        ("GoogleDrive", ImportSyncedFolder::GoogleDrive),
        ("Box", ImportSyncedFolder::Box),
        ("iCloud", ImportSyncedFolder::Icloud),
    ]
    .into_iter()
    .find(|(prefix, _)| dir_name.starts_with(prefix))
    .map(|(_, service)| service)
}

/// The path's components, lower-cased on Windows (case-insensitive paths).
fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| {
            let text = c.as_os_str().to_string_lossy().into_owned();
            if cfg!(windows) {
                text.to_lowercase()
            } else {
                text
            }
        })
        .collect()
}

fn inside(file: &Path, root: &Path) -> bool {
    let (file, root) = (parts(file), parts(root));
    !root.is_empty() && file.starts_with(&root)
}

/// The service whose sync folder holds `file`, if any. Compared both as
/// given and resolved (links, short names).
#[must_use]
pub fn synced_folder(
    file: &Path,
    roots: &[(ImportSyncedFolder, PathBuf)],
) -> Option<ImportSyncedFolder> {
    let resolved = file.canonicalize().ok();
    roots.iter().find_map(|(service, root)| {
        let root_resolved = root.canonicalize().ok();
        let hit = inside(file, root)
            || match (&resolved, &root_resolved) {
                (Some(f), Some(r)) => inside(f, r),
                (Some(f), None) => inside(f, root),
                _ => false,
            };
        hit.then_some(*service)
    })
}

/// [`synced_folder`] against this machine's sync roots.
#[must_use]
pub fn synced_folder_of(file: &Path) -> Option<ImportSyncedFolder> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let roots = sync_roots(home.as_deref(), &|name| std::env::var_os(name));
    synced_folder(file, &roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_in(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn overwrite_and_delete_removes_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file_in(tmp.path(), "export.csv", &vec![7_u8; 3 * CHUNK + 5]);
        overwrite_and_delete(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn overwrite_and_delete_refuses_read_only_and_keeps_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = file_in(tmp.path(), "export.csv", b"url,username\n");
        let original = std::fs::metadata(&path).unwrap().permissions();
        let mut read_only = original.clone();
        read_only.set_readonly(true);
        std::fs::set_permissions(&path, read_only).unwrap();
        let result = overwrite_and_delete(&path);
        assert!(result.is_err());
        assert!(path.exists());
        std::fs::set_permissions(&path, original).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"url,username\n");
    }

    #[test]
    fn overwrite_and_delete_refuses_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let error = overwrite_and_delete(tmp.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(tmp.path().exists());
    }

    #[test]
    fn overwrite_and_delete_refuses_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = file_in(tmp.path(), "target.csv", b"keep me");
        let link = tmp.path().join("link.csv");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&target, &link);
        if made.is_err() {
            // Creating symlinks needs a privilege on Windows; nothing to test.
            return;
        }
        let error = overwrite_and_delete(&link).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(&target).unwrap(), b"keep me");
        assert!(link.symlink_metadata().is_ok());
    }

    #[test]
    fn onedrive_known_folder_move_is_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let downloads = tmp.path().join("OneDrive").join("Downloads");
        std::fs::create_dir_all(&downloads).unwrap();
        let file = file_in(&downloads, "x.csv", b"a");
        let root = tmp.path().join("OneDrive");
        let var = |name: &str| (name == "OneDrive").then(|| root.clone().into_os_string());
        let roots = sync_roots(None, &var);
        assert_eq!(
            synced_folder(&file, &roots),
            Some(ImportSyncedFolder::OneDrive)
        );
    }

    #[test]
    fn cloud_storage_names_are_classified() {
        assert_eq!(
            cloud_storage_service("OneDrive-Personal"),
            Some(ImportSyncedFolder::OneDrive)
        );
        assert_eq!(
            cloud_storage_service("GoogleDrive-a@b.example"),
            Some(ImportSyncedFolder::GoogleDrive)
        );
        assert_eq!(cloud_storage_service("Other"), None);
    }

    #[test]
    fn files_outside_sync_roots_are_not_synced() {
        let tmp = tempfile::tempdir().unwrap();
        let file = file_in(tmp.path(), "x.csv", b"a");
        let elsewhere = tmp.path().join("OneDrive");
        let var = |name: &str| (name == "OneDrive").then(|| elsewhere.clone().into_os_string());
        let roots = sync_roots(Some(&tmp.path().join("home")), &var);
        assert_eq!(synced_folder(&file, &roots), None);
    }

    #[test]
    fn dropbox_and_icloud_folders_are_found_under_home() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("Dropbox").join("sub").join("x.csv");
        let roots = sync_roots(Some(tmp.path()), &|_| None);
        assert_eq!(
            synced_folder(&file, &roots),
            Some(ImportSyncedFolder::Dropbox)
        );
        let file = tmp
            .path()
            .join("Library/Mobile Documents/com~apple~CloudDocs/x.csv");
        assert_eq!(
            synced_folder(&file, &roots),
            Some(ImportSyncedFolder::Icloud)
        );
    }
}
