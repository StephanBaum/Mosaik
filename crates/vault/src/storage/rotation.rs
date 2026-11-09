//! Key rotation state of the local store (schema v9, CRYPTO_SPEC §6.4 to
//! §6.7, §10.1): the account-key epoch, the rotation journal, the attachment
//! re-seal queue, the folder floors, the retired Secret Key commitments, the
//! dismissed unverifiable account files, and the one-transaction local halves
//! of a full re-key ([`Store::commit_rekey`]) and of adopting one
//! ([`Store::adopt_rekey`]).
//!
//! Like everything in the store these hold ciphertext, public values and
//! commitments. No secret is ever written here: the journal row names a
//! phase and counts, never a key.

use std::collections::HashSet;

use mosaic_core::{
    AttachmentId, ContactsCiphertext, IdentityPublicKey, ItemId, KeyGeneration, PasswordParams,
    VaultId, VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::account::{
    UnlockRow, check_params, decode_unlock, read_account, update_account, upsert_unlock,
};
use super::attachments;
use super::error::{Error, Result};
use super::records::{RecordBody, next_outbox_seq};
use super::sharing::write_roster;
use super::sync_state::KeyFlags;
use super::vaults::{VaultKeyWrap, confirm_key, require_vault, rewrap_key, upsert_key};
use super::{Store, StoredUnlockMethod, now_ms, seq_from_sql, seq_to_sql, vault_id_from_sql};
use mosaic_core::UnlockMethod;

/// The length of a folder re-key marker (`.mrek`, CRYPTO_SPEC §10.1).
pub const FOLDER_MARKER_LEN: usize = 93;

/// What a rotation job rotates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationKind {
    /// A Secret Key rotation (§6.5).
    SecretKey,
    /// A full re-key (§6.6).
    Rekey,
    /// A KDF profile upgrade (§5.3).
    Profile,
    /// A vault key rotation with an attachment re-seal (§7.1).
    Vault,
}

impl RotationKind {
    /// The stored code: 1 Secret Key, 2 re-key, 3 profile, 4 vault.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::SecretKey => 1,
            Self::Rekey => 2,
            Self::Profile => 3,
            Self::Vault => 4,
        }
    }

    /// The kind of a stored code.
    #[must_use]
    pub const fn from_code(code: i64) -> Option<Self> {
        match code {
            1 => Some(Self::SecretKey),
            2 => Some(Self::Rekey),
            3 => Some(Self::Profile),
            4 => Some(Self::Vault),
            _ => None,
        }
    }
}

/// Where a rotation job stands (CRYPTO_SPEC §6.6). The numbers are stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RotationPhase {
    /// 1: the kit is shown and verified, the pending Secret Key is staged.
    Prepared,
    /// 2: hosted, `start` was sent.
    BackendStarted,
    /// 3: hosted, `finish` answered. The prepared keys died with the process.
    BackendAccepted,
    /// 4: folder, the new account file is written.
    FolderWritten,
    /// 5: the store holds the new epoch.
    LocalCommitted,
    /// 6: attachments are being re-sealed.
    Attachments,
    /// 7: pending uploads are being sent.
    Uploading,
}

impl RotationPhase {
    /// The stored number, 1 to 7.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Prepared => 1,
            Self::BackendStarted => 2,
            Self::BackendAccepted => 3,
            Self::FolderWritten => 4,
            Self::LocalCommitted => 5,
            Self::Attachments => 6,
            Self::Uploading => 7,
        }
    }

    /// The phase of a stored number.
    #[must_use]
    pub const fn from_number(n: i64) -> Option<Self> {
        match n {
            1 => Some(Self::Prepared),
            2 => Some(Self::BackendStarted),
            3 => Some(Self::BackendAccepted),
            4 => Some(Self::FolderWritten),
            5 => Some(Self::LocalCommitted),
            6 => Some(Self::Attachments),
            7 => Some(Self::Uploading),
            _ => None,
        }
    }

    /// Whether a job of `kind` has this phase: kinds 1 and 3 have the phases
    /// 1 to 3 and 5 only, a vault job 5 to 7.
    #[must_use]
    pub const fn valid_for(self, kind: RotationKind) -> bool {
        match kind {
            RotationKind::Rekey => true,
            RotationKind::SecretKey | RotationKind::Profile => matches!(
                self,
                Self::Prepared
                    | Self::BackendStarted
                    | Self::BackendAccepted
                    | Self::LocalCommitted
            ),
            RotationKind::Vault => matches!(
                self,
                Self::LocalCommitted | Self::Attachments | Self::Uploading
            ),
        }
    }
}

/// The rotation journal row (public values only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RotationJob {
    /// What is rotated.
    pub kind: RotationKind,
    /// Where it stands.
    pub phase: RotationPhase,
    /// When the job started (ms since the Unix epoch).
    pub started_at_ms: i64,
    /// The account-key epoch the job starts from (re-key).
    pub from_epoch: Option<u32>,
    /// The epoch it moves to (re-key).
    pub to_epoch: Option<u32>,
    /// A re-key on a folder: the account-file version to write; a vault
    /// rotation: the generation.
    pub target_version: Option<u64>,
    /// A re-key on a folder: the `.mrek` marker, built under the old `K_fold`
    /// before the commit dropped the old AK.
    pub folder_marker: Option<[u8; FOLDER_MARKER_LEN]>,
    /// Steps done (attachments re-sealed).
    pub done: u32,
    /// Steps in all.
    pub total: u32,
}

