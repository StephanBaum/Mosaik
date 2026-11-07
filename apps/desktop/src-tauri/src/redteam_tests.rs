//! M3 IPC red-team suite: what a compromised webview (XSS-equivalent) can do
//! with the command set, run through the real IPC layer (mock runtime, real
//! capability) and the `Backend` methods with fake platforms.
//!
//! Each test names the attack it tries; all of them must hold:
//! - hostile request bodies never panic, never unlock and never return a secret;
//! - references that point at the wrong field, item or vault return no secret;
//! - the re-authentication grant dies with the session that earned it;
//! - wrong passwords from every path that takes one share the five-try limit;
//! - hardware-vault commands refuse other vaults before touching a device, and a
//!   vault that closes during the touch gets no new key;
//! - a tampered state file stays within the §11 floor and the 30-day cap.
#![expect(
    clippy::too_many_lines,
    reason = "test helpers take their arguments by value; each attack is one scenario"
)]

use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use mosaic_core::Argon2Profile;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::clipboard::MemoryClipboard;
use crate::commands::names::COMMANDS;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::items::ItemDetail;
use crate::contract::quick_unlock::{
    EnrollBiometricRequest, EnrollHardwareKeyRequest, QuickUnlockStatus,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{
    CreateAccountRequest, QuickUnlockKind, QuickUnlockRequest, SaveKitRequest, SessionState,
    UnlockRequest,
};
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::keystore::MemoryKeystore;
use crate::persist::Persistence;
use crate::quick_unlock::fakes::{FakeBiometric, FakeHardwareKey};
use crate::quick_unlock::{NoHardwareKeys, QuickUnlockService};
use crate::state::{AppState, Backend, Slot};

/// Not the contract fixtures' password, so a fixture request never unlocks.
const PASSWORD: &str = "red-team master password \u{2206}";
const WRONG: &str = "not the password";
const ITEM_PASSWORD: &str = "Rt-It3m-Pa55word!";
const HIDDEN_VALUE: &str = "rt-hidden-4711";
/// RFC 6238 test secret; it must never reach the webview.
const OTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn typed<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

fn create(b: &Backend) -> String {
    let created = b
        .create_account(CreateAccountRequest {
            email: "red@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn unlock_with_password(b: &Backend) {
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
}

fn first_vault(b: &Backend) -> String {
    b.list_vaults().unwrap()[0].vault_id.0.to_string()
}

fn login_draft() -> Value {
    json!({
        "text": [
            { "field": "title", "value": "Bank" },
            { "field": "username", "value": "red" }
        ],
        "secrets": [{ "field": "password", "value": ITEM_PASSWORD }],
        "urls": [{ "op": "add", "url": "https://bank.example/login", "matchMode": "exactOrigin", "allowHttp": false }],
        "customFields": [
            { "op": "add", "label": "PIN", "kind": "hidden", "value": HIDDEN_VALUE },
            { "op": "add", "label": "Mail", "kind": "email", "value": "r@example.org" },
            { "op": "add", "label": "Backup", "kind": "otp", "value": format!("otpauth://totp/Bank:red?secret={OTP_SECRET}") }
        ],
        "tags": null,
        "otp": { "op": "set", "uri": format!("otpauth://totp/Bank:red?secret={OTP_SECRET}&issuer=Bank") }
    })
}

fn add_login(b: &Backend, vault: &str) -> ItemDetail {
    b.add_item(typed(
        json!({ "vaultId": vault, "kind": "login", "draft": login_draft() }),
    ))
    .unwrap()
}

fn totp_draft() -> Value {
    json!({
        "text": [{ "field": "title", "value": "Codes" }],
        "secrets": [], "urls": [], "customFields": [], "tags": null,
        "otp": { "op": "set", "uri": format!("otpauth://totp/Bank:red?secret={OTP_SECRET}&issuer=Bank") }
    })
}

fn ids(detail: &ItemDetail) -> (String, String) {
    (
        detail.summary.vault_id.0.to_string(),
        detail.summary.item_id.0.to_string(),
    )
}

fn element_of(detail: &ItemDetail, label: &str) -> String {
    detail
        .custom_fields
        .iter()
        .find(|f| f.label == label)
        .unwrap()
        .element_id
        .clone()
}

/// A random, well-formed vault ID that no vault has.
fn unknown_vault() -> String {
    mosaic_core::VaultId::new_v7().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// 1. Hostile bodies through the real IPC layer
// ---------------------------------------------------------------------------

/// Values of the wrong type for any field (none deserializes as a secret).
fn type_breakers() -> Vec<Value> {
    let mut deep = json!([]);
    for _ in 0..100 {
        deep = json!([deep]);
    }
    vec![
        Value::Null,
        json!(true),
        json!(-1),
        json!(1.5),
        json!(u64::MAX),
        json!(i64::MIN),
        json!(f64::MAX),
        json!([]),
        json!({}),
        deep,
        json!({ "__proto__": { "polluted": true }, "constructor": { "prototype": { "polluted": true } } }),
    ]
}

/// Strings meant to confuse IDs, names and lookups. Only sent to commands
/// that do not run Argon2id on a string field, so the run stays fast and no
/// wrong-password counter moves.
fn string_breakers() -> Vec<Value> {
    vec![
        json!(""),
        json!("\u{0}"),
        json!("\u{202e}gpj.exe"),
        json!("A".repeat(64 * 1024)),
        json!("019A2B3C-4D5E-7001-8000-000000000001"),
        json!("{019a2b3c-4d5e-7001-8000-000000000001}"),
        json!("00000000-0000-0000-0000-000000000000"),
        json!("../../../../etc/passwd"),
        json!("ssh-1"),
        json!("file-1"),
    ]
}

/// Commands whose requests carry a password that a valid body checks with
/// Argon2id.
const KDF_COMMANDS: &[&str] = &[
    "create_account",
    "unlock",
    "reauthenticate",
    "update_settings",
    "save_emergency_kit",
    "enroll_biometric",
    "enroll_hardware_key",
    "import_preview",
];

/// Commands that may answer `internal` for a valid request in this rig (no
/// native dialog can be shown).
const DIALOG_COMMANDS: &[&str] = &["save_emergency_kit", "import_choose_file"];

/// Every hostile body for `cmd`: bare hostile requests, and each top-level
/// field of every contract sample replaced by a hostile value or joined by
/// an unknown one.
fn hostile_bodies(cmd: &str, samples: &[Value]) -> Vec<Value> {
    let mut breakers = type_breakers();
    if !KDF_COMMANDS.contains(&cmd) {
        breakers.extend(string_breakers());
    }
    let mut bodies = vec![json!({}), json!({ "request": {} }), json!({ "req": null })];
    bodies.extend(breakers.iter().map(|b| json!({ "request": b })));
    for sample in samples {
        let Some(object) = sample.as_object() else {
            continue;
        };
        for key in object.keys() {
            for breaker in &breakers {
                let mut mutated = object.clone();
                mutated.insert(key.clone(), breaker.clone());
                bodies.push(json!({ "request": mutated }));
            }
        }
        for extra in ["__proto__", "constructor", "isAdmin"] {
            let mut mutated = object.clone();
            mutated.insert(extra.into(), json!({ "polluted": true }));
            bodies.push(json!({ "request": mutated }));
        }
    }
    bodies
}

/// A contract error, or Tauri refusing the arguments.
fn assert_error_shape(round: &str, cmd: &str, shown: &str, error: &Value) {
    match error.get("code").and_then(Value::as_str) {
        Some("internal") => assert!(
            DIALOG_COMMANDS.contains(&cmd),
            "{round} {cmd}: internal (a panic or a poisoned lock?) for {shown}"
        ),
        Some(_) => {}
        None => assert!(
            error.is_string(),
            "{round} {cmd}: unexpected error shape {error} for {shown}"
        ),
    }
}

#[test]
fn hostile_ipc_bodies_are_refused_without_a_panic_an_unlock_or_a_secret() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    let secret_key = create(&backend);
    let vault = first_vault(&backend);
    add_login(&backend, &vault);
    backend.lock(LockReason::Manual).unwrap();

    let fixtures = crate::contract::fixtures::build();
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend));
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
    let state = tauri::Manager::state::<AppState>(&app);
    let secrets = [
        PASSWORD,
        ITEM_PASSWORD,
        HIDDEN_VALUE,
        OTP_SECRET,
        secret_key.as_str(),
    ];

    let run = |round: &str, skip: &[&str]| {
        let mut calls = 0usize;
        for &cmd in COMMANDS {
            if skip.contains(&cmd) {
                continue;
            }
            let samples = fixtures["requests"][cmd]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for body in hostile_bodies(cmd, &samples) {
                calls += 1;
                let shown = body.to_string();
                let shown = shown.get(..200).unwrap_or(&shown);
                let answer = match call(cmd, body) {
                    Ok(body) => {
                        serde_json::to_string(&body.deserialize::<Value>().unwrap()).unwrap()
                    }
                    Err(error) => {
                        assert_error_shape(round, cmd, shown, &error);
                        error.to_string()
                    }
                };
                for secret in secrets {
                    assert!(
                        !answer.contains(secret),
                        "{round} {cmd}: a secret in the answer to {shown}"
                    );
                }
            }
        }
        assert!(calls > 1000, "{round}: only {calls} hostile calls");
        // Nothing panicked while holding the session (a poisoned mutex
        // answers `internal` from here on).
        state.backend().session_state().unwrap()
    };

    // Locked: no hostile body reaches an unlocked state.
    let after = run("locked", &[]);
    assert!(matches!(after, SessionState::Locked { .. }), "{after:?}");
    assert!(!state.backend().is_unlocked());

    // Unlocked: deeper validation; `lock` is left out so the round stays
    // unlocked, and the vault keeps its item.
    unlock_with_password(state.backend());
    // `verify_emergency_kit` is left out too: its fifth wrong answer locks (own test).
    // So are the sync commands that check the master password first (a fresh
    // login, a password change): the fifth wrong password locks, like
    // `reauthenticate` (own tests in `sync_handler_tests`).
    let after = run(
        "unlocked",
        &[
            "lock",
            "verify_emergency_kit",
            "sync_hosted_connect",
            "sync_device_remove",
            "sync_totp_begin",
            "sync_totp_disable",
            "change_master_password",
        ],
    );
    assert!(matches!(after, SessionState::Unlocked { .. }), "{after:?}");
    assert_eq!(
        state
            .backend()
            .list_items(&typed(
                json!({ "vaultId": null, "trash": "include", "favoritesOnly": false, "query": null })
            ))
            .unwrap()
            .len(),
        1,
        "no hostile body added, purged or moved an item"
    );
}

// ---------------------------------------------------------------------------
// 2. Secret references that point elsewhere
// ---------------------------------------------------------------------------

#[test]
fn reveal_and_copy_never_return_a_secret_they_were_not_asked_for() {
    let _kdf = crate::test_support::kdf_permit();
    let clipboard = MemoryClipboard::default();
    let b = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_clipboard(clipboard.factory());
    create(&b);
    let vault = first_vault(&b);
    let other = b
        .create_vault(&typed(json!({ "name": "Other" })))
        .unwrap()
        .vault_id
        .0
        .to_string();
    let detail = add_login(&b, &vault);
    let (vault_id, item_id) = ids(&detail);
    let reveal = |vault: &str, item: &str, secret: Value| {
        b.reveal_secret(&typed(
            json!({ "vaultId": vault, "itemId": item, "secret": secret }),
        ))
    };
    let mut answers: Vec<String> = Vec::new();
    let mut refused = |result: IpcResult<crate::contract::items::RevealedSecret>,
                       expected: ErrorCode,
                       what: &str| {
        match result {
            Ok(value) => panic!(
                "{what}: revealed {}",
                serde_json::to_string(&value).unwrap()
            ),
            Err(error) => {
                assert_eq!(error.code, expected, "{what}");
                answers.push(serde_json::to_string(&error).unwrap());
            }
        }
    };

    // Elements that are not hidden fields, and positions past the history.
    for (label, what) in [("Mail", "an email field"), ("Backup", "an OTP field")] {
        refused(
            reveal(
                &vault_id,
                &item_id,
                json!({ "kind": "customField", "elementId": element_of(&detail, label) }),
            ),
            ErrorCode::FieldEmpty,
            what,
        );
    }
    for element in ["", "0-0", "\u{202e}", &"9".repeat(4096)] {
        refused(
            reveal(
                &vault_id,
                &item_id,
                json!({ "kind": "customField", "elementId": element }),
            ),
            ErrorCode::FieldEmpty,
            "an unknown element",
        );
    }
    for index in [0, 1, u32::MAX] {
        refused(
            reveal(
                &vault_id,
                &item_id,
                json!({ "kind": "history", "index": index }),
            ),
            ErrorCode::FieldEmpty,
            "a history entry that does not exist",
        );
    }
    refused(
        reveal(
            &vault_id,
            &item_id,
            json!({ "kind": "field", "field": "cardPin" }),
        ),
        ErrorCode::FieldEmpty,
        "a secret field the item does not have",
    );
    // The right item under the wrong vault, and an unknown vault.
    refused(
        reveal(
            &other,
            &item_id,
            json!({ "kind": "field", "field": "password" }),
        ),
        ErrorCode::ItemNotFound,
        "the item through another vault",
    );
    refused(
        reveal(
            &unknown_vault(),
            &item_id,
            json!({ "kind": "field", "field": "password" }),
        ),
        ErrorCode::VaultNotFound,
        "the item through an unknown vault",
    );

    // An OTP custom field copies its code, never its seed or URI.
    let copy = |field: Value| {
        b.copy_to_clipboard(typed(json!({ "source": {
            "kind": "item", "vaultId": vault_id, "itemId": item_id, "field": field
        }})))
    };
    copy(json!({ "kind": "customField", "elementId": element_of(&detail, "Backup") })).unwrap();
    let copied = clipboard.content().unwrap();
    assert!(
        copied.len() == 6 && copied.chars().all(|c| c.is_ascii_digit()),
        "an OTP field copies a code"
    );

    // Deleted forever: neither reveal nor copy brings the secret back.
    b.edit_item(
        typed(json!({ "vaultId": vault_id, "itemId": item_id })),
        mosaic_vault::session::UnlockedSession::purge_item,
    )
    .unwrap();
    match reveal(
        &vault_id,
        &item_id,
        json!({ "kind": "field", "field": "password" }),
    ) {
        Ok(value) => panic!(
            "a purged item revealed {}",
            serde_json::to_string(&value).unwrap()
        ),
        Err(error) => answers.push(serde_json::to_string(&error).unwrap()),
    }
    assert!(
        copy(json!({ "kind": "password" })).is_err(),
        "a purged item copies nothing"
    );
    assert!(
        !clipboard
            .content()
            .unwrap_or_default()
            .contains(ITEM_PASSWORD)
    );
    for answer in &answers {
        for secret in [ITEM_PASSWORD, HIDDEN_VALUE, OTP_SECRET] {
            assert!(!answer.contains(secret), "{answer}");
        }
    }
}

// ---------------------------------------------------------------------------
// Rigs with fake platforms
// ---------------------------------------------------------------------------

struct Rig {
    backend: Backend,
    bio: Arc<FakeBiometric>,
    key: Arc<FakeHardwareKey>,
}

/// An unlocked account with an available biometric platform and `physical`
/// fake security keys (key 0 plugged in).
fn rig(physical: usize) -> Rig {
    let bio = FakeBiometric::new(true);
    let key = FakeHardwareKey::new(physical);
    let service =
        QuickUnlockService::new(Some(Box::new(Arc::clone(&bio))), Box::new(Arc::clone(&key)));
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_quick_unlock(service);
    create(&backend);
    Rig { backend, bio, key }
}

fn enroll_bio(b: &Backend, password: Option<&str>) -> IpcResult<QuickUnlockStatus> {
    b.enroll_biometric(EnrollBiometricRequest {
        password: password.map(SecretIn::test),
    })
}

fn enroll_key(b: &Backend, password: Option<&str>) -> IpcResult<QuickUnlockStatus> {
    b.enroll_hardware_key(EnrollHardwareKeyRequest {
        password: password.map(SecretIn::test),
        pin: None,
    })
}

fn offered(b: &Backend) -> Vec<QuickUnlockKind> {
    match b.session_state().unwrap() {
        SessionState::Locked { quick_unlock, .. } => quick_unlock,
        other => panic!("not locked: {other:?}"),
    }
}

fn hw_vault(b: &Backend) -> String {
    b.create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
        .unwrap()
        .vault_id
        .0
        .to_string()
}

/// Any vault but `not` (the list order is not specified).
fn plain_vault(b: &Backend, not: &str) -> String {
    b.list_vaults()
        .unwrap()
        .into_iter()
        .map(|v| v.vault_id.0.to_string())
        .find(|id| id != not)
        .unwrap()
}

// ---------------------------------------------------------------------------
// 3. The re-authentication grant
// ---------------------------------------------------------------------------

#[test]
fn the_reauth_grant_dies_with_its_session_even_during_a_prompt() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = Arc::new(r.backend);
    let cell: Arc<OnceLock<Weak<Backend>>> = Arc::default();
    cell.set(Arc::downgrade(&b)).ok();

    // The OS prompt is open while the vault locks and the password unlocks
    // it again: the new session never earned the grant.
    let weak = Arc::clone(&cell);
    *r.bio.on_enroll.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            b.lock(LockReason::Manual).unwrap();
            unlock_with_password(&b);
        }
    }));
    assert_eq!(
        code(enroll_bio(&b, Some(PASSWORD)).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert!(!r.bio.has_key(), "the key made during the prompt is gone");
    assert!(!b.quick_unlock_status().unwrap().biometric.enrolled);
    *r.bio.on_enroll.lock().unwrap() = None;

    // A grant from before a lock opens nothing after the next unlock.
    let vault = hw_vault(&b);
    let item = b
        .add_item(typed(
            json!({ "vaultId": vault, "kind": "totp", "draft": totp_draft() }),
        ))
        .unwrap();
    let plain = plain_vault(&b, &vault);
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    b.lock(LockReason::Manual).unwrap();
    unlock_with_password(&b);
    b.open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
        .unwrap();
    let calls = r.key.calls();
    assert_eq!(
        code(
            b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        ErrorCode::ReauthRequired
    );
    assert_eq!(r.key.calls(), calls, "refused before the device");
    assert_eq!(
        code(
            b.move_item(typed(json!({
                "vaultId": vault,
                "itemId": item.summary.item_id.0.to_string(),
                "targetVaultId": plain,
            })))
            .unwrap_err()
        ),
        ErrorCode::ReauthRequired
    );
    assert_eq!(
        code(enroll_bio(&b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert_eq!(
        code(enroll_key(&b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    // An empty password is not a password, and does not count as a try.
    for empty in [Some(""), None] {
        assert_eq!(
            code(enroll_key(&b, empty).unwrap_err()),
            ErrorCode::ReauthRequired
        );
    }
    assert!(b.is_unlocked());
}

// ---------------------------------------------------------------------------
// 4. One limit for wrong passwords, whatever command carries them
// ---------------------------------------------------------------------------

/// One command that carries a wrong password.
type Attempt<'a> = Box<dyn Fn(&Backend) -> IpcError + 'a>;

#[test]
fn wrong_passwords_from_every_command_share_one_limit_and_disable_quick_unlock() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = &r.backend;
    enroll_bio(b, Some(PASSWORD)).unwrap();
    enroll_key(b, Some(PASSWORD)).unwrap();
    *b.kit_pending.lock().unwrap() = None;

    let weakening = Settings {
        clipboard_clear_seconds: None,
        ..Settings::default()
    };
    let attempts: [Attempt<'_>; 5] = [
        Box::new(|b| {
            b.update_settings(UpdateSettingsRequest {
                settings: weakening.clone(),
                password: Some(SecretIn::test(WRONG)),
            })
            .unwrap_err()
        }),
        Box::new(|b| {
            b.save_emergency_kit(SaveKitRequest {
                labels: crate::emergency_kit::tests::labels(),
                password: Some(SecretIn::test(WRONG)),
            })
            .unwrap_err()
        }),
        Box::new(|b| enroll_bio(b, Some(WRONG)).unwrap_err()),
        Box::new(|b| enroll_key(b, Some(WRONG)).unwrap_err()),
        Box::new(|b| b.reauthenticate(SecretIn::test(WRONG)).unwrap_err()),
    ];
    let (prompts, calls) = (r.bio.prompts(), r.key.calls());
    for (n, attempt) in attempts.iter().enumerate() {
        assert_eq!(code(attempt(b)), ErrorCode::WrongCredentials, "try {n}");
        assert_eq!(b.is_unlocked(), n < 4, "try {n}");
    }
    assert_eq!(
        r.bio.prompts(),
        prompts,
        "no wrong password reached a prompt"
    );
    assert_eq!(r.key.calls(), calls, "or a device");
    assert_eq!(
        b.settings_snapshot().clipboard_clear_seconds,
        Settings::default().clipboard_clear_seconds
    );

    // Both quick methods are enrolled, but neither opens before the
    // password: the guessing cannot continue behind a Hello prompt.
    assert_eq!(offered(b), vec![]);
    for method in [QuickUnlockKind::Biometric, QuickUnlockKind::HardwareKey] {
        assert_eq!(
            code(
                b.unlock_quick(&mut QuickUnlockRequest { method, pin: None })
                    .unwrap_err()
            ),
            ErrorCode::PasswordRequired
        );
    }
    assert_eq!(r.bio.prompts(), prompts);
    assert_eq!(r.key.calls(), calls);
    unlock_with_password(b);
}

// ---------------------------------------------------------------------------
// 5. Hardware-vault commands on the wrong vault, and a close during the touch
// ---------------------------------------------------------------------------

#[test]
fn hardware_vault_commands_refuse_other_vaults_before_the_device() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = &r.backend;
    let vault = hw_vault(b);
    let plain = plain_vault(b, &vault);
    let unknown = unknown_vault();
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let calls = r.key.calls();
    let method = "00112233445566778899aabbccddeeff";

    for (target, expected) in [
        (&plain, ErrorCode::InvalidInput),
        (&unknown, ErrorCode::VaultNotFound),
    ] {
        let open = b
            .open_hardware_vault(&mut typed(json!({ "vaultId": target, "pin": null })))
            .unwrap_err();
        let add = b
            .add_hardware_vault_key(&mut typed(json!({ "vaultId": target, "pin": null })))
            .unwrap_err();
        let remove = b
            .remove_hardware_vault_key(&typed(json!({ "vaultId": target, "methodId": method })))
            .unwrap_err();
        let close = b
            .close_hardware_vault(&typed(json!({ "vaultId": target })))
            .unwrap_err();
        let keys = b
            .hardware_vault_keys(&typed(json!({ "vaultId": target })))
            .unwrap_err();
        for (what, error) in [
            ("open", open),
            ("add", add),
            ("remove", remove),
            ("close", close),
            ("keys", keys),
        ] {
            assert_eq!(error.code, expected, "{what} on {target}");
        }
    }
    for name in ["", "   ", "a\u{0}b", "line\nbreak", &"n".repeat(257)] {
        assert_eq!(
            code(
                b.create_hardware_vault(&mut typed(json!({ "name": name, "pin": null })))
                    .unwrap_err()
            ),
            ErrorCode::InvalidInput,
            "{name:?}"
        );
    }
    assert_eq!(r.key.calls(), calls, "no device call for any of them");

    // Only TOTP items get in, by add and by move; a closed vault takes none.
    let login = add_login(b, &plain);
    let (_, login_id) = ids(&login);
    assert_eq!(
        code(
            b.move_item(typed(
                json!({ "vaultId": plain, "itemId": login_id, "targetVaultId": vault })
            ))
            .unwrap_err()
        ),
        ErrorCode::ItemTypeNotAllowed
    );
    let codes = b
        .add_item(typed(
            json!({ "vaultId": plain, "kind": "totp", "draft": totp_draft() }),
        ))
        .unwrap();
    b.close_hardware_vault(&typed(json!({ "vaultId": vault })))
        .unwrap();
    assert_eq!(
        code(
            b.move_item(typed(json!({
                "vaultId": plain,
                "itemId": codes.summary.item_id.0.to_string(),
                "targetVaultId": vault,
            })))
            .unwrap_err()
        ),
        ErrorCode::VaultUnavailable
    );
    assert_eq!(
        b.list_items(&typed(
            json!({ "vaultId": vault, "trash": "include", "favoritesOnly": false, "query": null })
        ))
        .unwrap()
        .len(),
        0
    );
}

#[test]
fn a_hardware_vault_that_closes_during_the_touch_gets_no_key_and_eight_is_the_cap() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = Arc::new(r.backend);
    let vault = hw_vault(&b);
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let cell: Arc<OnceLock<Weak<Backend>>> = Arc::default();
    cell.set(Arc::downgrade(&b)).ok();

    // The 5-minute tick (or the user) closes the vault while the key blinks.
    let (weak, id) = (Arc::clone(&cell), vault.clone());
    *r.key.on_call.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            b.close_hardware_vault(&typed(json!({ "vaultId": id })))
                .unwrap();
        }
    }));
    assert_eq!(
        code(
            b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        ErrorCode::VaultUnavailable
    );
    *r.key.on_call.lock().unwrap() = None;
    b.open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
        .unwrap();
    let keys = |b: &Backend| {
        b.hardware_vault_keys(&typed(json!({ "vaultId": vault })))
            .unwrap()
            .keys
            .len()
    };
    assert_eq!(keys(&b), 1, "nothing was added");

    // At most eight keys; the ninth is refused before the device.
    let add = |b: &Backend| {
        let mut result =
            b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })));
        if matches!(&result, Err(e) if e.code == ErrorCode::ReauthRequired) {
            b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
            result = b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })));
        }
        result
    };
    while keys(&b) < 8 {
        add(&b).unwrap();
    }
    let calls = r.key.calls();
    assert_eq!(code(add(&b).unwrap_err()), ErrorCode::HardwareKeyLimit);
    assert_eq!(r.key.calls(), calls);
    assert_eq!(keys(&b), 8);
}

