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
//! | `vaults` | key wrapping (AK or TK), current and highest accepted key generation, [`VaultMetaCiphertext`] (§7.1, §7.4) |
//! | `vault_generation_floors` | the highest accepted generation of deleted vaults (§7.1) |
//! | `vault_keys` | one [`WrappedKey`] of VK per generation (§7.1) |
//! | `tk_wrappings` | one [`WrappedKey`] of TK per hardware key, for a hardware-protected TOTP vault (§7.4) |
//! | `records` | [`ItemCiphertext`] / [`TombstoneCiphertext`], `server_seq`, outbox (§7.2, §10) |
//! | `sync_cursors` | per-vault pull cursor (§10) |
//! | `attachments` | routing metadata; the ciphertext is a file named by the attachment ID (§7.3) |
//! | `local_device` | this install's device ID, which stamps local edits (§10, public per §12) |
//!
//! The API only takes and returns `mosaic-core` ciphertext types, so the
//! store cannot be handed a plaintext or a key. Every object is decoded
//! strictly on the way in and again on the way out. Apart from IDs, key
//! generations, sizes and sequence numbers (all of which the server sees,
//! §12), nothing is stored in cleartext (ADR-0010).
//!
//! **Anti-rollback (§7.1).** Each vault has a highest accepted key
//! generation that never decreases, not even across deleting and re-adding
//! the vault. New wrapped vault keys below it are rejected
//! ([`Error::KeyGenerationRollback`]), and every record and the vault
//! metadata must be written at exactly that generation: lower is a
//! rollback, higher is a generation whose key the caller has not accepted
//! ([`Error::KeyGenerationNotAccepted`]). Triggers in the schema repeat
//! these rules.
//!
//! **Record versions (§10).** The store does not interpret versions. The
//! highest accepted version vector of a record is the one inside its stored
//! ciphertext: the caller decrypts the stored record and the incoming one,
//! compares their authenticated vectors, and only then writes. The store
//! adds two cheap checks of its own: a record's `server_seq` never
//! decreases, and a remote write or a delete never silently drops a local
//! change that is still in the outbox ([`Error::PendingLocalChange`]).
//!
//! **One live session per store.** [`Store::open`] takes an exclusive OS
//! lock on a sidecar file `<path>-lock` and holds it until the store is
//! dropped or closed; a second open of the same file, from this process or
//! another one, fails with [`Error::StoreInUse`]. Two sessions on one file
//! would share one device ID and each edit its own stale copy of the
//! items, so one would overwrite the other's edit without a conflict
//! (ADR-0010 item 10).
//!
//! Durability: WAL journal, `synchronous = FULL`, foreign keys on,
//! `secure_delete` on (deleted rows are overwritten), `trusted_schema` off.
//! Every call that removes or replaces a wrapped key checkpoints and
//! truncates the WAL before it returns, so the old wrapping does not stay
//! behind in WAL frames. Attachment files are written to a temporary file,
//! synced, and renamed into place.
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
//! let queued = store.upsert_record(&vault, &item, &body, Origin::Local)?;
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
mod device;
mod error;
mod records;
mod schema;
mod vaults;

use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

pub use account::{AccountRekey, NewUnlockMethod, StoredUnlockMethod};
pub use attachments::{AttachmentInfo, validate_attachment_ciphertext};
pub use error::{Error, Result};
pub use records::{
    CorruptRecord, Origin, OutboxSeq, RecordBody, RecordEntry, RecordKind, StoredRecord,
};
pub use schema::LATEST_VERSION;
pub use vaults::{StoredTkWrapping, VaultKeyWrap, VaultState};

#[cfg(doc)]
use mosaic_core::{ItemCiphertext, TombstoneCiphertext, VaultMetaCiphertext, WrappedKey};
use mosaic_core::{KeyGeneration, VaultId};

/// Maximum length of an unlock method's (or a TK wrapping's) public
/// parameters.
pub const MAX_UNLOCK_PARAMS_LEN: usize = 4096;

