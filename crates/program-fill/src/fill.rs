//! The fill sequence (ADR-0164): user name, password, Enter, each write behind
//! a full re-verification, typed in short batches that never split a surrogate
//! pair, with a clean-up of the field after an abort.
//!
//! The operating system sits behind [`FillDriver`] and [`Clock`], so the order
//! of calls is tested on any OS. The password is read once into a zeroizing
//! UTF-16 buffer and never leaves this module otherwise.

use core::ops::Range;

use mosaic_core::SecretString;
use zeroize::Zeroizing;

use crate::verify::{
    CONTINUATION_TTL_MS, FieldFacts, FieldKind, PickSession, ProbeError, ProgramProbe, RuntimeId,
    VerifyFailure, WriteKind, check,
};

/// The most UTF-16 units typed in one batch.
pub const TYPE_BATCH_UNITS: usize = 8;
/// How long held modifier keys are waited for before the fill is abandoned.
pub const MODIFIER_WAIT_MS: u64 = 2_000;
/// How often held modifiers are polled while waiting.
pub const MODIFIER_POLL_MS: u64 = 20;
/// How long focus settles after `SetFocus` before typing starts.
pub const FOCUS_SETTLE_MS: u64 = 50;
/// UI frameworks whose user-name fields take `ValuePattern.SetValue` reliably.
pub const SETVALUE_FRAMEWORKS: &[&str] = &["Win32", "WinForm", "WPF"];

/// A key the fill presses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// Enter (submit).
    Enter,
    /// Ctrl+A.
    SelectAll,
    /// Backspace.
    Backspace,
}

/// A clock the fill waits on (a fake in tests).
pub trait Clock {
    /// Monotonic milliseconds.
    fn now_ms(&self) -> u64;
    /// Waits `ms` milliseconds.
    fn sleep_ms(&self, ms: u64);
}

/// The operating system side of a fill.
pub trait FillDriver {
    /// The process handle a session holds.
    type Held;

    /// Brings the session's window to the foreground, focuses the target with
    /// UI Automation, lets focus settle and resets the focus-event flag.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn focus(&self, session: &PickSession<Self::Held>) -> Result<(), ProbeError>;
    /// Whether a focus event arrived since the last [`FillDriver::focus`].
    fn focus_changed(&self) -> bool;
    /// Whether the target takes `SetValue` (its framework is trusted for it).
    fn can_set_value(&self, session: &PickSession<Self::Held>) -> bool;
    /// Sets the target's value and reads it back; `true` when equal.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn set_value(&self, session: &PickSession<Self::Held>, value: &str)
    -> Result<bool, ProbeError>;
    /// Types UTF-16 units; the number of input events inserted.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn send_units(&self, units: &[u16]) -> Result<u32, ProbeError>;
    /// Presses a key; the number of input events inserted.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn send_key(&self, key: Key) -> Result<u32, ProbeError>;
    /// Whether Shift, Ctrl, Alt or Win is held.
    fn modifiers_down(&self) -> bool;
}

/// Where the fill writes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FillPlan {
    /// The user-name field.
    pub username: Option<RuntimeId>,
    /// The password field.
    pub password: Option<RuntimeId>,
    /// Whether Enter is pressed after the password.
    pub submit: bool,
}

/// The values to write; the password stays a secret string.
pub struct FillSecrets<'a> {
    /// The user name.
    pub username: Option<&'a str>,
    /// The password.
    pub password: Option<&'a SecretString>,
}

/// Why a fill stopped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillAbort {
    /// The re-verification refused the write.
    Verify(VerifyFailure),
    /// Modifier keys were still held after [`MODIFIER_WAIT_MS`].
    ModifiersHeld,
    /// The operating system took fewer input events than sent.
    InputBlocked,
    /// `SetValue` did not read back equal.
    ReadBackMismatch,
    /// The driver failed.
    Probe(ProbeError),
}

/// How a fill ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillOutcome {
    /// Everything planned was written.
    Done {
        /// Whether Enter was pressed.
        submitted: bool,
    },
    /// A write was refused or failed; nothing more was written.
    Aborted {
        /// The write that stopped.
        at: WriteKind,
        /// Why.
        reason: FillAbort,
        /// Whether the typed part of the field was removed again.
        cleared: bool,
    },
    /// No field had a value to write.
    NothingToFill,
}

