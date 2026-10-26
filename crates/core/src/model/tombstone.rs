//! Tombstones: the plaintext of a deleted record (CRYPTO_SPEC §7.2).
//!
//! A tombstone is the state of a deleted item: its final version vector
//! and the deletion entries (who deleted it, when). It is a lossless
//! encoding of that state, so merging a tombstone gives the same result as
//! merging the deleted item it came from:
//!
//! ```json
//! {"schema_version":1,"vv":{"<device>":7},"deleted":[{"at":1730000000000,"c":7,"d":"<device>","v":null}]}
//! ```

use core::fmt;

use super::clock::{Timestamp, VersionVector};
use super::doc::{Doc, DocJson, Format, decode};
use super::item::{DeletionInfo, deletion_info};
use super::value::to_json;
use crate::envelope::{MAX_TOMBSTONE_PLAINTEXT_LEN, TombstoneCiphertext};
use crate::error::{Error, Result};
use crate::ids::{DeviceId, ItemId, VaultId};
use crate::secret::{SecretVec, VaultKey};

/// The plaintext of a deletion record. Get one from
/// [`crate::model::Item::to_tombstone`]; turn it back into a (deleted)
/// item with `Item::from` to merge it.
#[derive(PartialEq, Eq)]
pub struct Tombstone {
    pub(crate) doc: Doc,
}

impl fmt::Debug for Tombstone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tombstone")
            .field("version", self.doc.vv())
            .finish_non_exhaustive()
    }
}

impl Tombstone {
    /// The final version vector.
    #[must_use]
    pub const fn version(&self) -> &VersionVector {
        self.doc.vv()
    }

    fn info(&self) -> DeletionInfo {
        deletion_info(&self.doc).unwrap_or(DeletionInfo {
            deleted_at: Timestamp::from_unix_millis(0),
            deleted_by: Vec::new(),
        })
    }

    /// When the item was deleted (the latest deletion, by the deleting
    /// devices' clocks).
    #[must_use]
    pub fn deleted_at(&self) -> Timestamp {
        self.info().deleted_at
    }

    /// The devices that deleted the item.
    #[must_use]
    pub fn deleted_by(&self) -> Vec<DeviceId> {
        self.info().deleted_by
    }

    /// The canonical plaintext in a zeroizing buffer.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_TOMBSTONE_PLAINTEXT_LEN`],
    /// [`Error::Allocation`].
    pub fn to_json(&self) -> Result<SecretVec> {
        to_json(
            &DocJson {
                doc: &self.doc,
                format: Format::Tombstone,
            },
            MAX_TOMBSTONE_PLAINTEXT_LEN,
        )
    }

    /// Strictly decodes tombstone plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::UnsupportedVersion`],
    /// [`Error::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_TOMBSTONE_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Ok(Self {
            doc: decode(bytes, Format::Tombstone)?,
        })
    }

    /// Encrypts the tombstone for record `item_id` of `vault_id`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::Allocation`], [`Error::Randomness`].
    pub fn seal(
        &self,
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
    ) -> Result<TombstoneCiphertext> {
        let pt = self.to_json()?;
        TombstoneCiphertext::seal(vk, vault_id, item_id, pt.expose_secret())
    }

    /// Decrypts and decodes the tombstone of record `item_id`.
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`], [`Error::ItemCorrupted`] for every
    /// authentication failure, and the errors of [`Tombstone::from_json`].
    pub fn open(
        ct: &TombstoneCiphertext,
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
    ) -> Result<Self> {
        let pt = ct.open(vk, vault_id, item_id).map_err(|e| match e {
            Error::Decryption => Error::ItemCorrupted { item_id: *item_id },
            other => other,
        })?;
        Self::from_json(pt.expose_secret())
    }
}
