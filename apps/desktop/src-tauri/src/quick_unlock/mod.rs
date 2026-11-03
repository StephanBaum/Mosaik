//! Quick unlock (CRYPTO_SPEC §6.2, §6.3; ADR-0028): a wrapping of the
//! account key under a key `W` that only a user-verified platform operation
//! can produce.
//!
//! The vault side (`mosaic-vault`) stores the wrappings, enforces the
//! app-start rule, the maximum age and the failed-attempt limit, and deletes
//! a wrapping that expired or does not open. This module is the app side:
//!
//! - [`BiometricPlatform`]: the OS half. It creates a hardware-bound,
//!   non-exportable key and, after one user prompt, produces `W`. Windows
//!   Hello ([`windows_hello`]) and Touch ID ([`touch_id`]) implement it;
//!   Linux has none by spec.
//! - [`HardwareKeyPlatform`]: the security-key transport. Production uses
//!   [`NoHardwareKeys`] until the FIDO2 transport lands (ADR-0029), so
//!   every hardware-key command answers `hardwareKeyUnsupported`.
//! - [`QuickUnlockService`]: the two platforms, the re-authentication grant
//!   (it holds the vault's [`Reauthenticated`] proof), the count of wrong
//!   re-authentications, the cancel flag of a running device operation and
//!   the deadlines of open hardware-protected vaults.
//!
//! The command flows (`unlock_quick`, `enroll_biometric`, ...) are in
//! `commands/session.rs` and `commands/quick_unlock.rs`. They never hold the
//! session lock across an OS prompt or a device call.

pub mod hardware;
pub mod params;
#[cfg(target_os = "macos")]
pub mod touch_id;
#[cfg(windows)]
pub mod windows_hello;

#[cfg(test)]
pub(crate) mod fakes;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mosaic_core::{Kek, VaultId};
use mosaic_vault::session::{
    LockedSession, QuickMethod, QuickUnlockPolicy, REAUTH_GRANT as VAULT_REAUTH_GRANT,
    Reauthenticated,
};

pub use self::hardware::{
    CancelFlag, HardwareCandidate, HardwareKeyError, HardwareKeyPlatform, NoHardwareKeys,
};
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::quick_unlock::BiometricPlatformKind;
use crate::contract::session::QuickUnlockKind;

/// How long a right master password lets the enroll commands run without
/// asking again (the vault's own limit, CRYPTO_SPEC §6.2).
pub const REAUTH_GRANT: Duration = VAULT_REAUTH_GRANT;
/// Wrong re-authentications in a row, in one unlocked session, before the
/// vault locks (ADR-0028 item 8).
pub const MAX_FAILED_REAUTH: u8 = 5;
/// How long a hardware-protected vault stays open after its key was used
/// (CRYPTO_SPEC section 7.4). Viewing a code does not extend it.
pub const HARDWARE_VAULT_OPEN_FOR: Duration = Duration::from_secs(300);

/// Why a platform operation failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformError {
    /// The OS cannot do it (no hardware, not set up, key gone, wrong
    /// platform).
    Unavailable,
    /// The user dismissed the prompt.
    Cancelled,
    /// Anything else.
    Failed,
}

impl From<PlatformError> for IpcError {
    fn from(error: PlatformError) -> Self {
        Self::new(match error {
            PlatformError::Unavailable => ErrorCode::QuickUnlockUnavailable,
            PlatformError::Cancelled => ErrorCode::Cancelled,
            PlatformError::Failed => ErrorCode::Internal,
        })
    }
}

/// The OS half of biometric quick unlock.
pub trait BiometricPlatform: Send + Sync {
    /// Which facility this is.
    fn kind(&self) -> BiometricPlatformKind;

    /// Whether the OS offers it right now. Never prompts. Windows:
    /// `IsSupportedAsync`. macOS: a non-permanent Secure Enclave probe key
    /// with the same access control can be created.
    fn is_available(&self) -> bool;

    /// Whether the platform key named by `payload` exists. Never prompts.
    /// `false` for a payload that is not well-formed for this platform.
    fn key_exists(&self, payload: &[u8]) -> bool;

    /// Creates a new platform key under a fresh random name (the key nonce
    /// in the payload, see [`params`]; it is per enrollment, never per
    /// account). Returns the platform payload (the parameters of the
    /// wrapping) and `W`. May prompt. The caller deletes the key of the
    /// enrollment it replaces with [`BiometricPlatform::remove`].
    ///
    /// # Errors
    /// [`PlatformError`].
    fn enroll(&self) -> Result<(Vec<u8>, Kek), PlatformError>;

