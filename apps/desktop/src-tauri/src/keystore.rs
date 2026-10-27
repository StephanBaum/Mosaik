//! Where the Secret Key lives on this device (CORE_API: "Keep the
//! `SecretKey` in the OS keystore; the store never holds it").
//!
//! The OS keystore (Windows Credential Manager, macOS Keychain, Linux
//! Secret Service) is the unlock track's work, with its crate chosen in an
//! ADR. Until then the app keeps the key in memory for the life of the
//! process ([`MemoryKeystore`]): after a restart the unlock screen asks for
//! it from the Emergency Kit (`secretKeyStored: false`).

use std::sync::Mutex;

use mosaic_core::SecretKey;

use crate::contract::errors::{IpcError, IpcResult};

/// The Secret Key store of this install (one account per install).
pub trait SecretKeyStore: Send + Sync {
    /// A copy of the stored key, if any.
    ///
    /// # Errors
    /// `internal` if the keystore cannot be read.
    fn load(&self) -> IpcResult<Option<SecretKey>>;

    /// Stores the key, replacing any previous one.
    ///
    /// # Errors
    /// `internal` if the keystore cannot be written.
    fn save(&self, key: &SecretKey) -> IpcResult<()>;
}

/// Keeps the Secret Key in process memory only (zeroized on drop).
#[derive(Default)]
pub struct MemoryKeystore(Mutex<Option<SecretKey>>);

impl SecretKeyStore for MemoryKeystore {
    fn load(&self) -> IpcResult<Option<SecretKey>> {
        let guard = self.0.lock().map_err(|_| IpcError::INTERNAL)?;
        guard.as_ref().map(copy).transpose()
    }

    fn save(&self, key: &SecretKey) -> IpcResult<()> {
        let copied = copy(key)?;
        *self.0.lock().map_err(|_| IpcError::INTERNAL)? = Some(copied);
        Ok(())
    }
}

/// `SecretKey` is deliberately not `Clone`; copy through its display form
/// (both intermediate strings zeroize).
fn copy(key: &SecretKey) -> IpcResult<SecretKey> {
    let display = key.to_display().map_err(IpcError::from)?;
    SecretKey::parse(display.expose_secret()).map_err(IpcError::from)
}
