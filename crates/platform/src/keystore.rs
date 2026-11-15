//! Where the secrets of this device live (CORE_API: "Keep the `SecretKey` in
//! the OS keystore; the store never holds it"; CRYPTO_SPEC §5.2, §6.8).
//!
//! One implementation and one set of entry names for the desktop app and the
//! CLI, so both find the same Secret Key. Entries are generic credentials
//! under [`SERVICE`]:
//!
//! | what | service | user |
//! |---|---|---|
//! | Secret Key (`M1-…`) | [`SERVICE`] | `<account id>` |
//! | pending Secret Key of a rotation (M7) | `<SERVICE>.pending-secret-key` | `<account id>` |
//! | desktop sync token (base64url, 32 bytes) | [`SERVICE`] | `mosaic-sync-token-<account id>` |
//! | CLI sync token | [`SERVICE`] | `mosaic-cli-sync-token-<account id>` |
//! | CLI session (`session_id[16] \|\| W_cli[32]`, base64url) | [`SERVICE`] | `mosaic-cli-session-<account id>` |
//! | agent pairing (`pairing_id[16] \|\| K_pair[32]`, base64url; CRYPTO_SPEC §15, ADR-0160) | [`SERVICE`] | `mosaic-agent-pairing-<name>` |
//!
//! What the OS does with an entry:
//! - Windows: Credential Manager (generic credential, DPAPI-protected per
//!   user);
//! - macOS: the login Keychain (generic password);
//! - Linux: the kernel keyring (`keyutils`), which does **not** survive a
//!   reboot or logout. Secret Service persistence needs its D-Bus transport
//!   encryption and is left for an ADR. Without an entry, the unlock screen
//!   asks for the key from the Emergency Kit.
//!
//! Residual exposure: `keyring` passes the strings to the OS API in buffers
//! Mosaic cannot zeroize, and on Windows any process of the same user can
//! read the entry (the keystore protects against other users and offline
//! disk access, not against malware running as the user).
//!
//! Every store here has an in-process twin (`Memory…`), zeroized on drop.

use std::collections::BTreeMap;
use std::sync::Mutex;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use mosaic_core::browser_channel::PairingKey;
use mosaic_core::{AccountId, SecretKey, SecretString};
use mosaic_sync::http::{StoredToken, Token as _};
use zeroize::Zeroizing;

use crate::{PlatformError, Result};

/// The keystore service name of the app: the Tauri identifier
/// (`apps/desktop/src-tauri/tauri.conf.json`; a test compares them).
pub const SERVICE: &str = "io.github.stephanbaum.mosaic";
/// Appended to the service name for the pending Secret Key of a rotation (M7).
pub const PENDING_SERVICE_SUFFIX: &str = ".pending-secret-key";
/// Entry-name prefix of the desktop's sync token.
pub const SYNC_TOKEN_PREFIX: &str = "mosaic-sync-token-";
/// Entry-name prefix of the CLI's sync token (its own device).
pub const CLI_SYNC_TOKEN_PREFIX: &str = "mosaic-cli-sync-token-";
/// Entry-name prefix of the CLI session cache (CRYPTO_SPEC §6.8).
pub const CLI_SESSION_PREFIX: &str = "mosaic-cli-session-";

/// Entry-name prefix of an agent pairing (`mosaic agent pair --name <name>`).
pub const AGENT_PAIRING_PREFIX: &str = "mosaic-agent-pairing-";

/// The service name of the pending Secret Key entries under `service`.
#[must_use]
pub fn pending_service(service: &str) -> String {
    format!("{service}{PENDING_SERVICE_SUFFIX}")
}

pub(crate) fn entry(service: &str, user: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(service, user).map_err(|_| PlatformError::Keystore)
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| PlatformError::Poisoned)
}

// ---- the Secret Key ------------------------------------------------------------

/// The Secret Key store of this install (one entry per account).
pub trait SecretKeyStore: Send + Sync {
    /// A copy of the stored key of `account`, if any.
    ///
    /// # Errors
    /// When the keystore cannot be read.
    fn load(&self, account: AccountId) -> Result<Option<SecretKey>>;

    /// Stores the key of `account`, replacing any previous one.
    ///
    /// # Errors
    /// When the keystore cannot be written.
    fn save(&self, account: AccountId, key: &SecretKey) -> Result<()>;

