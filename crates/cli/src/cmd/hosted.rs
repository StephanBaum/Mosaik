//! The hosted backend from the CLI: `setup own-store --server` (a hosted join) and `recover`
//! (adopting a master password change made elsewhere). Both use the same `mosaic-sync` and
//! `mosaic-vault` calls as the desktop's `hosted_flow.rs`.

use std::path::Path;

use mosaic_core::{AccountId, MasterPassword, SecretKey};
use mosaic_sync::http::{StoredToken, canonical_origin, hosted};
use mosaic_sync::transport::{Rejection, TransportError};
use mosaic_sync::wire::json::LoginStatus;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::{Store, SyncConfig, SyncKind};

use super::sync::remove_store;
use crate::app::{App, guard_rotation};
use crate::error::{CliError, EXIT_AUTH, EXIT_OTHER, EXIT_SYNC, Result};
use crate::i18n::Msg;
use crate::spec::Parsed;

const DEVICE_NAME: &str = "mosaic-cli";

fn sync_failed(reason: &str) -> CliError {
    CliError::new("syncFailed", EXIT_SYNC, Msg::ErrSyncFailed).with("reason", reason)
}

fn transport_error(e: &TransportError) -> CliError {
    match e {
        TransportError::Rejected(Rejection::LoginFailed) => CliError::wrong_credentials(),
        other => sync_failed(&other.to_string()),
    }
}

fn typed_secret_key(app: &mut App) -> Result<SecretKey> {
    let typed = app.prompt.secret(Msg::PromptSecretKey, &[])?;
    SecretKey::parse(&typed)
        .map_err(|_| CliError::new("secretKeyInvalid", EXIT_AUTH, Msg::ErrSecretKeyInvalid))
}

fn two_factor_code(app: &mut App) -> Result<String> {
    app.prompt.line(Msg::PromptTwoFactor, &[])
}

/// `mosaic setup own-store --server URL --email E`: OPAQUE sign-in with a second-factor prompt if
/// the account has one, then the account goes into a new own store and the session token into the
/// keystore (`mosaic-cli-sync-token-<account>`).
pub fn setup(app: &mut App, p: &Parsed, target: &Path) -> Result<()> {
    let email = p
        .value("email")
        .map(str::trim)
        .filter(|e| e.contains('@'))
        .ok_or_else(|| CliError::usage(Msg::ErrMissingArgument).with("name", "--email"))?
        .to_owned();
    let url =
        canonical_origin(p.value("server").unwrap_or_default()).map_err(|e| transport_error(&e))?;
    let secret_key = typed_secret_key(app)?;
    let mut typed = app.prompt.secret(Msg::PromptPassword, &[])?;
    let password = MasterPassword::new(std::mem::take(&mut *typed));
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|_| CliError::file_write(parent))?;
    }
    let joined = join(app, target, &url, &email, &password, &secret_key);
    if joined.is_err() {
        remove_store(target);
    }
    let account = joined?;
    // The Secret Key is shared with the desktop: keep it if the keystore has none.
    let _ = app.be.keys.is_stored(account) || app.be.keys.save(account, &secret_key).is_ok();
    app.out.data(
        &serde_json::json!({ "store": target.display().to_string() }),
        |o| o.say(&o.t(Msg::SetupDone, &[])),
    );
    Ok(())
}

fn join(
    app: &mut App,
    target: &Path,
    url: &str,
    email: &str,
    password: &MasterPassword,
    secret_key: &SecretKey,
) -> Result<AccountId> {
    let store = Store::open(target)?;
    let install = store.install_id()?;
    let login = hosted::join_login(url, email, password, secret_key, install, DEVICE_NAME)
        .map_err(|e| transport_error(&e))?;
    if login.grant.status == LoginStatus::TwoFactorRequired {
        let code = two_factor_code(app)?;
        hosted::login_totp(url, &login.grant.token, &code).map_err(|e| transport_error(&e))?;
    }
    let account = hosted::join_fetch_account(url, &login.grant.token, &login.keys)
        .map_err(|e| transport_error(&e))?;
    let token = StoredToken::from_session(&login.grant.token);
    let mut session = LockedSession::open(store)
        .join_account_with_keys_at(&account.wrapped_ak, login.keys, account.ak_epoch)
        .map_err(|failed| CliError::from(failed.error))?;
    session.set_sync_config(Some(&SyncConfig {
        kind: SyncKind::Hosted,
        server_url: Some(url.to_owned()),
        email: Some(email.to_owned()),
        folder_path: None,
        device_name: DEVICE_NAME.to_owned(),
    }))?;
    let id = session.account_id();
    app.be
        .tokens
        .save(id, &token)
        .map_err(|_| sync_failed("the session token could not be stored"))?;
    Ok(id)
}

/// `mosaic recover`: adopts a master password change made elsewhere (CRYPTO_SPEC §9.4, "Change
/// applied elsewhere"). The local store still has the old password and unlocks with it; the new
/// password signs in to the server and must open to this device's own account key. Every CLI
/// session is deleted on success.
pub fn recover(app: &mut App) -> Result<()> {
    let mut u = app.unlock()?;
    guard_rotation(&u.session)?;
    let config = u
        .session
        .sync_config()?
        .ok_or_else(|| CliError::new("syncNotConfigured", EXIT_OTHER, Msg::ErrSyncNotConfigured))?;
    let (SyncKind::Hosted, Some(url), Some(email)) =
        (config.kind, config.server_url.clone(), config.email.clone())
    else {
        return Err(CliError::not_supported("recover for this backend"));
    };
    let account = u.account;
    let mut secret_key = if let Some(key) = app.be.keys.load(account).ok().flatten() {
        key
    } else {
        typed_secret_key(app)?
    };
    let mut typed = app.prompt.secret(Msg::PromptNewPassword, &[])?;
    let password = MasterPassword::new(std::mem::take(&mut *typed));
    let install = u.session.install_id();
    let mut code: Option<String> = None;
    let mut asked_key = false;
    let recovered = loop {
        match hosted::recover_password(
            &url,
            &email,
            &password,
            &secret_key,
            install,
            &config.device_name,
            code.as_deref(),
        ) {
            Ok(done) => break done,
            Err(TransportError::TwoFactorRequired) if code.is_none() => {
                code = Some(two_factor_code(app)?);
            }
            // The newest Emergency Kit may hold a new Secret Key (a re-key made elsewhere).
            Err(TransportError::Rejected(Rejection::LoginFailed)) if !asked_key => {
                asked_key = true;
                secret_key = typed_secret_key(app)?;
            }
            Err(e) => return Err(transport_error(&e)),
        }
    };
    let params = *recovered.keys.params();
    u.session
        .adopt_remote_password(recovered.keys, &recovered.account.wrapped_ak, params)?;
    // The credentials changed: no session survives (review decision H1b).
    if let Ok(sessions) = &app.be.sessions {
        let _ = sessions.delete_all();
    }
    let _ = app
        .be
        .tokens
        .save(account, &StoredToken::from_session(&recovered.grant.token));
    let _ = app.be.keys.save(account, &secret_key);
    app.out
        .data(&serde_json::json!({ "recovered": true }), |o| {
            o.say(&o.t(Msg::OutRecovered, &[]));
        });
    Ok(())
}
