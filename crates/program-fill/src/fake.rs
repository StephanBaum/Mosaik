//! A fake probe for tests: returns a stored snapshot, with the "process still
//! running" fact taken from a shared flag the test can flip (a stand-in for the
//! held process handle).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Mutex;

use crate::fill::{Clock, Continuation, FillDriver, FillOutcome, Key};
use crate::host::{FillRequest, Host, HostEvent, HostSink, Token};
use crate::verify::{
    FieldFacts, PickSession, ProbeError, ProgramProbe, Rect, RuntimeId, Snapshot, WindowFacts,
};

/// A stand-in for a held process handle.
#[derive(Clone, Debug)]
pub struct FakeProcess {
    /// Whether the process is still running.
    pub alive: Rc<Cell<bool>>,
}

/// A probe that answers with a stored result.
#[derive(Debug)]
pub struct FakeProbe {
    /// What the next snapshot returns (`process_running` is taken from the
    /// session's [`FakeProcess`]).
    pub snapshot: RefCell<Result<Snapshot, ProbeError>>,
}

impl ProgramProbe for FakeProbe {
    type Held = FakeProcess;

    fn snapshot(&self, session: &PickSession<FakeProcess>) -> Result<Snapshot, ProbeError> {
        let mut snap = self.snapshot.borrow().clone()?;
        snap.process_running = session.held.alive.get();
        Ok(snap)
    }
}

/// The snapshot in which everything about `session` still holds: the picked
/// field has focus, in the same window and image, clear of anything on top.
#[must_use]
pub fn good_snapshot<H>(session: &PickSession<H>) -> Snapshot {
    Snapshot {
        focused: Some(FieldFacts {
            runtime_id: session.target.clone(),
            pid: session.element_pid,
            kind: session.target_kind,
            enabled: true,
            offscreen: false,
            rect: Rect {
                left: 100,
                top: 100,
                right: 300,
                bottom: 130,
            },
        }),
        focused_is_target: true,
        process_running: true,
        image_final_path: session.final_path.clone(),
        foreground: Some(WindowFacts {
            root: session.root,
            pid: session.window_pid,
            cloaked: false,
            minimized: false,
        }),
        occlusion_clear: [true; 5],
    }
}

/// One call the fill made on a [`FakeDriver`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DriverCall {
    /// `focus` on the session's current target.
    Focus(RuntimeId),
    /// `set_value` with this text.
    SetValue(String),
    /// `send_units` with these UTF-16 units.
    SendUnits(Vec<u16>),
    /// `send_key`.
    SendKey(Key),
}

/// A fill driver that records its calls and answers from scripts.
#[derive(Debug)]
pub struct FakeDriver {
    calls: RefCell<Vec<DriverCall>>,
    modifiers: RefCell<VecDeque<bool>>,
    modifiers_after: Cell<bool>,
    focus_changes: RefCell<VecDeque<bool>>,
    /// Whether the target takes `SetValue`.
    pub can_set_value: Cell<bool>,
    /// Whether `set_value` reads back equal.
    pub read_back_ok: Cell<bool>,
    /// Whether `send_units` inserts fewer events than sent.
    pub short_send: Cell<bool>,
}

impl Default for FakeDriver {
    fn default() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            modifiers: RefCell::new(VecDeque::new()),
            modifiers_after: Cell::new(false),
            focus_changes: RefCell::new(VecDeque::new()),
            can_set_value: Cell::new(false),
            read_back_ok: Cell::new(true),
            short_send: Cell::new(false),
        }
    }
}

impl FakeDriver {
    /// The calls so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<DriverCall> {
        self.calls.borrow().clone()
    }

    /// `modifiers_down` answers `script` in order, then `after` for ever.
    pub fn set_modifiers(&self, script: &[bool], after: bool) {
        *self.modifiers.borrow_mut() = script.iter().copied().collect();
        self.modifiers_after.set(after);
    }

    /// `focus_changed` answers `script` in order, then `false`.
    pub fn set_focus_changes(&self, script: &[bool]) {
        *self.focus_changes.borrow_mut() = script.iter().copied().collect();
    }
}

impl FillDriver for FakeDriver {
    type Held = FakeProcess;

    fn focus(&self, session: &PickSession<FakeProcess>) -> Result<(), ProbeError> {
        self.calls
            .borrow_mut()
            .push(DriverCall::Focus(session.target.clone()));
        Ok(())
    }

    fn focus_changed(&self) -> bool {
        self.focus_changes.borrow_mut().pop_front().unwrap_or(false)
    }

    fn can_set_value(&self, _session: &PickSession<FakeProcess>) -> bool {
        self.can_set_value.get()
    }

    fn set_value(
        &self,
        _session: &PickSession<FakeProcess>,
        value: &str,
    ) -> Result<bool, ProbeError> {
        self.calls
            .borrow_mut()
            .push(DriverCall::SetValue(value.to_owned()));
        Ok(self.read_back_ok.get())
    }

    fn send_units(&self, units: &[u16]) -> Result<u32, ProbeError> {
        self.calls
            .borrow_mut()
            .push(DriverCall::SendUnits(units.to_vec()));
        let events = u32::try_from(units.len().saturating_mul(2)).unwrap_or(u32::MAX);
        Ok(if self.short_send.get() {
            events / 2
        } else {
            events
        })
    }

