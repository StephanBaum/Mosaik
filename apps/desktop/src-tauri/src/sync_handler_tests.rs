//! Tests of the sync commands (M6 T5): every command locked, without an
//! account and unlocked, the folder choice (`path_never_accepted_from_webview`,
//! the five-minute expiry), the token in the keystore, the idle timer, the
//! master password change, the joins (folder and hosted with a second
//! factor), `sync_acknowledge` and a lock. The hosted tests run the real
//! server on `127.0.0.1` (the loopback helper of the server's tests).
#![expect(clippy::too_many_lines, reason = "the scenarios are long")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::{AccountId, Argon2Profile, SecretKey};
use mosaic_sync::engine::SessionAccess as _;
use mosaic_sync::http::{StoredToken, Token as _, hosted};
use mosaic_sync::transport::TransportError;
use mosaic_sync::wire::{SessionCrypto, SyncCrypto as _, WireError};
use mosaic_totp::OtpAuth;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::{Store, SyncHalt};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{LockReason, SyncStatus};
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, SessionState, UnlockRequest};
use crate::contract::sync::{
    AcknowledgeRequest, ChangePasswordRequest, CheckServerRequest, CodeRequest,
    DeviceRemoveRequest, DeviceRenameRequest, FolderConnectRequest, FolderState,
    HostedConnectRequest, HostedMode, JoinFolderRequest, JoinHostedRequest, JoinOutcome,
    RecoverPasswordRequest, SyncConfigView, SyncOverview, SyncWarningView, TotpBeginRequest,
    TotpDisableRequest,
};
use crate::dialogs::{Dialogs, FileFilter};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::{MemoryKeystore, MemorySyncTokens, SecretKeyStore, SyncTokenStore};
use crate::state::{AppState, Backend, Slot};
use crate::test_support::kdf_permit;

#[path = "../../../../crates/server/tests/it/loopback.rs"]
pub(crate) mod loopback;

use loopback::LoopbackServer;

pub(crate) const PASSWORD: &str = "correct horse battery staple";
const NEW_PASSWORD: &str = "a different master password";
pub(crate) const DEVICE: &str = "Mosaic on Windows";
const ITEM_PASSWORD: &str = "It3m-Pa55word!";

fn req<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

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

struct SharedTokens(Arc<MemorySyncTokens>, Arc<AtomicBool>);

impl SyncTokenStore for SharedTokens {
    fn load(&self, account: AccountId) -> IpcResult<Option<StoredToken>> {
        self.0.load(account)
    }
    fn save(&self, account: AccountId, token: &StoredToken) -> IpcResult<()> {
        if self.1.load(Ordering::SeqCst) {
            return Err(IpcError::INTERNAL);
        }
        self.0.save(account, token)
    }
    fn delete(&self, account: AccountId) -> IpcResult<()> {
        self.0.delete(account)
    }
}

/// Answers the folder dialog with a fixed path and the confirmation with a
/// fixed answer (and remembers what it was asked).
struct PickDialog(
    Arc<Mutex<Option<PathBuf>>>,
    Arc<AtomicBool>,
    Arc<Mutex<Vec<String>>>,
);

impl Dialogs for PickDialog {
    fn save_file(&self, _: &str, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }
    fn open_file(&self, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }
    fn pick_folder(&self) -> IpcResult<Option<PathBuf>> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn confirm(&self, title: &str, message: &str) -> IpcResult<bool> {
        self.2.lock().unwrap().push(format!("{title}|{message}"));
        Ok(self.1.load(Ordering::SeqCst))
    }
}

fn ack() -> AcknowledgeRequest {
    AcknowledgeRequest {}
}

pub(crate) struct Env {
    state: AppState,
    events: Log,
    tokens: Arc<MemorySyncTokens>,
    keys: SharedKeystore,
    picked: Arc<Mutex<Option<PathBuf>>>,
    /// What the native confirmation answers.
    confirm: Arc<AtomicBool>,
    asked: Arc<Mutex<Vec<String>>>,
    /// Makes the keystore refuse every token save.
    fail_saves: Arc<AtomicBool>,
}

impl Env {
    pub(crate) fn new() -> Self {
        Self::with_store(Store::open_in_memory().unwrap(), None)
    }

