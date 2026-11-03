//! Filesystem helpers of the store: the database file's permissions, the
//! attachment directory, and crash-safe file writes.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::error::Result;

/// Suffix of temporary files in the attachment directory.
pub(super) const TMP_SUFFIX: &str = ".tmp";

/// Creates the database file with owner-only permissions on Unix before
/// SQLite opens it. SQLite would otherwise create it with the umask
/// default. A no-op elsewhere and for an existing file.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
pub(super) fn prepare_db_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Makes an existing database and its `-wal` / `-shm` files owner-only on
/// Unix (`0600`), for a store created by an older build or copied in with
/// looser permissions. SQLite creates the WAL and shared-memory files with
/// the database file's permissions. Runs only after the file was accepted
/// as a Mosaic store, so a foreign file keeps its permissions. A no-op
/// elsewhere.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
pub(super) fn restrict_db_files(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut paths = vec![path.to_path_buf()];
        for suffix in ["-wal", "-shm"] {
            let mut name = path.as_os_str().to_os_string();
            name.push(suffix);
            paths.push(name.into());
        }
        for p in paths {
            match fs::set_permissions(&p, fs::Permissions::from_mode(0o600)) {
                Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Creates the attachment directory (owner-only on Unix).
pub(super) fn create_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Makes a rename in `dir` durable. Directories cannot be synced through
/// the standard library on Windows; NTFS journals the rename itself.
#[cfg_attr(
    not(unix),
    allow(clippy::unnecessary_wraps, reason = "fallible on Unix")
)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(dir)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Writes `bytes` to `dir/name` through a synced temporary file and a
/// rename.
pub(super) fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!("{name}.{}.{n}{TMP_SUFFIX}", std::process::id()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, dir.join(name))?;
        sync_dir(dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(result?)
}

pub(super) fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}
