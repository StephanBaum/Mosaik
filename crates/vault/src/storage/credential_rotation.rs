//! Credential-rotation state of the local store (schema v10, CRYPTO_SPEC
//! §11): the breach check's cache, the last scan's summary and the
//! rotate-now queue's snooze and dismissal.
//!
//! These rows hold IDs, small integers and Lamport value marks. Never a
//! password, a hash or anything derived from one. They go with the item (a
//! trigger on `records`) and the vault (cascade). `reset_sync_state` and a
//! re-key do not touch them.

use mosaic_core::{ItemId, VaultId};
use rusqlite::{OptionalExtension, params};

use super::Store;
use super::error::{Error, Result};
use super::vault_id_from_sql;

/// Longest value mark the tables take (the CHECK of the schema).
const MAX_MARK_LEN: usize = 32;

/// One cached breach answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreachRow {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// 1 login password, 2 Wi-Fi password.
    pub field: u8,
    /// The Lamport mark of the checked value (1 to 32 bytes).
    pub mark: Vec<u8>,
    /// When it was checked (ms since the Unix epoch).
    pub checked_at_ms: i64,
    /// Whether the password was found in the data set.
    pub found: bool,
}

/// The summary of the last breach scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanRow {
    /// When it started.
    pub started_at_ms: Option<i64>,
    /// When it finished, if it did.
    pub finished_at_ms: Option<i64>,
    /// How many secrets it checked.
    pub checked: u32,
    /// Why it stopped early: 1 offline, 2 rate limited, 3 protocol,
    /// 4 cancelled.
    pub error: Option<u8>,
}

/// The snooze and dismissal of one item in the rotate-now queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueRow {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// Hidden until then (ms since the Unix epoch).
    pub snoozed_until_ms: Option<i64>,
    /// The password mark the user dismissed the entry at.
    pub dismissed_mark: Option<Vec<u8>>,
}

fn item_id_from_sql(bytes: &[u8]) -> Result<ItemId> {
    ItemId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

fn check_mark(mark: &[u8]) -> Result<()> {
    if mark.is_empty() || mark.len() > MAX_MARK_LEN {
        return Err(Error::Corrupt);
    }
    Ok(())
}

type BreachSql = (Vec<u8>, Vec<u8>, i64, Vec<u8>, i64, i64);

impl Store {
    /// Every cached breach answer.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Corrupt`].
    pub fn breach_rows(&self) -> Result<Vec<BreachRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, item_id, field, value_mark, checked_at, found FROM breach_state",
        )?;
        let rows = stmt.query_map([], |r| -> rusqlite::Result<BreachSql> {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (v, i, field, mark, at, found) = row?;
            out.push(BreachRow {
                vault_id: vault_id_from_sql(&v)?,
                item_id: item_id_from_sql(&i)?,
                field: u8::try_from(field).map_err(|_| Error::Corrupt)?,
                mark,
                checked_at_ms: at,
                found: found != 0,
            });
        }
        Ok(out)
    }

    /// Stores breach answers and the scan summary in one transaction. A row
    /// replaces the earlier row of the same item and field.
    ///
    /// # Errors
    /// [`Error::Corrupt`] for a mark outside 1 to 32 bytes or a field other
    /// than 1 or 2; [`Error::Database`] (for example an unknown vault).
    pub fn put_breach_results(&mut self, rows: &[BreachRow], scan: &ScanRow) -> Result<()> {
        for r in rows {
            check_mark(&r.mark)?;
            if !matches!(r.field, 1 | 2) {
                return Err(Error::Corrupt);
            }
        }
        let tx = self.write()?;
        for r in rows {
            tx.execute(
                "INSERT INTO breach_state (vault_id, item_id, field, value_mark, checked_at, found)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (vault_id, item_id, field) DO UPDATE SET
                     value_mark = excluded.value_mark,
                     checked_at = excluded.checked_at,
                     found = excluded.found",
                params![
                    r.vault_id.as_bytes(),
                    r.item_id.as_bytes(),
                    r.field,
                    r.mark,
                    r.checked_at_ms,
                    i64::from(r.found)
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO breach_scan (id, started_at, finished_at, checked, error)
             VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE SET
                 started_at = excluded.started_at,
                 finished_at = excluded.finished_at,
                 checked = excluded.checked,
                 error = excluded.error",
            params![
                scan.started_at_ms,
                scan.finished_at_ms,
                scan.checked,
                scan.error
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The last scan's summary, if a scan ran.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Corrupt`].
    pub fn breach_scan(&self) -> Result<Option<ScanRow>> {
        let row = self
            .conn
            .query_row(
                "SELECT started_at, finished_at, checked, error FROM breach_scan WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, Option<i64>>(0)?,
                        r.get::<_, Option<i64>>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(started, finished, checked, error)| {
            Ok(ScanRow {
                started_at_ms: started,
                finished_at_ms: finished,
                checked: u32::try_from(checked).map_err(|_| Error::Corrupt)?,
                error: error
                    .map(|e| u8::try_from(e).map_err(|_| Error::Corrupt))
                    .transpose()?,
            })
        })
        .transpose()
    }

    /// Every item's snooze and dismissal.
    ///
    /// # Errors
    /// [`Error::Database`], [`Error::Corrupt`].
    pub fn queue_rows(&self) -> Result<Vec<QueueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id, item_id, snoozed_until, dismissed_mark FROM rotation_queue_state",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<Vec<u8>>>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (v, i, snoozed, mark) = row?;
            out.push(QueueRow {
                vault_id: vault_id_from_sql(&v)?,
                item_id: item_id_from_sql(&i)?,
                snoozed_until_ms: snoozed,
                dismissed_mark: mark,
            });
        }
        Ok(out)
    }

    /// Hides an item from the queue until `until_ms`.
    ///
    /// # Errors
    /// [`Error::Database`] (for example an unknown vault).
    pub fn set_snooze(
        &mut self,
        vault_id: &VaultId,
        item_id: &ItemId,
        until_ms: i64,
    ) -> Result<()> {
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO rotation_queue_state (vault_id, item_id, snoozed_until)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (vault_id, item_id) DO UPDATE SET snoozed_until = excluded.snoozed_until",
            params![vault_id.as_bytes(), item_id.as_bytes(), until_ms],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Dismisses an item at a password mark.
    ///
    /// # Errors
    /// [`Error::Corrupt`] for a mark outside 1 to 32 bytes;
    /// [`Error::Database`].
    pub fn set_dismissed(
        &mut self,
        vault_id: &VaultId,
        item_id: &ItemId,
        mark: &[u8],
    ) -> Result<()> {
        check_mark(mark)?;
        let tx = self.write()?;
        tx.execute(
            "INSERT INTO rotation_queue_state (vault_id, item_id, dismissed_mark)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (vault_id, item_id) DO UPDATE SET dismissed_mark = excluded.dismissed_mark",
            params![vault_id.as_bytes(), item_id.as_bytes(), mark],
        )?;
        tx.commit()?;
        Ok(())
    }
}
