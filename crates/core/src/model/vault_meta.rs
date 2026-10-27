//! Vault metadata: name, icon and description (CRYPTO_SPEC §7.1).
//!
//! It uses the same replicated document and plaintext layout as items, so
//! renaming a vault on two devices merges like an item edit.

use core::fmt;

use super::clock::{Causality, Timestamp, VersionVector};
use super::doc::{Doc, DocJson, FieldConflict, Format, decode};
use super::value::{Value, to_json};
use crate::envelope::{MAX_VAULT_META_PLAINTEXT_LEN, VaultMetaCiphertext};
use crate::error::{Error, Result};
use crate::ids::{DeviceId, VaultId};
use crate::secret::{SecretVec, VaultKey};

const NAME: &str = "name";
const ICON: &str = "icon";
const DESCRIPTION: &str = "description";

/// Decrypted vault metadata.
#[derive(PartialEq, Eq)]
pub struct VaultMeta {
    doc: Doc,
}

impl fmt::Debug for VaultMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultMeta")
            .field("version", self.doc.vv())
            .finish_non_exhaustive()
    }
}

impl VaultMeta {
    /// Metadata of a new vault named `name`.
    ///
    /// # Errors
    /// None in practice (see [`crate::model::Item::new`]).
    pub fn new(name: &str, device: DeviceId, at: Timestamp) -> Result<Self> {
        let mut m = Self {
            doc: Doc::default(),
        };
        m.edit(device, at).set_name(name)?;
        Ok(m)
    }