/// What a delete does with local changes that are not uploaded yet: a
/// record in the outbox, or an attachment with its upload still pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingPolicy {
    /// Refuse the delete with [`Error::PendingLocalChange`], so no local
    /// change is lost silently (BRIEF §5).
    Refuse,
    /// Delete anyway and drop the pending changes. Only for an explicit
    /// user decision (for example "discard unsynced changes").
    Discard,
}

/// How long a write waits for another connection's lock before failing.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Suffix of the attachment directory next to the database file.
const ATTACHMENT_DIR_SUFFIX: &str = "-attachments";

/// Suffix of the lock file next to the database file.
const LOCK_FILE_SUFFIX: &str = "-lock";

/// The local store. See the [module documentation](self).
pub struct Store {
    conn: Connection,
    attachment_dir: Option<PathBuf>,
    /// The exclusive lock of a file store, held for the store's lifetime
    /// and released when the file is closed.
    _lock: Option<File>,
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
    /// [`Error::StoreInUse`] if the store is already open (in this process
    /// or another one), [`Error::NotAMosaicStore`] for another SQLite
    /// database,
    /// [`Error::SchemaTooNew`] for a store from a newer build,
    /// [`Error::Database`] / [`Error::Io`] otherwise.
    ///
    /// A foreign database or a newer store is refused before anything is
    /// written to it. On Unix, the database, its WAL and shared-memory
    /// files and the attachment directory are made owner-only (`0600` /
    /// `0700`), also when they already existed with looser permissions.
    ///
    /// `path` is resolved first (symbolic links, `.` and `..`, relative
    /// paths), and the lock file and the attachment directory are named
    /// after the resolved path, so every spelling of one database, a
    /// symbolic link to it included, shares one lock. A hard link to the
    /// database file is not detectable this way (SQLite has the same
    /// caveat) and is a residual risk (`THREAT_MODEL` R13).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        attachment_dir_for(path)?;
        attachments::prepare_db_file(path)?;
        let resolved = resolve_store_path(path)?;
        let path = resolved.as_path();
        let attachment_dir = attachment_dir_for(path)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        // Read-only, so a foreign file gets no lock file next to it; the
        // lock is taken before anything is written.
        schema::check_compatible(&conn, schema::latest_of(schema::MIGRATIONS))?;
        let lock = lock_store(path)?;
        let store = Self::init(conn, Some(attachment_dir), Some(lock))?;
        attachments::restrict_db_files(path)?;
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
        Self::init(Connection::open_in_memory()?, None, None)
    }

    fn init(
        mut conn: Connection,
        attachment_dir: Option<PathBuf>,
        lock: Option<File>,
    ) -> Result<Self> {
        // Connection settings only; none of them writes to the file.
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "trusted_schema", false)?;
        conn.pragma_update(None, "secure_delete", true)?;
        // Read-only check before the first write: switching to WAL changes
        // the file header, which must not happen to a foreign database.
        schema::check_compatible(&conn, schema::latest_of(schema::MIGRATIONS))?;
        // In memory the journal mode stays "memory"; on disk it is WAL.
        let _mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "wal", |r| r.get(0))?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        // Truncate the WAL file whenever it is reset, so stale frames
        // (old versions of deleted rows) do not linger past a checkpoint.
        let _limit: i64 =
            conn.pragma_update_and_check(None, "journal_size_limit", 0, |r| r.get(0))?;
        schema::migrate(&mut conn, schema::MIGRATIONS)?;
        Ok(Self {
            conn,
            attachment_dir,
            _lock: lock,
        })
    }

    /// The raw connection, so unit tests elsewhere in the crate can damage
    /// rows the way an outside writer would.
    #[cfg(test)]
    pub(crate) const fn conn_for_tests(&self) -> &Connection {
        &self.conn
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
        let invariants: [(&str, &str); 4] = [
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
            (
                "SELECT lower(hex(vault_id)) FROM vaults v JOIN vault_generation_floors f
                 USING (vault_id) WHERE v.highest_accepted_generation < f.generation",
                "accepted generation below its floor",
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

    /// Moves every committed page from the WAL into the database file
    /// (where `secure_delete` has already overwritten deleted rows) and
    /// truncates the WAL to zero bytes. Called after every change that
    /// removes or replaces a wrapped key (§6.2, §6.4, §6.6), so the old
    /// wrapping is not left in WAL frames if the process dies later.
    ///
    /// The checkpoint cannot finish while another connection is reading an
    /// older snapshot; the frames are then removed by a later checkpoint
    /// (at the latest on [`Store::close`]). A store is meant to be opened
    /// by one process, so this is the exception.
    fn scrub_wal(&self) -> Result<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }
}

/// Takes the exclusive lock of the store at `path` (an advisory OS lock on
/// `<path>-lock`, which stays behind empty). Waiting would only hide the
/// other session, so a held lock fails at once.
fn lock_store(path: &Path) -> Result<File> {
    let mut name = path.as_os_str().to_os_string();
    name.push(LOCK_FILE_SUFFIX);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(PathBuf::from(name))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(Error::StoreInUse),
        Err(TryLockError::Error(e)) => Err(e.into()),
    }
}

