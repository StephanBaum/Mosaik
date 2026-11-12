//! The settings file and its integrity (CRYPTO_SPEC §11.1, ADR-0030), ported
//! from the desktop's `settings_integrity.rs`.
//!
//! `settings.json` next to the store is untrusted input. While the vault is
//! locked there is no key, so only values at least as strict as the defaults
//! count. The first unlock after the start checks the MAC (over the settings,
//! the account ID and the store's local-only settings nonce): a valid file is
//! applied, an invalid one keeps the strict values and is rewritten with a
//! fresh nonce and MAC. Every save while unlocked replaces the nonce, so an
//! older copy of the file fails.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use mosaic_core::{SETTINGS_MAC_LEN, SettingsMacKey, new_settings_nonce};
use mosaic_vault::session::{QuickUnlockPolicy, UnlockedSession};
use serde::{Deserialize, Serialize};

use crate::Core;
use crate::error::{MobileError, Result};
use crate::state::Phase;

/// The format version of the file.
const FILE_VERSION: u32 = 1;
/// The largest file read.
const MAX_FILE_LEN: u64 = 64 * 1024;

/// The lock-on-background options, in seconds.
pub const BACKGROUND_LOCK_OPTIONS: [u32; 4] = [0, 30, 60, 300];
/// The idle-lock range, in minutes.
pub const IDLE_MINUTES: (u32, u32) = (1, 30);
/// The clipboard-clear range, in seconds.
pub const CLIPBOARD_SECONDS: (u32, u32) = (10, 300);
/// The quick-unlock maximum age range, in days (CRYPTO_SPEC §6.2).
pub const QUICK_UNLOCK_DAYS: (u32, u32) = (1, 30);

/// The settings the apps show. Lock policies cannot be turned off: every
/// value has a bounded range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileSettings {
    /// Lock this many seconds after the app goes to the background: 0, 30, 60
    /// or 300.
    pub auto_lock_background_seconds: u32,
    /// Lock after this many minutes without touch (1 to 30).
    pub idle_lock_minutes: u32,
    /// Clear the clipboard after this many seconds (10 to 300).
    pub clipboard_clear_seconds: u32,
    /// The maximum age of a quick-unlock enrolment, in days (1 to 30).
    pub quick_unlock_max_age_days: u32,
    /// iOS: offer passkeys through the system identity store.
    pub passkey_identities: bool,
    /// iOS: offer passwords through the system identity store (OD-M10-7).
    pub password_identities: bool,
}

impl Default for MobileSettings {
    fn default() -> Self {
        Self {
            auto_lock_background_seconds: 0,
            idle_lock_minutes: 5,
            clipboard_clear_seconds: 30,
            quick_unlock_max_age_days: 14,
            passkey_identities: false,
            password_identities: false,
        }
    }
}

impl MobileSettings {
    /// Clamps every value to its range (the lock-on-background value goes down
    /// to the next option, the stricter one).
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.auto_lock_background_seconds = BACKGROUND_LOCK_OPTIONS
            .iter()
            .rev()
            .copied()
            .find(|option| *option <= self.auto_lock_background_seconds)
            .unwrap_or(0);
        self.idle_lock_minutes = self.idle_lock_minutes.clamp(IDLE_MINUTES.0, IDLE_MINUTES.1);
        self.clipboard_clear_seconds = self
            .clipboard_clear_seconds
            .clamp(CLIPBOARD_SECONDS.0, CLIPBOARD_SECONDS.1);
        self.quick_unlock_max_age_days = self
            .quick_unlock_max_age_days
            .clamp(QUICK_UNLOCK_DAYS.0, QUICK_UNLOCK_DAYS.1);
        self
    }

    /// Per protection field, the stricter of these settings and the defaults:
    /// the shorter times, and the identity stores off. This is all an
    /// unverified file may set.
    #[must_use]
    pub fn stricter_than_default(self) -> Self {
        let default = Self::default();
        Self {
            auto_lock_background_seconds: self
                .auto_lock_background_seconds
                .min(default.auto_lock_background_seconds),
            idle_lock_minutes: self.idle_lock_minutes.min(default.idle_lock_minutes),
            clipboard_clear_seconds: self
                .clipboard_clear_seconds
                .min(default.clipboard_clear_seconds),
            quick_unlock_max_age_days: self
                .quick_unlock_max_age_days
                .min(default.quick_unlock_max_age_days),
            passkey_identities: false,
            password_identities: false,
        }
        .normalized()
    }

    /// The canonical bytes the MAC covers: one JSON object, keys in ascending
    /// byte order, no whitespace.
    fn canonical(&self) -> Vec<u8> {
        format!(
            "{{\"autoLockBackgroundSeconds\":{},\"clipboardClearSeconds\":{},\
             \"idleLockMinutes\":{},\"passkeyIdentities\":{},\"passwordIdentities\":{},\
             \"quickUnlockMaxAgeDays\":{}}}",
            self.auto_lock_background_seconds,
            self.clipboard_clear_seconds,
            self.idle_lock_minutes,
            self.passkey_identities,
            self.password_identities,
            self.quick_unlock_max_age_days,
        )
        .into_bytes()
    }
}

