//! The `checkpoints` table: the latest checkpoint of each device, as raw
//! bytes (the server cannot verify the MAC; the clients do).

use rusqlite::{Connection, OptionalExtension, params};

use super::{id16, int};
use crate::error::ApiError;

/// Stores a checkpoint if its counter is above the stored one. False if it
/// is not (stale).
pub fn put(
    conn: &Connection,
    account: &[u8; 16],
    device: &[u8; 16],
    counter: u64,
    body: &[u8],
    now: u64,
) -> Result<bool, ApiError> {
    let stored: Option<i64> = conn
        .query_row(
            "SELECT counter FROM checkpoints WHERE account_id = ?1 AND device_id = ?2",
            params![account.as_slice(), device.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    if stored.is_some_and(|s| u64::try_from(s).is_ok_and(|s| counter <= s)) {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO checkpoints (account_id, device_id, counter, body, received_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT (account_id, device_id) DO UPDATE SET counter = excluded.counter, \
         body = excluded.body, received_at = excluded.received_at",
        params![
            account.as_slice(),
            device.as_slice(),
            int(counter),
            body,
            int(now)
        ],
    )?;
    Ok(true)
}

/// A device and its latest checkpoint bytes.
pub type Stored = ([u8; 16], Vec<u8>);

/// The latest checkpoint of every device that was not removed.
pub fn list(conn: &Connection, account: &[u8; 16]) -> Result<Vec<Stored>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT c.device_id, c.body FROM checkpoints c JOIN devices d \
         ON d.account_id = c.account_id AND d.device_id = c.device_id \
         WHERE c.account_id = ?1 AND d.revoked_at IS NULL ORDER BY c.device_id",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| Ok((id16(r, 0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}
