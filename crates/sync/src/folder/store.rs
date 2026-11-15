//! The storage under the folder transport: [`FolderStore`] (ADR-0162) and its
//! local-directory implementation [`LocalStore`], which keeps the behaviour of
//! the folder backend on a directory exactly (`std::fs` only).
//!
//! Every path is a [`RelPath`]: a list of validated components below the
//! folder's root, built only from the layout constants, the canonical
//! [`folder_names`](crate::wire::folder_names) builders and names that a
//! listing returned. A component is never empty, `.` or `..`, and holds no
//! separator, `:`, NUL or control character, so no store can be made to leave
//! its root.
//!
//! Every write on a [`LocalStore`] goes through a temp file in the target
//! directory (`.<16 hex>.mtmp`, created with `create_new`), `write_all`,
//! `sync_all` and a rename, so a sync client never sees a half-written final
//! name from this process. Write-once names are checked for absence first.

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::transport::TransportError;

/// The extension of a temp file.
pub(crate) const TEMP_EXTENSION: &str = "mtmp";

/// At most this many directory items are looked at by
/// [`FolderStore::list_capped`], so a folder with millions of planted entries
/// costs bounded work.
pub(crate) const MAX_SCANNED: usize = 4096;

/// The longest path component, in bytes.
const MAX_COMPONENT_LEN: usize = 255;

/// What an entry of a listing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file (or, in a plain listing, anything that is not a
    /// directory or a link).
    File,
    /// A directory.
    Dir,
    /// A symbolic link or another object that is not a file or a directory
    /// (only reported by [`FolderStore::list_with_links`]).
    Link,
}

/// What a listing or a `stat` says about one entry.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Entry {
    /// The entry's name.
    pub name: OsString,
    /// What it is.
    pub kind: EntryKind,
    /// The length in bytes.
    pub len: u64,
    /// The last modification, if known.
    pub modified: Option<SystemTime>,
    /// An opaque version token (a provider's `rev` or `ETag`), if the store
    /// has one. A cached read is valid while the token is unchanged.
    pub version: Option<Box<[u8]>>,
}

impl Entry {
    /// Whether the entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Dir
    }

    /// Milliseconds since the Unix epoch of the last modification (0 if
    /// unknown).
    #[must_use]
    pub fn modified_ms(&self) -> u64 {
        self.modified
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// Whether `name` can be one component of a [`RelPath`].
#[must_use]
pub(crate) fn valid_component(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_COMPONENT_LEN
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | '\0' | ':') || c.is_control())
}

/// A path below the folder's root: a list of validated components. The root
/// itself has none.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct RelPath(Vec<String>);

impl RelPath {
    /// The folder's root.
    #[must_use]
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// This path with one more component; `None` if `name` is not a valid
    /// component.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<Self> {
        valid_component(name).then(|| self.known(name))
    }

    /// This path with the component of a listed entry; `None` if its name is
    /// not valid (or not UTF-8).
    #[must_use]
    pub fn child_of(&self, entry: &Entry) -> Option<Self> {
        self.child(entry.name.to_str()?)
    }

    /// The path without its last component; `None` for the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let (_, rest) = self.0.split_last()?;
        Some(Self(rest.to_vec()))
    }

    /// The components, from the root down.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// The last component; `None` for the root.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.0.last().map(String::as_str)
    }

    /// This path with one more component that the code itself defines: the
    /// layout constants and the `folder_names` builders. Never called with a
    /// name from a listing or from outside (a unit test proves that every
    /// builder output is valid).
    pub(crate) fn known(&self, name: &str) -> Self {
        debug_assert!(valid_component(name), "invalid component {name:?}");
        let mut parts = self.0.clone();
        parts.push(name.to_owned());
        Self(parts)
    }
}

/// The storage a folder transport runs on: a local directory or a provider's
/// API. All operations are on [`RelPath`]s below the store's root.
pub trait FolderStore: Send {
    /// Called at the start of every sync run (a remote store refreshes what it
    /// caches for the run).
    ///
    /// # Errors
    /// [`TransportError`].
    fn begin_run(&self) -> Result<(), TransportError> {
        Ok(())
    }

