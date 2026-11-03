//! Items: the typed view and the edit API over the replicated document.
//!
//! Every item field is one key of the document (see [`super::doc`]):
//!
//! | Key | Value |
//! |---|---|
//! | `type` | item type, set at creation (`login`, `passkey`, `totp`, `note`, `card`, `identity`, `ssh_key`, `api_key`, `wifi`, `document`) |
//! | `created_at`, `trashed_at` | wall-clock milliseconds; no `trashed_at` means not in the trash |
//! | `title`, `notes`, `favorite` | common fields |
//! | [`TextField`] / [`SecretField`] keys | one string each (`login.username`, `login.password`, `card.number`, ...) |
//! | `url:<id>` | a [`LoginUrl`] |
//! | `field:<id>` | a [`CustomField`] |
//! | `attachment:<uuid>` | an [`AttachmentRef`] |
//! | `tag:<name>` | `true` |
//! | `history:<id>` | a password history entry |
//! | `otp`, `passkey`, `ssh.key` | an [`OtpConfig`], [`PasskeyCredential`], [`SshKeyMaterial`] |
//! | `otp.counter`, `passkey.sign_count` | counters, merged by maximum |
//! | `deleted` | deletion entries (tombstone, or the flag of a delete that lost against a concurrent edit) |
//! | `conflict_copy_of` | the item ID (lowercase hyphenated UUID) of the record this item was split off from because a merge did not fit (ADR-0016) |
//!
//! Collections are one key per element, so concurrent edits of different
//! URLs, tags or custom fields never conflict. Keys this build does not
//! know are kept and merged like any other field.

use core::fmt;

mod edit;
mod fields;

pub use edit::ItemEdit;
pub use fields::{ElementId, HistoryEntry, ItemType, SecretField, TextField};

use super::clock::{Causality, Stamp, Timestamp, VersionVector};
use super::doc::{
    DELETED_KEY, Doc, DocJson, Entry, FieldConflict, Format, SCHEMA_VERSION, decode, parse_uuid,
};
use super::tombstone::Tombstone;
use super::types::{
    AttachmentRef, CustomField, LoginUrl, OtpConfig, PasskeyCredential, SshKeyMaterial,
};
use super::value::{Value, to_json};
use crate::envelope::{ItemCiphertext, MAX_ITEM_PLAINTEXT_LEN};
use crate::error::{Error, Result};
use crate::ids::{DeviceId, ItemId, VaultId};
use crate::secret::{SecretString, SecretVec, VaultKey};

/// Most password history entries kept per item; older ones are removed.
pub const MAX_HISTORY: usize = 32;
/// Longest tag name in bytes.
pub const MAX_TAG_LEN: usize = 128;

const TYPE: &str = "type";
const CREATED: &str = "created_at";
const TRASHED: &str = "trashed_at";
const FAVORITE: &str = "favorite";
const OTP: &str = "otp";
const OTP_COUNTER: &str = "otp.counter";
const PASSKEY: &str = "passkey";
const PASSKEY_SIGN_COUNT: &str = "passkey.sign_count";
const SSH_KEY: &str = "ssh.key";
const URL_PREFIX: &str = "url:";
const FIELD_PREFIX: &str = "field:";
const ATTACHMENT_PREFIX: &str = "attachment:";
const TAG_PREFIX: &str = "tag:";
const HISTORY_PREFIX: &str = "history:";
const CONFLICT_COPY_OF: &str = "conflict_copy_of";

/// Counters: the merged value is the largest one, never a conflict.
const MAX_KEYS: [&str; 2] = [OTP_COUNTER, PASSKEY_SIGN_COUNT];

/// Item metadata: concurrent values are resolved by the largest stamp only
/// and never shown as a conflict ([`Item::conflicts`]). The same holds for
/// `history:*` entries, `deleted` and the counters.
const METADATA_KEYS: [&str; 5] = [TYPE, CREATED, TRASHED, FAVORITE, CONFLICT_COPY_OF];

/// Whether concurrent values of `key` are a conflict the user resolves.
fn is_user_content(key: &str) -> bool {
    key != DELETED_KEY
        && !MAX_KEYS.contains(&key)
        && !METADATA_KEYS.contains(&key)
        && !key.starts_with(HISTORY_PREFIX)
}