    /// Removes the key of `account` (no-op when absent).
    ///
    /// # Errors
    /// When the keystore cannot be written.
    fn delete(&self, account: AccountId) -> Result<()>;

    /// Whether a key is stored for `account`. The default reads the key and
    /// drops it (zeroized); stores override it to avoid keeping a copy.
    fn is_stored(&self, account: AccountId) -> bool {
        self.load(account).ok().flatten().is_some()
    }
}

/// `SecretKey` is deliberately not `Clone`; copy through its display form
/// (both intermediate strings zeroize).
fn copy_key(key: &SecretKey) -> Result<SecretKey> {
    let display = key.to_display()?;
    Ok(SecretKey::parse(display.expose_secret())?)
}

/// Keeps the Secret Key in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemoryKeystore(Mutex<Option<(AccountId, SecretKey)>>);

impl SecretKeyStore for MemoryKeystore {
    fn load(&self, account: AccountId) -> Result<Option<SecretKey>> {
        let guard = lock(&self.0)?;
        match guard.as_ref() {
            Some((stored, key)) if *stored == account => copy_key(key).map(Some),
            _ => Ok(None),
        }
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> Result<()> {
        let copied = copy_key(key)?;
        *lock(&self.0)? = Some((account, copied));
        Ok(())
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        let mut guard = lock(&self.0)?;
        if guard.as_ref().is_some_and(|(stored, _)| *stored == account) {
            *guard = None;
        }
        Ok(())
    }
}

/// The OS keystore: one entry per account under `service`, holding the key's
/// `M1-…` display form.
pub struct OsKeystore {
    service: String,
}

impl OsKeystore {
    /// Entries under `service`.
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    /// The Secret Keys of the app ([`SERVICE`]).
    #[must_use]
    pub fn for_app() -> Self {
        Self::new(SERVICE)
    }

    fn entry(&self, account: AccountId) -> Result<keyring::Entry> {
        entry(&self.service, &account.to_string())
    }
}

impl SecretKeyStore for OsKeystore {
    fn load(&self, account: AccountId) -> Result<Option<SecretKey>> {
        match self.entry(account)?.get_password() {
            Ok(display) => {
                let display = SecretString::from(display);
                // A damaged entry reads as absent: the unlock screen asks
                // for the key, and a typed key that unlocks replaces it.
                Ok(SecretKey::parse(display.expose_secret()).ok())
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(PlatformError::Keystore),
        }
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> Result<()> {
        let display = key.to_display()?;
        self.entry(account)?
            .set_password(display.expose_secret())
            .map_err(|_| PlatformError::Keystore)
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        delete_entry(&self.entry(account)?)
    }
}

pub(crate) fn delete_entry(entry: &keyring::Entry) -> Result<()> {
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(PlatformError::Keystore),
    }
}

/// The Secret Key of a rotation in progress (M7): its own entries, so a crash
/// between the kit and the commit cannot touch the key in use.
pub struct PendingSecretKeyStore(OsKeystore);

impl PendingSecretKeyStore {
    /// The pending entries of the app (`<SERVICE>.pending-secret-key`).
    #[must_use]
    pub fn new() -> Self {
        Self::for_service(SERVICE)
    }

    /// The pending entries next to the Secret Keys of `service`.
    #[must_use]
    pub fn for_service(service: &str) -> Self {
        Self(OsKeystore::new(&pending_service(service)))
    }
}

impl Default for PendingSecretKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretKeyStore for PendingSecretKeyStore {
    fn load(&self, account: AccountId) -> Result<Option<SecretKey>> {
        self.0.load(account)
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> Result<()> {
        self.0.save(account, key)
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        self.0.delete(account)
    }
}

// ---- the sync session token ------------------------------------------------------

/// Where the sync session token of this install lives (M6 spec §7): one
/// entry per account, 32 bytes as base64url, deleted on disconnect, on a 401
/// and on device removal. It is a bearer credential for the sync server and
/// worth nothing without the master password and the Secret Key (the server
/// holds ciphertext).
pub trait SyncTokenStore: Send + Sync {
    /// The stored token of `account`, if any.
    ///
    /// # Errors
    /// When the store cannot be read.
    fn load(&self, account: AccountId) -> Result<Option<StoredToken>>;

    /// Stores the token of `account`, replacing any previous one.
    ///
    /// # Errors
    /// When the store cannot be written.
    fn save(&self, account: AccountId, token: &StoredToken) -> Result<()>;

