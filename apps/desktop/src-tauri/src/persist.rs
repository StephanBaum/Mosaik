//! The app's small plaintext state file, next to the vault in the app data
//! directory: the settings and which account this install belongs to
//! (account ID and email, for the unlock screen and the keystore entry).
//!
//! Nothing in it is secret (CRYPTO_SPEC §11: the store holds the
//! ciphertext; this file holds preferences). It is still treated as
//! untrusted input: the file is size-capped, parsed strictly
//! (`deny_unknown_fields`, canonical IDs), and every value is clamped to its
//! range. A missing, damaged or newer file reads as the defaults, which are
//! the secure ones (auto-lock and clipboard clearing on), and is replaced on
//! the next save.
//!
//! The protection settings also carry a MAC (CRYPTO_SPEC §11.1, ADR-0030)
//! that only an unlocked app can check. Until then the file's values count
//! only where they are at least as strict as the defaults
//! ([`stricter_than_default`]); [`Persistence::load_checked`] hands the
//! file's own values and MAC on, for the check after the first unlock.
//!
//! Writes are atomic: the new content goes to a temporary file in the same
//! directory, which then replaces the old one.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::ids::WireAccountId;
use crate::contract::settings::{Language, Settings, Theme};

/// The state file's name in the app data directory.
pub const STATE_FILE: &str = "settings.json";
/// The format version this build writes and reads.
pub const STATE_VERSION: u32 = 1;
/// Largest state file accepted (it is a few hundred bytes in practice).
pub const MAX_STATE_FILE_LEN: u64 = 64 * 1024;
/// Longest accepted account email (RFC 5321 path limit).
pub const MAX_EMAIL_LEN: usize = 254;

/// Shortest and longest auto-lock timeout, in minutes.
pub const AUTO_LOCK_MINUTES: (u32, u32) = (1, 480);
/// Shortest and longest clipboard timeout, in seconds.
pub const CLIPBOARD_SECONDS: (u32, u32) = (10, 300);
/// Shortest and longest quick-unlock maximum age, in days (CRYPTO_SPEC §6.2).
pub const QUICK_UNLOCK_DAYS: (u32, u32) = (1, 30);
/// Default quick-unlock maximum age, in days.
pub const DEFAULT_QUICK_UNLOCK_DAYS: u32 = 14;

/// Everything in the state file.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StoredState {
    /// The settings, clamped.
    pub settings: Settings,
    /// The account of this install, once known.
    pub account: Option<AccountInfo>,
    /// The user turned the SSH agent on (it still runs only while unlocked).
    pub ssh_agent_enabled: bool,
}

/// The account of this install: its ID and email (not secret).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountInfo {
    /// The account.
    pub account_id: WireAccountId,
    /// The email given at account creation (`None` if this install learned
    /// the account from an unlock after the state file was lost).
    pub email: Option<String>,
    /// The Emergency Kit check passed for this account (ADR-0026 item 3).
    #[serde(default)]
    pub kit_verified: bool,
}

/// The settings as the file holds them. Like [`Settings`], but a file from
/// before a setting existed reads with that setting's default (the wire
/// type keeps every key required).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedSettings {
    auto_lock_minutes: Option<u32>,
    lock_on_sleep: bool,
    lock_on_screen_lock: bool,
    clipboard_clear_seconds: Option<u32>,
    theme: Theme,
    language: Language,
    #[serde(default = "default_quick_unlock_days")]
    quick_unlock_max_age_days: u32,
}

fn default_quick_unlock_days() -> u32 {
    DEFAULT_QUICK_UNLOCK_DAYS
}

impl From<PersistedSettings> for Settings {
    fn from(p: PersistedSettings) -> Self {
        Self {
            auto_lock_minutes: p.auto_lock_minutes,
            lock_on_sleep: p.lock_on_sleep,
            lock_on_screen_lock: p.lock_on_screen_lock,
            clipboard_clear_seconds: p.clipboard_clear_seconds,
            theme: p.theme,
            language: p.language,
            quick_unlock_max_age_days: p.quick_unlock_max_age_days,
        }
    }
}

