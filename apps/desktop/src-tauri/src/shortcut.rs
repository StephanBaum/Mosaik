//! The global shortcut (brief §6: "global shortcut opens a command
//! palette"): Ctrl+Shift+Space on Windows and Linux, Option+Command+Space
//! on macOS by default, registered from Rust through
//! `tauri-plugin-global-shortcut`. The plugin's JavaScript commands stay
//! denied: no capability grants a `global-shortcut:*` permission.
//!
//! The shortcut shows and focuses the main window (there is no second
//! webview, so no second capability that could hand out secrets). When the
//! vault is locked the main window shows the unlock screen. When unlocked,
//! [`on_pressed`] also emits `quickSearchRequested`, which opens the palette.
//!
//! The plugin is installed in `setup` through [`install`], so a failure to
//! start the hotkey manager leaves the app running and shows up as
//! [`ShortcutStatus::Unavailable`].
//!
//! The combination is `settings.globalShortcut` (M9, M3-R10): validated by
//! [`parse_accelerator`] and re-registered by the [`Registrar`] when it
//! changes.
//!
//! Known gaps:
//!
//! - Wayland: no global hotkeys; the status is
//!   [`ShortcutStatus::Unsupported`] and the settings field is disabled.
//! - macOS: Option+Command+Space opens a Finder search by default. The
//!   system does not report that combination as taken, so the status can
//!   say `Registered` while Finder still wins.

use std::str::FromStr as _;
use std::sync::{Arc, Mutex};

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
    /// This system does not allow global shortcuts (Wayland): the settings
    /// field is disabled.
    Unsupported,
}

/// Why an accelerator string is refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ShortcutError {
    /// Empty, or longer than [`MAX_ACCELERATOR_LEN`] characters.
    #[error("the shortcut is empty or too long")]
    Length,
    /// Not a combination of modifiers and one key (for example
    /// `Ctrl+Shift+Space`).
    #[error("the shortcut is not a combination of modifiers and one key")]
    Syntax,
    /// No Ctrl, Alt or Command/Super in it: a bare key or Shift plus a key
    /// would take typing away from every program.
    #[error("the shortcut needs Ctrl, Alt or Command")]
    NoModifier,
}

/// The longest accelerator text accepted.
pub const MAX_ACCELERATOR_LEN: usize = 64;

/// A validated accelerator: modifiers (at least one of Ctrl, Alt, Command or
/// Super) and one key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Accelerator(Shortcut);

impl Accelerator {
    /// The shortcut to register.
    #[must_use]
    pub const fn shortcut(self) -> Shortcut {
        self.0
    }
}

/// Parses and validates an accelerator such as `Ctrl+Shift+Space`
/// (`settings.globalShortcut`, M3-R10).
///
/// # Errors
/// [`ShortcutError`].
pub fn parse_accelerator(text: &str) -> Result<Accelerator, ShortcutError> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_ACCELERATOR_LEN {
        return Err(ShortcutError::Length);
    }
    if !text.is_ascii() || text.chars().any(char::is_control) {
        return Err(ShortcutError::Syntax);
    }
    let shortcut = Shortcut::from_str(text).map_err(|_| ShortcutError::Syntax)?;
    if !shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err(ShortcutError::NoModifier);
    }
    Ok(Accelerator(shortcut))
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

/// The default accelerator of this OS (what [`shortcut`] is).
#[must_use]
pub fn default_accelerator() -> Accelerator {
    Accelerator(shortcut())
}

/// Whether the session is Wayland, where global shortcuts do not work
/// (`XDG_SESSION_TYPE=wayland`, or a Wayland display, on Linux).
#[must_use]
pub fn is_wayland(session_type: Option<&str>, wayland_display: Option<&str>) -> bool {
    cfg!(target_os = "linux")
        && (session_type.is_some_and(|t| t.eq_ignore_ascii_case("wayland"))
            || wayland_display.is_some_and(|d| !d.is_empty()))
}

