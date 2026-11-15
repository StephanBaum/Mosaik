//! The block list of program fill (ADR-0164): programs the user chose not to be offered logins
//! for. It lives next to the state file, is written atomically, and is not under the settings
//! MAC: a blocked program only ever loses an offer, never gains one.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::program_fill::BlockedProgram;

/// The most programs on the list.
pub const MAX_BLOCKED: usize = 200;
/// The file's name in the app data directory.
pub const BLOCKLIST_FILE: &str = "program_fill_blocked.json";
/// The longest key, in bytes (a path of 4096 bytes plus its prefix).
pub const MAX_KEY_BYTES: usize = 4200;
/// The longest label, in characters.
pub const MAX_LABEL_CHARS: usize = 256;
/// A file larger than this reads as empty.
const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// The blocked programs.
#[derive(Debug)]
pub struct BlockList {
    dir: Option<PathBuf>,
    entries: Vec<BlockedProgram>,
}

fn read_capped(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES).read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

fn entry_valid(entry: &BlockedProgram) -> bool {
    !entry.key.is_empty()
        && entry.key.len() <= MAX_KEY_BYTES
        && entry.label.chars().count() <= MAX_LABEL_CHARS
}

impl BlockList {
    /// The list stored in `dir`; an unreadable or malformed file is an empty list.
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        let mut entries: Vec<BlockedProgram> = read_capped(&dir.join(BLOCKLIST_FILE))
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        entries.retain(entry_valid);
        entries.truncate(MAX_BLOCKED);
        Self {
            dir: Some(dir.to_path_buf()),
            entries,
        }
    }

    /// A list that is kept nowhere (tests).
    #[must_use]
    pub const fn memory() -> Self {
        Self {
            dir: None,
            entries: Vec::new(),
        }
    }

    /// Whether `key` is blocked.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.entries.iter().any(|e| e.key == key)
    }

    /// The entries, in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[BlockedProgram] {
        &self.entries
    }

    /// Blocks a program (one entry per key).
    ///
    /// # Errors
    /// `invalidInput` for a key or label over its limit or more than [`MAX_BLOCKED`] entries;
    /// `storeUnavailable` when the file cannot be written (the entry stays for this run).
    pub fn add(&mut self, entry: BlockedProgram) -> IpcResult<()> {
        if !entry_valid(&entry) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        if self.contains(&entry.key) {
            return Ok(());
        }
        if self.entries.len() >= MAX_BLOCKED {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        self.entries.push(entry);
        self.save()
    }

    /// Takes a program off the list (no error for one that is not on it).
    ///
    /// # Errors
    /// `storeUnavailable` when the file cannot be written (the entry is gone for this run).
    pub fn remove(&mut self, key: &str) -> IpcResult<()> {
        let before = self.entries.len();
        self.entries.retain(|e| e.key != key);
        if self.entries.len() == before {
            return Ok(());
        }
        self.save()
    }

    fn save(&self) -> IpcResult<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let bytes = serde_json::to_vec(&self.entries).map_err(|_| IpcError::INTERNAL)?;
        write_atomic(&dir.join(BLOCKLIST_FILE), &bytes)
            .map_err(|_| IpcError::new(ErrorCode::StoreUnavailable))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    match fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let written = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str) -> BlockedProgram {
        BlockedProgram {
            key: key.to_owned(),
            label: "Tool".to_owned(),
        }
    }

    #[test]
    fn block_list_survives_a_restart_and_ignores_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut list = BlockList::in_dir(dir.path());
        assert_eq!(list.entries().len(), 0);
        list.add(entry("path:c:\\tools\\a.exe")).unwrap();
        list.add(entry("package:x")).unwrap();
        list.remove("package:x").unwrap();
        let again = BlockList::in_dir(dir.path());
        assert!(again.contains("path:c:\\tools\\a.exe"));
        assert!(!again.contains("package:x"));
        assert_eq!(again.entries().len(), 1);

        std::fs::write(dir.path().join(BLOCKLIST_FILE), b"\xff garbage {").unwrap();
        assert_eq!(BlockList::in_dir(dir.path()).entries().len(), 0);
        std::fs::write(
            dir.path().join(BLOCKLIST_FILE),
            vec![b' '; usize::try_from(MAX_FILE_BYTES).unwrap() + 1],
        )
        .unwrap();
        assert_eq!(BlockList::in_dir(dir.path()).entries().len(), 0);
    }

    #[test]
    fn block_list_dedups_and_caps() {
        let mut list = BlockList::memory();
        list.add(entry("a")).unwrap();
        list.add(entry("a")).unwrap();
        assert_eq!(list.entries().len(), 1);
        for i in 1..MAX_BLOCKED {
            list.add(entry(&format!("k{i}"))).unwrap();
        }
        assert_eq!(list.entries().len(), MAX_BLOCKED);
        assert_eq!(
            list.add(entry("one more")).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        let mut list = BlockList::memory();
        assert_eq!(
            list.add(entry(&"k".repeat(MAX_KEY_BYTES + 1)))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            list.add(BlockedProgram {
                key: "k".into(),
                label: "x".repeat(MAX_LABEL_CHARS + 1)
            })
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert!(list.add(entry("")).is_err());
    }
}
