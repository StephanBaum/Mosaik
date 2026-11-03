//! Touch ID through a Secure Enclave key (CRYPTO_SPEC §6.2 macOS row,
//! ADR-0028 item 4). macOS only; compiled and tested by the macOS CI job.
//!
//! A P-256 key is generated inside the Secure Enclave in the data-protection
//! keychain with access control `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
//! and `kSecAccessControlPrivateKeyUsage | kSecAccessControlBiometryCurrentSet`:
//! Touch ID only (no passcode fallback), and a change of enrolled fingers
//! invalidates the key. `W = random(32)` is encrypted to the key's public key
//! with `eciesEncryptionCofactorX963SHA256AESGCM`; the 113-byte ciphertext is
//! the wrapping's platform parameters. Opening it needs the private key, so
//! one Touch ID prompt per unlock.
//!
//! Everything goes through the safe API of `security-framework`; no `unsafe`
//! here. The key needs a Developer ID signed build with the
//! `application-identifier` and `keychain-access-groups` entitlements; an
//! unsigned build fails to create it (-34018) and reports Touch ID as
//! unavailable.

use mosaic_core::{Kek, SecretBytes};
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::item::{
    ItemClass, ItemSearchOptions, KeyClass, Location, Reference, SearchResult,
};
use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};
use security_framework_sys::access_control::{
    kSecAccessControlBiometryCurrentSet, kSecAccessControlPrivateKeyUsage,
};
use zeroize::Zeroize as _;

use super::{BiometricPlatform, PlatformError, params};
use crate::contract::quick_unlock::BiometricPlatformKind;

/// Touch ID.
pub struct TouchId;

/// `errSecItemNotFound`.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
/// `errSecUserCanceled` (`OSStatus` domain).
const ERR_SEC_USER_CANCELED: isize = -128;
/// `LAErrorUserCancel`, `LAErrorSystemCancel`, `LAErrorAppCancel`.
const LA_CANCEL_CODES: [isize; 3] = [-2, -4, -9];
const LA_DOMAIN: &str = "com.apple.LocalAuthentication";

const ALGORITHM: Algorithm = Algorithm::ECIESEncryptionCofactorX963SHA256AESGCM;

fn access_control() -> Result<SecAccessControl, PlatformError> {
    SecAccessControl::create_with_protection(
        Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
        kSecAccessControlPrivateKeyUsage | kSecAccessControlBiometryCurrentSet,
    )
    .map_err(|_| PlatformError::Unavailable)
}

/// The options of the quick-unlock key, without label and location (a
/// non-permanent probe key).
fn probe_options() -> Result<GenerateKeyOptions, PlatformError> {
    let mut options = GenerateKeyOptions::default();
    options
        .set_key_type(KeyType::ec_sec_prime_random())
        .set_size_in_bits(256)
        .set_token(Token::SecureEnclave)
        .set_access_control(access_control()?);
    Ok(options)
}

/// The private key with this label, if any (no prompt: only a reference).
fn find_private_key(label: &str) -> Result<Option<SecKey>, PlatformError> {
    let mut options = ItemSearchOptions::new();
    options
        .class(ItemClass::key())
        .key_class(KeyClass::private())
        .label(label)
        .ignore_legacy_keychains()
        .load_refs(true);
    match options.search() {
        Ok(results) => Ok(results.into_iter().find_map(|result| match result {
            SearchResult::Ref(Reference::Key(key)) => Some(key),
            _ => None,
        })),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
        Err(_) => Err(PlatformError::Failed),
    }
}

fn delete_keys(label: &str) -> Result<(), PlatformError> {
    let mut options = ItemSearchOptions::new();
    options
        .class(ItemClass::key())
        .label(label)
        .ignore_legacy_keychains();
    match options.delete() {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
        Err(_) => Err(PlatformError::Failed),
    }
}

/// Decrypts the stored ciphertext with the private key: one Touch ID prompt.
fn decrypt(key: &SecKey, ciphertext: &[u8]) -> Result<Kek, PlatformError> {
    let mut plain = key.decrypt_data(ALGORITHM, ciphertext).map_err(|error| {
        let code = error.code();
        let cancelled = if error.domain() == LA_DOMAIN {
            LA_CANCEL_CODES.contains(&code)
        } else {
            code == ERR_SEC_USER_CANCELED
        };
        if cancelled {
            PlatformError::Cancelled
        } else {
            PlatformError::Failed
        }
    })?;
    let secret = SecretBytes::<32>::from_slice(&plain).map_err(|_| PlatformError::Failed);
    plain.zeroize();
    secret.map(Kek::from_secret)
}

impl BiometricPlatform for TouchId {
    fn kind(&self) -> BiometricPlatformKind {
        BiometricPlatformKind::TouchId
    }

    fn is_available(&self) -> bool {
        probe_options().is_ok_and(|options| SecKey::new(&options).is_ok())
    }

    fn key_exists(&self, payload: &[u8]) -> bool {
        params::nonce_of(payload).is_some_and(|nonce| {
            find_private_key(&params::key_name(&nonce)).is_ok_and(|key| key.is_some())
        })
    }

    fn enroll(&self) -> Result<(Vec<u8>, Kek), PlatformError> {
        // A fresh random label per enrollment (never per account).
        let nonce = params::new_key_nonce().ok_or(PlatformError::Failed)?;
        let label = params::key_name(&nonce);
        let mut options = probe_options()?;
        options
            .set_label(label.clone())
            .set_location(Location::DataProtectionKeychain);
        // No Secure Enclave, no Touch ID or a missing entitlement (-34018).
        let private = SecKey::new(&options).map_err(|_| PlatformError::Unavailable)?;
        let result = (|| {
            let public = private.public_key().ok_or(PlatformError::Failed)?;
            let w = Kek::generate().map_err(|_| PlatformError::Failed)?;
            let ciphertext = public
                .encrypt_data(ALGORITHM, w.expose_secret())
                .map_err(|_| PlatformError::Failed)?;
            let payload = params::encode(BiometricPlatformKind::TouchId, &nonce, &ciphertext)
                .ok_or(PlatformError::Failed)?;
            // One prompt now proves that the stored ciphertext opens to W.
            let opened = decrypt(&private, &ciphertext)?;
            if opened != w {
                return Err(PlatformError::Unavailable);
            }
            Ok((payload, w))
        })();
        drop(private);
        if result.is_err() {
            let _ = delete_keys(&label);
        }
        result
    }

    fn release(&self, payload: &[u8]) -> Result<Kek, PlatformError> {
        let (nonce, ciphertext) = params::body_for(BiometricPlatformKind::TouchId, payload)
            .ok_or(PlatformError::Unavailable)?;
        let key = find_private_key(&params::key_name(&nonce))?.ok_or(PlatformError::Unavailable)?;
        decrypt(&key, ciphertext)
    }

    fn remove(&self, payload: &[u8]) -> Result<(), PlatformError> {
        let nonce = params::nonce_of(payload).ok_or(PlatformError::Unavailable)?;
        delete_keys(&params::key_name(&nonce))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_for_another_platform_is_unavailable() {
        let hello =
            params::encode(BiometricPlatformKind::WindowsHello, &[1; 16], &[0; 32]).unwrap();
        assert_eq!(
            TouchId.release(&hello).err(),
            Some(PlatformError::Unavailable)
        );
        assert_eq!(TouchId.release(&[]).err(), Some(PlatformError::Unavailable));
        assert!(!TouchId.key_exists(&[]));
    }
}
