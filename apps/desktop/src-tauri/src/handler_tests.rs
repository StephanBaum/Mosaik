//! Tests of every command handler (`Backend` methods) on an in-memory store,
//! with requests built from their wire JSON, an in-memory clipboard and a
//! recording event channel. The last tests check that no secret shows up in
//! an error, an event, a non-secret response or a debug print.
// Requests are built inline as JSON values and handed over by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::{AccountId, Argon2Profile, SecretKey, SecretString, Timestamp};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::adapters::Adapters;
use crate::autolock::{Monitor, OsEvent, tick_decision};
use crate::clipboard::MemoryClipboard;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::import::{ImportFileChoice, ImportFormat, ImportPreview};
use crate::contract::items::{ItemDetail, ItemSummary};
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, SessionState, UnlockRequest};
use crate::contract::session::{KitSaved, SaveKitRequest};
use crate::contract::settings::{Settings, Theme, UpdateSettingsRequest};
use crate::dialogs::{Dialogs, FileFilter};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::{MemoryKeystore, SecretKeyStore};
use crate::persist::Persistence;
use crate::state::{AppState, Backend, Slot};

const NOW: Timestamp = Timestamp::from_unix_millis(1_800_000_000_000);
const PASSWORD: &str = "correct horse battery staple";
const ITEM_PASSWORD: &str = "It3m-Pa55word!";
const HIDDEN_VALUE: &str = "8642-hidden-pin";
/// RFC 4226 / 6238 test secret "12345678901234567890".
const OTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn req<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

/// A keystore the test keeps a handle to (survives a "restart").
#[derive(Clone, Default)]
struct SharedKeystore(Arc<MemoryKeystore>);

impl SecretKeyStore for SharedKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        self.0.load(account)
    }
    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        self.0.save(account, key)
    }
    fn delete(&self, account: AccountId) -> IpcResult<()> {
        self.0.delete(account)
    }
}

/// Answers every dialog with a fixed path.
struct FixedDialog(Option<PathBuf>, Arc<Mutex<Vec<&'static str>>>);

impl Dialogs for FixedDialog {
    fn save_file(&self, _: &str, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.1.lock().unwrap().push(filter.name);
        Ok(self.0.clone())
    }
    fn open_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.1.lock().unwrap().push(filter.name);
        Ok(self.0.clone())
    }
}

struct Harness {
    backend: Backend,
    clipboard: MemoryClipboard,
    events: Log,
    secret_key: String,
}

fn backend_with(
    store: Store,
    keystore: SharedKeystore,
    persistence: Persistence,
) -> (Backend, MemoryClipboard, Log) {
    let clipboard = MemoryClipboard::default();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(store)),
        Box::new(keystore),
        || Ok(Argon2Profile::Baseline),
    )
    .with_persistence(persistence)
    .with_clipboard(clipboard.factory());
    let (channel, log) = recording_channel();
    backend.subscribe(channel).unwrap();
    (backend, clipboard, log)
}

