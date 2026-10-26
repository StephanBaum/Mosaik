//! Sync records: encrypted items and tombstones (CRYPTO_SPEC §7.2, §10),
//! with the outbox of local changes waiting for upload.

use mosaic_core::{ItemCiphertext, ItemId, KeyGeneration, TombstoneCiphertext, VaultId};
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::error::{Error, Result};
use super::vaults::require_vault;
use super::{
    PendingPolicy, Store, attachments, check_generation, generation_from_sql, now_ms, seq_from_sql,
    seq_to_sql, vault_id_from_sql,
};

/// What a record holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// An encrypted item ([`ItemCiphertext`]).
    Item,
    /// An encrypted deletion ([`TombstoneCiphertext`]).
    Tombstone,
}

impl RecordKind {
    /// The kind's stored code: 1 for an item, 2 for a tombstone.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Item => 1,
            Self::Tombstone => 2,
        }
    }

    /// Looks up a stored code.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Item),
            2 => Some(Self::Tombstone),
            _ => None,
        }
    }
}

/// A record's ciphertext: an item or a tombstone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordBody {
    /// An encrypted item.
    Item(ItemCiphertext),
    /// An encrypted tombstone.
    Tombstone(TombstoneCiphertext),
}

impl RecordBody {
    /// Strictly decodes a record ciphertext of the given kind, for example
    /// one received from the server.
    ///
    /// # Errors
    /// [`Error::Format`] with the `mosaic-core` decoding error.
    pub fn decode(kind: RecordKind, bytes: &[u8]) -> Result<Self> {
        match kind {
            RecordKind::Item => ItemCiphertext::from_bytes(bytes).map(Self::Item),
            RecordKind::Tombstone => TombstoneCiphertext::from_bytes(bytes).map(Self::Tombstone),
        }
        .map_err(Error::Format)
    }

    /// The record kind.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            Self::Item(_) => RecordKind::Item,
            Self::Tombstone(_) => RecordKind::Tombstone,
        }
    }

    /// The vault key generation in the ciphertext header.
    #[must_use]
    pub const fn key_generation(&self) -> KeyGeneration {
        match self {
            Self::Item(ct) => ct.key_generation(),
            Self::Tombstone(ct) => ct.key_generation(),
        }
    }

    /// The encoded ciphertext.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Item(ct) => ct.as_bytes(),
            Self::Tombstone(ct) => ct.as_bytes(),
        }
    }
}

/// Position of a local change in the outbox. Values increase with every
/// local write and are never reused, so an upload acknowledgement for an
/// older change cannot clear a newer one ([`Store::mark_uploaded`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutboxSeq(u64);

impl OutboxSeq {
    /// The sequence number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Where a record write comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A change made on this device. It goes into the outbox; the stored
    /// `server_seq` is kept.
    Local,
    /// A record pulled from the server, already authenticated and accepted
    /// by the caller. It sets `server_seq` and is not uploaded. Refused
    /// while the record has a pending local change.
    Remote {
        /// The server's sequence number for this version.
        server_seq: u64,
    },
    /// The caller's resolution of a remote record against a pending local
    /// change: it sets `server_seq` to the remote version's and goes into
    /// the outbox, in one transaction, so the local change is never lost.
    Merged {
        /// The server's sequence number of the remote version merged.
        server_seq: u64,
    },
}

/// A record as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRecord {
    /// The record's vault.
    pub vault_id: VaultId,
    /// The record ID, which is the item ID (§7.2).
    pub record_id: ItemId,
    /// The ciphertext. Its authenticated plaintext carries the record's
    /// highest accepted version vector (§10, ADR-0010).
    pub body: RecordBody,
    /// The server's sequence number of the last version synced, if any.
    pub server_seq: Option<u64>,
    /// Set while a local change waits for upload.
    pub outbox_seq: Option<OutboxSeq>,
    /// When the row was last written (ms since the Unix epoch).
    pub updated_at_ms: i64,
}

const SELECT_RECORD: &str = "SELECT vault_id, record_id, kind, key_generation, ciphertext,
    server_seq, outbox_seq, updated_at FROM records";

struct RawRecord {
    vault_id: Vec<u8>,
    record_id: Vec<u8>,
    kind: i64,
    generation: i64,
    ciphertext: Vec<u8>,
    server_seq: Option<i64>,
    outbox_seq: Option<i64>,
    updated_at: i64,
}

