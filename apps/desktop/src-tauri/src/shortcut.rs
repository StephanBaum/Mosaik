//! The global shortcut (brief §6: "global shortcut opens a command
//! palette"): Ctrl+Shift+Space on Windows and Linux, Option+Command+Space
//! on macOS, registered from Rust through `tauri-plugin-global-shortcut`.
//! The plugin's JavaScript commands stay denied: no capability grants a
//! `global-shortcut:*` permission.
//!
//! The shortcut shows and focuses the main window (there is no second
//! webview, so no second capability that could hand out secrets). When the
//! vault is locked the main window shows the unlock screen. When unlocked,
//! [`on_pressed`] also emits `quickSearchRequested`, which opens the palette.
//!
//! The plugin is installed in `setup` through [`install`], so a failure to
//! start the hotkey manager (pure Wayland without X, for example) leaves
//! the app running and shows up as [`ShortcutStatus::Unavailable`].
//!
//! Known gaps:
//!
//! - Wayland: global hotkeys work through X11/XWayland only; on a pure
//!   Wayland session the status is `Unavailable` or the key never arrives.
//! - macOS: Option+Command+Space opens a Finder search by default. The
//!   system does not report that combination as taken, so the status can
//!   say `Registered` while Finder still wins.
//! - The combination cannot be remapped yet.

use tauri::{AppHandle, Manager as _, Runtime};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt as _, Modifiers, Shortcut, ShortcutState,
};

use crate::contract::events::AppEvent;
use crate::state::AppState;

/// Whether the shortcut is ours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShortcutStatus {
    /// Registered; pressing it reaches Mosaic.
    Registered,
    /// Another program holds the combination.
    Conflict,
    /// The hotkey manager could not start on this system.
    Unavailable,
}

/// The shortcut of this OS.
#[must_use]
pub fn shortcut() -> Shortcut {
    if cfg!(target_os = "macos") {
        Shortcut::new(Some(Modifiers::ALT | Modifiers::SUPER), Code::Space)
    } else {
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)
    }
}

/// Installs the plugin and registers the shortcut. Never fails the app.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> ShortcutStatus {
    if app
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .is_err()
    {
        return ShortcutStatus::Unavailable;
    }
    let registered = app
        .global_shortcut()
        .on_shortcut(shortcut(), |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                on_pressed(app);
            }
        });
    if registered.is_ok() {
        ShortcutStatus::Registered
    } else {
        ShortcutStatus::Conflict
    }
}

/// What the shortcut does: show the main window, and count the press as
/// activity when unlocked.
pub fn on_pressed<R: Runtime>(app: &AppHandle<R>) {
    focus_main(app);
    if let Some(state) = app.try_state::<AppState>()
        && state.backend().is_unlocked()
    {
        state.backend().touch();
        state.backend().emit(&AppEvent::QuickSearchRequested);
    }
}

/// Shows the main window, which shows the unlock screen when locked.
pub fn focus_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        // Best effort: a window that cannot be raised stays where it is.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
