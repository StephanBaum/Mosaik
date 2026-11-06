//! The folder backend's pull cursor, per vault (CRYPTO_SPEC §10.1):
//! which of every other device's segments have been read.
//!
//! ```text
//! version:u8 (1) || n:u16 (<= 256)
//! || n x ( device_id[16] || contiguous:u64 || m:u16 (<= 1024) || m x above_seq:u64 )
//! ```
//!
//! `contiguous` is the highest sequence number such that every number up
//! to it was read; `above` are the numbers read out of order (sorted, all
//! greater than `contiguous`). The devices are strictly ascending. A cursor
//! is at most 64 KiB: the encoder drops `above` sets, largest first, until
//! it fits, which only makes the reader read some segments again (reading a
//! segment twice changes nothing).

use mosaic_core::InstallId;

use super::{Reader, WireError};

const VERSION: u8 = 1;
/// The most devices a cursor lists.
pub const MAX_DEVICES: usize = 256;
/// The most out-of-order sequence numbers per device.
pub const MAX_ABOVE: usize = 1024;
/// The longest cursor in bytes.
pub const MAX_LEN: usize = 65_536;

/// What was read of one device's log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCursor {
    /// The device.
    pub device_id: InstallId,
    /// Every sequence number up to this one was read.
    pub contiguous: u64,
    /// Sequence numbers read beyond `contiguous`, strictly ascending.
    pub above: Vec<u64>,
}

impl DeviceCursor {
    fn encoded_len(&self) -> usize {
        16 + 8 + 2 + self.above.len() * 8
    }

    /// Whether `seq` was read.
    #[must_use]
    pub fn has_read(&self, seq: u64) -> bool {
        seq <= self.contiguous || self.above.binary_search(&seq).is_ok()
    }

    /// Records that `seq` was read and folds `above` into `contiguous`
    /// where they meet.
    pub fn mark_read(&mut self, seq: u64) {
        if self.has_read(seq) {
            return;
        }
        if let Some(next) = self.contiguous.checked_add(1)
            && seq == next
        {
            self.contiguous = seq;
        } else if let Err(at) = self.above.binary_search(&seq) {
            self.above.insert(at, seq);
        }
        while let Some(&first) = self.above.first() {
            if self.contiguous.checked_add(1) == Some(first) {
                self.contiguous = first;
                self.above.remove(0);
            } else {
                break;
            }
        }
    }
}

/// The cursor of one vault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FolderCursor {
    /// One entry per device, strictly ascending by device ID.
    pub devices: Vec<DeviceCursor>,
}

impl FolderCursor {
    /// The entry of `device`, if the cursor has one.
    #[must_use]
    pub fn device(&self, device: &InstallId) -> Option<&DeviceCursor> {
        self.devices
            .binary_search_by(|d| d.device_id.cmp(device))
            .ok()
            .and_then(|i| self.devices.get(i))
    }

    /// The entry of `device`, added (empty) if missing.
    ///
    /// # Errors
    /// [`WireError::Field`] if the cursor already lists [`MAX_DEVICES`]
    /// devices.
    pub fn device_mut(&mut self, device: InstallId) -> Result<&mut DeviceCursor, WireError> {
        let at = match self.devices.binary_search_by(|d| d.device_id.cmp(&device)) {
            Ok(at) => at,
            Err(at) => {
                if self.devices.len() >= MAX_DEVICES {
                    return Err(WireError::Field);
                }
                self.devices.insert(
                    at,
                    DeviceCursor {
                        device_id: device,
                        contiguous: 0,
                        above: Vec::new(),
                    },
                );
                at
            }
        };
        self.devices.get_mut(at).ok_or(WireError::Field)
    }

    fn check(&self) -> Result<(), WireError> {
        if self.devices.len() > MAX_DEVICES {
            return Err(WireError::Field);
        }
        if self
            .devices
            .windows(2)
            .any(|w| w.first().map(|d| d.device_id) >= w.get(1).map(|d| d.device_id))
        {
            return Err(WireError::Order);
        }
        for d in &self.devices {
            if d.above.len() > MAX_ABOVE {
                return Err(WireError::Field);
            }
            if d.above.first().is_some_and(|&a| a <= d.contiguous)
                || d.above.windows(2).any(|w| w.first() >= w.get(1))
            {
                return Err(WireError::Order);
            }
        }
        Ok(())
    }

