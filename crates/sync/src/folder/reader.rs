//! Reading segments: the vault list, the existence check of a vault and the
//! pull (ADR-0043, CRYPTO_SPEC §10.1).
//!
//! A segment that fails to read, verify or parse is not marked read: it may
//! be a partial upload, and the next sync reads it again. Within one
//! process a failure is remembered for the file's length and modification
//! time, so a broken file is not re-read on every page.

use std::collections::BTreeMap;

use mosaic_core::{InstallId, KeyGeneration, VaultId};
use mosaic_vault::storage::RecordBody;

use super::fsutil::{self, Entry};
use super::{FolderCrypto, FolderTransport, Outcome, ScanEntry};
use crate::transport::{
    Cursor, PullPage, PullUnit, PulledKey, PulledMeta, PulledRecord, RemoteVault, TransportError,
};
use crate::wire::WireError;
use crate::wire::cursor::{DeviceCursor, FolderCursor, MAX_ABOVE};
use crate::wire::folder_names::{self, FolderName};
use crate::wire::segment::{MAX_SEGMENT_LEN, Segment, SegmentObject, SegmentPath, SnapshotInfo};

/// A segment file as a directory listing shows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SegmentFile {
    pub(crate) seq: u64,
    pub(crate) len: u64,
    pub(crate) modified: Option<std::time::SystemTime>,
}

/// Marks every sequence number up to `last` read.
fn mark_read_through(d: &mut DeviceCursor, last: u64) {
    if last > d.contiguous {
        d.contiguous = last;
        d.above.retain(|a| *a > last);
    }
    while let Some(&first) = d.above.first() {
        if d.contiguous.checked_add(1) == Some(first) {
            d.contiguous = first;
            d.above.remove(0);
        } else {
            break;
        }
    }
}

/// Decodes a stored cursor (empty is the start).
pub(crate) fn decode_cursor(cursor: &Cursor) -> Result<FolderCursor, TransportError> {
    if cursor.0.is_empty() {
        return Ok(FolderCursor::default());
    }
    FolderCursor::decode(&cursor.0).map_err(|_| TransportError::Protocol)
}

/// Encodes a cursor, dropping out-of-order sets beyond the format's bound
/// (re-reading a segment changes nothing).
pub(crate) fn encode_cursor(cursor: &FolderCursor) -> Result<Cursor, TransportError> {
    let mut trimmed = cursor.clone();
    for d in &mut trimmed.devices {
        d.above.truncate(MAX_ABOVE);
    }
    trimmed
        .encode()
        .map(Cursor)
        .map_err(|_| TransportError::Protocol)
}

/// The IDs of the directories in `entries`.
fn dir_ids(entries: &[Entry]) -> Vec<[u8; 16]> {
    entries
        .iter()
        .filter(|e| e.is_dir)
        .filter_map(|e| match folder_names::parse(&e.name) {
            Some(FolderName::Dir(id)) => Some(id),
            _ => None,
        })
        .collect()
}

impl<C: FolderCrypto> FolderTransport<C> {
    /// The vault directories.
    pub(crate) fn vault_ids(&self) -> Result<Vec<VaultId>, TransportError> {
        let entries = fsutil::list(&self.layout.vaults_dir())?;
        Ok(dir_ids(&entries)
            .into_iter()
            .filter_map(|id| VaultId::from_bytes(id).ok())
            .collect())
    }

    /// The devices with a log directory in `vault`, ascending.
    pub(crate) fn log_devices(&self, vault: &VaultId) -> Result<Vec<InstallId>, TransportError> {
        let entries = fsutil::list(&self.layout.logs_dir(vault))?;
        Ok(dir_ids(&entries)
            .into_iter()
            .filter_map(|id| InstallId::from_bytes(id).ok())
            .collect())
    }

