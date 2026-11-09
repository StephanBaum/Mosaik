//! The `accounts` table.

use rusqlite::{Connection, ErrorCode, OptionalExtension, Transaction, params};

use super::{id16, int, u64_col};
use crate::error::ApiError;
use mosaic_sync::wire::json::{AccountVersionDetail, codes};

/// An account row.
#[expect(
    clippy::struct_field_names,
    reason = "the columns of the accounts table"
)]
pub struct Account {
    pub account_id: [u8; 16],
    pub email: String,
    pub salt: [u8; 16],
    pub profile: u8,
    pub opaque_record: Vec<u8>,
    pub wrapped_ak: Vec<u8>,
    pub account_version: u64,
    pub totp_sealed: Option<Vec<u8>>,
    pub totp_pending_sealed: Option<Vec<u8>>,
    pub totp_last_step: Option<u64>,
}

/// What a registration stores.
pub struct NewAccount<'a> {
    pub account_id: [u8; 16],
    pub email: &'a str,
    pub salt: [u8; 16],
    pub profile: u8,
    pub opaque_record: &'a [u8],
    pub wrapped_ak: &'a [u8; 74],
    /// The account-key epoch the account starts at (at least 1).
    pub ak_epoch: u32,
}

const COLUMNS: &str = "account_id, email, salt, profile, opaque_record, wrapped_ak, account_version, \
     totp_sealed, totp_pending_sealed, totp_last_step";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    let step: Option<i64> = r.get(9)?;
    Ok(Account {
        account_id: id16(r, 0)?,
        email: r.get(1)?,
        salt: id16(r, 2)?,
        profile: r.get(3)?,
        opaque_record: r.get(4)?,
        wrapped_ak: r.get(5)?,
        account_version: u64_col(r, 6)?,
        totp_sealed: r.get(7)?,
        totp_pending_sealed: r.get(8)?,
        totp_last_step: step.and_then(|s| u64::try_from(s).ok()),
    })
}

/// The account with this (normalized) email.
pub fn by_email(conn: &Connection, email: &str) -> Result<Option<Account>, ApiError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM accounts WHERE email = ?1"),
            [email],
            row,
        )
        .optional()?)
}

/// The account with this ID.
pub fn by_id(conn: &Connection, id: &[u8; 16]) -> Result<Option<Account>, ApiError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM accounts WHERE account_id = ?1"),
            [id.as_slice()],
            row,
        )
        .optional()?)
}

/// Inserts an account; `account_exists` if the email or the ID is taken.
pub fn insert(tx: &Transaction<'_>, a: &NewAccount<'_>, now: u64) -> Result<(), ApiError> {
    let r = tx.execute(
        "INSERT INTO accounts (account_id, email, salt, profile, opaque_record, wrapped_ak, \
         account_version, created_at, ak_epoch) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
        params![
            a.account_id.as_slice(),
            a.email,
            a.salt.as_slice(),
            a.profile,
            a.opaque_record,
            a.wrapped_ak.as_slice(),
            int(now),
            a.ak_epoch
        ],
    );
    match r {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            Err(ApiError::conflict(codes::ACCOUNT_EXISTS))
        }
        Err(e) => Err(e.into()),
    }
}