/// The state of a deletion: when and by which devices.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeletionInfo {
    /// The latest deletion time.
    pub deleted_at: Timestamp,
    /// The devices that deleted the item (more than one after concurrent
    /// deletes).
    pub deleted_by: Vec<DeviceId>,
}

pub(crate) fn deletion_info(doc: &Doc) -> Option<DeletionInfo> {
    let reg = doc.register(DELETED_KEY)?;
    let deleted_at = reg.entries().map(Entry::at).max()?;
    let mut deleted_by: Vec<DeviceId> = reg.entries().map(|e| e.stamp().device()).collect();
    deleted_by.sort_unstable();
    deleted_by.dedup();
    Some(DeletionInfo {
        deleted_at,
        deleted_by,
    })
}

/// The result of [`Item::merge_and_repair`].
#[derive(Debug)]
pub struct MergeOutcome {
    /// The merged and repaired item. Persist and upload this one.
    pub item: Item,
    /// How many fields were written back after a concurrent delete.
    pub restored_fields: usize,
    /// Both replicas held one edit stamp with different contents. Two
    /// writers share a device ID (or a record is corrupted): both values
    /// are kept as a conflict, and the store warns and looks for the
    /// writer that reuses an ID.
    pub stamp_collision: bool,
}

/// What [`Item::split_oversized`] returns.
#[derive(Debug)]
pub struct OversizedSplit {
    /// The record to store (the same on every device that reaches the same
    /// merge result).
    pub item: Item,
    /// The values that moved out of the record, each as a new item with its
    /// derived ID ([`Item::conflict_copy_id`]). Store each one unless a
    /// record with that ID is already held (another device made the same
    /// copy, which is identical).
    pub conflict_copies: Vec<(ItemId, Item)>,
}

/// A vault item: the replicated state that is encrypted as one record.
///
/// Not `Clone`; `Debug` shows only the version vector and the field count.
/// Read fields through the typed getters, change them through
/// [`Item::edit`], and combine replicas with [`Item::merge`].
#[derive(PartialEq, Eq)]
pub struct Item {
    pub(crate) doc: Doc,
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item")
            .field("version", self.doc.vv())
            .field("fields", &self.doc.keys().count())
            .finish_non_exhaustive()
    }
}

impl Item {
    /// A new item of `item_type`, created by `device` at `at`.
    ///
    /// # Errors
    /// None in practice: the first stamp of a new item cannot overflow. The
    /// signature is fallible like every other edit.
    pub fn new(item_type: &ItemType, device: DeviceId, at: Timestamp) -> Result<Self> {
        let mut item = Self {
            doc: Doc::default(),
        };
        let mut e = item.edit(device, at);
        e.put(TYPE, Value::text(item_type.as_str()), false)?;
        e.put(CREATED, Value::int(at.as_unix_millis()), false)?;
        Ok(item)
    }

