//! The `devices` table.

use axum::http::StatusCode;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::{id16, int, u64_col};
use crate::config::MAX_DEVICES_PER_ACCOUNT;
use crate::error::ApiError;
use mosaic_sync::wire::json::{QuotaDetail, codes};

/// A device row.
pub struct DeviceRow {
    pub device_id: [u8; 16],
    pub name: String,
    pub created_at: u64,
    pub last_seen_at: u64,
}

/// A `quota_exceeded` error naming the quota.
pub fn quota(kind: &str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, codes::QUOTA_EXCEEDED).with_detail(&QuotaDetail {
        kind: kind.to_owned(),
    })
}

/// Registers the device of a login, or refreshes `last_seen_at`.
/// `device_revoked` if the device was removed; `quota_exceeded` past the
/// device limit.
pub fn upsert_on_login(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    device: &[u8; 16],
    name: &str,
    now: u64,
) -> Result<(), ApiError> {
    let existing: Option<Option<i64>> = tx
        .query_row(
            "SELECT revoked_at FROM devices WHERE account_id = ?1 AND device_id = ?2",
            params![account.as_slice(), device.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        Some(Some(_)) => Err(ApiError::new(StatusCode::FORBIDDEN, codes::DEVICE_REVOKED)),
        Some(None) => {
            tx.execute(
                "UPDATE devices SET last_seen_at = ?1 WHERE account_id = ?2 AND device_id = ?3",
                params![int(now), account.as_slice(), device.as_slice()],
            )?;
            Ok(())
        }
        None => {
            let n: i64 = tx.query_row(
                "SELECT COUNT(*) FROM devices WHERE account_id = ?1 AND revoked_at IS NULL",
                [account.as_slice()],
                |r| r.get(0),
            )?;
            if u64::try_from(n).unwrap_or(u64::MAX) >= MAX_DEVICES_PER_ACCOUNT {
                return Err(quota("devices"));
            }
            tx.execute(
                "INSERT INTO devices (account_id, device_id, name, created_at, last_seen_at) \
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![account.as_slice(), device.as_slice(), name, int(now)],
            )?;
            Ok(())
        }
    }
}

/// The account's devices that were not removed.
pub fn list(conn: &Connection, account: &[u8; 16]) -> Result<Vec<DeviceRow>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT device_id, name, created_at, last_seen_at FROM devices \
         WHERE account_id = ?1 AND revoked_at IS NULL ORDER BY created_at, device_id",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| {
        Ok(DeviceRow {
            device_id: id16(r, 0)?,
            name: r.get(1)?,
            created_at: u64_col(r, 2)?,
            last_seen_at: u64_col(r, 3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Renames a device; false if it does not exist or was removed.
pub fn rename(
    conn: &Connection,
    account: &[u8; 16],
    device: &[u8; 16],
    name: &str,
) -> Result<bool, ApiError> {
    let n = conn.execute(
        "UPDATE devices SET name = ?1 WHERE account_id = ?2 AND device_id = ?3 AND revoked_at IS NULL",
        params![name, account.as_slice(), device.as_slice()],
    )?;
    Ok(n == 1)
}

/// Removes a device: its sessions and checkpoint go, and later logins with
/// its ID fail. False if there is no such active device.
pub fn revoke(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    device: &[u8; 16],
    now: u64,
) -> Result<bool, ApiError> {
    let n = tx.execute(
        "UPDATE devices SET revoked_at = ?1 WHERE account_id = ?2 AND device_id = ?3 AND revoked_at IS NULL",
        params![int(now), account.as_slice(), device.as_slice()],
    )?;
    if n != 1 {
        return Ok(false);
    }
    super::sessions::delete_for_device(tx, account, device)?;
    tx.execute(
        "DELETE FROM checkpoints WHERE account_id = ?1 AND device_id = ?2",
        params![account.as_slice(), device.as_slice()],
    )?;
    Ok(true)
}
