//! Tests of every command handler (`Backend` methods) on an in-memory store,
//! with requests built from their wire JSON, an in-memory clipboard and a
//! recording event channel. The last tests check that no secret shows up in
//! an error, an event, a non-secret response or a debug print.
// Requests are built inline as JSON values and handed over by value.
#![expect(
    clippy::needless_pass_by_value,
    reason = "test helpers take their arguments by value"
)]

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

const NO_EDITS: &str =
    r#"{ "text": [], "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null }"#;

fn draft_with(extra: Value) -> Value {
    let mut draft: Value = serde_json::from_str(NO_EDITS).unwrap();
    if let (Some(map), Some(extra)) = (draft.as_object_mut(), extra.as_object()) {
        map.extend(extra.clone());
    }
    draft
}

fn add_ssh(b: &Backend, vault: &str) -> ItemDetail {
    let draft = draft_with(json!({
        "text": [{ "field": "title", "value": "Deploy key" }],
        "sshKey": { "op": "set", "privateKey": ED25519 }
    }));
    b.add_item(req(
        json!({ "vaultId": vault, "kind": "sshKey", "draft": draft }),
    ))
    .unwrap()
}

#[test]
fn update_item_relabels_hidden_field_without_reveal() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let detail = add_login(&h.backend, &vault_id(&h.backend));
    let pin = element_of(&detail, "PIN");
    let update = |label: &str, kind: &str| {
        h.backend.update_item(req(with_item(
            &detail,
            json!({ "draft": draft_with(json!({ "customFields": [
                { "op": "update", "elementId": pin, "label": label, "kind": kind, "value": null }
            ]})) }),
        )))
    };
    let renamed = update("PIN code", "hidden").unwrap();
    assert_eq!(element_of(&renamed, "PIN code"), pin);
    let revealed = h
        .backend
        .reveal_secret(&req(with_item(
            &renamed,
            json!({ "secret": { "kind": "customField", "elementId": pin } }),
        )))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&revealed).unwrap()["value"],
        json!(HIDDEN_VALUE),
        "the stored value survives a rename without a value"
    );
    // Another kind needs a value; an unknown element is refused too.
    assert_eq!(
        code(update("PIN", "text").unwrap_err()),
        ErrorCode::InvalidInput
    );
    let unknown = h.backend.update_item(req(with_item(
        &detail,
        json!({ "draft": draft_with(json!({ "customFields": [
            { "op": "update", "elementId": "999-0000", "label": "x", "kind": "hidden", "value": null }
        ]})) }),
    )));
    assert_eq!(code(unknown.unwrap_err()), ErrorCode::InvalidInput);
}

#[test]
fn ssh_key_on_login_is_invalid_input() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let draft = draft_with(json!({ "sshKey": { "op": "set", "privateKey": ED25519 } }));
    let added = h.backend.add_item(req(
        json!({ "vaultId": vault, "kind": "login", "draft": draft.clone() }),
    ));
    assert_eq!(code(added.unwrap_err()), ErrorCode::InvalidInput);
    let login = add_login(&h.backend, &vault);
    let updated = h
        .backend
        .update_item(req(with_item(&login, json!({ "draft": draft }))));
    assert_eq!(code(updated.unwrap_err()), ErrorCode::InvalidInput);
    // Not a key: refused with its own code, on a key item as well.
    let garbage = draft_with(json!({ "sshKey": { "op": "set", "privateKey": "hunter2" } }));
    let added = h.backend.add_item(req(
        json!({ "vaultId": vault, "kind": "sshKey", "draft": garbage }),
    ));
    assert_eq!(code(added.unwrap_err()), ErrorCode::SshKeyInvalid);
}

#[test]
fn detail_reports_ssh_summary() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let detail = add_ssh(&h.backend, &vault);
    let summary = detail.ssh_key.as_ref().unwrap();
    assert_eq!(summary.public_key.as_deref(), Some(ED25519_PUB.trim()));
    assert_eq!(
        summary.fingerprint.as_deref(),
        Some("SHA256:QjbS4EZK032epnEDVJGO7K3jwMFOWJMwAflXxbJSUUM")
    );
    assert_eq!(summary.algorithm.as_deref(), Some("ssh-ed25519"));
    assert!(!summary.encrypted && summary.agent_supported);
    // The private key never reaches a detail, a list or a reveal.
    let body = ED25519.lines().nth(1).unwrap();
    let json = serde_json::to_string(&detail).unwrap();
    assert!(!json.contains(body) && !json.contains("PRIVATE KEY"));
    // A login has no summary.
    let login = add_login(&h.backend, &vault);
    assert!(login.ssh_key.is_none());
}

