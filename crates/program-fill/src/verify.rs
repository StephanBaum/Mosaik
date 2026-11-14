//! Re-verification before every write (spec §5.1.2, ADR-0158): between the pick
//! and each write into a program, the target must still be exactly what the user
//! picked. Any difference, any probe error and any odd value writes nothing.
//!
//! The facts come from a [`ProgramProbe`] (UI Automation, the held process
//! handle and the foreground window on Windows, a fake in tests); this module
//! only decides.

use mosaic_origin::program::paths_equal;

use crate::identity::CapturedIdentity;

/// How long a picked field may be written to, in milliseconds.
pub const PICK_TTL_MS: u64 = 30_000;
/// How long a pick that continues a multi-step login (user name, then password)
/// may be written to, in milliseconds.
pub const CONTINUATION_TTL_MS: u64 = 60_000;
/// The smallest field, in pixels, that is filled: smaller ones are not shown to
/// the user in a way that lets them see what is filled.
pub const MIN_FIELD_PX: i32 = 4;

/// The UI Automation runtime ID of an element.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RuntimeId(pub Vec<i32>);

/// A screen rectangle in pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// A screen point in pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Point {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
}

/// What a field takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldKind {
    /// A password field.
    Password,
    /// Any other text field.
    Text,
}

/// What the probe read about the focused field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldFacts {
    /// The element's runtime ID.
    pub runtime_id: RuntimeId,
    /// The process that owns the element.
    pub pid: u32,
    /// Password or other text.
    pub kind: FieldKind,
    /// Whether the field is enabled.
    pub enabled: bool,
    /// Whether the field is off screen.
    pub offscreen: bool,
    /// The field's screen rectangle.
    pub rect: Rect,
}

/// What the probe read about the foreground window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowFacts {
    /// The root window handle, as an integer.
    pub root: isize,
    /// The process that owns the window.
    pub pid: u32,
    /// Whether the window is cloaked.
    pub cloaked: bool,
    /// Whether the window is minimised.
    pub minimized: bool,
}

/// Everything read fresh just before a write.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    /// The focused edit control; `None` when no edit has focus.
    pub focused: Option<FieldFacts>,
    /// Whether the focused element is the picked one (`CompareElements`).
    pub focused_is_target: bool,
    /// Whether the process is still running, read through the held handle.
    pub process_running: bool,
    /// The image path, read again through the held handle.
    pub image_final_path: String,
    /// The foreground window.
    pub foreground: Option<WindowFacts>,
    /// For each of [`probe_points`] of the target's rectangle: whether the
    /// element at that point is the target or one of its descendants.
    pub occlusion_clear: [bool; 5],
}

/// What the user picked, kept until the fill is done or expired.
pub struct PickSession<H> {
    /// Keeps the process handle open, so its PID cannot be reused by another
    /// process while the session lives.
    pub held: H,
    /// The process that owns the picked element.
    pub element_pid: u32,
    /// The process that owns the window.
    pub window_pid: u32,
    /// The root window handle, as an integer.
    pub root: isize,
    /// The picked element.
    pub target: RuntimeId,
    /// Password or other text.
    pub target_kind: FieldKind,
    /// The image path at pick time.
    pub final_path: String,
    /// The monotonic time of the pick, in milliseconds.
    pub picked_at_ms: u64,
    /// Whether the pick continues a multi-step login ([`CONTINUATION_TTL_MS`]).
    pub continuation: bool,
}

/// What is about to be written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteKind {
    /// The user name.
    Username,
    /// The password.
    Password,
    /// The submit key.
    Submit,
}