    /// Starts an edit by `device` at wall-clock time `at`. All changes made
    /// through one [`ItemEdit`] form one sync event with one Lamport stamp;
    /// an edit that changes nothing leaves the item untouched.
    pub fn edit(&mut self, device: DeviceId, at: Timestamp) -> ItemEdit<'_> {
        ItemEdit {
            doc: &mut self.doc,
            device,
            at,
            stamp: None,
            seq: 0,
        }
    }

    /// Merges two replicas of the same item. Pure, commutative, associative
    /// and idempotent; never loses a value that no device superseded.
    ///
    /// This is the lattice join, for tests, simulations and comparisons. A
    /// store that merges a received record into its local state and keeps
    /// the result uses [`Item::merge_and_repair`] instead.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            doc: self.doc.merge(&other.doc),
        }
    }

    /// Merges a `received` record into this local replica and repairs what
    /// a pure merge cannot (CRYPTO_SPEC §10). Every store persists the
    /// returned item instead of the plain [`Item::merge`].
    ///
    /// `self` is this device's own latest state of the item and `device`
    /// is this device's ID. On top of the merge, as local edits by
    /// `device` with fresh stamps (so the result syncs out like any edit):
    ///
    /// - **Concurrent delete.** If the merge is flagged
    ///   ([`Item::deleted_concurrently`]) by a deletion this replica had not
    ///   seen, every value this replica held that the deletion took away is
    ///   written back, conflict copies included, also in a field that
    ///   survives through a concurrent local write (the value this replica
    ///   showed stays shown). Fields the received side holds are left to
    ///   the merge: a value it dropped there may have been superseded. A
    ///   device that edited (or merely held) the item while another device
    ///   deleted it therefore keeps every field it had; the flag stays
    ///   until the user dismisses it or deletes again. This can also bring
    ///   back a field that a third device removed in the same concurrent
    ///   window; the flag puts the item in front of the user either way.
    /// - **Same stamp, different contents** (two writers with one device
    ///   ID): the value the merge dropped is written back, so both show up
    ///   as a conflict, and [`MergeOutcome::stamp_collision`] is set.
    /// - **Equal version vectors, different contents**: honest replicas
    ///   with equal vectors are equal, so a stamp names two edits (two
    ///   writers with one device ID that edited different fields; the pure
    ///   merge drops both sides' values). Every value either side held
    ///   that the merge dropped is written back, and
    ///   [`MergeOutcome::stamp_collision`] is set (ADR-0015).
    ///
    /// Restored values keep their original write times, and the value
    /// written last by the wall clock stays the one shown. Without a repair
    /// the result equals `self.merge(received)`.
    ///
    /// # Errors
    /// [`Error::DeviceIdReused`] if `received` holds edits by `device`
    /// that `self` has not seen (another process or install shares the
    /// ID, or local state came from a backup): a counter for `device`
    /// above this replica's, or one of `device`'s stamps with different
    /// contents. Rotate the device ID and call
    /// [`Item::merge_and_repair_after_reuse`] with the new one and the old
    /// one as reused; the collision is then kept as a conflict
    /// ([`MergeOutcome::stamp_collision`]). [`Error::ClockOverflow`].
    pub fn merge_and_repair(&self, received: &Self, device: DeviceId) -> Result<MergeOutcome> {
        self.merge_and_repair_after_reuse(received, device, &[])
    }

    /// [`Item::merge_and_repair`] after this replica rotated away from the
    /// device IDs in `reused` because another writer shares them
    /// ([`Error::DeviceIdReused`]). For an ID of `reused` that collides in
    /// this record (`received` has a counter for it above this replica's,
    /// or both hold one of its stamps in a field with other contents; an
    /// entry only one side holds may just be superseded), neither writer's
    /// edits under it are lost: this replica's values under it that `received`
    /// does not hold are written back under `device`, and `received`'s are
    /// kept although this replica's vector covers their counters. A value
    /// that one of the writers had superseded can come back as a conflict
    /// copy; [`MergeOutcome::stamp_collision`] is set (ADR-0015). IDs that
    /// do not collide in this record change nothing.
    ///
    /// # Errors
    /// As [`Item::merge_and_repair`].
    pub fn merge_and_repair_after_reuse(
        &self,
        received: &Self,
        device: DeviceId,
        reused: &[DeviceId],
    ) -> Result<MergeOutcome> {
        if received.doc.vv().get(&device) > self.doc.vv().get(&device) {
            return Err(Error::DeviceIdReused);
        }
        let (mut doc, repair) = self.doc.merge_and_repair(&received.doc, device, reused)?;
        if repair.own_stamp_collision {
            // A different edit under one of this device's own stamps: the
            // ID is shared, even though the counters are equal (two writers
            // that each made one edit from the same state).
            return Err(Error::DeviceIdReused);
        }
        if repair.stamp_collision {
            // Two writers under one ID can write the same history key (it
            // names the edit's stamp) with different passwords. The history
            // shows one value per key, so a value the repair wrote back
            // there would be invisible; it gets a key of its own (review
            // M2, fourth round).
            let crowded: Vec<String> = doc
                .registers()
                .filter(|(k, r)| k.starts_with(HISTORY_PREFIX) && r.len() > 1)
                .map(|(k, _)| k.to_owned())
                .collect();
            for key in crowded {
                doc.spread_losers(&key, device, |s| {
                    format!("{HISTORY_PREFIX}{:016x}.{}.0", s.counter(), s.device())
                })?;
            }
        }
        Ok(MergeOutcome {
            item: Self { doc },
            restored_fields: repair.restored_fields,
            stamp_collision: repair.stamp_collision,
        })
    }

    /// Resolves a merge of this local replica and `received` whose result
    /// does not fit the item size limit (`merge_and_repair_after_reuse`
    /// with the same `device` and `reused` succeeded, but the result fails
    /// to encode with [`Error::SizeLimit`]), so that the merge never fails,
    /// no value is lost and every device converges (ADR-0016, CRYPTO_SPEC
    /// §10).
    ///
    /// The record ([`OversizedSplit::item`]) is the merge result reduced as
    /// a pure function of that result, so every device that reaches the same
    /// state reduces it the same way, whatever pair of versions it merged
    /// and in which order: every content field keeps only the value it
    /// shows (its winner), and its other values (the conflict copies of the
    /// field) move out. If the record still does not fit, whole content
    /// fields move out too, largest value first. Metadata, the password
    /// history, counters, attachment references (their files are bound to
    /// the record ID) and the deletion entries always stay, so a concurrent
    /// delete keeps its notice. The vector is the merge's, so the record
    /// dominates both inputs.
    ///
    /// Each value that moved out becomes a conflict copy
    /// ([`OversizedSplit::conflict_copies`]): a new item of the same vault
    /// that holds the value under its field key with its original stamp,
    /// and [`Item::conflict_copy_of`] under the same stamp (left out if the
    /// copy would not fit otherwise); its vector covers exactly that stamp.
    /// It has no type of its own: show it with the original's type and
    /// title, through the mark. Its ID is [`Item::conflict_copy_id`]. The
    /// copy is a function of the value alone, so every device that splits
    /// it off builds the same copy under the same ID, and copies from
    /// different devices are one record.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the record does not fit even with every
    /// content field moved out (its history or attachment references alone
    /// exceed the limit), or a value does not fit a copy of its own;
    /// [`Error::DeviceIdReused`], [`Error::ClockOverflow`],
    /// [`Error::Allocation`].
    pub fn split_oversized(
        &self,
        received: &Self,
        item_id: &ItemId,
        vault_key: &VaultKey,
        device: DeviceId,
        reused: &[DeviceId],
    ) -> Result<OversizedSplit> {
        let merged = self
            .merge_and_repair_after_reuse(received, device, reused)?
            .item;
        if fits(&merged)? {
            // Not oversized after all: nothing to split.
            return Ok(OversizedSplit {
                item: merged,
                conflict_copies: Vec::new(),
            });
        }
        let movable = |k: &str| is_user_content(k) && !k.starts_with(ATTACHMENT_PREFIX);
        let (mut doc, mut pieces) = merged.doc.overflow_reduce(movable);
        loop {
            if fits(&Self { doc: doc.dup() })? {
                break;
            }
            match doc.take_largest(movable)? {
                Some(piece) => pieces.push(piece),
                None => return Err(Error::SizeLimit),
            }
        }
        let item = Self { doc };
        let mut conflict_copies = Vec::with_capacity(pieces.len());
        for (key, entry) in &pieces {
            let id = Self::conflict_copy_id(vault_key, item_id, key.as_str(), entry.stamp())?;
            let mark = Value::text(&item_id.to_string());
            let marked = Self {
                doc: Doc::conflict_copy(key, entry, Some((CONFLICT_COPY_OF, mark))),
            };
            let copy = if fits(&marked)? {
                Some(marked)
            } else {
                let bare = Self {
                    doc: Doc::conflict_copy(key, entry, None),
                };
                fits(&bare)?.then_some(bare)
            };
            conflict_copies.push((id, copy.ok_or(Error::SizeLimit)?));
        }
        Ok(OversizedSplit {
            item,
            conflict_copies,
        })
    }

    /// The ID of the conflict copy that holds the value `stamp` wrote under
    /// `key` in record `item_id` after a split (CRYPTO_SPEC §10, ADR-0016):
    /// a `UUIDv8` from `HKDF(VK, "", ctx("conflict-copy-id") || item_id ||
    /// u32(len(key)) || key || counter:u64 || device_id, 16)`. Every device
    /// holding `vault_key` derives the same ID for the same value, and the
    /// server cannot link the copy to the record.
    ///
    /// # Errors
    /// [`Error::InvalidId`] for a nil result (it never is: the version
    /// nibble is 8); [`Error::SizeLimit`] for a key above 4 GiB.
    pub fn conflict_copy_id(
        vault_key: &VaultKey,
        item_id: &ItemId,
        key: &str,
        stamp: Stamp,
    ) -> Result<ItemId> {
        let bytes = crate::kdf::conflict_copy_id(
            vault_key.key.expose_secret(),
            item_id.as_bytes(),
            key.as_bytes(),
            stamp.counter(),
            stamp.device().as_bytes(),
        )?;
        ItemId::from_bytes(bytes)
    }

    /// Brings a deleted item (a tombstone) back as a live item of
    /// `item_type`, created at `at`. A tombstone keeps no fields, so this
    /// is the only edit it accepts; every other edit of a deleted item
    /// fails with [`Error::ItemDeleted`]. A no-op on a live item.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn undelete(
        &mut self,
        item_type: &ItemType,
        device: DeviceId,
        at: Timestamp,
    ) -> Result<()> {
        if !self.doc.is_deleted() {
            return Ok(());
        }
        let s = self.doc.tick(device)?;
        self.doc.remove_key(DELETED_KEY);
        self.doc
            .write(TYPE, s, at, Value::text(item_type.as_str()), false)?;
        self.doc
            .write(CREATED, s, at, Value::int(at.as_unix_millis()), false)
    }

    /// The authenticated version vector.
    #[must_use]
    pub const fn version(&self) -> &VersionVector {
        self.doc.vv()
    }

    /// How this replica's version vector relates to `other`'s (CRYPTO_SPEC
    /// §10). For `stored.compare(&received)`: [`Causality::Dominates`] is a
    /// rollback or replay and the received record is ignored,
    /// [`Causality::Equal`] brings nothing new, and
    /// [`Causality::DominatedBy`] or [`Causality::Concurrent`] is merged.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Causality {
        self.doc.vv().compare(other.doc.vv())
    }

    /// The item type. `None` only on a deleted item, or on the deleting
    /// device after a delete lost against a concurrent edit, until the
    /// repair of a device that still held the item arrives (see
    /// [`Item::merge_and_repair`]). Show such an item as "unknown type"
    /// together with the [`Item::deleted_concurrently`] notice.
    #[must_use]
    pub fn item_type(&self) -> Option<ItemType> {
        self.doc
            .value(TYPE)
            .and_then(Value::as_str)
            .map(ItemType::parse)
    }

    /// When the item was created.
    #[must_use]
    pub fn created_at(&self) -> Option<Timestamp> {
        self.int(CREATED).map(Timestamp::from_unix_millis)
    }

    /// The latest wall-clock time any field was written.
    #[must_use]
    pub fn modified_at(&self) -> Option<Timestamp> {
        self.doc.modified_at()
    }

    /// The value of a text field.
    #[must_use]
    pub fn text(&self, field: TextField) -> Option<&str> {
        self.doc.value(field.key()).and_then(Value::as_str)
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.text(TextField::Title)
    }

    /// A copy of a secret field.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the copy cannot be reserved.
    pub fn secret(&self, field: SecretField) -> Result<Option<SecretString>> {
        match self.doc.value(field.key()) {
            Some(Value::String(t)) => Ok(Some(t.to_secret()?)),
            _ => Ok(None),
        }
    }

    /// Whether the item is a favorite.
    #[must_use]
    pub fn favorite(&self) -> bool {
        self.doc
            .value(FAVORITE)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// When the item was moved to the trash, if it is in the trash.
    #[must_use]
    pub fn trashed_at(&self) -> Option<Timestamp> {
        self.int(TRASHED).map(Timestamp::from_unix_millis)
    }

    /// The record this item was split off from because a merge with it did
    /// not fit the item size limit ([`Item::split_oversized`], ADR-0016).
    /// Show it to the user next to that item, then
    /// [`ItemEdit::dismiss_conflict_copy`] or delete the copy.
    #[must_use]
    pub fn conflict_copy_of(&self) -> Option<ItemId> {
        let s = self.doc.value(CONFLICT_COPY_OF)?.as_str()?;
        ItemId::from_bytes(parse_uuid(s).ok()?).ok()
    }

    /// The tags, sorted.
    #[must_use]
    pub fn tags(&self) -> Vec<&str> {
        self.doc
            .keys()
            .filter_map(|k| k.strip_prefix(TAG_PREFIX))
            .collect()
    }

    /// The login URLs in creation order.
    #[must_use]
    pub fn urls(&self) -> Vec<(ElementId, LoginUrl)> {
        self.elements(URL_PREFIX, LoginUrl::from_value)
    }

    /// The custom fields in creation order.
    #[must_use]
    pub fn custom_fields(&self) -> Vec<(ElementId, CustomField)> {
        self.elements(FIELD_PREFIX, CustomField::from_value)
    }

    /// The attachment references.
    #[must_use]
    pub fn attachments(&self) -> Vec<AttachmentRef> {
        self.elements(ATTACHMENT_PREFIX, AttachmentRef::from_value)
            .into_iter()
            .map(|(_, a)| a)
            .collect()
    }

    /// The password history, newest first.
    #[must_use]
    pub fn password_history(&self) -> Vec<HistoryEntry> {
        let mut h: Vec<HistoryEntry> = self
            .elements(HISTORY_PREFIX, HistoryEntry::from_value)
            .into_iter()
            .map(|(_, e)| e)
            .collect();
        h.sort_by_key(|e| core::cmp::Reverse(e.changed_at));
        h
    }

    /// The OTP configuration of a login or TOTP item.
    #[must_use]
    pub fn otp(&self) -> Option<OtpConfig> {
        self.doc
            .value(OTP)
            .and_then(|v| OtpConfig::from_value(v).ok())
    }

    /// The HOTP counter (the largest value any device wrote).
    #[must_use]
    pub fn otp_counter(&self) -> u64 {
        self.max_counter(OTP_COUNTER)
    }

    /// The passkey of a passkey item.
    #[must_use]
    pub fn passkey(&self) -> Option<PasskeyCredential> {
        self.doc
            .value(PASSKEY)
            .and_then(|v| PasskeyCredential::from_value(v).ok())
    }

    /// The passkey signature counter (the largest value any device wrote).
    #[must_use]
    pub fn passkey_sign_count(&self) -> u64 {
        self.max_counter(PASSKEY_SIGN_COUNT)
    }

    /// The SSH key material of an SSH key item.
    #[must_use]
    pub fn ssh_key(&self) -> Option<SshKeyMaterial> {
        self.doc
            .value(SSH_KEY)
            .and_then(|v| SshKeyMaterial::from_value(v).ok())
    }

    /// Content fields where concurrent edits left more than one distinct
    /// value. The UI shows the winner as the value and offers the losers;
    /// resolve with [`ItemEdit::resolve_conflict`] or
    /// [`ItemEdit::dismiss_conflict`].
    ///
    /// Metadata is never listed: `type`, `created_at`, `trashed_at`,
    /// `favorite` and `history:*` take the value with the largest stamp,
    /// the counters take the largest value, and a concurrent delete is
    /// [`Item::deleted_concurrently`].
    #[must_use]
    pub fn conflicts(&self) -> Vec<FieldConflict<'_>> {
        self.doc.conflicts(is_user_content)
    }

    /// Whether this is a deleted item (a tombstone).
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.doc.is_deleted()
    }

    /// A delete that lost against a concurrent edit: the item is live, but
    /// a device deleted it without seeing that edit. The pure merge keeps
    /// only the concurrent edits; [`Item::merge_and_repair`] on every
    /// device that had not seen the deletion writes back the fields it
    /// held. Show it to the user, then [`ItemEdit::dismiss_deletion`] or
    /// delete again.
    #[must_use]
    pub fn deleted_concurrently(&self) -> Option<DeletionInfo> {
        if self.doc.is_deleted() {
            None
        } else {
            deletion_info(&self.doc)
        }
    }

    /// Deletes the item (one edit). Seal the result with
    /// [`Item::to_tombstone`].
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn delete(&mut self, device: DeviceId, at: Timestamp) -> Result<()> {
        self.doc.delete(device, at)
    }

    /// The tombstone of a deleted item.
    #[must_use]
    pub fn to_tombstone(&self) -> Option<Tombstone> {
        self.doc.is_deleted().then(|| Tombstone {
            doc: self.doc.dup(),
        })
    }

    /// Field keys whose value this build cannot read: keys from a newer
    /// client, or known keys with an unexpected value. They are preserved
    /// and merged; only their display is unsupported.
    #[must_use]
    pub fn unreadable_fields(&self) -> Vec<&str> {
        self.doc
            .registers()
            .filter(|(k, r)| r.winner().is_none_or(|w| !readable(k, w.value())))
            .map(|(k, _)| k)
            .collect()
    }

    /// Every field key, in key order (raw access).
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.doc.keys()
    }

    /// Every live value of `key`, in stamp order (raw access).
    pub fn entries(&self, key: &str) -> impl Iterator<Item = &Entry> {
        self.doc
            .register(key)
            .into_iter()
            .flat_map(super::doc::Register::entries)
    }

    /// The winning value of `key` (raw access).
    #[must_use]
    pub fn value(&self, key: &str) -> Option<&Value> {
        self.doc.value(key)
    }

    /// The canonical plaintext (CRYPTO_SPEC §7.2) in a zeroizing buffer.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_ITEM_PLAINTEXT_LEN`],
    /// [`Error::Allocation`].
    pub fn to_json(&self) -> Result<SecretVec> {
        to_json(
            &DocJson {
                doc: &self.doc,
                format: Format::Document,
            },
            MAX_ITEM_PLAINTEXT_LEN,
        )
    }

    /// Strictly decodes item plaintext. Unknown fields and members are kept.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::UnsupportedVersion`] for a newer
    /// `schema_version`, [`Error::Malformed`] otherwise.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ITEM_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Ok(Self {
            doc: decode(bytes, Format::Document)?,
        })
    }

    /// Encrypts the item as record `item_id` of `vault_id` (CRYPTO_SPEC
    /// §7.2) with `schema_version` 1.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::Allocation`], [`Error::Randomness`].
    pub fn seal(
        &self,
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
    ) -> Result<ItemCiphertext> {
        let pt = self.to_json()?;
        ItemCiphertext::seal(vk, vault_id, item_id, SCHEMA_VERSION, pt.expose_secret())
    }

    /// Decrypts and decodes record `item_id` of `vault_id`.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`] for a header `schema_version` other
    /// than 1 (checked before decryption), [`Error::KeyGenerationMismatch`],
    /// [`Error::ItemCorrupted`] for every authentication or padding failure,
    /// and the errors of [`Item::from_json`].
    pub fn open(
        ct: &ItemCiphertext,
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
    ) -> Result<Self> {
        if ct.schema_version() != SCHEMA_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        let pt = ct.open(vk, vault_id, item_id).map_err(|e| match e {
            Error::Decryption => Error::ItemCorrupted { item_id: *item_id },
            other => other,
        })?;
        Self::from_json(pt.expose_secret())
    }

    fn int(&self, key: &str) -> Option<i64> {
        self.doc.value(key).and_then(Value::as_i64)
    }

    fn max_counter(&self, key: &str) -> u64 {
        self.entries(key)
            .filter_map(|e| e.value().as_u64())
            .max()
            .unwrap_or(0)
    }

    fn elements<T>(&self, prefix: &str, decode: fn(&Value) -> Result<T>) -> Vec<(ElementId, T)> {
        self.doc
            .registers()
            .filter_map(|(k, r)| {
                let id = k.strip_prefix(prefix)?;
                let v = decode(r.winner()?.value()).ok()?;
                Some((ElementId(id.to_owned()), v))
            })
            .collect()
    }
}