/// A change to the settings. `None` keeps a value. Raising the quick-unlock
/// age or turning the password identity store on needs the master password
/// (`reauth_password`).
#[derive(Default)]
pub struct SettingsUpdate {
    /// New lock-on-background seconds.
    pub auto_lock_background_seconds: Option<u32>,
    /// New idle-lock minutes.
    pub idle_lock_minutes: Option<u32>,
    /// New clipboard-clear seconds.
    pub clipboard_clear_seconds: Option<u32>,
    /// New quick-unlock maximum age in days.
    pub quick_unlock_max_age_days: Option<u32>,
    /// New passkey identity-store flag.
    pub passkey_identities: Option<bool>,
    /// New password identity-store flag.
    pub password_identities: Option<bool>,
    /// The master password, UTF-8, for the fields that need it.
    pub reauth_password: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileBody {
    version: u32,
    settings: MobileSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mac: Option<String>,
}

fn to_hex(bytes: &[u8]) -> String {
    crate::storage::hex(bytes)
}

fn mac_from_hex(text: &str) -> Option<[u8; SETTINGS_MAC_LEN]> {
    if text.len() != SETTINGS_MAC_LEN * 2
        || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    let mut out = [0u8; SETTINGS_MAC_LEN];
    for (byte, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(core::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

fn read_capped(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_FILE_LEN {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_LEN.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (u64::try_from(bytes.len()).ok()? <= MAX_FILE_LEN).then_some(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    match fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// What the core knows about the settings file.
pub(crate) struct SettingsGuard {
    path: PathBuf,
    /// The values as the file holds them (unclamped: the MAC covers them as
    /// written).
    on_disk: Option<MobileSettings>,
    mac: Option<[u8; SETTINGS_MAC_LEN]>,
    /// The file was read at the start and its MAC is not checked yet.
    unchecked: bool,
    /// What applies now.
    effective: MobileSettings,
    /// `K_settings`, only while unlocked and after the check.
    key: Option<SettingsMacKey>,
    /// The file was checked (or rewritten) under `K_settings` since the start:
    /// `effective` is then a verified value, not the strict stand-in of a
    /// locked phone after a restart.
    verified: bool,
}

impl SettingsGuard {
    /// Reads the file at the start; anything unusable gives the defaults.
    pub(crate) fn load(path: PathBuf) -> Self {
        let parsed = read_capped(&path)
            .and_then(|bytes| serde_json::from_slice::<FileBody>(&bytes).ok())
            .filter(|f| f.version == FILE_VERSION);
        match parsed {
            Some(file) => Self {
                effective: file.settings.stricter_than_default(),
                mac: file.mac.as_deref().and_then(mac_from_hex),
                on_disk: Some(file.settings),
                unchecked: true,
                key: None,
                verified: false,
                path,
            },
            None => Self {
                effective: MobileSettings::default(),
                on_disk: None,
                mac: None,
                unchecked: false,
                key: None,
                verified: false,
                path,
            },
        }
    }

    /// The settings that apply now.
    pub(crate) const fn effective(&self) -> MobileSettings {
        self.effective
    }

    /// The quick-unlock policy for an enrolment that carries `enrolled_days`
    /// (its maximum age at the enrolment, 1 to 30; the byte is not
    /// authenticated). Before the settings were checked since the start (a
    /// locked phone after a restart) the strict stand-in values must not
    /// shorten an enrolment that was verified when it was made (G1-10), so the
    /// enrolment's age applies, **shortened by the unverified file** (it may
    /// only shorten, so lowering the setting works in a new process, N1); once
    /// the settings were checked, the shorter of the enrolment and the
    /// verified value applies. The 30-day cap of CRYPTO_SPEC §6.2 holds either
    /// way. A quick unlock decided before the check is decided again after it
    /// ([`Core::finish_unlocked`]).
    pub(crate) fn quick_unlock_policy_for(&self, enrolled_days: u8) -> QuickUnlockPolicy {
        let mut days = u64::from(enrolled_days).min(30);
        if self.verified {
            days = days.min(u64::from(self.effective.quick_unlock_max_age_days));
        } else if let Some(file) = self.on_disk {
            days = days.min(u64::from(file.normalized().quick_unlock_max_age_days));
        }
        QuickUnlockPolicy::new(Duration::from_secs(days.saturating_mul(86_400)))
    }

    /// Forgets `K_settings` (every lock).
    pub(crate) fn locked(&mut self) {
        self.key = None;
    }

    /// Runs after every transition to unlocked: derives `K_settings`, checks
    /// the file's MAC once per run and writes the file with a fresh nonce and
    /// MAC when it was missing, invalid or stale. Errors leave the strict
    /// values in place.
    pub(crate) fn unlocked(&mut self, session: &mut UnlockedSession) {
        let Ok(key) = session.settings_mac_key() else {
            return;
        };
        let account = session.account_id();
        let Ok(nonce) = session.settings_nonce() else {
            return;
        };
        let mut rewrite = false;
        if self.unchecked {
            self.unchecked = false;
            let file = self.on_disk.unwrap_or_default();
            let valid = nonce
                .zip(self.mac)
                .is_some_and(|(nonce, mac)| key.verify(&account, &nonce, &file.canonical(), &mac));
            if valid {
                self.effective = file.normalized();
            } else {
                self.effective = file.stricter_than_default();
                rewrite = true;
            }
        }
        let current = self.on_disk == Some(self.effective) && self.mac.is_some();
        self.key = Some(key);
        self.verified = true;
        if rewrite || !current {
            let _ = self.save(session, self.effective);
        }
    }

    /// Writes `settings` with a fresh nonce and the MAC of what it writes.
    ///
    /// # Errors
    /// `locked` without the key, `storeUnavailable` for a failed write.
    pub(crate) fn save(
        &mut self,
        session: &mut UnlockedSession,
        settings: MobileSettings,
    ) -> Result<()> {
        let key = self.key.as_ref().ok_or(MobileError::Locked)?;
        let account = session.account_id();
        let old = session.settings_nonce()?;
        let nonce = new_settings_nonce()?;
        let mac = key.tag(&account, &nonce, &settings.canonical())?;
        // The nonce first: a crash between the two steps leaves a file the MAC
        // rejects (strict values), never one that verifies against a stale
        // nonce.
        session.set_settings_nonce(&nonce)?;
        let body = FileBody {
            version: FILE_VERSION,
            settings,
            mac: Some(to_hex(&mac)),
        };
        let bytes = serde_json::to_vec_pretty(&body).map_err(|_| MobileError::Internal)?;
        if write_atomic(&self.path, &bytes).is_err() {
            if let Some(old) = old {
                let _ = session.set_settings_nonce(&old);
            }
            return Err(MobileError::StoreUnavailable);
        }
        self.on_disk = Some(settings);
        self.mac = Some(mac);
        self.effective = settings;
        Ok(())
    }
}

impl Core {
    /// The settings that apply now: the stored values while unlocked and
    /// verified, the strict ones otherwise.
    #[must_use]
    pub fn settings(&self) -> MobileSettings {
        self.guard()
            .map_or_else(|_| MobileSettings::default(), |g| g.settings.effective())
    }

    /// Changes settings. Values are clamped. A weaker quick-unlock age or the
    /// password identity store needs the master password.
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `locked`, `reauthRequired`
    /// (the password is missing), `wrongCredentials`, `storeUnavailable`.
    pub fn update_settings(&self, update: SettingsUpdate) -> Result<MobileSettings> {
        self.require_app()?;
        let password = update
            .reauth_password
            .map(crate::secret::text_of)
            .transpose()?;
        let current = {
            let guard = self.guard()?;
            if !matches!(guard.phase, Phase::Unlocked(_)) {
                return Err(MobileError::Locked);
            }
            guard.settings.effective()
        };
        let wanted = MobileSettings {
            auto_lock_background_seconds: update
                .auto_lock_background_seconds
                .unwrap_or(current.auto_lock_background_seconds),
            idle_lock_minutes: update
                .idle_lock_minutes
                .unwrap_or(current.idle_lock_minutes),
            clipboard_clear_seconds: update
                .clipboard_clear_seconds
                .unwrap_or(current.clipboard_clear_seconds),
            quick_unlock_max_age_days: update
                .quick_unlock_max_age_days
                .unwrap_or(current.quick_unlock_max_age_days),
            passkey_identities: update
                .passkey_identities
                .unwrap_or(current.passkey_identities),
            password_identities: update
                .password_identities
                .unwrap_or(current.password_identities),
        }
        .normalized();
        let weaker = wanted.quick_unlock_max_age_days > current.quick_unlock_max_age_days
            || (wanted.password_identities && !current.password_identities);
        if weaker {
            let password = password.ok_or(MobileError::ReauthRequired)?;
            drop(self.verify_password(&password)?);
        }
        self.with_session(|session| {
            let mut guard = self.guard()?;
            guard.settings.save(session, wanted)
        })?;
        Ok(self.settings())
    }
}
