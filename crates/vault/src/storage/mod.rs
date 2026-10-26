//! Offline-first local store (CRYPTO_SPEC §11).
//!
//! A [`Store`] is one SQLite database plus a directory of attachment files
//! next to it. It holds the same ciphertext the server holds, and the
//! wrapped keys needed to unlock offline:
//!
//! | Table | Contents |
//! |---|---|
//! | `account` | `account_id`, Argon2id salt and profile (§5.3) |
//! | `unlock_methods` | one [`WrappedKey`] of AK per unlock method (§6) |
//! | `vaults` | current and highest accepted key generation, [`VaultMetaCiphertext`] (§7.1) |
//! | `vault_keys` | one [`WrappedKey`] of VK per generation (§7.1) |
//! | `records` | [`ItemCiphertext`] / [`TombstoneCiphertext`], caller's accepted state, `server_seq`, outbox (§7.2, §10) |
//! | `sync_cursors` | per-vault pull cursor (§10) |
//! | `attachments` | routing metadata; the ciphertext is a file named by the attachment ID (§7.3) |
//!
//! The API only takes and returns `mosaic-core` ciphertext types, so the
//! store cannot be handed a plaintext or a key. Every object is decoded
//! strictly on the way in and again on the way out.
//!
//! **Anti-rollback (§7.1).** Each vault has a highest accepted key
//! generation that never decreases. Wrapped vault keys below it are
//! rejected ([`Error::KeyGenerationRollback`]), and every record and the
//! vault metadata must be written at exactly that generation: lower is a
//! rollback, higher is a generation whose key the caller has not accepted
//! ([`Error::KeyGenerationNotAccepted`]). Triggers in the schema repeat
//! these rules.
//!
//! **Record versions (§10).** The store does not interpret versions. The
//! caller decrypts, checks the authenticated version vector against the
//! `accepted_state` it stored earlier, and only then writes. The store adds
//! two cheap checks of its own: a record's `server_seq` never decreases,
//! and a remote write never silently replaces a local change that is still
//! in the outbox ([`Error::PendingLocalChange`]).
//!
//! Durability: WAL journal, `synchronous = FULL`, foreign keys on,
//! `secure_delete` on (deleted rows are overwritten), `trusted_schema` off.
//! Attachment files are written to a temporary file, synced, and renamed
//! into place.
//!
//! ```
//! use mosaic_core::{AccountId, AccountKey, ItemCiphertext, ItemId, KeyGeneration, VaultId, VaultKey};
//! use mosaic_vault::storage::{Origin, RecordBody, Store};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut store = Store::open_in_memory()?;
//! let (account, ak) = (AccountId::new_v7()?, AccountKey::generate()?);
//! let vault = VaultId::new_v7()?;
//! let vk = VaultKey::generate(KeyGeneration::FIRST)?;
//! store.create_vault(&vault, KeyGeneration::FIRST, &vk.wrap(&ak, &account, &vault)?)?;
//!
//! let item = ItemId::new_v7()?;
//! let body = RecordBody::Item(ItemCiphertext::seal(&vk, &vault, &item, 1, b"item")?);
//! let queued = store.upsert_record(&vault, &item, &body, b"", Origin::Local)?;
//! assert_eq!(store.outbox(10)?.len(), 1);
//!
//! // After the upload, the server reports its sequence number.
//! assert!(store.mark_uploaded(&vault, &item, queued.unwrap(), 42)?);
//! assert_eq!(store.record(&vault, &item)?.unwrap().server_seq, Some(42));
//! # Ok(())
//! # }
//! ```

mod account;
mod attachments;
mod error;
mod records;
mod schema;
mod vaults;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

pub use account::StoredUnlockMethod;
pub use attachments::{AttachmentInfo, validate_attachment_ciphertext};
pub use error::{Error, Result};
pub use records::{Origin, OutboxSeq, RecordBody, RecordKind, StoredRecord};
pub use schema::LATEST_VERSION;
pub use vaults::VaultState;

#[cfg(doc)]
use mosaic_core::{ItemCiphertext, TombstoneCiphertext, VaultMetaCiphertext, WrappedKey};
use mosaic_core::{KeyGeneration, VaultId};

/// Maximum length of a caller's opaque accepted-state blob per record.
pub const MAX_ACCEPTED_STATE_LEN: usize = 64 * 1024;

/// Maximum length of an unlock method's public parameters.
pub const MAX_UNLOCK_PARAMS_LEN: usize = 4096;

/// How long a write waits for another connection's lock before failing.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Suffix of the attachment directory next to the database file.
const ATTACHMENT_DIR_SUFFIX: &str = "-attachments";

/// The local store. See the [module documentation](self).
pub struct Store {
    conn: Connection,
    attachment_dir: Option<PathBuf>,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store")
            .field("path", &self.conn.path())
            .field("attachment_dir", &self.attachment_dir)
            .finish_non_exhaustive()
    }
}