#[test]
fn copy_ssh_public_key() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let vault = vault_id(&h.backend);
    let detail = add_ssh(&h.backend, &vault);
    copy(&h.backend, &detail, json!({ "kind": "sshPublicKey" })).unwrap();
    assert_eq!(h.clipboard.content().as_deref(), Some(ED25519_PUB.trim()));
    let login = add_login(&h.backend, &vault);
    let error = copy(&h.backend, &login, json!({ "kind": "sshPublicKey" })).unwrap_err();
    assert_eq!(code(error), ErrorCode::FieldEmpty);
}

fn list_items_in(b: &Backend, trash: &str) -> Vec<ItemSummary> {
    b.list_items(&req(
        json!({ "vaultId": null, "trash": trash, "favoritesOnly": false, "query": null }),
    ))
    .unwrap()
}

fn check_list_get_and_reveal(b: &Backend, vault: &str, detail: &ItemDetail) {
    assert_eq!(list_items_in(b, "exclude").len(), 1);
    let found = b
        .list_items(&req(
            json!({ "vaultId": vault, "trash": "exclude", "favoritesOnly": false, "query": "git" }),
        ))
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(b.get_item(req(item_req(detail))).unwrap(), *detail);

    let reveal =
        |secret: Value| b.reveal_secret(&req(with_item(detail, json!({ "secret": secret }))));
    let password = reveal(json!({ "kind": "field", "field": "password" })).unwrap();
    let password = serde_json::to_value(&password).unwrap();
    assert_eq!(
        password,
        json!({ "value": ITEM_PASSWORD, "hideAfterSeconds": 60 })
    );
    let pin =
        reveal(json!({ "kind": "customField", "elementId": element_of(detail, "PIN") })).unwrap();
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
}

fn check_favorite_trash_and_restore(b: &Backend, detail: &ItemDetail) {
    let starred = b
        .set_favorite(req(with_item(detail, json!({ "favorite": true }))))
        .unwrap();
    assert!(starred.favorite);
    let favorites = b
        .list_items(&req(
            json!({ "vaultId": null, "trash": "exclude", "favoritesOnly": true, "query": null }),
        ))
        .unwrap();
    assert_eq!(favorites.len(), 1);

    b.edit_item(req(item_req(detail)), UnlockedSession::trash_item)
        .unwrap();
    assert_eq!(list_items_in(b, "exclude").len(), 0);
    assert_eq!(list_items_in(b, "only").len(), 1);
    assert_eq!(list_items_in(b, "include").len(), 1);
    b.edit_item(req(item_req(detail)), UnlockedSession::restore_item)
        .unwrap();
    assert_eq!(list_items_in(b, "exclude").len(), 1);
}