    fn with_store(store: Store, dir: Option<PathBuf>) -> Self {
        let tokens = Arc::new(MemorySyncTokens::default());
        let picked = Arc::new(Mutex::new(None));
        let confirm = Arc::new(AtomicBool::new(true));
        let asked = Arc::new(Mutex::new(Vec::new()));
        let fail_saves = Arc::new(AtomicBool::new(false));
        let keys = SharedKeystore::default();
        let mut backend = Backend::new(
            Slot::Locked(LockedSession::open(store)),
            Box::new(keys.clone()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_sync_tokens(Box::new(SharedTokens(
            Arc::clone(&tokens),
            Arc::clone(&fail_saves),
        )))
        .with_dialogs(Box::new(PickDialog(
            Arc::clone(&picked),
            Arc::clone(&confirm),
            Arc::clone(&asked),
        )));
        if let Some(dir) = dir {
            backend = backend.with_store_dir(dir);
        }
        let state = AppState::new(backend);
        let (channel, events) = recording_channel();
        state.backend().subscribe(channel).unwrap();
        Self {
            state,
            events,
            tokens,
            keys,
            picked,
            confirm,
            asked,
            fail_saves,
        }
    }

    pub(crate) fn b(&self) -> &Backend {
        self.state.backend()
    }

    pub(crate) fn create(&self) -> String {
        self.create_as("alex@example.com")
    }

    pub(crate) fn create_as(&self, email: &str) -> String {
        let created = self
            .b()
            .create_account(CreateAccountRequest {
                email: email.into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        serde_json::to_value(&created).unwrap()["emergencyKit"]["secretKey"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn account(&self) -> AccountId {
        self.b().account().unwrap().account_id.get()
    }

    fn pick(&self, dir: &std::path::Path) -> crate::contract::sync::FolderChoice {
        *self.picked.lock().unwrap() = Some(dir.to_path_buf());
        self.b().sync_pick_folder().unwrap().unwrap()
    }

    pub(crate) fn connect_folder(&self, dir: &std::path::Path) -> SyncOverview {
        let choice = self.pick(dir);
        self.b()
            .sync_folder_connect(&FolderConnectRequest {
                choice_id: choice.choice_id,
                device_name: DEVICE.into(),
            })
            .unwrap()
    }

    pub(crate) fn connect_hosted(&self, server: &LoopbackServer) -> SyncOverview {
        self.b()
            .sync_hosted_connect(HostedConnectRequest {
                server_url: server.base.clone(),
                mode: HostedMode::Register,
                password: SecretIn::test(PASSWORD),
                invite_code: None,
                device_name: DEVICE.into(),
            })
            .unwrap()
    }

    /// What the native confirmations were asked, oldest first.
    pub(crate) fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    /// What the native confirmation answers from now on.
    pub(crate) fn answer(&self, yes: bool) {
        self.confirm.store(yes, Ordering::SeqCst);
    }

    pub(crate) fn status(&self) -> SyncOverview {
        self.b().sync_status().unwrap()
    }

    /// Waits until a sync run finished (the status says up to date and a time).
    pub(crate) fn wait_synced(&self) {
        wait_for("a finished sync run", || {
            let state = self.status().state;
            state.status == SyncStatus::UpToDate && state.last_synced_at.is_some()
        });
    }

    fn unlock(&self) -> IpcResult<SessionState> {
        self.b().unlock(UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: None,
        })
    }
}

pub(crate) fn wait_for(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(90);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn totp_code(uri: &str, server: &LoopbackServer) -> String {
    use mosaic_server::Clock as _;
    let otp = OtpAuth::parse(uri).unwrap();
    otp.otp
        .code_at(server.clock.now_ms() / 1000)
        .unwrap()
        .expose_secret()
        .to_owned()
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
        "customFields": [],
        "tags": [],
        "otp": { "op": "remove" }
    })
}

fn add_login(b: &Backend) {
    let vault = b.list_vaults().unwrap()[0].vault_id.0.to_string();
    b.add_item(req(
        json!({ "vaultId": vault, "kind": "login", "draft": login_draft() }),
    ))
    .unwrap();
}

fn hosted_config(overview: &SyncOverview) -> (&str, &str) {
    match overview.config.as_ref().unwrap() {
        SyncConfigView::Hosted {
            server_url, email, ..
        } => (server_url, email),
        SyncConfigView::Folder { .. } => panic!("folder"),
    }
}

// ---- every command, in every state ---------------------------------------------

#[test]
fn every_command_locked_without_account_and_unlocked() {
    let _kdf = kdf_permit();
    let e = Env::new();
    // No account: everything that needs the session says locked, the folder
    // dialog answers "cancelled" without a dialog.
    let locked = |r: IpcResult<()>| code(r.unwrap_err());
    assert_eq!(code(e.b().sync_status().unwrap_err()), ErrorCode::Locked);
    assert_eq!(code(e.b().sync_now().unwrap_err()), ErrorCode::Locked);
    assert_eq!(
        code(e.b().sync_disconnect().unwrap_err()),
        ErrorCode::Locked
    );
    assert_eq!(
        code(e.b().sync_acknowledge(&ack()).unwrap_err()),
        ErrorCode::Locked
    );
    assert_eq!(code(e.b().sync_devices().unwrap_err()), ErrorCode::Locked);
    assert_eq!(
        locked(e.b().sync_device_rename(&DeviceRenameRequest {
            device_id: crate::contract::ids::WireInstallId::from(
                mosaic_core::InstallId::new_v7().unwrap()
            ),
            name: "x".into(),
        })),
        ErrorCode::Locked
    );
    assert_eq!(
        code(
            e.b()
                .sync_hosted_connect(HostedConnectRequest {
                    server_url: "http://127.0.0.1:1".into(),
                    mode: HostedMode::SignIn,
                    password: SecretIn::test(PASSWORD),
                    invite_code: None,
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::Locked
    );
    assert_eq!(
        code(
            e.b()
                .sync_folder_connect(&FolderConnectRequest {
                    choice_id: "choice-0".into(),
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::SyncChoiceExpired
    );
    assert!(e.b().sync_pick_folder().unwrap().is_none());
    assert_eq!(
        code(
            e.b()
                .sync_check_server(&CheckServerRequest {
                    server_url: "http://example.com".into()
                })
                .unwrap_err()
        ),
        ErrorCode::SyncServerInsecure
    );
    assert_eq!(
        code(
            e.b()
                .sync_check_server(&CheckServerRequest {
                    server_url: "http://127.0.0.1:1".into()
                })
                .unwrap_err()
        ),
        ErrorCode::SyncServerUnreachable
    );

    // Unlocked, sync off.
    let secret_key = e.create();
    let status = e.status();
    assert!(status.config.is_none());
    assert_eq!(status.state.status, SyncStatus::NotConfigured);
    assert_eq!(
        code(e.b().sync_now().unwrap_err()),
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(
        code(e.b().sync_disconnect().unwrap_err()),
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(
        code(e.b().sync_acknowledge(&ack()).unwrap_err()),
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(
        code(e.b().sync_devices().unwrap_err()),
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(
        code(
            e.b()
                .sync_totp_begin(&TotpBeginRequest {
                    password: SecretIn::test(PASSWORD)
                })
                .unwrap_err()
        ),
        ErrorCode::SyncNotConfigured
    );
    // A wrong master password counts and is refused before any network.
    assert_eq!(
        code(
            e.b()
                .sync_hosted_connect(HostedConnectRequest {
                    server_url: "http://127.0.0.1:1".into(),
                    mode: HostedMode::Register,
                    password: SecretIn::test("not the password"),
                    invite_code: None,
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::WrongCredentials
    );
    // A join needs an app without an account.
    assert_eq!(
        code(
            e.b()
                .join_hosted(JoinHostedRequest {
                    server_url: "http://127.0.0.1:1".into(),
                    email: "alex@example.com".into(),
                    password: SecretIn::test(PASSWORD),
                    secret_key: SecretIn::test(&secret_key),
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::AlreadyUnlocked
    );
}

// ---- the folder choice ---------------------------------------------------------

#[test]
fn path_never_accepted_from_webview() {
    // The request type has no path field, and unknown keys are refused.
    let with_path = json!({ "choiceId": "choice-1", "deviceName": DEVICE, "path": "C:\\Users" });
    assert!(serde_json::from_value::<FolderConnectRequest>(with_path).is_err());
    let join_with_path = json!({
        "choiceId": "x", "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "password": "p", "secretKey": "k", "deviceName": DEVICE, "folder": "D:\\"
    });
    assert!(serde_json::from_value::<JoinFolderRequest>(join_with_path).is_err());

    // A path in the place of a choice ID is just an unknown ID.
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    let as_choice = dir.path().display().to_string();
    let result = e.b().sync_folder_connect(&FolderConnectRequest {
        choice_id: as_choice,
        device_name: DEVICE.into(),
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncChoiceExpired);
    // And the folder stays untouched.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn choice_id_expires_after_five_minutes() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    let choice = e.pick(dir.path());
    assert_eq!(choice.contents, FolderState::Empty);
    assert!(choice.choice_id.starts_with("choice-"));
    e.b().sync.advance(Duration::from_secs(5 * 60 + 1));
    let result = e.b().sync_folder_connect(&FolderConnectRequest {
        choice_id: choice.choice_id,
        device_name: DEVICE.into(),
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncChoiceExpired);
    assert!(e.status().config.is_none());
    // A choice younger than that works, and is used up by the connect.
    let again = e.pick(dir.path());
    e.b()
        .sync_folder_connect(&FolderConnectRequest {
            choice_id: again.choice_id.clone(),
            device_name: DEVICE.into(),
        })
        .unwrap();
    assert_eq!(
        code(
            e.b()
                .sync_folder_connect(&FolderConnectRequest {
                    choice_id: again.choice_id,
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::SyncChoiceExpired
    );
}

#[test]
fn a_foreign_folder_is_refused_and_not_written() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"mine").unwrap();
    let choice = e.pick(dir.path());
    assert_eq!(choice.contents, FolderState::Other);
    let result = e.b().sync_folder_connect(&FolderConnectRequest {
        choice_id: choice.choice_id,
        device_name: DEVICE.into(),
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncFolderNotEmpty);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

// ---- folder sync, devices, idle timer, acknowledge, lock -------------------------

#[test]
fn folder_connect_syncs_lists_devices_and_renames() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    add_login(e.b());
    let dir = tempfile::tempdir().unwrap();
    let overview = e.connect_folder(dir.path());
    assert!(matches!(
        overview.config,
        Some(SyncConfigView::Folder { .. })
    ));
    e.wait_synced();
    // A second sync run: the engine keeps working.
    e.b().sync_now().unwrap();
    assert_eq!(e.status().state.status, SyncStatus::UpToDate);

    let devices = e.b().sync_devices().unwrap().devices;
    assert_eq!(devices.len(), 1);
    assert!(devices[0].current);
    assert!(!devices[0].can_remove);
    assert_eq!(devices[0].name.as_deref(), Some(DEVICE));

    let id = devices[0].device_id;
    e.b()
        .sync_device_rename(&DeviceRenameRequest {
            device_id: id,
            name: "Desk".into(),
        })
        .unwrap();
    let devices = e.b().sync_devices().unwrap().devices;
    assert_eq!(devices[0].name.as_deref(), Some("Desk"));
    // Another device's name is not this device's to change.
    let other =
        crate::contract::ids::WireInstallId::from(mosaic_core::InstallId::new_v7().unwrap());
    let result = e.b().sync_device_rename(&DeviceRenameRequest {
        device_id: other,
        name: "x".into(),
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::InvalidInput);
    // Removing is for the server only.
    let result = e.b().sync_device_remove(&DeviceRemoveRequest {
        device_id: other,
        password: SecretIn::test(PASSWORD),
        code: None,
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncNotConfigured);

    // Disconnect forgets the choice and keeps the data.
    let off = e.b().sync_disconnect().unwrap();
    assert!(off.config.is_none());
    assert_eq!(e.b().list_vaults().unwrap()[0].item_count, 1);
    let events = e.events.lock().unwrap().join("\n");
    assert!(events.contains("syncStateChanged"));
}

/// The run's own checkpoint write is not a foreign change: a folder sync with
/// nothing new does not start another run (it did, endlessly, every ~17 ms).
#[test]
fn an_idle_folder_sync_does_not_run_again() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    e.b().sync_now().unwrap();
    let before = e.status().state.last_synced_at;
    std::thread::sleep(Duration::from_millis(1500));
    let after = e.status().state;
    assert_eq!(after.status, SyncStatus::UpToDate);
    assert_eq!(
        after.last_synced_at, before,
        "an idle folder sync ran again"
    );
}

#[test]
fn sync_does_not_reset_idle_timer() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    e.b().activity.backdate(Duration::from_secs(600));
    e.b().sync_now().unwrap();
    add_login_poke_free(&e);
    e.b().sync_now().unwrap();
    let idle = e.b().activity.idle_at(Instant::now(), SystemTime::now());
    assert!(
        idle >= Duration::from_secs(590),
        "sync work counted as activity: idle {idle:?}"
    );
}

/// A local change through the sync thread's own path: poke, no `touch`.
fn add_login_poke_free(e: &Env) {
    e.b().sync_poke();
}

#[test]
fn sync_acknowledge_clears_a_halt_and_syncs() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();

    e.b()
        .with_unlocked(|s| Ok(s.set_sync_halt(SyncHalt::Rollback)?))
        .unwrap();
    e.b().sync_now().unwrap();
    let halted = e.status();
    assert_eq!(halted.state.status, SyncStatus::Error);
    assert!(
        halted.warnings.contains(&SyncWarningView::RollbackDetected),
        "{:?}",
        halted.warnings
    );
    // No automatic retry: another poke does not clear it.
    e.b().sync_poke();
    std::thread::sleep(Duration::from_millis(2_500));
    assert_eq!(e.status().state.status, SyncStatus::Error);

    // The native confirmation says no: nothing is cleared, nothing runs.
    e.confirm.store(false, Ordering::SeqCst);
    let declined = e.b().sync_acknowledge(&ack()).unwrap();
    assert!(
        declined
            .warnings
            .contains(&SyncWarningView::RollbackDetected)
    );
    assert!(
        e.b()
            .with_unlocked(|s| Ok(s.sync_findings()?.halt.is_some()))
            .unwrap()
    );
    assert_eq!(e.status().state.status, SyncStatus::Error);
    // It said what the webview told it to, once.
    assert_eq!(e.asked.lock().unwrap().len(), 1);
    // The texts are Rust's, English or German by the language setting.
    let asked = e.asked.lock().unwrap()[0].clone();
    assert!(
        asked.starts_with("Clear the sync warnings") || asked.starts_with("Sync-Warnungen"),
        "{asked}"
    );

    e.confirm.store(true, Ordering::SeqCst);
    let acknowledged = e.b().sync_acknowledge(&ack()).unwrap();
    assert!(
        !acknowledged
            .warnings
            .contains(&SyncWarningView::RollbackDetected)
    );
    wait_for("the sync after the acknowledgement", || {
        let s = e.status();
        s.state.status == SyncStatus::UpToDate && s.warnings.is_empty()
    });
    assert!(
        e.b()
            .with_unlocked(|s| Ok(s.sync_findings()?.is_clear()))
            .unwrap()
    );
}

#[test]
fn locking_stops_the_service_and_a_transport_op_fails_closed() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();

    // Handles of this generation: the engine's access and the transport's
    // crypto.
    let mut access = e.b().sync.session().unwrap();
    let crypto = SessionCrypto::new(access.clone()).unwrap();
    assert!(access.with(|s| s.install_id()).is_ok());
    assert!(crypto.checkpoint_mac(1, &[0; 32]).is_ok());

    e.b().lock(LockReason::Manual).unwrap();
    assert!(matches!(
        access.with(|s| s.install_id()),
        Err(mosaic_sync::engine::SyncError::Locked)
    ));
    assert_eq!(crypto.checkpoint_mac(1, &[0; 32]), Err(WireError::Locked));
    // A folder transport cannot be opened on a locked session either.
    let opened = crate::sync::open_folder(
        e.b().sync.session().unwrap(),
        dir.path(),
        e.account(),
        mosaic_core::InstallId::new_v7().unwrap(),
        DEVICE,
    );
    assert_eq!(opened.err(), Some(TransportError::Locked));

    // The state forgot everything, the thread is gone.
    assert_eq!(e.b().sync.state().status, SyncStatus::NotConfigured);

    // Unlocking rebuilds the service from the persisted state; the old
    // handles stay dead (another generation).
    e.unlock().unwrap();
    assert!(matches!(
        access.with(|s| s.install_id()),
        Err(mosaic_sync::engine::SyncError::Locked)
    ));
    e.wait_synced();
}

// ---- the folder join ---------------------------------------------------------------

#[test]
fn join_folder_then_unlock_flow() {
    let _kdf = kdf_permit();
    let a = Env::new();
    let secret_key = a.create();
    add_login(a.b());
    let dir = tempfile::tempdir().unwrap();
    a.connect_folder(dir.path());
    a.wait_synced();
    let account = a.account();

    // A new device: no account, the same folder.
    let b = Env::new();
    let choice = b.pick(dir.path());
    assert_eq!(choice.contents, FolderState::Mosaic);
    assert_eq!(choice.accounts.len(), 1);
    assert_eq!(choice.accounts[0].get(), account);
    let wrong = b.b().join_folder(JoinFolderRequest {
        choice_id: choice.choice_id.clone(),
        account_id: choice.accounts[0],
        password: SecretIn::test("not the password"),
        secret_key: SecretIn::test(&secret_key),
        device_name: DEVICE.into(),
    });
    assert_eq!(code(wrong.unwrap_err()), ErrorCode::WrongCredentials);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::NoAccount
    ));
    let joined = b
        .b()
        .join_folder(JoinFolderRequest {
            choice_id: choice.choice_id,
            account_id: choice.accounts[0],
            password: SecretIn::test(PASSWORD),
            secret_key: SecretIn::test(&secret_key),
            device_name: "Laptop".into(),
        })
        .unwrap();
    assert_eq!(joined, JoinOutcome::Unlocked);
    // Unlocked, kit verified, the Secret Key stored, sync configured.
    match b.b().session_state().unwrap() {
        SessionState::Unlocked { kit_verified, .. } => assert!(kit_verified),
        other => panic!("{other:?}"),
    }
    assert!(b.keys.0.load(account).unwrap().is_some());
    assert!(matches!(
        b.status().config,
        Some(SyncConfigView::Folder { .. })
    ));
    // The data arrives.
    wait_for("the first item on the new device", || {
        b.b()
            .list_vaults()
            .unwrap()
            .first()
            .is_some_and(|v| v.item_count == 1)
    });
    // And the new device unlocks with its password like any other.
    b.b().lock(LockReason::Manual).unwrap();
    assert!(matches!(b.unlock().unwrap(), SessionState::Unlocked { .. }));
}

#[test]
fn a_tampered_account_file_is_reported_and_leaves_the_app_without_account() {
    let _kdf = kdf_permit();
    let a = Env::new();
    let secret_key = a.create();
    let dir = tempfile::tempdir().unwrap();
    a.connect_folder(dir.path());
    a.wait_synced();
    // Flip the last byte of every account file (the MAC).
    let account = a.account();
    let account_dir = dir
        .path()
        .join("a")
        .join(account.to_string().replace('-', ""))
        .join("account");
    for entry in std::fs::read_dir(&account_dir).unwrap() {
        let path = entry.unwrap().path();
        let mut bytes = std::fs::read(&path).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        std::fs::write(&path, bytes).unwrap();
    }
    let b = Env::new();
    let choice = b.pick(dir.path());
    let result = b.b().join_folder(JoinFolderRequest {
        choice_id: choice.choice_id,
        account_id: choice.accounts[0],
        password: SecretIn::test(PASSWORD),
        secret_key: SecretIn::test(&secret_key),
        device_name: DEVICE.into(),
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncFolderDamaged);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::NoAccount
    ));
}

// ---- the server ------------------------------------------------------------------------

#[test]
fn hosted_connect_stores_the_token_and_disconnect_deletes_it() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    let check = e
        .b()
        .sync_check_server(&CheckServerRequest {
            server_url: server.base.clone(),
        })
        .unwrap();
    assert_eq!(check.server_url, server.base);
    let overview = e.connect_hosted(&server);
    assert_eq!(
        hosted_config(&overview),
        (server.base.as_str(), "alex@example.com")
    );
    let account = e.account();
    assert!(e.tokens.load(account).unwrap().is_some());
    e.wait_synced();
    assert_eq!(e.b().sync_devices().unwrap().devices.len(), 1);
    // Already configured.
    let again = e.b().sync_hosted_connect(HostedConnectRequest {
        server_url: server.base.clone(),
        mode: HostedMode::SignIn,
        password: SecretIn::test(PASSWORD),
        invite_code: None,
        device_name: DEVICE.into(),
    });
    assert_eq!(code(again.unwrap_err()), ErrorCode::SyncAlreadyConfigured);

    let off = e.b().sync_disconnect().unwrap();
    assert!(off.config.is_none());
    assert!(e.tokens.load(account).unwrap().is_none());
}

#[test]
fn the_token_is_deleted_on_a_401_and_when_the_device_is_removed() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();

    // The session ends on the server: the next run gets a 401.
    let token = e.tokens.load(account).unwrap().unwrap();
    hosted::logout(&server.base, &token).unwrap();
    e.b().sync_now().unwrap();
    wait_for("the signed-out status", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    assert!(e.tokens.load(account).unwrap().is_none());
    assert_eq!(e.status().state.error, Some(ErrorCode::SyncSignedOut));

    // Signing in again (the choice is still stored, the token is gone).
    server.advance_ms(60_000);
    e.b().sync_disconnect().unwrap();
    let signed_in = e
        .b()
        .sync_hosted_connect(HostedConnectRequest {
            server_url: server.base.clone(),
            mode: HostedMode::SignIn,
            password: SecretIn::test(PASSWORD),
            invite_code: None,
            device_name: DEVICE.into(),
        })
        .unwrap();
    assert!(signed_in.config.is_some());
    e.wait_synced();

    // Another device removes this one.
    server.advance_ms(60_000);
    let (opw, install, email) = e
        .b()
        .with_unlocked(|s| {
            let key = e.keys.load(s.account_id()).unwrap().unwrap();
            let opw =
                s.opaque_password(&mosaic_core::MasterPassword::new(PASSWORD.to_owned()), &key)?;
            Ok((opw, s.install_id(), "alex@example.com"))
        })
        .unwrap();
    let other = mosaic_core::InstallId::new_v7().unwrap();
    let grant = hosted::login(&server.base, email, account, &opw, other, "Other").unwrap();
    hosted::device_remove(&server.base, &grant.token, install).unwrap();
    e.b().sync_now().unwrap();
    wait_for("the signed-out status after the removal", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    assert!(e.tokens.load(account).unwrap().is_none());
}

#[test]
fn change_password_with_hosted_unreachable_refused() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    drop(server);
    let result = e.b().change_master_password(&ChangePasswordRequest {
        current_password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
        code: None,
    });
    assert_eq!(
        code(result.unwrap_err()),
        ErrorCode::SyncPasswordNeedsServer
    );
    // Nothing changed: the old password still unlocks.
    e.b().lock(LockReason::Manual).unwrap();
    assert!(e.unlock().is_ok());
}

#[test]
fn change_password_hosted_keeps_the_fresh_token_and_sync_working() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    let old = *e.tokens.load(account).unwrap().unwrap().expose_bytes();
    server.advance_ms(60_000);
    e.b()
        .change_master_password(&ChangePasswordRequest {
            current_password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    // The change ended the old session; the device holds the new one.
    let fresh = *e.tokens.load(account).unwrap().unwrap().expose_bytes();
    assert_ne!(old, fresh);
    e.b().sync_now().unwrap();
    wait_for("a sync after the change", || {
        e.status().state.status == SyncStatus::UpToDate
    });
    // The new password unlocks, the old one does not.
    e.b().lock(LockReason::Manual).unwrap();
    let old_password = e.b().unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    });
    assert_eq!(code(old_password.unwrap_err()), ErrorCode::WrongCredentials);
    assert!(
        e.b()
            .unlock(UnlockRequest {
                password: SecretIn::test(NEW_PASSWORD),
                secret_key: None,
            })
            .is_ok()
    );
}

#[test]
fn join_hosted_with_totp() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let a = Env::new();
    let secret_key = a.create();
    add_login(a.b());
    a.connect_hosted(&server);
    a.wait_synced();

    // Two-step verification on the sync account.
    server.advance_ms(60_000);
    let setup = a
        .b()
        .sync_totp_begin(&TotpBeginRequest {
            password: SecretIn::test(PASSWORD),
        })
        .unwrap();
    let uri = serde_json::to_value(&setup).unwrap()["otpauthUri"]
        .as_str()
        .unwrap()
        .to_owned();
    a.b()
        .sync_totp_confirm(&CodeRequest {
            code: SecretIn::test(&totp_code(&uri, &server)),
        })
        .unwrap();
    assert!(matches!(
        a.status().config,
        Some(SyncConfigView::Hosted {
            totp_enabled: true,
            ..
        })
    ));
    // The sync thread is back on the fresh token.
    a.b().sync_now().unwrap();
    a.wait_synced();

    // A new device joins: the password and the key open the account, the
    // code finishes the login.
    server.advance_ms(60_000);
    let b = Env::new();
    let first = b
        .b()
        .join_hosted(JoinHostedRequest {
            server_url: server.base.clone(),
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            secret_key: SecretIn::test(&secret_key),
            device_name: "Laptop".into(),
        })
        .unwrap();
    assert_eq!(first, JoinOutcome::TwoFactorRequired);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::NoAccount
    ));
    let wrong = b.b().join_hosted_two_factor(&CodeRequest {
        code: SecretIn::test("000000"),
    });
    assert_eq!(code(wrong.unwrap_err()), ErrorCode::SyncTwoFactorInvalid);
    // The first step's code was spent by the setup.
    server.advance_ms(30_000);
    let done = b
        .b()
        .join_hosted_two_factor(&CodeRequest {
            code: SecretIn::test(&totp_code(&uri, &server)),
        })
        .unwrap();
    assert_eq!(done, JoinOutcome::Unlocked);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::Unlocked {
            kit_verified: true,
            ..
        }
    ));
    wait_for("the item on the joined device", || {
        b.b()
            .list_vaults()
            .unwrap()
            .first()
            .is_some_and(|v| v.item_count == 1)
    });

    // A fresh login for the second factor's removal needs two codes: the
    // first attempt asks for the next one. (The device's note of the setup's
    // fresh login has run out by then.)
    a.b().sync.advance(Duration::from_secs(300));
    server.advance_ms(30_000);
    let first_try = a.b().sync_totp_disable(&TotpDisableRequest {
        password: SecretIn::test(PASSWORD),
        code: SecretIn::test(&totp_code(&uri, &server)),
    });
    assert_eq!(
        code(first_try.unwrap_err()),
        ErrorCode::SyncTwoFactorRequired
    );
    server.advance_ms(30_000);
    a.b()
        .sync_totp_disable(&TotpDisableRequest {
            password: SecretIn::test(PASSWORD),
            code: SecretIn::test(&totp_code(&uri, &server)),
        })
        .unwrap();
    assert!(matches!(
        a.status().config,
        Some(SyncConfigView::Hosted {
            totp_enabled: false,
            ..
        })
    ));
}

#[test]
fn the_sync_token_stores_keep_a_copy_per_account() {
    let tokens = MemorySyncTokens::default();
    let (a, b) = (AccountId::new_v7().unwrap(), AccountId::new_v7().unwrap());
    let mut bytes = [7u8; 32];
    let token = StoredToken::from_array(&mut bytes);
    assert_eq!(bytes, [0; 32], "the source array is wiped");
    assert!(tokens.load(a).unwrap().is_none());
    tokens.save(a, &token).unwrap();
    assert_eq!(*tokens.load(a).unwrap().unwrap().expose_bytes(), [7u8; 32]);
    assert!(tokens.load(b).unwrap().is_none());
    tokens.delete(a).unwrap();
    assert!(tokens.load(a).unwrap().is_none());
    assert!(!format!("{token:?}").contains('7'));
}

// ---- the T5 review follow-ups ------------------------------------------------------

fn hosted_sign_in(server: &LoopbackServer, password: &str) -> HostedConnectRequest {
    HostedConnectRequest {
        server_url: server.base.clone(),
        mode: HostedMode::SignIn,
        password: SecretIn::test(password),
        invite_code: None,
        device_name: DEVICE.into(),
    }
}

/// C-M1: a lock request (manual, idle, sleep, screen lock) waits for a master
/// password change in flight, and goes ahead after the cap.
#[test]
fn a_lock_during_a_password_change_is_deferred_until_it_ends() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let change = e.b().sync.begin_change().unwrap();
    assert_eq!(
        code(e.b().sync.begin_change().err().unwrap()),
        ErrorCode::Busy,
        "one change at a time"
    );
    let state = e.state.clone();
    let locker = std::thread::spawn(move || state.backend().lock(LockReason::Idle));
    std::thread::sleep(Duration::from_millis(400));
    assert!(e.b().is_unlocked(), "the lock waits for the change");
    assert!(!locker.is_finished());
    drop(change);
    locker.join().unwrap().unwrap();
    assert!(!e.b().is_unlocked());

    // After the cap the lock goes ahead even if the change is still running.
    e.unlock().unwrap();
    e.b().sync.set_defer_cap(Duration::from_millis(300));
    let _stuck = e.b().sync.begin_change().unwrap();
    let started = Instant::now();
    e.b().lock(LockReason::Manual).unwrap();
    assert!(started.elapsed() >= Duration::from_millis(250));
    assert!(!e.b().is_unlocked());
}

/// C-M1: the server took the change, the device never committed it (killed,
/// or locked past the cap). The new password brings the device back.
#[test]
fn a_password_change_the_device_missed_is_recovered_with_the_new_password() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    server.advance_ms(60_000);
    let (install, prepared) = e
        .b()
        .with_unlocked(|s| {
            let key = e.keys.load(s.account_id()).unwrap().unwrap();
            let prepared = s.prepare_master_password_change(
                &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::MasterPassword::new(NEW_PASSWORD.to_owned()),
                &key,
            )?;
            Ok((s.install_id(), prepared))
        })
        .unwrap();
    hosted::password_change(
        &server.base,
        "alex@example.com",
        &prepared,
        install,
        DEVICE,
        None,
    )
    .unwrap();
    drop(prepared);

    // The change ended this device's session.
    e.b().sync_now().unwrap();
    wait_for("the signed-out status", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    // Signing in with the password this device knows fails as "changed".
    server.advance_ms(60_000);
    let refused = e.b().sync_hosted_connect(hosted_sign_in(&server, PASSWORD));
    assert_eq!(code(refused.unwrap_err()), ErrorCode::SyncPasswordChanged);
    assert!(e.status().password_changed);
    // A wrong new password is refused and counted.
    server.advance_ms(60_000);
    let wrong = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test("not the new password"),
        code: None,
    });
    assert_eq!(code(wrong.unwrap_err()), ErrorCode::WrongCredentials);
    assert!(e.b().is_unlocked());
    // An invalid code is refused before any network call.
    let bad_code = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test(NEW_PASSWORD),
        code: Some(SecretIn::test("12a456")),
    });
    assert_eq!(code(bad_code.unwrap_err()), ErrorCode::InvalidInput);

    server.advance_ms(60_000);
    let recovered = e
        .b()
        .sync_recover_password(&RecoverPasswordRequest {
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    assert!(!recovered.password_changed);
    assert!(e.tokens.load(account).unwrap().is_some());
    e.b().sync_now().unwrap();
    e.wait_synced();
    // The device holds the new password now.
    e.b().lock(LockReason::Manual).unwrap();
    let old = e.b().unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    });
    assert_eq!(code(old.unwrap_err()), ErrorCode::WrongCredentials);
    assert!(
        e.b()
            .unlock(UnlockRequest {
                password: SecretIn::test(NEW_PASSWORD),
                secret_key: None,
            })
            .is_ok()
    );
}

/// C-M1 (folder): another run wrote the new account file; the new password
/// is adopted from the folder.
#[test]
fn a_folder_password_change_the_device_missed_is_recovered() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let (account, install, prepared) = e
        .b()
        .with_unlocked(|s| {
            let key = e.keys.load(s.account_id()).unwrap().unwrap();
            let prepared = s.prepare_master_password_change(
                &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::MasterPassword::new(NEW_PASSWORD.to_owned()),
                &key,
            )?;
            Ok((s.account_id(), s.install_id(), prepared))
        })
        .unwrap();
    // The folder gets the new account file; the commit never happens.
    let mut transport = crate::sync::open_folder(
        e.b().sync.session().unwrap(),
        dir.path(),
        account,
        install,
        DEVICE,
    )
    .unwrap();
    let version = transport.next_account_version().unwrap();
    transport
        .write_account_file(prepared.new_params(), prepared.new_wrapped_ak(), version)
        .unwrap();
    drop((transport, prepared));

    // The thread notices at its next connection; the flag appears.
    e.b().sync_restart();
    wait_for("the changed-password flag", || e.status().password_changed);
    let wrong = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test("not the new password"),
        code: None,
    });
    assert_eq!(code(wrong.unwrap_err()), ErrorCode::WrongCredentials);
    let recovered = e
        .b()
        .sync_recover_password(&RecoverPasswordRequest {
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    assert!(!recovered.password_changed);
    e.b().lock(LockReason::Manual).unwrap();
    assert!(
        e.b()
            .unlock(UnlockRequest {
                password: SecretIn::test(NEW_PASSWORD),
                secret_key: None,
            })
            .is_ok()
    );
}

/// C-M1: the keystore refuses the token after the server accepted the
/// change: the change stands and the device is signed out of sync.
#[test]
fn a_token_the_keystore_refuses_does_not_undo_a_password_change() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    e.fail_saves.store(true, Ordering::SeqCst);
    server.advance_ms(60_000);
    e.b()
        .change_master_password(&ChangePasswordRequest {
            current_password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    wait_for("the signed-out status", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    e.b().lock(LockReason::Manual).unwrap();
    assert!(
        e.b()
            .unlock(UnlockRequest {
                password: SecretIn::test(NEW_PASSWORD),
                secret_key: None,
            })
            .is_ok(),
        "the local commit happened"
    );
}

/// C-M2: a thread of an older generation, or one whose token was replaced by
/// a newer login, changes neither the token nor the status.
#[test]
fn an_older_threads_401_keeps_the_new_token_and_the_status() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    let old = e.tokens.load(account).unwrap().unwrap();
    let old_generation = e.b().sync.generation();

    // A newer login replaced the token: the thread's 401 leaves it alone.
    let mut bytes = [9u8; 32];
    let newer = StoredToken::from_array(&mut bytes);
    e.tokens.save(account, &newer).unwrap();
    e.b().sync_signed_out(old_generation, Some(&old));
    assert_eq!(
        *e.tokens.load(account).unwrap().unwrap().expose_bytes(),
        [9u8; 32]
    );
    assert_eq!(e.status().state.status, SyncStatus::UpToDate);

    // A thread that a restart replaced does nothing at all.
    e.b().sync.stop();
    e.b().sync_signed_out(old_generation, Some(&newer));
    assert!(e.tokens.load(account).unwrap().is_some());
    assert_eq!(e.status().state.status, SyncStatus::UpToDate);

    // The current thread with its own token: deleted, signed out.
    e.b().sync_signed_out(e.b().sync.generation(), Some(&newer));
    assert!(e.tokens.load(account).unwrap().is_none());
    assert_eq!(e.status().state.status, SyncStatus::SignedOut);
}

/// C-M3: signing in again to the same server keeps the sync state; another
/// server starts clean.
#[test]
fn signing_in_again_keeps_the_sync_state_of_the_same_server() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let other = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    let token = e.tokens.load(account).unwrap().unwrap();
    hosted::logout(&server.base, &token).unwrap();
    e.b().sync_now().unwrap();
    wait_for("the signed-out status", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    // What a reset would clear (the flag) and what it never clears (the halt).
    e.b()
        .with_unlocked(|s| {
            let vault = s.vaults()[0].vault_id;
            s.set_vault_flag(vault, mosaic_vault::storage::VaultFlag::ServerInconsistent)?;
            Ok(s.set_sync_halt(SyncHalt::Rollback)?)
        })
        .unwrap();

    server.advance_ms(60_000);
    e.b()
        .sync_hosted_connect(hosted_sign_in(&server, PASSWORD))
        .unwrap();
    assert_eq!(
        e.b().with_unlocked(|s| Ok(s.vault_flags()?.len())).unwrap(),
        1,
        "the finding survived the new sign-in"
    );
    assert!(e.tokens.load(account).unwrap().is_some());

    // Another server: the sync state starts again (the thread of the old
    // sign-in is gone first, so it cannot write the status back).
    e.b().sync.stop();
    e.b().sync_force_signed_out();
    e.b()
        .sync_hosted_connect(HostedConnectRequest {
            server_url: other.base.clone(),
            mode: HostedMode::Register,
            password: SecretIn::test(PASSWORD),
            invite_code: None,
            device_name: DEVICE.into(),
        })
        .unwrap();
    assert_eq!(
        e.b().with_unlocked(|s| Ok(s.vault_flags()?.len())).unwrap(),
        0,
        "another server starts from a clean sync state"
    );
}

/// C-L1: a wrong second-factor code leaves the keystore alone, and the next
/// code finishes the same login.
#[test]
fn a_wrong_code_leaves_the_keystore_alone_and_the_next_code_finishes_the_login() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    server.advance_ms(60_000);
    let setup = e
        .b()
        .sync_totp_begin(&TotpBeginRequest {
            password: SecretIn::test(PASSWORD),
        })
        .unwrap();
    let uri = serde_json::to_value(&setup).unwrap()["otpauthUri"]
        .as_str()
        .unwrap()
        .to_owned();
    e.b()
        .sync_totp_confirm(&CodeRequest {
            code: SecretIn::test(&totp_code(&uri, &server)),
        })
        .unwrap();

    // A second device of the account (to remove).
    server.advance_ms(60_000);
    let (opw, install) = e
        .b()
        .with_unlocked(|s| {
            let key = e.keys.load(s.account_id()).unwrap().unwrap();
            let opw =
                s.opaque_password(&mosaic_core::MasterPassword::new(PASSWORD.to_owned()), &key)?;
            Ok((opw, s.install_id()))
        })
        .unwrap();
    let other = mosaic_core::InstallId::new_v7().unwrap();
    let grant = hosted::login(
        &server.base,
        "alex@example.com",
        account,
        &opw,
        other,
        "Other",
    )
    .unwrap();
    hosted::login_totp(&server.base, &grant.token, &totp_code(&uri, &server)).unwrap();
    assert_ne!(other, install);

    let before = *e.tokens.load(account).unwrap().unwrap().expose_bytes();
    server.advance_ms(30_000);
    e.b().sync.advance(Duration::from_secs(300));
    let remove = |digits: String| {
        e.b().sync_device_remove(&DeviceRemoveRequest {
            device_id: crate::contract::ids::WireInstallId::from(other),
            password: SecretIn::test(PASSWORD),
            code: Some(SecretIn::test(&digits)),
        })
    };
    let wrong = remove("000000".into());
    assert_eq!(code(wrong.unwrap_err()), ErrorCode::SyncTwoFactorInvalid);
    assert_eq!(
        *e.tokens.load(account).unwrap().unwrap().expose_bytes(),
        before,
        "the keystore is unchanged"
    );
    server.advance_ms(30_000);
    remove(totp_code(&uri, &server)).unwrap();
    assert_ne!(
        *e.tokens.load(account).unwrap().unwrap().expose_bytes(),
        before,
        "the finished login is kept"
    );
}

/// C-L2: pending joins and choices that ran out are dropped by the sweep and
/// by a lock; a flow that spans a lock drops its result.
#[test]
fn expired_pending_state_is_swept_and_a_flow_across_a_lock_writes_nothing() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    let choice = e.pick(dir.path());
    e.b()
        .sync
        .set_pending(Some(crate::sync::PendingLogin::for_tests(
            e.b().sync.now() + Duration::from_mins(5),
        )));
    e.b().sync.advance(Duration::from_mins(6));
    e.b().sync.sweep();
    assert!(
        e.b().sync.take_pending().is_none(),
        "the pending join is gone"
    );
    assert_eq!(
        code(e.b().sync.choice(&choice.choice_id).unwrap_err()),
        ErrorCode::SyncChoiceExpired
    );

    // A lock ends what a flow started before it.
    let epoch = e.b().sync.epoch();
    let kept = e.pick(dir.path());
    e.b()
        .sync
        .set_pending(Some(crate::sync::PendingLogin::for_tests(
            e.b().sync.now() + Duration::from_mins(5),
        )));
    e.b().lock(LockReason::Manual).unwrap();
    assert_eq!(
        code(e.b().sync.check_epoch(epoch).unwrap_err()),
        ErrorCode::Locked
    );
    assert!(e.b().sync.take_pending().is_none(), "a lock drops it");
    assert_eq!(
        code(e.b().sync.choice(&kept.choice_id).unwrap_err()),
        ErrorCode::SyncChoiceExpired
    );
}

/// C-L3: the commands refuse oversized fields and malformed codes before
/// any network call, and the probe is closed on a locked app with an account.
#[test]
fn bounds_codes_and_the_probe_oracle() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let long_url = format!("https://{}.example.com", "a".repeat(2100));
    let connect = |url: String, invite: Option<String>| {
        e.b().sync_hosted_connect(HostedConnectRequest {
            server_url: url,
            mode: HostedMode::Register,
            password: SecretIn::test(PASSWORD),
            invite_code: invite.map(|c| SecretIn::test(&c)),
            device_name: DEVICE.into(),
        })
    };
    assert_eq!(
        code(connect(long_url.clone(), None).unwrap_err()),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(connect("http://127.0.0.1:1".into(), Some("i".repeat(129))).unwrap_err()),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(
            e.b()
                .sync_folder_connect(&FolderConnectRequest {
                    choice_id: "c".repeat(65),
                    device_name: DEVICE.into(),
                })
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(
            e.b()
                .sync_check_server(&CheckServerRequest {
                    server_url: long_url
                })
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );
    for bad in ["", "12345", "123456789", "12a456", "１２３４５６"] {
        let result = e.b().sync_hosted_two_factor(&CodeRequest {
            code: SecretIn::test(bad),
        });
        assert_eq!(
            code(result.unwrap_err()),
            ErrorCode::InvalidInput,
            "{bad:?}"
        );
    }
    let long_name = e.b().sync_hosted_connect(HostedConnectRequest {
        server_url: "http://127.0.0.1:1".into(),
        mode: HostedMode::Register,
        password: SecretIn::test(PASSWORD),
        invite_code: None,
        device_name: "d".repeat(65),
    });
    assert_eq!(code(long_name.unwrap_err()), ErrorCode::InvalidInput);

    // Locked with an account: no probe.
    e.b().lock(LockReason::Manual).unwrap();
    let probe = e.b().sync_check_server(&CheckServerRequest {
        server_url: "http://127.0.0.1:1".into(),
    });
    assert_eq!(code(probe.unwrap_err()), ErrorCode::Locked);
}

// ---- the verify follow-ups ------------------------------------------------------------

/// The request of `sync_acknowledge` has no fields: the webview cannot send
/// the dialog's text.
#[test]
fn acknowledge_takes_no_fields() {
    assert!(serde_json::from_value::<AcknowledgeRequest>(json!({})).is_ok());
    assert!(serde_json::from_value::<AcknowledgeRequest>(json!({ "title": "x" })).is_err());
}

/// Verify 1: the app locked past the cap during a change, the server took it,
/// the commit fails: the new token is not kept, the overview says the
/// password changed, and the new password restores sync.
#[test]
fn a_change_cut_off_by_a_lock_is_offered_for_recovery_on_the_next_unlock() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    let account = e.account();
    let old_token = *e.tokens.load(account).unwrap().unwrap().expose_bytes();
    server.advance_ms(60_000);
    let prepared = e
        .b()
        .with_unlocked(|s| {
            let key = e.keys.load(s.account_id()).unwrap().unwrap();
            Ok(s.prepare_master_password_change(
                &mosaic_core::MasterPassword::new(PASSWORD.to_owned()),
                &mosaic_core::MasterPassword::new(NEW_PASSWORD.to_owned()),
                &key,
            )?)
        })
        .unwrap();
    // The lock goes ahead after the cap; the server's answer comes later.
    e.b().lock(LockReason::Idle).unwrap();
    e.unlock().unwrap();
    let ctx = e.b().hosted_ctx().unwrap();
    let outcome = e.b().change_password_hosted(
        &ctx,
        &ChangePasswordRequest {
            current_password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        },
        prepared,
    );
    assert_eq!(code(outcome.err().unwrap()), ErrorCode::SyncPasswordChanged);
    // The new token was not saved; the old one (revoked) stays.
    assert_eq!(
        *e.tokens.load(account).unwrap().unwrap().expose_bytes(),
        old_token
    );
    assert!(e.status().password_changed);

    // Even after another lock and unlock the device still finds the change at
    // its next sign-in (the old session is gone), and the new password
    // brings it back.
    wait_for("the signed-out status", || {
        e.status().state.status == SyncStatus::SignedOut
    });
    server.advance_ms(60_000);
    e.b()
        .sync_recover_password(&RecoverPasswordRequest {
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    e.b().sync_now().unwrap();
    e.wait_synced();
    assert!(!e.status().password_changed);
}

/// Verify 2 (folder): a folder offering the version this device wrote last,
/// or an older one, offers an old password again.
#[test]
fn a_folder_account_file_at_or_below_the_last_version_is_a_rollback() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    e.b()
        .change_master_password(&ChangePasswordRequest {
            current_password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    // The folder holds the version this device wrote: nothing newer to adopt.
    let result = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test(NEW_PASSWORD),
        code: None,
    });
    assert_eq!(code(result.unwrap_err()), ErrorCode::SyncPasswordRollback);
}

/// Verify 3: a sleep locks at once; any other reason waits for a change.
#[test]
fn a_sleep_lock_is_not_deferred() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let _change = e.b().sync.begin_change().unwrap();
    let started = Instant::now();
    e.b().lock(LockReason::Sleep).unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!e.b().is_unlocked());
}

/// Verify 5: every lock reason ends a pending login or join, also in the
/// onboarding (no vault unlocked), and a result that comes after a lock is
/// not kept.
#[test]
fn any_lock_drops_a_pending_join_and_a_late_result_is_not_kept() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let pending =
        || crate::sync::PendingLogin::for_tests(e.b().sync.now() + Duration::from_mins(5));
    for reason in [LockReason::Sleep, LockReason::ScreenLock, LockReason::Idle] {
        e.b().sync.set_pending(Some(pending()));
        e.b().lock(reason).unwrap();
        assert!(e.b().sync.take_pending().is_none(), "{reason:?}");
    }
    let epoch = e.b().sync.epoch();
    e.create();
    e.b().lock(LockReason::Manual).unwrap();
    assert!(!e.b().sync.set_pending_in(epoch, pending()));
    assert!(e.b().sync.take_pending().is_none());
}

/// Verify 5: the idle tick sweeps expired pending entries.
#[test]
fn the_idle_tick_sweeps_expired_pending_entries() {
    let e = Env::new();
    e.b()
        .sync
        .set_pending(Some(crate::sync::PendingLogin::for_tests(
            e.b().sync.now() + Duration::from_mins(5),
        )));
    e.b().housekeeping();
    assert!(e.b().sync.take_pending().is_some(), "not expired yet");
    e.b()
        .sync
        .set_pending(Some(crate::sync::PendingLogin::for_tests(
            e.b().sync.now() + Duration::from_mins(5),
        )));
    e.b().sync.advance(Duration::from_mins(6));
    e.b().housekeeping();
    assert!(e.b().sync.take_pending().is_none());
}

/// Verify 6: a thread of an older generation neither writes nor reports, and
/// a waiting fresh login is taken only by the account it belongs to.
#[test]
fn an_older_generation_reports_nothing_and_a_fresh_login_belongs_to_its_account() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let generation = e.b().sync.generation();
    assert!(!e.b().sync.finish_ok(e.b(), generation + 7, Vec::new()));
    e.b()
        .sync
        .set_pending(Some(crate::sync::PendingLogin::for_tests(
            e.b().sync.now() + Duration::from_mins(5),
        )));
    // The pending login has no account: another account's flow gets nothing.
    assert!(
        e.b()
            .sync
            .take_fresh_pending("http://127.0.0.1:1", e.account())
            .is_none()
    );
}

// ---- the milestone review (M2, M3, I2) ----------------------------------------

fn change_request() -> ChangePasswordRequest {
    ChangePasswordRequest {
        current_password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
        code: None,
    }
}

fn unlocks_with_the_new_password_only(e: &Env) {
    e.b().lock(LockReason::Manual).unwrap();
    assert!(e.unlock().is_err(), "the old password no longer unlocks");
    assert!(
        e.b()
            .unlock(UnlockRequest {
                password: SecretIn::test(NEW_PASSWORD),
                secret_key: None,
            })
            .is_ok()
    );
}

/// M2: the answer to `password/finish` is lost after the server committed.
/// The device checks at once with the new password and commits the change
/// locally in the same call.
#[test]
fn a_lost_finish_answer_is_recovered_in_the_same_call() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    server.advance_ms(60_000);
    server.faults.lose_finish_answers.store(1, Ordering::SeqCst);
    e.b().change_master_password(&change_request()).unwrap();
    assert_eq!(server.faults.lose_finish_answers.load(Ordering::SeqCst), 0);
    assert!(!e.status().password_changed);
    unlocks_with_the_new_password_only(&e);
}

/// M2: the answer is lost and the check cannot reach the server either. The
/// outcome is unknown: the device keeps both passwords, sets
/// `passwordChanged`, and the recovery with the new password settles it.
#[test]
fn an_unconfirmed_password_change_is_unknown_and_the_recovery_settles_it() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    e.create();
    e.connect_hosted(&server);
    e.wait_synced();
    server.advance_ms(60_000);
    server.faults.lose_finish_answers.store(1, Ordering::SeqCst);
    server
        .faults
        .outage_after_loss
        .store(true, Ordering::SeqCst);
    let result = e.b().change_master_password(&change_request());
    assert_eq!(
        code(result.unwrap_err()),
        ErrorCode::SyncPasswordOutcomeUnknown
    );
    assert!(e.status().password_changed);
    // The server is back; it holds the new password and this device the old.
    server.faults.outage.store(false, Ordering::SeqCst);
    server.advance_ms(60_000);
    e.b()
        .sync_recover_password(&RecoverPasswordRequest {
            new_password: SecretIn::test(NEW_PASSWORD),
            code: None,
        })
        .unwrap();
    assert!(!e.status().password_changed);
    unlocks_with_the_new_password_only(&e);
}

