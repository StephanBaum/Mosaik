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
//!   locked again before it is stored. The flag is read and cleared **under
//!   the mutex**, together with the phase, so a lock is never lost (G1-1).
//! - Secrets go in as `Vec<u8>` and come out only as [`SecretValue`].
//! - Errors are [`MobileError`]: unit variants, no message, no ID, no value.
#![forbid(unsafe_code)]
#![expect(
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    reason = "the API mirrors the UniFFI surface, which hands owned values over, and its records are plain data"
)]

// The test hooks must never reach a release library (ADR-0089, G1-7): this
// refuses the build, in addition to `scripts/mobile/check-no-test-hooks.sh`
// that looks at the result. Debug builds (the host tests, the JVM unit tests,
// the instrumented tests and the XCTest builds) may enable the feature.
#[cfg(all(feature = "test-hooks", not(debug_assertions)))]
compile_error!("the `test-hooks` feature of mosaic-mobile is for debug builds only");

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

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_sync::engine::OwnedSession;
use mosaic_sync::http::TlsTrust;
use mosaic_vault::session::{BootId, LockedSession, UnlockedSession};
use mosaic_vault::storage::{Error as StoreError, Store};

pub use autofill::{
    AndroidCaller, AssetLinksStatus, Attr, FieldNode, FieldRefusal, FieldSelection, FillCandidate,
    FillCandidates, FillMatch, FillTarget, FillTargetKind, FillValues, OriginWarning,
    ServiceIdentifier, ServiceIdentifierKind,
};
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
pub use passkeys::{
    AndroidPasskeyRequest, IosAssertRequest, IosAssertion, IosRegisterRequest, IosRegistration,
    PasskeyCandidate, PasskeyCreateSession, PasskeyIdentity, PasskeySession, PasswordIdentity,
};
pub use quick_unlock::{QuickUnlockEnrolled, QuickUnlockSetup};
pub use secret::SecretValue;
pub use settings::{MobileSettings, SettingsUpdate};
pub use setup::SetupCode;
pub use state::{QuickUnlockAvailability, QuickUnlockPlatform, SessionState};
pub use storage::{
    SECURE_STORE_VERSION, SETTINGS_KEY_HINT, SecureStorage, StorageError, secret_key_name,
    secure_store_aad, sync_token_name,
};
pub use sync::{ResetConfirm, ResetOutcome, SyncOverview};

use state::{Inner, Pending, Phase, Slots, UnlockedInfo, UnlockedPhase};

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
    /// Counts locks. A secret value is handed out only if no lock happened
    /// since its call started (G1-6).
    generation: AtomicU64,
    /// Every secret value handed out; `lock()` wipes the live ones.
    issued: Mutex<Vec<Weak<SecretValue>>>,
    /// The asset-links fetch and its cache (ADR-0092, ADR-0093).
    pub(crate) assetlinks: autofill::AssetLinks,
    /// Milliseconds a long call sleeps before it takes the mutex again
    /// (test hooks only: it makes the races of the tests deterministic).
    #[cfg(feature = "test-hooks")]
    pub(crate) work_delay_ms: std::sync::atomic::AtomicU64,
}

