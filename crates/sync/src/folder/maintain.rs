//! Housekeeping of the folder backend after a sync run (ADR-0043):
//! compaction of this install's log, cleanup of its own temp, conflict and
//! unverifiable files and old account files, and the purge of expired
//! attachment blobs.

use std::collections::BTreeMap;

use mosaic_core::{AttachmentId, InstallId, VaultId};
use mosaic_vault::storage::{FolderRekeyFloor, MarkerStamp};

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
        let floors = cx.rekey_floors()?;
        self.compact_own_logs(cx, &floors)?;
        self.clean_own_dirs(cx.now_ms(), &floors)?;
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

    /// Whether the verified segments of the own log have a hole: a gap in
    /// their numbers after the newest complete snapshot set (or from 1 if
    /// there is none). A hole is what a skipped number, a rollback that was
    /// acknowledged or a change of backend leaves; readers count the missing
    /// numbers as lag until a snapshot closes them.
    fn own_log_has_hole(&mut self, vault: VaultId) -> Result<bool, TransportError> {
        let install = self.install;
        let files = self.segment_files(&vault, &install)?;
        let mut parts_seen: BTreeMap<(u64, u32), usize> = BTreeMap::new();
        let mut good: Vec<u64> = Vec::new();
        for file in &files {
            if let Outcome::Good { snapshot, .. } = self.probe(vault, install, file)? {
                good.push(file.seq);
                if let Some(info) = snapshot {
                    *parts_seen.entry((info.base, info.parts)).or_default() += 1;
                }
            }
        }
        let start = parts_seen
            .iter()
            .filter(|((_, parts), n)| u64::try_from(**n) == Ok(u64::from(*parts)))
            .map(|((base, parts), _)| base.saturating_add(u64::from(*parts)))
            .max()
            .unwrap_or(1);
        let mut expected = start;
        for seq in good.into_iter().filter(|s| *s >= start) {
            if seq != expected {
                return Ok(true);
            }
            expected = seq.saturating_add(1);
        }
        Ok(false)
    }

    fn compact_own_logs(
        &mut self,
        cx: &mut dyn MaintainContext,
        floors: &[FolderRekeyFloor],
    ) -> Result<(), TransportError> {
        for vault in self.vault_ids()? {
            let forced = self.force_compact.contains(&vault) || self.own_log_has_hole(vault)?;
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
            let now = cx.now_ms();
            for file in self.segment_files(&vault, &install)? {
                if file.seq < base && removable(floors, vault, file.seq, now) {
                    fsutil::remove(&dir.join(folder_names::segment_name(file.seq)))?;
                    self.scan.remove(&(vault, install, file.seq));
                }
            }
        }
        Ok(())
    }

    /// Whether the segment file was read (and did not verify) at its current
    /// length and time.
    pub(super) fn known_bad(&self, vault: VaultId, device: InstallId, file: &SegmentFile) -> bool {
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
    ///
    /// Own segments up to a re-key floor are not junk (they were sealed under
    /// the previous epoch and fail the MAC now): see [`removable`].
    fn clean_own_dirs(
        &mut self,
        now_ms: u64,
        floors: &[FolderRekeyFloor],
    ) -> Result<(), TransportError> {
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
            self.clean_own_log(vault, now_ms, floors)?;
        }
        Ok(())
    }

    /// The cleanup of one own log (see [`Self::clean_own_dirs`]).
    fn clean_own_log(
        &mut self,
        vault: VaultId,
        now_ms: u64,
        floors: &[FolderRekeyFloor],
    ) -> Result<(), TransportError> {
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
            if !removable(floors, vault, seq, now_ms) {
                continue;
            }
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
        Ok(self
            .highest_verified_account_file(found)?
            .map(|file| file.account_version))
    }

    /// The highest-version file among `found` (newest first) that verifies.
    pub(super) fn highest_verified_account_file(
        &self,
        found: &[(u64, [u8; 16])],
    ) -> Result<Option<AccountFile>, TransportError> {
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
            if let Some(file) = decoded(AccountFile::decode(&bytes, &path, &self.crypto))? {
                return Ok(Some(file));
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

/// Whether this install's own segment `seq` of `vault` may be removed by the
/// housekeeping (CRYPTO_SPEC §10.1, "Pre-re-key own segments"): a segment at
/// or below the vault's re-key floor was sealed under the previous epoch, so
/// it fails the MAC now but is not junk. Stale devices that still read the old
/// epoch need it, so it stays until a complete snapshot set of the new epoch
/// exists in the own log **and** [`BLOB_RETENTION_MS`] have passed since the
/// re-key. Segments above the floor, and vaults without one, follow the
/// ordinary rules. The 24-hour rule for unverifiable files never applies to
/// the pre-floor segments.
pub(super) fn removable(
    floors: &[FolderRekeyFloor],
    vault: VaultId,
    seq: u64,
    now_ms: u64,
) -> bool {
    let Some(floor) = floors.iter().find(|f| f.vault_id == vault) else {
        return true;
    };
    if seq > floor.floor_seq {
        return true;
    }
    let rekey_at = u64::try_from(floor.rekey_at_ms).unwrap_or(u64::MAX);
    floor.snapshot_done && now_ms.saturating_sub(rekey_at) > BLOB_RETENTION_MS
}

/// A temp file (`.<hex>.mtmp`) older than `TEMP_MAX_AGE`.
fn is_stale_temp(name: &str, modified_ms: u64, now_ms: u64) -> bool {
    name.ends_with(&format!(".{TEMP_EXTENSION}"))
        && now_ms.saturating_sub(modified_ms) > TEMP_MAX_AGE_MS
}
