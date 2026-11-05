//! The paired browsers: `browser-pairings.json` in the app data directory
//! (`CRYPTO_SPEC.md` §4, §15; ADR-0036 item 6).
//!
//! ```json
//! {"version":1,"pairings":[{"id":"<32 hex>","key":"<64 hex>","label":"Chrome",
//!   "browser":"chromium","createdAt":1761480000000,"lastUsedAt":null}]}
//! ```
//!
//! The key sits in the file in clear, like the extension's copy in its
//! `IndexedDB`: both are equally exposed to the same OS user, and a stolen key
//! is useless while the vault is locked (THREAT_MODEL R38). The file is
//! written atomically (temporary file, `fsync`, rename) with mode `0600` on
//! Unix; it is built in zeroizing memory. Keys are held as `PairingKey`
//! (zeroized on drop, no `Clone`, no `Debug` output).

use std::fmt::Write as _;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mosaic_core::browser_channel::{PAIRING_KEY_LEN, PairingKey, new_pairing_id};
use mosaic_native_messaging::protocol::{MAX_PAIRINGS, to_zeroizing};
use mosaic_native_messaging::{Hex, HexKey};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::contract::browser::BrowserFamily;
use crate::contract::ids::UnixMillis;

/// The file's name in the app data directory.
pub const FILE_NAME: &str = "browser-pairings.json";

/// `lastUsedAt` is written to disk at most this often per pairing.
pub const LAST_USED_SAVE_MS: UnixMillis = 10 * 60 * 1000;

/// The most bytes read from the file.
const MAX_FILE_BYTES: u64 = 64 * 1024;

/// The longest pairing label, in characters.
pub const MAX_LABEL_CHARS: usize = 64;

/// Why a change to the registry failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum RegistryError {
    /// [`MAX_PAIRINGS`] pairings exist.
    #[error("the browser pairing limit is reached")]
    Full,
    /// The OS random generator or the file failed.
    #[error("the browser pairings could not be saved")]
    Storage,
    /// The saved file exists but could not be read; it is left as it is.
    #[error("the saved browser pairings could not be read")]
    Unreadable,
}

/// One paired browser.
pub struct Pairing {
    /// The pairing ID.
    pub id: [u8; 16],
    /// `K_pair`. Shared with the connections that authenticate with it.
    pub key: Arc<PairingKey>,
    /// The user's name for it.
    pub label: String,
    /// The browser family the host verified at pairing.
    pub browser: BrowserFamily,
    /// When it was paired.
    pub created_at: UnixMillis,
    /// When it last filled or copied.
    pub last_used_at: Option<UnixMillis>,
    /// The value of `last_used_at` in the file.
    saved_used_at: Option<UnixMillis>,
}

/// All pairings, and where they are saved (`None`: memory only).
pub struct Registry {
    path: Option<PathBuf>,
    pairings: Vec<Pairing>,
    /// The file exists but could not be read (or is damaged): the registry
    /// is empty and read-only, so the file is never overwritten.
    unreadable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileIn {
    version: u8,
    pairings: Vec<PairingIn>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PairingIn {
    id: Hex<16>,
    key: HexKey,
    label: String,
    browser: BrowserFamily,
    created_at: UnixMillis,
    last_used_at: Option<UnixMillis>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileOut<'a> {
    version: u8,
    pairings: Vec<PairingOut<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairingOut<'a> {
    id: Hex<16>,
    key: KeyOut<'a>,
    label: &'a str,
    browser: BrowserFamily,
    created_at: UnixMillis,
    last_used_at: Option<UnixMillis>,
}

/// A key as 64 lowercase hex digits, built in zeroizing memory.
struct KeyOut<'a>(&'a PairingKey);

impl Serialize for KeyOut<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut text = Zeroizing::new(String::with_capacity(PAIRING_KEY_LEN * 2));
        for byte in self.0.expose_secret() {
            write!(text, "{byte:02x}").map_err(serde::ser::Error::custom)?;
        }
        serializer.serialize_str(&text)
    }
}

/// A label the user may give: 1 to 64 characters, no control, format or
/// default-ignorable characters (`is_spoofing_char`).
#[must_use]
pub fn label_is_valid(label: &str) -> bool {
    let count = label.chars().count();
    (1..=MAX_LABEL_CHARS).contains(&count)
        && !label
            .chars()
            .any(|c| c.is_control() || crate::commands::is_spoofing_char(c))
        && !label.trim().is_empty()
}

