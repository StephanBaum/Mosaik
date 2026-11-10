//! The opt-in breach check (M7b, CRYPTO_SPEC §12, ADR-0065): `breach_scan_start`
//! and `breach_scan_cancel`.
//!
//! A scan hashes the passwords under the session lock, then lets go of the
//! lock: the requests run on their own thread with a plan that holds only
//! hash suffixes (zeroized when it is dropped). The session is locked again
//! for the one moment that stores the answers. Locking the vault, cancelling
//! and quitting set the scan's cancel flag, which the pause between requests
//! watches, so a scan ends within about 50 ms of a lock (a request already on
//! the wire ends within the transport's 10 s timeout).
//!
//! Nothing starts a scan but the user's "Check now" and, only when the
//! setting is `weekly`, an unlock followed by 60 seconds: with the setting
//! `off` (the default) the code never builds a client.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use mosaic_core::Timestamp;
use mosaic_insights::Candidate;
use mosaic_insights::breach::{self, BreachPlan, RangeError, RangeSource, RunHooks};
use mosaic_vault::session::{ItemFilter, ScanError, ScanSummary};
use tauri::State;

use super::insights::now;
use crate::contract::credential_rotation::BreachScanError;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::settings::BreachMode;
use crate::state::{AppState, Backend};

/// An answer older than this many days is asked again (CRYPTO_SPEC §12).
pub const RECHECK_DAYS: u32 = 30;
/// After an unlock, a `weekly` scan waits this long.
pub const WEEKLY_DELAY: Duration = Duration::from_secs(60);
/// A `weekly` scan runs when the last one ended this long ago.
const WEEK_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// How often the pause between requests looks at the cancel flag.
const PAUSE_SLICE: Duration = Duration::from_millis(50);

/// Makes the source of one scan: the HTTPS client in the app, a fake in
/// tests.
pub type SourceFactory =
    Box<dyn Fn() -> Result<Box<dyn RangeSource + Send>, RangeError> + Send + Sync>;

/// The production source: the range API over TLS 1.3, directly.
fn production_source() -> Result<Box<dyn RangeSource + Send>, RangeError> {
    Ok(Box::new(mosaic_sync::http::pwned::RangeClient::new()?))
}

/// The scan's state: whether one runs, its cancel flag and where answers
/// come from.
pub struct BreachService {
    running: AtomicBool,
    /// The flag of the current (or last) scan; every scan has its own, so a
    /// lock that ended an old scan cannot end a newer one.
    cancel: Mutex<Arc<AtomicBool>>,
    source: Mutex<SourceFactory>,
}

impl Default for BreachService {
    fn default() -> Self {
        Self {
            running: AtomicBool::new(false),
            cancel: Mutex::new(Arc::new(AtomicBool::new(false))),
            source: Mutex::new(Box::new(production_source)),
        }
    }
}

impl BreachService {
    /// Whether a scan runs now.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Marks a scan as started and hands out its fresh cancel flag; `None`
    /// if one runs already.
    fn begin(&self) -> Option<Arc<AtomicBool>> {
        if self.running.swap(true, Ordering::AcqRel) {
            return None;
        }
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut current) = self.cancel.lock() {
            *current = Arc::clone(&flag);
        } else {
            self.running.store(false, Ordering::Release);
            return None;
        }
        Some(flag)
    }

    fn end(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Stops the running scan, if any: at the next look of its pause, or
    /// before its next request. Called by cancel, lock and exit.
    pub fn cancel(&self) {
        if let Ok(current) = self.cancel.lock() {
            current.store(true, Ordering::Release);
        }
    }

    /// Replaces where answers come from (tests).
    pub fn set_source(&self, factory: SourceFactory) {
        if let Ok(mut source) = self.source.lock() {
            *source = factory;
        }
    }

    fn source(&self) -> IpcResult<Box<dyn RangeSource + Send>> {
        let factory = self.source.lock().map_err(|_| IpcError::INTERNAL)?;
        (*factory)().map_err(|_| IpcError::INTERNAL)
    }
}

/// Ends the "running" state however the scan ends, unless it is defused
/// (the scan's thread then owns the state).
struct RunGuard<'a> {
    service: &'a BreachService,
    armed: bool,
}

impl<'a> RunGuard<'a> {
    const fn new(service: &'a BreachService) -> Self {
        Self {
            service,
            armed: true,
        }
    }

    fn defuse(mut self) {
        self.armed = false;
    }
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.service.end();
        }
    }
}

/// A scan that is prepared and not run yet: the plan (hash suffixes) and the
/// scan's cancel flag. `Debug` shows no plan.
pub struct ScanJob {
    plan: BreachPlan,
    started_at: Timestamp,
    cancel: Arc<AtomicBool>,
}

