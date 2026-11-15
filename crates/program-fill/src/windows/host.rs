//! `WindowsHost`: one worker thread that owns the only UI Automation client,
//! watches focus through the shim, and runs captures and fills (ADR-0164).
//!
//! The worker lives in the COM multithreaded apartment (`WindowsProbe::new`).
//! Calls from other threads go over a channel; one that is not answered in time
//! abandons the worker (its later events and queued commands are dropped) and
//! starts a new one.
//!
//! Events reach the sink on a dispatcher thread of their own, never on the
//! worker: the sink calls back into the host (`lone_text`, `fill`), and a call
//! made on the worker thread would wait for that same worker.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mosaic_winshim::FocusSubscription;

use super::driver::WindowsDriver;
use super::{CaptureError, Captured, WindowScan, WindowsProbe};
use crate::detect::{
    FieldRole, classify, container_rules_out_lone_text, lone_text, second_step_is_automatic,
};
use crate::fill::{Continuation, FillOutcome, FillPlan, FillSecrets, continues, run_fill};
use crate::host::{
    FieldOffer, FillRequest, Host, HostEvent, HostSink, SystemClock, Token, monotonic_ms,
};
use crate::verify::{CONTINUATION_TTL_MS, PICK_TTL_MS, ProbeError, Rect};

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
    /// The `u64` is the call's deadline on [`monotonic_ms`]: the worker never starts a fill
    /// past it, and the fill's session expires at it.
    Fill(FillRequest, Sender<Result<FillOutcome, ProbeError>>, u64),
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

/// An event and the epoch of the worker that produced it.
type Stamped = (u64, HostEvent);