/// How many accounts use each Argon2 profile.
pub fn profile_counts(conn: &Connection) -> Result<Vec<(u8, u64)>, ApiError> {
    let mut stmt = conn.prepare("SELECT profile, COUNT(*) FROM accounts GROUP BY profile")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, u8>(0)?, u64_col(r, 1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Replaces the registration and the password wrapping of AK, if the stored
/// version equals `expected` (CRYPTO_SPEC §9.4). Returns the new version.
pub fn update_password(
    tx: &Transaction<'_>,
    id: &[u8; 16],
    expected: u64,
    opaque_record: &[u8],
    wrapped_ak: &[u8; 74],
    salt: &[u8; 16],
    profile: u8,
) -> Result<u64, ApiError> {
    let n = tx.execute(
        "UPDATE accounts SET opaque_record = ?1, wrapped_ak = ?2, salt = ?3, profile = ?4, \
         account_version = account_version + 1 WHERE account_id = ?5 AND account_version = ?6",
        params![
            opaque_record,
            wrapped_ak.as_slice(),
            salt.as_slice(),
            profile,
            id.as_slice(),
            int(expected)
        ],
    )?;
    if n == 1 {
        return Ok(expected.saturating_add(1));
    }
    let current: Option<i64> = tx
        .query_row(
            "SELECT account_version FROM accounts WHERE account_id = ?1",
            [id.as_slice()],
            |r| r.get(0),
        )
        .optional()?;
    let current = current
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(ApiError::unauthorized)?;
    Err(
        ApiError::conflict(codes::ACCOUNT_VERSION_MISMATCH).with_detail(&AccountVersionDetail {
            account_version: current,
        }),
    )
}

/// Stores a pending (unconfirmed) TOTP secret.
pub fn set_totp_pending(conn: &Connection, id: &[u8; 16], sealed: &[u8]) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE accounts SET totp_pending_sealed = ?1 WHERE account_id = ?2",
        params![sealed, id.as_slice()],
    )?;
    Ok(())
}

/// Makes the pending TOTP secret the active one, with the matched step.
pub fn confirm_totp(conn: &Connection, id: &[u8; 16], step: u64) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE accounts SET totp_sealed = totp_pending_sealed, totp_pending_sealed = NULL, \
         totp_last_step = ?1 WHERE account_id = ?2 AND totp_pending_sealed IS NOT NULL",
        params![int(step), id.as_slice()],
    )?;
    Ok(())
}

/// Removes the TOTP configuration.
pub fn clear_totp(conn: &Connection, id: &[u8; 16]) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE accounts SET totp_sealed = NULL, totp_pending_sealed = NULL, totp_last_step = NULL \
         WHERE account_id = ?1",
        [id.as_slice()],
    )?;
    Ok(())
}

/// Records the used TOTP step (replay guard, ADR-0008) with a conditional
/// update: false if a step at or above it is already stored, so two
/// concurrent requests cannot both accept one code.
pub fn set_last_step(conn: &Connection, id: &[u8; 16], step: u64) -> Result<bool, ApiError> {
    let n = conn.execute(
        "UPDATE accounts SET totp_last_step = ?1 WHERE account_id = ?2 \
         AND (totp_last_step IS NULL OR totp_last_step < ?1)",
        params![int(step), id.as_slice()],
    )?;
    Ok(n == 1)
}

/// The window of the per-account TOTP failure budget.
pub const TOTP_FAIL_WINDOW_MS: u64 = 3_600_000;
/// Wrong TOTP codes allowed per account and window, from any session.
pub const TOTP_FAIL_BUDGET: u64 = 10;