/// Whether the Secret Key and the session token of the account are in secure
/// storage: asked with `contains`, so no secret is read to test presence.
fn stored_flags(phase: &Phase, storage: &dyn SecureStorage) -> (bool, bool) {
    let Phase::Locked(locked) = phase else {
        return (false, false);
    };
    locked.account_id().map_or((false, false), |account| {
        (
            storage
                .contains(&secret_key_name(&account))
                .unwrap_or(false),
            storage
                .contains(&sync_token_name(&account))
                .unwrap_or(false),
        )
    })
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
        // The platform is asked before the mutex exists, never under it.
        let (secret_key_stored, signed_in) = stored_flags(&phase, storage.as_ref());
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
                secret_key_stored,
                signed_in,
                pending: Pending::default(),
            }),
            generation: AtomicU64::new(0),
            issued: Mutex::new(Vec::new()),
            assetlinks: autofill::AssetLinks::new(),
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
    #[cfg_attr(
        not(feature = "test-hooks"),
        expect(clippy::unused_self, reason = "the delay exists in test builds only")
    )]
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

    /// Counts a lock (called with the core's mutex held).
    pub(crate) fn generation_bump(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// The lock counter, read when a call starts (see [`Core::issue_gated`]).
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Registers a secret value so `lock()` wipes it, and hands it out. For
    /// values that do not come from the unlocked session (the generator, a
    /// scanned setup code).
    pub(crate) fn issue(&self, value: SecretValue) -> Arc<SecretValue> {
        let value = Arc::new(value);
        if let Ok(mut issued) = self.issued.lock() {
            issued.retain(|w| w.strong_count() > 0);
            issued.push(Arc::downgrade(&value));
        }
        value
    }

    /// Registers and hands out a value read from the unlocked session, under
    /// the core's mutex. If a lock happened since the call began (`generation`
    /// is the counter then) or one is pending, the value is wiped and the call
    /// ends with `locked`: a `lock()` between the read and the hand-out cannot
    /// leave a live secret behind (G1-6). `lock()` bumps the counter under the
    /// same mutex and wipes the registered values after it, so either it sees
    /// this value or this call sees the lock.
    ///
    /// # Errors
    /// `locked`, `internal`.
    pub(crate) fn issue_gated(
        &self,
        value: SecretValue,
        generation: u64,
    ) -> Result<Arc<SecretValue>> {
        let guard = self.guard()?;
        if self.generation.load(Ordering::SeqCst) != generation || guard.pending.lock {
            value.wipe();
            return Err(MobileError::Locked);
        }
        Ok(self.issue(value))
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
        inner.drop_expired_join(now);
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
                let secret_key_stored = inner.secret_key_stored;
                // A boot marker that could not be deleted at the failed-attempt
                // limit is tried again here (G1-13).
                let _ = locked.enforce_boot_marker_limit();
                let quick_unlock = quick_unlock::availability(
                    self.config.platform,
                    &inner.settings,
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

    /// Records where the core stored or deleted the Secret Key and the session
    /// token, for `state()` and `sync_overview()`. `None` keeps a value.
    pub(crate) fn note_stored(&self, secret_key: Option<bool>, token: Option<bool>) {
        if let Ok(mut guard) = self.guard() {
            if let Some(v) = secret_key {
                guard.secret_key_stored = v;
            }
            if let Some(v) = token {
                guard.signed_in = v;
            }
        }
    }

    /// Locks. Drops every pending slot (except a pending join) and wipes every
    /// secret value handed out. Never waits for a long call: while one runs it
    /// only sets a flag, and the call locks again before it stores its result.
    pub fn lock(&self) {
        self.lock_with(false, false, false);
    }

    /// Locks and refuses quick unlock until the next master-password unlock
    /// (the re-authentication limit, ADR-0028 item 8; the boot marker goes too,
    /// CRYPTO_SPEC §6.9.2).
    pub(crate) fn lock_requiring_password(&self) {
        self.lock_with(true, false, false);
    }

    /// The one place a lock is decided. `close` (a `close_store()`) is decided
    /// in the same critical section as the lock: a close that finds a phase
    /// held out arms `pending.close` next to `pending.lock`, so whoever
    /// settles that phase locks it and then closes it.
    fn lock_with(&self, require_password: bool, keep_join: bool, close: bool) {
        let now = self.now_ms();
        let taken = {
            let Ok(mut guard) = self.guard() else {
                return;
            };
            let inner = &mut *guard;
            // The counter and the slots change under the mutex; the values
            // handed out are wiped after it (see `issue_gated`).
            self.generation.fetch_add(1, Ordering::SeqCst);
            inner.slots.clear_on_lock(keep_join);
            inner.settings.locked();
            if close {
                inner.drop_expired_join(now);
            }
            match std::mem::replace(&mut inner.phase, Phase::Busy) {
                Phase::Unlocked(unlocked) => {
                    // `finish_lock` closes the locked phase it stores.
                    inner.pending.close |= close;
                    Some(unlocked)
                }
                Phase::Unlocking | Phase::Busy => {
                    inner.pending.lock = true;
                    inner.pending.strict |= require_password;
                    inner.pending.close |= close;
                    inner.phase = Phase::Unlocking;
                    None
                }
                // A strict lock that finds the session locked already (a plain
                // `lock()` won a race with the re-authentication limit) still
                // requires the password and takes the boot marker (N4).
                Phase::Locked(mut locked) if require_password => {
                    locked.require_password();
                    inner.phase = Phase::Locked(locked);
                    inner.settle_fresh_phase();
                    None
                }
                // A settled phase is closed here, in the deciding section; the
                // dropped store releases its file lock. No request stays armed
                // on it (a join start may have kept one).
                Phase::Locked(_) | Phase::NoAccount(_) if close => {
                    inner.phase = Phase::Closed;
                    inner.store_busy = false;
                    inner.pending.lock = false;
                    inner.pending.strict = false;
                    inner.pending.close = false;
                    None
                }
                // A reopen is opening the store: it closes what it opened
                // (G3i-2).
                Phase::Closed => {
                    inner.pending.close |= close && inner.pending.reopening;
                    inner.phase = Phase::Closed;
                    None
                }
                // A lock on a phase that is settled is met by it: a request a
                // join start kept for a `finish_join` that never came is
                // cleared and a pending close applied. Any other phase keeps
                // its requests.
                other => {
                    inner.phase = other;
                    inner.settle_abandoned_join();
                    None
                }
            }
        };
        self.wipe_issued();
        if let Some(unlocked) = taken {
            self.finish_lock(*unlocked, require_password);
        }
    }

    /// Takes the session back from its holder and stores the locked phase.
    /// The holder's mutex is taken **without** the core's mutex (the order is
    /// always holder first, then core). In the same critical section as the
    /// phase it consumes every lock request that came while the session was
    /// out (the lock is done; a strict one still sets the password limit) and
    /// closes the store if a close is pending.
    fn finish_lock(&self, unlocked: UnlockedPhase, require_password: bool) {
        // A session that cannot be taken back (it cannot happen: only a lock
        // takes it) closes the store.
        let owned = unlocked.owned.take();
        let mut phase = owned.map_or(Phase::Closed, |session| {
            Phase::Locked(if require_password {
                session.lock_requiring_password()
            } else {
                session.lock()
            })
        });
        if let Ok(mut guard) = self.guard() {
            let late_strict = guard.pending.strict;
            guard.pending.lock = false;
            guard.pending.strict = false;
            if late_strict && let Phase::Locked(locked) = &mut phase {
                locked.require_password();
            }
            // A session that could not be taken back leaves the phase closed
            // already: a pending close has nothing left to close.
            let closed = matches!(phase, Phase::Closed);
            guard.phase = phase;
            if closed {
                guard.pending.close = false;
            }
            guard.close_if_requested();
        }
    }

    /// Locks, then drops the store and its file lock (iOS in the background,
    /// the extension before it ends). An unexpired pending join survives this
    /// (it holds no store; the Safari handoff needs it), unlike a `lock()`.
    pub fn close_store(&self) {
        self.lock_with(false, true, true);
    }

    /// Opens the store again after [`Core::close_store`].
    ///
    /// # Errors
    /// `storeUnavailable`.
    pub fn reopen_store(&self) -> Result<SessionState> {
        let closed = {
            let mut guard = self.guard()?;
            let closed = matches!(guard.phase, Phase::Closed);
            if closed {
                // A reopen that is already running does the opening; this call
                // only reports the state.
                if guard.pending.reopening {
                    drop(guard);
                    return Ok(self.state());
                }
                // A close that came before this open does not apply to it.
                guard.pending.close = false;
                guard.pending.reopening = true;
            }
            closed
        };
        if closed {
            // The file lock is taken outside the mutex.
            let (phase, busy) = match open_phase(&self.config) {
                Ok(opened) => opened,
                Err(e) => {
                    if let Ok(mut guard) = self.guard() {
                        guard.pending.reopening = false;
                        guard.pending.close = false;
                    }
                    return Err(e);
                }
            };
            // The presence flags are asked outside the mutex too: a core that
            // opened while another process held the store has none (N5).
            let (secret_key_stored, signed_in) = stored_flags(&phase, self.storage.as_ref());
            let mut guard = self.guard()?;
            guard.pending.reopening = false;
            // A `close_store()` during the open wins: the opened store is
            // dropped, and its file lock released, here (G3i-2).
            let close = std::mem::take(&mut guard.pending.close);
            if matches!(guard.phase, Phase::Closed) && !close {
                guard.phase = phase;
                guard.store_busy = busy;
                guard.secret_key_stored = secret_key_stored;
                guard.signed_in = signed_in;
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

    /// Puts a taken unlocked phase back; a `lock()` or `close_store()` that
    /// came meanwhile wins (a close locks first, then closes).
    ///
    /// # Errors
    /// `locked` when the phase was locked instead.
    pub(crate) fn restore_unlocked(&self, unlocked: Box<UnlockedPhase>) -> Result<()> {
        // The requests are read and cleared **under the mutex**, in the same step
        // that decides the phase: a `lock()` either ran before (the flag is set
        // and wins here) or runs after (it finds the unlocked phase). A flag
        // read before the mutex lost a lock that came in between (G1-1).
        let mut guard = self.guard()?;
        if let Some(strict) = guard.pending.take_lock() {
            self.generation.fetch_add(1, Ordering::SeqCst);
            guard.slots.clear_on_lock(false);
            guard.settings.locked();
            // The holder's mutex is taken without the core's.
            drop(guard);
            self.wipe_issued();
            self.finish_lock(*unlocked, strict);
            return Err(MobileError::Locked);
        }
        guard.phase = Phase::Unlocked(unlocked);
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