    /// One user prompt; returns `W` for the stored `payload`.
    ///
    /// # Errors
    /// [`PlatformError::Cancelled`] if the user dismisses the prompt;
    /// [`PlatformError::Unavailable`] if the key is gone or the payload is
    /// for another platform.
    fn release(&self, payload: &[u8]) -> Result<Kek, PlatformError>;

    /// Deletes the platform key named by `payload`. A key that is already
    /// gone is fine.
    ///
    /// # Errors
    /// [`PlatformError::Unavailable`] for a payload that is not well-formed
    /// for this platform, else [`PlatformError::Failed`].
    fn remove(&self, payload: &[u8]) -> Result<(), PlatformError>;
}

/// The wall clock in Unix milliseconds.
pub(crate) fn system_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

/// What the re-authentication grant remembers.
pub(crate) struct Grant {
    proof: Reauthenticated,
    at: Instant,
    /// The lock count when it was earned: a grant taken out before a lock is
    /// not put back after it.
    epoch: u64,
}

/// Quick unlock as the backend sees it.
pub struct QuickUnlockService {
    biometric: Option<Box<dyn BiometricPlatform>>,
    hardware: Box<dyn HardwareKeyPlatform>,
    /// One device operation at a time; a second one answers `busy`.
    pub(crate) hardware_op: Mutex<()>,
    /// One biometric enrollment at a time; a second one answers `busy`.
    pub(crate) biometric_op: Mutex<()>,
    cancel: CancelFlag,
    /// The proof of a right master password, valid for [`REAUTH_GRANT`] and
    /// cleared on lock. The vault enforces the limit; `at` lets the app
    /// answer `reauthRequired` before it prompts.
    reauth: Mutex<Option<Grant>>,
    failed_reauth: AtomicU8,
    /// Counts locks (see [`Grant`]).
    lock_epoch: AtomicU64,
    /// When each open hardware-protected vault closes: the monotonic and
    /// the wall-clock reading, and the vault closes when **either** passed
    /// (like `Activity::idle_at`, so a suspend that stops the monotonic
    /// clock does not keep it open).
    open_vaults: Mutex<BTreeMap<VaultId, (Instant, SystemTime)>>,
    clock: Box<dyn Fn() -> i64 + Send + Sync>,
}

impl Default for QuickUnlockService {
    /// No biometric method and no security keys.
    fn default() -> Self {
        Self::new(None, Box::new(NoHardwareKeys))
    }
}

impl QuickUnlockService {
    /// Quick unlock over these platforms.
    #[must_use]
    pub fn new(
        biometric: Option<Box<dyn BiometricPlatform>>,
        hardware: Box<dyn HardwareKeyPlatform>,
    ) -> Self {
        Self {
            biometric,
            hardware,
            hardware_op: Mutex::new(()),
            biometric_op: Mutex::new(()),
            cancel: CancelFlag::default(),
            reauth: Mutex::new(None),
            failed_reauth: AtomicU8::new(0),
            lock_epoch: AtomicU64::new(0),
            open_vaults: Mutex::new(BTreeMap::new()),
            clock: Box::new(system_now_ms),
        }
    }

    /// The platforms of this OS: Windows Hello on Windows, Touch ID on
    /// macOS, none on Linux; no security keys until ADR-0029.
    #[must_use]
    pub fn for_this_os() -> Self {
        #[cfg(windows)]
        let biometric: Option<Box<dyn BiometricPlatform>> =
            Some(Box::new(windows_hello::WindowsHello));
        #[cfg(target_os = "macos")]
        let biometric: Option<Box<dyn BiometricPlatform>> = Some(Box::new(touch_id::TouchId));
        #[cfg(not(any(windows, target_os = "macos")))]
        let biometric: Option<Box<dyn BiometricPlatform>> = None;
        Self::new(biometric, Box::new(NoHardwareKeys))
    }

