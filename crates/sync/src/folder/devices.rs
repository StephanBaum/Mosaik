//! Checkpoints, the device list and change detection of the folder backend.

use core::time::Duration;
use std::time::Instant;

use mosaic_core::InstallId;
use sha2::{Digest, Sha256};

use super::fsutil::{self, Entry};
use super::layout::{HEAD_NAME, INFO_NAME};
use super::{FolderCrypto, FolderTransport, decoded};
use crate::engine::consts::FOLDER_POLL_INTERVAL;
use crate::transport::{ChangeToken, RemoteDevice, TransportError};
use crate::wire::checkpoint::{Checkpoint, MAX_LEN};
use crate::wire::device_info::DeviceInfo;
use crate::wire::folder_names::{self, FolderName};

/// The longest `info.mdev` a reader looks at (the sealed file is far
/// smaller; this only bounds a hostile file).
const MAX_INFO_LEN: u64 = 4096;

/// The most checkpoint candidates (`head.mck` and conflict copies) read from
/// one device directory: `head.mck` first, then the newest by modification
/// time. A flood of planted `*.mck` files costs this much and no more.
const MAX_CHECKPOINT_CANDIDATES: usize = 32;

impl<C: FolderCrypto> FolderTransport<C> {
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
        self.own_checkpoints_read.clear();
        for device in self.device_ids()? {
            let dir = self.layout.device_dir(&device);
            let mut candidates: Vec<Entry> = fsutil::list(&dir)?
                .into_iter()
                .filter(|e| !e.is_dir && folder_names::is_checkpoint_candidate(&e.name))
                .collect();
            candidates.sort_by(|a, b| {
                (b.name == HEAD_NAME)
                    .cmp(&(a.name == HEAD_NAME))
                    .then(b.modified_ms().cmp(&a.modified_ms()))
                    .then(a.name.cmp(&b.name))
            });
            candidates.truncate(MAX_CHECKPOINT_CANDIDATES);
            for entry in candidates {
                let Ok(Some(bytes)) = fsutil::read_capped(&dir.join(&entry.name), MAX_LEN as u64)
                else {
                    continue;
                };
                if device == self.install {
                    self.own_checkpoints_read
                        .insert(entry.name.clone(), (entry.len, entry.modified));
                }
                out.push((device, bytes));
            }
        }
        Ok(out)
    }

    /// The devices that have a verified `head.mck` or `info.mdev`: a
    /// directory somebody planted under `d/` is not a device.
    pub(super) fn folder_devices(&mut self) -> Result<Vec<RemoteDevice>, TransportError> {
        let candidates = self.folder_checkpoints()?;
        let mut out = Vec::new();
        for device in self.device_ids()? {
            let info = match fsutil::read_capped(
                &self.layout.device_dir(&device).join(INFO_NAME),
                MAX_INFO_LEN,
            ) {
                Ok(Some(sealed)) => decoded(DeviceInfo::open(&sealed, device, &self.crypto))?,
                Ok(None) => None,
                Err(e) => return Err(e),
            };
            let mut last_seen: Option<Checkpoint> = None;
            for (d, bytes) in candidates.iter().filter(|(d, _)| *d == device) {
                if let Some(c) = decoded(Checkpoint::decode(bytes, *d, &self.crypto))?
                    && last_seen.as_ref().is_none_or(|l| c.counter > l.counter)
                {
                    last_seen = Some(c);
                }
            }
            if info.is_none() && last_seen.is_none() {
                continue;
            }
            out.push(RemoteDevice {
                device_id: device,
                name: info.as_ref().map(|i| i.name.clone()),
                created_at_ms: info.as_ref().map(|i| i.created_at_ms),
                last_seen_ms: last_seen.map(|c| c.issued_at_ms),
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
    /// size and modification time), without what this install wrote itself
    /// (its `d/<self>/` directory and its own logs). Its own writes, the
    /// checkpoint of every run among them, are never a foreign change: with
    /// them in, every run would start the next one.
    fn change_digest(&self) -> Result<ChangeToken, TransportError> {
        let mut files: Vec<(String, u64, u64)> = Vec::new();
        let parsable = |e: &Entry| folder_names::parse(&e.name).is_some();
        let account = fsutil::list(&self.layout.account_files())?;
        Self::collect_files("account", &account, parsable, &mut files);
        for device in self.device_ids()? {
            if device == self.install {
                continue;
            }
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
                if device == self.install {
                    continue;
                }
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
