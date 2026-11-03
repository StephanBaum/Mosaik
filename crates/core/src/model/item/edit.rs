//! The edit API of an item: [`ItemEdit`].

use core::fmt;

use super::{
    ATTACHMENT_PREFIX, CONFLICT_COPY_OF, ElementId, FAVORITE, FIELD_PREFIX, HISTORY_PREFIX,
    MAX_HISTORY, MAX_KEYS, MAX_TAG_LEN, OTP, OTP_COUNTER, PASSKEY, PASSKEY_SIGN_COUNT, SSH_KEY,
    SecretField, TAG_PREFIX, TRASHED, TextField, URL_PREFIX, history_stamp,
};
use crate::error::{Error, Result};
use crate::ids::{AttachmentId, DeviceId};
use crate::model::clock::{Stamp, Timestamp};
use crate::model::doc::{DELETED_KEY, Doc, Entry, Register, check_key};
use crate::model::types::{
    AttachmentRef, CustomField, CustomFieldValue, LoginUrl, OtpConfig, PasskeyCredential,
    SshKeyMaterial,
};
use crate::model::value::{Object, Text, Value};

pub(super) fn check_tag(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_TAG_LEN || name.chars().any(char::is_control) {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// The key of tag `name`, built in a zeroizing buffer (tag names are item
/// content; `format!` would leave an unwiped copy).
pub(super) fn tag_key(name: &str) -> Text {
    let mut s = String::with_capacity(TAG_PREFIX.len().saturating_add(name.len()));
    s.push_str(TAG_PREFIX);
    s.push_str(name);
    Text::from_string(s)
}

/// An edit of one item by one device. See [`Item::edit`].
pub struct ItemEdit<'a> {
    pub(super) doc: &'a mut Doc,
    pub(super) device: DeviceId,
    pub(super) at: Timestamp,
    pub(super) stamp: Option<Stamp>,
    pub(super) seq: u32,
}

impl fmt::Debug for ItemEdit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemEdit")
            .field("device", &self.device)
            .field("stamp", &self.stamp)
            .finish_non_exhaustive()
    }
}

