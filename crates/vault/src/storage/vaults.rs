//! Vaults, their wrapped keys per generation, encrypted metadata, TK
//! wrappings and sync cursors (CRYPTO_SPEC §7.1, §7.4, §10).

use mosaic_core::{KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
use super::records::check_pending;
use super::{
    MAX_UNLOCK_PARAMS_LEN, PendingPolicy, Store, attachments, check_generation,
    generation_from_sql, now_ms, seq_from_sql, seq_to_sql, vault_id_from_sql,
};

/// What a vault's keys are wrapped under. Fixed when the vault is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VaultKeyWrap {
    /// A normal vault: every VK is wrapped under AK (§7.1), AAD label
    /// `wrap/vault`. Unwrap with `WrappedKey::unwrap_vault_key`.
    AccountKey,
    /// A hardware-protected TOTP vault (§7.4): every VK is wrapped under the
    /// vault's TK, AAD label `wrap/vault-under-tk`, and TK itself is held
    /// only in the vault's TK wrappings ([`Store::tk_wrappings`]), one per
    /// hardware key. Never unwrapped with AK.
    TotpKey,
}

impl VaultKeyWrap {
    /// The stored code: 1 for [`Self::AccountKey`], 2 for [`Self::TotpKey`].
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::AccountKey => 1,
            Self::TotpKey => 2,
        }
    }

    /// Looks up a stored code.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::AccountKey),
            2 => Some(Self::TotpKey),
            _ => None,
        }
    }
}

/// The key-generation state of a vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultState {
    /// The vault.
    pub vault_id: VaultId,
    /// What the vault's keys are wrapped under, so the caller knows which
    /// unwrap to use.
    pub key_wrap: VaultKeyWrap,
    /// The generation of the newest vault key held. New items, tombstones
    /// and metadata are sealed under it.
    pub current_generation: KeyGeneration,
    /// The anti-rollback high-water mark (§7.1). It never decreases.
    /// New wrapped keys below it are rejected, and records and metadata
    /// are accepted only at exactly this generation.
    pub highest_accepted_generation: KeyGeneration,
}

/// A stored wrapping of a hardware-protected vault's TK (§7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredTkWrapping {
    /// The hardware key: `SHA-256(credential_id)[..16]`, as in
    /// `mosaic_core::UnlockMethod::hardware_key`.
    pub method_id: [u8; 16],
    /// TK wrapped under that key's `W_hw`.
    pub wrapped_tk: WrappedKey,
    /// Public parameters, in particular this wrapping's own `prf_salt_hw`.
    pub params: Vec<u8>,
    /// When the row was last written (ms since the Unix epoch).
    pub updated_at_ms: i64,
}

const SELECT_VAULT: &str =
    "SELECT vault_id, key_wrap, current_generation, highest_accepted_generation FROM vaults";

type VaultRow = (Vec<u8>, i64, i64, i64);

fn vault_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<VaultRow> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
}

fn decode_vault(row: VaultRow) -> Result<VaultState> {
    let (id, key_wrap, current, highest) = row;
    Ok(VaultState {
        vault_id: vault_id_from_sql(&id)?,
        key_wrap: u8::try_from(key_wrap)
            .ok()
            .and_then(VaultKeyWrap::from_code)
            .ok_or(Error::Corrupt)?,
        current_generation: generation_from_sql(current)?,
        highest_accepted_generation: generation_from_sql(highest)?,
    })
}

pub(super) fn read_vault(conn: &Connection, vault_id: &VaultId) -> Result<Option<VaultState>> {
    conn.query_row(
        &format!("{SELECT_VAULT} WHERE vault_id = ?1"),
        [vault_id.as_bytes()],
        vault_row,
    )
    .optional()?
    .map(decode_vault)
    .transpose()
}

/// The vault's state, or [`Error::UnknownVault`].
pub(super) fn require_vault(conn: &Connection, vault_id: &VaultId) -> Result<VaultState> {
    read_vault(conn, vault_id)?.ok_or(Error::UnknownVault)
}

fn read_floor(conn: &Connection, vault_id: &VaultId) -> Result<Option<KeyGeneration>> {
    conn.query_row(
        "SELECT generation FROM vault_generation_floors WHERE vault_id = ?1",
        [vault_id.as_bytes()],
        |r| r.get::<_, i64>(0),
    )
    .optional()?
    .map(generation_from_sql)
    .transpose()
}

fn upsert_key(
    conn: &Connection,
    vault_id: &VaultId,
    generation: KeyGeneration,
    wrapped: &WrappedKey,
) -> Result<()> {
    conn.execute(
        "INSERT INTO vault_keys (vault_id, generation, wrapped_vk, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (vault_id, generation) DO UPDATE SET
             wrapped_vk = excluded.wrapped_vk,
             updated_at = excluded.updated_at",
        params![
            vault_id.as_bytes(),
            generation.get(),
            wrapped.as_bytes(),
            now_ms()
        ],
    )?;
    Ok(())
}

