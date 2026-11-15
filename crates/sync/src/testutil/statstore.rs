//! A folder store that lags: it hides one file name from every listing but
//! still answers a direct `stat`, `read_capped` and `exists` for it, like a
//! provider whose listing is behind its writes (ADR-0162).

use std::sync::{Arc, Mutex};

use crate::folder::{Entry, FolderStore, LocalStore, RelPath};
use crate::transport::TransportError;

/// A shared handle to the name that [`StatOverride`] hides from listings.
pub type HiddenName = Arc<Mutex<Option<String>>>;

/// A [`LocalStore`] whose listings omit one file name.
#[derive(Debug)]
pub struct StatOverride {
    inner: LocalStore,
    hidden: HiddenName,
}

impl StatOverride {
    /// Wraps `inner`; nothing is hidden until [`StatOverride::hidden`] is set.
    #[must_use]
    pub fn new(inner: LocalStore) -> Self {
        Self {
            inner,
            hidden: HiddenName::default(),
        }
    }

    /// The handle that sets the hidden name (`None` shows everything).
    #[must_use]
    pub fn hidden(&self) -> HiddenName {
        Arc::clone(&self.hidden)
    }

    fn visible(&self, e: &Entry) -> bool {
        let hidden = self.hidden.lock().ok().and_then(|g| g.clone());
        hidden.is_none_or(|name| e.name.to_str() != Some(name.as_str()))
    }
}

impl FolderStore for StatOverride {
    fn begin_run(&self) -> Result<(), TransportError> {
        self.inner.begin_run()
    }

    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        let mut entries = self.inner.list(dir)?;
        entries.retain(|e| self.visible(e));
        Ok(entries)
    }

    fn list_with_links(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> {
        let mut entries = self.inner.list_with_links(dir)?;
        entries.retain(|e| self.visible(e));
        Ok(entries)
    }

    fn list_capped(
        &self,
        dir: &RelPath,
        limit: usize,
        keep: &dyn Fn(&Entry) -> bool,
    ) -> Result<Vec<Entry>, TransportError> {
        self.inner
            .list_capped(dir, limit, &|e| self.visible(e) && keep(e))
    }

    fn read_capped(&self, path: &RelPath, max: u64) -> Result<Option<Vec<u8>>, TransportError> {
        self.inner.read_capped(path, max)
    }

    fn write_atomic(
        &self,
        dir: &RelPath,
        name: &str,
        bytes: &[u8],
        replace: bool,
    ) -> Result<(), TransportError> {
        self.inner.write_atomic(dir, name, bytes, replace)
    }

    fn remove(&self, path: &RelPath) -> Result<(), TransportError> {
        self.inner.remove(path)
    }

    fn remove_empty_dir(&self, path: &RelPath) {
        self.inner.remove_empty_dir(path);
    }

    fn remove_squatter(&self, path: &RelPath) {
        self.inner.remove_squatter(path);
    }

    fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError> {
        self.inner.stat(path)
    }

    fn exists(&self, path: &RelPath) -> bool {
        self.inner.exists(path)
    }

    fn is_real_dir(&self, path: &RelPath) -> bool {
        self.inner.is_real_dir(path)
    }

    fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError> {
        self.inner.create_dir_all(dir)
    }
}