/// A job to open ([`Store::begin_rotation_job`]).
#[derive(Clone, Copy, Debug)]
pub struct NewRotationJob {
    /// What is rotated.
    pub kind: RotationKind,
    /// The first phase (1 for kinds 1 to 3, 6 for a vault job).
    pub phase: RotationPhase,
    /// The epoch the job starts from.
    pub from_epoch: Option<u32>,
    /// The epoch it moves to.
    pub to_epoch: Option<u32>,
    /// The folder account-file version or the vault generation.
    pub target_version: Option<u64>,
}

/// Where one attachment stands in a re-seal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResealState {
    /// Still to do.
    Todo,
    /// Sealed under a new ID, not swapped in (reserved; a step is one
    /// transaction, so this state is not stored today).
    Sealed,
    /// The item refers to the new copy.
    Swapped,
    /// Neither held nor fetchable: it keeps its old file key.
    Unreachable,
}

impl ResealState {
    const fn code(self) -> i64 {
        match self {
            Self::Todo => 0,
            Self::Sealed => 1,
            Self::Swapped => 2,
            Self::Unreachable => 3,
        }
    }

    const fn from_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(Self::Todo),
            1 => Some(Self::Sealed),
            2 => Some(Self::Swapped),
            3 => Some(Self::Unreachable),
            _ => None,
        }
    }
}

/// One row of the attachment re-seal queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResealRow {
    /// The attachment to replace.
    pub old_id: AttachmentId,
    /// Its vault.
    pub vault_id: VaultId,
    /// The item that refers to it.
    pub item_id: ItemId,
    /// The copy that replaced it, once swapped.
    pub new_id: Option<AttachmentId>,
    /// Where it stands.
    pub state: ResealState,
}

/// This device's own segments of a vault up to `floor_seq` were sealed under
/// the previous epoch (folder backend, CRYPTO_SPEC §10.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderRekeyFloor {
    /// The vault.
    pub vault_id: VaultId,
    /// The highest own sequence number of the previous epoch.
    pub floor_seq: u64,
    /// When the re-key happened (ms).
    pub rekey_at_ms: i64,
    /// A complete snapshot set of the new epoch exists in the own log.
    pub snapshot_done: bool,
}

/// The identity key to store with a new epoch.
#[derive(Clone, Copy, Debug)]
pub struct IdentityWrite<'a> {
    /// `isk` wrapped under the new AK.
    pub wrapped_isk: &'a WrappedKey,
    /// The public key.
    pub ipk: &'a IdentityPublicKey,
    /// The hosted server holds exactly this key.
    pub published: bool,
}

/// What both a re-key and the adoption of one change in the account (the
/// first transaction steps).
#[derive(Clone, Copy, Debug)]
pub struct EpochSwitch<'a> {
    /// The new parameters (same account ID, new salt, profile not weaker).
    pub params: &'a PasswordParams,
    /// The new AK wrapped under the new `KEK`.
    pub password_wrapping: &'a WrappedKey,
    /// The new epoch: exactly the stored one plus one.
    pub to_epoch: u32,
    /// Every vault key held under AK, of every generation, wrapped under the
    /// new AK (the caller may pass the backend's own wrapping for a generation
    /// it must not change).
    pub rewrapped_keys: &'a [(VaultId, KeyGeneration, WrappedKey)],
    /// The identity key of the new epoch, if the account has one.
    pub identity: Option<IdentityWrite<'a>>,
    /// The contact book sealed under the new `K_contacts`.
    pub contacts: Option<&'a ContactsCiphertext>,
    /// The retired commitment of the Secret Key this change replaces.
    pub retire_secret_key: &'a [u8; 32],
    /// Vaults to hold at the generation shown: an adoption leaves each owned
    /// vault at the generation this device held until the re-key's next one
    /// arrives (review M5). Empty for a re-key.
    pub hold: &'a [(VaultId, KeyGeneration)],
}

/// One owned vault that moves to a new generation in a re-key.
#[derive(Debug)]
pub struct RotatedVaultWrite<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// The new generation.
    pub generation: KeyGeneration,
    /// The new vault key wrapped under the new AK.
    pub wrapped: &'a WrappedKey,
    /// Every record re-sealed under the new key.
    pub records: &'a [(ItemId, RecordBody)],
    /// The metadata re-sealed under the new key.
    pub meta: Option<&'a VaultMetaCiphertext>,
    /// The roster at the new generation (shared vaults).
    pub roster: Option<&'a VaultRosterCiphertext>,
}

/// Everything [`Store::commit_rekey`] writes in its one transaction.
#[derive(Debug)]
pub struct RekeyWrite<'a> {
    /// The account part.
    pub switch: EpochSwitch<'a>,
    /// The owned vaults that move to a new generation.
    pub vaults: &'a [RotatedVaultWrite<'a>],
    /// The backend already holds the new keys and rosters (a hosted re-key
    /// commits after `finish`): nothing of them is queued and the keys count
    /// as confirmed. Otherwise they wait for upload.
    pub accepted_by_backend: bool,
    /// One re-seal row per attachment of an owned vault.
    pub attachments: &'a [(AttachmentId, VaultId, ItemId)],
    /// A folder re-key: the marker for devices of the old epoch.
    pub folder_marker: Option<&'a [u8; FOLDER_MARKER_LEN]>,
    /// A folder re-key: the account-file version to write.
    pub target_version: Option<u64>,
}

