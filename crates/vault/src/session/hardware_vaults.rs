//! Hardware-protected TOTP vaults (CRYPTO_SPEC §7.4, ADR-0028).
//!
//! A hardware-protected vault is a normal vault whose vault keys are wrapped
//! under its own key `TK`, and `TK` is wrapped once per registered security
//! key. AK never opens it, so a stolen master password alone is not enough.
//! The session holds `TK` and the vault keys only while the vault is open;
//! [`UnlockedSession::close_hardware_vault`] and
//! [`UnlockedSession::lock`] drop them (they zeroize on drop). The vault
//! holds one-time-code items only, and syncing such a vault is M6.

use std::collections::BTreeMap;

use mosaic_core::model::VaultMeta;
use mosaic_core::unlock::{PrfOutput, TotpVaultKey, credential_id_hash, prf_hw_vault_key};
use mosaic_core::{KeyGeneration, UnlockMethod, VaultId, VaultKey};

use super::unlock_methods::{
    HardwareKeyEnrollment, MAX_HARDWARE_KEYS, NewHardwareKey, VaultProtection, check_salt_unused,
    encode_hw_params,
};
use super::{
    Error, OpenVault, Result, UnlockedSession, load_vault_with, now, reseal_below_current,
};
use crate::storage::{Error as StorageError, VaultKeyWrap};

/// `SHA-256(credential_id)[..16]` after the length check.
fn method_id_of(credential_id: &[u8]) -> Result<[u8; 16]> {
    credential_id_hash(credential_id).map_err(|_| Error::InvalidHardwareKey)?;
    match UnlockMethod::hardware_key(credential_id) {
        UnlockMethod::HardwareKey(id) => Ok(id),
        _ => Err(Error::InvalidHardwareKey),
    }
}

impl UnlockedSession {
    fn tk_vault(&self, vault_id: VaultId) -> Result<&OpenVault> {
        let ov = self.vault(vault_id)?;
        if ov.protection == VaultKeyWrap::TotpKey {
            Ok(ov)
        } else {
            Err(Error::WrongVaultProtection)
        }
    }

    /// How a vault's keys are protected, and whether this session holds
    /// them now.
    ///
    /// # Errors
    /// [`Error::UnknownVault`].
    pub fn vault_protection(&self, vault_id: VaultId) -> Result<VaultProtection> {
        let ov = self.vault(vault_id)?;
        Ok(match ov.protection {
            VaultKeyWrap::AccountKey => VaultProtection::AccountKey,
            VaultKeyWrap::TotpKey => VaultProtection::HardwareKey {
                open: ov.tk.is_some(),
            },
        })
    }

    /// Creates a hardware-protected vault (§7.4): `TK = random(32)`, the
    /// first vault key (generation 1) under `TK`, `TK` wrapped under
    /// `W_hw` from `prf_hw` (the PRF output for `key.prf_salt`, which must
    /// be a salt of its own, never the §6.3 one), and the metadata with
    /// `name`. The vault, its metadata and its first TK wrapping are stored
    /// in one transaction. `key.prf_salt` must not equal any stored salt of
    /// the same credential ([`Error::InvalidHardwareKey`]). The vault is open afterwards.
    ///
    /// # Errors
    /// [`Error::InvalidHardwareKey`], [`Error::Store`], [`Error::Core`].
    pub fn create_hardware_vault(
        &mut self,
        name: &str,
        key: &NewHardwareKey,
        prf_hw: &PrfOutput,
    ) -> Result<VaultId> {
        key.check()?;
        let method_id = method_id_of(&key.credential_id)?;
        let params = encode_hw_params(key)?;
        check_salt_unused(&self.store, &method_id, &key.prf_salt, false, None)?;
        let vault_id = VaultId::new_v7()?;
        let tk = TotpVaultKey::generate()?;
        let vk = VaultKey::generate(KeyGeneration::FIRST)?;
        let vk_under_tk = vk.wrap_under_tk(&tk, &vault_id)?;
        let w_hw =
            prf_hw_vault_key(prf_hw, &key.credential_id).map_err(|_| Error::InvalidHardwareKey)?;
        let tk_wrapping = tk.wrap(&w_hw, &self.account_id, &vault_id, &method_id)?;
        drop(w_hw);
        let meta = VaultMeta::new(name, self.device, now())?;
        let meta_ct = meta.seal(&vk, &vault_id)?;
        self.store.create_tk_vault(
            &vault_id,
            KeyGeneration::FIRST,
            &vk_under_tk,
            &method_id,
            &tk_wrapping,
            &params,
            &meta_ct,
        )?;
        let mut ov = OpenVault::new(KeyGeneration::FIRST);
        ov.protection = VaultKeyWrap::TotpKey;
        ov.keys.insert(KeyGeneration::FIRST, vk);
        ov.meta = Some(meta);
        ov.tk = Some(tk);
        self.vaults.insert(vault_id, ov);
        Ok(vault_id)
    }