/// Claims one attempt of the per-account budget before a code is checked:
/// an atomic count, so concurrent guesses cannot exceed it. The attempt is
/// given back with [`refund_totp_slot`] when the code was right. The error is
/// the seconds until the window ends.
pub fn claim_totp_slot(
    conn: &Connection,
    id: &[u8; 16],
    now: u64,
) -> Result<Result<(), u32>, ApiError> {
    let n = conn.execute(
        "UPDATE accounts SET \
           totp_fail_count = CASE WHEN ?2 - totp_fail_window >= ?3 THEN 1 ELSE totp_fail_count + 1 END, \
           totp_fail_window = CASE WHEN ?2 - totp_fail_window >= ?3 THEN ?2 ELSE totp_fail_window END \
         WHERE account_id = ?1 AND (?2 - totp_fail_window >= ?3 OR totp_fail_count < ?4)",
        params![
            id.as_slice(),
            int(now),
            int(TOTP_FAIL_WINDOW_MS),
            int(TOTP_FAIL_BUDGET)
        ],
    )?;
    if n == 1 {
        return Ok(Ok(()));
    }
    let window: i64 = conn
        .query_row(
            "SELECT totp_fail_window FROM accounts WHERE account_id = ?1",
            [id.as_slice()],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    let end = u64::try_from(window)
        .unwrap_or(0)
        .saturating_add(TOTP_FAIL_WINDOW_MS);
    Ok(Err(u32::try_from(end.saturating_sub(now).div_ceil(1000))
        .unwrap_or(u32::MAX)
        .max(1)))
}

/// Gives an attempt of the budget back (the code was right).
pub fn refund_totp_slot(conn: &Connection, id: &[u8; 16]) -> Result<(), ApiError> {
    conn.execute(
        "UPDATE accounts SET totp_fail_count = MAX(totp_fail_count - 1, 0) WHERE account_id = ?1",
        [id.as_slice()],
    )?;
    Ok(())
}

/// Deletes the account and, by cascade, everything it owns. Returns the IDs
/// of its attachments so the caller removes their files after the commit.
pub fn delete(tx: &Transaction<'_>, id: &[u8; 16]) -> Result<Vec<[u8; 16]>, ApiError> {
    let mut stmt = tx.prepare(
        "SELECT attachment_id FROM attachments WHERE vault_id IN \
         (SELECT vault_id FROM vaults WHERE owner_account_id = ?1)",
    )?;
    let ids = stmt
        .query_map([id.as_slice()], |r| id16(r, 0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    tx.execute(
        "DELETE FROM accounts WHERE account_id = ?1",
        [id.as_slice()],
    )?;
    Ok(ids)
}

/// What an account uses, for the quotas.
#[derive(Default)]
pub struct Usage {
    pub vaults: u64,
    pub records: u64,
    pub record_bytes: u64,
    pub attachment_bytes: u64,
}

/// Counts what `id` owns.
pub fn usage(conn: &Connection, id: &[u8; 16]) -> Result<Usage, ApiError> {
    let id = id.as_slice();
    let vaults: i64 = conn.query_row(
        "SELECT COUNT(*) FROM vaults WHERE owner_account_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    let (records, record_bytes): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(length(ciphertext)), 0) FROM records WHERE vault_id IN \
         (SELECT vault_id FROM vaults WHERE owner_account_id = ?1)",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let attachment_bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size), 0) FROM attachments WHERE vault_id IN \
         (SELECT vault_id FROM vaults WHERE owner_account_id = ?1)",
        [id],
        |r| r.get(0),
    )?;
    let u = |v: i64| u64::try_from(v).unwrap_or(0);
    Ok(Usage {
        vaults: u(vaults),
        records: u(records),
        record_bytes: u(record_bytes),
        attachment_bytes: u(attachment_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    const T: u64 = 1_800_000_000_000;

    fn db() -> Db {
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

    /// A step is stored only if it is above the stored one: the second of two
    /// requests with one code loses.
    #[test]
    fn last_step_is_a_conditional_update() {
        let db = db();
        let id = [0u8; 16];
        db.run(|c| {
            assert!(set_last_step(c, &id, 5)?);
            assert!(!set_last_step(c, &id, 5)?);
            assert!(!set_last_step(c, &id, 4)?);
            assert!(set_last_step(c, &id, 6)?);
            Ok(())
        })
        .unwrap();
    }

    /// Ten claims per hour, then the seconds until the window ends; a refund
    /// gives one back; a new window starts fresh.
    #[test]
    fn totp_budget_is_claimed_atomically() {
        let db = db();
        let id = [0u8; 16];
        db.run(|c| {
            for _ in 0..10 {
                assert_eq!(claim_totp_slot(c, &id, T + 1_000)?, Ok(()));
            }
            assert_eq!(claim_totp_slot(c, &id, T + 61_000)?, Err(3_540));
            refund_totp_slot(c, &id)?;
            assert_eq!(claim_totp_slot(c, &id, T + 61_000)?, Ok(()));
            assert!(claim_totp_slot(c, &id, T + 62_000)?.is_err());
            assert_eq!(
                claim_totp_slot(c, &id, T + 1_000 + TOTP_FAIL_WINDOW_MS)?,
                Ok(())
            );
            Ok(())
        })
        .unwrap();
    }
}