fn check_move_purge_and_rename(b: &Backend, detail: &ItemDetail) {
    let other = b.create_vault(&req(json!({ "name": "Work" }))).unwrap();
    let moved = b
        .move_item(req(with_item(
            detail,
            json!({ "targetVaultId": other.vault_id.0.to_string() }),
        )))
        .unwrap();
    assert_eq!(moved.vault_id, other.vault_id);
    assert_ne!(
        moved.item_id, detail.summary.item_id,
        "a move makes a new ID"
    );
    assert_eq!(
        code(b.get_item(req(item_req(detail))).unwrap_err()),
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
    assert_eq!(list_items_in(b, "include").len(), 0);

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
fn list_get_reveal_favorite_trash_restore_purge_and_move() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let b = &h.backend;
    let vault = vault_id(b);
    let detail = add_login(b, &vault);
    check_list_get_and_reveal(b, &vault, &detail);
    check_favorite_trash_and_restore(b, &detail);
    check_move_purge_and_rename(b, &detail);
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
#[expect(clippy::too_many_lines, reason = "one scenario, step by step")]
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
    // Theme and language change while locked, without a password; a
    // protection setting needs the unlocked vault (it is written with a MAC).
    let themed = b
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                theme: Theme::Dark,
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
    assert_eq!(themed.theme, Theme::Dark);
    assert_eq!(
        code(
            b.update_settings(UpdateSettingsRequest {
                settings: Settings {
                    auto_lock_minutes: Some(100_000),
                    ..themed.clone()
                },
                password: None,
            })
            .unwrap_err()
        ),
        ErrorCode::Locked
    );
    assert_eq!(
        Persistence::in_dir(dir.path()).load().settings.theme,
        Theme::Dark,
        "saved"
    );
    let dark = Settings {
        theme: Theme::Dark,
        auto_lock_minutes: Some(480),
        ..Settings::default()
    };
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

    // Harmless protection changes work unlocked, clamped, without a password.
    create(b);
    let stored_dark = b
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                auto_lock_minutes: Some(100_000),
                ..themed
            },
            password: None,
        })
        .unwrap();
    assert_eq!(
        (stored_dark.theme, stored_dark.auto_lock_minutes),
        (Theme::Dark, Some(480))
    );
    let (_, file) = Persistence::in_dir(dir.path()).load_checked();
    assert_eq!(
        file.unwrap().state.settings,
        stored_dark,
        "saved, with a MAC"
    );

    let wrong = b
        .update_settings(UpdateSettingsRequest {
            settings: off.clone(),
            password: Some(SecretIn::test("wrong")),
        })
        .unwrap_err();
    assert_eq!(code(wrong), ErrorCode::WrongCredentials);
    assert!(
        matches!(b.session_state().unwrap(), SessionState::Unlocked { .. }),
        "a wrong re-auth does not lock (M3-R8)"
    );
    assert!(!events(&log).contains(&json!({ "type": "locked", "reason": "manual" })));
    assert_eq!(b.settings_snapshot(), dark, "unchanged");

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
    b.update_settings(UpdateSettingsRequest {
        settings: dark.clone(),
        password: None,
    })
    .unwrap();
    let (_, file) = Persistence::in_dir(dir.path()).load_checked();
    assert_eq!(file.unwrap().state.settings, dark);
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
    let last_group =
        json!({ "lastGroup": secret_key.rsplit('-').next().unwrap(), "pending": false });
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
                .verify_emergency_kit(&req(json!({ "lastGroup": "XXXX", "pending": false })))
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
fn quick_unlock_is_unavailable_without_a_platform() {
    let _kdf = crate::test_support::kdf_permit();
    let h = unlocked();
    let mut request = req(json!({ "method": "biometric" }));
    assert_eq!(
        code(h.backend.unlock_quick(&mut request).unwrap_err()),
        ErrorCode::AlreadyUnlocked
    );
    h.backend.lock(LockReason::Manual).unwrap();
    // The default backend has neither a biometric platform nor security keys.
    for (method, wanted) in [
        ("biometric", ErrorCode::QuickUnlockUnavailable),
        ("hardwareKey", ErrorCode::HardwareKeyUnsupported),
    ] {
        let error = h
            .backend
            .unlock_quick(&mut req(json!({ "method": method })))
            .unwrap_err();
        assert_eq!(code(error), wanted);
    }
    let unavailable = Backend::new(
        Slot::Unavailable(ErrorCode::StoreInUse),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    assert_eq!(
        code(unavailable.unlock_quick(&mut request).unwrap_err()),
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
        pending: false,
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
            json!({ "lastGroup": key.rsplit('-').next().unwrap(), "pending": false }),
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
    assert!(
        b.is_unlocked(),
        "a wrong password no longer locks the vault"
    );
    assert!(!events(&events_log).contains(&json!({ "type": "locked", "reason": "manual" })));
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
#[expect(clippy::too_many_lines, reason = "one scenario, step by step")]
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
    record_error(
        b.reauthenticate(
            SecretIn::test(ITEM_PASSWORD),
            crate::contract::session::ReauthPurpose::QuickUnlockEnroll,
        )
        .unwrap_err(),
    );
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
        include_str!("attachment_files.rs"),
        include_str!("flow_tests.rs"),
        include_str!("quick_unlock_tests.rs"),
        include_str!("autolock.rs"),
        include_str!("clipboard.rs"),
        include_str!("dialogs.rs"),
        include_str!("emergency_kit.rs"),
        include_str!("events.rs"),
        include_str!("keystore.rs"),
        include_str!("lib.rs"),
        include_str!("persist.rs"),
        include_str!("quick_unlock/fakes.rs"),
        include_str!("quick_unlock/hardware.rs"),
        include_str!("quick_unlock/mod.rs"),
        include_str!("quick_unlock/params.rs"),
        include_str!("quick_unlock/touch_id.rs"),
        include_str!("quick_unlock/windows_hello.rs"),
        include_str!("shortcut.rs"),
        include_str!("state.rs"),
        include_str!("test_support.rs"),
        include_str!("commands/attachments.rs"),
        include_str!("commands/clipboard.rs"),
        include_str!("commands/draft.rs"),
        include_str!("commands/generator.rs"),
        include_str!("commands/import.rs"),
        include_str!("commands/insights.rs"),
        include_str!("commands/items.rs"),
        include_str!("commands/quick_unlock.rs"),
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

// ---------------------------------------------------------------------------
// Attachments (a file-backed store: an in-memory one has no attachments)
// ---------------------------------------------------------------------------

type Hook = Box<dyn Fn() + Send>;

/// Native dialogs for tests: the paths to answer with, the names the save
/// dialog was asked to suggest, and a hook that runs "while the dialog is
/// open" (a lock, for example).
#[derive(Clone, Default)]
struct Picks {
    open: Arc<Mutex<Option<PathBuf>>>,
    save: Arc<Mutex<Option<PathBuf>>>,
    suggested: Arc<Mutex<Vec<String>>>,
    filters: Arc<Mutex<Vec<(&'static str, usize)>>>,
    during: Arc<Mutex<Option<Hook>>>,
}

impl Picks {
    fn open(&self, path: Option<&std::path::Path>) {
        *self.open.lock().unwrap() = path.map(std::path::Path::to_path_buf);
    }
    fn save(&self, path: Option<&std::path::Path>) {
        *self.save.lock().unwrap() = path.map(std::path::Path::to_path_buf);
    }
    fn run_hook(&self) {
        if let Some(hook) = self.during.lock().unwrap().as_ref() {
            hook();
        }
    }
}

impl Dialogs for Picks {
    fn save_file(&self, suggested: &str, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.suggested.lock().unwrap().push(suggested.to_owned());
        self.filters
            .lock()
            .unwrap()
            .push((filter.name, filter.extensions.len()));
        self.run_hook();
        Ok(self.save.lock().unwrap().clone())
    }
    fn open_file(&self, filter: FileFilter) -> IpcResult<Option<PathBuf>> {
        self.filters
            .lock()
            .unwrap()
            .push((filter.name, filter.extensions.len()));
        self.run_hook();
        Ok(self.open.lock().unwrap().clone())
    }
}

struct DiskRig {
    backend: Arc<Backend>,
    picks: Picks,
    dir: tempfile::TempDir,
    vault: String,
}

fn disk_rig() -> DiskRig {
    let dir = tempfile::tempdir().unwrap();
    let picks = Picks::default();
    let (backend, _clipboard, _events) = backend_with(
        Store::open(dir.path().join("vault.sqlite3")).unwrap(),
        SharedKeystore::default(),
        Persistence::memory(),
    );
    let backend = Arc::new(backend.with_dialogs(Box::new(picks.clone())));
    create(&backend);
    let vault = vault_id(&backend);
    DiskRig {
        backend,
        picks,
        dir,
        vault,
    }
}

fn add_document(b: &Backend, vault: &str) -> ItemDetail {
    b.add_item(req(json!({
        "vaultId": vault,
        "kind": "document",
        "draft": {
            "text": [{ "field": "title", "value": "Passport scan" }],
            "secrets": [], "urls": [], "customFields": [], "tags": null, "otp": null
        }
    })))
    .unwrap()
}

fn write_png(dir: &std::path::Path, name: &str, width: u32, height: u32) -> (PathBuf, Vec<u8>) {
    use mosaic_qr::{OwnedFrame, PixelLayout};
    let pixels: Vec<u8> = (0..width * height * 4)
        .map(|i| u8::try_from(i % 251).unwrap())
        .collect();
    let frame = OwnedFrame::new(
        width,
        height,
        PixelLayout::Rgba8,
        zeroize::Zeroizing::new(pixels.clone()),
    )
    .unwrap();
    let path = dir.join(name);
    std::fs::write(&path, mosaic_qr::testing::png(&frame)).unwrap();
    (path, pixels)
}

fn attachment_added(r: &DiskRig, item: &ItemDetail) -> ItemDetail {
    match r.backend.attachment_add(req(item_req(item))).unwrap() {
        crate::contract::attachments::AttachmentAdded::Added { item } => *item,
        crate::contract::attachments::AttachmentAdded::Cancelled => panic!("cancelled"),
    }
}

fn att_req(item: &ItemDetail, index: usize) -> Value {
    with_item(
        item,
        json!({ "attachmentId": item.attachments[index].attachment_id.0.to_string() }),
    )
}

fn ciphertext_path(b: &Backend, item: &ItemDetail, index: usize) -> PathBuf {
    let slot = b.slot().unwrap();
    let dir = slot
        .unlocked()
        .unwrap()
        .store()
        .attachment_dir()
        .unwrap()
        .to_path_buf();
    let name = item.attachments[index]
        .attachment_id
        .0
        .as_bytes()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").unwrap();
            hex
        });
    dir.join(name)
}

fn unlock_again(b: &Backend) {
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
}

fn lock_during_dialog(r: &DiskRig) {
    let weak = Arc::downgrade(&r.backend);
    *r.picks.during.lock().unwrap() = Some(Box::new(move || {
        if let Some(backend) = weak.upgrade() {
            backend.lock(LockReason::Manual).unwrap();
        }
    }));
}

#[test]
fn attachment_add_stores_and_lists() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let (path, _) = write_png(r.dir.path(), "scan.png", 6, 4);
    r.picks.open(Some(&path));
    let size = std::fs::metadata(&path).unwrap().len();
    let added = attachment_added(&r, &item);
    assert_eq!(added.attachments.len(), 1);
    let entry = &added.attachments[0];
    assert_eq!(entry.file_name, "scan.png");
    assert_eq!(entry.mime, "image/png");
    assert_eq!(entry.size, size);
    assert!(entry.available && entry.previewable);
    assert!(entry.added_at.is_some_and(|ms| ms > 1_500_000_000_000));
    let again = r.backend.get_item(req(item_req(&item))).unwrap();
    assert_eq!(again.attachments, added.attachments);
    // The path stays in Rust: nothing of it is in the response.
    let json = serde_json::to_string(&added).unwrap();
    assert!(!json.contains("scan.png\\"));
    assert!(!json.contains(r.dir.path().file_name().unwrap().to_str().unwrap()));
    assert_eq!(
        r.picks.filters.lock().unwrap().as_slice(),
        &[("All files", 0)]
    );
}

#[test]
fn attachment_names_from_sync_are_cleaned_for_display() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    {
        // An import or a sync can deliver a name this device never cleaned.
        let mut slot = r.backend.slot().unwrap();
        slot.unlocked_mut()
            .unwrap()
            .add_attachment(
                item.summary.vault_id.get(),
                item.summary.item_id.get(),
                "invoice\u{202E}fdp.exe\u{0}",
                "application/octet-stream",
                b"data",
            )
            .unwrap();
    }
    let shown = r.backend.get_item(req(item_req(&item))).unwrap();
    assert_eq!(shown.attachments[0].file_name, "invoicefdp.exe");
}

