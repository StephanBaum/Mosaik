//! The folder backend's segment file `.mseg` (CRYPTO_SPEC §10.1,
//! ADR-0043): a device's append-only batch of records, vault keys and
//! metadata for one vault.
//!
//! ```text
//! "MSEG" || 1 || flags:u8 (bit0 = snapshot) || account_id[16] || vault_id[16]
//! || device_id[16] || seq:u64 (>= 1) || created_at_ms:u64
//! || [ snap_base:u64 || snap_parts:u32 ]            (only when bit0 is set)
//! || count:u32 (1..=4096)
//! || count x ( kind:u8 || len:u32 || body[len] ) || mac[32]
//! mac = HMAC(K_fold, ctx("sync/folder-segment") || 0x00 || all preceding bytes)
//! kind 1 item        body = record_id[16] || ItemCiphertext
//! kind 2 tombstone   body = record_id[16] || TombstoneCiphertext
//! kind 3 vault key   body = generation:u32 || WrappedKey[74]
//! kind 4 vault meta  body = VaultMetaCiphertext
//! ```
//!
//! A snapshot may span several parts, each a segment of its own. The part
//! index is `seq - snap_base`, with `1 <= snap_parts <= 65536` and
//! `1 <= snap_base <= seq < snap_base + snap_parts`. [`SnapshotTracker`]
//! decides when a set of parts is complete and a reader may mark the
//! device's older sequence numbers read.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{
    AccountId, InstallId, ItemCiphertext, ItemId, KeyGeneration, TombstoneCiphertext, VaultId,
    VaultMetaCiphertext, WrappedKey,
};

use super::{Reader, SyncCrypto, WireError, split_mac};

const MAGIC: &[u8; 4] = b"MSEG";
const VERSION: u8 = 1;
const FLAG_SNAPSHOT: u8 = 0b0000_0001;
/// `magic || version || flags || account || vault || device || seq || created || count`,
/// without the snapshot fields.
const HEADER_LEN: usize = 4 + 1 + 1 + 16 + 16 + 16 + 8 + 8 + 4;
/// `snap_base || snap_parts`, present only in a snapshot part.
const SNAPSHOT_FIELDS_LEN: usize = 8 + 4;
/// The most parts one snapshot set has.
pub const MAX_SNAPSHOT_PARTS: u32 = 65_536;
/// The most a reader accepts of one segment file.
pub const MAX_SEGMENT_LEN: usize = 16 * 1024 * 1024;
/// The most objects one segment holds.
pub const MAX_OBJECTS: usize = 4096;
/// The most a writer puts into one segment (it starts another beyond it).
pub const WRITE_MAX_SEGMENT_LEN: usize = 8 * 1024 * 1024;
/// The most objects a writer puts into one (non-snapshot) segment.
pub const WRITE_MAX_OBJECTS: usize = 256;
/// The longest body of an item, tombstone or metadata object: the largest
/// plaintext (1 MiB) plus the envelope's overhead.
pub const MAX_OBJECT_LEN: usize = 1_048_576 + 64;
/// The length of a vault-key object's body.
const KEY_BODY_LEN: usize = 4 + WrappedKey::LEN;

/// One object of a segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SegmentObject {
    /// An item record.
    Item {
        /// The record ID.
        record_id: ItemId,
        /// The ciphertext.
        ciphertext: ItemCiphertext,
    },
    /// A tombstone.
    Tombstone {
        /// The record ID.
        record_id: ItemId,
        /// The ciphertext.
        ciphertext: TombstoneCiphertext,
    },
    /// A wrapped vault key.
    VaultKey {
        /// Its generation.
        generation: KeyGeneration,
        /// The key wrapped under the account key.
        wrapped: WrappedKey,
    },
    /// The vault's metadata.
    Meta(VaultMetaCiphertext),
}

impl SegmentObject {
    const fn kind(&self) -> u8 {
        match self {
            Self::Item { .. } => 1,
            Self::Tombstone { .. } => 2,
            Self::VaultKey { .. } => 3,
            Self::Meta(_) => 4,
        }
    }

