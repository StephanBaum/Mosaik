//! Housekeeping of the folder backend after a sync run (ADR-0043):
//! compaction of this install's log, cleanup of its own temp and conflict
//! files, and the purge of expired attachment blobs.

use mosaic_core::{AttachmentId, VaultId};

use super::fsutil::{self, TEMP_EXTENSION};
use super::layout::{HEAD_NAME, INFO_NAME};
use super::{FolderTransport, Outcome};
use crate::engine::consts::{BLOB_RETENTION_MS, COMPACT_BYTES, COMPACT_SEGMENTS, TEMP_MAX_AGE_MS};
use crate::transport::{MaintainContext, TransportError};
use crate::wire::SyncCrypto;
use crate::wire::folder_names;
use crate::wire::segment::SegmentObject;

impl<C: SyncCrypto + Send> FolderTransport<C> {
    pub(super) fn folder_maintain(
        &mut self,
        cx: &mut dyn MaintainContext,
    ) -> Result<(), TransportError> {
        self.compact_own_logs(cx)?;
        self.clean_own_dirs(cx.now_ms())?;
        self.purge_blobs(cx)
    }

    /// The sequence number after the last complete snapshot set in this
    /// install's log of `vault` (1 if there is none), and the files since.
    fn active_segments(&mut self, vault: VaultId) -> Result<(usize, u64), TransportError> {
        let install = self.install;
        let files = self.segment_files(&vault, &install)?;
        let mut parts_seen: std::collections::BTreeMap<(u64, u32), usize> =
            std::collections::BTreeMap::new();
        for file in &files {
            if let Outcome::Good {
                snapshot: Some(info),
                ..
            } = self.probe(vault, install, file)
            {
                *parts_seen.entry((info.base, info.parts)).or_default() += 1;
            }
        }
        let after = parts_seen
            .iter()
            .filter(|((_, parts), n)| u64::try_from(**n) == Ok(u64::from(*parts)))
            .map(|((base, parts), _)| base.saturating_add(u64::from(*parts)))
            .max()
            .unwrap_or(0);
        let active: Vec<_> = files.iter().filter(|f| f.seq >= after).collect();
        Ok((active.len(), active.iter().map(|f| f.len).sum()))
    }

    fn compact_own_logs(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        for vault in self.vault_ids()? {
            let (count, bytes) = self.active_segments(vault)?;
            if count <= COMPACT_SEGMENTS && bytes <= COMPACT_BYTES {
                continue;
            }
            let objects: Vec<SegmentObject> = cx.snapshot_objects(vault)?;
            if objects.is_empty() {
                continue;
            }
            // Every part is written before anything is deleted.
            let base = self.write_snapshot(vault, objects)?;
            let install = self.install;
            let dir = self.layout.log_dir(&vault, &install);
            for file in self.segment_files(&vault, &install)? {
                if file.seq < base {
                    fsutil::remove(&dir.join(folder_names::segment_name(file.seq)))?;
                    self.scan.remove(&(vault, install, file.seq));
                }
            }
        }
        Ok(())
    }

    /// Removes this install's conflict copies of `head.mck` and `info.mdev`
    /// and its temp files older than `TEMP_MAX_AGE` (own directories only).
    fn clean_own_dirs(&self, now_ms: u64) -> Result<(), TransportError> {
        let mut dirs = vec![self.layout.device_dir(&self.install)];
        for vault in self.vault_ids()? {
            dirs.push(self.layout.log_dir(&vault, &self.install));
        }
        for dir in dirs {
            for entry in fsutil::list(&dir)? {
                if entry.is_dir {
                    continue;
                }
                let name = entry.name.to_string_lossy();
                let stale_temp = name.ends_with(&format!(".{TEMP_EXTENSION}"))
                    && now_ms.saturating_sub(entry.modified_ms()) > TEMP_MAX_AGE_MS;
                let conflict_copy = name != HEAD_NAME
                    && name != INFO_NAME
                    && (name.ends_with(".mck") || name.ends_with(".mdev"));
                if stale_temp || conflict_copy {
                    fsutil::remove(&dir.join(&entry.name))?;
                }
            }
        }
        Ok(())
    }

    /// Deletes blobs whose newest delete marker is older than
    /// `BLOB_RETENTION` and that no local item refers to, and this
    /// install's own markers of blobs that are gone. Waits until the run saw
    /// no device behind: a stale device must not purge what others still
    /// use.
    fn purge_blobs(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        if !cx.caught_up() {
            return Ok(());
        }
        let now = cx.now_ms();
        for vault in self.vault_ids()? {
            let markers = self.verified_markers(&vault)?;
            for (attachment, list) in markers {
                let newest = list.iter().map(|(_, t)| *t).max().unwrap_or(now);
                if now.saturating_sub(newest) <= BLOB_RETENTION_MS {
                    continue;
                }
                self.purge_one(cx, vault, attachment)?;
            }
        }
        Ok(())
    }

    fn purge_one(
        &mut self,
        cx: &mut dyn MaintainContext,
        vault: VaultId,
        attachment: AttachmentId,
    ) -> Result<(), TransportError> {
        let blob = self.layout.blob_path(&vault, &attachment);
        if fsutil::exists(&blob) {
            if cx.attachment_referenced(attachment)? {
                return Ok(());
            }
            fsutil::remove(&blob)?;
        }
        fsutil::remove(&self.layout.marker_path(&vault, &attachment, &self.install))
    }
}
