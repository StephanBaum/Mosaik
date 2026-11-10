//! Input rules, the exclusive store lock and the session cache.

use mosaic_core::{MasterPassword, SecretKey};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use crate::common::{Fixture, PASSWORD, T0};

#[test]
fn password_flag_value_is_usage_error() {
    let f = Fixture::new();
    let out = f
        .cmd(&["--json", "unlock", "--password", "hunter2-secret"])
        .run();
    assert_eq!(out.code, 2);
    assert_eq!(out.error_json()["error"]["code"], "secretInArgv");
    assert!(!out.stdout.contains("hunter2-secret") && !out.stderr.contains("hunter2-secret"));
    // The `=` spelling is refused the same way.
    let out = f.cmd(&["unlock", "--password=hunter2-secret"]).run();
    assert_eq!(out.code, 2);
    assert!(!out.stderr.contains("hunter2-secret"));
}

#[test]
fn env_password_ignored_with_warning() {
    let f = Fixture::new();
    // The right password in the environment does not help; a wrong one on stdin fails.
    let out = f
        .cmd(&["unlock", "--password-stdin"])
        .env("MOSAIC_PASSWORD", PASSWORD)
        .stdin("not the password")
        .run();
    assert_eq!(out.code, 3);
    assert!(out.stderr.contains("MOSAIC_PASSWORD"), "{}", out.stderr);
    assert!(!out.stderr.contains(PASSWORD));
    assert!(!f.session_file().exists());
}

#[test]
fn no_input_refuses_prompt() {
    let f = Fixture::new();
    let out = f.cmd(&["--json", "--no-input", "list"]).run();
    assert_eq!(out.code, 3);
    assert_eq!(out.error_json()["error"]["code"], "inputRequired");
}

#[test]
fn password_stdin_and_fd() {
    let f = Fixture::new();
    f.unlock("5m");
    assert!(f.session_file().exists() && f.session_key().exists());
    #[cfg(unix)]
    {
        let g = Fixture::new();
        let pw = g.tmp.path().join("pw.txt");
        std::fs::write(&pw, format!("{PASSWORD}\n")).unwrap();
        let script = format!(
            "exec \"$0\" --store \"$1\" unlock --password-fd 3 3<'{}'",
            pw.display()
        );
        let out = std::process::Command::new("sh")
            .args([
                "-c",
                &script,
                env!("CARGO_BIN_EXE_mosaic"),
                g.store.to_str().unwrap(),
            ])
            .env("MOSAIC_TEST_DIR", &g.test_dir)
            .env("MOSAIC_TEST_NOW_MS", T0.to_string())
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(g.session_file().exists());
    }
}

#[test]
fn wrong_password_exit_3() {
    let f = Fixture::new();
    let out = f
        .cmd(&["--json", "unlock", "--password-stdin"])
        .stdin("wrong password")
        .run();
    assert_eq!(out.code, 3);
    assert_eq!(out.error_json()["error"]["code"], "wrongCredentials");
    assert!(
        !f.session_file().exists(),
        "a failed unlock leaves no session"
    );
}

#[test]
fn store_in_use_exit_5() {
    let f = Fixture::new();
    // The desktop holds its store for its whole run: here this test does.
    let held = Store::open(&f.store).unwrap();
    let out = f.cmd(&["--json", "--no-input", "list"]).run();
    assert_eq!(out.code, 5);
    assert_eq!(out.error_json()["error"]["code"], "storeInUse");
    assert!(
        out.error_json()["error"]["message"]
            .as_str()
            .unwrap()
            .contains("own-store")
    );
    assert_eq!(f.cmd(&["status"]).run().code, 5);
    drop(held);
    assert_eq!(f.cmd(&["status"]).run().code, 0);
}

#[test]
fn unlock_creates_session_and_lock_removes_it() {
    let f = Fixture::new();
    f.unlock("5m");
    let listed = f.cmd(&["--json", "--no-input", "list"]).run();
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    // The session part in the keystore is 64 base64url characters; the file is 162 bytes.
    assert_eq!(std::fs::read(f.session_file()).unwrap().len(), 162);
    assert_eq!(std::fs::read_to_string(f.session_key()).unwrap().len(), 64);
    let status = f.cmd(&["--json", "status"]).run().json();
    assert_eq!(status["data"]["locked"], false);
    assert_eq!(status["data"]["session"]["expiresAt"], T0 + 300_000);
    assert_eq!(f.cmd(&["lock"]).run().code, 0);
    assert!(!f.session_file().exists() && !f.session_key().exists());
    assert_eq!(f.cmd(&["--no-input", "list"]).run().code, 3);
}

#[test]
fn ttl_over_30_minutes_is_a_usage_error() {
    let f = Fixture::new();
    let out = f
        .cmd(&["unlock", "--ttl", "31m", "--password-stdin"])
        .stdin(PASSWORD)
        .run();
    assert_eq!(out.code, 2);
    assert_eq!(
        f.cmd(&["unlock", "--ttl", "0m", "--password-stdin"])
            .stdin(PASSWORD)
            .run()
            .code,
        2
    );
}