    fn encode_body(&self, out: &mut Vec<u8>) {
        match self {
            Self::Item {
                record_id,
                ciphertext,
            } => {
                out.extend_from_slice(record_id.as_bytes());
                out.extend_from_slice(ciphertext.as_bytes());
            }
            Self::Tombstone {
                record_id,
                ciphertext,
            } => {
                out.extend_from_slice(record_id.as_bytes());
                out.extend_from_slice(ciphertext.as_bytes());
            }
            Self::VaultKey {
                generation,
                wrapped,
            } => {
                out.extend_from_slice(&generation.get().to_be_bytes());
                out.extend_from_slice(wrapped.as_bytes());
            }
            Self::Meta(meta) => out.extend_from_slice(meta.as_bytes()),
        }
    }

    fn decode(kind: u8, body: &[u8]) -> Result<Self, WireError> {
        match kind {
            1 | 2 => {
                let mut r = Reader::new(body);
                let id = r.array::<16>()?;
                let record_id = ItemId::from_bytes(id).map_err(|_| WireError::Id)?;
                let rest = r.bytes(r.remaining())?;
                Ok(if kind == 1 {
                    Self::Item {
                        record_id,
                        ciphertext: ItemCiphertext::from_bytes(rest)
                            .map_err(|_| WireError::Ciphertext)?,
                    }
                } else {
                    Self::Tombstone {
                        record_id,
                        ciphertext: TombstoneCiphertext::from_bytes(rest)
                            .map_err(|_| WireError::Ciphertext)?,
                    }
                })
            }
            3 => {
                if body.len() != KEY_BODY_LEN {
                    return Err(WireError::Length);
                }
                let mut r = Reader::new(body);
                let generation = KeyGeneration::new(r.u32()?).map_err(|_| WireError::Field)?;
                let wrapped = WrappedKey::from_bytes(r.bytes(WrappedKey::LEN)?)
                    .map_err(|_| WireError::Ciphertext)?;
                r.finish()?;
                Ok(Self::VaultKey {
                    generation,
                    wrapped,
                })
            }
            4 => Ok(Self::Meta(
                VaultMetaCiphertext::from_bytes(body).map_err(|_| WireError::Ciphertext)?,
            )),
            _ => Err(WireError::Field),
        }
    }
}

/// The snapshot fields of a segment: it is one part of the set of
/// `parts` segments with the contiguous sequence numbers
/// `base ..= base + parts - 1` (CRYPTO_SPEC §10.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotInfo {
    /// The sequence number of the set's first part (at least 1).
    pub base: u64,
    /// How many parts the set has, 1 to [`MAX_SNAPSHOT_PARTS`].
    pub parts: u32,
}

impl SnapshotInfo {
    /// The highest sequence number of the set.
    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.base
            .saturating_add(u64::from(self.parts.saturating_sub(1)))
    }

    /// Checks the bounds for a part with sequence number `seq`.
    fn check(&self, seq: u64) -> Result<(), WireError> {
        let in_set = self.base >= 1
            && self.base <= seq
            && seq - self.base < u64::from(self.parts)
            && self.base.checked_add(u64::from(self.parts)).is_some();
        if (1..=MAX_SNAPSHOT_PARTS).contains(&self.parts) && in_set {
            Ok(())
        } else {
            Err(WireError::Field)
        }
    }
}

/// The IDs and sequence number of a segment as its path names them
/// (`a/<account>/v/<vault>/log/<device>/<seq>.mseg`); the file must say the
/// same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentPath {
    /// The account directory.
    pub account_id: AccountId,
    /// The vault directory.
    pub vault_id: VaultId,
    /// The device's log directory.
    pub device_id: InstallId,
    /// The sequence number in the file name.
    pub seq: u64,
}

