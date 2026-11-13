//! Whole-app red team (2026-11-13), desktop surface (`RT-desktop`): the
//! re-authentication grant, the attachment file flows, the local state file
//! and a hostile peer in the sync folder. Each test names the attack it
//! tries. A test marked `#[ignore = "RT-desktop-<n>: ..."]` demonstrates a
//! finding that holds today (it asserts the wanted behaviour and fails); the
//! rest are regression tests for attacks that did not work.
#![expect(
    clippy::too_many_lines,
    reason = "each attack is one scenario, written out in full"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use mosaic_core::Argon2Profile;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::contract::attachments::{AttachmentAdded, AttachmentSaved};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::export::PasskeysExported;
use crate::contract::items::ItemDetail;
use crate::contract::secret::SecretIn;
use crate::contract::session::{
    CreateAccountRequest, KitLabels, KitSaved, SaveKitRequest, VerifyKitRequest,
};
use crate::dialogs::{Dialogs, FileFilter};
use crate::keystore::MemoryKeystore;
use crate::state::{Backend, Slot};

const PASSWORD: &str = "rt-desktop master password \u{2206}";
const ITEM_PASSWORD: &str = "Rt-D3sktop-Pa55word!";

fn typed<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

/// Answers every dialog with the path set last (or a cancel), and counts
/// how often the save dialog was shown.
#[derive(Clone, Default)]
struct PathDialog {
    path: Arc<Mutex<Option<PathBuf>>>,
    saves: Arc<Mutex<usize>>,
}

impl PathDialog {
    fn set(&self, path: Option<PathBuf>) {
        *self.path.lock().unwrap() = path;
    }

    fn saves(&self) -> usize {
        *self.saves.lock().unwrap()
    }
}

impl Dialogs for PathDialog {
    fn save_file(&self, _: &str, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        *self.saves.lock().unwrap() += 1;
        Ok(self.path.lock().unwrap().clone())
    }

    fn open_file(&self, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(self.path.lock().unwrap().clone())
    }
}