/// Registers a new accelerator in place of the registered one; the answer is
/// the status of the new one (the old one stays when it fails).
pub type Registrar = Arc<dyn Fn(Accelerator) -> ShortcutStatus + Send + Sync>;

/// Installs the plugin and registers `accelerator`. Never fails the app. The
/// registrar re-registers when the setting changes.
pub fn install<R: Runtime>(
    app: &AppHandle<R>,
    accelerator: Accelerator,
) -> (ShortcutStatus, Option<Registrar>) {
    let wayland = is_wayland(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
    );
    if wayland {
        return (ShortcutStatus::Unsupported, None);
    }
    if app
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .is_err()
    {
        return (ShortcutStatus::Unavailable, None);
    }
    let registrar = registrar(app.clone());
    let status = registrar(accelerator);
    (status, Some(registrar))
}

fn registrar<R: Runtime>(app: AppHandle<R>) -> Registrar {
    let current: Mutex<Option<Shortcut>> = Mutex::new(None);
    Arc::new(move |wanted| {
        let Ok(mut held) = current.lock() else {
            return ShortcutStatus::Unavailable;
        };
        let new = wanted.shortcut();
        if *held == Some(new) {
            return ShortcutStatus::Registered;
        }
        let registered = app.global_shortcut().on_shortcut(new, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                on_pressed(app);
            }
        });
        if registered.is_err() {
            return ShortcutStatus::Conflict;
        }
        if let Some(old) = held.replace(new) {
            // Best effort: a failed unregister leaves a second combination
            // that opens the window, which does no harm.
            let _ = app.global_shortcut().unregister(old);
        }
        ShortcutStatus::Registered
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_accelerators_parse() {
        for text in [
            "Ctrl+Shift+Space",
            "Alt+Super+Space",
            "ctrl+alt+k",
            "Cmd+Shift+P",
        ] {
            assert!(parse_accelerator(text).is_ok(), "{text}");
        }
        let parsed = parse_accelerator("Ctrl+Shift+Space").unwrap().shortcut();
        assert!(parsed.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::Space));
    }

    #[test]
    fn surrounding_space_is_ignored() {
        assert_eq!(
            parse_accelerator("  Ctrl+Alt+K ").unwrap(),
            parse_accelerator("Ctrl+Alt+K").unwrap()
        );
    }

    #[test]
    fn empty_and_overlong_text_is_refused() {
        assert_eq!(parse_accelerator(""), Err(ShortcutError::Length));
        assert_eq!(parse_accelerator("   "), Err(ShortcutError::Length));
        let long = format!("Ctrl+{}", "A".repeat(MAX_ACCELERATOR_LEN));
        assert_eq!(parse_accelerator(&long), Err(ShortcutError::Length));
    }

    #[test]
    fn broken_syntax_is_refused() {
        for text in [
            "Ctrl+",
            "+K",
            "Ctrl+K+L",
            "Ctrl+Banana",
            "Ctrl++K",
            "Strg+\u{c4}",
            "Ctrl+K\u{0}",
        ] {
            assert_eq!(
                parse_accelerator(text),
                Err(ShortcutError::Syntax),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_shortcut_needs_ctrl_alt_or_command() {
        for text in ["K", "Space", "F5", "Shift+K", "Shift+Space"] {
            assert_eq!(
                parse_accelerator(text),
                Err(ShortcutError::NoModifier),
                "{text}"
            );
        }
        assert!(parse_accelerator("Ctrl+K").is_ok());
        assert!(parse_accelerator("Alt+F5").is_ok());
    }

    #[test]
    fn wayland_is_told_by_the_session() {
        let linux = cfg!(target_os = "linux");
        assert_eq!(is_wayland(Some("wayland"), None), linux);
        assert_eq!(is_wayland(Some("Wayland"), None), linux);
        assert_eq!(is_wayland(None, Some("wayland-0")), linux);
        assert!(!is_wayland(Some("x11"), None));
        assert!(!is_wayland(None, Some("")));
        assert!(!is_wayland(None, None));
    }
}