/// Why a write must not happen.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum VerifyFailure {
    /// The pick is too old, or the clock went back.
    #[error("pick expired")]
    Expired,
    /// The process ended.
    #[error("process gone")]
    ProcessGone,
    /// Focus is not on the picked field.
    #[error("focus moved")]
    FocusMoved,
    /// The foreground window is not the picked one.
    #[error("window changed")]
    WindowChanged,
    /// The image path is not the one at pick time.
    #[error("image path changed")]
    PathChanged,
    /// The field is not of the kind the write needs.
    #[error("field kind changed")]
    FieldKindChanged,
    /// The field is disabled, off screen or too small.
    #[error("field unusable")]
    FieldUnusable,
    /// Something covers the field.
    #[error("field occluded")]
    Occluded,
    /// The probe failed or timed out.
    #[error("probe failed")]
    ProbeFailed,
}

/// Why the probe could not read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ProbeError {
    /// The target did not answer in time.
    #[error("timeout")]
    Timeout,
    /// The element or process is gone.
    #[error("gone")]
    Gone,
    /// Any other OS error.
    #[error("os error")]
    Os,
}

/// Reads the facts of [`Snapshot`] from the operating system.
pub trait ProgramProbe {
    /// The kept process handle of a session.
    type Held;

    /// Reads a fresh snapshot for `session`, with timeouts.
    ///
    /// # Errors
    /// [`ProbeError`].
    fn snapshot(&self, session: &PickSession<Self::Held>) -> Result<Snapshot, ProbeError>;
}

impl<H> PickSession<H> {
    /// A session for the identity picked at `picked_at_ms`.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "one value per fact of the pick; a builder would only rename them"
    )]
    pub fn new(
        held: H,
        identity: &CapturedIdentity,
        element_pid: u32,
        window_pid: u32,
        root: isize,
        target: RuntimeId,
        target_kind: FieldKind,
        picked_at_ms: u64,
    ) -> Self {
        Self {
            held,
            element_pid,
            window_pid,
            root,
            target,
            target_kind,
            final_path: identity.final_path.clone(),
            picked_at_ms,
            continuation: false,
        }
    }

    fn expired(&self, now_ms: u64) -> bool {
        let ttl = if self.continuation {
            CONTINUATION_TTL_MS
        } else {
            PICK_TTL_MS
        };
        now_ms < self.picked_at_ms || now_ms - self.picked_at_ms >= ttl
    }
}

/// Five points of a field's rectangle to test for occlusion: the centre and the
/// four points a quarter of the width and height in from each corner. `None` if
/// the rectangle is inverted or smaller than [`MIN_FIELD_PX`] in either
/// direction. Computed in `i64`, so no rectangle overflows.
#[must_use]
pub fn probe_points(rect: Rect) -> Option<[Point; 5]> {
    let (left, top) = (i64::from(rect.left), i64::from(rect.top));
    let (right, bottom) = (i64::from(rect.right), i64::from(rect.bottom));
    let (width, height) = (right - left, bottom - top);
    let min = i64::from(MIN_FIELD_PX);
    if width < min || height < min {
        return None;
    }
    let (qx, qy) = (width / 4, height / 4);
    let at = |x: i64, y: i64| {
        Some(Point {
            x: i32::try_from(x).ok()?,
            y: i32::try_from(y).ok()?,
        })
    };
    Some([
        at(left + width / 2, top + height / 2)?,
        at(left + qx, top + qy)?,
        at(right - qx, top + qy)?,
        at(left + qx, bottom - qy)?,
        at(right - qx, bottom - qy)?,
    ])
}

