//! Settings commands. Settings are stored next to the vault in the state
//! file (`persist`), clamped to their ranges, and turning a protection off
//! needs the master password again (UX_GUIDELINES §8).

use tauri::State;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::persist::{has_lock_trigger, normalize};
use crate::state::{AppState, Backend};

/// `get_settings`.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> IpcResult<Settings> {
    Ok(state.backend().settings_snapshot())
}

/// `update_settings`.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    request: UpdateSettingsRequest,
) -> IpcResult<Settings> {
    state.run(move |b| b.update_settings(request)).await
}

/// Whether going from `old` to `new` turns a protection off: auto-lock on
/// idle, on sleep or on screen lock, or clipboard clearing. The contract
/// names auto-lock and clipboard clearing; the sleep and screen-lock
/// switches are part of auto-lock here, the conservative reading.
#[must_use]
pub fn weakens(old: &Settings, new: &Settings) -> bool {
    (old.auto_lock_minutes.is_some() && new.auto_lock_minutes.is_none())
        || (old.clipboard_clear_seconds.is_some() && new.clipboard_clear_seconds.is_none())
        || (old.lock_on_sleep && !new.lock_on_sleep)
        || (old.lock_on_screen_lock && !new.lock_on_screen_lock)
}

impl Backend {
    /// Stores new settings (clamped). A change that turns a protection off
    /// needs `password`, checked against the vault.
    ///
    /// # Errors
    /// `invalidInput` when no lock trigger this system can deliver is on (CRYPTO_SPEC §11; screen lock counts only where `screenLockDetection`); `reauthRequired` without a password for a weakening change; `locked`
    /// for a weakening change while locked; `wrongCredentials` (the vault is
    /// then locked, see [`Backend::reauthenticate`]); `storeUnavailable` if
    /// the file cannot be written (the settings apply anyway).
    pub fn update_settings(&self, request: UpdateSettingsRequest) -> IpcResult<Settings> {
        if !has_lock_trigger(&request.settings, self.screen_lock_detection()) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let new = normalize(request.settings);
        let old = self.settings_snapshot();
        if weakens(&old, &new) {
            let password = request
                .password
                .ok_or(IpcError::new(ErrorCode::ReauthRequired))?;
            if password.expose_secret().is_empty() {
                return Err(IpcError::new(ErrorCode::ReauthRequired));
            }
            self.reauthenticate(password)?;
        }
        self.touch();
        let stored = self.update_stored(|stored| stored.settings = new.clone());
        stored.map(|s| s.settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_turning_a_protection_off_weakens() {
        let on = Settings::default();
        assert!(!weakens(&on, &on));
        let mut longer = on.clone();
        longer.auto_lock_minutes = Some(480);
        longer.clipboard_clear_seconds = Some(300);
        assert!(!weakens(&on, &longer), "longer timeouts stay allowed");
        for change in [
            |s: &mut Settings| s.auto_lock_minutes = None,
            |s: &mut Settings| s.clipboard_clear_seconds = None,
            |s: &mut Settings| s.lock_on_sleep = false,
            |s: &mut Settings| s.lock_on_screen_lock = false,
        ] {
            let mut off = on.clone();
            change(&mut off);
            assert!(weakens(&on, &off));
            assert!(!weakens(&off, &on), "turning it back on is free");
        }
    }
}