impl core::fmt::Debug for ScanJob {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ScanJob(..)")
    }
}

impl ScanJob {
    /// How many requests the scan makes.
    #[must_use]
    pub fn prefix_count(&self) -> usize {
        self.plan.prefix_count()
    }
}

/// `breach_scan_start`: returns at once; the scan runs on its own thread and
/// reports through `breachScanProgress` and `breachScanFinished`.
#[tauri::command]
pub async fn breach_scan_start(state: State<'_, AppState>) -> IpcResult<()> {
    let weak = state.downgrade();
    state.run(move |b| b.breach_scan_start(&weak)).await
}

/// `breach_scan_cancel`.
#[tauri::command]
pub async fn breach_scan_cancel(state: State<'_, AppState>) -> IpcResult<()> {
    state.run(Backend::breach_scan_cancel).await
}

/// A uniform number below `bound` from the OS random number generator.
fn random_below(bound: u32) -> u32 {
    if bound <= 1 {
        return 0;
    }
    // Rejection sampling: no modulo bias.
    let zone = u32::MAX - (u32::MAX % bound);
    loop {
        let mut bytes = [0u8; 4];
        if getrandom::fill(&mut bytes).is_err() {
            // The OS generator failed: a fixed order is acceptable for the
            // shuffle and the pause (neither is a secret).
            return 0;
        }
        let value = u32::from_le_bytes(bytes);
        if value < zone {
            return value % bound;
        }
    }
}

