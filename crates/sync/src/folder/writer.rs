//! Writing segments (ADR-0043): every push appends one or more segments to
//! this install's own log; compaction writes a snapshot as contiguous parts
//! and only then deletes the older segments (`maintain`).

use mosaic_core::{KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use mosaic_vault::storage::RecordBody;

use super::{FolderTransport, fsutil};
use crate::transport::{KeyPushResult, PushRecord, PushResult, TransportError, VaultCreateResult};
use crate::wire::SyncCrypto;
use crate::wire::folder_names;
use crate::wire::segment::{
    Segment, SegmentObject, SnapshotInfo, WRITE_MAX_OBJECTS, WRITE_MAX_SEGMENT_LEN,
};

/// The bytes of a segment besides its objects, with room to spare.
const SEGMENT_OVERHEAD: usize = 256;

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

impl<C: SyncCrypto + Send> FolderTransport<C> {
    /// The highest sequence number in this install's log of `vault` (0 if
    /// none).
    pub(crate) fn own_high_seq(&self, vault: VaultId) -> Result<u64, TransportError> {
        let on_disk = self
            .segment_files(&vault, &self.install)?
            .last()
            .map_or(0, |f| f.seq);
        let memo = self
            .own_next
            .get(&vault)
            .map_or(0, |next| next.saturating_sub(1));
        Ok(on_disk.max(memo))
    }

    fn next_own_seq(&self, vault: VaultId) -> Result<u64, TransportError> {
        self.own_high_seq(vault)?
            .checked_add(1)
            .ok_or(TransportError::Io)
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
            .map_err(|_| TransportError::Protocol)?;
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
            self.write_segment(vault, seq, None, objects)?;
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
        let base = self.next_own_seq(vault)?;
        let info = SnapshotInfo { base, parts };
        let mut seq = base;
        for objects in chunks {
            self.write_segment(vault, seq, Some(info), objects)?;
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