struct Failure {
    at: WriteKind,
    reason: FillAbort,
    cleared: bool,
}

impl Failure {
    const fn new(at: WriteKind, reason: FillAbort) -> Self {
        Self {
            at,
            reason,
            cleared: false,
        }
    }
}

/// Consecutive ranges of at most [`TYPE_BATCH_UNITS`] units, each shortened by
/// one when it would end between the halves of a surrogate pair.
#[must_use]
pub fn utf16_batches(units: &[u16]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < units.len() {
        let mut end = (start + TYPE_BATCH_UNITS).min(units.len());
        if end < units.len() && end - start > 1 {
            let last_is_high = units
                .get(end - 1)
                .is_some_and(|u| (0xD800..0xDC00).contains(u));
            let next_is_low = units.get(end).is_some_and(|u| (0xDC00..0xE000).contains(u));
            if last_is_high && next_is_low {
                end -= 1;
            }
        }
        out.push(start..end);
        start = end;
    }
    out
}

fn to_utf16(value: &str) -> Zeroizing<Vec<u16>> {
    // `len()` UTF-8 bytes are an upper bound of the UTF-16 units, so the
    // vector never grows and never leaves a copy behind.
    let mut units = Zeroizing::new(Vec::with_capacity(value.len()));
    units.extend(value.encode_utf16());
    units
}

fn verify<P: ProgramProbe>(
    probe: &P,
    session: &PickSession<P::Held>,
    write: WriteKind,
    clock: &dyn Clock,
) -> Result<(), FillAbort> {
    check(probe, session, write, clock.now_ms()).map_err(FillAbort::Verify)
}

fn wait_for_modifiers<D: FillDriver>(driver: &D, clock: &dyn Clock) -> bool {
    let start = clock.now_ms();
    while driver.modifiers_down() {
        if clock.now_ms().saturating_sub(start) >= MODIFIER_WAIT_MS {
            return false;
        }
        clock.sleep_ms(MODIFIER_POLL_MS);
    }
    true
}

/// Types `value` into the focused target, one verified batch at a time. After
/// an abort with at least one batch sent, the field is cleared when it still
/// verifies.
fn type_value<P, D>(
    probe: &P,
    driver: &D,
    session: &PickSession<P::Held>,
    kind: WriteKind,
    value: &str,
    clock: &dyn Clock,
) -> Result<(), Failure>
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    let units = to_utf16(value);
    let mut sent = false;
    for range in utf16_batches(&units) {
        let batch = units.get(range).unwrap_or_default();
        if let Err(reason) = send_batch(probe, driver, session, kind, batch, clock, &mut sent) {
            let mut failure = Failure::new(kind, reason);
            if sent {
                failure.cleared = clear_field(probe, driver, session, kind, clock);
            }
            return Err(failure);
        }
    }
    Ok(())
}

/// One batch: a focus event or a failing check stops it before anything is
/// sent; `sent` is set once the units are handed to the driver.
fn send_batch<P, D>(
    probe: &P,
    driver: &D,
    session: &PickSession<P::Held>,
    kind: WriteKind,
    batch: &[u16],
    clock: &dyn Clock,
    sent: &mut bool,
) -> Result<(), FillAbort>
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    if driver.focus_changed() {
        return Err(FillAbort::Verify(VerifyFailure::FocusMoved));
    }
    verify(probe, session, kind, clock)?;
    // A key pressed after the first check would turn typed text into shortcuts.
    if driver.modifiers_down() {
        return Err(FillAbort::ModifiersHeld);
    }
    let wanted = u32::try_from(batch.len().saturating_mul(2)).unwrap_or(u32::MAX);
    *sent = true;
    match driver.send_units(batch) {
        Ok(n) if n >= wanted => Ok(()),
        Ok(_) => Err(FillAbort::InputBlocked),
        Err(e) => Err(FillAbort::Probe(e)),
    }
}