    /// The segment files of one device's log, ascending by sequence number.
    pub(crate) fn segment_files(
        &self,
        vault: &VaultId,
        device: &InstallId,
    ) -> Result<Vec<SegmentFile>, TransportError> {
        let entries = fsutil::list(&self.layout.log_dir(vault, device))?;
        let mut out: Vec<SegmentFile> = entries
            .iter()
            .filter(|e| !e.is_dir)
            .filter_map(|e| match folder_names::parse(&e.name) {
                Some(FolderName::Segment { seq }) => Some(SegmentFile {
                    seq,
                    len: e.len,
                    modified: e.modified,
                }),
                _ => None,
            })
            .collect();
        out.sort_by_key(|f| f.seq);
        Ok(out)
    }

    fn remember(&mut self, key: (VaultId, InstallId, u64), file: &SegmentFile, outcome: Outcome) {
        self.scan.insert(
            key,
            ScanEntry {
                len: file.len,
                modified: file.modified,
                outcome,
            },
        );
    }

    /// Reads, verifies and parses one segment; `None` if it does not
    /// verify or parse (to be retried). The outcome is cached, except while
    /// the session is locked: then nothing can be concluded about the file.
    ///
    /// # Errors
    /// [`TransportError::Locked`].
    pub(crate) fn read_segment(
        &mut self,
        vault: VaultId,
        device: InstallId,
        file: &SegmentFile,
    ) -> Result<Option<Segment>, TransportError> {
        let key = (vault, device, file.seq);
        if let Some(entry) = self.scan.get(&key)
            && entry.len == file.len
            && entry.modified == file.modified
            && matches!(entry.outcome, Outcome::Bad)
        {
            return Ok(None);
        }
        let path = self
            .layout
            .log_dir(&vault, &device)
            .join(folder_names::segment_name(file.seq));
        let segment = match fsutil::read_capped(&path, MAX_SEGMENT_LEN as u64) {
            Ok(Some(bytes)) => {
                let decoded = Segment::decode(
                    &bytes,
                    &SegmentPath {
                        account_id: self.account,
                        vault_id: vault,
                        device_id: device,
                        seq: file.seq,
                    },
                    &self.crypto,
                );
                match decoded {
                    Err(WireError::Locked) => return Err(TransportError::Locked),
                    other => other.ok(),
                }
            }
            // Too long for a segment: bad content, remembered below.
            Err(TransportError::Protocol) => None,
            // Gone, not a regular file, or locked by the sync client for the
            // moment: not remembered, so the next sync tries again.
            Ok(None) | Err(_) => return Ok(None),
        };
        let outcome = segment.as_ref().map_or(Outcome::Bad, |s| {
            let mut max_generation: Option<KeyGeneration> = None;
            let mut has_key = false;
            for o in &s.objects {
                if let SegmentObject::VaultKey { generation, .. } = o {
                    has_key = true;
                    max_generation = max_generation.max(Some(*generation));
                }
            }
            Outcome::Good {
                max_generation,
                has_key,
                snapshot: s.snapshot,
            }
        });
        self.remember(key, file, outcome);
        Ok(segment)
    }

    /// The outcome of a segment file, from the cache when it is current.
    ///
    /// # Errors
    /// [`TransportError::Locked`].
    pub(crate) fn probe(
        &mut self,
        vault: VaultId,
        device: InstallId,
        file: &SegmentFile,
    ) -> Result<Outcome, TransportError> {
        if let Some(entry) = self.scan.get(&(vault, device, file.seq))
            && entry.len == file.len
            && entry.modified == file.modified
        {
            return Ok(entry.outcome);
        }
        let _ = self.read_segment(vault, device, file)?;
        Ok(self
            .scan
            .get(&(vault, device, file.seq))
            .map_or(Outcome::Bad, |e| e.outcome))
    }

