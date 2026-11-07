//! Sync in the desktop app (M6, ADR-0042 to ADR-0046): the service that runs
//! the sync engine, and the flows that connect, join and disconnect.
//!
//! - [`SyncService`] (this module) owns the sync thread of the unlocked app:
//!   it starts on unlock when sync is configured and stops on lock (every
//!   transport operation then fails closed with `Locked`; the engine and the
//!   transport are dropped, and the next unlock rebuilds them from the
//!   persisted engine state). Each start is a generation: a thread of an
//!   older generation can neither reach the session nor change the state.
//! - [`access`]: the session handle of one generation.
//! - [`runner`]: the loop of the thread.
//! - [`hosted_flow`] and [`folder_flow`]: the commands for the two backends.
//!
//! The token of the hosted session lives in the keystore
//! ([`crate::keystore::SyncTokenStore`]); folder paths live in the vault
//! store and never reach the webview except as text to display (ADR-0034).

pub mod access;
mod errors;
mod folder_flow;
mod hosted_flow;
mod runner;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use mosaic_core::{AccountId, InstallId};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, SyncEngine};
use mosaic_sync::folder::{FolderConfig, FolderTransport};
use mosaic_sync::http::{HttpTransport, hosted};
use mosaic_sync::transport::{TransportError, TransportKind};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_vault::session::{SyncConfig, SyncKind, UnlockedSession};

pub use errors::{sync_code, transport_code};
pub(crate) use hosted_flow::PendingLogin;

use self::access::BackendSession;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, SyncState, SyncStatus};
use crate::contract::sync::{SyncConfigView, SyncOverview, SyncWarningView};
use crate::state::{Backend, Slot, guard};

/// How long a chosen folder stays valid (`choiceId`, ADR-0034).
pub const CHOICE_TTL: Duration = Duration::from_mins(5);
/// The most folder choices held at once.
const MAX_CHOICES: usize = 8;
/// How long "Sync now" waits for the run to end before it answers.
const SYNC_NOW_WAIT: Duration = Duration::from_secs(120);

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
}

impl Default for View {
    fn default() -> Self {
        Self {
            status: SyncStatus::NotConfigured,
            last_synced_at: None,
            error: None,
            warnings: Vec::new(),
            totp_enabled: None,
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
    /// When the last fresh login of this process ended (a second factor
    /// that needs two codes reuses it for five minutes).
    fresh_at: Mutex<Option<Instant>>,
    /// Added to the clock that expires choices and logins (tests move it).
    skew: Mutex<Duration>,
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
            fresh_at: Mutex::new(None),
            skew: Mutex::new(Duration::ZERO),
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
        self.stop();
        self.update_view(|view| {
            *view = View::default();
            true
        });
        if let Ok(mut choices) = self.choices.lock() {
            choices.clear();
        }
        if let Ok(mut pending) = self.pending_login.lock() {
            *pending = None;
        }
        if let Ok(mut fresh) = self.fresh_at.lock() {
            *fresh = None;
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

    pub(crate) fn set_pending(&self, pending: Option<PendingLogin>) {
        if let Ok(mut slot) = self.pending_login.lock() {
            *slot = pending;
        }
    }

    pub(crate) fn take_pending(&self) -> Option<PendingLogin> {
        self.pending_login.lock().ok().and_then(|mut p| p.take())
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
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(240))
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
                backend.sync_signed_out();
                return;
            };
            // Best effort: whether the account has a second factor (for the
            // settings; a failure leaves it unknown).
            drop(backend);
            if let Ok(info) = hosted::account_info(url, &token)
                && let Some(backend) = ctx.session().current()
            {
                backend.sync.update_view(|view| {
                    view.totp_enabled = Some(info.totp_enabled);
                    true
                });
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
                .map(|t| SyncEngine::new(t, EngineConfig::for_kind(TransportKind::Folder)))
            };
            runner::run(ctx, make, rx);
        }
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
    /// user signs in again.
    pub(crate) fn sync_signed_out(&self) {
        if let Some(account) = self.account() {
            let _ = self.sync_tokens.delete(account.account_id.get());
        }
        self.sync.update_view(|view| {
            view.status = SyncStatus::SignedOut;
            view.error = Some(ErrorCode::SyncSignedOut);
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
            };
        }
        SyncOverview {
            config,
            state: self.sync.state(),
            warnings: self.sync.view().warnings,
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

    /// `sync_acknowledge`: the user confirmed (the UI asked first) that the
    /// halt and the findings against the server are known. They are cleared
    /// (`clear_sync_halt`, `clear_vault_findings`) and a sync starts.
    ///
    /// # Errors
    /// `locked`, `syncNotConfigured`, or the store's code.
    pub fn sync_acknowledge(&self) -> IpcResult<SyncOverview> {
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