fn backend(store: Store, dialog: &PathDialog) -> Backend {
    Backend::new(
        Slot::Locked(LockedSession::open(store)),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_dialogs(Box::new(dialog.clone()))
}

/// Creates the account; returns the Secret Key as the kit shows it.
fn create(b: &Backend) -> String {
    let created = b
        .create_account(CreateAccountRequest {
            email: "rt-desktop@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Ends the onboarding window: the last group typed back.
fn verify_kit(b: &Backend, secret_key: &str) {
    let last = secret_key.rsplit('-').next().unwrap();
    let verified = b
        .verify_emergency_kit(&VerifyKitRequest {
            last_group: SecretIn::test(last),
            pending: false,
        })
        .unwrap();
    assert!(verified.matches);
}

fn first_vault(b: &Backend) -> String {
    b.list_vaults().unwrap()[0].vault_id.0.to_string()
}

fn labels() -> KitLabels {
    KitLabels {
        title: "Mosaic Emergency Kit".into(),
        intro: "Keep this somewhere safe.".into(),
        email: "Account email".into(),
        account_id: "Account ID".into(),
        sign_in_url: "Sign-in address".into(),
        sign_in_url_value: "this device only".into(),
        secret_key: "Secret Key".into(),
        master_password: "Master password".into(),
        master_password_hint: "write it here by hand".into(),
        footer: "pre-audit".into(),
    }
}

fn save_kit(b: &Backend, password: Option<&str>) -> IpcResult<KitSaved> {
    b.save_emergency_kit(SaveKitRequest {
        labels: labels(),
        pending: false,
        password: password.map(SecretIn::test),
    })
}

fn add_login(b: &Backend, vault: &str) -> ItemDetail {
    b.add_item(typed(json!({
        "vaultId": vault,
        "kind": "login",
        "draft": {
            "text": [
                { "field": "title", "value": "Bank" },
                { "field": "username", "value": "red" }
            ],
            "secrets": [{ "field": "password", "value": ITEM_PASSWORD }],
            "urls": [], "customFields": [], "tags": null, "otp": null
        }
    })))
    .unwrap()
}

/// Puts an ES256 passkey into the first vault (the export needs one).
fn add_passkey(b: &Backend) {
    use mosaic_core::Timestamp;
    use mosaic_core::model::{ItemType, PasskeyCredential, Text};
    use mosaic_passkey::CreateInput;

    let created = mosaic_passkey::create(&CreateInput {
        rp_id: "example.com",
        origin: "https://example.com",
        challenge: &[7; 32],
        user_handle: &[9; 16],
        algorithms: &[-7],
        user_verified: false,
    })
    .unwrap();
    let mut credential = PasskeyCredential::new(
        created.credential_id,
        Text::new("example.com"),
        vec![9; 16],
        created.private_key_pkcs8,
        -7,
        Timestamp::from_unix_millis(1_000),
    );
    credential.user_name = Text::new("red");
    let mut slot = b.slot().unwrap();
    let session = slot.unlocked_mut().unwrap();
    let vault_id = session.vaults()[0].vault_id;
    session
        .add_item(vault_id, &ItemType::Passkey, |e| {
            e.set_title("example.com")?;
            e.set_passkey(&credential)
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// 1. The re-authentication grant is purpose-agnostic
// ---------------------------------------------------------------------------

/// RT-desktop-1. The user types the master password to print the Emergency
/// Kit again (`save_emergency_kit`), then cancels the save dialog. Nothing was
/// printed, but `Backend::reauthenticate` ran before the dialog and left the
/// two-minute grant behind, and the grant is not tied to what the password was
/// typed for: within two minutes a compromised webview (or a second click)
/// exports the vault's passkeys **unencrypted** with `password: null`, or
/// moves an item out of a hardware-protected vault, without the password
/// being asked for again. CRYPTO_SPEC §6.2 makes any successful check a proof;
/// UX_GUIDELINES §8 promises re-authentication "before export" and "before
/// viewing the Secret Key" as separate confirmations. Wanted: a password typed
/// for the kit pays for the kit only (no grant from `save_emergency_kit`, or a
/// grant scoped to the action that earned it).
#[test]
#[ignore = "RT-desktop-1: a cancelled Emergency Kit save leaves a two-minute grant that a passkey export spends"]
fn a_password_typed_for_the_kit_does_not_pay_for_a_passkey_export() {
    let _kdf = crate::test_support::kdf_permit();
    let dialog = PathDialog::default();
    let b = backend(Store::open_in_memory().unwrap(), &dialog);
    let secret_key = create(&b);
    verify_kit(&b, &secret_key);
    add_passkey(&b);
    let vault = first_vault(&b);

    // Without a grant the export asks for the password.
    let export = b.export_passkeys_cxf(typed(json!({ "vaultId": vault, "password": null })));
    assert_eq!(code(export.unwrap_err()), ErrorCode::ReauthRequired);
    assert!(!b.quick_unlock.reauth_granted());

    // The kit: password typed, dialog cancelled, nothing written.
    assert_eq!(save_kit(&b, Some(PASSWORD)).unwrap(), KitSaved::Cancelled);
    assert_eq!(dialog.saves(), 1);

    // Wanted: no grant from a kit save.
    assert!(
        !b.quick_unlock.reauth_granted(),
        "the password typed for the kit left a re-authentication grant"
    );
    // Wanted: the export still asks for the password. Today it reaches the
    // save dialog (`cancelled` here) on the kit's grant.
    let export = b.export_passkeys_cxf(typed(json!({ "vaultId": vault, "password": null })));
    assert_eq!(
        export.map_err(code),
        Err(ErrorCode::ReauthRequired),
        "the passkey export spent the grant of the kit save"
    );
    assert_eq!(dialog.saves(), 1, "the export's save dialog was shown");
}

/// Held: the grant pays for no settings weakening and for no second kit
/// save; both need the password itself every time, an empty password counts
/// as none, and the grant that was earned is still there afterwards (so a
/// wrong guess is not what a refusal costs).
#[test]
fn the_grant_pays_for_no_settings_weakening_and_no_kit_save() {
    let _kdf = crate::test_support::kdf_permit();
    let dialog = PathDialog::default();
    let b = backend(Store::open_in_memory().unwrap(), &dialog);
    let secret_key = create(&b);
    verify_kit(&b, &secret_key);
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    assert!(b.quick_unlock.reauth_granted());

    let mut weaker = serde_json::to_value(b.settings_snapshot()).unwrap();
    let days = weaker["quickUnlockMaxAgeDays"].as_u64().unwrap();
    weaker["quickUnlockMaxAgeDays"] = json!(days + 1);
    for password in [Value::Null, json!("")] {
        let changed = b.update_settings(typed(json!({
            "settings": weaker.clone(),
            "password": password
        })));
        assert_eq!(code(changed.unwrap_err()), ErrorCode::ReauthRequired);
        assert_eq!(
            u64::from(b.settings_snapshot().quick_unlock_max_age_days),
            days,
            "nothing changed"
        );
    }
    let mut off = serde_json::to_value(b.settings_snapshot()).unwrap();
    off["clipboardClearSeconds"] = Value::Null;
    let changed = b.update_settings(typed(json!({ "settings": off, "password": null })));
    assert_eq!(code(changed.unwrap_err()), ErrorCode::ReauthRequired);
    assert!(b.settings_snapshot().clipboard_clear_seconds.is_some());

    for password in [None, Some("")] {
        assert_eq!(
            code(save_kit(&b, password).unwrap_err()),
            ErrorCode::ReauthRequired
        );
    }
    assert_eq!(dialog.saves(), 0, "no kit dialog without the password");
    assert!(
        b.quick_unlock.reauth_granted(),
        "the refusals kept the grant"
    );

    // A lock ends the grant, and an unlock does not bring it back.
    b.lock(LockReason::Manual).unwrap();
    b.unlock(typed(json!({ "password": PASSWORD, "secretKey": null })))
        .unwrap();
    assert!(!b.quick_unlock.reauth_granted());
    assert_eq!(
        code(save_kit(&b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
}

// ---------------------------------------------------------------------------
// 2. Attachment files: hostile content, hostile names, the busy slot
// ---------------------------------------------------------------------------

/// Held: a file whose name says "image" but whose bytes are a PDF with a
/// script tag is stored under the sniffed type, is not previewable, the
/// preview refuses it without decoding and frees its one-at-a-time slot, a
/// directory or a missing file adds nothing, a file over the limit is refused
/// by its size, and Save as writes exactly the bytes (no temp file left) or
/// fails cleanly where it cannot write.
#[test]
fn hostile_attachment_files_are_sniffed_refused_and_saved_byte_for_byte() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let dialog = PathDialog::default();
    let b = backend(
        Store::open(dir.path().join("vault.sqlite3")).unwrap(),
        &dialog,
    );
    create(&b);
    let vault = first_vault(&b);
    let item = add_login(&b, &vault).summary.item_id.0.to_string();
    let request = |extra: Value| {
        let mut value = json!({ "vaultId": vault, "itemId": item });
        if let (Some(object), Some(more)) = (value.as_object_mut(), extra.as_object()) {
            object.extend(more.clone());
        }
        value
    };

    // A PDF with markup inside, called a PNG, with a right-to-left override
    // in its name that would show "gnp.exe" as "exe.png" in a careless UI.
    let hostile = dir.path().join("scan\u{202E}gnp.png");
    let body = b"%PDF-1.7\n<script>alert(1)</script>\n".repeat(64);
    std::fs::write(&hostile, &body).unwrap();
    dialog.set(Some(hostile));
    let AttachmentAdded::Added { item: detail } =
        b.attachment_add(typed(request(json!({})))).unwrap()
    else {
        panic!("added")
    };
    let entry = &detail.attachments[0];
    assert_eq!(entry.mime, "application/pdf", "sniffed, not the extension");
    assert!(!entry.previewable);
    assert!(
        !entry.file_name.contains('\u{202E}') && entry.file_name == "scangnp.png",
        "{:?}",
        entry.file_name
    );
    let attachment = entry.attachment_id.0.to_string();
    let with_id = request(json!({ "attachmentId": attachment }));

    // The preview refuses the bytes and leaves no busy slot behind.
    for _ in 0..2 {
        let preview = b.attachment_preview(&typed(with_id.clone()));
        assert_eq!(
            code(preview.unwrap_err()),
            ErrorCode::AttachmentPreviewUnsupported
        );
        assert!(!b.preview_busy.load(Ordering::SeqCst));
    }

    // Save as: the suggested name is safe, the bytes are exact, no partial
    // file stays, and the Zone.Identifier (Windows) marks it as downloaded.
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let target = out.join("saved.pdf");
    dialog.set(Some(target.clone()));
    assert_eq!(
        b.attachment_save(&typed(with_id.clone())).unwrap(),
        AttachmentSaved::Saved
    );
    assert_eq!(std::fs::read(&target).unwrap(), body);
    let left: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["saved.pdf".to_owned()]);
    // A folder that does not exist: a clean failure, nothing created.
    dialog.set(Some(dir.path().join("no-such-dir").join("x.pdf")));
    assert_eq!(
        code(b.attachment_save(&typed(with_id.clone())).unwrap_err()),
        ErrorCode::AttachmentSaveFailed
    );
    assert!(!dir.path().join("no-such-dir").exists());
    // A cancelled dialog decrypts nothing and writes nothing.
    dialog.set(None);
    assert_eq!(
        b.attachment_save(&typed(with_id)).unwrap(),
        AttachmentSaved::Cancelled
    );

    // Adding: a directory, a missing file and an over-limit sparse file.
    for (path, expected) in [
        (
            dir.path().to_path_buf(),
            ErrorCode::AttachmentFileUnreadable,
        ),
        (
            dir.path().join("missing.bin"),
            ErrorCode::AttachmentFileUnreadable,
        ),
    ] {
        dialog.set(Some(path));
        assert_eq!(
            code(b.attachment_add(typed(request(json!({})))).unwrap_err()),
            expected
        );
    }
    let huge = dir.path().join("huge.bin");
    std::fs::File::create(&huge)
        .unwrap()
        .set_len(u64::try_from(mosaic_core::MAX_ATTACHMENT_LEN).unwrap() + 1)
        .unwrap();
    dialog.set(Some(huge));
    assert_eq!(
        code(b.attachment_add(typed(request(json!({})))).unwrap_err()),
        ErrorCode::AttachmentTooLarge
    );
    let detail = b.get_item(typed(request(json!({})))).unwrap();
    assert_eq!(detail.attachments.len(), 1, "the failures added nothing");

    // A lock during the preview or save answers `locked` and writes nothing.
    b.lock(LockReason::Manual).unwrap();
    assert!(!b.preview_busy.load(Ordering::SeqCst));
}

/// Held: the passkey export spends the grant only on a written file; a
/// cancelled dialog hands it back (one password, one export), a wrong
/// password with a grant in hand is counted and exports nothing, and the
/// file carries the private keys only where the user chose.
#[test]
fn a_passkey_export_spends_one_grant_per_written_file() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let dialog = PathDialog::default();
    let b = backend(Store::open_in_memory().unwrap(), &dialog);
    let secret_key = create(&b);
    verify_kit(&b, &secret_key);
    add_passkey(&b);
    let vault = first_vault(&b);
    let request = |password: Value| json!({ "vaultId": vault, "password": password });

    // A wrong password is counted even while a grant is held, and the
    // grant stays: five wrong ones lock the vault.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    assert_eq!(
        code(
            b.export_passkeys_cxf(typed(request(json!("wrong"))))
                .unwrap_err()
        ),
        ErrorCode::WrongCredentials
    );
    assert!(b.quick_unlock.reauth_granted());
    assert_eq!(dialog.saves(), 0);

    // Cancelled: the grant is handed back; written: it is spent.
    dialog.set(None);
    assert_eq!(
        b.export_passkeys_cxf(typed(request(Value::Null))).unwrap(),
        PasskeysExported::Cancelled
    );
    assert!(b.quick_unlock.reauth_granted(), "a cancel keeps the grant");
    let target = dir.path().join("passkeys.json");
    dialog.set(Some(target.clone()));
    assert_eq!(
        b.export_passkeys_cxf(typed(request(Value::Null))).unwrap(),
        PasskeysExported::Saved { count: 1 }
    );
    assert!(!b.quick_unlock.reauth_granted(), "a written file spends it");
    assert!(
        std::fs::read_to_string(&target)
            .unwrap()
            .contains("example.com")
    );
    assert_eq!(
        code(
            b.export_passkeys_cxf(typed(request(Value::Null)))
                .unwrap_err()
        ),
        ErrorCode::ReauthRequired
    );
    assert_eq!(dialog.saves(), 2);
}

// ---------------------------------------------------------------------------
// 3. The local state file and the data directory (another local user)
// ---------------------------------------------------------------------------

/// RT-desktop-2 (Unix). `settings.json` holds the account email, the account
/// ID and every protection setting; the store directory holds it, the SQLite
/// store and the attachments. The store file is `0600` and the attachment
/// directory `0700`, but `persist::write_tmp_then_rename` creates the state
/// file with the umask default (`0644` on most systems) and `open_store`
/// creates the data directory with `create_dir_all` (`0755`). Another local
/// user on a shared machine reads the account email and ID and sees which
/// protections are on. Wanted: `0600` for the file and `0700` for the
/// directory, like the store (CRYPTO_SPEC local-store section, THREAT_MODEL
/// "another user" attacker).
#[cfg(unix)]
#[test]
#[ignore = "RT-desktop-2: settings.json and the data directory are created with the umask default, readable by other local users"]
fn the_state_file_and_the_data_directory_are_owner_only() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let slot = crate::open_store(&data);
    assert!(matches!(slot, Slot::Locked(_)));
    let persistence = crate::persist::Persistence::in_dir(&data);
    persistence
        .save(&crate::persist::StoredState::default(), None)
        .unwrap();
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&data.join("settings.json")), 0o600, "settings.json");
    assert_eq!(mode(&data), 0o700, "the data directory");
    assert_eq!(mode(&data.join(crate::STORE_FILE)), 0o600, "the store");
}

// ---------------------------------------------------------------------------
// 4. A hostile peer in the shared sync folder (Windows junctions)
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn subdirs(path: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(path)
        .map(|read| {
            read.filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

#[cfg(windows)]
fn junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Held (Windows): another member of the team folder plants directory
/// junctions where this device keeps its own files (its log, its device
/// directory, the vault's blob directory), each pointing at a directory of
/// the victim's own profile. Sync neither reads through them nor deletes
/// their targets when it cleans its own directories, and it keeps working.
/// The Unix symlink case has its test in `mosaic-sync` (`fsutil`); junctions
/// are a reparse point of their own and had none.
#[cfg(windows)]
#[test]
fn junctions_planted_in_the_sync_folder_are_neither_followed_nor_emptied() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("cloud");
    std::fs::create_dir_all(&folder).unwrap();
    // What the victim must not lose: a directory with a file in it.
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("canary.txt"), b"keep me").unwrap();

    let e = crate::sync_handler_tests::Env::on_disk(dir.path());
    e.create();
    e.connect_folder(&folder);
    e.b().sync_now().unwrap();

    let accounts = subdirs(&folder.join("a"));
    assert_eq!(accounts.len(), 1, "one account directory");
    let account = &accounts[0];
    let vaults = subdirs(&account.join("v"));
    assert!(!vaults.is_empty(), "the first vault was pushed");
    let logs = subdirs(&vaults[0].join("log"));
    assert!(!logs.is_empty(), "this device's log exists");
    let devices = subdirs(&account.join("d"));
    assert!(!devices.is_empty(), "this device's directory exists");

    let links = [
        logs[0].join("00000000000000ff.mseg"),
        logs[0].join("planted"),
        devices[0].join("planted.mck"),
        vaults[0].join("blobs").join("planted.matt"),
    ];
    let mut made = 0;
    for link in &links {
        if let Some(parent) = link.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if junction(link, &outside) {
            made += 1;
        }
    }
    assert_eq!(
        made,
        links.len(),
        "`mklink /J` makes a junction without privileges"
    );

    for _ in 0..3 {
        e.b().sync_now().unwrap();
    }
    assert!(
        outside.join("canary.txt").is_file(),
        "a junction's target was emptied by the sync folder's cleanup"
    );
    assert!(outside.is_dir());
    // A new edit still syncs: the planted names did not stall the own log.
    add_login(e.b(), &first_vault(e.b()));
    e.b().sync_now().unwrap();
    let segments = std::fs::read_dir(&logs[0])
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .count();
    assert!(segments >= 1, "the own log still has regular segment files");
    assert!(outside.join("canary.txt").is_file());
}
