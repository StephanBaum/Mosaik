//! File-system helpers of the folder transport (`std::fs` only).
//!
//! Every write goes through a temp file in the target directory
//! (`.<16 hex>.mtmp`, created with `create_new`), `write_all`, `sync_all`
//! and a rename, so a sync client never sees a half-written final name from
//! this process. Write-once names are checked for absence first.

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::Path;
use std::time::SystemTime;

use crate::transport::TransportError;

/// The extension of a temp file.
pub(crate) const TEMP_EXTENSION: &str = "mtmp";

/// What a directory listing says about an entry.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) name: OsString,
    pub(crate) is_dir: bool,
    pub(crate) len: u64,
    pub(crate) modified: Option<SystemTime>,
}

impl Entry {
    /// Milliseconds since the Unix epoch of the last modification (0 if
    /// unknown).
    pub(crate) fn modified_ms(&self) -> u64 {
        self.modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// Maps an I/O error to the transport's error. Nothing of the error (it
/// carries paths) is kept.
pub(crate) fn io_error(e: &std::io::Error) -> TransportError {
    match e.kind() {
        ErrorKind::NotFound => TransportError::NotFound,
        _ => TransportError::Io,
    }
}

/// The entry of one directory item; `None` for one that vanished while it was
/// listed (a sync client at work) and for a symlink. `DirEntry::metadata` does
/// not follow symlinks; Mosaic never writes one, so a link (anyone with write
/// access to the folder can plant one) is not an entry at all.
fn entry_of(item: &fs::DirEntry) -> Option<Entry> {
    let meta = item.metadata().ok()?;
    if meta.file_type().is_symlink() {
        return None;
    }
    Some(Entry {
        name: item.file_name(),
        is_dir: meta.is_dir(),
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// The entries of a directory; a missing directory has none.
pub(crate) fn list(dir: &Path) -> Result<Vec<Entry>, TransportError> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(&e)),
    };
    let mut out: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|i| entry_of(&i))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// At most `max_scanned` directory items are looked at by [`list_capped`], so a
/// folder with millions of planted entries costs bounded work.
pub(crate) const MAX_SCANNED: usize = 4096;

/// The entries of a directory that `keep` accepts, sorted by name, from a read
/// that stops as soon as `limit` accepted entries are found or [`MAX_SCANNED`]
/// items have been looked at (no full read, no full sort). Which entries come
/// first is the file system's order, so when the directory holds more than
/// `limit` accepted entries the listed ones are an arbitrary subset. A missing
/// directory has none.
pub(crate) fn list_capped(
    dir: &Path,
    limit: usize,
    keep: impl Fn(&Entry) -> bool,
) -> Result<Vec<Entry>, TransportError> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(&e)),
    };
    let mut out = Vec::new();
    for item in read.take(MAX_SCANNED) {
        if out.len() >= limit {
            break;
        }
        let Some(entry) = item.ok().and_then(|i| entry_of(&i)) else {
            continue;
        };
        if keep(&entry) {
            out.push(entry);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Whether `path` is a real directory: a symlink to one is not (as in [`list`]).
pub(crate) fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

/// Reads a regular file of at most `max` bytes; `None` if it is missing or
/// not a regular file (a symlink, a directory, a device or a pipe is never
/// followed or opened), `Err(Protocol)` if it is longer. The read itself is
/// capped at `max + 1` bytes, so a file that grows or is swapped between the
/// check and the read cannot make this allocate more.
pub(crate) fn read_capped(path: &Path, max: u64) -> Result<Option<Vec<u8>>, TransportError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(&e)),
    };
    if !meta.is_file() {
        return Ok(None);
    }
    if meta.len() > max {
        return Err(TransportError::Protocol);
    }
    let file = match open_regular(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(&e)),
    };
    // The path may have been swapped between the check and the open.
    match file.metadata() {
        Ok(m) if m.is_file() => {}
        Ok(_) => return Ok(None),
        Err(e) => return Err(io_error(&e)),
    }
    let mut bytes = Vec::new();
    match file.take(max.saturating_add(1)).read_to_end(&mut bytes) {
        Ok(_) if u64::try_from(bytes.len()).is_ok_and(|n| n <= max) => Ok(Some(bytes)),
        Ok(_) => Err(TransportError::Protocol),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(&e)),
    }
}

/// Opens a file for reading. On Unix with `O_NONBLOCK`, so that a FIFO that
/// replaced the file does not block the open (the caller checks that the
/// opened file is a regular file), and `O_NOFOLLOW`, so that a symlink put in
/// place of the file between the check and the open is refused, not followed.
fn open_regular(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    options.open(path)
}

