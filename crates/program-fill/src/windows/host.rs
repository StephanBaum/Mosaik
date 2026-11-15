//! `WindowsHost`: one worker thread that owns the only UI Automation client,
//! watches focus through the shim, and runs captures and fills (ADR-0164).
//!
//! The worker lives in the COM multithreaded apartment (`WindowsProbe::new`).
//! Calls from other threads go over a channel; one that is not answered in time
//! abandons the worker (its later events are dropped) and starts a new one.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mosaic_winshim::FocusSubscription;

use super::driver::WindowsDriver;
use super::{CaptureError, Captured, WindowsProbe};
use crate::detect::{FieldRole, classify, lone_text};
use crate::fill::{Continuation, FillOutcome, FillPlan, FillSecrets, continues, run_fill};
use crate::host::{
    FieldOffer, FillRequest, Host, HostEvent, HostSink, SystemClock, Token, monotonic_ms,
};
use crate::verify::{CONTINUATION_TTL_MS, ProbeError, Rect};

/// Focus events are acted on after this long without another.
pub const DEBOUNCE_MS: u64 = 150;
/// A call to the worker that is not answered in this time abandons it.
pub const HOST_CALL_TIMEOUT_MS: u64 = 5_000;
/// A fill types, waits for modifier keys and re-verifies in batches, and ends
/// by its own pick expiry (30 s), so its call may take that long and a little
/// more. The worker is never abandoned mid-fill earlier than this.
pub const FILL_CALL_TIMEOUT_MS: u64 = 35_000;
/// How many identified programs the worker remembers.
pub const IDENTITY_CACHE: usize = 16;
/// How long a starting worker may take to be ready.
const START_TIMEOUT_MS: u64 = 10_000;
/// A value on the focus channel that is not a process ID: wakes the worker so
/// it looks at its command queue.
pub(super) const WAKE: u32 = u32::MAX;
/// The focus channel's size (events beyond it are dropped, never blocking).
const PING_QUEUE: usize = 64;

enum Command {
    CaptureNow(Sender<Result<HostEvent, ProbeError>>),
    LoneText(Token, Sender<Result<bool, ProbeError>>),
    Fill(FillRequest, Sender<Result<FillOutcome, ProbeError>>),
    Refocus(Token, Sender<Result<(), ProbeError>>),
    FieldRect(Token, Sender<Result<Option<Rect>, ProbeError>>),
    Arm(Token, Continuation),
    WindowClear(isize, Rect, Sender<Result<bool, ProbeError>>),
}

/// The handles to a running worker.
struct Worker {
    cmd: Sender<Command>,
    wake: SyncSender<u32>,
    epoch: u64,
}

struct State {
    worker: Option<Worker>,
    sink: Option<HostSink>,
}

/// The Windows [`Host`].
pub struct WindowsHost {
    state: Mutex<State>,
    /// The epoch whose events are delivered; a worker with another one is
    /// abandoned or stopped, and its events are dropped.
    current: Arc<AtomicU64>,
}

