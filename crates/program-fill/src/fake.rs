//! A fake probe for tests: returns a stored snapshot, with the "process still
//! running" fact taken from a shared flag the test can flip (a stand-in for the
//! held process handle).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::verify::{
    FieldFacts, PickSession, ProbeError, ProgramProbe, Rect, Snapshot, WindowFacts,
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