fn raw_record(r: &Row<'_>) -> rusqlite::Result<RawRecord> {
    Ok(RawRecord {
        vault_id: r.get(0)?,
        record_id: r.get(1)?,
        kind: r.get(2)?,
        generation: r.get(3)?,
        ciphertext: r.get(4)?,
        server_seq: r.get(5)?,
        outbox_seq: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

/// A stored record that failed validation on the way out (see
/// [`Error::Corrupt`]). Listings report it in place of the record, so one
/// damaged row does not hide the rest of the vault or block the outbox.
/// The caller can quarantine it, for example re-fetch it from the server
/// or delete it with [`Store::delete_record`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorruptRecord {
    /// The record's vault, if its stored ID is valid.
    pub vault_id: Option<VaultId>,
    /// The record ID, if its stored value is valid.
    pub record_id: Option<ItemId>,
}

/// One row of a record listing: the record, or the IDs of a corrupt one.
pub type RecordEntry = core::result::Result<StoredRecord, CorruptRecord>;

/// Decodes a listed row; a decoding failure becomes a [`CorruptRecord`].
fn entry(raw: &RawRecord) -> RecordEntry {
    let vault_id = VaultId::from_slice(&raw.vault_id).ok();
    let record_id = ItemId::from_slice(&raw.record_id).ok();
    decode_record(raw).map_err(|_| CorruptRecord {
        vault_id,
        record_id,
    })
}

fn decode_record(raw: &RawRecord) -> Result<StoredRecord> {
    let kind = u8::try_from(raw.kind)
        .ok()
        .and_then(RecordKind::from_code)
        .ok_or(Error::Corrupt)?;
    let body = RecordBody::decode(kind, &raw.ciphertext).map_err(|_| Error::Corrupt)?;
    if body.key_generation() != generation_from_sql(raw.generation)? {
        return Err(Error::Corrupt);
    }
    Ok(StoredRecord {
        vault_id: vault_id_from_sql(&raw.vault_id)?,
        record_id: ItemId::from_slice(&raw.record_id).map_err(|_| Error::Corrupt)?,
        body,
        server_seq: raw.server_seq.map(seq_from_sql).transpose()?,
        outbox_seq: raw
            .outbox_seq
            .map(|s| seq_from_sql(s).map(OutboxSeq))
            .transpose()?,
        updated_at_ms: raw.updated_at,
    })
}

/// The sync columns of an existing record: `(server_seq, outbox_seq)`.
fn sync_columns(
    conn: &Connection,
    vault_id: &VaultId,
    record_id: &ItemId,
) -> Result<Option<(Option<u64>, Option<u64>)>> {
    let row: Option<(Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT server_seq, outbox_seq FROM records WHERE vault_id = ?1 AND record_id = ?2",
            params![vault_id.as_bytes(), record_id.as_bytes()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(server, outbox)| {
        Ok((
            server.map(seq_from_sql).transpose()?,
            outbox.map(seq_from_sql).transpose()?,
        ))
    })
    .transpose()
}

fn check_server_seq(stored: Option<u64>, offered: u64) -> Result<()> {
    match stored {
        Some(stored) if stored > offered => Err(Error::StaleServerSeq { stored, offered }),
        _ => Ok(()),
    }
}

fn next_outbox_seq(conn: &Connection) -> Result<OutboxSeq> {
    let last: i64 = conn.query_row(
        "UPDATE outbox_counter SET last = last + 1 WHERE id = 1 RETURNING last",
        [],
        |r| r.get(0),
    )?;
    Ok(OutboxSeq(seq_from_sql(last)?))
}

/// Whether deleting the records matched by `filter` (an SQL condition on
/// `records`, with `?1` the vault ID and optionally `?2` the record ID)
/// would drop a pending local change: a record in the outbox, or an
/// attachment whose upload is still pending.
fn has_pending(conn: &Connection, filter: &str, args: impl rusqlite::Params) -> Result<bool> {
    Ok(conn.query_row(
        &format!(
            "SELECT EXISTS (SELECT 1 FROM records WHERE {filter} AND outbox_seq IS NOT NULL)
                 OR EXISTS (SELECT 1 FROM attachments WHERE {filter} AND upload_pending = 1)"
        ),
        args,
        |r| r.get(0),
    )?)
}

/// Refuses with [`Error::PendingLocalChange`] under [`PendingPolicy::Refuse`]
/// if the delete would drop a pending local change.
pub(super) fn check_pending(
    conn: &Connection,
    policy: PendingPolicy,
    filter: &str,
    args: impl rusqlite::Params,
) -> Result<()> {
    if policy == PendingPolicy::Refuse && has_pending(conn, filter, args)? {
        return Err(Error::PendingLocalChange);
    }
    Ok(())
}

impl Store {
    /// Inserts or replaces a record.
    ///
    /// The body must be sealed under the vault's highest accepted key
    /// generation (§7.1). The caller has already authenticated the record
    /// and checked its version vector against the one inside the stored
    /// ciphertext (§10): a dominated record is not written at all, a
    /// concurrent one is merged first. See [`Origin`] for how the sync
    /// columns change. Returns the outbox position for [`Origin::Local`]
    /// and [`Origin::Merged`].
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::KeyGenerationRollback`],
    /// [`Error::KeyGenerationNotAccepted`], [`Error::StaleServerSeq`] if a
    /// remote or merged `server_seq` is below the stored one,
    /// [`Error::PendingLocalChange`] for [`Origin::Remote`] while a local
    /// change waits for upload, [`Error::SeqOutOfRange`].
    pub fn upsert_record(
        &mut self,
        vault_id: &VaultId,
        record_id: &ItemId,
        body: &RecordBody,
        origin: Origin,
    ) -> Result<Option<OutboxSeq>> {
        let tx = self.write()?;
        let vault = require_vault(&tx, vault_id)?;
        check_generation(vault.highest_accepted_generation, body.key_generation())?;
        let (stored_server, stored_outbox) =
            sync_columns(&tx, vault_id, record_id)?.unwrap_or((None, None));
        let (server_seq, outbox_seq) = match origin {
            Origin::Local => (stored_server, Some(next_outbox_seq(&tx)?)),
            Origin::Remote { server_seq } => {
                check_server_seq(stored_server, server_seq)?;
                if stored_outbox.is_some() {
                    return Err(Error::PendingLocalChange);
                }
                (Some(server_seq), None)
            }
            Origin::Merged { server_seq } => {
                check_server_seq(stored_server, server_seq)?;
                (Some(server_seq), Some(next_outbox_seq(&tx)?))
            }
        };
        tx.execute(
            "INSERT INTO records (vault_id, record_id, kind, key_generation, ciphertext,
                                  server_seq, outbox_seq, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (vault_id, record_id) DO UPDATE SET
                 kind = excluded.kind,
                 key_generation = excluded.key_generation,
                 ciphertext = excluded.ciphertext,
                 server_seq = excluded.server_seq,
                 outbox_seq = excluded.outbox_seq,
                 updated_at = excluded.updated_at",
            params![
                vault_id.as_bytes(),
                record_id.as_bytes(),
                body.kind().code(),
                body.key_generation().get(),
                body.as_bytes(),
                server_seq.map(seq_to_sql).transpose()?,
                outbox_seq.map(|s| seq_to_sql(s.0)).transpose()?,
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(outbox_seq)
    }

    /// One record, if stored.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn record(&self, vault_id: &VaultId, record_id: &ItemId) -> Result<Option<StoredRecord>> {
        let raw = self
            .conn
            .query_row(
                &format!("{SELECT_RECORD} WHERE vault_id = ?1 AND record_id = ?2"),
                params![vault_id.as_bytes(), record_id.as_bytes()],
                raw_record,
            )
            .optional()?;
        raw.as_ref().map(decode_record).transpose()
    }

    fn list(&self, sql: &str, args: impl rusqlite::Params) -> Result<Vec<RecordEntry>> {
        let mut stmt = self.conn.prepare(&format!("{SELECT_RECORD} {sql}"))?;
        let rows = stmt.query_map(args, raw_record)?;
        rows.map(|raw| Ok(entry(&raw?))).collect()
    }

    /// Every record of a vault, ordered by record ID. A row that fails
    /// validation is reported as a [`CorruptRecord`] in its place, so it
    /// does not hide the others.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn records(&self, vault_id: &VaultId) -> Result<Vec<RecordEntry>> {
        self.list(
            "WHERE vault_id = ?1 ORDER BY record_id",
            [vault_id.as_bytes()],
        )
    }

    /// Up to `limit` records with a pending local change, oldest change
    /// first, across all vaults. A row that fails validation is reported as
    /// a [`CorruptRecord`] in its place, so it does not block the uploads
    /// behind it.
    ///
    /// A pending record below its vault's highest accepted generation (see
    /// [`Store::records_below_generation`]) is still listed: re-seal it
    /// under the current key with [`Origin::Local`] before uploading, or
    /// peers reject it as a rollback (§7.1).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn outbox(&self, limit: usize) -> Result<Vec<RecordEntry>> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        self.list(
            "WHERE outbox_seq IS NOT NULL ORDER BY outbox_seq LIMIT ?1",
            [limit],
        )
    }

    /// The records of a vault sealed under a generation below
    /// `generation`, ordered by record ID; corrupt rows are reported as in
    /// [`Store::records`].
    ///
    /// After [`Store::accept_vault_key`] raises the accepted generation,
    /// the ones with a pending local change (`outbox_seq` set) must be
    /// re-sealed under the new key and stored again with
    /// [`Origin::Local`]: peers that have accepted the new generation
    /// reject the old ciphertext (§7.1), so the edit would be lost. The
    /// others are re-encrypted by the vault owner's rotation (§8.3).
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn records_below_generation(
        &self,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<Vec<RecordEntry>> {
        self.list(
            "WHERE vault_id = ?1 AND key_generation < ?2 ORDER BY record_id",
            params![vault_id.as_bytes(), generation.get()],
        )
    }

    /// Records that the server stored the local change at `outbox_seq` as
    /// `server_seq`. The change leaves the outbox only if it is still the
    /// pending one; if a newer local change was made meanwhile, that one
    /// stays queued (and `false` is returned). `server_seq` is stored
    /// either way.
    ///
    /// # Errors
    /// [`Error::UnknownRecord`], [`Error::StaleServerSeq`],
    /// [`Error::SeqOutOfRange`].
    pub fn mark_uploaded(
        &mut self,
        vault_id: &VaultId,
        record_id: &ItemId,
        outbox_seq: OutboxSeq,
        server_seq: u64,
    ) -> Result<bool> {
        let seq = seq_to_sql(server_seq)?;
        let tx = self.write()?;
        let (stored_server, stored_outbox) =
            sync_columns(&tx, vault_id, record_id)?.ok_or(Error::UnknownRecord)?;
        check_server_seq(stored_server, server_seq)?;
        let cleared = stored_outbox == Some(outbox_seq.0);
        tx.execute(
            "UPDATE records SET server_seq = ?3,
                 outbox_seq = CASE WHEN ?4 THEN NULL ELSE outbox_seq END
             WHERE vault_id = ?1 AND record_id = ?2",
            params![vault_id.as_bytes(), record_id.as_bytes(), seq, cleared],
        )?;
        tx.commit()?;
        Ok(cleared)
    }

    /// Removes a record's row and its attachments from this store. Returns
    /// whether the record existed. It also works on a corrupt row.
    ///
    /// This is a local purge, not a synced deletion. A deletion the user
    /// makes, including a hard delete from the trash (§7.4), is stored as a
    /// tombstone with [`Store::upsert_record`] ([`RecordBody::Tombstone`]),
    /// which keeps the record's rollback anchor: its authenticated final
    /// version vector (§10). Purging the row drops that anchor, so a
    /// server that replays an older, validly sealed version of the record
    /// at the current generation would afterwards look like a new record.
    /// Purge only what the server no longer serves either.
    ///
    /// # Errors
    /// [`Error::PendingLocalChange`] under [`PendingPolicy::Refuse`] if the
    /// record is in the outbox or has an attachment not yet uploaded;
    /// [`Error::Database`], [`Error::Io`].
    pub fn delete_record(
        &mut self,
        vault_id: &VaultId,
        record_id: &ItemId,
        policy: PendingPolicy,
    ) -> Result<bool> {
        let dir = self.attachment_dir.clone();
        let tx = self.write()?;
        let args = params![vault_id.as_bytes(), record_id.as_bytes()];
        check_pending(&tx, policy, "vault_id = ?1 AND record_id = ?2", args)?;
        let ids = attachments::ids_for_record(&tx, vault_id, record_id)?;
        let n = tx.execute(
            "DELETE FROM records WHERE vault_id = ?1 AND record_id = ?2",
            args,
        )?;
        tx.commit()?;
        attachments::remove_files(dir.as_deref(), &ids)?;
        Ok(n > 0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::vaults::tests::{Fixture, fixture, g};
    use super::*;
    use mosaic_core::VaultKey;

    pub(crate) fn item(f: &Fixture, vk: &VaultKey, id: &ItemId, pt: &[u8]) -> RecordBody {
        RecordBody::Item(ItemCiphertext::seal(vk, &f.vault, id, 1, pt).unwrap())
    }

    pub(crate) fn tombstone(f: &Fixture, vk: &VaultKey, id: &ItemId) -> RecordBody {
        RecordBody::Tombstone(TombstoneCiphertext::seal(vk, &f.vault, id, b"gone").unwrap())
    }

    pub(crate) fn store_with_vault() -> (Store, Fixture, VaultKey) {
        let mut s = Store::open_in_memory().unwrap();
        let f = fixture();
        let (vk, w) = f.key(1);
        s.create_vault(&f.vault, g(1), &w).unwrap();
        (s, f, vk)
    }

    #[test]
    fn kinds_and_bodies() {
        assert_eq!(RecordKind::from_code(1), Some(RecordKind::Item));
        assert_eq!(RecordKind::from_code(2), Some(RecordKind::Tombstone));
        assert_eq!(RecordKind::from_code(0), None);
        assert_eq!(RecordKind::Tombstone.code(), 2);
        let (_, f, vk) = store_with_vault();
        let id = ItemId::new_v7().unwrap();
        let body = item(&f, &vk, &id, b"x");
        assert_eq!(
            RecordBody::decode(RecordKind::Item, body.as_bytes()).unwrap(),
            body
        );
        assert!(matches!(
            RecordBody::decode(RecordKind::Item, b""),
            Err(Error::Format(_))
        ));
        assert!(matches!(
            RecordBody::decode(RecordKind::Tombstone, &[2; 64]),
            Err(Error::Format(_))
        ));
        let t = tombstone(&f, &vk, &id);
        assert_eq!(t.kind(), RecordKind::Tombstone);
        assert_eq!(t.key_generation(), g(1));
        assert_eq!(
            RecordBody::decode(RecordKind::Tombstone, t.as_bytes()).unwrap(),
            t
        );
        assert_eq!(OutboxSeq(3).get(), 3);
    }

    #[test]
    #[allow(clippy::too_many_lines, reason = "one scenario, step by step")]
    fn local_remote_merged_and_upload() {
        let (mut s, f, vk) = store_with_vault();
        let id = ItemId::new_v7().unwrap();
        let v1 = item(&f, &vk, &id, b"v1");
        assert_eq!(s.record(&f.vault, &id).unwrap(), None);

        // Local write: queued.
        let q1 = s
            .upsert_record(&f.vault, &id, &v1, Origin::Local)
            .unwrap()
            .unwrap();
        let r = s.record(&f.vault, &id).unwrap().unwrap();
        assert_eq!(r.body, v1);
        assert_eq!(r.server_seq, None);
        assert_eq!(r.outbox_seq, Some(q1));
        assert_eq!(s.outbox(10).unwrap(), vec![Ok(r)]);

        // A newer local edit before the upload finished.
        let v2 = item(&f, &vk, &id, b"v2");
        let q2 = s
            .upsert_record(&f.vault, &id, &v2, Origin::Local)
            .unwrap()
            .unwrap();
        assert!(q2 > q1);
        // The ack for v1 does not clear v2.
        assert!(!s.mark_uploaded(&f.vault, &id, q1, 5).unwrap());
        let r = s.record(&f.vault, &id).unwrap().unwrap();
        assert_eq!((r.server_seq, r.outbox_seq), (Some(5), Some(q2)));
        // A remote version cannot overwrite the pending local change.
        let remote = item(&f, &vk, &id, b"remote");
        assert!(matches!(
            s.upsert_record(&f.vault, &id, &remote, Origin::Remote { server_seq: 6 }),
            Err(Error::PendingLocalChange)
        ));
        // Merged resolution keeps it queued and records the remote seq.
        let merged = item(&f, &vk, &id, b"merged");
        let q3 = s
            .upsert_record(&f.vault, &id, &merged, Origin::Merged { server_seq: 6 })
            .unwrap()
            .unwrap();
        assert!(q3 > q2);
        assert!(matches!(
            s.mark_uploaded(&f.vault, &id, q3, 4),
            Err(Error::StaleServerSeq {
                stored: 6,
                offered: 4
            })
        ));
        assert!(s.mark_uploaded(&f.vault, &id, q3, 7).unwrap());
        let r = s.record(&f.vault, &id).unwrap().unwrap();
        assert_eq!((r.server_seq, r.outbox_seq), (Some(7), None));
        assert_eq!(s.outbox(10).unwrap().len(), 0);

        // Remote apply: stale seq refused, equal and newer accepted.
        assert!(matches!(
            s.upsert_record(&f.vault, &id, &remote, Origin::Remote { server_seq: 6 }),
            Err(Error::StaleServerSeq { .. })
        ));
        assert!(matches!(
            s.upsert_record(&f.vault, &id, &remote, Origin::Merged { server_seq: 6 }),
            Err(Error::StaleServerSeq { .. })
        ));
        assert_eq!(
            s.upsert_record(&f.vault, &id, &remote, Origin::Remote { server_seq: 7 })
                .unwrap(),
            None
        );
        let t = tombstone(&f, &vk, &id);
        s.upsert_record(&f.vault, &id, &t, Origin::Remote { server_seq: 9 })
            .unwrap();
        let r = s.record(&f.vault, &id).unwrap().unwrap();
        assert_eq!(r.body.kind(), RecordKind::Tombstone);
        assert_eq!(r.server_seq, Some(9));
        assert!(r.updated_at_ms > 0);

        assert!(matches!(
            s.mark_uploaded(&f.vault, &ItemId::new_v7().unwrap(), q1, 1),
            Err(Error::UnknownRecord)
        ));
        assert!(matches!(
            s.mark_uploaded(&f.vault, &id, q1, u64::MAX),
            Err(Error::SeqOutOfRange)
        ));
        assert!(matches!(
            s.upsert_record(
                &f.vault,
                &id,
                &t,
                Origin::Remote {
                    server_seq: u64::MAX
                }
            ),
            Err(Error::SeqOutOfRange)
        ));

        assert!(
            s.delete_record(&f.vault, &id, PendingPolicy::Refuse)
                .unwrap()
        );
        assert!(
            !s.delete_record(&f.vault, &id, PendingPolicy::Refuse)
                .unwrap()
        );
        assert_eq!(s.record(&f.vault, &id).unwrap(), None);
    }

    /// BRIEF §5 "never silently lose data": a delete refuses to drop a
    /// change still waiting in the outbox unless told to discard it.
    #[test]
    fn deletes_never_drop_pending_changes() {
        let (mut s, f, vk) = store_with_vault();
        let id = ItemId::new_v7().unwrap();
        let q = s
            .upsert_record(&f.vault, &id, &item(&f, &vk, &id, b"x"), Origin::Local)
            .unwrap()
            .unwrap();
        assert!(matches!(
            s.delete_record(&f.vault, &id, PendingPolicy::Refuse),
            Err(Error::PendingLocalChange)
        ));
        assert!(matches!(
            s.delete_vault(&f.vault, PendingPolicy::Refuse),
            Err(Error::PendingLocalChange)
        ));
        assert_eq!(s.outbox(10).unwrap().len(), 1);
        assert!(s.record(&f.vault, &id).unwrap().is_some());
        assert!(s.vault(&f.vault).unwrap().is_some());

        // Once uploaded, the delete goes through.
        assert!(s.mark_uploaded(&f.vault, &id, q, 1).unwrap());
        assert!(
            s.delete_record(&f.vault, &id, PendingPolicy::Refuse)
                .unwrap()
        );

        // An explicit discard drops the pending change.
        let other = ItemId::new_v7().unwrap();
        s.upsert_record(
            &f.vault,
            &other,
            &item(&f, &vk, &other, b"y"),
            Origin::Local,
        )
        .unwrap();
        assert!(
            s.delete_record(&f.vault, &other, PendingPolicy::Discard)
                .unwrap()
        );
        assert_eq!(s.outbox(10).unwrap().len(), 0);
        s.upsert_record(
            &f.vault,
            &other,
            &item(&f, &vk, &other, b"z"),
            Origin::Local,
        )
        .unwrap();
        assert!(s.delete_vault(&f.vault, PendingPolicy::Discard).unwrap());
        assert_eq!(s.outbox(10).unwrap().len(), 0);
        assert!(!s.delete_vault(&f.vault, PendingPolicy::Refuse).unwrap());
    }

    #[test]
    fn generation_rules() {
        let (mut s, f, vk1) = store_with_vault();
        let id = ItemId::new_v7().unwrap();
        let other = VaultId::new_v7().unwrap();
        let body = item(&f, &vk1, &id, b"x");
        assert!(matches!(
            s.upsert_record(&other, &id, &body, Origin::Local),
            Err(Error::UnknownVault)
        ));
        s.upsert_record(&f.vault, &id, &body, Origin::Local)
            .unwrap();

        let (vk2, w2) = f.key(2);
        let ahead = item(&f, &vk2, &id, b"y");
        assert!(matches!(
            s.upsert_record(&f.vault, &id, &ahead, Origin::Local),
            Err(Error::KeyGenerationNotAccepted {
                highest_accepted: 1,
                offered: 2
            })
        ));
        s.accept_vault_key(&f.vault, g(2), &w2).unwrap();
        // The generation-1 record is still readable but cannot be written.
        assert_eq!(s.record(&f.vault, &id).unwrap().unwrap().body, body);
        assert!(matches!(
            s.upsert_record(&f.vault, &id, &body, Origin::Remote { server_seq: 1 }),
            Err(Error::KeyGenerationRollback {
                highest_accepted: 2,
                offered: 1
            })
        ));
        assert!(matches!(
            s.prune_vault_keys(&f.vault, g(2)),
            Err(Error::VaultKeyInUse)
        ));
        s.upsert_record(&f.vault, &id, &ahead, Origin::Local)
            .unwrap();
        assert_eq!(s.prune_vault_keys(&f.vault, g(2)).unwrap(), 1);
        assert_eq!(s.records(&f.vault).unwrap().len(), 1);
    }

    /// §7.1: after a rotation, pending local edits sealed under the old
    /// generation are listed so they can be re-sealed instead of lost.
    #[test]
    fn records_below_generation_after_rotation() {
        let (mut s, f, vk1) = store_with_vault();
        let mut ids: Vec<ItemId> = (0..3).map(|_| ItemId::new_v7().unwrap()).collect();
        ids.sort();
        let [pending, synced, fresh] = [ids[0], ids[1], ids[2]];
        s.upsert_record(
            &f.vault,
            &pending,
            &item(&f, &vk1, &pending, b"edit"),
            Origin::Local,
        )
        .unwrap();
        s.upsert_record(
            &f.vault,
            &synced,
            &item(&f, &vk1, &synced, b"s"),
            Origin::Remote { server_seq: 1 },
        )
        .unwrap();
        assert_eq!(s.records_below_generation(&f.vault, g(1)).unwrap(), vec![]);

        let (vk2, w2) = f.key(2);
        s.accept_vault_key(&f.vault, g(2), &w2).unwrap();
        s.upsert_record(
            &f.vault,
            &fresh,
            &item(&f, &vk2, &fresh, b"f"),
            Origin::Local,
        )
        .unwrap();
        let below: Vec<_> = s
            .records_below_generation(&f.vault, g(2))
            .unwrap()
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            below.iter().map(|r| r.record_id).collect::<Vec<_>>(),
            [pending, synced]
        );
        let stale: Vec<_> = below.iter().filter(|r| r.outbox_seq.is_some()).collect();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].record_id, pending);

        // Re-sealed under the new key, the edit stays queued and is no
        // longer below the generation.
        let q = s
            .upsert_record(
                &f.vault,
                &pending,
                &item(&f, &vk2, &pending, b"edit"),
                Origin::Local,
            )
            .unwrap();
        assert!(q.is_some());
        let below = s.records_below_generation(&f.vault, g(2)).unwrap();
        assert_eq!(below.len(), 1);
        assert_eq!(below[0].as_ref().unwrap().record_id, synced);
    }

    #[test]
    fn records_listing_and_outbox_order() {
        let (mut s, f, vk) = store_with_vault();
        let mut ids: Vec<ItemId> = (0..5).map(|_| ItemId::new_v7().unwrap()).collect();
        ids.sort();
        // Queue in reverse ID order: the outbox follows write order, the
        // listing follows ID order.
        for id in ids.iter().rev() {
            s.upsert_record(&f.vault, id, &item(&f, &vk, id, b"x"), Origin::Local)
                .unwrap();
        }
        let listed: Vec<_> = s
            .records(&f.vault)
            .unwrap()
            .into_iter()
            .map(|r| r.unwrap().record_id)
            .collect();
        assert_eq!(listed, ids);
        let queued: Vec<_> = s
            .outbox(3)
            .unwrap()
            .into_iter()
            .map(|r| r.unwrap().record_id)
            .collect();
        assert_eq!(
            queued,
            ids.iter().rev().take(3).copied().collect::<Vec<_>>()
        );
        assert_eq!(s.outbox(usize::MAX).unwrap().len(), 5);
        assert_eq!(s.outbox(0).unwrap().len(), 0);
    }

    /// One damaged row is reported in place; it does not hide the other
    /// records or block the outbox, and it can be purged.
    #[test]
    fn corrupt_records_are_reported() {
        let (mut s, f, vk) = store_with_vault();
        let mut ids: Vec<ItemId> = (0..3).map(|_| ItemId::new_v7().unwrap()).collect();
        ids.sort();
        for id in &ids {
            s.upsert_record(&f.vault, id, &item(&f, &vk, id, b"x"), Origin::Local)
                .unwrap();
        }
        let bad = ids[1];
        // Kind says tombstone, bytes are an item.
        s.conn
            .execute(
                "UPDATE records SET kind = 2 WHERE record_id = ?1",
                [bad.as_bytes()],
            )
            .unwrap();
        assert!(matches!(s.record(&f.vault, &bad), Err(Error::Corrupt)));
        let corrupt = CorruptRecord {
            vault_id: Some(f.vault),
            record_id: Some(bad),
        };
        for listing in [s.records(&f.vault).unwrap(), s.outbox(10).unwrap()] {
            assert_eq!(listing.len(), 3);
            assert_eq!(listing[1], Err(corrupt));
            assert_eq!(listing[0].as_ref().unwrap().record_id, ids[0]);
            assert_eq!(listing[2].as_ref().unwrap().record_id, ids[2]);
        }
        s.conn
            .execute(
                "UPDATE records SET kind = 1 WHERE record_id = ?1",
                [bad.as_bytes()],
            )
            .unwrap();
        assert!(s.record(&f.vault, &bad).is_ok());
        // Column generation disagrees with the header (trigger bypassed by
        // dropping it, as an outside writer could).
        s.conn
            .execute_batch("DROP TRIGGER records_generation_update;")
            .unwrap();
        s.conn
            .execute(
                "UPDATE records SET key_generation = 5 WHERE record_id = ?1",
                [bad.as_bytes()],
            )
            .unwrap();
        assert_eq!(s.records(&f.vault).unwrap()[1], Err(corrupt));
        assert_eq!(
            s.records_below_generation(&f.vault, g(9)).unwrap()[1],
            Err(corrupt)
        );
        assert_ne!(s.integrity_check().unwrap().len(), 0);
        // The corrupt row can be purged on its own.
        assert!(
            s.delete_record(&f.vault, &bad, PendingPolicy::Discard)
                .unwrap()
        );
        assert!(
            s.records(&f.vault)
                .unwrap()
                .iter()
                .all(core::result::Result::is_ok)
        );

        let raw = |vault_id: Vec<u8>, record_id: Vec<u8>, kind| RawRecord {
            vault_id,
            record_id,
            kind,
            generation: 1,
            ciphertext: vec![],
            server_seq: None,
            outbox_seq: None,
            updated_at: 0,
        };
        assert!(matches!(
            decode_record(&raw(vec![1; 16], vec![1; 16], 7)),
            Err(Error::Corrupt)
        ));
        assert_eq!(
            entry(&raw(vec![0; 16], vec![1; 3], 1)),
            Err(CorruptRecord {
                vault_id: None,
                record_id: None
            })
        );
    }
}
