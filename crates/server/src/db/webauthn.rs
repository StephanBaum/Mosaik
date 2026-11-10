//! The `webauthn_credentials` table and the per-account user handle
//! (CRYPTO_SPEC §9.3). The public key is stored in clear (it is public).

use rusqlite::{Connection, ErrorCode, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};

use super::{int, u64_col};
use crate::error::ApiError;

/// Security keys per account.
pub const MAX_CREDENTIALS: u64 = 10;

/// A stored credential.
pub struct Row {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub label: String,
    pub created_at: u64,
    pub last_used_at: Option<u64>,
}

/// Why a credential could not be stored.
#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    /// The account has [`MAX_CREDENTIALS`] already.
    Limit,
    /// The credential ID exists (for any account).
    Duplicate,
}

/// The credential reference the API shows: the first 16 bytes of the SHA-256
/// of the credential ID.
#[must_use]
pub fn credential_ref(credential_id: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(credential_id);
    let mut out = [0u8; 16];
    out.copy_from_slice(digest.get(..16).unwrap_or(&[0; 16]));
    out
}

/// The account's credentials, oldest first.
pub fn list(conn: &Connection, account: &[u8; 16]) -> Result<Vec<Row>, ApiError> {
    let mut stmt = conn.prepare(
        "SELECT credential_id, public_key, sign_count, label, created_at, last_used_at \
         FROM webauthn_credentials WHERE account_id = ?1 ORDER BY created_at, credential_id",
    )?;
    let rows = stmt.query_map([account.as_slice()], |r| {
        let last: Option<i64> = r.get(5)?;
        Ok(Row {
            credential_id: r.get(0)?,
            public_key: r.get(1)?,
            sign_count: u32::try_from(u64_col(r, 2)?).unwrap_or(u32::MAX),
            label: r.get(3)?,
            created_at: u64_col(r, 4)?,
            last_used_at: last.and_then(|v| u64::try_from(v).ok()),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// How many credentials the account has.
pub fn count(conn: &Connection, account: &[u8; 16]) -> Result<u64, ApiError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM webauthn_credentials WHERE account_id = ?1",
        [account.as_slice()],
        |r| r.get(0),
    )?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// How many accounts hold at least one credential (the startup warning).
pub fn accounts_with_credentials(conn: &Connection) -> Result<u64, ApiError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT account_id) FROM webauthn_credentials",
        [],
        |r| r.get(0),
    )?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// The account's user handle, created on first use (16 bytes from the OS
/// CSPRNG, never the account ID).
pub fn user_handle(tx: &Transaction<'_>, account: &[u8; 16]) -> Result<[u8; 16], ApiError> {
    let read = |tx: &Transaction<'_>| -> Result<Option<Vec<u8>>, ApiError> {
        Ok(tx
            .query_row(
                "SELECT webauthn_user_handle FROM accounts WHERE account_id = ?1",
                [account.as_slice()],
                |r| r.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()?
            .flatten())
    };
    if let Some(h) = read(tx)? {
        return h.try_into().map_err(|_| ApiError::internal());
    }
    let mut fresh = [0u8; 16];
    getrandom::fill(&mut fresh).map_err(|_| ApiError::internal())?;
    tx.execute(
        "UPDATE accounts SET webauthn_user_handle = ?2 \
         WHERE account_id = ?1 AND webauthn_user_handle IS NULL",
        params![account.as_slice(), fresh.as_slice()],
    )?;
    read(tx)?
        .and_then(|h| h.try_into().ok())
        .ok_or_else(ApiError::internal)
}

/// The existing user handle, if any (it is created with the first credential).
pub fn existing_user_handle(
    conn: &Connection,
    account: &[u8; 16],
) -> Result<Option<[u8; 16]>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT webauthn_user_handle FROM accounts WHERE account_id = ?1",
            [account.as_slice()],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()?
        .flatten()
        .and_then(|h| h.try_into().ok()))
}