/// Removes whatever sits at `path`: a file, a directory with its content, or
/// a symlink (only the link itself, never its target). Used on names in the
/// own log that this device never wrote.
pub(crate) fn remove_squatter(path: &Path) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if meta.file_type().is_symlink() {
        // A link to a directory is a directory entry on Windows.
        if fs::remove_file(path).is_err() {
            let _ = fs::remove_dir(path);
        }
    } else if meta.is_dir() {
        // `remove_dir_all` does not follow links inside.
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

/// Whether a file or directory exists.
pub(crate) fn exists(path: &Path) -> bool {
    path.try_exists().unwrap_or(false)
}

fn random_hex() -> Result<String, TransportError> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|_| TransportError::Io)?;
    Ok(crate::wire::hex(&b))
}

/// Writes `bytes` to `dir/name` through a temp file and a rename. With
/// `replace == false` the name must be absent (`Io` otherwise): write-once
/// names are never overwritten. The directory is created.
pub(crate) fn write_atomic(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    replace: bool,
) -> Result<(), TransportError> {
    fs::create_dir_all(dir).map_err(|e| io_error(&e))?;
    let target = dir.join(name);
    if !replace && exists(&target) {
        return Err(TransportError::Io);
    }
    let tmp = dir.join(format!(".{}.{TEMP_EXTENSION}", random_hex()?));
    let written = (|| -> std::io::Result<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(io_error(&e));
    }
    if !replace && exists(&target) {
        let _ = fs::remove_file(&tmp);
        return Err(TransportError::Io);
    }
    fs::rename(&tmp, &target).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        io_error(&e)
    })
}

/// Removes an empty directory; a missing or non-empty one is left alone.
pub(crate) fn remove_empty_dir(path: &Path) {
    let _ = fs::remove_dir(path);
}

/// Removes a file; a missing file is fine.
pub(crate) fn remove(path: &Path) -> Result<(), TransportError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_capped_stops_at_the_limit_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            fs::create_dir(dir.path().join(format!("d{i:02}"))).unwrap();
            fs::write(dir.path().join(format!("f{i:02}")), b"x").unwrap();
        }
        let dirs = list_capped(dir.path(), 5, |e| e.is_dir).unwrap();
        assert_eq!(dirs.len(), 5);
        assert!(dirs.iter().all(|e| e.is_dir));
        assert!(dirs.windows(2).all(|w| w[0].name < w[1].name), "sorted");
        // Under the limit, everything that is kept is listed.
        assert_eq!(
            list_capped(dir.path(), 100, |e| e.is_dir).unwrap().len(),
            20
        );
        assert!(
            list_capped(&dir.path().join("missing"), 5, |_| true)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn list_capped_scans_a_bounded_number_of_items() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..(MAX_SCANNED + 50) {
            fs::write(dir.path().join(format!("f{i}")), b"").unwrap();
        }
        // Nothing is kept, so the read runs to its scan bound and no further.
        let seen = std::cell::Cell::new(0usize);
        let kept = list_capped(dir.path(), 1, |_| {
            seen.set(seen.get() + 1);
            false
        })
        .unwrap();
        assert!(kept.is_empty());
        assert_eq!(seen.get(), MAX_SCANNED);
    }

    #[test]
    fn is_real_dir_is_true_for_directories_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, b"x").unwrap();
        assert!(is_real_dir(dir.path()));
        assert!(!is_real_dir(&file));
        assert!(!is_real_dir(&dir.path().join("missing")));
    }

    #[test]
    fn read_capped_reads_only_small_regular_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, b"12345").unwrap();
        assert_eq!(read_capped(&file, 5).unwrap(), Some(b"12345".to_vec()));
        assert_eq!(read_capped(&file, 4), Err(TransportError::Protocol));
        assert_eq!(read_capped(&dir.path().join("missing"), 5).unwrap(), None);
        // A directory is not a file.
        assert_eq!(read_capped(dir.path(), 5).unwrap(), None);
    }

    /// A link is neither listed nor read, however large its target.
    #[cfg(unix)]
    #[test]
    fn symlinks_are_neither_listed_nor_read() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::write(&target, b"secret").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let dev = dir.path().join("dev-zero");
        std::os::unix::fs::symlink("/dev/zero", &dev).unwrap();
        assert_eq!(read_capped(&link, 100).unwrap(), None);
        assert_eq!(read_capped(&dev, 100).unwrap(), None);
        let names: Vec<_> = list(dir.path())
            .unwrap()
            .into_iter()
            .map(|e| e.name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["target".to_owned()]);
    }
}
