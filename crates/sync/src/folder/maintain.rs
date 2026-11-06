//! Housekeeping of the folder backend after a sync run (ADR-0043):
//! compaction of this install's log, cleanup of its own temp, conflict and
//! unverifiable files and old account files, and the purge of expired
//! attachment blobs.

use std::collections::BTreeMap;

use mosaic_core::{AttachmentId, InstallId, VaultId};
use mosaic_vault::storage::MarkerStamp;

use super::fsutil::{self, TEMP_EXTENSION};
use super::layout::{HEAD_NAME, INFO_NAME};
use super::reader::SegmentFile;
use super::{FolderCrypto, FolderTransport, Outcome, decoded};
use crate::engine::consts::{BLOB_RETENTION_MS, COMPACT_BYTES, COMPACT_SEGMENTS, TEMP_MAX_AGE_MS};
use crate::transport::{MaintainContext, TransportError};
use crate::wire::account_file::{self, AccountFile, AccountFilePath};
use crate::wire::folder_names::{self, FolderName};
use crate::wire::segment::SegmentObject;

/// How many account files are looked at when the newest verified version is
/// searched (newest first; a flood of planted names costs this much).
const MAX_ACCOUNT_CANDIDATES: usize = 32;

impl<C: FolderCrypto> FolderTransport<C> {
    pub(super) fn folder_maintain(
        &mut self,
        cx: &mut dyn MaintainContext,
    ) -> Result<(), TransportError> {
        self.compact_own_logs(cx)?;
        self.clean_own_dirs(cx.now_ms())?;
        self.clean_account_files()?;
        self.purge_blobs(cx)
    }

