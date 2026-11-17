//! Crate tests: the session flow on a real in-memory store, and the wiring
//! of every command through Tauri's mock runtime with the real config and
//! capabilities.

use mosaic_core::Argon2Profile;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::commands::names::COMMANDS;
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::generator::GenerateRequest;
use crate::contract::items::{ListItemsRequest, TrashFilter};
use crate::contract::secret::SecretIn;
use crate::contract::session::{
    CreateAccountRequest, SessionState, UnlockRequest, VerifyKitRequest,
};
use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

fn backend() -> Backend {
    let store = Store::open_in_memory().unwrap();
    Backend::new(
        Slot::Locked(LockedSession::open(store)),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
}

fn create(backend: &Backend) -> String {
    let created = backend
        .create_account(CreateAccountRequest {
            email: " alex@example.com ".into(),
            password: SecretIn::test("correct horse battery staple"),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    let json = serde_json::to_value(&created).unwrap();
    json["emergencyKit"]["secretKey"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn unlock_request(password: &str, secret_key: Option<&str>) -> UnlockRequest {
    UnlockRequest {
        password: SecretIn::test(password),
        secret_key: secret_key.map(SecretIn::test),
    }
}

#[test]
fn account_lifecycle() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = backend();
    assert_eq!(backend.session_state().unwrap(), SessionState::NoAccount);
    assert_eq!(
        backend.list_vaults().unwrap_err(),
        IpcError::LOCKED,
        "nothing readable without an account"
    );

    let secret_key = create(&backend);
    assert!(secret_key.starts_with("M1-"));
    assert!(matches!(
        backend.session_state().unwrap(),
        SessionState::Unlocked { email: Some(ref e), .. } if e == "alex@example.com"
    ));
    let vaults = backend.list_vaults().unwrap();
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].name.as_deref(), Some("Personal"));

    let last_group = secret_key.rsplit('-').next().unwrap();
    let verify = |group: &str| {
        backend
            .verify_emergency_kit(&VerifyKitRequest {
                last_group: SecretIn::test(group),
                pending: false,
            })
            .unwrap()
            .matches
    };
    assert!(verify(last_group));
    assert!(!verify("0000"));

    let locked = backend.lock(LockReason::Manual).unwrap();
    assert!(matches!(
        locked,
        SessionState::Locked {
            secret_key_stored: true,
            ..
        }
    ));
    assert_eq!(
        backend.lock(LockReason::Manual).unwrap(),
        locked,
        "idempotent"
    );
    assert_eq!(backend.list_vaults().unwrap_err(), IpcError::LOCKED);

    let wrong = backend.unlock(unlock_request("wrong", None)).unwrap_err();
    assert_eq!(wrong.code, ErrorCode::WrongCredentials);
    let mistyped = backend
        .unlock(unlock_request(
            "correct horse battery staple",
            Some("M1-XXXX"),
        ))
        .unwrap_err();
    assert_eq!(mistyped.code, ErrorCode::SecretKeyMalformed);
    assert!(matches!(
        backend.session_state().unwrap(),
        SessionState::Locked { .. }
    ));

    let unlocked = backend
        .unlock(unlock_request("correct horse battery staple", None))
        .unwrap();
    assert!(matches!(unlocked, SessionState::Unlocked { .. }));
    let again = backend
        .unlock(unlock_request("correct horse battery staple", None))
        .unwrap_err();
    assert_eq!(again.code, ErrorCode::AlreadyUnlocked);

    let items = backend
        .list_items(&ListItemsRequest {
            vault_id: None,
            trash: TrashFilter::Exclude,
            favorites_only: false,
            query: Some("anything".into()),
        })
        .unwrap();
    assert_eq!(items, Vec::new());
}

#[test]
fn a_second_account_is_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = backend();
    create(&backend);
    backend.lock(LockReason::Manual).unwrap();
    let error = backend
        .create_account(CreateAccountRequest {
            email: "b@example.com".into(),
            password: SecretIn::test("pw"),
            default_vault_name: "Personal".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::AccountExists);
    assert!(matches!(
        backend.session_state().unwrap(),
        SessionState::Locked { .. }
    ));
}

#[test]
fn account_input_is_validated() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = backend();
    for (email, password, vault) in [
        ("not-an-email", "pw", "Personal"),
        ("a@example.com", "pw", "   "),
        ("a@example.com", "", "Personal"),
    ] {
        let error = backend
            .create_account(CreateAccountRequest {
                email: email.into(),
                password: SecretIn::test(password),
                default_vault_name: vault.into(),
            })
            .unwrap_err();
        assert!(
            matches!(
                error.code,
                ErrorCode::InvalidInput | ErrorCode::EmptyPassword
            ),
            "{email:?} {vault:?}: {error:?}"
        );
    }
    assert_eq!(backend.session_state().unwrap(), SessionState::NoAccount);
}

#[test]
fn a_typed_secret_key_unlocks_and_is_stored() {
    let _kdf = crate::test_support::kdf_permit();
    let first = backend();
    let secret_key = create(&first);
    // A fresh backend over the same store, with an empty keystore: what a
    // restart looks like until the OS keystore lands.
    let Slot::Unlocked(session) = std::mem::replace(&mut *first.slot().unwrap(), Slot::Taken)
    else {
        panic!("not unlocked");
    };
    let restarted = Backend::new(
        Slot::Locked(session.lock()),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    assert!(matches!(
        restarted.session_state().unwrap(),
        SessionState::Locked {
            secret_key_stored: false,
            ..
        }
    ));
    let needs_key = restarted
        .unlock(unlock_request("correct horse battery staple", None))
        .unwrap_err();
    assert_eq!(needs_key.code, ErrorCode::SecretKeyRequired);
    restarted
        .unlock(unlock_request(
            "correct horse battery staple",
            Some(&secret_key),
        ))
        .unwrap();
    restarted.lock(LockReason::Idle).unwrap();
    restarted
        .unlock(unlock_request("correct horse battery staple", None))
        .unwrap();
}

#[test]
fn generator_works_locked_and_rejects_bad_options() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = backend();
    let generated = backend
        .generate(GenerateRequest::Passphrase {
            words: 5,
            separator: "-".into(),
            capitalize: false,
            append_digit: false,
        })
        .unwrap();
    assert!(generated.entropy_bits > 60.0);
    assert!(backend.last_generated.lock().unwrap().is_some());
    let error = backend
        .generate(GenerateRequest::Password {
            length: 2,
            lowercase: true,
            uppercase: false,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}

#[test]
fn capability_grants_exactly_the_commands() {
    let text = include_str!("../capabilities/main.json");
    let capability: Value = serde_json::from_str(text).unwrap();
    assert_eq!(capability["windows"], json!(["main"]));
    assert!(
        capability.get("remote").is_none(),
        "no remote origin may invoke"
    );
    let granted: Vec<&str> = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    let expected: Vec<String> = COMMANDS
        .iter()
        .map(|c| format!("allow-{}", c.replace('_', "-")))
        .collect();
    assert_eq!(granted, expected);
}

/// The picker windows (ADR-0164) have a capability of their own each, which grants exactly their
/// picker commands, and no other window or capability grants a picker command.
#[test]
fn picker_capabilities_grant_exactly_the_picker_commands() {
    use crate::commands::names::{CHIP_COMMANDS, LIST_COMMANDS};
    let allow = |commands: &[&str]| -> Vec<String> {
        commands
            .iter()
            .map(|c| format!("allow-{}", c.replace('_', "-")))
            .collect()
    };
    for (text, window, commands) in [
        (
            include_str!("../capabilities/picker.json"),
            "picker",
            LIST_COMMANDS,
        ),
        (
            include_str!("../capabilities/chip.json"),
            "picker-chip",
            CHIP_COMMANDS,
        ),
    ] {
        let capability: Value = serde_json::from_str(text).unwrap();
        assert_eq!(capability["windows"], json!([window]));
        assert!(
            capability.get("remote").is_none(),
            "no remote origin may invoke"
        );
        let granted: Vec<String> = capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(granted, allow(commands), "{window}");
        for name in &granted {
            for denied in [
                "core",
                "shell",
                "fs",
                "http",
                "dialog",
                "opener",
                "clipboard",
            ] {
                assert!(!name.contains(denied), "{window} grants {name}");
            }
        }
    }
    // The main window grants no picker command.
    let main: Value = serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    for permission in main["permissions"].as_array().unwrap() {
        assert!(
            !permission.as_str().unwrap().starts_with("allow-picker"),
            "{permission}"
        );
    }
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(
        config["app"]["security"]["capabilities"],
        json!(["main", "picker", "chip"])
    );
}

#[test]
fn tauri_config_keeps_the_security_baseline() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let security = &config["app"]["security"];
    let csp = security["csp"].as_str().unwrap();
    for required in [
        "default-src 'none'",
        "script-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "frame-ancestors 'none'",
    ] {
        assert!(csp.contains(required), "CSP lacks {required}");
    }
    for forbidden in ["unsafe-eval", "unsafe-inline", "http:", "https:", "*"] {
        let sources = csp.replace("http://ipc.localhost", "");
        assert!(!sources.contains(forbidden), "CSP allows {forbidden}");
    }
    assert_eq!(security["freezePrototype"], json!(true));
    assert_eq!(security["capabilities"], json!(["main", "picker", "chip"]));
    assert_eq!(
        config["app"]["windows"][0]["contentProtected"],
        json!(true),
        "the main window is excluded from screen capture"
    );
    assert_eq!(config["app"]["withGlobalTauri"], json!(false));
    assert!(
        config
            .get("plugins")
            .is_none_or(|p| p.as_object().is_some_and(serde_json::Map::is_empty))
    );
}

/// A command that no handler registers is refused by the IPC
/// layer and touches nothing.
#[test]
fn unregistered_commands_are_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend()));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let error = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "export_vault".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({ "request": {} })),
            headers: tauri::http::HeaderMap::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_owned(),
        },
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("not found") || error.contains("not allowed"),
        "{error}"
    );
    let state = tauri::Manager::state::<AppState>(&app);
    assert!(state.backend().kit_pending.lock().unwrap().is_none());
    assert!(
        state
            .backend()
            .import_files
            .lock()
            .unwrap()
            .get("file-1")
            .is_none()
    );
}

