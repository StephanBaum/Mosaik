//! Where the Secret Key lives on this device (CORE_API: "Keep the
//! `SecretKey` in the OS keystore; the store never holds it"; CRYPTO_SPEC
//! §5.2).
//!
//! - [`OsKeystore`]: the OS credential store through the `keyring` crate,
//!   one entry per account (service = the app identifier, user = the
//!   account ID), holding the key's `M1-…` display form:
//!   - Windows: Credential Manager (generic credential, DPAPI-protected per
//!     user);
//!   - macOS: the login Keychain (generic password);
//!   - Linux: the kernel keyring (`keyutils`), which does **not** survive a
//!     reboot or logout. Secret Service persistence needs its D-Bus
//!     transport encryption (DH + AES in `secret-service`) or a plain
//!     session and is left for an ADR (`docs/PROGRESS.md`). Without an
//!     entry, the unlock screen asks for the key from the Emergency Kit.
//! - [`MemoryKeystore`]: process memory only (zeroized on drop).
//! - [`LayeredKeystore`]: the OS store with a memory fallback. Writes go to
//!   the OS store first; only when that write fails is the key kept in the
//!   memory layer (for the life of the process, so the Emergency Kit can
//!   still be saved after a lock), and the next start asks for it. OS reads
//!   are not cached: a key read from the OS store is handed to the caller
//!   and dropped. `is_stored` answers "is there a key?" from a cached
//!   boolean (or one read whose key is zeroized at once). An OS keystore
//!   failure is never fatal.
//!
//! Residual exposure: `keyring` passes the display string to the OS API in
//! buffers Mosaic cannot zeroize, and the OS keeps the entry as long as the
//! account exists on this device (by design: that is the point of storing
//! it).
//!
//! - `keyring` passes the display string through OS buffers that cannot be
//!   zeroized.
//! - On Windows, any process of the same user can call `CredRead` and read
//!   the entry: the keystore protects against other users and offline
//!   disk access, not against malware running as the user.

use std::sync::Mutex;

use mosaic_core::{AccountId, SecretKey, SecretString};
use mosaic_sync::http::{StoredToken, Token as _};

use crate::contract::errors::{IpcError, IpcResult};

/// The Secret Key store of this install (one entry per account).
pub trait SecretKeyStore: Send + Sync {
    /// A copy of the stored key of `account`, if any.
    ///
    /// # Errors
    /// `internal` if the keystore cannot be read.
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>>;

    /// Stores the key of `account`, replacing any previous one.
    ///
    /// # Errors
    /// `internal` if the keystore cannot be written.
    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()>;

    /// Removes the key of `account` (no-op when absent).
    ///
    /// # Errors
    /// `internal` if the keystore cannot be written.
    fn delete(&self, account: AccountId) -> IpcResult<()>;

    /// Whether a key is stored for `account`. The default reads the key and
    /// drops it (zeroized); stores override it to avoid keeping a copy.
    fn is_stored(&self, account: AccountId) -> bool {
        self.load(account).ok().flatten().is_some()
    }
}

/// Keeps the Secret Key in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemoryKeystore(Mutex<Option<(AccountId, SecretKey)>>);

impl SecretKeyStore for MemoryKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        let guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        match guard.as_ref() {
            Some((stored, key)) if *stored == account => copy(key).map(Some),
            _ => Ok(None),
        }
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        let copied = copy(key)?;
        *self.0.lock().map_err(|_| IpcError::INTERNAL)? = Some((account, copied));
        Ok(())
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        let mut guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        if guard.as_ref().is_some_and(|(stored, _)| *stored == account) {
            *guard = None;
        }
        Ok(())
    }
}

/// `SecretKey` is deliberately not `Clone`; copy through its display form
/// (both intermediate strings zeroize).
fn copy(key: &SecretKey) -> IpcResult<SecretKey> {
    let display = key.to_display().map_err(IpcError::from)?;
    SecretKey::parse(display.expose_secret()).map_err(IpcError::from)
}

