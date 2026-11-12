//! Security keys for the hosted sync account (M9, ADR-0077 to ADR-0079): the
//! second-factor step at connect and join, and the key list in the settings.
//!
//! The WebAuthn ceremony cannot run in the webview (its origin is
//! `tauri://localhost`, not the server's relying-party origin), so it runs in
//! the user's system browser on a page the server serves. Rust asks the server
//! for a handoff, opens `<canonical origin>/webauthn/#<handoffId>` and shows the
//! user the six-digit match code to type on that page; the webview polls the
//! status command and never sees the handoff ID.
//!
//! **The browser is opened from one place only.** [`Backend::open_handoff`]
//! builds the URL from the stored canonical origin and a handoff ID the server
//! returned ([`hosted::handoff_page_url`]) and hands it to the [`UrlOpener`].
//! No request type carries a URL, and the webview has no opener permission
//! (`redteam_surface_tests`, `opener_has_one_call_site`).
//!
//! Login handoffs live in the waiting login ([`PendingLogin`]), so a lock, an
//! expiry and the next sign-in drop them with it. The account handoffs (add a
//! key, confirm a second factor for removals) live in one [`AccountFlow`] on the
//! sync service, with the same drops.

use core::fmt;
use std::time::Instant;

use mosaic_core::{AccountId, MasterPassword, OpaquePassword, SessionToken};
use mosaic_sync::http::{StoredToken, hosted};
use mosaic_sync::transport::{Rejection, TransportError};
use mosaic_sync::wire::json::{
    HandoffPurpose, HandoffStartResponse, HandoffState as WireState, LoginStatus,
};

use super::SyncService;
use super::hosted_flow::{HostedCtx, PENDING_TTL, PendingKind, PendingLogin, code_of, device_name};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::ids::UnixMillis;
use crate::contract::secret::SecretIn;
use crate::contract::sync::{
    HandoffState, HandoffStep, LoginMethods, WebauthnAccountBegin, WebauthnAccountStatus,
    WebauthnAddRequest, WebauthnBegin, WebauthnConfirmRequest, WebauthnKeyView, WebauthnKeys,
    WebauthnRemoveRequest, WebauthnStatus,
};
use crate::state::Backend;

/// Opens a page in the system browser.
pub(crate) trait UrlOpener: Send + Sync {
    /// Whether the system accepted the request. The caller builds the URL
    /// ([`Backend::open_handoff`]); an opener never takes one from the webview.
    fn open(&self, url: &str) -> bool;
}

/// The system's default browser.
pub(crate) struct SystemOpener;

impl UrlOpener for SystemOpener {
    fn open(&self, url: &str) -> bool {
        open::that_detached(url).is_ok()
    }
}

/// Opens nothing (a backend without a window, and tests that do not care).
pub(crate) struct NoOpener;

impl UrlOpener for NoOpener {
    fn open(&self, _url: &str) -> bool {
        false
    }
}

/// A handoff ID. It only leaves Rust inside the URL of the browser page.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandoffId([u8; 16]);

impl fmt::Debug for HandoffId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HandoffId(..)")
    }
}

/// A running handoff.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ActiveHandoff {
    id: HandoffId,
    expires: Instant,
}

/// What an account handoff is for.
pub(crate) enum AccountKind {
    /// Registers a security key with this name.
    Register { label: String },
    /// Proves a second factor for the next five minutes.
    Confirm,
}

/// The browser step of adding a key or confirming a second factor.
pub(crate) struct AccountFlow {
    kind: AccountKind,
    step: HandoffStep,
    url: String,
    account: AccountId,
    /// While `step` is `Login`: the session that waits for its security key.
    login: Option<SessionToken>,
    handoff: ActiveHandoff,
}

impl AccountFlow {
    /// Whether the running handoff ran out.
    pub(super) fn expired(&self, now: Instant) -> bool {
        self.handoff.expires <= now
    }
}

impl SyncService {
    fn set_account_flow_in(&self, epoch: u64, flow: AccountFlow) -> bool {
        let Ok(mut slot) = self.account_flow.lock() else {
            return false;
        };
        *slot = Some(flow);
        if self.epoch() == epoch {
            true
        } else {
            *slot = None;
            false
        }
    }

    fn take_account_flow(&self) -> Option<AccountFlow> {
        self.account_flow.lock().ok().and_then(|mut f| f.take())
    }

