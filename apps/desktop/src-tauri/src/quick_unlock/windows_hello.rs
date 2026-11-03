//! Windows Hello through the `WinRT` `KeyCredentialManager` (safe
//! projections of the `windows` crate; no `unsafe` here). CRYPTO_SPEC §6.2
//! Windows row and ADR-0028 item 3.
//!
//! A Hello key signs a stored random challenge; RSA PKCS#1 v1.5 is
//! deterministic, so enrollment signs twice and refuses if the signatures
//! differ. `W = HKDF(ikm = signature, salt = challenge, info =
//! "hello-unlock")` is derived in `mosaic-core`. Enrollment also requires
//! `GetAttestationAsync` to report `Success` or `TemporaryFailure`: a key
//! without a TPM (a software key) is deleted and refused.
//!
//! The calls block until the user answers the Hello prompt, so they run on
//! the blocking pool, never on the UI thread. The Hello PIN always works as
//! a fallback and cannot be switched off (THREAT_MODEL R23).

use mosaic_core::unlock::{HELLO_CHALLENGE_LEN, hello_unlock_key};
use mosaic_core::{AccountId, Kek, SecretBytes, SecretVec};
use subtle::ConstantTimeEq;
use windows::Security::Credentials::{
    KeyCredentialAttestationStatus, KeyCredentialCreationOption, KeyCredentialManager,
    KeyCredentialStatus,
};
use windows::Security::Cryptography::CryptographicBuffer;
use windows::core::{Array, HSTRING};
use zeroize::Zeroize as _;

use super::{BiometricPlatform, PlatformError, params};
use crate::contract::quick_unlock::BiometricPlatformKind;

/// Windows Hello.
pub struct WindowsHello;

/// The Hello key's name for an account.
#[must_use]
pub fn key_name(account: AccountId) -> String {
    format!("Mosaic quick unlock {account}")
}

fn winrt<T>(result: windows::core::Result<T>) -> Result<T, PlatformError> {
    result.map_err(|_| PlatformError::Failed)
}

fn status_error(status: KeyCredentialStatus) -> PlatformError {
    if status == KeyCredentialStatus::UserCanceled {
        PlatformError::Cancelled
    } else if status == KeyCredentialStatus::NotFound {
        PlatformError::Unavailable
    } else {
        PlatformError::Failed
    }
}

fn delete_key(name: &str) {
    let _ = KeyCredentialManager::DeleteAsync(&HSTRING::from(name)).and_then(|op| op.join());
}

/// Signs `challenge` with the named Hello key (one user prompt).
fn sign(name: &str, challenge: &[u8]) -> Result<SecretVec, PlatformError> {
    let opened =
        winrt(KeyCredentialManager::OpenAsync(&HSTRING::from(name)).and_then(|op| op.join()))?;
    let status = winrt(opened.Status())?;
    if status != KeyCredentialStatus::Success {
        return Err(status_error(status));
    }
    let credential = winrt(opened.Credential())?;
    let data = winrt(CryptographicBuffer::CreateFromByteArray(challenge))?;
    let signed = winrt(credential.RequestSignAsync(&data).and_then(|op| op.join()))?;
    let status = winrt(signed.Status())?;
    if status != KeyCredentialStatus::Success {
        return Err(status_error(status));
    }
    let buffer = winrt(signed.Result())?;
    let mut bytes = Array::<u8>::new();
    winrt(CryptographicBuffer::CopyToByteArray(&buffer, &mut bytes))?;
    let signature = SecretVec::from_slice(&bytes).map_err(|_| PlatformError::Failed);
    // The WinRT array is freed by the OS allocator; clear it first.
    bytes.zeroize();
    signature
}

fn challenge_of(payload: &[u8]) -> Result<[u8; HELLO_CHALLENGE_LEN], PlatformError> {
    params::body_for(BiometricPlatformKind::WindowsHello, payload)
        .and_then(|body| <[u8; HELLO_CHALLENGE_LEN]>::try_from(body).ok())
        .ok_or(PlatformError::Unavailable)
}

