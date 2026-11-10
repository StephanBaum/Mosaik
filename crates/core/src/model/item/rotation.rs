//! Credential rotation attempts in the item plaintext (M7b, CRYPTO_SPEC
//! §7.4, ADR-0066).
//!
//! A guided password change stores the new password in the item **before**
//! anything reaches the site: an *attempt* under its own key
//! `rotation:<id>`. One key per attempt, so attempts started concurrently on
//! two devices never collide. The attempt ends in one edit:
//!
//! - [`ItemEdit::confirm_rotation`]: the new value becomes the password, the
//!   old one goes to the history, every other open attempt goes to the
//!   history as [`HistoryReason::Unconfirmed`];
//! - [`ItemEdit::fail_rotation`]: the password stays, the attempt's value
//!   goes to the history as unconfirmed.
//!
//! **Invariant:** a value that was ever an attempt's new password is never
//! deleted. It is the password, an open attempt, or a history entry, until
//! the history's count limit ([`super::MAX_HISTORY`]) trims the oldest
//! entries. [`ItemEdit::clear_password_history`] refuses while an attempt is
//! open and keeps recent unconfirmed entries.

use core::fmt;

use super::{
    HISTORY_PREFIX, HistoryEntry, HistoryId, HistoryReason, Item, ItemEdit, ROTATION_POLICY,
    ROTATION_PREFIX, SecretField, TYPE, parse_element_id,
};
use crate::error::{Error, Result};
use crate::ids::DeviceId;
use crate::model::clock::{Stamp, Timestamp};
use crate::model::doc::{Doc, Entry, Register};
use crate::model::value::{Object, Text, Value};
use crate::secret::SecretString;

/// Most open attempts per item ([`Error::TooManyAttempts`]).
pub const MAX_OPEN_ATTEMPTS: usize = 4;
/// Longest new password of an attempt, in characters.
pub const MAX_PENDING_CHARS: usize = 1024;
/// How long an unconfirmed history entry survives
/// [`ItemEdit::clear_password_history`], in milliseconds (30 days).
const UNCONFIRMED_KEEP_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// The reminder intervals [`ItemEdit::set_rotation_policy`] accepts.
const POLICY_DAYS: [u16; 4] = [30, 90, 180, 365];

/// How far a rotation attempt got. Only rises.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RotationStage {
    /// Stored in the item; nothing was given to a page yet.
    Prepared,
    /// The new password was handed to a page (the site may have applied it).
    Delivered,
}

impl RotationStage {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Delivered => "delivered",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "prepared" => Some(Self::Prepared),
            "delivered" => Some(Self::Delivered),
            _ => None,
        }
    }
}

/// Where the rules the new password was generated under came from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RulesSource {
    /// The page's `passwordrules` attribute.
    Page,
    /// The vendored per-site rules of the generator.
    SiteQuirks,
}

impl RulesSource {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::SiteQuirks => "siteQuirks",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "page" => Some(Self::Page),
            "siteQuirks" => Some(Self::SiteQuirks),
            _ => None,
        }
    }
}

/// The ID of one rotation attempt: the key suffix, which names the creating
/// edit (`{counter:016x}.{device}.{seq}`, at most 64 ASCII characters).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RotationAttemptId {
    stamp: Stamp,
    seq: u32,
}

impl RotationAttemptId {
    /// Parses a stored ID (as printed by `Display`).
    ///
    /// # Errors
    /// [`Error::Malformed`] if `id` is not of the form
    /// `{counter:016x}.{device}.{seq}`.
    pub fn from_stored(id: &str) -> Result<Self> {
        let (stamp, seq) = parse_element_id(id).ok_or(Error::Malformed)?;
        Ok(Self { stamp, seq })
    }

    /// The edit that started the attempt.
    #[must_use]
    pub const fn creation_stamp(&self) -> Stamp {
        self.stamp
    }

    fn key(&self) -> String {
        format!("{ROTATION_PREFIX}{self}")
    }
}

impl fmt::Display for RotationAttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016x}.{}.{}",
            self.stamp.counter(),
            self.stamp.device(),
            self.seq
        )
    }
}

/// An opaque mark of the current value of a secret field: the Lamport stamp
/// of the register's winning entry (24 bytes). It changes on every write of
/// the field and is **not** derived from the value, so it is safe to store
/// outside the item (breach state, snooze, dismissal).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ValueMark([u8; ValueMark::LEN]);