    /// Uses this clock (Unix milliseconds) for expiry decisions.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_clock(mut self, clock: Box<dyn Fn() -> i64 + Send + Sync>) -> Self {
        self.clock = clock;
        self
    }

    /// The biometric platform, if this OS has one.
    #[must_use]
    pub fn biometric(&self) -> Option<&dyn BiometricPlatform> {
        self.biometric.as_deref()
    }

    /// The biometric platform if it is there and available right now.
    #[must_use]
    pub fn available_biometric(&self) -> Option<&dyn BiometricPlatform> {
        self.biometric().filter(|p| p.is_available())
    }

    /// The security-key platform.
    #[must_use]
    pub fn hardware(&self) -> &dyn HardwareKeyPlatform {
        &*self.hardware
    }

    /// The cancel flag of the running device operation.
    #[must_use]
    pub fn cancel_flag(&self) -> &CancelFlag {
        &self.cancel
    }

    /// The current time in Unix milliseconds.
    #[must_use]
    pub fn now_ms(&self) -> i64 {
        (self.clock)()
    }

    /// Methods the unlock screen may offer on this locked session, in
    /// display order. Never prompts and never writes: an expired enrollment
    /// is removed by reconciling, not here.
    #[must_use]
    pub fn offered(
        &self,
        locked: &LockedSession,
        policy: &QuickUnlockPolicy,
    ) -> Vec<QuickUnlockKind> {
        let mut offered = Vec::new();
        if let Some(platform) = self.biometric()
            && platform.is_available()
            && !locked.password_required(QuickMethod::Biometric)
            && locked
                .quick_unlock_enrollment()
                .ok()
                .flatten()
                .is_some_and(|e| {
                    !e.is_expired(policy, self.now_ms())
                        && params::platform_of(&e.platform_params) == Some(platform.kind())
                })
        {
            offered.push(QuickUnlockKind::Biometric);
        }
        if self.hardware.is_supported()
            && !locked.password_required(QuickMethod::HardwareKey)
            && locked.hardware_keys().is_ok_and(|keys| !keys.is_empty())
        {
            offered.push(QuickUnlockKind::HardwareKey);
        }
        offered
    }

    fn grant(&self) -> Option<MutexGuard<'_, Option<Grant>>> {
        self.reauth.lock().ok()
    }

    /// Remembers the proof of a right master password.
    pub(crate) fn grant_reauth(&self, proof: Reauthenticated) {
        if let Some(mut grant) = self.grant() {
            *grant = Some(Grant {
                proof,
                at: Instant::now(),
                epoch: self.lock_epoch.load(Ordering::SeqCst),
            });
        }
        self.failed_reauth.store(0, Ordering::SeqCst);
    }

    /// Whether a grant is held and not older than [`REAUTH_GRANT`].
    pub(crate) fn reauth_granted(&self) -> bool {
        self.grant().is_some_and(|grant| {
            grant
                .as_ref()
                .is_some_and(|g| g.at.elapsed() <= REAUTH_GRANT)
        })
    }

    /// Runs `f` with the held proof and **consumes** it when `f` succeeds
    /// (an enrollment uses the grant up: the next one asks again). A failed
    /// `f` keeps it. `None` without a grant.
    pub(crate) fn consume_reauth<T, E>(
        &self,
        f: impl FnOnce(&Reauthenticated) -> Result<T, E>,
    ) -> Option<Result<T, E>> {
        let mut grant = self.grant()?;
        let result = f(&grant.as_ref()?.proof);
        if result.is_ok() {
            *grant = None;
        }
        Some(result)
    }

    /// Takes the held grant out, atomically: of two concurrent commands that
    /// need one grant, only one gets it. `None` without a grant or when it
    /// is older than [`REAUTH_GRANT`]. A command that fails after taking it
    /// hands it back with [`QuickUnlockService::restore_reauth`]; one that
    /// succeeds simply drops it (the grant is spent).
    pub(crate) fn take_reauth(&self) -> Option<Grant> {
        self.grant()?
            .take()
            .filter(|g| g.at.elapsed() <= REAUTH_GRANT)
    }

    /// Puts a grant taken by [`QuickUnlockService::take_reauth`] back after
    /// the command failed, unless the vault locked meanwhile or a newer
    /// grant was earned.
    pub(crate) fn restore_reauth(&self, grant: Grant) {
        if let Some(mut slot) = self.grant()
            && slot.is_none()
            && grant.epoch == self.lock_epoch.load(Ordering::SeqCst)
        {
            *slot = Some(grant);
        }
    }

    /// Counts one wrong re-authentication and returns the count.
    pub(crate) fn reauth_failed(&self) -> u8 {
        self.failed_reauth
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1)
    }

    /// Records that `vault` was opened (or its key used again) at `now` /
    /// `wall`.
    pub(crate) fn note_vault_open(&self, vault: VaultId, now: Instant, wall: SystemTime) {
        if let Ok(mut open) = self.open_vaults.lock() {
            open.insert(
                vault,
                (
                    now + HARDWARE_VAULT_OPEN_FOR,
                    wall + HARDWARE_VAULT_OPEN_FOR,
                ),
            );
        }
    }

    /// Forgets the deadline of `vault` (it closed).
    pub(crate) fn note_vault_closed(&self, vault: VaultId) {
        if let Ok(mut open) = self.open_vaults.lock() {
            open.remove(&vault);
        }
    }

    /// How long `vault` stays open, if it is tracked: the shorter of the
    /// two clocks' remainders.
    pub(crate) fn vault_left(
        &self,
        vault: VaultId,
        now: Instant,
        wall: SystemTime,
    ) -> Option<Duration> {
        let open = self.open_vaults.lock().ok()?;
        let (mono, wall_deadline) = open.get(&vault)?;
        Some(
            mono.saturating_duration_since(now)
                .min(wall_deadline.duration_since(wall).unwrap_or(Duration::ZERO)),
        )
    }

    /// Whether `vault` has a deadline.
    pub(crate) fn vault_tracked(&self, vault: VaultId) -> bool {
        self.open_vaults
            .lock()
            .is_ok_and(|open| open.contains_key(&vault))
    }

    /// The tracked vaults whose deadline passed on either clock at `now` /
    /// `wall`. Nothing is removed: the caller forgets a vault with
    /// [`Self::note_vault_closed`] once it really closed.
    pub(crate) fn expired_vaults(&self, now: Instant, wall: SystemTime) -> Vec<VaultId> {
        let Ok(open) = self.open_vaults.lock() else {
            return Vec::new();
        };
        open.iter()
            .filter(|(_, (mono, wall_deadline))| *mono <= now || *wall_deadline <= wall)
            .map(|(vault, _)| *vault)
            .collect()
    }

    /// What follows every lock: a running device operation is cancelled and
    /// the grant, the failure count and the deadlines of open vaults are
    /// dropped.
    pub(crate) fn on_lock(&self) {
        self.cancel.cancel();
        // The epoch moves under the grant's lock, so a grant that is out
        // with a command cannot be put back after this lock.
        if let Some(mut grant) = self.grant() {
            self.lock_epoch.fetch_add(1, Ordering::SeqCst);
            *grant = None;
        }
        self.failed_reauth.store(0, Ordering::SeqCst);
        if let Ok(mut open) = self.open_vaults.lock() {
            open.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_errors_map_to_their_codes() {
        for (error, code) in [
            (
                PlatformError::Unavailable,
                ErrorCode::QuickUnlockUnavailable,
            ),
            (PlatformError::Cancelled, ErrorCode::Cancelled),
            (PlatformError::Failed, ErrorCode::Internal),
        ] {
            assert_eq!(IpcError::from(error).code, code);
        }
    }

    #[test]
    fn the_default_service_offers_nothing() {
        let service = QuickUnlockService::default();
        assert!(service.biometric().is_none());
        assert!(service.available_biometric().is_none());
        assert!(!service.hardware().is_supported());
        assert!(!service.reauth_granted());
        assert!(service.consume_reauth(|_| Ok::<(), ()>(())).is_none());
        assert_eq!(service.reauth_failed(), 1);
        assert_eq!(service.reauth_failed(), 2);
        let vault = VaultId::from_bytes([
            0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, 1, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .unwrap();
        let (now, wall) = (Instant::now(), SystemTime::now());
        service.note_vault_open(vault, now, wall);
        assert!(service.vault_tracked(vault));
        assert_eq!(
            service.vault_left(vault, now, wall),
            Some(HARDWARE_VAULT_OPEN_FOR)
        );
        assert_eq!(service.expired_vaults(now, wall), Vec::new());
        let step = HARDWARE_VAULT_OPEN_FOR + Duration::from_secs(1);
        // Either clock passing is enough (a suspend stops one of them).
        assert_eq!(service.expired_vaults(now + step, wall), vec![vault]);
        assert_eq!(service.expired_vaults(now, wall + step), vec![vault]);
        assert_eq!(
            service.vault_left(vault, now + step, wall),
            Some(Duration::ZERO)
        );
        assert_eq!(
            service.expired_vaults(now + step, wall),
            vec![vault],
            "nothing is removed until the close succeeded"
        );
        service.note_vault_closed(vault);
        assert!(!service.vault_tracked(vault));
        assert_eq!(service.vault_left(vault, now, wall), None);
        service.note_vault_open(vault, now, wall);
        service.on_lock();
        assert!(!service.vault_tracked(vault), "a lock clears them");
        assert_eq!(service.reauth_failed(), 1, "a lock resets the count");
        assert!(service.cancel_flag().is_cancelled(), "a lock cancels");
    }

    #[test]
    fn without_a_grant_nothing_runs() {
        let service = QuickUnlockService::default();
        // No grant: nothing runs.
        assert!(service.consume_reauth(|_| Ok::<(), ()>(())).is_none());
    }
}
