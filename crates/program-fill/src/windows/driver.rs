//! The Windows side of a fill: focus, `SetValue`, typing and keys, through the
//! shim (ADR-0163, ADR-0164). The rules (what is checked before every write)
//! live in [`crate::fill`]; this only performs one step at a time.

use std::sync::mpsc::Receiver;
use std::time::Duration;

use mosaic_winshim::{self as shim, Uia, UiaElement};

use super::WinHeld;
use super::host::WAKE;
use super::uia::probe_error;
use crate::fill::{FOCUS_SETTLE_MS, FillDriver, Key, SETVALUE_FRAMEWORKS};
use crate::verify::{PickSession, ProbeError};

/// Drives one fill. Focus events arrive on `pings` (the host's focus channel):
/// every one after the last [`FillDriver::focus`] means focus moved.
pub(crate) struct WindowsDriver<'a> {
    pub(crate) uia: &'a Uia,
    pub(crate) pings: &'a Receiver<u32>,
}

impl WindowsDriver<'_> {
    /// Whether a focus event is waiting; takes every waiting one.
    fn drain(&self) -> bool {
        let mut any = false;
        while let Ok(pid) = self.pings.try_recv() {
            any |= pid != WAKE;
        }
        any
    }
}

fn target_of(session: &PickSession<WinHeld>) -> Result<&UiaElement, ProbeError> {
    session
        .held
        .element_for(&session.target)
        .ok_or(ProbeError::Gone)
}

impl FillDriver for WindowsDriver<'_> {
    type Held = WinHeld;

    fn focus(&self, session: &PickSession<WinHeld>) -> Result<(), ProbeError> {
        let element = target_of(session)?;
        // A refusal to take the foreground is not an error here: the check
        // before the write reads the foreground window and decides.
        shim::set_foreground(session.root).map_err(probe_error)?;
        self.uia.set_focus(element).map_err(probe_error)?;
        std::thread::sleep(Duration::from_millis(FOCUS_SETTLE_MS));
        // Events caused by this very call are not "focus moved".
        self.drain();
        Ok(())
    }

    fn focus_changed(&self) -> bool {
        self.drain()
    }

    fn can_set_value(&self, session: &PickSession<WinHeld>) -> bool {
        target_of(session).is_ok_and(|element| {
            let facts = element.facts();
            SETVALUE_FRAMEWORKS.contains(&facts.framework_id.as_str())
                && facts.has_value_pattern
                && !facts.value_read_only
                && !facts.is_password
        })
    }

    fn set_value(&self, session: &PickSession<WinHeld>, value: &str) -> Result<bool, ProbeError> {
        let element = target_of(session)?;
        self.uia.set_value(element, value).map_err(probe_error)?;
        let read_back = self.uia.value(element).map_err(probe_error)?;
        Ok(read_back == value)
    }

    fn send_units(&self, units: &[u16]) -> Result<u32, ProbeError> {
        shim::send_unicode(units).map_err(probe_error)
    }

    fn send_key(&self, key: Key) -> Result<u32, ProbeError> {
        shim::send_key(match key {
            Key::Enter => shim::Key::Enter,
            Key::SelectAll => shim::Key::SelectAll,
            Key::Backspace => shim::Key::Backspace,
        })
        .map_err(probe_error)
    }

    fn modifiers_down(&self) -> bool {
        shim::modifiers_down()
    }
}
