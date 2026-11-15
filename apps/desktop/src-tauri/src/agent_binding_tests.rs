//! Agent bindings through the item IPC (ADR-0159, builder checks B1 and B4): saving a binding in
//! this device's editor confirms it, a binding that arrived by sync needs a review here, bindings
//! stay out of shared vaults and out of items whose value does not qualify, and the detail shows
//! no binding while agent access is off.

// Test helpers take owned values; short names for the vault, item and backend.
#![expect(
    clippy::needless_pass_by_value,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    reason = "test helpers take their arguments by value; short names for ID triples"
)]

use mosaic_agent::log::{LogEntry, LogKind};
use mosaic_core::Argon2Profile;
use mosaic_core::model::{AgentBindingRecord, AgentInject};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::items::ItemDetail;
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, UnlockRequest};
use crate::keystore::MemoryKeystore;
use crate::persist::Persistence;
use crate::state::{Backend, Slot};

const API_KEY: &str = "sk-live-0123456789abcdef";

fn req<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn backend() -> Backend {
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_persistence(Persistence::memory());
    backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test("correct horse battery staple"),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    backend
}

fn vault(b: &Backend) -> String {
    b.list_vaults().unwrap()[0].vault_id.0.to_string()
}

fn bearer(origins: &[&str]) -> Value {
    json!({
        "op": "add",
        "field": { "kind": "field", "field": "apiKeySecret" },
        "origins": origins,
        "inject": { "kind": "bearer", "headerName": null, "headerPrefix": null, "basicUser": null, "queryName": null },
        "env": null
    })
}

fn api_draft(secret: &str, bindings: Vec<Value>) -> Value {
    json!({
        "text": [{ "field": "title", "value": "Example API" }],
        "secrets": [{ "field": "apiKeySecret", "value": secret }],
        "urls": [], "customFields": [], "tags": null, "otp": null, "sshKey": null,
        "agentBindings": bindings
    })
}

fn add_api(b: &Backend, secret: &str, bindings: Vec<Value>) -> Result<ItemDetail, IpcError> {
    let v = vault(b);
    b.add_item(req(
        json!({ "vaultId": v, "kind": "apiKey", "draft": api_draft(secret, bindings) }),
    ))
}

fn enable(b: &Backend) {
    b.update_stored(|s| s.agent_access_enabled = true).unwrap();
}

fn ids(d: &ItemDetail) -> (String, String) {
    (
        d.summary.vault_id.0.to_string(),
        d.summary.item_id.0.to_string(),
    )
}

fn get(b: &Backend, d: &ItemDetail) -> ItemDetail {
    let (v, i) = ids(d);
    b.get_item(req(json!({ "vaultId": v, "itemId": i })))
        .unwrap()
}

fn log_entries(b: &Backend) -> Vec<LogEntry> {
    let slot = b.slot().unwrap();
    slot.unlocked()
        .unwrap()
        .agent_log(None, 100)
        .unwrap()
        .iter()
        .filter_map(|(_, plain)| LogEntry::from_json(plain.expose_secret()))
        .collect()
}

/// What another device's sync would leave: a changed binding with no mark here.
fn change_binding_elsewhere(b: &Backend, d: &ItemDetail, origins: &[&str]) {
    let (v, i) = (d.summary.vault_id.get(), d.summary.item_id.get());
    let mut slot = b.slot().unwrap();
    let session = slot.unlocked_mut().unwrap();
    let (id, _) = session
        .item(v, i)
        .unwrap()
        .agent_bindings()
        .into_iter()
        .next()
        .unwrap();
    let record = AgentBindingRecord::new("api_key.secret", origins, AgentInject::Bearer);
    session
        .update_item(v, i, |edit| edit.set_agent_binding(&id, &record))
        .unwrap();
}

#[test]
fn saving_a_binding_confirms_it_on_this_device() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let d = add_api(&b, API_KEY, vec![bearer(&["api.example.com"])]).unwrap();
    let [view] = d.agent_bindings.as_slice() else {
        panic!("one binding");
    };
    assert!(view.confirmed && view.usable);
    assert_eq!(
        view.origins,
        ["https://api.example.com"],
        "stored canonical"
    );
    assert!(view.last_confirmed.is_none());
    let entries = log_entries(&b);
    let [entry] = entries.as_slice() else {
        panic!("one log entry");
    };
    assert_eq!(entry.kind, LogKind::BindingConfirmed);
    assert_eq!(
        entry.origins.as_deref(),
        Some(&["https://api.example.com".to_owned()][..])
    );
    // The marks survive a read: nothing in the log or the view holds the value.
    let json = serde_json::to_string(&get(&b, &d)).unwrap();
    assert!(!json.contains(API_KEY), "canary found");
    assert!(
        !String::from_utf8_lossy(&entry.to_json()).contains(API_KEY),
        "canary found"
    );
}