impl From<Tombstone> for Item {
    fn from(t: Tombstone) -> Self {
        Self { doc: t.doc }
    }
}

/// Whether this build can display the value of `key`.
fn readable(key: &str, v: &Value) -> bool {
    if TextField::from_key(key).is_some() || SecretField::from_key(key).is_some() || key == TYPE {
        return v.as_str().is_some();
    }
    if key.starts_with(TAG_PREFIX) {
        return true;
    }
    match key {
        CREATED | TRASHED => v.as_i64().is_some(),
        CONFLICT_COPY_OF => v
            .as_str()
            .is_some_and(|s| parse_uuid(s).is_ok_and(|b| ItemId::from_bytes(b).is_ok())),
        FAVORITE => v.as_bool().is_some(),
        OTP_COUNTER | PASSKEY_SIGN_COUNT => v.as_u64().is_some(),
        OTP => OtpConfig::from_value(v).is_ok(),
        PASSKEY => PasskeyCredential::from_value(v).is_ok(),
        SSH_KEY => SshKeyMaterial::from_value(v).is_ok(),
        DELETED_KEY => true,
        _ if key.starts_with(URL_PREFIX) => LoginUrl::from_value(v).is_ok(),
        _ if key.starts_with(FIELD_PREFIX) => CustomField::from_value(v).is_ok(),
        _ if key.starts_with(ATTACHMENT_PREFIX) => AttachmentRef::from_value(v).is_ok(),
        _ if key.starts_with(HISTORY_PREFIX) => HistoryEntry::from_value(v).is_ok(),
        _ => false,
    }
}