/// A segment: one device's batch of objects for one vault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// `Some` for a part of a snapshot: the writer's whole current state of
    /// the vault. A reader marks every sequence number of that device up to
    /// the set's last one as read once all parts are verified
    /// ([`SnapshotTracker`]).
    pub snapshot: Option<SnapshotInfo>,
    /// The account.
    pub account_id: AccountId,
    /// The vault.
    pub vault_id: VaultId,
    /// The writing install.
    pub device_id: InstallId,
    /// The device's sequence number of this segment (at least 1).
    pub seq: u64,
    /// When it was written (ms since the Unix epoch).
    pub created_at_ms: u64,
    /// The objects, 1 to [`MAX_OBJECTS`].
    pub objects: Vec<SegmentObject>,
}

impl Segment {
    /// Encodes and MACs the segment.
    ///
    /// # Errors
    /// [`WireError::Field`] for a sequence number of 0 or a count outside
    /// 1 to [`MAX_OBJECTS`] or snapshot fields outside their bounds;
    /// [`WireError::Length`] if an object or the file
    /// exceeds the reader's limits; [`WireError::Crypto`].
    pub fn encode(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        if self.seq == 0 || self.objects.is_empty() || self.objects.len() > MAX_OBJECTS {
            return Err(WireError::Field);
        }
        let count = u32::try_from(self.objects.len()).map_err(|_| WireError::Field)?;
        if let Some(info) = &self.snapshot {
            info.check(self.seq)?;
        }
        let mut out = Vec::with_capacity(HEADER_LEN + SNAPSHOT_FIELDS_LEN);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(if self.snapshot.is_some() {
            FLAG_SNAPSHOT
        } else {
            0
        });
        out.extend_from_slice(self.account_id.as_bytes());
        out.extend_from_slice(self.vault_id.as_bytes());
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&self.created_at_ms.to_be_bytes());
        if let Some(info) = &self.snapshot {
            out.extend_from_slice(&info.base.to_be_bytes());
            out.extend_from_slice(&info.parts.to_be_bytes());
        }
        out.extend_from_slice(&count.to_be_bytes());
        let mut body = Vec::new();
        for object in &self.objects {
            body.clear();
            object.encode_body(&mut body);
            let len = u32::try_from(body.len()).map_err(|_| WireError::Length)?;
            if object.kind() != 3 && body.len() > MAX_OBJECT_LEN {
                return Err(WireError::Length);
            }
            out.push(object.kind());
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&body);
            if out.len().saturating_add(32) > MAX_SEGMENT_LEN {
                return Err(WireError::Length);
            }
        }
        let mac = crypto.folder_mac(FolderMacPurpose::Segment, &out)?;
        out.extend_from_slice(&mac);
        Ok(out)
    }

    /// Decodes a segment file: length bounds, MAC, then the content, which
    /// must match `path`.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Mac`], [`WireError::Magic`],
    /// [`WireError::Version`], [`WireError::Field`], [`WireError::Id`],
    /// [`WireError::Path`], [`WireError::Ciphertext`],
    /// [`WireError::Crypto`].
    pub fn decode(
        bytes: &[u8],
        path: &SegmentPath,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        if bytes.len() < HEADER_LEN + 32 || bytes.len() > MAX_SEGMENT_LEN {
            return Err(WireError::Length);
        }
        let (body, mac) = split_mac(bytes)?;
        if !crypto.verify_folder_mac(FolderMacPurpose::Segment, body, &mac)? {
            return Err(WireError::Mac);
        }
        let mut r = Reader::new(body);
        if &r.array::<4>()? != MAGIC {
            return Err(WireError::Magic);
        }
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let flags = r.u8()?;
        if flags & !FLAG_SNAPSHOT != 0 {
            return Err(WireError::Field);
        }
        let account_id = AccountId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let vault_id = VaultId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let device_id = InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let seq = r.u64()?;
        let created_at_ms = r.u64()?;
        let snapshot = if flags & FLAG_SNAPSHOT != 0 {
            Some(SnapshotInfo {
                base: r.u64()?,
                parts: r.u32()?,
            })
        } else {
            None
        };
        let count = usize::try_from(r.u32()?).map_err(|_| WireError::Field)?;
        if seq == 0 || count == 0 || count > MAX_OBJECTS {
            return Err(WireError::Field);
        }
        if let Some(info) = &snapshot {
            info.check(seq)?;
        }
        if (account_id, vault_id, device_id, seq)
            != (path.account_id, path.vault_id, path.device_id, path.seq)
        {
            return Err(WireError::Path);
        }
        // Every object takes at least five bytes: a count the rest cannot
        // hold is refused before anything is allocated.
        if count.saturating_mul(5) > r.remaining() {
            return Err(WireError::Length);
        }
        let mut objects = Vec::with_capacity(count);
        for _ in 0..count {
            let kind = r.u8()?;
            let len = usize::try_from(r.u32()?).map_err(|_| WireError::Length)?;
            if kind != 3 && len > MAX_OBJECT_LEN {
                return Err(WireError::Length);
            }
            objects.push(SegmentObject::decode(kind, r.bytes(len)?)?);
        }
        r.finish()?;
        Ok(Self {
            snapshot,
            account_id,
            vault_id,
            device_id,
            seq,
            created_at_ms,
            objects,
        })
    }
}

