//! SQLite storage (ADR-0044): `rusqlite` with the amalgamation compiled in,
//! WAL, foreign keys, STRICT tables, bound parameters only, one connection
//! behind a mutex.
//!
//! The handlers call [`Db::run`] / [`Db::run_tx`] with a closure. On a
//! multi-threaded runtime the closure runs inside `block_in_place`, so a slow
//! query does not stall the other connections' tasks; on a current-thread
//! runtime (the tests, which pause the clock) it runs inline.

pub mod accounts;
pub mod attachments;
pub mod audit;
pub mod checkpoints;
pub mod devices;
pub mod directory;
pub mod emergency;
pub mod invites;
pub mod records;
pub mod rekey;
pub mod schema;
pub mod sessions;
pub mod sharing;
pub mod vaults;

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::{Connection, Transaction, TransactionBehavior};
use tokio::runtime::{Handle, RuntimeFlavor};

use crate::error::ApiError;

/// Why the database could not be opened.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DbError {
    /// SQLite failed.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A file mode could not be set.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The database was written by a newer server.
    #[error("the database schema is newer than this server")]
    TooNew,
}

/// The database.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Opens (and migrates) the database file `path`, creating it if needed.
    ///
    /// # Errors
    /// [`DbError`].
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        // Before WAL is on: the WAL and shared-memory files SQLite creates
        // take the mode of the database file.
        crate::fsperm::restrict_file(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    /// An in-memory database (tests).
    ///
    /// # Errors
    /// [`DbError`].
    pub fn open_in_memory() -> Result<Self, DbError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "trusted_schema", "OFF")?;
        // Temporary tables and sort files stay in memory (a read-only container
        // file system has no temp directory).
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        // Deleted rows are overwritten, not left in free pages.
        conn.pragma_update(None, "secure_delete", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `f` with the connection.
    ///
    /// # Errors
    /// Whatever `f` returns.
    pub fn run<R>(
        &self,
        f: impl FnOnce(&Connection) -> Result<R, ApiError>,
    ) -> Result<R, ApiError> {
        in_place(|| f(&self.lock()))
    }

    /// Runs `f` in one immediate transaction: committed if `f` returns `Ok`,
    /// rolled back otherwise.
    ///
    /// # Errors
    /// Whatever `f` returns, or a SQLite failure.
    pub fn run_tx<R>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<R, ApiError>,
    ) -> Result<R, ApiError> {
        in_place(|| {
            let mut conn = self.lock();
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let out = f(&tx)?;
            tx.commit()?;
            Ok(out)
        })
    }

    /// A consistent copy of the database to `dest` (`VACUUM INTO`).
    ///
    /// # Errors
    /// [`DbError`].
    pub fn backup_to(&self, dest: &Path) -> Result<(), DbError> {
        let conn = self.lock();
        let dest = dest.to_string_lossy();
        conn.execute("VACUUM INTO ?1", [dest.as_ref()])?;
        Ok(())
    }
}

fn in_place<R>(f: impl FnOnce() -> R) -> R {
    match Handle::try_current() {
        Ok(h) if h.runtime_flavor() == RuntimeFlavor::MultiThread => tokio::task::block_in_place(f),
        _ => f(),
    }
}

