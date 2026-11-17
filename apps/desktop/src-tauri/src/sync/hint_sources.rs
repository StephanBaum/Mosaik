//! Reads the provider clients' configuration for the folder setup code hint
//! (ADR-0161): Dropbox's `info.json`, the `OneDrive` roots and the Nextcloud
//! client's `nextcloud.cfg`. The bytes go to `mosaic_sync::folder::hint`, which
//! interprets them. Every source is optional and best effort: a source that is
//! missing, unreadable, not a regular file or larger than
//! [`MAX_HINT_SOURCE`] counts as absent.

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use mosaic_sync::folder::hint::{HintSources, MAX_HINT_SOURCE};

/// The most entries of `~/Library/CloudStorage` that are looked at.
#[cfg(not(windows))]
const MAX_CLOUD_STORAGE_ENTRIES: usize = 64;

/// The bytes of a regular file of at most [`MAX_HINT_SOURCE`] bytes.
fn read_small(path: &Path) -> Option<Vec<u8>> {
    if !std::fs::metadata(path).ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(u64::try_from(MAX_HINT_SOURCE).ok()?.checked_add(1)?)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_HINT_SOURCE).then_some(bytes)
}

/// The first of `candidates` that reads.
fn first_readable(candidates: &[PathBuf]) -> Option<Vec<u8>> {
    candidates.iter().find_map(|p| read_small(p))
}

#[cfg(windows)]
fn env_path(name: &str, tail: &[&str]) -> Option<PathBuf> {
    let base = std::env::var_os(name).filter(|v| !v.is_empty())?;
    let mut path = PathBuf::from(base);
    path.extend(tail);
    Some(path)
}

/// Gathers what the desktop knows about the provider clients on this
/// computer.
#[must_use]
pub(crate) fn gather_hint_sources() -> HintSources {
    platform_sources()
}

#[cfg(windows)]
fn platform_sources() -> HintSources {
    let dropbox: Vec<PathBuf> = [
        env_path("APPDATA", &["Dropbox", "info.json"]),
        env_path("LOCALAPPDATA", &["Dropbox", "info.json"]),
    ]
    .into_iter()
    .flatten()
    .collect();
    let onedrive_roots = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .into_iter()
        .filter_map(|name| env_path(name, &[]))
        .collect();
    let nextcloud: Vec<PathBuf> = env_path("APPDATA", &["Nextcloud", "nextcloud.cfg"])
        .into_iter()
        .collect();
    HintSources {
        dropbox_info_json: first_readable(&dropbox),
        onedrive_roots,
        nextcloud_cfg: first_readable(&nextcloud),
    }
}

#[cfg(not(windows))]
fn platform_sources() -> HintSources {
    let Some(home) = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
    else {
        return HintSources::default();
    };
    let dropbox = [home.join(".dropbox").join("info.json")];
    let nextcloud = [
        home.join(".config").join("Nextcloud").join("nextcloud.cfg"),
        home.join("Library")
            .join("Preferences")
            .join("Nextcloud")
            .join("nextcloud.cfg"),
    ];
    HintSources {
        dropbox_info_json: first_readable(&dropbox),
        onedrive_roots: onedrive_cloud_storage(&home.join("Library").join("CloudStorage")),
        nextcloud_cfg: first_readable(&nextcloud),
    }
}

/// The `OneDrive-*` folders of macOS's `CloudStorage`.
#[cfg(not(windows))]
fn onedrive_cloud_storage(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .take(MAX_CLOUD_STORAGE_ENTRIES)
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("OneDrive"))
        .map(|e| e.path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_small_reads_a_small_file_and_refuses_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("a.json");
        std::fs::write(&small, b"{}").unwrap();
        assert_eq!(read_small(&small).as_deref(), Some(b"{}".as_slice()));
        let big = dir.path().join("big.cfg");
        std::fs::write(&big, vec![b'x'; MAX_HINT_SOURCE + 1]).unwrap();
        assert_eq!(read_small(&big), None);
        let exact = dir.path().join("exact.cfg");
        std::fs::write(&exact, vec![b'x'; MAX_HINT_SOURCE]).unwrap();
        assert_eq!(read_small(&exact).map(|b| b.len()), Some(MAX_HINT_SOURCE));
        // A directory and a missing file are absent.
        assert_eq!(read_small(dir.path()), None);
        assert_eq!(read_small(&dir.path().join("missing")), None);
    }
}