    /// The page of the running handoff (login or account), if one still runs.
    fn active_handoff(&self) -> Option<(String, HandoffId)> {
        let now = self.now();
        if let Ok(pending) = self.pending_login.lock()
            && let Some(p) = pending.as_ref()
            && p.expires > now
            && let Some(h) = p.handoff.filter(|h| h.expires > now)
        {
            return Some((p.url.clone(), h.id));
        }
        let flow = self.account_flow.lock().ok()?;
        let f = flow.as_ref().filter(|f| !f.expired(now))?;
        Some((f.url.clone(), f.handoff.id))
    }

    /// The status call of a login handoff (tests may answer it themselves).
    #[cfg_attr(
        not(test),
        expect(clippy::unused_self, reason = "tests read an override")
    )]
    fn login_status(
        &self,
        url: &str,
        token: &SessionToken,
        id: HandoffId,
    ) -> Result<WireState, TransportError> {
        #[cfg(test)]
        if let Some(state) = self.handoff_override.lock().ok().and_then(|s| *s) {
            return Ok(state);
        }
        hosted::login_webauthn_status(url, token, id.0)
    }

    /// Tests: answer every handoff status call with `state` (`None`: ask the server).
    #[cfg(test)]
    pub(crate) fn override_handoff_state(&self, state: Option<WireState>) {
        if let Ok(mut slot) = self.handoff_override.lock() {
            *slot = state;
        }
    }

    /// Tests: runs `f` with the token of the waiting login.
    #[cfg(test)]
    pub(crate) fn with_pending_token<R>(&self, f: impl FnOnce(&SessionToken) -> R) -> Option<R> {
        let pending = self.pending_login.lock().ok()?;
        pending.as_ref().map(|p| f(&p.token))
    }
}