#[test]
fn attachment_add_cancelled() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    r.picks.open(None);
    assert_eq!(
        r.backend.attachment_add(req(item_req(&item))).unwrap(),
        crate::contract::attachments::AttachmentAdded::Cancelled
    );
    let again = r.backend.get_item(req(item_req(&item))).unwrap();
    assert_eq!(again.attachments, Vec::new());
}

#[test]
fn attachment_add_too_large() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let path = r.dir.path().join("huge.bin");
    let len = u64::try_from(mosaic_core::MAX_ATTACHMENT_LEN).unwrap() + 1;
    std::fs::File::create(&path).unwrap().set_len(len).unwrap();
    r.picks.open(Some(&path));
    let error = r.backend.attachment_add(req(item_req(&item))).unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentTooLarge);
    r.picks.open(Some(r.dir.path()));
    let error = r.backend.attachment_add(req(item_req(&item))).unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentFileUnreadable, "a folder");
    let again = r.backend.get_item(req(item_req(&item))).unwrap();
    assert_eq!(again.attachments, Vec::new());
}

#[test]
fn attachment_add_lock_during_dialog_stores_nothing() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let (path, _) = write_png(r.dir.path(), "scan.png", 3, 3);
    r.picks.open(Some(&path));
    lock_during_dialog(&r);
    let error = r.backend.attachment_add(req(item_req(&item))).unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
    *r.picks.during.lock().unwrap() = None;
    unlock_again(&r.backend);
    let again = r.backend.get_item(req(item_req(&item))).unwrap();
    assert_eq!(again.attachments, Vec::new());
    let slot = r.backend.slot().unwrap();
    let store = slot.unlocked().unwrap().store();
    let record = item.summary.item_id.get();
    let vault = item.summary.vault_id.get();
    assert_eq!(
        store.attachments_for_record(&vault, &record).unwrap(),
        Vec::new()
    );
}

