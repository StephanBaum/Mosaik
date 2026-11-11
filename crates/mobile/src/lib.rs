//! Mosaic mobile core (M10, ADR-0089): all logic of the Android and iOS apps
//! behind plain Rust types. The platform shells call [`Core`] through the
//! UniFFI crate `mosaic-mobile-ffi`; nothing here knows UniFFI.
//!
//! Normative specification: `docs/MOBILE.md`, `docs/CRYPTO_SPEC.md` §6.2,
//! §6.9 and §16. Mosaic is pre-audit.
//!
//! - One [`Core`] per process. Every method blocks; the platform calls it off
//!   the main thread.
//! - The mutex of a core is **never held** across Argon2id, the network, a
//!   [`SecureStorage`] callback or another core call. A long operation takes
//!   its session out (the core reports [`SessionState::Busy`]), works, and puts
//!   the result back. A `lock()` meanwhile sets a flag, and the session is
//!   locked again before it is stored.
//! - Secrets go in as `Vec<u8>` and come out only as [`SecretValue`].
//! - Errors are [`MobileError`]: unit variants, no message, no ID, no value.
#![forbid(unsafe_code)]
#![expect(
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    reason = "the API mirrors the UniFFI surface, which hands owned values over, and its records are plain data"
)]

pub mod autofill;
mod config;
mod error;
mod generator;
mod items;
mod join;
mod otp;
pub mod passkeys;
pub mod quick_unlock;
mod secret;
mod settings;
mod setup;
mod state;
mod storage;
mod sync;
#[cfg(feature = "test-hooks")]
pub mod test_hooks;
mod unlock;

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_sync::engine::OwnedSession;
use mosaic_sync::http::TlsTrust;
use mosaic_vault::session::{BootId, LockedSession, UnlockedSession};
use mosaic_vault::storage::{Error as StoreError, Store};

pub use config::{CoreConfig, CoreContext, Platform};
pub use error::{MobileError, Result, error_codes};
pub use generator::{GeneratedSecret, GeneratorMode, GeneratorOptions};
pub use items::{
    AttachmentView, FieldView, ItemDetail, ItemFilter, ItemKey, ItemKind, ItemSummary, LoginDraft,
    NoteDraft, OtpEdit, PasskeyView, TrashMode, UrlDraft, UrlView, VaultSummary,
};
pub use join::{
    HandoffInfo, HandoffStatus, JoinHostedRequest, JoinOutcome, SecondFactorMethods, ServerInfo,
};
pub use otp::{LumaFrame, OtpCode, OtpPreview, OtpScheme, OtpView};
pub use quick_unlock::{QuickUnlockEnrolled, QuickUnlockSetup};
pub use secret::SecretValue;
pub use settings::{MobileSettings, SettingsUpdate};
pub use state::{QuickUnlockAvailability, QuickUnlockPlatform, SessionState};
pub use storage::{
    SECURE_STORE_VERSION, SETTINGS_KEY_HINT, SecureStorage, StorageError, secret_key_name,
    secure_store_aad, sync_token_name,
};
pub use sync::{ResetConfirm, ResetOutcome, SyncOverview};

use state::{Inner, Phase, Slots, UnlockedInfo, UnlockedPhase};

/// The version of this library.
#[must_use]
pub const fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The clock of a core: the system clock plus an offset that only the test
/// hooks set.
#[derive(Default)]
pub(crate) struct CoreClock {
    offset_ms: AtomicI64,
}

