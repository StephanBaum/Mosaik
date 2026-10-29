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
//! - [`LayeredKeystore`]: memory in front of the OS store. Reads try memory
//!   first, writes go to both, and an OS keystore failure is never fatal:
//!   the key stays usable for this process, and the next start asks for it.
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

/// Memory in front of a persistent store (see the module documentation).
pub struct LayeredKeystore {
    memory: MemoryKeystore,
    persistent: Box<dyn SecretKeyStore>,
}

impl LayeredKeystore {
    /// Memory in front of `persistent`.
    #[must_use]
    pub fn new(persistent: Box<dyn SecretKeyStore>) -> Self {
        Self {
            memory: MemoryKeystore::default(),
            persistent,
        }
    }
}

impl SecretKeyStore for LayeredKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        if let Some(key) = self.memory.load(account)? {
            return Ok(Some(key));
        }
        // An unreadable OS keystore is the same as no entry: ask the user.
        let Ok(Some(key)) = self.persistent.load(account) else {
            return Ok(None);
        };
        self.memory.save(account, &key)?;
        Ok(Some(key))
    }

    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        self.memory.save(account, key)?;
        // Best effort: without the OS entry the next start asks for the key.
        let _ = self.persistent.save(account, key);
        Ok(())
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

    #[test]
    fn layered_keystore_reads_through_and_caches() {
        let persistent = MemoryKeystore::default();
        let a = AccountId::new_v7().unwrap();
        let k = key();
        persistent.save(a, &k).unwrap();
        let store = LayeredKeystore::new(Box::new(persistent));
        assert!(same(&store.load(a).unwrap().unwrap(), &k));
        assert!(same(&store.memory.load(a).unwrap().unwrap(), &k), "cached");
    }
}
