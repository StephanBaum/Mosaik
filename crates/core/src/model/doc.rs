//! The replicated document behind items, vault metadata and tombstones.
//!
//! A document is a version vector plus a map from field keys to
//! *registers*. A register holds the values of one field that no replica
//! has superseded yet, each tagged with the [`Stamp`] of the edit that
//! wrote it. Normally that is one value; after concurrent edits it is
//! several, the largest stamp being the winner and the others conflict
//! copies.
//!
//! # Merge
//!
//! `merge(a, b)` keeps an entry with stamp `s` of field `k` if
//! - both sides have it, or
//! - one side has it and the other side's version vector does not cover
//!   `s` (the other side has not seen that edit yet).
//!
//! An entry that one side has seen (covered by its vector) but no longer
//! holds was superseded or removed there, and stays removed. Version
//! vectors are joined pointwise, and members without a clock (unknown
//! top-level members) are joined by taking the larger value. For every
//! single `(k, s)` the four states "absent", "present but not seen by the
//! other", "present and seen", "removed" form a chain, and the merge takes
//! their maximum, so the merge is commutative, associative and idempotent
//! for any inputs, including ill-formed ones. When both sides hold the same
//! stamp with different contents (two writers sharing a device ID, or
//! forged or corrupted data) the larger content wins, which keeps the laws;
//! [`Doc::repair_after_merge`] detects it and adds the other value back.
//!
//! A deletion is an entry under the reserved key `deleted` in a document
//! whose other fields were all removed by the same edit. Its vector covers
//! every edit the deleting device had seen, so merging it with a replica
//! that has only seen those edits removes them all again, while edits made
//! concurrently survive together with the `deleted` entry, which flags the
//! conflict. The pure merge cannot tell which removed fields the editing
//! replica still wants; [`Doc::repair_after_merge`], run by every replica
//! that had not seen the deletion, writes them back as a new edit.

use core::cmp::Ordering;
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::ser::{SerializeMap as _, SerializeSeq as _};
use serde::{Serialize, Serializer};

use super::clock::{Stamp, Timestamp, VersionVector};
use super::value::{Object, Text, Value, parse_json};
use crate::error::{Error, Result};
use crate::ids::DeviceId;

/// The reserved field key of deletion entries.
pub(crate) const DELETED_KEY: &str = "deleted";

/// Top-level member names that both plaintext layouts reserve. A decoder
/// rejects any of them as an unknown member, so neither layout can carry
/// the other's member and re-encode it as a duplicate key.
const RESERVED_MEMBERS: [&str; 4] = ["schema_version", "vv", "fields", DELETED_KEY];

/// Longest field key in bytes.
pub const MAX_FIELD_KEY_LEN: usize = 256;

/// One value of a field, written by one edit.
#[derive(PartialEq, Eq)]
pub struct Entry {
    stamp: Stamp,
    at: Timestamp,
    value: Value,
    extra: Object,
}

impl Entry {
    pub(crate) const fn new(stamp: Stamp, at: Timestamp, value: Value) -> Self {
        Self {
            stamp,
            at,
            value,
            extra: Object::new(),
        }
    }

    /// The edit that wrote this value. The winner of a field is the entry
    /// with the largest stamp.
    #[must_use]
    pub const fn stamp(&self) -> Stamp {
        self.stamp
    }

    /// When the value was written, by the writer's clock (display only).
    #[must_use]
    pub const fn at(&self) -> Timestamp {
        self.at
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Total order on everything but the stamp, for the merge tie-break.
    fn payload_cmp(&self, other: &Self) -> Ordering {
        self.at
            .cmp(&other.at)
            .then_with(|| self.value.cmp(&other.value))
            .then_with(|| self.extra.cmp(&other.extra))
    }

    fn dup(&self) -> Self {
        Self {
            stamp: self.stamp,
            at: self.at,
            value: self.value.dup(),
            extra: self.extra.dup(),
        }
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("stamp", &self.stamp)
            .field("at", &self.at)
            .finish_non_exhaustive()
    }
}

/// The live values of one field, by stamp. Never empty inside a [`Doc`].
#[derive(Default, PartialEq, Eq)]
pub(crate) struct Register(BTreeMap<Stamp, Entry>);

impl Register {
    /// The entry with the largest stamp.
    pub(crate) fn winner(&self) -> Option<&Entry> {
        self.0.last_key_value().map(|(_, e)| e)
    }

