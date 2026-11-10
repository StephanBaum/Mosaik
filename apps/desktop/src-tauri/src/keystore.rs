//! Where the Secret Key lives on this device (CORE_API: "Keep the
//! `SecretKey` in the OS keystore; the store never holds it"; CRYPTO_SPEC
//! §5.2).
//!
//! The stores themselves (the OS keystore through `keyring`, the entry names,
//! the memory stores) live in `mosaic-platform`, shared with the CLI; this
//! module adapts them to the IPC error type and adds the app's fallback:
//!
//! - [`OsKeystore`]: the OS credential store, one entry per account (see
//!   `mosaic_platform::keystore` for the entry names and what each OS uses).
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
//! Residual exposure: see `mosaic_platform::keystore`.

use std::sync::Mutex;

use mosaic_core::{AccountId, SecretKey};
use mosaic_platform::PlatformError;
use mosaic_platform::keystore as platform;
use mosaic_sync::http::StoredToken;

use crate::contract::errors::{IpcError, IpcResult};

pub(crate) use platform::copy_token;

impl From<PlatformError> for IpcError {
    fn from(error: PlatformError) -> Self {
        match error {
            PlatformError::Core(core) => Self::from(core),
            _ => Self::INTERNAL,
        }
    }
}

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
pub struct MemoryKeystore(platform::MemoryKeystore);

impl SecretKeyStore for MemoryKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        Ok(platform::SecretKeyStore::load(&self.0, account)?)
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        Ok(platform::SecretKeyStore::save(&self.0, account, key)?)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        Ok(platform::SecretKeyStore::delete(&self.0, account)?)
    }
}

/// The OS keystore (see `mosaic_platform::keystore` for what each OS uses).
pub struct OsKeystore(platform::OsKeystore);

impl OsKeystore {
    /// Entries under `service` (the app identifier).
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self(platform::OsKeystore::new(service))
    }

    /// The pending Secret Keys of a rotation (M7) next to the keys of `service`.
    #[must_use]
    pub fn pending(service: &str) -> Self {
        Self::new(&platform::pending_service(service))
    }
}

impl SecretKeyStore for OsKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        Ok(platform::SecretKeyStore::load(&self.0, account)?)
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        Ok(platform::SecretKeyStore::save(&self.0, account, key)?)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        Ok(platform::SecretKeyStore::delete(&self.0, account)?)
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

/// Keeps tokens in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemorySyncTokens(platform::MemorySyncTokens);

impl SyncTokenStore for MemorySyncTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        Ok(platform::SyncTokenStore::load(&self.0, account)?)
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        Ok(platform::SyncTokenStore::save(&self.0, account, token)?)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        Ok(platform::SyncTokenStore::delete(&self.0, account)?)
    }
}

/// The OS credential store: entry `mosaic-sync-token-<account id>` under the
/// app's service name.
pub struct OsSyncTokens(platform::OsSyncTokens);

impl OsSyncTokens {
    /// Entries under `service` (the app identifier).
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self(platform::OsSyncTokens::new(service))
    }
}

impl SyncTokenStore for OsSyncTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        Ok(platform::SyncTokenStore::load(&self.0, account)?)
    }

    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        Ok(platform::SyncTokenStore::save(&self.0, account, token)?)
    }

    fn delete(&self, account: AccountId) -> IpcResult<()> {
        Ok(platform::SyncTokenStore::delete(&self.0, account)?)
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
