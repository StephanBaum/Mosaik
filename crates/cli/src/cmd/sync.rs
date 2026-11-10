//! `sync`, `setup own-store` and `recover`.

use std::path::PathBuf;
use std::sync::Arc;

use mosaic_core::{MasterPassword, SecretKey};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, OwnedSession, SyncEngine, SyncError, SyncReport};
use mosaic_sync::folder::{self, FolderConfig, FolderContents, FolderTransport, JoinError};
use mosaic_sync::http::{HttpTransport, StoredToken};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::{Store, SyncConfig, SyncKind};
use serde::Serialize;

use crate::app::{App, guard_rotation};
use crate::error::{CliError, EXIT_AUTH, EXIT_NOT_FOUND, EXIT_OTHER, EXIT_SYNC, Result};
use crate::i18n::Msg;
use crate::spec::Parsed;

const DEVICE_NAME: &str = "mosaic-cli";

fn sync_failed(reason: &str) -> CliError {
    CliError::new("syncFailed", EXIT_SYNC, Msg::ErrSyncFailed).with("reason", reason)
}

fn map_sync_error(e: SyncError) -> CliError {
    match e {
        SyncError::RollbackDetected | SyncError::IdentityDuplicated => {
            CliError::new("syncHalted", EXIT_SYNC, Msg::ErrSyncHalted)
        }
        SyncError::Transport(t) => sync_failed(&t.to_string()),
        SyncError::Session(s) => s.into(),
        SyncError::Backoff { .. } => sync_failed("backing off"),
        SyncError::AccountRekeyed => sync_failed("the account was re-keyed on another device"),
        _ => CliError::internal(),
    }
}

const fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else {
        Platform::Linux
    }
}

/// The name of a finding, without its IDs.
fn kind_of(debug: &str) -> String {
    debug
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[derive(Serialize)]
struct SyncOut {
    pulled: u32,
    pushed: u32,
    findings: Vec<String>,
}

fn run_once(
    app: &App,
    owned: &mut OwnedSession,
    config: &SyncConfig,
    account: mosaic_core::AccountId,
    install: mosaic_core::InstallId,
) -> Result<SyncReport> {
    match config.kind {
        SyncKind::Folder => {
            let root = config.folder_path.clone().ok_or_else(CliError::internal)?;
            let crypto = SessionCrypto::new(owned.clone()).map_err(|_| CliError::internal())?;
            let transport = FolderTransport::open(FolderConfig {
                root,
                account_id: account,
                install_id: install,
                crypto,
                clock: Arc::new(SystemClock),
                device: DeviceInfo {
                    created_at_ms: app.now,
                    name: config.device_name.clone(),
                    platform: platform(),
                },
            })
            .map_err(|e| sync_failed(&e.to_string()))?;
            SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Folder))
                .sync_once(owned, &SystemClock)
                .map_err(map_sync_error)
        }
        SyncKind::Hosted => {
            let url = config.server_url.clone().ok_or_else(CliError::internal)?;
            let token: StoredToken =
                app.be.tokens.load(account).ok().flatten().ok_or_else(|| {
                    sync_failed("not signed in to the server (sign in with the app)")
                })?;
            let transport = HttpTransport::connect(&url, &token, install)
                .map_err(|e| sync_failed(&e.to_string()))?;
            SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted))
                .sync_once(owned, &SystemClock)
                .map_err(map_sync_error)
        }
    }
}

fn backend_name(config: &SyncConfig) -> &'static str {
    match config.kind {
        SyncKind::Hosted => "hosted",
        SyncKind::Folder => "folder",
    }
}

fn print_findings(app: &App, list: &[String]) {
    for kind in list {
        app.out.note(Msg::SyncFinding, &[("kind", kind.clone())]);
    }
}

/// `mosaic sync`: returns the exit code (6 on a halt or a finding).
pub fn sync(app: &mut App, p: &Parsed) -> Result<u8> {
    if p.has("status") {
        return status(app);
    }
    let u = app.unlock()?;
    guard_rotation(&u.session)?;
    let config = u
        .session
        .sync_config()?
        .ok_or_else(|| CliError::new("syncNotConfigured", EXIT_OTHER, Msg::ErrSyncNotConfigured))?;
    if p.has("acknowledge") {
        return acknowledge(app, u.session);
    }
    let (account, install) = (u.account, u.session.install_id());
    let mut owned = OwnedSession::new(u.session);
    let report = run_once(app, &mut owned, &config, account, install);
    drop(owned.take());
    let report = report?;
    let findings: Vec<String> = report
        .warnings
        .iter()
        .map(|w| kind_of(&format!("{w:?}")))
        .collect();
    let doc = SyncOut {
        pulled: report.pulled,
        pushed: report.pushed,
        findings: findings.clone(),
    };
    app.out.data(&doc, |o| {
        o.say(&o.t(
            Msg::SyncDone,
            &[
                ("pulled", report.pulled.to_string()),
                ("pushed", report.pushed.to_string()),
            ],
        ));
    });
    print_findings(app, &findings);
    Ok(if findings.is_empty() { 0 } else { EXIT_SYNC })
}

#[derive(Serialize)]
struct StatusOut {
    backend: &'static str,
    halted: bool,
    findings: Vec<String>,
}