    /// The security keys registered for a vault (decoded parameters,
    /// ordered by `method_id`), so the platform can build its candidate
    /// list. Works while the vault is closed.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::WrongVaultProtection`],
    /// [`Error::Store`] (`Corrupt` for parameters that do not decode).
    pub fn hardware_vault_keys(&self, vault_id: VaultId) -> Result<Vec<HardwareKeyEnrollment>> {
        self.tk_vault(vault_id)?;
        self.store
            .tk_wrappings(&vault_id)?
            .into_iter()
            .map(|row| {
                HardwareKeyEnrollment::from_params(row.method_id, &row.params, row.updated_at_ms)
            })
            .collect()
    }

    /// Opens a hardware-protected vault: unwraps `TK` with `W_hw` from
    /// `prf_hw` (the PRF output of the registered credential
    /// `credential_id` for that wrapping's `prf_salt_hw`), then every
    /// readable vault key under `TK`, the metadata and the records, and
    /// adds the items to the search index. Opening an open vault only
    /// re-checks the key.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::WrongVaultProtection`],
    /// [`Error::InvalidHardwareKey`], [`Error::UnknownHardwareKey`] (no
    /// wrapping for this credential), [`Error::HardwareKeyRejected`] (the
    /// PRF output does not open it; the vault stays closed),
    /// [`Error::Store`], [`Error::Core`].
    pub fn open_hardware_vault(
        &mut self,
        vault_id: VaultId,
        credential_id: &[u8],
        prf_hw: &PrfOutput,
    ) -> Result<()> {
        let already_open = self.tk_vault(vault_id)?.tk.is_some();
        let method_id = method_id_of(credential_id)?;
        let row = self
            .store
            .tk_wrapping(&vault_id, &method_id)?
            .ok_or(Error::UnknownHardwareKey)?;
        let w_hw =
            prf_hw_vault_key(prf_hw, credential_id).map_err(|_| Error::InvalidHardwareKey)?;
        let tk = match row.wrapped_tk.unwrap_totp_vault_key(
            &w_hw,
            &self.account_id,
            &vault_id,
            &method_id,
        ) {
            Ok(tk) => tk,
            Err(mosaic_core::Error::Decryption) => return Err(Error::HardwareKeyRejected),
            Err(e) => return Err(e.into()),
        };
        drop(w_hw);
        if already_open {
            return Ok(());
        }
        let state = self.store.vault(&vault_id)?.ok_or(Error::UnknownVault)?;
        let mut keys = BTreeMap::new();
        for (generation, wrapped) in self.store.readable_vault_keys(&vault_id)? {
            if let Ok(vk) = wrapped.unwrap_vault_key_under_tk(&tk, &vault_id, generation) {
                keys.insert(generation, vk);
            }
        }
        let mut ov = load_vault_with(&self.store, &state, keys)?;
        ov.tk = Some(tk);
        if ov.available() {
            // Finishes a rotation that was interrupted before every record
            // was re-sealed (ADR-0012 item 7), like an unlock does.
            reseal_below_current(&mut self.store, vault_id, &ov)?;
        }
        for (item_id, item) in &ov.items {
            self.index.update(vault_id, *item_id, item);
        }
        self.vaults.insert(vault_id, ov);
        Ok(())
    }