fn migrate(conn: &mut Connection) -> Result<(), DbError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let version = usize::try_from(version).unwrap_or(usize::MAX);
    if version > schema::latest_version() {
        return Err(DbError::TooNew);
    }
    for (n, sql) in schema::MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(
            None,
            "user_version",
            i64::try_from(n + 1).unwrap_or(i64::MAX),
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// A 16-byte ID column from a row.
pub(crate) fn id16(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<[u8; 16]> {
    let v: Vec<u8> = row.get(idx)?;
    v.try_into().map_err(|_| rusqlite::Error::InvalidQuery)
}

/// An ID as a lowercase hyphenated UUID string (what the API shows).
pub(crate) fn uuid_text(id: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(36);
    for (i, b) in id.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        for n in [b >> 4, b & 15] {
            out.push(char::from(HEX.get(usize::from(n)).copied().unwrap_or(b'0')));
        }
    }
    out
}

/// A `u64` from an INTEGER column.
pub(crate) fn u64_col(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<u64> {
    let v: i64 = row.get(idx)?;
    u64::try_from(v).map_err(|_| rusqlite::Error::InvalidQuery)
}

/// A `u64` to an INTEGER parameter (saturating at `i64::MAX`).
pub(crate) fn int(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite");
        {
            let db = Db::open(&path).unwrap();
            db.run(|c| {
                let n: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
                assert_eq!(usize::try_from(n).unwrap(), schema::latest_version());
                Ok(())
            })
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        db.backup_to(&dir.path().join("copy.sqlite")).unwrap();
        let copy = Db::open(&dir.path().join("copy.sqlite")).unwrap();
        copy.run(|c| {
            let n: i64 = c.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))?;
            assert_eq!(n, 0);
            Ok(())
        })
        .unwrap();
    }

    /// A v1 database upgrades in place: memberships become owner memberships
    /// with an origin and a time, the audit table accepts the new kinds, and
    /// the new tables enforce their rules.
    #[test]
    fn migrates_v1_to_v2() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(schema::MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute_batch(
            "INSERT INTO accounts (account_id, email, salt, profile, opaque_record, wrapped_ak,                account_version, created_at) VALUES (zeroblob(16), 'a@b.c', zeroblob(16), 1, x'00',                zeroblob(74), 1, 0);
             INSERT INTO vaults (vault_id, owner_account_id, current_generation, head_seq, created_at)                VALUES (zeroblob(16), zeroblob(16), 1, 2, 5000);
             INSERT INTO memberships (account_id, vault_id, role) VALUES (zeroblob(16), zeroblob(16), 1);
             INSERT INTO audit (account_id, at, kind, device_id) VALUES (zeroblob(16), 10, 6, NULL);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();

        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(usize::try_from(v).unwrap(), schema::latest_version());
        let m: (i64, i64, i64) = conn
            .query_row("SELECT role, origin, added_at FROM memberships", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(m, (1, 0, 5000));
        let a: (i64, i64) = conn
            .query_row("SELECT kind, at FROM audit", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(a, (6, 10));
        let (ipk, seq): (Option<Vec<u8>>, i64) = conn
            .query_row("SELECT ipk, contacts_seq FROM accounts", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((ipk, seq), (None, 0));

        // The rebuilt tables keep their rules and gain the new ones.
        let bad = |sql: &str| conn.execute(sql, []).is_err();
        assert!(!bad("INSERT INTO audit VALUES (zeroblob(16), 1, 14, NULL)"));
        assert!(!bad("INSERT INTO audit VALUES (zeroblob(16), 1, 16, NULL)"));
        assert!(bad("INSERT INTO audit VALUES (zeroblob(16), 1, 17, NULL)"));
        assert!(bad(
            "INSERT INTO memberships VALUES (zeroblob(16), zeroblob(16), 5, 0, 0)"
        ));
        assert!(bad(
            "INSERT INTO memberships VALUES (x'01010101010101010101010101010101', zeroblob(16), 2, 0, 0)"
        ));
        assert!(bad("UPDATE accounts SET ipk = x'00'"));
        let invite = "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation,              seal, created_at, expires_at, state) VALUES (randomblob(16), zeroblob(16), zeroblob(16),              zeroblob(16), 3, 1, zeroblob(82), 0, 1, 0)";
        assert!(!bad(invite));
        assert!(bad(invite), "one open invite per vault and recipient");
        assert!(bad(
            "INSERT INTO emergency_grants (grantor, grantee, wait_days, scope, state, created_at)              VALUES (zeroblob(16), zeroblob(16), 3, 1, 0, 0)"
        ));
        // The cascade still reaches the rebuilt tables.
        conn.execute("DELETE FROM accounts", []).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM memberships) + (SELECT COUNT(*) FROM audit)                  + (SELECT COUNT(*) FROM share_invites)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    /// A v2 database upgrades in place: accounts start at epoch 1, invites and
    /// audit rows survive the rebuild and the new kinds and states are
    /// accepted, rotation requests follow their vault and account, and the
    /// epoch trigger refuses a fall and a jump.
    #[test]
    fn migrates_v2_to_v3() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(schema::MIGRATIONS[0]).unwrap();
        conn.execute_batch(schema::MIGRATIONS[1]).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        conn.execute_batch(
            "INSERT INTO accounts (account_id, email, salt, profile, opaque_record, wrapped_ak,                 account_version, created_at) VALUES (zeroblob(16), 'a@b.c', zeroblob(16), 1, x'00',                 zeroblob(74), 1, 0);
             INSERT INTO vaults (vault_id, owner_account_id, current_generation, head_seq, created_at)                 VALUES (zeroblob(16), zeroblob(16), 1, 2, 5000);
             INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation,                 seal, created_at, expires_at, delivered_at, state, hidden) VALUES (x'01010101010101010101010101010101',                 zeroblob(16), zeroblob(16), x'02020202020202020202020202020202', 3, 1, zeroblob(82), 1, 9, 4, 0, 1);
             INSERT INTO audit (account_id, at, kind, device_id) VALUES (zeroblob(16), 10, 14, NULL);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();

        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(usize::try_from(v).unwrap(), schema::latest_version());
        assert_eq!(schema::latest_version(), 3);
        let epoch: i64 = conn
            .query_row("SELECT ak_epoch FROM accounts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(epoch, 1);
        let invite: (i64, i64, i64, i64, i64) = conn
            .query_row(
                "SELECT state, hidden, delivered_at, created_at, expires_at FROM share_invites",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(invite, (0, 1, 4, 1, 9));
        let audit: (i64, i64) = conn
            .query_row("SELECT kind, at FROM audit", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(audit, (14, 10));

        let bad = |sql: &str| conn.execute(sql, []).is_err();
        assert!(!bad("INSERT INTO audit VALUES (zeroblob(16), 1, 15, NULL)"));
        assert!(!bad("INSERT INTO audit VALUES (zeroblob(16), 1, 16, NULL)"));
        assert!(bad("INSERT INTO audit VALUES (zeroblob(16), 1, 17, NULL)"));
        assert!(!bad("UPDATE share_invites SET state = 5"));
        assert!(bad("UPDATE share_invites SET state = 6"));
        // The invite indexes came back: one open invite per vault and recipient.
        let again = "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation,              seal, created_at, expires_at, state) VALUES (randomblob(16), zeroblob(16), zeroblob(16),              x'02020202020202020202020202020202', 3, 1, zeroblob(82), 1, 9, 0)";
        assert!(!bad(again), "state 5 is not open");
        assert!(bad(again), "one open invite per vault and recipient");
        // The epoch never falls and rises by exactly one.
        assert!(bad("UPDATE accounts SET ak_epoch = 0"));
        assert!(bad("UPDATE accounts SET ak_epoch = 3"));
        assert!(!bad("UPDATE accounts SET ak_epoch = 2"));
        assert!(bad("UPDATE accounts SET ak_epoch = 1"));
        assert!(!bad("UPDATE accounts SET email = 'c@d.e'"));
        // Rotation requests: reason 1 only, one per vault and requester, gone
        // with the vault or the account.
        assert!(bad(
            "INSERT INTO vault_rotation_requests VALUES (zeroblob(16), zeroblob(16), 2, 0)"
        ));
        assert!(!bad(
            "INSERT INTO vault_rotation_requests VALUES (zeroblob(16), zeroblob(16), 1, 0)"
        ));
        assert!(bad(
            "INSERT INTO vault_rotation_requests VALUES (zeroblob(16), zeroblob(16), 1, 1)"
        ));
        conn.execute("DELETE FROM accounts", []).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM vault_rotation_requests) + (SELECT COUNT(*) FROM audit)                  + (SELECT COUNT(*) FROM share_invites)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }
}
