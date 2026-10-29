//! Quick unlock (CRYPTO_SPEC §6.2): a wrapping of AK under a key `W` that
//! only a user-verified platform operation can produce.
//!
//! Two halves:
//!
//! - [`QuickUnlockPlatform`]: the OS side. It creates a hardware-bound,
//!   non-exportable key and, after user verification, produces the secret
//!   input `W` is derived from. Implemented here for **Windows Hello**
//!   ([`windows_hello`], the §6.2 Windows row: a `KeyCredentialManager` RSA
//!   key signs a stored random challenge; RSA PKCS#1 v1.5 is
//!   deterministic, so enrollment signs twice and fails if the signatures
//!   differ). macOS (Secure Enclave key with a biometry access control) is
//!   not built: it needs `SecKeyCreateEncryptedData` and access-control
//!   objects that only `unsafe` Security-framework bindings reach. Linux
//!   has no quick unlock by spec (only FIDO2, §6.3).
//! - [`QuickUnlockVault`]: the vault side, which turns that secret into
//!   `W` (`HKDF(ikm = Sign(challenge), salt = challenge,
//!   info = ctx("hello-unlock"))`), stores the `method = 0x02` wrapping of
//!   AK and unlocks with it. It needs AK, which only `mosaic-vault`'s
//!   session holds, and the frozen M2 API has no method for it yet
//!   (CORE_API "Expected additions": quick unlock on `LockedSession`). The
//!   derivation belongs in `mosaic-core` next to the other key flows. Until
//!   both land, [`NoVaultSupport`] is the only implementation, so no method
//!   is offered and `unlock_quick` answers `quickUnlockUnavailable`.
//!
//! Proposed additions (additive, pre-approved in `CORE_API`): in
//! `mosaic-core`, `quick_unlock_key(platform_secret, challenge, platform)`
//! for §6.2; in `mosaic-vault`, `UnlockedSession::enroll_quick_unlock(&W)
//! -> Result<()>`, `UnlockedSession::remove_quick_unlock()` and
//! `LockedSession::unlock_quick(&W) -> Result<UnlockedSession,
//! UnlockError>`, with `method_id` = device ID and the §6.2 expiry
//! (default 14 days) enforced by deleting the wrapping and the platform key.

use mosaic_core::{AccountId, SecretVec};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::session::QuickUnlockKind;

/// Public data of an enrollment, kept next to the wrapping (not secret).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Enrollment {
    /// The platform key's name in the OS store.
    pub key_name: String,
    /// The random challenge the platform key signs (Windows Hello).
    pub challenge: Vec<u8>,
}

/// The OS half of quick unlock.
pub trait QuickUnlockPlatform: Send + Sync {
    /// Which method this is.
    fn kind(&self) -> QuickUnlockKind;

    /// Whether the OS offers it right now (hardware present, set up).
    fn is_available(&self) -> bool;

    /// Creates the platform key for `account` (replacing an old one) and
    /// returns the enrollment and the user-verified secret.
    ///
    /// # Errors
    /// `quickUnlockUnavailable` if the OS refuses or the user cancels.
    fn enroll(&self, account: AccountId) -> IpcResult<(Enrollment, SecretVec)>;

    /// Asks the user to verify and returns the secret of `enrollment`.
    ///
    /// # Errors
    /// `quickUnlockUnavailable` if the key is gone or the user cancels.
    fn release(&self, enrollment: &Enrollment) -> IpcResult<SecretVec>;

    /// Deletes the platform key (expiry, removal, full re-key §6.6).
    ///
    /// # Errors
    /// `internal` if the OS refuses.
    fn remove(&self, enrollment: &Enrollment) -> IpcResult<()>;
}

/// The vault half of quick unlock (see the module documentation).
pub trait QuickUnlockVault: Send + Sync {
    /// Whether the vault can store and use a quick-unlock wrapping.
    fn supported(&self) -> bool;
}

/// The vault half until `mosaic-vault` has the session methods.
pub struct NoVaultSupport;

impl QuickUnlockVault for NoVaultSupport {
    fn supported(&self) -> bool {
        false
    }
}

/// Quick unlock as the backend sees it.
pub struct QuickUnlock {
    platforms: Vec<Box<dyn QuickUnlockPlatform>>,
    vault: Box<dyn QuickUnlockVault>,
}