/// The OS keystore (see the module documentation for what each OS uses).
pub struct OsKeystore {
    service: String,
}

impl OsKeystore {
    /// Entries under `service` (the app identifier).
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    fn entry(&self, account: AccountId) -> IpcResult<keyring::Entry> {
        keyring::Entry::new(&self.service, &account.to_string()).map_err(|_| IpcError::INTERNAL)
    }
}

impl SecretKeyStore for OsKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        match self.entry(account)?.get_password() {
            Ok(display) => {
                let display = SecretString::from(display);
                // A damaged entry reads as absent: the unlock screen asks
                // for the key, and a typed key that unlocks replaces it.
                Ok(SecretKey::parse(display.expose_secret()).ok())
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(IpcError::INTERNAL),
        }
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        let display = key.to_display().map_err(IpcError::from)?;
        self.entry(account)?
            .set_password(display.expose_secret())
            .map_err(|_| IpcError::INTERNAL)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(IpcError::INTERNAL),
        }
    }
}

/// The OS store with a memory fallback (see the module documentation).
pub struct LayeredKeystore {
    /// Holds a key only when the persistent write failed.
    memory: MemoryKeystore,
    persistent: Box<dyn SecretKeyStore>,
    /// Whether the persistent store holds a key: `(account, present)`.
    present: Mutex<Option<(AccountId, bool)>>,
}

impl LayeredKeystore {
    /// The fallback memory layer in front of `persistent`.
    #[must_use]
    pub fn new(persistent: Box<dyn SecretKeyStore>) -> Self {
        Self {
            memory: MemoryKeystore::default(),
            persistent,
            present: Mutex::new(None),
        }
    }

    fn remember(&self, account: AccountId, present: bool) {
        if let Ok(mut cached) = self.present.lock() {
            *cached = Some((account, present));
        }
    }
}

impl SecretKeyStore for LayeredKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        if let Some(key) = self.memory.load(account)? {
            return Ok(Some(key));
        }
        // An unreadable OS keystore is the same as no entry: ask the user.
        // The key is returned, not cached.
        let found = self.persistent.load(account).ok().flatten();
        self.remember(account, found.is_some());
        Ok(found)
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        // Without the OS entry the next start asks for the key; until then
        // the memory layer keeps it so the kit can still be saved.
        if self.persistent.save(account, key).is_ok() {
            self.memory.delete(account)?;
        } else {
            self.memory.save(account, key)?;
        }
        self.remember(account, true);
        Ok(())
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        self.remember(account, false);
        self.memory.delete(account)?;
        self.persistent.delete(account)
    }

    fn is_stored(&self, account: AccountId) -> bool {
        if self.memory.is_stored(account) {
            return true;
        }
        if let Ok(cached) = self.present.lock()
            && let Some((cached_account, present)) = *cached
            && cached_account == account
        {
            return present;
        }
        // One read; the key is dropped (zeroized) at once, only the bool stays.
        let present = self.persistent.load(account).ok().flatten().is_some();
        self.remember(account, present);
        present
    }
}

// ---- the sync session token ---------------------------------------------------

/// Where the sync session token of this install lives (M6 spec §7): one
/// entry per account, 32 bytes as base64url, deleted on disconnect, on a 401
/// and on device removal. It is a bearer credential for the sync server and
/// worth nothing without the master password and the Secret Key (the server
/// holds ciphertext), so it gets the same home as the Secret Key and the
/// same fallback.
pub trait SyncTokenStore: Send + Sync {
    /// The stored token of `account`, if any.
    ///
    /// # Errors
    /// `internal` if the store cannot be read.
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>>;

    /// Stores the token of `account`, replacing any previous one.
    ///
    /// # Errors
    /// `internal` if the store cannot be written.
    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()>;