fn create(backend: &Backend) -> String {
    let created = backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn unlocked() -> Harness {
    let (backend, clipboard, events) = backend_with(
        Store::open_in_memory().unwrap(),
        SharedKeystore::default(),
        Persistence::memory(),
    );
    let secret_key = create(&backend);
    Harness {
        backend,
        clipboard,
        events,
        secret_key,
    }
}

fn vault_id(backend: &Backend) -> String {
    backend.list_vaults().unwrap()[0].vault_id.0.to_string()
}

fn events(log: &Log) -> Vec<Value> {
    log.lock()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn login_draft() -> Value {
    json!({
        "text": [
            { "field": "title", "value": "GitHub" },
            { "field": "username", "value": "alex" },
            { "field": "notes", "value": null }
        ],
        "secrets": [{ "field": "password", "value": ITEM_PASSWORD }],
        "urls": [{ "op": "add", "url": "https://github.com/login", "matchMode": "exactOrigin", "allowHttp": false }],
        "customFields": [
            { "op": "add", "label": "PIN", "kind": "hidden", "value": HIDDEN_VALUE },
            { "op": "add", "label": "Recovery", "kind": "email", "value": "r@example.org" },
            { "op": "add", "label": "Backup code", "kind": "otp", "value": format!("otpauth://totp/Backup:alex?secret={OTP_SECRET}") }
        ],
        "tags": ["work", "dev"],
        "otp": { "op": "set", "uri": format!("otpauth://totp/GitHub:alex?secret={OTP_SECRET}&issuer=GitHub") }
    })
}

fn add_login(backend: &Backend, vault: &str) -> ItemDetail {
    backend
        .add_item(req(
            json!({ "vaultId": vault, "kind": "login", "draft": login_draft() }),
        ))
        .unwrap()
}

fn item_req(detail: &ItemDetail) -> Value {
    json!({ "vaultId": detail.summary.vault_id.0.to_string(), "itemId": detail.summary.item_id.0.to_string() })
}

fn with_item(detail: &ItemDetail, extra: Value) -> Value {
    let mut value = item_req(detail);
    if let (Some(map), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        map.extend(extra.clone());
    }
    value
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

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

#[test]
fn add_item_applies_the_whole_draft_and_masks_secrets() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    assert_eq!(detail.summary.title.as_deref(), Some("GitHub"));
    assert_eq!(detail.summary.subtitle.as_deref(), Some("alex"));
    assert_eq!(detail.summary.host.as_deref(), Some("github.com"));
    assert!(detail.summary.has_otp);
    let mut tags = detail.summary.tags.clone();
    tags.sort();
    assert_eq!(tags, vec!["dev", "work"]);
    assert_eq!(detail.secrets.len(), 1);
    assert_eq!(detail.urls.len(), 1);
    assert_eq!(detail.custom_fields.len(), 3);
    let pin = detail
        .custom_fields
        .iter()
        .find(|f| f.label == "PIN")
        .unwrap();
    assert_eq!(pin.value, None, "hidden values are never in a detail");
    let otp = detail.otp.as_ref().unwrap();
    assert_eq!((otp.digits, otp.period), (6, Some(30)));
    let json = serde_json::to_string(&detail).unwrap();
    for secret in [ITEM_PASSWORD, HIDDEN_VALUE, OTP_SECRET] {
        assert!(!json.contains(secret));
    }
}

#[test]
fn update_item_edits_by_element_and_keeps_history() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    let pin = element_of(&detail, "PIN");
    let recovery = element_of(&detail, "Recovery");
    let url = detail.urls[0].element_id.clone();
    let updated = h
        .backend
        .update_item(req(with_item(
            &detail,
            json!({ "draft": {
                "text": [{ "field": "title", "value": "GitHub (work)" }],
                "secrets": [{ "field": "password", "value": "N3w-Pa55word" }],
                "urls": [
                    { "op": "update", "elementId": url, "url": "https://github.com", "matchMode": "etld1", "allowHttp": false },
                    { "op": "add", "url": "https://gist.github.com", "matchMode": "exactOrigin", "allowHttp": false }
                ],
                "customFields": [
                    { "op": "update", "elementId": pin, "label": "PIN code", "kind": "hidden", "value": "1111" },
                    { "op": "remove", "elementId": recovery }
                ],
                "tags": ["work"],
                "otp": { "op": "remove" }
            }}),
        )))
        .unwrap();
    assert_eq!(updated.summary.title.as_deref(), Some("GitHub (work)"));
    assert_eq!(
        updated.history_count, 1,
        "the old password moved to history"
    );
    assert_eq!(updated.urls.len(), 2);
    assert_eq!(updated.summary.tags, vec!["work"]);
    assert!(updated.otp.is_none());
    assert_eq!(updated.custom_fields.len(), 2);
    let history = h
        .backend
        .reveal_secret(&req(with_item(
            &updated,
            json!({ "secret": { "kind": "history", "index": 0 } }),
        )))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&history).unwrap()["value"],
        json!(ITEM_PASSWORD)
    );
    // A draft with nothing in it changes nothing.
    let same = h
        .backend
        .update_item(req(with_item(
            &updated,
            json!({ "draft": { "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null } }),
        )))
        .unwrap();
    assert_eq!(same, updated);
}

#[test]
fn malformed_drafts_are_invalid_input() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let detail = add_login(&h.backend, &vault);
    let empty = json!({ "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null });
    let unsupported = h
        .backend
        .add_item(req(
            json!({ "vaultId": vault, "kind": "unsupported", "draft": empty }),
        ))
        .unwrap_err();
    assert_eq!(code(unsupported), ErrorCode::InvalidInput);
    let bad_drafts = [
        json!({ "text": [], "secrets": [], "urls": [{ "op": "remove", "elementId": "bad\nid" }], "customFields": [], "tags": null, "otp": null }),
        json!({ "text": [], "secrets": [], "urls": [{ "op": "add", "url": "  ", "matchMode": "exactOrigin", "allowHttp": false }], "customFields": [], "tags": null, "otp": null }),
        json!({ "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": { "op": "set", "uri": "otpauth://totp/x?secret=" } }),
        json!({ "text": [], "secrets": [], "urls": [], "customFields": [{ "op": "add", "label": "x", "kind": "otp", "value": "not a uri" }], "tags": null, "otp": null }),
        json!({ "text": [], "secrets": [], "urls": [], "customFields": [{ "op": "add", "label": "bad\u{7}", "kind": "text", "value": "v" }], "tags": null, "otp": null }),
        json!({ "text": [], "secrets": [], "urls": [], "customFields": [], "tags": [""], "otp": null }),
        json!({ "text": [], "secrets": [], "urls": [{ "op": "update", "elementId": "999-0000", "url": "https://x", "matchMode": "exactOrigin", "allowHttp": false }], "customFields": [], "tags": null, "otp": null }),
    ];
    for draft in bad_drafts {
        let error = h
            .backend
            .update_item(req(with_item(&detail, json!({ "draft": draft.clone() }))))
            .unwrap_err();
        assert_eq!(code(error), ErrorCode::InvalidInput, "{draft}");
    }
    let unknown = h
        .backend
        .update_item(req(json!({
            "vaultId": vault,
            "itemId": "019a2b3c-4d5e-7000-8000-0000000000ff",
            "draft": empty
        })))
        .unwrap_err();
    assert_eq!(code(unknown), ErrorCode::ItemNotFound);
}

#[test]
fn list_get_reveal_favorite_trash_restore_purge_and_move() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let vault = vault_id(b);
    let detail = add_login(b, &vault);
    let all = |trash: &str| -> Vec<ItemSummary> {
        b.list_items(&req(
            json!({ "vaultId": null, "trash": trash, "favoritesOnly": false, "query": null }),
        ))
        .unwrap()
    };
    assert_eq!(all("exclude").len(), 1);
    let found = b
        .list_items(&req(
            json!({ "vaultId": vault, "trash": "exclude", "favoritesOnly": false, "query": "git" }),
        ))
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(b.get_item(req(item_req(&detail))).unwrap(), detail);

    let reveal =
        |secret: Value| b.reveal_secret(&req(with_item(&detail, json!({ "secret": secret }))));
    let password = reveal(json!({ "kind": "field", "field": "password" })).unwrap();
    let password = serde_json::to_value(&password).unwrap();
    assert_eq!(
        password,
        json!({ "value": ITEM_PASSWORD, "hideAfterSeconds": 60 })
    );
    let pin =
        reveal(json!({ "kind": "customField", "elementId": element_of(&detail, "PIN") })).unwrap();
    assert_eq!(
        serde_json::to_value(&pin).unwrap()["value"],
        json!(HIDDEN_VALUE)
    );
    assert_eq!(
        code(reveal(json!({ "kind": "field", "field": "cardPin" })).unwrap_err()),
        ErrorCode::FieldEmpty
    );
    assert_eq!(
        code(reveal(json!({ "kind": "history", "index": 5 })).unwrap_err()),
        ErrorCode::FieldEmpty
    );

    let starred = b
        .set_favorite(req(with_item(&detail, json!({ "favorite": true }))))
        .unwrap();
    assert!(starred.favorite);
    let favorites = b
        .list_items(&req(
            json!({ "vaultId": null, "trash": "exclude", "favoritesOnly": true, "query": null }),
        ))
        .unwrap();
    assert_eq!(favorites.len(), 1);

    b.edit_item(req(item_req(&detail)), UnlockedSession::trash_item)
        .unwrap();
    assert_eq!(all("exclude").len(), 0);
    assert_eq!(all("only").len(), 1);
    assert_eq!(all("include").len(), 1);
    b.edit_item(req(item_req(&detail)), UnlockedSession::restore_item)
        .unwrap();
    assert_eq!(all("exclude").len(), 1);

    let other = b.create_vault(&req(json!({ "name": "Work" }))).unwrap();
    let moved = b
        .move_item(req(with_item(
            &detail,
            json!({ "targetVaultId": other.vault_id.0.to_string() }),
        )))
        .unwrap();
    assert_eq!(moved.vault_id, other.vault_id);
    assert_ne!(
        moved.item_id, detail.summary.item_id,
        "a move makes a new ID"
    );
    assert_eq!(
        code(b.get_item(req(item_req(&detail))).unwrap_err()),
        ErrorCode::ItemDeleted
    );
    let moved_req =
        json!({ "vaultId": moved.vault_id.0.to_string(), "itemId": moved.item_id.0.to_string() });
    let same_vault = b
        .move_item(req(json!({ "vaultId": moved.vault_id.0.to_string(), "itemId": moved.item_id.0.to_string(), "targetVaultId": moved.vault_id.0.to_string() })))
        .unwrap_err();
    assert_eq!(code(same_vault), ErrorCode::SameVault);
    b.edit_item(req(moved_req.clone()), UnlockedSession::purge_item)
        .unwrap();
    assert_eq!(
        code(b.get_item(req(moved_req)).unwrap_err()),
        ErrorCode::ItemDeleted
    );
    assert_eq!(all("include").len(), 0);

    let renamed = b
        .rename_vault(&req(
            json!({ "vaultId": other.vault_id.0.to_string(), "name": "Job" }),
        ))
        .unwrap();
    assert_eq!(renamed.name.as_deref(), Some("Job"));
    assert_eq!(
        code(b.create_vault(&req(json!({ "name": " " }))).unwrap_err()),
        ErrorCode::InvalidInput
    );
}

#[test]
fn item_commands_need_an_unlocked_vault() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let detail = add_login(&h.backend, &vault);
    h.backend.lock(LockReason::Manual).unwrap();
    let b = &h.backend;
    let draft = json!({ "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null });
    let results: Vec<IpcError> = vec![
        b.list_vaults().unwrap_err(),
        b.list_items(&req(json!({ "vaultId": null, "trash": "exclude", "favoritesOnly": false, "query": null }))).unwrap_err(),
        b.get_item(req(item_req(&detail))).unwrap_err(),
        b.add_item(req(json!({ "vaultId": vault, "kind": "login", "draft": draft.clone() }))).unwrap_err(),
        b.update_item(req(with_item(&detail, json!({ "draft": draft })))).unwrap_err(),
        b.totp_code(req(item_req(&detail))).unwrap_err(),
        b.copy_to_clipboard(req(json!({ "source": { "kind": "item", "vaultId": vault, "itemId": detail.summary.item_id.0.to_string(), "field": { "kind": "password" } } }))).unwrap_err(),
        b.security_report(NOW).unwrap_err(),
        b.import_choose_file(req(json!({ "format": "enpass" }))).unwrap_err(),
    ];
    for error in results {
        assert_eq!(code(error), ErrorCode::Locked);
    }
}

// ---------------------------------------------------------------------------
// Clipboard and codes
// ---------------------------------------------------------------------------

fn copy(
    b: &Backend,
    detail: &ItemDetail,
    field: Value,
) -> IpcResult<crate::contract::clipboard::CopyReceipt> {
    b.copy_to_clipboard(req(json!({ "source": {
        "kind": "item",
        "vaultId": detail.summary.vault_id.0.to_string(),
        "itemId": detail.summary.item_id.0.to_string(),
        "field": field
    }})))
}

#[test]
fn copy_writes_the_clipboard_from_rust_and_reports_the_clear() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    let receipt = copy(&h.backend, &detail, json!({ "kind": "password" })).unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some(ITEM_PASSWORD));
    assert_eq!(receipt.clear_after_seconds, Some(30));
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let clears_at = receipt.clears_at.unwrap();
    assert!((now + 25_000..=now + 31_000).contains(&clears_at));

    copy(&h.backend, &detail, json!({ "kind": "username" })).unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some("alex"));
    copy(
        &h.backend,
        &detail,
        json!({ "kind": "text", "field": "title" }),
    )
    .unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some("GitHub"));
    copy(
        &h.backend,
        &detail,
        json!({ "kind": "customField", "elementId": element_of(&detail, "PIN") }),
    )
    .unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some(HIDDEN_VALUE));
    copy(
        &h.backend,
        &detail,
        json!({ "kind": "customField", "elementId": element_of(&detail, "Recovery") }),
    )
    .unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some("r@example.org"));
    copy(
        &h.backend,
        &detail,
        json!({ "kind": "customField", "elementId": element_of(&detail, "Backup code") }),
    )
    .unwrap();
    assert_eq!(h.clipboard.content().map(|c| c.len()), Some(6));
    copy(&h.backend, &detail, json!({ "kind": "totp" })).unwrap();
    let code_now = h.clipboard.content().unwrap();
    assert!(code_now.len() == 6 && code_now.chars().all(|c| c.is_ascii_digit()));

    for (field, expected) in [
        (
            json!({ "kind": "secret", "field": "cardNumber" }),
            ErrorCode::FieldEmpty,
        ),
        (
            json!({ "kind": "text", "field": "email" }),
            ErrorCode::FieldEmpty,
        ),
        (
            json!({ "kind": "customField", "elementId": "1-nope" }),
            ErrorCode::FieldEmpty,
        ),
    ] {
        assert_eq!(
            code(copy(&h.backend, &detail, field).unwrap_err()),
            expected
        );
    }

    // clear_clipboard clears our value and tells the webview.
    h.backend.clear_clipboard().unwrap();
    assert_eq!(h.clipboard.content(), None);
    assert!(events(&h.events).contains(&json!({ "type": "clipboardCleared" })));
    // A value the user copied since is left alone.
    copy(&h.backend, &detail, json!({ "kind": "password" })).unwrap();
    h.clipboard.put("user text");
    h.backend.clear_clipboard().unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some("user text"));
}

