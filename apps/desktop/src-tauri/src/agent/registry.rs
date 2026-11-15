//! The paired agent clients: `agent-pairings.json` in the app data directory
//! (`CRYPTO_SPEC.md` §15, ADR-0160).
//!
//! ```json
//! {"version":1,"pairings":[{"id":"<32 hex>","key":"<64 hex>","name":"Claude Code",
//!   "createdAt":1761480000000,"lastUsedAt":null}]}
//! ```
//!
//! The file is the desktop's copy of `K_pair`, as `browser-pairings.json` is for browsers; the
//! client's copy is in its OS keystore. A key alone opens nothing while the vault is locked, and
//! every use still needs the user's grant. The file is written atomically with mode `0600` on
//! Unix and built in zeroizing memory; keys are held as `PairingKey` (zeroized on drop, no
//! `Clone`, no `Debug` output). A file that cannot be read leaves an empty, read-only registry
//! and is never overwritten.

use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mosaic_core::browser_channel::{PAIRING_KEY_LEN, PairingKey, new_pairing_id};
use mosaic_native_messaging::protocol::to_zeroizing;
use mosaic_native_messaging::{Hex, HexKey};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use zeroize::Zeroizing;

use crate::browser::write_atomic;
use crate::contract::ids::UnixMillis;

/// The file's name in the app data directory.
pub const FILE_NAME: &str = "agent-pairings.json";

/// Agent pairings at most (spec §5.2).
pub const MAX_PAIRINGS: usize = mosaic_agent::wire::MAX_PAIRINGS;

/// `lastUsedAt` is written to disk at most this often per pairing.
pub const LAST_USED_SAVE_MS: UnixMillis = 10 * 60 * 1000;

const MAX_FILE_BYTES: u64 = 64 * 1024;

/// Why a change to the registry failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum RegistryError {
    /// [`MAX_PAIRINGS`] pairings exist.
    #[error("the agent pairing limit is reached")]
    Full,
    /// The OS random generator or the file failed.
    #[error("the agent pairings could not be saved")]
    Storage,
    /// The saved file exists but could not be read; it is left as it is.
    #[error("the saved agent pairings could not be read")]
    Unreadable,
}

/// One paired agent client.
pub struct Pairing {
    /// The pairing ID.
    pub id: [u8; 16],
    /// `K_pair`, shared with the connections that authenticate with it.
    pub key: Arc<PairingKey>,
    /// The name the client gave at pairing (ASCII, checked by the endpoint). Not verified.
    pub name: String,
    /// When it was paired.
    pub created_at: UnixMillis,
    /// When it last made a request.
    pub last_used_at: Option<UnixMillis>,
    saved_used_at: Option<UnixMillis>,
}

/// All pairings, and where they are saved (`None`: memory only).
pub struct AgentRegistry {
    path: Option<PathBuf>,
    pairings: Vec<Pairing>,
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
    name: String,
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
    name: &'a str,
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

impl AgentRegistry {
    /// A registry that is never saved.
    #[must_use]
    pub const fn memory() -> Self {
        Self {
            path: None,
            pairings: Vec::new(),
            unreadable: false,
        }
    }

    /// The registry saved in `dir`. A missing file is an empty registry; any other failure to
    /// read it gives an empty, read-only one, so a transient error cannot cost every pairing.
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        let path = dir.join(FILE_NAME);
        let (pairings, unreadable) = match fs::File::open(&path) {
            Ok(file) => parse_file(file).map_or((Vec::new(), true), |p| (p, false)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Vec::new(), false),
            Err(_) => (Vec::new(), true),
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

    /// Adds a pairing with a fresh ID and key and saves the file. Returns the ID and a second
    /// handle on the same key for `pairApproved`.
    ///
    /// # Errors
    /// [`RegistryError::Full`], [`RegistryError::Storage`], [`RegistryError::Unreadable`]
    /// (nothing is added then).
    pub fn add(
        &mut self,
        name: String,
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
            name,
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
    /// [`RegistryError::Storage`] (the pairing stays then), [`RegistryError::Unreadable`].
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

    /// Records a use. The file is rewritten at most every [`LAST_USED_SAVE_MS`] per pairing.
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
                    name: &p.name,
                    created_at: p.created_at,
                    last_used_at: p.last_used_at,
                })
                .collect(),
        };
        let bytes = to_zeroizing(&file).map_err(|_| RegistryError::Storage)?;
        write_atomic(path, &bytes).map_err(|_| RegistryError::Storage)
    }
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
        if out.iter().any(|p| p.id == entry.id.0)
            || !mosaic_native_messaging::agent_preauth::pairing_name_is_valid(&entry.name)
        {
            return None;
        }
        out.push(Pairing {
            id: entry.id.0,
            key: Arc::new(entry.key.0),
            name: entry.name,
            created_at: entry.created_at,
            last_used_at: entry.last_used_at,
            saved_used_at: entry.last_used_at,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registry_round_trips_through_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = AgentRegistry::in_dir(dir.path());
        assert!(registry.list().is_empty());
        let (id, key) = registry.add("Claude Code".into(), 1_000).unwrap();
        assert_eq!(
            key.expose_secret(),
            registry.get(&id).unwrap().key.expose_secret()
        );
        registry.note_used(&id, 2_000);
        let again = AgentRegistry::in_dir(dir.path());
        let pairing = again.get(&id).unwrap();
        assert_eq!(pairing.name, "Claude Code");
        assert_eq!(pairing.key.expose_secret(), key.expose_secret());
        assert!(registry.remove(&id).unwrap());
        assert!(AgentRegistry::in_dir(dir.path()).list().is_empty());
    }

    #[test]
    fn an_unreadable_file_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, b"{\"version\":2,\"pairings\":[]}").unwrap();
        let mut registry = AgentRegistry::in_dir(dir.path());
        assert!(registry.is_unreadable());
        assert_eq!(
            registry.add("Cursor".into(), 1).err(),
            Some(RegistryError::Unreadable)
        );
        assert_eq!(fs::read(&path).unwrap(), b"{\"version\":2,\"pairings\":[]}");
    }

    #[test]
    fn the_limit_holds() {
        let mut registry = AgentRegistry::memory();
        for n in 0..MAX_PAIRINGS {
            registry.add(format!("Client {n}"), 1).unwrap();
        }
        assert_eq!(
            registry.add("One more".into(), 1).err(),
            Some(RegistryError::Full)
        );
    }
}