impl Registry {
    /// A registry that is never saved.
    #[must_use]
    pub const fn memory() -> Self {
        Self {
            path: None,
            pairings: Vec::new(),
            unreadable: false,
        }
    }

    /// The registry saved in `dir`. A missing file is an empty registry. Any
    /// other failure to read it (permissions, a damaged, oversized or
    /// unknown file) gives an empty registry that is read-only: nothing is
    /// added or removed, and the file stays as it is until the user fixes
    /// it, so a transient error cannot cost every pairing.
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        let path = dir.join(FILE_NAME);
        let (pairings, unreadable) = match read_file(&path) {
            Loaded::Missing => (Vec::new(), false),
            Loaded::Read(pairings) => (pairings, false),
            Loaded::Unreadable => (Vec::new(), true),
        };
        Self {
            path: Some(path),
            pairings,
            unreadable,
        }
    }

    /// Whether the saved file could not be read (the registry is read-only).
    #[must_use]
    pub const fn is_unreadable(&self) -> bool {
        self.unreadable
    }

    /// The pairings, oldest first.
    #[must_use]
    pub fn list(&self) -> &[Pairing] {
        &self.pairings
    }

    /// One pairing.
    #[must_use]
    pub fn get(&self, id: &[u8; 16]) -> Option<&Pairing> {
        self.pairings.iter().find(|p| p.id == *id)
    }

    /// Adds a pairing with a fresh ID and key and saves the file. Returns
    /// the ID and a second handle on the same key for `pairApproved`.
    ///
    /// # Errors
    /// [`RegistryError::Full`], [`RegistryError::Storage`],
    /// [`RegistryError::Unreadable`] (nothing is added then).
    pub fn add(
        &mut self,
        label: String,
        browser: BrowserFamily,
        now: UnixMillis,
    ) -> Result<([u8; 16], PairingKey), RegistryError> {
        if self.unreadable {
            return Err(RegistryError::Unreadable);
        }
        if self.pairings.len() >= MAX_PAIRINGS {
            return Err(RegistryError::Full);
        }
        let id = new_pairing_id().map_err(|_| RegistryError::Storage)?;
        let key = PairingKey::generate().map_err(|_| RegistryError::Storage)?;
        let mut copy = *key.expose_secret();
        // `from_bytes` zeroizes `copy`.
        let handed_out = PairingKey::from_bytes(&mut copy);
        self.pairings.push(Pairing {
            id,
            key: Arc::new(key),
            label,
            browser,
            created_at: now,
            last_used_at: None,
            saved_used_at: None,
        });
        if let Err(e) = self.save() {
            self.pairings.pop();
            return Err(e);
        }
        Ok((id, handed_out))
    }

    /// Removes a pairing and saves the file. `false`: no such pairing.
    ///
    /// # Errors
    /// [`RegistryError::Storage`] (the pairing stays then),
    /// [`RegistryError::Unreadable`].
    pub fn remove(&mut self, id: &[u8; 16]) -> Result<bool, RegistryError> {
        if self.unreadable {
            return Err(RegistryError::Unreadable);
        }
        let Some(at) = self.pairings.iter().position(|p| p.id == *id) else {
            return Ok(false);
        };
        let removed = self.pairings.remove(at);
        if let Err(e) = self.save() {
            self.pairings.insert(at, removed);
            return Err(e);
        }
        Ok(true)
    }

    /// Records a use. The file is rewritten at most every
    /// [`LAST_USED_SAVE_MS`] per pairing; a failed write is retried at the
    /// next use.
    pub fn note_used(&mut self, id: &[u8; 16], now: UnixMillis) {
        let Some(pairing) = self.pairings.iter_mut().find(|p| p.id == *id) else {
            return;
        };
        pairing.last_used_at = Some(now);
        let due = pairing
            .saved_used_at
            .is_none_or(|saved| now.saturating_sub(saved) >= LAST_USED_SAVE_MS);
        if due && self.save().is_ok() {
            for pairing in self.pairings.iter_mut().filter(|p| p.id == *id) {
                pairing.saved_used_at = Some(now);
            }
        }
    }

    fn save(&self) -> Result<(), RegistryError> {
        if self.unreadable {
            return Err(RegistryError::Unreadable);
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = FileOut {
            version: 1,
            pairings: self
                .pairings
                .iter()
                .map(|p| PairingOut {
                    id: Hex(p.id),
                    key: KeyOut(&p.key),
                    label: &p.label,
                    browser: p.browser,
                    created_at: p.created_at,
                    last_used_at: p.last_used_at,
                })
                .collect(),
        };
        let bytes = to_zeroizing(&file).map_err(|_| RegistryError::Storage)?;
        write_atomic(path, &bytes).map_err(|_| RegistryError::Storage)
    }
}