#[test]
fn locking_clears_what_mosaic_copied() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    copy(&h.backend, &detail, json!({ "kind": "password" })).unwrap();
    h.backend.lock(LockReason::Idle).unwrap();
    assert_eq!(h.clipboard.content(), None);
    let log = events(&h.events);
    assert!(log.contains(&json!({ "type": "clipboardCleared" })));
    assert_eq!(
        log.last(),
        Some(&json!({ "type": "locked", "reason": "idle" }))
    );
}

#[test]
fn generated_passwords_copy_from_rust_while_locked_and_drop_on_lock() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let generated = |request: Value| {
        let g = b.generate(req(request)).unwrap();
        serde_json::to_value(&g).unwrap()["value"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    b.lock(LockReason::Manual).unwrap();
    let missing = b
        .copy_to_clipboard(req(json!({ "source": { "kind": "generated" } })))
        .unwrap_err();
    assert_eq!(
        code(missing),
        ErrorCode::FieldEmpty,
        "nothing generated yet"
    );
    let value = generated(
        json!({ "kind": "password", "length": 24, "lowercase": true, "uppercase": true, "digits": true, "symbols": false, "excludeAmbiguous": true }),
    );
    assert_eq!(value.chars().count(), 24);
    b.copy_to_clipboard(req(json!({ "source": { "kind": "generated" } })))
        .unwrap();
    assert_eq!(h.clipboard.content(), Some(value));
    let site = generated(json!({ "kind": "forSite", "host": "example.com" }));
    assert_ne!(site, "");
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
    b.lock(LockReason::Manual).unwrap();
    assert!(
        b.last_generated.lock().unwrap().is_none(),
        "dropped on lock"
    );
}

#[test]
fn auto_clear_follows_the_setting() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    let off = Settings {
        clipboard_clear_seconds: None,
        ..Settings::default()
    };
    h.backend
        .update_settings(UpdateSettingsRequest {
            settings: off,
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap();
    let detail = h.backend.get_item(req(item_req(&detail))).unwrap();
    let receipt = copy(&h.backend, &detail, json!({ "kind": "password" })).unwrap();
    assert_eq!(
        (receipt.clears_at, receipt.clear_after_seconds),
        (None, None)
    );
}

#[test]
fn totp_codes_for_time_and_counter_based_items() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let vault = vault_id(b);
    let totp = add_login(b, &vault);
    let code_json = serde_json::to_value(b.totp_code(req(item_req(&totp))).unwrap()).unwrap();
    assert_eq!(code_json["period"], json!(30));
    let remaining = code_json["secondsRemaining"].as_u64().unwrap();
    assert!((1..=30).contains(&remaining));
    assert_eq!(code_json["code"].as_str().unwrap().len(), 6);

    let hotp = b
        .add_item(req(json!({ "vaultId": vault, "kind": "totp", "draft": {
            "text": [{ "field": "title", "value": "Counter" }], "secrets": [], "urls": [], "customFields": [], "tags": null,
            "otp": { "op": "set", "uri": format!("otpauth://hotp/Example:alex?secret={OTP_SECRET}&counter=0") }
        }})))
        .unwrap();
    let first = serde_json::to_value(b.totp_code(req(item_req(&hotp))).unwrap()).unwrap();
    let second = serde_json::to_value(b.totp_code(req(item_req(&hotp))).unwrap()).unwrap();
    // RFC 4226 appendix D: counter 0 and 1.
    assert_eq!(
        first,
        json!({ "code": "755224", "secondsRemaining": null, "period": null })
    );
    assert_eq!(second["code"], json!("287082"));
    copy(b, &hotp, json!({ "kind": "totp" })).unwrap();
    assert_eq!(
        h.clipboard.content().as_deref(),
        Some("359152"),
        "copy advances too"
    );

    let note = b
        .add_item(req(json!({ "vaultId": vault, "kind": "secureNote", "draft": {
            "text": [{ "field": "notes", "value": "plain" }], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null
        }})))
        .unwrap();
    assert_eq!(
        code(b.totp_code(req(item_req(&note))).unwrap_err()),
        ErrorCode::NoOtp
    );
    assert_eq!(
        code(copy(b, &note, json!({ "kind": "totp" })).unwrap_err()),
        ErrorCode::NoOtp
    );
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[test]
fn settings_persist_clamp_and_need_reauth_to_weaken() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let (backend, _clipboard, log) = backend_with(
        Store::open_in_memory().unwrap(),
        SharedKeystore::default(),
        Persistence::in_dir(dir.path()),
    );
    let b = &backend;
    assert_eq!(b.settings_snapshot(), Settings::default());
    // Harmless changes work while locked, without a password.
    let dark = b
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                theme: Theme::Dark,
                auto_lock_minutes: Some(100_000),
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
    assert_eq!(
        (dark.theme, dark.auto_lock_minutes),
        (Theme::Dark, Some(480))
    );
    assert_eq!(
        Persistence::in_dir(dir.path()).load().settings,
        dark,
        "saved"
    );

    let off = Settings {
        auto_lock_minutes: None,
        ..dark.clone()
    };
    let needs = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: None,
        })
        .unwrap_err();
    assert_eq!(code(needs), ErrorCode::ReauthRequired);
    let empty = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: Some(SecretIn::test("")),
        })
        .unwrap_err();
    assert_eq!(code(empty), ErrorCode::ReauthRequired);
    let locked = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err();
    assert_eq!(
        code(locked),
        ErrorCode::Locked,
        "weakening needs an unlocked vault"
    );

    create(b);
    let wrong = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: Some(SecretIn::test("wrong")),
        })
        .unwrap_err();
    assert_eq!(code(wrong), ErrorCode::WrongCredentials);
    assert!(
        matches!(b.session_state().unwrap(), SessionState::Locked { .. }),
        "a wrong re-auth locks"
    );
    assert!(events(&log).contains(&json!({ "type": "locked", "reason": "manual" })));
    assert_eq!(b.settings_snapshot(), dark, "unchanged");

    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
    let stored = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap();
    assert_eq!(stored, off);
    assert!(
        matches!(b.session_state().unwrap(), SessionState::Unlocked { .. }),
        "still unlocked"
    );
    // Turning it back on needs nothing.
    b.lock(LockReason::Manual).unwrap();
    b.update_settings(UpdateSettingsRequest {
        settings: dark.clone(),
        password: None,
    })
    .unwrap();
    assert_eq!(Persistence::in_dir(dir.path()).load().settings, dark);
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