/// Replaces the wrapping of a held vault key in place. Returns whether the
/// key was held.
pub(super) fn rewrap_key(
    conn: &Connection,
    vault_id: &VaultId,
    generation: KeyGeneration,
    wrapped: &WrappedKey,
) -> Result<bool> {
    let n = conn.execute(
        "UPDATE vault_keys SET wrapped_vk = ?3, updated_at = ?4
         WHERE vault_id = ?1 AND generation = ?2",
        params![
            vault_id.as_bytes(),
            generation.get(),
            wrapped.as_bytes(),
            now_ms()
        ],
    )?;
    Ok(n > 0)
}

type TkRow = (Vec<u8>, Vec<u8>, Vec<u8>, i64);

fn tk_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<TkRow> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
}

fn decode_tk(row: TkRow) -> Result<StoredTkWrapping> {
    let (method_id, wrapped, params, updated_at_ms) = row;
    Ok(StoredTkWrapping {
        method_id: method_id.try_into().map_err(|_| Error::Corrupt)?,
        wrapped_tk: WrappedKey::from_bytes(&wrapped).map_err(|_| Error::Corrupt)?,
        params,
        updated_at_ms,
    })
}

impl Store {
    /// Adds a normal vault ([`VaultKeyWrap::AccountKey`]) with its first
    /// wrapped vault key. Same as [`Store::create_vault_with`].
    ///
    /// # Errors
    /// See [`Store::create_vault_with`].
    pub fn create_vault(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        wrapped_vault_key: &WrappedKey,
    ) -> Result<VaultState> {
        self.create_vault_with(
            vault_id,
            generation,
            VaultKeyWrap::AccountKey,
            wrapped_vault_key,
        )
    }