/// One attachment swapped for its re-sealed copy ([`Store::swap_attachment`]).
/// The new file is already staged ([`Store::stage_attachment_file`]).
#[derive(Debug)]
pub struct AttachmentSwap<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// The item (record) that refers to the attachment.
    pub item_id: ItemId,
    /// The item re-sealed with the new reference.
    pub record: &'a RecordBody,
    /// The attachment replaced.
    pub old_id: AttachmentId,
    /// The copy under a fresh ID and file key.
    pub new_id: AttachmentId,
    /// The length of the new ciphertext.
    pub new_len: u64,
}

fn read_job(conn: &Connection) -> Result<Option<RotationJob>> {
    type Row = (
        i64,
        i64,
        i64,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<Vec<u8>>,
        i64,
        i64,
    );
    let row: Option<Row> = conn
        .query_row(
            "SELECT kind, phase, started_at, from_epoch, to_epoch, target_version,
                    folder_marker, done, total FROM rotation_job WHERE id = 1",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            },
        )
        .optional()?;
    let epoch = |e: Option<i64>| {
        e.map(|e| u32::try_from(e).map_err(|_| Error::Corrupt))
            .transpose()
    };
    row.map(
        |(kind, phase, started, from, to, target, marker, done, total)| {
            Ok(RotationJob {
                kind: RotationKind::from_code(kind).ok_or(Error::Corrupt)?,
                phase: RotationPhase::from_number(phase).ok_or(Error::Corrupt)?,
                started_at_ms: started,
                from_epoch: epoch(from)?,
                to_epoch: epoch(to)?,
                target_version: target.map(seq_from_sql).transpose()?,
                folder_marker: marker
                    .map(|m| <[u8; FOLDER_MARKER_LEN]>::try_from(m).map_err(|_| Error::Corrupt))
                    .transpose()?,
                done: u32::try_from(done).map_err(|_| Error::Corrupt)?,
                total: u32::try_from(total).map_err(|_| Error::Corrupt)?,
            })
        },
    )
    .transpose()
}

fn read_epoch(conn: &Connection) -> Result<u32> {
    let e: Option<i64> = conn
        .query_row("SELECT ak_epoch FROM account WHERE id = 1", [], |r| {
            r.get(0)
        })
        .optional()?;
    u32::try_from(e.ok_or(Error::NoAccount)?).map_err(|_| Error::Corrupt)
}