#[test]
fn a_restart_remembers_the_account_and_finds_the_key() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let keystore = SharedKeystore::default();
    let store_path = dir.path().join("vault.sqlite3");
    let (first, _c, _l) = backend_with(
        Store::open(&store_path).unwrap(),
        keystore.clone(),
        Persistence::in_dir(dir.path()),
    );
    let secret_key = create(&first);
    drop(first);

    let (second, _c, _l) = backend_with(
        Store::open(&store_path).unwrap(),
        keystore,
        Persistence::in_dir(dir.path()),
    );
    assert_eq!(
        second.session_state().unwrap(),
        SessionState::Locked {
            email: Some("alex@example.com".into()),
            secret_key_stored: true,
            quick_unlock: vec![],
        }
    );
    let last_group = json!({ "lastGroup": secret_key.rsplit('-').next().unwrap() });
    assert_eq!(
        code(
            second
                .verify_emergency_kit(&req(last_group.clone()))
                .unwrap_err()
        ),
        ErrorCode::Locked,
        "no oracle on the key while locked"
    );
    second
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: None,
        })
        .unwrap();
    assert!(
        second
            .verify_emergency_kit(&req(last_group))
            .unwrap()
            .matches
    );

    // Without the keystore entry the key is typed, then remembered.
    drop(second);
    let (third, _c, _l) = backend_with(
        Store::open(&store_path).unwrap(),
        SharedKeystore::default(),
        Persistence::in_dir(dir.path()),
    );
    assert!(matches!(
        third.session_state().unwrap(),
        SessionState::Locked {
            secret_key_stored: false,
            ..
        }
    ));
    assert_eq!(
        code(
            third
                .verify_emergency_kit(&req(json!({ "lastGroup": "XXXX" })))
                .unwrap_err()
        ),
        ErrorCode::Locked
    );
    third
        .unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&secret_key)),
        })
        .unwrap();
    third.lock(LockReason::Manual).unwrap();
    assert!(matches!(
        third.session_state().unwrap(),
        SessionState::Locked {
            secret_key_stored: true,
            ..
        }
    ));
}