impl ValueMark {
    /// Length of the encoded mark.
    pub const LEN: usize = 24;

    /// The mark as bytes: the counter (big endian) then the device ID.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }

    /// Reads a stored mark.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        <[u8; Self::LEN]>::try_from(bytes).ok().map(Self)
    }

    fn of(stamp: Stamp) -> Self {
        let mut out = [0u8; Self::LEN];
        let (counter, device) = out.split_at_mut(8);
        counter.copy_from_slice(&stamp.counter().to_be_bytes());
        device.copy_from_slice(stamp.device().as_bytes());
        Self(out)
    }
}

/// An open rotation attempt. `Debug` never shows `pending`.
pub struct RotationAttempt {
    /// The attempt's ID.
    pub id: RotationAttemptId,
    /// The new password, stored before it reaches any site.
    pub pending: SecretString,
    /// When the attempt started, by the writer's clock.
    pub started_at: Timestamp,
    /// How far it got.
    pub stage: RotationStage,
    /// Where the generator rules came from, if known.
    pub rules_source: Option<RulesSource>,
}

impl fmt::Debug for RotationAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RotationAttempt")
            .field("id", &self.id)
            .field("started_at", &self.started_at)
            .field("stage", &self.stage)
            .field("rules_source", &self.rules_source)
            .finish_non_exhaustive()
    }
}

/// A decoded `rotation:<id>` value, the new password in a zeroizing buffer.
/// `object` is the stored object itself: a rewrite changes only `stage` and
/// keeps members this build does not know (CRYPTO_SPEC §7.5).
struct Parsed {
    pending: Text,
    started_at: Timestamp,
    stage: RotationStage,
    rules_source: Option<RulesSource>,
    object: Object,
}

impl Parsed {
    fn from_value(v: &Value) -> Result<Self> {
        let o = v.as_object().ok_or(Error::Malformed)?;
        let pending = match o.get("pending") {
            Some(Value::String(t)) => t.dup(),
            _ => return Err(Error::Malformed),
        };
        let started_at = o
            .get("started_at")
            .and_then(Value::as_i64)
            .ok_or(Error::Malformed)?;
        let stage = o
            .get("stage")
            .and_then(Value::as_str)
            .and_then(RotationStage::parse)
            .ok_or(Error::Malformed)?;
        let rules_source = match o.get("rules_source") {
            None | Some(Value::Null) => None,
            Some(v) => Some(
                v.as_str()
                    .and_then(RulesSource::parse)
                    .ok_or(Error::Malformed)?,
            ),
        };
        Ok(Self {
            pending,
            started_at: Timestamp::from_unix_millis(started_at),
            stage,
            rules_source,
            object: o.dup(),
        })
    }

    /// A new attempt, stage `prepared`.
    fn fresh(pending: &str, started_at: Timestamp, rules_source: Option<RulesSource>) -> Self {
        let mut object = Object::new();
        object.insert("pending", Value::text(pending));
        object.insert("started_at", Value::int(started_at.as_unix_millis()));
        object.insert("stage", Value::text(RotationStage::Prepared.as_str()));
        object.insert(
            "rules_source",
            rules_source.map_or(Value::Null, |r| Value::text(r.as_str())),
        );
        Self {
            pending: Text::new(pending),
            started_at,
            stage: RotationStage::Prepared,
            rules_source,
            object,
        }
    }

    /// The stored object with `stage` replaced and everything else kept.
    fn to_value(&self, stage: RotationStage) -> Value {
        let mut o = self.object.dup();
        o.insert("stage", Value::text(stage.as_str()));
        Value::Object(o)
    }
}

/// Every distinct `pending` value of a register, the winner's first. One
/// register holds several only after a device-ID collision (ADR-0015).
fn distinct_pendings(reg: &Register) -> Vec<Text> {
    let mut out: Vec<Text> = Vec::new();
    let winner = reg.winner().map(Entry::value);
    for v in winner
        .into_iter()
        .chain(reg.entries().rev().map(Entry::value))
    {
        if let Ok(p) = Parsed::from_value(v)
            && !out.contains(&p.pending)
        {
            out.push(p.pending);
        }
    }
    out
}