/// What reading the file gave.
enum Loaded {
    /// There is no file yet.
    Missing,
    /// The pairings in it.
    Read(Vec<Pairing>),
    /// It exists but cannot be used.
    Unreadable,
}

fn read_file(path: &Path) -> Loaded {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Missing,
        Err(_) => return Loaded::Unreadable,
    };
    parse_file(file).map_or(Loaded::Unreadable, Loaded::Read)
}

fn parse_file(file: fs::File) -> Option<Vec<Pairing>> {
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes).ok()?;
    if u64::try_from(bytes.len()).ok()? > MAX_FILE_BYTES {
        return None;
    }
    let parsed: FileIn = serde_json::from_slice(&bytes).ok()?;
    if parsed.version != 1 || parsed.pairings.len() > MAX_PAIRINGS {
        return None;
    }
    let mut out: Vec<Pairing> = Vec::new();
    for entry in parsed.pairings {
        if out.iter().any(|p| p.id == entry.id.0) || !label_is_valid(&entry.label) {
            return None;
        }
        out.push(Pairing {
            id: entry.id.0,
            key: Arc::new(entry.key.0),
            label: entry.label,
            browser: entry.browser,
            created_at: entry.created_at,
            last_used_at: entry.last_used_at,
            saved_used_at: entry.last_used_at,
        });
    }
    Some(out)
}

/// Writes `bytes` to a temporary file next to `path` and renames it over
/// `path` (the pattern of `persist::write_atomic`), private to the user on
/// Unix. A leftover temporary file is removed first (it could have other
/// permissions), the new one is created exclusively, and the temporary file
/// is removed again if anything fails.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    match fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let written = write_tmp_then_rename(&tmp, path, bytes);
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