#[test]
fn quick_unlock_is_unavailable_until_the_vault_supports_it() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let request = req(json!({ "method": "biometric" }));
    assert_eq!(
        code(h.backend.unlock_quick(&request).unwrap_err()),
        ErrorCode::AlreadyUnlocked
    );
    h.backend.lock(LockReason::Manual).unwrap();
    for method in ["biometric", "hardwareKey"] {
        let error = h
            .backend
            .unlock_quick(&req(json!({ "method": method })))
            .unwrap_err();
        assert_eq!(code(error), ErrorCode::QuickUnlockUnavailable);
    }
    let unavailable = Backend::new(
        Slot::Unavailable(ErrorCode::StoreInUse),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    assert_eq!(
        code(unavailable.unlock_quick(&request).unwrap_err()),
        ErrorCode::StoreInUse
    );
    assert_eq!(
        code(unavailable.session_state().unwrap_err()),
        ErrorCode::StoreInUse
    );
    assert_eq!(
        code(unavailable.lock(LockReason::Manual).unwrap_err()),
        ErrorCode::StoreInUse
    );
}

fn kit_request(password: Option<&str>) -> SaveKitRequest {
    SaveKitRequest {
        labels: crate::emergency_kit::tests::labels(),
        password: password.map(SecretIn::test),
    }
}

fn kit_dir(path: &std::path::Path) -> FixedDialog {
    FixedDialog(Some(path.to_path_buf()), Arc::new(Mutex::new(Vec::new())))
}

#[test]
fn the_kit_saves_without_a_password_during_onboarding() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kit.pdf");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let b = h.backend.with_dialogs(Box::new(FixedDialog(
        Some(path.clone()),
        Arc::clone(&asked),
    )));
    assert_eq!(
        b.save_emergency_kit(kit_request(None)).unwrap(),
        KitSaved::Saved
    );
    let pdf = std::fs::read(&path).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(
        pdf.windows(h.secret_key.len())
            .any(|w| w == h.secret_key.as_bytes())
    );
    assert!(pdf.windows(16).any(|w| w == b"alex@example.com"));
    assert_eq!(*asked.lock().unwrap(), vec!["PDF"]);

    let mut bad = kit_request(None);
    bad.labels.title = String::new();
    assert_eq!(
        code(b.save_emergency_kit(bad).unwrap_err()),
        ErrorCode::InvalidInput
    );
}

#[test]
fn after_verification_the_kit_needs_the_password() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kit.pdf");
    let key = h.secret_key.clone();
    let events_log = h.events.clone();
    let b = h.backend.with_dialogs(Box::new(kit_dir(&path)));
    let verified = b
        .verify_emergency_kit(&req(
            json!({ "lastGroup": key.rsplit('-').next().unwrap() }),
        ))
        .unwrap();
    assert!(verified.matches);

    assert_eq!(
        code(b.save_emergency_kit(kit_request(None)).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert_eq!(
        code(b.save_emergency_kit(kit_request(Some(""))).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert!(!path.exists(), "nothing is written without re-auth");
    assert_eq!(
        b.save_emergency_kit(kit_request(Some(PASSWORD))).unwrap(),
        KitSaved::Saved
    );
    assert!(path.exists());

    let error = b
        .save_emergency_kit(kit_request(Some("not the password")))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::WrongCredentials);
    assert!(!b.is_unlocked(), "a wrong password locks the vault");
    assert!(events(&events_log).contains(&json!({ "type": "locked", "reason": "manual" })));
}

#[test]
fn a_cancelled_kit_dialog_writes_nothing() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        h.backend.save_emergency_kit(kit_request(None)).unwrap(),
        KitSaved::Cancelled
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn saving_the_kit_while_locked_is_refused() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    h.backend.lock(LockReason::Manual).unwrap();
    assert_eq!(
        code(h.backend.save_emergency_kit(kit_request(None)).unwrap_err()),
        ErrorCode::Locked
    );
}

#[test]
fn the_kit_window_closes_on_lock() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    b.lock(LockReason::Manual).unwrap();
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
    assert_eq!(
        code(b.save_emergency_kit(kit_request(None)).unwrap_err()),
        ErrorCode::ReauthRequired
    );
}

#[test]
fn every_lock_reason_reaches_the_webview_and_the_hooks() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    h.backend
        .on_lock(Box::new(move |reason| sink.lock().unwrap().push(reason)));
    for reason in LockReason::ALL {
        h.backend
            .unlock(UnlockRequest {
                password: SecretIn::test(PASSWORD),
                secret_key: None,
            })
            .ok();
        h.backend.lock(*reason).unwrap();
    }
    assert_eq!(*seen.lock().unwrap(), LockReason::ALL.to_vec());
    let locked: Vec<Value> = events(&h.events)
        .into_iter()
        .filter(|e| e["type"] == json!("locked"))
        .collect();
    assert_eq!(locked.len(), LockReason::ALL.len());
    assert_eq!(
        locked[3],
        json!({ "type": "locked", "reason": "screenLock" })
    );
}

// ---------------------------------------------------------------------------
// Auto-lock
// ---------------------------------------------------------------------------

fn screen_lock_only() -> Settings {
    Settings {
        auto_lock_minutes: None,
        lock_on_sleep: false,
        lock_on_screen_lock: true,
        ..Settings::default()
    }
}

#[test]
fn screen_lock_only_is_refused_without_detection() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    assert!(!b.app_status().screen_lock_detection);
    let before = b.settings_snapshot();
    let err = b
        .update_settings(UpdateSettingsRequest {
            settings: screen_lock_only(),
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert_eq!(b.settings_snapshot(), before);
}

#[test]
fn tick_floor_applies_to_a_stored_screen_lock_only_config() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    b.update_stored(|s| s.settings = screen_lock_only())
        .unwrap();
    b.activity.backdate(Duration::from_mins(11));
    assert_eq!(
        tick_decision(b, false, Instant::now(), SystemTime::now()),
        Some(LockReason::Idle)
    );
}

