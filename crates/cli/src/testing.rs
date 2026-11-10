//! Test backends (feature `test-util`, debug builds only): file-based stand-ins for the OS
//! keystore, the session store, the clipboard and the terminal, all under `MOSAIC_TEST_DIR`, plus a
//! settable clock. They exist so the integration tests can drive the real binary.
//!
//! - `MOSAIC_TEST_DIR/keystore/secret-key-<account hex>`: the Secret Key display string.
//! - `MOSAIC_TEST_DIR/run/session-<account hex>.mcls` and `keystore/session-<account hex>.key`.
//! - `MOSAIC_TEST_DIR/keystore/sync-token-<account hex>`: the hosted session token (hex).
//! - `MOSAIC_TEST_KEYSTORE=refuse`: saving a session fails, as a refusing keystore does.
//! - `MOSAIC_TEST_DIR/clipboard.txt`: the clipboard (a missing or empty file is an empty clipboard).
//! - `MOSAIC_TEST_PROMPTS`: a file of lines; every prompt (secret or not) consumes the first line.
//! - `MOSAIC_TEST_NOW_MS`: the clock.

use std::fs;
use std::path::PathBuf;

use mosaic_core::{AccountId, CliSessionFile, CliSessionKey, SecretKey, SecretString};
use mosaic_platform::PlatformError;
use mosaic_platform::clipboard::{Clipboard, ImageRead};
use mosaic_platform::keystore::{CliSessionStore, SecretKeyStore, SyncTokenStore};
use mosaic_sync::http::{StoredToken, Token as _};
use zeroize::Zeroizing;

use crate::backends::Backends;
use crate::error::{CliError, Result};
use crate::input::Terminal;

/// Lowercase hex of `bytes`.
#[must_use]
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}
type PResult<T> = core::result::Result<T, PlatformError>;

fn dir() -> Option<PathBuf> {
    std::env::var_os("MOSAIC_TEST_DIR").map(PathBuf::from)
}

struct FileKeys(PathBuf);

impl FileKeys {
    fn path(&self, account: AccountId) -> PathBuf {
        self.0
            .join("keystore")
            .join(format!("secret-key-{}", hex(account.as_bytes())))
    }
}

impl SecretKeyStore for FileKeys {
    fn load(&self, account: AccountId) -> PResult<Option<SecretKey>> {
        match fs::read_to_string(self.path(account)) {
            Ok(text) => Ok(SecretKey::parse(text.trim()).ok()),
            Err(_) => Ok(None),
        }
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> PResult<()> {
        let path = self.path(account);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| PlatformError::Keystore)?;
        }
        let display = key.to_display()?;
        fs::write(path, display.expose_secret()).map_err(|_| PlatformError::Keystore)
    }

    fn delete(&self, account: AccountId) -> PResult<()> {
        let _ = fs::remove_file(self.path(account));
        Ok(())
    }
}

/// `MOSAIC_TEST_DIR/keystore/sync-token-<account hex>`: the hosted session token (hex), so a token
/// stored by one `mosaic` run is found by the next.
struct FileTokens(PathBuf);

impl FileTokens {
    fn path(&self, account: AccountId) -> PathBuf {
        self.0
            .join("keystore")
            .join(format!("sync-token-{}", hex(account.as_bytes())))
    }
}

impl SyncTokenStore for FileTokens {
    fn load(&self, account: AccountId) -> PResult<Option<StoredToken>> {
        let Ok(text) = fs::read_to_string(self.path(account)) else {
            return Ok(None);
        };
        let text = text.trim();
        let mut bytes = Zeroizing::new([0u8; 32]);
        if text.len() != 64 || !text.is_ascii() {
            return Ok(None);
        }
        for (slot, pair) in bytes.iter_mut().zip(text.as_bytes().chunks(2)) {
            let pair = core::str::from_utf8(pair).map_err(|_| PlatformError::Keystore)?;
            let Ok(byte) = u8::from_str_radix(pair, 16) else {
                return Ok(None);
            };
            *slot = byte;
        }
        Ok(Some(StoredToken::from_array(&mut bytes)))
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> PResult<()> {
        let path = self.path(account);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| PlatformError::Keystore)?;
        }
        fs::write(path, hex(token.expose_bytes())).map_err(|_| PlatformError::Keystore)
    }

    fn delete(&self, account: AccountId) -> PResult<()> {
        let _ = fs::remove_file(self.path(account));
        Ok(())
    }
}

struct FileSessions {
    root: PathBuf,
    refuse: bool,
}

impl FileSessions {
    fn file(&self, hex_id: &str) -> PathBuf {
        self.root.join("run").join(format!("session-{hex_id}.mcls"))
    }

    fn key(&self, hex_id: &str) -> PathBuf {
        self.root
            .join("keystore")
            .join(format!("session-{hex_id}.key"))
    }

    fn remove(&self, hex_id: &str) {
        let _ = fs::remove_file(self.file(hex_id));
        let _ = fs::remove_file(self.key(hex_id));
    }