/// An attempt as its register holds it. The stage is the highest any
/// live entry has (it only rises); the rest is the winner's.
fn attempt_of(reg: &Register) -> Option<(Parsed, RotationStage)> {
    let parsed = Parsed::from_value(reg.winner()?.value()).ok()?;
    let stage = reg
        .entries()
        .filter_map(|e| Parsed::from_value(e.value()).ok())
        .map(|p| p.stage)
        .max()
        .unwrap_or(parsed.stage);
    Some((parsed, stage))
}

/// Whether `key` holds a rotation attempt this build can read.
pub(super) fn attempt_readable(key: &str, v: &Value) -> bool {
    key.strip_prefix(ROTATION_PREFIX)
        .is_some_and(|id| RotationAttemptId::from_stored(id).is_ok())
        && Parsed::from_value(v).is_ok()
}

/// The reminder interval of a `rotation_policy` value.
pub(super) fn policy_of(v: &Value) -> Option<u16> {
    let days = v.as_object()?.get("every_days")?.as_u64()?;
    let days = u16::try_from(days).ok()?;
    POLICY_DAYS.contains(&days).then_some(days)
}

/// Whether a history value is an unconfirmed entry moved there less than 30
/// days before `now`.
pub(super) fn is_recent_unconfirmed(v: &Value, now: i64) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    o.get("reason").and_then(Value::as_str) == Some("unconfirmed")
        && o.get("changed_at")
            .and_then(Value::as_i64)
            .is_some_and(|at| now.saturating_sub(at) < UNCONFIRMED_KEEP_MS)
}

impl Item {
    /// The open rotation attempts, sorted by start time and then ID.
    #[must_use]
    pub fn rotation_attempts(&self) -> Vec<RotationAttempt> {
        let mut out: Vec<RotationAttempt> = self
            .doc
            .registers()
            .filter_map(|(k, reg)| {
                let id = RotationAttemptId::from_stored(k.strip_prefix(ROTATION_PREFIX)?).ok()?;
                let (p, stage) = attempt_of(reg)?;
                Some(RotationAttempt {
                    id,
                    pending: p.pending.to_secret().ok()?,
                    started_at: p.started_at,
                    stage,
                    rules_source: p.rules_source,
                })
            })
            .collect();
        out.sort_by_key(|a| (a.started_at, a.id));
        out
    }

    /// The open attempt `id`.
    #[must_use]
    pub fn rotation_attempt(&self, id: RotationAttemptId) -> Option<RotationAttempt> {
        let reg = self.doc.register(&id.key())?;
        let (p, stage) = attempt_of(reg)?;
        Some(RotationAttempt {
            id,
            pending: p.pending.to_secret().ok()?,
            started_at: p.started_at,
            stage,
            rules_source: p.rules_source,
        })
    }

    /// The reminder interval in days ("remind me every 90 days"), if set.
    #[must_use]
    pub fn rotation_policy(&self) -> Option<u16> {
        self.doc.value(ROTATION_POLICY).and_then(policy_of)
    }

    /// An opaque mark of the current value of `field` (see [`ValueMark`]):
    /// the Lamport stamp of the register's winning entry. Changes on every
    /// write of the field, also when the same text is written again by a
    /// merge or a conflict resolution; not derived from the value.
    #[must_use]
    pub fn secret_mark(&self, field: SecretField) -> Option<ValueMark> {
        self.doc
            .winner(field.key())
            .map(|w| ValueMark::of(w.stamp()))
    }
}

