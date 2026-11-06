//! Local sync state (schema v3, CRYPTO_SPEC §10, §11, ADR-0045): this
//! install's ID, the backend choice, pull cursors, device-local sequence
//! blocks, the checkpoint counter, the upload queue of vault keys and
//! metadata, and the attachment deletions still to send.
//!
//! None of it is derived from item plaintext (§11): IDs, flags, counters and
//! the user's own backend settings are cleartext local metadata.

use std::path::PathBuf;

use mosaic_core::{
    AttachmentId, InstallId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey,
};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
use super::records::next_outbox_seq;
use super::vaults::require_vault;
use super::{
    Store, VaultKeyWrap, check_generation, generation_from_sql, now_ms, seq_from_sql, seq_to_sql,
    vault_id_from_sql,
};

/// How many device-local sequence numbers one reservation takes at least
/// ([`Store::allocate_local_seqs`]).
pub const LOCAL_SEQ_BLOCK: u64 = 1024;

/// The longest pull cursor the store keeps, in bytes.
const MAX_CURSOR_LEN: usize = 65_536;

/// Who generated a vault key, which decides whether it waits for upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyOrigin {
    /// Generated on this install (a new vault or a rotation): queued for
    /// upload, and this install is the key's device in the canonical-winner
    /// rule (CRYPTO_SPEC §10.1).
    Local,
    /// Delivered by the backend: nothing to upload.
    Remote,
}

/// The `vault_keys` flags of a new row.
#[derive(Clone, Copy, Debug)]
pub(super) struct KeyFlags {
    pub upload_pending: bool,
    pub local_origin: bool,
}

impl KeyFlags {
    /// A key that is neither queued nor generated here (the schema default).
    pub(super) const NEUTRAL: Self = Self {
        upload_pending: false,
        local_origin: false,
    };

    pub(super) const fn of(origin: KeyOrigin) -> Self {
        match origin {
            KeyOrigin::Local => Self {
                upload_pending: true,
                local_origin: true,
            },
            KeyOrigin::Remote => Self::NEUTRAL,
        }
    }
}

/// Which backend a store syncs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    /// The Mosaic server.
    Hosted,
    /// A user-chosen folder.
    Folder,
}

impl SyncKind {
    const fn code(self) -> i64 {
        match self {
            Self::Hosted => 1,
            Self::Folder => 2,
        }
    }

    const fn from_code(code: i64) -> Option<Self> {
        match code {
            1 => Some(Self::Hosted),
            2 => Some(Self::Folder),
            _ => None,
        }
    }
}

/// The backend choice of this store (cleartext local metadata, §11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConfig {
    /// The backend kind.
    pub kind: SyncKind,
    /// Hosted: the server's base URL. Folder: `None`.
    pub server_url: Option<String>,
    /// Hosted: the sync account's email, if known. Folder: `None`.
    pub email: Option<String>,
    /// Folder: the chosen directory. Hosted: `None`.
    pub folder_path: Option<PathBuf>,
    /// This install's name in the device list (1 to 64 characters).
    pub device_name: String,
}

impl SyncConfig {
    /// The same checks as the schema's `CHECK`s, so a bad value is a typed
    /// error instead of a database error.
    fn validate(&self) -> Result<()> {
        let fits =
            |s: &Option<String>, max: usize| s.as_ref().is_none_or(|s| s.chars().count() <= max);
        let name = self.device_name.chars().count();
        let path = self
            .folder_path
            .as_ref()
            .map(|p| p.to_str().ok_or(Error::SyncDataInvalid))
            .transpose()?;
        let shape = match self.kind {
            SyncKind::Hosted => self.server_url.is_some() && self.folder_path.is_none(),
            SyncKind::Folder => {
                self.folder_path.is_some() && self.server_url.is_none() && self.email.is_none()
            }
        };
        let ok = shape
            && (1..=64).contains(&name)
            && fits(&self.server_url, 2048)
            && fits(&self.email, 320)
            && path.is_none_or(|p| p.chars().count() <= 4096);
        if ok {
            Ok(())
        } else {
            Err(Error::SyncDataInvalid)
        }
    }
}