fn clear_field<P, D>(
    probe: &P,
    driver: &D,
    session: &PickSession<P::Held>,
    kind: WriteKind,
    clock: &dyn Clock,
) -> bool
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    verify(probe, session, kind, clock).is_ok()
        && !driver.modifiers_down()
        && driver.send_key(Key::SelectAll).is_ok()
        && driver.send_key(Key::Backspace).is_ok()
}

fn focus<P, D>(driver: &D, session: &PickSession<P::Held>, at: WriteKind) -> Result<(), Failure>
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    driver
        .focus(session)
        .map_err(|e| Failure::new(at, FillAbort::Probe(e)))
}

type Target<'a, T> = Option<(&'a RuntimeId, T)>;

/// The writes of a fill after the modifier wait; whether Enter was pressed.
fn fill_steps<P, D>(
    probe: &P,
    driver: &D,
    session: &mut PickSession<P::Held>,
    plan: &FillPlan,
    username: Target<'_, &str>,
    password: Target<'_, &SecretString>,
    clock: &dyn Clock,
) -> Result<bool, Failure>
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    if let Some((id, name)) = username {
        session.retarget(id.clone(), FieldKind::Text);
        focus::<P, D>(driver, session, WriteKind::Username)?;
        verify(probe, session, WriteKind::Username, clock)
            .map_err(|r| Failure::new(WriteKind::Username, r))?;
        if driver.can_set_value(session) {
            match driver.set_value(session, name) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(Failure::new(
                        WriteKind::Username,
                        FillAbort::ReadBackMismatch,
                    ));
                }
                Err(e) => {
                    return Err(Failure::new(WriteKind::Username, FillAbort::Probe(e)));
                }
            }
        } else {
            type_value(probe, driver, session, WriteKind::Username, name, clock)?;
        }
    }
    let mut submitted = false;
    if let Some((id, secret)) = password {
        session.retarget(id.clone(), FieldKind::Password);
        focus::<P, D>(driver, session, WriteKind::Password)?;
        type_value(
            probe,
            driver,
            session,
            WriteKind::Password,
            secret.expose_secret(),
            clock,
        )?;
        if plan.submit {
            submitted = press_enter(probe, driver, session, WriteKind::Submit, clock)?;
        }
    } else if plan.submit && plan.password.is_none() && username.is_some() {
        // A lone text field: the first page of a two-step login.
        submitted = press_enter(probe, driver, session, WriteKind::SubmitText, clock)?;
    }
    Ok(submitted)
}

/// Enter after a full check; at least two input events must go in.
fn press_enter<P, D>(
    probe: &P,
    driver: &D,
    session: &PickSession<P::Held>,
    write: WriteKind,
    clock: &dyn Clock,
) -> Result<bool, Failure>
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    verify(probe, session, write, clock).map_err(|r| Failure::new(write, r))?;
    if driver.modifiers_down() {
        return Err(Failure::new(write, FillAbort::ModifiersHeld));
    }
    match driver.send_key(Key::Enter) {
        Ok(n) if n >= 2 => Ok(true),
        Ok(_) => Err(Failure::new(write, FillAbort::InputBlocked)),
        Err(e) => Err(Failure::new(write, FillAbort::Probe(e))),
    }
}