    fn ids(&self) -> Vec<String> {
        let Ok(read) = fs::read_dir(self.root.join("run")) else {
            return Vec::new();
        };
        read.filter_map(core::result::Result::ok)
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let id = name.strip_prefix("session-")?.strip_suffix(".mcls")?;
                Some(id.to_owned())
            })
            .collect()
    }
}

impl CliSessionStore for FileSessions {
    fn save(&self, account: AccountId, key: &CliSessionKey, file: &CliSessionFile) -> PResult<()> {
        if self.refuse {
            return Err(PlatformError::Keystore);
        }
        let id = hex(account.as_bytes());
        for path in [self.file(&id), self.key(&id)] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|_| PlatformError::File)?;
            }
        }
        fs::write(self.file(&id), file.as_bytes()).map_err(|_| PlatformError::File)?;
        let value = key.to_keystore_value()?;
        fs::write(self.key(&id), value.expose_secret()).map_err(|_| PlatformError::File)
    }

    fn load(&self, account: AccountId) -> PResult<Option<(CliSessionKey, CliSessionFile)>> {
        let id = hex(account.as_bytes());
        let (Ok(bytes), Ok(text)) = (fs::read(self.file(&id)), fs::read_to_string(self.key(&id)))
        else {
            return Ok(None);
        };
        let key = CliSessionKey::from_keystore_value(text.trim()).ok();
        let file = CliSessionFile::from_bytes(&bytes).ok();
        Ok(key.zip(file))
    }

    fn delete(&self, account: AccountId) -> PResult<()> {
        self.remove(&hex(account.as_bytes()));
        Ok(())
    }

    fn sweep_expired(&self, now_ms: u64) -> PResult<usize> {
        let mut removed = 0;
        for id in self.ids() {
            let claimed = fs::read(self.file(&id))
                .ok()
                .and_then(|b| CliSessionFile::from_bytes(&b).ok())
                .and_then(|f| f.claimed_expires_at_ms());
            if claimed.is_none_or(|expires| expires <= now_ms) {
                self.remove(&id);
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn delete_all(&self) -> PResult<usize> {
        let ids = self.ids();
        for id in &ids {
            self.remove(id);
        }
        Ok(ids.len())
    }
}

struct FileClipboard(PathBuf);

impl Clipboard for FileClipboard {
    fn set(&mut self, text: &str) -> PResult<()> {
        fs::write(&self.0, text).map_err(|_| PlatformError::Clipboard)
    }

    fn get_text(&mut self) -> PResult<Option<SecretString>> {
        match fs::read_to_string(&self.0) {
            Ok(text) if !text.is_empty() => Ok(Some(SecretString::from(text))),
            _ => Ok(None),
        }
    }

    fn clear(&mut self) -> PResult<()> {
        fs::write(&self.0, "").map_err(|_| PlatformError::Clipboard)
    }

    fn read_image(&mut self) -> PResult<ImageRead> {
        Ok(ImageRead::Empty)
    }
}

struct ScriptedTerminal(PathBuf);

impl ScriptedTerminal {
    fn next(&self) -> Result<String> {
        let text = fs::read_to_string(&self.0).map_err(|_| CliError::input_required())?;
        let (first, rest) = text.split_once('\n').unwrap_or((text.as_str(), ""));
        if text.is_empty() {
            return Err(CliError::input_required());
        }
        let first = first.to_owned();
        fs::write(&self.0, rest).map_err(|_| CliError::input_required())?;
        Ok(first)
    }
}

impl Terminal for ScriptedTerminal {
    fn secret(&mut self, _prompt: &str) -> Result<Zeroizing<String>> {
        self.next().map(Zeroizing::new)
    }

    fn line(&mut self, _prompt: &str) -> Result<String> {
        self.next()
    }
}

/// The file-based backends, if `MOSAIC_TEST_DIR` is set.
pub fn backends() -> Option<Backends> {
    let root = dir()?;
    let refuse = std::env::var("MOSAIC_TEST_KEYSTORE").is_ok_and(|v| v == "refuse");
    Some(Backends {
        keys: Box::new(FileKeys(root.clone())),
        sessions: Ok(Box::new(FileSessions {
            root: root.clone(),
            refuse,
        })),
        tokens: Box::new(FileTokens(root)),
    })
}

/// The file clipboard, if `MOSAIC_TEST_DIR` is set.
pub fn clipboard() -> Option<Box<dyn Clipboard>> {
    Some(Box::new(FileClipboard(dir()?.join("clipboard.txt"))))
}

/// The scripted terminal, if `MOSAIC_TEST_PROMPTS` is set.
pub fn terminal() -> Option<Box<dyn Terminal>> {
    std::env::var_os("MOSAIC_TEST_PROMPTS")
        .map(|p| Box::new(ScriptedTerminal(PathBuf::from(p))) as Box<dyn Terminal>)
}

/// The settable clock.
pub fn now_ms() -> Option<u64> {
    std::env::var("MOSAIC_TEST_NOW_MS").ok()?.parse().ok()
}
