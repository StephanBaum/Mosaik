//! Agent access state of the local store (schema v13, CRYPTO_SPEC §11, ADR-0160): the keyed marks
//! that confirm an item's agent binding on this device, and the sealed agent log.
//!
//! The store sees only 32-byte marks and opaque sealed records with a cleartext time. It never
//! holds a key. Neither table is synced, queued in the outbox, exported or touched by
//! `reset_sync_state`; marks go with the item (a trigger on `records`) and the vault (cascade).

use mosaic_core::{ItemId, VaultId};
use rusqlite::params;

use super::Store;
use super::error::{Error, Result};

/// One stored log record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentLogRow {
    /// The entry ID (bound into the AAD).
    pub entry_id: [u8; 16],
    /// The entry's wall-clock time in ms (bound into the AAD).
    pub at_ms: u64,
    /// The sealed record.
    pub sealed: Vec<u8>,
}

fn to_sql_ms(at_ms: u64) -> Result<i64> {
    i64::try_from(at_ms).map_err(|_| Error::Corrupt)
}

impl Store {
    /// Whether `mark` is stored for the item.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn agent_mark_exists(
        &self,
        vault_id: &VaultId,
        item_id: &ItemId,
        mark: &[u8; 32],
    ) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM agent_marks
                            WHERE vault_id = ?1 AND item_id = ?2 AND mark = ?3)",
            params![vault_id.as_bytes(), item_id.as_bytes(), mark],
            |r| r.get(0),
        )?)
    }

    /// Stores a mark (a second insert of the same mark changes nothing).
    ///
    /// # Errors
    /// [`Error::Database`] (also for an unknown vault: the foreign key refuses).
    pub fn put_agent_mark(
        &mut self,
        vault_id: &VaultId,
        item_id: &ItemId,
        mark: &[u8; 32],
    ) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "INSERT OR IGNORE INTO agent_marks (vault_id, item_id, mark) VALUES (?1, ?2, ?3)",
            params![vault_id.as_bytes(), item_id.as_bytes(), mark],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Every mark stored for the item.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Corrupt`].
    pub fn agent_marks_of(&self, vault_id: &VaultId, item_id: &ItemId) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self.conn.prepare(
            "SELECT mark FROM agent_marks WHERE vault_id = ?1 AND item_id = ?2 ORDER BY mark",
        )?;
        let rows = stmt.query_map(params![vault_id.as_bytes(), item_id.as_bytes()], |r| {
            r.get::<_, Vec<u8>>(0)
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(<[u8; 32]>::try_from(row?.as_slice()).map_err(|_| Error::Corrupt)?);
        }
        Ok(out)
    }

    /// Deletes the item's marks that are not in `keep`.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn retain_agent_marks(
        &mut self,
        vault_id: &VaultId,
        item_id: &ItemId,
        keep: &[[u8; 32]],
    ) -> Result<()> {
        let stored = self.agent_marks_of(vault_id, item_id)?;
        let tx = self.write()?;
        for mark in stored.iter().filter(|m| !keep.contains(m)) {
            tx.execute(
                "DELETE FROM agent_marks WHERE vault_id = ?1 AND item_id = ?2 AND mark = ?3",
                params![vault_id.as_bytes(), item_id.as_bytes(), mark],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Adds a sealed log record, then prunes: records older than `min_at_ms` go, and so does
    /// everything beyond the newest `max_entries`. One transaction.
    ///
    /// # Errors
    /// [`Error::Database`] (also for a record outside 42 to 65536 bytes or a repeated entry ID),
    /// [`Error::Corrupt`] for a time beyond `i64`.
    pub fn append_agent_log(
        &mut self,
        row: &AgentLogRow,
        min_at_ms: u64,
        max_entries: usize,
    ) -> Result<()> {
        let at = to_sql_ms(row.at_ms)?;
        let min = to_sql_ms(min_at_ms)?;
        let keep = i64::try_from(max_entries).map_err(|_| Error::Corrupt)?;
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO agent_log (entry_id, at_ms, sealed) VALUES (?1, ?2, ?3)",
            params![row.entry_id, at, row.sealed],
        )?;
        tx.execute("DELETE FROM agent_log WHERE at_ms < ?1", [min])?;
        tx.execute(
            "DELETE FROM agent_log WHERE rowid IN
               (SELECT rowid FROM agent_log ORDER BY at_ms DESC, rowid DESC LIMIT -1 OFFSET ?1)",
            [keep],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Up to `limit` log rows older than `before_ms` (all when `None`), newest first.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Corrupt`].
    pub fn agent_log_rows(&self, before_ms: Option<u64>, limit: usize) -> Result<Vec<AgentLogRow>> {
        let before = before_ms.map_or(Ok(i64::MAX), to_sql_ms)?;
        let limit = i64::try_from(limit).map_err(|_| Error::Corrupt)?;
        let mut stmt = self.conn.prepare(
            "SELECT entry_id, at_ms, sealed FROM agent_log WHERE at_ms < ?1
             ORDER BY at_ms DESC, rowid DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![before, limit], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (entry_id, at_ms, sealed) = row?;
            out.push(AgentLogRow {
                entry_id: <[u8; 16]>::try_from(entry_id.as_slice()).map_err(|_| Error::Corrupt)?,
                at_ms: u64::try_from(at_ms).map_err(|_| Error::Corrupt)?,
                sealed,
            });
        }
        Ok(out)
    }
}
