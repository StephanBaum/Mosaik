//! The two picker windows of program fill (ADR-0164) as the service sees them: the chip next to
//! the field and the list under it. The service decides what is shown and where; the windows
//! only show it. The Tauri implementation is in the same module (`TauriPickerWindows`).

use mosaic_program_fill::verify::{Point, Rect};

/// Which picker window.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PickerLabel {
    /// The small button at the field (never takes focus).
    Chip,
    /// The list, the link step and the notices.
    List,
}

/// What the service asks of the picker windows.
pub trait PickerWindows: Send + Sync {
    /// Shows the chip at `at` without activating it; `false` when it could not be shown.
    fn show_chip(&self, at: Rect) -> bool;
    /// Shows the list at `at`; with `focus` it takes the keyboard focus, without it (a notice)
    /// it does not.
    fn show_list(&self, at: Rect, focus: bool) -> bool;
    /// Hides a window.
    fn hide(&self, which: PickerLabel);
    /// Where the window is now, in physical pixels (`None` when hidden).
    fn placed(&self, which: PickerLabel) -> Option<Rect>;
    /// The native handle of the window.
    fn hwnd(&self, which: PickerLabel) -> Option<isize>;
    /// Whether the window is the foreground window of the system.
    fn is_foreground(&self, which: PickerLabel) -> bool;
    /// The monitor that holds `p`: its rectangle and scale factor.
    fn monitor_at(&self, p: Point) -> Option<(Rect, f64)>;
    /// Tells the window its state changed (a new generation).
    fn changed(&self, which: PickerLabel, generation: u64);
    /// Brings the main window forward (the unlock screen).
    fn focus_main(&self);
}

/// No picker windows: program fill is off or this is not Windows.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoPickerWindows;

impl PickerWindows for NoPickerWindows {
    fn show_chip(&self, _at: Rect) -> bool {
        false
    }

    fn show_list(&self, _at: Rect, _focus: bool) -> bool {
        false
    }

    fn hide(&self, _which: PickerLabel) {}

    fn placed(&self, _which: PickerLabel) -> Option<Rect> {
        None
    }

    fn hwnd(&self, _which: PickerLabel) -> Option<isize> {
        None
    }

    fn is_foreground(&self, _which: PickerLabel) -> bool {
        false
    }

    fn monitor_at(&self, _p: Point) -> Option<(Rect, f64)> {
        None
    }

    fn changed(&self, _which: PickerLabel, _generation: u64) {}

    fn focus_main(&self) {}
}
