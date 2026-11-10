//! Storage of the CLI session cache (CRYPTO_SPEC §6.8): the keystore part
//! (`CliSessionKey`: `session_id || W_cli`, entry `mosaic-cli-session-<account
//! id>`) and the file part (`<runtime_dir>/mosaic/session-<account id
//! hex32>.mcls`, 0600 in a 0700 directory), kept together.
//!
//! Sealing and opening are `mosaic_core`'s; this module only stores, deletes
//! and sweeps. Nothing here decides that a session is valid.
//!
//! Expiry is enforced when a session is opened, and also by deleting: on
//! Windows and macOS both parts survive a reboot, so every `mosaic` run and
//! the desktop call [`CliSessionStore::sweep_expired`] (at start, lock and
//! unlock), and every credential change calls
//! [`CliSessionStore::delete_all`]. The sweep trusts the unauthenticated
//! `expires_at` of the header: deleting is fail-safe, a forged value can only
//! end a session early. Only files can be enumerated (the keystore has no
//! listing), so a keystore entry whose file is already gone is removed by
//! [`CliSessionStore::delete`] of its account, not by the sweep.

use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mosaic_core::{AccountId, CliSessionFile, CliSessionKey, SecretString};

use crate::keystore::{CLI_SESSION_PREFIX, SERVICE, delete_entry, entry, lock};
use crate::{PlatformError, Result};

const FILE_PREFIX: &str = "session-";
const FILE_SUFFIX: &str = ".mcls";
const DIR_NAME: &str = "mosaic";

/// Where the CLI's session (both parts) lives.
pub trait CliSessionStore: Send + Sync {
    /// Stores both parts of the session of `account`, replacing any previous
    /// one. If the file cannot be written, the keystore part is removed again.
    ///
    /// # Errors
    /// When either part cannot be written (the caller then runs without a
    /// session).
    fn save(&self, account: AccountId, key: &CliSessionKey, file: &CliSessionFile) -> Result<()>;

    /// Both parts, or `None` when either is missing or damaged.
    ///
    /// # Errors
    /// When the keystore or the file cannot be read.
    fn load(&self, account: AccountId) -> Result<Option<(CliSessionKey, CliSessionFile)>>;

    /// Deletes both parts of `account` (no-op when absent). Both deletions
    /// are tried even if the first fails.
    ///
    /// # Errors
    /// The first deletion error.
    fn delete(&self, account: AccountId) -> Result<()>;

    /// Deletes every session whose file claims an `expires_at` before
    /// `now_ms` (Unix milliseconds), and every session file that cannot be
    /// parsed. Returns how many sessions it removed.
    ///
    /// # Errors
    /// The first error; the other sessions are still tried.
    fn sweep_expired(&self, now_ms: u64) -> Result<usize>;

    /// Deletes every session: for a password change, Secret Key rotation,
    /// profile upgrade or re-key. Returns how many it removed.
    ///
    /// # Errors
    /// The first error; the other sessions are still tried.
    fn delete_all(&self) -> Result<usize>;
}

/// A session store in process memory only (tests, and the fallback when no OS
/// store is available: it then caches nothing across processes).
#[derive(Default)]
pub struct MemoryCliSessions(Mutex<BTreeMap<AccountId, (SecretString, CliSessionFile)>>);

impl CliSessionStore for MemoryCliSessions {
    fn save(&self, account: AccountId, key: &CliSessionKey, file: &CliSessionFile) -> Result<()> {
        let value = key.to_keystore_value()?;
        lock(&self.0)?.insert(account, (value, file.clone()));
        Ok(())
    }

    fn load(&self, account: AccountId) -> Result<Option<(CliSessionKey, CliSessionFile)>> {
        let guard = lock(&self.0)?;
        Ok(guard.get(&account).and_then(|(value, file)| {
            CliSessionKey::from_keystore_value(value.expose_secret())
                .ok()
                .map(|key| (key, file.clone()))
        }))
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        lock(&self.0)?.remove(&account);
        Ok(())
    }

    fn sweep_expired(&self, now_ms: u64) -> Result<usize> {
        let mut guard = lock(&self.0)?;
        let before = guard.len();
        guard.retain(|_, (_, file)| file.claimed_expires_at_ms().is_some_and(|at| at >= now_ms));
        Ok(before - guard.len())
    }

    fn delete_all(&self) -> Result<usize> {
        let mut guard = lock(&self.0)?;
        let count = guard.len();
        guard.clear();
        Ok(count)
    }
}

/// The OS keystore part plus the session files in a directory.
pub struct OsCliSessions {
    service: String,
    dir: PathBuf,
}

impl OsCliSessions {
    /// The app's session store: keystore entries under [`SERVICE`], files in
    /// `<runtime_dir>/mosaic`.
    ///
    /// # Errors
    /// [`PlatformError::NoRuntimeDir`] on Linux without `$XDG_RUNTIME_DIR`:
    /// there is no session then (no persistent fallback).
    pub fn new() -> Result<Self> {
        Ok(Self::with_dir(
            SERVICE,
            crate::paths::runtime_dir()?.join(DIR_NAME),
        ))
    }