/// Creates the key and checks that it is TPM-backed (ADR-0028 item 3).
fn create_attested_key(name: &str) -> Result<(), PlatformError> {
    let created = winrt(
        KeyCredentialManager::RequestCreateAsync(
            &HSTRING::from(name),
            KeyCredentialCreationOption::ReplaceExisting,
        )
        .and_then(|op| op.join()),
    )?;
    let status = winrt(created.Status())?;
    if status != KeyCredentialStatus::Success {
        return Err(status_error(status));
    }
    let credential = winrt(created.Credential())?;
    let attestation = credential
        .GetAttestationAsync()
        .and_then(|op| op.join())
        .and_then(|result| result.Status());
    match attestation {
        Ok(status)
            if status == KeyCredentialAttestationStatus::Success
                || status == KeyCredentialAttestationStatus::TemporaryFailure =>
        {
            Ok(())
        }
        _ => {
            // NotSupported or UnknownError: no TPM, a software key.
            delete_key(name);
            Err(PlatformError::Unavailable)
        }
    }
}

impl BiometricPlatform for WindowsHello {
    fn kind(&self) -> BiometricPlatformKind {
        BiometricPlatformKind::WindowsHello
    }

    fn is_available(&self) -> bool {
        KeyCredentialManager::IsSupportedAsync()
            .and_then(|op| op.join())
            .unwrap_or(false)
    }

    fn key_exists(&self, account: AccountId) -> bool {
        KeyCredentialManager::OpenAsync(&HSTRING::from(key_name(account)))
            .and_then(|op| op.join())
            .and_then(|opened| opened.Status())
            .is_ok_and(|status| status == KeyCredentialStatus::Success)
    }

    fn enroll(&self, account: AccountId) -> Result<(Vec<u8>, Kek), PlatformError> {
        let name = key_name(account);
        create_attested_key(&name)?;
        let result = (|| {
            let challenge =
                SecretBytes::<HELLO_CHALLENGE_LEN>::random().map_err(|_| PlatformError::Failed)?;
            let first = sign(&name, challenge.expose_secret())?;
            let second = sign(&name, challenge.expose_secret())?;
            // §6.2: W must be reproducible; a non-deterministic signer would
            // lock the user out later.
            if !bool::from(first.expose_secret().ct_eq(second.expose_secret())) {
                return Err(PlatformError::Unavailable);
            }
            let w = hello_unlock_key(&first, challenge.expose_secret())
                .map_err(|_| PlatformError::Failed)?;
            let payload = params::encode(
                BiometricPlatformKind::WindowsHello,
                challenge.expose_secret(),
            )
            .ok_or(PlatformError::Failed)?;
            Ok((payload, w))
        })();
        if result.is_err() {
            delete_key(&name);
        }
        result
    }

    fn release(&self, account: AccountId, payload: &[u8]) -> Result<Kek, PlatformError> {
        let challenge = challenge_of(payload)?;
        let signature = sign(&key_name(account), &challenge)?;
        hello_unlock_key(&signature, &challenge).map_err(|_| PlatformError::Failed)
    }

    fn remove(&self, account: AccountId) -> Result<(), PlatformError> {
        winrt(
            KeyCredentialManager::DeleteAsync(&HSTRING::from(key_name(account)))
                .and_then(|op| op.join()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_for_another_platform_is_unavailable() {
        let touch = params::encode(BiometricPlatformKind::TouchId, &[0; 113]).unwrap();
        assert_eq!(challenge_of(&touch), Err(PlatformError::Unavailable));
        assert_eq!(challenge_of(&[]), Err(PlatformError::Unavailable));
        let hello = params::encode(BiometricPlatformKind::WindowsHello, &[9; 32]).unwrap();
        assert_eq!(challenge_of(&hello), Ok([9; 32]));
    }

    #[test]
    fn the_key_name_is_per_account() {
        let a = AccountId::from_bytes([
            0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, 1, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .unwrap();
        assert_eq!(
            key_name(a),
            "Mosaic quick unlock 019a2b3c-4d5e-7001-8000-000000000001"
        );
    }

    #[test]
    fn an_unknown_key_does_not_exist_and_nothing_prompts() {
        // No Hello prompt can appear: opening a key that was never created
        // fails at once (also on a machine without Hello).
        let a = AccountId::from_bytes([
            0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, 0xEE, 0x80, 0, 0, 0, 0, 0, 0, 0xEE,
        ])
        .unwrap();
        assert!(!WindowsHello.key_exists(a));
    }
}
