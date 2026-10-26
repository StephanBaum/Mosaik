//! The vault and item data model (M2): item types, their plaintext
//! serialization, and the sync merge. Pure data and logic, no IO and no
//! system clock (callers pass device IDs and wall-clock times).
//!
//! # Layers
//!
//! - [`Item`], [`VaultMeta`] and [`Tombstone`] are what gets encrypted
//!   into [`crate::ItemCiphertext`], [`crate::VaultMetaCiphertext`] and
//!   [`crate::TombstoneCiphertext`] (`seal` / `open` on each type).
//! - Every one of them is a replicated document: a version vector and a
//!   map of field registers, each value tagged with the Lamport [`Stamp`]
//!   of the edit that wrote it. [`Item::merge`] is a pure join
//!   (commutative, associative, idempotent). The winner of a field is the
//!   value with the largest stamp; concurrent losers stay on the item as
//!   conflict copies ([`Item::conflicts`]) until a device resolves them.
//!   A delete that races with an edit loses: the edit survives and the item
//!   is flagged ([`Item::deleted_concurrently`]).
//! - The plaintext is self-describing JSON (`serde_json`) with
//!   `schema_version` 1. Unknown fields, unknown members of known values and unknown
//!   top-level members are preserved and written back, so an older client
//!   never drops what a newer one wrote.
//! - All strings are [`Text`] (zeroized on drop), secrets are read into the
//!   crate's secret types, and plaintext buffers are [`crate::SecretVec`].
//!
//! ```
//! use mosaic_core::model::{Item, ItemType, SecretField, Timestamp};
//! use mosaic_core::DeviceId;
//!
//! # fn main() -> Result<(), mosaic_core::Error> {
//! let laptop = DeviceId::new_v7()?;
//! let phone = DeviceId::new_v7()?;
//! let now = Timestamp::from_unix_millis(1_761_000_000_000);
//!
//! let mut item = Item::new(&ItemType::Login, laptop, now)?;
//! item.edit(laptop, now).set_title("Example")?;
//!
//! // Both devices edit the same field while offline.
//! let mut on_phone = Item::from_json(item.to_json()?.expose_secret())?;
//! item.edit(laptop, now).set_secret(SecretField::Password, "from-laptop")?;
//! on_phone.edit(phone, now).set_secret(SecretField::Password, "from-phone")?;
//!
//! let merged = item.merge(&on_phone);
//! assert_eq!(merged, on_phone.merge(&item));
//! assert_eq!(merged.conflicts().len(), 1); // the loser is kept, not lost
//! # Ok(())
//! # }
//! ```

mod clock;
mod doc;
mod item;
mod tombstone;
mod types;
mod value;
mod vault_meta;

pub use clock::{Causality, Stamp, Timestamp, VersionVector};
pub use doc::{Entry, FieldConflict, MAX_FIELD_KEY_LEN, SCHEMA_VERSION};
pub use item::{
    DeletionInfo, ElementId, HistoryEntry, Item, ItemEdit, ItemType, MAX_HISTORY, MAX_TAG_LEN,
    SecretField, TextField,
};
pub use tombstone::Tombstone;
pub use types::{
    AttachmentRef, CustomField, CustomFieldValue, LoginUrl, MAX_CREDENTIAL_ID_LEN,
    MAX_OTP_SECRET_LEN, MAX_USER_HANDLE_LEN, OtpAlgorithm, OtpConfig, OtpKind, PasskeyCredential,
    SshKeyMaterial, UrlMatch,
};
pub use value::{MAX_JSON_DEPTH, Number, Object, Text, Value};
pub use vault_meta::{VaultMeta, VaultMetaEdit};