impl Default for WindowsHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsHost {
    /// A host that is not watching yet; call [`Host::start`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                worker: None,
                sink: None,
            }),
            current: Arc::new(AtomicU64::new(0)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Starts a worker and waits until its client runs.
    fn spawn(&self, sink: &HostSink) -> Result<Worker, ProbeError> {
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ping_tx, ping_rx) = mpsc::sync_channel(PING_QUEUE);
        let (ready_tx, ready_rx) = mpsc::channel();
        let wake = ping_tx.clone();
        let context = Context {
            epoch,
            current: Arc::clone(&self.current),
            sink: Arc::clone(sink),
        };
        std::thread::Builder::new()
            .name("mosaic-program-fill".to_owned())
            .spawn(move || worker_main(&context, &cmd_rx, ping_tx, ping_rx, &ready_tx))
            .map_err(|_| ProbeError::Os)?;
        match ready_rx.recv_timeout(Duration::from_millis(START_TIMEOUT_MS)) {
            Ok(Ok(())) => Ok(Worker {
                cmd: cmd_tx,
                wake,
                epoch,
            }),
            Ok(Err(err)) => Err(err),
            Err(_) => {
                // A worker that never became ready is abandoned.
                self.current.fetch_add(1, Ordering::SeqCst);
                Err(ProbeError::Timeout)
            }
        }
    }

    /// Drops the worker of `epoch` (it hung) and starts a new one when the
    /// host is still started.
    fn abandon(&self, epoch: u64) {
        let sink = {
            let mut state = self.lock();
            if state.worker.as_ref().is_none_or(|w| w.epoch != epoch) {
                return;
            }
            self.current.fetch_add(1, Ordering::SeqCst);
            state.worker = None;
            state.sink.clone()
        };
        if let Some(sink) = sink
            && let Ok(worker) = self.spawn(&sink)
        {
            let mut state = self.lock();
            if state.worker.is_none() && state.sink.is_some() {
                state.worker = Some(worker);
            }
        }
    }

    /// Sends a command and waits for its answer.
    fn call<T>(
        &self,
        timeout_ms: u64,
        make: impl FnOnce(Sender<T>) -> Command,
    ) -> Result<T, ProbeError> {
        let (cmd, wake, epoch) = {
            let state = self.lock();
            let worker = state.worker.as_ref().ok_or(ProbeError::Gone)?;
            (worker.cmd.clone(), worker.wake.clone(), worker.epoch)
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        cmd.send(make(reply_tx)).map_err(|_| ProbeError::Gone)?;
        // A full channel means the worker is busy and will look anyway.
        wake.try_send(WAKE).ok();
        match reply_rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(value) => Ok(value),
            Err(RecvTimeoutError::Timeout) => {
                self.abandon(epoch);
                Err(ProbeError::Timeout)
            }
            Err(RecvTimeoutError::Disconnected) => Err(ProbeError::Gone),
        }
    }
}