    /// Closes a hardware-protected vault: drops `TK`, the vault keys, the
    /// items and their search entries (all zeroize on drop). Closing a
    /// closed vault does nothing.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::WrongVaultProtection`].
    pub fn close_hardware_vault(&mut self, vault_id: VaultId) -> Result<()> {
        let ov = self.vaults.get_mut(&vault_id).ok_or(Error::UnknownVault)?;
        if ov.protection != VaultKeyWrap::TotpKey {
            return Err(Error::WrongVaultProtection);
        }
        if ov.tk.is_none() {
            return Ok(());
        }
        for item_id in ov.items.keys() {
            self.index.remove(vault_id, *item_id);
        }
        let mut closed = OpenVault::new(ov.generation);
        closed.protection = VaultKeyWrap::TotpKey;
        *ov = closed;
        Ok(())
    }

    /// Registers another security key for an open hardware-protected vault
    /// (at most [`MAX_HARDWARE_KEYS`]; replacing a registered credential
    /// does not count). `prf_hw` is the key's PRF output for
    /// `key.prf_salt`, a salt of its own.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::WrongVaultProtection`],
    /// [`Error::VaultUnavailable`] (closed), [`Error::InvalidHardwareKey`],
    /// [`Error::HardwareKeyLimit`], [`Error::Store`], [`Error::Core`].
    pub fn add_hardware_vault_key(
        &mut self,
        vault_id: VaultId,
        key: &NewHardwareKey,
        prf_hw: &PrfOutput,
    ) -> Result<HardwareKeyEnrollment> {
        let tk = self
            .tk_vault(vault_id)?
            .tk
            .as_ref()
            .ok_or(Error::VaultUnavailable)?;
        key.check()?;
        let method_id = method_id_of(&key.credential_id)?;
        let params = encode_hw_params(key)?;
        check_salt_unused(
            &self.store,
            &method_id,
            &key.prf_salt,
            false,
            Some(vault_id),
        )?;
        let rows = self.store.tk_wrappings(&vault_id)?;
        let replaces = rows.iter().any(|r| r.method_id == method_id);
        if !replaces && rows.len() >= MAX_HARDWARE_KEYS {
            return Err(Error::HardwareKeyLimit);
        }
        let w_hw =
            prf_hw_vault_key(prf_hw, &key.credential_id).map_err(|_| Error::InvalidHardwareKey)?;
        let wrapped = tk.wrap(&w_hw, &self.account_id, &vault_id, &method_id)?;
        drop(w_hw);
        self.store
            .put_tk_wrapping(&vault_id, &method_id, &wrapped, &params)?;
        let row = self
            .store
            .tk_wrapping(&vault_id, &method_id)?
            .ok_or(Error::Store(StorageError::Corrupt))?;
        HardwareKeyEnrollment::from_params(method_id, &row.params, row.updated_at_ms)
    }

    /// Removes a security key from an open hardware-protected vault
    /// (opening it proved possession of a remaining key). `TK` does not
    /// change (THREAT_MODEL R24). Returns whether the key was registered.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::WrongVaultProtection`],
    /// [`Error::VaultUnavailable`] (closed), [`Error::LastHardwareKey`],
    /// [`Error::Store`].
    pub fn remove_hardware_vault_key(
        &mut self,
        vault_id: VaultId,
        method_id: &[u8; 16],
    ) -> Result<bool> {
        if self.tk_vault(vault_id)?.tk.is_none() {
            return Err(Error::VaultUnavailable);
        }
        match self.store.remove_tk_wrapping(&vault_id, method_id) {
            Ok(removed) => Ok(removed),
            Err(StorageError::LastTkWrapping) => Err(Error::LastHardwareKey),
            Err(e) => Err(e.into()),
        }
    }
}
