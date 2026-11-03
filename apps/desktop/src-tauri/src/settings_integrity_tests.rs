//! Settings integrity (CRYPTO_SPEC §11.1, ADR-0030): a state file edited
//! outside Mosaic never weakens a protection, locked or unlocked.
//!
//! Every phase "restarts" the app: a new `Backend` over the same state file
//! and store file, with the first one dropped.
#![allow(clippy::too_many_lines)]

use std::path::Path;

use mosaic_core::Argon2Profile;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::contract::errors::ErrorCode;
use crate::contract::events::LockReason;
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, UnlockRequest};
use crate::contract::settings::{Language, Settings, Theme, UpdateSettingsRequest};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::persist::{Persistence, STATE_FILE};
use crate::state::{Backend, Slot};

const PASSWORD: &str = "settings integrity master password";

fn start(dir: &Path) -> (Backend, Log) {
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(
            Store::open(dir.join("vault.db")).unwrap(),
        )),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_persistence(Persistence::in_dir(dir));
    let (channel, log) = recording_channel();
    backend.subscribe(channel).unwrap();
    (backend, log)
}

fn unlock(b: &Backend, secret_key: &str) {
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test(secret_key)),
    })
    .unwrap();
}

fn resets(log: &Log) -> usize {
    log.lock()
        .unwrap()
        .iter()
        .filter(|e| serde_json::from_str::<Value>(e).unwrap()["type"] == "settingsReset")
        .count()
}

fn read_file(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join(STATE_FILE)).unwrap()).unwrap()
}

fn write_file(dir: &Path, file: &Value) {
    std::fs::write(
        dir.join(STATE_FILE),
        serde_json::to_vec_pretty(file).unwrap(),
    )
    .unwrap();
}

fn update(b: &Backend, settings: Settings) -> Settings {
    b.update_settings(UpdateSettingsRequest {
        settings,
        password: Some(SecretIn::test(PASSWORD)),
    })
    .unwrap()
}

/// What a user may legitimately have chosen: every protection weaker than the
/// default.
fn weak() -> Settings {
    Settings {
        auto_lock_minutes: Some(120),
        lock_on_sleep: false,
        lock_on_screen_lock: true,
        clipboard_clear_seconds: None,
        theme: Theme::Dark,
        language: Language::De,
        quick_unlock_max_age_days: 30,
    }
}

/// The protection fields of the defaults, theme and language of `weak()`.
fn strict_dark() -> Settings {
    Settings {
        theme: Theme::Dark,
        language: Language::De,
        ..Settings::default()
    }
}

