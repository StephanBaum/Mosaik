//! The sync thread: one engine over one transport, run until the app locks,
//! the user disconnects or the service restarts.
//!
//! Loop: [`SyncEngine::sync_once`], publish the state, then wait for the
//! next reason to run: the backend changed (a long poll in short slices for
//! the server, a directory scan for a folder), a local change was poked
//! (debounced), the user asked ("Sync now") or the backoff ran out. A halt
//! (rollback, duplicated install) stops the automatic runs until the user
//! acknowledges it. Sync work is not user activity: nothing here touches the
//! auto-lock timer.
//!
//! The transport is a type parameter: the hosted and the folder transport
//! each get their own monomorphized loop, so no `dyn Transport` (and no
//! forwarding impl) is needed.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use mosaic_core::{InstallId, VaultId};
use mosaic_sync::clock::{Clock, SystemClock};
use mosaic_sync::engine::consts::{BACKOFF_INITIAL_MS, BACKOFF_MAX_MS, LOCAL_CHANGE_DEBOUNCE_MS};
use mosaic_sync::engine::{Backoff, SessionAccess, SyncEngine, SyncError, SyncReport, SyncWarning};
use mosaic_sync::http::StoredToken;
use mosaic_sync::transport::{ChangeToken, Transport, TransportError};
use mosaic_vault::storage::{SyncHalt, VaultFlag};

use super::access::BackendSession;
use super::errors::{sync_code, transport_code};
use super::{Cmd, SyncService};
use crate::contract::errors::ErrorCode;
use crate::contract::events::{AppEvent, SyncStatus};
use crate::contract::ids::WireVaultId;
use crate::contract::sync::SyncWarningView;
use crate::state::Backend;

/// How long one wait for a backend change lasts before the loop looks at its
/// commands again (a local change waits at most this long to be seen).
const HOSTED_SLICE: Duration = Duration::from_secs(5);
const FOLDER_SLICE: Duration = Duration::from_secs(2);
/// A run without any reason, so a missed change or a stuck queue heals.
const IDLE_RUN: Duration = Duration::from_mins(15);
/// The wait after a failure that is not the backend's (a store error).
const SESSION_RETRY: Duration = Duration::from_mins(1);

/// What one sync thread knows about itself.
pub(super) struct Ctx {
    pub(super) weak: Weak<Backend>,
    pub(super) generation: u64,
    pub(super) hosted: bool,
    /// The session token this thread runs with (hosted): a 401 deletes
    /// exactly this one from the keystore, never a newer one.
    pub(super) token: std::sync::OnceLock<StoredToken>,
}

impl Ctx {
    pub(super) fn session(&self) -> BackendSession {
        BackendSession::new(self.weak.clone(), self.generation)
    }

    /// The backend while this generation is current.
    fn backend(&self) -> Option<Arc<Backend>> {
        self.session().current()
    }
}

enum Next {
    /// Fine; wait for the next reason.
    Idle,
    /// Run again no sooner than this.
    Retry(Duration),
    /// Automatic sync is stopped until the user acknowledges.
    Halted,
    /// The thread ends (locked, signed out).
    Stop,
}

/// What the loop remembers between runs.
struct State<T: Transport> {
    engine: Option<SyncEngine<T>>,
    connect_backoff: Backoff,
    change: Option<ChangeToken>,
    waiters: Vec<SyncSender<()>>,
    due: bool,
    forced: bool,
    debounce: Option<Instant>,
    halted: bool,
    retry_at: Option<Instant>,
    last_run: Instant,
}

impl<T: Transport> State<T> {
    fn new() -> Self {
        Self {
            engine: None,
            connect_backoff: Backoff::default(),
            change: None,
            waiters: Vec::new(),
            due: true,
            forced: false,
            debounce: None,
            halted: false,
            retry_at: None,
            last_run: Instant::now(),
        }
    }

    fn command(&mut self, command: Cmd) {
        match command {
            Cmd::Poke => {
                self.debounce.get_or_insert_with(|| {
                    Instant::now() + Duration::from_millis(LOCAL_CHANGE_DEBOUNCE_MS)
                });
            }
            Cmd::SyncNow(done) => {
                self.waiters.push(done);
                self.due = true;
                self.forced = true;
            }
            Cmd::Acknowledged => {
                self.halted = false;
                self.due = true;
                self.forced = true;
            }
        }
    }