    /// The entries in ascending stamp order.
    pub(crate) fn entries(&self) -> impl DoubleEndedIterator<Item = &Entry> {
        self.0.values()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn get(&self, stamp: &Stamp) -> Option<&Entry> {
        self.0.get(stamp)
    }

    fn join(a: Option<&Self>, avv: &VersionVector, b: Option<&Self>, bvv: &VersionVector) -> Self {
        let empty = Self::default();
        let a = a.unwrap_or(&empty);
        let b = b.unwrap_or(&empty);
        let stamps: BTreeSet<&Stamp> = a.0.keys().chain(b.0.keys()).collect();
        let mut out = Self::default();
        for s in stamps {
            let keep = match (a.0.get(s), b.0.get(s)) {
                (Some(x), Some(y)) => Some(if x.payload_cmp(y) == Ordering::Less {
                    y
                } else {
                    x
                }),
                (Some(x), None) => (!bvv.covers(s)).then_some(x),
                (None, Some(y)) => (!avv.covers(s)).then_some(y),
                (None, None) => None,
            };
            if let Some(e) = keep {
                out.0.insert(*s, e.dup());
            }
        }
        out
    }
}

/// A field whose register holds more than one distinct value.
#[derive(Debug)]
pub struct FieldConflict<'a> {
    /// The field key.
    pub key: &'a str,
    /// The value shown as the field's value (largest stamp).
    pub winner: &'a Entry,
    /// The other distinct values, newest stamp first. They stay on the
    /// item until a device resolves or dismisses the conflict.
    pub losers: Vec<&'a Entry>,
}

/// What [`Doc::repair_after_merge`] found and did.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Repair {
    /// Fields brought back after a concurrent delete.
    pub(crate) restored_fields: usize,
    /// Both replicas held the same stamp with different contents.
    pub(crate) stamp_collision: bool,
}

/// The replicated state. See the module documentation.
#[derive(Default, PartialEq, Eq)]
pub(crate) struct Doc {
    vv: VersionVector,
    fields: BTreeMap<Text, Register>,
    extra: Object,
}

impl Doc {
    pub(crate) const fn vv(&self) -> &VersionVector {
        &self.vv
    }

    pub(crate) fn register(&self, key: &str) -> Option<&Register> {
        self.fields.get(key)
    }

    pub(crate) fn winner(&self, key: &str) -> Option<&Entry> {
        self.register(key).and_then(Register::winner)
    }

    /// The winning value of `key`.
    pub(crate) fn value(&self, key: &str) -> Option<&Value> {
        self.winner(key).map(Entry::value)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(Text::as_str)
    }

    pub(crate) fn registers(&self) -> impl Iterator<Item = (&str, &Register)> {
        self.fields.iter().map(|(k, r)| (k.as_str(), r))
    }

    /// The merge described in the module documentation.
    pub(crate) fn merge(&self, other: &Self) -> Self {
        let keys: BTreeSet<&Text> = self.fields.keys().chain(other.fields.keys()).collect();
        let mut fields = BTreeMap::new();
        for k in keys {
            let r = Register::join(
                self.fields.get(k.as_str()),
                &self.vv,
                other.fields.get(k.as_str()),
                &other.vv,
            );
            if r.len() > 0 {
                fields.insert(k.dup(), r);
            }
        }
        Self {
            vv: self.vv.join(&other.vv),
            fields,
            // No clock: top-level members are grow-only by contract
            // (ADR-0009 item 4), so the larger value is the newer one.
            extra: self.extra.join_max(&other.extra),
        }
    }

    /// A new Lamport stamp for a local edit by `device`.
    pub(crate) fn tick(&mut self, device: DeviceId) -> Result<Stamp> {
        self.vv.tick(device)
    }

    /// Writes `value` under `key` with an already ticked `stamp`. With
    /// `keep_losers`, only the current winner is superseded and conflict
    /// copies stay; otherwise every value of the field is superseded.
    ///
    /// # Errors
    /// [`Error::Malformed`] for a key the decoder would reject.
    pub(crate) fn write(
        &mut self,
        key: &str,
        stamp: Stamp,
        at: Timestamp,
        value: Value,
        keep_losers: bool,
    ) -> Result<()> {
        check_key(key)?;
        let reg = self.fields.entry(Text::new(key)).or_default();
        if keep_losers {
            reg.0.pop_last();
        } else {
            reg.0.clear();
        }
        reg.0.insert(stamp, Entry::new(stamp, at, value));
        Ok(())
    }