/// Runs a fill: user name, then password, then Enter. The first abort wins and
/// nothing is written after it.
///
/// - No target with a non-empty value: [`FillOutcome::NothingToFill`].
/// - Held modifier keys are waited for up to [`MODIFIER_WAIT_MS`].
/// - The user name is set through `SetValue` where the driver allows it (read
///   back), else typed. The password is always typed, never set.
/// - Enter only when the plan says so and a password was typed, or, for a plan
///   without a password field (a lone text field), after the user name.
#[must_use]
pub fn run_fill<P, D>(
    probe: &P,
    driver: &D,
    session: &mut PickSession<P::Held>,
    plan: &FillPlan,
    secrets: &FillSecrets<'_>,
    clock: &dyn Clock,
) -> FillOutcome
where
    P: ProgramProbe,
    D: FillDriver<Held = P::Held>,
{
    let username = plan
        .username
        .as_ref()
        .zip(secrets.username.filter(|u| !u.is_empty()));
    let password = plan
        .password
        .as_ref()
        .zip(secrets.password.filter(|p| !p.expose_secret().is_empty()));
    let first = if username.is_some() {
        WriteKind::Username
    } else {
        WriteKind::Password
    };
    if username.is_none() && password.is_none() {
        return FillOutcome::NothingToFill;
    }
    if !wait_for_modifiers(driver, clock) {
        return FillOutcome::Aborted {
            at: first,
            reason: FillAbort::ModifiersHeld,
            cleared: false,
        };
    }
    match fill_steps(probe, driver, session, plan, username, password, clock) {
        Ok(submitted) => FillOutcome::Done { submitted },
        Err(f) => FillOutcome::Aborted {
            at: f.at,
            reason: f.reason,
            cleared: f.cleared,
        },
    }
}
/// An armed second step of a two-step login (ADR-0164): the user name was
/// filled, the password field comes next in the same window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Continuation {
    /// The root window of the pick.
    pub root: isize,
    /// The process that owns the window.
    pub window_pid: u32,
    /// The process that owns the element.
    pub element_pid: u32,
    /// The monotonic time of the pick.
    pub picked_at_ms: u64,
    /// Whether the continuation was used.
    pub used: bool,
}