impl Store {
    /// Opens (or creates) the store at `path`, creating the attachment
    /// directory `<path>-attachments` next to it, and migrates the schema
    /// to [`LATEST_VERSION`].
    ///
    /// # Errors
    /// [`Error::InvalidPath`] if `path` has no file name,
    /// [`Error::NotAMosaicStore`] for another SQLite database,
    /// [`Error::SchemaTooNew`] for a store from a newer build,
    /// [`Error::Database`] / [`Error::Io`] otherwise.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let attachment_dir = attachment_dir_for(path)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        attachments::prepare_db_file(path)?;
        let conn = Connection::open_with_flags(path, flags)?;
        let store = Self::init(conn, Some(attachment_dir))?;
        if let Some(dir) = &store.attachment_dir {
            attachments::create_dir(dir)?;
        }
        Ok(store)
    }

    /// Opens a fresh, empty in-memory store (for tests and ephemeral
    /// sessions). It has no attachment directory: attachment methods return
    /// [`Error::AttachmentsUnavailable`].
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, None)
    }

    fn init(mut conn: Connection, attachment_dir: Option<PathBuf>) -> Result<Self> {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "trusted_schema", false)?;
        conn.pragma_update(None, "secure_delete", true)?;
        // In memory the journal mode stays "memory"; on disk it is WAL.
        let _mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "wal", |r| r.get(0))?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        schema::migrate(&mut conn, schema::MIGRATIONS)?;
        Ok(Self {
            conn,
            attachment_dir,
        })
    }

    /// The schema version of the open store.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn schema_version(&self) -> Result<u32> {
        let v: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?;
        u32::try_from(v).map_err(|_| Error::Corrupt)
    }

    /// The attachment directory, or `None` for an in-memory store.
    #[must_use]
    pub fn attachment_dir(&self) -> Option<&Path> {
        self.attachment_dir.as_deref()
    }

    /// Checks the store and returns a list of problems; an empty list means
    /// the store is consistent.
    ///
    /// Runs SQLite's `integrity_check` and `foreign_key_check`, then checks
    /// the store's own invariants: every vault holds a wrapped key for its
    /// current generation, no record or metadata is above the accepted
    /// generation, and every attachment file exists with the recorded size.
    /// The messages contain table names and public IDs only.
    ///
    /// # Errors
    /// [`Error::Database`] if the checks themselves cannot run.
    pub fn integrity_check(&self) -> Result<Vec<String>> {
        let mut problems = Vec::new();
        self.conn.pragma_query(None, "integrity_check", |row| {
            let msg: String = row.get(0)?;
            if msg != "ok" {
                problems.push(format!("integrity_check: {msg}"));
            }
            Ok(())
        })?;
        self.conn.pragma_query(None, "foreign_key_check", |row| {
            let table: String = row.get(0)?;
            let parent: String = row.get(2)?;
            problems.push(format!("foreign key: {table} references missing {parent}"));
            Ok(())
        })?;
        let invariants: [(&str, &str); 3] = [
            (
                "SELECT lower(hex(vault_id)) FROM vaults v WHERE NOT EXISTS (
                     SELECT 1 FROM vault_keys k
                     WHERE k.vault_id = v.vault_id AND k.generation = v.current_generation)",
                "no wrapped key for the current generation",
            ),
            (
                "SELECT lower(hex(r.vault_id)) FROM records r JOIN vaults v USING (vault_id)
                 WHERE r.key_generation > v.highest_accepted_generation",
                "record above the accepted generation",
            ),
            (
                "SELECT lower(hex(vault_id)) FROM vault_keys k JOIN vaults v USING (vault_id)
                 WHERE k.generation > v.highest_accepted_generation",
                "vault key above the accepted generation",
            ),
        ];
        for (sql, what) in invariants {
            let mut stmt = self.conn.prepare(sql)?;
            let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for id in ids {
                problems.push(format!("vault {}: {what}", id?));
            }
        }
        self.check_attachment_files(&mut problems)?;
        Ok(problems)
    }

    /// Checkpoints the WAL into the database file and closes the store.
    /// Dropping a store also closes it; this variant reports errors.
    ///
    /// # Errors
    /// [`Error::Database`].
    pub fn close(self) -> Result<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        self.conn.close().map_err(|(_, e)| Error::Database(e))
    }

    /// Begins an `IMMEDIATE` write transaction.
    fn write(&mut self) -> Result<Transaction<'_>> {
        Ok(self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?)
    }
}

fn attachment_dir_for(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().ok_or(Error::InvalidPath)?;
    let mut dir = name.to_os_string();
    dir.push(ATTACHMENT_DIR_SUFFIX);
    Ok(path.with_file_name(dir))
}

/// Milliseconds since the Unix epoch, for `updated_at` columns. Clamped to
/// 0 for a clock before 1970.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// A sequence number as an SQLite integer.
fn seq_to_sql(seq: u64) -> Result<i64> {
    i64::try_from(seq).map_err(|_| Error::SeqOutOfRange)
}