    /// Adds a copy of `entry` (value, time and unknown members) to `key`
    /// under a fresh stamp of `device`, keeping every value already there.
    /// The repair after a merge uses it to bring back values a merge
    /// dropped (see [`Doc::repair_after_merge`]).
    fn readd(&mut self, key: &Text, entry: &Entry, device: DeviceId) -> Result<()> {
        let stamp = self.tick(device)?;
        let reg = self.fields.entry(key.dup()).or_default();
        reg.0.insert(
            stamp,
            Entry {
                stamp,
                at: entry.at,
                value: entry.value.dup(),
                extra: entry.extra.dup(),
            },
        );
        Ok(())
    }

    /// Repairs `self`, the merge of `local` and `received`, as a local edit
    /// by `device`. Returns what it found. See
    /// [`crate::model::Item::merge_and_repair`] for the contract.
    ///
    /// 1. Same stamp, different contents (only possible when two writers
    ///    share a device ID): the merge kept one; the other value is added
    ///    back under a fresh stamp, so both are visible as a conflict.
    /// 2. A delete that lost against a concurrent edit, and `local` had not
    ///    seen that deletion: every field `local` held that the merge
    ///    dropped is added back (all its values, conflict copies included)
    ///    under fresh stamps. The deletion entry stays as the flag.
    pub(crate) fn repair_after_merge(
        &mut self,
        local: &Self,
        received: &Self,
        device: DeviceId,
    ) -> Result<Repair> {
        let mut repair = Repair::default();
        for (key, mine) in &local.fields {
            let Some(theirs) = received.fields.get(key.as_str()) else {
                continue;
            };
            for (s, a) in &mine.0 {
                let Some(b) = theirs.0.get(s) else {
                    continue;
                };
                if a.payload_cmp(b) == Ordering::Equal {
                    continue;
                }
                repair.stamp_collision = true;
                if a.value != b.value {
                    let loser = if a.payload_cmp(b) == Ordering::Less {
                        a
                    } else {
                        b
                    };
                    self.readd(key, loser, device)?;
                }
            }
        }
        let missed_a_deletion = !self.is_deleted()
            && self
                .register(DELETED_KEY)
                .into_iter()
                .flat_map(Register::entries)
                .any(|d| !local.vv.covers(&d.stamp));
        if missed_a_deletion {
            for (key, reg) in &local.fields {
                if key.as_str() == DELETED_KEY || self.fields.contains_key(key.as_str()) {
                    continue;
                }
                for e in reg.entries() {
                    self.readd(key, e, device)?;
                }
                repair.restored_fields = repair.restored_fields.saturating_add(1);
            }
        }
        Ok(repair)
    }

    /// Removes every value of `key`. The caller has ticked the clock.
    pub(crate) fn remove_key(&mut self, key: &str) -> Option<Register> {
        self.fields.remove(key)
    }

    /// Removes one value of `key`. The caller has ticked the clock.
    pub(crate) fn remove_entry(&mut self, key: &str, stamp: &Stamp) -> Option<Entry> {
        let reg = self.fields.get_mut(key)?;
        let e = reg.0.remove(stamp);
        if reg.0.is_empty() {
            self.fields.remove(key);
        }
        e
    }

    /// Whether this is a deletion: only `deleted` entries remain.
    pub(crate) fn is_deleted(&self) -> bool {
        self.fields.len() == 1 && self.fields.contains_key(DELETED_KEY)
    }

    /// Deletes: one edit that removes every field and records the deletion.
    /// A no-op on a document that is already deleted.
    pub(crate) fn delete(&mut self, device: DeviceId, at: Timestamp) -> Result<()> {
        if self.is_deleted() {
            return Ok(());
        }
        let stamp = self.tick(device)?;
        self.fields.clear();
        self.write(DELETED_KEY, stamp, at, Value::Null, false)
    }

    /// The latest wall-clock time of any value.
    pub(crate) fn modified_at(&self) -> Option<Timestamp> {
        self.fields
            .values()
            .flat_map(Register::entries)
            .map(Entry::at)
            .max()
    }