/// `mosaic sync --status`: reads the store only (no unlock).
fn status(app: &mut App) -> Result<u8> {
    let locked = app.open_store()?;
    let store = locked.store();
    let config = store
        .sync_config()?
        .ok_or_else(|| CliError::new("syncNotConfigured", EXIT_OTHER, Msg::ErrSyncNotConfigured))?;
    let state = store.engine_state()?;
    let mut findings: Vec<String> = state.halt.iter().map(|h| format!("{h:?}")).collect();
    findings.extend(
        store
            .vault_flags()?
            .iter()
            .map(|(_, flag)| format!("{flag:?}")),
    );
    let doc = StatusOut {
        backend: backend_name(&config),
        halted: state.halt.is_some(),
        findings: findings.clone(),
    };
    app.out.data(&doc, |o| {
        if findings.is_empty() {
            o.say(&o.t(Msg::SyncClean, &[]));
        }
    });
    print_findings(app, &findings);
    Ok(if findings.is_empty() { 0 } else { EXIT_SYNC })
}

/// `mosaic sync --acknowledge`: the word must be typed on the terminal (no `--yes`).
fn acknowledge(app: &mut App, mut session: UnlockedSession) -> Result<u8> {
    let findings = session.sync_findings()?;
    if findings.is_clear() {
        app.out.say(&app.out.t(Msg::SyncClean, &[]));
        return Ok(0);
    }
    let mut kinds: Vec<String> = findings.halt.iter().map(|h| format!("{h:?}")).collect();
    kinds.extend(findings.vault_flags.iter().map(|(_, f)| format!("{f:?}")));
    kinds.extend(
        findings
            .no_common_view
            .iter()
            .map(|_| "NoCommonView".to_owned()),
    );
    print_findings(app, &kinds);
    let word = app.out.t(Msg::SyncAcknowledgeWord, &[]);
    let typed = app
        .prompt
        .line(Msg::PromptAcknowledge, &[("word", word.clone())])?;
    if typed != word {
        return Err(CliError::new("cancelled", EXIT_OTHER, Msg::ImportCancelled));
    }
    session.clear_sync_halt()?;
    session.clear_vault_findings(&findings.vaults())?;
    app.out
        .data(&serde_json::json!({ "acknowledged": true }), |o| {
            o.say(&o.t(Msg::SyncAcknowledged, &[]));
        });
    Ok(0)
}

fn own_store_path(app: &App, p: &Parsed) -> Result<PathBuf> {
    if p.value("store").is_some() || std::env::var_os("MOSAIC_STORE").is_some_and(|v| !v.is_empty())
    {
        return Ok(app.store_path.clone());
    }
    mosaic_platform::paths::cli_store().map_err(|_| CliError::internal())
}

fn remove_store(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    for suffix in ["-lock", "-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(name));
    }
}

/// `mosaic setup own-store`: joins the account as a separate device over a sync folder.
pub fn setup(app: &mut App, p: &Parsed) -> Result<()> {
    if p.value("server").is_some() {
        return Err(CliError::not_supported("setup own-store --server"));
    }
    let folder_path = p
        .value("folder")
        .map(PathBuf::from)
        .ok_or_else(|| CliError::usage(Msg::ErrMissingArgument).with("name", "--folder"))?;
    let target = own_store_path(app, p)?;
    if target.exists() {
        return Err(
            CliError::new("storeExists", EXIT_OTHER, Msg::ErrStoreExists)
                .with("path", target.display().to_string()),
        );
    }
    let account = match folder::inspect(&folder_path) {
        FolderContents::Mosaic { accounts } => match accounts.as_slice() {
            [one] => *one,
            _ => return Err(CliError::not_found(&folder_path.display().to_string())),
        },
        _ => return Err(CliError::not_found(&folder_path.display().to_string())),
    };
    let secret_key = if let Some(key) = app.be.keys.load(account).ok().flatten() {
        key
    } else {
        let typed = app.prompt.secret(Msg::PromptSecretKey, &[])?;
        SecretKey::parse(&typed)
            .map_err(|_| CliError::new("secretKeyInvalid", EXIT_AUTH, Msg::ErrSecretKeyInvalid))?
    };
    let mut typed = app.prompt.secret(Msg::PromptPassword, &[])?;
    let password = MasterPassword::new(std::mem::take(&mut *typed));
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|_| CliError::file_write(parent))?;
    }
    let store = Store::open(&target)?;
    let joined = folder::join_from_folder(
        &folder_path,
        account,
        LockedSession::open(store),
        &password,
        &secret_key,
    );
    let mut session = match joined {
        Ok(session) => session,
        Err(failure) => {
            drop(failure.session);
            remove_store(&target);
            return Err(match failure.error {
                JoinError::WrongCredentials => CliError::wrong_credentials(),
                JoinError::NoAccountFile | JoinError::Folder(_) => {
                    CliError::new("notFound", EXIT_NOT_FOUND, Msg::ErrNotFound)
                        .with("ref", folder_path.display().to_string())
                }
                JoinError::FolderDamaged => {
                    sync_failed("the account file in the folder is damaged")
                }
                _ => CliError::internal(),
            });
        }
    };
    let config = SyncConfig {
        kind: SyncKind::Folder,
        server_url: None,
        email: None,
        folder_path: Some(folder_path),
        device_name: DEVICE_NAME.to_owned(),
    };
    session.set_sync_config(Some(&config))?;
    drop(session);
    // The Secret Key is shared with the desktop: keep it if the keystore has none.
    let _ = app.be.keys.is_stored(account) || app.be.keys.save(account, &secret_key).is_ok();
    app.out.data(
        &serde_json::json!({ "store": target.display().to_string() }),
        |o| {
            o.say(&o.t(Msg::SetupDone, &[]));
        },
    );
    Ok(())
}

/// `mosaic recover`.
pub fn recover() -> Result<()> {
    Err(CliError::not_supported("recover"))
}