#[test]
fn synced_binding_needs_review() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let d = add_api(&b, API_KEY, vec![bearer(&["https://api.example.com"])]).unwrap();
    change_binding_elsewhere(
        &b,
        &d,
        &["https://api.example.com", "https://evil.example.net"],
    );
    let after = get(&b, &d);
    let [view] = after.agent_bindings.as_slice() else {
        panic!("one binding");
    };
    assert!(
        !view.confirmed && !view.usable,
        "a changed binding needs a review"
    );
    assert_eq!(view.origins.len(), 2);
    let last = view.last_confirmed.as_ref().unwrap();
    assert_eq!(
        last.origins,
        ["https://api.example.com"],
        "the old destinations"
    );
}

#[test]
fn confirm_makes_it_usable() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let d = add_api(&b, API_KEY, vec![bearer(&["https://api.example.com"])]).unwrap();
    change_binding_elsewhere(
        &b,
        &d,
        &["https://api.example.com", "https://eu.example.com"],
    );
    let (v, i) = ids(&d);
    let element = get(&b, &d).agent_bindings[0].element_id.clone();
    let confirmed = b
        .agent_binding_confirm(&req(
            json!({ "vaultId": v, "itemId": i, "elementId": element }),
        ))
        .unwrap();
    assert!(confirmed.agent_bindings[0].confirmed && confirmed.agent_bindings[0].usable);
    assert_eq!(
        log_entries(&b)
            .iter()
            .filter(|e| e.kind == LogKind::BindingConfirmed)
            .count(),
        2
    );
    // An unknown binding is refused.
    let e = b
        .agent_binding_confirm(&req(
            json!({ "vaultId": v, "itemId": i, "elementId": "9-0" }),
        ))
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentBindingInvalid);
}

#[test]
fn bindings_refused_in_shared_vaults() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let d = add_api(&b, API_KEY, vec![]).unwrap();
    {
        // Shares the vault in the store (as a sync of its membership would), then unlocks again.
        let mut slot = b.slot().unwrap();
        let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken) else {
            panic!("unlocked");
        };
        let me = session.account_id();
        let mut store = session.lock().into_store();
        store
            .set_vault_sharing(&d.summary.vault_id.get(), 1, &me, false)
            .unwrap();
        *slot = Slot::Locked(LockedSession::open(store));
    }
    b.unlock(UnlockRequest {
        password: SecretIn::test("correct horse battery staple"),
        secret_key: None,
    })
    .unwrap();
    let (v, i) = ids(&d);
    let e = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": api_draft(API_KEY, vec![bearer(&["https://api.example.com"])])
        })))
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentVaultShared);
    assert!(get(&b, &d).agent_bindings.is_empty(), "nothing was saved");
}

#[test]
fn short_value_and_bad_origin_are_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let origin = ["https://api.example.com"];
    let code = |r: Result<ItemDetail, IpcError>| r.unwrap_err().code;
    assert_eq!(
        code(add_api(&b, "short", vec![bearer(&origin)])),
        ErrorCode::AgentValueUnusable
    );
    assert_eq!(
        code(add_api(&b, "0123456789ab\ncd", vec![bearer(&origin)])),
        ErrorCode::AgentValueUnusable
    );
    for bad in [
        "http://api.example.com",
        "https://user@api.example.com",
        "not a url",
        "https://api.example.com/path",
    ] {
        assert_eq!(
            code(add_api(&b, API_KEY, vec![bearer(&[bad])])),
            ErrorCode::AgentOriginInvalid,
            "{bad}"
        );
    }
    assert_eq!(
        code(add_api(&b, API_KEY, vec![bearer(&[])])),
        ErrorCode::AgentBindingInvalid,
        "no destination"
    );
    // A binding on an empty field has no value to qualify.
    let v = vault(&b);
    let empty = b.add_item(req(json!({
        "vaultId": v, "kind": "apiKey",
        "draft": {
            "text": [{ "field": "title", "value": "Empty" }], "secrets": [], "urls": [],
            "customFields": [], "tags": null, "otp": null, "sshKey": null,
            "agentBindings": [bearer(&origin)]
        }
    })));
    assert_eq!(code(empty), ErrorCode::AgentValueUnusable);
    // Nothing was stored by any of the refusals.
    assert!(
        b.list_items(&req(
            json!({ "vaultId": null, "trash": "exclude", "favoritesOnly": false, "query": null })
        ))
        .unwrap()
        .is_empty()
    );
}

#[test]
fn bindings_hidden_while_agent_access_is_off() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    // Off: a binding change is refused.
    let e = add_api(&b, API_KEY, vec![bearer(&["https://api.example.com"])]).unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentAccessOff);
    // On: it is saved; off again: the detail lists none, and the command refuses.
    enable(&b);
    let d = add_api(&b, API_KEY, vec![bearer(&["https://api.example.com"])]).unwrap();
    assert_eq!(d.agent_bindings.len(), 1);
    b.update_stored(|s| s.agent_access_enabled = false).unwrap();
    assert!(get(&b, &d).agent_bindings.is_empty());
    let (v, i) = ids(&d);
    let e = b
        .agent_binding_confirm(&req(
            json!({ "vaultId": v, "itemId": i, "elementId": "1-0" }),
        ))
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentAccessOff);
}

