//! Settings integrity (CRYPTO_SPEC §11.1, ADR-0030): what the backend does
//! with the MAC in `settings.json`.
//!
//! The state file is untrusted input. While the vault is locked there is no
//! key, so only values at least as strict as the defaults count
//! ([`crate::persist::stricter_than_default`]). The first unlock after the
//! start checks the MAC and either applies the file or keeps the strict
//! values, rewrites the file with a fresh MAC and tells the user once
//! (`settingsReset`). Every save while unlocked writes the MAC of what it
//! writes; a save while locked writes the file's settings back untouched.

use mosaic_core::SettingsMacKey;
use mosaic_vault::session::UnlockedSession;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::settings::Settings;
use crate::persist::{
    FileSettings, MAC_LEN, StoredState, canonical_settings, same_protection, stricter_than_default,
};
use crate::state::{Backend, Slot, guard};

/// What the backend knows about the settings section of the file.
#[derive(Default)]
pub struct SettingsGuard {
    /// The settings section as the file holds it (what a locked save writes
    /// back unchanged).
    on_disk: Option<Settings>,
    /// The MAC the file holds with it.
    mac: Option<[u8; MAC_LEN]>,
    /// The file was read at the start and its MAC is not checked yet.
    unchecked: bool,
    /// `K_settings`, only while unlocked and after the check.
    key: Option<SettingsMacKey>,
}

impl SettingsGuard {
    /// The guard for a file read at the start; `None`: no usable file.
    #[must_use]
    pub fn new(file: Option<FileSettings>) -> Self {
        match file {
            Some(file) => Self {
                on_disk: Some(file.settings),
                mac: file.mac,
                unchecked: true,
                key: None,
            },
            None => Self::default(),
        }
    }

    /// Whether the key is available (unlocked and checked).
    #[must_use]
    pub const fn has_key(&self) -> bool {
        self.key.is_some()
    }
}

impl Backend {
    /// Whether the settings MAC key is available, that is, the vault is
    /// unlocked.
    #[must_use]
    pub fn settings_key_available(&self) -> bool {
        self.settings_guard.lock().is_ok_and(|g| g.has_key())
    }

    /// Runs after every transition to unlocked: derives `K_settings`, checks
    /// the file's MAC once per run and writes the settings with a MAC. Errors
    /// leave the strict values in place (a failed derivation cannot happen
    /// for 32 bytes; a failed write is the usual `storeUnavailable` that the
    /// next save reports).
    pub(crate) fn settings_unlocked(&self, slot: &Slot) {
        let Slot::Unlocked(session) = slot else {
            return;
        };
        if self.check_settings(session).unwrap_or(false) {
            self.emit(&AppEvent::SettingsReset);
        }
    }

    /// Returns whether the user must be told that settings were reset.
    fn check_settings(&self, session: &UnlockedSession) -> IpcResult<bool> {
        let key = session.settings_mac_key().map_err(|_| IpcError::INTERNAL)?;
        let mut notice = false;
        let in_sync = {
            let mut g = guard(&self.settings_guard)?;
            if g.unchecked {
                g.unchecked = false;
                let file = g.on_disk.clone().unwrap_or_default();
                let valid = g
                    .mac
                    .as_ref()
                    .is_some_and(|mac| key.verify(&canonical_settings(&file), mac));
                let mut stored = guard(&self.stored)?;
                if valid {
                    // Apply the file's protection values; the theme and
                    // language of this run stay.
                    let wanted = crate::persist::normalize(file);
                    stored.settings = Settings {
                        theme: stored.settings.theme,
                        language: stored.settings.language,
                        ..wanted
                    };
                } else {
                    // The strict values are in place already; tell the user
                    // if the file asked for something weaker.
                    let strict = stricter_than_default(&stored.settings);
                    notice = !same_protection(&crate::persist::normalize(file), &strict);
                    stored.settings = strict;
                }
            }
            let stored = guard(&self.stored)?;
            let current = g.on_disk.as_ref() == Some(&stored.settings) && g.mac.is_some();
            g.key = Some(key);
            current
        };
        if !in_sync {
            // Fresh MAC for what the app now holds (also writes the file
            // when none existed).
            let _ = self.update_stored(|_| {});
        }
        Ok(notice)
    }

    /// Forgets `K_settings` (every lock).
    pub(crate) fn settings_locked(&self) {
        if let Ok(mut g) = self.settings_guard.lock() {
            g.key = None;
        }
    }

    /// Writes the state file: with a fresh MAC of the settings while the key
    /// is available, otherwise with the file's own settings section and MAC.
    pub(crate) fn save_state(&self, snapshot: &StoredState) -> IpcResult<()> {
        let mut g = guard(&self.settings_guard)?;
        if let Some(key) = &g.key {
            let mac = key
                .tag(&canonical_settings(&snapshot.settings))
                .map_err(|_| IpcError::INTERNAL)?;
            self.persistence.save(snapshot, Some(&mac))?;
            g.on_disk = Some(snapshot.settings.clone());
            g.mac = Some(mac);
        } else {
            let mut file_state = snapshot.clone();
            if let Some(on_disk) = &g.on_disk {
                file_state.settings = on_disk.clone();
            }
            self.persistence.save(&file_state, g.mac.as_ref())?;
            if g.on_disk.is_none() {
                g.on_disk = Some(file_state.settings);
            }
        }
        Ok(())
    }
}