#[test]
fn idle_sleep_and_screen_lock_decisions() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let now = Instant::now();
    let wall = SystemTime::now();
    assert_eq!(tick_decision(b, false, now, wall), None, "just used");
    assert_eq!(tick_decision(b, true, now, wall), Some(LockReason::Sleep));
    b.activity.backdate(Duration::from_mins(11));
    assert_eq!(
        tick_decision(b, false, Instant::now(), SystemTime::now()),
        Some(LockReason::Idle),
        "default 10 minutes"
    );

    let relaxed = Settings {
        auto_lock_minutes: None,
        lock_on_sleep: false,
        lock_on_screen_lock: true,
        ..Settings::default()
    };
    assert_eq!(
        b.update_settings(UpdateSettingsRequest {
            settings: relaxed,
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err()
        .code,
        ErrorCode::InvalidInput,
        "screen lock alone is no trigger without detection"
    );
    b.update_settings(UpdateSettingsRequest {
        settings: Settings {
            auto_lock_minutes: Some(60),
            lock_on_sleep: false,
            lock_on_screen_lock: false,
            ..Settings::default()
        },
        password: Some(SecretIn::test(PASSWORD)),
    })
    .unwrap();
    b.activity.backdate(Duration::from_mins(30));
    assert_eq!(
        tick_decision(b, true, Instant::now(), SystemTime::now()),
        None,
        "idle (60 min) not reached and sleep off"
    );
    b.on_os_event(OsEvent::Sleep);
    assert!(b.is_unlocked(), "the settings say no");

    b.update_settings(UpdateSettingsRequest {
        settings: Settings::default(),
        password: None,
    })
    .unwrap();
    b.on_os_event(OsEvent::ScreenLock);
    assert!(!b.is_unlocked());
    assert_eq!(
        tick_decision(b, false, Instant::now(), SystemTime::now()),
        None,
        "already locked"
    );
    assert!(events(&h.events).contains(&json!({ "type": "locked", "reason": "screenLock" })));
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
    b.on_os_event(OsEvent::Sleep);
    assert!(!b.is_unlocked());
}

#[test]
fn a_sleep_locks_even_while_a_command_holds_the_session() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let _g = b.slot().unwrap();
    assert_eq!(
        tick_decision(b, true, Instant::now(), SystemTime::now()),
        Some(LockReason::Sleep)
    );
}

#[test]
fn update_settings_refuses_turning_every_lock_trigger_off() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let before = b.settings_snapshot();
    let all_off = Settings {
        auto_lock_minutes: None,
        lock_on_sleep: false,
        lock_on_screen_lock: false,
        ..Settings::default()
    };
    let error = b
        .update_settings(UpdateSettingsRequest {
            settings: all_off,
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::InvalidInput);
    assert_eq!(b.settings_snapshot(), before);
    assert!(b.is_unlocked());
}

#[test]
fn the_monitor_locks_an_idle_vault() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let one_minute = Settings {
        auto_lock_minutes: Some(1),
        ..Settings::default()
    };
    h.backend
        .update_settings(UpdateSettingsRequest {
            settings: one_minute,
            password: None,
        })
        .unwrap();
    h.backend.activity.backdate(Duration::from_secs(90));
    let state = AppState::new(h.backend);
    let mut monitor = Monitor::spawn(state.downgrade(), Duration::from_millis(10));
    let start = Instant::now();
    while state.backend().is_unlocked() {
        assert!(start.elapsed() < Duration::from_secs(10), "not locked");
        std::thread::sleep(Duration::from_millis(5));
    }
    monitor.stop();
    assert!(events(&h.events).contains(&json!({ "type": "locked", "reason": "idle" })));
    // The thread also ends on its own once the backend is gone.
    let monitor = Monitor::spawn(state.downgrade(), Duration::from_millis(5));
    drop(state);
    drop(monitor);
}

// ---------------------------------------------------------------------------
// Import and the SSH agent, through the real crates
// ---------------------------------------------------------------------------

const ENPASS: &[u8] = include_bytes!("../../../../crates/import/tests/fixtures/enpass.json");
const KDBX: &[u8] = include_bytes!("../../../../crates/import/tests/fixtures/aes-argon2d.kdbx");
const ED25519: &str = include_str!("../../../../crates/ssh-agent/tests/keys/ed25519");
const ED25519_PUB: &str = include_str!("../../../../crates/ssh-agent/tests/keys/ed25519.pub");

fn chosen(b: &Backend, format: &str) -> String {
    match b
        .import_choose_file(req(json!({ "format": format })))
        .unwrap()
    {
        ImportFileChoice::Chosen { file_token, .. } => file_token,
        ImportFileChoice::Cancelled => panic!("the dialog was cancelled"),
    }
}

fn preview_of(b: &Backend, token: &str, format: &str, password: Value) -> IpcResult<ImportPreview> {
    b.import_preview(req(
        json!({ "fileToken": token, "format": format, "password": password }),
    ))
}

#[test]
fn import_enpass_fixture_preview_commit_and_lock() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("enpass.json");
    std::fs::write(&file, ENPASS).unwrap();
    let b = h
        .backend
        .with_dialogs(Box::new(FixedDialog(Some(file), Arc::default())));
    let vault = vault_id(&b);
    // The same site and user name as the fixture's first entry.
    b.add_item(req(json!({
        "vaultId": vault,
        "kind": "login",
        "draft": {
            "text": [
                { "field": "title", "value": "Mail" },
                { "field": "username", "value": "alice@example.com" }
            ],
            "secrets": [],
            "urls": [{ "op": "add", "url": "https://mail.example.com/inbox", "matchMode": "exactOrigin", "allowHttp": false }],
            "customFields": [],
            "tags": null,
            "otp": null
        }
    })))
    .unwrap();

    let token = chosen(&b, "enpass");
    let preview = preview_of(&b, &token, "enpass", Value::Null).unwrap();
    assert_eq!(preview.format, ImportFormat::Enpass);
    let flagged: Vec<u32> = preview
        .entries
        .iter()
        .filter(|e| e.duplicate_of.is_some())
        .map(|e| e.index)
        .collect();
    assert_eq!(flagged, [0], "{preview:?}");
    assert_ne!(preview.skipped.len(), 0);

    let result = b
        .import_commit(&req(json!({
            "previewId": preview.preview_id,
            "targetVaultId": vault,
            "skip": flagged,
        })))
        .unwrap();
    assert_eq!(result.skipped, 1);
    assert_eq!(result.imported, 8);
    assert_eq!(result.failed, 0);
    // The held import is gone after the commit.
    let again = b
        .import_commit(&req(json!({
            "previewId": preview.preview_id,
            "targetVaultId": vault,
            "skip": [],
        })))
        .unwrap_err();
    assert_eq!(code(again), ErrorCode::ImportPreviewExpired);

    // A new preview, then a lock: the preview and the token are gone.
    let second = preview_of(&b, &token, "enpass", Value::Null).unwrap();
    b.lock(LockReason::Manual).unwrap();
    let error = b
        .import_commit(&req(json!({
            "previewId": second.preview_id,
            "targetVaultId": vault,
            "skip": [],
        })))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
    assert_eq!(
        code(preview_of(&b, &token, "enpass", Value::Null).unwrap_err()),
        ErrorCode::ImportFileUnreadable
    );
}