impl ItemEdit<'_> {
    /// The OTP configuration as this edit currently sees it, so a caller can
    /// adjust it in the same edit ([`Item::otp`] for the item itself).
    #[must_use]
    pub fn otp(&self) -> Option<OtpConfig> {
        self.doc
            .value(OTP)
            .and_then(|v| OtpConfig::from_value(v).ok())
    }

    /// The stamp of this edit, ticked on the first change. A deleted item
    /// takes no edits ([`Item::undelete`] first).
    fn stamp(&mut self) -> Result<Stamp> {
        if let Some(s) = self.stamp {
            return Ok(s);
        }
        if self.doc.is_deleted() {
            return Err(Error::ItemDeleted);
        }
        let s = self.doc.tick(self.device)?;
        self.stamp = Some(s);
        Ok(s)
    }

    fn next_id(&mut self) -> Result<String> {
        let s = self.stamp()?;
        let id = format!("{:016x}.{}.{}", s.counter(), s.device(), self.seq);
        self.seq = self.seq.checked_add(1).ok_or(Error::ClockOverflow)?;
        Ok(id)
    }

    /// Writes `value` unless it already is the field's only (or, with
    /// `keep_losers`, winning) value.
    pub(super) fn put(&mut self, key: &str, value: Value, keep_losers: bool) -> Result<()> {
        check_key(key)?;
        if let Some(reg) = self.doc.register(key)
            && let Some(w) = reg.winner()
            && *w.value() == value
            && (keep_losers || reg.len() == 1)
        {
            return Ok(());
        }
        let s = self.stamp()?;
        self.doc.write(key, s, self.at, value, keep_losers)
    }

    fn remove(&mut self, key: &str) -> Result<bool> {
        if self.doc.register(key).is_none() {
            return Ok(false);
        }
        self.stamp()?;
        self.doc.remove_key(key);
        Ok(true)
    }

    /// Adds an old secret value to the password history and trims it.
    fn push_history(&mut self, field: SecretField, old: Text) -> Result<()> {
        let key = format!("{HISTORY_PREFIX}{}", self.next_id()?);
        let mut o = Object::new();
        o.insert("field", Value::text(field.key()));
        o.insert("value", Value::String(old));
        o.insert("changed_at", Value::int(self.at.as_unix_millis()));
        self.put(&key, Value::Object(o), false)?;
        // Trim the oldest entries by the Lamport stamp of the edit that
        // moved them into the history, never by the wall clock (display
        // only, and a skewed clock would trim the value this edit just moved
        // here), and never the entry this edit added. That stamp is the one
        // encoded in the key: a delete-race repair re-adds restored entries
        // under fresh stamps of the repairing device, so the register's
        // winner stamp would make the oldest entry look like the newest
        // (review M2, fourth round). Keys that do not parse fall back to the
        // winner stamp.
        let mut keys: Vec<(Option<Stamp>, String)> = self
            .doc
            .registers()
            .filter(|(k, _)| k.starts_with(HISTORY_PREFIX) && *k != key.as_str())
            .map(|(k, r)| {
                let order = history_stamp(k).or_else(|| r.winner().map(Entry::stamp));
                (order, k.to_owned())
            })
            .collect();
        let limit = MAX_HISTORY.saturating_sub(1);
        if keys.len() > limit {
            keys.sort();
            let excess = keys.len() - limit;
            for (_, k) in keys.into_iter().take(excess) {
                self.remove(&k)?;
            }
        }
        Ok(())
    }

    /// The winning string of `field`, copied.
    fn current_secret(&self, field: SecretField) -> Option<Text> {
        match self.doc.value(field.key()) {
            Some(Value::String(t)) => Some(t.dup()),
            _ => None,
        }
    }

    /// Sets a text field.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_text(&mut self, field: TextField, value: &str) -> Result<()> {
        self.put(field.key(), Value::text(value), true)
    }

    /// Sets the title.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_title(&mut self, value: &str) -> Result<()> {
        self.set_text(TextField::Title, value)
    }

    /// Removes a text field.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn clear_text(&mut self, field: TextField) -> Result<()> {
        self.remove(field.key()).map(|_| ())
    }

    /// Sets a secret field. For fields that keep history the previous value
    /// goes into the password history.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_secret(&mut self, field: SecretField, value: &str) -> Result<()> {
        if let Some(old) = self.current_secret(field)
            && old.as_str() != value
            && field.keeps_history()
        {
            self.push_history(field, old)?;
        }
        self.put(field.key(), Value::text(value), true)
    }

    /// Removes a secret field (keeping the old value in the history for
    /// fields that keep history).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn clear_secret(&mut self, field: SecretField) -> Result<()> {
        if let Some(old) = self.current_secret(field)
            && field.keeps_history()
        {
            self.push_history(field, old)?;
        }
        self.remove(field.key()).map(|_| ())
    }

    /// Removes every password history entry.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn clear_password_history(&mut self) -> Result<()> {
        let keys: Vec<String> = self
            .doc
            .keys()
            .filter(|k| k.starts_with(HISTORY_PREFIX))
            .map(str::to_owned)
            .collect();
        for k in keys {
            self.remove(&k)?;
        }
        Ok(())
    }

    /// Marks or unmarks the item as a favorite.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_favorite(&mut self, favorite: bool) -> Result<()> {
        self.put(FAVORITE, Value::Bool(favorite), false)
    }

    /// Moves the item to the trash (CRYPTO_SPEC §7.4).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn trash(&mut self) -> Result<()> {
        if self.doc.register(TRASHED).is_some() {
            return Ok(());
        }
        self.put(TRASHED, Value::int(self.at.as_unix_millis()), false)
    }

    /// Restores the item from the trash.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn restore_from_trash(&mut self) -> Result<()> {
        self.remove(TRASHED).map(|_| ())
    }

    /// Adds a tag.
    ///
    /// # Errors
    /// [`Error::Malformed`] for an empty tag, one longer than
    /// [`MAX_TAG_LEN`] or one with control characters;
    /// [`Error::ClockOverflow`].
    pub fn add_tag(&mut self, name: &str) -> Result<()> {
        check_tag(name)?;
        self.put(tag_key(name).as_str(), Value::Bool(true), false)
    }

    /// Removes a tag.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn remove_tag(&mut self, name: &str) -> Result<()> {
        self.remove(tag_key(name).as_str()).map(|_| ())
    }

    /// Adds a login URL.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn add_url(&mut self, url: &LoginUrl) -> Result<ElementId> {
        let id = self.next_id()?;
        self.put(&format!("{URL_PREFIX}{id}"), url.to_value(), false)?;
        Ok(ElementId(id))
    }

    /// Replaces a login URL.
    ///
    /// # Errors
    /// [`Error::Malformed`] if the item has no URL `id`;
    /// [`Error::ClockOverflow`].
    pub fn set_url(&mut self, id: &ElementId, url: &LoginUrl) -> Result<()> {
        let key = self.existing(URL_PREFIX, id)?;
        self.put(&key, url.to_value(), true)
    }

    /// The key of element `id` under `prefix`, if the item has it.
    fn existing(&self, prefix: &str, id: &ElementId) -> Result<String> {
        let key = format!("{prefix}{}", id.0);
        if self.doc.register(&key).is_none() {
            return Err(Error::Malformed);
        }
        Ok(key)
    }

    /// Removes a login URL.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn remove_url(&mut self, id: &ElementId) -> Result<()> {
        self.remove(&format!("{URL_PREFIX}{}", id.0)).map(|_| ())
    }

    /// Adds a custom field.
    ///
    /// # Errors
    /// [`Error::Malformed`] for an invalid OTP value;
    /// [`Error::ClockOverflow`].
    pub fn add_custom_field(&mut self, field: &CustomField) -> Result<ElementId> {
        check_custom(field)?;
        let id = self.next_id()?;
        self.put(&format!("{FIELD_PREFIX}{id}"), field.to_value(), false)?;
        Ok(ElementId(id))
    }

    /// Replaces a custom field.
    ///
    /// # Errors
    /// [`Error::Malformed`] for an invalid OTP value or if the item has no
    /// custom field `id`; [`Error::ClockOverflow`].
    pub fn set_custom_field(&mut self, id: &ElementId, field: &CustomField) -> Result<()> {
        check_custom(field)?;
        let key = self.existing(FIELD_PREFIX, id)?;
        self.put(&key, field.to_value(), true)
    }

    /// Removes a custom field.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn remove_custom_field(&mut self, id: &ElementId) -> Result<()> {
        self.remove(&format!("{FIELD_PREFIX}{}", id.0)).map(|_| ())
    }

    /// Adds (or replaces) an attachment reference.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above 50 MiB; [`Error::ClockOverflow`].
    pub fn set_attachment(&mut self, attachment: &AttachmentRef) -> Result<()> {
        let v = attachment.to_value()?;
        self.put(
            &format!("{ATTACHMENT_PREFIX}{}", attachment.attachment_id),
            v,
            true,
        )
    }

    /// Removes an attachment reference. The caller deletes the blob.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn remove_attachment(&mut self, id: &AttachmentId) -> Result<()> {
        self.remove(&format!("{ATTACHMENT_PREFIX}{id}")).map(|_| ())
    }

    /// Sets the OTP configuration.
    ///
    /// # Errors
    /// [`Error::Malformed`] if [`OtpConfig::validate`] fails;
    /// [`Error::ClockOverflow`].
    pub fn set_otp(&mut self, otp: &OtpConfig) -> Result<()> {
        otp.validate()?;
        self.put(OTP, otp.to_value(), true)
    }

    /// Removes the OTP configuration.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn clear_otp(&mut self) -> Result<()> {
        self.remove(OTP).map(|_| ())
    }

    /// Raises the HOTP counter. Counters never go down: a value at or below
    /// the current one is ignored.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_otp_counter(&mut self, counter: u64) -> Result<()> {
        self.raise(OTP_COUNTER, counter)
    }

    /// Sets the passkey credential.
    ///
    /// # Errors
    /// [`Error::Malformed`] if [`PasskeyCredential::validate`] fails;
    /// [`Error::ClockOverflow`].
    pub fn set_passkey(&mut self, passkey: &PasskeyCredential) -> Result<()> {
        passkey.validate()?;
        self.put(PASSKEY, passkey.to_value(), true)
    }

    /// Raises the passkey signature counter (never lowers it).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_passkey_sign_count(&mut self, count: u64) -> Result<()> {
        self.raise(PASSKEY_SIGN_COUNT, count)
    }

    /// Sets the SSH key material.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_ssh_key(&mut self, key: &SshKeyMaterial) -> Result<()> {
        self.put(SSH_KEY, key.to_value(), true)
    }

    fn raise(&mut self, key: &str, value: u64) -> Result<()> {
        let current = self
            .doc
            .register(key)
            .into_iter()
            .flat_map(Register::entries)
            .filter_map(|e| e.value().as_u64())
            .max();
        if current.is_some_and(|c| c >= value)
            && self.doc.register(key).is_some_and(|r| r.len() == 1)
        {
            return Ok(());
        }
        self.put(key, Value::uint(value.max(current.unwrap_or(0))), false)
    }

    /// The entry of counter `key` with the largest value.
    fn max_entry(&self, key: &str) -> Option<(Stamp, u64)> {
        self.doc
            .register(key)?
            .entries()
            .filter_map(|e| e.value().as_u64().map(|v| (e.stamp(), v)))
            .max_by_key(|&(_, v)| v)
    }

    /// Resolves a conflict on `key` by keeping the value written by edit
    /// `keep` (one of the entries listed by [`Item::conflicts`]). The other
    /// values of a field that keeps history go into the password history.
    /// Counters (`otp.counter`, `passkey.sign_count`) never go down: for
    /// them only the entry with the largest value can be kept.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `key` has no value with stamp `keep`, or
    /// `keep` is a counter value below the largest one;
    /// [`Error::ClockOverflow`].
    pub fn resolve_conflict(&mut self, key: &str, keep: Stamp) -> Result<()> {
        let reg = self.doc.register(key).ok_or(Error::Malformed)?;
        let chosen = reg.get(&keep).ok_or(Error::Malformed)?.value().dup();
        if MAX_KEYS.contains(&key) {
            return match self.max_entry(key) {
                Some((_, max)) if chosen.as_u64() == Some(max) => self.raise(key, max),
                _ => Err(Error::Malformed),
            };
        }
        let is_winner = reg.winner().map(Entry::stamp) == Some(keep);
        let others: Vec<(Stamp, Option<Text>)> = reg
            .entries()
            .filter(|e| e.stamp() != keep)
            .map(|e| (e.stamp(), e.value().as_str().map(Text::new)))
            .collect();
        if others.is_empty() {
            return Ok(());
        }
        if let Some(field) = SecretField::from_key(key).filter(|f| f.keeps_history()) {
            let mut seen: Vec<Text> = Vec::new();
            for (_, v) in &others {
                if let Some(v) = v
                    && Some(v.as_str()) != chosen.as_str()
                    && !seen.contains(v)
                {
                    seen.push(v.dup());
                    self.push_history(field, v.dup())?;
                }
            }
        }
        self.stamp()?;
        if is_winner {
            for (s, _) in others {
                self.doc.remove_entry(key, &s);
            }
        } else {
            let s = self.stamp()?;
            self.doc.write(key, s, self.at, chosen, false)?;
        }
        Ok(())
    }

    /// Keeps the winning value of `key` and drops the conflict copies. For
    /// a counter the winner is the largest value.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `key` has no value; [`Error::ClockOverflow`].
    pub fn dismiss_conflict(&mut self, key: &str) -> Result<()> {
        let winner = if MAX_KEYS.contains(&key) {
            self.max_entry(key).map(|(s, _)| s)
        } else {
            self.doc.winner(key).map(Entry::stamp)
        };
        self.resolve_conflict(key, winner.ok_or(Error::Malformed)?)
    }

    /// Clears the "conflict copy" mark (see [`Item::conflict_copy_of`]).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn dismiss_conflict_copy(&mut self) -> Result<()> {
        self.remove(CONFLICT_COPY_OF).map(|_| ())
    }

    /// Clears the "deleted concurrently" flag (see
    /// [`Item::deleted_concurrently`]).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn dismiss_deletion(&mut self) -> Result<()> {
        if self.doc.is_deleted() {
            return Ok(());
        }
        self.remove(DELETED_KEY).map(|_| ())
    }
}

fn check_custom(field: &CustomField) -> Result<()> {
    if let CustomFieldValue::Otp(c) = &field.value {
        c.validate()?;
    }
    Ok(())
}
