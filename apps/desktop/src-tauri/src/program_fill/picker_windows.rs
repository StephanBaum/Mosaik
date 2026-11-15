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
    /// Destroys both windows (the vault locked, or program fill stopped), so no title or user
    /// name stays in a webview. They are built again on the next use.
    fn close_all(&self) {}
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

/// The Tauri windows of program fill (Windows only; elsewhere [`NoPickerWindows`] stands in).
///
/// The two windows are built on first use, hidden, without decorations, a shadow or a taskbar
/// entry, always on top and excluded from screen capture. The chip never takes focus. This is
/// the only place in the app that builds a window (a source test keeps it so).
#[cfg(windows)]
pub use tauri_windows::TauriPickerWindows;

#[cfg(windows)]
mod tauri_windows {
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use mosaic_program_fill::verify::{Point, Rect};
    use tauri::{
        AppHandle, Manager as _, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl,
        WebviewWindow, WebviewWindowBuilder,
    };

    use super::{PickerLabel, PickerWindows};
    use crate::commands::picker::{CHIP_LABEL, LIST_LABEL};

    /// The picker windows over a Tauri app.
    pub struct TauriPickerWindows<R: Runtime> {
        app: AppHandle<R>,
        /// Where each visible window was last put.
        placed: Mutex<HashMap<PickerLabel, Rect>>,
    }

    impl<R: Runtime> TauriPickerWindows<R> {
        /// The windows of `app` (none is built yet).
        #[must_use]
        pub fn new(app: AppHandle<R>) -> Self {
            Self {
                app,
                placed: Mutex::new(HashMap::new()),
            }
        }

        const fn label(which: PickerLabel) -> &'static str {
            match which {
                PickerLabel::Chip => CHIP_LABEL,
                PickerLabel::List => LIST_LABEL,
            }
        }

        fn existing(&self, which: PickerLabel) -> Option<WebviewWindow<R>> {
            self.app.get_webview_window(Self::label(which))
        }

        /// The window, built hidden on first use.
        fn window(&self, which: PickerLabel) -> Option<WebviewWindow<R>> {
            if let Some(window) = self.existing(which) {
                return Some(window);
            }
            let page = match which {
                PickerLabel::Chip => "chip.html",
                PickerLabel::List => "picker.html",
            };
            let mut builder = WebviewWindowBuilder::new(
                &self.app,
                Self::label(which),
                WebviewUrl::App(page.into()),
            )
            .title("Mosaic")
            .visible(false)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .shadow(false)
            .content_protected(true);
            if which == PickerLabel::Chip {
                builder = builder.focusable(false);
            }
            builder.build().ok()
        }

        fn placed_map(&self) -> MutexGuard<'_, HashMap<PickerLabel, Rect>> {
            self.placed.lock().unwrap_or_else(PoisonError::into_inner)
        }

        /// Puts the window at `at` (physical pixels).
        fn place(window: &WebviewWindow<R>, at: Rect) -> bool {
            let width = u32::try_from((at.right - at.left).max(1)).unwrap_or(1);
            let height = u32::try_from((at.bottom - at.top).max(1)).unwrap_or(1);
            window.set_size(PhysicalSize::new(width, height)).is_ok()
                && window
                    .set_position(PhysicalPosition::new(at.left, at.top))
                    .is_ok()
        }

        fn hwnd_of(window: &WebviewWindow<R>) -> Option<isize> {
            window
                .hwnd()
                .ok()
                .map(|handle| handle.0.addr().cast_signed())
        }
    }

    impl<R: Runtime> PickerWindows for TauriPickerWindows<R> {
        fn show_chip(&self, at: Rect) -> bool {
            let Some(window) = self.window(PickerLabel::Chip) else {
                return false;
            };
            let Some(hwnd) = Self::hwnd_of(&window) else {
                return false;
            };
            let shown = Self::place(&window, at) && mosaic_winshim::show_no_activate(hwnd).is_ok();
            if shown {
                self.placed_map().insert(PickerLabel::Chip, at);
            }
            shown
        }

        fn show_list(&self, at: Rect, focus: bool) -> bool {
            let Some(window) = self.window(PickerLabel::List) else {
                return false;
            };
            if !Self::place(&window, at) {
                return false;
            }
            let shown = if focus {
                window.show().is_ok() && window.set_focus().is_ok()
            } else {
                // A notice never takes the focus from the program the user is in.
                Self::hwnd_of(&window).is_some_and(|h| mosaic_winshim::show_no_activate(h).is_ok())
            };
            if shown {
                self.placed_map().insert(PickerLabel::List, at);
            }
            shown
        }

        fn hide(&self, which: PickerLabel) {
            if let Some(window) = self.existing(which) {
                // Best effort: a window that cannot be hidden is destroyed by the next lock.
                let _ = window.hide();
            }
            self.placed_map().remove(&which);
        }

        fn placed(&self, which: PickerLabel) -> Option<Rect> {
            let window = self.existing(which)?;
            if !window.is_visible().ok()? {
                return None;
            }
            let position = window.outer_position().ok()?;
            let size = window.outer_size().ok()?;
            let right = position.x.checked_add(i32::try_from(size.width).ok()?)?;
            let bottom = position.y.checked_add(i32::try_from(size.height).ok()?)?;
            Some(Rect {
                left: position.x,
                top: position.y,
                right,
                bottom,
            })
        }

        fn hwnd(&self, which: PickerLabel) -> Option<isize> {
            Self::hwnd_of(&self.existing(which)?)
        }

        fn is_foreground(&self, which: PickerLabel) -> bool {
            let hwnd = self.hwnd(which);
            hwnd.is_some() && mosaic_winshim::foreground_window() == hwnd
        }

        fn monitor_at(&self, p: Point) -> Option<(Rect, f64)> {
            let monitor = self
                .app
                .monitor_from_point(f64::from(p.x), f64::from(p.y))
                .ok()
                .flatten()?;
            let position = monitor.position();
            let size = monitor.size();
            let right = position.x.checked_add(i32::try_from(size.width).ok()?)?;
            let bottom = position.y.checked_add(i32::try_from(size.height).ok()?)?;
            Some((
                Rect {
                    left: position.x,
                    top: position.y,
                    right,
                    bottom,
                },
                monitor.scale_factor(),
            ))
        }

        fn changed(&self, _which: PickerLabel, _generation: u64) {}

        fn focus_main(&self) {
            crate::shortcut::focus_main(&self.app);
        }

        fn close_all(&self) {
            for which in [PickerLabel::Chip, PickerLabel::List] {
                if let Some(window) = self.existing(which) {
                    // Best effort: a window that stays is hidden and holds no secret.
                    let _ = window.destroy();
                }
            }
            self.placed_map().clear();
        }
    }
}
