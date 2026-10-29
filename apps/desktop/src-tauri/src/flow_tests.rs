//! One end-to-end flow through the command layer on a real (in-memory)
//! store: create an account, check the Emergency Kit, lock, unlock, add an
//! item, copy its password, watch the clipboard clear, lock again and see
//! that nothing is left behind.
//!
//! The zeroization itself is shown by the consumed `UnlockedSession`
//! (`session.lock()` drops it) together with the `mosaic-core` zeroize
//! tests. Memory cannot be inspected from safe Rust, so this test checks
//! the observable effects of the lock: the slot, the clipboard, the cached
//! generated password, the pending kit state, every secret command, the
//! agent, and the absence of secrets in events and errors.
#![allow(clippy::needless_pass_by_value)]

use std::time::{Duration, Instant};

use mosaic_core::{Argon2Profile, SecretKey};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::adapters::{Adapters, ImportService, SshAgentService};
use crate::clipboard::MemoryClipboard;
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, SessionState, UnlockRequest};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::state::{Backend, Slot};

const PASSWORD: &str = "correct horse battery staple";
const SECRET: &str = "Fl0w-Test-Secret-9271!";

fn req<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

fn log_has(log: &Log, wanted: &Value) -> bool {
    log.lock()
        .unwrap()
        .iter()
        .any(|entry| serde_json::from_str::<Value>(entry).unwrap() == *wanted)
}

fn wait_until(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    condition()
}

fn unlock(b: &Backend, password: &str) -> Result<SessionState, IpcError> {
    b.unlock(UnlockRequest {
        password: SecretIn::test(password),
        secret_key: None,
    })
}

