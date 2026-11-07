//! The `attachments` table (the bytes live in [`crate::blobs`]).

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::{accounts, devices, id16, int, u64_col, vaults};
use crate::error::ApiError;
use mosaic_sync::wire::json::codes;

/// The row of an attachment.
pub struct Row {
    pub vault_id: [u8; 16],
    pub size: u64,
    pub deleted_at: Option<u64>,
}

/// Looks an attachment up by ID.
pub fn get(conn: &Connection, id: &[u8; 16]) -> Result<Option<Row>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT vault_id, size, deleted_at FROM attachments WHERE attachment_id = ?1",
            [id.as_slice()],
            |r| {
                let d: Option<i64> = r.get(2)?;
                Ok(Row {
                    vault_id: id16(r, 0)?,
                    size: u64_col(r, 1)?,
                    deleted_at: d.and_then(|d| u64::try_from(d).ok()),
                })
            },
        )
        .optional()?)
}

/// An attachment of a vault: ID, size, soft-delete time.
pub type Listed = ([u8; 16], u64, Option<u64>);

/// Lists a vault's attachments: `(id, size, deleted_at)`.
pub fn list(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
) -> Result<Vec<Listed>, ApiError> {
    vaults::access(conn, account, vault)?;
    let mut stmt = conn.prepare(
        "SELECT attachment_id, size, deleted_at FROM attachments WHERE vault_id = ?1 ORDER BY attachment_id",
    )?;
    let rows = stmt.query_map([vault.as_slice()], |r| {
        let d: Option<i64> = r.get(2)?;
        Ok((
            id16(r, 0)?,
            u64_col(r, 1)?,
            d.and_then(|d| u64::try_from(d).ok()),
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Whether `put` stored a new row.
pub enum Put {
    /// A new attachment (the caller writes the blob).
    Created,
    /// The same attachment is already stored (the caller rewrites the blob
    /// only if the file is missing).
    Existing,
}

/// Registers an attachment of `size` bytes. `attachment_exists` if the ID
/// is used with another size or in another vault; `quota_exceeded` past the
/// account's attachment bytes. An identical, soft-deleted attachment is
/// restored.
pub fn put(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    vault: &[u8; 16],
    id: &[u8; 16],
    size: u64,
    quota_bytes: u64,
    now: u64,
) -> Result<Put, ApiError> {
    vaults::access(tx, account, vault)?;
    if let Some(row) = get(tx, id)? {
        if row.vault_id != *vault || row.size != size {
            return Err(ApiError::conflict(codes::ATTACHMENT_EXISTS));
        }
        if row.deleted_at.is_some() {
            tx.execute(
                "UPDATE attachments SET deleted_at = NULL WHERE attachment_id = ?1",
                [id.as_slice()],
            )?;
        }
        return Ok(Put::Existing);
    }
    let usage = accounts::usage(tx, account)?;
    if usage.attachment_bytes.saturating_add(size) > quota_bytes {
        return Err(devices::quota("attachmentBytes"));
    }
    tx.execute(
        "INSERT INTO attachments (attachment_id, vault_id, size, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id.as_slice(), vault.as_slice(), int(size), int(now)],
    )?;
    Ok(Put::Created)
}

/// Soft-deletes an attachment of a vault the account can use. Absent is
/// fine.
pub fn soft_delete(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
    id: &[u8; 16],
    now: u64,
) -> Result<(), ApiError> {
    vaults::access(conn, account, vault)?;
    conn.execute(
        "UPDATE attachments SET deleted_at = ?1 WHERE attachment_id = ?2 AND vault_id = ?3 AND deleted_at IS NULL",
        params![int(now), id.as_slice(), vault.as_slice()],
    )?;
    Ok(())
}

/// Clears the soft delete; false if the attachment is absent.
pub fn restore(
    conn: &Connection,
    account: &[u8; 16],
    vault: &[u8; 16],
    id: &[u8; 16],
) -> Result<bool, ApiError> {
    vaults::access(conn, account, vault)?;
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM attachments WHERE attachment_id = ?1 AND vault_id = ?2",
        params![id.as_slice(), vault.as_slice()],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Ok(false);
    }
    conn.execute(
        "UPDATE attachments SET deleted_at = NULL WHERE attachment_id = ?1",
        [id.as_slice()],
    )?;
    Ok(true)
}

/// Deletes the rows soft-deleted before `before` and returns their IDs (the
/// caller removes the files).
pub fn purge_deleted(tx: &Transaction<'_>, before: u64) -> Result<Vec<[u8; 16]>, ApiError> {
    let mut stmt = tx.prepare(
        "SELECT attachment_id FROM attachments WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
    )?;
    let ids = stmt
        .query_map([int(before)], |r| id16(r, 0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    tx.execute(
        "DELETE FROM attachments WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
        [int(before)],
    )?;
    Ok(ids)
}

/// Removes the row of an attachment whose blob could not be written.
pub fn remove_row(conn: &Connection, id: &[u8; 16]) -> Result<(), ApiError> {
    conn.execute(
        "DELETE FROM attachments WHERE attachment_id = ?1",
        [id.as_slice()],
    )?;
    Ok(())
}
