//! App settings. Not secret; stored next to the vault in plaintext by the
//! settings track. Turning auto-lock off needs re-authentication
//! (UX_GUIDELINES §8), which `update_settings` enforces in Rust.

use serde::{Deserialize, Serialize};

use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// Color theme.
    pub enum Theme {
        /// Follow the OS (the default).
        System,
        /// Light.
        Light,
        /// Dark.
        Dark,
    }
);

wire_enum!(
    /// UI language (ADR-0168): the locales of `packages/i18n`. Rust's own texts (the native
    /// dialogs in `sync::strings`) exist in English and German only; every other language
    /// shows them in English.
    pub enum Language {
        /// Follow the OS (the default).
        System,
        /// English.
        En,
        /// German.
        De,
        /// Spanish.
        Es,
        /// French.
        Fr,
        /// Italian.
        It,
        /// Portuguese (Brazilian).
        Pt,
        /// Dutch.
        Nl,
        /// Polish.
        Pl,
        /// Turkish.
        Tr,
        /// Japanese.
        Ja,
        /// Korean.
        Ko,
        /// Chinese, Simplified.
        Zh,
    }
);

wire_enum!(
    /// How the opt-in breach check runs (CRYPTO_SPEC §12, ADR-0065).
    pub enum BreachMode {
        /// Never (the default): no request is ever made.
        Off,
        /// Only when the user presses "Check now".
        Manual,
        /// Also 60 seconds after an unlock when the last scan is a week old.
        Weekly,
    }
);

/// Every setting. `update_settings` takes the whole object and returns what
/// was stored (values are clamped to their ranges).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one switch per setting on the wire; a state machine would only rename them"
)]
pub struct Settings {
    /// Lock after this many idle minutes (1 to 480; `null`: never).
    pub auto_lock_minutes: Option<u32>,
    /// Lock when the system sleeps.
    pub lock_on_sleep: bool,
    /// Lock when the screen locks.
    pub lock_on_screen_lock: bool,
    /// Clear the clipboard after this many seconds (10 to 300; `null`: never,
    /// with a warning in the UI).
    pub clipboard_clear_seconds: Option<u32>,
    /// Color theme.
    pub theme: Theme,
    /// UI language.
    pub language: Language,
    /// How many days a biometric quick-unlock enrollment is used before the
    /// master password is needed again (1 to 30). Raising it needs the
    /// master password (CRYPTO_SPEC §6.2).
    pub quick_unlock_max_age_days: u32,
    /// The breach check (M7b). Under the settings MAC: a file edited outside
    /// Mosaic reads as `off`. Not `off` only together with `breachConsentAt`.
    pub breach_check: BreachMode,
    /// When the user agreed to the breach check (Unix ms); `null` exactly
    /// when `breachCheck` is `off`. Setting it is the consent.
    pub breach_consent_at: Option<i64>,
    /// Show "n passwords to change" after an unlock (default on). No
    /// request is made for it.
    pub rotation_reminders: bool,
    /// The global shortcut that opens the palette, as an accelerator such as
    /// `Ctrl+Shift+Space` (M3-R10). Validated in Rust by
    /// `shortcut::parse_accelerator`; not a protection setting, so it
    /// changes while locked too.
    pub global_shortcut: String,
    /// Offer logins in other Windows programs (ADR-0164). Off by default and under the
    /// settings MAC: a file edited outside Mosaic reads as `false`. Windows only.
    pub program_fill: bool,
    /// Press Enter after filling a program's login (default on; a binding can turn it off).
    /// Under the settings MAC together with `programFill`.
    pub program_fill_submit: bool,
    /// The global shortcut that opens the program fill list (default: none). Validated like
    /// `globalShortcut`; must differ from it.
    pub program_fill_hotkey: Option<String>,
    /// The user dismissed or answered the first-unlock card about program fill.
    pub program_fill_card_dismissed: bool,
}

/// The default global shortcut of this OS, as an accelerator string.
#[must_use]
pub fn default_global_shortcut() -> String {
    if cfg!(target_os = "macos") {
        "Alt+Super+Space".to_owned()
    } else {
        "Ctrl+Shift+Space".to_owned()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_lock_minutes: Some(10),
            lock_on_sleep: true,
            lock_on_screen_lock: true,
            clipboard_clear_seconds: Some(30),
            theme: Theme::System,
            language: Language::System,
            quick_unlock_max_age_days: 14,
            breach_check: BreachMode::Off,
            breach_consent_at: None,
            rotation_reminders: true,
            global_shortcut: default_global_shortcut(),
            program_fill: false,
            program_fill_submit: true,
            program_fill_hotkey: None,
            program_fill_card_dismissed: false,
        }
    }
}

/// `update_settings`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSettingsRequest {
    /// The new settings.
    pub settings: Settings,
    /// The master password. Required (else `reauthRequired`) when the change
    /// turns auto-lock or clipboard clearing off, or raises the quick-unlock
    /// maximum age; `null` otherwise.
    pub password: Option<SecretIn>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn settings() -> Vec<Settings> {
        vec![
            Settings {
                // The default of a Windows or Linux desktop, so the fixture
                // is the same on every OS.
                global_shortcut: "Ctrl+Shift+Space".into(),
                ..Settings::default()
            },
            Settings {
                auto_lock_minutes: None,
                lock_on_sleep: false,
                lock_on_screen_lock: false,
                clipboard_clear_seconds: None,
                theme: Theme::Dark,
                language: Language::De,
                quick_unlock_max_age_days: 30,
                breach_check: BreachMode::Weekly,
                breach_consent_at: Some(1_761_000_000_000),
                rotation_reminders: false,
                global_shortcut: "Ctrl+Alt+K".into(),
                program_fill: true,
                program_fill_submit: false,
                program_fill_hotkey: Some("Ctrl+Alt+L".into()),
                program_fill_card_dismissed: true,
            },
        ]
    }
}