#[test]
fn update_replaces_and_remove_drops_a_binding() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let d = add_api(&b, API_KEY, vec![bearer(&["https://api.example.com"])]).unwrap();
    let (v, i) = ids(&d);
    let element = d.agent_bindings[0].element_id.clone();
    let update = json!({
        "op": "update", "elementId": element,
        "field": { "kind": "field", "field": "apiKeySecret" },
        "origins": ["https://api.example.com", "https://eu.example.com:8443"],
        "inject": { "kind": "query", "headerName": null, "headerPrefix": null, "basicUser": null, "queryName": "key" },
        "env": "EXAMPLE_API_KEY"
    });
    let updated = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null,
                       "otp": null, "sshKey": null, "agentBindings": [update] }
        })))
        .unwrap();
    let view = &updated.agent_bindings[0];
    assert!(view.confirmed, "an edit saved here is the review");
    assert_eq!(view.origins.len(), 2);
    assert_eq!(view.env.as_deref(), Some("EXAMPLE_API_KEY"));
    let remove = json!({ "op": "remove", "elementId": element });
    let removed = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null,
                       "otp": null, "sshKey": null, "agentBindings": [remove] }
        })))
        .unwrap();
    assert!(removed.agent_bindings.is_empty());
    // A removed (or unknown) binding cannot be updated.
    let again = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null,
                       "otp": null, "sshKey": null,
                       "agentBindings": [{ "op": "remove", "elementId": element }] }
        })))
        .unwrap_err();
    assert_eq!(again.code, ErrorCode::AgentBindingInvalid);
}

#[test]
fn a_hidden_custom_field_can_be_bound() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let v = vault(&b);
    let d: ItemDetail = b
        .add_item(req(json!({
            "vaultId": v, "kind": "login",
            "draft": { "text": [{ "field": "title", "value": "Svc" }], "secrets": [], "urls": [],
                       "customFields": [{ "op": "add", "label": "Token", "kind": "hidden", "value": API_KEY }],
                       "tags": null, "otp": null, "sshKey": null }
        })))
        .unwrap();
    let field = d.custom_fields[0].element_id.clone();
    let (v, i) = ids(&d);
    let updated = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null,
                       "otp": null, "sshKey": null,
                       "agentBindings": [{
                           "op": "add",
                           "field": { "kind": "customField", "elementId": field },
                           "origins": ["https://api.example.com"],
                           "inject": { "kind": "header", "headerName": "X-Api-Key", "headerPrefix": "", "basicUser": null, "queryName": null },
                           "env": null }] }
        })))
        .unwrap();
    assert!(updated.agent_bindings[0].usable);
    // A password-history entry is not a field an agent may use.
    let e = b
        .update_item(req(json!({
            "vaultId": v, "itemId": i,
            "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null,
                       "otp": null, "sshKey": null,
                       "agentBindings": [{
                           "op": "add",
                           "field": { "kind": "history", "index": 0 },
                           "origins": ["https://api.example.com"],
                           "inject": { "kind": "bearer", "headerName": null, "headerPrefix": null, "basicUser": null, "queryName": null },
                           "env": null }] }
        })))
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentBindingInvalid);
}

#[test]
fn a_basic_user_from_a_text_field_uses_the_wire_name() {
    let _kdf = crate::test_support::kdf_permit();
    let b = backend();
    enable(&b);
    let basic = |user: &str| {
        json!({
            "op": "add",
            "field": { "kind": "field", "field": "apiKeySecret" },
            "origins": ["https://api.example.com"],
            "inject": { "kind": "basic", "headerName": null, "headerPrefix": null,
                        "basicUser": { "kind": "field", "value": user }, "queryName": null },
            "env": null
        })
    };
    let draft = |binding: Value| {
        json!({
            "text": [{ "field": "title", "value": "Example API" }, { "field": "apiKeyId", "value": "key-id-1" }],
            "secrets": [{ "field": "apiKeySecret", "value": API_KEY }],
            "urls": [], "customFields": [], "tags": null, "otp": null, "sshKey": null,
            "agentBindings": [binding]
        })
    };
    let v = vault(&b);
    let d: ItemDetail = b
        .add_item(req(
            json!({ "vaultId": v, "kind": "apiKey", "draft": draft(basic("apiKeyId")) }),
        ))
        .unwrap();
    let user = d.agent_bindings[0].inject.basic_user.as_ref().unwrap();
    assert_eq!(
        user.value.as_deref(),
        Some("apiKeyId"),
        "the wire name, not the model key"
    );
    // The model key is not a wire name.
    let e = b
        .add_item(req(
            json!({ "vaultId": v, "kind": "apiKey", "draft": draft(basic("api_key.key_id")) }),
        ))
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentBindingInvalid);
}