impl CoreClock {
    /// Unix milliseconds.
    pub(crate) fn now_ms(&self) -> u64 {
        let system = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        system.saturating_add_signed(self.offset_ms.load(Ordering::SeqCst))
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn set_offset_ms(&self, offset: i64) {
        self.offset_ms.store(offset, Ordering::SeqCst);
    }
}

impl mosaic_sync::clock::Clock for CoreClock {
    fn now_ms(&self) -> u64 {
        Self::now_ms(self)
    }
}

/// The mobile core. See the [crate documentation](crate).
pub struct Core {
    pub(crate) config: CoreConfig,
    pub(crate) storage: Arc<dyn SecureStorage>,
    pub(crate) clock: CoreClock,
    pub(crate) inner: Mutex<Inner>,
    /// `lock()` was called while a long call held the session out.
    pub(crate) lock_requested: AtomicBool,
    /// Every secret value handed out; `lock()` wipes the live ones.
    issued: Mutex<Vec<Weak<SecretValue>>>,
    /// Milliseconds a long call sleeps before it takes the mutex again
    /// (test hooks only: it makes the races of the tests deterministic).
    #[cfg(feature = "test-hooks")]
    pub(crate) work_delay_ms: std::sync::atomic::AtomicU64,
}

/// Opens the store file and classifies it.
fn open_phase(config: &CoreConfig) -> Result<(Phase, bool)> {
    std::fs::create_dir_all(&config.store_dir).map_err(|_| MobileError::StoreUnavailable)?;
    let store = match Store::open(config.store_path()) {
        Ok(store) => store,
        Err(StoreError::StoreInUse) => return Ok((Phase::Closed, true)),
        Err(e) => return Err(e.into()),
    };
    let mut locked = LockedSession::open(store);
    if locked.has_account()? {
        adopt_boot(config, &mut locked);
        Ok((Phase::Locked(locked), false))
    } else {
        Ok((Phase::NoAccount(locked), false))
    }
}

/// CRYPTO_SPEC §6.9.2: a stored boot marker that equals this boot lets the
/// quick unlock apply. An empty or invalid boot ID never matches.
fn adopt_boot(config: &CoreConfig, locked: &mut LockedSession) {
    if let Ok(boot) = BootId::new(&config.boot_id) {
        let _ = locked.adopt_boot_marker(&boot);
    }
}

/// Sets the process-wide TLS trust (CRYPTO_SPEC §6.9.6).
fn init_tls(config: &CoreConfig) -> Result<()> {
    let trust = if config.trust_anchors_der.is_empty() {
        #[cfg(not(target_os = "android"))]
        {
            TlsTrust::Platform
        }
        #[cfg(target_os = "android")]
        {
            return Err(MobileError::Internal);
        }
    } else {
        TlsTrust::Anchors(
            config
                .trust_anchors_der
                .iter()
                .map(|der| mosaic_sync::http::CertificateDer::from(der.clone()))
                .collect(),
        )
    };
    mosaic_sync::http::set_tls_trust(trust).map_err(|_| MobileError::Internal)
}

impl Core {
    /// Opens the core: sets the TLS trust, opens the store (another process
    /// holding it is reported by [`Core::state`] as `StoreBusy`, not an error)
    /// and reads the settings file.
    ///
    /// # Errors
    /// `storeUnavailable` for a store that cannot be opened, `internal` for a
    /// TLS trust that cannot be set.
    pub fn open(config: CoreConfig, storage: Arc<dyn SecureStorage>) -> Result<Self> {
        init_tls(&config)?;
        let (phase, store_busy) = open_phase(&config)?;
        let settings = settings::SettingsGuard::load(config.settings_path());
        Ok(Self {
            config,
            storage,
            clock: CoreClock::default(),
            inner: Mutex::new(Inner {
                phase,
                store_busy,
                expired_alias: None,
                slots: Slots::default(),
                settings,
                last_sync_at_ms: None,
                last_sync_error: None,
            }),
            lock_requested: AtomicBool::new(false),
            issued: Mutex::new(Vec::new()),
            #[cfg(feature = "test-hooks")]
            work_delay_ms: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// The configuration this core was opened with.
    #[must_use]
    pub const fn config(&self) -> &CoreConfig {
        &self.config
    }

    pub(crate) fn guard(&self) -> Result<MutexGuard<'_, Inner>> {
        self.inner.lock().map_err(|_| MobileError::Internal)
    }

    /// Unix milliseconds of this core's clock.
    pub(crate) fn now_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    /// Unix seconds of this core's clock.
    pub(crate) fn now_s(&self) -> u64 {
        self.now_ms() / 1000
    }

    /// Sleeps the configured test delay (no-op in release builds).
    pub(crate) fn work_pause(&self) {
        #[cfg(feature = "test-hooks")]
        {
            let ms = self.work_delay_ms.load(Ordering::SeqCst);
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
        }
    }

    /// The extension context refuses the calls that need Argon2id, the
    /// network or a change (CRYPTO_SPEC §6.9.3).
    pub(crate) fn require_app(&self) -> Result<()> {
        match self.config.context {
            CoreContext::App => Ok(()),
            CoreContext::IosAutofillExtension => Err(MobileError::InvalidInput),
        }
    }

    /// Registers a secret value so `lock()` wipes it, and hands it out.
    pub(crate) fn issue(&self, value: SecretValue) -> Arc<SecretValue> {
        let value = Arc::new(value);
        if let Ok(mut issued) = self.issued.lock() {
            issued.retain(|w| w.strong_count() > 0);
            issued.push(Arc::downgrade(&value));
        }
        value
    }

    fn wipe_issued(&self) {
        let live: Vec<Arc<SecretValue>> = self
            .issued
            .lock()
            .map(|mut issued| {
                let live = issued.iter().filter_map(Weak::upgrade).collect();
                issued.clear();
                live
            })
            .unwrap_or_default();
        for value in live {
            value.wipe();
        }
    }

    /// The state of the core. Total: it never waits for a long call.
    #[must_use]
    pub fn state(&self) -> SessionState {
        let now = self.now_ms();
        let Ok(mut guard) = self.guard() else {
            return SessionState::StoreClosed;
        };
        let inner = &mut *guard;
        match &mut inner.phase {
            Phase::Closed => {
                if inner.store_busy {
                    SessionState::StoreBusy
                } else {
                    SessionState::StoreClosed
                }
            }
            Phase::NoAccount(_) => SessionState::NoAccount,
            Phase::Locked(locked) => {
                let email = locked.sync_config().ok().flatten().and_then(|c| c.email);
                let secret_key_stored = locked
                    .account_id()
                    .is_ok_and(|id| self.secret_key_stored(&id));
                let policy = inner.settings.quick_unlock_policy();
                let quick_unlock = quick_unlock::availability(
                    self.config.platform,
                    &policy,
                    now,
                    locked,
                    &mut inner.expired_alias,
                );
                SessionState::Locked {
                    email,
                    secret_key_stored,
                    quick_unlock,
                }
            }
            Phase::Unlocking | Phase::Busy => SessionState::Busy,
            Phase::Unlocked(unlocked) => SessionState::Unlocked {
                account_id: unlocked.info.account.to_string(),
                email: unlocked.info.email.clone(),
                quick_unlock_enrolled: unlocked.info.quick_unlock_enrolled,
            },
        }
    }

    /// Whether the Secret Key of `account` is in secure storage. A failing
    /// storage reads as "not stored" (the user types it).
    fn secret_key_stored(&self, account: &mosaic_core::AccountId) -> bool {
        matches!(
            storage::read(self.storage.as_ref(), &storage::secret_key_name(account)),
            Ok(Some(_))
        )
    }

    /// Locks. Drops every pending slot (except a pending join) and wipes every
    /// secret value handed out. Never waits for a long call: while one runs it
    /// only sets a flag, and the call locks again before it stores its result.
    pub fn lock(&self) {
        self.wipe_issued();
        let taken = {
            let Ok(mut guard) = self.guard() else {
                return;
            };
            guard.slots.clear_on_lock();
            guard.settings.locked();
            match std::mem::replace(&mut guard.phase, Phase::Busy) {
                Phase::Unlocked(unlocked) => Some(unlocked),
                Phase::Unlocking | Phase::Busy => {
                    self.lock_requested.store(true, Ordering::SeqCst);
                    guard.phase = Phase::Unlocking;
                    None
                }
                other => {
                    guard.phase = other;
                    None
                }
            }
        };
        if let Some(unlocked) = taken {
            self.finish_lock(*unlocked);
        }
    }

    /// Takes the session back from its holder and stores the locked phase.
    /// The holder's mutex is taken **without** the core's mutex (the order is
    /// always holder first, then core).
    fn finish_lock(&self, unlocked: UnlockedPhase) {
        // A session that cannot be taken back (it cannot happen: only a lock
        // takes it) closes the store.
        let phase = unlocked
            .owned
            .take()
            .map_or(Phase::Closed, |session| Phase::Locked(session.lock()));
        if let Ok(mut guard) = self.guard() {
            guard.phase = phase;
        }
    }

    /// Locks, then drops the store and its file lock (iOS in the background,
    /// the extension before it ends).
    pub fn close_store(&self) {
        self.lock();
        let Ok(mut guard) = self.guard() else {
            return;
        };
        if matches!(guard.phase, Phase::Locked(_) | Phase::NoAccount(_)) {
            guard.phase = Phase::Closed;
            guard.store_busy = false;
        }
    }

    /// Opens the store again after [`Core::close_store`].
    ///
    /// # Errors
    /// `storeUnavailable`.
    pub fn reopen_store(&self) -> Result<SessionState> {
        let closed = matches!(self.guard()?.phase, Phase::Closed);
        if closed {
            // The file lock is taken outside the mutex.
            let (phase, busy) = open_phase(&self.config)?;
            let mut guard = self.guard()?;
            if matches!(guard.phase, Phase::Closed) {
                guard.phase = phase;
                guard.store_busy = busy;
            }
        }
        Ok(self.state())
    }

    /// The unlocked session's handle, for one call. The mutex of the core is
    /// released before `f` runs.
    pub(crate) fn with_session<R>(
        &self,
        f: impl FnOnce(&mut UnlockedSession) -> Result<R>,
    ) -> Result<R> {
        use mosaic_sync::wire::SessionHandle as _;
        let owned = self.owned_session()?;
        owned.with_session_mut(f).ok_or(MobileError::Locked)?
    }

    /// A clone of the unlocked session's holder.
    pub(crate) fn owned_session(&self) -> Result<OwnedSession> {
        let guard = self.guard()?;
        match &guard.phase {
            Phase::Unlocked(unlocked) => Ok(unlocked.owned.clone()),
            Phase::Locked(_) => Err(MobileError::Locked),
            Phase::NoAccount(_) => Err(MobileError::NoAccount),
            Phase::Closed => Err(if guard.store_busy {
                MobileError::StoreInUse
            } else {
                MobileError::StoreUnavailable
            }),
            Phase::Unlocking | Phase::Busy => Err(MobileError::Busy),
        }
    }

    /// The account of the locked or unlocked phase.
    pub(crate) fn account_id(&self) -> Result<mosaic_core::AccountId> {
        let guard = self.guard()?;
        match &guard.phase {
            Phase::Unlocked(unlocked) => Ok(unlocked.info.account),
            Phase::Locked(locked) => Ok(locked.account_id()?),
            Phase::NoAccount(_) => Err(MobileError::NoAccount),
            Phase::Closed => Err(MobileError::StoreUnavailable),
            Phase::Unlocking | Phase::Busy => Err(MobileError::Busy),
        }
    }

    /// Takes the unlocked phase out for a long call; the core reports `Busy`
    /// until [`Core::restore_unlocked`].
    pub(crate) fn take_unlocked(&self) -> Result<Box<UnlockedPhase>> {
        let mut guard = self.guard()?;
        match std::mem::replace(&mut guard.phase, Phase::Busy) {
            Phase::Unlocked(unlocked) => Ok(unlocked),
            other => {
                let error = match &other {
                    Phase::Locked(_) => MobileError::Locked,
                    Phase::NoAccount(_) => MobileError::NoAccount,
                    Phase::Closed => MobileError::StoreUnavailable,
                    _ => MobileError::Busy,
                };
                guard.phase = other;
                Err(error)
            }
        }
    }

    /// Puts a taken unlocked phase back; a `lock()` that came meanwhile wins.
    ///
    /// # Errors
    /// `locked` when the phase was locked instead.
    pub(crate) fn restore_unlocked(&self, unlocked: Box<UnlockedPhase>) -> Result<()> {
        if self.lock_requested.swap(false, Ordering::SeqCst) {
            {
                let mut guard = self.guard()?;
                guard.slots.clear_on_lock();
                guard.settings.locked();
            }
            self.wipe_issued();
            self.finish_lock(*unlocked);
            return Err(MobileError::Locked);
        }
        self.guard()?.phase = Phase::Unlocked(unlocked);
        Ok(())
    }

    /// The cached facts of the unlocked phase, changed under the mutex.
    pub(crate) fn update_info(&self, f: impl FnOnce(&mut UnlockedInfo)) {
        if let Ok(mut guard) = self.guard()
            && let Phase::Unlocked(unlocked) = &mut guard.phase
        {
            f(&mut unlocked.info);
        }
    }
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Core>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_is_send_and_sync() {
        const fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Core>();
    }

    #[test]
    fn secret_value_debug_is_redacted() {
        let value = SecretValue::from_text("hunter2");
        assert_eq!(format!("{value:?}"), "SecretValue([REDACTED])");
        assert!(!format!("{value:?}").contains("hunter2"));
    }

    #[test]
    fn version_is_the_crate_version() {
        assert_eq!(core_version(), env!("CARGO_PKG_VERSION"));
    }
}
