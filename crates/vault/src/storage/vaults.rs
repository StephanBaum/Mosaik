//! Vaults, their wrapped keys per generation, encrypted metadata and sync
//! cursors (CRYPTO_SPEC §7.1, §10).

use mosaic_core::{KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
use super::{
    Store, attachments, check_generation, generation_from_sql, now_ms, seq_from_sql, seq_to_sql,
};

/// The key-generation state of a vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultState {
    /// The vault.
    pub vault_id: VaultId,
    /// The generation of the newest vault key held. New items, tombstones
    /// and metadata are sealed under it.
    pub current_generation: KeyGeneration,
    /// The anti-rollback high-water mark (§7.1). It never decreases.
    /// Wrapped keys below it are rejected, and records and metadata are
    /// accepted only at exactly this generation.
    pub highest_accepted_generation: KeyGeneration,
}

pub(super) fn read_vault(conn: &Connection, vault_id: &VaultId) -> Result<Option<VaultState>> {
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT current_generation, highest_accepted_generation FROM vaults
             WHERE vault_id = ?1",
            [vault_id.as_bytes()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(current, highest)| {
        Ok(VaultState {
            vault_id: *vault_id,
            current_generation: generation_from_sql(current)?,
            highest_accepted_generation: generation_from_sql(highest)?,
        })
    })
    .transpose()
}

/// The vault's state, or [`Error::UnknownVault`].
pub(super) fn require_vault(conn: &Connection, vault_id: &VaultId) -> Result<VaultState> {
    read_vault(conn, vault_id)?.ok_or(Error::UnknownVault)
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

impl Store {
    /// Adds a vault with its first wrapped vault key. `generation` is
    /// normally [`KeyGeneration::FIRST`] for a new vault, or the vault's
    /// current generation when a device first learns about it.
    ///
    /// # Errors
    /// [`Error::VaultExists`].
    pub fn create_vault(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        wrapped_vault_key: &WrappedKey,
    ) -> Result<VaultState> {
        let tx = self.write()?;
        if read_vault(&tx, vault_id)?.is_some() {
            return Err(Error::VaultExists);
        }
        tx.execute(
            "INSERT INTO vaults (vault_id, current_generation, highest_accepted_generation, updated_at)
             VALUES (?1, ?2, ?2, ?3)",
            params![vault_id.as_bytes(), generation.get(), now_ms()],
        )?;
        upsert_key(&tx, vault_id, generation, wrapped_vault_key)?;
        tx.commit()?;
        Ok(VaultState {
            vault_id: *vault_id,
            current_generation: generation,
            highest_accepted_generation: generation,
        })
    }

    /// Records an accepted wrapped vault key: after a rotation (§8.3), or a
    /// re-wrap of the current generation. The caller has unwrapped it with
    /// AK, which authenticates the generation in the AAD.
    ///
    /// A generation above the highest accepted one raises both the current
    /// and the highest accepted generation to it. The highest accepted
    /// generation never decreases.
    ///
    /// # Errors
    /// [`Error::UnknownVault`]; [`Error::KeyGenerationRollback`] if
    /// `generation` is below the highest accepted generation.
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
        Ok(state)
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
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, current_generation, highest_accepted_generation FROM vaults
             ORDER BY vault_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get(1)?, r.get(2)?)))?;
        rows.map(|row| {
            let (id, current, highest) = row?;
            Ok(VaultState {
                vault_id: super::vault_id_from_sql(&id)?,
                current_generation: generation_from_sql(current)?,
                highest_accepted_generation: generation_from_sql(highest)?,
            })
        })
        .collect()
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

    /// Deletes a vault with its keys, records, cursor and attachments.
    /// Returns whether the vault existed.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Io`].
    pub fn delete_vault(&mut self, vault_id: &VaultId) -> Result<bool> {
        let dir = self.attachment_dir.clone();
        let tx = self.write()?;
        let ids = attachments::ids_in_vault(&tx, vault_id)?;
        let n = tx.execute(
            "DELETE FROM vaults WHERE vault_id = ?1",
            [vault_id.as_bytes()],
        )?;
        tx.commit()?;
        attachments::remove_files(dir.as_deref(), &ids)?;
        Ok(n > 0)
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
        // Re-wrapping the accepted generation is allowed.
        let (_, w3b) = f.key(3);
        s.accept_vault_key(&f.vault, g(3), &w3b).unwrap();
        assert_eq!(s.vault_key(&f.vault, g(3)).unwrap(), Some(w3b.clone()));
        assert_eq!(s.vault_key(&f.vault, g(2)).unwrap(), None);
        let keys: Vec<_> = s.vault_keys(&f.vault).unwrap();
        assert_eq!(keys, vec![(g(1), w1), (g(3), w3b)]);
        assert_eq!(s.vaults().unwrap(), vec![st]);
        assert_eq!(s.integrity_check().unwrap().len(), 0);

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

        assert!(s.delete_vault(&f.vault).unwrap());
        assert!(!s.delete_vault(&f.vault).unwrap());
        assert_eq!(s.vault_keys(&f.vault).unwrap().len(), 0);
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
    }

    fn hex(b: &[u8]) -> String {
        use core::fmt::Write as _;
        b.iter().fold(String::new(), |mut s, x| {
            let _ = write!(s, "{x:02x}");
            s
        })
    }
}