// ---------------------------------------------------------------------------
// 6. Local malware edits the state file
// ---------------------------------------------------------------------------

#[test]
fn a_tampered_state_file_stays_within_the_floor_and_the_thirty_day_cap() {
    let dir = tempfile::tempdir().unwrap();
    for settings in [
        json!({
            "autoLockMinutes": null, "lockOnSleep": false, "lockOnScreenLock": false,
            "clipboardClearSeconds": 0, "theme": "dark", "language": "de",
            "quickUnlockMaxAgeDays": u32::MAX
        }),
        json!({
            "autoLockMinutes": u32::MAX, "lockOnSleep": true, "lockOnScreenLock": true,
            "clipboardClearSeconds": u32::MAX, "theme": "system", "language": "system",
            "quickUnlockMaxAgeDays": 0
        }),
    ] {
        let file = json!({ "version": 1, "settings": settings, "account": null });
        std::fs::write(
            dir.path().join(crate::persist::STATE_FILE),
            serde_json::to_vec(&file).unwrap(),
        )
        .unwrap();
        let b = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_persistence(Persistence::in_dir(dir.path()))
        .with_quick_unlock(QuickUnlockService::new(None, Box::new(NoHardwareKeys)));
        let s = b.settings_snapshot();
        assert!(
            crate::persist::has_lock_trigger(&s, crate::persist::SCREEN_LOCK_POSSIBLE),
            "{s:?}"
        );
        assert!(s.auto_lock_minutes.is_none_or(|m| (1..=480).contains(&m)));
        assert!(
            s.clipboard_clear_seconds
                .is_none_or(|c| (10..=300).contains(&c))
        );
        assert!((1..=30).contains(&s.quick_unlock_max_age_days));
        assert!(b.quick_unlock_policy().max_age() <= Duration::from_hours(30 * 24));
    }
}

