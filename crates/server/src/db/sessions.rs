//! The `sessions` table. Only SHA-256 hashes of tokens are stored.

use rusqlite::{Connection, OptionalExtension, params};

use super::{id16, int, u64_col};
use crate::error::ApiError;

/// A live session.
pub struct SessionRow {
    pub account_id: [u8; 16],
    pub device_id: [u8; 16],
    pub fresh_until: u64,
    pub pending_2fa: bool,
}

/// What a new session is created with.
pub struct NewSession<'a> {
    pub token_hash: &'a [u8; 32],
    pub account: &'a [u8; 16],
    pub device: &'a [u8; 16],
    pub now: u64,
    pub expires_at: u64,
    pub fresh_until: u64,
    pub pending: bool,
}

/// Stores a new session.
pub fn insert(conn: &Connection, s: &NewSession<'_>) -> Result<(), ApiError> {
    conn.execute(
        "INSERT INTO sessions (token_hash, account_id, device_id, created_at, last_used_at, \
         expires_at, fresh_until, pending_2fa, totp_attempts) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, 0)",
        params![
            s.token_hash.as_slice(),
            s.account.as_slice(),
            s.device.as_slice(),
            int(s.now),
            int(s.expires_at),
            int(s.fresh_until),
            i64::from(s.pending)
        ],
    )?;
    Ok(())
}

/// Finds a live session and slides its idle expiry. A session past its idle
/// or absolute lifetime is deleted and not returned.
pub fn lookup_and_touch(
    conn: &Connection,
    token_hash: &[u8; 32],
    now: u64,
    idle_ms: u64,
    max_ms: u64,
) -> Result<Option<SessionRow>, ApiError> {
    let found = conn
        .query_row(
            "SELECT account_id, device_id, created_at, expires_at, fresh_until, pending_2fa \
             FROM sessions WHERE token_hash = ?1",
            [token_hash.as_slice()],
            |r| {
                Ok((
                    SessionRow {
                        account_id: id16(r, 0)?,
                        device_id: id16(r, 1)?,
                        fresh_until: u64_col(r, 4)?,
                        pending_2fa: r.get::<_, i64>(5)? != 0,
                    },
                    u64_col(r, 2)?,
                    u64_col(r, 3)?,
                ))
            },
        )
        .optional()?;
    let Some((row, created, expires)) = found else {
        return Ok(None);
    };
    let absolute = created.saturating_add(max_ms);
    if expires <= now || absolute <= now {
        delete(conn, token_hash)?;
        return Ok(None);
    }
    let new_expiry = now.saturating_add(idle_ms).min(absolute);
    conn.execute(
        "UPDATE sessions SET last_used_at = ?1, expires_at = ?2 WHERE token_hash = ?3",
        params![int(now), int(new_expiry), token_hash.as_slice()],
    )?;
    Ok(Some(row))
}

/// Deletes one session.
pub fn delete(conn: &Connection, token_hash: &[u8; 32]) -> Result<(), ApiError> {
    conn.execute(
        "DELETE FROM sessions WHERE token_hash = ?1",
        [token_hash.as_slice()],
    )?;
    Ok(())
}

/// Deletes every session of a device.
pub fn delete_for_device(
    conn: &Connection,
    account: &[u8; 16],
    device: &[u8; 16],
) -> Result<(), ApiError> {
    conn.execute(
        "DELETE FROM sessions WHERE account_id = ?1 AND device_id = ?2",
        params![account.as_slice(), device.as_slice()],
    )?;
    Ok(())
}

/// Counts a wrong TOTP code on a pending session and returns the new count.
pub fn bump_attempts(conn: &Connection, token_hash: &[u8; 32]) -> Result<u32, ApiError> {
    conn.execute(
        "UPDATE sessions SET totp_attempts = totp_attempts + 1 WHERE token_hash = ?1",
        [token_hash.as_slice()],
    )?;
    let n: i64 = conn
        .query_row(
            "SELECT totp_attempts FROM sessions WHERE token_hash = ?1",
            [token_hash.as_slice()],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(i64::MAX);
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Promotes a pending session after a valid TOTP code.
pub fn complete_2fa(conn: &Connection, token_hash: &[u8; 32]) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE sessions SET pending_2fa = 0, totp_attempts = 0 WHERE token_hash = ?1",
        [token_hash.as_slice()],
    )?;
    Ok(())
}

/// Deletes sessions past their expiry.
pub fn purge_expired(conn: &Connection, now: u64) -> Result<usize, ApiError> {
    Ok(conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", [int(now)])?)
}