fn millis(ms: u64) -> UnixMillis {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

/// The IPC error of a failed security-key call.
fn webauthn_error(error: TransportError) -> IpcError {
    match error {
        // A login handoff for an account without a key answers `badRequest`.
        TransportError::Rejected(Rejection::BadRequest) => {
            IpcError::new(ErrorCode::SyncWebauthnUnavailable)
        }
        other => IpcError::from(other),
    }
}

/// Reads the 32 hex digits of a key reference.
fn parse_ref(text: &str) -> IpcResult<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 32 {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        let digits = text
            .get(2 * i..2 * i + 2)
            .ok_or(IpcError::new(ErrorCode::InvalidInput))?;
        *byte =
            u8::from_str_radix(digits, 16).map_err(|_| IpcError::new(ErrorCode::InvalidInput))?;
    }
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn state_of(state: WireState) -> HandoffState {
    match state {
        WireState::Waiting => HandoffState::Waiting,
        WireState::Done => HandoffState::Done,
        WireState::Failed => HandoffState::Failed,
        WireState::Expired => HandoffState::Expired,
    }
}

impl Backend {
    /// Opens the security-key page of a handoff in the system browser. The URL
    /// is `<origin>/webauthn/#<id>`, built here from the stored canonical
    /// origin and the ID the server returned: **the only call site of the
    /// opener** (`opener_has_one_call_site`). `false` if the origin is not
    /// canonical or the system refused; the user can retry with
    /// `sync_webauthn_reopen`.
    fn open_handoff(&self, origin: &str, id: &HandoffId) -> bool {
        match hosted::handoff_page_url(origin, &id.0) {
            Ok(url) => self.opener.open(&url),
            Err(_) => false,
        }
    }

    /// Opens the page of a handoff the server just started. A browser that
    /// does not open is not an error: the user has "Re-open".
    fn handoff_begin(&self, url: &str, start: &HandoffStartResponse) -> ActiveHandoff {
        let id = HandoffId(start.handoff_id.0);
        self.open_handoff(url, &id);
        ActiveHandoff {
            id,
            expires: self.sync.now() + PENDING_TTL,
        }
    }

    /// The waiting login (a connect, a join or a fresh login), if it still is valid.
    fn take_login_for_handoff(&self) -> IpcResult<PendingLogin> {
        let pending = self
            .sync
            .take_pending()
            .ok_or(IpcError::new(ErrorCode::SyncSignedOut))?;
        if pending.expires <= self.sync.now() {
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        }
        Ok(pending)
    }

    // ---- the second-factor step at connect and join -------------------------------

    /// `sync_login_methods`: which second factors the waiting login can use.
    ///
    /// # Errors
    /// `syncSignedOut` (no login waits, or it ran out) and the transport's
    /// `sync*` codes.
    pub fn sync_login_methods(&self) -> IpcResult<LoginMethods> {
        let epoch = self.sync.epoch();
        let pending = self.take_login_for_handoff()?;
        let answer = hosted::login_methods(&pending.url, &pending.token);
        self.sync.set_pending_in(epoch, pending);
        let methods = answer?;
        Ok(LoginMethods {
            totp: methods.totp,
            webauthn: methods.webauthn,
        })
    }

    /// `sync_hosted_webauthn_begin`: starts the login handoff of the waiting
    /// `sync_hosted_connect` and opens the browser.
    ///
    /// # Errors
    /// `syncSignedOut`, `syncWebauthnUnavailable`, `syncRateLimited` and the
    /// transport's `sync*` codes.
    pub fn sync_hosted_webauthn_begin(&self) -> IpcResult<WebauthnBegin> {
        self.login_handoff_begin(false)
    }

    /// `join_hosted_webauthn_begin`: the same for the waiting `join_hosted`.
    ///
    /// # Errors
    /// `accountExists`, `alreadyUnlocked` and the codes of
    /// `sync_hosted_webauthn_begin`.
    pub fn join_hosted_webauthn_begin(&self) -> IpcResult<WebauthnBegin> {
        self.join_prerequisites()?;
        self.login_handoff_begin(true)
    }

    fn login_handoff_begin(&self, join: bool) -> IpcResult<WebauthnBegin> {
        let epoch = self.sync.epoch();
        let mut pending = self.take_login_for_handoff()?;
        if matches!(pending.kind, PendingKind::Join { .. }) != join {
            self.sync.set_pending_in(epoch, pending);
            return Err(IpcError::new(ErrorCode::SyncSignedOut));
        }
        let result = hosted::login_webauthn_handoff(&pending.url, &pending.token).map(|start| {
            let handoff = self.handoff_begin(&pending.url, &start);
            pending.handoff = Some(handoff);
            WebauthnBegin {
                match_code: start.match_code.clone(),
                expires_at: millis(start.expires_at),
            }
        });
        self.sync.set_pending_in(epoch, pending);
        result.map_err(webauthn_error)
    }

    /// `sync_webauthn_status`: where the login handoff stands. On `done` the
    /// waiting connect, join or fresh login is finished before this answers.
    ///
    /// # Errors
    /// `syncSignedOut`, `syncHandoffExpired` (no handoff runs), the codes of
    /// the finished connect or join, and the transport's `sync*` codes.
    pub fn sync_webauthn_status(&self) -> IpcResult<WebauthnStatus> {
        let epoch = self.sync.epoch();
        let mut pending = self.take_login_for_handoff()?;
        let Some(handoff) = pending.handoff else {
            self.sync.set_pending_in(epoch, pending);
            return Err(IpcError::new(ErrorCode::SyncHandoffExpired));
        };
        let reply = |state| Ok(WebauthnStatus { state });
        if handoff.expires <= self.sync.now() {
            pending.handoff = None;
            self.sync.set_pending_in(epoch, pending);
            return reply(HandoffState::Expired);
        }
        match self
            .sync
            .login_status(&pending.url, &pending.token, handoff.id)
        {
            Ok(WireState::Done) => {
                self.finish_login_handoff(epoch, pending)?;
                reply(HandoffState::Done)
            }
            Ok(WireState::Waiting) => {
                self.sync.set_pending_in(epoch, pending);
                reply(HandoffState::Waiting)
            }
            // Failed and expired: the login itself may still wait for a code.
            Ok(ended @ (WireState::Failed | WireState::Expired)) => {
                pending.handoff = None;
                self.sync.set_pending_in(epoch, pending);
                reply(state_of(ended))
            }
            Err(TransportError::NotFound | TransportError::Rejected(Rejection::HandoffExpired)) => {
                pending.handoff = None;
                self.sync.set_pending_in(epoch, pending);
                reply(HandoffState::Expired)
            }
            Err(error) => {
                self.sync.set_pending_in(epoch, pending);
                Err(webauthn_error(error))
            }
        }
    }

    /// The end of a login handoff: the session is complete, so the connect, the
    /// join or the fresh login finishes as it does after a code.
    fn finish_login_handoff(&self, epoch: u64, pending: PendingLogin) -> IpcResult<()> {
        let PendingLogin {
            url,
            email,
            device_name,
            account,
            token,
            kind,
            ..
        } = pending;
        match kind {
            PendingKind::Connect => {
                let account = account.ok_or(IpcError::new(ErrorCode::SyncSignedOut))?;
                self.finish_connect(epoch, account, &url, &email, &device_name, &token)
                    .map(|_| ())
            }
            PendingKind::Join { keys, secret_key } => self
                .finish_join(&url, &email, &device_name, &token, keys, &secret_key)
                .map(|_| ()),
            PendingKind::Fresh => {
                let account = account.ok_or(IpcError::new(ErrorCode::SyncSignedOut))?;
                self.sync.check_epoch(epoch)?;
                self.sync
                    .save_token(self, account, &StoredToken::from_session(&token))?;
                self.sync.note_fresh();
                self.sync_restart();
                Ok(())
            }
        }
    }

    /// `sync_webauthn_reopen`: opens the browser again for the running handoff.
    ///
    /// # Errors
    /// `syncHandoffExpired` if none runs.
    pub fn sync_webauthn_reopen(&self) -> IpcResult<()> {
        let (url, id) = self
            .sync
            .active_handoff()
            .ok_or(IpcError::new(ErrorCode::SyncHandoffExpired))?;
        self.open_handoff(&url, &id);
        Ok(())
    }

    // ---- the key list ---------------------------------------------------------------

    /// `sync_webauthn_list`: the account's security keys.
    ///
    /// # Errors
    /// `syncNotConfigured` (a folder), `syncSignedOut` and the transport's
    /// `sync*` codes.
    pub fn sync_webauthn_list(&self) -> IpcResult<WebauthnKeys> {
        let ctx = self.hosted_ctx()?;
        let token = self.stored_token(ctx.account)?;
        let entries = hosted::account_webauthn_list(&ctx.url, &token)?;
        let totp = match hosted::account_info(&ctx.url, &token) {
            Ok(info) => {
                self.sync.update_view(|view| {
                    view.totp_enabled = Some(info.totp_enabled);
                    true
                });
                info.totp_enabled
            }
            Err(_) => self.totp_enabled_of(&ctx),
        };
        self.sync.update_view(|view| {
            view.webauthn_keys = u32::try_from(entries.len()).ok();
            true
        });
        Ok(WebauthnKeys {
            totp,
            keys: entries
                .into_iter()
                .map(|e| WebauthnKeyView {
                    credential_ref: hex(&e.credential_ref.0),
                    label: e.label,
                    created_at: millis(e.created_at),
                    last_used_at: e.last_used_at.map(millis),
                })
                .collect(),
        })
    }

    /// Best effort: the key count after an add or a removal.
    fn refresh_key_count(&self, ctx: &HostedCtx, token: &StoredToken) {
        if let Ok(keys) = hosted::account_webauthn_list(&ctx.url, token) {
            self.sync.update_view(|view| {
                view.webauthn_keys = u32::try_from(keys.len()).ok();
                true
            });
        }
    }

    /// `sync_webauthn_add_begin`: a fresh login, then the register handoff.
    ///
    /// # Errors
    /// `invalidInput`, `wrongCredentials`, `syncTwoFactorRequired` (a code is
    /// needed and the account has no security key), `syncTwoFactorInvalid`,
    /// `syncWebauthnUnavailable`, `syncWebauthnLimit`, `syncRateLimited` and the
    /// transport's `sync*` codes.
    pub fn sync_webauthn_add_begin(
        &self,
        request: &WebauthnAddRequest,
    ) -> IpcResult<WebauthnAccountBegin> {
        let label = device_name(&request.label)?;
        let code = code_of(request.code.as_ref())?;
        self.account_begin(&request.password, code, AccountKind::Register { label })
    }

    /// `sync_webauthn_confirm_begin`: a fresh login, then a confirm handoff.
    ///
    /// # Errors
    /// As [`Self::sync_webauthn_add_begin`], without the label and limit codes.
    pub fn sync_webauthn_confirm_begin(
        &self,
        request: &WebauthnConfirmRequest,
    ) -> IpcResult<WebauthnAccountBegin> {
        self.account_begin(&request.password, None, AccountKind::Confirm)
    }

    fn account_begin(
        &self,
        password: &SecretIn,
        code: Option<&str>,
        kind: AccountKind,
    ) -> IpcResult<WebauthnAccountBegin> {
        let ctx = self.hosted_ctx()?;
        let epoch = self.sync.epoch();
        let mp = MasterPassword::new(password.expose_secret().to_owned());
        if mp.expose_secret().is_empty() {
            return Err(IpcError::new(ErrorCode::EmptyPassword));
        }
        // Counts wrong passwords towards the five-failure lock.
        self.verify_master_only(&mp)?;
        let reuse = self.sync.fresh_recently();
        let opw: Option<OpaquePassword> = if reuse {
            None
        } else {
            let key = self
                .secret_key_of(ctx.account)
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
            Some(self.with_unlocked(|session| Ok(session.opaque_password(&mp, &key)?))?)
        };
        drop(mp);
        // A fresh login ends this device's running session: the thread stops
        // before it and starts again with the token the login granted.
        self.sync.stop();
        let result = self.account_start(&ctx, epoch, opw.as_ref(), code, kind);
        self.sync_restart();
        result
    }

    fn account_start(
        &self,
        ctx: &HostedCtx,
        epoch: u64,
        opw: Option<&OpaquePassword>,
        code: Option<&str>,
        kind: AccountKind,
    ) -> IpcResult<WebauthnAccountBegin> {
        let token = match opw {
            // A fresh login of the last four minutes stands.
            None => self.stored_token(ctx.account)?,
            Some(opw) => {
                let grant = hosted::login(
                    &ctx.url,
                    &ctx.email,
                    ctx.account,
                    opw,
                    ctx.install,
                    &ctx.device_name,
                )
                .map_err(|e| self.password_changed_error(e))?;
                if grant.status == LoginStatus::TwoFactorRequired {
                    let Some(code) = code else {
                        return self.account_login_handoff(ctx, epoch, grant.token, kind);
                    };
                    hosted::login_totp(&ctx.url, &grant.token, code)?;
                }
                self.sync.check_epoch(epoch)?;
                let stored = StoredToken::from_session(&grant.token);
                self.sync.save_token(self, ctx.account, &stored)?;
                self.sync.note_fresh();
                stored
            }
        };
        self.account_handoff_start(ctx, epoch, &token, kind)
    }

    /// The fresh login waits for a second factor and no code was given: a
    /// security key takes its place, through a login handoff first.
    fn account_login_handoff(
        &self,
        ctx: &HostedCtx,
        epoch: u64,
        pending: SessionToken,
        kind: AccountKind,
    ) -> IpcResult<WebauthnAccountBegin> {
        let methods = hosted::login_methods(&ctx.url, &pending)?;
        if !methods.webauthn {
            // The account has no usable key: the code step is next, as ever.
            self.hold_fresh(ctx, epoch, pending);
            return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
        }
        let start = hosted::login_webauthn_handoff(&ctx.url, &pending).map_err(webauthn_error)?;
        let handoff = self.handoff_begin(&ctx.url, &start);
        let flow = AccountFlow {
            kind,
            step: HandoffStep::Login,
            url: ctx.url.clone(),
            account: ctx.account,
            login: Some(pending),
            handoff,
        };
        if !self.sync.set_account_flow_in(epoch, flow) {
            return Err(IpcError::LOCKED);
        }
        Ok(WebauthnAccountBegin {
            step: HandoffStep::Login,
            match_code: start.match_code.clone(),
            expires_at: millis(start.expires_at),
        })
    }

    /// Starts the register or confirm handoff with a complete session.
    fn account_handoff_start(
        &self,
        ctx: &HostedCtx,
        epoch: u64,
        token: &StoredToken,
        kind: AccountKind,
    ) -> IpcResult<WebauthnAccountBegin> {
        let (purpose, label, step) = match &kind {
            AccountKind::Register { label } => (
                HandoffPurpose::Register,
                Some(label.as_str()),
                HandoffStep::Register,
            ),
            AccountKind::Confirm => (HandoffPurpose::Confirm, None, HandoffStep::Confirm),
        };
        let start = hosted::account_webauthn_handoff(&ctx.url, token, purpose, label).map_err(
            |e| match e {
                // The server allows ten keys per account.
                TransportError::QuotaExceeded => IpcError::new(ErrorCode::SyncWebauthnLimit),
                other => webauthn_error(other),
            },
        )?;
        let handoff = self.handoff_begin(&ctx.url, &start);
        let flow = AccountFlow {
            kind,
            step,
            url: ctx.url.clone(),
            account: ctx.account,
            login: None,
            handoff,
        };
        if !self.sync.set_account_flow_in(epoch, flow) {
            return Err(IpcError::LOCKED);
        }
        Ok(WebauthnAccountBegin {
            step,
            match_code: start.match_code.clone(),
            expires_at: millis(start.expires_at),
        })
    }

    /// `sync_webauthn_add_status`: where the account handoff stands. When a login
    /// step is done, the next handoff starts and the answer is `waiting` with
    /// its step and code.
    ///
    /// # Errors
    /// `syncHandoffExpired` (none runs), `syncFreshLoginExpired`,
    /// `syncSignedOut` and the transport's `sync*` codes.
    pub fn sync_webauthn_add_status(&self) -> IpcResult<WebauthnAccountStatus> {
        let epoch = self.sync.epoch();
        let flow = self
            .sync
            .take_account_flow()
            .ok_or(IpcError::new(ErrorCode::SyncHandoffExpired))?;
        let step = flow.step;
        let plain = |state, step| {
            Ok(WebauthnAccountStatus {
                state,
                step,
                match_code: None,
                expires_at: None,
            })
        };
        if flow.expired(self.sync.now()) {
            return plain(HandoffState::Expired, step);
        }
        let asked = if let (Some(token), HandoffStep::Login) = (&flow.login, step) {
            self.sync.login_status(&flow.url, token, flow.handoff.id)
        } else {
            let token = self.stored_token(flow.account)?;
            hosted::account_webauthn_status(&flow.url, &token, flow.handoff.id.0)
        };
        match asked {
            Ok(WireState::Done) if step == HandoffStep::Login => {
                self.advance_login_step(epoch, flow)
            }
            Ok(WireState::Done) => {
                if matches!(flow.kind, AccountKind::Register { .. })
                    && let Ok(ctx) = self.hosted_ctx()
                    && let Ok(token) = self.stored_token(ctx.account)
                {
                    self.refresh_key_count(&ctx, &token);
                }
                plain(HandoffState::Done, step)
            }
            Ok(WireState::Waiting) => {
                self.sync.set_account_flow_in(epoch, flow);
                plain(HandoffState::Waiting, step)
            }
            Ok(ended @ (WireState::Failed | WireState::Expired)) => plain(state_of(ended), step),
            Err(TransportError::NotFound | TransportError::Rejected(Rejection::HandoffExpired)) => {
                plain(HandoffState::Expired, step)
            }
            Err(error) => {
                self.sync.set_account_flow_in(epoch, flow);
                Err(webauthn_error(error))
            }
        }
    }

    /// The login step of an account flow is done: the session is complete, so
    /// it is kept, and the register or confirm handoff starts.
    fn advance_login_step(
        &self,
        epoch: u64,
        flow: AccountFlow,
    ) -> IpcResult<WebauthnAccountStatus> {
        let AccountFlow {
            kind,
            account,
            login,
            ..
        } = flow;
        let session = login.ok_or(IpcError::new(ErrorCode::SyncHandoffExpired))?;
        self.sync.check_epoch(epoch)?;
        let stored = StoredToken::from_session(&session);
        self.sync.save_token(self, account, &stored)?;
        self.sync.note_fresh();
        self.sync_restart();
        let ctx = self.hosted_ctx()?;
        let next = self.account_handoff_start(&ctx, epoch, &stored, kind)?;
        Ok(WebauthnAccountStatus {
            state: HandoffState::Waiting,
            step: next.step,
            match_code: Some(next.match_code),
            expires_at: Some(next.expires_at),
        })
    }

    /// `sync_webauthn_remove`: removes one key. The server wants the session of
    /// a fresh login and a second factor: a code, or a `confirm` from the last
    /// five minutes. Removing the last factor turns two-step verification off.
    ///
    /// # Errors
    /// `invalidInput`, `syncTwoFactorRequired` (no code and no recent
    /// confirm), `syncTwoFactorInvalid`, `syncFreshLoginExpired` and the
    /// transport's `sync*` codes.
    pub fn sync_webauthn_remove(&self, request: &WebauthnRemoveRequest) -> IpcResult<()> {
        let code = code_of(request.code.as_ref())?;
        let reference = parse_ref(&request.credential_ref)?;
        let ctx = self.hosted_ctx()?;
        let token = self.stored_token(ctx.account)?;
        match hosted::account_webauthn_remove(&ctx.url, &token, reference, code) {
            Ok(()) => {}
            Err(TransportError::Rejected(Rejection::TwoFactorInvalid)) if code.is_none() => {
                return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
            }
            Err(TransportError::NotFound) => return Err(IpcError::new(ErrorCode::InvalidInput)),
            Err(error) => return Err(error.into()),
        }
        self.refresh_key_count(&ctx, &token);
        Ok(())
    }
}
