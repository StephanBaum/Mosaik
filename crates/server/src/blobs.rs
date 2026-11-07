//! Attachment ciphertext on disk: `<data_dir>/blobs/<first 2 hex>/<32 hex>.matt`
//! (M6 spec §6.4). Files are written to a temporary name and renamed, so a
//! reader never sees a partial file.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Lowercase hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// The blob store.
#[derive(Clone, Debug)]
pub struct Blobs {
    root: PathBuf,
}

impl Blobs {
    /// A store under `<data_dir>/blobs`.
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("blobs"),
        }
    }

    fn path(&self, id: &[u8; 16]) -> PathBuf {
        let name = hex(id);
        self.root
            .join(name.get(..2).unwrap_or("00"))
            .join(format!("{name}.matt"))
    }

    /// Writes (or replaces) the blob `id`.
    ///
    /// # Errors
    /// I/O errors.
    pub fn write(&self, id: &[u8; 16], bytes: &[u8]) -> io::Result<()> {
        let target = self.path(id);
        let dir = target
            .parent()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        fs::create_dir_all(dir)?;
        let mut rnd = [0u8; 8];
        getrandom::fill(&mut rnd).map_err(|_| io::Error::from(io::ErrorKind::Other))?;
        let tmp = dir.join(format!(".{}.tmp", hex(&rnd)));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt as _;
                options.mode(0o600);
            }
            let mut f = options.open(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&tmp, &target)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    /// Reads the blob `id`; `None` if the file is absent.
    ///
    /// # Errors
    /// I/O errors other than "not found".
    pub fn read(&self, id: &[u8; 16]) -> io::Result<Option<Vec<u8>>> {
        match fs::read(self.path(id)) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Deletes the blob `id` (absent is fine).
    ///
    /// # Errors
    /// I/O errors other than "not found".
    pub fn remove(&self, id: &[u8; 16]) -> io::Result<()> {
        match fs::remove_file(self.path(id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Whether the blob `id` is on disk.
    #[must_use]
    pub fn exists(&self, id: &[u8; 16]) -> bool {
        self.path(id).is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_remove() {
        let dir = tempfile::tempdir().unwrap();
        let b = Blobs::new(dir.path());
        let id = [0xabu8; 16];
        assert!(b.read(&id).unwrap().is_none());
        b.write(&id, b"hello").unwrap();
        assert_eq!(b.read(&id).unwrap().unwrap(), b"hello");
        assert!(b.path(&id).starts_with(dir.path().join("blobs").join("ab")));
        b.write(&id, b"again").unwrap();
        assert_eq!(b.read(&id).unwrap().unwrap(), b"again");
        b.remove(&id).unwrap();
        b.remove(&id).unwrap();
        assert!(!b.exists(&id));
    }
}