#[test]
fn the_global_shortcut_is_the_documented_one() {
    use tauri_plugin_global_shortcut::{Code, Modifiers};
    let shortcut = crate::shortcut::shortcut();
    let expected = if cfg!(target_os = "macos") {
        Modifiers::ALT | Modifiers::SUPER
    } else {
        Modifiers::CONTROL | Modifiers::SHIFT
    };
    assert!(shortcut.matches(expected, Code::Space));
}

/// Every command in `COMMANDS` is registered in the handler and granted by
/// the capability (the real `tauri.conf.json` and ACL, mock runtime).
#[test]
fn every_command_is_registered_and_allowed() {
    let _kdf = crate::test_support::kdf_permit();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend()));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let call = |cmd: &str| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(json!({})),
                headers: tauri::http::HeaderMap::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_owned(),
            },
        )
    };
    for &cmd in COMMANDS {
        if let Err(error) = call(cmd) {
            let text = error.to_string();
            assert!(
                !text.contains("not found") && !text.contains("not allowed"),
                "{cmd}: {text}"
            );
        }
    }
    let unknown = call("not_a_command").unwrap_err().to_string();
    assert!(
        unknown.contains("not found") || unknown.contains("not allowed"),
        "{unknown}"
    );
}

/// Every picker command is registered, and each picker window is granted exactly its own
/// commands (the real ACL, mock runtime): the list window reaches the list commands and the chip
/// window the chip commands, and neither reaches the other's.
#[test]
fn every_picker_command_is_registered_and_allowed_for_its_window() {
    use crate::commands::names::{CHIP_COMMANDS, LIST_COMMANDS};
    let _kdf = crate::test_support::kdf_permit();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend()));
    let local = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    let call = |webview: &tauri::WebviewWindow<tauri::test::MockRuntime>, cmd: &str| {
        tauri::test::get_ipc_response(
            webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: local.parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(json!({})),
                headers: tauri::http::HeaderMap::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_owned(),
            },
        )
    };
    let refused = |result: Result<_, Value>| {
        result.err().is_some_and(|e| {
            let text = e.to_string();
            text.contains("not found") || text.contains("not allowed")
        })
    };
    let picker = tauri::WebviewWindowBuilder::new(&app, "picker", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let chip = tauri::WebviewWindowBuilder::new(&app, "picker-chip", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    // `picker_subscribe` needs a channel and is exercised by the surface tests of the windows.
    for &cmd in LIST_COMMANDS.iter().filter(|c| **c != "picker_subscribe") {
        assert!(!refused(call(&picker, cmd)), "list window: {cmd}");
        if !CHIP_COMMANDS.contains(&cmd) {
            assert!(
                refused(call(&chip, cmd)),
                "chip window must not reach {cmd}"
            );
        }
    }
    for &cmd in CHIP_COMMANDS.iter().filter(|c| **c != "picker_subscribe") {
        assert!(!refused(call(&chip, cmd)), "chip window: {cmd}");
        if !LIST_COMMANDS.contains(&cmd) {
            assert!(
                refused(call(&picker, cmd)),
                "list window must not reach {cmd}"
            );
        }
    }
}

/// Every command's request samples (the contract fixtures, as the webview
/// sends them) go through the real IPC layer and the command wrappers, on
/// a vault without an account and on an unlocked one: each answer is a
/// value or a contract error, never a Tauri deserialization error.
#[test]
fn every_request_sample_reaches_its_command() {
    let _kdf = crate::test_support::kdf_permit();
    let fixtures = crate::contract::fixtures::build();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend()));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    let call = |cmd: &str, body: Value| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: tauri::http::HeaderMap::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_owned(),
            },
        )
    };
    let run_all = |round: &str| {
        for &cmd in COMMANDS {
            let Some(samples) = fixtures["requests"][cmd].as_array() else {
                continue;
            };
            for sample in samples {
                let body = if sample.is_null() {
                    json!({})
                } else {
                    json!({ "request": sample })
                };
                if let Err(error) = call(cmd, body) {
                    assert!(
                        error.get("code").and_then(Value::as_str).is_some(),
                        "{round} {cmd}: {error}"
                    );
                }
            }
        }
    };
    // The first round creates the account with the `create_account`
    // sample (and leaves it unlocked until the `lock` sample); the next
    // rounds run against it.
    run_all("no account");
    let state = tauri::Manager::state::<AppState>(&app);
    assert_ne!(
        state.backend().session_state().unwrap(),
        crate::contract::session::SessionState::NoAccount
    );
    state
        .backend()
        .unlock(unlock_request("correct horse battery staple", None))
        .unwrap();
    run_all("unlocked");
    run_all("locked");
}

/// Cross-feature red team F4: a file dropped on the program-fill chip or list starts no import;
/// only the main window (the label of `tauri.conf.json`) takes drops.
#[test]
fn only_the_main_window_accepts_file_drops() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let main = config["app"]["windows"][0]["label"].as_str().unwrap();
    assert!(crate::accepts_file_drop(main));
    for label in [
        crate::commands::picker::CHIP_LABEL,
        crate::commands::picker::LIST_LABEL,
        "",
        "Main",
    ] {
        assert!(!crate::accepts_file_drop(label), "{label:?}");
    }
}
