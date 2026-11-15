//! Sync in the desktop app (M6, ADR-0042 to ADR-0046): the service that runs
//! the sync engine, and the flows that connect, join and disconnect.
//!
//! - [`SyncService`] (this module) owns the sync thread of the unlocked app:
//!   it starts on unlock when sync is configured and stops on lock (every
//!   transport operation then fails closed with `Locked`; the engine and the
//!   transport are dropped, and the next unlock rebuilds them from the
//!   persisted engine state). Each start is a generation: a thread of an
//!   older generation can neither reach the session nor change the state:
//!   every write of the thread (status, warnings, token deletion) is checked
//!   against the generation, under the lock of what it writes.
//! - [`access`]: the session handle of one generation.
//! - [`runner`]: the loop of the thread.
//! - [`hosted_flow`] and [`folder_flow`]: the commands for the two backends.
//!
//! The token of the hosted session lives in the keystore
//! ([`crate::keystore::SyncTokenStore`]); folder paths live in the vault
//! store and never reach the webview except as text to display (ADR-0034).
//!
//! Besides the generation, the service has a **lock epoch**: it counts the
//! locks (and disconnects, [`SyncService::reset`]). A flow that talks to the
//! network records it first and drops its result, writing nothing, if it
//! changed (C-L2). A lock never waits for a master password change in flight
//! (ADR-0155): the change either commits in one slot step or is recovered.

pub mod access;
mod errors;
mod folder_flow;
mod hosted_flow;
mod rotation_flow;
mod runner;
mod sharing_flow;
pub(crate) mod strings;
pub(crate) mod webauthn_flow;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use mosaic_core::{AccountId, InstallId, VaultId};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, SyncEngine};
use mosaic_sync::folder::{FolderConfig, FolderTransport};
use mosaic_sync::http::{HttpTransport, StoredToken, Token as _, hosted};
use mosaic_sync::transport::{RevokedReason, TransportError, TransportKind};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_vault::session::{SyncConfig, SyncKind, UnlockedSession};

pub use errors::{sync_code, transport_code};
pub(crate) use hosted_flow::PendingLogin;

use self::access::BackendSession;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, SyncState, SyncStatus};
use crate::contract::sync::{AcknowledgeRequest, SyncConfigView, SyncOverview, SyncWarningView};
use crate::state::{Backend, Slot, guard};

/// How long a chosen folder stays valid (`choiceId`, ADR-0034).
pub const CHOICE_TTL: Duration = Duration::from_mins(5);
/// The most folder choices held at once.
const MAX_CHOICES: usize = 8;
/// How long "Sync now" waits for the run to end before it answers.
const SYNC_NOW_WAIT: Duration = Duration::from_secs(120);
/// How long the mark of a fresh login lasts (the server allows five minutes).
const FRESH_TTL: Duration = Duration::from_secs(240);

/// What the thread is told.
pub(crate) enum Cmd {
    /// A local change: sync soon (debounced).
    Poke,
    /// Sync now and answer when the run ended.
    SyncNow(SyncSender<()>),
    /// The user acknowledged a halt or findings: run again.
    Acknowledged,
}

/// What the UI is shown.
#[derive(Clone)]
pub(crate) struct View {
    pub status: SyncStatus,
    pub last_synced_at: Option<i64>,
    pub error: Option<ErrorCode>,
    pub warnings: Vec<SyncWarningView>,
    /// Whether the hosted sync account has a second factor (`None`: unknown).
    pub totp_enabled: Option<bool>,
    /// How many security keys the hosted sync account has (`None`: unknown).
    pub webauthn_keys: Option<u32>,
    /// The backend holds a master password this device does not know.
    pub password_changed: bool,
    /// Why the server ended the session (set with `SignedOut`).
    pub revoked: Option<RevokedReason>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            status: SyncStatus::NotConfigured,
            last_synced_at: None,
            error: None,
            warnings: Vec::new(),
            totp_enabled: None,
            webauthn_keys: None,
            password_changed: false,
            revoked: None,
        }
    }
}

/// What a sync thread runs against.
pub(crate) enum RunKind {
    Hosted { url: String },
    Folder { root: PathBuf },
}

/// Everything a sync thread needs besides the backend.
pub(crate) struct RunConfig {
    pub kind: RunKind,
    pub device_name: String,
    pub account: AccountId,
    pub install: InstallId,
}