/// The verified account file of the folder, read the way the app reads it.
fn folder_account_file(
    e: &Env,
    dir: &std::path::Path,
) -> mosaic_sync::wire::account_file::AccountFile {
    let (install, account) = e
        .b()
        .with_unlocked(|s| Ok((s.install_id(), s.account_id())))
        .unwrap();
    crate::sync::open_folder(e.b().sync.session().unwrap(), dir, account, install, DEVICE)
        .unwrap()
        .verified_account_file()
        .unwrap()
        .unwrap()
}

/// M3: the password changed while the device used another backend. The old
/// folder still holds the retired password; reconnecting replaces it with
/// the device's own at a higher version, so a join with the old password
/// fails.
#[test]
fn reconnecting_a_folder_replaces_a_password_this_device_retired() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let e = Env::new();
    let secret_key = e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let old_salt = folder_account_file(&e, dir.path()).salt;
    e.b().sync_disconnect().unwrap();
    e.connect_hosted(&server);
    e.wait_synced();
    server.advance_ms(60_000);
    e.b().change_master_password(&change_request()).unwrap();
    e.b().sync_disconnect().unwrap();
    // The folder still offers the retired password at version 1.
    assert_eq!(folder_account_file(&e, dir.path()).salt, old_salt);
    e.connect_folder(dir.path());
    e.wait_synced();
    let file = folder_account_file(&e, dir.path());
    assert_eq!(file.account_version, 2);
    assert_ne!(file.salt, old_salt);
    assert!(!e.status().password_changed);
    // A new device cannot join with the old password any more.
    let b = Env::new();
    let choice = b.pick(dir.path());
    let join = |password: &str| {
        b.b().join_folder(JoinFolderRequest {
            choice_id: choice.choice_id.clone(),
            account_id: choice.accounts[0],
            password: SecretIn::test(password),
            secret_key: SecretIn::test(&secret_key),
            device_name: "Laptop".into(),
        })
    };
    assert_eq!(
        code(join(PASSWORD).unwrap_err()),
        ErrorCode::WrongCredentials
    );
    assert_eq!(join(NEW_PASSWORD).unwrap(), JoinOutcome::Unlocked);
}

