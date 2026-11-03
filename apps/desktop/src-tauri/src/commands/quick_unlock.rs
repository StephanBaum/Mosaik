//! Quick-unlock commands (CRYPTO_SPEC §6.2, §6.3; ADR-0028): unlocking with
//! Windows Hello, Touch ID or a security key (`unlock_quick`), the status of
//! the methods, and enrolling or removing them.
//!
//! Every flow that prompts the user or talks to a device does so **without**
//! the session locked: it reads what it needs under the guard, drops it,
//! prompts, takes the guard again and re-checks the state. A state that
//! changed in between (locked, unlocked, another command running) ends the
//! flow with `locked`, `alreadyUnlocked` or `busy`, and a platform key made
//! in the meantime is deleted again.

use std::sync::TryLockError;
use std::time::Duration;

use mosaic_core::unlock::new_prf_salt;
use mosaic_vault::session::{
    DEFAULT_RP_ID, Error as SessionError, HardwareKeyEnrollment, MAX_HARDWARE_KEYS, NewHardwareKey,
    QuickMethod, QuickUnlockEnrollment, QuickUnlockPolicy, UnlockError, UnlockedSession,
};
use tauri::State;

use super::session::take_locked;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::quick_unlock::{
    BiometricStatus, EnrollBiometricRequest, EnrollHardwareKeyRequest, HardwareKeyStatus,
    HardwareKeySummary, QuickUnlockStatus, RemoveHardwareKeyRequest,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{QuickUnlockKind, QuickUnlockRequest, SessionState};
use crate::quick_unlock::{
    BiometricPlatform, CancelFlag, HardwareCandidate, HardwareKeyError, PlatformError,
};
use crate::state::{AppState, Backend, Slot};

/// `unlock_quick`.
#[tauri::command]
pub async fn unlock_quick(
    state: State<'_, AppState>,
    request: QuickUnlockRequest,
) -> IpcResult<SessionState> {
    state
        .run(move |b| {
            let mut request = request;
            b.unlock_quick(&mut request)
        })
        .await
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

/// `enroll_hardware_key`.
#[tauri::command]
pub async fn enroll_hardware_key(
    state: State<'_, AppState>,
    request: EnrollHardwareKeyRequest,
) -> IpcResult<QuickUnlockStatus> {
    state.run(move |b| b.enroll_hardware_key(request)).await
}

/// `remove_hardware_key`.
#[tauri::command]
pub async fn remove_hardware_key(
    state: State<'_, AppState>,
    request: RemoveHardwareKeyRequest,
) -> IpcResult<QuickUnlockStatus> {
    state.run(move |b| b.remove_hardware_key(&request)).await
}

/// `hardware_key_cancel`.
#[tauri::command]
pub async fn hardware_key_cancel(state: State<'_, AppState>) -> IpcResult<()> {
    state.backend().hardware_key_cancel();
    Ok(())
}

pub(crate) use crate::contract::quick_unlock::{method_id_hex, parse_method_id};

/// The error for a slot that is not `Locked`.
fn not_locked(slot: &Slot) -> IpcError {
    match slot {
        Slot::Unlocked(_) => IpcError::new(ErrorCode::AlreadyUnlocked),
        Slot::Unavailable(code) => IpcError::new(*code),
        Slot::Taken => IpcError::new(ErrorCode::Busy),
        Slot::Locked(_) => IpcError::INTERNAL,
    }
}

/// The error for a slot that is not the unlocked session of the account a
/// flow started with, after a prompt or a device call: it locked meanwhile.
pub(crate) fn changed_while_waiting(slot: &Slot) -> IpcError {
    match slot {
        Slot::Unlocked(_) | Slot::Locked(_) => IpcError::LOCKED,
        Slot::Unavailable(code) => IpcError::new(*code),
        Slot::Taken => IpcError::new(ErrorCode::Busy),
    }
}

/// The wire form of registered keys.
pub(crate) fn summaries(keys: &[HardwareKeyEnrollment]) -> Vec<HardwareKeySummary> {
    keys.iter()
        .map(|key| HardwareKeySummary {
            method_id: method_id_hex(&key.method_id),
            added_at: key.added_at_ms,
        })
        .collect()
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
    pub(crate) fn reconcile_slot(&self, slot: &mut Slot) {
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
        // No enrollment: nothing to do. A platform key is named by the
        // nonce in the enrollment, so one that lost its enrollment (a crash
        // between the two steps) cannot be found again; the flows delete
        // theirs on every failure path, which leaves only that window.
        let Ok(Some(enrollment)) = enrollment else {
            return;
        };
        let payload = &enrollment.platform_params;
        // Before the settings are verified (the start of the app, a locked
        // settings change) only the spec cap deletes: the file's age is not
        // trusted yet and the strict stand-in (14 days) would drop an
        // enrollment the user's verified age still allows (ADR-0030 item 10).
        let policy = if self.settings_key_available() {
            self.quick_unlock_policy()
        } else {
            QuickUnlockPolicy::new(Duration::from_secs(
                u64::from(crate::persist::QUICK_UNLOCK_DAYS.1) * 86_400,
            ))
        };
        let stale = enrollment.is_expired(&policy, self.quick_unlock.now_ms())
            || crate::quick_unlock::params::platform_of(payload) != Some(platform.kind())
            || !platform.key_exists(payload);
        if stale {
            Self::remove_enrollment(slot);
            let _ = platform.remove(payload);
        }
    }

    /// [`Backend::reconcile_slot`], taking the session itself. Does nothing
    /// while a command owns the session.
    pub fn reconcile_quick_unlock(&self) {
        if let Ok(mut slot) = self.slot() {
            self.reconcile_slot(&mut slot);
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
                keys: summaries(&session.hardware_keys()?),
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
    pub fn unlock_quick(&self, request: &mut QuickUnlockRequest) -> IpcResult<SessionState> {
        match request.method {
            QuickUnlockKind::Biometric => self.unlock_biometric(request),
            QuickUnlockKind::HardwareKey => self.unlock_hardware(request),
        }
    }

    fn unlock_biometric(&self, request: &QuickUnlockRequest) -> IpcResult<SessionState> {
        let policy = self.quick_unlock_policy();
        let payload = {
            let mut slot = self.slot()?;
            let locked = match &mut *slot {
                Slot::Locked(locked) => locked,
                other => return Err(not_locked(other)),
            };
            if request.pin.is_some() {
                return Err(IpcError::new(ErrorCode::InvalidInput));
            }
            let platform = self
                .quick_unlock
                .available_biometric()
                .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
            let enrollment = locked
                .quick_unlock_enrollment()?
                .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
            if enrollment.is_expired(&policy, self.quick_unlock.now_ms()) {
                locked.remove_quick_unlock()?;
                let _ = platform.remove(&enrollment.platform_params);
                return Err(IpcError::new(ErrorCode::QuickUnlockExpired));
            }
            if locked.password_required(QuickMethod::Biometric) {
                return Err(IpcError::new(ErrorCode::PasswordRequired));
            }
            enrollment.platform_params
        };

        let platform = self
            .quick_unlock
            .biometric()
            .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;
        // The OS prompt: nothing is held.
        let w = match platform.release(&payload) {
            Ok(w) => w,
            Err(PlatformError::Unavailable) => {
                // The key is gone or the payload is not ours: the enrollment
                // can never open again.
                if let Ok(mut slot) = self.slot() {
                    Self::remove_enrollment(&mut slot);
                }
                let _ = platform.remove(&payload);
                return Err(IpcError::new(ErrorCode::QuickUnlockUnavailable));
            }
            Err(error) => return Err(error.into()),
        };

        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        let unlocked = locked.unlock_quick(&w, &policy, self.quick_unlock.now_ms());
        // `W` is not needed past the vault's check: zeroize it now, not at
        // the end of the flow.
        drop(w);
        match unlocked {
            Ok(session) => self.finish_quick_unlock(&mut slot, session),
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                if matches!(
                    error,
                    SessionError::QuickUnlockInvalidated | SessionError::QuickUnlockExpired
                ) {
                    let _ = platform.remove(&payload);
                }
                Err(error.into())
            }
        }
    }

    fn unlock_hardware(&self, request: &mut QuickUnlockRequest) -> IpcResult<SessionState> {
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

        let candidates: Vec<HardwareCandidate<'_>> = keys
            .iter()
            .map(|key| HardwareCandidate {
                rp_id: &key.rp_id,
                credential_id: &key.credential_id,
                prf_salt: &key.prf_salt,
            })
            .collect();
        // The PIN is dropped (zeroized) as soon as the device call is over.
        let pin = request.pin.take();
        let evaluated = self.device_op(|cancel| {
            hardware.evaluate(&candidates, pin.as_ref().map(SecretIn::as_secret), cancel)
        });
        drop(pin);
        let (index, prf) = match evaluated? {
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
        let unlocked = locked.unlock_with_hardware_key(credential, &prf);
        drop(prf);
        match unlocked {
            Ok(session) => self.finish_quick_unlock(&mut slot, session),
            Err(UnlockError { session, error }) => {
                *slot = Slot::Locked(session);
                Err(error.into())
            }
        }
    }

    /// Runs one security-key operation: one at a time (a second caller gets
    /// `busy`), with the cancel flag cleared first. The session is never
    /// held while this runs. The outer error is `busy`; the inner one is the
    /// device's.
    pub(crate) fn device_op<T>(
        &self,
        op: impl FnOnce(&CancelFlag) -> Result<T, HardwareKeyError>,
    ) -> IpcResult<Result<T, HardwareKeyError>> {
        let guard = match self.quick_unlock.hardware_op.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Err(IpcError::new(ErrorCode::Busy)),
            Err(TryLockError::Poisoned(_)) => return Err(IpcError::INTERNAL),
        };
        let cancel = self.quick_unlock.cancel_flag();
        cancel.reset();
        let result = op(cancel);
        drop(guard);
        Ok(result)
    }

    /// The end of every successful quick unlock: like a password unlock.
    fn finish_quick_unlock(
        &self,
        slot: &mut Slot,
        session: UnlockedSession,
    ) -> IpcResult<SessionState> {
        let account_id = session.account_id();
        *slot = Slot::Unlocked(Box::new(session));
        self.settings_unlocked(slot);
        self.remember_account(slot, account_id, None);
        self.start_ssh_if_enabled(slot);
        self.touch();
        self.reconcile_slot(slot);
        self.state_of(slot)
    }

    /// Sets up the biometric method (Windows Hello or Touch ID). One at a
    /// time: a second call while one runs answers `busy` before it asks
    /// for anything. A success uses up the re-authentication grant; a
    /// failure keeps it.
    ///
    /// # Errors
    /// `locked`, `busy`, `reauthRequired`, `wrongCredentials` (the vault
    /// locks after five in a row), `quickUnlockUnavailable`, `cancelled`,
    /// or a store error.
    pub fn enroll_biometric(
        &self,
        request: EnrollBiometricRequest,
    ) -> IpcResult<QuickUnlockStatus> {
        self.touch();
        let account = self.slot()?.unlocked()?.account_id();
        let _one_at_a_time = match self.quick_unlock.biometric_op.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Err(IpcError::new(ErrorCode::Busy)),
            Err(TryLockError::Poisoned(_)) => return Err(IpcError::INTERNAL),
        };
        self.require_reauth(request.password)?;
        let platform = self
            .quick_unlock
            .available_biometric()
            .ok_or(IpcError::new(ErrorCode::QuickUnlockUnavailable))?;

        // The OS prompts (create the key, prove it opens): nothing is held.
        let (payload, w) = platform.enroll()?;

        let mut slot = self.slot()?;
        let session = match &mut *slot {
            Slot::Unlocked(session) if session.account_id() == account => session,
            other => {
                let error = changed_while_waiting(other);
                let _ = platform.remove(&payload);
                return Err(error);
            }
        };
        // The key of the enrollment this one replaces.
        let previous = session
            .quick_unlock_enrollment()
            .ok()
            .flatten()
            .map(|e| e.platform_params);
        let enrolled = self
            .quick_unlock
            .consume_reauth(|proof| session.enroll_quick_unlock(&w, &payload, proof));
        drop(w);
        match enrolled {
            Some(Ok(_)) => {
                if let Some(previous) = previous.filter(|p| *p != payload) {
                    let _ = platform.remove(&previous);
                }
            }
            Some(Err(error)) => {
                let _ = platform.remove(&payload);
                return Err(error.into());
            }
            None => {
                let _ = platform.remove(&payload);
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
        let enrollment = session.quick_unlock_enrollment()?;
        session.remove_quick_unlock()?;
        if let (Some(platform), Some(enrollment)) = (self.quick_unlock.biometric(), enrollment) {
            // An already missing key is fine.
            let _ = platform.remove(&enrollment.platform_params);
        }
        self.status_of(slot.unlocked()?)
    }

    /// Deletes the platform key of `removed` after the vault removed its
    /// quick-unlock enrollment on its own (a master password change, a
    /// Secret Key rotation or a re-key: `PasswordChanged::removed_quick_unlock`).
    /// Best effort.
    pub fn platform_key_after_removal(&self, removed: Option<&QuickUnlockEnrollment>) {
        if let (Some(removed), Some(platform)) = (removed, self.quick_unlock.biometric()) {
            let _ = platform.remove(&removed.platform_params);
        }
    }

    /// Registers a security key for unlock: the key is created and asked
    /// once (touch, PIN), then the vault wraps the account key under the
    /// PRF output.
    ///
    /// # Errors
    /// `locked`, `hardwareKeyUnsupported`, `reauthRequired`,
    /// `wrongCredentials` (the vault locks after five in a row),
    /// `hardwareKeyLimit`, `busy`, `cancelled`, the device codes, or a store
    /// error.
    pub fn enroll_hardware_key(
        &self,
        mut request: EnrollHardwareKeyRequest,
    ) -> IpcResult<QuickUnlockStatus> {
        self.touch();
        let account = self.slot()?.unlocked()?.account_id();
        let hardware = self.quick_unlock.hardware();
        if !hardware.is_supported() {
            return Err(IpcError::new(ErrorCode::HardwareKeyUnsupported));
        }
        self.require_reauth(request.password.take())?;
        if self.slot()?.unlocked()?.hardware_keys()?.len() >= MAX_HARDWARE_KEYS {
            return Err(IpcError::new(ErrorCode::HardwareKeyLimit));
        }
        let salt = new_prf_salt()?;
        let pin = request.pin.take();
        let registered = self.device_op(|cancel| {
            hardware.register(
                DEFAULT_RP_ID,
                &salt,
                pin.as_ref().map(SecretIn::as_secret),
                cancel,
            )
        });
        drop(pin);
        let (credential_id, prf) = registered??;
        let key = NewHardwareKey::new(credential_id, DEFAULT_RP_ID.to_owned(), salt)?;

        let mut slot = self.slot()?;
        let session = match &mut *slot {
            Slot::Unlocked(session) if session.account_id() == account => session,
            other => return Err(changed_while_waiting(other)),
        };
        let enrolled = self
            .quick_unlock
            .consume_reauth(|proof| session.enroll_hardware_key(&key, &prf, proof));
        drop(prf);
        match enrolled {
            Some(Ok(_)) => {}
            Some(Err(error)) => return Err(error.into()),
            None => return Err(IpcError::new(ErrorCode::ReauthRequired)),
        }
        self.status_of(slot.unlocked()?)
    }

    /// Removes a registered security key. No re-authentication: removing
    /// only narrows access.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (not 32 lowercase hex digits),
    /// `hardwareKeyNotRecognized` (not registered), or a store error.
    pub fn remove_hardware_key(
        &self,
        request: &RemoveHardwareKeyRequest,
    ) -> IpcResult<QuickUnlockStatus> {
        self.touch();
        let method_id =
            parse_method_id(&request.method_id).ok_or(IpcError::new(ErrorCode::InvalidInput))?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        if !session.remove_hardware_key(&method_id)? {
            return Err(IpcError::new(ErrorCode::HardwareKeyNotRecognized));
        }
        self.status_of(slot.unlocked()?)
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