/// Collects the verified snapshot parts of one device's log in one vault
/// and says when a set is complete (CRYPTO_SPEC §10.1).
///
/// A reader applies the records of every verified segment at once and marks
/// that segment's sequence number read. A snapshot part additionally lets it
/// mark every older sequence number read, but only when all parts of its set
/// are in: until then the older segments are still needed, because the
/// parts that are missing may hold state they do not. A set is identified by
/// `(snap_base, snap_parts)`; a part that disagrees with the parts seen
/// before, or whose range overlaps another set's, is refused.
#[derive(Debug, Default)]
pub struct SnapshotTracker {
    /// `snap_base` to the set that starts there. Sets never overlap.
    sets: BTreeMap<u64, TrackedSet>,
}

#[derive(Debug)]
struct TrackedSet {
    parts: u32,
    /// The sequence numbers of the parts seen (cleared once complete).
    seen: BTreeSet<u64>,
    complete: bool,
}

impl SnapshotTracker {
    /// An empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notes a verified, parsed segment of the device this tracker is for.
    /// Returns `Some(last_seq)` when this segment completes a snapshot set:
    /// the reader may mark every sequence number of the device up to
    /// `last_seq` read. Returns `None` for any other segment (a segment
    /// without snapshot fields, a part of an incomplete set, or a part that
    /// was seen before): the reader marks only the segment's own sequence
    /// number read.
    ///
    /// # Errors
    /// [`WireError::Field`] if the part disagrees with the set it names
    /// (another `snap_parts` for the same `snap_base`, or a range that
    /// overlaps a different set).
    pub fn add(&mut self, segment: &Segment) -> Result<Option<u64>, WireError> {
        let Some(info) = segment.snapshot else {
            return Ok(None);
        };
        info.check(segment.seq)?;
        let last = info.last_seq();
        // Sets are disjoint ranges, so only the neighbours of `base` can
        // overlap the new one.
        if let Some((base, set)) = self.sets.range(..=info.base).next_back() {
            let same = *base == info.base && set.parts == info.parts;
            let end = base.saturating_add(u64::from(set.parts.saturating_sub(1)));
            if !same && end >= info.base {
                return Err(WireError::Field);
            }
        }
        if let Some((base, set)) = self.sets.range(info.base..).next() {
            let same = *base == info.base && set.parts == info.parts;
            if !same && *base <= last {
                return Err(WireError::Field);
            }
        }
        let set = self.sets.entry(info.base).or_insert_with(|| TrackedSet {
            parts: info.parts,
            seen: BTreeSet::new(),
            complete: false,
        });
        if set.complete {
            return Ok(None);
        }
        set.seen.insert(segment.seq);
        if u64::try_from(set.seen.len()) == Ok(u64::from(set.parts)) {
            set.complete = true;
            set.seen.clear();
            return Ok(Some(last));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountKey, VaultKey};

    use super::*;
    use crate::wire::KeyedCrypto;

    struct Fixture {
        crypto: KeyedCrypto,
        vault_key: VaultKey,
        path: SegmentPath,
        item_id: ItemId,
    }

    fn fixture() -> Fixture {
        let ak = AccountKey::generate().unwrap();
        let account_id = AccountId::new_v7().unwrap();
        let device_id = InstallId::new_v7().unwrap();
        Fixture {
            crypto: KeyedCrypto::new(&ak, account_id, device_id).unwrap(),
            vault_key: VaultKey::generate(KeyGeneration::FIRST).unwrap(),
            path: SegmentPath {
                account_id,
                vault_id: VaultId::new_v7().unwrap(),
                device_id,
                seq: 3,
            },
            item_id: ItemId::new_v7().unwrap(),
        }
    }

    fn segment(f: &Fixture) -> Segment {
        let vk = &f.vault_key;
        let vault = f.path.vault_id;
        let tomb_id = ItemId::new_v7().unwrap();
        Segment {
            snapshot: None,
            account_id: f.path.account_id,
            vault_id: vault,
            device_id: f.path.device_id,
            seq: f.path.seq,
            created_at_ms: 1_700_000_000_000,
            objects: vec![
                SegmentObject::VaultKey {
                    generation: KeyGeneration::FIRST,
                    wrapped: vk
                        .wrap(&AccountKey::generate().unwrap(), &f.path.account_id, &vault)
                        .unwrap(),
                },
                SegmentObject::Meta(
                    VaultMetaCiphertext::seal(vk, &vault, b"{\"meta\":1}").unwrap(),
                ),
                SegmentObject::Item {
                    record_id: f.item_id,
                    ciphertext: ItemCiphertext::seal(vk, &vault, &f.item_id, 1, b"item").unwrap(),
                },
                SegmentObject::Tombstone {
                    record_id: tomb_id,
                    ciphertext: TombstoneCiphertext::seal(vk, &vault, &tomb_id, b"tomb").unwrap(),
                },
            ],
        }
    }

    #[test]
    fn round_trip() {
        let f = fixture();
        let mut s = segment(&f);
        let bytes = s.encode(&f.crypto).unwrap();
        assert_eq!(&bytes[..5], b"MSEG\x01");
        assert_eq!(Segment::decode(&bytes, &f.path, &f.crypto).unwrap(), s);
        let plain_len = bytes.len();
        s.snapshot = Some(SnapshotInfo { base: 2, parts: 3 });
        let bytes = s.encode(&f.crypto).unwrap();
        assert_eq!(bytes[5], 1);
        assert_eq!(bytes.len(), plain_len + SNAPSHOT_FIELDS_LEN);
        // base and parts follow created_at_ms, before count.
        let at = HEADER_LEN - 4;
        assert_eq!(bytes[at..at + 8], 2u64.to_be_bytes());
        assert_eq!(bytes[at + 8..at + 12], 3u32.to_be_bytes());
        assert_eq!(
            Segment::decode(&bytes, &f.path, &f.crypto)
                .unwrap()
                .snapshot,
            Some(SnapshotInfo { base: 2, parts: 3 })
        );
    }

    /// The segment of `f` as part `seq` of the snapshot set `(base, parts)`.
    fn part(f: &Fixture, seq: u64, base: u64, parts: u32) -> Segment {
        let mut s = segment(f);
        s.seq = seq;
        s.snapshot = Some(SnapshotInfo { base, parts });
        s
    }

    fn path_of(f: &Fixture, seq: u64) -> SegmentPath {
        SegmentPath { seq, ..f.path }
    }

    /// The snapshot bounds hold in the encoder and the decoder.
    #[test]
    fn snapshot_bounds() {
        let f = fixture();
        let ok = |seq: u64, base: u64, parts: u32| {
            let encoded = part(&f, seq, base, parts).encode(&f.crypto).is_ok();
            (encoded,)
        };
        // Inside: the first and the last part, one part, the maximum count.
        assert!(ok(3, 3, 1).0);
        assert!(ok(3, 3, 3).0);
        assert!(ok(5, 3, 3).0);
        assert!(ok(1, 1, MAX_SNAPSHOT_PARTS).0);
        assert!(ok(MAX_SNAPSHOT_PARTS.into(), 1, MAX_SNAPSHOT_PARTS).0);
        // Outside: no parts, too many parts, seq below the base, seq past the
        // last part, base 0, and an overflowing set.
        assert!(!ok(3, 3, 0).0);
        assert!(!ok(3, 3, MAX_SNAPSHOT_PARTS + 1).0);
        assert!(!ok(2, 3, 3).0);
        assert!(!ok(6, 3, 3).0);
        assert!(!ok(3, 0, 5).0);
        assert!(!ok(u64::MAX, u64::MAX - 1, 5).0);
        assert!(!ok(u64::from(MAX_SNAPSHOT_PARTS) + 1, 1, MAX_SNAPSHOT_PARTS).0);

        // The decoder refuses what the encoder would not write: forge the
        // fields behind a valid MAC.
        let good = part(&f, 4, 3, 2).encode(&f.crypto).unwrap();
        let (body, _) = good.split_at(good.len() - 32);
        let at = HEADER_LEN - 4;
        let forge_fields = |base: u64, parts: u32| {
            let mut b = body.to_vec();
            b[at..at + 8].copy_from_slice(&base.to_be_bytes());
            b[at + 8..at + 12].copy_from_slice(&parts.to_be_bytes());
            forge(&f, b)
        };
        let decode = |b: &[u8]| Segment::decode(b, &path_of(&f, 4), &f.crypto);
        assert!(decode(&forge_fields(3, 2)).is_ok());
        for (base, parts) in [
            (3, 0),
            (3, MAX_SNAPSHOT_PARTS + 1),
            (5, 2),
            (2, 2),
            (0, 5),
            (3, 1),
            (u64::MAX, 2),
        ] {
            assert_eq!(
                decode(&forge_fields(base, parts)),
                Err(WireError::Field),
                "{base} {parts}"
            );
        }
        // The flag clear means no fields: a flagged file without them is
        // short, and fields without the flag shift the count.
        let mut unflagged = body.to_vec();
        unflagged[5] = 0;
        assert!(decode(&forge(&f, unflagged)).is_err());
    }

    /// Parts arrive in reverse order and an older segment is missing:
    /// nothing below `snap_base` is marked read until the last part is in.
    #[test]
    fn snapshot_parts_in_reverse_order() {
        let f = fixture();
        let mut t = SnapshotTracker::new();
        // Seqs 1 and 2 are older segments that never arrive; the set is
        // seqs 3..=5.
        assert_eq!(t.add(&part(&f, 5, 3, 3)), Ok(None));
        assert_eq!(t.add(&part(&f, 4, 3, 3)), Ok(None));
        // A repeated part completes nothing.
        assert_eq!(t.add(&part(&f, 4, 3, 3)), Ok(None));
        assert_eq!(t.add(&part(&f, 3, 3, 3)), Ok(Some(5)));
        // Once complete, a repeat is not a new completion.
        assert_eq!(t.add(&part(&f, 3, 3, 3)), Ok(None));
        // A plain segment never completes anything.
        assert_eq!(t.add(&segment(&f)), Ok(None));
        // A one-part snapshot completes at once.
        assert_eq!(t.add(&part(&f, 9, 9, 1)), Ok(Some(9)));
    }

    #[test]
    fn a_part_that_disagrees_is_refused() {
        let f = fixture();
        let mut t = SnapshotTracker::new();
        assert_eq!(t.add(&part(&f, 3, 3, 3)), Ok(None));
        // Another part count for the same base.
        assert_eq!(t.add(&part(&f, 4, 3, 2)), Err(WireError::Field));
        assert_eq!(t.add(&part(&f, 4, 3, 4)), Err(WireError::Field));
        // A different set that starts inside, or reaches into, the first.
        assert_eq!(t.add(&part(&f, 4, 4, 2)), Err(WireError::Field));
        assert_eq!(t.add(&part(&f, 2, 2, 2)), Err(WireError::Field));
        // The set is not harmed by the refused parts.
        assert_eq!(t.add(&part(&f, 4, 3, 3)), Ok(None));
        assert_eq!(t.add(&part(&f, 5, 3, 3)), Ok(Some(5)));
        // The next set may start right after the last part.
        assert_eq!(t.add(&part(&f, 6, 6, 2)), Ok(None));
        assert_eq!(t.add(&part(&f, 7, 6, 2)), Ok(Some(7)));
        // But not inside a completed one.
        assert_eq!(t.add(&part(&f, 5, 5, 1)), Err(WireError::Field));
    }

    /// Re-MACs `body` so a test can corrupt the content and still pass the
    /// MAC, exercising the parser behind it.
    fn forge(f: &Fixture, mut body: Vec<u8>) -> Vec<u8> {
        let mac = f
            .crypto
            .folder_mac(FolderMacPurpose::Segment, &body)
            .unwrap();
        body.extend_from_slice(&mac);
        body
    }

    #[test]
    fn rejections() {
        let f = fixture();
        let bytes = segment(&f).encode(&f.crypto).unwrap();
        let decode = |b: &[u8]| Segment::decode(b, &f.path, &f.crypto);

        // Bad MAC: any flipped bit, and a short or empty input.
        for i in [0, 4, 5, 30, bytes.len() / 2, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(decode(&bad), Err(WireError::Mac), "byte {i}");
        }
        assert_eq!(decode(&[]), Err(WireError::Length));
        assert_eq!(decode(&bytes[..HEADER_LEN + 31]), Err(WireError::Length));
        // Trailing bytes (with a MAC over them) are refused.
        let (body, _) = bytes.split_at(bytes.len() - 32);
        let mut longer = body.to_vec();
        longer.push(0);
        assert_eq!(decode(&forge(&f, longer)), Err(WireError::Length));
        // An object count the content does not hold.
        let mut few = body.to_vec();
        few[HEADER_LEN - 1] = 5;
        assert_eq!(decode(&forge(&f, few)), Err(WireError::Length));
        let mut zero = body.to_vec();
        zero[HEADER_LEN - 1] = 0;
        assert_eq!(decode(&forge(&f, zero)), Err(WireError::Field));
        let mut huge = body.to_vec();
        huge[HEADER_LEN - 4..HEADER_LEN].copy_from_slice(&5000u32.to_be_bytes());
        assert_eq!(decode(&forge(&f, huge)), Err(WireError::Field));

        // Magic, version, flags, kind.
        for (index, value, error) in [
            (0usize, b'X', WireError::Magic),
            (4, 2, WireError::Version),
            (5, 2, WireError::Field),
            (5, 0x81, WireError::Field),
            (HEADER_LEN, 9, WireError::Field),
        ] {
            let mut bad = body.to_vec();
            bad[index] = value;
            assert_eq!(decode(&forge(&f, bad)), Err(error), "byte {index}");
        }
        // Nil IDs and a sequence number of 0.
        for offset in [6usize, 22, 38] {
            let mut bad = body.to_vec();
            bad[offset..offset + 16].fill(0);
            assert_eq!(
                decode(&forge(&f, bad)),
                Err(WireError::Id),
                "id at {offset}"
            );
        }
        let mut zero_seq = body.to_vec();
        zero_seq[54..62].fill(0);
        assert_eq!(decode(&forge(&f, zero_seq)), Err(WireError::Field));
    }

    #[test]
    fn path_mismatch_is_refused() {
        let f = fixture();
        let bytes = segment(&f).encode(&f.crypto).unwrap();
        let mut other = f.path;
        other.seq += 1;
        assert_eq!(
            Segment::decode(&bytes, &other, &f.crypto),
            Err(WireError::Path)
        );
        other = f.path;
        other.device_id = InstallId::new_v7().unwrap();
        assert_eq!(
            Segment::decode(&bytes, &other, &f.crypto),
            Err(WireError::Path)
        );
        other = f.path;
        other.vault_id = VaultId::new_v7().unwrap();
        assert_eq!(
            Segment::decode(&bytes, &other, &f.crypto),
            Err(WireError::Path)
        );
        other = f.path;
        other.account_id = AccountId::new_v7().unwrap();
        assert_eq!(
            Segment::decode(&bytes, &other, &f.crypto),
            Err(WireError::Path)
        );
    }

    #[test]
    fn another_account_cannot_forge_a_segment() {
        let f = fixture();
        let bytes = segment(&f).encode(&f.crypto).unwrap();
        let ak = AccountKey::generate().unwrap();
        let foreign = KeyedCrypto::new(&ak, f.path.account_id, f.path.device_id).unwrap();
        assert_eq!(
            Segment::decode(&bytes, &f.path, &foreign),
            Err(WireError::Mac)
        );
    }

    #[test]
    fn object_bounds() {
        let f = fixture();
        let bytes = segment(&f).encode(&f.crypto).unwrap();
        let (body, _) = bytes.split_at(bytes.len() - 32);
        // A vault-key object of the wrong length; an oversized object; a
        // ciphertext that is not an envelope; a nil record ID.
        let first = HEADER_LEN;
        let mut short_key = body.to_vec();
        short_key[first + 1..first + 5].copy_from_slice(&77u32.to_be_bytes());
        assert!(Segment::decode(&forge(&f, short_key), &f.path, &f.crypto).is_err());
        let mut oversize = body.to_vec();
        oversize[first + 1..first + 5].copy_from_slice(&u32::MAX.to_be_bytes());
        // kind 3 first: change it to a meta object with a huge length.
        oversize[first] = 4;
        assert_eq!(
            Segment::decode(&forge(&f, oversize), &f.path, &f.crypto),
            Err(WireError::Length)
        );
        let mut s = segment(&f);
        s.objects = vec![SegmentObject::Meta(
            VaultMetaCiphertext::seal(&f.vault_key, &f.path.vault_id, b"x").unwrap(),
        )];
        let one = s.encode(&f.crypto).unwrap();
        let (one_body, _) = one.split_at(one.len() - 32);
        let mut garbage = one_body.to_vec();
        let at = HEADER_LEN + 5;
        garbage[at] ^= 0xFF;
        assert_eq!(
            Segment::decode(&forge(&f, garbage), &f.path, &f.crypto),
            Err(WireError::Ciphertext)
        );
        let mut nil = segment(&f);
        nil.objects = vec![SegmentObject::Item {
            record_id: f.item_id,
            ciphertext: ItemCiphertext::seal(&f.vault_key, &f.path.vault_id, &f.item_id, 1, b"i")
                .unwrap(),
        }];
        let item = nil.encode(&f.crypto).unwrap();
        let (item_body, _) = item.split_at(item.len() - 32);
        let mut nil_id = item_body.to_vec();
        nil_id[HEADER_LEN + 5..HEADER_LEN + 21].fill(0);
        assert_eq!(
            Segment::decode(&forge(&f, nil_id), &f.path, &f.crypto),
            Err(WireError::Id)
        );
    }

    #[test]
    fn encode_checks_its_input() {
        let f = fixture();
        let mut s = segment(&f);
        s.seq = 0;
        assert_eq!(s.encode(&f.crypto), Err(WireError::Field));
        let mut s = segment(&f);
        s.objects.clear();
        assert_eq!(s.encode(&f.crypto), Err(WireError::Field));
        let mut s = segment(&f);
        let one = s.objects[1].clone();
        s.objects = vec![one; MAX_OBJECTS + 1];
        assert_eq!(s.encode(&f.crypto), Err(WireError::Field));
        // The limit of the count itself is accepted and decodes.
        s.objects.truncate(MAX_OBJECTS);
        let bytes = s.encode(&f.crypto).unwrap();
        assert_eq!(
            Segment::decode(&bytes, &f.path, &f.crypto)
                .unwrap()
                .objects
                .len(),
            MAX_OBJECTS
        );
    }
}