    /// Removes the token of `account` (no-op when absent).
    ///
    /// # Errors
    /// `internal` if the store cannot be written.
    fn delete(&self, account: AccountId) -> IpcResult<()>;
}

/// A copy of a token (it is deliberately not `Clone`).
fn copy_token(token: &StoredToken) -> StoredToken {
    let mut bytes = *token.expose_bytes();
    StoredToken::from_array(&mut bytes)
}

/// Keeps tokens in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemorySyncTokens(Mutex<std::collections::BTreeMap<AccountId, StoredToken>>);

impl SyncTokenStore for MemorySyncTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        let guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        Ok(guard.get(&account).map(copy_token))
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        let mut guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        guard.insert(account, copy_token(token));
        Ok(())
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        let mut guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        guard.remove(&account);
        Ok(())
    }
}

/// The OS credential store: entry `mosaic-sync-token-<account id>` under the
/// app's service name.
pub struct OsSyncTokens {
    service: String,
}

impl OsSyncTokens {
    /// Entries under `service` (the app identifier).
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    fn entry(&self, account: AccountId) -> IpcResult<keyring::Entry> {
        keyring::Entry::new(&self.service, &format!("mosaic-sync-token-{account}"))
            .map_err(|_| IpcError::INTERNAL)
    }
}

impl SyncTokenStore for OsSyncTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        use base64ct::{Base64UrlUnpadded, Encoding as _};
        match self.entry(account)?.get_password() {
            Ok(text) => {
                let text = zeroize::Zeroizing::new(text);
                let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
                // A damaged entry reads as absent: the user signs in again.
                Ok(
                    match Base64UrlUnpadded::decode(text.as_str(), bytes.as_mut_slice()) {
                        Ok(decoded) if decoded.len() == 32 => {
                            Some(StoredToken::from_array(&mut bytes))
                        }
                        _ => None,
                    },
                )
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(IpcError::INTERNAL),
        }
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        use base64ct::{Base64UrlUnpadded, Encoding as _};
        let mut buffer = zeroize::Zeroizing::new([0u8; 64]);
        let text = Base64UrlUnpadded::encode(token.expose_bytes(), buffer.as_mut_slice())
            .map_err(|_| IpcError::INTERNAL)?;
        self.entry(account)?
            .set_password(text)
            .map_err(|_| IpcError::INTERNAL)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(IpcError::INTERNAL),
        }
    }
}

/// The OS store with a memory fallback when it cannot be written (the token
/// then lasts for the life of the process and the next start signs in
/// again).
pub struct LayeredSyncTokens {
    memory: MemorySyncTokens,
    persistent: Box<dyn SyncTokenStore>,
}

impl LayeredSyncTokens {
    /// The memory layer in front of `persistent`.
    #[must_use]
    pub fn new(persistent: Box<dyn SyncTokenStore>) -> Self {
        Self {
            memory: MemorySyncTokens::default(),
            persistent,
        }
    }
}