#[test]
fn attachment_add_hardware_vault_refused() {
    use crate::quick_unlock::QuickUnlockService;
    let _kdf = crate::test_support::kdf_permit();
    let key = crate::quick_unlock::fakes::FakeHardwareKey::new(1);
    let dir = tempfile::tempdir().unwrap();
    let picks = Picks::default();
    let (path, _) = write_png(dir.path(), "scan.png", 2, 2);
    picks.open(Some(&path));
    let (backend, _c, _l) = backend_with(
        Store::open(dir.path().join("vault.sqlite3")).unwrap(),
        SharedKeystore::default(),
        Persistence::memory(),
    );
    let backend = backend
        .with_quick_unlock(QuickUnlockService::new(None, Box::new(Arc::clone(&key))))
        .with_dialogs(Box::new(picks.clone()));
    create(&backend);
    let vault = backend
        .create_hardware_vault(&mut req(json!({ "name": "Codes", "pin": null })))
        .unwrap()
        .vault_id
        .0
        .to_string();
    let error = backend
        .attachment_add(req(json!({
            "vaultId": vault, "itemId": "019a2b3c-0000-7000-8000-000000000002"
        })))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ItemTypeNotAllowed);
    assert!(
        picks.filters.lock().unwrap().is_empty(),
        "refused before any dialog opened"
    );
}

