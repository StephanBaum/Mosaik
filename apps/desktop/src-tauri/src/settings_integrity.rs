//! Settings integrity (CRYPTO_SPEC §11.1, ADR-0030): what the backend does
//! with the MAC in `settings.json`.
//!
//! The state file is untrusted input. While the vault is locked there is no
//! key, so only values at least as strict as the defaults count
//! ([`crate::persist::stricter_than_default`]) and the SSH agent reads as
//! off. The first unlock after the start checks the MAC (which covers the
//! settings, the SSH flag, the account email and kit flag, the account ID and
//! the store's local-only settings nonce) and either applies the file or
//! keeps the strict values, switches the agent off, forgets the email and the
//! kit flag, rewrites the file with a fresh nonce and MAC and, if a
//! protection differed, tells the user once (`settingsReset`). Every save
//! while unlocked replaces the nonce and writes the MAC of what it writes (so
//! a restored older file fails); a save while locked writes the file's
//! protected section back untouched.

use mosaic_core::{SettingsMacKey, new_settings_nonce};
use mosaic_vault::session::UnlockedSession;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::settings::Settings;
use crate::persist::{
    FileSettings, MAC_LEN, StoredState, canonical_settings, same_protection, stricter_than_default,
};
use crate::state::{Backend, Slot, guard};

/// What the backend knows about the protected part of the file.
#[derive(Default)]
pub struct SettingsGuard {
    /// The protected part as the file holds it (what a locked save writes
    /// back unchanged).
    on_disk: Option<StoredState>,
    /// The MAC the file holds with it.
    mac: Option<[u8; MAC_LEN]>,
    /// The file was read at the start and its MAC is not checked yet.
    unchecked: bool,
    /// The user switched the SSH agent off while locked, before the check:
    /// the file's `true` must not bring it back.
    ssh_off: bool,
    /// `K_settings`, only while unlocked and after the check.
    key: Option<SettingsMacKey>,
}

