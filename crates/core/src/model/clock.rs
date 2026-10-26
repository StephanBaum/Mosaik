//! Logical clocks for sync: Lamport stamps and version vectors.
//!
//! Ordering decisions never use wall-clock time. A [`Stamp`] is a Lamport
//! timestamp `(counter, device)`: a device that edits an item picks a
//! counter one larger than every counter it has seen for that item, so an
//! edit made after seeing another always carries the larger stamp. Ties
//! between concurrent edits are broken by the device ID. The same stamp
//! identifies the edit ("dot") in the item's [`VersionVector`], whose entry
//! for a device is the counter of that device's latest edit.
//!
//! [`Timestamp`] is wall-clock time. It is stored for display (modified,
//! deleted, password changed) and never decides a merge.

use core::cmp::Ordering;
use core::fmt;
use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::ids::DeviceId;

/// Wall-clock time in milliseconds since the Unix epoch. Informational
/// only: merges never look at it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Wraps milliseconds since the Unix epoch.
    #[must_use]
    pub const fn from_unix_millis(ms: i64) -> Self {
        Self(ms)
    }

    /// Milliseconds since the Unix epoch.
    #[must_use]
    pub const fn as_unix_millis(self) -> i64 {
        self.0
    }
}

/// A Lamport timestamp `(counter, device)`, ordered by counter and then by
/// device ID. It also names one edit of an item.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Stamp {
    counter: u64,
    device: DeviceId,
}

impl Stamp {
    /// A stamp. Counters start at 1.
    ///
    /// # Errors
    /// [`Error::Malformed`] for counter 0.
    pub const fn new(counter: u64, device: DeviceId) -> Result<Self> {
        if counter == 0 {
            return Err(Error::Malformed);
        }
        Ok(Self { counter, device })
    }

    /// The Lamport counter.
    #[must_use]
    pub const fn counter(&self) -> u64 {
        self.counter
    }

    /// The device that made the edit.
    #[must_use]
    pub const fn device(&self) -> DeviceId {
        self.device
    }
}

/// How two version vectors relate (CRYPTO_SPEC §10).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Causality {
    /// Both describe the same set of edits.
    Equal,
    /// `self` has seen every edit `other` has, and more.
    Dominates,
    /// `other` has seen every edit `self` has, and more. A record whose
    /// authenticated vector is dominated by the stored one is a rollback or
    /// a replay and is ignored.
    DominatedBy,
    /// Each has edits the other has not seen: merge.
    Concurrent,
}

/// A per-item version vector `{DeviceId: u64}`. Entries are never 0; a
/// missing device means 0.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct VersionVector(BTreeMap<DeviceId, u64>);

impl VersionVector {
    /// The empty vector.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// The counter of `device`'s latest edit, or 0.
    #[must_use]
    pub fn get(&self, device: &DeviceId) -> u64 {
        self.0.get(device).copied().unwrap_or(0)
    }

    /// Whether the edit `stamp` is included.
    #[must_use]
    pub fn covers(&self, stamp: &Stamp) -> bool {
        stamp.counter <= self.get(&stamp.device)
    }

    /// The largest counter of any device, or 0.
    #[must_use]
    pub fn max_counter(&self) -> u64 {
        self.0.values().copied().max().unwrap_or(0)
    }

    /// Number of devices with an entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no device has an entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The entries in device order.
    pub fn iter(&self) -> impl Iterator<Item = (&DeviceId, u64)> {
        self.0.iter().map(|(d, c)| (d, *c))
    }

    /// Raises the entry of the stamp's device to include it (never lowers
    /// an entry).
    pub fn observe(&mut self, stamp: &Stamp) {
        let e = self.0.entry(stamp.device).or_insert(0);
        *e = (*e).max(stamp.counter);
    }