    /// Adds a vault with its first wrapped vault key, wrapped as
    /// `key_wrap` says. `generation` is normally [`KeyGeneration::FIRST`]
    /// for a new vault, or the vault's current generation when a device
    /// first learns about it. For a [`VaultKeyWrap::TotpKey`] vault, store
    /// the TK wrappings next ([`Store::put_tk_wrapping`]).
    ///
    /// A vault this store held before and deleted keeps its anti-rollback
    /// mark ([`Store::generation_floor`]): it can only come back at that
    /// generation or a later one, so a server cannot "remove" a vault and
    /// re-share an older generation of it (§7.1).
    ///
    /// # Errors
    /// [`Error::VaultExists`]; [`Error::KeyGenerationRollback`] if
    /// `generation` is below the vault's generation floor.
    pub fn create_vault_with(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        key_wrap: VaultKeyWrap,
        wrapped_vault_key: &WrappedKey,
    ) -> Result<VaultState> {
        let tx = self.write()?;
        if read_vault(&tx, vault_id)?.is_some() {
            return Err(Error::VaultExists);
        }
        if let Some(floor) = read_floor(&tx, vault_id)?
            && generation < floor
        {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: floor.get(),
                offered: generation.get(),
            });
        }
        tx.execute(
            "INSERT INTO vaults (vault_id, key_wrap, current_generation,
                                 highest_accepted_generation, updated_at)
             VALUES (?1, ?2, ?3, ?3, ?4)",
            params![
                vault_id.as_bytes(),
                key_wrap.code(),
                generation.get(),
                now_ms()
            ],
        )?;
        upsert_key(&tx, vault_id, generation, wrapped_vault_key)?;
        tx.commit()?;
        Ok(VaultState {
            vault_id: *vault_id,
            key_wrap,
            current_generation: generation,
            highest_accepted_generation: generation,
        })
    }

    /// The highest generation accepted for a vault before it was deleted
    /// from this store, if it ever was. It never decreases and is never
    /// removed.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn generation_floor(&self, vault_id: &VaultId) -> Result<Option<KeyGeneration>> {
        read_floor(&self.conn, vault_id)
    }

    /// Records an accepted wrapped vault key after a rotation (§8.3). The
    /// caller has unwrapped it (with AK, or with TK for a
    /// [`VaultKeyWrap::TotpKey`] vault), which authenticates the generation
    /// in the AAD.
    ///
    /// A generation above the highest accepted one raises both the current
    /// and the highest accepted generation to it. The highest accepted
    /// generation never decreases. Afterwards, re-seal the records that
    /// [`Store::records_below_generation`] lists. Offering the held
    /// wrapping of the highest accepted generation again is a no-op; any
    /// other wrapping of a generation that is already held is refused
    /// ([`Error::VaultKeyConflict`]): this store cannot tell a second
    /// wrapping of the same key from a different key of the same
    /// generation, so it never replaces one here. To re-wrap a held key,
    /// use [`Store::rewrap_vault_key`].
    ///
    /// # Errors
    /// [`Error::UnknownVault`]; [`Error::KeyGenerationRollback`] if
    /// `generation` is below the highest accepted generation;
    /// [`Error::VaultKeyConflict`] if a different wrapping of `generation`
    /// is held.
    pub fn accept_vault_key(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        wrapped_vault_key: &WrappedKey,
    ) -> Result<VaultState> {
        let tx = self.write()?;
        let mut state = require_vault(&tx, vault_id)?;
        if generation < state.highest_accepted_generation {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: state.highest_accepted_generation.get(),
                offered: generation.get(),
            });
        }
        let held: Option<Vec<u8>> = tx
            .query_row(
                "SELECT wrapped_vk FROM vault_keys WHERE vault_id = ?1 AND generation = ?2",
                params![vault_id.as_bytes(), generation.get()],
                |r| r.get(0),
            )
            .optional()?;
        match held {
            Some(bytes) if bytes.as_slice() == wrapped_vault_key.as_bytes().as_slice() => {
                return Ok(state);
            }
            Some(_) => {
                return Err(Error::VaultKeyConflict {
                    generation: generation.get(),
                });
            }
            None => {}
        }
        if generation > state.highest_accepted_generation {
            state.highest_accepted_generation = generation;
            state.current_generation = generation;
            tx.execute(
                "UPDATE vaults SET current_generation = ?2, highest_accepted_generation = ?2,
                     updated_at = ?3
                 WHERE vault_id = ?1",
                params![vault_id.as_bytes(), generation.get(), now_ms()],
            )?;
        }
        upsert_key(&tx, vault_id, generation, wrapped_vault_key)?;
        tx.commit()?;
        self.scrub_wal()?;
        Ok(state)
    }

    /// Replaces the wrapping of a vault key this store already holds, of
    /// any generation, for example after the caller re-wrapped it under a
    /// new AK. It cannot add a key: no generation below the accepted one
    /// is ever added (§7.1). For a whole §6.6 re-key, use
    /// [`Store::rekey_account`], which changes every wrapping in one
    /// transaction.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::UnknownVaultKey`] if no key of
    /// `generation` is held.
    pub fn rewrap_vault_key(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        wrapped_vault_key: &WrappedKey,
    ) -> Result<()> {
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        if !rewrap_key(&tx, vault_id, generation, wrapped_vault_key)? {
            return Err(Error::UnknownVaultKey);
        }
        tx.commit()?;
        self.scrub_wal()
    }

    /// The key-generation state of a vault, or `None` if it is unknown.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault(&self, vault_id: &VaultId) -> Result<Option<VaultState>> {
        read_vault(&self.conn, vault_id)
    }

    /// Every vault, ordered by ID.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vaults(&self) -> Result<Vec<VaultState>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT_VAULT} ORDER BY vault_id"))?;
        let rows = stmt.query_map([], vault_row)?;
        rows.map(|row| decode_vault(row?)).collect()
    }

    /// The wrapped vault key of `generation`, if held.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault_key(
        &self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<Option<WrappedKey>> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT wrapped_vk FROM vault_keys WHERE vault_id = ?1 AND generation = ?2",
                params![vault_id.as_bytes(), generation.get()],
                |r| r.get(0),
            )
            .optional()?;
        row.map(|b| WrappedKey::from_bytes(&b).map_err(|_| Error::Corrupt))
            .transpose()
    }

    /// Every wrapped vault key held for a vault, oldest generation first.
    /// Older generations stay readable for records not yet re-encrypted.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault_keys(&self, vault_id: &VaultId) -> Result<Vec<(KeyGeneration, WrappedKey)>> {
        let mut stmt = self.conn.prepare(
            "SELECT generation, wrapped_vk FROM vault_keys WHERE vault_id = ?1
             ORDER BY generation",
        )?;
        let rows = stmt.query_map([vault_id.as_bytes()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|row| {
            let (generation, wrapped) = row?;
            Ok((
                generation_from_sql(generation)?,
                WrappedKey::from_bytes(&wrapped).map_err(|_| Error::Corrupt)?,
            ))
        })
        .collect()
    }

    /// Every wrapped vault key held for a vault that decodes, oldest
    /// generation first, leaving out damaged rows instead of failing like
    /// [`Store::vault_keys`]. A session loading a vault uses it, so one
    /// damaged key row does not hide the other generations (records still
    /// sealed under them stay readable and keep their rollback anchor).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn readable_vault_keys(
        &self,
        vault_id: &VaultId,
    ) -> Result<Vec<(KeyGeneration, WrappedKey)>> {
        let mut stmt = self.conn.prepare(
            "SELECT generation, wrapped_vk FROM vault_keys WHERE vault_id = ?1
             ORDER BY generation",
        )?;
        let rows = stmt.query_map([vault_id.as_bytes()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        let mut keys = Vec::new();
        for row in rows {
            let (generation, wrapped) = row?;
            if let (Ok(generation), Ok(wrapped)) = (
                generation_from_sql(generation),
                WrappedKey::from_bytes(&wrapped),
            ) {
                keys.push((generation, wrapped));
            }
        }
        Ok(keys)
    }

    /// Deletes the wrapped vault keys of every generation below
    /// `keep_from`, once no record and no metadata still needs them.
    /// Returns the number of keys deleted. The accepted generation is not
    /// affected.
    ///
    /// # Errors
    /// [`Error::UnknownVault`]; [`Error::VaultKeyInUse`] if `keep_from` is
    /// above the current generation, or a record or the metadata is below
    /// `keep_from`.
    pub fn prune_vault_keys(
        &mut self,
        vault_id: &VaultId,
        keep_from: KeyGeneration,
    ) -> Result<usize> {
        let tx = self.write()?;
        let state = require_vault(&tx, vault_id)?;
        if keep_from > state.current_generation {
            return Err(Error::VaultKeyInUse);
        }
        let in_use: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM records WHERE vault_id = ?1 AND key_generation < ?2)
                 OR EXISTS (SELECT 1 FROM vaults WHERE vault_id = ?1 AND meta_generation < ?2)",
            params![vault_id.as_bytes(), keep_from.get()],
            |r| r.get(0),
        )?;
        if in_use {
            return Err(Error::VaultKeyInUse);
        }
        let n = tx.execute(
            "DELETE FROM vault_keys WHERE vault_id = ?1 AND generation < ?2",
            params![vault_id.as_bytes(), keep_from.get()],
        )?;
        tx.commit()?;
        self.scrub_wal()?;
        Ok(n)
    }

    /// Stores the vault's encrypted metadata (§7.1). It must be sealed
    /// under the highest accepted generation.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::KeyGenerationRollback`],
    /// [`Error::KeyGenerationNotAccepted`].
    pub fn set_vault_meta(&mut self, vault_id: &VaultId, meta: &VaultMetaCiphertext) -> Result<()> {
        let tx = self.write()?;
        let state = require_vault(&tx, vault_id)?;
        check_generation(state.highest_accepted_generation, meta.key_generation())?;
        tx.execute(
            "UPDATE vaults SET meta = ?2, meta_generation = ?3, updated_at = ?4 WHERE vault_id = ?1",
            params![
                vault_id.as_bytes(),
                meta.as_bytes(),
                meta.key_generation().get(),
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The vault's encrypted metadata, if set.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Corrupt`].
    pub fn vault_meta(&self, vault_id: &VaultId) -> Result<Option<VaultMetaCiphertext>> {
        let (meta, generation): (Option<Vec<u8>>, Option<i64>) = self
            .conn
            .query_row(
                "SELECT meta, meta_generation FROM vaults WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(Error::UnknownVault)?;
        let (Some(meta), Some(generation)) = (meta, generation) else {
            return Ok(None);
        };
        let meta = VaultMetaCiphertext::from_bytes(&meta).map_err(|_| Error::Corrupt)?;
        if meta.key_generation() != generation_from_sql(generation)? {
            return Err(Error::Corrupt);
        }
        Ok(Some(meta))
    }

    /// Deletes a vault with its keys, TK wrappings, records, cursor and
    /// attachments. Returns whether the vault existed.
    ///
    /// The vault's highest accepted generation is kept as its generation
    /// floor ([`Store::generation_floor`]), so the vault can only come back
    /// at that generation or a later one.
    ///
    /// # Errors
    /// [`Error::PendingLocalChange`] under [`PendingPolicy::Refuse`] if a
    /// record of the vault is in the outbox or an attachment is not yet
    /// uploaded; [`Error::Database`], [`Error::Io`].
    pub fn delete_vault(&mut self, vault_id: &VaultId, policy: PendingPolicy) -> Result<bool> {
        let dir = self.attachment_dir.clone();
        let tx = self.write()?;
        let Some(state) = read_vault(&tx, vault_id)? else {
            return Ok(false);
        };
        check_pending(&tx, policy, "vault_id = ?1", [vault_id.as_bytes()])?;
        let ids = attachments::ids_in_vault(&tx, vault_id)?;
        tx.execute(
            "INSERT INTO vault_generation_floors (vault_id, generation, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (vault_id) DO UPDATE SET
                 generation = max(generation, excluded.generation),
                 updated_at = excluded.updated_at",
            params![
                vault_id.as_bytes(),
                state.highest_accepted_generation.get(),
                now_ms()
            ],
        )?;
        tx.execute(
            "DELETE FROM vaults WHERE vault_id = ?1",
            [vault_id.as_bytes()],
        )?;
        tx.commit()?;
        attachments::remove_files(dir.as_deref(), &ids)?;
        self.scrub_wal()?;
        Ok(true)
    }

    /// Stores (or replaces) the wrapping of a hardware-protected vault's TK
    /// for one hardware key (§7.4). `method_id` is
    /// `SHA-256(credential_id)[..16]`; `params` holds this wrapping's own
    /// `prf_salt_hw`. TK wrappings live apart from the AK wrappings
    /// ([`Store::put_unlock_method`]), so the same hardware key can hold
    /// both without one replacing the other.
    ///
    /// # Errors
    /// [`Error::UnlockParamsTooLarge`], [`Error::UnknownVault`],
    /// [`Error::KeyWrapMismatch`] for a vault under AK.
    pub fn put_tk_wrapping(
        &mut self,
        vault_id: &VaultId,
        method_id: &[u8; 16],
        wrapped_tk: &WrappedKey,
        params: &[u8],
    ) -> Result<()> {
        if params.len() > MAX_UNLOCK_PARAMS_LEN {
            return Err(Error::UnlockParamsTooLarge);
        }
        let tx = self.write()?;
        if require_vault(&tx, vault_id)?.key_wrap != VaultKeyWrap::TotpKey {
            return Err(Error::KeyWrapMismatch);
        }
        tx.execute(
            "INSERT INTO tk_wrappings (vault_id, method_id, wrapped_tk, params, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (vault_id, method_id) DO UPDATE SET
                 wrapped_tk = excluded.wrapped_tk,
                 params = excluded.params,
                 updated_at = excluded.updated_at",
            params![
                vault_id.as_bytes(),
                method_id,
                wrapped_tk.as_bytes(),
                params,
                now_ms()
            ],
        )?;
        tx.commit()?;
        self.scrub_wal()
    }

    /// The TK wrapping of a vault for one hardware key, if stored.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn tk_wrapping(
        &self,
        vault_id: &VaultId,
        method_id: &[u8; 16],
    ) -> Result<Option<StoredTkWrapping>> {
        self.conn
            .query_row(
                "SELECT method_id, wrapped_tk, params, updated_at FROM tk_wrappings
                 WHERE vault_id = ?1 AND method_id = ?2",
                params![vault_id.as_bytes(), method_id],
                tk_row,
            )
            .optional()?
            .map(decode_tk)
            .transpose()
    }

    /// Every TK wrapping of a vault, ordered by `method_id`.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn tk_wrappings(&self, vault_id: &VaultId) -> Result<Vec<StoredTkWrapping>> {
        let mut stmt = self.conn.prepare(
            "SELECT method_id, wrapped_tk, params, updated_at FROM tk_wrappings
             WHERE vault_id = ?1 ORDER BY method_id",
        )?;
        let rows = stmt.query_map([vault_id.as_bytes()], tk_row)?;
        rows.map(|row| decode_tk(row?)).collect()
    }

    /// Deletes the TK wrapping for one hardware key, for example when the
    /// key is revoked. Returns whether a row was deleted.
    ///
    /// # Errors
    /// [`Error::LastTkWrapping`] if it is the vault's only TK wrapping:
    /// without one, the vault's keys are lost for good (§7.4).
    pub fn remove_tk_wrapping(&mut self, vault_id: &VaultId, method_id: &[u8; 16]) -> Result<bool> {
        let tx = self.write()?;
        let (exists, others): (bool, bool) = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM tk_wrappings WHERE vault_id = ?1 AND method_id = ?2),
                    EXISTS (SELECT 1 FROM tk_wrappings WHERE vault_id = ?1 AND method_id <> ?2)",
            params![vault_id.as_bytes(), method_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if !exists {
            return Ok(false);
        }
        if !others {
            return Err(Error::LastTkWrapping);
        }
        tx.execute(
            "DELETE FROM tk_wrappings WHERE vault_id = ?1 AND method_id = ?2",
            params![vault_id.as_bytes(), method_id],
        )?;
        tx.commit()?;
        self.scrub_wal()?;
        Ok(true)
    }

    /// The vault's sync cursor: the highest `server_seq` pulled, if any.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn sync_cursor(&self, vault_id: &VaultId) -> Result<Option<u64>> {
        let row: Option<i64> = self
            .conn
            .query_row(
                "SELECT server_seq FROM sync_cursors WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| r.get(0),
            )
            .optional()?;
        row.map(seq_from_sql).transpose()
    }

    /// Advances the vault's sync cursor. Setting the same value again is a
    /// no-op; a lower value is refused.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::StaleServerSeq`],
    /// [`Error::SeqOutOfRange`].
    pub fn advance_sync_cursor(&mut self, vault_id: &VaultId, server_seq: u64) -> Result<()> {
        let seq = seq_to_sql(server_seq)?;
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        let stored: Option<i64> = tx
            .query_row(
                "SELECT server_seq FROM sync_cursors WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(stored) = stored.map(seq_from_sql).transpose()?
            && stored > server_seq
        {
            return Err(Error::StaleServerSeq {
                stored,
                offered: server_seq,
            });
        }
        tx.execute(
            "INSERT INTO sync_cursors (vault_id, server_seq, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (vault_id) DO UPDATE SET
                 server_seq = excluded.server_seq,
                 updated_at = excluded.updated_at",
            params![vault_id.as_bytes(), seq, now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mosaic_core::{AccountId, AccountKey, VaultKey};

    pub(crate) struct Fixture {
        pub account: AccountId,
        pub ak: AccountKey,
        pub vault: VaultId,
    }

    pub(crate) fn fixture() -> Fixture {
        Fixture {
            account: AccountId::new_v7().unwrap(),
            ak: AccountKey::generate().unwrap(),
            vault: VaultId::new_v7().unwrap(),
        }
    }

    pub(crate) fn g(n: u32) -> KeyGeneration {
        KeyGeneration::new(n).unwrap()
    }

    impl Fixture {
        pub fn key(&self, generation: u32) -> (VaultKey, WrappedKey) {
            let vk = VaultKey::generate(g(generation)).unwrap();
            let wrapped = vk.wrap(&self.ak, &self.account, &self.vault).unwrap();
            (vk, wrapped)
        }
    }

    #[test]
    fn vault_lifecycle_and_generations() {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let (vk1, w1) = f.key(1);
        assert_eq!(s.vault(&f.vault).unwrap(), None);
        assert!(matches!(
            s.accept_vault_key(&f.vault, g(1), &w1),
            Err(Error::UnknownVault)
        ));
        let st = s.create_vault(&f.vault, g(1), &w1).unwrap();
        assert_eq!(st.current_generation, g(1));
        assert!(matches!(
            s.create_vault(&f.vault, g(1), &w1),
            Err(Error::VaultExists)
        ));
        let back = s
            .vault_key(&f.vault, g(1))
            .unwrap()
            .unwrap()
            .unwrap_vault_key(&f.ak, &f.account, &f.vault, g(1))
            .unwrap();
        assert_eq!(back, vk1);

        let (_, w3) = f.key(3);
        let st = s.accept_vault_key(&f.vault, g(3), &w3).unwrap();
        assert_eq!(st.highest_accepted_generation, g(3));
        assert_eq!(st.current_generation, g(3));
        assert_eq!(s.vault(&f.vault).unwrap(), Some(st));
        let (_, w2) = f.key(2);
        assert!(matches!(
            s.accept_vault_key(&f.vault, g(2), &w2),
            Err(Error::KeyGenerationRollback {
                highest_accepted: 3,
                offered: 2
            })
        ));
        // The held wrapping again is a no-op; a different key (or wrapping)
        // of a held generation is refused and changes nothing (ADR-0013).
        assert_eq!(s.accept_vault_key(&f.vault, g(3), &w3).unwrap(), st);
        let (_, w3b) = f.key(3);
        assert!(matches!(
            s.accept_vault_key(&f.vault, g(3), &w3b),
            Err(Error::VaultKeyConflict { generation: 3 })
        ));
        assert!(matches!(
            s.accept_vault_key(&f.vault, g(1), &w3b),
            Err(Error::KeyGenerationRollback {
                highest_accepted: 3,
                offered: 1
            })
        ));
        assert_eq!(s.vault_key(&f.vault, g(3)).unwrap(), Some(w3.clone()));
        // Re-wrapping a held key goes through `rewrap_vault_key`.
        s.rewrap_vault_key(&f.vault, g(3), &w3b).unwrap();
        assert_eq!(s.vault_key(&f.vault, g(3)).unwrap(), Some(w3b.clone()));
        assert_eq!(s.vault_key(&f.vault, g(2)).unwrap(), None);
        let keys: Vec<_> = s.vault_keys(&f.vault).unwrap();
        assert_eq!(keys, vec![(g(1), w1), (g(3), w3b.clone())]);
        assert_eq!(s.vaults().unwrap(), vec![st]);
        assert_eq!(s.integrity_check().unwrap().len(), 0);

        // A held key below the mark can be re-wrapped in place (for a §6.6
        // re-key), but no key is ever added below it.
        let w1b = vk1.wrap(&f.ak, &f.account, &f.vault).unwrap();
        s.rewrap_vault_key(&f.vault, g(1), &w1b).unwrap();
        assert_eq!(
            s.vault_key(&f.vault, g(1))
                .unwrap()
                .unwrap()
                .unwrap_vault_key(&f.ak, &f.account, &f.vault, g(1))
                .unwrap(),
            vk1
        );
        assert_eq!(s.vault_key(&f.vault, g(1)).unwrap(), Some(w1b.clone()));
        assert!(matches!(
            s.rewrap_vault_key(&f.vault, g(2), &w2),
            Err(Error::UnknownVaultKey)
        ));
        assert_eq!(s.vault_key(&f.vault, g(2)).unwrap(), None);
        assert!(matches!(
            s.rewrap_vault_key(&VaultId::new_v7().unwrap(), g(1), &w1b),
            Err(Error::UnknownVault)
        ));
        assert_eq!(s.vault(&f.vault).unwrap(), Some(st));

        // Pruning: never the current key; old keys go once unused.
        assert!(matches!(
            s.prune_vault_keys(&f.vault, g(4)),
            Err(Error::VaultKeyInUse)
        ));
        assert_eq!(s.prune_vault_keys(&f.vault, g(3)).unwrap(), 1);
        assert_eq!(s.prune_vault_keys(&f.vault, g(3)).unwrap(), 0);
        assert_eq!(s.vault_keys(&f.vault).unwrap().len(), 1);
        assert!(matches!(
            s.prune_vault_keys(&VaultId::new_v7().unwrap(), g(1)),
            Err(Error::UnknownVault)
        ));

        assert!(s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert!(!s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert_eq!(s.vault_keys(&f.vault).unwrap().len(), 0);
    }

    /// §7.1: deleting a vault keeps its anti-rollback mark, so a server
    /// cannot report the vault as removed and re-share an older generation.
    #[test]
    fn deleted_vault_keeps_its_generation_floor() {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let (_, w1) = f.key(1);
        let (_, w3) = f.key(3);
        assert_eq!(s.generation_floor(&f.vault).unwrap(), None);
        s.create_vault(&f.vault, g(1), &w1).unwrap();
        s.accept_vault_key(&f.vault, g(3), &w3).unwrap();
        assert!(s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert_eq!(s.generation_floor(&f.vault).unwrap(), Some(g(3)));

        for (n, w) in [(1, &w1), (2, &f.key(2).1)] {
            assert!(matches!(
                s.create_vault(&f.vault, g(n), w),
                Err(Error::KeyGenerationRollback {
                    highest_accepted: 3,
                    offered
                }) if offered == n
            ));
        }
        assert_eq!(s.vault(&f.vault).unwrap(), None);
        // Back at the floor generation, then deleted again at a higher one:
        // the floor follows, and never goes down.
        s.create_vault(&f.vault, g(3), &w3).unwrap();
        assert_eq!(s.integrity_check().unwrap().len(), 0);
        let (_, w5) = f.key(5);
        s.accept_vault_key(&f.vault, g(5), &w5).unwrap();
        assert!(s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert_eq!(s.generation_floor(&f.vault).unwrap(), Some(g(5)));
        s.create_vault(&f.vault, g(6), &f.key(6).1).unwrap();
        assert!(s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
        assert_eq!(s.generation_floor(&f.vault).unwrap(), Some(g(6)));
        // Other vaults are unaffected.
        let other = VaultId::new_v7().unwrap();
        assert_eq!(s.generation_floor(&other).unwrap(), None);
        s.create_vault(&other, g(1), &w1).unwrap();
    }

    /// §7.4: a hardware-protected TOTP vault's TK wrappings live apart
    /// from the AK wrappings, so the same hardware key can hold both.
    #[test]
    fn totp_vault_and_tk_wrappings() {
        use mosaic_core::{
            AccountSalt, Argon2Profile, Kek, PasswordParams, SecretBytes, UnlockMethod,
        };

        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let params = PasswordParams {
            account_id: f.account,
            salt: AccountSalt::generate().unwrap(),
            profile: Argon2Profile::Baseline,
        };
        let kek = Kek::from_secret(SecretBytes::random().unwrap());
        let password =
            f.ak.wrap(&kek, &f.account, &UnlockMethod::Password)
                .unwrap();
        s.init_account(&params, &password).unwrap();
        // The §6.3 AK wrapping of one hardware key.
        let hw = UnlockMethod::hardware_key(b"credential");
        let UnlockMethod::HardwareKey(hw_id) = hw else {
            unreachable!()
        };
        let ak_wrapping = f.ak.wrap(&kek, &f.account, &hw).unwrap();
        s.put_unlock_method(&hw, &ak_wrapping, &[0xA1; 32]).unwrap();

        // A normal vault takes no TK wrapping.
        let (_, w1) = f.key(1);
        s.create_vault(&f.vault, g(1), &w1).unwrap();
        assert_eq!(
            s.vault(&f.vault).unwrap().unwrap().key_wrap,
            VaultKeyWrap::AccountKey
        );
        // The bytes are opaque to the store; any wrapped key will do.
        let (_, tk_wrapping) = f.key(1);
        assert!(matches!(
            s.put_tk_wrapping(&f.vault, &hw_id, &tk_wrapping, &[0xB2; 32]),
            Err(Error::KeyWrapMismatch)
        ));
        assert!(matches!(
            s.put_tk_wrapping(&VaultId::new_v7().unwrap(), &hw_id, &tk_wrapping, &[]),
            Err(Error::UnknownVault)
        ));

        let totp = VaultId::new_v7().unwrap();
        let (_, vk_under_tk) = f.key(1);
        let st = s
            .create_vault_with(&totp, g(1), VaultKeyWrap::TotpKey, &vk_under_tk)
            .unwrap();
        assert_eq!(st.key_wrap, VaultKeyWrap::TotpKey);
        assert_eq!(s.vault(&totp).unwrap(), Some(st));
        assert!(s.vaults().unwrap().contains(&st));
        s.put_tk_wrapping(&totp, &hw_id, &tk_wrapping, &[0xB2; 32])
            .unwrap();
        assert!(matches!(
            s.put_tk_wrapping(&totp, &hw_id, &tk_wrapping, &[0; MAX_UNLOCK_PARAMS_LEN + 1]),
            Err(Error::UnlockParamsTooLarge)
        ));

        // The same hardware key's AK wrapping is untouched.
        let stored = s.unlock_method(&hw).unwrap().unwrap();
        assert_eq!(stored.wrapped_account_key, ak_wrapping);
        assert_eq!(stored.params, [0xA1; 32]);
        let tk = s.tk_wrapping(&totp, &hw_id).unwrap().unwrap();
        assert_eq!(tk.method_id, hw_id);
        assert_eq!(tk.wrapped_tk, tk_wrapping);
        assert_eq!(tk.params, [0xB2; 32]);
        assert_eq!(s.tk_wrapping(&f.vault, &hw_id).unwrap(), None);

        // The last wrapping cannot be removed; with a second key it can.
        assert!(matches!(
            s.remove_tk_wrapping(&totp, &hw_id),
            Err(Error::LastTkWrapping)
        ));
        let second = [0x22; 16];
        s.put_tk_wrapping(&totp, &second, &w1, &[]).unwrap();
        let all = s.tk_wrappings(&totp).unwrap();
        let mut expected = vec![second, hw_id];
        expected.sort_unstable();
        assert_eq!(
            all.iter().map(|t| t.method_id).collect::<Vec<_>>(),
            expected
        );
        assert!(s.remove_tk_wrapping(&totp, &hw_id).unwrap());
        assert!(!s.remove_tk_wrapping(&totp, &hw_id).unwrap());
        assert_eq!(s.tk_wrappings(&totp).unwrap().len(), 1);
        assert_eq!(s.unlock_method(&hw).unwrap().unwrap(), stored);

        // Corrupt rows are reported.
        s.conn
            .execute("UPDATE tk_wrappings SET wrapped_tk = zeroblob(74)", [])
            .unwrap();
        assert!(matches!(s.tk_wrappings(&totp), Err(Error::Corrupt)));
        assert!(s.delete_vault(&totp, PendingPolicy::Refuse).unwrap());
        assert_eq!(s.tk_wrappings(&totp).unwrap().len(), 0);
        assert!(matches!(
            decode_vault((vec![1; 16], 3, 1, 1)),
            Err(Error::Corrupt)
        ));
        assert_eq!(VaultKeyWrap::from_code(0), None);
        assert_eq!(VaultKeyWrap::TotpKey.code(), 2);
    }

    #[test]
    fn vault_meta_follows_the_accepted_generation() {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let (vk1, w1) = f.key(1);
        assert!(matches!(s.vault_meta(&f.vault), Err(Error::UnknownVault)));
        s.create_vault(&f.vault, g(1), &w1).unwrap();
        assert_eq!(s.vault_meta(&f.vault).unwrap(), None);
        let meta1 = VaultMetaCiphertext::seal(&vk1, &f.vault, b"name").unwrap();
        s.set_vault_meta(&f.vault, &meta1).unwrap();
        assert_eq!(s.vault_meta(&f.vault).unwrap(), Some(meta1.clone()));

        let (vk2, w2) = f.key(2);
        s.accept_vault_key(&f.vault, g(2), &w2).unwrap();
        // Old metadata stays readable, but cannot be written again, and the
        // old key cannot be pruned while the metadata needs it.
        assert_eq!(s.vault_meta(&f.vault).unwrap(), Some(meta1.clone()));
        assert!(matches!(
            s.set_vault_meta(&f.vault, &meta1),
            Err(Error::KeyGenerationRollback { .. })
        ));
        assert!(matches!(
            s.prune_vault_keys(&f.vault, g(2)),
            Err(Error::VaultKeyInUse)
        ));
        let (vk3, _) = f.key(3);
        let meta3 = VaultMetaCiphertext::seal(&vk3, &f.vault, b"name").unwrap();
        assert!(matches!(
            s.set_vault_meta(&f.vault, &meta3),
            Err(Error::KeyGenerationNotAccepted { .. })
        ));
        let meta2 = VaultMetaCiphertext::seal(&vk2, &f.vault, b"name").unwrap();
        s.set_vault_meta(&f.vault, &meta2).unwrap();
        assert_eq!(s.prune_vault_keys(&f.vault, g(2)).unwrap(), 1);
        let opened = s
            .vault_meta(&f.vault)
            .unwrap()
            .unwrap()
            .open(&vk2, &f.vault)
            .unwrap();
        assert_eq!(opened.expose_secret(), b"name");

        // A stored meta generation that disagrees with the header is corrupt
        // (written by an outside writer that dropped the trigger).
        s.conn
            .execute_batch("DROP TRIGGER vaults_meta_generation;")
            .unwrap();
        s.conn
            .execute("UPDATE vaults SET meta_generation = 1", [])
            .unwrap();
        assert!(matches!(s.vault_meta(&f.vault), Err(Error::Corrupt)));
        s.conn
            .execute("UPDATE vaults SET meta_generation = 2, meta = x'0102'", [])
            .unwrap();
        assert!(matches!(s.vault_meta(&f.vault), Err(Error::Corrupt)));
    }

    #[test]
    fn sync_cursor_is_monotonic() {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        assert!(matches!(
            s.advance_sync_cursor(&f.vault, 1),
            Err(Error::UnknownVault)
        ));
        let (_, w1) = f.key(1);
        s.create_vault(&f.vault, g(1), &w1).unwrap();
        assert_eq!(s.sync_cursor(&f.vault).unwrap(), None);
        s.advance_sync_cursor(&f.vault, 0).unwrap();
        s.advance_sync_cursor(&f.vault, 10).unwrap();
        s.advance_sync_cursor(&f.vault, 10).unwrap();
        assert_eq!(s.sync_cursor(&f.vault).unwrap(), Some(10));
        assert!(matches!(
            s.advance_sync_cursor(&f.vault, 9),
            Err(Error::StaleServerSeq {
                stored: 10,
                offered: 9
            })
        ));
        assert!(matches!(
            s.advance_sync_cursor(&f.vault, u64::MAX),
            Err(Error::SeqOutOfRange)
        ));
        assert_eq!(s.sync_cursor(&f.vault).unwrap(), Some(10));
    }

    #[test]
    fn integrity_check_reports_broken_invariants() {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let (_, w1) = f.key(1);
        s.create_vault(&f.vault, g(1), &w1).unwrap();
        s.conn.execute("DELETE FROM vault_keys", []).unwrap();
        let problems = s.integrity_check().unwrap();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("no wrapped key"));
        assert!(problems[0].contains(&hex(f.vault.as_bytes())));
        // A floor above the live vault's mark (written from outside).
        s.conn
            .execute(
                "INSERT INTO vault_generation_floors (vault_id, generation, updated_at)
                 VALUES (?1, 5, 0)",
                [f.vault.as_bytes()],
            )
            .unwrap();
        let problems = s.integrity_check().unwrap();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[1].contains("below its floor"));
    }

    fn hex(b: &[u8]) -> String {
        use core::fmt::Write as _;
        b.iter().fold(String::new(), |mut s, x| {
            let _ = write!(s, "{x:02x}");
            s
        })
    }
}
