//! Sharing state of the local store (schema v8, CRYPTO_SPEC §8): the
//! account's identity key, the synced contact book, the per-vault `shared`
//! flag and own role, the roster, pending revocations and findings, and the
//! one-transaction commit of a shared-vault rotation.
//!
//! Like everything in the store, these hold only ciphertext, public values
//! (IDs, roles, generations) and the wrapped `isk`.

use mosaic_core::{
    AccountId, AttachmentId, ContactsCiphertext, IdentityPublicKey, ItemId, KeyGeneration, VaultId,
    VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use rusqlite::{OptionalExtension, params};

use super::error::{Error, Result};
use super::fs::{remove_file_if_present, write_atomic};
use super::records::{RecordBody, next_outbox_seq};
use super::sync_state::{KeyFlags, record_deletions};
use super::vaults::{confirm_key, require_vault, upsert_key};
use super::{
    Store, attachments, generation_from_sql, now_ms, seq_from_sql, seq_to_sql, vault_id_from_sql,
};

/// The identity key row (`isk` wrapped under AK, `ipk`, whether the server
/// holds them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredIdentity {
    /// `isk` wrapped under the account key.
    pub wrapped_isk: WrappedKey,
    /// The public key.
    pub ipk: IdentityPublicKey,
    /// When the key was made (ms since the Unix epoch).
    pub created_at_ms: i64,
    /// The hosted server holds exactly this key.
    pub published: bool,
}

/// The stored contact book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredContacts {
    /// The sealed book.
    pub ciphertext: ContactsCiphertext,
    /// The server's sequence number of the version last seen.
    pub seq: Option<u64>,
    /// The stored book has not been uploaded.
    pub pending: bool,
}

/// The `shared` flag of a vault with this account's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultSharing {
    /// The vault.
    pub vault_id: VaultId,
    /// This account's role: owner 1, manage 2, write 3, read 4.
    pub own_role: u8,
    /// The vault's owner (fixed at the first time this device saw it shared).
    pub owner_account: AccountId,
    /// When the server first refused this device the vault (ms), if it did.
    pub access_lost_at_ms: Option<i64>,
}

/// The stored roster of a vault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRoster {
    /// The generation it is sealed under.
    pub generation: KeyGeneration,
    /// The sealed roster.
    pub ciphertext: VaultRosterCiphertext,
    /// The server's sequence number of the version last seen.
    pub seq: Option<u64>,
    /// The stored roster has not been uploaded.
    pub pending: bool,
}

/// A revocation still to be carried out by the owner's device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingRevocationRow {
    /// The vault.
    pub vault_id: VaultId,
    /// The account to lock out.
    pub account_id: AccountId,
    /// Why (1 removed, 2 left, 3 invite declined, 4 invite cancelled,
    /// 5 unconfirmed member).
    pub reason: u8,
    /// When it was requested (ms).
    pub requested_at_ms: i64,
}

/// A finding against the server about sharing, as stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindingRow {
    /// The vault, if the finding is about one.
    pub vault_id: Option<VaultId>,
    /// The kind code (1 to 4).
    pub kind: u8,
    /// A key generation, if it matters.
    pub generation: Option<u32>,
    /// An account, if it matters.
    pub account_id: Option<AccountId>,
    /// When it was recorded (ms).
    pub at_ms: i64,
}

/// Everything [`Store::commit_shared_rotation`] writes in its one
/// transaction. Attachment files for `new_attachments` are already staged
/// ([`Store::stage_attachment_file`]).
#[derive(Debug)]
pub struct SharedRotationWrite<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// The new generation.
    pub generation: KeyGeneration,
    /// The new vault key wrapped under AK.
    pub wrapped: &'a WrappedKey,
    /// Every record re-sealed under the new key.
    pub records: &'a [(ItemId, RecordBody)],
    /// The metadata re-sealed under the new key.
    pub meta: Option<&'a VaultMetaCiphertext>,
    /// The roster at the new generation.
    pub roster: &'a VaultRosterCiphertext,
    /// Re-sealed attachments: new ID, owning record, ciphertext length.
    pub new_attachments: &'a [(AttachmentId, ItemId, u64)],
    /// The attachments they replace.
    pub old_attachments: &'a [AttachmentId],
    /// Revocations carried out by this rotation.
    pub revoked: &'a [AccountId],
}

type SharingRow = (Vec<u8>, i64, Vec<u8>, Option<i64>);

