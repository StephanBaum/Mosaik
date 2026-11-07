//! The commands for the Mosaic server: check, register or sign in, the
//! second factor at login, join from a new device, the devices list, the
//! sync account's own second factor and the master password change.
//!
//! The master password and the Secret Key stay in zeroizing types
//! (`MasterPassword`, `SecretKey`, `OpaquePassword`); only OPAQUE messages and
//! the session token leave the process. The token is kept in the keystore
//! ([`crate::keystore::SyncTokenStore`]). The server allows one session per
//! device: every fresh login (second factor, device removal, password
//! change) ends this device's running session, so the sync thread stops
//! before it and starts again with the token the login granted.

use std::time::{Duration, Instant};

use mosaic_core::{
    AccountId, InstallId, MasterPassword, OpaquePassword, SecretKey, SecretString, SessionToken,
};
use mosaic_sync::http::{HttpTransport, StoredToken, canonical_origin, hosted};
use mosaic_sync::transport::{Rejection, Transport, TransportError};
use mosaic_sync::wire::json::{LoginStatus, RegistrationMode as WireRegistration};
use mosaic_vault::session::{JoinKeys, SyncConfig, SyncKind};

use crate::commands::session::{master_password, take_locked, validate_text};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::SyncStatus;
use crate::contract::secret::{SecretIn, SecretOut};
use crate::contract::sync::{
    ChangePasswordRequest, CheckServerRequest, CodeRequest, DeviceRemoveRequest,
    DeviceRenameRequest, HostedConnectRequest, HostedMode, JoinHostedRequest, JoinOutcome,
    RegistrationMode, ServerCheck, SyncDevice, SyncDevices, SyncOverview, TotpBeginRequest,
    TotpDisableRequest, TotpSetupInfo,
};
use crate::persist::valid_email;
use crate::state::{Backend, Slot};

/// How long a login that waits for its second-factor code lives (the server
/// ends a pending session after five minutes).
const PENDING_TTL: Duration = Duration::from_mins(5);
/// Longest device name (the store's limit).
const MAX_DEVICE_NAME: usize = 64;

/// What a login that waits for its code is for.
pub(crate) enum PendingKind {
    /// `sync_hosted_connect`.
    Connect,
    /// `join_hosted`: the derived keys and the Secret Key wait with it.
    Join {
        keys: JoinKeys,
        secret_key: SecretKey,
    },
}

/// A hosted login that waits for its second-factor code.
pub(crate) struct PendingLogin {
    pub(crate) url: String,
    pub(crate) email: String,
    pub(crate) device_name: String,
    pub(crate) account: Option<AccountId>,
    pub(crate) token: SessionToken,
    pub(crate) kind: PendingKind,
    pub(crate) expires: Instant,
}

/// The stored hosted choice of the unlocked app.
pub(super) struct HostedCtx {
    pub(super) account: AccountId,
    pub(super) install: InstallId,
    pub(super) url: String,
    pub(super) email: String,
    pub(super) device_name: String,
}

/// A fresh session of this device.
struct Fresh {
    url: String,
    token: StoredToken,
    /// The code was spent on the login (a second one is needed for a call
    /// that checks a code of its own).
    code_spent: bool,
}

pub(super) fn device_name(name: &str) -> IpcResult<String> {
    validate_text(name, MAX_DEVICE_NAME).map(str::to_owned)
}

fn canonical(url: &str) -> IpcResult<String> {
    canonical_origin(url.trim()).map_err(IpcError::from)
}

fn secret_text(text: &str) -> SecretOut {
    SecretOut::new(SecretString::from(text.to_owned()))
}

fn code_of(code: Option<&SecretIn>) -> Option<&str> {
    code.map(SecretIn::expose_secret).filter(|c| !c.is_empty())
}