    /// Starts an edit; all changes through it are one sync event.
    pub fn edit(&mut self, device: DeviceId, at: Timestamp) -> VaultMetaEdit<'_> {
        VaultMetaEdit {
            doc: &mut self.doc,
            device,
            at,
            stamp: None,
        }
    }

    /// The vault name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.doc.value(NAME).and_then(Value::as_str)
    }

    /// The icon identifier.
    #[must_use]
    pub fn icon(&self) -> Option<&str> {
        self.doc.value(ICON).and_then(Value::as_str)
    }

    /// The description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.doc.value(DESCRIPTION).and_then(Value::as_str)
    }

    /// Merges two replicas (see [`crate::model::Item::merge`]).
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            doc: self.doc.merge(&other.doc),
        }
    }

    /// Merges `received` into this local replica like
    /// [`crate::model::Item::merge_and_repair`] (CRYPTO_SPEC §10); every
    /// store persists the result instead of the plain [`VaultMeta::merge`].
    /// Metadata is never deleted, so the only repairs are for device-ID
    /// collisions (two writers sharing a device ID): one stamp with two
    /// contents, or equal version vectors with different contents (ADR-0015).
    /// The values the merge dropped are written back as local edits by
    /// `device`, so both names show as a conflict, and the returned flag is
    /// set.
    ///
    /// # Errors
    /// [`Error::DeviceIdReused`] if `received` holds edits by `device` that
    /// `self` has not seen (a counter for `device` above this replica's,
    /// or one of `device`'s stamps with different contents); rotate the
    /// device ID and call [`VaultMeta::merge_and_repair_after_reuse`] with
    /// the new one. [`Error::ClockOverflow`].
    pub fn merge_and_repair(&self, received: &Self, device: DeviceId) -> Result<(Self, bool)> {
        self.merge_and_repair_after_reuse(received, device, &[])
    }

    /// [`VaultMeta::merge_and_repair`] after this replica rotated away from
    /// the device IDs in `reused`, as
    /// [`crate::model::Item::merge_and_repair_after_reuse`].
    ///
    /// # Errors
    /// As [`VaultMeta::merge_and_repair`].
    pub fn merge_and_repair_after_reuse(
        &self,
        received: &Self,
        device: DeviceId,
        reused: &[DeviceId],
    ) -> Result<(Self, bool)> {
        if received.doc.vv().get(&device) > self.doc.vv().get(&device) {
            return Err(Error::DeviceIdReused);
        }
        let (doc, repair) = self.doc.merge_and_repair(&received.doc, device, reused)?;
        if repair.own_stamp_collision {
            return Err(Error::DeviceIdReused);
        }
        Ok((Self { doc }, repair.stamp_collision))
    }

    /// The version vector.
    #[must_use]
    pub const fn version(&self) -> &VersionVector {
        self.doc.vv()
    }

    /// See [`crate::model::Item::compare`].
    #[must_use]
    pub fn compare(&self, other: &Self) -> Causality {
        self.doc.vv().compare(other.doc.vv())
    }

    /// Fields with concurrent distinct values.
    #[must_use]
    pub fn conflicts(&self) -> Vec<FieldConflict<'_>> {
        self.doc.conflicts(|_| true)
    }

    /// The canonical plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_VAULT_META_PLAINTEXT_LEN`],
    /// [`Error::Allocation`].
    pub fn to_json(&self) -> Result<SecretVec> {
        to_json(
            &DocJson {
                doc: &self.doc,
                format: Format::Document,
            },
            MAX_VAULT_META_PLAINTEXT_LEN,
        )
    }

    /// Strictly decodes vault-metadata plaintext.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::UnsupportedVersion`],
    /// [`Error::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_VAULT_META_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Ok(Self {
            doc: decode(bytes, Format::Document)?,
        })
    }

    /// Encrypts the metadata of `vault_id`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`], [`Error::Allocation`], [`Error::Randomness`].
    pub fn seal(&self, vk: &VaultKey, vault_id: &VaultId) -> Result<VaultMetaCiphertext> {
        let pt = self.to_json()?;
        VaultMetaCiphertext::seal(vk, vault_id, pt.expose_secret())
    }

    /// Decrypts and decodes the metadata of `vault_id`.
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`], [`Error::Decryption`], and the
    /// errors of [`VaultMeta::from_json`].
    pub fn open(ct: &VaultMetaCiphertext, vk: &VaultKey, vault_id: &VaultId) -> Result<Self> {
        let pt = ct.open(vk, vault_id)?;
        Self::from_json(pt.expose_secret())
    }
}

/// An edit of vault metadata by one device.
pub struct VaultMetaEdit<'a> {
    doc: &'a mut Doc,
    device: DeviceId,
    at: Timestamp,
    stamp: Option<super::clock::Stamp>,
}

impl fmt::Debug for VaultMetaEdit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultMetaEdit")
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}

impl VaultMetaEdit<'_> {
    fn put(&mut self, key: &str, value: Option<&str>) -> Result<()> {
        let unchanged = match (self.doc.register(key), value) {
            (None, None) => true,
            (Some(r), Some(v)) => {
                r.len() == 1 && r.winner().and_then(|w| w.value().as_str()) == Some(v)
            }
            _ => false,
        };
        if unchanged {
            return Ok(());
        }
        let s = if let Some(s) = self.stamp {
            s
        } else {
            let s = self.doc.tick(self.device)?;
            self.stamp = Some(s);
            s
        };
        match value {
            Some(v) => self.doc.write(key, s, self.at, Value::text(v), false)?,
            None => {
                self.doc.remove_key(key);
            }
        }
        Ok(())
    }

    /// Renames the vault (resolving any name conflict).
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_name(&mut self, name: &str) -> Result<()> {
        self.put(NAME, Some(name))
    }

    /// Sets or clears the icon.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_icon(&mut self, icon: Option<&str>) -> Result<()> {
        self.put(ICON, icon)
    }

    /// Sets or clears the description.
    ///
    /// # Errors
    /// [`Error::ClockOverflow`].
    pub fn set_description(&mut self, description: Option<&str>) -> Result<()> {
        self.put(DESCRIPTION, description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::KeyGeneration;

    fn dev(b: u8) -> DeviceId {
        DeviceId::from_bytes([b; 16]).unwrap()
    }

    #[test]
    fn edit_merge_seal() {
        let at = Timestamp::from_unix_millis(1);
        let mut m = VaultMeta::new("Personal", dev(1), at).unwrap();
        assert_eq!(m.name(), Some("Personal"));
        let v0 = m.version().clone();
        m.edit(dev(1), at).set_name("Personal").unwrap();
        assert_eq!(m.version(), &v0, "unchanged edit is a no-op");
        let mut a = VaultMeta::from_json(m.to_json().unwrap().expose_secret()).unwrap();
        let mut b = VaultMeta::from_json(m.to_json().unwrap().expose_secret()).unwrap();
        {
            let mut e = a.edit(dev(2), at);
            e.set_name("Home").unwrap();
            e.set_icon(Some("house")).unwrap();
        }
        b.edit(dev(3), at).set_description(Some("d")).unwrap();
        b.edit(dev(3), at).set_name("Family").unwrap();
        let merged = a.merge(&b);
        assert_eq!(merged, b.merge(&a));
        assert_eq!(merged.compare(&a), Causality::Dominates);
        assert_eq!(a.compare(&b), Causality::Concurrent);
        assert_eq!(merged.icon(), Some("house"));
        assert_eq!(merged.description(), Some("d"));
        assert_eq!(merged.conflicts().len(), 1);
        let mut resolved = VaultMeta::from_json(merged.to_json().unwrap().expose_secret()).unwrap();
        resolved.edit(dev(1), at).set_name("Home").unwrap();
        assert!(resolved.conflicts().is_empty());
        resolved.edit(dev(1), at).set_icon(None).unwrap();
        assert_eq!(resolved.icon(), None);
        let v = resolved.version().clone();
        resolved.edit(dev(1), at).set_icon(None).unwrap();
        assert_eq!(resolved.version(), &v);

        let vault = VaultId::from_bytes([0x0B; 16]).unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        let ct = resolved.seal(&vk, &vault).unwrap();
        assert_eq!(VaultMeta::open(&ct, &vk, &vault).unwrap(), resolved);
        let other = VaultId::from_bytes([0x0A; 16]).unwrap();
        assert_eq!(
            VaultMeta::open(&ct, &vk, &other).unwrap_err(),
            Error::Decryption
        );
        assert!(format!("{resolved:?}").starts_with("VaultMeta"));
        assert!(format!("{:?}", resolved.edit(dev(1), at)).starts_with("VaultMetaEdit"));
    }

    #[test]
    fn merge_and_repair_keeps_both_names_of_a_shared_stamp() {
        // Review M2 (crypto, sync, tests): two writers with one device ID
        // rename from the same state, so both edits carry the same stamp.
        let at = Timestamp::from_unix_millis(1);
        let base = VaultMeta::new("Base", dev(1), at).unwrap();
        let copy = |m: &VaultMeta| VaultMeta::from_json(m.to_json().unwrap().expose_secret());
        let (mut a, mut b) = (copy(&base).unwrap(), copy(&base).unwrap());
        a.edit(dev(4), at).set_name("from A").unwrap();
        b.edit(dev(4), at).set_name("from B").unwrap();
        assert_eq!(a.compare(&b), Causality::Equal);
        // The pure join keeps one name only.
        assert_eq!(a.merge(&b).conflicts().len(), 0);

        // The sharing writer itself must rotate its ID first.
        assert_eq!(
            a.merge_and_repair(&b, dev(4)).unwrap_err(),
            Error::DeviceIdReused
        );
        // Any other device keeps both names as a conflict.
        let (merged, collision) = a.merge_and_repair(&b, dev(5)).unwrap();
        assert!(collision);
        let conflicts = merged.conflicts();
        assert_eq!(conflicts.len(), 1);
        let mut names: Vec<&str> = conflicts[0]
            .losers
            .iter()
            .chain([&conflicts[0].winner])
            .filter_map(|e| e.value().as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(names, ["from A", "from B"]);

        // A received counter of this device above its own: reuse.
        let mut ahead = copy(&base).unwrap();
        ahead.edit(dev(1), at).set_name("not mine").unwrap();
        assert_eq!(
            base.merge_and_repair(&ahead, dev(1)).unwrap_err(),
            Error::DeviceIdReused
        );
        // Without a collision the result is the plain merge.
        let (plain, collision) = base.merge_and_repair(&a, dev(1)).unwrap();
        assert!(!collision);
        assert_eq!(plain, base.merge(&a));
    }

    #[test]
    fn size_limit() {
        let at = Timestamp::from_unix_millis(1);
        let m = VaultMeta::new(&"n".repeat(MAX_VAULT_META_PLAINTEXT_LEN), dev(1), at).unwrap();
        assert_eq!(m.to_json().unwrap_err(), Error::SizeLimit);
        assert_eq!(
            VaultMeta::from_json(&vec![b' '; MAX_VAULT_META_PLAINTEXT_LEN + 1]).unwrap_err(),
            Error::SizeLimit
        );
    }
}
