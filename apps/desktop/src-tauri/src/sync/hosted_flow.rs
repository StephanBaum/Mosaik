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
//! before it and starts again with the token the login granted. A token is
//! saved only once its session is complete (a login that still waits for its
//! second-factor code is held in memory, [`PendingLogin`]), and every flow
//! drops its result if the app locked while it talked to the network.

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
    CheckServerRequest, CodeRequest, DeviceRemoveRequest, DeviceRenameRequest,
    HostedConnectRequest, HostedMode, JoinHostedRequest, JoinOutcome, RegistrationMode,
    ServerCheck, SyncDevice, SyncDevices, SyncOverview, TotpBeginRequest, TotpDisableRequest,
    TotpSetupInfo,
};
use crate::persist::valid_email;
use crate::state::{Backend, Slot};

/// How long a login that waits for its second-factor code lives (the server
/// ends a pending session after five minutes).
const PENDING_TTL: Duration = Duration::from_mins(5);
/// Longest device name (the store's limit).
const MAX_DEVICE_NAME: usize = 64;
/// The longest server address and invite code the commands accept.
const MAX_SERVER_URL: usize = 2048;
const MAX_INVITE_CODE: usize = 128;

/// What a login that waits for its code is for.
pub(crate) enum PendingKind {
    /// `sync_hosted_connect`.
    Connect,
    /// `join_hosted`: the derived keys and the Secret Key wait with it.
    Join {
        keys: JoinKeys,
        secret_key: SecretKey,
    },
    /// A fresh login (device removal, second-factor setup) whose code was
    /// missing or wrong: the next call with a code finishes it.
    Fresh,
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

#[cfg(test)]
impl PendingLogin {
    /// A pending login that waits for a code (tests).
    pub(crate) fn for_tests(expires: Instant) -> Self {
        Self {
            url: "http://127.0.0.1:1".into(),
            email: "alex@example.com".into(),
            device_name: "Test".into(),
            account: None,
            token: mosaic_core::sync_auth::session_token(&[7u8; 32]).unwrap(),
            kind: PendingKind::Fresh,
            expires,
        }
    }
}

/// The stored hosted choice of the unlocked app.
pub(crate) struct HostedCtx {
    pub(super) account: AccountId,
    pub(super) install: InstallId,
    pub(super) url: String,
    pub(super) email: String,
    pub(super) device_name: String,
}

/// A fresh session of this device.
pub(super) struct Fresh {
    pub(super) url: String,
    pub(super) token: StoredToken,
    /// The code was spent on the login (a second one is needed for a call
    /// that checks a code of its own).
    pub(super) code_spent: bool,
}

pub(super) fn device_name(name: &str) -> IpcResult<String> {
    validate_text(name, MAX_DEVICE_NAME).map(str::to_owned)
}

fn canonical(url: &str) -> IpcResult<String> {
    if url.len() > MAX_SERVER_URL {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    canonical_origin(url.trim()).map_err(IpcError::from)
}

/// A second-factor code: six to eight ASCII digits, checked before any
/// network call.
pub(super) fn valid_code(code: &str) -> IpcResult<&str> {
    if (6..=8).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_digit()) {
        Ok(code)
    } else {
        Err(IpcError::new(ErrorCode::InvalidInput))
    }
}

fn secret_text(text: &str) -> SecretOut {
    SecretOut::new(SecretString::from(text.to_owned()))
}

/// The code of an optional field: absent or empty is `None`, anything else
/// must be a valid code.
pub(super) fn code_of(code: Option<&SecretIn>) -> IpcResult<Option<&str>> {
    code.map(SecretIn::expose_secret)
        .filter(|c| !c.is_empty())
        .map(valid_code)
        .transpose()
}

impl Backend {
    pub(crate) fn hosted_ctx(&self) -> IpcResult<HostedCtx> {
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

    pub(super) fn stored_token(&self, account: AccountId) -> IpcResult<StoredToken> {
        self.sync_tokens
            .load(account)?
            .ok_or(IpcError::new(ErrorCode::SyncSignedOut))
    }

    /// `sync_check_server`.
    ///
    /// Allowed only while unlocked, or while the app has no account yet (the
    /// onboarding): a locked app with an account would otherwise let the
    /// webview probe addresses.
    ///
    /// # Errors
    /// `locked`, `syncServerInvalid`, `syncServerInsecure`,
    /// `syncServerUnreachable`, `syncServerMismatch`, `syncServerIncompatible`.
    pub fn sync_check_server(&self, request: &CheckServerRequest) -> IpcResult<ServerCheck> {
        {
            let slot = self.slot()?;
            match &*slot {
                Slot::Unlocked(_) => {}
                Slot::Locked(locked) if !locked.has_account()? => {}
                Slot::Locked(_) => return Err(IpcError::LOCKED),
                Slot::Unavailable(code) => return Err(IpcError::new(*code)),
                Slot::Taken => return Err(IpcError::new(ErrorCode::Busy)),
            }
        }
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
        let invite = request
            .invite_code
            .as_ref()
            .map(SecretIn::expose_secret)
            .filter(|c| !c.is_empty());
        if invite.is_some_and(|c| c.len() > MAX_INVITE_CODE) {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        let password = master_password(request.password);
        if password.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        let epoch = self.sync.epoch();
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
                // The password was just verified against this device: a login
                // that fails now means the server holds another one.
                hosted::login(&url, &email, account, &opw, install, &device)
                    .map_err(|e| self.password_changed_error(e))?
            }
        };
        self.sync.check_epoch(epoch)?;
        if grant.status == LoginStatus::TwoFactorRequired {
            // Stored and the epoch checked again under the same lock: a lock
            // between the check above and the store drops the login.
            let kept = self.sync.set_pending_in(
                epoch,
                PendingLogin {
                    url,
                    email,
                    device_name: device,
                    account: Some(account),
                    token: grant.token,
                    kind: PendingKind::Connect,
                    expires: self.sync.now() + PENDING_TTL,
                },
            );
            return Err(IpcError::new(if kept {
                ErrorCode::SyncTwoFactorRequired
            } else {
                ErrorCode::Locked
            }));
        }
        self.finish_connect(epoch, account, &url, &email, &device, &grant.token)
    }