    /// What the verified segments of a vault say. Stops reading as soon as
    /// `stop` is met: a vault is listed once one segment verifies, and it
    /// exists once one carries a vault key.
    fn vault_summary(
        &mut self,
        vault: VaultId,
        stop: Stop,
    ) -> Result<VaultSummary, TransportError> {
        let mut summary = VaultSummary::default();
        for device in self.log_devices(&vault)? {
            for file in self.segment_files(&vault, &device)? {
                if let Outcome::Good {
                    max_generation,
                    has_key,
                    ..
                } = self.probe(vault, device, &file)?
                {
                    summary.verified = true;
                    summary.has_key |= has_key;
                    summary.max_generation = summary.max_generation.max(max_generation);
                }
                if match stop {
                    Stop::FirstVerified => summary.verified,
                    Stop::Key => summary.has_key,
                } {
                    return Ok(summary);
                }
            }
        }
        Ok(summary)
    }

    pub(super) fn folder_list_vaults(&mut self) -> Result<Vec<RemoteVault>, TransportError> {
        let mut out = Vec::new();
        for vault in self.vault_ids()? {
            let summary = self.vault_summary(vault, Stop::FirstVerified)?;
            if summary.verified {
                out.push(RemoteVault {
                    vault_id: vault,
                    generation: summary.max_generation.unwrap_or(KeyGeneration::FIRST),
                });
            }
        }
        Ok(out)
    }

    /// Whether any verified segment of the vault carries a vault key.
    pub(super) fn vault_has_key(&mut self, vault: VaultId) -> Result<bool, TransportError> {
        Ok(self.vault_summary(vault, Stop::Key)?.has_key)
    }

