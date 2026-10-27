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
    /// UI language.
    pub enum Language {
        /// Follow the OS (the default).
        System,
        /// English.
        En,
        /// German.
        De,
    }
);

/// Every setting. `update_settings` takes the whole object and returns what
/// was stored (values are clamped to their ranges).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    /// turns auto-lock or clipboard clearing off; `null` otherwise.
    pub password: Option<SecretIn>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn settings() -> Vec<Settings> {
        vec![
            Settings::default(),
            Settings {
                auto_lock_minutes: None,
                lock_on_sleep: false,
                lock_on_screen_lock: false,
                clipboard_clear_seconds: None,
                theme: Theme::Dark,
                language: Language::De,
            },
        ]
    }
}