#[test]
fn a_new_preview_replaces_the_held_one() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("enpass.json");
    std::fs::write(&file, ENPASS).unwrap();
    let b = h
        .backend
        .with_dialogs(Box::new(FixedDialog(Some(file), Arc::default())));
    let vault = vault_id(&b);
    let token = chosen(&b, "enpass");
    let first = preview_of(&b, &token, "enpass", Value::Null).unwrap();
    let second = preview_of(&b, &token, "enpass", Value::Null).unwrap();
    assert_ne!(first.preview_id, second.preview_id);
    let old = b
        .import_commit(&req(
            json!({ "previewId": first.preview_id, "targetVaultId": vault, "skip": [] }),
        ))
        .unwrap_err();
    assert_eq!(code(old), ErrorCode::ImportPreviewExpired);
    b.import_cancel(&req(json!({ "previewId": second.preview_id })))
        .unwrap();
    let cancelled = b
        .import_commit(&req(
            json!({ "previewId": second.preview_id, "targetVaultId": vault, "skip": [] }),
        ))
        .unwrap_err();
    assert_eq!(code(cancelled), ErrorCode::ImportPreviewExpired);
}

#[test]
fn import_errors_map_to_codes() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("export");
    let b = h
        .backend
        .with_dialogs(Box::new(FixedDialog(Some(file.clone()), Arc::default())));

    std::fs::write(&file, b"this is not an export").unwrap();
    let token = chosen(&b, "enpass");
    assert_eq!(
        code(preview_of(&b, &token, "enpass", Value::Null).unwrap_err()),
        ErrorCode::ImportFormatInvalid
    );

    std::fs::write(&file, KDBX).unwrap();
    let token = chosen(&b, "keepass");
    assert_eq!(
        code(preview_of(&b, &token, "keepass", json!("not the password")).unwrap_err()),
        ErrorCode::WrongCredentials
    );
    assert_eq!(
        code(preview_of(&b, &token, "keepass", Value::Null).unwrap_err()),
        ErrorCode::WrongCredentials,
        "a missing password is not told apart"
    );
    assert!(preview_of(&b, &token, "keepass", json!("fixture-password")).is_ok());

    // One byte over the limit is refused from the file's metadata alone.
    let handle = std::fs::File::create(&file).unwrap();
    handle
        .set_len(mosaic_import::MAX_FILE_LEN as u64 + 1)
        .unwrap();
    drop(handle);
    let token = chosen(&b, "csv");
    assert_eq!(
        code(preview_of(&b, &token, "csv", Value::Null).unwrap_err()),
        ErrorCode::ImportTooLarge
    );
}

fn add_ssh_key(b: &Backend) -> (mosaic_core::VaultId, mosaic_core::ItemId) {
    use mosaic_core::model::{ItemType, SshKeyMaterial, Text, TextField};
    let vault = b.list_vaults().unwrap()[0].vault_id.get();
    let mut slot = b.slot().unwrap();
    let session = slot.unlocked_mut().unwrap();
    let item = session
        .add_item(vault, &ItemType::SshKey, |e| {
            e.set_text(TextField::Title, "Deploy key")?;
            e.set_ssh_key(&SshKeyMaterial::new(
                SecretString::from_str_copy(ED25519).unwrap(),
                Text::new(ED25519_PUB.trim()),
                Text::new("SHA256:ignored"),
            ))
        })
        .unwrap();
    (vault, item)
}

#[test]
fn ssh_agent_lifecycle() {
    let _kdf = crate::test_support::kdf_permit();
    let endpoint = crate::test_support::ssh_test_endpoint();
    let h = unlocked();
    let b = h.backend.with_adapters(Adapters {
        import: crate::adapters::ImportService::default(),
        ssh: crate::adapters::SshAgentService::new(Some(endpoint.endpoint.clone())),
    });
    let (vault, item) = add_ssh_key(&b);

    assert!(!b.ssh_agent_status().running);
    let on = b
        .ssh_agent_set_enabled(req(json!({ "enabled": true })))
        .unwrap();
    assert!(on.enabled && on.running, "{on:?}");
    assert_eq!(on.key_count, 1);
    assert!(on.endpoint.is_some() && on.error.is_none());

    b.edit_item(
        req(json!({ "vaultId": vault.to_string(), "itemId": item.to_string() })),
        UnlockedSession::trash_item,
    )
    .unwrap();
    assert_eq!(
        b.ssh_agent_status().key_count,
        0,
        "a trashed key is not served"
    );
    b.edit_item(
        req(json!({ "vaultId": vault.to_string(), "itemId": item.to_string() })),
        UnlockedSession::restore_item,
    )
    .unwrap();
    assert_eq!(b.ssh_agent_status().key_count, 1);

    b.lock(LockReason::Manual).unwrap();
    let locked = b.ssh_agent_status();
    assert!(locked.enabled && !locked.running, "stops with the lock");
    assert_eq!(
        code(
            b.ssh_agent_set_enabled(req(json!({ "enabled": true })))
                .unwrap_err()
        ),
        ErrorCode::Locked
    );

    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
    assert!(
        b.ssh_agent_status().running,
        "the setting persists across the lock"
    );

    let off = b
        .ssh_agent_set_enabled(req(json!({ "enabled": false })))
        .unwrap();
    assert!(!off.enabled && !off.running);
    assert_eq!(
        code(
            b.ssh_respond(&req(json!({ "requestId": "x", "decision": "deny" })))
                .unwrap_err()
        ),
        ErrorCode::SshRequestExpired
    );
    drop(endpoint);
}

#[test]
fn the_shortcut_and_screen_lock_report_through_app_status() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let status = h.backend.app_status();
    assert_eq!(
        status.shortcut,
        crate::contract::session::ShortcutState::Unavailable
    );
    assert!(!status.screen_lock_detection);
}

// ---------------------------------------------------------------------------
// No secrets in errors, events, responses or debug output
// ---------------------------------------------------------------------------

#[test]
fn drafts_with_too_many_entries_are_invalid_input() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let changes: Vec<Value> = (0..=crate::commands::draft::MAX_DRAFT_ENTRIES)
        .map(|_| json!({ "field": "notes", "value": "x" }))
        .collect();
    assert_eq!(changes.len(), 513);
    let error = h
        .backend
        .add_item(req(json!({
            "vaultId": vault,
            "kind": "login",
            "draft": { "text": changes, "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null }
        })))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::InvalidInput);
}

/// Locks the backend while its open-file dialog is up.
struct LockingDialog(Arc<std::sync::OnceLock<std::sync::Weak<Backend>>>);

