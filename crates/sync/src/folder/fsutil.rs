//! File-system helpers of the folder transport (`std::fs` only).
//!
//! Every write goes through a temp file in the target directory
//! (`.<16 hex>.mtmp`, created with `create_new`), `write_all`, `sync_all`
//! and a rename, so a sync client never sees a half-written final name from
//! this process. Write-once names are checked for absence first.

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write as _};
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

/// The entries of a directory; a missing directory has none.
pub(crate) fn list(dir: &Path) -> Result<Vec<Entry>, TransportError> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_error(&e)),
    };
    let mut out = Vec::new();
    for item in read {
        let Ok(item) = item else { continue };
        // An entry that vanishes while it is listed (a sync client at work)
        // is skipped.
        let Ok(meta) = item.metadata() else { continue };
        out.push(Entry {
            name: item.file_name(),
            is_dir: meta.is_dir(),
            len: meta.len(),
            modified: meta.modified().ok(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Reads a regular file of at most `max` bytes; `None` if it is missing,
/// `Err(Protocol)` if it is longer.
pub(crate) fn read_capped(path: &Path, max: u64) -> Result<Option<Vec<u8>>, TransportError> {
    let meta = match fs::metadata(path) {
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
    match fs::read(path) {
        Ok(bytes) if u64::try_from(bytes.len()).is_ok_and(|n| n <= max) => Ok(Some(bytes)),
        Ok(_) => Err(TransportError::Protocol),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(&e)),
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

/// Removes a file; a missing file is fine.
pub(crate) fn remove(path: &Path) -> Result<(), TransportError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(&e)),
    }
}