fn write_tmp_then_rename(tmp: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = create_private(tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

#[cfg(not(unix))]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_checked() {
        assert!(label_is_valid("Chrome"));
        assert!(label_is_valid(&"x".repeat(64)));
        assert!(!label_is_valid(""));
        assert!(!label_is_valid("   "));
        assert!(!label_is_valid(&"x".repeat(65)));
        assert!(!label_is_valid("Chr\u{202e}ome"), "bidi override");
        assert!(!label_is_valid("a\u{200b}b"), "zero width space");
        assert!(!label_is_valid("a\nb"));
    }

    #[test]
    fn a_registry_round_trips_through_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::in_dir(dir.path());
        assert!(registry.list().is_empty());
        let (id, key) = registry
            .add("Chrome".into(), BrowserFamily::Chromium, 1_000)
            .unwrap();
        assert_eq!(
            key.expose_secret(),
            registry.get(&id).unwrap().key.expose_secret()
        );
        registry.note_used(&id, 2_000);

        let text = fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert!(text.starts_with("{\"version\":1,"), "{text}");
        assert!(text.contains("\"label\":\"Chrome\""), "{text}");

        let reloaded = Registry::in_dir(dir.path());
        let pairing = reloaded.get(&id).unwrap();
        assert_eq!(pairing.key.expose_secret(), key.expose_secret());
        assert_eq!(pairing.created_at, 1_000);
        assert_eq!(pairing.last_used_at, Some(2_000));
    }

    #[test]
    fn last_used_is_saved_at_most_every_ten_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::in_dir(dir.path());
        let (id, _) = registry
            .add("Chrome".into(), BrowserFamily::Chromium, 0)
            .unwrap();
        registry.note_used(&id, 1_000);
        registry.note_used(&id, 2_000);
        assert_eq!(
            Registry::in_dir(dir.path()).get(&id).unwrap().last_used_at,
            Some(1_000),
            "the second use is only in memory"
        );
        registry.note_used(&id, 1_000 + LAST_USED_SAVE_MS);
        assert_eq!(
            Registry::in_dir(dir.path()).get(&id).unwrap().last_used_at,
            Some(1_000 + LAST_USED_SAVE_MS)
        );
        assert_eq!(
            registry.get(&id).unwrap().last_used_at,
            Some(1_000 + LAST_USED_SAVE_MS)
        );
    }

    #[test]
    fn the_seventeenth_pairing_is_refused_and_removal_works() {
        let mut registry = Registry::memory();
        let mut ids = Vec::new();
        for n in 0..MAX_PAIRINGS {
            let (id, _) = registry
                .add(format!("B{n}"), BrowserFamily::Firefox, 0)
                .unwrap();
            ids.push(id);
        }
        assert_eq!(
            registry
                .add("one more".into(), BrowserFamily::Firefox, 0)
                .err(),
            Some(RegistryError::Full)
        );
        assert_eq!(registry.remove(&ids[3]), Ok(true));
        assert_eq!(registry.remove(&ids[3]), Ok(false));
        assert!(
            registry
                .add("again".into(), BrowserFamily::Firefox, 0)
                .is_ok()
        );
    }

    #[test]
    fn a_damaged_file_is_an_empty_read_only_registry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        for bad in [
            "not json".to_owned(),
            r#"{"version":2,"pairings":[]}"#.to_owned(),
            r#"{"version":1,"pairings":[],"extra":1}"#.to_owned(),
            format!(
                r#"{{"version":1,"pairings":[{{"id":"{}","key":"zz","label":"x","browser":"chromium","createdAt":0,"lastUsedAt":null}}]}}"#,
                "ab".repeat(16)
            ),
            format!(
                r#"{{"version":1,"pairings":[{{"id":"{}","key":"{}","label":"","browser":"chromium","createdAt":0,"lastUsedAt":null}}]}}"#,
                "ab".repeat(16),
                "cd".repeat(32)
            ),
        ] {
            fs::write(&path, &bad).unwrap();
            let mut registry = Registry::in_dir(dir.path());
            assert!(registry.list().is_empty());
            assert!(registry.is_unreadable());
            assert_eq!(
                registry
                    .add("Chrome".into(), BrowserFamily::Chromium, 0)
                    .err(),
                Some(RegistryError::Unreadable)
            );
            assert_eq!(registry.remove(&[1; 16]), Err(RegistryError::Unreadable));
            registry.note_used(&[1; 16], 5);
            assert_eq!(fs::read(&path).unwrap(), bad.as_bytes(), "left as it was");
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_registry_that_can_be_written() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::in_dir(dir.path());
        assert!(!registry.is_unreadable());
        assert!(
            registry
                .add("Chrome".into(), BrowserFamily::Chromium, 0)
                .is_ok()
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_is_unreadable_not_missing() {
        // A directory where the file should be: opening or reading it fails,
        // which is never "not found".
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(FILE_NAME)).unwrap();
        assert!(Registry::in_dir(dir.path()).is_unreadable());
    }

    #[test]
    fn a_leftover_temporary_file_does_not_stop_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("browser-pairings.json.tmp");
        fs::write(&tmp, "old").unwrap();
        let mut registry = Registry::in_dir(dir.path());
        registry
            .add("Chrome".into(), BrowserFamily::Chromium, 0)
            .unwrap();
        assert!(!tmp.exists(), "renamed over the file");
        assert_eq!(Registry::in_dir(dir.path()).list().len(), 1);
    }

    #[test]
    fn a_failed_write_removes_the_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        // The target is a directory: the rename fails after the write.
        let target = dir.path().join("target.json");
        fs::create_dir(&target).unwrap();
        assert!(write_atomic(&target, b"{}").is_err());
        assert!(!dir.path().join("target.json.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private_on_unix() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::in_dir(dir.path());
        registry
            .add("Chrome".into(), BrowserFamily::Chromium, 0)
            .unwrap();
        let mode = fs::metadata(dir.path().join(FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn a_leftover_temporary_file_with_open_permissions_is_not_reused() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("browser-pairings.json.tmp");
        fs::write(&tmp, "old").unwrap();
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644)).unwrap();
        let mut registry = Registry::in_dir(dir.path());
        registry
            .add("Chrome".into(), BrowserFamily::Chromium, 0)
            .unwrap();
        let mode = fs::metadata(dir.path().join(FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