struct State {
    worker: Option<Worker>,
    sink: Option<HostSink>,
    /// Feeds the dispatcher thread that calls the sink; dropped by `stop`,
    /// which ends that thread.
    events: Option<Sender<Stamped>>,
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
                events: None,
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
    fn spawn(&self, events: &Sender<Stamped>) -> Result<Worker, ProbeError> {
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ping_tx, ping_rx) = mpsc::sync_channel(PING_QUEUE);
        let (ready_tx, ready_rx) = mpsc::channel();
        let wake = ping_tx.clone();
        let context = Context {
            epoch,
            current: Arc::clone(&self.current),
            events: events.clone(),
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
    /// host is still started. The old worker is woken so that it sees its epoch
    /// is over and leaves without running what is still queued for it.
    fn abandon(&self, epoch: u64) {
        let (events, old) = {
            let mut state = self.lock();
            if state.worker.as_ref().is_none_or(|w| w.epoch != epoch) {
                return;
            }
            self.current.fetch_add(1, Ordering::SeqCst);
            let old = state.worker.take();
            (state.events.clone(), old)
        };
        if let Some(old) = old {
            old.wake.try_send(WAKE).ok();
        }
        if let Some(events) = events
            && let Ok(worker) = self.spawn(&events)
        {
            let mut state = self.lock();
            if state.worker.is_none() && state.sink.is_some() {
                state.worker = Some(worker);
            } else {
                // Stopped (or restarted) meanwhile: the new worker must not survive.
                self.current.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    /// Delivers `event` as the current worker would (tests).
    #[cfg(test)]
    fn inject_event(&self, event: HostEvent) {
        let events = self.lock().events.clone();
        if let Some(events) = events {
            events
                .send((self.current.load(Ordering::SeqCst), event))
                .ok();
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
        let events = {
            let mut state = self.lock();
            if state.worker.is_some() {
                return Ok(());
            }
            let events = if let (Some(events), Some(_)) = (&state.events, &state.sink) {
                events.clone()
            } else {
                let (tx, rx) = mpsc::channel();
                let current = Arc::clone(&self.current);
                let dispatch_sink = Arc::clone(&sink);
                std::thread::Builder::new()
                    .name("mosaic-program-fill-events".to_owned())
                    .spawn(move || dispatcher_main(&rx, &dispatch_sink, &current))
                    .map_err(|_| ProbeError::Os)?;
                state.events = Some(tx.clone());
                tx
            };
            state.sink = Some(Arc::clone(&sink));
            events
        };
        let worker = self.spawn(&events)?;
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
            state.events = None;
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
        {
            let deadline = fill_deadline(monotonic_ms());
            self.call(FILL_CALL_TIMEOUT_MS, |tx| {
                Command::Fill(request, tx, deadline)
            })?
        }
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
    events: Sender<Stamped>,
}

impl Context {
    /// Whether this worker is still the host's current one.
    fn is_current(&self) -> bool {
        self.current.load(Ordering::SeqCst) == self.epoch
    }

    /// Hands the event to the dispatcher; it never runs the sink here.
    fn emit(&self, event: HostEvent) {
        if self.is_current() {
            self.events.send((self.epoch, event)).ok();
        }
    }
}

/// Calls the sink, in order, for events of the current epoch only. Ends when
/// the host drops its sender (`stop`) and the queue is empty.
fn dispatcher_main(events: &Receiver<Stamped>, sink: &HostSink, current: &AtomicU64) {
    while let Ok((epoch, event)) = events.recv() {
        if current.load(Ordering::SeqCst) == epoch {
            sink(event);
        }
    }
}

/// When a fill sent at `sent_ms` must be over: a second before its caller gives up.
fn fill_deadline(sent_ms: u64) -> u64 {
    sent_ms.saturating_add(FILL_CALL_TIMEOUT_MS.saturating_sub(1_000))
}

/// The pick time to give a session so that it expires no later than `deadline`
/// (the pick's own time when that is earlier). Never in the future of `now_ms`.
fn capped_pick_start(picked_at_ms: u64, deadline_ms: u64, ttl_ms: u64, now_ms: u64) -> u64 {
    picked_at_ms.max(deadline_ms.saturating_sub(ttl_ms).min(now_ms))
}

/// What a focus change reports: an offer or a continuation goes on to the
/// desktop, anything else means the field is gone.
fn focus_report(captured: Result<HostEvent, ProbeError>) -> Option<HostEvent> {
    match captured {
        Ok(
            event
            @ (HostEvent::Field(_) | HostEvent::Continue { .. } | HostEvent::ContinueOffer(_)),
        ) => Some(event),
        _ => None,
    }
}

/// What a pass over the command queue ended with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Drain {
    /// The queue is empty.
    Idle,
    /// The worker is abandoned or stopped, or its host is gone: it must leave,
    /// and what is still queued is dropped unrun (a queued fill most of all).
    Leave,
}

/// Hands each queued command to `handle`, checking before every one that the
/// worker is still current.
fn drain_commands(
    context: &Context,
    commands: &Receiver<Command>,
    mut handle: impl FnMut(Command),
) -> Drain {
    loop {
        if !context.is_current() {
            return Drain::Leave;
        }
        match commands.try_recv() {
            Ok(Command::Fill(_, reply, deadline)) if monotonic_ms() >= deadline => {
                // Its caller gave up (or is about to): a late fill is never started.
                reply.send(Err(ProbeError::Timeout)).ok();
            }
            Ok(command) => handle(command),
            Err(TryRecvError::Empty) => return Drain::Idle,
            Err(TryRecvError::Disconnected) => return Drain::Leave,
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
        if drain_commands(context, commands, |command| worker.handle(command)) == Drain::Leave {
            drop(subscription);
            return;
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
                if !context.is_current() {
                    drop(subscription);
                    return;
                }
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
            Command::Fill(request, reply, deadline) => {
                reply.send(self.fill(&request, deadline)).ok();
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
        if let Some(event) = focus_report(self.capture_event()) {
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
        let offer = FieldOffer {
            token,
            identity: captured.identity.clone(),
            role,
            rect: captured.field.rect,
            target: captured.session.target.clone(),
        };
        if continues_login {
            // Used once: the first step's handle is released with it.
            self.armed = None;
            // Without a new pick only a native window with exactly one password
            // field continues; web content, or a window with other password
            // fields, is offered like any field instead (ADR-0164 amendment).
            let automatic = !captured.web_content && {
                let edits = self
                    .probe
                    .window_edits(captured.session.root, WindowScan::Password)
                    .ok();
                second_step_is_automatic(false, edits.as_deref(), &captured.field.runtime_id)
            };
            self.current = Some(Current { token, captured });
            return Ok(if automatic {
                HostEvent::Continue {
                    token,
                    near: offer.rect,
                }
            } else {
                HostEvent::ContinueOffer(offer)
            });
        }
        self.current = Some(Current { token, captured });
        Ok(HostEvent::Field(offer))
    }

    fn lone_text(&mut self, token: Token) -> Result<bool, ProbeError> {
        let current = self
            .current
            .as_ref()
            .filter(|c| c.token == token)
            .ok_or(ProbeError::Gone)?;
        let root = current.captured.session.root;
        let target = current.captured.session.target.clone();
        // The container is searched anyway: two usable edits there decide it
        // without the costlier search of the whole window.
        let (container, _) = self.probe.field_set(&current.captured)?;
        if container_rules_out_lone_text(&container) {
            return Ok(false);
        }
        let edits = self.probe.window_edits(root, WindowScan::Usable)?;
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
    ///
    /// Only `picked_at_ms` is read from `continuation`: the window and the processes
    /// are the capture's own, so a caller that does not know them cannot arm a wrong one.
    fn arm(&mut self, token: Token, continuation: Continuation) {
        if self.current.as_ref().is_some_and(|c| c.token == token)
            && let Some(current) = self.current.take()
        {
            let session = &current.captured.session;
            self.armed = Some(Armed {
                continuation: Continuation {
                    root: session.root,
                    window_pid: session.window_pid,
                    element_pid: session.element_pid,
                    picked_at_ms: continuation.picked_at_ms,
                    used: false,
                },
                captured: current.captured,
            });
        }
    }

    fn fill(&mut self, request: &FillRequest, deadline_ms: u64) -> Result<FillOutcome, ProbeError> {
        let probe = &self.probe;
        let pings = &self.pings;
        let current = self
            .current
            .as_mut()
            .filter(|c| c.token == request.token)
            .ok_or(ProbeError::Gone)?;
        let ttl = if request.continuation {
            CONTINUATION_TTL_MS
        } else {
            PICK_TTL_MS
        };
        // A continuation may outlive its call: the session ends with the call.
        current.captured.session.picked_at_ms =
            capped_pick_start(request.picked_at_ms, deadline_ms, ttl, monotonic_ms());
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

    fn fill_request() -> FillRequest {
        FillRequest {
            token: 1,
            username: Some("u".to_owned()),
            password: Some(SecretString::from_str_copy("p").unwrap()),
            submit: true,
            picked_at_ms: monotonic_ms(),
            continuation: false,
        }
    }

    fn context(epoch: u64, current: &Arc<AtomicU64>) -> Context {
        let (events, _rx) = mpsc::channel();
        Context {
            epoch,
            current: Arc::clone(current),
            events,
        }
    }

    #[test]
    fn a_sink_can_call_back_into_the_host_from_inside_an_event() {
        let host = Arc::new(WindowsHost::new());
        let (done_tx, done_rx) = mpsc::channel();
        let inner = Arc::clone(&host);
        host.start(Arc::new(move |_| {
            // The service does exactly this on a field offer: `lone_text`,
            // and on a continuation `fill`. On the worker's own thread both
            // would wait for the worker until the call timed out.
            let started = Instant::now();
            let lone = inner.lone_text(9);
            let fill = inner.fill(fill_request());
            done_tx.send((lone, fill, started.elapsed())).ok();
        }))
        .unwrap();
        host.inject_event(HostEvent::Gone);
        let (lone, fill, took) = done_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("the sink ran");
        // No capture has token 9 (or 1): the worker answered, quickly.
        assert_eq!(lone, Err(ProbeError::Gone));
        assert_eq!(fill.map(|_| ()), Err(ProbeError::Gone));
        assert!(took < Duration::from_secs(2), "{took:?}");
        host.stop();
    }

    #[test]
    fn events_reach_the_sink_in_order_and_never_after_stop() {
        let host = WindowsHost::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        host.start(Arc::new(move |event| {
            if let Ok(mut list) = log.lock() {
                list.push(event);
            }
        }))
        .unwrap();
        for near in 0..20 {
            host.inject_event(HostEvent::NoField {
                near: Some(Rect {
                    left: near,
                    top: 0,
                    right: near + 1,
                    bottom: 1,
                }),
            });
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while seen.lock().unwrap().len() < 20 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        host.stop();
        let lefts: Vec<i32> = seen
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                HostEvent::NoField { near: Some(r) } => Some(r.left),
                _ => None,
            })
            .collect();
        assert_eq!(lefts, (0..20).collect::<Vec<_>>());
        host.inject_event(HostEvent::Gone);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(seen.lock().unwrap().len(), 20);
    }

    #[test]
    fn an_abandoned_worker_does_not_run_what_is_queued_for_it() {
        let current = Arc::new(AtomicU64::new(1));
        let context = context(1, &current);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (reply_tx, reply_rx) = mpsc::channel();
        cmd_tx
            .send(Command::Fill(fill_request(), reply_tx, u64::MAX))
            .unwrap();
        // The host gave up on this worker (a new epoch) while it was busy.
        current.fetch_add(1, Ordering::SeqCst);
        let mut ran = 0;
        let outcome = drain_commands(&context, &cmd_rx, |_| ran += 1);
        assert_eq!(outcome, Drain::Leave);
        assert_eq!(ran, 0, "a queued fill must never run late");
        // The reply side learns it is gone instead of waiting on an answer.
        drop(cmd_rx);
        assert!(reply_rx.recv().is_err());
    }

    #[test]
    fn a_queued_fill_past_its_call_deadline_is_never_started() {
        let current = Arc::new(AtomicU64::new(1));
        let context = context(1, &current);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (late_tx, late_rx) = mpsc::channel();
        let (ok_tx, _ok_rx) = mpsc::channel();
        cmd_tx
            .send(Command::Fill(fill_request(), late_tx, 0))
            .unwrap();
        cmd_tx
            .send(Command::Fill(fill_request(), ok_tx, u64::MAX))
            .unwrap();
        let mut ran = 0;
        assert_eq!(drain_commands(&context, &cmd_rx, |_| ran += 1), Drain::Idle);
        assert_eq!(ran, 1, "only the fill inside its deadline runs");
        assert_eq!(late_rx.recv(), Ok(Err(ProbeError::Timeout)));
    }

    #[test]
    fn a_session_never_outlives_its_call() {
        // A continuation picked 50 s ago, called with 34 s to go: it ends with the call.
        let now = 100_000;
        let picked = now - 50_000;
        let deadline = now + 34_000;
        let start = capped_pick_start(picked, deadline, CONTINUATION_TTL_MS, now);
        assert_eq!(start + CONTINUATION_TTL_MS, deadline);
        // A fresh pick inside its limits is unchanged.
        assert_eq!(
            capped_pick_start(now, deadline, CONTINUATION_TTL_MS, now),
            now
        );
        // Never in the future.
        assert!(capped_pick_start(now, now + 50_000, 30_000, now) <= now);
        assert_eq!(fill_deadline(10), 10 + FILL_CALL_TIMEOUT_MS - 1_000);
    }

    #[test]
    fn a_focus_change_forwards_offers_and_both_continuations() {
        let rect = Rect {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
        };
        let continue_ = HostEvent::Continue {
            token: 2,
            near: rect,
        };
        assert_eq!(focus_report(Ok(continue_.clone())), Some(continue_));
        let offer = crate::host::FieldOffer {
            token: 3,
            identity: crate::identity::CapturedIdentity {
                pid: 1,
                final_path: String::new(),
                exe_name: "a.exe".to_owned(),
                package_family: None,
                signer_id: None,
                signer_display: None,
                file_sha256: None,
                class: mosaic_origin::program::ProgramClass::Ordinary,
            },
            role: FieldRole::Password { username: None },
            rect,
            target: crate::verify::RuntimeId(vec![1]),
        };
        for event in [
            HostEvent::Field(offer.clone()),
            HostEvent::ContinueOffer(offer),
        ] {
            assert_eq!(focus_report(Ok(event.clone())), Some(event));
        }
        assert_eq!(focus_report(Ok(HostEvent::Gone)), None);
        assert_eq!(focus_report(Ok(HostEvent::NoField { near: None })), None);
        assert_eq!(focus_report(Err(ProbeError::Timeout)), None);
    }

    #[test]
    fn a_current_worker_runs_its_queue_and_leaves_when_the_epoch_ends_midway() {
        let current = Arc::new(AtomicU64::new(1));
        let context = context(1, &current);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let replies: Vec<_> = (0..3)
            .map(|_| {
                let (tx, rx) = mpsc::channel();
                cmd_tx
                    .send(Command::Fill(fill_request(), tx, u64::MAX))
                    .unwrap();
                rx
            })
            .collect();
        let mut ran = 0;
        let outcome = drain_commands(&context, &cmd_rx, |_| {
            ran += 1;
            if ran == 1 {
                // Abandoned while the first one runs: the other two are dropped.
                current.fetch_add(1, Ordering::SeqCst);
            }
        });
        assert_eq!((outcome, ran), (Drain::Leave, 1));
        drop(cmd_rx);
        assert!(replies.iter().all(|rx| rx.recv().is_err()));
        // And an idle current worker just reports an empty queue.
        let current = Arc::new(AtomicU64::new(5));
        let (_tx, rx) = mpsc::channel::<Command>();
        assert_eq!(
            drain_commands(&self::context(5, &current), &rx, |_| {}),
            Drain::Idle
        );
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