#[test]
fn attachment_preview_png_roundtrip() {
    use std::sync::atomic::Ordering;
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let (path, pixels) = write_png(r.dir.path(), "scan.png", 6, 4);
    r.picks.open(Some(&path));
    let item = attachment_added(&r, &item);
    let preview = r
        .backend
        .attachment_preview(&req(att_req(&item, 0)))
        .unwrap();
    assert_eq!(preview.get(..8), Some(&[6, 0, 0, 0, 4, 0, 0, 0][..]));
    assert_eq!(preview.get(8..), Some(pixels.as_slice()));
    assert!(!r.backend.preview_busy.load(Ordering::SeqCst));
    // A file that is not an image has no preview, and the flag is released.
    let text = r.dir.path().join("notes.txt");
    std::fs::write(&text, b"just text").unwrap();
    r.picks.open(Some(&text));
    let item = attachment_added(&r, &item);
    let error = r
        .backend
        .attachment_preview(&req(att_req(&item, 1)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentPreviewUnsupported);
    assert!(!r.backend.preview_busy.load(Ordering::SeqCst));
    // Locked: nothing is returned.
    r.backend.lock(LockReason::Manual).unwrap();
    let error = r
        .backend
        .attachment_preview(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
}

#[test]
fn attachment_preview_busy() {
    use std::sync::atomic::Ordering;
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let (path, _) = write_png(r.dir.path(), "scan.png", 2, 2);
    r.picks.open(Some(&path));
    let item = attachment_added(&r, &item);
    r.backend.preview_busy.store(true, Ordering::SeqCst);
    let error = r
        .backend
        .attachment_preview(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Busy);
    assert!(
        r.backend.preview_busy.load(Ordering::SeqCst),
        "the running preview's flag is not cleared by the refused one"
    );
    r.backend.preview_busy.store(false, Ordering::SeqCst);
    assert!(
        r.backend
            .attachment_preview(&req(att_req(&item, 0)))
            .is_ok()
    );
}

#[test]
fn attachment_save_writes_plaintext() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let source = r.dir.path().join("notes.txt");
    std::fs::write(&source, b"recovery codes: 1234").unwrap();
    r.picks.open(Some(&source));
    let item = attachment_added(&r, &item);
    let out = r.dir.path().join("saved.txt");
    r.picks.save(Some(&out));
    let saved = r.backend.attachment_save(&req(att_req(&item, 0))).unwrap();
    assert_eq!(saved, crate::contract::attachments::AttachmentSaved::Saved);
    assert_eq!(std::fs::read(&out).unwrap(), b"recovery codes: 1234");
    assert_eq!(r.picks.suggested.lock().unwrap().as_slice(), ["notes.txt"]);
    // The vault locking during the save dialog writes nothing.
    let later = r.dir.path().join("later.txt");
    r.picks.save(Some(&later));
    lock_during_dialog(&r);
    let error = r
        .backend
        .attachment_save(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
    assert!(!later.exists());
    *r.picks.during.lock().unwrap() = None;
    // A target that cannot be written is a save error, not a panic.
    unlock_again(&r.backend);
    r.picks
        .save(Some(&r.dir.path().join("no/such/folder/f.txt")));
    let error = r
        .backend
        .attachment_save(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentSaveFailed);
}

#[test]
fn attachment_save_cancelled_decrypts_nothing() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let source = r.dir.path().join("notes.txt");
    std::fs::write(&source, b"data").unwrap();
    r.picks.open(Some(&source));
    let item = attachment_added(&r, &item);
    // Damage the stored ciphertext: only a decryption would notice.
    std::fs::write(ciphertext_path(&r.backend, &item, 0), b"x").unwrap();
    r.picks.save(None);
    assert_eq!(
        r.backend.attachment_save(&req(att_req(&item, 0))).unwrap(),
        crate::contract::attachments::AttachmentSaved::Cancelled
    );
    let out = r.dir.path().join("out.txt");
    r.picks.save(Some(&out));
    let error = r
        .backend
        .attachment_save(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentUnavailable);
    assert!(!out.exists());
    let error = r
        .backend
        .attachment_preview(&req(att_req(&item, 0)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentUnavailable);
}

#[test]
fn attachment_remove_then_not_found() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let source = r.dir.path().join("notes.txt");
    std::fs::write(&source, b"data").unwrap();
    r.picks.open(Some(&source));
    let item = attachment_added(&r, &item);
    let stored = ciphertext_path(&r.backend, &item, 0);
    assert!(stored.exists());
    let removed = r
        .backend
        .attachment_remove(&req(att_req(&item, 0)))
        .unwrap();
    assert_eq!(removed.attachments, Vec::new());
    assert!(!stored.exists(), "the encrypted file leaves the store");
    for error in [
        r.backend
            .attachment_remove(&req(att_req(&item, 0)))
            .unwrap_err(),
        r.backend
            .attachment_preview(&req(att_req(&item, 0)))
            .unwrap_err(),
        r.backend
            .attachment_save(&req(att_req(&item, 0)))
            .unwrap_err(),
    ] {
        assert_eq!(code(error), ErrorCode::AttachmentNotFound);
    }
}

#[test]
fn unknown_attachment_maps_to_attachment_not_found() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let item = add_document(&r.backend, &r.vault);
    let unknown = with_item(
        &item,
        json!({ "attachmentId": "019a2b3c-0000-7000-8000-0000000000aa" }),
    );
    let error = r.backend.attachment_remove(&req(unknown)).unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentNotFound);
    // And an unknown item stays an unknown item.
    let nothing = json!({
        "vaultId": r.vault,
        "itemId": "019a2b3c-0000-7000-8000-0000000000bb",
        "attachmentId": "019a2b3c-0000-7000-8000-0000000000aa",
    });
    let error = r.backend.attachment_preview(&req(nothing)).unwrap_err();
    assert_eq!(code(error), ErrorCode::ItemNotFound);
}