impl SyncTokenStore for LayeredSyncTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        if let Some(token) = self.memory.load(account)? {
            return Ok(Some(token));
        }
        Ok(self.persistent.load(account).ok().flatten())
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        if self.persistent.save(account, token).is_ok() {
            self.memory.delete(account)
        } else {
            self.memory.save(account, token)
        }
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        self.memory.delete(account)?;
        self.persistent.delete(account)
    }
}

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
    fn memory_keystore_is_per_account() {
        let store = MemoryKeystore::default();
        let (a, b) = (AccountId::new_v7().unwrap(), AccountId::new_v7().unwrap());
        let k = key();
        assert!(store.load(a).unwrap().is_none());
        store.save(a, &k).unwrap();
        assert!(same(&store.load(a).unwrap().unwrap(), &k));
        assert!(store.load(b).unwrap().is_none(), "other account");
        store.delete(b).unwrap();
        assert!(store.load(a).unwrap().is_some(), "delete of b keeps a");
        store.delete(a).unwrap();
        assert!(store.load(a).unwrap().is_none());
    }

    /// A persistent store that fails every call.
    struct Failing;
    impl SecretKeyStore for Failing {
        fn load(&self, _: AccountId) -> IpcResult<Option<SecretKey>> {
            Err(IpcError::INTERNAL)
        }
        fn save(&self, _: AccountId, _: &SecretKey) -> IpcResult<()> {
            Err(IpcError::INTERNAL)
        }
        fn delete(&self, _: AccountId) -> IpcResult<()> {
            Err(IpcError::INTERNAL)
        }
    }

    #[test]
    fn layered_keystore_survives_a_failing_os_store() {
        let store = LayeredKeystore::new(Box::new(Failing));
        let a = AccountId::new_v7().unwrap();
        assert!(store.load(a).unwrap().is_none(), "unreadable means absent");
        let k = key();
        store.save(a, &k).unwrap();
        assert!(same(&store.load(a).unwrap().unwrap(), &k), "from memory");
        assert!(store.delete(a).is_err(), "the OS delete error is reported");
        assert!(store.load(a).unwrap().is_none(), "memory is cleared anyway");
    }

    #[test]
    #[ignore = "writes to the OS keystore"]
    fn os_keystore_round_trip() {
        let store = OsKeystore::new("io.github.stephanbaum.mosaic.test");
        let account = AccountId::new_v7().unwrap();
        let k = key();
        store.save(account, &k).unwrap();
        assert!(same(&store.load(account).unwrap().unwrap(), &k));
        store.delete(account).unwrap();
        assert!(store.load(account).unwrap().is_none());
    }

    /// A persistent store the test can also reach directly.
    struct Shared(std::sync::Arc<MemoryKeystore>);
    impl SecretKeyStore for Shared {
        fn load(&self, a: AccountId) -> IpcResult<Option<SecretKey>> {
            self.0.load(a)
        }
        fn save(&self, a: AccountId, k: &SecretKey) -> IpcResult<()> {
            self.0.save(a, k)
        }
        fn delete(&self, a: AccountId) -> IpcResult<()> {
            self.0.delete(a)
        }
    }

    #[test]
    fn layered_keystore_does_not_cache_os_reads() {
        let persistent = std::sync::Arc::new(MemoryKeystore::default());
        let layered = LayeredKeystore::new(Box::new(Shared(persistent.clone())));
        let a = AccountId::new_v7().unwrap();
        let k = key();
        layered.save(a, &k).unwrap();
        assert!(layered.memory.load(a).unwrap().is_none(), "not in memory");
        assert!(same(&layered.load(a).unwrap().unwrap(), &k));
        assert!(layered.memory.load(a).unwrap().is_none(), "read not cached");
        persistent.delete(a).unwrap();
        assert!(layered.load(a).unwrap().is_none(), "gone from the OS store");
    }

    #[test]
    fn layered_keystore_keeps_a_copy_only_when_the_os_write_fails() {
        let layered = LayeredKeystore::new(Box::new(Failing));
        let a = AccountId::new_v7().unwrap();
        let k = key();
        layered.save(a, &k).unwrap();
        assert!(same(&layered.load(a).unwrap().unwrap(), &k));
        assert!(layered.memory.load(a).unwrap().is_some());
    }

    #[test]
    fn is_stored_keeps_no_key() {
        let persistent = std::sync::Arc::new(MemoryKeystore::default());
        let layered = LayeredKeystore::new(Box::new(Shared(persistent.clone())));
        let a = AccountId::new_v7().unwrap();
        assert!(!layered.is_stored(a));
        persistent.save(a, &key()).unwrap();
        let fresh = LayeredKeystore::new(Box::new(Shared(persistent)));
        assert!(fresh.is_stored(a));
        assert!(fresh.memory.load(a).unwrap().is_none());
    }
}