    /// The entries of a directory, sorted by name; a missing directory has
    /// none. Links are not entries.
    ///
    /// # Errors
    /// [`TransportError`].
    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError>;

    /// As [`list`](Self::list), but links (and other objects that are neither
    /// files nor directories) are reported as [`EntryKind::Link`].
    ///
    /// # Errors
    /// [`TransportError`].
    fn list_with_links(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        self.list(dir)
    }

    /// The entries of a directory that `keep` accepts, sorted by name, from a
    /// read that stops as soon as `limit` accepted entries are found or
    /// [`MAX_SCANNED`] items have been looked at. When more than `limit`
    /// accepted entries exist, which ones are listed is up to the store.
    ///
    /// # Errors
    /// [`TransportError`].
    fn list_capped(
        &self,
        dir: &RelPath,
        limit: usize,
        keep: &dyn Fn(&Entry) -> bool,
    ) -> Result<Vec<Entry>, TransportError>;

    /// Reads a regular file of at most `max` bytes; `None` if it is missing or
    /// not a regular file, `Err(Protocol)` if it is longer.
    ///
    /// # Errors
    /// [`TransportError`].
    fn read_capped(&self, path: &RelPath, max: u64) -> Result<Option<Vec<u8>>, TransportError>;

    /// Writes `dir/name`: the file appears whole or not at all. With
    /// `replace == false` the name must be absent (`Io` otherwise). The
    /// directory is created.
    ///
    /// # Errors
    /// [`TransportError`].
    fn write_atomic(
        &self,
        dir: &RelPath,
        name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), TransportError>;

    /// Removes a file; a missing file is fine.
    ///
    /// # Errors
    /// [`TransportError`].
    fn remove(&self, path: &RelPath) -> Result<(), TransportError>;

    /// Removes an empty directory; a missing or non-empty one is left alone.
    fn remove_empty_dir(&self, path: &RelPath);

    /// Removes whatever sits on a reserved name: a file, a directory with its
    /// content, or a link (only the link). Best effort.
    fn remove_squatter(&self, path: &RelPath);

    /// A direct, authoritative look at one entry (never a cached listing):
    /// `None` if it does not exist.
    ///
    /// # Errors
    /// [`TransportError`].
    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError>;

    /// Whether a file or directory exists.
    fn exists(&self, path: &RelPath) -> bool {
        matches!(self.stat(path), Ok(Some(_)))
    }

    /// Whether `path` is a real directory.
    fn is_real_dir(&self, path: &RelPath) -> bool {
        matches!(self.stat(path), Ok(Some(e)) if e.is_dir())
    }

    /// Creates a directory and its parents.
    ///
    /// # Errors
    /// [`TransportError`].
    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError>;
}

/// Maps an I/O error to the transport's error. Nothing of the error (it
/// carries paths) is kept.
pub(crate) fn io_error(e: &std::io::Error) -> TransportError {
    match e.kind() {
        ErrorKind::NotFound => TransportError::NotFound,
        _ => TransportError::Io,
    }
}

/// A folder on a local disk.
#[derive(Clone, Debug)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    /// A store on the directory `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    fn path(&self, p: &RelPath) -> PathBuf {
        let mut out = self.root.clone();
        out.extend(p.components());
        out
    }
}

/// The entry of one directory item; `None` for one that vanished while it was
/// listed (a sync client at work) and, unless `with_links`, for a symlink.
/// `DirEntry::metadata` does not follow symlinks; Mosaic never writes one, so
/// a link (anyone with write access to the folder can plant one) is not an
/// entry at all.
fn entry_of(item: &fs::DirEntry, with_links: bool) -> Option<Entry> {
    let meta = item.metadata().ok()?;
    let kind = if meta.file_type().is_symlink() {
        if !with_links {
            return None;
        }
        EntryKind::Link
    } else if meta.is_dir() {
        EntryKind::Dir
    } else if with_links && !meta.is_file() {
        // A pipe or a device: not a file, so a squatter.
        EntryKind::Link
    } else {
        EntryKind::File
    };
    Some(Entry {
        name: item.file_name(),
        kind,
        len: meta.len(),
        modified: meta.modified().ok(),
        version: None,
    })
}