    /// The pointwise maximum.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for (d, c) in &other.0 {
            let e = out.0.entry(*d).or_insert(0);
            *e = (*e).max(*c);
        }
        out
    }

    /// Compares two vectors pointwise.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Causality {
        let mut greater = false;
        let mut less = false;
        for d in self.0.keys().chain(other.0.keys()) {
            match self.get(d).cmp(&other.get(d)) {
                Ordering::Greater => greater = true,
                Ordering::Less => less = true,
                Ordering::Equal => {}
            }
        }
        match (greater, less) {
            (false, false) => Causality::Equal,
            (true, false) => Causality::Dominates,
            (false, true) => Causality::DominatedBy,
            (true, true) => Causality::Concurrent,
        }
    }

    /// The next Lamport stamp for `device`: one more than every counter in
    /// the vector. The vector is updated to include it.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`] if the counter would exceed `u64::MAX`.
    pub(crate) fn tick(&mut self, device: DeviceId) -> Result<Stamp> {
        let counter = self
            .max_counter()
            .checked_add(1)
            .ok_or(Error::ClockOverflow)?;
        self.0.insert(device, counter);
        Ok(Stamp { counter, device })
    }

    /// Inserts a decoded entry.
    ///
    /// # Errors
    /// [`Error::Malformed`] for counter 0 or a duplicate device.
    pub(crate) fn insert_decoded(&mut self, device: DeviceId, counter: u64) -> Result<()> {
        if counter == 0 || self.0.insert(device, counter).is_some() {
            return Err(Error::Malformed);
        }
        Ok(())
    }
}

impl fmt::Debug for VersionVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}

#[cfg(test)]
#[allow(clippy::many_single_char_names, reason = "short test names")]
mod tests {
    use super::*;

    fn dev(b: u8) -> DeviceId {
        DeviceId::from_bytes([b; 16]).unwrap()
    }

    fn vv(entries: &[(u8, u64)]) -> VersionVector {
        let mut v = VersionVector::new();
        for &(d, c) in entries {
            v.observe(&Stamp::new(c, dev(d)).unwrap());
        }
        v
    }

    #[test]
    fn compare_cases() {
        let a = vv(&[(1, 2), (2, 1)]);
        assert_eq!(a.compare(&a), Causality::Equal);
        assert_eq!(a.compare(&vv(&[(1, 2)])), Causality::Dominates);
        assert_eq!(vv(&[(1, 2)]).compare(&a), Causality::DominatedBy);
        assert_eq!(a.compare(&vv(&[(1, 3)])), Causality::Concurrent);
        assert_eq!(
            VersionVector::new().compare(&VersionVector::new()),
            Causality::Equal
        );
        assert_eq!(VersionVector::new().compare(&a), Causality::DominatedBy);
    }

    #[test]
    fn join_covers_tick() {
        let a = vv(&[(1, 2), (2, 5)]);
        let b = vv(&[(1, 4), (3, 1)]);
        let j = a.join(&b);
        assert_eq!(j, b.join(&a));
        assert_eq!(j.get(&dev(1)), 4);
        assert_eq!(j.get(&dev(2)), 5);
        assert_eq!(j.get(&dev(4)), 0);
        assert_eq!(j.len(), 3);
        assert!(!j.is_empty());
        assert!(j.covers(&Stamp::new(5, dev(2)).unwrap()));
        assert!(!j.covers(&Stamp::new(6, dev(2)).unwrap()));
        let mut t = j.clone();
        let s = t.tick(dev(9)).unwrap();
        assert_eq!(s.counter(), 6);
        assert_eq!(s.device(), dev(9));
        assert!(t.covers(&s) && !j.covers(&s));
        assert_eq!(t.iter().count(), 4);
        let mut obs = vv(&[(1, 9)]);
        obs.observe(&Stamp::new(3, dev(1)).unwrap());
        assert_eq!(obs.get(&dev(1)), 9, "observe never lowers");
        assert!(format!("{obs:?}").contains('9'));
    }

    #[test]
    fn overflow_and_zero() {
        let mut v = vv(&[(1, u64::MAX)]);
        assert_eq!(v.tick(dev(1)).unwrap_err(), Error::ClockOverflow);
        assert_eq!(Stamp::new(0, dev(1)).unwrap_err(), Error::Malformed);
        let mut d = VersionVector::new();
        assert_eq!(d.insert_decoded(dev(1), 0).unwrap_err(), Error::Malformed);
        d.insert_decoded(dev(1), 1).unwrap();
        assert_eq!(d.insert_decoded(dev(1), 2).unwrap_err(), Error::Malformed);
    }

    #[test]
    fn stamp_order_is_counter_then_device() {
        assert!(Stamp::new(1, dev(9)).unwrap() < Stamp::new(2, dev(1)).unwrap());
        assert!(Stamp::new(2, dev(1)).unwrap() < Stamp::new(2, dev(2)).unwrap());
        assert_eq!(Timestamp::from_unix_millis(-5).as_unix_millis(), -5);
    }
}