#[test]
fn a_tampered_file_is_ignored_locked_and_reset_after_unlock() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    // Phase 0: a user with weak, legitimately chosen settings.
    let secret_key = {
        let (b, log) = start(d);
        let created = b
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        let key = serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(update(&b, weak()), weak());
        assert_eq!(resets(&log), 0);
        assert!(read_file(d)["mac"].as_str().is_some_and(|m| m.len() == 64));
        key
    };

    // Phase 1: a valid MAC is accepted. Locked, only the strict values
    // count; a theme change while locked is kept and written at unlock.
    {
        let (b, log) = start(d);
        assert_eq!(b.settings_snapshot(), strict_dark(), "locked: strict");
        assert_eq!(
            b.quick_unlock_policy().max_age(),
            std::time::Duration::from_hours(14 * 24),
            "the expiry check before the first unlock uses the strict age"
        );
        let mut light = b.settings_snapshot();
        light.theme = Theme::Light;
        assert_eq!(
            b.update_settings(UpdateSettingsRequest {
                settings: light.clone(),
                password: None,
            })
            .unwrap()
            .theme,
            Theme::Light
        );
        let mut longer = light.clone();
        longer.auto_lock_minutes = Some(480);
        assert_eq!(
            b.update_settings(UpdateSettingsRequest {
                settings: longer,
                password: None
            })
            .unwrap_err()
            .code,
            ErrorCode::Locked,
            "a protection setting does not change while locked"
        );
        unlock(&b, &secret_key);
        let expected = Settings {
            theme: Theme::Light,
            ..weak()
        };
        assert_eq!(b.settings_snapshot(), expected, "valid MAC: applied");
        assert_eq!(resets(&log), 0, "no notice for a good file");
        assert_eq!(
            b.quick_unlock_policy().max_age(),
            std::time::Duration::from_hours(30 * 24)
        );
        b.lock(LockReason::Manual).unwrap();
        assert_eq!(
            b.settings_snapshot(),
            expected,
            "verified values stay in memory"
        );
    }
    assert_eq!(read_file(d)["settings"]["theme"], "light");

    // Phase 2: malware sets a protection to its weakest, MAC left as it was.
    for (field, value) in [
        ("autoLockMinutes", Value::Null),
        ("clipboardClearSeconds", json!(300)),
        ("quickUnlockMaxAgeDays", json!(29)),
        ("lockOnSleep", json!(true)),
    ] {
        let mut file = read_file(d);
        file["settings"][field] = value;
        write_file(d, &file);
        let (b, log) = start(d);
        assert_eq!(
            b.settings_snapshot(),
            Settings {
                theme: Theme::Light,
                ..strict_dark()
            },
            "{field}: locked, strict values"
        );
        unlock(&b, &secret_key);
        assert_eq!(
            b.settings_snapshot(),
            Settings {
                theme: Theme::Light,
                ..strict_dark()
            },
            "{field}: reset after unlock, the theme stays"
        );
        assert_eq!(resets(&log), 1, "{field}: the user is told once");
        let rewritten = read_file(d);
        assert_eq!(rewritten["settings"]["autoLockMinutes"], 10);
        assert_eq!(rewritten["settings"]["clipboardClearSeconds"], 30);
        // A second run over the rewritten file is clean.
        drop(b);
        let (b, log) = start(d);
        unlock(&b, &secret_key);
        assert_eq!(
            resets(&log),
            0,
            "{field}: the rewritten file has a valid MAC"
        );
        assert_eq!(b.settings_snapshot().auto_lock_minutes, Some(10));
        // Put the weak (but valid) settings back for the next case.
        update(
            &b,
            Settings {
                theme: Theme::Light,
                ..weak()
            },
        );
    }

    // Phase 3: a file from before the MAC (no `mac`): weakening fields reset
    // with a notice, theme and language kept.
    {
        let mut file = read_file(d);
        file["settings"]["theme"] = json!("light");
        file.as_object_mut().unwrap().remove("mac");
        write_file(d, &file);
        let (b, log) = start(d);
        unlock(&b, &secret_key);
        assert_eq!(
            b.settings_snapshot(),
            Settings {
                theme: Theme::Light,
                ..strict_dark()
            }
        );
        assert_eq!(resets(&log), 1);
        assert!(read_file(d)["mac"].is_string(), "the MAC is written");
    }

    // Phase 4: only the theme changed and the MAC is gone: nothing weaker was
    // asked for, so the theme survives and nobody is alarmed.
    {
        let mut file = read_file(d);
        file["settings"]["theme"] = json!("system");
        file.as_object_mut().unwrap().remove("mac");
        write_file(d, &file);
        let (b, log) = start(d);
        unlock(&b, &secret_key);
        assert_eq!(b.settings_snapshot().theme, Theme::System);
        assert_eq!(resets(&log), 0);
    }

    // Phase 5: a garbled MAC is a missing one.
    {
        let mut file = read_file(d);
        file["mac"] = json!("zz");
        file["settings"]["clipboardClearSeconds"] = json!(300);
        write_file(d, &file);
        let (b, log) = start(d);
        unlock(&b, &secret_key);
        assert_eq!(b.settings_snapshot().clipboard_clear_seconds, Some(30));
        assert_eq!(resets(&log), 1);
    }
}

#[test]
fn a_mac_from_another_account_does_not_verify() {
    let _kdf = crate::test_support::kdf_permit();
    let weak_file = |dir: &Path| {
        let (b, _) = start(dir);
        b.create_account(CreateAccountRequest {
            email: "a@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "P".into(),
        })
        .unwrap();
        update(&b, weak());
        read_file(dir)
    };
    let one = tempfile::tempdir().unwrap();
    let copied = weak_file(one.path());
    // A second install adopts the first one's file (same bytes, other AK).
    let two = tempfile::tempdir().unwrap();
    let (b, log) = start(two.path());
    let created = b
        .create_account(CreateAccountRequest {
            email: "a@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "P".into(),
        })
        .unwrap();
    let key = serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(b);
    let mut file = copied;
    file["account"] = read_file(two.path())["account"].clone();
    write_file(two.path(), &file);
    let (b, log2) = start(two.path());
    unlock(&b, &key);
    assert_eq!(
        b.settings_snapshot(),
        strict_dark(),
        "another account's MAC does not apply its weak settings"
    );
    assert_eq!(resets(&log2), 1);
    assert_eq!(resets(&log), 0);
}