// ---------------------------------------------------------------------------
// 7. Hardening after the red-team pass: one-use grant, attempt limits, names
// ---------------------------------------------------------------------------

#[test]
fn a_reauth_grant_pays_for_one_key_or_one_exit_and_failures_keep_it() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = &r.backend;
    let vault = hw_vault(b);
    let plain = plain_vault(b, &vault);
    let mut items = Vec::new();
    for _ in 0..2 {
        items.push(
            b.add_item(typed(
                json!({ "vaultId": vault, "kind": "totp", "draft": totp_draft() }),
            ))
            .unwrap(),
        );
    }
    let add_key = || b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })));
    let move_out = |i: usize, target: &str| {
        b.move_item(typed(json!({
            "vaultId": vault,
            "itemId": items[i].summary.item_id.0.to_string(),
            "targetVaultId": target,
        })))
    };

    // Adding a key uses the grant up.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    assert_eq!(add_key().unwrap().keys.len(), 2);
    assert_eq!(code(add_key().unwrap_err()), ErrorCode::ReauthRequired);

    // Moving out of the vault uses it up too; a move that fails keeps it.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    assert_eq!(
        code(move_out(0, &vault).unwrap_err()),
        ErrorCode::SameVault,
        "a failed move"
    );
    move_out(0, &plain).unwrap();
    assert_eq!(
        code(move_out(1, &plain).unwrap_err()),
        ErrorCode::ReauthRequired,
        "the grant went with the first move"
    );
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    move_out(1, &plain).unwrap();

    // Moving between ordinary vaults neither needs nor uses the grant.
    let note = b
        .add_item(typed(
            json!({ "vaultId": plain, "kind": "totp", "draft": totp_draft() }),
        ))
        .unwrap();
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    b.move_item(typed(json!({
        "vaultId": plain,
        "itemId": note.summary.item_id.0.to_string(),
        "targetVaultId": vault,
    })))
    .unwrap();
    assert!(b.quick_unlock.reauth_granted(), "still held");
}

