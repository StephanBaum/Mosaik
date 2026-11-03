//! The security-key half of quick unlock (CRYPTO_SPEC §6.3, §7.4): the
//! platform trait the commands call, and the production default, which
//! reports hardware keys as unsupported until the FIDO2 transport lands
//! (ADR-0029, gated on the owner). Nothing here talks to a device.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mosaic_core::SecretString;
use mosaic_core::unlock::PrfOutput;

use crate::contract::errors::{ErrorCode, IpcError};

/// Why a security-key operation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareKeyError {
    /// This build cannot talk to security keys.
    Unsupported,
    /// No key is plugged in.
    NoDevice,
    /// The key was not touched in time (30 s).
    Timeout,
    /// The user or the app cancelled.
    Cancelled,
    /// The key needs its PIN and none was given.
    PinRequired,
    /// The PIN is wrong.
    PinInvalid,
    /// The PIN is blocked.
    PinBlocked,
    /// The key has no PIN set (and UV is required).
    NoPinSet,
    /// None of the candidate credentials is on the plugged-in key.
    NotRecognized,
    /// Anything else.
    Failed,
}

impl From<HardwareKeyError> for IpcError {
    fn from(error: HardwareKeyError) -> Self {
        Self::new(match error {
            HardwareKeyError::Unsupported => ErrorCode::HardwareKeyUnsupported,
            HardwareKeyError::NoDevice => ErrorCode::HardwareKeyNotFound,
            HardwareKeyError::Timeout => ErrorCode::HardwareKeyTimeout,
            HardwareKeyError::Cancelled => ErrorCode::Cancelled,
            HardwareKeyError::PinRequired => ErrorCode::HardwareKeyPinRequired,
            HardwareKeyError::PinInvalid => ErrorCode::HardwareKeyPinInvalid,
            HardwareKeyError::PinBlocked => ErrorCode::HardwareKeyPinBlocked,
            HardwareKeyError::NoPinSet => ErrorCode::HardwareKeyNoPin,
            HardwareKeyError::NotRecognized => ErrorCode::HardwareKeyNotRecognized,
            HardwareKeyError::Failed => ErrorCode::Internal,
        })
    }
}

/// A shared cancel flag, set by `hardware_key_cancel` and on lock, polled by
/// the device code while it waits for a touch.
#[derive(Clone, Debug, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// Asks the running operation to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Clears the flag before a new operation.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Whether a cancel was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A registered credential the plugged-in key may hold.
#[derive(Clone, Copy, Debug)]
pub struct HardwareCandidate<'a> {
    /// The credential's RP ID.
    pub rp_id: &'a str,
    /// The credential ID.
    pub credential_id: &'a [u8],
    /// The PRF salt registered with it (`prf_salt` or `prf_salt_hw`).
    pub prf_salt: &'a [u8; 32],
}

/// The security-key transport. Every call blocks until a touch, a timeout
/// or a cancel, so the commands call it on the blocking pool and never with
/// the session locked.
pub trait HardwareKeyPlatform: Send + Sync {
    /// `false`: every hardware-key command answers `hardwareKeyUnsupported`
    /// without touching a device.
    fn is_supported(&self) -> bool;

    /// `makeCredential` (`rk = false`, user verification, `hmac-secret`)
    /// followed by one `getAssertion` with user verification and the salt
    /// `prf_salt_input(prf_salt)`. Returns the credential ID and the PRF
    /// output.
    ///
    /// The transport **must check the UV flag** (bit 2) in the
    /// authenticator data of the `getAssertion` response (CRYPTO_SPEC §6.3)
    /// and fail with [`HardwareKeyError::Failed`] if it is not set: a PRF
    /// output obtained without user verification must never reach the
    /// vault.
    ///
    /// # Errors
    /// [`HardwareKeyError`].
    fn register(
        &self,
        rp_id: &str,
        prf_salt: &[u8; 32],
        pin: Option<&SecretString>,
        cancel: &CancelFlag,
    ) -> Result<(Vec<u8>, PrfOutput), HardwareKeyError>;

    /// A silent preflight (`up = false`) over `candidates` to find the
    /// credential the plugged-in key holds, then one `getAssertion` with
    /// user verification and that candidate's salt. Returns the candidate's
    /// index and the PRF output. The UV-flag check of
    /// [`HardwareKeyPlatform::register`] applies here too.
    ///
    /// # Errors
    /// [`HardwareKeyError`].
    fn evaluate(
        &self,
        candidates: &[HardwareCandidate<'_>],
        pin: Option<&SecretString>,
        cancel: &CancelFlag,
    ) -> Result<(usize, PrfOutput), HardwareKeyError>;
}

/// The production platform until the FIDO2 transport exists: unsupported on
/// every OS, so the UI keeps every hardware-key control hidden.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHardwareKeys;

impl HardwareKeyPlatform for NoHardwareKeys {
    fn is_supported(&self) -> bool {
        false
    }

    fn register(
        &self,
        _rp_id: &str,
        _prf_salt: &[u8; 32],
        _pin: Option<&SecretString>,
        _cancel: &CancelFlag,
    ) -> Result<(Vec<u8>, PrfOutput), HardwareKeyError> {
        Err(HardwareKeyError::Unsupported)
    }

    fn evaluate(
        &self,
        _candidates: &[HardwareCandidate<'_>],
        _pin: Option<&SecretString>,
        _cancel: &CancelFlag,
    ) -> Result<(usize, PrfOutput), HardwareKeyError> {
        Err(HardwareKeyError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_map_to_their_codes() {
        let cases = [
            (
                HardwareKeyError::Unsupported,
                ErrorCode::HardwareKeyUnsupported,
            ),
            (HardwareKeyError::NoDevice, ErrorCode::HardwareKeyNotFound),
            (HardwareKeyError::Timeout, ErrorCode::HardwareKeyTimeout),
            (HardwareKeyError::Cancelled, ErrorCode::Cancelled),
            (
                HardwareKeyError::PinRequired,
                ErrorCode::HardwareKeyPinRequired,
            ),
            (
                HardwareKeyError::PinInvalid,
                ErrorCode::HardwareKeyPinInvalid,
            ),
            (
                HardwareKeyError::PinBlocked,
                ErrorCode::HardwareKeyPinBlocked,
            ),
            (HardwareKeyError::NoPinSet, ErrorCode::HardwareKeyNoPin),
            (
                HardwareKeyError::NotRecognized,
                ErrorCode::HardwareKeyNotRecognized,
            ),
            (HardwareKeyError::Failed, ErrorCode::Internal),
        ];
        for (error, code) in cases {
            assert_eq!(IpcError::from(error).code, code, "{error:?}");
        }
    }

    #[test]
    fn the_default_platform_is_unsupported_and_cancel_is_shared() {
        let platform = NoHardwareKeys;
        assert!(!platform.is_supported());
        let cancel = CancelFlag::default();
        let alias = cancel.clone();
        assert!(!alias.is_cancelled());
        cancel.cancel();
        assert!(alias.is_cancelled());
        alias.reset();
        assert!(!cancel.is_cancelled());
        assert_eq!(
            platform
                .register("mosaic.invalid", &[0; 32], None, &cancel)
                .err(),
            Some(HardwareKeyError::Unsupported)
        );
        assert_eq!(
            platform.evaluate(&[], None, &cancel).err(),
            Some(HardwareKeyError::Unsupported)
        );
    }
}