fn account_from_sql(bytes: &[u8]) -> Result<AccountId> {
    AccountId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

fn opt_seq(seq: Option<i64>) -> Result<Option<u64>> {
    seq.map(seq_from_sql).transpose()
}

impl Store {
    // ---- identity key ----------------------------------------------------

    /// The identity key row, if there is one.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn identity_key(&self) -> Result<Option<StoredIdentity>> {
        let row: Option<(Vec<u8>, Vec<u8>, i64, bool)> = self
            .conn
            .query_row(
                "SELECT wrapped_isk, ipk, created_at, published FROM identity_key WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(wrapped, ipk, created_at_ms, published)| {
            Ok(StoredIdentity {
                wrapped_isk: WrappedKey::from_bytes(&wrapped).map_err(|_| Error::Corrupt)?,
                ipk: IdentityPublicKey::from_slice(&ipk).map_err(|_| Error::Corrupt)?,
                created_at_ms,
                published,
            })
        })
        .transpose()
    }

    /// Stores the identity key. There is at most one.
    ///
    /// # Errors
    /// [`Error::IdentityExists`] if one is stored.
    pub fn insert_identity_key(
        &mut self,
        wrapped_isk: &WrappedKey,
        ipk: &IdentityPublicKey,
        published: bool,
    ) -> Result<()> {
        let tx = self.write()?;
        let exists: bool = tx.query_row("SELECT EXISTS (SELECT 1 FROM identity_key)", [], |r| {
            r.get(0)
        })?;
        if exists {
            return Err(Error::IdentityExists);
        }
        tx.execute(
            "INSERT INTO identity_key (id, wrapped_isk, ipk, created_at, published)
             VALUES (1, ?1, ?2, ?3, ?4)",
            params![wrapped_isk.as_bytes(), ipk.as_bytes(), now_ms(), published],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Replaces an identity key the server never held by the server's
    /// (adoption); the new key counts as published.
    ///
    /// # Errors
    /// [`Error::IdentityExists`] if the stored key is published or missing.
    pub fn adopt_identity_key(
        &mut self,
        wrapped_isk: &WrappedKey,
        ipk: &IdentityPublicKey,
    ) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE identity_key SET wrapped_isk = ?1, ipk = ?2, published = 1
             WHERE id = 1 AND published = 0",
            params![wrapped_isk.as_bytes(), ipk.as_bytes()],
        )?;
        if n == 0 {
            return Err(Error::IdentityExists);
        }
        Ok(())
    }

    /// Notes that the server holds the stored identity key.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_identity_published(&mut self) -> Result<()> {
        self.conn
            .execute("UPDATE identity_key SET published = 1 WHERE id = 1", [])?;
        Ok(())
    }

    // ---- contact book ------------------------------------------------------

    /// The stored contact book, if any.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn contacts_doc(&self) -> Result<Option<StoredContacts>> {
        let row: Option<(Vec<u8>, Option<i64>, bool)> = self
            .conn
            .query_row(
                "SELECT ciphertext, seq, pending FROM contacts_doc WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        row.map(|(ct, seq, pending)| {
            Ok(StoredContacts {
                ciphertext: ContactsCiphertext::from_bytes(&ct).map_err(|_| Error::Corrupt)?,
                seq: opt_seq(seq)?,
                pending,
            })
        })
        .transpose()
    }

    /// Stores the contact book with its server sequence number and upload
    /// flag.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn put_contacts_doc(
        &mut self,
        ciphertext: &ContactsCiphertext,
        seq: Option<u64>,
        pending: bool,
    ) -> Result<()> {
        let seq = seq.map(seq_to_sql).transpose()?;
        self.conn.execute(
            "INSERT INTO contacts_doc (id, ciphertext, seq, pending) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET
                 ciphertext = excluded.ciphertext, seq = excluded.seq,
                 pending = excluded.pending",
            params![ciphertext.as_bytes(), seq, pending],
        )?;
        Ok(())
    }

    /// Sets the server sequence number of the stored contact book and its
    /// upload flag.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn settle_contacts_doc(&mut self, seq: u64, pending: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE contacts_doc SET seq = ?1, pending = ?2 WHERE id = 1",
            params![seq_to_sql(seq)?, pending],
        )?;
        Ok(())
    }

    // ---- shared flag -------------------------------------------------------

    /// The sharing row of a vault.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault_sharing(&self, vault_id: &VaultId) -> Result<Option<VaultSharing>> {
        let row: Option<SharingRow> = self
            .conn
            .query_row(
                "SELECT vault_id, own_role, owner_account, access_lost_at
                 FROM vault_sharing WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(id, role, owner, lost)| {
            Ok(VaultSharing {
                vault_id: vault_id_from_sql(&id)?,
                own_role: u8::try_from(role).map_err(|_| Error::Corrupt)?,
                owner_account: account_from_sql(&owner)?,
                access_lost_at_ms: lost,
            })
        })
        .transpose()
    }

    /// Sets the `shared` flag of a vault with this account's role. The flag
    /// is monotone: a row is never removed, and its owner never changes
    /// (a trigger enforces both). `clear_lost` forgets an earlier loss of
    /// access (the account was invited again).
    ///
    /// # Errors
    /// [`Error::Database`] (the owner trigger) for another owner.
    pub fn set_vault_sharing(
        &mut self,
        vault_id: &VaultId,
        own_role: u8,
        owner: &AccountId,
        clear_lost: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO vault_sharing (vault_id, shared, own_role, owner_account)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT (vault_id) DO UPDATE SET
                 own_role = excluded.own_role,
                 owner_account = excluded.owner_account,
                 access_lost_at = CASE WHEN ?4 THEN NULL ELSE access_lost_at END",
            params![vault_id.as_bytes(), own_role, owner.as_bytes(), clear_lost],
        )?;
        Ok(())
    }

    /// Records that the server refuses this device the vault. Returns
    /// whether a sharing row exists.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_access_lost(&mut self, vault_id: &VaultId) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE vault_sharing SET access_lost_at = coalesce(access_lost_at, ?2)
             WHERE vault_id = ?1",
            params![vault_id.as_bytes(), now_ms()],
        )?;
        Ok(n > 0)
    }

    /// How many shared vaults this device still has access to (rows without
    /// `access_lost_at`). A backend change is refused while there are any.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn live_shared_vaults(&self) -> Result<u64> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM vault_sharing WHERE access_lost_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        u64::try_from(n).map_err(|_| Error::Corrupt)
    }

    // ---- roster ------------------------------------------------------------

    /// The stored roster of a vault.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn vault_roster(&self, vault_id: &VaultId) -> Result<Option<StoredRoster>> {
        let row: Option<(i64, Vec<u8>, Option<i64>, bool)> = self
            .conn
            .query_row(
                "SELECT key_generation, ciphertext, seq, pending FROM vault_roster
                 WHERE vault_id = ?1",
                [vault_id.as_bytes()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(generation, ct, seq, pending)| {
            let ciphertext = VaultRosterCiphertext::from_bytes(&ct).map_err(|_| Error::Corrupt)?;
            let generation = generation_from_sql(generation)?;
            if ciphertext.key_generation() != generation {
                return Err(Error::Corrupt);
            }
            Ok(StoredRoster {
                generation,
                ciphertext,
                seq: opt_seq(seq)?,
                pending,
            })
        })
        .transpose()
    }

    /// Stores the roster of a vault. A roster never goes back to an earlier
    /// generation.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::KeyGenerationRollback`] below the
    /// stored generation, [`Error::SeqOutOfRange`].
    pub fn put_vault_roster(
        &mut self,
        vault_id: &VaultId,
        ciphertext: &VaultRosterCiphertext,
        seq: Option<u64>,
        pending: bool,
    ) -> Result<()> {
        let seq = seq.map(seq_to_sql).transpose()?;
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        write_roster(&tx, vault_id, ciphertext, seq, pending)?;
        tx.commit()?;
        Ok(())
    }

    /// Sets the server sequence number of the stored roster and its upload
    /// flag.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn settle_vault_roster(
        &mut self,
        vault_id: &VaultId,
        seq: u64,
        pending: bool,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE vault_roster SET seq = ?2, pending = ?3 WHERE vault_id = ?1",
            params![vault_id.as_bytes(), seq_to_sql(seq)?, pending],
        )?;
        Ok(())
    }

    // ---- pending revocations -----------------------------------------------

    /// Queues a revocation (the first request for an account stays).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Database`].
    pub fn add_pending_revocation(
        &mut self,
        vault_id: &VaultId,
        account: &AccountId,
        reason: u8,
    ) -> Result<()> {
        let tx = self.write()?;
        require_vault(&tx, vault_id)?;
        tx.execute(
            "INSERT OR IGNORE INTO pending_revocations (vault_id, account_id, reason, requested_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![vault_id.as_bytes(), account.as_bytes(), reason, now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Every queued revocation, oldest first.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn pending_revocations(&self) -> Result<Vec<PendingRevocationRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, account_id, reason, requested_at FROM pending_revocations
             ORDER BY requested_at, vault_id, account_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (vault, account, reason, at) = row?;
            Ok(PendingRevocationRow {
                vault_id: vault_id_from_sql(&vault)?,
                account_id: account_from_sql(&account)?,
                reason: u8::try_from(reason).map_err(|_| Error::Corrupt)?,
                requested_at_ms: at,
            })
        })
        .collect()
    }

    /// Drops a queued revocation. Returns whether there was one.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn remove_pending_revocation(
        &mut self,
        vault_id: &VaultId,
        account: &AccountId,
    ) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM pending_revocations WHERE vault_id = ?1 AND account_id = ?2",
            params![vault_id.as_bytes(), account.as_bytes()],
        )?;
        Ok(n > 0)
    }

    // ---- findings ----------------------------------------------------------

    /// Records a finding, replacing an equal one (so a repeated run does not
    /// pile them up).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn add_sharing_finding(
        &mut self,
        kind: u8,
        vault_id: Option<&VaultId>,
        generation: Option<u32>,
        account: Option<&AccountId>,
    ) -> Result<()> {
        let vault = vault_id.map(|v| v.as_bytes().to_vec());
        let account = account.map(|a| a.as_bytes().to_vec());
        let tx = self.write()?;
        tx.execute(
            "DELETE FROM sharing_findings
             WHERE kind = ?1 AND vault_id IS ?2 AND generation IS ?3 AND account_id IS ?4",
            params![kind, vault, generation, account],
        )?;
        tx.execute(
            "INSERT INTO sharing_findings (vault_id, kind, generation, account_id, at)
             VALUES (?2, ?1, ?3, ?4, ?5)",
            params![kind, vault, generation, account, now_ms()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Every finding, oldest first.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn sharing_findings(&self) -> Result<Vec<FindingRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, kind, generation, account_id, at FROM sharing_findings
             ORDER BY at, rowid",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<Vec<u8>>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<Vec<u8>>>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (vault, kind, generation, account, at) = row?;
            Ok(FindingRow {
                vault_id: vault.map(|v| vault_id_from_sql(&v)).transpose()?,
                kind: u8::try_from(kind).map_err(|_| Error::Corrupt)?,
                generation: generation
                    .map(|g| u32::try_from(g).map_err(|_| Error::Corrupt))
                    .transpose()?,
                account_id: account.map(|a| account_from_sql(&a)).transpose()?,
                at_ms: at,
            })
        })
        .collect()
    }

    /// Clears every finding (the user acknowledged them).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn clear_sharing_findings(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM sharing_findings", [])?;
        Ok(())
    }

    // ---- shared rotation ---------------------------------------------------

    /// Writes an attachment ciphertext file under its (new) ID before the
    /// transaction that adds its row ([`Store::commit_shared_rotation`]), so
    /// large attachments are staged one at a time. A crash leaves at most an
    /// orphan file, which [`Store::sweep_orphan_attachments`] removes.
    ///
    /// # Errors
    /// [`Error::AttachmentsUnavailable`], [`Error::AttachmentMalformed`],
    /// [`Error::AttachmentTooLarge`], [`Error::AttachmentExists`],
    /// [`Error::Io`].
    pub fn stage_attachment_file(&self, id: &AttachmentId, ciphertext: &[u8]) -> Result<()> {
        attachments::validate_attachment_ciphertext(ciphertext)?;
        let dir = self
            .attachment_dir
            .as_deref()
            .ok_or(Error::AttachmentsUnavailable)?;
        let name = attachments::file_name(id);
        if dir.join(&name).exists() {
            return Err(Error::AttachmentExists);
        }
        write_atomic(dir, &name, ciphertext)
    }

    /// Removes staged files whose transaction did not happen.
    ///
    /// # Errors
    /// [`Error::Io`].
    pub fn discard_staged_attachment_files(&self, ids: &[AttachmentId]) -> Result<()> {
        if let Some(dir) = self.attachment_dir.as_deref() {
            for id in ids {
                remove_file_if_present(&dir.join(attachments::file_name(id)))?;
            }
        }
        Ok(())
    }

    /// The store half of a shared-vault rotation, in ONE transaction: the
    /// new wrapped key (generated here, already accepted by the server, so
    /// nothing is queued for it), the vault's generation, every record and
    /// the metadata re-sealed under it and queued, the roster at the new
    /// generation, the re-sealed attachments (new rows queued for upload; the
    /// old rows removed, and the old ones the server holds recorded for
    /// deletion), and the revocations it carried out. Either all of it or
    /// none (a crash must never leave records under a key the store lacks).
    ///
    /// The old attachment files go after the commit. The caller must send
    /// the recorded deletions only after the referencing records were
    /// accepted.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::KeyGenerationRollback`] unless
    /// `generation` is above the accepted one, [`Error::UnknownRecord`],
    /// [`Error::SeqOutOfRange`], [`Error::Database`], [`Error::Io`].
    #[expect(
        clippy::too_many_lines,
        reason = "one transaction: the steps are listed in the order they must run"
    )]
    pub fn commit_shared_rotation(&mut self, w: &SharedRotationWrite<'_>) -> Result<()> {
        let dir = self.attachment_dir.clone();
        let vault_id = &w.vault_id;
        let tx = self.write()?;
        let state = require_vault(&tx, vault_id)?;
        if w.generation <= state.highest_accepted_generation {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: state.highest_accepted_generation.get(),
                offered: w.generation.get(),
            });
        }
        tx.execute(
            "UPDATE vaults SET current_generation = ?2, highest_accepted_generation = ?2,
                 updated_at = ?3
             WHERE vault_id = ?1",
            params![vault_id.as_bytes(), w.generation.get(), now_ms()],
        )?;
        upsert_key(
            &tx,
            vault_id,
            w.generation,
            w.wrapped,
            KeyFlags {
                upload_pending: false,
                local_origin: true,
            },
        )?;
        confirm_key(&tx, vault_id, w.generation)?;
        for (record_id, body) in w.records {
            let outbox = next_outbox_seq(&tx)?;
            let n = tx.execute(
                "UPDATE records SET kind = ?3, key_generation = ?4, ciphertext = ?5,
                     outbox_seq = ?6, updated_at = ?7
                 WHERE vault_id = ?1 AND record_id = ?2",
                params![
                    vault_id.as_bytes(),
                    record_id.as_bytes(),
                    body.kind().code(),
                    body.key_generation().get(),
                    body.as_bytes(),
                    seq_to_sql(outbox.get())?,
                    now_ms()
                ],
            )?;
            if n == 0 {
                return Err(Error::UnknownRecord);
            }
        }
        if let Some(meta) = w.meta {
            tx.execute(
                "UPDATE vaults SET meta = ?2, meta_generation = ?3, meta_upload_pending = 1,
                     updated_at = ?4
                 WHERE vault_id = ?1",
                params![
                    vault_id.as_bytes(),
                    meta.as_bytes(),
                    meta.key_generation().get(),
                    now_ms()
                ],
            )?;
        }
        // The rotate request carried the roster; its sequence number is
        // learned with the next pull.
        write_roster(&tx, vault_id, w.roster, None, false)?;
        for (id, record, len) in w.new_attachments {
            tx.execute(
                "INSERT INTO attachments (attachment_id, vault_id, record_id, size,
                                          upload_pending, created_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                params![
                    id.as_bytes(),
                    vault_id.as_bytes(),
                    record.as_bytes(),
                    i64::try_from(*len).map_err(|_| Error::AttachmentTooLarge)?,
                    now_ms()
                ],
            )?;
        }
        for id in w.old_attachments {
            record_deletions(&tx, "attachment_id = ?1", [id.as_bytes()])?;
            tx.execute(
                "DELETE FROM attachments WHERE attachment_id = ?1",
                [id.as_bytes()],
            )?;
        }
        for account in w.revoked {
            tx.execute(
                "DELETE FROM pending_revocations WHERE vault_id = ?1 AND account_id = ?2",
                params![vault_id.as_bytes(), account.as_bytes()],
            )?;
        }
        tx.commit()?;
        attachments::remove_files(dir.as_deref(), w.old_attachments)?;
        self.scrub_wal()
    }
}

fn write_roster(
    conn: &rusqlite::Connection,
    vault_id: &VaultId,
    ciphertext: &VaultRosterCiphertext,
    seq: Option<i64>,
    pending: bool,
) -> Result<()> {
    let stored: Option<i64> = conn
        .query_row(
            "SELECT key_generation FROM vault_roster WHERE vault_id = ?1",
            [vault_id.as_bytes()],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(stored) = stored {
        let stored = generation_from_sql(stored)?;
        if ciphertext.key_generation() < stored {
            return Err(Error::KeyGenerationRollback {
                highest_accepted: stored.get(),
                offered: ciphertext.key_generation().get(),
            });
        }
    }
    conn.execute(
        "INSERT INTO vault_roster (vault_id, key_generation, ciphertext, seq, pending)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (vault_id) DO UPDATE SET
             key_generation = excluded.key_generation, ciphertext = excluded.ciphertext,
             seq = excluded.seq, pending = excluded.pending",
        params![
            vault_id.as_bytes(),
            ciphertext.key_generation().get(),
            ciphertext.as_bytes(),
            seq,
            pending
        ],
    )?;
    Ok(())
}