// ---------------------------------------------------------------------------
// Passkey export (ADR-0041)
// ---------------------------------------------------------------------------

/// Adds a passkey item of `algorithm` to the vault; returns its credential ID.
fn add_passkey(r: &DiskRig, rp_id: &str, algorithm: i64, trashed: bool) -> Vec<u8> {
    use mosaic_core::model::{ItemType, PasskeyCredential, Text};
    let key = mosaic_passkey::Es256Key::generate().unwrap();
    let id = mosaic_passkey::new_credential_id().unwrap();
    let mut credential = PasskeyCredential::new(
        id.clone(),
        Text::new(rp_id),
        vec![9; 16],
        key.to_pkcs8_der().unwrap(),
        algorithm,
        NOW,
    );
    credential.user_name = Text::new("alice");
    let vault = r.backend.list_vaults().unwrap()[0].vault_id.get();
    let mut slot = r.backend.slot().unwrap();
    let session = slot.unlocked_mut().unwrap();
    let item = session
        .add_item(vault, &ItemType::Passkey, |e| {
            e.set_title(rp_id)?;
            e.set_passkey(&credential)
        })
        .unwrap();
    if trashed {
        let trash = |e: &mut mosaic_core::ItemEdit<'_>| e.trash();
        session.update_item(vault, item, trash).unwrap();
    }
    id
}

fn export_request(vault: &str, password: Value) -> crate::contract::export::ExportPasskeysRequest {
    req(json!({ "vaultId": vault, "password": password }))
}

#[test]
fn passkey_export_needs_reauth_and_round_trips_through_cxf_import() {
    use crate::contract::export::PasskeysExported;
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    let id = add_passkey(&r, "example.com", -7, false);
    // Not exported: another algorithm, and a trashed one.
    add_passkey(&r, "ed.example", -8, false);
    add_passkey(&r, "gone.example", -7, true);
    let out = r.dir.path().join("passkeys.json");
    r.picks.save(Some(&out));

    // No password and no earlier verification: nothing happens, no dialog.
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ReauthRequired);
    assert!(r.picks.suggested.lock().unwrap().is_empty());
    assert!(!out.exists());
    // A wrong password.
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, json!("not the password")))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::WrongCredentials);
    assert!(!out.exists());

    // The right password: saved, with the date in the suggested name.
    let saved = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, json!(PASSWORD)))
        .unwrap();
    assert_eq!(saved, PasskeysExported::Saved { count: 1 });
    let suggested = r.picks.suggested.lock().unwrap().clone();
    assert!(suggested[0].starts_with("mosaic-passkeys-"));
    assert_eq!(
        std::path::Path::new(&suggested[0]).extension().unwrap(),
        "json"
    );
    assert_eq!(r.picks.filters.lock().unwrap()[0].0, "JSON");
    let bytes = std::fs::read(&out).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains(r#""exporterRpId":"mosaic.local""#));
    assert!(!text.contains("ed.example") && !text.contains("gone.example"));
    // Only the file itself is left in the folder (no temp copy).
    let names: Vec<_> = std::fs::read_dir(r.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().all(|n| !n.ends_with(".mosaic-partial")),
        "{names:?}"
    );

    // The re-authentication is spent: another export needs the password again.
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ReauthRequired);

    // The file imports as CXF: the same credential, as a passkey item.
    r.picks.open(Some(&out));
    let token = chosen(&r.backend, "cxf");
    let preview = preview_of(&r.backend, &token, "cxf", Value::Null).unwrap();
    assert_eq!(preview.counts.len(), 1);
    assert_eq!(preview.counts[0].count, 1);
    assert_eq!(preview.entries.len(), 1);
    assert!(
        preview.entries[0].duplicate_of.is_some(),
        "the passkey is already in the vault"
    );
    // The importer reads the same bytes: the credential ID matches.
    let import = mosaic_import::parse(mosaic_import::Format::Cxf, &bytes, None).unwrap();
    assert_eq!(
        import.entries[0].draft.passkeys[0].credential.credential_id,
        id
    );
    assert_eq!(
        import.entries[0].draft.passkeys[0]
            .credential
            .rp_id
            .as_str(),
        "example.com"
    );
}