/// The sync flags of one vault (schema v3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultSyncState {
    /// The vault exists on the backend.
    pub remote_created: bool,
    /// The stored metadata has not been uploaded.
    pub meta_upload_pending: bool,
    /// The backend's sequence number of the metadata last seen, the base for
    /// the next metadata upload.
    pub meta_server_seq: Option<u64>,
}

/// Creates the install ID if the store has none yet.
pub(super) fn ensure_install_identity(conn: &Connection) -> Result<()> {
    let exists: bool =
        conn.query_row("SELECT EXISTS (SELECT 1 FROM install_identity)", [], |r| {
            r.get(0)
        })?;
    if !exists {
        let id = InstallId::new_v7().map_err(Error::Format)?;
        conn.execute(
            "INSERT OR IGNORE INTO install_identity (id, install_id, created_at)
             VALUES (1, ?1, ?2)",
            params![id.as_bytes(), now_ms()],
        )?;
    }
    Ok(())
}

/// Records the uploaded attachments selected by `filter` (an SQL condition
/// on `attachments`) as deleted, before their rows go: the backend still
/// holds them. An attachment that was never uploaded is not recorded.
pub(super) fn record_deletions(
    conn: &Connection,
    filter: &str,
    args: impl rusqlite::Params,
) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO attachment_deletions (attachment_id, vault_id, deleted_at)
             SELECT attachment_id, vault_id, {} FROM attachments
             WHERE upload_pending = 0 AND {filter}",
            now_ms()
        ),
        args,
    )?;
    Ok(())
}

impl Store {
    /// This install's stable ID (CRYPTO_SPEC §10), created when the store
    /// is first opened with schema v3. It names the install in checkpoints,
    /// folder paths and the server's device list; it is not the writer ID.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn install_id(&self) -> Result<InstallId> {
        let raw: Vec<u8> = self
            .conn
            .query_row(
                "SELECT install_id FROM install_identity WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::Corrupt)?;
        InstallId::from_slice(&raw).map_err(|_| Error::Corrupt)
    }

