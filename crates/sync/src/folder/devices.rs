//! Checkpoints, the device list and change detection of the folder backend.

use core::time::Duration;
use std::time::Instant;

use mosaic_core::InstallId;
use sha2::{Digest, Sha256};

use super::FolderTransport;
use super::fsutil::{self, Entry};
use super::layout::INFO_NAME;
use crate::engine::consts::FOLDER_POLL_INTERVAL;
use crate::transport::{ChangeToken, RemoteDevice, TransportError};
use crate::wire::SyncCrypto;
use crate::wire::checkpoint::{Checkpoint, MAX_LEN};
use crate::wire::device_info::DeviceInfo;
use crate::wire::folder_names::{self, FolderName};

/// The longest `info.mdev` a reader looks at (the sealed file is far
/// smaller; this only bounds a hostile file).
const MAX_INFO_LEN: u64 = 4096;

impl<C: SyncCrypto + Send> FolderTransport<C> {
    /// The device directories under `d/`.
    fn device_ids(&self) -> Result<Vec<InstallId>, TransportError> {
        Ok(fsutil::list(&self.layout.devices_dir())?
            .iter()
            .filter(|e| e.is_dir)
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Dir(id)) => InstallId::from_bytes(id).ok(),
                _ => None,
            })
            .collect())
    }

    pub(super) fn folder_checkpoints(
        &mut self,
    ) -> Result<Vec<(InstallId, Vec<u8>)>, TransportError> {
        let mut out = Vec::new();
        for device in self.device_ids()? {
            let dir = self.layout.device_dir(&device);
            for entry in fsutil::list(&dir)? {
                if entry.is_dir || !folder_names::is_checkpoint_candidate(&entry.name) {
                    continue;
                }
                if let Ok(Some(bytes)) = fsutil::read_capped(&dir.join(&entry.name), MAX_LEN as u64)
                {
                    out.push((device, bytes));
                }
            }
        }
        Ok(out)
    }

    pub(super) fn folder_devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        let candidates = self.folder_checkpoints()?;
        let mut out = Vec::new();
        for device in self.device_ids()? {
            let info = fsutil::read_capped(
                &self.layout.device_dir(&device).join(INFO_NAME),
                MAX_INFO_LEN,
            )?
            .and_then(|sealed| DeviceInfo::open(&sealed, device, &self.crypto).ok());
            let last_seen_ms = candidates
                .iter()
                .filter(|(d, _)| *d == device)
                .filter_map(|(d, bytes)| Checkpoint::decode(bytes, *d, &self.crypto).ok())
                .max_by_key(|c| c.counter)
                .map(|c| c.issued_at_ms);
            out.push(RemoteDevice {
                device_id: device,
                name: info.as_ref().map(|i| i.name.clone()),
                created_at_ms: info.as_ref().map(|i| i.created_at_ms),
                last_seen_ms,
            });
        }
        Ok(out)
    }

    /// Adds `(relative name, size, mtime)` of the parsable files of one
    /// directory to `files`.
    fn collect_files(
        prefix: &str,
        entries: &[Entry],
        wanted: impl Fn(&Entry) -> bool,
        files: &mut Vec<(String, u64, u64)>,
    ) {
        for e in entries.iter().filter(|e| !e.is_dir && wanted(e)) {
            files.push((
                format!("{prefix}/{}", e.name.to_string_lossy()),
                e.len,
                e.modified_ms(),
            ));
        }
    }

    /// A digest over the parsable files of the account directory (path,
    /// size and modification time).
    fn change_digest(&self) -> Result<ChangeToken, TransportError> {
        let mut files: Vec<(String, u64, u64)> = Vec::new();
        let parsable = |e: &Entry| folder_names::parse(&e.name).is_some();
        let account = fsutil::list(&self.layout.account_files())?;
        Self::collect_files("account", &account, parsable, &mut files);
        for device in self.device_ids()? {
            let dir = fsutil::list(&self.layout.device_dir(&device))?;
            let prefix = format!("d/{}", crate::wire::hex(device.as_bytes()));
            Self::collect_files(
                &prefix,
                &dir,
                |e| folder_names::is_checkpoint_candidate(&e.name) || parsable(e),
                &mut files,
            );
        }
        for vault in self.vault_ids()? {
            let vhex = crate::wire::hex(vault.as_bytes());
            let blobs = fsutil::list(&self.layout.blobs_dir(&vault))?;
            Self::collect_files(&format!("v/{vhex}/blobs"), &blobs, parsable, &mut files);
            for device in self.log_devices(&vault)? {
                let log = fsutil::list(&self.layout.log_dir(&vault, &device))?;
                let prefix = format!("v/{vhex}/log/{}", crate::wire::hex(device.as_bytes()));
                Self::collect_files(&prefix, &log, parsable, &mut files);
            }
        }
        files.sort();
        let mut hash = Sha256::new();
        for (name, len, modified) in &files {
            hash.update(name.as_bytes());
            hash.update([0]);
            hash.update(len.to_be_bytes());
            hash.update(modified.to_be_bytes());
        }
        Ok(ChangeToken(hash.finalize().to_vec()))
    }

    pub(super) fn folder_wait_for_change(
        &mut self,
        since: Option<&ChangeToken>,
        timeout: Duration,
    ) -> Result<ChangeToken, TransportError> {
        let deadline = Instant::now().checked_add(timeout);
        loop {
            let token = self.change_digest()?;
            if since != Some(&token) {
                return Ok(token);
            }
            let remaining = deadline.map_or(Duration::ZERO, |d| {
                d.saturating_duration_since(Instant::now())
            });
            if remaining.is_zero() {
                return Ok(token);
            }
            std::thread::sleep(remaining.min(FOLDER_POLL_INTERVAL));
        }
    }
}