    /// Fields with more than one distinct value, for keys `include` accepts.
    pub(crate) fn conflicts(&self, include: impl Fn(&str) -> bool) -> Vec<FieldConflict<'_>> {
        let mut out = Vec::new();
        for (key, reg) in self.registers() {
            if reg.len() < 2 || !include(key) {
                continue;
            }
            let Some(winner) = reg.winner() else {
                continue;
            };
            let mut losers: Vec<&Entry> = Vec::new();
            for e in reg.entries().rev() {
                if e.value != winner.value && !losers.iter().any(|l| l.value == e.value) {
                    losers.push(e);
                }
            }
            if !losers.is_empty() {
                out.push(FieldConflict {
                    key,
                    winner,
                    losers,
                });
            }
        }
        out
    }

    /// A second zeroizing copy.
    pub(crate) fn dup(&self) -> Self {
        Self {
            vv: self.vv.clone(),
            fields: self
                .fields
                .iter()
                .map(|(k, r)| {
                    (
                        k.dup(),
                        Register(r.0.iter().map(|(s, e)| (*s, e.dup())).collect()),
                    )
                })
                .collect(),
            extra: self.extra.dup(),
        }
    }
}

impl fmt::Debug for Doc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Doc")
            .field("vv", &self.vv)
            .field("fields", &self.fields.len())
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// JSON encoding
// ---------------------------------------------------------------------------

/// The two plaintext layouts of a document.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Format {
    /// Items and vault metadata:
    /// `{"schema_version":1,"vv":{..},"fields":{key:[entry..]}, ..}`.
    Document,
    /// Tombstones: `{"schema_version":1,"vv":{..},"deleted":[entry..], ..}`.
    Tombstone,
}

/// The `schema_version` of every plaintext this build writes and reads.
pub const SCHEMA_VERSION: u16 = 1;

/// Parses a canonical lowercase hyphenated UUID into a device ID.
pub(crate) fn parse_device(s: &str) -> Result<DeviceId> {
    DeviceId::from_bytes(parse_uuid(s)?).map_err(|_| Error::Malformed)
}

/// Parses a canonical lowercase hyphenated UUID (the `Display` form of the
/// ID types) into 16 bytes. The nil UUID is left to the ID constructors.
pub(crate) fn parse_uuid(s: &str) -> Result<[u8; 16]> {
    let b = s.as_bytes();
    if b.len() != 36 {
        return Err(Error::Malformed);
    }
    let mut out = [0u8; 16];
    let mut nibble = 0usize;
    for (i, &c) in b.iter().enumerate() {
        if matches!(i, 8 | 13 | 18 | 23) {
            if c != b'-' {
                return Err(Error::Malformed);
            }
            continue;
        }
        let v = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => return Err(Error::Malformed),
        };
        let slot = out.get_mut(nibble / 2).ok_or(Error::Malformed)?;
        *slot = (*slot << 4) | v;
        nibble += 1;
    }
    Ok(out)
}

/// The field-key rule of the decoder: 1 to [`MAX_FIELD_KEY_LEN`] bytes, no
/// control characters. Every write checks it too, so the edit API never
/// produces a document that does not decode.
pub(crate) fn check_key(key: &str) -> Result<()> {
    if key.is_empty() || key.len() > MAX_FIELD_KEY_LEN || key.chars().any(char::is_control) {
        return Err(Error::Malformed);
    }
    Ok(())
}

fn decode_vv(v: Value) -> Result<VersionVector> {
    let Value::Object(o) = v else {
        return Err(Error::Malformed);
    };
    let mut vv = VersionVector::new();
    for (k, c) in o.into_iter_owned() {
        let counter = c.as_u64().ok_or(Error::Malformed)?;
        vv.insert_decoded(parse_device(k.as_str())?, counter)?;
    }
    Ok(vv)
}

fn decode_entry(v: Value) -> Result<Entry> {
    let Value::Object(o) = v else {
        return Err(Error::Malformed);
    };
    let (mut c, mut d, mut at, mut value) = (None, None, None, None);
    let mut extra = Object::new();
    for (k, v) in o.into_iter_owned() {
        match k.as_str() {
            "c" => c = Some(v.as_u64().ok_or(Error::Malformed)?),
            "d" => d = Some(parse_device(v.as_str().ok_or(Error::Malformed)?)?),
            "at" => at = Some(v.as_i64().ok_or(Error::Malformed)?),
            "v" => value = Some(v),
            _ => {
                extra.insert_text(k, v);
            }
        }
    }
    match (c, d, at, value) {
        (Some(c), Some(d), Some(at), Some(value)) => Ok(Entry {
            stamp: Stamp::new(c, d)?,
            at: Timestamp::from_unix_millis(at),
            value,
            extra,
        }),
        _ => Err(Error::Malformed),
    }
}