    /// Removes the token of `account` (no-op when absent).
    ///
    /// # Errors
    /// When the store cannot be written.
    fn delete(&self, account: AccountId) -> Result<()>;
}

/// A copy of a token (it is deliberately not `Clone`).
#[must_use]
pub fn copy_token(token: &StoredToken) -> StoredToken {
    let mut bytes = *token.expose_bytes();
    StoredToken::from_array(&mut bytes)
}

/// Keeps tokens in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemorySyncTokens(Mutex<BTreeMap<AccountId, StoredToken>>);

impl SyncTokenStore for MemorySyncTokens {
    fn load(&self, account: AccountId) -> Result<Option<StoredToken>> {
        Ok(lock(&self.0)?.get(&account).map(copy_token))
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> Result<()> {
        lock(&self.0)?.insert(account, copy_token(token));
        Ok(())
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        lock(&self.0)?.remove(&account);
        Ok(())
    }
}

/// The OS credential store: entry `<prefix><account id>` under `service`.
pub struct OsSyncTokens {
    service: String,
    prefix: &'static str,
}

impl OsSyncTokens {
    /// The desktop's tokens (`mosaic-sync-token-<account id>`) under `service`.
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
            prefix: SYNC_TOKEN_PREFIX,
        }
    }

    /// The CLI's tokens (`mosaic-cli-sync-token-<account id>`) under [`SERVICE`].
    #[must_use]
    pub fn for_cli() -> Self {
        Self {
            service: SERVICE.to_owned(),
            prefix: CLI_SYNC_TOKEN_PREFIX,
        }
    }

    fn entry(&self, account: AccountId) -> Result<keyring::Entry> {
        entry(&self.service, &format!("{}{account}", self.prefix))
    }
}

impl SyncTokenStore for OsSyncTokens {
    fn load(&self, account: AccountId) -> Result<Option<StoredToken>> {
        match self.entry(account)?.get_password() {
            Ok(text) => Ok(decode_token(&Zeroizing::new(text))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(PlatformError::Keystore),
        }
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> Result<()> {
        let mut buffer = Zeroizing::new([0u8; 64]);
        let text = Base64UrlUnpadded::encode(token.expose_bytes(), buffer.as_mut_slice())
            .map_err(|_| PlatformError::Keystore)?;
        self.entry(account)?
            .set_password(text)
            .map_err(|_| PlatformError::Keystore)
    }

    fn delete(&self, account: AccountId) -> Result<()> {
        delete_entry(&self.entry(account)?)
    }
}

/// A damaged entry reads as absent: the user signs in again.
fn decode_token(text: &str) -> Option<StoredToken> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    match Base64UrlUnpadded::decode(text, bytes.as_mut_slice()) {
        Ok(decoded) if decoded.len() == 32 => Some(StoredToken::from_array(&mut bytes)),
        _ => None,
    }
}

// ---- agent pairings --------------------------------------------------------------

/// Where the MCP relay keeps its pairings with the desktop (`mosaic agent pair`; CRYPTO_SPEC §15,
/// ADR-0160): one entry per client name, `pairing_id[16] || K_pair[32]` as base64url. There is no
/// file fallback in production: a pairing key is a credential for the agent endpoint and lives
/// only in the OS keystore. (On Linux the kernel keyring forgets it at reboot: pair again.)
pub trait AgentPairingStore: Send + Sync {
    /// The pairing stored under `name`, if any.
    ///
    /// # Errors
    /// When the store cannot be read.
    fn load(&self, name: &str) -> Result<Option<([u8; 16], PairingKey)>>;

    /// Stores the pairing under `name`, replacing any previous one.
    ///
    /// # Errors
    /// When the store cannot be written.
    fn save(&self, name: &str, id: &[u8; 16], key: &PairingKey) -> Result<()>;

