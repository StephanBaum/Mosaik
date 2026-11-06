//! Writing segments (ADR-0043): every push appends one or more segments to
//! this install's own log; compaction writes a snapshot as contiguous parts
//! and only then deletes the older segments (`maintain`).

use mosaic_core::{KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use mosaic_vault::storage::RecordBody;

use super::reader::SegmentFile;
use super::{FolderCrypto, FolderTransport, fsutil, lock_or};
use crate::transport::{KeyPushResult, PushRecord, PushResult, TransportError, VaultCreateResult};
use crate::wire::folder_names::{self, FolderName};
use crate::wire::segment::{
    Segment, SegmentObject, SnapshotInfo, WRITE_MAX_OBJECTS, WRITE_MAX_SEGMENT_LEN,
};

/// The bytes of a segment besides its objects, with room to spare.
const SEGMENT_OVERHEAD: usize = 256;

/// How many taken names the writer steps past before it gives up.
const MAX_NAME_ATTEMPTS: u32 = 16;

/// The encoded size of an object: kind, length and body.
fn object_len(object: &SegmentObject) -> usize {
    5 + match object {
        SegmentObject::Item { ciphertext, .. } => 16 + ciphertext.as_bytes().len(),
        SegmentObject::Tombstone { ciphertext, .. } => 16 + ciphertext.as_bytes().len(),
        SegmentObject::VaultKey { .. } => 4 + WrappedKey::LEN,
        SegmentObject::Meta(meta) => meta.as_bytes().len(),
    }
}

/// Splits objects into segments of at most `WRITE_MAX_OBJECTS` objects and
/// `WRITE_MAX_SEGMENT_LEN` bytes, keeping their order.
pub(crate) fn chunk(objects: Vec<SegmentObject>) -> Vec<Vec<SegmentObject>> {
    let mut chunks = Vec::new();
    let mut current: Vec<SegmentObject> = Vec::new();
    let mut size = SEGMENT_OVERHEAD;
    for object in objects {
        let len = object_len(&object);
        if !current.is_empty()
            && (current.len() >= WRITE_MAX_OBJECTS
                || size.saturating_add(len) > WRITE_MAX_SEGMENT_LEN)
        {
            chunks.push(core::mem::take(&mut current));
            size = SEGMENT_OVERHEAD;
        }
        size = size.saturating_add(len);
        current.push(object);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn record_object(record: &PushRecord<'_>) -> SegmentObject {
    match record.body {
        RecordBody::Item(ciphertext) => SegmentObject::Item {
            record_id: record.record_id,
            ciphertext: ciphertext.clone(),
        },
        RecordBody::Tombstone(ciphertext) => SegmentObject::Tombstone {
            record_id: record.record_id,
            ciphertext: ciphertext.clone(),
        },
    }
}

impl<C: FolderCrypto> FolderTransport<C> {
    /// The highest sequence number in this install's log of `vault` whose
    /// segment verifies (0 if none). A file that does not verify (junk
    /// planted by anyone with write access, a partial copy) does not count,
    /// so a planted name can neither inflate the checkpoint nor overflow the
    /// numbering.
    fn own_verified_high(&mut self, vault: VaultId) -> Result<u64, TransportError> {
        let install = self.install;
        let files = self.segment_files(&vault, &install)?;
        for file in files.iter().rev() {
            if matches!(
                self.probe(vault, install, file)?,
                super::Outcome::Good { .. }
            ) {
                return Ok(file.seq);
            }
        }
        Ok(0)
    }

    /// The highest sequence number in this install's log of `vault` (0 if
    /// none), as the checkpoint reports it.
    pub(crate) fn own_high_seq(&mut self, vault: VaultId) -> Result<u64, TransportError> {
        let on_disk = self.own_verified_high(vault)?;
        let memo = self
            .own_next
            .get(&vault)
            .map_or(0, |next| next.saturating_sub(1));
        Ok(on_disk.max(memo))
    }

    /// The next sequence number to write: above everything verified on disk,
    /// everything reserved before (a folder that was rolled back, a snapshot
    /// set that failed half-way) and everything written in this run. A name
    /// that is taken by something unverified (planted junk, a directory) is
    /// stepped past, at most [`MAX_NAME_ATTEMPTS`] times.
    fn next_own_seq(&mut self, vault: VaultId) -> Result<u64, TransportError> {
        let state = self
            .crypto
            .own_seq(vault)
            .map_err(|e| lock_or(e, TransportError::Io))?;
        let memo = self
            .own_next
            .get(&vault)
            .map_or(0, |next| next.saturating_sub(1));
        self.clear_above_reserved(vault, state.reserved)?;
        let floor = self.own_verified_high(vault)?.max(state.reserved).max(memo);
        let mut seq = floor.checked_add(1).ok_or(TransportError::Io)?;
        let dir = self.layout.log_dir(&vault, &self.install);
        for attempt in 0..MAX_NAME_ATTEMPTS {
            if !fsutil::exists(&dir.join(folder_names::segment_name(seq))) {
                if attempt > 0 {
                    self.force_compact.insert(vault);
                }
                return Ok(seq);
            }
            seq = seq.checked_add(1).ok_or(TransportError::Io)?;
        }
        Err(TransportError::Io)
    }

    /// Everything in the own log above `reserved` was never written by this
    /// device (a number is reserved before its file exists): files that do
    /// not verify go at once, with no age wait, and so do directories (with
    /// their content) and links (the link only). A file that verifies stays;
    /// the writer steps past it. This keeps planted names from stopping the
    /// writer, whatever their modification time.
    fn clear_above_reserved(
        &mut self,
        vault: VaultId,
        reserved: u64,
    ) -> Result<(), TransportError> {
        let install = self.install;
        let dir = self.layout.log_dir(&vault, &install);
        let Ok(read) = std::fs::read_dir(&dir) else {
            return Ok(());
        };
        for item in read.flatten() {
            let Some(FolderName::Segment { seq }) = folder_names::parse(&item.file_name()) else {
                continue;
            };
            if seq <= reserved {
                continue;
            }
            let path = item.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                fsutil::remove_squatter(&path);
                continue;
            }
            let file = SegmentFile {
                seq,
                len: meta.len(),
                modified: meta.modified().ok(),
            };
            // Read first: a file that cannot be read now is not a bad one.
            let _ = self.probe(vault, install, &file)?;
            if self.known_bad(vault, install, &file) {
                fsutil::remove_squatter(&path);
                self.scan.remove(&(vault, install, seq));
            }
        }
        Ok(())
    }

    /// Persists the reservation of `through` (before the write).
    fn reserve(&self, vault: VaultId, through: u64) -> Result<(), TransportError> {
        self.crypto
            .reserve_own_seq(vault, through)
            .map_err(|e| lock_or(e, TransportError::Io))
    }

    /// Persists that `seq` was written (after the write).
    fn confirm(&self, vault: VaultId, seq: u64) -> Result<(), TransportError> {
        self.crypto
            .confirm_own_seq(vault, seq)
            .map_err(|e| lock_or(e, TransportError::Io))
    }

    /// Whether a file at or above `written` is in the own log of `vault` and
    /// counts as present: it verifies, or it cannot be read right now (the
    /// sync client may not have delivered it whole). A file that was read in
    /// full and fails its MAC does not count: anyone with write access could
    /// plant one above `written` to hide a rollback.
    fn own_log_reaches(&mut self, vault: VaultId, written: u64) -> Result<bool, TransportError> {
        let install = self.install;
        let files = self.segment_files(&vault, &install)?;
        for file in files.iter().rev().take_while(|f| f.seq >= written).take(32) {
            let outcome = self.probe(vault, install, file)?;
            if matches!(outcome, super::Outcome::Good { .. })
                || !self.known_bad(vault, install, file)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether the own log of any of `vaults` lost its newest segments: the
    /// highest segment file of the log is below the highest sequence number
    /// this install wrote successfully. Anything listed at or above it
    /// (even a file that does not verify now) counts as present, so a file
    /// the sync client has not fully delivered is no rollback.
    pub(super) fn folder_own_logs_rolled_back(
        &mut self,
        vaults: &[VaultId],
    ) -> Result<bool, TransportError> {
        for vault in vaults {
            let state = self
                .crypto
                .own_seq(*vault)
                .map_err(|e| lock_or(e, TransportError::Io))?;
            if state.written == 0 {
                continue;
            }
            if !self.own_log_reaches(*vault, state.written)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn write_segment(
        &self,
        vault: VaultId,
        seq: u64,
        snapshot: Option<SnapshotInfo>,
        objects: Vec<SegmentObject>,
    ) -> Result<(), TransportError> {
        let segment = Segment {
            snapshot,
            account_id: self.account,
            vault_id: vault,
            device_id: self.install,
            seq,
            created_at_ms: self.now_ms(),
            objects,
        };
        let bytes = segment
            .encode(&self.crypto)
            .map_err(|e| lock_or(e, TransportError::Protocol))?;
        fsutil::write_atomic(
            &self.layout.log_dir(&vault, &self.install),
            &folder_names::segment_name(seq),
            &bytes,
            false,
        )
    }

    /// Appends the objects to this install's log as one or more segments.
    pub(crate) fn append_segments(
        &mut self,
        vault: VaultId,
        objects: Vec<SegmentObject>,
    ) -> Result<(), TransportError> {
        for objects in chunk(objects) {
            let seq = self.next_own_seq(vault)?;
            // Persist the number before the file exists: if the folder is
            // rolled back later, the number is never used again.
            self.reserve(vault, seq)?;
            self.write_segment(vault, seq, None, objects)?;
            self.confirm(vault, seq)?;
            self.own_next.insert(vault, seq.saturating_add(1));
        }
        Ok(())
    }

    /// Writes a snapshot as contiguous parts and returns the sequence number
    /// of its first part. Nothing is deleted.
    pub(crate) fn write_snapshot(
        &mut self,
        vault: VaultId,
        objects: Vec<SegmentObject>,
    ) -> Result<u64, TransportError> {
        let chunks = chunk(objects);
        let parts = u32::try_from(chunks.len()).map_err(|_| TransportError::Protocol)?;
        let mut base = self.next_own_seq(vault)?;
        // Every name of the range must be free: a range that collides with a
        // file that verifies (which the writer must not remove) is moved past
        // it.
        let dir = self.layout.log_dir(&vault, &self.install);
        let mut attempts = 0;
        let last = loop {
            let last = base
                .checked_add(u64::from(parts.saturating_sub(1)))
                .ok_or(TransportError::Io)?;
            let taken =
                (base..=last).find(|s| fsutil::exists(&dir.join(folder_names::segment_name(*s))));
            let Some(taken) = taken else { break last };
            attempts += 1;
            if attempts > MAX_NAME_ATTEMPTS {
                return Err(TransportError::Io);
            }
            self.force_compact.insert(vault);
            base = taken.checked_add(1).ok_or(TransportError::Io)?;
        };
        let info = SnapshotInfo { base, parts };
        // The whole range is reserved before the first part is written: a
        // set that fails half-way leaves the next set after its range, so the
        // two never overlap.
        self.reserve(vault, last)?;
        let mut seq = base;
        for objects in chunks {
            self.write_segment(vault, seq, Some(info), objects)?;
            self.confirm(vault, seq)?;
            seq = seq.checked_add(1).ok_or(TransportError::Io)?;
            self.own_next.insert(vault, seq);
        }
        Ok(base)
    }

    pub(super) fn folder_create_vault(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        meta: &VaultMetaCiphertext,
    ) -> Result<VaultCreateResult, TransportError> {
        if self.vault_has_key(vault)? {
            return Ok(VaultCreateResult::Exists);
        }
        self.append_segments(
            vault,
            vec![
                SegmentObject::VaultKey {
                    generation,
                    wrapped: wrapped.clone(),
                },
                SegmentObject::Meta(meta.clone()),
            ],
        )?;
        Ok(VaultCreateResult::Created)
    }

    pub(super) fn folder_push_records(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        records: &[PushRecord<'_>],
    ) -> Result<Vec<PushResult>, TransportError> {
        if records
            .iter()
            .any(|r| r.body.key_generation() != generation)
        {
            return Err(TransportError::GenerationMismatch {
                current: generation,
            });
        }
        if records.is_empty() {
            return Ok(Vec::new());
        }
        self.append_segments(vault, records.iter().map(record_object).collect())?;
        Ok(records
            .iter()
            .map(|_| PushResult::Accepted { seq: None })
            .collect())
    }

    pub(super) fn folder_push_meta(
        &mut self,
        vault: VaultId,
        meta: &VaultMetaCiphertext,
    ) -> Result<PushResult, TransportError> {
        self.append_segments(vault, vec![SegmentObject::Meta(meta.clone())])?;
        Ok(PushResult::Accepted { seq: None })
    }

    pub(super) fn folder_push_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
    ) -> Result<KeyPushResult, TransportError> {
        self.append_segments(
            vault,
            vec![SegmentObject::VaultKey {
                generation,
                wrapped: wrapped.clone(),
            }],
        )?;
        Ok(KeyPushResult::Accepted { seq: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_core::{VaultKey, VaultMetaCiphertext};

    fn meta_objects(n: usize, size: usize) -> Vec<SegmentObject> {
        let vault = VaultId::new_v7().unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        (0..n)
            .map(|_| {
                SegmentObject::Meta(
                    VaultMetaCiphertext::seal(&vk, &vault, &vec![1u8; size]).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn segments_hold_at_most_256_objects_in_order() {
        let chunks = chunk(meta_objects(600, 10));
        let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![256, 256, 88]);
        assert_eq!(chunk(Vec::new()).len(), 0);
    }
}
