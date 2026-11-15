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

/// Overwrites the file once with zeros, flushes it to disk and deletes it.
///
/// Refuses (and leaves the file as it was) when `path` is not a regular
/// file (a symlink, a directory), when it cannot be opened for writing
/// (read-only, in use, a network share that refuses) or when it cannot be
/// removed afterwards.
///
/// # Errors
/// `InvalidInput` for a non-regular file; the underlying I/O error otherwise.
pub fn overwrite_and_delete(path: &Path) -> io::Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    // Opened for writing without truncation: a read-only file fails here,
    // before anything is changed.
    let mut file = OpenOptions::new().write(true).open(path)?;
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

    #[cfg(unix)]
    #[test]
    fn overwrite_and_delete_refuses_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = file_in(tmp.path(), "target.csv", b"keep me");
        let link = tmp.path().join("link.csv");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let error = overwrite_and_delete(&link).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(&target).unwrap(), b"keep me");
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