impl From<&Settings> for PersistedSettings {
    fn from(s: &Settings) -> Self {
        Self {
            auto_lock_minutes: s.auto_lock_minutes,
            lock_on_sleep: s.lock_on_sleep,
            lock_on_screen_lock: s.lock_on_screen_lock,
            clipboard_clear_seconds: s.clipboard_clear_seconds,
            theme: s.theme,
            language: s.language,
            quick_unlock_max_age_days: s.quick_unlock_max_age_days,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StateFile {
    version: u32,
    settings: PersistedSettings,
    account: Option<AccountInfo>,
    #[serde(default)]
    ssh_agent_enabled: bool,
    /// Hex of the settings MAC (CRYPTO_SPEC §11.1); absent in older files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mac: Option<String>,
}

/// Length of the settings MAC in bytes.
pub const MAC_LEN: usize = mosaic_core::SETTINGS_MAC_LEN;

/// The settings section of the file, as the file holds it, and its MAC:
/// what the first unlock checks (ADR-0030).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileSettings {
    /// The values as read, not clamped (the MAC covers them as written).
    pub settings: Settings,
    /// The MAC, if the file has a well-formed one.
    pub mac: Option<[u8; MAC_LEN]>,
}

/// Default idle timeout, minutes (CRYPTO_SPEC §11).
pub const DEFAULT_AUTO_LOCK_MINUTES: u32 = 10;

/// Whether this build can detect a locked screen at all (only the Linux
/// D-Bus source exists; Windows and macOS have none yet).
pub const SCREEN_LOCK_POSSIBLE: bool = cfg!(target_os = "linux");

/// Whether at least one auto-lock trigger that can fire is on (§11: they
/// cannot all be disabled). `lock_on_sleep` counts everywhere (the clock-gap
/// detector covers it); `lock_on_screen_lock` counts only where
/// `screen_lock_detection` is true.
#[must_use]
pub fn has_lock_trigger(s: &Settings, screen_lock_detection: bool) -> bool {
    s.auto_lock_minutes.is_some()
        || s.lock_on_sleep
        || (s.lock_on_screen_lock && screen_lock_detection)
}

/// Clamps every setting to its range (`docs/DESKTOP_IPC.md`: values are
/// clamped, not refused; if every auto-lock trigger is off, the
/// default idle timeout is switched on (CRYPTO_SPEC §11).
#[must_use]
pub fn normalize(mut settings: Settings) -> Settings {
    settings.auto_lock_minutes = settings
        .auto_lock_minutes
        .map(|m| m.clamp(AUTO_LOCK_MINUTES.0, AUTO_LOCK_MINUTES.1));
    settings.clipboard_clear_seconds = settings
        .clipboard_clear_seconds
        .map(|s| s.clamp(CLIPBOARD_SECONDS.0, CLIPBOARD_SECONDS.1));
    settings.quick_unlock_max_age_days = settings
        .quick_unlock_max_age_days
        .clamp(QUICK_UNLOCK_DAYS.0, QUICK_UNLOCK_DAYS.1);
    if !has_lock_trigger(&settings, SCREEN_LOCK_POSSIBLE) {
        settings.auto_lock_minutes = Some(DEFAULT_AUTO_LOCK_MINUTES);
    }
    settings
}

/// Whether an email is acceptable for the state file and the kit: non-empty
/// after trimming, at most [`MAX_EMAIL_LEN`] bytes, with an `@` and no
/// control characters.
#[must_use]
pub fn valid_email(email: &str) -> bool {
    !email.is_empty()
        && email.len() <= MAX_EMAIL_LEN
        && email.trim() == email
        && email.contains('@')
        && !email.chars().any(char::is_control)
}

/// Whether two settings agree on every protection field (everything but
/// theme and language).
#[must_use]
pub fn same_protection(a: &Settings, b: &Settings) -> bool {
    a.auto_lock_minutes == b.auto_lock_minutes
        && a.lock_on_sleep == b.lock_on_sleep
        && a.lock_on_screen_lock == b.lock_on_screen_lock
        && a.clipboard_clear_seconds == b.clipboard_clear_seconds
        && a.quick_unlock_max_age_days == b.quick_unlock_max_age_days
}

/// Per protection field, the stricter of `settings` and the defaults: the
/// shorter auto-lock and clipboard time (`null` becomes the default), the
/// shorter quick-unlock age, both lock-on switches on. Theme and language
/// are kept. This is all an unverified file may set (ADR-0030).
#[must_use]
pub fn stricter_than_default(settings: &Settings) -> Settings {
    let default = Settings::default();
    let shorter = |value: Option<u32>, limit: Option<u32>| match (value, limit) {
        (Some(v), Some(l)) => Some(v.min(l)),
        (_, limit) => limit,
    };
    normalize(Settings {
        auto_lock_minutes: shorter(settings.auto_lock_minutes, default.auto_lock_minutes),
        lock_on_sleep: settings.lock_on_sleep || default.lock_on_sleep,
        lock_on_screen_lock: settings.lock_on_screen_lock || default.lock_on_screen_lock,
        clipboard_clear_seconds: shorter(
            settings.clipboard_clear_seconds,
            default.clipboard_clear_seconds,
        ),
        theme: settings.theme,
        language: settings.language,
        quick_unlock_max_age_days: settings
            .quick_unlock_max_age_days
            .min(default.quick_unlock_max_age_days),
    })
}

/// The canonical bytes the settings MAC covers (CRYPTO_SPEC §11.1): one JSON
/// object, keys in ascending byte order, no whitespace.
#[must_use]
pub fn canonical_settings(settings: &Settings) -> Vec<u8> {
    fn number(value: Option<u32>) -> String {
        value.map_or_else(|| "null".to_owned(), |v| v.to_string())
    }
    fn word<T: Serialize>(value: &T) -> String {
        // The wire enums serialize to a plain string such as `"system"`.
        serde_json::to_string(value).unwrap_or_default()
    }
    format!(
        "{{\"autoLockMinutes\":{},\"clipboardClearSeconds\":{},\"language\":{},\"lockOnScreenLock\":{},\"lockOnSleep\":{},\"quickUnlockMaxAgeDays\":{},\"theme\":{}}}",
        number(settings.auto_lock_minutes),
        number(settings.clipboard_clear_seconds),
        word(&settings.language),
        settings.lock_on_screen_lock,
        settings.lock_on_sleep,
        settings.quick_unlock_max_age_days,
        word(&settings.theme),
    )
    .into_bytes()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn mac_from_hex(text: &str) -> Option<[u8; MAC_LEN]> {
    if text.len() != MAC_LEN * 2 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut out = [0u8; MAC_LEN];
    for (byte, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

/// Parses the state file's bytes. Anything unexpected gives `None`. The
/// settings in the state are the ones that count while locked
/// ([`stricter_than_default`]); [`parse_checked`] also returns the file's own.
#[must_use]
pub fn parse(bytes: &[u8]) -> Option<StoredState> {
    parse_checked(bytes).map(|(state, _)| state)
}

/// [`parse`], plus the file's settings and MAC for the check after unlock.
#[must_use]
pub fn parse_checked(bytes: &[u8]) -> Option<(StoredState, FileSettings)> {
    let file: StateFile = serde_json::from_slice(bytes).ok()?;
    if file.version != STATE_VERSION {
        return None;
    }
    let account = file.account.map(|mut a| {
        a.email = a.email.filter(|e| valid_email(e));
        a
    });
    let raw: Settings = file.settings.into();
    let state = StoredState {
        settings: stricter_than_default(&raw),
        account,
        ssh_agent_enabled: file.ssh_agent_enabled,
    };
    let checked = FileSettings {
        settings: raw,
        mac: file.mac.as_deref().and_then(mac_from_hex),
    };
    Some((state, checked))
}

/// Serializes the state for the file, with the MAC of its settings if given.
///
/// # Errors
/// `internal` if serialization fails (it cannot for these types).
pub fn render(state: &StoredState, mac: Option<&[u8; MAC_LEN]>) -> IpcResult<Vec<u8>> {
    let file = StateFile {
        version: STATE_VERSION,
        settings: PersistedSettings::from(&state.settings),
        account: state.account.clone(),
        ssh_agent_enabled: state.ssh_agent_enabled,
        mac: mac.map(|m| to_hex(m)),
    };
    serde_json::to_vec_pretty(&file).map_err(|_| IpcError::INTERNAL)
}

/// Where the state lives: a file, or nowhere (tests and an app data
/// directory that cannot be created).
#[derive(Clone, Debug)]
pub struct Persistence {
    path: Option<PathBuf>,
}

impl Persistence {
    /// State in `dir`/[`STATE_FILE`].
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            path: Some(dir.join(STATE_FILE)),
        }
    }

    /// No file: state lives in memory for the life of the process.
    #[must_use]
    pub const fn memory() -> Self {
        Self { path: None }
    }

    /// Reads the state; defaults when there is no usable file. The settings
    /// are the ones that count while locked ([`stricter_than_default`]).
    #[must_use]
    pub fn load(&self) -> StoredState {
        self.load_checked().0
    }

    /// [`Persistence::load`], plus the file's own settings and MAC (`None`
    /// when there is no usable file).
    #[must_use]
    pub fn load_checked(&self) -> (StoredState, Option<FileSettings>) {
        self.path
            .as_deref()
            .and_then(read_capped)
            .and_then(|bytes| parse_checked(&bytes))
            .map_or_else(
                || (StoredState::default(), None),
                |(state, file)| (state, Some(file)),
            )
    }

    /// Writes the state atomically, with the MAC of its settings.
    ///
    /// # Errors
    /// `storeUnavailable` if the file cannot be written.
    pub fn save(&self, state: &StoredState, mac: Option<&[u8; MAC_LEN]>) -> IpcResult<()> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let bytes = render(state, mac)?;
        write_atomic(path, &bytes)
            .map_err(|_| IpcError::new(crate::contract::errors::ErrorCode::StoreUnavailable))
    }
}

fn read_capped(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_STATE_FILE_LEN {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_STATE_FILE_LEN.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (u64::try_from(bytes.len()).ok()? <= MAX_STATE_FILE_LEN).then_some(bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    // Replaces an existing file on every desktop OS (on Windows through
    // `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::contract::ids::samples as ids;
    fn sample() -> StoredState {
        StoredState {
            settings: Settings {
                auto_lock_minutes: Some(5),
                lock_on_sleep: true,
                lock_on_screen_lock: true,
                clipboard_clear_seconds: Some(20),
                theme: Theme::Dark,
                language: Language::De,
                quick_unlock_max_age_days: 7,
            },
            account: Some(AccountInfo {
                account_id: ids::account(),
                email: Some("alex@example.com".into()),
                kit_verified: true,
            }),
            ssh_agent_enabled: true,
        }
    }

    #[test]
    fn round_trips_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let persistence = Persistence::in_dir(dir.path());
        assert_eq!(persistence.load(), StoredState::default(), "no file yet");
        persistence.save(&sample(), None).unwrap();
        assert_eq!(persistence.load(), sample());
        let mut changed = sample();
        changed.settings.theme = Theme::Light;
        persistence.save(&changed, None).unwrap();
        assert_eq!(persistence.load(), changed, "replaced in place");
        assert!(!dir.path().join("settings.json.tmp").exists());
    }

    #[test]
    fn memory_persistence_keeps_nothing() {
        let persistence = Persistence::memory();
        persistence.save(&sample(), None).unwrap();
        assert_eq!(persistence.load(), StoredState::default());
    }

    #[test]
    fn unusable_files_read_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        let persistence = Persistence::in_dir(dir.path());
        let good = String::from_utf8(render(&sample(), None).unwrap()).unwrap();
        let cases = [
            "not json".to_owned(),
            good.replace("\"version\": 1", "\"version\": 2"),
            good.replace("\"theme\"", "\"extra\": 1, \"theme\""),
            good.replace(&ids::account().0.to_string(), "not-a-uuid"),
            " ".repeat(usize::try_from(MAX_STATE_FILE_LEN).unwrap() + 1),
        ];
        for case in cases {
            fs::write(&path, case.as_bytes()).unwrap();
            assert_eq!(persistence.load(), StoredState::default(), "{case:.60}");
        }
    }

    #[test]
    fn an_invalid_email_is_dropped() {
        let mut state = sample();
        if let Some(account) = state.account.as_mut() {
            account.email = Some("no-at-sign".into());
        }
        let parsed = parse(&render(&state, None).unwrap()).unwrap();
        assert_eq!(parsed.account.unwrap().email, None);
        assert_eq!(parsed.settings, sample().settings);
    }

    #[test]
    fn values_are_clamped() {
        let clamped = normalize(Settings {
            auto_lock_minutes: Some(0),
            clipboard_clear_seconds: Some(100_000),
            ..Settings::default()
        });
        assert_eq!(clamped.auto_lock_minutes, Some(1));
        assert_eq!(clamped.clipboard_clear_seconds, Some(300));
        assert_eq!(normalize(Settings::default()), Settings::default());
        for (days, wanted) in [(0, 1), (14, 14), (31, 30), (u32::MAX, 30)] {
            let n = normalize(Settings {
                quick_unlock_max_age_days: days,
                ..Settings::default()
            });
            assert_eq!(n.quick_unlock_max_age_days, wanted);
        }
    }

    #[test]
    fn a_file_from_before_the_quick_unlock_age_reads_the_default() {
        let good = String::from_utf8(render(&sample(), None).unwrap()).unwrap();
        let old = good.replace(",\n    \"quickUnlockMaxAgeDays\": 7", "");
        assert!(!old.contains("quickUnlockMaxAge"), "{old}");
        let parsed = parse(old.as_bytes()).unwrap();
        assert_eq!(
            parsed.settings.quick_unlock_max_age_days,
            DEFAULT_QUICK_UNLOCK_DAYS
        );
    }

    #[test]
    fn all_triggers_off_reads_as_idle_lock() {
        let mut state = sample();
        state.settings.auto_lock_minutes = None;
        state.settings.lock_on_sleep = false;
        state.settings.lock_on_screen_lock = false;
        let text = String::from_utf8(render(&state, None).unwrap()).unwrap();
        assert!(
            text.contains("\"autoLockMinutes\": null"),
            "file really says null"
        );
        let parsed = parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.settings.auto_lock_minutes, Some(10));
    }

    #[test]
    fn email_rules() {
        assert!(valid_email("a@b"));
        for bad in ["", "ab", " a@b", "a@b\n", &format!("{}@b", "a".repeat(260))] {
            assert!(!valid_email(bad), "{bad:?}");
        }
    }

    #[test]
    fn normalize_counts_screen_lock_only_where_possible() {
        let n = normalize(Settings {
            auto_lock_minutes: None,
            lock_on_sleep: false,
            lock_on_screen_lock: true,
            ..Settings::default()
        });
        assert_eq!(
            n.auto_lock_minutes,
            if SCREEN_LOCK_POSSIBLE {
                None
            } else {
                Some(DEFAULT_AUTO_LOCK_MINUTES)
            }
        );
    }

    fn tampered(settings: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "version": 1, "settings": settings, "account": null
        }))
        .unwrap()
    }

    #[test]
    fn locked_a_weaker_file_value_falls_back_to_the_default() {
        let weak = tampered(&serde_json::json!({
            "autoLockMinutes": null, "lockOnSleep": false, "lockOnScreenLock": false,
            "clipboardClearSeconds": null, "theme": "dark", "language": "de",
            "quickUnlockMaxAgeDays": 30
        }));
        let (state, file) = parse_checked(&weak).unwrap();
        let wanted = Settings {
            theme: Theme::Dark,
            language: Language::De,
            ..Settings::default()
        };
        assert_eq!(state.settings, wanted, "defaults, but theme and language");
        assert_eq!(
            file.settings.auto_lock_minutes, None,
            "the file's own value"
        );
        assert_eq!(file.mac, None);
        // Longer timeouts fall back too; shorter ones stay.
        let mixed = tampered(&serde_json::json!({
            "autoLockMinutes": 480, "lockOnSleep": true, "lockOnScreenLock": true,
            "clipboardClearSeconds": 15, "theme": "system", "language": "system",
            "quickUnlockMaxAgeDays": 3
        }));
        let s = parse(&mixed).unwrap().settings;
        assert_eq!(
            (
                s.auto_lock_minutes,
                s.clipboard_clear_seconds,
                s.quick_unlock_max_age_days
            ),
            (Some(10), Some(15), 3)
        );
    }

    #[test]
    fn stricter_than_default_is_idempotent_and_never_weaker() {
        let strict = stricter_than_default(&Settings {
            auto_lock_minutes: Some(480),
            lock_on_sleep: false,
            lock_on_screen_lock: false,
            clipboard_clear_seconds: None,
            quick_unlock_max_age_days: 30,
            ..Settings::default()
        });
        assert_eq!(strict, Settings::default());
        assert_eq!(stricter_than_default(&strict), strict);
    }

    /// The canonical form is fixed by CRYPTO_SPEC §11.1 (and the same bytes
    /// are the golden message of `mosaic-core`).
    #[test]
    fn canonical_form_is_the_specified_bytes() {
        assert_eq!(
            String::from_utf8(canonical_settings(&Settings::default())).unwrap(),
            r#"{"autoLockMinutes":10,"clipboardClearSeconds":30,"language":"system","lockOnScreenLock":true,"lockOnSleep":true,"quickUnlockMaxAgeDays":14,"theme":"system"}"#
        );
        let other = Settings {
            auto_lock_minutes: None,
            clipboard_clear_seconds: None,
            lock_on_sleep: false,
            lock_on_screen_lock: false,
            theme: Theme::Light,
            language: Language::De,
            quick_unlock_max_age_days: 30,
        };
        assert_eq!(
            String::from_utf8(canonical_settings(&other)).unwrap(),
            r#"{"autoLockMinutes":null,"clipboardClearSeconds":null,"language":"de","lockOnScreenLock":false,"lockOnSleep":false,"quickUnlockMaxAgeDays":30,"theme":"light"}"#
        );
    }

    #[test]
    fn every_field_changes_the_canonical_bytes() {
        let base = Settings::default();
        let edits: [fn(&mut Settings); 7] = [
            |s| s.auto_lock_minutes = Some(11),
            |s| s.lock_on_sleep = false,
            |s| s.lock_on_screen_lock = false,
            |s| s.clipboard_clear_seconds = None,
            |s| s.theme = Theme::Dark,
            |s| s.language = Language::En,
            |s| s.quick_unlock_max_age_days = 15,
        ];
        for change in edits {
            let mut changed = base.clone();
            change(&mut changed);
            assert_ne!(canonical_settings(&changed), canonical_settings(&base));
        }
    }

    #[test]
    fn the_mac_round_trips_and_bad_ones_read_as_missing() {
        let mac: [u8; MAC_LEN] = core::array::from_fn(|i| u8::try_from(i * 7).unwrap());
        let text = String::from_utf8(render(&sample(), Some(&mac)).unwrap()).unwrap();
        assert_eq!(parse_checked(text.as_bytes()).unwrap().1.mac, Some(mac));
        let hex = to_hex(&mac);
        for bad in [
            hex[..62].to_owned(),
            format!("{hex}00"),
            hex.to_uppercase(),
            hex.replace(&hex[..1], "g"),
        ] {
            let edited = text.replace(&hex, &bad);
            assert_eq!(
                parse_checked(edited.as_bytes()).unwrap().1.mac,
                None,
                "{bad}"
            );
        }
        let without = String::from_utf8(render(&sample(), None).unwrap()).unwrap();
        assert!(!without.contains("\"mac\""));
    }

    proptest! {
        #[test]
        fn normalized_values_are_in_range(minutes in any::<Option<u32>>(), seconds in any::<Option<u32>>()) {
            let n = normalize(Settings {
                auto_lock_minutes: minutes,
                clipboard_clear_seconds: seconds,
                ..Settings::default()
            });
            prop_assert_eq!(n.auto_lock_minutes.is_some(), minutes.is_some());
            prop_assert_eq!(n.clipboard_clear_seconds.is_some(), seconds.is_some());
            if let Some(m) = n.auto_lock_minutes {
                prop_assert!((AUTO_LOCK_MINUTES.0..=AUTO_LOCK_MINUTES.1).contains(&m));
            }
            if let Some(s) = n.clipboard_clear_seconds {
                prop_assert!((CLIPBOARD_SECONDS.0..=CLIPBOARD_SECONDS.1).contains(&s));
            }
            prop_assert!(has_lock_trigger(&n, SCREEN_LOCK_POSSIBLE));
            prop_assert_eq!(normalize(n.clone()), n);
        }

        #[test]
        fn parse_never_panics_and_only_yields_normalized_state(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            if let Some(state) = parse(&bytes) {
                prop_assert_eq!(normalize(state.settings.clone()), state.settings);
            }
        }

        #[test]
        fn rendered_state_parses_back(minutes in proptest::option::of(1u32..=480), seconds in proptest::option::of(10u32..=300), sleep in any::<bool>(), screen_lock in any::<bool>()) {
            let mut state = sample();
            state.settings.auto_lock_minutes = minutes;
            state.settings.clipboard_clear_seconds = seconds;
            state.settings.lock_on_sleep = sleep;
            state.settings.lock_on_screen_lock = screen_lock;
            let parsed = parse(&render(&state, None).unwrap()).unwrap();
            prop_assert!(has_lock_trigger(&parsed.settings, SCREEN_LOCK_POSSIBLE));
            // Locked, only values at least as strict as the defaults count.
            state.settings = stricter_than_default(&state.settings);
            prop_assert_eq!(parsed, state);
        }
    }
}