impl Host for WindowsHost {
    fn start(&self, sink: HostSink) -> Result<(), ProbeError> {
        {
            let mut state = self.lock();
            state.sink = Some(Arc::clone(&sink));
            if state.worker.is_some() {
                return Ok(());
            }
        }
        let worker = self.spawn(&sink)?;
        let mut state = self.lock();
        if state.worker.is_none() && state.sink.is_some() {
            state.worker = Some(worker);
        } else {
            // Stopped while starting: the new worker must not survive.
            self.current.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    fn stop(&self) {
        let worker = {
            let mut state = self.lock();
            state.sink = None;
            state.worker.take()
        };
        self.current.fetch_add(1, Ordering::SeqCst);
        if let Some(worker) = worker {
            // Dropping the command sender ends the loop; the wake makes it look.
            let Worker { cmd, wake, .. } = worker;
            drop(cmd);
            wake.try_send(WAKE).ok();
        }
    }

    fn capture_now(&self) -> Result<HostEvent, ProbeError> {
        self.call(HOST_CALL_TIMEOUT_MS, Command::CaptureNow)?
    }

    fn lone_text(&self, token: Token) -> Result<bool, ProbeError> {
        self.call(HOST_CALL_TIMEOUT_MS, |tx| Command::LoneText(token, tx))?
    }

    fn fill(&self, request: FillRequest) -> Result<FillOutcome, ProbeError> {
        self.call(FILL_CALL_TIMEOUT_MS, |tx| Command::Fill(request, tx))?
    }

    fn refocus(&self, token: Token) -> Result<(), ProbeError> {
        self.call(HOST_CALL_TIMEOUT_MS, |tx| Command::Refocus(token, tx))?
    }

    fn field_rect(&self, token: Token) -> Result<Option<Rect>, ProbeError> {
        self.call(HOST_CALL_TIMEOUT_MS, |tx| Command::FieldRect(token, tx))?
    }

    fn arm_continuation(&self, token: Token, continuation: Continuation) {
        let (cmd, wake) = {
            let state = self.lock();
            match state.worker.as_ref() {
                Some(w) => (w.cmd.clone(), w.wake.clone()),
                None => return,
            }
        };
        if cmd.send(Command::Arm(token, continuation)).is_ok() {
            wake.try_send(WAKE).ok();
        }
    }

    fn window_clear(&self, own_hwnd: isize, rect: Rect) -> Result<bool, ProbeError> {
        self.call(HOST_CALL_TIMEOUT_MS, |tx| {
            Command::WindowClear(own_hwnd, rect, tx)
        })?
    }
}

/// What a worker needs to deliver events.
struct Context {
    epoch: u64,
    current: Arc<AtomicU64>,
    sink: HostSink,
}

impl Context {
    fn emit(&self, event: HostEvent) {
        if self.current.load(Ordering::SeqCst) == self.epoch {
            (self.sink)(event);
        }
    }
}

/// The capture the worker keeps for the field on offer.
struct Current {
    token: Token,
    captured: Captured,
}

/// The first step of a two-step login, kept until the second or expiry. Its
/// capture holds the process handle, so the process ID cannot be reused.
struct Armed {
    continuation: Continuation,
    captured: Captured,
}

struct WorkerState {
    probe: WindowsProbe,
    pings: Receiver<u32>,
    current: Option<Current>,
    armed: Option<Armed>,
    next_token: Token,
}

fn worker_main(
    context: &Context,
    commands: &Receiver<Command>,
    ping_tx: SyncSender<u32>,
    ping_rx: Receiver<u32>,
    ready: &Sender<Result<(), ProbeError>>,
) {
    let probe = match WindowsProbe::new() {
        Ok(probe) => probe,
        Err(err) => {
            ready.send(Err(err)).ok();
            return;
        }
    };
    // Declared after the probe, so it is dropped (and the handler removed)
    // first, on this thread.
    let subscription: FocusSubscription = match probe.uia().subscribe_focus(ping_tx) {
        Ok(subscription) => subscription,
        Err(err) => {
            ready.send(Err(super::uia::probe_error(err))).ok();
            return;
        }
    };
    ready.send(Ok(())).ok();
    let own_pid = probe.own_pid();
    let mut worker = WorkerState {
        probe,
        pings: ping_rx,
        current: None,
        armed: None,
        next_token: 0,
    };
    let mut pending: Option<Instant> = None;
    loop {
        // Commands first: a wake on the focus channel means one is waiting.
        loop {
            match commands.try_recv() {
                Ok(command) => worker.handle(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    drop(subscription);
                    return;
                }
            }
        }
        let wait =
            pending.map(|since| Duration::from_millis(DEBOUNCE_MS).saturating_sub(since.elapsed()));
        let received = match wait {
            Some(wait) => worker.pings.recv_timeout(wait),
            None => worker
                .pings
                .recv()
                .map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(WAKE) => {}
            // Events of Mosaic's own windows never offer anything.
            Ok(pid) if pid == own_pid => {}
            Ok(_) => pending = Some(Instant::now()),
            Err(RecvTimeoutError::Timeout) => {
                pending = None;
                let event = worker.focus_event();
                context.emit(event);
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

impl WorkerState {
    fn token(&mut self) -> Token {
        self.next_token += 1;
        self.next_token
    }

    fn expire_armed(&mut self, now: u64) {
        if self
            .armed
            .as_ref()
            .is_some_and(|a| now.saturating_sub(a.continuation.picked_at_ms) >= CONTINUATION_TTL_MS)
        {
            self.armed = None;
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::CaptureNow(reply) => {
                reply.send(self.capture_event()).ok();
            }
            Command::LoneText(token, reply) => {
                reply.send(self.lone_text(token)).ok();
            }
            Command::Fill(request, reply) => {
                reply.send(self.fill(&request)).ok();
            }
            Command::Refocus(token, reply) => {
                reply.send(self.refocus(token)).ok();
            }
            Command::FieldRect(token, reply) => {
                reply.send(self.field_rect(token)).ok();
            }
            Command::Arm(token, continuation) => self.arm(token, continuation),
            Command::WindowClear(hwnd, rect, reply) => {
                reply.send(self.probe.window_clear(hwnd, rect)).ok();
            }
        }
    }

    /// What a focus change means: an offer, a continuation, or nothing.
    fn focus_event(&mut self) -> HostEvent {
        if let Ok(event @ (HostEvent::Field(_) | HostEvent::Continue { .. })) = self.capture_event()
        {
            return event;
        }
        self.current = None;
        HostEvent::Gone
    }

    /// Captures the focused field now. A login field becomes the current
    /// capture (the previous one is dropped, its process handle with it).
    fn capture_event(&mut self) -> Result<HostEvent, ProbeError> {
        let now = monotonic_ms();
        self.expire_armed(now);
        let captured = match self.probe.capture_focused(now) {
            Ok(captured) => captured,
            Err(CaptureError::NotAField) => {
                return Ok(HostEvent::NoField {
                    near: self.probe.focused_rect(),
                });
            }
            Err(CaptureError::Refused(refusal)) => {
                return Ok(HostEvent::Refused {
                    refusal,
                    near: self.probe.focused_rect(),
                });
            }
            Err(CaptureError::Probe(err)) => return Err(err),
        };
        let near = Some(captured.field.rect);
        let Ok((edits, focused)) = self.probe.field_set(&captured) else {
            return Ok(HostEvent::NoField { near });
        };
        let Some(role) = classify(&edits, focused) else {
            return Ok(HostEvent::NoField { near });
        };
        let token = self.token();
        let continues_login = self.armed.as_ref().is_some_and(|armed| {
            continues(
                &armed.continuation,
                &captured.field,
                captured.session.root,
                captured.session.window_pid,
                now,
            ) && armed.captured.identity.pid == captured.identity.pid
        });
        if continues_login {
            // Used once: the first step's handle is released with it.
            self.armed = None;
            self.current = Some(Current { token, captured });
            return Ok(HostEvent::Continue { token });
        }
        let offer = FieldOffer {
            token,
            identity: captured.identity.clone(),
            role,
            rect: captured.field.rect,
            target: captured.session.target.clone(),
        };
        self.current = Some(Current { token, captured });
        Ok(HostEvent::Field(offer))
    }

    fn current_for(&mut self, token: Token) -> Result<&mut Current, ProbeError> {
        self.current
            .as_mut()
            .filter(|c| c.token == token)
            .ok_or(ProbeError::Gone)
    }

    fn lone_text(&mut self, token: Token) -> Result<bool, ProbeError> {
        let root = self.current_for(token)?.captured.session.root;
        let target = self.current_for(token)?.captured.session.target.clone();
        let edits = self.probe.window_edits(root)?;
        Ok(lone_text(&edits, &target))
    }

    fn refocus(&mut self, token: Token) -> Result<(), ProbeError> {
        let pings = &self.pings;
        let probe = &self.probe;
        let current = self
            .current
            .as_ref()
            .filter(|c| c.token == token)
            .ok_or(ProbeError::Gone)?;
        let driver = WindowsDriver {
            uia: probe.uia(),
            pings,
        };
        crate::fill::FillDriver::focus(&driver, &current.captured.session)
    }

    fn field_rect(&mut self, token: Token) -> Result<Option<Rect>, ProbeError> {
        let current = self
            .current
            .as_ref()
            .filter(|c| c.token == token)
            .ok_or(ProbeError::Gone)?;
        match self
            .probe
            .uia()
            .refresh(&current.captured.session.held.element)
        {
            Ok(fresh) => {
                let [left, top, right, bottom] = fresh.facts().rect;
                Ok(Some(Rect {
                    left,
                    top,
                    right,
                    bottom,
                }))
            }
            Err(mosaic_winshim::ShimError::NotFound) => Ok(None),
            Err(err) => Err(super::uia::probe_error(err)),
        }
    }

    /// Keeps the filled capture for the second step. Too late (the capture is
    /// already replaced) arms nothing.
    fn arm(&mut self, token: Token, continuation: Continuation) {
        if self.current.as_ref().is_some_and(|c| c.token == token)
            && let Some(current) = self.current.take()
        {
            self.armed = Some(Armed {
                continuation,
                captured: current.captured,
            });
        }
    }

    fn fill(&mut self, request: &FillRequest) -> Result<FillOutcome, ProbeError> {
        let probe = &self.probe;
        let pings = &self.pings;
        let current = self
            .current
            .as_mut()
            .filter(|c| c.token == request.token)
            .ok_or(ProbeError::Gone)?;
        current.captured.session.picked_at_ms = request.picked_at_ms;
        current.captured.session.continuation = request.continuation;
        // The role is read again from the window as it is now.
        let (edits, focused) = probe.field_set_attached(&mut current.captured)?;
        let role = classify(&edits, focused).ok_or(ProbeError::Gone)?;
        let id_of = |index: usize| edits.get(index).map(|e| e.runtime_id.clone());
        let own = edits.get(focused).map(|e| e.runtime_id.clone());
        let plan = match role {
            FieldRole::Password { username } => FillPlan {
                username: username.and_then(id_of),
                password: own,
                submit: request.submit,
            },
            FieldRole::Username { password } => FillPlan {
                username: own,
                password: id_of(password),
                submit: request.submit,
            },
            FieldRole::Text => FillPlan {
                username: own,
                password: None,
                submit: request.submit,
            },
        };
        let driver = WindowsDriver {
            uia: probe.uia(),
            pings,
        };
        let secrets = FillSecrets {
            username: request.username.as_deref(),
            password: request.password.as_ref(),
        };
        Ok(run_fill(
            probe,
            &driver,
            &mut current.captured.session,
            &plan,
            &secrets,
            &SystemClock,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_core::SecretString;

    fn sink() -> HostSink {
        Arc::new(|_| {})
    }

    #[test]
    fn a_host_starts_and_stops_without_a_desktop_focus() {
        let host = WindowsHost::new();
        host.start(sink()).unwrap();
        // Whatever has focus on this desktop, the answer is a value.
        match host.capture_now() {
            Ok(HostEvent::Field(_) | HostEvent::Refused { .. } | HostEvent::NoField { .. })
            | Err(_) => {}
            Ok(other) => panic!("unexpected {other:?}"),
        }
        host.stop();
    }

    #[test]
    fn calls_after_stop_fail_cleanly() {
        let host = WindowsHost::new();
        let request = || FillRequest {
            token: 1,
            username: Some("u".to_owned()),
            password: Some(SecretString::from_str_copy("p").unwrap()),
            submit: false,
            picked_at_ms: monotonic_ms(),
            continuation: false,
        };
        // Never started.
        assert_eq!(host.fill(request()), Err(ProbeError::Gone));
        host.start(sink()).unwrap();
        host.stop();
        assert_eq!(host.fill(request()), Err(ProbeError::Gone));
        assert_eq!(host.lone_text(1), Err(ProbeError::Gone));
        assert_eq!(host.refocus(1), Err(ProbeError::Gone));
        assert_eq!(host.field_rect(1), Err(ProbeError::Gone));
    }

    #[test]
    fn an_unknown_token_is_gone_and_writes_nothing() {
        let host = WindowsHost::new();
        host.start(sink()).unwrap();
        let outcome = host.fill(FillRequest {
            token: 4242,
            username: Some("u".to_owned()),
            password: Some(SecretString::from_str_copy("p").unwrap()),
            submit: true,
            picked_at_ms: monotonic_ms(),
            continuation: false,
        });
        assert_eq!(outcome, Err(ProbeError::Gone));
        assert_eq!(host.lone_text(4242), Err(ProbeError::Gone));
        host.stop();
    }

    #[test]
    fn the_window_check_refuses_a_missing_window() {
        let host = WindowsHost::new();
        host.start(sink()).unwrap();
        let rect = Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 40,
        };
        // No window answers at the points of this rectangle as window 0x7FFF_FFF0.
        assert_eq!(host.window_clear(0x7FFF_FFF0, rect), Ok(false));
        host.stop();
    }

    #[test]
    #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]
    fn a_focused_password_field_produces_one_offer_after_the_debounce() {
        if std::env::var_os("MOSAIC_MANUAL_TESTS").is_none() {
            return;
        }
        // Focus a password field of a program within 10 s: exactly one Field
        // event arrives, 150 ms after the last focus event.
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&events);
        let host = WindowsHost::new();
        host.start(Arc::new(move |event| {
            if let Ok(mut list) = seen.lock() {
                list.push(event);
            }
        }))
        .unwrap();
        std::thread::sleep(Duration::from_secs(10));
        host.stop();
        let list = events.lock().unwrap();
        let fields = list
            .iter()
            .filter(|e| matches!(e, HostEvent::Field(_)))
            .count();
        assert_eq!(fields, 1, "{list:?}");
    }

    #[test]
    #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]
    fn a_hung_program_times_out_and_the_host_recovers() {
        if std::env::var_os("MOSAIC_MANUAL_TESTS").is_none() {
            return;
        }
        // Focus a field of a program that is hung (phase 1 step 3 window) within
        // 5 s: capture_now answers Err(Timeout) in about 5 s, and the next
        // call, after the program responds again or another is focused, works.
        std::thread::sleep(Duration::from_secs(5));
        let host = WindowsHost::new();
        host.start(sink()).unwrap();
        let started = Instant::now();
        let first = host.capture_now();
        assert!(
            matches!(first, Err(ProbeError::Timeout))
                && started.elapsed() < Duration::from_millis(5_500),
            "{first:?} after {:?}",
            started.elapsed()
        );
        std::thread::sleep(Duration::from_secs(5));
        assert!(host.capture_now().is_ok());
        host.stop();
    }
}