/// The absolute path of the store at `path` with every symbolic link,
/// `.` and `..` resolved. A database that does not exist yet (on systems
/// where `prepare_db_file` does not create it) is resolved through its
/// directory.
fn resolve_store_path(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().ok_or(Error::InvalidPath)?;
    let resolved = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = match path.parent() {
                Some(p) if !p.as_os_str().is_empty() => p,
                _ => Path::new("."),
            };
            std::fs::canonicalize(parent)?.join(name)
        }
        Err(e) => return Err(e.into()),
    };
    Ok(without_verbatim_prefix(resolved))
}

/// On Windows `canonicalize` returns `\\?\C:\...`; the plain `C:\...` form
/// names the same file and is what SQLite and users expect. Other paths
/// (and every path elsewhere) are returned unchanged.
fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut components = path.components();
        if let Some(Component::Prefix(prefix)) = components.next()
            && let Prefix::VerbatimDisk(drive) = prefix.kind()
        {
            let mut plain = PathBuf::from(format!("{}:", char::from(drive)));
            plain.push(components.as_path());
            return plain;
        }
    }
    path
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
        // The store resolves its path (macOS /var -> /private/var, Windows
        // 8.3 short names), so compare canonical parents, not raw strings.
        assert_eq!(att.file_name().unwrap(), "vault.db-attachments");
        assert_eq!(
            std::fs::canonicalize(att.parent().unwrap()).unwrap(),
            std::fs::canonicalize(dir.path()).unwrap()
        );
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

    /// Two sessions on one file would share a device ID and overwrite each
    /// other's edits from stale caches (sync review, t1); the second open
    /// is refused until the first store is closed or dropped.
    #[test]
    fn a_store_is_open_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let first = Store::open(&path).unwrap();
        assert!(matches!(Store::open(&path), Err(Error::StoreInUse)));
        // Another spelling of the same path is the same lock file.
        let other_spelling = dir.path().join(".").join("vault.db");
        assert!(matches!(
            Store::open(&other_spelling),
            Err(Error::StoreInUse)
        ));
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let dotted = dir.path().join("sub").join("..").join("vault.db");
        assert!(matches!(Store::open(&dotted), Err(Error::StoreInUse)));
        first.close().unwrap();
        let second = Store::open(&path).unwrap();
        assert!(matches!(Store::open(&path), Err(Error::StoreInUse)));
        drop(second);
        Store::open(&path).unwrap();
    }

    /// Review M2, third round (tests): a symbolic link to the database used
    /// to get its own lock file (`alias.db-lock`) and its own attachment
    /// directory while SQLite followed the link to the same database, so
    /// two sessions could open one store again.
    #[cfg(unix)]
    #[test]
    fn a_symlink_to_an_open_store_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let first = Store::open(&path).unwrap();
        let alias = dir.path().join("alias.db");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        assert!(matches!(Store::open(&alias), Err(Error::StoreInUse)));
        // A relative path to the same file shares the lock too.
        let linked_dir = dir.path().join("linked");
        std::os::unix::fs::symlink(dir.path(), &linked_dir).unwrap();
        assert!(matches!(
            Store::open(linked_dir.join("vault.db")),
            Err(Error::StoreInUse)
        ));
        let dir_of_first = first.attachment_dir().unwrap().to_path_buf();
        drop(first);
        let through_alias = Store::open(&alias).unwrap();
        assert_eq!(through_alias.attachment_dir().unwrap(), dir_of_first);
        assert!(!dir.path().join("alias.db-lock").exists());
        assert!(!dir.path().join("alias.db-attachments").exists());
    }

    /// Review M2, fourth round (tests): the symbolic-link refusal on
    /// Windows. Creating a symbolic link needs a privilege (or developer
    /// mode) there; without it the test has nothing to check and returns.
    #[cfg(windows)]
    #[test]
    fn a_windows_symlink_to_an_open_store_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let first = Store::open(&path).unwrap();
        let alias = dir.path().join("alias.db");
        if std::os::windows::fs::symlink_file(&path, &alias).is_err() {
            return;
        }
        assert!(matches!(Store::open(&alias), Err(Error::StoreInUse)));
        let dir_of_first = first.attachment_dir().unwrap().to_path_buf();
        drop(first);
        let through_alias = Store::open(&alias).unwrap();
        assert_eq!(through_alias.attachment_dir().unwrap(), dir_of_first);
        assert!(!dir.path().join("alias.db-lock").exists());
    }

    /// Serializes the tests that change the process's current directory.
    static CWD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Restores the current directory when dropped.
    struct CwdGuard(PathBuf);

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.0).unwrap();
        }
    }

    /// Review M2, fourth round (tests): a bare relative file name whose
    /// database does not exist yet resolves through the current directory,
    /// and every spelling of it (bare, `./`, absolute) shares one lock.
    #[test]
    fn a_relative_store_name_shares_the_lock_with_every_spelling() {
        let _serial = CWD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().unwrap();
        let _restore = CwdGuard(std::env::current_dir().unwrap());
        std::env::set_current_dir(dir.path()).unwrap();
        assert!(!Path::new("relative.db").exists());
        let cwd = without_verbatim_prefix(std::fs::canonicalize(".").unwrap());
        assert_eq!(
            resolve_store_path(Path::new("relative.db")).unwrap(),
            cwd.join("relative.db")
        );
        let first = Store::open("relative.db").unwrap();
        assert!(matches!(
            Store::open("./relative.db"),
            Err(Error::StoreInUse)
        ));
        assert!(matches!(
            Store::open(cwd.join("relative.db")),
            Err(Error::StoreInUse)
        ));
        assert_eq!(
            first.attachment_dir().unwrap(),
            cwd.join("relative.db-attachments")
        );
    }

    /// Review M2, fourth round (tests): only the verbatim disk prefix is
    /// removed; a verbatim UNC path and plain paths stay as they are.
    #[test]
    fn only_the_verbatim_disk_prefix_is_removed() {
        #[cfg(windows)]
        {
            assert_eq!(
                without_verbatim_prefix(PathBuf::from(r"\\?\C:\Users\x\vault.db")),
                PathBuf::from(r"C:\Users\x\vault.db")
            );
            for unchanged in [
                r"\\?\UNC\server\share\vault.db",
                r"\\server\share\vault.db",
                r"C:\Users\x\vault.db",
                r"\\?\GLOBALROOT\Device\vault.db",
            ] {
                assert_eq!(
                    without_verbatim_prefix(PathBuf::from(unchanged)),
                    PathBuf::from(unchanged),
                    "{unchanged}"
                );
            }
        }
        assert_eq!(
            without_verbatim_prefix(PathBuf::from("/srv/vault.db")),
            PathBuf::from("/srv/vault.db")
        );
    }

    #[test]
    fn a_store_path_is_resolved_to_one_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let s = Store::open(&path).unwrap();
        let resolved = s.attachment_dir().unwrap().to_path_buf();
        assert!(resolved.is_absolute());
        assert!(!resolved.to_string_lossy().starts_with(r"\\?\"));
        assert_eq!(
            resolved.file_name().unwrap(),
            std::ffi::OsStr::new("vault.db-attachments")
        );
        drop(s);
        assert_eq!(
            resolve_store_path(&dir.path().join(".").join("vault.db")).unwrap(),
            resolve_store_path(&path).unwrap()
        );
        assert!(matches!(
            resolve_store_path(Path::new("..")),
            Err(Error::InvalidPath)
        ));
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
        c.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1);")
            .unwrap();
        drop(c);
        let before = std::fs::read(&other).unwrap();
        assert_eq!(before[18..20], [1, 1], "rollback-journal file");
        assert!(matches!(Store::open(&other), Err(Error::NotAMosaicStore)));
        // The foreign file is not touched: no switch to WAL, no side files.
        let after = std::fs::read(&other).unwrap();
        assert_eq!(after[18..20], [1, 1], "switched to WAL");
        assert!(after.iter().eq(before.iter()), "foreign file modified");
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["garbage.db", "other.db"]);
        assert!(std::fs::read(&garbage).unwrap().iter().all(|&b| b == 0x5A));

        // A foreign database with only a user_version is refused as well.
        let versioned = dir.path().join("versioned.db");
        let c = Connection::open(&versioned).unwrap();
        c.pragma_update(None, "user_version", 1).unwrap();
        drop(c);
        let before = std::fs::read(&versioned).unwrap();
        assert!(matches!(
            Store::open(&versioned),
            Err(Error::NotAMosaicStore)
        ));
        assert!(std::fs::read(&versioned).unwrap().iter().eq(before.iter()));
    }

    /// Unix: the database, its WAL and shared-memory files, the attachment
    /// directory and attachment files are owner-only, and a store found
    /// with looser permissions is tightened on open.
    #[cfg(unix)]
    #[test]
    fn files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        use mosaic_core::{
            AttachmentId, AttachmentKey, ItemCiphertext, ItemId, VaultKey, seal_attachment,
        };

        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.db");
        let wal = dir.path().join("v.db-wal");
        let shm = dir.path().join("v.db-shm");
        let mut s = Store::open(&path).unwrap();
        let (f, vk) = (
            vaults::tests::fixture(),
            VaultKey::generate(KeyGeneration::FIRST).unwrap(),
        );
        s.create_vault(
            &f.vault,
            KeyGeneration::FIRST,
            &vk.wrap(&f.ak, &f.account, &f.vault).unwrap(),
        )
        .unwrap();
        let rid = ItemId::new_v7().unwrap();
        let body = RecordBody::Item(ItemCiphertext::seal(&vk, &f.vault, &rid, 1, b"x").unwrap());
        s.upsert_record(&f.vault, &rid, &body, Origin::Local)
            .unwrap();
        let aid = AttachmentId::new_v7().unwrap();
        let ct = seal_attachment(
            &AttachmentKey::generate().unwrap(),
            &f.vault,
            &rid,
            &aid,
            b"file",
        )
        .unwrap();
        s.put_attachment(&f.vault, &rid, &aid, &ct, true).unwrap();
        let att_dir = s.attachment_dir().unwrap().to_path_buf();
        let att_file = std::fs::read_dir(&att_dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert!(wal.exists() && shm.exists());
        for p in [&path, &wal, &shm, &att_file] {
            assert_eq!(mode(p), 0o600, "{}", p.display());
        }
        assert_eq!(mode(&att_dir), 0o700);
        drop(s);

        // Loosened from outside: tightened again on open.
        for p in [&path, &wal, &shm] {
            if p.exists() {
                std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
        }
        std::fs::set_permissions(&att_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let s = Store::open(&path).unwrap();
        for p in [&path, &wal, &shm] {
            if p.exists() {
                assert_eq!(mode(p), 0o600, "{}", p.display());
            }
        }
        assert_eq!(mode(&att_dir), 0o700);
        drop(s);

        // A foreign file keeps its permissions.
        let other = dir.path().join("other.db");
        let c = Connection::open(&other).unwrap();
        c.execute_batch("CREATE TABLE t (x INTEGER);").unwrap();
        drop(c);
        std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(Store::open(&other), Err(Error::NotAMosaicStore)));
        assert_eq!(mode(&other), 0o644);
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