impl RunConfig {
    /// The run of a stored backend choice (`None` for an inconsistent one).
    pub(crate) fn of(config: &SyncConfig, account: AccountId, install: InstallId) -> Option<Self> {
        let kind = match config.kind {
            SyncKind::Hosted => RunKind::Hosted {
                url: config.server_url.clone()?,
            },
            SyncKind::Folder => RunKind::Folder {
                root: config.folder_path.clone()?,
            },
        };
        Some(Self {
            kind,
            device_name: config.device_name.clone(),
            account,
            install,
        })
    }
}

/// The sync service of the app. See the module documentation.
pub struct SyncService {
    weak: Mutex<Option<Weak<Backend>>>,
    generation: AtomicU64,
    commands: Mutex<Option<Sender<Cmd>>>,
    view: Mutex<View>,
    choices: Mutex<BTreeMap<String, (PathBuf, Instant)>>,
    pending_login: Mutex<Option<PendingLogin>>,
    /// The browser step of adding a security key or confirming a second factor
    /// (M9, [`webauthn_flow`]).
    account_flow: Mutex<Option<webauthn_flow::AccountFlow>>,
    /// Tests answer the handoff status call with this instead of the server.
    #[cfg(test)]
    handoff_override: Mutex<Option<mosaic_sync::wire::json::HandoffState>>,
    /// When the last fresh login of this process ended (a second factor
    /// that needs two codes reuses it for five minutes).
    fresh_at: Mutex<Option<Instant>>,
    /// Added to the clock that expires choices and logins (tests move it).
    skew: Mutex<Duration>,
    /// Counts the locks and resets (see the module documentation).
    epoch: AtomicU64,
    /// Whether a master password change is in flight.
    change: Mutex<bool>,
    /// Serializes saving and compare-and-deleting the session token.
    token_lock: Mutex<()>,
    /// The invites and emergency requests the last event told the webview about.
    sharing_counts: Mutex<(u32, u32)>,
    /// Attachments the last key change of a shared vault could not protect.
    unprotected: Mutex<BTreeMap<VaultId, u32>>,
}

impl Default for SyncService {
    fn default() -> Self {
        Self {
            weak: Mutex::new(None),
            generation: AtomicU64::new(0),
            commands: Mutex::new(None),
            view: Mutex::new(View::default()),
            choices: Mutex::new(BTreeMap::new()),
            pending_login: Mutex::new(None),
            account_flow: Mutex::new(None),
            #[cfg(test)]
            handoff_override: Mutex::new(None),
            fresh_at: Mutex::new(None),
            skew: Mutex::new(Duration::ZERO),
            epoch: AtomicU64::new(0),
            change: Mutex::new(false),
            token_lock: Mutex::new(()),
            sharing_counts: Mutex::new((0, 0)),
            unprotected: Mutex::new(BTreeMap::new()),
        }
    }
}

/// A master password change in flight ([`SyncService::begin_change`]); only
/// one runs at a time. A lock does not wait for it.
pub(crate) struct ChangeGuard<'a>(&'a SyncService);

impl Drop for ChangeGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut busy) = self.0.change.lock() {
            *busy = false;
        }
    }
}

