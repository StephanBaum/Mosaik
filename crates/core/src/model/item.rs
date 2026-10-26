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
//!
//! Collections are one key per element, so concurrent edits of different
//! URLs, tags or custom fields never conflict. Keys this build does not
//! know are kept and merged like any other field.

use core::fmt;

use super::clock::{Causality, Stamp, Timestamp, VersionVector};
use super::doc::{DELETED_KEY, Doc, DocJson, Entry, FieldConflict, Format, SCHEMA_VERSION, decode};
use super::tombstone::Tombstone;
use super::types::{
    AttachmentRef, CustomField, LoginUrl, OtpConfig, PasskeyCredential, SshKeyMaterial,
};
use super::value::{Object, Text, Value, to_json};
use crate::envelope::{ItemCiphertext, MAX_ITEM_PLAINTEXT_LEN};
use crate::error::{Error, Result};
use crate::ids::{AttachmentId, DeviceId, ItemId, VaultId};
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

/// Counters: the merged value is the largest one, never a conflict.
const MAX_KEYS: [&str; 2] = [OTP_COUNTER, PASSKEY_SIGN_COUNT];

/// The kind of an item.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ItemType {
    /// Username, password, URLs, optional OTP.
    Login,
    /// A WebAuthn credential.
    Passkey,
    /// An OTP generator on its own.
    Totp,
    /// Notes only.
    SecureNote,
    /// A payment card.
    CreditCard,
    /// Personal details.
    Identity,
    /// An SSH key pair.
    SshKey,
    /// An API key or token.
    ApiKey,
    /// Wi-Fi credentials.
    Wifi,
    /// Files (attachments) with notes.
    Document,
    /// A type written by a newer client. Shown as unsupported; its fields
    /// are kept and merged as usual.
    Other(String),
}

impl ItemType {
    /// The identifier stored in the `type` field.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Login => "login",
            Self::Passkey => "passkey",
            Self::Totp => "totp",
            Self::SecureNote => "note",
            Self::CreditCard => "card",
            Self::Identity => "identity",
            Self::SshKey => "ssh_key",
            Self::ApiKey => "api_key",
            Self::Wifi => "wifi",
            Self::Document => "document",
            Self::Other(s) => s,
        }
    }

    /// Parses a stored identifier; unknown ones become [`ItemType::Other`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "login" => Self::Login,
            "passkey" => Self::Passkey,
            "totp" => Self::Totp,
            "note" => Self::SecureNote,
            "card" => Self::CreditCard,
            "identity" => Self::Identity,
            "ssh_key" => Self::SshKey,
            "api_key" => Self::ApiKey,
            "wifi" => Self::Wifi,
            "document" => Self::Document,
            other => Self::Other(other.to_owned()),
        }
    }
}

macro_rules! field_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $key:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum $name {
            $($(#[$vmeta])* $variant),*
        }

        impl $name {
            /// Every field, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The document key of this field.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key),*
                }
            }

            /// The field stored under `key`, if any.
            #[must_use]
            pub fn from_key(key: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|f| f.key() == key)
            }
        }
    };
}

field_enum!(
    /// Plain text fields.
    TextField {
        /// The item's title.
        Title => "title",
        /// Free-form notes (the whole content of a secure note).
        Notes => "notes",
        /// Login user name or email.
        Username => "login.username",
        /// Name on a card.
        CardholderName => "card.cardholder",
        /// Card network or brand.
        CardBrand => "card.brand",
        /// Card expiry, `MM/YYYY`.
        CardExpiry => "card.expiry",
        /// Identity: given name.
        FirstName => "identity.first_name",
        /// Identity: middle name.
        MiddleName => "identity.middle_name",
        /// Identity: family name.
        LastName => "identity.last_name",
        /// Identity: email.
        Email => "identity.email",
        /// Identity: phone.
        Phone => "identity.phone",
        /// Identity: company.
        Company => "identity.company",
        /// Identity: address line 1.
        AddressLine1 => "identity.address1",
        /// Identity: address line 2.
        AddressLine2 => "identity.address2",
        /// Identity: city.
        City => "identity.city",
        /// Identity: state or region.
        Region => "identity.region",
        /// Identity: postal code.
        PostalCode => "identity.postal_code",
        /// Identity: country.
        Country => "identity.country",
        /// Identity: date of birth, `YYYY-MM-DD`.
        BirthDate => "identity.birth_date",
        /// API key: key ID or client ID.
        ApiKeyId => "api_key.key_id",
        /// API key: host or endpoint.
        ApiKeyHost => "api_key.host",
        /// Wi-Fi network name.
        WifiSsid => "wifi.ssid",
        /// Wi-Fi security type (for example `WPA3`).
        WifiSecurity => "wifi.security",
        /// SSH key comment.
        SshComment => "ssh.comment",
    }
);