    /// The sequence number after the last complete snapshot set in this
    /// install's log of `vault` (1 if there is none), and the files since.
    fn active_segments(&mut self, vault: VaultId) -> Result<(usize, u64), TransportError> {
        let install = self.install;
        let files = self.segment_files(&vault, &install)?;
        let mut parts_seen: BTreeMap<(u64, u32), usize> = BTreeMap::new();
        for file in &files {
            if let Outcome::Good {
                snapshot: Some(info),
                ..
            } = self.probe(vault, install, file)?
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
            let forced = self.force_compact.contains(&vault);
            let (count, bytes) = self.active_segments(vault)?;
            if !forced && count <= COMPACT_SEGMENTS && bytes <= COMPACT_BYTES {
                continue;
            }
            let objects: Vec<SegmentObject> = cx.snapshot_objects(vault)?;
            if objects.is_empty() {
                continue;
            }
            // Every part is written before anything is deleted.
            let base = self.write_snapshot(vault, objects)?;
            self.force_compact.remove(&vault);
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

    /// Whether the segment file was read (and did not verify) at its current
    /// length and time.
    fn known_bad(&self, vault: VaultId, device: InstallId, file: &SegmentFile) -> bool {
        self.scan.get(&(vault, device, file.seq)).is_some_and(|e| {
            e.len == file.len && e.modified == file.modified && matches!(e.outcome, Outcome::Bad)
        })
    }

    /// Cleans this install's own directories only:
    ///
    /// - temp files older than `TEMP_MAX_AGE`;
    /// - conflict copies of `head.mck` that the last `checkpoints()` call of
    ///   this run read (an unread copy might be the only one that survived),
    ///   and of `info.mdev`;
    /// - in an own log, segment files (and directories squatting on a
    ///   segment name) that never verified and are older than
    ///   `TEMP_MAX_AGE`: junk anyone with write access can plant, which the
    ///   writer never needs.
    fn clean_own_dirs(&mut self, now_ms: u64) -> Result<(), TransportError> {
        let install = self.install;
        let device_dir = self.layout.device_dir(&install);
        for entry in fsutil::list(&device_dir)? {
            if entry.is_dir {
                continue;
            }
            let name = entry.name.to_string_lossy();
            let stale_temp = is_stale_temp(&name, entry.modified_ms(), now_ms);
            let conflict_copy = name != HEAD_NAME
                && ((name.ends_with(".mck")
                    && self.own_checkpoints_read.get(&entry.name).is_some_and(
                        |(len, modified)| *len == entry.len && *modified == entry.modified,
                    ))
                    || (name.ends_with(".mdev") && name != INFO_NAME));
            if stale_temp || conflict_copy {
                fsutil::remove(&device_dir.join(&entry.name))?;
            }
        }
        for vault in self.vault_ids()? {
            self.clean_own_log(vault, now_ms)?;
        }
        Ok(())
    }

    /// The cleanup of one own log (see [`Self::clean_own_dirs`]).
    fn clean_own_log(&mut self, vault: VaultId, now_ms: u64) -> Result<(), TransportError> {
        let install = self.install;
        let dir = self.layout.log_dir(&vault, &install);
        for entry in fsutil::list(&dir)? {
            let name = entry.name.to_string_lossy().into_owned();
            let old = now_ms.saturating_sub(entry.modified_ms()) > TEMP_MAX_AGE_MS;
            let segment = match folder_names::parse(&entry.name) {
                Some(FolderName::Segment { seq }) => Some(seq),
                _ => None,
            };
            if entry.is_dir {
                // A directory squatting on a segment name.
                if segment.is_some() && old {
                    fsutil::remove_empty_dir(&dir.join(&entry.name));
                }
                continue;
            }
            if is_stale_temp(&name, entry.modified_ms(), now_ms) {
                fsutil::remove(&dir.join(&entry.name))?;
                continue;
            }
            let Some(seq) = segment.filter(|_| old) else {
                continue;
            };
            let file = SegmentFile {
                seq,
                len: entry.len,
                modified: entry.modified,
            };
            // Read it first: an unreadable file is not a bad one.
            let _ = self.probe(vault, install, &file)?;
            if self.known_bad(vault, install, &file) {
                fsutil::remove(&dir.join(&entry.name))?;
                self.scan.remove(&(vault, install, seq));
            }
        }
        Ok(())
    }

    /// Removes this install's account files of a lower version once a
    /// verified file of a higher version exists, from any device: a device
    /// keeps only its newest file, and an old wrapping of `AK` under the old
    /// password does not linger.
    fn clean_account_files(&mut self) -> Result<(), TransportError> {
        let dir = self.layout.account_files();
        let mut found: Vec<(u64, [u8; 16])> = fsutil::list(&dir)?
            .iter()
            .filter(|e| !e.is_dir)
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Account { version, device }) => Some((version, device)),
                _ => None,
            })
            .collect();
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let Some(highest) = self.highest_verified_account_version(&found)? else {
            return Ok(());
        };
        for (version, device) in found {
            if version < highest && device == *self.install.as_bytes() {
                fsutil::remove(&dir.join(folder_names::account_name(version, &device)))?;
            }
        }
        Ok(())
    }

    /// The highest version among `found` (newest first) whose file verifies.
    pub(super) fn highest_verified_account_version(
        &self,
        found: &[(u64, [u8; 16])],
    ) -> Result<Option<u64>, TransportError> {
        let dir = self.layout.account_files();
        for (version, device) in found.iter().take(MAX_ACCOUNT_CANDIDATES) {
            let Ok(device_id) = InstallId::from_bytes(*device) else {
                continue;
            };
            let Some(bytes) = fsutil::read_capped(
                &dir.join(folder_names::account_name(*version, device)),
                account_file::LEN as u64,
            )?
            else {
                continue;
            };
            let path = AccountFilePath {
                account_id: self.account,
                account_version: *version,
                device_id,
            };
            if decoded(AccountFile::decode(&bytes, &path, &self.crypto))?.is_some() {
                return Ok(Some(*version));
            }
        }
        Ok(None)
    }

    /// Deletes blobs whose newest delete marker this device first saw more
    /// than `BLOB_RETENTION` ago (by its own clock: the time inside a marker
    /// is whatever its writer's clock said) and that no local item refers to,
    /// and this install's own markers of blobs that are gone. Waits until the
    /// run saw no device behind, no unresolved key conflict and nothing
    /// undecryptable: a stale device must not purge what others still use.
    fn purge_blobs(&mut self, cx: &mut dyn MaintainContext) -> Result<(), TransportError> {
        let now = cx.now_ms();
        for vault in self.vault_ids()? {
            let markers = self.verified_markers(&vault)?;
            let flat: Vec<MarkerStamp> = markers
                .iter()
                .flat_map(|(attachment, list)| {
                    list.iter().map(|(device, stamp)| MarkerStamp {
                        attachment_id: *attachment,
                        device_id: *device,
                        stamp_ms: *stamp,
                    })
                })
                .collect();
            // The first-seen times are recorded in every run, caught up or
            // not, so the retention clock keeps running.
            let seen = cx.marker_first_seen(vault, &flat, now)?;
            if !cx.caught_up() {
                continue;
            }
            let mut newest: BTreeMap<AttachmentId, u64> = BTreeMap::new();
            for (marker, first) in flat.iter().zip(seen) {
                let e = newest.entry(marker.attachment_id).or_insert(0);
                *e = (*e).max(first);
            }
            for (attachment, first_seen) in newest {
                if now.saturating_sub(first_seen) <= BLOB_RETENTION_MS {
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

/// A temp file (`.<hex>.mtmp`) older than `TEMP_MAX_AGE`.
fn is_stale_temp(name: &str, modified_ms: u64, now_ms: u64) -> bool {
    name.ends_with(&format!(".{TEMP_EXTENSION}"))
        && now_ms.saturating_sub(modified_ms) > TEMP_MAX_AGE_MS
}