impl SyncService {
    /// Gives the service the backend it serves (a weak handle: the threads
    /// end with the app).
    pub(crate) fn attach(&self, backend: Weak<Backend>) {
        if let Ok(mut weak) = self.weak.lock() {
            *weak = Some(backend);
        }
    }

    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == generation
    }

    pub(crate) fn now(&self) -> Instant {
        Instant::now() + self.skew.lock().map_or(Duration::ZERO, |s| *s)
    }

    /// Moves the clock of the choice and login expiry forward (tests).
    #[cfg(test)]
    pub(crate) fn advance(&self, by: Duration) {
        if let Ok(mut skew) = self.skew.lock() {
            *skew += by;
        }
    }

    /// The state the UI shows.
    pub(crate) fn state(&self) -> SyncState {
        self.view.lock().map_or(
            SyncState {
                status: SyncStatus::Error,
                last_synced_at: None,
                error: Some(ErrorCode::Internal),
            },
            |v| SyncState {
                status: v.status,
                last_synced_at: v.last_synced_at,
                error: v.error,
            },
        )
    }

    pub(crate) fn view(&self) -> View {
        self.view.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Changes the view; `true` if `change` says something changed.
    pub(crate) fn update_view(&self, change: impl FnOnce(&mut View) -> bool) -> bool {
        self.view.lock().is_ok_and(|mut view| change(&mut view))
    }

    /// Ends the running thread, if any (its handles fail from now on).
    pub(crate) fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut commands) = self.commands.lock() {
            *commands = None;
        }
    }

    /// Ends the thread and forgets everything shown and every pending flow
    /// (a lock).
    pub(crate) fn reset(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.stop();
        self.update_view(|view| {
            *view = View::default();
            true
        });
        if let Ok(mut choices) = self.choices.lock() {
            choices.clear();
        }
        // Dropping a pending join wipes its keys and Secret Key.
        if let Ok(mut pending) = self.pending_login.lock() {
            *pending = None;
        }
        if let Ok(mut flow) = self.account_flow.lock() {
            *flow = None;
        }
        if let Ok(mut fresh) = self.fresh_at.lock() {
            *fresh = None;
        }
        if let Ok(mut counts) = self.sharing_counts.lock() {
            *counts = (0, 0);
        }
        if let Ok(mut unprotected) = self.unprotected.lock() {
            unprotected.clear();
        }
    }

    /// Notes the sharing counts a run found; `true` if they differ from what
    /// the webview was told last (only the current generation may).
    pub(crate) fn set_sharing_counts(&self, generation: u64, invites: u32, requests: u32) -> bool {
        let Ok(mut counts) = self.sharing_counts.lock() else {
            return false;
        };
        if !self.is_current(generation) || *counts == (invites, requests) {
            return false;
        }
        *counts = (invites, requests);
        true
    }

    /// Notes how many attachments the key change of `vault` could not protect.
    pub(crate) fn set_unprotected(&self, vault: VaultId, count: u32) {
        if let Ok(mut map) = self.unprotected.lock() {
            if count == 0 {
                map.remove(&vault);
            } else {
                map.insert(vault, count);
            }
        }
    }

    /// The attachments of `vault` the last key change could not protect.
    pub(crate) fn unprotected(&self, vault: VaultId) -> u32 {
        self.unprotected
            .lock()
            .ok()
            .and_then(|map| map.get(&vault).copied())
            .unwrap_or(0)
    }

    /// The lock epoch: a flow records it before it talks to anything and
    /// compares it afterwards ([`Self::check_epoch`]).
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    /// `locked` if the app locked (or reset) since `epoch` was read: the
    /// flow must drop its results and write nothing.
    pub(crate) fn check_epoch(&self, epoch: u64) -> IpcResult<()> {
        if self.epoch() == epoch {
            Ok(())
        } else {
            Err(IpcError::LOCKED)
        }
    }

    /// Drops what expired: pending logins (and the keys and Secret Key a
    /// pending join holds), folder choices and the fresh-login mark. Runs
    /// on every sync command and on lock.
    pub(crate) fn sweep(&self) {
        let now = self.now();
        if let Ok(mut choices) = self.choices.lock() {
            choices.retain(|_, (_, at)| now.saturating_duration_since(*at) < CHOICE_TTL);
        }
        if let Ok(mut pending) = self.pending_login.lock()
            && pending.as_ref().is_some_and(|p| p.expires <= now)
        {
            *pending = None;
        }
        if let Ok(mut flow) = self.account_flow.lock()
            && flow.as_ref().is_some_and(|f| f.expired(now))
        {
            *flow = None;
        }
        if let Ok(mut fresh) = self.fresh_at.lock()
            && fresh.is_some_and(|at| now.saturating_duration_since(at) >= FRESH_TTL)
        {
            *fresh = None;
        }
    }

    /// Marks a master password change in flight until the guard drops.
    ///
    /// # Errors
    /// `busy` if another change is in flight.
    pub(crate) fn begin_change(&self) -> IpcResult<ChangeGuard<'_>> {
        let mut busy = guard(&self.change)?;
        if *busy {
            return Err(IpcError::new(ErrorCode::Busy));
        }
        *busy = true;
        Ok(ChangeGuard(self))
    }

    /// The current generation (tests).
    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Sets whether the backend holds a master password this device does not
    /// know (only the thread of the current generation may).
    pub(crate) fn set_password_changed(&self, generation: u64, changed: bool) -> bool {
        self.update_view(|view| {
            if !self.is_current(generation) || view.password_changed == changed {
                return false;
            }
            view.password_changed = changed;
            true
        })
    }

    /// Saves the session token, serialized with [`Self::delete_token_if`].
    pub(crate) fn save_token(
        &self,
        backend: &Backend,
        account: AccountId,
        token: &StoredToken,
    ) -> IpcResult<()> {
        let _lock = guard(&self.token_lock)?;
        backend.sync_tokens.save(account, token)
    }

    /// Deletes the stored token only if it still is `expected` (constant-time
    /// comparison): a thread of an older generation must not delete the
    /// token of a newer login. `false` if a different token is stored now
    /// (a newer login replaced it: leave everything alone); `true` if it was
    /// `expected` (deleted) or there is none.
    pub(crate) fn delete_token_if(
        &self,
        backend: &Backend,
        account: AccountId,
        expected: &StoredToken,
    ) -> bool {
        use subtle::ConstantTimeEq as _;
        let Ok(_lock) = guard(&self.token_lock) else {
            return false;
        };
        match backend.sync_tokens.load(account) {
            Ok(Some(stored)) => {
                if bool::from(stored.expose_bytes().ct_eq(expected.expose_bytes())) {
                    // A token that cannot be deleted stays dead: the next
                    // start gets a 401 again.
                    let _ = backend.sync_tokens.delete(account);
                    true
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    fn send(&self, command: Cmd) {
        if let Ok(commands) = self.commands.lock()
            && let Some(tx) = commands.as_ref()
        {
            let _ = tx.send(command);
        }
    }

    /// The session handle of the current generation, for a flow that needs a
    /// temporary transport outside the thread.
    ///
    /// # Errors
    /// `internal` if the service was never attached to a backend.
    pub(crate) fn session(&self) -> IpcResult<BackendSession> {
        let weak = self
            .weak
            .lock()
            .ok()
            .and_then(|w| w.clone())
            .ok_or(IpcError::INTERNAL)?;
        Ok(BackendSession::new(
            weak,
            self.generation.load(Ordering::SeqCst),
        ))
    }

    /// A local change happened: sync soon.
    pub(crate) fn poke(&self) {
        self.send(Cmd::Poke);
    }

    /// Runs a sync and waits for its end (at most [`SYNC_NOW_WAIT`]).
    pub(crate) fn sync_now(&self) {
        let (tx, rx) = mpsc::sync_channel(1);
        let running = self
            .commands
            .lock()
            .is_ok_and(|c| c.as_ref().is_some_and(|c| c.send(Cmd::SyncNow(tx)).is_ok()));
        if running {
            let _ = rx.recv_timeout(SYNC_NOW_WAIT);
        }
    }

    /// Starts the sync thread for `run` (ending the previous one).
    pub(crate) fn start(&self, run: RunConfig) {
        let Some(weak) = self.weak.lock().ok().and_then(|w| w.clone()) else {
            return;
        };
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::channel();
        if let Ok(mut commands) = self.commands.lock() {
            *commands = Some(tx);
        }
        self.update_view(|view| {
            view.status = SyncStatus::Syncing;
            view.error = None;
            true
        });
        let ctx = runner::Ctx {
            weak,
            generation,
            hosted: matches!(run.kind, RunKind::Hosted { .. }),
            token: std::sync::OnceLock::new(),
        };
        // A thread that cannot be spawned leaves the status at "syncing";
        // the next unlock tries again.
        let _ = std::thread::Builder::new()
            .name("mosaic-sync".into())
            .spawn(move || thread_main(&ctx, &run, &rx));
    }

    // ---- folder choices (ADR-0034) ----------------------------------------------

    /// Remembers a chosen folder under a fresh random ID.
    pub(crate) fn remember_choice(&self, path: PathBuf) -> IpcResult<String> {
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).map_err(|_| IpcError::INTERNAL)?;
        let mut text = String::from("choice-");
        for byte in id {
            let _ = write!(text, "{byte:02x}");
        }
        let id = text;
        let now = self.now();
        let mut choices = guard(&self.choices)?;
        choices.retain(|_, (_, at)| now.saturating_duration_since(*at) < CHOICE_TTL);
        while choices.len() >= MAX_CHOICES {
            let Some(oldest) = choices
                .iter()
                .min_by_key(|(_, (_, at))| *at)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            choices.remove(&oldest);
        }
        choices.insert(id.clone(), (path, now));
        Ok(id)
    }

    /// The folder behind `id`, while it is valid.
    ///
    /// # Errors
    /// `syncChoiceExpired` for an unknown or expired ID.
    pub(crate) fn choice(&self, id: &str) -> IpcResult<PathBuf> {
        let now = self.now();
        let mut choices = guard(&self.choices)?;
        match choices.get(id) {
            Some((path, at)) if now.saturating_duration_since(*at) < CHOICE_TTL => Ok(path.clone()),
            Some(_) => {
                choices.remove(id);
                Err(IpcError::new(ErrorCode::SyncChoiceExpired))
            }
            None => Err(IpcError::new(ErrorCode::SyncChoiceExpired)),
        }
    }

    pub(crate) fn forget_choice(&self, id: &str) {
        if let Ok(mut choices) = self.choices.lock() {
            choices.remove(id);
        }
    }

    // ---- pending flows ----------------------------------------------------------

    /// Test hook: production flows go through the epoch-checked `set_pending_in`.
    #[cfg(test)]
    pub(crate) fn set_pending(&self, pending: Option<PendingLogin>) {
        if let Ok(mut slot) = self.pending_login.lock() {
            *slot = pending;
        }
    }

    pub(crate) fn take_pending(&self) -> Option<PendingLogin> {
        self.pending_login.lock().ok().and_then(|mut p| p.take())
    }

    /// The token of a login that waits for its second-factor code (a fresh
    /// login of device removal or second-factor setup), if it is still valid
    /// for `url` and belongs to `account`.
    pub(crate) fn take_fresh_pending(
        &self,
        url: &str,
        account: AccountId,
    ) -> Option<mosaic_core::SessionToken> {
        let now = self.now();
        let mut slot = self.pending_login.lock().ok()?;
        match slot.as_ref() {
            Some(p)
                if matches!(p.kind, hosted_flow::PendingKind::Fresh)
                    && p.url == url
                    && p.account == Some(account)
                    && p.expires > now => {}
            _ => return None,
        }
        slot.take().map(|p| p.token)
    }

    /// Keeps a pending login or join, but only if the lock epoch still is
    /// `epoch` (a flow that spans a lock must not leave anything behind).
    /// `false` if it was dropped.
    pub(crate) fn set_pending_in(&self, epoch: u64, pending: PendingLogin) -> bool {
        // Set under the lock, then check the epoch: a lock bumps the epoch
        // before it drops the pending state, so either the check sees the
        // bump (and the login is taken back here) or the drop that follows
        // the bump finds it. A check before the store would leave a gap.
        let Ok(mut slot) = self.pending_login.lock() else {
            return false;
        };
        *slot = Some(pending);
        if self.epoch() == epoch {
            true
        } else {
            *slot = None;
            false
        }
    }

    /// Drops every pending login and join (and the keys and Secret Key a join
    /// holds), whatever the state of the vault: every lock does it, also in
    /// the onboarding where no vault is unlocked.
    pub(crate) fn drop_pending(&self) {
        if let Ok(mut pending) = self.pending_login.lock() {
            *pending = None;
        }
        if let Ok(mut flow) = self.account_flow.lock() {
            *flow = None;
        }
        if let Ok(mut fresh) = self.fresh_at.lock() {
            *fresh = None;
        }
    }

    pub(crate) fn note_fresh(&self) {
        let now = self.now();
        if let Ok(mut fresh) = self.fresh_at.lock() {
            *fresh = Some(now);
        }
    }

    /// Whether a fresh login ended within the last four minutes (the server
    /// allows five).
    pub(crate) fn fresh_recently(&self) -> bool {
        let now = self.now();
        self.fresh_at
            .lock()
            .ok()
            .and_then(|f| *f)
            .is_some_and(|at| now.saturating_duration_since(at) < FRESH_TTL)
    }
}

fn current_platform() -> Platform {
    match std::env::consts::OS {
        "windows" => Platform::Windows,
        "macos" => Platform::Macos,
        _ => Platform::Linux,
    }
}

/// The body of the sync thread: builds the transport over the right
/// backend and runs the loop.
fn thread_main(ctx: &runner::Ctx, run: &RunConfig, rx: &mpsc::Receiver<Cmd>) {
    match &run.kind {
        RunKind::Hosted { url } => {
            let Some(backend) = ctx.session().current() else {
                return;
            };
            let Ok(Some(token)) = backend.sync_tokens.load(run.account) else {
                backend.sync_signed_out(ctx.generation, None, None);
                return;
            };
            // The token this thread runs with: a 401 deletes only this one.
            let _ = ctx.token.set(crate::keystore::copy_token(&token));
            // Best effort: whether the account has a second factor (for the
            // settings; a failure leaves it unknown).
            drop(backend);
            if let Ok((info, served_epoch)) = hosted::account_epoch(url, &token)
                && let Some(backend) = ctx.session().current()
            {
                backend.sync.update_view(|view| {
                    if !backend.sync.is_current(ctx.generation) {
                        return false;
                    }
                    view.totp_enabled = Some(info.totp_enabled);
                    true
                });
                // A server that holds another master password than this
                // device: sync pauses until the user brings the new one
                // (`sync_recover_password`).
                if password_differs(&backend, &info.wrapped_ak, &info.params) {
                    let local_epoch = backend.with_unlocked(|s| Ok(s.ak_epoch())).ok();
                    backend.sync.set_password_changed(ctx.generation, true);
                    backend.sync.set_status(
                        &backend,
                        ctx.generation,
                        SyncStatus::Error,
                        Some(password_change_code(local_epoch, served_epoch)),
                    );
                    return;
                }
            }
            let install = run.install;
            let make = || {
                HttpTransport::connect(url, &token, install)
                    .map(|t| SyncEngine::new(t, EngineConfig::for_kind(TransportKind::Hosted)))
            };
            runner::run(ctx, make, rx);
        }
        RunKind::Folder { root } => {
            let make = || {
                open_folder(
                    ctx.session(),
                    root,
                    run.account,
                    run.install,
                    &run.device_name,
                )
                .map(|mut t| {
                    folder_password_check(ctx, &mut t);
                    SyncEngine::new(t, EngineConfig::for_kind(TransportKind::Folder))
                })
            };
            runner::run(ctx, make, rx);
        }
    }
}

/// What a server that holds other keys than this device means: an account-key epoch above this
/// device's is a re-key made elsewhere (the new Secret Key is needed); an equal one is a password
/// change; a lower one is not a re-key at all (the engine reports it as `AccountRollback`) and an
/// unknown local epoch is not guessed at.
pub(crate) fn password_change_code(local_epoch: Option<u32>, served_epoch: u32) -> ErrorCode {
    if local_epoch.is_some_and(|local| served_epoch > local) {
        ErrorCode::SyncAccountRekeyed
    } else {
        ErrorCode::SyncPasswordChanged
    }
}

/// Whether the backend's wrapped AK and parameters differ from this device's.
fn password_differs(
    backend: &Backend,
    wrapped_ak: &mosaic_core::WrappedKey,
    params: &mosaic_core::PasswordParams,
) -> bool {
    backend
        .with_unlocked(|session| Ok(session.account_bootstrap()?))
        .is_ok_and(|local| {
            local.wrapped_ak.as_bytes() != wrapped_ak.as_bytes()
                || local.params.salt != params.salt
                || local.params.profile != params.profile
        })
}

/// Reconciles the folder's account file with this device's password
/// (CRYPTO_SPEC §10.1, "Reconnecting to a folder"): a file that holds a
/// retired password, or an older version, is replaced by this device's own
/// at a higher version (the folder never keeps a superseded password live);
/// a newer file that is not retired is another device's change, and sets
/// the flag the settings show ([`View::password_changed`]). A folder that
/// cannot be read leaves everything as it was.
fn folder_password_check(
    ctx: &runner::Ctx,
    transport: &mut FolderTransport<SessionCrypto<BackendSession>>,
) {
    let Some(backend) = ctx.session().current() else {
        return;
    };
    let Ok(found) = backend.reconcile_folder_account(transport, None) else {
        return;
    };
    if let Some(version) = found.version() {
        let _ = backend.with_unlocked(|s| Ok(s.raise_folder_account_version(version)?));
    }
    let changed = found == folder_flow::FolderAccount::Changed;
    if backend.sync.set_password_changed(ctx.generation, changed) {
        backend.emit_sync_state();
    }
}

/// The folder transport over `session`: it opens the folder and publishes
/// this device's sealed info.
///
/// # Errors
/// [`TransportError`] of [`FolderTransport::open`]; `Locked` if the session
/// is locked.
pub(crate) fn open_folder(
    session: BackendSession,
    root: &std::path::Path,
    account: AccountId,
    install: InstallId,
    device_name: &str,
) -> Result<FolderTransport<SessionCrypto<BackendSession>>, TransportError> {
    let crypto = SessionCrypto::new(session).map_err(|_| TransportError::Locked)?;
    let now = mosaic_sync::clock::Clock::now_ms(&SystemClock);
    FolderTransport::open(FolderConfig {
        root: root.to_path_buf(),
        account_id: account,
        install_id: install,
        crypto,
        clock: Arc::new(SystemClock),
        device: DeviceInfo {
            created_at_ms: now,
            name: device_name.to_owned(),
            platform: current_platform(),
        },
    })
}

impl Backend {
    /// What follows an unlock: the sync thread starts if sync is set up.
    pub(crate) fn sync_after_unlock(&self, slot: &Slot) {
        let Ok(session) = slot.unlocked() else {
            return;
        };
        match session.sync_config() {
            Ok(Some(config)) => {
                match RunConfig::of(&config, session.account_id(), session.install_id()) {
                    Some(run) => self.sync.start(run),
                    None => {
                        self.sync.reset();
                    }
                }
            }
            _ => {
                self.sync.reset();
            }
        }
    }

    /// Starts the sync thread from the stored choice (after a flow changed
    /// the token or the choice).
    pub(crate) fn sync_restart(&self) {
        if let Ok(slot) = self.slot() {
            self.sync_after_unlock(&slot);
        }
    }

    /// What follows a lock: the thread ends and everything it showed is
    /// forgotten.
    pub(crate) fn sync_after_lock(&self) {
        self.sync.reset();
    }

    /// A local change happened (an item or vault was changed): sync soon.
    pub(crate) fn sync_poke(&self) {
        self.sync.poke();
    }

    pub(crate) fn emit_sync_state(&self) {
        self.emit(&AppEvent::SyncStateChanged {
            state: self.sync.state(),
        });
    }

    /// The server ended this device's session: the token is deleted and the
    /// user signs in again. `generation` and `token` are the thread's own: a
    /// thread of an older generation does nothing, and the token is deleted
    /// only if it still is the one the thread ran with (a newer login keeps
    /// its token and the status stays as the login left it).
    pub(crate) fn sync_signed_out(
        &self,
        generation: u64,
        token: Option<&StoredToken>,
        reason: Option<RevokedReason>,
    ) {
        if !self.sync.is_current(generation) {
            return;
        }
        if let (Some(account), Some(token)) = (self.account(), token)
            && !self
                .sync
                .delete_token_if(self, account.account_id.get(), token)
        {
            return;
        }
        let changed = self.sync.update_view(|view| {
            if !self.sync.is_current(generation) {
                return false;
            }
            view.status = SyncStatus::SignedOut;
            view.error = Some(ErrorCode::SyncSignedOut);
            view.revoked = reason;
            true
        });
        if changed {
            self.emit_sync_state();
        }
    }

    /// The master password change committed here, but the token it brought
    /// could not be kept: the device signs in again (C-M1).
    pub(crate) fn sync_force_signed_out(&self) {
        self.sync.update_view(|view| {
            view.status = SyncStatus::SignedOut;
            view.error = Some(ErrorCode::SyncSignedOut);
            view.revoked = None;
            true
        });
        self.emit_sync_state();
    }

    fn config_view(&self, config: &SyncConfig) -> Option<SyncConfigView> {
        match config.kind {
            SyncKind::Hosted => Some(SyncConfigView::Hosted {
                server_url: config.server_url.clone()?,
                email: config.email.clone().unwrap_or_default(),
                device_name: config.device_name.clone(),
                totp_enabled: self.sync.view().totp_enabled.unwrap_or(false),
                webauthn_keys: self.sync.view().webauthn_keys.unwrap_or(0),
            }),
            SyncKind::Folder => Some(SyncConfigView::Folder {
                display_path: config.folder_path.as_ref()?.display().to_string(),
                device_name: config.device_name.clone(),
            }),
        }
    }

    pub(crate) fn sync_overview(&self, session: &UnlockedSession) -> SyncOverview {
        let config = session
            .sync_config()
            .ok()
            .flatten()
            .and_then(|c| self.config_view(&c));
        if config.is_none() {
            return SyncOverview {
                config: None,
                state: SyncState {
                    status: SyncStatus::NotConfigured,
                    last_synced_at: None,
                    error: None,
                },
                warnings: Vec::new(),
                sharing_findings: Vec::new(),
                password_changed: false,
                rekey_pending: false,
                revoked: None,
            };
        }
        let view = self.sync.view();
        SyncOverview {
            config,
            state: self.sync.state(),
            warnings: view.warnings,
            sharing_findings: sharing_flow::findings_view(session),
            password_changed: view.password_changed,
            rekey_pending: session.rekey_pending().unwrap_or(false),
            revoked: view
                .revoked
                .filter(|_| view.status == SyncStatus::SignedOut)
                .map(errors::revoked_view),
        }
    }

    /// `sync_status`.
    ///
    /// # Errors
    /// `locked`, or the store's code.
    pub fn sync_status(&self) -> IpcResult<SyncOverview> {
        let slot = self.slot()?;
        Ok(self.sync_overview(slot.unlocked()?))
    }

    /// `sync_now`: runs a sync and answers when it ended (or after two
    /// minutes).
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`.
    pub fn sync_now(&self) -> IpcResult<SyncOverview> {
        {
            let slot = self.slot()?;
            if slot.unlocked()?.sync_config()?.is_none() {
                return Err(IpcError::new(ErrorCode::SyncNotConfigured));
            }
        }
        // Not user activity: the idle timer is not touched.
        self.sync.sync_now();
        self.sync_status()
    }

    /// `sync_disconnect`: signs out of the server (best effort), forgets the
    /// token and the backend choice. The local data stays.
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, or the store's code.
    pub fn sync_disconnect(&self) -> IpcResult<SyncOverview> {
        let (account, config) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let config = session
                .sync_config()?
                .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
            // A backend change is refused while shared vaults are reachable
            // (CRYPTO_SPEC §10.2): nothing is signed out or forgotten first.
            if sharing_flow::has_live_shared_vaults(session) {
                return Err(IpcError::new(ErrorCode::SyncHasSharedVaults));
            }
            (session.account_id(), config)
        };
        // The thread ends first: its handles fail from here on.
        self.sync.stop();
        if config.kind == SyncKind::Hosted
            && let (Some(url), Ok(Some(token))) =
                (config.server_url.as_deref(), self.sync_tokens.load(account))
        {
            // The server may be unreachable: the device forgets the session
            // either way.
            let _ = hosted::logout(url, &token);
        }
        self.sync_tokens.delete(account)?;
        {
            let mut slot = self.slot()?;
            slot.unlocked_mut()?.set_sync_config(None)?;
        }
        self.sync.reset();
        self.emit_sync_state();
        self.sync_status()
    }

    /// `sync_acknowledge`: asks with a **native** confirmation (Rust owns the
    /// dialog and its texts, English or German by the language setting; the
    /// webview can neither skip, reword nor answer it) whether the halt and the
    /// findings against the server are known. On "OK" they are cleared
    /// (`clear_sync_halt`, `clear_vault_findings`) and a sync starts; on
    /// "Cancel" nothing changes and the unchanged overview is returned.
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, or the store's code.
    pub fn sync_acknowledge(&self, _request: &AcknowledgeRequest) -> IpcResult<SyncOverview> {
        {
            let slot = self.slot()?;
            if slot.unlocked()?.sync_config()?.is_none() {
                return Err(IpcError::new(ErrorCode::SyncNotConfigured));
            }
        }
        let texts = strings::acknowledge(self.settings_snapshot().language);
        // The slot is not held while the dialog is open.
        if !self.dialogs.confirm(texts.title, &texts.message())? {
            return self.sync_status();
        }
        {
            let mut slot = self.slot()?;
            let session = slot.unlocked_mut()?;
            if session.sync_config()?.is_none() {
                return Err(IpcError::new(ErrorCode::SyncNotConfigured));
            }
            let findings = session.sync_findings()?;
            if findings.halt.is_some() {
                session.clear_sync_halt()?;
            }
            let vaults = findings.vaults();
            if !vaults.is_empty() {
                session.clear_vault_findings(&vaults)?;
            }
            session.clear_sharing_findings()?;
        }
        // What the findings showed is gone until a run finds it again.
        self.sync.update_view(|view| {
            view.warnings.retain(|w| {
                !matches!(
                    w,
                    SyncWarningView::RollbackDetected
                        | SyncWarningView::IdentityDuplicated
                        | SyncWarningView::ServerInconsistent { .. }
                        | SyncWarningView::ServerRollback { .. }
                        | SyncWarningView::NoCommonView { .. }
                )
            });
            true
        });
        self.sync.send(Cmd::Acknowledged);
        self.sync_status()
    }

    /// A blocking use of the session for a flow: `f` runs on the unlocked
    /// session with the slot held.
    pub(crate) fn with_unlocked<R>(
        &self,
        f: impl FnOnce(&mut UnlockedSession) -> IpcResult<R>,
    ) -> IpcResult<R> {
        let mut slot = self.slot()?;
        f(slot.unlocked_mut()?)
    }
}