/// A stored sequence number (the schema guarantees `>= 0`).
fn seq_from_sql(seq: i64) -> Result<u64> {
    u64::try_from(seq).map_err(|_| Error::Corrupt)
}

/// A stored key generation.
fn generation_from_sql(g: i64) -> Result<KeyGeneration> {
    u32::try_from(g)
        .ok()
        .and_then(|g| KeyGeneration::new(g).ok())
        .ok_or(Error::Corrupt)
}

/// A stored vault ID.
fn vault_id_from_sql(bytes: &[u8]) -> Result<VaultId> {
    VaultId::from_slice(bytes).map_err(|_| Error::Corrupt)
}

/// The §7.1 rule for every object written into a vault: exactly the highest
/// accepted generation.
fn check_generation(highest_accepted: KeyGeneration, offered: KeyGeneration) -> Result<()> {
    use core::cmp::Ordering;
    match offered.cmp(&highest_accepted) {
        Ordering::Equal => Ok(()),
        Ordering::Less => Err(Error::KeyGenerationRollback {
            highest_accepted: highest_accepted.get(),
            offered: offered.get(),
        }),
        Ordering::Greater => Err(Error::KeyGenerationNotAccepted {
            highest_accepted: highest_accepted.get(),
            offered: offered.get(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_is_migrated() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), LATEST_VERSION);
        assert!(s.attachment_dir().is_none());
        assert_eq!(s.integrity_check().unwrap().len(), 0);
        assert!(format!("{s:?}").starts_with("Store"));
        s.close().unwrap();
    }

    #[test]
    fn open_file_store_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let s = Store::open(&path).unwrap();
        let att = s.attachment_dir().unwrap().to_path_buf();
        assert_eq!(att, dir.path().join("vault.db-attachments"));
        assert!(att.is_dir());
        let mode: String = s
            .conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = s
            .conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        let sync: i64 = s
            .conn
            .pragma_query_value(None, "synchronous", |r| r.get(0))
            .unwrap();
        assert_eq!(sync, 2, "FULL");
        s.close().unwrap();
        let s = Store::open(&path).unwrap();
        assert_eq!(s.schema_version().unwrap(), LATEST_VERSION);
    }

    #[test]
    fn invalid_paths_and_foreign_files() {
        assert!(matches!(Store::open(".."), Err(Error::InvalidPath)));

        let dir = tempfile::tempdir().unwrap();
        let garbage = dir.path().join("garbage.db");
        std::fs::write(&garbage, vec![0x5A; 8192]).unwrap();
        assert!(matches!(Store::open(&garbage), Err(Error::Database(_))));

        let other = dir.path().join("other.db");
        let c = Connection::open(&other).unwrap();
        c.execute_batch("CREATE TABLE t (x INTEGER);").unwrap();
        drop(c);
        assert!(matches!(Store::open(&other), Err(Error::NotAMosaicStore)));
    }

    #[test]
    fn newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        Store::open(&path).unwrap().close().unwrap();
        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", LATEST_VERSION + 1)
            .unwrap();
        drop(c);
        assert!(matches!(
            Store::open(&path),
            Err(Error::SchemaTooNew { found, supported })
                if found == LATEST_VERSION + 1 && supported == LATEST_VERSION
        ));
    }

    #[test]
    fn helpers() {
        assert!(now_ms() > 0);
        assert_eq!(seq_to_sql(5).unwrap(), 5);
        assert!(matches!(seq_to_sql(u64::MAX), Err(Error::SeqOutOfRange)));
        assert!(matches!(seq_from_sql(-1), Err(Error::Corrupt)));
        assert!(matches!(generation_from_sql(0), Err(Error::Corrupt)));
        assert!(matches!(generation_from_sql(1 << 40), Err(Error::Corrupt)));
        assert!(matches!(vault_id_from_sql(&[0; 16]), Err(Error::Corrupt)));
        let g = |n| KeyGeneration::new(n).unwrap();
        assert!(check_generation(g(2), g(2)).is_ok());
        assert!(matches!(
            check_generation(g(2), g(1)),
            Err(Error::KeyGenerationRollback {
                highest_accepted: 2,
                offered: 1
            })
        ));
        assert!(matches!(
            check_generation(g(2), g(3)),
            Err(Error::KeyGenerationNotAccepted {
                highest_accepted: 2,
                offered: 3
            })
        ));
    }

    #[test]
    fn errors_display_without_data() {
        let e = Error::KeyGenerationRollback {
            highest_accepted: 3,
            offered: 2,
        };
        assert_eq!(
            e.to_string(),
            "key generation 2 is below the accepted generation 3"
        );
        let io: Error = std::io::Error::other("x").into();
        assert!(std::error::Error::source(&io).is_some());
    }
}