#[test]
fn five_wrong_kit_answers_lock_the_vault_and_need_the_password() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    let secret_key = create(&backend);
    let b = &backend;
    let right = secret_key.rsplit('-').next().unwrap().to_owned();
    let check = |group: &str| {
        b.verify_emergency_kit(&typed(json!({ "lastGroup": group })))
            .map(|v| v.matches)
    };
    for _ in 0..4 {
        assert_eq!(check("WRNG"), Ok(false));
    }
    assert_eq!(check(&right), Ok(true), "a right answer resets the count");
    for _ in 0..4 {
        assert_eq!(check("WRNG"), Ok(false));
    }
    assert_eq!(
        code(check("WRNG").unwrap_err()),
        ErrorCode::TooManyAttempts,
        "the fifth wrong one"
    );
    assert!(!b.is_unlocked(), "the vault locked");
    assert_eq!(code(check(&right).unwrap_err()), ErrorCode::Locked);
    unlock_with_password(b);
    assert_eq!(check("WRNG"), Ok(false), "the count started over");
    assert_eq!(check(&right), Ok(true));
}

static CLOCK_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fake_now() -> std::time::Instant {
    static BASE: OnceLock<std::time::Instant> = OnceLock::new();
    *BASE.get_or_init(std::time::Instant::now)
        + Duration::from_millis(CLOCK_MS.load(std::sync::atomic::Ordering::SeqCst))
}