impl Default for QuickUnlock {
    fn default() -> Self {
        Self {
            platforms: Vec::new(),
            vault: Box::new(NoVaultSupport),
        }
    }
}

impl QuickUnlock {
    /// Quick unlock over these platform methods and vault support.
    #[must_use]
    pub fn new(
        platforms: Vec<Box<dyn QuickUnlockPlatform>>,
        vault: Box<dyn QuickUnlockVault>,
    ) -> Self {
        Self { platforms, vault }
    }

    /// The platform methods of this OS.
    #[must_use]
    pub fn for_this_os() -> Self {
        #[cfg(windows)]
        let platforms: Vec<Box<dyn QuickUnlockPlatform>> =
            vec![Box::new(windows_hello::WindowsHello)];
        #[cfg(not(windows))]
        let platforms: Vec<Box<dyn QuickUnlockPlatform>> = Vec::new();
        Self::new(platforms, Box::new(NoVaultSupport))
    }

    /// Methods the unlock screen may offer: available on this device and
    /// usable by the vault, in display order. Enrollment state comes with
    /// the vault support; without it the list is empty.
    #[must_use]
    pub fn offered(&self) -> Vec<QuickUnlockKind> {
        if !self.vault.supported() {
            return Vec::new();
        }
        self.platforms
            .iter()
            .filter(|p| p.is_available())
            .map(|p| p.kind())
            .collect()
    }

    /// The platform for `kind`, if the vault can use it.
    ///
    /// # Errors
    /// `quickUnlockUnavailable`.
    pub fn platform(&self, kind: QuickUnlockKind) -> IpcResult<&dyn QuickUnlockPlatform> {
        if !self.vault.supported() {
            return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
        }
        self.platforms
            .iter()
            .find(|p| p.kind() == kind && p.is_available())
            .map(AsRef::as_ref)
            .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))
    }
}

#[cfg(windows)]
pub mod windows_hello {
    //! Windows Hello through the `WinRT` `KeyCredentialManager` (safe
    //! projections of the `windows` crate; no `unsafe` here).
    //!
    //! The calls block until the user answers the Hello prompt, so they run
    //! on the blocking pool, never on the UI thread.

    use mosaic_core::{AccountId, SecretBytes, SecretVec};
    use subtle::ConstantTimeEq;
    use windows::Security::Credentials::{
        KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
    };
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::core::{Array, HSTRING};
    use zeroize::Zeroize as _;

    use super::{Enrollment, QuickUnlockPlatform};
    use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
    use crate::contract::session::QuickUnlockKind;

    /// Length of the random challenge (CRYPTO_SPEC §6.2: 32 bytes).
    pub const CHALLENGE_LEN: usize = 32;

    /// Windows Hello.
    pub struct WindowsHello;

    fn unavailable<E>(_: E) -> IpcError {
        IpcError::new(ErrorCode::QuickUnlockUnavailable)
    }

    /// The Hello key's name for an account.
    #[must_use]
    pub fn key_name(account: AccountId) -> String {
        format!("Mosaic quick unlock {account}")
    }

    /// Signs `challenge` with the named Hello key (one user prompt).
    fn sign(key_name: &str, challenge: &[u8]) -> IpcResult<SecretVec> {
        let opened = KeyCredentialManager::OpenAsync(&HSTRING::from(key_name))
            .and_then(|op| op.join())
            .map_err(unavailable)?;
        if opened.Status().map_err(unavailable)? != KeyCredentialStatus::Success {
            return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
        }
        let credential = opened.Credential().map_err(unavailable)?;
        let data = CryptographicBuffer::CreateFromByteArray(challenge).map_err(unavailable)?;
        let signed = credential
            .RequestSignAsync(&data)
            .and_then(|op| op.join())
            .map_err(unavailable)?;
        if signed.Status().map_err(unavailable)? != KeyCredentialStatus::Success {
            return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
        }
        let buffer = signed.Result().map_err(unavailable)?;
        let mut bytes = Array::<u8>::new();
        CryptographicBuffer::CopyToByteArray(&buffer, &mut bytes).map_err(unavailable)?;
        let signature = SecretVec::from_slice(&bytes).map_err(|_| IpcError::INTERNAL);
        // The WinRT array is freed by the OS allocator; clear it first.
        bytes.zeroize();
        signature
    }