/// Whether `item` fits the item size limit.
fn fits(item: &Item) -> Result<bool> {
    match item.to_json() {
        Ok(_) => Ok(true),
        Err(Error::SizeLimit) => Ok(false),
        Err(e) => Err(e),
    }
}

/// The stamp of the edit that wrote a history key
/// (`history:{counter:016x}.{device}.{seq}`, see `ItemEdit::next_id`), or
/// `None` if the key does not have that form.
fn history_stamp(key: &str) -> Option<Stamp> {
    let mut parts = key.strip_prefix(HISTORY_PREFIX)?.split('.');
    let counter = parts.next().filter(|c| c.len() == 16)?;
    let counter = u64::from_str_radix(counter, 16).ok()?;
    let device = parts.next()?;
    parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let hex: Vec<u8> = device.bytes().filter(|&b| b != b'-').collect();
    if hex.len() != 32 || device.len() != 36 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (out, pair) in bytes.iter_mut().zip(hex.as_chunks::<2>().0) {
        let pair = core::str::from_utf8(pair).ok()?;
        *out = u8::from_str_radix(pair, 16).ok()?;
    }
    Stamp::new(counter, DeviceId::from_bytes(bytes).ok()?).ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::*;

    /// The history key parser behind the trim order (review M2, fourth
    /// round): exactly the form `ItemEdit::next_id` writes, anything else
    /// is `None` (the trim then falls back to the winning stamp).
    #[test]
    fn history_stamps_parse_only_the_written_form() {
        let d = DeviceId::from_bytes([0xAB; 16]).unwrap();
        let good = format!("history:{:016x}.{d}.3", 0x2a_u64);
        assert_eq!(history_stamp(&good), Some(Stamp::new(0x2a, d).unwrap()));
        for bad in [
            String::from("history:"),
            String::from("history:weird"),
            format!("history:{:x}.{d}.0", 0x2a_u64),
            format!("history:{:016x}.{d}", 0x2a_u64),
            format!("history:{:016x}.{d}.x", 0x2a_u64),
            format!("history:{:016x}.{d}.0.1", 0x2a_u64),
            format!("history:{:016x}.abc.0", 0x2a_u64),
            format!("history:{:016x}.{}.0", 0x2a_u64, "g".repeat(36)),
            format!(
                "history:{:016x}.{}.0",
                0x2a_u64, "00000000-0000-0000-0000-000000000000"
            ),
            format!("history:{:016x}.{d}.0", 0_u64),
            format!("tag:{:016x}.{d}.0", 0x2a_u64),
        ] {
            assert_eq!(history_stamp(&bad), None, "{bad}");
        }
    }

    /// `ItemEdit::otp` shows the configuration set earlier in the same edit.
    #[test]
    fn an_edit_sees_its_own_otp_configuration() {
        let d = DeviceId::from_bytes([0xAB; 16]).unwrap();
        let at = Timestamp::from_unix_millis(1);
        let mut item = Item::new(&ItemType::Totp, d, at).unwrap();
        let mut e = item.edit(d, at);
        assert!(e.otp().is_none());
        let mut config = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
        e.set_otp(&config).unwrap();
        assert!(!e.otp().unwrap().hardware_protected);
        config.hardware_protected = true;
        e.set_otp(&config).unwrap();
        assert!(e.otp().unwrap().hardware_protected);
    }
}