    /// Entries under `service`, files in `dir` (tests use their own).
    #[must_use]
    pub fn with_dir(service: &str, dir: PathBuf) -> Self {
        Self {
            service: service.to_owned(),
            dir,
        }
    }

    fn entry(&self, account: AccountId) -> Result<keyring::Entry> {
        entry(&self.service, &format!("{CLI_SESSION_PREFIX}{account}"))
    }

    fn path(&self, account: AccountId) -> PathBuf {
        self.dir.join(file_name(account))
    }

    /// The session files in the directory, by account.
    fn listed(&self) -> Result<Vec<(AccountId, PathBuf)>> {
        let read = match fs::read_dir(&self.dir) {
            Ok(read) => read,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(PlatformError::File),
        };
        Ok(read
            .filter_map(core::result::Result::ok)
            .filter_map(|e| {
                let name = e.file_name();
                let account = parse_file_name(name.to_str()?)?;
                Some((account, e.path()))
            })
            .collect())
    }

    fn remove_file(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(PlatformError::File),
        }
    }

    fn write_file(&self, account: AccountId, file: &CliSessionFile) -> Result<()> {
        create_private_dir(&self.dir)?;
        let temp = self.dir.join(format!(".{}.tmp", file_name(account)));
        let written = open_private(&temp)
            .and_then(|mut f| f.write_all(file.as_bytes()).and_then(|()| f.sync_all()))
            .map_err(|_| PlatformError::File);
        if written.is_err() {
            let _ = fs::remove_file(&temp);
            return written;
        }
        fs::rename(&temp, self.path(account)).map_err(|_| {
            let _ = fs::remove_file(&temp);
            PlatformError::File
        })
    }
}

impl CliSessionStore for OsCliSessions {
    fn save(&self, account: AccountId, key: &CliSessionKey, file: &CliSessionFile) -> Result<()> {
        let value = key.to_keystore_value()?;
        self.entry(account)?
            .set_password(value.expose_secret())
            .map_err(|_| PlatformError::Keystore)?;
        self.write_file(account, file).inspect_err(|_| {
            let _ = self.entry(account).and_then(|e| delete_entry(&e));
        })
    }

    fn load(&self, account: AccountId) -> Result<Option<(CliSessionKey, CliSessionFile)>> {
        let value = match self.entry(account)?.get_password() {
            Ok(text) => SecretString::from(text),
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(_) => return Err(PlatformError::Keystore),
        };
        let bytes = match fs::read(self.path(account)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(PlatformError::File),
        };
        // A damaged part reads as no session.
        let (Ok(key), Ok(file)) = (
            CliSessionKey::from_keystore_value(value.expose_secret()),
            CliSessionFile::from_bytes(&bytes),
        ) else {
            return Ok(None);
        };
        Ok(Some((key, file)))
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        let key = self.entry(account).and_then(|e| delete_entry(&e));
        let file = Self::remove_file(&self.path(account));
        key.and(file)
    }

    fn sweep_expired(&self, now_ms: u64) -> Result<usize> {
        let mut removed = 0;
        let mut first_error = None;
        for (account, path) in self.listed()? {
            let expired = match fs::read(&path) {
                Ok(bytes) => CliSessionFile::from_bytes(&bytes)
                    .ok()
                    .and_then(|f| f.claimed_expires_at_ms())
                    .is_none_or(|at| at < now_ms),
                // Unreadable now: leave it for the next run.
                Err(_) => false,
            };
            if expired {
                match self.delete(account) {
                    Ok(()) => removed += 1,
                    Err(e) => first_error = first_error.or(Some(e)),
                }
            }
        }
        first_error.map_or(Ok(removed), Err)
    }

    fn delete_all(&self) -> Result<usize> {
        let mut removed = 0;
        let mut first_error = None;
        for (account, _) in self.listed()? {
            match self.delete(account) {
                Ok(()) => removed += 1,
                Err(e) => first_error = first_error.or(Some(e)),
            }
        }
        first_error.map_or(Ok(removed), Err)
    }
}

/// `session-<account id as 32 lowercase hex digits>.mcls`.
fn file_name(account: AccountId) -> String {
    use core::fmt::Write as _;
    let mut name = String::from(FILE_PREFIX);
    for b in account.as_bytes() {
        let _ = write!(name, "{b:02x}");
    }
    name.push_str(FILE_SUFFIX);
    name
}

fn parse_file_name(name: &str) -> Option<AccountId> {
    let hex = name.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_SUFFIX)?;
    if hex.len() != 32 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    AccountId::from_bytes(bytes).ok()
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|_| PlatformError::File)?;
    // An existing directory keeps its mode from before: tighten it.
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|_| PlatformError::File)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> Result<()> {
    // The per-user local data directory is private to the user by its ACL.
    fs::create_dir_all(dir).map_err(|_| PlatformError::File)
}