field_enum!(
    /// Secret text fields. They are read into a [`SecretString`].
    SecretField {
        /// Login password.
        Password => "login.password",
        /// Card number.
        CardNumber => "card.number",
        /// Card verification code.
        CardVerificationCode => "card.cvv",
        /// Card PIN.
        CardPin => "card.pin",
        /// Identity: passport number.
        PassportNumber => "identity.passport_number",
        /// Identity: driver's license number.
        LicenseNumber => "identity.license_number",
        /// Identity: national ID or social security number.
        NationalId => "identity.national_id",
        /// API key: the secret or token.
        ApiKeySecret => "api_key.secret",
        /// Wi-Fi password.
        WifiPassword => "wifi.password",
    }
);

impl SecretField {
    /// Whether old values go into the password history when the value
    /// changes or a conflict is resolved.
    #[must_use]
    pub const fn keeps_history(self) -> bool {
        matches!(
            self,
            Self::Password | Self::CardPin | Self::ApiKeySecret | Self::WifiPassword
        )
    }
}

/// The ID of one element of a collection (URL, custom field). Derived from
/// the creating edit's stamp, so it is unique without randomness.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ElementId(String);

impl ElementId {
    /// The ID as stored after the key prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps a stored ID.
    #[must_use]
    pub fn from_stored(id: &str) -> Self {
        Self(id.to_owned())
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One password history entry.
pub struct HistoryEntry {
    /// The field the value belonged to.
    pub field: SecretField,
    /// The old value.
    pub value: SecretString,
    /// When it was replaced.
    pub changed_at: Timestamp,
}

impl fmt::Debug for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HistoryEntry")
            .field("field", &self.field)
            .field("changed_at", &self.changed_at)
            .finish_non_exhaustive()
    }
}

impl HistoryEntry {
    fn from_value(v: &Value) -> Result<Self> {
        let o = v.as_object().ok_or(Error::Malformed)?;
        let field = o
            .get("field")
            .and_then(Value::as_str)
            .and_then(SecretField::from_key)
            .ok_or(Error::Malformed)?;
        let value = match o.get("value") {
            Some(Value::String(t)) => t.to_secret()?,
            _ => return Err(Error::Malformed),
        };
        let changed_at = o
            .get("changed_at")
            .and_then(Value::as_i64)
            .ok_or(Error::Malformed)?;
        Ok(Self {
            field,
            value,
            changed_at: Timestamp::from_unix_millis(changed_at),
        })
    }
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
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            doc: self.doc.merge(&other.doc),
        }
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

    /// The item type; `None` only if a concurrent delete removed it (see
    /// [`Item::deleted_concurrently`]).
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

    /// Fields where concurrent edits left more than one distinct value.
    /// The UI shows the winner as the value and offers the losers; resolve
    /// with [`ItemEdit::resolve_conflict`] or [`ItemEdit::dismiss_conflict`].
    #[must_use]
    pub fn conflicts(&self) -> Vec<FieldConflict<'_>> {
        self.doc
            .conflicts(|k| k != DELETED_KEY && !MAX_KEYS.contains(&k))
    }

    /// Whether this is a deleted item (a tombstone).
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.doc.is_deleted()
    }

    /// A delete that lost against a concurrent edit: the item is live, but
    /// a device deleted it without seeing that edit. Fields the deleting
    /// device had seen are gone; the concurrent edits survive. Show it to
    /// the user, then [`ItemEdit::dismiss_deletion`] or delete again.
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
                Some((ElementId::from_stored(id), v))
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