impl ItemEdit<'_> {
    /// Every readable open attempt as `(key, every distinct pending value)`.
    fn open_attempts(&self) -> Vec<(String, Vec<Text>)> {
        self.doc
            .registers()
            .filter(|(k, _)| k.starts_with(ROTATION_PREFIX))
            .map(|(k, r)| (k.to_owned(), distinct_pendings(r)))
            .filter(|(_, v)| !v.is_empty())
            .collect()
    }

    /// The attempt `id` (winner and highest stage) and every distinct
    /// pending value of its register, the winner's first.
    fn attempt(&self, id: RotationAttemptId) -> Result<(String, Parsed, RotationStage, Vec<Text>)> {
        let key = id.key();
        let reg = self.doc.register(&key).ok_or(Error::RotationNotFound)?;
        let (p, stage) = attempt_of(reg).ok_or(Error::RotationNotFound)?;
        Ok((key, p, stage, distinct_pendings(reg)))
    }

    /// Starts a rotation attempt: stores `pending`, the new password, in the
    /// item before anything reaches a site. The current password is not
    /// touched.
    ///
    /// # Errors
    /// [`Error::NotALogin`] unless the item is a login;
    /// [`Error::Malformed`] if `pending` is empty or longer than
    /// [`MAX_PENDING_CHARS`] characters; [`Error::TooManyAttempts`] with
    /// [`MAX_OPEN_ATTEMPTS`] attempts open; [`Error::ClockOverflow`],
    /// [`Error::ItemDeleted`].
    pub fn begin_rotation(
        &mut self,
        pending: &str,
        rules: Option<RulesSource>,
    ) -> Result<RotationAttemptId> {
        if self.doc.value(TYPE).and_then(Value::as_str) != Some("login") {
            return Err(Error::NotALogin);
        }
        let chars = pending.chars().count();
        if chars == 0 || chars > MAX_PENDING_CHARS {
            return Err(Error::Malformed);
        }
        let open = self
            .doc
            .keys()
            .filter(|k| k.starts_with(ROTATION_PREFIX))
            .count();
        if open >= MAX_OPEN_ATTEMPTS {
            return Err(Error::TooManyAttempts);
        }
        let id = RotationAttemptId::from_stored(&self.next_id()?)?;
        let attempt = Parsed::fresh(pending, self.at, rules);
        self.put(&id.key(), attempt.to_value(RotationStage::Prepared), false)?;
        Ok(id)
    }

    /// Marks the attempt as handed to a page. `Ok(true)` if the stage
    /// changed, `Ok(false)` if it already was delivered.
    ///
    /// # Errors
    /// [`Error::RotationNotFound`]; [`Error::ClockOverflow`].
    pub fn mark_rotation_delivered(&mut self, id: RotationAttemptId) -> Result<bool> {
        let (key, p, stage, _) = self.attempt(id)?;
        if stage == RotationStage::Delivered && p.stage == stage {
            return Ok(false);
        }
        // Entries with other values (a device-ID collision) stay.
        self.put(&key, p.to_value(RotationStage::Delivered), true)?;
        Ok(true)
    }

    /// The site accepted the new password: `id`'s value becomes the
    /// password (the old one goes to the history as replaced), every other
    /// open attempt goes to the history as unconfirmed, and no attempt
    /// stays. One edit, one stamp.
    ///
    /// # Errors
    /// [`Error::RotationNotFound`]; [`Error::ClockOverflow`].
    pub fn confirm_rotation(&mut self, id: RotationAttemptId) -> Result<()> {
        let (key, chosen, _, own) = self.attempt(id)?;
        let others: Vec<(String, Vec<Text>)> = self
            .open_attempts()
            .into_iter()
            .filter(|(k, _)| *k != key)
            .collect();
        self.set_secret(SecretField::Password, chosen.pending.as_str())?;
        // Every other value, of this register and of the other attempts.
        let mut moved: Vec<Text> = Vec::new();
        let extra = own.iter().chain(others.iter().flat_map(|(_, v)| v.iter()));
        for v in extra {
            if *v != chosen.pending && !moved.contains(v) {
                moved.push(v.dup());
                self.push_history(SecretField::Password, v.dup(), HistoryReason::Unconfirmed)?;
            }
        }
        for (k, _) in others {
            self.remove(&k)?;
        }
        self.remove(&key)?;
        Ok(())
    }

    /// The site did not accept the new password, or the user gave up: the
    /// password stays, `id`'s value goes to the history as unconfirmed (the
    /// site may have applied it anyway) and the attempt is closed.
    ///
    /// # Errors
    /// [`Error::RotationNotFound`]; [`Error::ClockOverflow`].
    pub fn fail_rotation(&mut self, id: RotationAttemptId) -> Result<()> {
        let (key, _, _, own) = self.attempt(id)?;
        for v in own {
            self.push_history(SecretField::Password, v, HistoryReason::Unconfirmed)?;
        }
        self.remove(&key)?;
        Ok(())
    }

    /// Rollback: history entry `id`'s value becomes the field's value, and
    /// the value it replaces goes to the history as replaced. The entry
    /// itself is removed (its value is the field's value now).
    ///
    /// # Errors
    /// [`Error::HistoryNotFound`]; [`Error::ClockOverflow`].
    pub fn restore_history(&mut self, id: &HistoryId) -> Result<()> {
        let key = format!("{HISTORY_PREFIX}{id}");
        let entry = self
            .doc
            .value(&key)
            .and_then(|v| HistoryEntry::from_value(id.to_string(), v).ok())
            .ok_or(Error::HistoryNotFound)?;
        self.remove(&key)?;
        self.set_secret(entry.field, entry.value.expose_secret())
    }

    /// Sets (or, with `None`, clears) the reminder interval in days.
    ///
    /// # Errors
    /// [`Error::Malformed`] unless `every_days` is 30, 90, 180 or 365;
    /// [`Error::ClockOverflow`].
    pub fn set_rotation_policy(&mut self, every_days: Option<u16>) -> Result<()> {
        match every_days {
            None => self.remove(ROTATION_POLICY).map(|_| ()),
            Some(d) if POLICY_DAYS.contains(&d) => {
                let mut o = Object::new();
                o.insert("every_days", Value::uint(u64::from(d)));
                self.put(ROTATION_POLICY, Value::Object(o), false)
            }
            Some(_) => Err(Error::Malformed),
        }
    }

    /// The repair of one attempt that lost against a concurrent close (see
    /// [`repair_lost_attempts`]).
    fn repair_attempt(&mut self, key: &str, pendings: &[Text]) -> Result<()> {
        let held_as_password = |doc: &Doc, v: &Text| {
            doc.register(SecretField::Password.key())
                .into_iter()
                .flat_map(Register::entries)
                .any(|e| e.value().as_str() == Some(v.as_str()))
        };
        let in_history = |doc: &Doc, v: &Text| {
            doc.registers()
                .filter(|(k, _)| k.starts_with(HISTORY_PREFIX))
                .flat_map(|(_, r)| r.entries())
                .any(|e| {
                    e.value()
                        .as_object()
                        .and_then(|o| o.get("value"))
                        .and_then(Value::as_str)
                        == Some(v.as_str())
                })
        };
        // Every value the register holds is kept somewhere.
        for v in pendings {
            if !held_as_password(self.doc, v) && !in_history(self.doc, v) {
                self.push_history(SecretField::Password, v.dup(), HistoryReason::Unconfirmed)?;
            }
        }
        // Confirmed on the other device: nothing is open any more.
        if pendings
            .first()
            .is_some_and(|w| held_as_password(self.doc, w))
        {
            self.remove(key)?;
        }
        Ok(())
    }
}