impl Backend {
    pub(super) fn hosted_ctx(&self) -> IpcResult<HostedCtx> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let config = session
            .sync_config()?
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        match (config.kind, config.server_url) {
            (SyncKind::Hosted, Some(url)) => Ok(HostedCtx {
                account: session.account_id(),
                install: session.install_id(),
                url,
                email: config.email.unwrap_or_default(),
                device_name: config.device_name,
            }),
            _ => Err(IpcError::new(ErrorCode::SyncNotConfigured)),
        }
    }

    fn stored_token(&self, account: AccountId) -> IpcResult<StoredToken> {
        self.sync_tokens
            .load(account)?
            .ok_or(IpcError::new(ErrorCode::SyncSignedOut))
    }

    /// `sync_check_server`.
    ///
    /// # Errors
    /// `syncServerInvalid`, `syncServerInsecure`, `syncServerUnreachable`,
    /// `syncServerMismatch`, `syncServerIncompatible`.
    pub fn sync_check_server(&self, request: &CheckServerRequest) -> IpcResult<ServerCheck> {
        let url = canonical(&request.server_url)?;
        let info = HttpTransport::info(&url)?;
        Ok(ServerCheck {
            server_url: url,
            registration: match info.registration {
                WireRegistration::Open => RegistrationMode::Open,
                WireRegistration::Invite => RegistrationMode::Invite,
                WireRegistration::Closed => RegistrationMode::Closed,
            },
        })
    }

    /// `sync_hosted_connect`: registers the sync account (or signs in to
    /// it) and starts syncing. An account with a second factor answers
    /// `syncTwoFactorRequired`; `sync_hosted_two_factor` finishes.
    ///
    /// # Errors
    /// `locked`, `syncAlreadyConfigured`, `invalidInput`, `emptyPassword`,
    /// `wrongCredentials`, `secretKeyRequired`, `syncAccountMismatch`,
    /// `syncTwoFactorRequired` and the transport's `sync*` codes.
    pub fn sync_hosted_connect(&self, request: HostedConnectRequest) -> IpcResult<SyncOverview> {
        let device = device_name(&request.device_name)?;
        let url = canonical(&request.server_url)?;
        let password = master_password(request.password);
        if password.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        // Counts wrong passwords towards the five-failure lock, like every
        // check of the master password.
        self.verify_master_only(&password)?;
        let (account, install, bootstrap, opw, email) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            // A device the server signed out may sign in again; any other
            // configured device disconnects first.
            if session.sync_config()?.is_some() && self.sync.view().status != SyncStatus::SignedOut
            {
                return Err(IpcError::new(ErrorCode::SyncAlreadyConfigured));
            }
            let email = self
                .account()
                .and_then(|a| a.email)
                .ok_or(IpcError::new(ErrorCode::InvalidInput))?;
            let key = self
                .secret_key_of(session.account_id())
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
            (
                session.account_id(),
                session.install_id(),
                session.account_bootstrap()?,
                session.opaque_password(&password, &key)?,
                email,
            )
        };
        drop(password);
        let invite = request
            .invite_code
            .as_ref()
            .map(SecretIn::expose_secret)
            .filter(|c| !c.is_empty());
        let grant = match request.mode {
            HostedMode::Register => {
                hosted::register(&url, &email, invite, &bootstrap, &opw, install, &device)?;
                hosted::login(&url, &email, account, &opw, install, &device)?
            }
            HostedMode::SignIn => {
                let params = hosted::prelogin(&url, &email)?;
                if params.account_id != account {
                    return Err(IpcError::new(ErrorCode::SyncAccountMismatch));
                }
                hosted::login(&url, &email, account, &opw, install, &device)?
            }
        };
        if grant.status == LoginStatus::TwoFactorRequired {
            self.sync.set_pending(Some(PendingLogin {
                url,
                email,
                device_name: device,
                account: Some(account),
                token: grant.token,
                kind: PendingKind::Connect,
                expires: self.sync.now() + PENDING_TTL,
            }));
            return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
        }
        self.finish_connect(account, &url, &email, &device, &grant.token)
    }

    /// The end of a connect: the token is kept, the choice is stored, the
    /// local data is queued for the backend and the thread starts.
    fn finish_connect(
        &self,
        account: AccountId,
        url: &str,
        email: &str,
        device: &str,
        token: &SessionToken,
    ) -> IpcResult<SyncOverview> {
        let stored = StoredToken::from_session(token);
        self.sync_tokens.save(account, &stored)?;
        let stored_ok = self.with_unlocked(|session| {
            if session.account_id() != account {
                return Err(IpcError::LOCKED);
            }
            session.set_sync_config(Some(&SyncConfig {
                kind: SyncKind::Hosted,
                server_url: Some(url.to_owned()),
                email: Some(email.to_owned()),
                folder_path: None,
                device_name: device.to_owned(),
            }))?;
            session.reset_sync_state()?;
            Ok(())
        });
        if let Err(error) = stored_ok {
            let _ = self.sync_tokens.delete(account);
            return Err(error);
        }
        // Best effort: whether the account has a second factor.
        if let Ok(info) = hosted::account_info(url, &stored) {
            self.sync.update_view(|view| {
                view.totp_enabled = Some(info.totp_enabled);
                true
            });
        }
        self.sync_restart();
        self.sync_status()
    }

    /// `sync_hosted_two_factor`.
    ///
    /// # Errors
    /// `syncSignedOut` when no login waits (or it ran out),
    /// `syncTwoFactorInvalid` (the login keeps waiting), and the
    /// transport's `sync*` codes.
    pub fn sync_hosted_two_factor(&self, request: &CodeRequest) -> IpcResult<SyncOverview> {
        let pending = self.take_waiting_login(false)?;
        let (Some(account), PendingKind::Connect) = (pending.account, &pending.kind) else {
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        };
        match hosted::login_totp(&pending.url, &pending.token, request.code.expose_secret()) {
            Ok(_) => {}
            Err(error) => {
                if error == TransportError::Rejected(Rejection::TwoFactorInvalid) {
                    self.sync.set_pending(Some(pending));
                }
                return Err(error.into());
            }
        }
        self.finish_connect(
            account,
            &pending.url,
            &pending.email,
            &pending.device_name,
            &pending.token,
        )
    }

    /// The waiting login, if it is still valid.
    fn take_waiting_login(&self, join: bool) -> IpcResult<PendingLogin> {
        let pending = self
            .sync
            .take_pending()
            .ok_or(IpcError::new(ErrorCode::SyncSignedOut))?;
        let is_join = matches!(pending.kind, PendingKind::Join { .. });
        if pending.expires <= self.sync.now() || is_join != join {
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        }
        Ok(pending)
    }

    // ---- join -------------------------------------------------------------------

    /// What a join needs from the app before it talks to anything: this
    /// install's ID, and an app with no account yet.
    pub(super) fn join_prerequisites(&self) -> IpcResult<InstallId> {
        let slot = self.slot()?;
        match &*slot {
            Slot::Locked(locked) => {
                if locked.has_account()? {
                    return Err(IpcError::new(ErrorCode::AccountExists));
                }
                Ok(locked.store().install_id()?)
            }
            Slot::Unlocked(_) => Err(IpcError::new(ErrorCode::AlreadyUnlocked)),
            Slot::Unavailable(code) => Err(IpcError::new(*code)),
            Slot::Taken => Err(IpcError::new(ErrorCode::Busy)),
        }
    }

    /// `join_hosted`: joins the sync account of a server from a new device.
    /// An account with a second factor answers `twoFactorRequired`;
    /// `join_hosted_two_factor` finishes.
    ///
    /// # Errors
    /// `accountExists`, `alreadyUnlocked`, `invalidInput`, `emptyPassword`,
    /// `secretKeyMalformed`, `secretKeyMistyped`, `wrongCredentials` and the
    /// transport's `sync*` codes.
    pub fn join_hosted(&self, request: JoinHostedRequest) -> IpcResult<JoinOutcome> {
        let email = request.email.trim().to_owned();
        if !valid_email(&email) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let device = device_name(&request.device_name)?;
        let url = canonical(&request.server_url)?;
        let secret_key = SecretKey::parse(request.secret_key.expose_secret())?;
        let password = master_password(request.password);
        if password.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        let install = self.join_prerequisites()?;
        let login = hosted::join_login(&url, &email, &password, &secret_key, install, &device)?;
        drop(password);
        if login.grant.status == LoginStatus::TwoFactorRequired {
            self.sync.set_pending(Some(PendingLogin {
                url,
                email,
                device_name: device,
                account: None,
                token: login.grant.token,
                kind: PendingKind::Join {
                    keys: login.keys,
                    secret_key,
                },
                expires: self.sync.now() + PENDING_TTL,
            }));
            return Ok(JoinOutcome::TwoFactorRequired);
        }
        self.finish_join(
            &url,
            &email,
            &device,
            &login.grant.token,
            login.keys,
            &secret_key,
        )
    }

    /// `join_hosted_two_factor`.
    ///
    /// # Errors
    /// `syncSignedOut`, `syncTwoFactorInvalid` (the login keeps waiting), and
    /// the errors of `join_hosted`.
    pub fn join_hosted_two_factor(&self, request: &CodeRequest) -> IpcResult<JoinOutcome> {
        let pending = self.take_waiting_login(true)?;
        let PendingLogin {
            url,
            email,
            device_name,
            account,
            token,
            kind,
            expires,
        } = pending;
        let PendingKind::Join { keys, secret_key } = kind else {
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        };
        if let Err(error) = hosted::login_totp(&url, &token, request.code.expose_secret()) {
            if error == TransportError::Rejected(Rejection::TwoFactorInvalid) {
                self.sync.set_pending(Some(PendingLogin {
                    url,
                    email,
                    device_name,
                    account,
                    token,
                    kind: PendingKind::Join { keys, secret_key },
                    expires,
                }));
            }
            return Err(error.into());
        }
        self.finish_join(&url, &email, &device_name, &token, keys, &secret_key)
    }

    /// The end of a hosted join: the wrapped account key is fetched and
    /// opened, the account is created in the (new) store, and the device
    /// is set up like one that had the account all along: the Secret Key in
    /// the keystore, the token, the backend choice, the settings account
    /// block (kit verified: the Secret Key was typed from the kit) and an
    /// unlocked session. There is no `reset_sync_state`: the store is new.
    fn finish_join(
        &self,
        url: &str,
        email: &str,
        device: &str,
        token: &SessionToken,
        keys: JoinKeys,
        secret_key: &SecretKey,
    ) -> IpcResult<JoinOutcome> {
        let account = hosted::join_fetch_account(url, token, &keys)?;
        let stored = StoredToken::from_session(token);
        let config = SyncConfig {
            kind: SyncKind::Hosted,
            server_url: Some(url.to_owned()),
            email: Some(email.to_owned()),
            folder_path: None,
            device_name: device.to_owned(),
        };
        let mut slot = self.slot()?;
        let locked = take_locked(&mut slot)?;
        match locked.join_account_with_keys(&account.wrapped_ak, keys) {
            Ok(session) => {
                self.complete_join(
                    slot,
                    session,
                    secret_key,
                    Some(email),
                    &config,
                    Some(&stored),
                )?;
                self.sync.update_view(|view| {
                    view.totp_enabled = Some(account.totp_enabled);
                    true
                });
                Ok(JoinOutcome::Unlocked)
            }
            Err(failed) => {
                *slot = Slot::Locked(failed.session);
                Err(failed.error.into())
            }
        }
    }

    // ---- devices ----------------------------------------------------------------

    /// `sync_devices`.
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, `syncSignedOut` and the transport's
    /// `sync*` codes.
    pub fn sync_devices(&self) -> IpcResult<SyncDevices> {
        let (config, account, install) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            (
                session
                    .sync_config()?
                    .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?,
                session.account_id(),
                session.install_id(),
            )
        };
        let hosted_kind = config.kind == SyncKind::Hosted;
        let remote = if let Some(url) = config.server_url.as_deref().filter(|_| hosted_kind) {
            let token = self.stored_token(account)?;
            HttpTransport::connect(url, &token, install)?.devices()?
        } else {
            self.folder_devices(&config, account, install)?
        };
        let platform = own_platform();
        let mut devices: Vec<SyncDevice> = remote
            .into_iter()
            .map(|d| {
                let current = d.device_id == install;
                SyncDevice {
                    device_id: d.device_id.into(),
                    name: d.name,
                    platform: current.then_some(platform),
                    current,
                    last_seen_at: d
                        .last_seen_ms
                        .map(|ms| i64::try_from(ms).unwrap_or(i64::MAX)),
                    can_remove: hosted_kind && !current,
                }
            })
            .collect();
        devices.sort_by(|a, b| {
            b.current
                .cmp(&a.current)
                .then(b.last_seen_at.cmp(&a.last_seen_at))
        });
        Ok(SyncDevices { devices })
    }

    /// `sync_device_rename`: hosted renames any device of the account; a
    /// folder renames this device only (it rewrites its own `info.mdev`).
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, `invalidInput`, and the transport's
    /// `sync*` codes.
    pub fn sync_device_rename(&self, request: &DeviceRenameRequest) -> IpcResult<()> {
        let name = device_name(&request.name)?;
        let (config, account, install) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            (
                session
                    .sync_config()?
                    .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?,
                session.account_id(),
                session.install_id(),
            )
        };
        let id = request.device_id.get();
        match (config.kind, config.server_url.as_deref()) {
            (SyncKind::Hosted, Some(url)) => {
                let token = self.stored_token(account)?;
                hosted::device_rename(url, &token, id, &name)?;
            }
            _ if id != install => return Err(IpcError::new(ErrorCode::InvalidInput)),
            _ => {}
        }
        if id == install {
            let renamed = SyncConfig {
                device_name: name,
                ..config
            };
            self.with_unlocked(|session| Ok(session.set_sync_config(Some(&renamed))?))?;
            // The folder's `info.mdev` is rewritten when the transport opens.
            self.sync_restart();
        }
        Ok(())
    }

    /// `sync_device_remove` (hosted): a fresh login with the master password,
    /// then the server ends the device's sessions and refuses its logins.
    ///
    /// # Errors
    /// `syncDeviceIsCurrent`, `wrongCredentials`, `syncTwoFactorRequired`
    /// (the account has a second factor and no code was given),
    /// `syncTwoFactorInvalid`, and the transport's `sync*` codes.
    pub fn sync_device_remove(&self, request: &DeviceRemoveRequest) -> IpcResult<()> {
        let ctx = self.hosted_ctx()?;
        let id = request.device_id.get();
        if id == ctx.install {
            return Err(IpcError::new(ErrorCode::SyncDeviceIsCurrent));
        }
        self.with_fresh_hosted(&request.password, request.code.as_ref(), false, |fresh| {
            hosted::device_remove(&fresh.url, &fresh.token, id)
        })
    }

    // ---- the sync account's second factor ----------------------------------------

    /// `sync_totp_begin`: a fresh login, then the server's pending secret
    /// (the fifth secret response, DESKTOP_IPC §6). The session stays fresh
    /// for five minutes for `sync_totp_confirm`.
    ///
    /// # Errors
    /// `invalidInput` when a second factor is on already; the codes of a
    /// fresh login.
    pub fn sync_totp_begin(&self, request: &TotpBeginRequest) -> IpcResult<TotpSetupInfo> {
        if self.sync.view().totp_enabled == Some(true) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let setup = self.with_fresh_hosted(&request.password, None, false, |fresh| {
            hosted::totp_setup(&fresh.url, &fresh.token)
        })?;
        Ok(TotpSetupInfo {
            secret: secret_text(setup.secret.expose_secret()),
            otpauth_uri: secret_text(setup.otpauth_uri.expose_secret()),
        })
    }

    /// `sync_totp_confirm`: turns the second factor on with the first code.
    ///
    /// # Errors
    /// `syncTwoFactorInvalid`, `syncFreshLoginExpired` (more than five
    /// minutes since `sync_totp_begin`), and the transport's `sync*` codes.
    pub fn sync_totp_confirm(&self, request: &CodeRequest) -> IpcResult<()> {
        let ctx = self.hosted_ctx()?;
        let token = self.stored_token(ctx.account)?;
        hosted::totp_confirm(&ctx.url, &token, request.code.expose_secret())?;
        self.sync.update_view(|view| {
            view.totp_enabled = Some(true);
            true
        });
        Ok(())
    }

    /// `sync_totp_disable`: a fresh login (with a code), then the server
    /// checks a code of its own. A server accepts a code once, so the second
    /// check needs the next code: the first attempt then answers
    /// `syncTwoFactorRequired` (the login stays fresh for four minutes) and
    /// the UI asks for the next code.
    ///
    /// # Errors
    /// `syncTwoFactorRequired`, `syncTwoFactorInvalid`, `wrongCredentials`, and
    /// the transport's `sync*` codes.
    pub fn sync_totp_disable(&self, request: &TotpDisableRequest) -> IpcResult<()> {
        let code = request.code.expose_secret();
        self.with_fresh_hosted(&request.password, Some(&request.code), true, |fresh| {
            match hosted::totp_disable(&fresh.url, &fresh.token, code) {
                Err(TransportError::Rejected(Rejection::TwoFactorInvalid)) if fresh.code_spent => {
                    Err(TransportError::TwoFactorRequired)
                }
                other => other,
            }
        })?;
        self.sync.update_view(|view| {
            view.totp_enabled = Some(false);
            true
        });
        Ok(())
    }

    /// A call that needs a fresh login of this device. The master password
    /// is checked first (and counted), the sync thread stops (the login ends
    /// its session), the login runs (with the second-factor code when the
    /// account has one), the token it grants is kept at once, `op` runs, and
    /// the thread starts again with the token in the keystore.
    fn with_fresh_hosted<R>(
        &self,
        password: &SecretIn,
        code: Option<&SecretIn>,
        reuse: bool,
        op: impl FnOnce(&Fresh) -> Result<R, TransportError>,
    ) -> IpcResult<R> {
        let ctx = self.hosted_ctx()?;
        let mp = MasterPassword::new(password.expose_secret().to_owned());
        if mp.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        self.verify_master_only(&mp)?;
        let code = code_of(code);
        let reuse = reuse && self.sync.fresh_recently();
        if !reuse && self.sync.view().totp_enabled == Some(true) && code.is_none() {
            return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
        }
        let opw: Option<OpaquePassword> = if reuse {
            None
        } else {
            let key = self
                .secret_key_of(ctx.account)
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
            Some(self.with_unlocked(|session| Ok(session.opaque_password(&mp, &key)?))?)
        };
        drop(mp);
        self.sync.stop();
        let result = (|| {
            let (token, code_spent) = match &opw {
                None => (
                    self.sync_tokens
                        .load(ctx.account)
                        .ok()
                        .flatten()
                        .ok_or(TransportError::Unauthorized)?,
                    false,
                ),
                Some(opw) => {
                    let grant = hosted::login(
                        &ctx.url,
                        &ctx.email,
                        ctx.account,
                        opw,
                        ctx.install,
                        &ctx.device_name,
                    )?;
                    // The login ended this device's older session: keep the
                    // new one before anything else can fail.
                    let token = StoredToken::from_session(&grant.token);
                    let _ = self.sync_tokens.save(ctx.account, &token);
                    let mut spent = false;
                    if grant.status == LoginStatus::TwoFactorRequired {
                        let code = code.ok_or(TransportError::TwoFactorRequired)?;
                        hosted::login_totp(&ctx.url, &grant.token, code)?;
                        spent = true;
                    }
                    self.sync.note_fresh();
                    (token, spent)
                }
            };
            op(&Fresh {
                url: ctx.url.clone(),
                token,
                code_spent,
            })
        })();
        self.sync_restart();
        result.map_err(IpcError::from)
    }

    // ---- master password -----------------------------------------------------------

    /// The hosted half of `change_master_password`: the server first (a fresh
    /// login with the old password, the second factor if the account has
    /// one, the new registration and the re-wrapped account key), the device
    /// afterwards. An unreachable server refuses the change
    /// (`syncPasswordNeedsServer`): nothing changed anywhere.
    pub(super) fn change_password_hosted(
        &self,
        ctx: &HostedCtx,
        request: &ChangePasswordRequest,
        prepared: mosaic_vault::session::PreparedPasswordChange,
    ) -> IpcResult<mosaic_vault::session::PasswordChanged> {
        let code = code_of(request.code.as_ref());
        if self.sync.view().totp_enabled == Some(true) && code.is_none() {
            return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
        }
        self.sync.stop();
        let result = hosted::password_change(
            &ctx.url,
            &ctx.email,
            &prepared,
            ctx.install,
            &ctx.device_name,
            code,
        );
        let outcome = match result {
            Ok(changed) => {
                // The change revoked every other session of the account; this
                // device keeps the fresh one it got back.
                let token = StoredToken::from_session(&changed.grant.token);
                self.sync_tokens.save(ctx.account, &token)?;
                self.with_unlocked(|session| Ok(session.commit_master_password_change(prepared)?))
            }
            Err(TransportError::Unreachable | TransportError::Timeout) => {
                Err(IpcError::new(ErrorCode::SyncPasswordNeedsServer))
            }
            Err(error) => Err(error.into()),
        };
        self.sync_restart();
        outcome
    }
}

/// This device's platform, for the device list.
fn own_platform() -> crate::contract::sync::DevicePlatform {
    use crate::contract::sync::DevicePlatform;
    match std::env::consts::OS {
        "windows" => DevicePlatform::Windows,
        "macos" => DevicePlatform::Macos,
        _ => DevicePlatform::Linux,
    }
}