/// Waits `ms` milliseconds, looking at `cancel` every [`PAUSE_SLICE`]; `false`
/// as soon as it is set.
#[must_use]
pub fn pause_on(cancel: &AtomicBool, ms: u32) -> bool {
    let deadline = Instant::now() + Duration::from_millis(u64::from(ms));
    loop {
        if cancel.load(Ordering::Acquire) {
            return false;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        std::thread::sleep(left.min(PAUSE_SLICE));
    }
}

fn scan_error(error: RangeError) -> BreachScanError {
    wire_scan_error(ScanError::from(error))
}

pub(crate) fn wire_scan_error(error: ScanError) -> BreachScanError {
    match error {
        ScanError::Offline => BreachScanError::Offline,
        ScanError::RateLimited => BreachScanError::RateLimited,
        ScanError::Protocol => BreachScanError::Protocol,
        ScanError::Cancelled => BreachScanError::Cancelled,
    }
}

impl Backend {
    /// Starts a scan of the passwords the cache does not cover, on a thread.
    ///
    /// # Errors
    /// `locked`; `breachConsentRequired` unless the setting is `manual` or
    /// `weekly` (the consent is recorded with it); `breachRunning`;
    /// `internal` if the client or the thread cannot start.
    pub fn breach_scan_start(&self, weak: &Weak<Self>) -> IpcResult<()> {
        // From here on a scan is "running": the guard ends that on an error,
        // the thread's run ends it otherwise.
        let job = self.breach_scan_prepare(now())?;
        let running = RunGuard::new(&self.breach);
        let mut source = self.breach.source()?;
        let weak = Weak::clone(weak);
        std::thread::Builder::new()
            .name("breach-scan".into())
            .spawn(move || {
                if let Some(backend) = weak.upgrade() {
                    let cancel = Arc::clone(&job.cancel);
                    backend.breach_scan_run(job, &mut *source, &mut |ms| pause_on(&cancel, ms));
                }
            })
            .map_err(|_| IpcError::INTERNAL)?;
        running.defuse();
        Ok(())
    }

    /// Checks the consent, plans the scan and marks it as running.
    ///
    /// # Errors
    /// As [`Backend::breach_scan_start`], except for the client and thread.
    pub fn breach_scan_prepare(&self, now: Timestamp) -> IpcResult<ScanJob> {
        self.touch();
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let settings = self.settings_snapshot();
        if settings.breach_check == BreachMode::Off || settings.breach_consent_at.is_none() {
            return Err(IpcError::new(ErrorCode::BreachConsentRequired));
        }
        let cancel = self
            .breach
            .begin()
            .ok_or(IpcError::new(ErrorCode::BreachRunning))?;
        let running = RunGuard::new(&self.breach);
        let states = session.breach_states()?;
        let entries = session.items(&ItemFilter::default());
        let plan = breach::plan(
            entries.iter().map(|e| Candidate {
                vault_id: e.vault_id,
                item_id: e.item_id,
                item: e.item,
            }),
            &states,
            now,
            RECHECK_DAYS,
        )
        .map_err(|_| IpcError::INTERNAL)?;
        running.defuse();
        Ok(ScanJob {
            plan,
            started_at: now,
            cancel,
        })
    }

    /// Runs a prepared scan to its end and stores the answers. `pause` is the
    /// wait between requests (it returns `false` when the scan must stop).
    /// The "running" state ends here.
    pub fn breach_scan_run(
        &self,
        job: ScanJob,
        source: &mut dyn RangeSource,
        pause: &mut dyn FnMut(u32) -> bool,
    ) {
        let _running = RunGuard::new(&self.breach);
        let clock = Instant::now();
        let ScanJob {
            plan,
            started_at,
            cancel,
        } = job;
        let events = self.events.clone();
        let mut rng = random_below;
        let mut progress = |done: u32, total: u32| {
            events.emit(&AppEvent::BreachScanProgress { done, total });
        };
        let (results, error) = breach::run(
            plan,
            source,
            &mut RunHooks {
                rng: &mut rng,
                pause,
                progress: &mut progress,
                cancel: &cancel,
            },
        );
        let checked = u32::try_from(results.len()).unwrap_or(u32::MAX);
        let found = u32::try_from(results.iter().filter(|r| r.found).count()).unwrap_or(u32::MAX);
        let summary = ScanSummary {
            started_at,
            // The scan's own start plus the time it took, so the clock the
            // job was planned with is the one the summary keeps.
            finished_at: Some(Timestamp::from_unix_millis(
                started_at
                    .as_unix_millis()
                    .saturating_add(i64::try_from(clock.elapsed().as_millis()).unwrap_or(i64::MAX)),
            )),
            checked,
            error: error.map(ScanError::from),
        };
        // After a lock there is no session to store into: the answers are
        // dropped (they are only a cache) and the next scan asks again.
        if let Ok(mut slot) = self.slot()
            && let Ok(session) = slot.unlocked_mut()
        {
            let _ = session.record_breach_results(&results, summary);
        }
        self.emit(&AppEvent::BreachScanFinished {
            checked,
            found,
            error: error.map(scan_error),
        });
    }

    /// Stops the running scan (it reports `cancelled`).
    ///
    /// # Errors
    /// `locked`.
    pub fn breach_scan_cancel(&self) -> IpcResult<()> {
        self.touch();
        self.slot()?.unlocked()?;
        self.breach.cancel();
        Ok(())
    }

    /// Whether a `weekly` scan is due at `now`: the setting is `weekly` with
    /// its consent, the vault is unlocked, no scan runs and the last one
    /// ended more than a week ago (or never did).
    #[must_use]
    pub fn breach_weekly_due(&self, now: Timestamp) -> bool {
        let settings = self.settings_snapshot();
        if settings.breach_check != BreachMode::Weekly
            || settings.breach_consent_at.is_none()
            || self.breach.is_running()
        {
            return false;
        }
        let Ok(slot) = self.slot() else {
            return false;
        };
        let Ok(session) = slot.unlocked() else {
            return false;
        };
        let last = session
            .breach_scan()
            .ok()
            .flatten()
            .and_then(|s| s.finished_at);
        last.is_none_or(|t| now.as_unix_millis().saturating_sub(t.as_unix_millis()) >= WEEK_MS)
    }

    /// The unlock hook's work: after [`WEEKLY_DELAY`], start a scan if one is
    /// due. A lock in between ends it quietly.
    pub fn breach_weekly_after_unlock(weak: Weak<Self>) {
        let _ = std::thread::Builder::new()
            .name("breach-weekly".into())
            .spawn(move || {
                std::thread::sleep(WEEKLY_DELAY);
                if let Some(backend) = weak.upgrade()
                    && backend.breach_weekly_due(now())
                {
                    let _ = backend.breach_scan_start(&weak);
                }
            });
    }

    /// What the queue and the dashboard show about the check.
    pub(crate) fn breach_status(
        &self,
        session: &mosaic_vault::session::UnlockedSession,
    ) -> crate::contract::credential_rotation::BreachStatus {
        let settings = self.settings_snapshot();
        let scan = session.breach_scan().ok().flatten();
        let running = self.breach.is_running();
        crate::contract::credential_rotation::BreachStatus {
            mode: settings.breach_check,
            consent_at: settings.breach_consent_at,
            last_finished_at: scan
                .and_then(|s| s.finished_at)
                .map(Timestamp::as_unix_millis),
            running,
            last_error: if running {
                None
            } else {
                scan.and_then(|s| s.error).map(wire_scan_error)
            },
        }
    }
}

/// Used by the lock path: the scan must stop at once.
impl Backend {
    pub(crate) fn breach_after_lock(&self) {
        self.breach.cancel();
    }
}
