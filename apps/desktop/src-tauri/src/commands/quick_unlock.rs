//! Quick-unlock commands (CRYPTO_SPEC §6.2, §6.3; ADR-0028): unlocking with
//! Windows Hello or Touch ID (`unlock_quick`), the status of the methods,
//! and enrolling or removing the biometric method.
//!
//! Every flow that prompts the user or talks to a device does so **without**
//! the session locked: it reads what it needs under the guard, drops it,
//! prompts, takes the guard again and re-checks the state. A state that
//! changed in between (locked, unlocked, another command running) ends the
//! flow with `locked`, `alreadyUnlocked` or `busy`, and a platform key made
//! in the meantime is deleted again.

use std::sync::TryLockError;
use std::time::Duration;

use mosaic_core::AccountId;
use mosaic_vault::session::{
    Error as SessionError, MAX_HARDWARE_KEYS, QuickMethod, QuickUnlockEnrollment,
    QuickUnlockPolicy, UnlockError, UnlockedSession,
};
use tauri::State;

use super::session::take_locked;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::quick_unlock::{
    BiometricStatus, EnrollBiometricRequest, HardwareKeyStatus, HardwareKeySummary,
    QuickUnlockStatus,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{QuickUnlockKind, QuickUnlockRequest, SessionState};
use crate::quick_unlock::{BiometricPlatform, HardwareCandidate, HardwareKeyError, PlatformError};
use crate::state::{AppState, Backend, Slot};

/// `unlock_quick`.
#[tauri::command]
pub async fn unlock_quick(
    state: State<'_, AppState>,
    request: QuickUnlockRequest,
) -> IpcResult<SessionState> {
    state.run(move |b| b.unlock_quick(&request)).await
}

/// `quick_unlock_status`.
#[tauri::command]
pub async fn quick_unlock_status(state: State<'_, AppState>) -> IpcResult<QuickUnlockStatus> {
    state.run(Backend::quick_unlock_status).await
}

/// `enroll_biometric`.
#[tauri::command]
pub async fn enroll_biometric(
    state: State<'_, AppState>,
    request: EnrollBiometricRequest,
) -> IpcResult<QuickUnlockStatus> {
    state.run(move |b| b.enroll_biometric(request)).await
}

/// `remove_biometric`.
#[tauri::command]
pub async fn remove_biometric(state: State<'_, AppState>) -> IpcResult<QuickUnlockStatus> {
    state.run(Backend::remove_biometric).await
}

/// `hardware_key_cancel`.
#[tauri::command]
pub async fn hardware_key_cancel(state: State<'_, AppState>) -> IpcResult<()> {
    state.backend().hardware_key_cancel();
    Ok(())
}

/// Lowercase hex of a method ID.
pub(crate) fn method_id_hex(method_id: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    method_id
        .iter()
        .fold(String::with_capacity(32), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// A method ID from exactly 32 lowercase hex digits (the hardware-key
/// commands).
#[allow(dead_code)]
pub(crate) fn parse_method_id(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    for (byte, [high, low]) in out.iter_mut().zip(pairs) {
        *byte = (digit(*high)? << 4) | digit(*low)?;
    }
    Some(out)
}

/// The error for a slot that is not `Locked`.
fn not_locked(slot: &Slot) -> IpcError {
    match slot {
        Slot::Unlocked(_) => IpcError::new(ErrorCode::AlreadyUnlocked),
        Slot::Unavailable(code) => IpcError::new(*code),
        Slot::Taken => IpcError::new(ErrorCode::Busy),
        Slot::Locked(_) => IpcError::INTERNAL,
    }
}

fn hardware_summaries(session: &UnlockedSession) -> Result<Vec<HardwareKeySummary>, SessionError> {
    Ok(session
        .hardware_keys()?
        .iter()
        .map(|key| HardwareKeySummary {
            method_id: method_id_hex(&key.method_id),
            added_at: key.added_at_ms,
        })
        .collect())
}

impl Backend {
    /// The quick-unlock policy from the settings (maximum age in days).
    pub(crate) fn quick_unlock_policy(&self) -> QuickUnlockPolicy {
        let days = u64::from(self.settings_snapshot().quick_unlock_max_age_days);
        QuickUnlockPolicy::new(Duration::from_secs(days.saturating_mul(86_400)))
    }

    /// Removes the quick-unlock wrapping from whichever session variant the
    /// slot holds. Best effort.
    fn remove_enrollment(slot: &mut Slot) {
        match slot {
            Slot::Locked(locked) => {
                let _ = locked.remove_quick_unlock();
            }
            Slot::Unlocked(session) => {
                let _ = session.remove_quick_unlock();
            }
            Slot::Unavailable(_) | Slot::Taken => {}
        }
    }

    /// Brings the platform key and the stored wrapping into agreement: an
    /// enrollment that expired, whose platform key is gone or that was made
    /// for another platform is removed with its key; a platform key without
    /// an enrollment is removed. Runs at start, after every unlock and after
    /// a settings change. Errors are swallowed: it is housekeeping, and
    /// nothing in it is shown.
    pub(crate) fn reconcile_slot(&self, slot: &mut Slot, account: AccountId) {
        let Some(platform) = self.quick_unlock.biometric() else {
            // No platform here: an enrollment (a copied store) cannot be used.
            if matches!(slot, Slot::Locked(_) | Slot::Unlocked(_)) {
                let enrolled = match slot {
                    Slot::Locked(locked) => locked.quick_unlock_enrollment().ok().flatten(),
                    Slot::Unlocked(session) => session.quick_unlock_enrollment().ok().flatten(),
                    Slot::Unavailable(_) | Slot::Taken => None,
                };
                if enrolled.is_some() {
                    Self::remove_enrollment(slot);
                }
            }
            return;
        };
        let enrollment = match slot {
            Slot::Locked(locked) => locked.quick_unlock_enrollment(),
            Slot::Unlocked(session) => session.quick_unlock_enrollment(),
            Slot::Unavailable(_) | Slot::Taken => return,
        };
        let Ok(enrollment) = enrollment else {
            return;
        };
        let key_exists = platform.key_exists(account);
        match enrollment {
            Some(enrollment) => {
                let stale = enrollment
                    .is_expired(&self.quick_unlock_policy(), self.quick_unlock.now_ms())
                    || !key_exists
                    || crate::quick_unlock::params::platform_of(&enrollment.platform_params)
                        != Some(platform.kind());
                if stale {
                    Self::remove_enrollment(slot);
                    let _ = platform.remove(account);
                }
            }
            None => {
                if key_exists {
                    let _ = platform.remove(account);
                }
            }
        }
    }

    /// [`Backend::reconcile_slot`] for the stored account, taking the
    /// session itself. Does nothing while a command owns the session.
    pub fn reconcile_quick_unlock(&self) {
        let Some(account) = self.account().map(|a| a.account_id.get()) else {
            return;
        };
        if let Ok(mut slot) = self.slot() {
            self.reconcile_slot(&mut slot, account);
        }
    }

    /// The status of the quick-unlock methods, for the settings screen.
    ///
    /// # Errors
    /// `locked`, or a store error.
    pub fn quick_unlock_status(&self) -> IpcResult<QuickUnlockStatus> {
        let slot = self.slot()?;
        self.status_of(slot.unlocked()?)
    }

    fn status_of(&self, session: &UnlockedSession) -> IpcResult<QuickUnlockStatus> {
        let policy = self.quick_unlock_policy();
        let platform = self.quick_unlock.biometric();
        let enrollment = session
            .quick_unlock_enrollment()?
            .filter(|e| !e.is_expired(&policy, self.quick_unlock.now_ms()));
        Ok(QuickUnlockStatus {
            biometric: BiometricStatus {
                platform: platform.map(BiometricPlatform::kind),
                available: platform.is_some_and(BiometricPlatform::is_available),
                enrolled: enrollment.is_some(),
                enrolled_at: enrollment.as_ref().map(|e| e.enrolled_at_ms),
                expires_at: enrollment.as_ref().map(|e| e.expires_at_ms(&policy)),
            },
            hardware_keys: HardwareKeyStatus {
                supported: self.quick_unlock.hardware().is_supported(),
                keys: hardware_summaries(session)?,
                max_keys: u32::try_from(MAX_HARDWARE_KEYS).unwrap_or(u32::MAX),
            },
            max_age_days: u32::try_from(policy.max_age().as_secs() / 86_400).unwrap_or(30),
        })
    }

    /// Unlocks with a quick method (CRYPTO_SPEC §6.2, §6.3). The Secret Key
    /// is not needed. The OS prompt or the device call runs with the
    /// session unlocked by nobody else and not held.
    ///
    /// # Errors
    /// `alreadyUnlocked`, `quickUnlockUnavailable`, `quickUnlockExpired`,
    /// `quickUnlockInvalidated`, `passwordRequired`, `cancelled`, `busy`,
    /// `invalidInput` (a PIN with `biometric`), the hardware-key codes, or
    /// the store's code.
    pub fn unlock_quick(&self, request: &QuickUnlockRequest) -> IpcResult<SessionState> {
        match request.method {
            QuickUnlockKind::Biometric => self.unlock_biometric(request),
            QuickUnlockKind::HardwareKey => self.unlock_hardware(request),
        }
    }

    fn unlock_biometric(&self, request: &QuickUnlockRequest) -> IpcResult<SessionState> {
        let policy = self.quick_unlock_policy();
        let (account, payload) = {
            let mut slot = self.slot()?;
            let locked = match &mut *slot {
                Slot::Locked(locked) => locked,
                other => return Err(not_locked(other)),
            };
            if request.pin.is_some() {
                return Err(IpcError::new(ErrorCode::InvalidInput));
            }
            let account = self
                .account()
                .map(|a| a.account_id.get())
                .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
            let platform = self
                .quick_unlock
                .available_biometric()
                .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
            let enrollment = locked
                .quick_unlock_enrollment()?
                .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
            if enrollment.is_expired(&policy, self.quick_unlock.now_ms()) {
                locked.remove_quick_unlock()?;
                let _ = platform.remove(account);
                return Err(IpcError::new(ErrorCode::QuickUnlockExpired));
            }
            if locked.password_required(QuickMethod::Biometric) {
                return Err(IpcError::new(ErrorCode::PasswordRequired));
            }
            (account, enrollment.platform_params)
        };

        let platform = self
            .quick_unlock
            .biometric()
            .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
        // The OS prompt: nothing is held.
        let w = match platform.release(account, &payload) {
            Ok(w) => w,
            Err(PlatformError::Unavailable) => {
                // The key is gone or the payload is not ours: the enrollment
                // can never open again.
                if let Ok(mut slot) = self.slot() {
                    Self::remove_enrollment(&mut slot);
                }
                let _ = platform.remove(account);
                return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
            }
            Err(error) => return Err(error.into()),
        };

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match locked.unlock_quick(&w, &policy, self.quick_unlock.now_ms()) {
            Ok(session) => self.finish_quick_unlock(&mut slot, session),
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                if matches!(
                    error,
                    SessionError::QuickUnlockInvalidated | SessionError::QuickUnlockExpired
                ) {
                    let _ = platform.remove(account);
                }
                Err(error.into())
            }
        }
    }

    fn unlock_hardware(&self, request: &QuickUnlockRequest) -> IpcResult<SessionState> {
        let hardware = self.quick_unlock.hardware();
        let keys = {
            let mut slot = self.slot()?;
            let locked = match &mut *slot {
                Slot::Locked(locked) => locked,
                other => return Err(not_locked(other)),
            };
            if !hardware.is_supported() {
                return Err(IpcError::new(ErrorCode::HardwareKeyUnsupported));
            }
            let keys = locked.hardware_keys()?;
            if keys.is_empty() {
                return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
            }
            if locked.password_required(QuickMethod::HardwareKey) {
                return Err(IpcError::new(ErrorCode::PasswordRequired));
            }
            keys
        };

        // One device operation at a time; the session is not held.
        let op = match self.quick_unlock.hardware_op.try_lock() {
            Ok(op) => op,
            Err(TryLockError::WouldBlock) => return Err(IpcError::new(ErrorCode::Busy)),
            Err(TryLockError::Poisoned(_)) => return Err(IpcError::INTERNAL),
        };
        let cancel = self.quick_unlock.cancel_flag();
        cancel.reset();
        let candidates: Vec<HardwareCandidate<'_>> = keys
            .iter()
            .map(|key| HardwareCandidate {
                rp_id: &key.rp_id,
                credential_id: &key.credential_id,
                prf_salt: &key.prf_salt,
            })
            .collect();
        let pin = request.pin.as_ref().map(SecretIn::as_secret);
        let evaluated = hardware.evaluate(&candidates, pin, cancel);
        drop(op);
        let (index, prf) = match evaluated {
            Ok(done) => done,
            Err(error) => {
                if error == HardwareKeyError::PinInvalid
                    && let Ok(mut slot) = self.slot()
                    && let Slot::Locked(locked) = &mut *slot
                {
                    locked.note_failed_hardware_attempt();
                }
                return Err(error.into());
            }
        };
        let credential = keys
            .get(index)
            .map(|key| key.credential_id.as_slice())
            .ok_or(IpcError::INTERNAL)?;

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match locked.unlock_with_hardware_key(credential, &prf) {
            Ok(session) => self.finish_quick_unlock(&mut slot, session),
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                Err(error.into())
            }
        }
    }

    /// The end of every successful quick unlock: like a password unlock.
    fn finish_quick_unlock(
        &self,
        slot: &mut Slot,
        session: UnlockedSession,
    ) -> IpcResult<SessionState> {
        let account_id = session.account_id();
        *slot = Slot::Unlocked(Box::new(session));
        self.remember_account(account_id, None);
        self.start_ssh_if_enabled(slot);
        self.touch();
        self.reconcile_slot(slot, account_id);
        self.state_of(slot)
    }

    /// Sets up the biometric method (Windows Hello or Touch ID).
    ///
    /// # Errors
    /// `locked`, `reauthRequired`, `wrongCredentials` (the vault locks after
    /// five in a row), `quickUnlockUnavailable`, `cancelled`, or a store
    /// error.
    pub fn enroll_biometric(
        &self,
        request: EnrollBiometricRequest,
    ) -> IpcResult<QuickUnlockStatus> {
        self.touch();
        let account = self.slot()?.unlocked()?.account_id();
        self.require_reauth(request.password)?;
        let platform = self
            .quick_unlock
            .available_biometric()
            .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;

        // The OS prompts (create the key, prove it opens): nothing is held.
        let (payload, w) = platform.enroll(account)?;

        let mut slot = self.slot()?;
        let session = match &mut *slot {
            Slot::Unlocked(session) if session.account_id() == account => session,
            other => {
                let error = match other {
                    Slot::Unlocked(_) | Slot::Locked(_) => IpcError::LOCKED,
                    Slot::Unavailable(code) => IpcError::new(*code),
                    Slot::Taken => IpcError::new(ErrorCode::Busy),
                };
                let _ = platform.remove(account);
                return Err(error);
            }
        };
        let enrolled = self
            .quick_unlock
            .with_reauth(|proof| session.enroll_quick_unlock(&w, &payload, proof));
        match enrolled {
            Some(Ok(_)) => {}
            Some(Err(error)) => {
                let _ = platform.remove(account);
                return Err(error.into());
            }
            None => {
                let _ = platform.remove(account);
                return Err(IpcError::new(ErrorCode::ReauthRequired));
            }
        }
        self.status_of(slot.unlocked()?)
    }

    /// Removes the biometric method: the wrapping, then the platform key.
    /// No re-authentication: removing only narrows access.
    ///
    /// # Errors
    /// `locked`, or a store error.
    pub fn remove_biometric(&self) -> IpcResult<QuickUnlockStatus> {
        self.touch();
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let account = session.account_id();
        session.remove_quick_unlock()?;
        if let Some(platform) = self.quick_unlock.biometric() {
            // An already missing key is fine.
            let _ = platform.remove(account);
        }
        self.status_of(slot.unlocked()?)
    }

    /// Deletes the platform key of `account` after the vault removed its
    /// quick-unlock enrollment on its own (a master password change, a
    /// Secret Key rotation or a re-key: `PasswordChanged::removed_quick_unlock`).
    /// Best effort.
    pub fn platform_key_after_removal(
        &self,
        account: AccountId,
        removed: Option<&QuickUnlockEnrollment>,
    ) {
        if removed.is_some()
            && let Some(platform) = self.quick_unlock.biometric()
        {
            let _ = platform.remove(account);
        }
    }

    /// Cancels a running security-key operation (no-op when idle).
    pub fn hardware_key_cancel(&self) {
        self.quick_unlock.cancel_flag().cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_ids_are_32_lowercase_hex_digits() {
        let id = [
            0x00, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xff, 0x10, 0x20, 0x30, 0x40,
            0x50, 0x60,
        ];
        let text = method_id_hex(&id);
        assert_eq!(text, "000123456789abcdefff102030405060");
        assert_eq!(parse_method_id(&text), Some(id));
        for bad in [
            "",
            "000123456789abcdefff10203040506",
            "000123456789abcdefff1020304050600",
            "000123456789ABCDEFFF102030405060",
            "000123456789abcdefff10203040506g",
            "000123456789abcdefff1020304050 6",
        ] {
            assert_eq!(parse_method_id(bad), None, "{bad:?}");
        }
    }
}