/// M3: another device of the folder changed the password. This device sees
/// it when its folder thread starts and offers the recovery.
#[test]
fn a_password_change_by_another_folder_device_is_noticed_at_thread_start() {
    let _kdf = kdf_permit();
    let a = Env::new();
    let secret_key = a.create();
    let dir = tempfile::tempdir().unwrap();
    a.connect_folder(dir.path());
    a.wait_synced();
    let b = Env::new();
    let choice = b.pick(dir.path());
    b.b()
        .join_folder(JoinFolderRequest {
            choice_id: choice.choice_id,
            account_id: choice.accounts[0],
            password: SecretIn::test(PASSWORD),
            secret_key: SecretIn::test(&secret_key),
            device_name: "Laptop".into(),
        })
        .unwrap();
    b.wait_synced();
    assert!(!b.status().password_changed);
    a.b().change_master_password(&change_request()).unwrap();
    // The folder thread of device b starts again with its next unlock.
    b.b().lock(LockReason::Manual).unwrap();
    b.unlock().unwrap();
    wait_for("the changed-password flag on the second device", || {
        b.status().password_changed
    });
}

/// I2: a pending login is stored under the lock and the lock epoch checked
/// after it; one stored for an epoch that has passed is not kept.
#[test]
fn a_pending_login_is_not_kept_for_an_epoch_that_has_passed() {
    let e = Env::new();
    let pending =
        || crate::sync::PendingLogin::for_tests(e.b().sync.now() + Duration::from_mins(5));
    let epoch = e.b().sync.epoch();
    assert!(e.b().sync.set_pending_in(epoch, pending()));
    assert!(e.b().sync.take_pending().is_some());
    e.b().sync.reset();
    assert!(!e.b().sync.set_pending_in(epoch, pending()));
    assert!(e.b().sync.take_pending().is_none());
}