    /// The error for a failed hosted call after the local password was
    /// verified: a login the server refuses then means the server holds a
    /// master password this device does not know. The flag lets the settings
    /// offer [`Backend::sync_recover_password`].
    pub(super) fn password_changed_error(&self, error: TransportError) -> IpcError {
        if error == TransportError::Rejected(Rejection::LoginFailed) {
            self.sync.update_view(|view| {
                view.password_changed = true;
                true
            });
            self.emit_sync_state();
            IpcError::new(ErrorCode::SyncPasswordChanged)
        } else {
            error.into()
        }
    }

    /// The end of a connect: the token is kept, the choice is stored, the
    /// local data is queued for the backend and the thread starts. A device
    /// that signs in again to the same server keeps its sync state (cursors,
    /// checkpoints, observations, strikes, halt and findings): only the token
    /// is replaced. Another server starts from a clean sync state.
    fn finish_connect(
        &self,
        epoch: u64,
        account: AccountId,
        url: &str,
        email: &str,
        device: &str,
        token: &SessionToken,
    ) -> IpcResult<SyncOverview> {
        self.sync.check_epoch(epoch)?;
        let stored = StoredToken::from_session(token);
        self.sync.save_token(self, account, &stored)?;
        let stored_ok = self.with_unlocked(|session| {
            if session.account_id() != account {
                return Err(IpcError::LOCKED);
            }
            let same_server = session.sync_config()?.is_some_and(|c| {
                c.kind == SyncKind::Hosted && c.server_url.as_deref() == Some(url)
            });
            session.set_sync_config(Some(&SyncConfig {
                kind: SyncKind::Hosted,
                server_url: Some(url.to_owned()),
                email: Some(email.to_owned()),
                folder_path: None,
                device_name: device.to_owned(),
            }))?;
            if !same_server {
                session.reset_sync_state()?;
            }
            Ok(())
        });
        if let Err(error) = stored_ok {
            let _ = self.sync_tokens.delete(account);
            return Err(error);
        }
        self.sync.update_view(|view| {
            view.password_changed = false;
            true
        });
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
        let code = valid_code(request.code.expose_secret())?;
        let epoch = self.sync.epoch();
        let pending = self.take_waiting_login(false)?;
        let (Some(account), PendingKind::Connect) = (pending.account, &pending.kind) else {
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        };
        match hosted::login_totp(&pending.url, &pending.token, code) {
            Ok(_) => {}
            Err(error) => {
                if error == TransportError::Rejected(Rejection::TwoFactorInvalid) {
                    self.sync.set_pending_in(epoch, pending);
                }
                return Err(error.into());
            }
        }
        self.finish_connect(
            epoch,
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
        if pending.expires <= self.sync.now()
            || is_join != join
            || matches!(pending.kind, PendingKind::Fresh)
        {
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
        let epoch = self.sync.epoch();
        let login = hosted::join_login(&url, &email, &password, &secret_key, install, &device)?;
        drop(password);
        if login.grant.status == LoginStatus::TwoFactorRequired {
            let kept = self.sync.set_pending_in(
                epoch,
                PendingLogin {
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
                },
            );
            if !kept {
                return Err(IpcError::LOCKED);
            }
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
        let code = valid_code(request.code.expose_secret())?;
        let epoch = self.sync.epoch();
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
        if let Err(error) = hosted::login_totp(&url, &token, code) {
            if error == TransportError::Rejected(Rejection::TwoFactorInvalid) {
                self.sync.set_pending_in(
                    epoch,
                    PendingLogin {
                        url,
                        email,
                        device_name,
                        account,
                        token,
                        kind: PendingKind::Join { keys, secret_key },
                        expires,
                    },
                );
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
        let code = valid_code(request.code.expose_secret())?;
        let ctx = self.hosted_ctx()?;
        let token = self.stored_token(ctx.account)?;
        hosted::totp_confirm(&ctx.url, &token, code)?;
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
        let code = valid_code(request.code.expose_secret())?;
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
    /// account has one), `op` runs, and the thread starts again.
    ///
    /// The token the login grants is saved only once its session is complete:
    /// a login that waits for a code that was missing or wrong is held in
    /// memory ([`PendingKind::Fresh`]) and the next call with a code finishes
    /// it, without a second login. Whether the account has a second factor is
    /// read first when it is not known, so a missing code is refused before
    /// the login ends this device's session.
    fn with_fresh_hosted<R>(
        &self,
        password: &SecretIn,
        code: Option<&SecretIn>,
        reuse: bool,
        op: impl FnOnce(&Fresh) -> Result<R, TransportError>,
    ) -> IpcResult<R> {
        self.with_fresh_hosted_ipc(password, code, reuse, |fresh| {
            op(fresh).map_err(IpcError::from)
        })
    }

    /// [`Self::with_fresh_hosted`] for an operation that fails with an IPC
    /// error of its own (the sharing calls that need a fresh login).
    pub(super) fn with_fresh_hosted_ipc<R>(
        &self,
        password: &SecretIn,
        code: Option<&SecretIn>,
        reuse: bool,
        op: impl FnOnce(&Fresh) -> IpcResult<R>,
    ) -> IpcResult<R> {
        let ctx = self.hosted_ctx()?;
        let epoch = self.sync.epoch();
        let mp = MasterPassword::new(password.expose_secret().to_owned());
        if mp.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        self.verify_master_only(&mp)?;
        let code = code_of(code)?;
        let reuse = reuse && self.sync.fresh_recently();
        if !reuse && code.is_none() && self.totp_enabled_of(&ctx) {
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
        let result = self
            .fresh_session(&ctx, epoch, opw.as_ref(), code)
            .and_then(|fresh| op(&fresh));
        self.sync_restart();
        result
    }

    /// Whether the sync account has a second factor: the last known answer,
    /// else the server's (with the stored token), else "no".
    pub(super) fn totp_enabled_of(&self, ctx: &HostedCtx) -> bool {
        if let Some(known) = self.sync.view().totp_enabled {
            return known;
        }
        let asked = self
            .sync_tokens
            .load(ctx.account)
            .ok()
            .flatten()
            .and_then(|token| hosted::account_info(&ctx.url, &token).ok());
        asked.is_some_and(|info| {
            self.sync.update_view(|view| {
                view.totp_enabled = Some(info.totp_enabled);
                true
            });
            info.totp_enabled
        })
    }

    /// The session of a fresh call: the stored one (`opw` is `None`), or a
    /// new login that is completed with `code` when the account asks for one
    /// and saved to the keystore only then.
    fn fresh_session(
        &self,
        ctx: &HostedCtx,
        epoch: u64,
        opw: Option<&OpaquePassword>,
        code: Option<&str>,
    ) -> IpcResult<Fresh> {
        let Some(opw) = opw else {
            let token = self
                .sync_tokens
                .load(ctx.account)
                .ok()
                .flatten()
                .ok_or(TransportError::Unauthorized)?;
            return Ok(Fresh {
                url: ctx.url.clone(),
                token,
                code_spent: false,
            });
        };
        // A login that waits for its code from an earlier call, or a new one.
        let (token, needs_code) = if let Some(token) =
            code.and_then(|_| self.sync.take_fresh_pending(&ctx.url, ctx.account))
        {
            (token, true)
        } else {
            let grant = hosted::login(
                &ctx.url,
                &ctx.email,
                ctx.account,
                opw,
                ctx.install,
                &ctx.device_name,
            )
            .map_err(|e| self.password_changed_error(e))?;
            (grant.token, grant.status == LoginStatus::TwoFactorRequired)
        };
        if needs_code {
            let Some(code) = code else {
                self.hold_fresh(ctx, epoch, token);
                return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
            };
            if let Err(error) = hosted::login_totp(&ctx.url, &token, code) {
                if error == TransportError::Rejected(Rejection::TwoFactorInvalid) {
                    self.hold_fresh(ctx, epoch, token);
                }
                return Err(error.into());
            }
        }
        // The session is complete: only now does the keystore change. A lock
        // meanwhile drops the result and writes nothing.
        self.sync.check_epoch(epoch)?;
        let stored = StoredToken::from_session(&token);
        self.sync.save_token(self, ctx.account, &stored)?;
        self.sync.note_fresh();
        Ok(Fresh {
            url: ctx.url.clone(),
            token: stored,
            code_spent: needs_code,
        })
    }

    /// Keeps a login that waits for a code, for the next call.
    fn hold_fresh(&self, ctx: &HostedCtx, epoch: u64, token: SessionToken) {
        // After a lock the token is dropped, not kept.
        self.sync.set_pending_in(
            epoch,
            PendingLogin {
                url: ctx.url.clone(),
                email: ctx.email.clone(),
                device_name: ctx.device_name.clone(),
                account: Some(ctx.account),
                token,
                kind: PendingKind::Fresh,
                expires: self.sync.now() + PENDING_TTL,
            },
        );
    }

    // ---- master password -----------------------------------------------------------

    /// The hosted half of `change_master_password`: the server first (a fresh
    /// login with the old password, the second factor if the account has
    /// one, the new registration and the re-wrapped account key), the device
    /// afterwards. An unreachable server refuses the change
    /// (`syncPasswordNeedsServer`) as long as the change was not sent: nothing
    /// changed anywhere. Once it was sent, a lost answer is checked with the
    /// new password at once; if that fails too the error is
    /// `syncPasswordOutcomeUnknown` and `passwordChanged` is set
    /// (`sync_recover_password` settles it).
    ///
    /// The local commit runs at once after the server accepted. The token the
    /// change brought is kept after it and is not fatal: if the keystore
    /// refuses it, the change stands and the device is signed out of sync (it
    /// signs in again). If the commit itself fails (the app locked past the
    /// three-minute deferral), the server holds the new password and the
    /// error is `syncPasswordChanged`: `sync_recover_password` finishes it.
    pub(crate) fn change_password_hosted(
        &self,
        ctx: &HostedCtx,
        code: Option<&SecretIn>,
        prepared: mosaic_vault::session::PreparedPasswordChange,
    ) -> IpcResult<mosaic_vault::session::PasswordChanged> {
        let code = code_of(code)?;
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
                // The device commits first; the change revoked every other
                // session of the account.
                let committed = self
                    .with_unlocked(|session| Ok(session.commit_master_password_change(prepared)?))
                    .map_err(|_| IpcError::new(ErrorCode::SyncPasswordChanged));
                if committed.is_ok() {
                    let token = StoredToken::from_session(&changed.grant.token);
                    if self.sync.save_token(self, ctx.account, &token).is_err() {
                        // The token the keystore refused is not the old one
                        // either (the change ended that session): signed out.
                        let _ = self.sync_tokens.delete(ctx.account);
                        self.sync_force_signed_out();
                    }
                } else {
                    // The server holds the new password and this device the
                    // old one (the app locked past the cap): the new token is
                    // not kept, the old one stays (the change revoked it, so
                    // the device is signed out), and the settings offer the
                    // recovery with the new password.
                    self.sync.update_view(|view| {
                        view.password_changed = true;
                        true
                    });
                    self.emit_sync_state();
                }
                committed
            }
            Err(TransportError::Unreachable | TransportError::Timeout) => {
                Err(IpcError::new(ErrorCode::SyncPasswordNeedsServer))
            }
            Err(TransportError::OutcomeUnknown) => {
                // The server may hold the new password and this device the
                // old one: both stay, the settings offer the recovery.
                self.sync.update_view(|view| {
                    view.password_changed = true;
                    true
                });
                self.emit_sync_state();
                Err(IpcError::new(ErrorCode::SyncPasswordOutcomeUnknown))
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
