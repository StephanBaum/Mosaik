//! Vaults: create, list, rename (encrypted metadata, CRYPTO_SPEC §7.1) and
//! key rotation with monotonic key generations.

use core::fmt;

use mosaic_core::model::VaultMeta;
use mosaic_core::{DeviceId, KeyGeneration, VaultId, VaultKey};

use super::sync::key_acceptance_error;
use super::{Error, OpenVault, Result, UnlockedSession, now, reseal_below_current};

/// A vault as listed by [`UnlockedSession::vaults`]. `Debug` does not show
/// the name (user content, CRYPTO_SPEC §12), only whether there is one.
#[derive(Clone, Copy)]
pub struct VaultSummary<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// Its name, if the metadata is present and readable.
    pub name: Option<&'a str>,
    /// The highest accepted key generation (§7.1).
    pub key_generation: KeyGeneration,
    /// Whether this session holds the vault's current key. A
    /// hardware-protected TOTP vault (§7.4) is not available here.
    pub available: bool,
    /// Live items, in and outside the trash.
    pub item_count: usize,
}

impl fmt::Debug for VaultSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultSummary")
            .field("vault_id", &self.vault_id)
            .field("has_name", &self.name.is_some())
            .field("key_generation", &self.key_generation)
            .field("available", &self.available)
            .field("item_count", &self.item_count)
            .finish()
    }
}

impl UnlockedSession {
    /// Creates a vault: a fresh vault key of generation 1 wrapped under the
    /// account key, and its encrypted metadata with `name`. The name is
    /// user content; it never reaches the disk or the server in cleartext.
    ///
    /// # Errors
    /// [`Error::Store`], [`Error::Core`].
    pub fn create_vault(&mut self, name: &str) -> Result<VaultId> {
        let vault_id = VaultId::new_v7()?;
        let vk = VaultKey::generate(KeyGeneration::FIRST)?;
        let wrapped = vk.wrap(&self.account_key, &self.account_id, &vault_id)?;
        let meta = VaultMeta::new(name, self.device, now())?;
        let meta_ct = meta.seal(&vk, &vault_id)?;
        self.store
            .create_vault(&vault_id, KeyGeneration::FIRST, &wrapped)?;
        self.store.set_vault_meta(&vault_id, &meta_ct)?;
        let mut ov = OpenVault::new(KeyGeneration::FIRST);
        ov.keys.insert(KeyGeneration::FIRST, vk);
        ov.meta = Some(meta);
        self.vaults.insert(vault_id, ov);
        Ok(vault_id)
    }

    /// Every vault this store knows, by ID.
    #[must_use]
    pub fn vaults(&self) -> Vec<VaultSummary<'_>> {
        self.vaults
            .iter()
            .map(|(id, ov)| VaultSummary {
                vault_id: *id,
                name: ov.meta.as_ref().and_then(VaultMeta::name),
                key_generation: ov.generation,
                available: ov.available(),
                item_count: ov.items.values().filter(|i| !i.is_deleted()).count(),
            })
            .collect()
    }

    /// The decrypted metadata of a vault, if present and readable.
    ///
    /// # Errors
    /// [`Error::UnknownVault`].
    pub fn vault_meta(&self, vault_id: VaultId) -> Result<Option<&VaultMeta>> {
        Ok(self.vault(vault_id)?.meta.as_ref())
    }

    /// Renames a vault: an edit of its encrypted metadata, re-sealed under
    /// the current key.
    ///
    /// A vault without readable metadata (not received yet, or damaged)
    /// gets new metadata written under a fresh one-off writer ID instead of
    /// this device's ID: the device may have edited the lost metadata
    /// before, and restarting its own counter would reuse those stamps,
    /// so the new name could silently replace (or be replaced by) a name
    /// it never saw. Under a fresh ID the rename is concurrent with any
    /// earlier metadata, and a merge keeps both names (ADR-0012 item 4).
    /// Metadata from before the damage that holds this device's own edits
    /// is then reported as `DeviceIdReused`, as for a store restored from
    /// a backup; rotate the device ID and apply it again.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::VaultUnavailable`],
    /// [`Error::Store`], [`Error::Core`].
    pub fn rename_vault(&mut self, vault_id: VaultId, name: &str) -> Result<()> {
        let device = self.device;
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        let vk = ov.current_key()?;
        let meta = if let Some(m) = &ov.meta {
            let mut meta = VaultMeta::from_json(m.to_json()?.expose_secret())?;
            meta.edit(device, now()).set_name(name)?;
            meta
        } else {
            VaultMeta::new(name, DeviceId::new_v7()?, now())?
        };
        let ct = meta.seal(vk, &vault_id)?;
        self.store.set_vault_meta(&vault_id, &ct)?;
        ov.meta = Some(meta);
        // The unreadable row is replaced; received metadata now merges
        // with this rename instead of replacing it.
        ov.meta_unreadable = false;
        Ok(())
    }

    /// Rotates a vault's key (CRYPTO_SPEC §8.3): a new key one generation
    /// up, wrapped under the account key and accepted (the store raises the
    /// anti-rollback mark, §7.1), then every readable record and the
    /// metadata re-encrypted under it and queued for upload. Older keys
    /// stay held for records that could not be re-encrypted. If the
    /// re-encryption is interrupted, the next unlock finishes it.
    ///
    /// Two devices that rotate the same vault concurrently create two keys
    /// for one generation; each refuses the other's
    /// ([`Error::VaultKeyConflict`], ADR-0013).
    ///
    /// Attachments keep their attachment IDs and file keys: this rotation
    /// is not a revocation. Revoking a vault member (CRYPTO_SPEC §8.3,
    /// sharing in M6/M7) must also re-seal every attachment under a fresh
    /// attachment ID and file key (ADR-0017); until then nobody outside
    /// this account can hold a vault key.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::VaultUnavailable`],
    /// [`Error::Core`] (`InvalidKeyGeneration` at the last generation),
    /// [`Error::KeyGenerationRollback`] / [`Error::VaultKeyConflict`] if
    /// the store already holds another key at or above the next
    /// generation, [`Error::Store`].
    pub fn rotate_vault_key(&mut self, vault_id: VaultId) -> Result<KeyGeneration> {
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        ov.current_key()?;
        let next = ov.generation.next()?;
        let vk = VaultKey::generate(next)?;
        let wrapped = vk.wrap(&self.account_key, &self.account_id, &vault_id)?;
        self.store
            .accept_vault_key(&vault_id, next, &wrapped)
            .map_err(key_acceptance_error)?;
        ov.keys.insert(next, vk);
        ov.generation = next;
        reseal_below_current(&mut self.store, vault_id, ov)?;
        Ok(next)
    }
}