#[test]
#[allow(clippy::too_many_lines)]
fn full_flow_create_unlock_add_copy_clear_lock() {
    let _kdf = crate::test_support::kdf_permit();
    let endpoint = crate::test_support::ssh_test_endpoint();
    let clip = MemoryClipboard::default();
    let (channel, log) = recording_channel();
    let b = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_clipboard(clip.factory())
    .with_adapters(Adapters {
        import: ImportService::default(),
        ssh: SshAgentService::new(Some(endpoint.endpoint.clone())),
    });
    b.subscribe(channel).unwrap();

    // 1. A fresh install has no account.
    assert_eq!(b.session_state().unwrap(), SessionState::NoAccount);

    // 2. Create the account: unlocked, the kit still unverified.
    let created = b
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    assert!(matches!(
        created.session,
        SessionState::Unlocked {
            kit_verified: false,
            ..
        }
    ));
    let kit_json = serde_json::to_value(&created.emergency_kit).unwrap();
    let secret_key_text = kit_json["secretKey"].as_str().unwrap().to_owned();
    SecretKey::parse(&secret_key_text).unwrap();
    let last_group = secret_key_text.rsplit('-').next().unwrap().to_owned();

    // 3. The kit check: a wrong group, then the right one.
    let wrong = b
        .verify_emergency_kit(&req(json!({ "lastGroup": "ZZZZ" })))
        .unwrap();
    assert!(!wrong.matches);
    let right = b
        .verify_emergency_kit(&req(json!({ "lastGroup": last_group })))
        .unwrap();
    assert!(right.matches);
    assert!(matches!(
        b.session_state().unwrap(),
        SessionState::Unlocked {
            kit_verified: true,
            ..
        }
    ));

    // 4. Lock: the device keeps the Secret Key, and the webview is told.
    let state = b.lock(LockReason::Manual).unwrap();
    assert!(matches!(
        state,
        SessionState::Locked {
            secret_key_stored: true,
            ..
        }
    ));
    assert!(log_has(
        &log,
        &json!({ "type": "locked", "reason": "manual" })
    ));

    // 5. Unlock: a wrong password fails and stays locked, the right one works.
    assert_eq!(
        code(unlock(&b, "wrong").unwrap_err()),
        ErrorCode::WrongCredentials
    );
    assert!(matches!(
        b.session_state().unwrap(),
        SessionState::Locked { .. }
    ));
    assert!(matches!(
        unlock(&b, PASSWORD).unwrap(),
        SessionState::Unlocked { .. }
    ));

    // 6. Add a login. The detail names the secret field but never holds it.
    let vault = b.list_vaults().unwrap()[0].vault_id.0.to_string();
    let detail = b
        .add_item(req(json!({
            "vaultId": vault,
            "kind": "login",
            "draft": {
                "text": [
                    { "field": "title", "value": "Example" },
                    { "field": "username", "value": "alex" }
                ],
                "secrets": [{ "field": "password", "value": SECRET }],
                "urls": [{ "op": "add", "url": "https://example.com", "matchMode": "exactOrigin", "allowHttp": false }],
                "customFields": [],
                "tags": null,
                "otp": null
            }
        })))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&detail.secrets).unwrap(),
        json!(["password"])
    );
    assert!(!serde_json::to_string(&detail).unwrap().contains(SECRET));
    let item = detail.summary.item_id.0.to_string();
    let copy_request = || {
        req(
            json!({ "source": { "kind": "item", "vaultId": vault, "itemId": item,
                                 "field": { "kind": "password" } } }),
        )
    };

    // 7. A one-second clipboard timeout (test only: this bypasses `normalize`).
    b.update_stored(|s| s.settings.clipboard_clear_seconds = Some(1))
        .unwrap();

    // 8. Copy: the clipboard holds the password and a clear is scheduled.
    let receipt = b.copy_to_clipboard(copy_request()).unwrap();
    assert_eq!(receipt.clear_after_seconds, Some(1));
    assert!(receipt.clears_at.is_some());
    assert_eq!(clip.content().as_deref(), Some(SECRET));

    // 9. Rust clears it on its own.
    assert!(wait_until(Duration::from_secs(5), || clip
        .content()
        .is_none()));
    assert!(wait_until(Duration::from_secs(5), || log_has(
        &log,
        &json!({ "type": "clipboardCleared" })
    )));

    // 10. With clearing off: copy again, reveal, generate, start the agent.
    b.update_stored(|s| s.settings.clipboard_clear_seconds = None)
        .unwrap();
    b.copy_to_clipboard(copy_request()).unwrap();
    assert_eq!(clip.content().as_deref(), Some(SECRET));
    let revealed = b
        .reveal_secret(&req(json!({
            "vaultId": vault, "itemId": item,
            "secret": { "kind": "field", "field": "password" }
        })))
        .unwrap();
    assert_eq!(serde_json::to_value(&revealed).unwrap()["value"], SECRET);
    b.generate(req(json!({
        "kind": "password", "length": 20, "lowercase": true, "uppercase": true,
        "digits": true, "symbols": false, "excludeAmbiguous": true
    })))
    .unwrap();
    assert!(b.last_generated.lock().unwrap().is_some());
    let agent = b
        .ssh_agent_set_enabled(req(json!({ "enabled": true })))
        .unwrap();
    assert!(agent.running);

    // 11. Lock: everything is gone.
    b.lock(LockReason::Manual).unwrap();
    assert!(matches!(*b.slot().unwrap(), Slot::Locked(_)));
    assert!(clip.content().is_none(), "the clipboard held our value");
    assert!(b.last_generated.lock().unwrap().is_none());
    assert!(b.kit_pending.lock().unwrap().is_none());
    let locked = ErrorCode::Locked;
    assert_eq!(
        code(
            b.get_item(req(json!({ "vaultId": vault, "itemId": item })))
                .unwrap_err()
        ),
        locked
    );
    assert_eq!(
        code(
            b.reveal_secret(&req(json!({
                "vaultId": vault, "itemId": item,
                "secret": { "kind": "field", "field": "password" }
            })))
            .unwrap_err()
        ),
        locked
    );
    assert_eq!(
        code(
            b.list_items(&req(json!({ "vaultId": null, "trash": "exclude",
                                       "favoritesOnly": false, "query": null })))
                .unwrap_err()
        ),
        locked
    );
    assert_eq!(
        code(
            b.security_report(mosaic_core::Timestamp::from_unix_millis(1_800_000_000_000))
                .unwrap_err()
        ),
        locked
    );
    assert_eq!(
        code(b.copy_to_clipboard(copy_request()).unwrap_err()),
        locked
    );
    assert_eq!(
        code(
            b.import_choose_file(req(json!({ "format": "csv" })))
                .unwrap_err()
        ),
        locked
    );
    assert!(!b.ssh_agent_status().running);
    let events: Vec<Value> = log
        .lock()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect();
    assert_eq!(
        events.last().unwrap(),
        &json!({ "type": "locked", "reason": "manual" })
    );
    let text = serde_json::to_string(&events).unwrap();
    assert!(!text.contains(SECRET) && !text.contains(&secret_key_text));
    let error_text = format!(
        "{:?}",
        b.get_item(req(json!({ "vaultId": vault, "itemId": item })))
    );
    assert!(!error_text.contains(SECRET) && !error_text.contains(&secret_key_text));

    // 12. Unlock again: the item survived the lock.
    unlock(&b, PASSWORD).unwrap();
    let all = b
        .list_items(&req(json!({ "vaultId": null, "trash": "exclude",
                                  "favoritesOnly": false, "query": null })))
        .unwrap();
    assert_eq!(all.len(), 1);
    b.lock(LockReason::Manual).unwrap();
    drop(endpoint);
}