fn advance(ms: u64) {
    CLOCK_MS.fetch_add(ms, std::sync::atomic::Ordering::SeqCst);
}

#[test]
fn wrong_passwords_back_off_from_the_third_one_and_a_success_resets() {
    let _kdf = crate::test_support::kdf_permit();
    let mut backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    backend.unlock_clock = fake_now;
    let b = &backend;
    create(b);
    b.lock(LockReason::Manual).unwrap();
    let attempt = |password: &str| {
        b.unlock(UnlockRequest {
            password: SecretIn::test(password),
            secret_key: None,
        })
    };
    for _ in 0..3 {
        assert_eq!(
            code(attempt(WRONG).unwrap_err()),
            ErrorCode::WrongCredentials
        );
    }
    // Refused before any key derivation, even with the right password.
    let started = std::time::Instant::now();
    assert_eq!(
        code(attempt(PASSWORD).unwrap_err()),
        ErrorCode::UnlockDelayed
    );
    assert!(started.elapsed() < Duration::from_millis(500), "no KDF ran");
    advance(999);
    assert_eq!(
        code(attempt(PASSWORD).unwrap_err()),
        ErrorCode::UnlockDelayed
    );
    advance(1);
    // The 4th wrong password doubles the wait; a refused attempt is not counted.
    assert_eq!(
        code(attempt(WRONG).unwrap_err()),
        ErrorCode::WrongCredentials
    );
    advance(1_999);
    assert_eq!(code(attempt(WRONG).unwrap_err()), ErrorCode::UnlockDelayed);
    advance(1);
    // The right password works once the wait is over and clears the count.
    attempt(PASSWORD).unwrap();
    b.lock(LockReason::Manual).unwrap();
    for _ in 0..2 {
        assert_eq!(
            code(attempt(WRONG).unwrap_err()),
            ErrorCode::WrongCredentials,
            "free again"
        );
    }
    attempt(PASSWORD).unwrap();
    // Other commands are not held up by the wait: it never sleeps.
    b.lock(LockReason::Manual).unwrap();
    for _ in 0..3 {
        let _ = attempt(WRONG);
    }
    assert!(b.session_state().is_ok());
}