fn read_dir_or_none(dir: &Path) -> Result<Option<fs::ReadDir>, TransportError> {
    match fs::read_dir(dir) {
        Ok(r) => Ok(Some(r)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(&e)),
    }
}

fn list_dir(dir: &Path, with_links: bool) -> Result<Vec<Entry>, TransportError> {
    let Some(read) = read_dir_or_none(dir)? else {
        return Ok(Vec::new());
    };
    let mut out: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|i| entry_of(&i, with_links))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
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

fn random_hex() -> Result<String, TransportError> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|_| TransportError::Io)?;
    Ok(crate::wire::hex(&b))
}

impl FolderStore for LocalStore {
    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        list_dir(&self.path(dir), false)
    }

    fn list_with_links(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        list_dir(&self.path(dir), true)
    }

    /// The entries of a directory that `keep` accepts, sorted by name, from a
    /// read that stops as soon as `limit` accepted entries are found or
    /// [`MAX_SCANNED`] items have been looked at (no full read, no full sort).
    /// Which entries come first is the file system's order, so when the
    /// directory holds more than `limit` accepted entries the listed ones are
    /// an arbitrary subset.
    fn list_capped(
        &self,
        dir: &RelPath,
        limit: usize,
        keep: &dyn Fn(&Entry) -> bool,
    ) -> Result<Vec<Entry>, TransportError> {
        let Some(read) = read_dir_or_none(&self.path(dir))? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for item in read.take(MAX_SCANNED) {
            if out.len() >= limit {
                break;
            }
            let Some(entry) = item.ok().and_then(|i| entry_of(&i, false)) else {
                continue;
            };
            if keep(&entry) {
                out.push(entry);
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Reads a regular file of at most `max` bytes; `None` if it is missing or
    /// not a regular file (a symlink, a directory, a device or a pipe is never
    /// followed or opened), `Err(Protocol)` if it is longer. The read itself
    /// is capped at `max + 1` bytes, so a file that grows or is swapped
    /// between the check and the read cannot make this allocate more.
    fn read_capped(&self, path: &RelPath, max: u64) -> Result<Option<Vec<u8>>, TransportError> {
        let path = self.path(path);
        let meta = match fs::symlink_metadata(&path) {
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
        let file = match open_regular(&path) {
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

    /// Writes `bytes` to `dir/name` through a temp file and a rename. With
    /// `replace == false` the name must be absent (`Io` otherwise): write-once
    /// names are never overwritten. The directory is created.
    fn write_atomic(
        &self,
        dir: &RelPath,
        name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), TransportError> {
        if !valid_component(name) {
            return Err(TransportError::Protocol);
        }
        let dir = self.path(dir);
        fs::create_dir_all(&dir).map_err(|e| io_error(&e))?;
        let target = dir.join(name);
        if !replace && target.try_exists().unwrap_or(false) {
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
        if !replace && target.try_exists().unwrap_or(false) {
            let _ = fs::remove_file(&tmp);
            return Err(TransportError::Io);
        }
        fs::rename(&tmp, &target).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            io_error(&e)
        })
    }

    fn remove(&self, path: &RelPath) -> Result<(), TransportError> {
        match fs::remove_file(self.path(path)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(&e)),
        }
    }

    fn remove_empty_dir(&self, path: &RelPath) {
        let _ = fs::remove_dir(self.path(path));
    }

    /// Removes whatever sits at `path`: a file, a directory with its content,
    /// or a symlink (only the link itself, never its target). Used on names
    /// in the own log that this device never wrote.
    fn remove_squatter(&self, path: &RelPath) {
        let path = self.path(path);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            return;
        };
        if meta.file_type().is_symlink() {
            // A link to a directory is a directory entry on Windows.
            if fs::remove_file(&path).is_err() {
                let _ = fs::remove_dir(&path);
            }
        } else if meta.is_dir() {
            // `remove_dir_all` does not follow links inside.
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }

    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        let full = self.path(path);
        let meta = match fs::symlink_metadata(&full) {
            Ok(m) => m,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(&e)),
        };
        let kind = if meta.file_type().is_symlink() {
            EntryKind::Link
        } else if meta.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };
        Ok(Some(Entry {
            name: full.file_name().map(OsString::from).unwrap_or_default(),
            kind,
            len: meta.len(),
            modified: meta.modified().ok(),
            version: None,
        }))
    }

    /// Whether a file or directory exists (a link is followed, as before).
    fn exists(&self, path: &RelPath) -> bool {
        self.path(path).try_exists().unwrap_or(false)
    }

    /// Whether `path` is a real directory: a symlink to one is not. The
    /// folder root is the exception: the user chose it, and a link to a
    /// directory is a folder.
    fn is_real_dir(&self, path: &RelPath) -> bool {
        if path.parent().is_none() {
            return self.root.is_dir();
        }
        fs::symlink_metadata(self.path(path)).is_ok_and(|m| m.is_dir())
    }

    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError> {
        fs::create_dir_all(self.path(dir)).map_err(|e| io_error(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::folder::layout::{HEAD_NAME, INFO_NAME, MARKER_NAME, README_NAME};
    use crate::wire::folder_names;

    fn store(dir: &tempfile::TempDir) -> LocalStore {
        LocalStore::new(dir.path())
    }

    fn rel(name: &str) -> RelPath {
        RelPath::root().child(name).unwrap()
    }

    #[test]
    fn list_capped_stops_at_the_limit_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            fs::create_dir(dir.path().join(format!("d{i:02}"))).unwrap();
            fs::write(dir.path().join(format!("f{i:02}")), b"x").unwrap();
        }
        let store = store(&dir);
        let root = RelPath::root();
        let dirs = store.list_capped(&root, 5, &|e| e.is_dir()).unwrap();
        assert_eq!(dirs.len(), 5);
        assert!(dirs.iter().all(Entry::is_dir));
        assert!(dirs.windows(2).all(|w| w[0].name < w[1].name), "sorted");
        // Under the limit, everything that is kept is listed.
        assert_eq!(
            store
                .list_capped(&root, 100, &|e| e.is_dir())
                .unwrap()
                .len(),
            20
        );
        assert!(
            store
                .list_capped(&rel("missing"), 5, &|_| true)
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
        let kept = store(&dir)
            .list_capped(&RelPath::root(), 1, &|_| {
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
        fs::write(dir.path().join("f"), b"x").unwrap();
        let store = store(&dir);
        assert!(store.is_real_dir(&RelPath::root()));
        assert!(!store.is_real_dir(&rel("f")));
        assert!(!store.is_real_dir(&rel("missing")));
    }

    #[test]
    fn read_capped_reads_only_small_regular_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("f"), b"12345").unwrap();
        let store = store(&dir);
        assert_eq!(
            store.read_capped(&rel("f"), 5).unwrap(),
            Some(b"12345".to_vec())
        );
        assert_eq!(
            store.read_capped(&rel("f"), 4),
            Err(TransportError::Protocol)
        );
        assert_eq!(store.read_capped(&rel("missing"), 5).unwrap(), None);
        // A directory is not a file.
        assert_eq!(store.read_capped(&RelPath::root(), 5).unwrap(), None);
    }

    #[test]
    fn write_atomic_is_write_once_unless_replacing() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(&dir);
        let sub = rel("sub");
        store.write_atomic(&sub, "f", b"one", false).unwrap();
        assert_eq!(
            store.write_atomic(&sub, "f", b"two", false),
            Err(TransportError::Io)
        );
        store.write_atomic(&sub, "f", b"two", true).unwrap();
        assert_eq!(
            store.read_capped(&sub.child("f").unwrap(), 10).unwrap(),
            Some(b"two".to_vec())
        );
        assert_eq!(
            store.write_atomic(&sub, "../f", b"x", true),
            Err(TransportError::Protocol)
        );
        // No temp file is left behind.
        let names: Vec<_> = store
            .list(&sub)
            .unwrap()
            .into_iter()
            .map(|e| e.name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["f".to_owned()]);
    }

    #[test]
    fn stat_is_a_direct_look() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("f"), b"abc").unwrap();
        let store = store(&dir);
        let file = store.stat(&rel("f")).unwrap().unwrap();
        assert_eq!((file.kind, file.len), (EntryKind::File, 3));
        assert!(store.stat(&rel("missing")).unwrap().is_none());
        assert!(store.stat(&RelPath::root()).unwrap().unwrap().is_dir());
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
        let store = store(&dir);
        assert_eq!(store.read_capped(&rel("link"), 100).unwrap(), None);
        assert_eq!(store.read_capped(&rel("dev-zero"), 100).unwrap(), None);
        let names: Vec<_> = store
            .list(&RelPath::root())
            .unwrap()
            .into_iter()
            .map(|e| e.name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["target".to_owned()]);
    }

    #[cfg(unix)]
    #[test]
    fn list_with_links_reports_links() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("target"), b"x").unwrap();
        std::os::unix::fs::symlink(dir.path().join("target"), dir.path().join("link")).unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        let entries = store(&dir).list_with_links(&RelPath::root()).unwrap();
        let kinds: Vec<_> = entries
            .iter()
            .map(|e| (e.name.to_string_lossy().into_owned(), e.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("link".to_owned(), EntryKind::Link),
                ("sub".to_owned(), EntryKind::Dir),
                ("target".to_owned(), EntryKind::File),
            ]
        );
    }

    #[test]
    fn child_refuses_unsafe_components() {
        let root = RelPath::root();
        for bad in ["", ".", "..", "a/b", "a\\b", "a\0", "c:", "a\nb"] {
            assert!(root.child(bad).is_none(), "{bad:?}");
        }
        assert!(root.child(&"a".repeat(256)).is_none());
        assert!(root.child(&"a".repeat(255)).is_some());
        let head = root.child("head.mck").unwrap();
        assert_eq!(head.components().collect::<Vec<_>>(), vec!["head.mck"]);
        assert_eq!(head.parent(), Some(root.clone()));
        assert_eq!(root.parent(), None);
    }

    #[test]
    fn every_builder_output_is_a_valid_component() {
        let id = [0xABu8; 16];
        let names = [
            MARKER_NAME.to_owned(),
            README_NAME.to_owned(),
            HEAD_NAME.to_owned(),
            INFO_NAME.to_owned(),
            folder_names::dir_name(&id),
            folder_names::segment_name(u64::MAX),
            folder_names::account_name(u64::MAX, &id),
            folder_names::rekey_marker_name(u32::MAX, &id),
            folder_names::blob_name(&id),
            folder_names::blob_delete_name(&id, &id),
            format!(".{}.{TEMP_EXTENSION}", "0123456789abcdef"),
        ];
        for name in &names {
            assert!(valid_component(name), "{name}");
        }
        // The layout's own components (every path it builds).
        let account = mosaic_core::AccountId::from_bytes(id).unwrap();
        let device = mosaic_core::InstallId::from_bytes(id).unwrap();
        let vault = mosaic_core::VaultId::from_bytes(id).unwrap();
        let attachment = mosaic_core::AttachmentId::from_bytes(id).unwrap();
        let layout = crate::folder::layout::Layout::new(&account);
        for path in [
            layout.dir(),
            layout.account_files(),
            layout.devices_dir(),
            layout.device_dir(&device),
            layout.vaults_dir(),
            layout.vault_dir(&vault),
            layout.logs_dir(&vault),
            layout.log_dir(&vault, &device),
            layout.blobs_dir(&vault),
            layout.blob_path(&vault, &attachment),
            layout.marker_path(&vault, &attachment, &device),
        ] {
            assert!(path.components().all(valid_component), "{path:?}");
        }
    }
}