fn decode_register(v: Value, vv: &VersionVector) -> Result<Register> {
    let Value::Array(items) = v else {
        return Err(Error::Malformed);
    };
    if items.is_empty() {
        return Err(Error::Malformed);
    }
    let mut reg = Register::default();
    for item in items {
        let e = decode_entry(item)?;
        if !vv.covers(&e.stamp) || reg.0.insert(e.stamp, e).is_some() {
            return Err(Error::Malformed);
        }
    }
    Ok(reg)
}

/// Strictly decodes a document plaintext.
///
/// # Errors
/// [`Error::UnsupportedVersion`] for a `schema_version` other than 1;
/// [`Error::Malformed`] for invalid JSON, a missing or mistyped member, a
/// top-level member reserved by the other layout, an invalid field key, an
/// empty register, a duplicate stamp or an entry the version vector does
/// not cover.
pub(crate) fn decode(bytes: &[u8], format: Format) -> Result<Doc> {
    let Value::Object(top) = parse_json(bytes)? else {
        return Err(Error::Malformed);
    };
    let (mut schema, mut vv, mut fields) = (None, None, None);
    let mut extra = Object::new();
    for (k, v) in top.into_iter_owned() {
        match (k.as_str(), format) {
            ("schema_version", _) => schema = Some(v),
            ("vv", _) => vv = Some(v),
            ("fields", Format::Document) | ("deleted", Format::Tombstone) => fields = Some(v),
            // The other layout's member: never an unknown extra.
            (name, _) if RESERVED_MEMBERS.contains(&name) => return Err(Error::Malformed),
            _ => {
                extra.insert_text(k, v);
            }
        }
    }
    match schema.as_ref().map(Value::as_u64) {
        Some(Some(1)) => {}
        Some(Some(_)) => return Err(Error::UnsupportedVersion),
        _ => return Err(Error::Malformed),
    }
    let vv = decode_vv(vv.ok_or(Error::Malformed)?)?;
    let fields_value = fields.ok_or(Error::Malformed)?;
    let mut map = BTreeMap::new();
    match format {
        Format::Document => {
            let Value::Object(o) = fields_value else {
                return Err(Error::Malformed);
            };
            for (k, v) in o.into_iter_owned() {
                check_key(k.as_str())?;
                let reg = decode_register(v, &vv)?;
                map.insert(k, reg);
            }
        }
        Format::Tombstone => {
            map.insert(Text::new(DELETED_KEY), decode_register(fields_value, &vv)?);
        }
    }
    Ok(Doc {
        vv,
        fields: map,
        extra,
    })
}

/// Serializes a document in one of the two layouts. The output is
/// canonical: members in a fixed order, objects in key order, entries in
/// stamp order, compact JSON.
pub(crate) struct DocJson<'a> {
    pub(crate) doc: &'a Doc,
    pub(crate) format: Format,
}

struct VvJson<'a>(&'a VersionVector);
struct FieldsJson<'a>(&'a BTreeMap<Text, Register>);
struct RegisterJson<'a>(&'a Register);
struct EntryJson<'a>(&'a Entry);

impl Serialize for DocJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("schema_version", &SCHEMA_VERSION)?;
        m.serialize_entry("vv", &VvJson(&self.doc.vv))?;
        match self.format {
            Format::Document => m.serialize_entry("fields", &FieldsJson(&self.doc.fields))?,
            Format::Tombstone => {
                let empty = Register::default();
                let reg = self.doc.fields.get(DELETED_KEY).unwrap_or(&empty);
                m.serialize_entry("deleted", &RegisterJson(reg))?;
            }
        }
        for (k, v) in self.doc.extra.iter() {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

impl Serialize for VvJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (d, c) in self.0.iter() {
            m.serialize_entry(&d.to_string(), &c)?;
        }
        m.end()
    }
}

impl Serialize for FieldsJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, r) in self.0 {
            m.serialize_entry(k, &RegisterJson(r))?;
        }
        m.end()
    }
}

