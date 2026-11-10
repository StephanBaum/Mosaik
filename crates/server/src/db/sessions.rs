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
    /// The account version at the session's login (its KE3): a password
    /// change is accepted only on top of it.
    pub account_version: u64,
    /// Until when a completed `Confirm` handoff counts as a second factor
    /// (0: never).
    pub second_factor_until: u64,
    /// When the session ends.
    pub expires_at: u64,
}

/// Why sessions were deleted (CRYPTO_SPEC §9.5). The numbers are stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Revoke {
    Rekey = 1,
    PasswordChange = 2,
    DeviceRemoved = 3,
    Operator = 4,
}

impl Revoke {
    /// The text of `detail.revoked`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rekey => "rekey",
            Self::PasswordChange => "passwordChange",
            Self::DeviceRemoved => "deviceRemoved",
            Self::Operator => "operator",
        }
    }

    fn from_id(n: i64) -> Option<Self> {
        match n {
            1 => Some(Self::Rekey),
            2 => Some(Self::PasswordChange),
            3 => Some(Self::DeviceRemoved),
            4 => Some(Self::Operator),
            _ => None,
        }
    }
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
    pub account_version: u64,
}

/// Stores a new session.
pub fn insert(conn: &Connection, s: &NewSession<'_>) -> Result<(), ApiError> {
    conn.execute(
        "INSERT INTO sessions (token_hash, account_id, device_id, created_at, last_used_at, \
         expires_at, fresh_until, pending_2fa, totp_attempts, account_version) \
         VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, 0, ?8)",
        params![
            s.token_hash.as_slice(),
            s.account.as_slice(),
            s.device.as_slice(),
            int(s.now),
            int(s.expires_at),
            int(s.fresh_until),
            i64::from(s.pending),
            int(s.account_version)
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
            "SELECT account_id, device_id, created_at, expires_at, fresh_until, pending_2fa, \
             account_version, second_factor_until FROM sessions WHERE token_hash = ?1",
            [token_hash.as_slice()],
            |r| {
                Ok((
                    SessionRow {
                        account_id: id16(r, 0)?,
                        device_id: id16(r, 1)?,
                        fresh_until: u64_col(r, 4)?,
                        pending_2fa: r.get::<_, i64>(5)? != 0,
                        account_version: u64_col(r, 6)?,
                        second_factor_until: u64_col(r, 7)?,
                        expires_at: u64_col(r, 3)?,
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
    if row.pending_2fa {
        // A pending session does not slide: it ends five minutes after login.
        return Ok(Some(row));
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

/// Records the sessions of `account` that match `filter` (an SQL condition
/// over `account_id = ?1` and the extra parameter `?2`) in `revoked_sessions`.
fn record_revoked(
    conn: &Connection,
    account: &[u8; 16],
    extra: &[u8],
    filter: &str,
    reason: Revoke,
    now: u64,
) -> Result<(), ApiError> {
    conn.execute(
        &format!(
            "INSERT OR REPLACE INTO revoked_sessions (token_hash, account_id, reason, revoked_at) \n             SELECT token_hash, account_id, ?3, ?4 FROM sessions WHERE account_id = ?1 AND {filter}"
        ),
        params![account.as_slice(), extra, reason as u8, int(now)],
    )?;
    Ok(())
}

/// After a password change or a re-key (`reason`): deletes every other session
/// of the account (a token stolen before the change dies with it, and is
/// remembered with the reason) and moves the changing session to the new
/// account version.
pub fn after_password_change(
    conn: &Connection,
    account: &[u8; 16],
    keep: &[u8; 32],
    new_version: u64,
    reason: Revoke,
    now: u64,
) -> Result<(), ApiError> {
    record_revoked(conn, account, keep, "token_hash <> ?2", reason, now)?;
    conn.execute(
        "DELETE FROM sessions WHERE account_id = ?1 AND token_hash <> ?2",
        params![account.as_slice(), keep.as_slice()],
    )?;
    conn.execute(
        "UPDATE sessions SET account_version = ?2 WHERE token_hash = ?1",
        params![keep.as_slice(), int(new_version)],
    )?;
    Ok(())
}

/// Deletes every session of the account for the operator (reason 4).
pub fn delete_all_for_operator(
    conn: &Connection,
    account: &[u8; 16],
    now: u64,
) -> Result<(), ApiError> {
    record_revoked(conn, account, &[], "1 = 1", Revoke::Operator, now)?;
    conn.execute(
        "DELETE FROM sessions WHERE account_id = ?1",
        [account.as_slice()],
    )?;
    Ok(())
}

/// Deletes the sessions of a removed device and remembers them (reason 3).
pub fn delete_for_removed_device(
    conn: &Connection,
    account: &[u8; 16],
    device: &[u8; 16],
    now: u64,
) -> Result<(), ApiError> {
    record_revoked(
        conn,
        account,
        device,
        "device_id = ?2",
        Revoke::DeviceRemoved,
        now,
    )?;
    delete_for_device(conn, account, device)
}

/// Why a token that matches no session was deleted, if it was revoked.
pub fn revoked_reason(
    conn: &Connection,
    token_hash: &[u8; 32],
) -> Result<Option<Revoke>, ApiError> {
    let n: Option<i64> = conn
        .query_row(
            "SELECT reason FROM revoked_sessions WHERE token_hash = ?1",
            [token_hash.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(n.and_then(Revoke::from_id))
}

/// Drops revocation records older than `before`.
pub fn purge_revoked(conn: &Connection, before: u64) -> Result<usize, ApiError> {
    Ok(conn.execute(
        "DELETE FROM revoked_sessions WHERE revoked_at < ?1",
        [int(before)],
    )?)
}

/// Starts the five-minute window of a completed `Confirm` handoff. `false`
/// when the session no longer exists (or is pending, or belongs to another
/// account): the caller refuses.
pub fn set_second_factor_until(
    conn: &Connection,
    token_hash: &[u8; 32],
    account: &[u8; 16],
    until: u64,
) -> Result<bool, ApiError> {
    let n = conn.execute(
        "UPDATE sessions SET second_factor_until = ?3          WHERE token_hash = ?1 AND account_id = ?2 AND pending_2fa = 0",
        params![token_hash.as_slice(), account.as_slice(), int(until)],
    )?;
    Ok(n == 1)
}

/// Whether a session with this hash exists for the account, pending or full.
pub fn has_session(
    conn: &Connection,
    token_hash: &[u8; 32],
    account: &[u8; 16],
    pending: bool,
) -> Result<bool, ApiError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions          WHERE token_hash = ?1 AND account_id = ?2 AND pending_2fa = ?3",
        params![token_hash.as_slice(), account.as_slice(), i64::from(pending)],
        |r| r.get(0),
    )?;
    Ok(n == 1)
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

/// Counts one TOTP attempt on a pending session before the code is checked,
/// atomically: the count is returned, or `None` when the session is gone,
/// not pending or has used up its `max` attempts.
pub fn claim_attempt(
    conn: &Connection,
    token_hash: &[u8; 32],
    max: u32,
) -> Result<Option<u32>, ApiError> {
    let n: Option<i64> = conn
        .query_row(
            "UPDATE sessions SET totp_attempts = totp_attempts + 1 \
             WHERE token_hash = ?1 AND pending_2fa = 1 AND totp_attempts < ?2 \
             RETURNING totp_attempts",
            params![token_hash.as_slice(), max],
            |r| r.get(0),
        )
        .optional()?;
    Ok(n.map(|n| u32::try_from(n).unwrap_or(u32::MAX)))
}

/// Promotes a pending session after a valid TOTP code and gives it its idle
/// lifetime.
pub fn complete_2fa(
    conn: &Connection,
    token_hash: &[u8; 32],
    expires_at: u64,
) -> Result<bool, ApiError> {
    let n = conn.execute(
        "UPDATE sessions SET pending_2fa = 0, totp_attempts = 0, expires_at = ?2 \n             WHERE token_hash = ?1 AND pending_2fa = 1",
        params![token_hash.as_slice(), int(expires_at)],
    )?;
    Ok(n == 1)
}

/// Keeps at most `cap` sessions of an account (the one named by `keep`
/// included): the ones idle longest are deleted.
pub fn evict_over_cap(
    conn: &Connection,
    account: &[u8; 16],
    keep: &[u8; 32],
    cap: usize,
) -> Result<(), ApiError> {
    conn.execute(
        "DELETE FROM sessions WHERE token_hash IN (SELECT token_hash FROM sessions \
         WHERE account_id = ?1 AND token_hash <> ?2 ORDER BY last_used_at DESC, created_at DESC \
         LIMIT -1 OFFSET ?3)",
        params![
            account.as_slice(),
            keep.as_slice(),
            i64::try_from(cap.saturating_sub(1)).unwrap_or(i64::MAX)
        ],
    )?;
    Ok(())
}

/// Deletes sessions past their expiry.
pub fn purge_expired(conn: &Connection, now: u64) -> Result<usize, ApiError> {
    Ok(conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", [int(now)])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn db_with_account() -> Db {
        let db = Db::open_in_memory().unwrap();
        db.run(|c| {
            c.execute(
                "INSERT INTO accounts (account_id, email, salt, profile, opaque_record, wrapped_ak, \
                 account_version, created_at) VALUES (zeroblob(16), 'a@b.c', zeroblob(16), 1, x'00', \
                 zeroblob(74), 1, 0)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        db
    }

    fn new<'a>(
        hash: &'a [u8; 32],
        device: &'a [u8; 16],
        now: u64,
        pending: bool,
    ) -> NewSession<'a> {
        NewSession {
            token_hash: hash,
            account: &[0u8; 16],
            device,
            now,
            expires_at: now + 1_000_000,
            fresh_until: now,
            pending,
            account_version: 1,
        }
    }

    /// At most `cap` sessions stay, the newest ones and the one kept.
    #[test]
    fn eviction_keeps_cap_newest() {
        let db = db_with_account();
        db.run(|c| {
            for i in 0..70u8 {
                let hash = [i; 32];
                insert(c, &new(&hash, &[i; 16], u64::from(i) + 1, false))?;
            }
            let keep = [0u8; 32]; // the oldest, but kept
            evict_over_cap(c, &[0u8; 16], &keep, 64)?;
            let n: i64 = c.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
            assert_eq!(n, 64);
            let kept: i64 = c.query_row(
                "SELECT COUNT(*) FROM sessions WHERE token_hash = ?1",
                [keep.as_slice()],
                |r| r.get(0),
            )?;
            assert_eq!(kept, 1);
            let newest: i64 = c.query_row(
                "SELECT COUNT(*) FROM sessions WHERE token_hash = ?1",
                [[69u8; 32].as_slice()],
                |r| r.get(0),
            )?;
            assert_eq!(newest, 1);
            let evicted: i64 = c.query_row(
                "SELECT COUNT(*) FROM sessions WHERE token_hash = ?1",
                [[1u8; 32].as_slice()],
                |r| r.get(0),
            )?;
            assert_eq!(evicted, 0, "the oldest idle ones go first");
            Ok(())
        })
        .unwrap();
    }

    /// A pending session allows its attempts once, however fast they come,
    /// does not slide and ends after its own expiry.
    #[test]
    fn attempts_are_claimed_atomically_and_pending_does_not_slide() {
        let db = db_with_account();
        db.run(|c| {
            let hash = [9u8; 32];
            insert(c, &new(&hash, &[1; 16], 1000, true))?;
            let claimed: Vec<_> = (0..20)
                .map(|_| claim_attempt(c, &hash, 5).unwrap())
                .collect();
            assert_eq!(claimed.iter().filter(|c| c.is_some()).count(), 5);
            assert_eq!(claimed[4], Some(5));
            // Lookups do not extend a pending session.
            assert!(lookup_and_touch(c, &hash, 2000, 99_000_000, 99_000_000)?.is_some());
            let exp: i64 = c.query_row("SELECT expires_at FROM sessions", [], |r| r.get(0))?;
            assert_eq!(exp, 1_001_000);
            assert!(lookup_and_touch(c, &hash, 1_001_000, 1, 99_000_000)?.is_none());
            Ok(())
        })
        .unwrap();
    }
}