/// Repairs `doc`, the merge of `local` and `received`, for attempts that
/// lost against a concurrent close (CRYPTO_SPEC §7.4): one side removed
/// `rotation:<id>` (it has seen the edit that created the attempt but holds
/// no key) while the other wrote the key concurrently
/// ([`ItemEdit::mark_rotation_delivered`]), so the pure merge keeps the
/// attempt. The attempt stays open ("delivered beats close"); if the other
/// device made its value the password the attempt is closed, and if neither
/// the password nor any history entry holds the value any more it is added
/// to the history as unconfirmed. The result is one local edit by `device`.
pub(super) fn repair_lost_attempts(
    doc: &mut Doc,
    local: &Doc,
    received: &Doc,
    device: DeviceId,
) -> Result<()> {
    if doc.is_deleted() {
        return Ok(());
    }
    let lost: Vec<(String, Vec<Text>, Timestamp)> = doc
        .registers()
        .filter_map(|(k, reg)| {
            let id = RotationAttemptId::from_stored(k.strip_prefix(ROTATION_PREFIX)?).ok()?;
            let w = reg.winner()?;
            let raced = [local, received].iter().any(|side| {
                side.register(k).is_none()
                    && side.vv().covers(&id.stamp)
                    && !side.vv().covers(&w.stamp())
            });
            if !raced {
                return None;
            }
            Some((k.to_owned(), distinct_pendings(reg), w.at()))
        })
        .collect();
    if lost.is_empty() {
        return Ok(());
    }
    let mut edit = ItemEdit {
        doc,
        device,
        at: Timestamp::from_unix_millis(0),
        stamp: None,
        seq: 0,
    };
    for (key, pendings, at) in lost {
        edit.at = at;
        edit.repair_attempt(&key, &pendings)?;
    }
    Ok(())
}