/// Decides, from a fresh snapshot, whether `write` may happen now. The first
/// failing check wins: expiry, process, focus, window, image path, field kind,
/// field state, occlusion.
///
/// # Errors
/// [`VerifyFailure`].
pub fn verify_before_write<H>(
    session: &PickSession<H>,
    snap: &Snapshot,
    write: WriteKind,
    now_ms: u64,
) -> Result<(), VerifyFailure> {
    if session.expired(now_ms) {
        return Err(VerifyFailure::Expired);
    }
    if !snap.process_running {
        return Err(VerifyFailure::ProcessGone);
    }
    let focused = snap
        .focused
        .as_ref()
        .filter(|f| {
            snap.focused_is_target && f.runtime_id == session.target && f.pid == session.element_pid
        })
        .ok_or(VerifyFailure::FocusMoved)?;
    let window_ok = snap.foreground.is_some_and(|w| {
        w.root == session.root && w.pid == session.window_pid && !w.cloaked && !w.minimized
    });
    if !window_ok {
        return Err(VerifyFailure::WindowChanged);
    }
    if !paths_equal(&snap.image_final_path, &session.final_path) {
        return Err(VerifyFailure::PathChanged);
    }
    let needed = match write {
        WriteKind::Username => FieldKind::Text,
        WriteKind::Password | WriteKind::Submit => FieldKind::Password,
    };
    if focused.kind != needed || session.target_kind != needed {
        return Err(VerifyFailure::FieldKindChanged);
    }
    if !focused.enabled || focused.offscreen || probe_points(focused.rect).is_none() {
        return Err(VerifyFailure::FieldUnusable);
    }
    if snap.occlusion_clear.iter().any(|clear| !clear) {
        return Err(VerifyFailure::Occluded);
    }
    Ok(())
}