/// Stores a credential: at most [`MAX_CREDENTIALS`] per account, the
/// credential ID unique server-wide.
pub fn insert(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    credential_id: &[u8],
    public_key: &[u8],
    sign_count: u32,
    label: &str,
    now: u64,
) -> Result<Result<(), InsertError>, ApiError> {
    if count(tx, account)? >= MAX_CREDENTIALS {
        return Ok(Err(InsertError::Limit));
    }
    let r = tx.execute(
        "INSERT INTO webauthn_credentials (account_id, credential_id, public_key, sign_count, \
         label, created_at, last_used_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        params![
            account.as_slice(),
            credential_id,
            public_key,
            sign_count,
            label,
            int(now)
        ],
    );
    match r {
        Ok(_) => Ok(Ok(())),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == ErrorCode::ConstraintViolation => {
            Ok(Err(InsertError::Duplicate))
        }
        Err(e) => Err(e.into()),
    }
}

/// Stores the new sign count as a compare-and-set (CRYPTO_SPEC §9.3): false
/// if no row changed, which means a concurrent finish stored an equal or
/// higher count first (or both counts are not the allowed 0 / 0).
pub fn store_count(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    credential_id: &[u8],
    new: u32,
    now: u64,
) -> Result<bool, ApiError> {
    let n = tx.execute(
        "UPDATE webauthn_credentials SET sign_count = ?3, last_used_at = ?4 \
         WHERE account_id = ?1 AND credential_id = ?2 \
         AND (sign_count < ?3 OR (sign_count = 0 AND ?3 = 0))",
        params![account.as_slice(), credential_id, new, int(now)],
    )?;
    Ok(n == 1)
}

/// Removes the credential with this reference; false if there is none.
pub fn remove(
    tx: &Transaction<'_>,
    account: &[u8; 16],
    reference: &[u8; 16],
) -> Result<bool, ApiError> {
    for row in list(tx, account)? {
        if &credential_ref(&row.credential_id) == reference {
            tx.execute(
                "DELETE FROM webauthn_credentials WHERE account_id = ?1 AND credential_id = ?2",
                params![account.as_slice(), row.credential_id],
            )?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Removes every credential of the account; returns how many.
pub fn delete_all(tx: &Transaction<'_>, account: &[u8; 16]) -> Result<usize, ApiError> {
    Ok(tx.execute(
        "DELETE FROM webauthn_credentials WHERE account_id = ?1",
        [account.as_slice()],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

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

    /// The count is stored only if it rises, or if both counts are 0: of two
    /// concurrent finishes with one count, the second loses.
    #[test]
    fn sign_count_is_a_compare_and_set() {
        let db = db();
        let acct = [0u8; 16];
        let id = [5u8; 16];
        db.run_tx(|tx| {
            assert_eq!(insert(tx, &acct, &id, &[1], 0, "k", 1)?, Ok(()));
            // 0 on both sides is accepted, again and again.
            assert!(store_count(tx, &acct, &id, 0, 2)?);
            assert!(store_count(tx, &acct, &id, 0, 3)?);
            assert!(store_count(tx, &acct, &id, 7, 4)?);
            // An equal or lower count is refused; 0 after a non-zero is too.
            assert!(!store_count(tx, &acct, &id, 7, 5)?);
            assert!(!store_count(tx, &acct, &id, 3, 5)?);
            assert!(!store_count(tx, &acct, &id, 0, 5)?);
            assert!(store_count(tx, &acct, &id, 8, 6)?);
            assert!(
                !store_count(tx, &acct, &[6u8; 16], 9, 7)?,
                "no such credential"
            );
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn at_most_ten_and_unique_ids() {
        let db = db();
        let acct = [0u8; 16];
        db.run_tx(|tx| {
            for i in 0..10u8 {
                assert_eq!(insert(tx, &acct, &[i; 16], &[1], 0, "k", 1)?, Ok(()));
            }
            assert_eq!(
                insert(tx, &acct, &[10; 16], &[1], 0, "k", 1)?,
                Err(InsertError::Limit)
            );
            assert_eq!(count(tx, &acct)?, 10);
            assert!(remove(tx, &acct, &credential_ref(&[3; 16]))?);
            assert_eq!(
                insert(tx, &acct, &[4; 16], &[1], 0, "k", 1)?,
                Err(InsertError::Duplicate)
            );
            // The handle is made once.
            let h = user_handle(tx, &acct)?;
            assert_eq!(user_handle(tx, &acct)?, h);
            assert_ne!(h, [0u8; 16]);
            Ok(())
        })
        .unwrap();
    }
}