impl SettingsGuard {
    /// The guard for a file read at the start; `None`: no usable file.
    #[must_use]
    pub fn new(file: Option<FileSettings>) -> Self {
        match file {
            Some(file) => Self {
                on_disk: Some(file.state),
                mac: file.mac,
                unchecked: true,
                ssh_off: false,
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

/// What the check decided.
struct Checked {
    /// The MAC did not verify (a rewrite is due whatever the values are).
    invalid: bool,
    /// The user must be told that settings were reset.
    notice: bool,
}

/// Puts the file's protection values into `stored` when its MAC `valid`ated
/// (the theme and language of this run stay), and the strict ones when not.
fn adopt_file(stored: &mut StoredState, file: StoredState, valid: bool, ssh_off: bool) -> Checked {
    if valid {
        let wanted = crate::persist::normalize(file.settings);
        stored.settings = Settings {
            theme: stored.settings.theme,
            language: stored.settings.language,
            ..wanted
        };
        stored.ssh_agent_enabled = file.ssh_agent_enabled && !ssh_off;
        return Checked {
            invalid: false,
            notice: false,
        };
    }
    // The strict values are in place already; tell the user if the file
    // asked for something weaker or for the SSH agent.
    let strict = stricter_than_default(&stored.settings);
    let notice = !same_protection(&crate::persist::normalize(file.settings), &strict)
        || file.ssh_agent_enabled;
    stored.settings = strict;
    stored.ssh_agent_enabled = false;
    if let Some(account) = stored.account.as_mut() {
        account.email = None;
        account.kit_verified = false;
    }
    Checked {
        invalid: true,
        notice,
    }
}

impl Backend {
    /// Whether the settings MAC key is available, that is, the vault is
    /// unlocked.
    #[must_use]
    pub fn settings_key_available(&self) -> bool {
        self.settings_guard.lock().is_ok_and(|g| g.has_key())
    }

    /// Notes that the SSH agent was switched off while the key was not
    /// available (before the first check the file's `true` must not win).
    pub(crate) fn note_ssh_off(&self) {
        if let Ok(mut g) = self.settings_guard.lock()
            && g.key.is_none()
        {
            g.ssh_off = true;
        }
    }

    /// Runs after every transition to unlocked: derives `K_settings`, checks
    /// the file's MAC once per run and writes the state with a fresh nonce
    /// and MAC. Errors leave the strict values in place (a failed derivation
    /// cannot happen for 32 bytes; a failed write is the usual
    /// `storeUnavailable` that the next save reports).
    pub(crate) fn settings_unlocked(&self, slot: &mut Slot) {
        if self.check_settings(slot).unwrap_or(false) {
            self.emit(&AppEvent::SettingsReset);
        }
    }

    /// Returns whether the user must be told that settings were reset.
    fn check_settings(&self, slot: &mut Slot) -> IpcResult<bool> {
        let (key, account_id, nonce) = {
            let Slot::Unlocked(session) = &*slot else {
                return Ok(false);
            };
            let session: &UnlockedSession = session;
            (
                session.settings_mac_key().map_err(|_| IpcError::INTERNAL)?,
                session.account_id(),
                session.settings_nonce().map_err(|_| IpcError::INTERNAL)?,
            )
        };
        let mut checked = Checked {
            invalid: false,
            notice: false,
        };
        let in_sync = {
            let mut g = guard(&self.settings_guard)?;
            if g.unchecked {
                g.unchecked = false;
                let ssh_off = std::mem::take(&mut g.ssh_off);
                let file = g.on_disk.clone().unwrap_or_default();
                let valid = nonce.zip(g.mac).is_some_and(|(nonce, mac)| {
                    key.verify(&account_id, &nonce, &canonical_settings(&file), &mac)
                });
                let mut stored = guard(&self.stored)?;
                checked = adopt_file(&mut stored, file, valid, ssh_off);
            }
            let stored = guard(&self.stored)?;
            let current = g.on_disk.as_ref() == Some(&*stored) && g.mac.is_some();
            g.key = Some(key);
            current
        };
        if checked.invalid || !in_sync {
            // Fresh nonce and MAC for what the app now holds (also writes the
            // file when none existed, and replaces a MAC that did not verify
            // even when the values are the strict ones).
            let _ = self.update_stored_in(slot, |_| {});
        }
        Ok(checked.notice)
    }

    /// After a re-key (or the adoption of one) `K_settings` is another key:
    /// derives it again from the session's new account key and writes the state
    /// file with a fresh nonce and MAC, before any other settings read. A crash
    /// before this leaves a file the MAC rejects (strict values and a notice).
    pub(crate) fn settings_remac(&self) {
        let Ok(mut slot) = self.slot() else {
            return;
        };
        let key = match &*slot {
            Slot::Unlocked(session) => session.settings_mac_key().ok(),
            _ => None,
        };
        let Some(key) = key else {
            return;
        };
        if let Ok(mut g) = self.settings_guard.lock() {
            g.key = Some(key);
        }
        let _ = self.update_stored_in(&mut slot, |_| {});
    }

    /// Forgets `K_settings` (every lock).
    pub(crate) fn settings_locked(&self) {
        if let Ok(mut g) = self.settings_guard.lock() {
            g.key = None;
        }
    }

    /// Writes the state file: with a fresh nonce and the MAC of the state
    /// while the key is available and the session unlocked, otherwise with
    /// the file's own protected section and MAC.
    pub(crate) fn save_state(&self, slot: &mut Slot, snapshot: &StoredState) -> IpcResult<()> {
        let mut g = guard(&self.settings_guard)?;
        if let (Some(key), Slot::Unlocked(session)) = (&g.key, &mut *slot) {
            let store_error = |_| IpcError::new(ErrorCode::StoreUnavailable);
            let account_id = session.account_id();
            let old = session.settings_nonce().map_err(store_error)?;
            let nonce = new_settings_nonce().map_err(|_| IpcError::INTERNAL)?;
            let mac = key
                .tag(&account_id, &nonce, &canonical_settings(snapshot))
                .map_err(|_| IpcError::INTERNAL)?;
            // The nonce first: a crash between the two steps leaves a file
            // the MAC rejects (strict values and a notice), never one that
            // verifies against a stale nonce.
            session.set_settings_nonce(&nonce).map_err(store_error)?;
            if let Err(error) = self.persistence.save(snapshot, Some(&mac)) {
                if let Some(old) = old {
                    let _ = session.set_settings_nonce(&old);
                }
                return Err(error);
            }
            g.on_disk = Some(snapshot.clone());
            g.mac = Some(mac);
        } else {
            let file_state = g.on_disk.clone().unwrap_or_else(|| snapshot.clone());
            self.persistence.save(&file_state, g.mac.as_ref())?;
            if g.on_disk.is_none() {
                g.on_disk = Some(file_state);
            }
        }
        Ok(())
    }
}