/// Whether a focused field continues the login: unused, within
/// [`CONTINUATION_TTL_MS`] of the pick, in the same root window with the same
/// window and element process, and a password field.
#[must_use]
pub const fn continues(
    c: &Continuation,
    field: &FieldFacts,
    root: isize,
    window_pid: u32,
    now_ms: u64,
) -> bool {
    !c.used
        && now_ms >= c.picked_at_ms
        && now_ms - c.picked_at_ms < CONTINUATION_TTL_MS
        && c.root == root
        && c.window_pid == window_pid
        && field.pid == c.element_pid
        && matches!(field.kind, FieldKind::Password)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use mosaic_core::SecretString;
    use mosaic_origin::program::ProgramClass;
    use proptest::prelude::*;

    use super::*;
    use crate::fake::{DriverCall, FakeClock, FakeDriver, FakeProcess, good_snapshot};
    use crate::identity::CapturedIdentity;
    use crate::verify::{PICK_TTL_MS, Rect, Snapshot, WindowFacts};

    const PICKED: u64 = 10_000;

    fn id(n: i32) -> RuntimeId {
        RuntimeId(vec![42, n])
    }

    fn session() -> PickSession<FakeProcess> {
        let identity = CapturedIdentity {
            pid: 100,
            final_path: r"C:\Apps\tool.exe".to_owned(),
            exe_name: "tool.exe".to_owned(),
            package_family: None,
            signer_id: Some([1; 32]),
            signer_display: None,
            file_sha256: None,
            class: ProgramClass::Ordinary,
        };
        PickSession::new(
            FakeProcess {
                alive: Rc::new(Cell::new(true)),
            },
            &identity,
            100,
            100,
            7,
            id(1),
            FieldKind::Text,
            PICKED,
        )
    }

    #[derive(Clone, Copy)]
    enum Step {
        Good,
        NoFocus,
        OtherWindow,
    }

    /// A probe that answers per call from a script (the last step repeats).
    struct ScriptedProbe {
        script: Vec<Step>,
        calls: Cell<usize>,
    }

    impl ScriptedProbe {
        fn new(script: Vec<Step>) -> Self {
            Self {
                script,
                calls: Cell::new(0),
            }
        }
    }

    impl ProgramProbe for ScriptedProbe {
        type Held = FakeProcess;

        fn snapshot(&self, session: &PickSession<FakeProcess>) -> Result<Snapshot, ProbeError> {
            let n = self.calls.get();
            self.calls.set(n + 1);
            let step = self
                .script
                .get(n)
                .or_else(|| self.script.last())
                .copied()
                .unwrap_or(Step::Good);
            let mut snap = good_snapshot(session);
            match step {
                Step::Good => {}
                Step::NoFocus => {
                    snap.focused = None;
                    snap.focused_is_target = false;
                }
                Step::OtherWindow => {
                    snap.foreground = Some(WindowFacts {
                        root: session.root + 1,
                        pid: session.window_pid,
                        cloaked: false,
                        minimized: false,
                    });
                }
            }
            snap.process_running = session.held.alive.get();
            Ok(snap)
        }
    }

    fn plan(submit: bool) -> FillPlan {
        FillPlan {
            username: Some(id(1)),
            password: Some(id(2)),
            submit,
        }
    }

    fn secret(s: &str) -> SecretString {
        SecretString::from_str_copy(s).unwrap()
    }

    fn sent_units(driver: &FakeDriver) -> Vec<Vec<u16>> {
        driver
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                DriverCall::SendUnits(u) => Some(u),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_pick_fills_user_name_then_password_then_presses_enter() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let mut s = session();
        let pw = secret("0123456789AB");
        let out = run_fill(
            &probe,
            &driver,
            &mut s,
            &plan(true),
            &FillSecrets {
                username: Some("alice"),
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: true });
        let alice: Vec<u16> = "alice".encode_utf16().collect();
        let first: Vec<u16> = "01234567".encode_utf16().collect();
        let second: Vec<u16> = "89AB".encode_utf16().collect();
        assert_eq!(
            driver.calls(),
            vec![
                DriverCall::Focus(id(1)),
                DriverCall::SendUnits(alice),
                DriverCall::Focus(id(2)),
                DriverCall::SendUnits(first),
                DriverCall::SendUnits(second),
                DriverCall::SendKey(Key::Enter),
            ]
        );
    }

    #[test]
    fn the_user_name_uses_set_value_where_allowed_and_must_read_back() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        driver.can_set_value.set(true);
        let clock = FakeClock::new(PICKED);
        let mut s = session();
        let pw = secret("pw");
        let secrets = FillSecrets {
            username: Some("alice"),
            password: Some(&pw),
        };
        let out = run_fill(&probe, &driver, &mut s, &plan(false), &secrets, &clock);
        assert_eq!(out, FillOutcome::Done { submitted: false });
        assert!(
            driver
                .calls()
                .contains(&DriverCall::SetValue("alice".to_owned()))
        );
        // A read-back mismatch stops before the password.
        let driver = FakeDriver::default();
        driver.can_set_value.set(true);
        driver.read_back_ok.set(false);
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &secrets,
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Username,
                reason: FillAbort::ReadBackMismatch,
                cleared: false
            }
        );
        assert_eq!(sent_units(&driver).len(), 0);
    }

    #[test]
    fn the_password_is_typed_even_where_set_value_is_possible() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        driver.can_set_value.set(true);
        let clock = FakeClock::new(PICKED);
        let pw = secret("pw");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(false),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: false });
        let calls = driver.calls();
        assert!(!calls.iter().any(|c| matches!(c, DriverCall::SetValue(_))));
        assert_eq!(sent_units(&driver).len(), 1);
    }

    #[test]
    fn a_modifier_pressed_mid_fill_stops_the_next_batch_and_the_clear_chord() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        // The wait and batch 1 see no modifier; batch 2 does, and stays held.
        driver.set_modifiers(&[false, false, true], true);
        let clock = FakeClock::new(PICKED);
        let pw = secret("0123456789AB");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::ModifiersHeld,
                cleared: false
            }
        );
        // One batch went in; no Ctrl+A or Backspace followed.
        assert_eq!(sent_units(&driver).len(), 1);
        assert!(
            !driver
                .calls()
                .contains(&DriverCall::SendKey(Key::SelectAll))
        );
        assert!(!driver.calls().contains(&DriverCall::SendKey(Key::Enter)));
    }

    #[test]
    fn focus_change_during_typing_aborts_and_cleans_up() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        // Batch 1 passes, a focus event arrives before batch 2.
        driver.set_focus_changes(&[false, true]);
        let clock = FakeClock::new(PICKED);
        let pw = secret("0123456789AB");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::Verify(VerifyFailure::FocusMoved),
                cleared: true
            }
        );
        let calls = driver.calls();
        assert!(calls.ends_with(&[
            DriverCall::SendKey(Key::SelectAll),
            DriverCall::SendKey(Key::Backspace)
        ]));
        assert_eq!(sent_units(&driver).len(), 1);

        // When the field no longer verifies at clean-up time, nothing is sent.
        let probe = ScriptedProbe::new(vec![Step::Good, Step::NoFocus]);
        let driver = FakeDriver::default();
        driver.set_focus_changes(&[false, true]);
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::Verify(VerifyFailure::FocusMoved),
                cleared: false
            }
        );
        assert!(
            !driver
                .calls()
                .contains(&DriverCall::SendKey(Key::SelectAll))
        );
    }

    #[test]
    fn window_swap_or_foreground_change_aborts_typing() {
        let probe = ScriptedProbe::new(vec![Step::Good, Step::OtherWindow]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let pw = secret("0123456789ABCDEF0123");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert!(matches!(
            out,
            FillOutcome::Aborted {
                reason: FillAbort::Verify(VerifyFailure::WindowChanged),
                ..
            }
        ));
        assert_eq!(sent_units(&driver).len(), 1);
    }

    #[test]
    fn short_send_input_aborts() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        driver.short_send.set(true);
        let clock = FakeClock::new(PICKED);
        let pw = secret("pw");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert!(matches!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Password,
                reason: FillAbort::InputBlocked,
                ..
            }
        ));
        assert!(!driver.calls().contains(&DriverCall::SendKey(Key::Enter)));
    }

    #[test]
    fn held_modifiers_delay_then_abort_without_typing() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let pw = secret("pw");
        let secrets = FillSecrets {
            username: Some("alice"),
            password: Some(&pw),
        };
        // Down for 500 ms (25 polls), then up.
        let driver = FakeDriver::default();
        driver.set_modifiers(&[true; 25], false);
        let clock = FakeClock::new(PICKED);
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(false),
            &secrets,
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: false });
        assert!(clock.now_ms() >= PICKED + 500);
        // Held for good.
        let driver = FakeDriver::default();
        driver.set_modifiers(&[], true);
        let clock = FakeClock::new(PICKED);
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &secrets,
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Username,
                reason: FillAbort::ModifiersHeld,
                cleared: false
            }
        );
        assert_eq!(driver.calls().len(), 0);
        assert!(clock.now_ms() >= PICKED + MODIFIER_WAIT_MS);
    }

    #[test]
    fn empty_password_fills_the_user_name_and_never_submits() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let pw = secret("");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: Some("alice"),
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: false });
        assert_eq!(sent_units(&driver).len(), 1);
        assert!(!driver.calls().contains(&DriverCall::SendKey(Key::Enter)));
        // And with nothing at all there is nothing to do.
        let out = run_fill(
            &probe,
            &FakeDriver::default(),
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: Some(""),
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::NothingToFill);
    }

    #[test]
    fn a_lone_text_field_gets_the_user_name_and_enter() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let lone = FillPlan {
            username: Some(id(1)),
            password: None,
            submit: true,
        };
        let secrets = FillSecrets {
            username: Some("alice"),
            password: None,
        };
        let out = run_fill(&probe, &driver, &mut session(), &lone, &secrets, &clock);
        assert_eq!(out, FillOutcome::Done { submitted: true });
        assert!(driver.calls().ends_with(&[DriverCall::SendKey(Key::Enter)]));
        // With submit off nothing is pressed.
        let driver = FakeDriver::default();
        let quiet = FillPlan {
            submit: false,
            ..lone
        };
        let out = run_fill(&probe, &driver, &mut session(), &quiet, &secrets, &clock);
        assert_eq!(out, FillOutcome::Done { submitted: false });
        assert!(!driver.calls().contains(&DriverCall::SendKey(Key::Enter)));
    }

    #[test]
    fn submit_off_leaves_focus_in_the_password_field() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let pw = secret("pw");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(false),
            &FillSecrets {
                username: Some("alice"),
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: false });
        assert!(!driver.calls().contains(&DriverCall::SendKey(Key::Enter)));
    }

    #[test]
    fn an_expired_pick_writes_nothing() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED + PICK_TTL_MS);
        let pw = secret("pw");
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(true),
            &FillSecrets {
                username: Some("alice"),
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(
            out,
            FillOutcome::Aborted {
                at: WriteKind::Username,
                reason: FillAbort::Verify(VerifyFailure::Expired),
                cleared: false
            }
        );
        assert_eq!(sent_units(&driver).len(), 0);
    }

    #[test]
    fn surrogate_pairs_are_never_split_across_batches() {
        let units: Vec<u16> = "1234567\u{1F600}x".encode_utf16().collect();
        assert_eq!(utf16_batches(&units), vec![0..7, 7..10]);
        let long = vec![0x41_u16; 70];
        let ranges = utf16_batches(&long);
        assert_eq!(ranges.len(), 9);
        let mut next = 0;
        for r in &ranges {
            assert_eq!(r.start, next);
            assert!(r.len() <= TYPE_BATCH_UNITS);
            next = r.end;
        }
        assert_eq!(next, 70);
        assert_eq!(utf16_batches(&[]).len(), 0);
        // A lone high surrogate at the very end stays in its batch.
        assert_eq!(utf16_batches(&[0xD800]), vec![0..1]);
    }

    #[test]
    fn the_whole_password_is_typed_exactly() {
        let probe = ScriptedProbe::new(vec![Step::Good]);
        let driver = FakeDriver::default();
        let clock = FakeClock::new(PICKED);
        let text = "P\u{e4}ssw\u{f6}rd-\u{1F600}-12345678\u{1F600}\u{1F600}\u{1F600}xyz";
        let pw = secret(text);
        let out = run_fill(
            &probe,
            &driver,
            &mut session(),
            &plan(false),
            &FillSecrets {
                username: None,
                password: Some(&pw),
            },
            &clock,
        );
        assert_eq!(out, FillOutcome::Done { submitted: false });
        let typed: Vec<u16> = sent_units(&driver).into_iter().flatten().collect();
        let expected: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(typed, expected);
    }

    #[test]
    fn continuation_needs_same_window_and_time() {
        let c = Continuation {
            root: 7,
            window_pid: 100,
            element_pid: 100,
            picked_at_ms: 1_000,
            used: false,
        };
        let field = FieldFacts {
            runtime_id: id(2),
            pid: 100,
            kind: FieldKind::Password,
            enabled: true,
            offscreen: false,
            rect: Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 30,
            },
        };
        assert!(continues(&c, &field, 7, 100, 1_000 + 59_999));
        assert!(!continues(&c, &field, 7, 100, 1_000 + 60_000));
        assert!(!continues(
            &Continuation { used: true, ..c },
            &field,
            7,
            100,
            1_001
        ));
        assert!(!continues(&c, &field, 8, 100, 1_001));
        assert!(!continues(&c, &field, 7, 101, 1_001));
        let other = FieldFacts {
            pid: 101,
            ..field.clone()
        };
        assert!(!continues(&c, &other, 7, 100, 1_001));
        let text = FieldFacts {
            kind: FieldKind::Text,
            ..field.clone()
        };
        assert!(!continues(&c, &text, 7, 100, 1_001));
        assert!(!continues(&c, &field, 7, 100, 999));
    }

    proptest! {
        #[test]
        fn no_write_follows_a_failing_check(good in 0_usize..8, len in 1_usize..40) {
            let probe = ScriptedProbe::new(
                std::iter::repeat_n(Step::Good, good)
                    .chain(std::iter::once(Step::NoFocus))
                    .collect(),
            );
            let driver = FakeDriver::default();
            let clock = FakeClock::new(PICKED);
            let pw = secret(&"a".repeat(len));
            let out = run_fill(
                &probe,
                &driver,
                &mut session(),
                &plan(true),
                &FillSecrets { username: Some("u"), password: Some(&pw) },
                &clock,
            );
            // Every typed batch needs its own passing check before it.
            prop_assert!(sent_units(&driver).len() <= good);
            if good == 0 {
                prop_assert!(!driver.calls().iter().any(|c| matches!(
                    c,
                    DriverCall::SendUnits(_) | DriverCall::SetValue(_) | DriverCall::SendKey(_)
                )));
            }
            let _ = out;
        }
    }
}