#[test]
fn names_with_bidi_or_zero_width_characters_are_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(0);
    let b = &r.backend;
    let vault = first_vault(b);
    let bad = [
        "\u{061C}",
        "\u{200B}",
        "\u{200C}",
        "\u{200D}",
        "\u{200E}",
        "\u{200F}",
        "\u{202A}",
        "\u{202B}",
        "\u{202C}",
        "\u{202D}",
        "\u{202E}",
        "\u{2066}",
        "\u{2067}",
        "\u{2068}",
        "\u{2069}",
        "\u{FEFF}",
        "\u{00AD}",
        "\u{034F}",
        "\u{115F}",
        "\u{1160}",
        "\u{180E}",
        "\u{2060}",
        "\u{2064}",
        "\u{206F}",
        "\u{3164}",
        "\u{FE00}",
        "\u{FE0F}",
        "\u{FFF9}",
        "\u{FFFB}",
        "\u{E0001}",
        "\u{E0020}",
        "\u{E0FFF}",
    ];
    for c in bad {
        for name in [
            format!("{c}Work"),
            format!("Work{c}gpj"),
            format!("Work{c}"),
        ] {
            assert_eq!(
                code(b.create_vault(&typed(json!({ "name": name }))).unwrap_err()),
                ErrorCode::InvalidInput,
                "create {c:?}"
            );
            assert_eq!(
                code(
                    b.rename_vault(&typed(json!({ "vaultId": vault, "name": name })))
                        .unwrap_err()
                ),
                ErrorCode::InvalidInput,
                "rename {c:?}"
            );
        }
    }
    // Ordinary non-ASCII names stay fine.
    for ok in [
        "Privat \u{fc}\u{df}",
        "\u{4f60}\u{597d}",
        "Caf\u{e9} \u{1F510}",
    ] {
        b.create_vault(&typed(json!({ "name": ok }))).unwrap();
    }
    // The first vault of a new account is checked too, before any key work.
    let fresh = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    let error = fresh
        .create_account(CreateAccountRequest {
            email: "x@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Pers\u{202e}onal".into(),
        })
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::InvalidInput);
}