    /// The backend choice, if sync is set up.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn sync_config(&self) -> Result<Option<SyncConfig>> {
        let row = self
            .conn
            .query_row(
                "SELECT kind, server_url, email, folder_path, device_name FROM sync_config
                 WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(kind, server_url, email, folder_path, device_name)| {
            Ok(SyncConfig {
                kind: SyncKind::from_code(kind).ok_or(Error::Corrupt)?,
                server_url,
                email,
                folder_path: folder_path.map(PathBuf::from),
                device_name,
            })
        })
        .transpose()
    }

    /// Sets the backend choice, or clears it with `None`. The cursors and
    /// queues are left alone: [`Store::reset_sync_state`] is the caller's
    /// next step when the backend changes.
    ///
    /// # Errors
    /// [`Error::SyncDataInvalid`] for a value over its limit, a kind with
    /// the wrong fields, or a folder path that is not UTF-8.
    pub fn set_sync_config(&mut self, config: Option<&SyncConfig>) -> Result<()> {
        if let Some(c) = config {
            c.validate()?;
        }
        let tx = self.write()?;
        match config {
            None => {
                tx.execute("DELETE FROM sync_config WHERE id = 1", [])?;
            }
            Some(c) => {
                let folder = c.folder_path.as_deref().and_then(std::path::Path::to_str);
                tx.execute(
                    "INSERT INTO sync_config (id, kind, server_url, email, folder_path,
                                              device_name, updated_at)
                     VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT (id) DO UPDATE SET
                         kind = excluded.kind, server_url = excluded.server_url,
                         email = excluded.email, folder_path = excluded.folder_path,
                         device_name = excluded.device_name, updated_at = excluded.updated_at",
                    params![
                        c.kind.code(),
                        c.server_url,
                        c.email,
                        folder,
                        c.device_name,
                        now_ms()
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The opaque pull cursor of a vault (at most 64 KiB), if one is stored.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn sync_cursor_blob(&self, vault_id: &VaultId) -> Result<Option<Vec<u8>>> {
        Ok(self
            .conn
            .query_row(
                "SELECT cursor FROM sync_cursor_blobs WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Stores the opaque pull cursor of a vault.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::SyncDataInvalid`] above 64 KiB.
    pub fn set_sync_cursor_blob(&mut self, vault_id: &VaultId, cursor: &[u8]) -> Result<()> {
        if cursor.len() > MAX_CURSOR_LEN {
            return Err(Error::SyncDataInvalid);
        }
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        tx.execute(
            "INSERT INTO sync_cursor_blobs (vault_id, cursor, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (vault_id) DO UPDATE SET
                 cursor = excluded.cursor, updated_at = excluded.updated_at",
            params![vault_id.as_bytes(), cursor, now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Reserves `n` consecutive device-local sequence numbers for a vault
    /// (for backends without a server sequence) and returns the first.
    ///
    /// The store reserves blocks of at least [`LOCAL_SEQ_BLOCK`]: the first
    /// number of the next block is written before any number of the block is
    /// handed out, so a crash skips numbers and never reuses one. Numbers
    /// start at 1.
    ///
    /// # Errors
    /// [`Error::SyncDataInvalid`] for `n == 0`, [`Error::UnknownVault`],
    /// [`Error::SeqOutOfRange`].
    pub fn allocate_local_seqs(&mut self, vault_id: &VaultId, n: u32) -> Result<u64> {
        if n == 0 {
            return Err(Error::SyncDataInvalid);
        }
        let n = u64::from(n);
        if let Some((next, end)) = self.local_seq.get_mut(vault_id)
            && end.saturating_sub(*next) >= n
        {
            let first = *next;
            *next = next.saturating_add(n);
            return Ok(first);
        }
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        let stored: Option<i64> = tx
            .query_row(
                "SELECT next_seq FROM sync_local_seq WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| r.get(0),
            )
            .optional()?;
        let start = stored.map(seq_from_sql).transpose()?.unwrap_or(1);
        let end = start
            .checked_add(n.max(LOCAL_SEQ_BLOCK))
            .ok_or(Error::SeqOutOfRange)?;
        tx.execute(
            "INSERT INTO sync_local_seq (vault_id, next_seq) VALUES (?1, ?2)
             ON CONFLICT (vault_id) DO UPDATE SET next_seq = excluded.next_seq",
            params![vault_id.as_bytes(), seq_to_sql(end)?],
        )?;
        tx.commit()?;
        self.local_seq
            .insert(*vault_id, (start.saturating_add(n), end));
        Ok(start)
    }

    /// The checkpoint counter (CRYPTO_SPEC §10): the highest value used.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn checkpoint_counter(&self) -> Result<u64> {
        let counter: i64 = self
            .conn
            .query_row(
                "SELECT counter FROM sync_checkpoint WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::Corrupt)?;
        seq_from_sql(counter)
    }

    /// Persists the checkpoint counter plus one and returns it. The counter
    /// never decreases (a trigger enforces it).
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn next_checkpoint_counter(&mut self) -> Result<u64> {
        let counter: i64 = self
            .conn
            .query_row(
                "UPDATE sync_checkpoint SET counter = counter + 1 WHERE id = 1
                 RETURNING counter",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::Corrupt)?;
        seq_from_sql(counter)
    }

    /// The attachments deleted here after they had been uploaded, oldest
    /// first: the backend still has to be told.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn pending_attachment_deletions(&self) -> Result<Vec<(VaultId, AttachmentId)>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, attachment_id FROM attachment_deletions
             ORDER BY deleted_at, attachment_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|row| {
            let (vault, attachment) = row?;
            Ok((
                vault_id_from_sql(&vault)?,
                AttachmentId::from_slice(&attachment).map_err(|_| Error::Corrupt)?,
            ))
        })
        .collect()
    }

    /// Forgets a pending attachment deletion once the backend has it.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_attachment_deletion_sent(&mut self, attachment_id: &AttachmentId) -> Result<()> {
        self.conn.execute(
            "DELETE FROM attachment_deletions WHERE attachment_id = ?1",
            [attachment_id.as_bytes()],
        )?;
        Ok(())
    }

    /// The sync flags of a vault.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Corrupt`].
    pub fn vault_sync_state(&self, vault_id: &VaultId) -> Result<VaultSyncState> {
        let (created, pending, seq): (bool, bool, Option<i64>) = self
            .conn
            .query_row(
                "SELECT remote_created, meta_upload_pending, meta_server_seq FROM vaults
                 WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or(Error::UnknownVault)?;
        Ok(VaultSyncState {
            remote_created: created,
            meta_upload_pending: pending,
            meta_server_seq: seq.map(seq_from_sql).transpose()?,
        })
    }

    /// The wrapped keys of a vault that wait for upload, oldest generation
    /// first.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn pending_vault_keys(
        &self,
        vault_id: &VaultId,
    ) -> Result<Vec<(KeyGeneration, WrappedKey)>> {
        let mut stmt = self.conn.prepare(
            "SELECT generation, wrapped_vk FROM vault_keys
             WHERE vault_id = ?1 AND upload_pending = 1 ORDER BY generation",
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

    /// Whether this install generated the vault key of `generation`, or
    /// `None` if no such key is held.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn vault_key_origin(
        &self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<Option<KeyOrigin>> {
        let local: Option<bool> = self
            .conn
            .query_row(
                "SELECT local_origin FROM vault_keys WHERE vault_id = ?1 AND generation = ?2",
                params![vault_id.as_bytes(), generation.get()],
                |r| r.get(0),
            )
            .optional()?;
        Ok(local.map(|l| {
            if l {
                KeyOrigin::Local
            } else {
                KeyOrigin::Remote
            }
        }))
    }

    /// Clears the upload flag of a vault key. Returns whether it was set.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_vault_key_uploaded(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE vault_keys SET upload_pending = 0
             WHERE vault_id = ?1 AND generation = ?2 AND upload_pending = 1",
            params![vault_id.as_bytes(), generation.get()],
        )?;
        Ok(n > 0)
    }

    /// Records that the backend created the vault with its key of
    /// `generation` and, if `meta` is given, that metadata: the vault is
    /// marked as existing there, the upload flags of its keys up to
    /// `generation` are cleared, and the metadata flag is cleared only if the
    /// stored metadata is still exactly `meta`.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Database`].
    pub fn mark_vault_created(
        &mut self,
        vault_id: &VaultId,
        generation: KeyGeneration,
        meta: Option<&VaultMetaCiphertext>,
    ) -> Result<()> {
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        tx.execute(
            "UPDATE vaults SET remote_created = 1 WHERE vault_id = ?1",
            [vault_id.as_bytes()],
        )?;
        tx.execute(
            "UPDATE vault_keys SET upload_pending = 0
             WHERE vault_id = ?1 AND generation <= ?2",
            params![vault_id.as_bytes(), generation.get()],
        )?;
        if let Some(meta) = meta {
            tx.execute(
                "UPDATE vaults SET meta_upload_pending = 0 WHERE vault_id = ?1 AND meta = ?2",
                params![vault_id.as_bytes(), meta.as_bytes()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Stores a vault's metadata like [`Store::set_vault_meta`] and sets its
    /// upload flag to `queued` in the same transaction. A hardware-protected vault
    /// is never queued.
    ///
    /// # Errors
    /// As [`Store::set_vault_meta`].
    pub fn set_vault_meta_queued(
        &mut self,
        vault_id: &VaultId,
        meta: &VaultMetaCiphertext,
        queued: bool,
    ) -> Result<()> {
        let tx = self.write()?;
        let state = require_vault(&tx, vault_id)?;
        check_generation(state.highest_accepted_generation, meta.key_generation())?;
        let queued = queued && state.key_wrap == VaultKeyWrap::AccountKey;
        tx.execute(
            "UPDATE vaults SET meta = ?2, meta_generation = ?3, meta_upload_pending = ?4,
                 updated_at = ?5
             WHERE vault_id = ?1",
            params![
                vault_id.as_bytes(),
                meta.as_bytes(),
                meta.key_generation().get(),
                queued,
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Records that the backend stored `uploaded` as the vault's metadata at
    /// `seq`. The upload flag is cleared only if the stored metadata is still
    /// exactly `uploaded` (an edit made meanwhile stays queued). The
    /// sequence number is kept if it is not below the stored one. Returns
    /// whether the flag was cleared.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::SeqOutOfRange`].
    pub fn mark_vault_meta_uploaded(
        &mut self,
        vault_id: &VaultId,
        uploaded: &VaultMetaCiphertext,
        seq: u64,
    ) -> Result<bool> {
        let seq = seq_to_sql(seq)?;
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        let same: bool = tx.query_row(
            "SELECT meta = ?2 FROM vaults WHERE vault_id = ?1",
            params![vault_id.as_bytes(), uploaded.as_bytes()],
            |r| Ok(r.get::<_, Option<bool>>(0)?.unwrap_or(false)),
        )?;
        tx.execute(
            "UPDATE vaults SET
                 meta_server_seq = CASE WHEN meta_server_seq IS NULL OR meta_server_seq <= ?2
                                        THEN ?2 ELSE meta_server_seq END,
                 meta_upload_pending = CASE WHEN ?3 THEN 0 ELSE meta_upload_pending END
             WHERE vault_id = ?1",
            params![vault_id.as_bytes(), seq, same],
        )?;
        tx.commit()?;
        Ok(same)
    }

    /// Records the backend's sequence number of the vault's metadata when it
    /// is not below the stored one.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::SeqOutOfRange`].
    pub fn note_vault_meta_seq(&mut self, vault_id: &VaultId, seq: u64) -> Result<()> {
        let seq = seq_to_sql(seq)?;
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        tx.execute(
            "UPDATE vaults SET meta_server_seq = ?2
             WHERE vault_id = ?1 AND (meta_server_seq IS NULL OR meta_server_seq < ?2)",
            params![vault_id.as_bytes(), seq],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Prepares the store for a (new) backend, in one transaction: every
    /// record of a vault under AK loses its `server_seq` and is queued for
    /// upload; the vault's current key, its metadata and every attachment are
    /// queued; the vaults count as not created on the backend; the cursors,
    /// the local sequence blocks and the pending attachment deletions are
    /// cleared. Hardware-protected vaults (they never sync) are left alone.
    /// The checkpoint counter and the install ID are kept.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::SeqOutOfRange`].
    pub fn reset_sync_state(&mut self) -> Result<()> {
        const AK_VAULTS: &str = "SELECT vault_id FROM vaults WHERE key_wrap = 1";
        let tx = self.write()?;
        let unqueued: Vec<(Vec<u8>, Vec<u8>)> = {
            let mut stmt = tx.prepare(&format!(
                "SELECT vault_id, record_id FROM records
                 WHERE outbox_seq IS NULL AND vault_id IN ({AK_VAULTS})
                 ORDER BY vault_id, record_id"
            ))?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (vault, record) in unqueued {
            let outbox = next_outbox_seq(&tx)?;
            tx.execute(
                "UPDATE records SET outbox_seq = ?3 WHERE vault_id = ?1 AND record_id = ?2",
                params![vault, record, seq_to_sql(outbox.get())?],
            )?;
        }
        for sql in [
            format!("UPDATE records SET server_seq = NULL WHERE vault_id IN ({AK_VAULTS})"),
            "UPDATE vault_keys SET upload_pending = 1
             WHERE vault_id IN (SELECT vault_id FROM vaults WHERE key_wrap = 1)
               AND generation = (SELECT current_generation FROM vaults v
                                 WHERE v.vault_id = vault_keys.vault_id)"
                .to_owned(),
            "UPDATE vaults SET remote_created = 0, meta_server_seq = NULL,
                 meta_upload_pending = CASE WHEN meta IS NULL THEN 0 ELSE 1 END
             WHERE key_wrap = 1"
                .to_owned(),
            format!("UPDATE attachments SET upload_pending = 1 WHERE vault_id IN ({AK_VAULTS})"),
            "DELETE FROM sync_cursor_blobs".to_owned(),
            "DELETE FROM sync_local_seq".to_owned(),
            "DELETE FROM sync_cursors".to_owned(),
            "DELETE FROM attachment_deletions".to_owned(),
        ] {
            tx.execute(&sql, [])?;
        }
        tx.commit()?;
        self.local_seq.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::{ItemCiphertext, ItemId, VaultKey};

    use super::super::records::{Origin, RecordBody};
    use super::super::vaults::tests::{fixture, g};
    use super::*;
    use crate::storage::PendingPolicy;

    fn hosted() -> SyncConfig {
        SyncConfig {
            kind: SyncKind::Hosted,
            server_url: Some("https://sync.example".to_owned()),
            email: Some("a@b.example".to_owned()),
            folder_path: None,
            device_name: "Laptop".to_owned(),
        }
    }

    #[test]
    fn install_id_is_created_once_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.db");
        let s = Store::open(&path).unwrap();
        let id = s.install_id().unwrap();
        s.close().unwrap();
        let s = Store::open(&path).unwrap();
        assert_eq!(s.install_id().unwrap(), id);
        drop(s);
        let other = tempfile::tempdir().unwrap();
        let t = Store::open(other.path().join("v.db")).unwrap();
        assert_ne!(t.install_id().unwrap(), id, "distinct per store");
    }

    #[test]
    fn sync_config_round_trips_and_validates() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.sync_config().unwrap(), None);
        s.set_sync_config(Some(&hosted())).unwrap();
        assert_eq!(s.sync_config().unwrap(), Some(hosted()));
        let folder = SyncConfig {
            kind: SyncKind::Folder,
            server_url: None,
            email: None,
            folder_path: Some(PathBuf::from("/tmp/mosaic")),
            device_name: "NAS".to_owned(),
        };
        s.set_sync_config(Some(&folder)).unwrap();
        assert_eq!(s.sync_config().unwrap(), Some(folder));
        s.set_sync_config(None).unwrap();
        assert_eq!(s.sync_config().unwrap(), None);

        let mut bad = hosted();
        bad.device_name = String::new();
        assert!(matches!(
            s.set_sync_config(Some(&bad)),
            Err(Error::SyncDataInvalid)
        ));
        bad.device_name = "x".repeat(65);
        assert!(matches!(
            s.set_sync_config(Some(&bad)),
            Err(Error::SyncDataInvalid)
        ));
        let mut bad = hosted();
        bad.folder_path = Some(PathBuf::from("/x"));
        assert!(matches!(
            s.set_sync_config(Some(&bad)),
            Err(Error::SyncDataInvalid)
        ));
        let mut bad = hosted();
        bad.server_url = Some("u".repeat(2049));
        assert!(matches!(
            s.set_sync_config(Some(&bad)),
            Err(Error::SyncDataInvalid)
        ));
        assert_eq!(s.sync_config().unwrap(), None, "nothing was stored");
    }

    #[test]
    fn local_seq_blocks_never_reused_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.db");
        let mut s = Store::open(&path).unwrap();
        let (f, vk) = (fixture(), VaultKey::generate(g(1)).unwrap());
        s.create_vault(
            &f.vault,
            g(1),
            &vk.wrap(&f.ak, &f.account, &f.vault).unwrap(),
        )
        .unwrap();
        assert_eq!(s.allocate_local_seqs(&f.vault, 3).unwrap(), 1);
        assert_eq!(s.allocate_local_seqs(&f.vault, 2).unwrap(), 4);
        // A request larger than the rest of the block starts a new block.
        let big = u32::try_from(LOCAL_SEQ_BLOCK).unwrap();
        let third = s.allocate_local_seqs(&f.vault, big).unwrap();
        assert!(third > LOCAL_SEQ_BLOCK, "{third}");
        let last_before = s.allocate_local_seqs(&f.vault, 1).unwrap();
        s.close().unwrap();
        let mut s = Store::open(&path).unwrap();
        let after = s.allocate_local_seqs(&f.vault, 1).unwrap();
        assert!(
            after > last_before,
            "{after} > {last_before}: a crash skips, never reuses"
        );
        assert!(matches!(
            s.allocate_local_seqs(&f.vault, 0),
            Err(Error::SyncDataInvalid)
        ));
        assert!(matches!(
            s.allocate_local_seqs(&VaultId::new_v7().unwrap(), 1),
            Err(Error::UnknownVault)
        ));
    }

    #[test]
    fn cursors_and_counter() {
        let mut s = Store::open_in_memory().unwrap();
        let (f, vk) = (fixture(), VaultKey::generate(g(1)).unwrap());
        s.create_vault(
            &f.vault,
            g(1),
            &vk.wrap(&f.ak, &f.account, &f.vault).unwrap(),
        )
        .unwrap();
        assert_eq!(s.sync_cursor_blob(&f.vault).unwrap(), None);
        s.set_sync_cursor_blob(&f.vault, b"abc").unwrap();
        s.set_sync_cursor_blob(&f.vault, b"abd").unwrap();
        assert_eq!(s.sync_cursor_blob(&f.vault).unwrap().unwrap(), b"abd");
        s.set_sync_cursor_blob(&f.vault, &vec![0; 65_536]).unwrap();
        assert!(matches!(
            s.set_sync_cursor_blob(&f.vault, &vec![0; 65_537]),
            Err(Error::SyncDataInvalid)
        ));
        assert!(matches!(
            s.set_sync_cursor_blob(&VaultId::new_v7().unwrap(), b""),
            Err(Error::UnknownVault)
        ));
        assert_eq!(s.checkpoint_counter().unwrap(), 0);
        assert_eq!(s.next_checkpoint_counter().unwrap(), 1);
        assert_eq!(s.next_checkpoint_counter().unwrap(), 2);
        assert_eq!(s.checkpoint_counter().unwrap(), 2);
    }

    fn item(f: &super::super::vaults::tests::Fixture, vk: &VaultKey, id: &ItemId) -> RecordBody {
        RecordBody::Item(ItemCiphertext::seal(vk, &f.vault, id, 1, b"x").unwrap())
    }

    /// Only an attachment that had been uploaded leaves a deletion behind.
    #[test]
    fn attachment_deletion_recorded_only_after_upload() {
        use mosaic_core::{AttachmentKey, seal_attachment};
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().join("v.db")).unwrap();
        let (f, vk) = (fixture(), VaultKey::generate(g(1)).unwrap());
        s.create_vault(
            &f.vault,
            g(1),
            &vk.wrap(&f.ak, &f.account, &f.vault).unwrap(),
        )
        .unwrap();
        let rid = ItemId::new_v7().unwrap();
        s.upsert_record(&f.vault, &rid, &item(&f, &vk, &rid), Origin::Local)
            .unwrap();
        let mut ids = Vec::new();
        for pending in [true, false, false] {
            let aid = AttachmentId::new_v7().unwrap();
            let ct = seal_attachment(
                &AttachmentKey::generate().unwrap(),
                &f.vault,
                &rid,
                &aid,
                b"f",
            )
            .unwrap();
            s.put_attachment(&f.vault, &rid, &aid, &ct, pending)
                .unwrap();
            ids.push(aid);
        }
        // Never uploaded: nothing to tell the backend.
        assert!(s.delete_attachment(&ids[0]).unwrap());
        assert_eq!(s.pending_attachment_deletions().unwrap().len(), 0);
        // Uploaded: recorded.
        assert!(s.delete_attachment(&ids[1]).unwrap());
        assert_eq!(
            s.pending_attachment_deletions().unwrap(),
            vec![(f.vault, ids[1])]
        );
        // The cascade of a record delete records the rest.
        assert!(
            s.delete_record(&f.vault, &rid, PendingPolicy::Discard)
                .unwrap()
        );
        let mut got: Vec<_> = s
            .pending_attachment_deletions()
            .unwrap()
            .into_iter()
            .map(|(_, a)| a)
            .collect();
        got.sort();
        let mut want = vec![ids[1], ids[2]];
        want.sort();
        assert_eq!(got, want);
        s.mark_attachment_deletion_sent(&ids[1]).unwrap();
        assert_eq!(s.pending_attachment_deletions().unwrap().len(), 1);
        // Storing the ID again makes it live again.
        s.upsert_record(&f.vault, &rid, &item(&f, &vk, &rid), Origin::Local)
            .unwrap();
        let ct = seal_attachment(
            &AttachmentKey::generate().unwrap(),
            &f.vault,
            &rid,
            &ids[2],
            b"f",
        )
        .unwrap();
        s.put_attachment(&f.vault, &rid, &ids[2], &ct, false)
            .unwrap();
        assert_eq!(s.pending_attachment_deletions().unwrap().len(), 0);
        // A vault delete records its uploaded attachments too.
        assert!(s.delete_vault(&f.vault, PendingPolicy::Discard).unwrap());
        assert_eq!(
            s.pending_attachment_deletions().unwrap(),
            vec![(f.vault, ids[2])]
        );
    }

    #[test]
    fn vault_flags_and_queue() {
        let mut s = Store::open_in_memory().unwrap();
        let (f, vk) = (fixture(), VaultKey::generate(g(1)).unwrap());
        let wrapped = vk.wrap(&f.ak, &f.account, &f.vault).unwrap();
        s.create_vault_origin(&f.vault, g(1), &wrapped, KeyOrigin::Local)
            .unwrap();
        assert_eq!(
            s.vault_sync_state(&f.vault).unwrap(),
            VaultSyncState {
                remote_created: false,
                meta_upload_pending: false,
                meta_server_seq: None
            }
        );
        assert_eq!(s.pending_vault_keys(&f.vault).unwrap().len(), 1);
        assert_eq!(
            s.vault_key_origin(&f.vault, g(1)).unwrap(),
            Some(KeyOrigin::Local)
        );
        assert_eq!(s.vault_key_origin(&f.vault, g(2)).unwrap(), None);
        // A key accepted from the backend is not queued.
        let vk2 = VaultKey::generate(g(2)).unwrap();
        let w2 = vk2.wrap(&f.ak, &f.account, &f.vault).unwrap();
        s.accept_vault_key_origin(&f.vault, g(2), &w2, KeyOrigin::Remote)
            .unwrap();
        assert_eq!(s.pending_vault_keys(&f.vault).unwrap().len(), 1);
        assert_eq!(
            s.vault_key_origin(&f.vault, g(2)).unwrap(),
            Some(KeyOrigin::Remote)
        );
        assert!(s.mark_vault_key_uploaded(&f.vault, g(1)).unwrap());
        assert!(!s.mark_vault_key_uploaded(&f.vault, g(1)).unwrap());
        assert_eq!(s.pending_vault_keys(&f.vault).unwrap().len(), 0);
        // Created remotely clears the keys up to its generation only.
        let vk3 = VaultKey::generate(g(3)).unwrap();
        let w3 = vk3.wrap(&f.ak, &f.account, &f.vault).unwrap();
        s.accept_vault_key_origin(&f.vault, g(3), &w3, KeyOrigin::Local)
            .unwrap();
        s.mark_vault_created(&f.vault, g(2), None).unwrap();
        assert!(s.vault_sync_state(&f.vault).unwrap().remote_created);
        assert_eq!(s.pending_vault_keys(&f.vault).unwrap().len(), 1);
        s.mark_vault_created(&f.vault, g(3), None).unwrap();
        assert_eq!(s.pending_vault_keys(&f.vault).unwrap().len(), 0);

        let remote = VaultId::new_v7().unwrap();
        s.create_vault_origin(
            &remote,
            g(1),
            &vk.wrap(&f.ak, &f.account, &remote).unwrap(),
            KeyOrigin::Remote,
        )
        .unwrap();
        let state = s.vault_sync_state(&remote).unwrap();
        assert!(state.remote_created && !state.meta_upload_pending);
        assert_eq!(s.pending_vault_keys(&remote).unwrap().len(), 0);
    }
}