#[test]
fn passkey_export_cancel_and_failures_hand_the_reauthentication_back() {
    use crate::contract::export::PasskeysExported;
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    add_passkey(&r, "example.com", -7, false);
    // A cancelled dialog writes nothing and keeps the grant.
    r.picks.save(None);
    let cancelled = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, json!(PASSWORD)))
        .unwrap();
    assert_eq!(cancelled, PasskeysExported::Cancelled);
    // A folder that does not exist: the write fails, the grant stays.
    r.picks
        .save(Some(&r.dir.path().join("no/such/folder/p.json")));
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::AttachmentSaveFailed);
    // The vault locks while the dialog is open: nothing is built or written.
    let later = r.dir.path().join("later.json");
    r.picks.save(Some(&later));
    lock_during_dialog(&r);
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
    assert!(!later.exists());
    *r.picks.during.lock().unwrap() = None;
    unlock_again(&r.backend);
    // After a fresh password it works.
    let saved = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, json!(PASSWORD)))
        .unwrap();
    assert_eq!(saved, PasskeysExported::Saved { count: 1 });
    assert!(later.exists());
}

#[test]
fn passkey_export_ids_are_random_per_export_and_not_the_vaults() {
    use crate::contract::export::PasskeysExported;
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    add_passkey(&r, "example.com", -7, false);
    add_passkey(&r, "example.org", -7, false);
    let ids_of = |name: &str| -> Vec<String> {
        let out = r.dir.path().join(name);
        r.picks.save(Some(&out));
        let saved = r
            .backend
            .export_passkeys_cxf(export_request(&r.vault, json!(PASSWORD)))
            .unwrap();
        assert_eq!(saved, PasskeysExported::Saved { count: 2 });
        let doc: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        let account = &doc["accounts"][0];
        let mut ids = vec![account["id"].as_str().unwrap().to_owned()];
        ids.extend(
            account["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i["id"].as_str().unwrap().to_owned()),
        );
        ids
    };
    let (first, second) = (ids_of("a.json"), ids_of("b.json"));
    assert_eq!(first.len(), 3);
    // Fresh IDs per export: no ID repeats, within a file or across files.
    let all: std::collections::HashSet<&String> = first.iter().chain(&second).collect();
    assert_eq!(all.len(), 6);
    // 16 random bytes in base64url, and neither the vault ID nor an item ID.
    let mut real = vec![mosaic_passkey::b64url(
        r.backend.list_vaults().unwrap()[0]
            .vault_id
            .get()
            .as_bytes(),
    )];
    {
        let slot = r.backend.slot().unwrap();
        let session = slot.unlocked().unwrap();
        real.extend(
            session
                .items(&mosaic_vault::session::ItemFilter::default())
                .iter()
                .map(|i| mosaic_passkey::b64url(i.item_id.as_bytes())),
        );
    }
    assert_eq!(real.len(), 3);
    for id in first.iter().chain(&second) {
        assert_eq!(
            mosaic_passkey::from_b64url(id, 64).unwrap().len(),
            16,
            "{id}"
        );
        assert!(!real.contains(id), "{id}");
    }
}

#[test]
fn a_passkey_dialog_password_does_not_pay_for_a_passkey_export() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    add_passkey(&r, "example.com", -7, false);
    let out = r.dir.path().join("passkeys.json");
    r.picks.save(Some(&out));
    // The passkey dialog verifies the master password without earning the
    // two-minute re-authentication grant.
    r.backend
        .verify_master_only(&mosaic_core::MasterPassword::new(PASSWORD.to_owned()))
        .unwrap();
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ReauthRequired);
    assert!(r.picks.suggested.lock().unwrap().is_empty());
    assert!(!out.exists());
}

#[test]
fn passkey_export_refuses_empty_and_unknown_vaults() {
    let _kdf = crate::test_support::kdf_permit();
    let r = disk_rig();
    // No passkey in the vault.
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, json!(PASSWORD)))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ItemNotFound);
    let error = r
        .backend
        .export_passkeys_cxf(export_request(
            "019a2b3c-0000-7000-8000-0000000000aa",
            json!(PASSWORD),
        ))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::VaultNotFound);
    assert!(r.picks.suggested.lock().unwrap().is_empty());
    // Nothing was verified, so a wrong request did not cost a password try.
    r.backend.lock(LockReason::Manual).unwrap();
    let error = r
        .backend
        .export_passkeys_cxf(export_request(&r.vault, Value::Null))
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::Locked);
}