fn check_tag(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_TAG_LEN || name.chars().any(char::is_control) {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// The key of tag `name`, built in a zeroizing buffer (tag names are item
/// content; `format!` would leave an unwiped copy).
fn tag_key(name: &str) -> Text {
    let mut s = String::with_capacity(TAG_PREFIX.len().saturating_add(name.len()));
    s.push_str(TAG_PREFIX);
    s.push_str(name);
    Text::from_string(s)
}

/// An edit of one item by one device. See [`Item::edit`].
pub struct ItemEdit<'a> {
    doc: &'a mut Doc,
    device: DeviceId,
    at: Timestamp,
    stamp: Option<Stamp>,
    seq: u32,
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
    /// The stamp of this edit, ticked on the first change. Changing a
    /// deleted item restores it.
    fn stamp(&mut self) -> Result<Stamp> {
        if let Some(s) = self.stamp {
            return Ok(s);
        }
        let s = self.doc.tick(self.device)?;
        if self.doc.is_deleted() {
            self.doc.remove_key(DELETED_KEY);
        }
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
    fn put(&mut self, key: &str, value: Value, keep_losers: bool) -> Result<()> {
        if let Some(reg) = self.doc.register(key)
            && let Some(w) = reg.winner()
            && *w.value() == value
            && (keep_losers || reg.len() == 1)
        {
            return Ok(());
        }
        let s = self.stamp()?;
        self.doc.write(key, s, self.at, value, keep_losers);
        Ok(())
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
        let mut keys: Vec<(Timestamp, String)> = self
            .doc
            .registers()
            .filter(|(k, _)| k.starts_with(HISTORY_PREFIX))
            .map(|(k, r)| {
                let at = r
                    .winner()
                    .and_then(|w| w.value().as_object())
                    .and_then(|o| o.get("changed_at"))
                    .and_then(Value::as_i64)
                    .map_or(
                        Timestamp::from_unix_millis(i64::MIN),
                        Timestamp::from_unix_millis,
                    );
                (at, k.to_owned())
            })
            .collect();
        if keys.len() > MAX_HISTORY {
            keys.sort();
            let excess = keys.len() - MAX_HISTORY;
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
    /// [`Error::ClockOverflow`].
    pub fn set_url(&mut self, id: &ElementId, url: &LoginUrl) -> Result<()> {
        self.put(&format!("{URL_PREFIX}{}", id.0), url.to_value(), true)
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
    /// [`Error::Malformed`] for an invalid OTP value;
    /// [`Error::ClockOverflow`].
    pub fn set_custom_field(&mut self, id: &ElementId, field: &CustomField) -> Result<()> {
        check_custom(field)?;
        self.put(&format!("{FIELD_PREFIX}{}", id.0), field.to_value(), true)
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
            .flat_map(super::doc::Register::entries)
            .filter_map(|e| e.value().as_u64())
            .max();
        if current.is_some_and(|c| c >= value)
            && self.doc.register(key).is_some_and(|r| r.len() == 1)
        {
            return Ok(());
        }
        self.put(key, Value::uint(value.max(current.unwrap_or(0))), false)
    }

    /// Resolves a conflict on `key` by keeping the value written by edit
    /// `keep` (one of the entries listed by [`Item::conflicts`]). The other
    /// values of a field that keeps history go into the password history.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `key` has no value with stamp `keep`;
    /// [`Error::ClockOverflow`].
    pub fn resolve_conflict(&mut self, key: &str, keep: Stamp) -> Result<()> {
        let reg = self.doc.register(key).ok_or(Error::Malformed)?;
        let chosen = reg.get(&keep).ok_or(Error::Malformed)?.value().dup();
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
            self.doc.write(key, s, self.at, chosen, false);
        }
        Ok(())
    }

    /// Keeps the winning value of `key` and drops the conflict copies.
    ///
    /// # Errors
    /// [`Error::Malformed`] if `key` has no value; [`Error::ClockOverflow`].
    pub fn dismiss_conflict(&mut self, key: &str) -> Result<()> {
        let winner = self
            .doc
            .winner(key)
            .map(Entry::stamp)
            .ok_or(Error::Malformed)?;
        self.resolve_conflict(key, winner)
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

    /// Writes back, as part of this edit, every field that `previous` has
    /// and this item lacks. After a delete lost against a concurrent edit,
    /// the device that still holds the full pre-merge item uses this to
    /// restore the fields the delete removed.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn restore_missing_from(&mut self, previous: &Item) -> Result<()> {
        for (key, reg) in previous.doc.registers() {
            if key == DELETED_KEY || self.doc.register(key).is_some() {
                continue;
            }
            if let Some(w) = reg.winner() {
                self.put(key, w.value().dup(), false)?;
            }
        }
        Ok(())
    }
}

fn check_custom(field: &CustomField) -> Result<()> {
    if let super::types::CustomFieldValue::Otp(c) = &field.value {
        c.validate()?;
    }
    Ok(())
}
