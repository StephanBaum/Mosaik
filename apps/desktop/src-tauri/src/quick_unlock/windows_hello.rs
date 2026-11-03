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
//! The Hello key is named by a random per-enrollment nonce stored in the
//! wrapping's platform parameters ([`params`]), not by the account.
//! Enrollment signs with the credential object `RequestCreateAsync`
//! returned, never with one re-opened by name, so the key that was checked
//! for a TPM is the key that signs. At unlock the key is opened by name and
//! not attested again (that is a network call): a key swapped in under the
//! name signs differently, so `W` differs and the unwrap fails (it can deny
//! the unlock, not grant one). For an unpackaged Win32 app Windows may not
//! isolate Hello keys per app (THREAT_MODEL R23, tracked for M9).
//!
//! The calls block until the user answers the Hello prompt, so they run on
//! the blocking pool, never on the UI thread. The Hello PIN always works as
//! a fallback and cannot be switched off (THREAT_MODEL R23).

use mosaic_core::unlock::{HELLO_CHALLENGE_LEN, hello_unlock_key};
use mosaic_core::{Kek, SecretBytes, SecretVec};
use subtle::ConstantTimeEq;
use windows::Security::Credentials::{
    KeyCredential, KeyCredentialAttestationStatus, KeyCredentialCreationOption,
    KeyCredentialManager, KeyCredentialStatus,
};
use windows::Security::Cryptography::CryptographicBuffer;
use windows::core::{Array, HSTRING};
use zeroize::Zeroize as _;

use super::{BiometricPlatform, PlatformError, params};
use crate::contract::quick_unlock::BiometricPlatformKind;

/// Windows Hello.
pub struct WindowsHello;

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

/// Opens the named Hello key.
fn open_key(name: &str) -> Result<KeyCredential, PlatformError> {
    let opened =
        winrt(KeyCredentialManager::OpenAsync(&HSTRING::from(name)).and_then(|op| op.join()))?;
    let status = winrt(opened.Status())?;
    if status != KeyCredentialStatus::Success {
        return Err(status_error(status));
    }
    winrt(opened.Credential())
}

/// Signs `challenge` with `credential` (one user prompt).
fn sign(credential: &KeyCredential, challenge: &[u8]) -> Result<SecretVec, PlatformError> {
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

/// The key nonce and the challenge of a Windows Hello payload.
fn parts_of(
    payload: &[u8],
) -> Result<([u8; params::KEY_NONCE_LEN], [u8; HELLO_CHALLENGE_LEN]), PlatformError> {
    params::body_for(BiometricPlatformKind::WindowsHello, payload)
        .and_then(|(nonce, body)| {
            <[u8; HELLO_CHALLENGE_LEN]>::try_from(body)
                .ok()
                .map(|challenge| (nonce, challenge))
        })
        .ok_or(PlatformError::Unavailable)
}

/// Creates the key and checks that it is TPM-backed (ADR-0028 item 3).
/// Returns the credential `RequestCreateAsync` made, so the caller signs
/// with exactly the key that was checked.
fn create_attested_key(name: &str) -> Result<KeyCredential, PlatformError> {
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
            Ok(credential)
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

    fn key_exists(&self, payload: &[u8]) -> bool {
        parts_of(payload).is_ok_and(|(nonce, _)| {
            KeyCredentialManager::OpenAsync(&HSTRING::from(params::key_name(&nonce)))
                .and_then(|op| op.join())
                .and_then(|opened| opened.Status())
                .is_ok_and(|status| status == KeyCredentialStatus::Success)
        })
    }

    fn enroll(&self) -> Result<(Vec<u8>, Kek), PlatformError> {
        let nonce = params::new_key_nonce().ok_or(PlatformError::Failed)?;
        let name = params::key_name(&nonce);
        let credential = create_attested_key(&name)?;
        let result = (|| {
            let challenge =
                SecretBytes::<HELLO_CHALLENGE_LEN>::random().map_err(|_| PlatformError::Failed)?;
            // Both signatures come from the credential that was attested.
            let first = sign(&credential, challenge.expose_secret())?;
            let second = sign(&credential, challenge.expose_secret())?;
            // §6.2: W must be reproducible; a non-deterministic signer would
            // lock the user out later.
            if !bool::from(first.expose_secret().ct_eq(second.expose_secret())) {
                return Err(PlatformError::Unavailable);
            }
            let w = hello_unlock_key(&first, challenge.expose_secret())
                .map_err(|_| PlatformError::Failed)?;
            let payload = params::encode(
                BiometricPlatformKind::WindowsHello,
                &nonce,
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

    fn release(&self, payload: &[u8]) -> Result<Kek, PlatformError> {
        let (nonce, challenge) = parts_of(payload)?;
        let credential = open_key(&params::key_name(&nonce))?;
        let signature = sign(&credential, &challenge)?;
        hello_unlock_key(&signature, &challenge).map_err(|_| PlatformError::Failed)
    }

    fn remove(&self, payload: &[u8]) -> Result<(), PlatformError> {
        let (nonce, _) = parts_of(payload)?;
        winrt(
            KeyCredentialManager::DeleteAsync(&HSTRING::from(params::key_name(&nonce)))
                .and_then(|op| op.join()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_for_another_platform_is_unavailable() {
        let touch = params::encode(BiometricPlatformKind::TouchId, &[1; 16], &[0; 113]).unwrap();
        assert_eq!(parts_of(&touch), Err(PlatformError::Unavailable));
        assert_eq!(parts_of(&[]), Err(PlatformError::Unavailable));
        let hello =
            params::encode(BiometricPlatformKind::WindowsHello, &[1; 16], &[9; 32]).unwrap();
        assert_eq!(parts_of(&hello), Ok(([1; 16], [9; 32])));
        assert_eq!(WindowsHello.remove(&touch), Err(PlatformError::Unavailable));
        assert_eq!(
            WindowsHello.release(&touch).err(),
            Some(PlatformError::Unavailable)
        );
        assert!(!WindowsHello.key_exists(&touch));
    }

    #[test]
    fn an_unknown_key_does_not_exist_and_nothing_prompts() {
        // No Hello prompt can appear: opening a key that was never created
        // fails at once (also on a machine without Hello).
        let payload =
            params::encode(BiometricPlatformKind::WindowsHello, &[0xEE; 16], &[9; 32]).unwrap();
        assert!(!WindowsHello.key_exists(&payload));
    }
}
