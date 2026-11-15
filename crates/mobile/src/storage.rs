//! The platform's secure storage (CRYPTO_SPEC §6.9.1). Rust decides names and
//! lifetimes; the platform implements `get`, `put` and `delete` over the
//! Android Keystore or the iOS Keychain.

use mosaic_core::AccountId;
use mosaic_core::encoding::label;
use zeroize::Zeroizing;

use crate::error::{MobileError, Result};

/// `SecureStorage` failed (the platform could not read, write or delete).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("secure storage failed")]
pub struct StorageError;

/// Implemented by the platform. A value that is stored but cannot be read back
/// (damaged, wrong version, failing tag) is an error, never `Ok(None)`.
pub trait SecureStorage: Send + Sync {
    /// The value stored under `name`, or `None` if there is none.
    ///
    /// # Errors
    /// [`StorageError`] when the platform cannot read the value.
    fn get(&self, name: &str) -> core::result::Result<Option<Zeroizing<Vec<u8>>>, StorageError>;
    /// Whether a value is stored under `name`. The core asks this instead of
    /// [`Self::get`] whenever it only needs presence, so a secret is never read
    /// (and a decryption never runs) to answer "is it there". The default reads
    /// the value; the platforms override it with a cheap existence check.
    ///
    /// # Errors
    /// [`StorageError`] when the platform cannot tell.
    fn contains(&self, name: &str) -> core::result::Result<bool, StorageError> {
        Ok(self.get(name)?.is_some())
    }
    /// Stores `value` under `name`, replacing any earlier value.
    ///
    /// # Errors
    /// [`StorageError`] when the platform cannot write the value.
    fn put(&self, name: &str, value: &[u8]) -> core::result::Result<(), StorageError>;
    /// Deletes the value under `name` (not an error if there is none).
    ///
    /// # Errors
    /// [`StorageError`] when the platform cannot delete the value.
    fn delete(&self, name: &str) -> core::result::Result<(), StorageError>;
}

/// The reserved name of a future settings key hint (not used in M10).
pub const SETTINGS_KEY_HINT: &str = "settings-key-hint";

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// `secret-key/<account_id hex32>`: the 16-byte Secret Key, raw.
#[must_use]
pub fn secret_key_name(account: &AccountId) -> String {
    format!("secret-key/{}", hex(account.as_bytes()))
}

/// `sync-token/<account_id hex32>`: the 32-byte stored session token.
#[must_use]
pub fn sync_token_name(account: &AccountId) -> String {
    format!("sync-token/{}", hex(account.as_bytes()))
}

/// The version byte of an Android secure-store file; it is bound into the AAD
/// (the header-binding rule of CRYPTO_SPEC §3).
pub const SECURE_STORE_VERSION: u8 = 0x01;

/// The AAD of an Android secure-store file (CRYPTO_SPEC §6.9.1):
/// `0x01 || aad("mobile/secure-store", u32(len(name)) || name)`, that is
/// `0x01 || ctx(label) || 0x00 || len:u32 || name`.
#[must_use]
pub fn secure_store_aad(name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 32 + 1 + 4 + name.len());
    out.push(SECURE_STORE_VERSION);
    out.extend_from_slice(&label::MOBILE_SECURE_STORE.ctx());
    out.push(0x00);
    out.extend_from_slice(&u32::try_from(name.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(name.as_bytes());
    out
}

impl From<StorageError> for MobileError {
    fn from(_: StorageError) -> Self {
        Self::StorageUnavailable
    }
}

/// Reads a value, mapping a failure to `storageUnavailable`.
pub(crate) fn read(storage: &dyn SecureStorage, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
    Ok(storage.get(name)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_store_aad_binds_version_and_name() {
        let aad = secure_store_aad("secret-key/0123");
        // 0x01, the label context, 0x00, the name length, the name.
        assert_eq!(aad.first(), Some(&SECURE_STORE_VERSION));
        let ctx = label::MOBILE_SECURE_STORE.ctx();
        assert_eq!(aad.get(1..=ctx.len()), Some(ctx.as_slice()));
        let rest = &aad[1 + ctx.len()..];
        assert_eq!(rest.first(), Some(&0x00));
        assert_eq!(&rest[1..5], &15u32.to_be_bytes());
        assert_eq!(&rest[5..], b"secret-key/0123");
        // A file moved to another name, or a changed version, fails its tag.
        assert_ne!(aad, secure_store_aad("secret-key/0124"));
        assert_ne!(aad, secure_store_aad("secret-key/012"));
        assert_ne!(secure_store_aad("a"), secure_store_aad("a\0"));
        let mut other_version = aad.clone();
        other_version[0] = 0x02;
        assert_ne!(aad, other_version);
        // The names of CRYPTO_SPEC 6.9.1.
        let account = AccountId::from_bytes([0xab; 16]).unwrap();
        assert_eq!(
            secret_key_name(&account),
            format!("secret-key/{}", "ab".repeat(16))
        );
        assert_eq!(
            sync_token_name(&account),
            format!("sync-token/{}", "ab".repeat(16))
        );
    }
}
