//! The discovery lease (Windows): proof that a desktop is alive behind
//! `endpoint.json` (`docs/NATIVE_MESSAGING.md` §1, THREAT_MODEL R42).
//!
//! Named pipe names are machine-global. If the desktop crashed and left
//! `endpoint.json` behind, another account could create a pipe with that
//! name and answer the host. To tell a live file from a stale one, the
//! desktop keeps a handle to the file open for its whole lifetime:
//!
//! | Handle | Access | Share mode |
//! |---|---|---|
//! | the desktop's lease | write | `FILE_SHARE_READ` |
//! | the host's probe | read | `FILE_SHARE_READ` |
//! | the host's reader | read | all (the std default) |
//!
//! Windows grants an open only if the requested access is allowed by the
//! share mode of every existing handle and the requested share mode allows
//! the access of every existing handle. The probe asks for read access, which
//! the lease allows, but its share mode lacks `FILE_SHARE_WRITE` while the
//! lease holds write access, so it fails with a sharing violation exactly
//! while a lease exists. The reader shares everything, so it works next to
//! the lease. No lease (a crash, a stale file) means the probe succeeds and
//! the host refuses the file without connecting.
//!
//! The lease handle denies `FILE_SHARE_WRITE` and `FILE_SHARE_DELETE` to
//! others, so nobody can rewrite or delete the file under it, and a second
//! desktop cannot replace it (its rename is denied).