#[cfg(unix)]
fn open_private(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_private(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountKey, AccountSalt, CliSessionBinding, InstallId, seal_cli_session};

    use super::*;

    const NOW: u64 = 1_800_000_000_000;
    const MIN: u64 = 60_000;

    fn make(account: AccountId, now: u64, ttl: u64) -> (CliSessionKey, CliSessionFile) {
        let binding = CliSessionBinding {
            account_id: account,
            install_id: InstallId::new_v7().unwrap(),
            ak_epoch: 1,
            salt: AccountSalt::generate().unwrap(),
        };
        seal_cli_session(&AccountKey::generate().unwrap(), &binding, now, ttl).unwrap()
    }

    #[test]
    fn session_file_names_round_trip() {
        let account = AccountId::new_v7().unwrap();
        let name = file_name(account);
        assert!(name.starts_with("session-") && name.ends_with(FILE_SUFFIX));
        assert_eq!(name.len(), "session-".len() + 32 + ".mcls".len());
        assert_eq!(parse_file_name(&name), Some(account));
        for bad in [
            "session-.mcls",
            "session-XYZ.mcls",
            "other.mcls",
            &name.to_uppercase(),
            &format!("{name}.tmp"),
            &format!(".{name}.tmp"),
        ] {
            assert_eq!(parse_file_name(bad), None, "{bad}");
        }
    }

    fn exercise(store: &dyn CliSessionStore) {
        let (a, b, c) = (
            AccountId::new_v7().unwrap(),
            AccountId::new_v7().unwrap(),
            AccountId::new_v7().unwrap(),
        );
        assert!(store.load(a).unwrap().is_none());
        let (ka, fa) = make(a, NOW, 5 * MIN);
        store.save(a, &ka, &fa).unwrap();
        let (loaded_key, loaded_file) = store.load(a).unwrap().unwrap();
        assert_eq!(loaded_key.session_id(), ka.session_id());
        assert_eq!(loaded_file, fa);
        assert!(store.load(b).unwrap().is_none(), "per account");

        // Expired sessions go, live ones stay (the instant itself is live).
        let (kb, fb) = make(b, NOW - 10 * MIN, MIN);
        store.save(b, &kb, &fb).unwrap();
        let (kc, fc) = make(c, NOW - 4 * MIN, 5 * MIN);
        store.save(c, &kc, &fc).unwrap();
        assert_eq!(store.sweep_expired(NOW).unwrap(), 1, "only b expired");
        assert!(store.load(b).unwrap().is_none());
        assert!(store.load(a).unwrap().is_some());
        assert!(store.load(c).unwrap().is_some(), "expires after now");
        assert_eq!(store.sweep_expired(NOW).unwrap(), 0, "idempotent");
        assert_eq!(store.sweep_expired(NOW + 5 * MIN + 1).unwrap(), 2);
        assert!(store.load(a).unwrap().is_none());

        store.save(a, &ka, &fa).unwrap();
        store.save(b, &kb, &fb).unwrap();
        store.delete(c).unwrap();
        assert_eq!(store.delete_all().unwrap(), 2);
        assert!(store.load(a).unwrap().is_none() && store.load(b).unwrap().is_none());
        assert_eq!(store.delete_all().unwrap(), 0);
    }

    #[test]
    fn memory_sessions_save_sweep_and_delete_all() {
        exercise(&MemoryCliSessions::default());
    }

    #[test]
    fn unparsable_session_files_are_swept_and_other_files_are_left() {
        let dir = std::env::temp_dir().join(format!(
            "mosaic-platform-test-{}",
            AccountId::new_v7().unwrap()
        ));
        fs::create_dir_all(&dir).unwrap();
        let account = AccountId::new_v7().unwrap();
        let store = OsCliSessions::with_dir("io.github.stephanbaum.mosaic.test", dir.clone());
        fs::write(store.path(account), b"junk").unwrap();
        fs::write(dir.join("notes.txt"), b"mine").unwrap();
        assert_eq!(store.sweep_expired(NOW).unwrap(), 1);
        assert!(!store.path(account).exists());
        assert!(dir.join("notes.txt").exists(), "foreign files stay");
        assert_eq!(store.delete_all().unwrap(), 0);
        // A missing directory is no sessions.
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(store.sweep_expired(NOW).unwrap(), 0);
        assert_eq!(store.delete_all().unwrap(), 0);
    }

    #[test]
    #[ignore = "writes to the OS keystore"]
    fn os_sessions_round_trip_sweep_and_delete_all() {
        let dir = std::env::temp_dir().join(format!(
            "mosaic-platform-test-{}",
            AccountId::new_v7().unwrap()
        ));
        let store = OsCliSessions::with_dir("io.github.stephanbaum.mosaic.test", dir.clone());
        exercise(&store);
        let _ = fs::remove_dir_all(dir);
    }
}