    fn send_key(&self, key: Key) -> Result<u32, ProbeError> {
        self.calls.borrow_mut().push(DriverCall::SendKey(key));
        Ok(2)
    }

    fn modifiers_down(&self) -> bool {
        self.modifiers
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| self.modifiers_after.get())
    }
}

/// A clock whose `sleep_ms` advances it.
#[derive(Debug)]
pub struct FakeClock(Cell<u64>);

impl FakeClock {
    /// A clock at `ms`.
    #[must_use]
    pub const fn new(ms: u64) -> Self {
        Self(Cell::new(ms))
    }
}

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.get()
    }

    fn sleep_ms(&self, ms: u64) {
        self.0.set(self.0.get().saturating_add(ms));
    }
}

/// One call on a [`FakeHost`]. A fill is recorded without its password.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HostCall {
    /// `start`.
    Start,
    /// `stop`.
    Stop,
    /// `capture_now`.
    CaptureNow,
    /// `lone_text`.
    LoneText(Token),
    /// `fill`, without the password.
    Fill {
        /// The capture.
        token: Token,
        /// The user name.
        username: Option<String>,
        /// Whether a password was present.
        has_password: bool,
        /// Whether Enter was asked for.
        submit: bool,
        /// Whether this was a continuation.
        continuation: bool,
    },
    /// `refocus`.
    Refocus(Token),
    /// `field_rect`.
    FieldRect(Token),
    /// `arm_continuation`.
    ArmContinuation(Token, Continuation),
    /// `window_clear`.
    WindowClear {
        /// The window asked about.
        own_hwnd: isize,
        /// The rectangle asked about.
        rect: Rect,
    },
}

/// A host with scripted answers and a log of its calls.
pub struct FakeHost {
    /// Every call, in order.
    pub log: Mutex<Vec<HostCall>>,
    /// What `capture_now` returns.
    pub capture: Mutex<Result<HostEvent, ProbeError>>,
    /// What `lone_text` returns.
    pub lone: Mutex<Result<bool, ProbeError>>,
    /// What `fill` returns.
    pub fill_result: Mutex<Result<FillOutcome, ProbeError>>,
    /// What `field_rect` returns.
    pub rect: Mutex<Result<Option<Rect>, ProbeError>>,
    /// What `window_clear` returns.
    pub clear: Mutex<Result<bool, ProbeError>>,
    sink: Mutex<Option<HostSink>>,
}

impl Default for FakeHost {
    fn default() -> Self {
        Self {
            log: Mutex::new(Vec::new()),
            capture: Mutex::new(Ok(HostEvent::NoField { near: None })),
            lone: Mutex::new(Ok(false)),
            fill_result: Mutex::new(Ok(FillOutcome::Done { submitted: false })),
            rect: Mutex::new(Ok(None)),
            clear: Mutex::new(Ok(true)),
            sink: Mutex::new(None),
        }
    }
}

impl FakeHost {
    fn record(&self, call: HostCall) {
        if let Ok(mut log) = self.log.lock() {
            log.push(call);
        }
    }

    /// The calls so far.
    #[must_use]
    pub fn calls(&self) -> Vec<HostCall> {
        self.log.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// Delivers `event` to the sink given to `start`, as the host thread would.
    pub fn emit(&self, event: HostEvent) {
        let sink = self.sink.lock().ok().and_then(|s| s.clone());
        if let Some(sink) = sink {
            sink(event);
        }
    }
}

fn scripted<T: Clone>(slot: &Mutex<Result<T, ProbeError>>) -> Result<T, ProbeError> {
    slot.lock().map_or(Err(ProbeError::Os), |v| v.clone())
}

impl Host for FakeHost {
    fn start(&self, sink: HostSink) -> Result<(), ProbeError> {
        self.record(HostCall::Start);
        if let Ok(mut slot) = self.sink.lock() {
            *slot = Some(sink);
        }
        Ok(())
    }

    fn stop(&self) {
        self.record(HostCall::Stop);
        if let Ok(mut slot) = self.sink.lock() {
            *slot = None;
        }
    }

    fn capture_now(&self) -> Result<HostEvent, ProbeError> {
        self.record(HostCall::CaptureNow);
        scripted(&self.capture)
    }

    fn lone_text(&self, token: Token) -> Result<bool, ProbeError> {
        self.record(HostCall::LoneText(token));
        scripted(&self.lone)
    }

    fn fill(&self, request: FillRequest) -> Result<FillOutcome, ProbeError> {
        self.record(HostCall::Fill {
            token: request.token,
            username: request.username.clone(),
            has_password: request.password.is_some(),
            submit: request.submit,
            continuation: request.continuation,
        });
        scripted(&self.fill_result)
    }

    fn refocus(&self, token: Token) -> Result<(), ProbeError> {
        self.record(HostCall::Refocus(token));
        Ok(())
    }

    fn field_rect(&self, token: Token) -> Result<Option<Rect>, ProbeError> {
        self.record(HostCall::FieldRect(token));
        scripted(&self.rect)
    }

    fn arm_continuation(&self, token: Token, continuation: Continuation) {
        self.record(HostCall::ArmContinuation(token, continuation));
    }

    fn window_clear(&self, own_hwnd: isize, rect: Rect) -> Result<bool, ProbeError> {
        self.record(HostCall::WindowClear { own_hwnd, rect });
        scripted(&self.clear)
    }
}