    pub(super) fn folder_pull(
        &mut self,
        vault: VaultId,
        cursor: &Cursor,
        limit: usize,
    ) -> Result<PullPage, TransportError> {
        let mut cur = decode_cursor(cursor)?;
        let mut page = PullPage {
            keys: Vec::new(),
            metas: Vec::new(),
            records: Vec::new(),
            next: Cursor::default(),
            more: false,
            units: Vec::new(),
        };
        let mut objects = 0usize;
        'devices: for device in self.log_devices(&vault)? {
            let files = self.segment_files(&vault, &device)?;
            for file in &files {
                if cur.device(&device).is_some_and(|d| d.has_read(file.seq)) {
                    continue;
                }
                if objects >= limit {
                    page.more = true;
                    break 'devices;
                }
                let Some(segment) = self.read_segment(vault, device, file)? else {
                    continue;
                };
                self.mark_segment_read(vault, device, &segment, &files, &mut cur)?;
                let first_record = page.records.len();
                if device == self.install {
                    // This install's own log comes back for its vault keys
                    // only: a rotation that was pushed but not committed
                    // before a crash is otherwise never learned (the records
                    // are this device's own already).
                    let keys = segment
                        .objects
                        .into_iter()
                        .filter(|o| matches!(o, SegmentObject::VaultKey { .. }))
                        .collect();
                    add_objects(&mut page, device, keys);
                } else {
                    objects = objects.saturating_add(segment.objects.len());
                    add_objects(&mut page, device, segment.objects);
                }
                page.units.push(PullUnit {
                    first_record,
                    end_record: page.records.len(),
                    commit: encode_cursor(&cur)?,
                });
            }
        }
        page.next = encode_cursor(&cur)?;
        Ok(page)
    }

    /// Marks a verified segment read and, when it belongs to a snapshot set
    /// whose parts are all on disk and verified, every older segment of the
    /// device too (the numbers below the set's first part). The completeness is read from the folder each time a part
    /// arrives, never from memory: the cursor commits only through the units
    /// the engine applied, so a set that completes in a unit that is
    /// deferred, or across a relock, completes the next time it is read.
    fn mark_segment_read(
        &mut self,
        vault: VaultId,
        device: InstallId,
        segment: &Segment,
        files: &[SegmentFile],
        cur: &mut FolderCursor,
    ) -> Result<(), TransportError> {
        let through = match segment.snapshot {
            // The gaps below the set are closed: the set holds the writer's
            // whole state. The parts themselves are read one by one (each is
            // applied before its number is marked), never marked in advance.
            Some(info) if self.snapshot_set_complete(vault, device, info, files)? => {
                Some(info.base.saturating_sub(1))
            }
            _ => None,
        };
        let d = cur
            .device_mut(device)
            .map_err(|_| TransportError::Protocol)?;
        d.mark_read(segment.seq);
        if let Some(last) = through {
            mark_read_through(d, last);
        }
        Ok(())
    }

    /// Whether every part `base ..= base + parts - 1` of the set is a listed
    /// file that verifies and names this very set. Sets that overlap or
    /// disagree cannot both be complete, because a sequence number is one
    /// file.
    fn snapshot_set_complete(
        &mut self,
        vault: VaultId,
        device: InstallId,
        info: SnapshotInfo,
        files: &[SegmentFile],
    ) -> Result<bool, TransportError> {
        let by_seq: BTreeMap<u64, &SegmentFile> = files.iter().map(|f| (f.seq, f)).collect();
        for offset in 0..u64::from(info.parts) {
            let Some(seq) = info.base.checked_add(offset) else {
                return Ok(false);
            };
            let Some(file) = by_seq.get(&seq) else {
                return Ok(false);
            };
            match self.probe(vault, device, file)? {
                Outcome::Good {
                    snapshot: Some(found),
                    ..
                } if found == info => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

/// When to stop reading a vault's segments.
#[derive(Clone, Copy)]
enum Stop {
    /// At the first segment that verifies.
    FirstVerified,
    /// At the first segment that carries a vault key.
    Key,
}

/// What the segments of one vault say.
#[derive(Default)]
struct VaultSummary {
    verified: bool,
    has_key: bool,
    max_generation: Option<KeyGeneration>,
}

/// Appends a segment's objects to the page.
fn add_objects(page: &mut PullPage, device: InstallId, objects: Vec<SegmentObject>) {
    for object in objects {
        match object {
            SegmentObject::Item {
                record_id,
                ciphertext,
            } => page.records.push(PulledRecord {
                record_id,
                body: RecordBody::Item(ciphertext),
                seq: None,
            }),
            SegmentObject::Tombstone {
                record_id,
                ciphertext,
            } => page.records.push(PulledRecord {
                record_id,
                body: RecordBody::Tombstone(ciphertext),
                seq: None,
            }),
            SegmentObject::VaultKey {
                generation,
                wrapped,
            } => page.keys.push(PulledKey {
                generation,
                wrapped,
                device_id: device,
                seq: None,
            }),
            SegmentObject::Meta(ciphertext) => page.metas.push(PulledMeta {
                ciphertext,
                seq: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_core::InstallId;

    fn device() -> InstallId {
        InstallId::new_v7().unwrap()
    }

    #[test]
    fn a_snapshot_closes_the_gaps_below_it() {
        let mut d = DeviceCursor {
            device_id: device(),
            contiguous: 3,
            above: vec![6, 9, 12],
        };
        mark_read_through(&mut d, 8);
        // The number that meets the mark is folded in.
        assert_eq!((d.contiguous, d.above.clone()), (9, vec![12]));
        // Marking below the mark changes nothing.
        mark_read_through(&mut d, 2);
        assert_eq!(d.contiguous, 9);
    }

    #[test]
    fn a_cursor_with_too_many_gaps_is_trimmed_not_refused() {
        let mut c = FolderCursor::default();
        let d = c.device_mut(device()).unwrap();
        d.contiguous = 1;
        d.above = (0..(MAX_ABOVE as u64 + 50)).map(|i| 10 + 2 * i).collect();
        let bytes = encode_cursor(&c).unwrap();
        let back = decode_cursor(&bytes).unwrap();
        assert_eq!(back.devices[0].above.len(), MAX_ABOVE);
        assert_eq!(
            decode_cursor(&Cursor::default()).unwrap(),
            FolderCursor::default()
        );
        assert!(decode_cursor(&Cursor(vec![9])).is_err());
    }
}
