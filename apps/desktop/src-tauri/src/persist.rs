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
//! Writes are atomic: the new content goes to a temporary file in the same
//! directory, which then replaces the old one.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::ids::WireAccountId;
use crate::contract::settings::Settings;

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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StateFile {
    version: u32,
    settings: Settings,
    account: Option<AccountInfo>,
    #[serde(default)]
    ssh_agent_enabled: bool,
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

/// Parses the state file's bytes. Anything unexpected gives `None`.
#[must_use]
pub fn parse(bytes: &[u8]) -> Option<StoredState> {
    let file: StateFile = serde_json::from_slice(bytes).ok()?;
    if file.version != STATE_VERSION {
        return None;
    }
    let account = file.account.map(|mut a| {
        a.email = a.email.filter(|e| valid_email(e));
        a
    });
    Some(StoredState {
        settings: normalize(file.settings),
        account,
        ssh_agent_enabled: file.ssh_agent_enabled,
    })
}

/// Serializes the state for the file.
///
/// # Errors
/// `internal` if serialization fails (it cannot for these types).
pub fn render(state: &StoredState) -> IpcResult<Vec<u8>> {
    let file = StateFile {
        version: STATE_VERSION,
        settings: state.settings.clone(),
        account: state.account.clone(),
        ssh_agent_enabled: state.ssh_agent_enabled,
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

    /// Reads the state; defaults when there is no usable file.
    #[must_use]
    pub fn load(&self) -> StoredState {
        self.path
            .as_deref()
            .and_then(read_capped)
            .and_then(|bytes| parse(&bytes))
            .unwrap_or_default()
    }

    /// Writes the state atomically.
    ///
    /// # Errors
    /// `storeUnavailable` if the file cannot be written.
    pub fn save(&self, state: &StoredState) -> IpcResult<()> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let bytes = render(state)?;
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
    use crate::contract::settings::{Language, Theme};

    fn sample() -> StoredState {
        StoredState {
            settings: Settings {
                auto_lock_minutes: Some(5),
                lock_on_sleep: true,
                lock_on_screen_lock: false,
                clipboard_clear_seconds: None,
                theme: Theme::Dark,
                language: Language::De,
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
        persistence.save(&sample()).unwrap();
        assert_eq!(persistence.load(), sample());
        let mut changed = sample();
        changed.settings.theme = Theme::Light;
        persistence.save(&changed).unwrap();
        assert_eq!(persistence.load(), changed, "replaced in place");
        assert!(!dir.path().join("settings.json.tmp").exists());
    }

    #[test]
    fn memory_persistence_keeps_nothing() {
        let persistence = Persistence::memory();
        persistence.save(&sample()).unwrap();
        assert_eq!(persistence.load(), StoredState::default());
    }

    #[test]
    fn unusable_files_read_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        let persistence = Persistence::in_dir(dir.path());
        let good = String::from_utf8(render(&sample()).unwrap()).unwrap();
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
        let parsed = parse(&render(&state).unwrap()).unwrap();
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
    }

    #[test]
    fn all_triggers_off_reads_as_idle_lock() {
        let mut state = sample();
        state.settings.auto_lock_minutes = None;
        state.settings.lock_on_sleep = false;
        state.settings.lock_on_screen_lock = false;
        let text = String::from_utf8(render(&state).unwrap()).unwrap();
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
            let parsed = parse(&render(&state).unwrap()).unwrap();
            prop_assert!(has_lock_trigger(&parsed.settings, SCREEN_LOCK_POSSIBLE));
            if minutes.is_none() && !sleep && !(screen_lock && SCREEN_LOCK_POSSIBLE) {
                prop_assert_eq!(parsed.settings.auto_lock_minutes, Some(DEFAULT_AUTO_LOCK_MINUTES));
                state.settings.auto_lock_minutes = Some(DEFAULT_AUTO_LOCK_MINUTES);
            }
            prop_assert_eq!(parsed, state);
        }
    }
}