    impl QuickUnlockPlatform for WindowsHello {
        fn kind(&self) -> QuickUnlockKind {
            QuickUnlockKind::Biometric
        }

        fn is_available(&self) -> bool {
            KeyCredentialManager::IsSupportedAsync()
                .and_then(|op| op.join())
                .unwrap_or(false)
        }

        fn enroll(&self, account: AccountId) -> IpcResult<(Enrollment, SecretVec)> {
            let key_name = key_name(account);
            let created = KeyCredentialManager::RequestCreateAsync(
                &HSTRING::from(key_name.as_str()),
                KeyCredentialCreationOption::ReplaceExisting,
            )
            .and_then(|op| op.join())
            .map_err(unavailable)?;
            if created.Status().map_err(unavailable)? != KeyCredentialStatus::Success {
                return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
            }
            let challenge =
                SecretBytes::<CHALLENGE_LEN>::random().map_err(|_| IpcError::INTERNAL)?;
            let challenge = challenge.expose_secret().to_vec();
            let first = sign(&key_name, &challenge)?;
            let second = sign(&key_name, &challenge)?;
            // §6.2: W must be reproducible; a non-deterministic signer
            // would lock the user out later.
            if !bool::from(first.expose_secret().ct_eq(second.expose_secret())) {
                let _ = KeyCredentialManager::DeleteAsync(&HSTRING::from(key_name.as_str()))
                    .and_then(|op| op.join());
                return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
            }
            Ok((
                Enrollment {
                    key_name,
                    challenge,
                },
                first,
            ))
        }

        fn release(&self, enrollment: &Enrollment) -> IpcResult<SecretVec> {
            sign(&enrollment.key_name, &enrollment.challenge)
        }

        fn remove(&self, enrollment: &Enrollment) -> IpcResult<()> {
            KeyCredentialManager::DeleteAsync(&HSTRING::from(enrollment.key_name.as_str()))
                .and_then(|op| op.join())
                .map_err(|_| IpcError::INTERNAL)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(bool);
    impl QuickUnlockPlatform for Fake {
        fn kind(&self) -> QuickUnlockKind {
            QuickUnlockKind::Biometric
        }
        fn is_available(&self) -> bool {
            self.0
        }
        fn enroll(&self, _: AccountId) -> IpcResult<(Enrollment, SecretVec)> {
            Err(IpcError::new(ErrorCode::QuickUnlockUnavailable))
        }
        fn release(&self, _: &Enrollment) -> IpcResult<SecretVec> {
            Err(IpcError::new(ErrorCode::QuickUnlockUnavailable))
        }
        fn remove(&self, _: &Enrollment) -> IpcResult<()> {
            Ok(())
        }
    }

    struct Supported;
    impl QuickUnlockVault for Supported {
        fn supported(&self) -> bool {
            true
        }
    }

    #[test]
    fn nothing_is_offered_without_vault_support() {
        let quick = QuickUnlock::new(vec![Box::new(Fake(true))], Box::new(NoVaultSupport));
        assert_eq!(quick.offered(), Vec::new());
        assert_eq!(
            quick
                .platform(QuickUnlockKind::Biometric)
                .err()
                .map(|e| e.code),
            Some(ErrorCode::QuickUnlockUnavailable)
        );
        assert_eq!(QuickUnlock::default().offered(), Vec::new());
        assert_eq!(QuickUnlock::for_this_os().offered(), Vec::new());
    }

    #[test]
    fn available_platforms_are_offered_with_vault_support() {
        let quick = QuickUnlock::new(
            vec![Box::new(Fake(true)), Box::new(Fake(false))],
            Box::new(Supported),
        );
        assert_eq!(quick.offered(), vec![QuickUnlockKind::Biometric]);
        assert!(quick.platform(QuickUnlockKind::Biometric).is_ok());
        assert!(quick.platform(QuickUnlockKind::HardwareKey).is_err());
        let off = QuickUnlock::new(vec![Box::new(Fake(false))], Box::new(Supported));
        assert_eq!(off.offered(), Vec::new());
        assert!(off.platform(QuickUnlockKind::Biometric).is_err());
    }
}