// ---------------------------------------------------------------------------
// 4. Races between parallel commands
// ---------------------------------------------------------------------------

/// A clock that never moves, so a backoff set by one attempt stays in force
/// for the whole test (and no other test's clock can end it).
fn frozen_now() -> std::time::Instant {
    static BASE: OnceLock<std::time::Instant> = OnceLock::new();
    *BASE.get_or_init(std::time::Instant::now)
}

#[test]
fn parallel_wrong_passwords_run_one_derivation_past_the_backoff() {
    let _kdf = crate::test_support::kdf_permit();
    let mut backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    backend.unlock_clock = frozen_now;
    let b = &backend;
    create(b);
    b.lock(LockReason::Manual).unwrap();
    let attempt = |password: &str| {
        b.unlock(UnlockRequest {
            password: SecretIn::test(password),
            secret_key: None,
        })
    };
    for _ in 0..2 {
        assert_eq!(
            code(attempt(WRONG).unwrap_err()),
            ErrorCode::WrongCredentials
        );
    }
    // Eight parallel attempts at the edge of the limit: the one that runs
    // is the third wrong password, which starts the backoff; the other seven
    // read the throttle only after it and are refused before any derivation.
    let barrier = std::sync::Barrier::new(8);
    let codes: Vec<ErrorCode> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    code(attempt(WRONG).unwrap_err())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(
        codes
            .iter()
            .filter(|c| **c == ErrorCode::WrongCredentials)
            .count(),
        1,
        "{codes:?}"
    );
    assert_eq!(
        codes
            .iter()
            .filter(|c| **c == ErrorCode::UnlockDelayed)
            .count(),
        7,
        "{codes:?}"
    );
}

#[test]
fn parallel_kit_guesses_cannot_pass_the_five_answer_cap() {
    let _kdf = crate::test_support::kdf_permit();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    create(&backend);
    let b = &backend;
    let barrier = std::sync::Barrier::new(16);
    let results: Vec<Result<bool, ErrorCode>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..16)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    b.verify_emergency_kit(&typed(json!({ "lastGroup": "WRNG" })))
                        .map(|v| v.matches)
                        .map_err(code)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let answered = results.iter().filter(|r| **r == Ok(false)).count();
    assert_eq!(answered, 4, "four wrong answers are answered: {results:?}");
    // The fifth wrong answer locks. Guesses that arrive between its cap check and the lock see
    // the full counter and are refused the same way, so at least one, not exactly one, says so.
    assert!(
        results.contains(&Err(ErrorCode::TooManyAttempts)),
        "the fifth locks: {results:?}"
    );
    assert!(
        results.iter().all(|r| matches!(
            r,
            Ok(false) | Err(ErrorCode::TooManyAttempts | ErrorCode::Locked)
        )),
        "{results:?}"
    );
    assert!(!b.is_unlocked());
}

#[test]
fn one_reauth_grant_lets_only_one_of_two_parallel_moves_out_of_a_hardware_vault() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(1);
    let b = &r.backend;
    let vault = hw_vault(b);
    let plain = plain_vault(b, &vault);
    let items: Vec<_> = (0..2)
        .map(|_| {
            b.add_item(typed(
                json!({ "vaultId": vault, "kind": "totp", "draft": totp_draft() }),
            ))
            .unwrap()
        })
        .collect();
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let results: Vec<Result<(), ErrorCode>> = std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .iter()
            .map(|item| {
                let (vault, plain) = (&vault, &plain);
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    b.move_item(typed(json!({
                        "vaultId": vault,
                        "itemId": item.summary.item_id.0.to_string(),
                        "targetVaultId": plain,
                    })))
                    .map(|_| ())
                    .map_err(code)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(ErrorCode::ReauthRequired))
            .count(),
        1,
        "{results:?}"
    );
    assert!(!b.quick_unlock.reauth_granted(), "the grant was spent");
}

#[test]
fn a_reauth_grant_taken_out_before_a_lock_is_not_put_back_after_it() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(0);
    let b = &r.backend;
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let grant = b.quick_unlock.take_reauth().expect("granted");
    assert!(!b.quick_unlock.reauth_granted(), "taken out");
    b.lock(LockReason::Manual).unwrap();
    unlock_with_password(b);
    b.quick_unlock.restore_reauth(grant);
    assert!(
        !b.quick_unlock.reauth_granted(),
        "a failed command of the old session cannot revive its grant"
    );
    // Within the same session a failed command does hand it back.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let grant = b.quick_unlock.take_reauth().expect("granted");
    b.quick_unlock.restore_reauth(grant);
    assert!(b.quick_unlock.reauth_granted());
}

