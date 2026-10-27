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
    assert_eq!(config["app"]["withGlobalTauri"], json!(false));
    assert!(
        config
            .get("plugins")
            .is_none_or(|p| p.as_object().is_some_and(serde_json::Map::is_empty))
    );
}

/// Every command in `COMMANDS` is registered in the handler and granted by
/// the capability (the real `tauri.conf.json` and ACL, mock runtime).
#[test]
fn every_command_is_registered_and_allowed() {
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(tauri::generate_context!())
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