    /// Encodes the cursor. If it would exceed [`MAX_LEN`], the `above` sets
    /// are dropped, largest first (the smallest device ID first among equal
    /// sets), until it fits.
    ///
    /// # Errors
    /// [`WireError::Field`] or [`WireError::Order`] for a cursor that
    /// breaks the format's rules.
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        self.check()?;
        let devices: Vec<&DeviceCursor> = self.devices.iter().collect();
        let mut dropped: Vec<usize> = Vec::new();
        let mut len = 3 + devices.iter().map(|d| d.encoded_len()).sum::<usize>();
        while len > MAX_LEN {
            let Some((i, d)) = devices
                .iter()
                .enumerate()
                .filter(|(i, d)| !d.above.is_empty() && !dropped.contains(i))
                .max_by_key(|(i, d)| (d.above.len(), core::cmp::Reverse(*i)))
            else {
                return Err(WireError::Length);
            };
            len -= d.above.len() * 8;
            dropped.push(i);
        }
        let n = u16::try_from(devices.len()).map_err(|_| WireError::Field)?;
        let mut out = Vec::with_capacity(len);
        out.push(VERSION);
        out.extend_from_slice(&n.to_be_bytes());
        for (i, d) in devices.iter().enumerate() {
            out.extend_from_slice(d.device_id.as_bytes());
            out.extend_from_slice(&d.contiguous.to_be_bytes());
            if dropped.contains(&i) {
                out.extend_from_slice(&0u16.to_be_bytes());
            } else {
                let m = u16::try_from(d.above.len()).map_err(|_| WireError::Field)?;
                out.extend_from_slice(&m.to_be_bytes());
                for a in &d.above {
                    out.extend_from_slice(&a.to_be_bytes());
                }
            }
        }
        Ok(out)
    }

    /// Decodes a cursor strictly.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Version`], [`WireError::Field`],
    /// [`WireError::Id`], [`WireError::Order`].
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() > MAX_LEN {
            return Err(WireError::Length);
        }
        let mut r = Reader::new(bytes);
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let n = usize::from(r.u16()?);
        if n > MAX_DEVICES {
            return Err(WireError::Field);
        }
        // Every device takes at least 26 bytes.
        if n * 26 > r.remaining() {
            return Err(WireError::Length);
        }
        let mut devices = Vec::with_capacity(n);
        for _ in 0..n {
            let device_id = InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
            let contiguous = r.u64()?;
            let m = usize::from(r.u16()?);
            if m > MAX_ABOVE {
                return Err(WireError::Field);
            }
            if m * 8 > r.remaining() {
                return Err(WireError::Length);
            }
            let mut above = Vec::with_capacity(m);
            for _ in 0..m {
                above.push(r.u64()?);
            }
            devices.push(DeviceCursor {
                device_id,
                contiguous,
                above,
            });
        }
        r.finish()?;
        let cursor = Self { devices };
        cursor.check()?;
        Ok(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(b: u8) -> InstallId {
        InstallId::from_bytes([b; 16]).unwrap()
    }

    fn cursor() -> FolderCursor {
        FolderCursor {
            devices: vec![
                DeviceCursor {
                    device_id: id(1),
                    contiguous: 5,
                    above: vec![7, 9, 10],
                },
                DeviceCursor {
                    device_id: id(2),
                    contiguous: 0,
                    above: vec![],
                },
            ],
        }
    }

    #[test]
    fn round_trip_and_layout() {
        let c = cursor();
        let bytes = c.encode().unwrap();
        assert_eq!(bytes.len(), 3 + (16 + 8 + 2 + 24) + (16 + 8 + 2));
        assert_eq!(&bytes[..3], &[1, 0, 2]);
        assert_eq!(FolderCursor::decode(&bytes).unwrap(), c);
        let empty = FolderCursor::default();
        assert_eq!(empty.encode().unwrap(), vec![1, 0, 0]);
        assert_eq!(FolderCursor::decode(&[1, 0, 0]).unwrap(), empty);
    }

    #[test]
    fn rejections() {
        let bytes = cursor().encode().unwrap();
        assert_eq!(FolderCursor::decode(&[]), Err(WireError::Length));
        assert_eq!(FolderCursor::decode(&[2, 0, 0]), Err(WireError::Version));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(FolderCursor::decode(&trailing), Err(WireError::Length));
        assert_eq!(
            FolderCursor::decode(&bytes[..bytes.len() - 1]),
            Err(WireError::Length)
        );
        assert_eq!(FolderCursor::decode(&[1, 1, 0]), Err(WireError::Length));
        assert_eq!(
            FolderCursor::decode(&[1, 0xFF, 0xFF]),
            Err(WireError::Field)
        );
        assert_eq!(
            FolderCursor::decode(&vec![0; MAX_LEN + 1]),
            Err(WireError::Length)
        );
        let mut nil = bytes.clone();
        nil[3..19].fill(0);
        assert_eq!(FolderCursor::decode(&nil), Err(WireError::Id));
        // Above values: not above contiguous, unsorted, duplicate.
        for above in [vec![5u64], vec![4], vec![9, 7], vec![7, 7]] {
            let c = FolderCursor {
                devices: vec![DeviceCursor {
                    device_id: id(1),
                    contiguous: 5,
                    above,
                }],
            };
            assert_eq!(c.encode(), Err(WireError::Order));
        }
        // Devices: duplicate and unsorted.
        let mut dup = cursor();
        dup.devices[1].device_id = id(1);
        assert_eq!(dup.encode(), Err(WireError::Order));
        let mut unsorted = cursor();
        unsorted.devices.reverse();
        assert_eq!(unsorted.encode(), Err(WireError::Order));
        // The same on the decoding side.
        let mut raw = bytes.clone();
        let second = 3 + 16 + 8 + 2 + 24;
        raw.copy_within(3..19, second);
        assert_eq!(FolderCursor::decode(&raw), Err(WireError::Order));
        let mut low = bytes.clone();
        low[3 + 16 + 8 + 2..3 + 16 + 8 + 2 + 8].copy_from_slice(&5u64.to_be_bytes());
        assert_eq!(FolderCursor::decode(&low), Err(WireError::Order));
        // m above the limit.
        let mut many = bytes;
        many[3 + 16 + 8..3 + 16 + 8 + 2].copy_from_slice(&1025u16.to_be_bytes());
        assert_eq!(FolderCursor::decode(&many), Err(WireError::Field));
    }

    #[test]
    fn oversized_cursors_drop_the_largest_above_sets_first() {
        // 256 devices with 1024 numbers each would be 2 MiB.
        let devices: Vec<DeviceCursor> = (1..=200u8)
            .map(|b| DeviceCursor {
                device_id: id(b),
                contiguous: 1,
                above: (0..u64::from(b) * 5)
                    .map(|i| 10 + i)
                    .take(MAX_ABOVE)
                    .collect(),
            })
            .collect();
        let c = FolderCursor { devices };
        let bytes = c.encode().unwrap();
        assert!(bytes.len() <= MAX_LEN);
        let back = FolderCursor::decode(&bytes).unwrap();
        assert_eq!(back.devices.len(), 200);
        for (kept, full) in back.devices.iter().zip(&c.devices) {
            assert_eq!(kept.contiguous, full.contiguous);
            assert!(kept.above.is_empty() || kept.above == full.above);
        }
        let dropped: Vec<usize> = back
            .devices
            .iter()
            .zip(&c.devices)
            .filter(|(k, f)| k.above.is_empty() && !f.above.is_empty())
            .map(|(_, f)| f.above.len())
            .collect();
        let kept_max = back.devices.iter().map(|d| d.above.len()).max().unwrap();
        assert_ne!(dropped.len(), 0);
        assert!(
            dropped.iter().all(|&len| len >= kept_max),
            "the largest sets went first"
        );
    }

    #[test]
    fn mark_read_folds_above_into_contiguous() {
        let mut d = DeviceCursor {
            device_id: id(1),
            contiguous: 0,
            above: vec![],
        };
        for seq in [3, 1, 5, 2, 2, 1] {
            d.mark_read(seq);
        }
        assert_eq!((d.contiguous, d.above.clone()), (3, vec![5]));
        d.mark_read(4);
        assert_eq!((d.contiguous, d.above.clone()), (5, vec![]));
        assert!(d.has_read(5) && !d.has_read(6));
        let mut c = FolderCursor::default();
        c.device_mut(id(2)).unwrap().mark_read(1);
        c.device_mut(id(1)).unwrap().mark_read(1);
        assert_eq!(c.devices[0].device_id, id(1));
        assert_eq!(c.device(&id(2)).unwrap().contiguous, 1);
        assert!(c.device(&id(3)).is_none());
        let mut full = FolderCursor::default();
        for i in 1..=u16::try_from(MAX_DEVICES).unwrap() {
            let mut raw = [0u8; 16];
            raw[14..].copy_from_slice(&i.to_be_bytes());
            full.device_mut(InstallId::from_bytes(raw).unwrap())
                .unwrap();
        }
        assert_eq!(
            full.device_mut(InstallId::new_v7().unwrap()).err(),
            Some(WireError::Field)
        );
    }
}