#[test]
fn item_titles_with_invisible_or_direction_changing_characters_are_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(0);
    let b = &r.backend;
    let vault = first_vault(b);
    let draft = |title: &str| {
        json!({
            "text": [{ "field": "title", "value": title }],
            "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null
        })
    };
    for c in [
        "\u{202E}",
        "\u{200B}",
        "\u{00AD}",
        "\u{FE0F}",
        "\u{E0001}",
        "\u{2066}",
        "\u{FEFF}",
    ] {
        assert_eq!(
            code(
                b.add_item(typed(
                    json!({ "vaultId": vault, "kind": "login", "draft": draft(&format!("Ba{c}nk")) })
                ))
                .unwrap_err()
            ),
            ErrorCode::InvalidInput,
            "add {c:?}"
        );
    }
    let ok = b
        .add_item(typed(
            json!({ "vaultId": vault, "kind": "login", "draft": draft("Caf\u{e9} \u{1F510}") }),
        ))
        .unwrap();
    let (vault_id, item_id) = ids(&ok);
    let edit = b.update_item(typed(json!({
        "vaultId": vault_id, "itemId": item_id, "draft": draft("Ev\u{202E}il")
    })));
    assert_eq!(code(edit.unwrap_err()), ErrorCode::InvalidInput, "update");
}

// ---------------------------------------------------------------------------
// 9. A passkey's private key (M5, ADR-0039 D7)
// ---------------------------------------------------------------------------

/// Every way a private key might be written out: hex, base64, base64url, as a
/// JSON array, for the whole PKCS#8 document and for the bare scalar.
pub(crate) fn key_needles(pkcs8: &[u8]) -> Vec<String> {
    use core::fmt::Write as _;

    use p256::pkcs8::DecodePrivateKey as _;
    let scalar = p256::SecretKey::from_pkcs8_der(pkcs8).unwrap().to_bytes();
    let mut needles = Vec::new();
    for bytes in [pkcs8, scalar.as_slice()] {
        let url = mosaic_passkey::b64url(bytes);
        needles.push(url.replace('-', "+").replace('_', "/"));
        needles.push(url);
        needles.push(bytes.iter().fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        }));
        needles.push(serde_json::to_string(&bytes.to_vec()).unwrap());
    }
    needles
}

#[test]
fn a_passkey_private_key_reaches_no_command_and_no_item_view() {
    use mosaic_core::Timestamp;
    use mosaic_core::model::{ItemType, PasskeyCredential, Text};
    use mosaic_passkey::CreateInput;

    let _kdf = crate::test_support::kdf_permit();
    let clipboard = MemoryClipboard::default();
    let b = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_clipboard(clipboard.factory());
    create(&b);
    let vault = first_vault(&b);
    let created = mosaic_passkey::create(&CreateInput {
        rp_id: "example.com",
        origin: "https://example.com",
        challenge: &[7; 32],
        user_handle: &[9; 16],
        algorithms: &[-7],
        user_verified: false,
    })
    .unwrap();
    let der = created.private_key_pkcs8.expose_secret().to_vec();
    let mut credential = PasskeyCredential::new(
        created.credential_id,
        Text::new("example.com"),
        vec![9; 16],
        created.private_key_pkcs8,
        -7,
        Timestamp::from_unix_millis(1_000),
    );
    credential.user_name = Text::new("red");
    let item = {
        let mut slot = b.slot().unwrap();
        let session = slot.unlocked_mut().unwrap();
        let vault_id = session.vaults()[0].vault_id;
        session
            .add_item(vault_id, &ItemType::Passkey, |e| {
                e.set_title("example.com")?;
                e.set_passkey(&credential)
            })
            .unwrap()
            .to_string()
    };
    let needles = key_needles(&der);

    let mut views = Vec::new();
    let detail = b
        .get_item(typed(json!({ "vaultId": vault, "itemId": item })))
        .unwrap();
    assert!(detail.secrets.is_empty() && detail.passkey.is_some());
    views.push(serde_json::to_string(&detail).unwrap());
    for query in [Value::Null, json!("example")] {
        let listed = b
            .list_items(&typed(json!({
                "vaultId": null, "trash": "include", "favoritesOnly": false, "query": query
            })))
            .unwrap();
        assert_eq!(listed.len(), 1);
        views.push(serde_json::to_string(&listed).unwrap());
    }
    // No secret field, hidden field or history entry is the key.
    let fields: Vec<Value> = crate::contract::items::SecretFieldKey::ALL
        .iter()
        .map(|field| serde_json::to_value(field).unwrap())
        .collect();
    for field in &fields {
        let revealed = b.reveal_secret(&typed(json!({
            "vaultId": vault, "itemId": item, "secret": { "kind": "field", "field": field }
        })));
        assert_eq!(
            code(revealed.unwrap_err()),
            ErrorCode::FieldEmpty,
            "{field}"
        );
        let copied = b.copy_to_clipboard(typed(json!({
            "source": { "kind": "item", "vaultId": vault, "itemId": item,
                        "field": { "kind": "secret", "field": field } }
        })));
        assert!(copied.is_err(), "{field}");
    }
    for secret in [
        json!({ "kind": "history", "index": 0 }),
        json!({ "kind": "customField", "elementId": "passkey" }),
        json!({ "kind": "customField", "elementId": "private_key" }),
    ] {
        let revealed = b.reveal_secret(&typed(json!({
            "vaultId": vault, "itemId": item, "secret": secret
        })));
        assert!(revealed.is_err(), "{secret}");
    }
    assert!(clipboard.content().is_none(), "nothing was copied");

    for view in &views {
        assert!(
            !view.contains("privateKey") && !view.contains("private_key"),
            "{view}"
        );
        for needle in &needles {
            assert!(!view.contains(needle.as_str()), "{view}");
        }
    }
}