impl Dialogs for LockingDialog {
    fn save_file(&self, _: &str, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }
    fn open_file(&self, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        if let Some(backend) = self.0.get().and_then(std::sync::Weak::upgrade) {
            backend.lock(LockReason::Manual).unwrap();
        }
        Ok(Some(PathBuf::from("export.csv")))
    }
}

#[test]
fn a_file_chosen_after_the_vault_locked_gets_no_token() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let cell = Arc::new(std::sync::OnceLock::new());
    let b = Arc::new(
        h.backend
            .with_dialogs(Box::new(LockingDialog(Arc::clone(&cell)))),
    );
    cell.set(Arc::downgrade(&b)).ok();
    let error = b
        .import_choose_file(req(json!({ "format": "csv" })))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
    assert!(b.import_files.lock().unwrap().get("file-1").is_none());
}

#[test]
#[allow(clippy::too_many_lines)]
fn no_secret_leaks_into_errors_events_or_debug_output() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let vault = vault_id(b);
    let detail = add_login(b, &vault);
    let secrets = [
        PASSWORD,
        ITEM_PASSWORD,
        HIDDEN_VALUE,
        OTP_SECRET,
        h.secret_key.as_str(),
    ];
    let mut texts: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut record_error = |error: IpcError| {
        errors.push(format!(
            "{error:?} {error} {}",
            serde_json::to_string(&error).unwrap()
        ));
    };

    // Failing calls that carried a secret in the request.
    record_error(
        b.unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: None,
        })
        .unwrap_err(),
    );
    record_error(b.reauthenticate(SecretIn::test(ITEM_PASSWORD)).unwrap_err());
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test(&h.secret_key)),
    })
    .unwrap();
    record_error(
        b.update_item(req(with_item(&detail, json!({ "draft": {
            "text": [], "secrets": [{ "field": "password", "value": ITEM_PASSWORD }], "urls": [],
            "customFields": [{ "op": "add", "label": "x", "kind": "otp", "value": format!("otpauth://totp/x?secret={OTP_SECRET}&digits=99") }],
            "tags": null, "otp": null
        }}))))
        .unwrap_err(),
    );
    b.lock(LockReason::Manual).unwrap();
    record_error(
        b.unlock(UnlockRequest {
            password: SecretIn::test(ITEM_PASSWORD),
            secret_key: Some(SecretIn::test(&h.secret_key)),
        })
        .unwrap_err(),
    );
    let mut mistyped = h.secret_key.clone();
    let last = mistyped.pop().unwrap();
    mistyped.push(if last == '0' { '1' } else { '0' });
    record_error(
        b.unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: Some(SecretIn::test(&mistyped)),
        })
        .unwrap_err(),
    );
    record_error(
        b.update_settings(UpdateSettingsRequest {
            settings: Settings {
                auto_lock_minutes: None,
                ..Settings::default()
            },
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err(),
    );

    // Debug output of requests and responses that hold secrets.
    let unlock = UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test(&h.secret_key)),
    };
    texts.push(format!("{unlock:?}"));
    b.unlock(unlock).unwrap();
    let revealed = b
        .reveal_secret(&req(with_item(
            &detail,
            json!({ "secret": { "kind": "field", "field": "password" } }),
        )))
        .unwrap();
    texts.push(format!("{revealed:?}"));
    texts.push(format!(
        "{:?}",
        b.totp_code(req(item_req(&detail))).unwrap()
    ));
    texts.push(format!("{:?}", b.get_item(req(item_req(&detail))).unwrap()));
    texts.push(serde_json::to_string(&b.get_item(req(item_req(&detail))).unwrap()).unwrap());
    texts.push(serde_json::to_string(&b.list_items(&req(json!({ "vaultId": null, "trash": "include", "favoritesOnly": false, "query": null }))).unwrap()).unwrap());
    let draft: crate::contract::items::AddItemRequest =
        req(json!({ "vaultId": vault, "kind": "login", "draft": login_draft() }));
    texts.push(format!("{draft:?}"));
    let session_debug = format!("{:?}", b.slot().unwrap().unlocked().unwrap());
    texts.push(session_debug);
    copy(b, &detail, json!({ "kind": "password" })).unwrap();
    // Re-authentication for the kit, the empty clipboard field, and the
    // lock-trigger floor.
    record_error(
        b.update_settings(UpdateSettingsRequest {
            settings: Settings {
                auto_lock_minutes: None,
                lock_on_sleep: false,
                lock_on_screen_lock: false,
                ..Settings::default()
            },
            password: Some(SecretIn::test(PASSWORD)),
        })
        .unwrap_err(),
    );
    record_error(copy(b, &detail, json!({ "kind": "text", "field": "email" })).unwrap_err());
    texts.push(format!("{:?}", kit_request(Some(PASSWORD))));
    if let Ok(mut pending) = b.kit_pending.lock() {
        *pending = None;
    }
    record_error(
        b.save_emergency_kit(kit_request(Some(ITEM_PASSWORD)))
            .unwrap_err(),
    );
    b.lock(LockReason::Manual).unwrap();
    texts.extend(h.events.lock().unwrap().iter().cloned());
    texts.extend(errors);

    for text in &texts {
        for secret in secrets {
            assert!(!text.contains(secret), "secret in: {text}");
        }
    }
}

#[test]
fn the_crate_has_no_logging_or_printing() {
    // Secrets are never logged because nothing is: no logger, no prints in
    // non-test code (the workspace lints deny print and dbg as well).
    let sources = [
        include_str!("adapters/mod.rs"),
        include_str!("adapters/import.rs"),
        include_str!("adapters/ssh.rs"),
        include_str!("flow_tests.rs"),
        include_str!("autolock.rs"),
        include_str!("clipboard.rs"),
        include_str!("dialogs.rs"),
        include_str!("emergency_kit.rs"),
        include_str!("events.rs"),
        include_str!("keystore.rs"),
        include_str!("lib.rs"),
        include_str!("persist.rs"),
        include_str!("quick_unlock.rs"),
        include_str!("shortcut.rs"),
        include_str!("state.rs"),
        include_str!("test_support.rs"),
        include_str!("commands/clipboard.rs"),
        include_str!("commands/draft.rs"),
        include_str!("commands/generator.rs"),
        include_str!("commands/import.rs"),
        include_str!("commands/insights.rs"),
        include_str!("commands/items.rs"),
        include_str!("commands/session.rs"),
        include_str!("commands/settings.rs"),
        include_str!("commands/ssh.rs"),
        include_str!("commands/totp.rs"),
        include_str!("commands/vaults.rs"),
    ];
    for source in sources {
        for forbidden in [
            "println!",
            "eprintln!",
            "print!(",
            "dbg!",
            "log::info",
            "log::warn",
            "log::debug",
            "log::error",
            "log::trace",
            "tracing::",
            "info!(",
            "warn!(",
            "debug!(",
            "error!(",
        ] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
}