    /// Removes the pairing under `name` (no-op when absent).
    ///
    /// # Errors
    /// When the store cannot be written.
    fn delete(&self, name: &str) -> Result<()>;
}

/// The stored form of a pairing: `id || key` as base64url (64 characters).
///
/// # Errors
/// [`PlatformError::Keystore`] if encoding fails (it cannot with these sizes).
pub fn encode_agent_pairing(id: &[u8; 16], key: &PairingKey) -> Result<Zeroizing<String>> {
    let mut raw = Zeroizing::new([0u8; 48]);
    raw[..16].copy_from_slice(id);
    raw[16..].copy_from_slice(key.expose_secret());
    let mut buffer = Zeroizing::new([0u8; 64]);
    let text = Base64UrlUnpadded::encode(raw.as_slice(), buffer.as_mut_slice())
        .map_err(|_| PlatformError::Keystore)?;
    Ok(Zeroizing::new(text.to_owned()))
}

/// Reads the stored form; a damaged entry reads as absent (the user pairs again).
#[must_use]
pub fn decode_agent_pairing(text: &str) -> Option<([u8; 16], PairingKey)> {
    let mut raw = Zeroizing::new([0u8; 48]);
    match Base64UrlUnpadded::decode(text, raw.as_mut_slice()) {
        Ok(decoded) if decoded.len() == 48 => {
            let mut id = [0u8; 16];
            id.copy_from_slice(&raw[..16]);
            let mut key = Zeroizing::new([0u8; 32]);
            key.copy_from_slice(&raw[16..]);
            Some((id, PairingKey::from_bytes(&mut key)))
        }
        _ => None,
    }
}

/// Keeps pairings in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemoryAgentPairings(Mutex<BTreeMap<String, Zeroizing<String>>>);

impl AgentPairingStore for MemoryAgentPairings {
    fn load(&self, name: &str) -> Result<Option<([u8; 16], PairingKey)>> {
        Ok(lock(&self.0)?
            .get(name)
            .and_then(|t| decode_agent_pairing(t)))
    }

    fn save(&self, name: &str, id: &[u8; 16], key: &PairingKey) -> Result<()> {
        lock(&self.0)?.insert(name.to_owned(), encode_agent_pairing(id, key)?);
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        lock(&self.0)?.remove(name);
        Ok(())
    }
}

/// The OS credential store: entry `mosaic-agent-pairing-<name>` under [`SERVICE`].
#[derive(Default)]
pub struct OsAgentPairings;

impl OsAgentPairings {
    fn entry(name: &str) -> Result<keyring::Entry> {
        entry(SERVICE, &format!("{AGENT_PAIRING_PREFIX}{name}"))
    }
}

impl AgentPairingStore for OsAgentPairings {
    fn load(&self, name: &str) -> Result<Option<([u8; 16], PairingKey)>> {
        match Self::entry(name)?.get_password() {
            Ok(text) => Ok(decode_agent_pairing(&Zeroizing::new(text))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(PlatformError::Keystore),
        }
    }

    fn save(&self, name: &str, id: &[u8; 16], key: &PairingKey) -> Result<()> {
        Self::entry(name)?
            .set_password(&encode_agent_pairing(id, key)?)
            .map_err(|_| PlatformError::Keystore)
    }

    fn delete(&self, name: &str) -> Result<()> {
        delete_entry(&Self::entry(name)?)
    }
}

pub use crate::session::{CliSessionStore, MemoryCliSessions, OsCliSessions};

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SecretKey {
        SecretKey::generate().unwrap()
    }

    fn same(a: &SecretKey, b: &SecretKey) -> bool {
        a.to_display().unwrap().expose_secret() == b.to_display().unwrap().expose_secret()
    }

    #[test]
    fn keystore_entry_names_stable() {
        // Existing installs keep their entries: these strings never change.
        assert_eq!(SERVICE, "io.github.stephanbaum.mosaic");
        assert_eq!(
            pending_service(SERVICE),
            "io.github.stephanbaum.mosaic.pending-secret-key"
        );
        assert_eq!(SYNC_TOKEN_PREFIX, "mosaic-sync-token-");
        assert_eq!(CLI_SYNC_TOKEN_PREFIX, "mosaic-cli-sync-token-");
        assert_eq!(CLI_SESSION_PREFIX, "mosaic-cli-session-");
        let account = AccountId::new_v7().unwrap();
        // The Secret Key's user name is the bare account ID.
        let (service, user) = (SERVICE, account.to_string());
        assert_eq!(user.len(), 36, "hyphenated UUID");
        assert_eq!(
            OsSyncTokens::new(service).prefix,
            "mosaic-sync-token-",
            "desktop"
        );
        assert_eq!(
            OsSyncTokens::for_cli().prefix,
            "mosaic-cli-sync-token-",
            "CLI"
        );
        assert_eq!(OsSyncTokens::for_cli().service, SERVICE);
        assert_eq!(
            PendingSecretKeyStore::new().0.service,
            pending_service(SERVICE)
        );
        assert_eq!(OsKeystore::for_app().service, SERVICE);
    }