    /// Takes every queued command; `false` when the sender is gone.
    fn drain(&mut self, rx: &Receiver<Cmd>) -> bool {
        loop {
            match rx.try_recv() {
                Ok(command) => self.command(command),
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
    }

    /// Waits for a command for up to `wait`; `false` when the sender is gone.
    fn pause(&mut self, rx: &Receiver<Cmd>, wait: Duration) -> bool {
        match rx.recv_timeout(wait) {
            Ok(command) => {
                self.command(command);
                true
            }
            Err(RecvTimeoutError::Timeout) => true,
            Err(RecvTimeoutError::Disconnected) => false,
        }
    }

    fn timers(&mut self) {
        if self.debounce.is_some_and(|at| Instant::now() >= at) {
            self.debounce = None;
            self.due = true;
        }
        if self.retry_at.is_some_and(|at| Instant::now() >= at) {
            self.retry_at = None;
            self.due = true;
        }
        if !self.due && self.last_run.elapsed() >= IDLE_RUN {
            self.due = true;
        }
    }

    fn waiting_out(&self) -> bool {
        self.retry_at.is_some_and(|at| Instant::now() < at)
    }

    /// Whether a run starts now: wanted, and not held back by a halt or a
    /// backoff (unless the user asked).
    fn should_run(&self) -> bool {
        self.due && (self.forced || (!self.halted && !self.waiting_out()))
    }

    fn finish_waiters(&mut self) {
        for waiter in self.waiters.drain(..) {
            let _ = waiter.try_send(());
        }
    }

    /// How long to wait for a command when there is no backend to poll.
    fn idle_wait(&self, slice: Duration) -> Duration {
        let now = Instant::now();
        [self.retry_at, self.debounce]
            .into_iter()
            .flatten()
            .map(|at| at.saturating_duration_since(now))
            .min()
            .map_or(slice, |wait| wait.min(slice))
            .max(Duration::from_millis(50))
    }
}

/// Runs the loop. `make` builds the engine (it may talk to the network); it
/// is called again after a failure and when the user asks to sync now while
/// the engine backs off.
pub(super) fn run<T: Transport>(
    ctx: &Ctx,
    mut make: impl FnMut() -> Result<SyncEngine<T>, TransportError>,
    rx: &Receiver<Cmd>,
) {
    let mut access = ctx.session();
    let mut st = State::<T>::new();
    let slice = if ctx.hosted {
        HOSTED_SLICE
    } else {
        FOLDER_SLICE
    };
    loop {
        let Some(backend) = ctx.backend() else {
            st.finish_waiters();
            return;
        };
        if !st.drain(rx) {
            st.finish_waiters();
            return;
        }
        st.timers();
        if st.should_run() {
            if run_and_publish(&backend, ctx, &mut st, &mut make, &mut access) {
                return;
            }
            continue;
        }
        drop(backend);
        // Nothing to run now: wait for a backend change or a command.
        let keep_going =
            if st.halted || st.waiting_out() || st.engine.is_none() || st.debounce.is_some() {
                let wait = st.idle_wait(slice);
                st.pause(rx, wait)
            } else {
                poll(ctx, &mut st, rx, slice)
            };
        if !keep_going {
            st.finish_waiters();
            return;
        }
    }
}

/// One run and what follows it. `true` when the thread ends.
fn run_and_publish<T: Transport>(
    backend: &Arc<Backend>,
    ctx: &Ctx,
    st: &mut State<T>,
    make: &mut impl FnMut() -> Result<SyncEngine<T>, TransportError>,
    access: &mut BackendSession,
) -> bool {
    st.due = false;
    if st.forced {
        // "Sync now" does not wait for a backoff: a fresh engine forgets it.
        if st
            .engine
            .as_ref()
            .is_some_and(|e| e.next_retry_at().is_some())
        {
            st.engine = None;
        }
        st.connect_backoff.reset();
        st.retry_at = None;
    }
    st.forced = false;
    backend
        .sync
        .set_status(backend, ctx.generation, SyncStatus::Syncing, None);
    let result = run_once(make, &mut st.engine, access, &mut st.connect_backoff);
    if !ctx.hosted {
        // A folder run writes to the folder itself (its checkpoint, every
        // run). Those writes are not news: take the folder's state as it is
        // now, or the next poll sees the run's own write as a foreign
        // change and starts another run, without end.
        st.change = st
            .engine
            .as_mut()
            .and_then(|e| e.transport_mut().wait_for_change(None, Duration::ZERO).ok());
    }
    let next = after_run(ctx, result, &mut st.engine);
    st.last_run = Instant::now();
    st.finish_waiters();
    match next {
        Next::Idle => {
            st.retry_at = None;
            st.halted = false;
        }
        Next::Retry(wait) => st.retry_at = Some(Instant::now() + wait),
        Next::Halted => st.halted = true,
        Next::Stop => return true,
    }
    false
}

/// Waits for the backend to change (a slice), or for a command when it
/// cannot be asked. `false` when the thread ends.
fn poll<T: Transport>(ctx: &Ctx, st: &mut State<T>, rx: &Receiver<Cmd>, slice: Duration) -> bool {
    let Some(engine) = st.engine.as_mut() else {
        return true;
    };
    match engine
        .transport_mut()
        .wait_for_change(st.change.as_ref(), slice)
    {
        Ok(token) => {
            if st.change.as_ref().is_some_and(|old| *old != token) {
                st.due = true;
            }
            st.change = Some(token);
            true
        }
        Err(TransportError::Locked) => false,
        Err(TransportError::Unauthorized) => {
            if let Some(backend) = ctx.backend() {
                backend.sync_signed_out(ctx.generation, ctx.token.get());
            }
            false
        }
        // The next run finds out what is wrong and backs off.
        Err(_) => {
            let alive = st.pause(rx, slice);
            st.due = true;
            alive
        }
    }
}

fn run_once<T: Transport>(
    make: &mut impl FnMut() -> Result<SyncEngine<T>, TransportError>,
    engine: &mut Option<SyncEngine<T>>,
    access: &mut BackendSession,
    connect_backoff: &mut Backoff,
) -> Result<SyncReport, SyncError> {
    let clock = SystemClock;
    if engine.is_none() {
        let now = clock.now_ms();
        if let Some(at) = connect_backoff.retry_at
            && now < at
        {
            return Err(SyncError::Backoff { retry_at_ms: at });
        }
        match make() {
            Ok(built) => {
                connect_backoff.reset();
                *engine = Some(built);
            }
            Err(error) => {
                if matches!(
                    error,
                    TransportError::Unreachable
                        | TransportError::Timeout
                        | TransportError::RateLimited { .. }
                        | TransportError::Io
                        | TransportError::FolderUnavailable
                ) {
                    connect_backoff.fail(now, 0, 1000, BACKOFF_INITIAL_MS, BACKOFF_MAX_MS);
                }
                return Err(SyncError::from(error));
            }
        }
    }
    match engine.as_mut() {
        Some(engine) => engine.sync_once(access, &clock),
        None => Err(SyncError::Locked),
    }
}

/// Publishes what a run found and decides what the loop does next.
///
/// The network call took time: the backend and the generation are fetched
/// again, and a thread of an older generation stops without writing.
fn after_run<T: Transport>(
    ctx: &Ctx,
    result: Result<SyncReport, SyncError>,
    engine: &mut Option<SyncEngine<T>>,
) -> Next {
    let Some(backend) = ctx.backend() else {
        return Next::Stop;
    };
    let backend = &backend;
    match result {
        Ok(report) => {
            publish_report(backend, ctx, &report, engine);
            Next::Idle
        }
        Err(SyncError::Locked) => Next::Stop,
        Err(SyncError::RollbackDetected | SyncError::IdentityDuplicated) => {
            let warnings = warnings_of(ctx, &[], 0, engine);
            backend
                .sync
                .finish_halted(backend, ctx.generation, warnings);
            Next::Halted
        }
        Err(SyncError::Backoff { retry_at_ms }) => {
            let wait = until(retry_at_ms);
            backend
                .sync
                .set_status(backend, ctx.generation, SyncStatus::Offline, None);
            Next::Retry(wait)
        }
        Err(SyncError::Transport(TransportError::Unauthorized)) => {
            backend.sync_signed_out(ctx.generation, ctx.token.get());
            Next::Stop
        }
        Err(SyncError::Transport(error)) => {
            let wait = engine
                .as_ref()
                .and_then(SyncEngine::next_retry_at)
                .map_or(Duration::from_millis(BACKOFF_INITIAL_MS), until);
            let offline = matches!(
                error,
                TransportError::Unreachable
                    | TransportError::Timeout
                    | TransportError::RateLimited { .. }
            ) || (ctx.hosted && error == TransportError::Io);
            if offline {
                backend
                    .sync
                    .set_status(backend, ctx.generation, SyncStatus::Offline, None);
            } else {
                let code = transport_code(error);
                backend
                    .sync
                    .set_status(backend, ctx.generation, SyncStatus::Error, Some(code));
            }
            Next::Retry(wait)
        }
        Err(other) => {
            let code: ErrorCode = sync_code(&other);
            backend
                .sync
                .set_status(backend, ctx.generation, SyncStatus::Error, Some(code));
            Next::Retry(SESSION_RETRY)
        }
    }
}

/// The time from now to `at_ms` (zero if it passed).
fn until(at_ms: u64) -> Duration {
    Duration::from_millis(at_ms.saturating_sub(SystemClock.now_ms()))
}

/// A finished run: the warnings, the state, the events.
fn publish_report<T: Transport>(
    backend: &Arc<Backend>,
    ctx: &Ctx,
    report: &SyncReport,
    engine: &mut Option<SyncEngine<T>>,
) {
    let warnings = warnings_of(
        ctx,
        &report.warnings,
        report.attachments_heal_failed,
        engine,
    );
    // A thread of an older generation tells nobody anything.
    if !backend.sync.finish_ok(backend, ctx.generation, warnings) {
        return;
    }
    if !report.changed_vaults.is_empty() {
        backend.emit(&AppEvent::SyncDataChanged {
            vault_ids: report
                .changed_vaults
                .iter()
                .map(|v| WireVaultId::from(*v))
                .collect(),
        });
    }
    note_sharing(backend, ctx, report);
    let conflicts = report.merged.saturating_add(report.split_copies);
    if conflicts > 0 || report.restored_fields > 0 || report.replaced_unreadable > 0 {
        backend.emit(&AppEvent::SyncAttention {
            conflicts,
            restored: report.restored_fields,
            replaced_unreadable: report.replaced_unreadable,
        });
    }
}

/// The sharing part of a finished run: the invites and emergency requests
/// that wait for the user (an event when they changed) and the attachments
/// a key change of a shared vault could not protect.
fn note_sharing(backend: &Arc<Backend>, ctx: &Ctx, report: &SyncReport) {
    for vault in &report.rotated_vaults {
        let count = report
            .unreachable_attachments
            .iter()
            .filter(|(v, _)| v == vault)
            .count();
        backend
            .sync
            .set_unprotected(*vault, u32::try_from(count).unwrap_or(u32::MAX));
    }
    if backend.sync.set_sharing_counts(
        ctx.generation,
        report.invites_incoming,
        report.emergency_requests,
    ) {
        backend.emit(&AppEvent::SharingChanged {
            invites: report.invites_incoming,
            emergency_requests: report.emergency_requests,
        });
    }
}

// ---- warnings -----------------------------------------------------------------

fn millis(ms: u64) -> i64 {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

/// What the store remembers of the backend's misbehaviour, and the names of
/// the vaults.
#[derive(Default)]
struct Findings {
    found: Option<mosaic_vault::session::SyncFindings>,
    vault_names: BTreeMap<VaultId, Option<String>>,
}

fn read_findings(ctx: &Ctx) -> Findings {
    let mut access = ctx.session();
    access
        .with(|s| Findings {
            found: s.sync_findings().ok(),
            vault_names: s
                .vaults()
                .iter()
                .map(|v| (v.vault_id, v.name.map(str::to_owned)))
                .collect(),
        })
        .unwrap_or_default()
}

/// The names of the devices, when a warning is about one.
fn device_names<T: Transport>(
    reported: &[SyncWarning],
    findings: &Findings,
    engine: &mut Option<SyncEngine<T>>,
) -> BTreeMap<InstallId, String> {
    let needed = reported.iter().any(|w| {
        matches!(
            w,
            SyncWarning::FreezeSuspected { .. }
                | SyncWarning::MissingFiles { .. }
                | SyncWarning::NoCommonView { .. }
        )
    }) || findings
        .found
        .as_ref()
        .is_some_and(|f| !f.no_common_view.is_empty());
    if !needed {
        return BTreeMap::new();
    }
    engine
        .as_mut()
        .and_then(|e| e.transport_mut().devices().ok())
        .map(|devices| {
            devices
                .into_iter()
                .filter_map(|d| d.name.map(|n| (d.device_id, n)))
                .collect()
        })
        .unwrap_or_default()
}

/// The warnings of a run, plus the persisted findings (which stay until the
/// user acknowledges them, whether or not this run repeated them).
fn warnings_of<T: Transport>(
    ctx: &Ctx,
    reported: &[SyncWarning],
    heal_failed: u32,
    engine: &mut Option<SyncEngine<T>>,
) -> Vec<SyncWarningView> {
    let findings = read_findings(ctx);
    let names = device_names(reported, &findings, engine);
    let device = |id: &InstallId| names.get(id).cloned();
    let vault = |id: &VaultId| findings.vault_names.get(id).cloned().flatten();
    let mut out: Vec<SyncWarningView> = reported
        .iter()
        .filter_map(|warning| match warning {
            SyncWarning::FreezeSuspected {
                device: d,
                since_ms,
            } => Some(SyncWarningView::FreezeSuspected {
                device_name: device(d),
                since: millis(*since_ms),
            }),
            SyncWarning::MissingFiles {
                device: d,
                since_ms,
            } => Some(SyncWarningView::MissingFiles {
                device_name: device(d),
                since: millis(*since_ms),
            }),
            SyncWarning::NoCommonView {
                device: d,
                since_ms,
                ..
            } => Some(SyncWarningView::NoCommonView {
                device_name: device(d),
                since: millis(*since_ms),
            }),
            SyncWarning::RollbackDetected => Some(SyncWarningView::RollbackDetected),
            SyncWarning::IdentityDuplicated => Some(SyncWarningView::IdentityDuplicated),
            SyncWarning::ServerInconsistent { vault: v } => {
                Some(SyncWarningView::ServerInconsistent {
                    vault_name: vault(v),
                })
            }
            SyncWarning::ServerRollback { vault: v } => Some(SyncWarningView::ServerRollback {
                vault_name: vault(v),
            }),
            SyncWarning::Undecryptable { count } => {
                Some(SyncWarningView::Undecryptable { count: *count })
            }
            SyncWarning::HardwareVaultsLocal { count } => {
                Some(SyncWarningView::HardwareVaultsLocal { count: *count })
            }
            SyncWarning::RotationBlocked { vault: v } => Some(SyncWarningView::RotationBlocked {
                vault_name: vault(v),
            }),
            // A warning a later version adds: not shown.
            _ => None,
        })
        .collect();
    if heal_failed > 0 {
        out.push(SyncWarningView::AttachmentsNotRepaired { count: heal_failed });
    }
    if let Some(found) = &findings.found {
        match found.halt {
            Some(SyncHalt::Rollback) => out.push(SyncWarningView::RollbackDetected),
            Some(SyncHalt::Identity) => out.push(SyncWarningView::IdentityDuplicated),
            None => {}
        }
        out.extend(found.vault_flags.iter().map(|(v, flag)| match flag {
            VaultFlag::ServerInconsistent => SyncWarningView::ServerInconsistent {
                vault_name: vault(v),
            },
            VaultFlag::ServerRolledBack => SyncWarningView::ServerRollback {
                vault_name: vault(v),
            },
        }));
        out.extend(found.no_common_view.iter().map(|(d, _, since_ms)| {
            SyncWarningView::NoCommonView {
                device_name: device(d),
                since: millis(*since_ms),
            }
        }));
    }
    let mut unique: Vec<SyncWarningView> = Vec::new();
    for view in out {
        if !unique.contains(&view) {
            unique.push(view);
        }
    }
    unique
}

impl SyncService {
    /// Sets the status shown to the UI and tells the webview if it changed.
    ///
    /// Every write of the thread checks that `generation` is still current,
    /// under the lock of the view: a thread that a lock, a disconnect or a
    /// restart replaced writes nothing and tells nobody.
    pub(super) fn set_status(
        &self,
        backend: &Backend,
        generation: u64,
        status: SyncStatus,
        error: Option<ErrorCode>,
    ) {
        if self.update_view(|view| {
            if !self.is_current(generation) {
                return false;
            }
            let changed = view.status != status || view.error != error;
            view.status = status;
            view.error = error;
            changed
        }) {
            backend.emit_sync_state();
        }
    }

    pub(crate) fn finish_ok(
        &self,
        backend: &Backend,
        generation: u64,
        warnings: Vec<SyncWarningView>,
    ) -> bool {
        let now = SystemClock.now_ms();
        let wrote = self.update_view(|view| {
            if !self.is_current(generation) {
                return false;
            }
            view.status = SyncStatus::UpToDate;
            view.error = None;
            view.last_synced_at = Some(millis(now));
            view.warnings = warnings;
            true
        });
        if wrote {
            backend.emit_sync_state();
        }
        wrote
    }

    pub(super) fn finish_halted(
        &self,
        backend: &Backend,
        generation: u64,
        warnings: Vec<SyncWarningView>,
    ) {
        if self.update_view(|view| {
            if !self.is_current(generation) {
                return false;
            }
            view.status = SyncStatus::Error;
            view.error = None;
            view.warnings = warnings;
            true
        }) {
            backend.emit_sync_state();
        }
    }
}
