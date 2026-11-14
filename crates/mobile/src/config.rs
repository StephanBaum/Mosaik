//! How the platform opens the core (CRYPTO_SPEC §6.9.2, §6.9.3, §6.9.6).

use std::path::PathBuf;

/// The platform the core runs on. It decides the quick-unlock platform byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Android: Keystore quick unlock (platform byte `0x04`).
    Android,
    /// iOS: Secure Enclave quick unlock (platform byte `0x03`).
    Ios,
}

/// Who opens the core. The iOS Credential Provider extension never runs
/// Argon2id and never opens a network connection (CRYPTO_SPEC §6.9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreContext {
    /// The app process (on Android also the Autofill Service and the
    /// credential provider).
    App,
    /// The iOS Credential Provider extension.
    IosAutofillExtension,
}

/// Everything the core needs at open. Nothing here is secret.
#[derive(Clone, Debug)]
pub struct CoreConfig {
    /// The directory of `store.sqlite` and `settings.json`. Android:
    /// `noBackupFilesDir/mosaic`; iOS: the app group's
    /// `Library/Application Support/Mosaic`.
    pub store_dir: PathBuf,
    /// The platform.
    pub platform: Platform,
    /// The context.
    pub context: CoreContext,
    /// The boot ID, 1 to 32 bytes; empty when the platform could not read it
    /// (no boot marker can then match, so the password is required).
    pub boot_id: Vec<u8>,
    /// DER trust anchors (Android only; empty on iOS).
    pub trust_anchors_der: Vec<Vec<u8>>,
}

impl CoreConfig {
    /// The store file.
    pub(crate) fn store_path(&self) -> PathBuf {
        self.store_dir.join("store.sqlite")
    }

    /// The settings file next to the store.
    pub(crate) fn settings_path(&self) -> PathBuf {
        self.store_dir.join("settings.json")
    }
}