fn attachment_id(bytes: &[u8]) -> Result<AttachmentId> {
    AttachmentId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

fn item_id(bytes: &[u8]) -> Result<ItemId> {
    ItemId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

type ResealSql = (Vec<u8>, Vec<u8>, Vec<u8>, Option<Vec<u8>>, i64);

fn decode_reseal(row: ResealSql) -> Result<ResealRow> {
    let (old, vault, item, new, state) = row;
    Ok(ResealRow {
        old_id: attachment_id(&old)?,
        vault_id: vault_id_from_sql(&vault)?,
        item_id: item_id(&item)?,
        new_id: new.map(|n| attachment_id(&n)).transpose()?,
        state: ResealState::from_code(state).ok_or(Error::Corrupt)?,
    })
}

const RESEAL_COLS: &str = "old_id, vault_id, item_id, new_id, state";

fn reseal_sql(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResealSql> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
}

/// The first steps of a re-key and of an adoption, in the caller's
/// transaction. Returns the unlock wrappings it deleted.
fn switch_epoch(tx: &Transaction<'_>, sw: &EpochSwitch<'_>) -> Result<Vec<StoredUnlockMethod>> {
    let stored = read_account(tx)?.ok_or(Error::NoAccount)?;
    check_params(sw.params, &stored)?;
    let stored_epoch = read_epoch(tx)?;
    // A re-key may jump past the epochs it saw elsewhere (CRYPTO_SPEC §6.6);
    // the session checks that an adoption moves by exactly one.
    if sw.to_epoch <= stored_epoch {
        return Err(Error::EpochMismatch {
            stored: stored_epoch,
            offered: sw.to_epoch,
        });
    }
    tx.execute(
        "INSERT OR IGNORE INTO retired_salts (salt, retired_at) VALUES (?1, ?2)",
        params![stored.salt.as_bytes(), now_ms()],
    )?;
    update_account(tx, sw.params)?;
    tx.execute(
        "UPDATE account SET ak_epoch = ?1 WHERE id = 1",
        params![sw.to_epoch],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO retired_secret_keys (commitment, retired_at) VALUES (?1, ?2)",
        params![sw.retire_secret_key, now_ms()],
    )?;
    // Every other wrapping of the old AK goes (quick unlock and every
    // hardware key, §6.6); the new password wrapping replaces the old one.
    let removed = delete_other_unlocks(tx, &stored.account_id)?;
    upsert_unlock(
        tx,
        &sw.params.account_id,
        &UnlockMethod::Password,
        sw.password_wrapping,
        &[],
    )?;
    rewrap_all(tx, sw.rewrapped_keys)?;
    for (vault, generation) in sw.hold {
        tx.execute(
            "INSERT INTO rekey_hold (vault_id, generation) VALUES (?1, ?2)
             ON CONFLICT (vault_id) DO UPDATE SET
                 generation = MAX(generation, excluded.generation)",
            params![vault.as_bytes(), generation.get()],
        )?;
    }
    if let Some(id) = &sw.identity {
        write_identity(tx, id)?;
    }
    if let Some(ct) = sw.contacts {
        tx.execute(
            "UPDATE contacts_doc SET ciphertext = ?1, pending = 1 WHERE id = 1",
            params![ct.as_bytes()],
        )?;
    }
    tx.execute(
        "UPDATE sync_engine_state SET rekey_pending = 0 WHERE id = 1",
        [],
    )?;
    // State that belongs to the old epoch. Own counters, `reserved` and
    // `written` stay.
    for sql in [
        "DELETE FROM sync_peer_checkpoint",
        "DELETE FROM sync_view_compare",
        "DELETE FROM sync_lag",
        "DELETE FROM sync_no_common_view",
        "DELETE FROM sync_digest_skip",
        "DELETE FROM sync_key_candidate",
    ] {
        tx.execute(sql, [])?;
    }
    Ok(removed)
}

fn delete_other_unlocks(
    tx: &Transaction<'_>,
    account: &mosaic_core::AccountId,
) -> Result<Vec<StoredUnlockMethod>> {
    let rows: Vec<UnlockRow> = {
        let mut stmt = tx.prepare(
            "SELECT method, method_id, wrapped_ak, params, updated_at FROM unlock_methods
             WHERE method <> 1 ORDER BY method, method_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    tx.execute("DELETE FROM unlock_methods WHERE method <> 1", [])?;
    // A row that does not decode is deleted all the same; it is not reported.
    Ok(rows
        .into_iter()
        .filter_map(|row| decode_unlock(account, row).ok())
        .collect())
}

fn rewrap_all(tx: &Transaction<'_>, keys: &[(VaultId, KeyGeneration, WrappedKey)]) -> Result<()> {
    let mut covered = HashSet::new();
    for (vault_id, generation, wrapped) in keys {
        if require_vault(tx, vault_id)?.key_wrap != VaultKeyWrap::AccountKey {
            return Err(Error::KeyWrapMismatch);
        }
        if !rewrap_key(tx, vault_id, *generation, wrapped)? {
            return Err(Error::UnknownVaultKey);
        }
        covered.insert((*vault_id, *generation));
    }
    let held: i64 = tx.query_row(
        "SELECT count(*) FROM vault_keys JOIN vaults USING (vault_id) WHERE key_wrap = 1",
        [],
        |r| r.get(0),
    )?;
    if usize::try_from(held).ok() != Some(covered.len()) {
        return Err(Error::RekeyIncomplete);
    }
    Ok(())
}

fn write_identity(tx: &Transaction<'_>, id: &IdentityWrite<'_>) -> Result<()> {
    let exists: bool = tx.query_row("SELECT EXISTS (SELECT 1 FROM identity_key)", [], |r| {
        r.get(0)
    })?;
    if exists {
        // The replacement counter is set to the account's new epoch in the
        // same statement (the account row is updated first): the v9 triggers
        // allow a published key to change only that way (review L3).
        tx.execute(
            "UPDATE identity_key SET wrapped_isk = ?1, ipk = ?2, published = ?3,
                 created_at = ?4,
                 epoch = (SELECT ak_epoch FROM account WHERE id = 1) WHERE id = 1",
            params![
                id.wrapped_isk.as_bytes(),
                id.ipk.as_bytes(),
                id.published,
                now_ms()
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO identity_key (id, wrapped_isk, ipk, created_at, published, epoch)
             VALUES (1, ?1, ?2, ?3, ?4, (SELECT ak_epoch FROM account WHERE id = 1))",
            params![
                id.wrapped_isk.as_bytes(),
                id.ipk.as_bytes(),
                now_ms(),
                id.published
            ],
        )?;
    }
    Ok(())
}

fn write_rotated_vault(
    tx: &Transaction<'_>,
    w: &RotatedVaultWrite<'_>,
    accepted: bool,
) -> Result<()> {
    let vault_id = &w.vault_id;
    let state = require_vault(tx, vault_id)?;
    if w.generation <= state.highest_accepted_generation {
        return Err(Error::KeyGenerationRollback {
            highest_accepted: state.highest_accepted_generation.get(),
            offered: w.generation.get(),
        });
    }
    tx.execute(
        "UPDATE vaults SET current_generation = ?2, highest_accepted_generation = ?2,
             updated_at = ?3 WHERE vault_id = ?1",
        params![vault_id.as_bytes(), w.generation.get(), now_ms()],
    )?;
    upsert_key(
        tx,
        vault_id,
        w.generation,
        w.wrapped,
        KeyFlags {
            upload_pending: !accepted,
            local_origin: true,
        },
    )?;
    if accepted {
        confirm_key(tx, vault_id, w.generation)?;
    }
    for (record_id, body) in w.records {
        let outbox = next_outbox_seq(tx)?;
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
                 updated_at = ?4 WHERE vault_id = ?1",
            params![
                vault_id.as_bytes(),
                meta.as_bytes(),
                meta.key_generation().get(),
                now_ms()
            ],
        )?;
    }
    if let Some(roster) = w.roster {
        // A hosted rekey request carried the roster; its sequence number is
        // learned with the next pull.
        write_roster(tx, vault_id, roster, None, !accepted)?;
    }
    Ok(())
}

impl Store {
    /// The account-key epoch (CRYPTO_SPEC §6.7), starting at 1.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::Corrupt`], [`Error::Database`].
    pub fn ak_epoch(&self) -> Result<u32> {
        read_epoch(&self.conn)
    }

    /// The rotation journal row, if a job is open.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn rotation_job(&self) -> Result<Option<RotationJob>> {
        read_job(&self.conn)
    }

    /// Opens a rotation job.
    ///
    /// # Errors
    /// [`Error::RotationInProgress`] if one is open,
    /// [`Error::InvalidRotationPhase`] for a phase the kind does not have.
    pub fn begin_rotation_job(&mut self, new: &NewRotationJob) -> Result<()> {
        if !new.phase.valid_for(new.kind) {
            return Err(Error::InvalidRotationPhase);
        }
        let tx = self.write()?;
        if read_job(&tx)?.is_some() {
            return Err(Error::RotationInProgress);
        }
        tx.execute(
            "INSERT INTO rotation_job (id, kind, phase, started_at, from_epoch, to_epoch,
                                       target_version)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                new.kind.code(),
                new.phase.number(),
                now_ms(),
                new.from_epoch,
                new.to_epoch,
                new.target_version.map(seq_to_sql).transpose()?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Moves the job to `phase`. Phases may go back (a resume that finds the
    /// backend did not apply the change returns to 1) and a folder re-key
    /// records phase 4 after the local commit, so the order is not enforced.
    ///
    /// # Errors
    /// [`Error::NoRotationJob`], [`Error::InvalidRotationPhase`].
    pub fn set_rotation_phase(&mut self, phase: RotationPhase) -> Result<()> {
        let tx = self.write()?;
        let job = read_job(&tx)?.ok_or(Error::NoRotationJob)?;
        if !phase.valid_for(job.kind) {
            return Err(Error::InvalidRotationPhase);
        }
        tx.execute(
            "UPDATE rotation_job SET phase = ?1 WHERE id = 1",
            params![phase.number()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Closes the job. It refuses while attachments are still to re-seal, and
    /// for a re-key or credential change that is not committed locally. The
    /// re-seal rows of finished work go; unreachable ones stay as the list of
    /// attachments that keep their old key.
    ///
    /// # Errors
    /// [`Error::NoRotationJob`], [`Error::RotationNotFinished`].
    pub fn finish_rotation_job(&mut self) -> Result<()> {
        let tx = self.write()?;
        let job = read_job(&tx)?.ok_or(Error::NoRotationJob)?;
        let todo: i64 = tx.query_row(
            "SELECT count(*) FROM reseal_attachments WHERE state IN (0, 1)",
            [],
            |r| r.get(0),
        )?;
        let committed = match job.kind {
            RotationKind::Rekey => read_epoch(&tx)? >= job.to_epoch.unwrap_or(u32::MAX),
            RotationKind::SecretKey | RotationKind::Profile => job.phase.number() >= 5,
            RotationKind::Vault => true,
        };
        if todo > 0 || !committed {
            return Err(Error::RotationNotFinished);
        }
        tx.execute("DELETE FROM reseal_attachments WHERE state = 2", [])?;
        tx.execute("DELETE FROM rotation_job WHERE id = 1", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Drops a job the backend has not seen yet (phase 1).
    ///
    /// # Errors
    /// [`Error::NoRotationJob`], [`Error::RotationTooFar`] from phase 2 on.
    pub fn abandon_rotation_job(&mut self) -> Result<()> {
        let tx = self.write()?;
        let job = read_job(&tx)?.ok_or(Error::NoRotationJob)?;
        if job.phase != RotationPhase::Prepared {
            return Err(Error::RotationTooFar);
        }
        tx.execute("DELETE FROM rotation_job WHERE id = 1", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Drops a job whose local part is committed (phase 5 or later, and for
    /// a re-key the stored epoch at the job's target) after the user
    /// confirmed it (review L4, ADR-0064): attachments still to re-seal keep
    /// their old ID and file key and are listed as unreachable, and the job
    /// no longer blocks a change of backend.
    ///
    /// # Errors
    /// [`Error::NoRotationJob`], [`Error::RotationTooFar`] before the commit:
    /// phases 1 to 4 are resume-only (in 1 to 3 the backend may know the
    /// change; a folder re-key at phase 4 is moved on by the next continue).
    pub fn abandon_committed_rotation_job(&mut self) -> Result<()> {
        let tx = self.write()?;
        let job = read_job(&tx)?.ok_or(Error::NoRotationJob)?;
        let committed = job.phase >= RotationPhase::LocalCommitted
            && match job.kind {
                RotationKind::Rekey => read_epoch(&tx)? >= job.to_epoch.unwrap_or(u32::MAX),
                _ => true,
            };
        if !committed {
            return Err(Error::RotationTooFar);
        }
        tx.execute(
            "UPDATE reseal_attachments SET state = 3 WHERE state IN (0, 1)",
            [],
        )?;
        tx.execute("DELETE FROM rotation_job WHERE id = 1", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Queues the attachments of the owned vaults for the open re-key job of
    /// a device that adopted its own re-key after a crash (review M2) and
    /// moves the job to phase 6.
    ///
    /// # Errors
    /// [`Error::NoRotationJob`], [`Error::InvalidRotationPhase`] (not a re-key
    /// job).
    pub fn queue_rekey_job_reseal(
        &mut self,
        rows: &[(AttachmentId, VaultId, ItemId)],
    ) -> Result<()> {
        let tx = self.write()?;
        let job = read_job(&tx)?.ok_or(Error::NoRotationJob)?;
        if job.kind != RotationKind::Rekey {
            return Err(Error::InvalidRotationPhase);
        }
        insert_reseal_rows(&tx, rows)?;
        tx.execute(
            "UPDATE rotation_job SET phase = 6, total = ?1, done = 0 WHERE id = 1",
            params![i64::try_from(rows.len()).map_err(|_| Error::Corrupt)?],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Whether pushes of `vault` wait for the next key generation after an
    /// adopted re-key (review M5): a hold row exists and the vault has not
    /// moved above the generation it names.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn rekey_held(&self, vault: &VaultId) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM rekey_hold h JOIN vaults v ON v.vault_id = h.vault_id
                            WHERE h.vault_id = ?1 AND v.current_generation <= h.generation)",
            params![vault.as_bytes()],
            |r| r.get(0),
        )?)
    }

    /// Raises the stored account-key epoch to `epoch` (never lowers it): a
    /// device that joined an account that was re-keyed already (review H2).
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::Database`].
    pub fn raise_ak_epoch(&mut self, epoch: u32) -> Result<()> {
        read_epoch(&self.conn)?;
        self.conn.execute(
            "UPDATE account SET ak_epoch = ?1 WHERE id = 1 AND ak_epoch < ?1",
            params![epoch],
        )?;
        Ok(())
    }

    /// Opens a vault rotation job at phase 6 with one re-seal row per
    /// attachment (replacing finished rows of an earlier job).
    ///
    /// # Errors
    /// [`Error::RotationInProgress`].
    pub fn queue_attachment_reseal(
        &mut self,
        generation: KeyGeneration,
        rows: &[(AttachmentId, VaultId, ItemId)],
    ) -> Result<()> {
        let tx = self.write()?;
        if read_job(&tx)?.is_some() {
            return Err(Error::RotationInProgress);
        }
        insert_reseal_rows(&tx, rows)?;
        tx.execute(
            "INSERT INTO rotation_job (id, kind, phase, started_at, target_version, done, total)
             VALUES (1, 4, 6, ?1, ?2, 0, ?3)",
            params![
                now_ms(),
                i64::from(generation.get()),
                i64::try_from(rows.len()).map_err(|_| Error::Corrupt)?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Every row of the re-seal queue, oldest first by attachment ID.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn reseal_rows(&self) -> Result<Vec<ResealRow>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {RESEAL_COLS} FROM reseal_attachments ORDER BY old_id"
        ))?;
        let rows = stmt.query_map([], reseal_sql)?;
        rows.map(|r| decode_reseal(r?)).collect()
    }

    /// The next attachment still to re-seal.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn next_reseal_todo(&self) -> Result<Option<ResealRow>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {RESEAL_COLS} FROM reseal_attachments WHERE state = 0
                     ORDER BY old_id LIMIT 1"
                ),
                [],
                reseal_sql,
            )
            .optional()?
            .map(decode_reseal)
            .transpose()
    }

    /// Records that an attachment is neither held nor fetchable: it keeps its
    /// old file key (THREAT_MODEL R85). Returns whether a todo row changed.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_reseal_unreachable(&mut self, old: &AttachmentId) -> Result<bool> {
        let tx = self.write()?;
        let n = tx.execute(
            "UPDATE reseal_attachments SET state = 3 WHERE old_id = ?1 AND state = 0",
            [old.as_bytes()],
        )?;
        tx.execute("UPDATE rotation_job SET done = done + 1 WHERE id = 1", [])?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// Marks an attachment as done without a swap (its item no longer refers
    /// to it). Returns whether a todo row changed.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_reseal_obsolete(&mut self, old: &AttachmentId) -> Result<bool> {
        let tx = self.write()?;
        let n = tx.execute(
            "UPDATE reseal_attachments SET state = 2 WHERE old_id = ?1 AND state = 0",
            [old.as_bytes()],
        )?;
        tx.execute("UPDATE rotation_job SET done = done + 1 WHERE id = 1", [])?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// One attachment re-seal step in ONE transaction: the item re-sealed
    /// with the new reference (queued for upload), the new attachment row
    /// (queued for upload), the old row gone and the old ID recorded for
    /// deletion once the referencing record is accepted, and the queue row
    /// swapped. The old file goes after the commit.
    ///
    /// # Errors
    /// [`Error::UnknownRecord`], [`Error::AttachmentTooLarge`],
    /// [`Error::Database`], [`Error::Io`].
    pub fn swap_attachment(&mut self, w: &AttachmentSwap<'_>) -> Result<()> {
        let dir = self.attachment_dir.clone();
        let len = i64::try_from(w.new_len).map_err(|_| Error::AttachmentTooLarge)?;
        let tx = self.write()?;
        let outbox = next_outbox_seq(&tx)?;
        let n = tx.execute(
            "UPDATE records SET kind = ?3, key_generation = ?4, ciphertext = ?5,
                 outbox_seq = ?6, updated_at = ?7
             WHERE vault_id = ?1 AND record_id = ?2",
            params![
                w.vault_id.as_bytes(),
                w.item_id.as_bytes(),
                w.record.kind().code(),
                w.record.key_generation().get(),
                w.record.as_bytes(),
                seq_to_sql(outbox.get())?,
                now_ms()
            ],
        )?;
        if n == 0 {
            return Err(Error::UnknownRecord);
        }
        tx.execute(
            "INSERT INTO attachments (attachment_id, vault_id, record_id, size,
                                      upload_pending, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![
                w.new_id.as_bytes(),
                w.vault_id.as_bytes(),
                w.item_id.as_bytes(),
                len,
                now_ms()
            ],
        )?;
        // The old copy may exist on the backend although it is not held here.
        let local: Option<bool> = tx
            .query_row(
                "SELECT upload_pending FROM attachments WHERE attachment_id = ?1",
                [w.old_id.as_bytes()],
                |r| r.get(0),
            )
            .optional()?;
        if local != Some(true) {
            tx.execute(
                "INSERT OR REPLACE INTO attachment_deletions (attachment_id, vault_id, deleted_at)
                 VALUES (?1, ?2, ?3)",
                params![w.old_id.as_bytes(), w.vault_id.as_bytes(), now_ms()],
            )?;
        }
        tx.execute(
            "DELETE FROM attachments WHERE attachment_id = ?1",
            [w.old_id.as_bytes()],
        )?;
        tx.execute(
            "UPDATE reseal_attachments SET state = 2, new_id = ?2 WHERE old_id = ?1",
            params![w.old_id.as_bytes(), w.new_id.as_bytes()],
        )?;
        tx.execute("UPDATE rotation_job SET done = done + 1 WHERE id = 1", [])?;
        tx.commit()?;
        attachments::remove_files(dir.as_deref(), &[w.old_id])
    }

    /// The §6.6 local commit in ONE transaction: the new parameters, password
    /// wrapping and epoch, every held vault key re-wrapped, the new key,
    /// records, metadata and roster of every owned vault, the identity, the
    /// contact book, the deleted quick-unlock and hardware-key wrappings
    /// (returned, so the app deletes the platform keys), the cleared peer
    /// state, the attachment queue and the journal at phase 5. Then the WAL is
    /// scrubbed. Either all of it is stored or nothing, so a crash can never
    /// leave vault keys under one AK and the password under another.
    ///
    /// # Errors
    /// [`Error::RotationInProgress`] if a job of another kind is open,
    /// [`Error::EpochMismatch`], [`Error::AccountMismatch`],
    /// [`Error::ProfileDowngrade`], [`Error::RekeyIncomplete`],
    /// [`Error::UnknownVaultKey`], [`Error::KeyWrapMismatch`],
    /// [`Error::KeyGenerationRollback`], [`Error::UnknownRecord`] and the
    /// errors of the writes.
    pub fn commit_rekey(&mut self, w: &RekeyWrite<'_>) -> Result<Vec<StoredUnlockMethod>> {
        let tx = self.write()?;
        let job = read_job(&tx)?;
        if job.as_ref().is_some_and(|j| j.kind != RotationKind::Rekey) {
            return Err(Error::RotationInProgress);
        }
        let from_epoch = read_epoch(&tx)?;
        let removed = switch_epoch(&tx, &w.switch)?;
        for v in w.vaults {
            write_rotated_vault(&tx, v, w.accepted_by_backend)?;
        }
        tx.execute("DELETE FROM reseal_attachments", [])?;
        insert_reseal_rows(&tx, w.attachments)?;
        tx.execute(
            "INSERT OR REPLACE INTO rotation_job (id, kind, phase, started_at, from_epoch,
                 to_epoch, target_version, folder_marker, done, total)
             VALUES (1, 2, 5, COALESCE((SELECT started_at FROM rotation_job WHERE id = 1), ?1),
                     ?2, ?3, ?4, ?5, 0, ?6)",
            params![
                now_ms(),
                from_epoch,
                w.switch.to_epoch,
                w.target_version.map(seq_to_sql).transpose()?,
                w.folder_marker.map(<[u8; FOLDER_MARKER_LEN]>::as_slice),
                i64::try_from(w.attachments.len()).map_err(|_| Error::Corrupt)?
            ],
        )?;
        let folder: bool = tx
            .query_row("SELECT kind FROM sync_config WHERE id = 1", [], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .is_some_and(|k| k == 2);
        if folder {
            tx.execute(
                "INSERT OR REPLACE INTO folder_rekey_floor (vault_id, floor_seq, rekey_at,
                                                            snapshot_done)
                 SELECT o.vault_id, o.reserved, ?1, 0 FROM sync_own_seq o
                 JOIN vaults v ON v.vault_id = o.vault_id WHERE v.key_wrap = 1",
                [now_ms()],
            )?;
        }
        tx.commit()?;
        self.scrub_wal()?;
        Ok(removed)
    }

    /// The local half of adopting a re-key another device made: like
    /// [`Store::commit_rekey`] without new vault keys, records or attachment
    /// work. A re-key job of this device moves to phase 5.
    ///
    /// # Errors
    /// [`Error::EpochMismatch`] unless `to_epoch` is the stored epoch plus
    /// one, and the other errors of [`Store::commit_rekey`].
    pub fn adopt_rekey(&mut self, sw: &EpochSwitch<'_>) -> Result<Vec<StoredUnlockMethod>> {
        let tx = self.write()?;
        let removed = switch_epoch(&tx, sw)?;
        tx.execute(
            "UPDATE rotation_job SET phase = 5 WHERE kind = 2 AND phase < 5",
            [],
        )?;
        tx.commit()?;
        self.scrub_wal()?;
        Ok(removed)
    }

    // ---- folder floors -----------------------------------------------------

    /// The pre-re-key own-segment floors of the folder backend.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn folder_rekey_floors(&self) -> Result<Vec<FolderRekeyFloor>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, floor_seq, rekey_at, snapshot_done FROM folder_rekey_floor
             ORDER BY vault_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, bool>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (vault, floor, at, done) = row?;
            Ok(FolderRekeyFloor {
                vault_id: vault_id_from_sql(&vault)?,
                floor_seq: seq_from_sql(floor)?,
                rekey_at_ms: at,
                snapshot_done: done,
            })
        })
        .collect()
    }

    /// Records a floor (a device that adopted a re-key notes the own
    /// segments it wrote under the old epoch). A floor that exists is raised,
    /// never lowered.
    ///
    /// # Errors
    /// [`Error::SeqOutOfRange`], [`Error::Database`].
    pub fn note_folder_rekey_floor(
        &mut self,
        vault_id: &VaultId,
        floor: u64,
        now: u64,
    ) -> Result<()> {
        let floor = seq_to_sql(floor)?;
        let now = seq_to_sql(now)?;
        self.conn.execute(
            "INSERT INTO folder_rekey_floor (vault_id, floor_seq, rekey_at, snapshot_done)
             VALUES (?1, ?2, ?3, 0)
             ON CONFLICT (vault_id) DO UPDATE SET floor_seq = MAX(floor_seq, excluded.floor_seq)",
            params![vault_id.as_bytes(), floor, now],
        )?;
        Ok(())
    }

    /// Records that a complete new-epoch snapshot set of the vault exists in
    /// the own log. Returns whether a floor row exists.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn mark_folder_rekey_snapshot(&mut self, vault_id: &VaultId) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE folder_rekey_floor SET snapshot_done = 1 WHERE vault_id = ?1",
            [vault_id.as_bytes()],
        )?;
        Ok(n > 0)
    }

    // ---- the re-key-pending flag ---------------------------------------------------

    /// Whether a verified re-key marker named this device's epoch: the account
    /// was re-keyed elsewhere and the user has to adopt it (CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn rekey_pending(&self) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT rekey_pending FROM sync_engine_state WHERE id = 1",
            [],
            |r| r.get(0),
        )?)
    }

    /// Sets or clears the re-key-pending flag (adopting clears it).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn set_rekey_pending(&mut self, pending: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_engine_state SET rekey_pending = ?1 WHERE id = 1",
            [pending],
        )?;
        Ok(())
    }

    // ---- retired Secret Keys and dismissed files -------------------------------

    /// Every retired Secret Key commitment (CRYPTO_SPEC §6.7).
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn retired_secret_keys(&self) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self
            .conn
            .prepare("SELECT commitment FROM retired_secret_keys ORDER BY commitment")?;
        let rows = stmt.query_map([], |r| r.get::<_, Vec<u8>>(0))?;
        rows.map(|r| <[u8; 32]>::try_from(r?).map_err(|_| Error::Corrupt))
            .collect()
    }

    /// Dismisses an unverifiable higher-epoch account file by path and
    /// SHA-256: it never raises a finding again.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn dismiss_unverifiable_file(&mut self, path: &str, sha256: &[u8; 32]) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO rekey_dismissed_files (path, sha256, dismissed_at)
             VALUES (?1, ?2, ?3)",
            params![path, sha256, now_ms()],
        )?;
        Ok(())
    }

    /// Undoes the dismissal of a file (a re-key marker the user restores,
    /// ADR-0064). Returns whether a dismissal was removed.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn undismiss_file(&mut self, path: &str, sha256: &[u8; 32]) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM rekey_dismissed_files WHERE path = ?1 AND sha256 = ?2",
            params![path, sha256],
        )?;
        Ok(n > 0)
    }

    /// Whether a backend has ever seen the vault (ADR-0064): learned from a
    /// backend or created there. Never cleared, not even by a change of
    /// backend. An adoption holds only such vaults.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::Database`].
    pub fn vault_backend_known(&self, vault: &VaultId) -> Result<bool> {
        self.conn
            .query_row(
                "SELECT backend_known FROM vaults WHERE vault_id = ?1",
                params![vault.as_bytes()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::UnknownVault)
    }

    /// Whether the file was dismissed.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn file_dismissed(&self, path: &str, sha256: &[u8; 32]) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM rekey_dismissed_files WHERE path = ?1 AND sha256 = ?2)",
            params![path, sha256],
            |r| r.get(0),
        )?)
    }
}

fn insert_reseal_rows(
    tx: &Transaction<'_>,
    rows: &[(AttachmentId, VaultId, ItemId)],
) -> Result<()> {
    for (old, vault, item) in rows {
        tx.execute(
            "INSERT OR REPLACE INTO reseal_attachments (old_id, vault_id, item_id, new_id, state)
             VALUES (?1, ?2, ?3, NULL, ?4)",
            params![
                old.as_bytes(),
                vault.as_bytes(),
                item.as_bytes(),
                ResealState::Todo.code()
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_and_kinds_round_trip() {
        for n in 1..=7 {
            assert_eq!(
                i64::from(RotationPhase::from_number(n).unwrap().number()),
                n
            );
        }
        assert!(RotationPhase::from_number(0).is_none());
        assert!(RotationPhase::from_number(8).is_none());
        for k in 1..=4 {
            assert_eq!(i64::from(RotationKind::from_code(k).unwrap().code()), k);
        }
        assert!(RotationKind::from_code(5).is_none());
        // Kinds 1 and 3 have the phases 1 to 3 and 5 only.
        for kind in [RotationKind::SecretKey, RotationKind::Profile] {
            for (n, ok) in [
                (1, true),
                (2, true),
                (3, true),
                (4, false),
                (5, true),
                (6, false),
                (7, false),
            ] {
                assert_eq!(
                    RotationPhase::from_number(n).unwrap().valid_for(kind),
                    ok,
                    "{kind:?} {n}"
                );
            }
        }
        assert_eq!(FOLDER_MARKER_LEN, 93);
    }
}