    #[test]
    fn agent_pairings_round_trip_and_stay_per_name() {
        let store = MemoryAgentPairings::default();
        let id = [7u8; 16];
        let key = PairingKey::from_bytes(&mut [9u8; 32]);
        assert!(store.load("Claude Code").unwrap().is_none());
        store.save("Claude Code", &id, &key).unwrap();
        let (got_id, got_key) = store.load("Claude Code").unwrap().unwrap();
        assert_eq!(got_id, id);
        assert_eq!(got_key.expose_secret(), key.expose_secret());
        assert!(store.load("Cursor").unwrap().is_none(), "other name");
        store.delete("Cursor").unwrap();
        assert!(store.load("Claude Code").unwrap().is_some());
        store.delete("Claude Code").unwrap();
        assert!(store.load("Claude Code").unwrap().is_none());
        assert_eq!(AGENT_PAIRING_PREFIX, "mosaic-agent-pairing-");
    }

    #[test]
    fn a_damaged_agent_pairing_reads_as_absent() {
        let id = [1u8; 16];
        let key = PairingKey::from_bytes(&mut [2u8; 32]);
        let text = encode_agent_pairing(&id, &key).unwrap();
        assert_eq!(text.len(), 64);
        assert!(decode_agent_pairing(&text).is_some());
        for bad in ["", "AAAA", &text[..63], &format!("{}A", &*text), "!!!"] {
            assert!(decode_agent_pairing(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn service_matches_tauri_identifier() {
        let conf = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/desktop/src-tauri/tauri.conf.json"
        ))
        .unwrap();
        let needle = format!("\"identifier\": \"{SERVICE}\"");
        assert!(conf.contains(&needle), "tauri.conf.json identifier differs");
        assert_eq!(crate::paths::APP_ID, SERVICE);
    }

    #[test]
    fn memory_keystore_is_per_account() {
        let store = MemoryKeystore::default();
        let (a, b) = (AccountId::new_v7().unwrap(), AccountId::new_v7().unwrap());
        let k = key();
        assert!(store.load(a).unwrap().is_none());
        store.save(a, &k).unwrap();
        assert!(same(&store.load(a).unwrap().unwrap(), &k));
        assert!(store.is_stored(a));
        assert!(store.load(b).unwrap().is_none(), "other account");
        store.delete(b).unwrap();
        assert!(store.load(a).unwrap().is_some(), "delete of b keeps a");
        store.delete(a).unwrap();
        assert!(store.load(a).unwrap().is_none());
    }

    #[test]
    fn memory_sync_tokens_are_per_account_and_copies() {
        let store = MemorySyncTokens::default();
        let (a, b) = (AccountId::new_v7().unwrap(), AccountId::new_v7().unwrap());
        let mut bytes = [7u8; 32];
        let token = StoredToken::from_array(&mut bytes);
        assert_eq!(bytes, [0; 32], "the source array is wiped");
        store.save(a, &token).unwrap();
        assert_eq!(*store.load(a).unwrap().unwrap().expose_bytes(), [7u8; 32]);
        assert!(store.load(b).unwrap().is_none());
        store.delete(a).unwrap();
        assert!(store.load(a).unwrap().is_none());
    }

    #[test]
    fn token_encoding_round_trips_and_damage_reads_as_absent() {
        let mut bytes = [9u8; 32];
        let token = StoredToken::from_array(&mut bytes);
        let mut buffer = [0u8; 64];
        let text = Base64UrlUnpadded::encode(token.expose_bytes(), &mut buffer).unwrap();
        assert_eq!(text.len(), 43);
        assert_eq!(*decode_token(text).unwrap().expose_bytes(), [9u8; 32]);
        assert!(decode_token("short").is_none());
        assert!(decode_token(&"A".repeat(60)).is_none());
        assert!(decode_token("!!!").is_none());
    }

    #[test]
    #[ignore = "writes to the OS keystore"]
    fn os_keystore_round_trip() {
        let service = "io.github.stephanbaum.mosaic.test";
        let store = OsKeystore::new(service);
        let account = AccountId::new_v7().unwrap();
        let k = key();
        store.save(account, &k).unwrap();
        assert!(same(&store.load(account).unwrap().unwrap(), &k));
        store.delete(account).unwrap();
        assert!(store.load(account).unwrap().is_none());
    }
}