use std::fs::{self, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use super::{
    is_valid_pipe_name, read_discovery_file, remove_discovery_file, write_discovery_file_io,
};
use crate::error::Error;

/// `ERROR_SHARING_VIOLATION` (winerror.h): the file is open in a way that
/// conflicts with the requested access or share mode.
const ERROR_SHARING_VIOLATION: i32 = 32;
/// `ERROR_ACCESS_DENIED` (winerror.h): what a rename over a file that is
/// open without `FILE_SHARE_DELETE` reports.
const ERROR_ACCESS_DENIED: i32 = 5;
/// `FILE_SHARE_READ` (winnt.h). `OpenOptionsExt::share_mode` takes the raw
/// bit mask and std exports no names for the bits.
const FILE_SHARE_READ: u32 = 0x1;

/// Attempts to take the lease, and the pause between them. A host's probe
/// holds the file for a moment and blocks the write.
const ATTEMPTS: u32 = 10;
const RETRY_PAUSE: Duration = Duration::from_millis(25);

/// The desktop's hold on `endpoint.json`. Dropping it closes the handle and
/// deletes the file.
#[derive(Debug)]
pub struct DiscoveryLease {
    path: PathBuf,
    file: Option<fs::File>,
}

impl DiscoveryLease {
    /// Writes the discovery file for `pipe` atomically, then opens it with
    /// write access and `FILE_SHARE_READ` and keeps it open until the lease
    /// is dropped. Call it after the pipe is bound.
    ///
    /// # Errors
    /// [`Error::EndpointInvalid`] for a bad pipe name or a path without a
    /// directory, [`Error::EndpointInUse`] if another desktop holds the
    /// file, [`Error::Io`] otherwise.
    pub fn create(path: &Path, pipe: &str) -> Result<Self, Error> {
        if !is_valid_pipe_name(pipe) || path.parent().is_none() {
            return Err(Error::EndpointInvalid);
        }
        let mut attempt = 1;
        loop {
            match Self::try_create(path, pipe) {
                Ok(lease) => return Ok(lease),
                Err(e) if is_contention(&e) && attempt < ATTEMPTS => {
                    attempt += 1;
                    sleep(RETRY_PAUSE);
                }
                Err(e) if is_contention(&e) => return Err(Error::EndpointInUse),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn try_create(path: &Path, pipe: &str) -> io::Result<Self> {
        write_discovery_file_io(path, pipe)?;
        let file = OpenOptions::new()
            .write(true)
            .share_mode(FILE_SHARE_READ)
            .open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file: Some(file),
        })
    }
}

impl Drop for DiscoveryLease {
    fn drop(&mut self) {
        // Close the handle first: the lease denies deletion while open.
        drop(self.file.take());
        let _ = remove_discovery_file(&self.path);
    }
}

fn is_contention(e: &io::Error) -> bool {
    matches!(
        e.raw_os_error(),
        Some(ERROR_SHARING_VIOLATION | ERROR_ACCESS_DENIED)
    )
}

/// Whether a desktop holds the discovery file at `path` (the host's probe).
///
/// # Errors
/// [`Error::DesktopNotRunning`] if the file does not exist, [`Error::Io`]
/// for any other failure to open it.
pub fn discovery_is_live(path: &Path) -> Result<bool, Error> {
    match OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
    {
        // Nobody holds the file open for writing: no desktop, a stale file.
        Ok(_probe) => Ok(false),
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Error::DesktopNotRunning),
        Err(e) => Err(e.into()),
    }
}

/// Reads the pipe name out of the discovery file at `path`, but only if a
/// desktop holds it ([`discovery_is_live`]).
///
/// # Errors
/// [`Error::DesktopNotRunning`] if the file is missing or stale, and the
/// errors of [`read_discovery_file`].
pub fn read_live_discovery_file(path: &Path) -> Result<String, Error> {
    if discovery_is_live(path)? {
        read_discovery_file(path)
    } else {
        Err(Error::DesktopNotRunning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::{new_windows_pipe_name, write_discovery_file};

    fn discovery_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("browser").join("endpoint.json")
    }

    #[test]
    fn a_held_lease_is_live_and_readable() {
        let dir = tempfile::tempdir().unwrap();
        let path = discovery_path(&dir);
        let pipe = new_windows_pipe_name().unwrap();
        let lease = DiscoveryLease::create(&path, &pipe).unwrap();
        // The probe is refused by the lease, the full-sharing read is not.
        assert_eq!(discovery_is_live(&path), Ok(true));
        assert_eq!(read_live_discovery_file(&path), Ok(pipe.clone()));
        assert_eq!(read_discovery_file(&path), Ok(pipe));
        drop(lease);
    }

    #[test]
    fn without_a_lease_the_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = discovery_path(&dir);
        assert_eq!(discovery_is_live(&path), Err(Error::DesktopNotRunning));
        assert_eq!(
            read_live_discovery_file(&path),
            Err(Error::DesktopNotRunning)
        );
        // What a crashed desktop leaves behind: the file, no holder.
        let pipe = new_windows_pipe_name().unwrap();
        write_discovery_file(&path, &pipe).unwrap();
        assert_eq!(discovery_is_live(&path), Ok(false));
        assert_eq!(
            read_live_discovery_file(&path),
            Err(Error::DesktopNotRunning)
        );
    }

    #[test]
    fn dropping_the_lease_removes_the_file_and_a_restart_can_lease_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = discovery_path(&dir);
        let lease = DiscoveryLease::create(&path, &new_windows_pipe_name().unwrap()).unwrap();
        drop(lease);
        assert!(!path.exists());
        assert_eq!(
            read_live_discovery_file(&path),
            Err(Error::DesktopNotRunning)
        );
        // A crash left a stale file: the next start replaces and leases it.
        let stale = new_windows_pipe_name().unwrap();
        write_discovery_file(&path, &stale).unwrap();
        let fresh = new_windows_pipe_name().unwrap();
        let lease = DiscoveryLease::create(&path, &fresh).unwrap();
        assert_eq!(read_live_discovery_file(&path), Ok(fresh));
        drop(lease);
    }

    #[test]
    fn a_second_desktop_cannot_take_over_a_live_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = discovery_path(&dir);
        let first = new_windows_pipe_name().unwrap();
        let lease = DiscoveryLease::create(&path, &first).unwrap();
        let other = new_windows_pipe_name().unwrap();
        assert_eq!(
            DiscoveryLease::create(&path, &other).err(),
            Some(Error::EndpointInUse)
        );
        assert_eq!(read_live_discovery_file(&path), Ok(first));
        assert_eq!(
            DiscoveryLease::create(&path, "not a pipe").err(),
            Some(Error::EndpointInvalid)
        );
        drop(lease);
    }
}