/// Reads a snapshot through `probe` and verifies it. An expired session is
/// refused without asking the program anything; a probe error is
/// [`VerifyFailure::ProbeFailed`].
///
/// # Errors
/// [`VerifyFailure`].
pub fn check<P: ProgramProbe>(
    probe: &P,
    session: &PickSession<P::Held>,
    write: WriteKind,
    now_ms: u64,
) -> Result<(), VerifyFailure> {
    if session.expired(now_ms) {
        return Err(VerifyFailure::Expired);
    }
    let snap = probe
        .snapshot(session)
        .map_err(|_| VerifyFailure::ProbeFailed)?;
    verify_before_write(session, &snap, write, now_ms)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use mosaic_origin::program::ProgramClass;
    use proptest::prelude::*;

    use super::*;
    use crate::fake::{FakeProbe, FakeProcess, good_snapshot};

    const PICKED: u64 = 10_000;

    fn session() -> (PickSession<FakeProcess>, Rc<Cell<bool>>) {
        let alive = Rc::new(Cell::new(true));
        let identity = CapturedIdentity {
            pid: 100,
            final_path: r"C:\Apps\Tool\tool.exe".to_owned(),
            exe_name: "tool.exe".to_owned(),
            package_family: None,
            signer_subject_sha256: Some([1; 32]),
            signer_display: None,
            file_sha256: None,
            class: ProgramClass::Ordinary,
        };
        let s = PickSession::new(
            FakeProcess {
                alive: Rc::clone(&alive),
            },
            &identity,
            100,
            100,
            0x1234,
            RuntimeId(vec![42, 1, 2]),
            FieldKind::Password,
            PICKED,
        );
        (s, alive)
    }

    fn probe_of(snap: Snapshot) -> FakeProbe {
        FakeProbe {
            snapshot: RefCell::new(Ok(snap)),
        }
    }

    fn run(
        s: &PickSession<FakeProcess>,
        edit: impl FnOnce(&mut Snapshot),
        write: WriteKind,
    ) -> Result<(), VerifyFailure> {
        let mut snap = good_snapshot(s);
        edit(&mut snap);
        check(&probe_of(snap), s, write, PICKED + 1_000)
    }

    #[test]
    fn a_good_snapshot_passes() {
        let (s, _) = session();
        assert_eq!(run(&s, |_| {}, WriteKind::Password), Ok(()));
        assert_eq!(run(&s, |_| {}, WriteKind::Submit), Ok(()));
    }

    #[test]
    fn focus_change_between_pick_and_write_writes_nothing() {
        let (s, _) = session();
        let wrong = |snap: &mut Snapshot| snap.focused_is_target = false;
        assert_eq!(
            run(&s, wrong, WriteKind::Password),
            Err(VerifyFailure::FocusMoved)
        );
        let none = |snap: &mut Snapshot| snap.focused = None;
        assert_eq!(
            run(&s, none, WriteKind::Password),
            Err(VerifyFailure::FocusMoved)
        );
        let wrong_element = |snap: &mut Snapshot| {
            if let Some(f) = snap.focused.as_mut() {
                f.runtime_id = RuntimeId(vec![9]);
            }
        };
        assert_eq!(
            run(&s, wrong_element, WriteKind::Password),
            Err(VerifyFailure::FocusMoved)
        );
        let wrong_process = |snap: &mut Snapshot| {
            if let Some(f) = snap.focused.as_mut() {
                f.pid = 999;
            }
        };
        assert_eq!(
            run(&s, wrong_process, WriteKind::Password),
            Err(VerifyFailure::FocusMoved)
        );
    }

    #[test]
    fn window_swap_or_foreground_change_aborts_typing() {
        let (s, _) = session();
        let edits: [fn(&mut WindowFacts); 4] = [
            |w| w.root = 0x9999,
            |w| w.pid = 7,
            |w| w.cloaked = true,
            |w| w.minimized = true,
        ];
        for edit in edits {
            let r = run(
                &s,
                |snap| {
                    if let Some(w) = snap.foreground.as_mut() {
                        edit(w);
                    }
                },
                WriteKind::Password,
            );
            assert_eq!(r, Err(VerifyFailure::WindowChanged));
        }
        let none = |snap: &mut Snapshot| snap.foreground = None;
        assert_eq!(
            run(&s, none, WriteKind::Password),
            Err(VerifyFailure::WindowChanged)
        );
    }

    #[test]
    fn pid_reuse_cannot_take_over_a_session() {
        let (s, alive) = session();
        // The process ended and a new one took the same PID: the held handle
        // says the one we picked is gone, whatever the facts of the new one say.
        alive.set(false);
        assert_eq!(
            run(&s, |_| {}, WriteKind::Password),
            Err(VerifyFailure::ProcessGone)
        );
        alive.set(true);
        let moved = |snap: &mut Snapshot| snap.image_final_path = r"C:\Other\evil.exe".to_owned();
        assert_eq!(
            run(&s, moved, WriteKind::Password),
            Err(VerifyFailure::PathChanged)
        );
        // Case is not a change.
        let upper =
            |snap: &mut Snapshot| snap.image_final_path = r"C:\APPS\TOOL\TOOL.EXE".to_owned();
        assert_eq!(run(&s, upper, WriteKind::Password), Ok(()));
    }

    #[test]
    fn occluding_window_blocks_the_fill() {
        let (s, _) = session();
        let covered = |snap: &mut Snapshot| snap.occlusion_clear[2] = false;
        assert_eq!(
            run(&s, covered, WriteKind::Password),
            Err(VerifyFailure::Occluded)
        );
    }

    #[test]
    fn password_never_written_into_a_non_password_field() {
        let (s, _) = session();
        let text = |snap: &mut Snapshot| {
            if let Some(f) = snap.focused.as_mut() {
                f.kind = FieldKind::Text;
            }
        };
        for write in [WriteKind::Password, WriteKind::Submit] {
            assert_eq!(run(&s, text, write), Err(VerifyFailure::FieldKindChanged));
        }
        // A user name goes into a text field only.
        assert_eq!(
            run(&s, |_| {}, WriteKind::Username),
            Err(VerifyFailure::FieldKindChanged)
        );
        // A pick of a text field never takes a password, even if the field
        // now reads as a password field.
        let (mut s, _) = session();
        s.target_kind = FieldKind::Text;
        assert_eq!(
            run(&s, |_| {}, WriteKind::Password),
            Err(VerifyFailure::FieldKindChanged)
        );
        assert_eq!(run(&s, text, WriteKind::Username), Ok(()));
    }

    #[test]
    fn disabled_offscreen_or_tiny_fields_are_unusable() {
        let (s, _) = session();
        let edits: [fn(&mut FieldFacts); 4] = [
            |f| f.enabled = false,
            |f| f.offscreen = true,
            |f| f.rect.right = f.rect.left + 3,
            |f| f.rect.bottom = f.rect.top + 3,
        ];
        for edit in edits {
            let r = run(
                &s,
                |snap| {
                    if let Some(f) = snap.focused.as_mut() {
                        edit(f);
                    }
                },
                WriteKind::Password,
            );
            assert_eq!(r, Err(VerifyFailure::FieldUnusable));
        }
    }

    #[test]
    fn a_session_from_the_future_or_long_ago_is_expired() {
        let (mut s, _) = session();
        let at = |s: &PickSession<FakeProcess>, now: u64| {
            check(&probe_of(good_snapshot(s)), s, WriteKind::Password, now)
        };
        assert_eq!(at(&s, PICKED - 1), Err(VerifyFailure::Expired));
        assert_eq!(at(&s, PICKED + PICK_TTL_MS - 1), Ok(()));
        assert_eq!(at(&s, PICKED + PICK_TTL_MS), Err(VerifyFailure::Expired));
        assert_eq!(at(&s, u64::MAX), Err(VerifyFailure::Expired));
        s.picked_at_ms = 0;
        assert_eq!(at(&s, u64::MAX), Err(VerifyFailure::Expired));
        s.picked_at_ms = PICKED;
        s.continuation = true;
        assert_eq!(at(&s, PICKED + CONTINUATION_TTL_MS - 1), Ok(()));
        assert_eq!(
            at(&s, PICKED + CONTINUATION_TTL_MS),
            Err(VerifyFailure::Expired)
        );
        // A wrapped clock near the top of the range is not "valid forever".
        s.picked_at_ms = u64::MAX - 5;
        assert_eq!(at(&s, 3), Err(VerifyFailure::Expired));
    }

    #[test]
    fn probe_points_refuse_tiny_or_inverted_rects() {
        let r = |left, top, right, bottom| Rect {
            left,
            top,
            right,
            bottom,
        };
        assert_eq!(probe_points(r(0, 0, 3, 10)), None);
        assert_eq!(probe_points(r(0, 0, 10, 3)), None);
        assert_eq!(probe_points(r(10, 10, 0, 0)), None);
        assert_eq!(probe_points(r(-32_000, -32_000, -32_000, -32_000)), None);
        let points = probe_points(r(0, 0, 100, 40)).unwrap();
        assert_eq!(points[0], Point { x: 50, y: 20 });
        assert_eq!(points[1], Point { x: 25, y: 10 });
        assert_eq!(points[4], Point { x: 75, y: 30 });
        assert!(probe_points(r(0, 0, 4, 4)).is_some());
    }

    #[test]
    fn probe_points_do_not_overflow() {
        let rect = Rect {
            left: i32::MIN,
            top: i32::MIN,
            right: i32::MAX,
            bottom: i32::MAX,
        };
        let points = probe_points(rect).unwrap();
        for p in points {
            assert!(p.x >= rect.left && p.x <= rect.right);
            assert!(p.y >= rect.top && p.y <= rect.bottom);
        }
    }

    #[test]
    fn probe_errors_fail_closed() {
        let (s, _) = session();
        for e in [ProbeError::Timeout, ProbeError::Gone, ProbeError::Os] {
            let probe = FakeProbe {
                snapshot: RefCell::new(Err(e)),
            };
            assert_eq!(
                check(&probe, &s, WriteKind::Password, PICKED + 1),
                Err(VerifyFailure::ProbeFailed)
            );
        }
    }

    proptest! {
        #[test]
        fn probe_points_are_none_or_inside_the_rect(
            left in any::<i32>(), top in any::<i32>(), right in any::<i32>(), bottom in any::<i32>()
        ) {
            let rect = Rect { left, top, right, bottom };
            if let Some(points) = probe_points(rect) {
                for p in points {
                    prop_assert!(p.x >= left && p.x <= right);
                    prop_assert!(p.y >= top && p.y <= bottom);
                }
            }
        }
    }
}