impl Serialize for RegisterJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for e in self.0.entries() {
            seq.serialize_element(&EntryJson(e))?;
        }
        seq.end()
    }
}

impl Serialize for EntryJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> core::result::Result<S::Ok, S::Error> {
        let e = self.0;
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("at", &e.at.as_unix_millis())?;
        m.serialize_entry("c", &e.stamp.counter())?;
        m.serialize_entry("d", &e.stamp.device().to_string())?;
        m.serialize_entry("v", &e.value)?;
        for (k, v) in e.extra.iter() {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::value::to_json;

    const D1: &str = "01010101-0101-0101-0101-010101010101";

    fn dev(b: u8) -> DeviceId {
        DeviceId::from_bytes([b; 16]).unwrap()
    }

    fn json(doc: &Doc, format: Format) -> String {
        let out = to_json(&DocJson { doc, format }, 1 << 20).unwrap();
        String::from_utf8(out.expose_secret().to_vec()).unwrap()
    }

    fn set(doc: &mut Doc, d: u8, key: &str, v: &str) -> Stamp {
        let s = doc.tick(dev(d)).unwrap();
        doc.write(key, s, Timestamp::from_unix_millis(7), Value::text(v), true)
            .unwrap();
        s
    }

    #[test]
    fn parse_device_is_strict() {
        assert_eq!(parse_device(D1).unwrap(), dev(1));
        for bad in [
            "01010101-0101-0101-0101-01010101010",
            "01010101-0101-0101-0101-0101010101010",
            "01010101x0101-0101-0101-010101010101",
            "0101010A-0101-0101-0101-010101010101",
            "0101010g-0101-0101-0101-010101010101",
            "00000000-0000-0000-0000-000000000000",
        ] {
            assert_eq!(parse_device(bad).unwrap_err(), Error::Malformed, "{bad}");
        }
    }

    #[test]
    fn canonical_layout() {
        let mut doc = Doc::default();
        set(&mut doc, 1, "title", "a");
        doc.extra.insert("zz", Value::Bool(true));
        assert_eq!(
            json(&doc, Format::Document),
            format!(
                r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"title":[{{"at":7,"c":1,"d":"{D1}","v":"a"}}]}},"zz":true}}"#
            )
        );
        let back = decode(json(&doc, Format::Document).as_bytes(), Format::Document).unwrap();
        assert_eq!(back, doc);
        doc.delete(dev(1), Timestamp::from_unix_millis(9)).unwrap();
        let t = json(&doc, Format::Tombstone);
        assert_eq!(
            t,
            format!(
                r#"{{"schema_version":1,"vv":{{"{D1}":2}},"deleted":[{{"at":9,"c":2,"d":"{D1}","v":null}}],"zz":true}}"#
            )
        );
        assert_eq!(decode(t.as_bytes(), Format::Tombstone).unwrap(), doc);
        // A tombstone layout is not a document layout and vice versa.
        assert_eq!(
            decode(t.as_bytes(), Format::Document).unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    #[allow(clippy::too_many_lines, reason = "one table of cases")]
    fn decode_rejects() {
        let ok = format!(r#"{{"schema_version":1,"vv":{{"{D1}":2}},"fields":{{}}}}"#);
        assert!(decode(ok.as_bytes(), Format::Document).is_ok());
        let entry = |extra: &str| {
            format!(
                r#"{{"schema_version":1,"vv":{{"{D1}":2}},"fields":{{"k":[{{"c":1,"d":"{D1}","at":0,"v":1{extra}}}]}}}}"#
            )
        };
        assert!(decode(entry("").as_bytes(), Format::Document).is_ok());
        assert!(decode(entry(r#","new":[1]"#).as_bytes(), Format::Document).is_ok());
        let cases = [
            ("[]".to_owned(), Error::Malformed),
            (r#"{"vv":{},"fields":{}}"#.to_owned(), Error::Malformed),
            (
                r#"{"schema_version":"1","vv":{},"fields":{}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":2,"vv":{},"fields":{}}"#.to_owned(),
                Error::UnsupportedVersion,
            ),
            (
                r#"{"schema_version":0,"vv":{},"fields":{}}"#.to_owned(),
                Error::UnsupportedVersion,
            ),
            (
                r#"{"schema_version":1,"fields":{}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":[],"fields":{}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                format!(r#"{{"schema_version":1,"vv":{{"{D1}":0}},"fields":{{}}}}"#),
                Error::Malformed,
            ),
            (
                format!(r#"{{"schema_version":1,"vv":{{"{D1}":-1}},"fields":{{}}}}"#),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{"x":1},"fields":{}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{},"fields":[]}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{},"fields":{"k":[]}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{},"fields":{"k":{}}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{},"fields":{"":[]}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                r#"{"schema_version":1,"vv":{},"fields":{"a\nb":[]}}"#.to_owned(),
                Error::Malformed,
            ),
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{}},"fields":{{"{}":[]}}}}"#,
                    "k".repeat(MAX_FIELD_KEY_LEN + 1)
                ),
                Error::Malformed,
            ),
            // Entry not covered by the vector.
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":2,"d":"{D1}","at":0,"v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
            // Duplicate stamp.
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":1,"d":"{D1}","at":0,"v":1}},{{"c":1,"d":"{D1}","at":0,"v":2}}]}}}}"#
                ),
                Error::Malformed,
            ),
            // Counter 0, missing members, wrong types.
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":0,"d":"{D1}","at":0,"v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":1,"d":"{D1}","v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":1,"d":"{D1}","at":1.5,"v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":1,"d":1,"at":0,"v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
            (
                format!(r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[1]}}}}"#),
                Error::Malformed,
            ),
            (
                format!(
                    r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{"k":[{{"c":"1","d":"{D1}","at":0,"v":1}}]}}}}"#
                ),
                Error::Malformed,
            ),
        ];
        for (input, err) in cases {
            assert_eq!(
                decode(input.as_bytes(), Format::Document).unwrap_err(),
                err,
                "{input}"
            );
        }
        assert_eq!(
            decode(
                br#"{"schema_version":1,"vv":{},"deleted":[]}"#,
                Format::Tombstone
            )
            .unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    fn concurrent_writes_conflict_and_merge() {
        let mut base = Doc::default();
        set(&mut base, 1, "title", "base");
        let mut a = base.dup();
        let mut b = base.dup();
        let sa = set(&mut a, 2, "title", "A");
        let sb = set(&mut b, 3, "title", "B");
        let m = a.merge(&b);
        assert_eq!(m, b.merge(&a));
        assert_eq!(m.merge(&m), m);
        assert_eq!(m.register("title").unwrap().len(), 2);
        assert_eq!(m.winner("title").unwrap().stamp(), sa.max(sb));
        let c = m.conflicts(|_| true);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].key, "title");
        assert_eq!(c[0].losers.len(), 1);
        assert!(m.conflicts(|k| k != "title").is_empty());
        // A later write keeps the loser as a conflict copy.
        let mut m2 = m.dup();
        set(&mut m2, 1, "title", "C");
        assert_eq!(m2.register("title").unwrap().len(), 2);
        assert_eq!(m2.value("title").unwrap().as_str(), Some("C"));
        // Merging the superseding state back removes the superseded winner.
        let back = m.merge(&m2);
        assert_eq!(back, m2);
        assert!(format!("{m2:?}").contains("fields"));
        assert!(format!("{:?}", m2.winner("title").unwrap()).contains("stamp"));
    }

    #[test]
    fn delete_and_concurrent_edit() {
        let mut base = Doc::default();
        set(&mut base, 1, "title", "t");
        set(&mut base, 1, "user", "u");
        let mut deleter = base.dup();
        deleter
            .delete(dev(2), Timestamp::from_unix_millis(1))
            .unwrap();
        assert!(deleter.is_deleted());
        let before = deleter.dup();
        deleter
            .delete(dev(2), Timestamp::from_unix_millis(2))
            .unwrap();
        assert_eq!(deleter, before, "deleting twice is a no-op");
        // Merging the stale live state does not resurrect anything.
        assert_eq!(deleter.merge(&base), deleter);
        // A concurrent edit survives and keeps the deletion entry as a flag.
        let mut editor = base.dup();
        set(&mut editor, 3, "user", "v");
        let m = deleter.merge(&editor);
        assert!(!m.is_deleted());
        assert!(m.register(DELETED_KEY).is_some());
        assert_eq!(m.value("user").unwrap().as_str(), Some("v"));
        assert!(m.value("title").is_none(), "seen by the deleter");
        assert_eq!(m.modified_at(), Some(Timestamp::from_unix_millis(7)));
    }

    #[test]
    fn same_stamp_different_content_breaks_ties_by_content() {
        let mut a = Doc::default();
        let s = a.tick(dev(1)).unwrap();
        let mut b = a.dup();
        a.write(
            "k",
            s,
            Timestamp::from_unix_millis(1),
            Value::text("x"),
            true,
        )
        .unwrap();
        b.write(
            "k",
            s,
            Timestamp::from_unix_millis(1),
            Value::text("y"),
            true,
        )
        .unwrap();
        let m = a.merge(&b);
        assert_eq!(m, b.merge(&a));
        assert_eq!(m.value("k").unwrap().as_str(), Some("y"));
        assert!(m.remove_entry_test("k", &s));

        // The repair finds the collision and brings the other value back
        // under a fresh stamp, so both are visible.
        let mut repaired = a.merge(&b);
        let found = repaired.repair_after_merge(&a, &b, dev(9)).unwrap();
        assert!(found.stamp_collision);
        assert_eq!(found.restored_fields, 0);
        assert_eq!(repaired.register("k").unwrap().len(), 2);
        assert_eq!(repaired.value("k").unwrap().as_str(), Some("x"));
        assert_eq!(repaired.conflicts(|_| true).len(), 1);
        // Same stamp, same value, different time: flagged, nothing added.
        let mut left = a.dup();
        let s2 = left.tick(dev(1)).unwrap();
        let mut right = left.dup();
        left.write("t", s2, Timestamp::from_unix_millis(1), Value::Null, false)
            .unwrap();
        right
            .write("t", s2, Timestamp::from_unix_millis(2), Value::Null, false)
            .unwrap();
        let mut both = left.merge(&right);
        let before = both.dup();
        assert!(
            both.repair_after_merge(&left, &right, dev(9))
                .unwrap()
                .stamp_collision
        );
        assert_eq!(both, before);
        // Honest replicas: nothing found, nothing changed.
        let mut aa = a.merge(&a);
        assert_eq!(
            aa.repair_after_merge(&a, &a, dev(9)).unwrap(),
            Repair::default()
        );
        assert_eq!(aa, a);
    }

    #[test]
    fn reserved_members_are_rejected_in_both_layouts() {
        let entry = format!(r#"[{{"at":0,"c":1,"d":"{D1}","v":null}}]"#);
        let doc_with_deleted =
            format!(r#"{{"schema_version":1,"vv":{{"{D1}":1}},"fields":{{}},"deleted":{entry}}}"#);
        let tomb_with_fields =
            format!(r#"{{"schema_version":1,"vv":{{"{D1}":1}},"deleted":{entry},"fields":{{}}}}"#);
        assert_eq!(
            decode(doc_with_deleted.as_bytes(), Format::Document).unwrap_err(),
            Error::Malformed
        );
        assert_eq!(
            decode(tomb_with_fields.as_bytes(), Format::Tombstone).unwrap_err(),
            Error::Malformed
        );
        // Any other unknown member is still kept.
        let ok =
            format!(r#"{{"schema_version":1,"vv":{{"{D1}":1}},"deleted":{entry},"later":{{}}}}"#);
        assert!(decode(ok.as_bytes(), Format::Tombstone).is_ok());
    }

    #[test]
    fn writes_check_keys_like_the_decoder() {
        let mut doc = Doc::default();
        let s = doc.tick(dev(1)).unwrap();
        let at = Timestamp::from_unix_millis(0);
        let long = "k".repeat(MAX_FIELD_KEY_LEN + 1);
        for bad in ["", "a\nb", "url:\u{7f}", long.as_str()] {
            assert_eq!(
                doc.write(bad, s, at, Value::Null, false).unwrap_err(),
                Error::Malformed
            );
        }
        let max = "k".repeat(MAX_FIELD_KEY_LEN);
        doc.write(&max, s, at, Value::Null, false).unwrap();
        let json = json(&doc, Format::Document);
        assert_eq!(decode(json.as_bytes(), Format::Document).unwrap(), doc);
    }

    impl Doc {
        fn remove_entry_test(&self, key: &str, s: &Stamp) -> bool {
            let mut d = self.dup();
            d.remove_entry(key, s).is_some() && d.register(key).is_none()
        }
    }
}
