//! The `audit` table: event type, time and device, no content.

use rusqlite::{Connection, params};

use super::int;
use crate::error::ApiError;

/// An audit event kind (CRYPTO_SPEC §12).
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Kind {
    Created = 1,
    Login = 2,
    TotpOn = 3,
    TotpOff = 4,
    PasswordChanged = 5,
    DeviceRemoved = 6,
}

/// Appends an event.
pub fn record(
    conn: &Connection,
    account: &[u8; 16],
    now: u64,
    kind: Kind,
    device: Option<&[u8; 16]>,
) -> Result<(), ApiError> {
    conn.execute(
        "INSERT INTO audit (account_id, at, kind, device_id) VALUES (?1, ?2, ?3, ?4)",
        params![
            account.as_slice(),
            int(now),
            kind as u8,
            device.map(<[u8; 16]>::as_slice)
        ],
    )?;
    Ok(())
}

/// Deletes events older than `before`.
pub fn purge(conn: &Connection, before: u64) -> Result<usize, ApiError> {
    Ok(conn.execute("DELETE FROM audit WHERE at < ?1", [int(before)])?)
}