#[test]
fn session_expires_and_is_swept() {
    let f = Fixture::new();
    f.unlock("1m");
    let early = f.cmd(&["--no-input", "list"]).at(T0 + 30_000).run();
    assert_eq!(early.code, 0, "{}", early.stderr);
    // The TTL is absolute: a successful use did not extend it.
    let late = f.cmd(&["--no-input", "list"]).at(T0 + 61_000).run();
    assert_eq!(late.code, 3);
    assert!(
        !f.session_file().exists() && !f.session_key().exists(),
        "swept at the start of the run"
    );
}

#[test]
fn any_invocation_sweeps_expired_parts() {
    let f = Fixture::new();
    f.unlock("1m");
    // `generate` never touches the store, but it still sweeps.
    let out = f.cmd(&["generate"]).at(T0 + 120_000).run();
    assert_eq!(out.code, 0);
    assert!(!f.session_file().exists() && !f.session_key().exists());
}

#[test]
fn session_tampered_file_refused() {
    let f = Fixture::new();
    f.unlock("5m");
    let mut bytes = std::fs::read(f.session_file()).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(f.session_file(), bytes).unwrap();
    let out = f.cmd(&["--no-input", "list"]).run();
    assert_eq!(out.code, 3);
    assert!(
        !f.session_file().exists() && !f.session_key().exists(),
        "both parts deleted"
    );
}

#[test]
fn session_keystore_mismatch_refused() {
    let f = Fixture::new();
    f.unlock("5m");
    // A well-formed key part that is not this session's.
    std::fs::write(f.session_key(), "A".repeat(64)).unwrap();
    let out = f.cmd(&["--no-input", "list"]).run();
    assert_eq!(out.code, 3);
    assert!(!f.session_file().exists() && !f.session_key().exists());
}

#[test]
fn session_invalid_after_password_change() {
    let f = Fixture::new();
    f.unlock("5m");
    assert_eq!(f.cmd(&["--no-input", "list"]).run().code, 0);
    // A password change made elsewhere (the desktop) replaces the account salt.
    let store = Store::open(&f.store).unwrap();
    let mut session = LockedSession::open(store)
        .unlock(
            &MasterPassword::new(PASSWORD.to_owned()),
            &SecretKey::parse(&f.secret_key).unwrap(),
        )
        .map_err(|e| e.error)
        .unwrap();
    session
        .change_master_password(
            &MasterPassword::new(PASSWORD.to_owned()),
            &MasterPassword::new("a brand new password!".to_owned()),
            &SecretKey::parse(&f.secret_key).unwrap(),
        )
        .unwrap();
    drop(session);
    let out = f.cmd(&["--no-input", "list"]).run();
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert!(!f.session_file().exists() && !f.session_key().exists());
}

#[test]
fn no_session_when_keystore_refuses() {
    let f = Fixture::new();
    let out = f
        .cmd(&["--json", "unlock", "--password-stdin"])
        .env("MOSAIC_TEST_KEYSTORE", "refuse")
        .stdin(PASSWORD)
        .run();
    assert_eq!(out.code, 1);
    assert_eq!(out.error_json()["error"]["code"], "sessionNotCached");
    assert!(!f.session_file().exists());
    // The command itself still works without a session, through the password.
    let out = f.cmd(&["--json", "list"]).prompts(&[PASSWORD]).run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        !f.session_file().exists(),
        "a password unlock outside `unlock` makes no session"
    );
}

#[test]
fn a_session_is_created_only_after_a_password_unlock_and_never_renewed() {
    let f = Fixture::new();
    f.unlock("1m");
    let first = f.cmd(&["--json", "status"]).run().json()["data"]["session"]["expiresAt"].clone();
    // A command run from the session does not create or extend one (decision L3).
    assert_eq!(
        f.cmd(&[
            "--no-input",
            "get",
            "mosaic://Personal/GitHub",
            "--field",
            "username"
        ])
        .at(T0 + 10_000)
        .run()
        .code,
        0
    );
    let second =
        f.cmd(&["--json", "status"]).at(T0 + 10_000).run().json()["data"]["session"]["expiresAt"]
            .clone();
    assert_eq!(first, second);
}

#[test]
fn usage_errors_name_the_position_not_the_value() {
    let f = Fixture::new();
    let out = f.cmd(&["lock", "topsecret-value"]).run();
    assert_eq!(out.code, 2);
    assert!(!out.stderr.contains("topsecret-value"), "{}", out.stderr);
    assert!(out.stderr.contains("#2"), "{}", out.stderr);
    let out = f.cmd(&["frobnicate-topsecret"]).run();
    assert_eq!(out.code, 2);
    assert!(!out.stderr.contains("topsecret"), "{}", out.stderr);
}
