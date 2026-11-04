//! App state: the vault session behind a mutex, plus the pieces the
//! commands share (keystore, settings, clipboard, events, activity,
//! adapters). Commands run their work on Tauri's blocking pool
//! ([`AppState::run`]), so Argon2id, SQLite and OS prompts never block the
//! event loop, and they lock the session for the whole operation, so two
//! commands never see a half-changed session.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use mosaic_core::{AccountId, Argon2Profile, SecretString};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use tauri::ipc::Channel;

use crate::adapters::{Adapters, EventSink};
use crate::autolock::Activity;
use crate::clipboard::{BackendFactory, ClipboardService, MemoryClipboard};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, LockReason};
use crate::contract::settings::Settings;
use crate::dialogs::{Dialogs, NoDialogs};
use crate::events::Events;
use crate::keystore::SecretKeyStore;
use crate::otp_pending::PendingOtps;
use crate::persist::{AccountInfo, Persistence, StoredState};
use crate::quick_unlock::QuickUnlockService;
use crate::screen_capture::{NoScreenCapture, ScreenCapture};
use crate::settings_integrity::SettingsGuard;
use crate::shortcut::ShortcutStatus;
use crate::unlock_throttle::UnlockThrottle;

/// Where the vault session is.
pub enum Slot {
    /// The store could not be opened (for example `StoreInUse`); every
    /// command fails with this code.
    Unavailable(ErrorCode),
    /// Keys are not in memory.
    Locked(LockedSession),
    /// Unlocked: the only state that holds keys and decrypted items.
    Unlocked(Box<UnlockedSession>),
    /// Placeholder while a command owns the session (never visible to
    /// another command, which waits for the mutex instead).
    Taken,
}

impl Slot {
    /// The unlocked session, or the error for this state.
    ///
    /// # Errors
    /// `locked`, or the code of an unavailable store.
    pub fn unlocked(&self) -> IpcResult<&UnlockedSession> {
        match self {
            Self::Unlocked(session) => Ok(session),
            Self::Locked(_) => Err(IpcError::LOCKED),
            Self::Unavailable(code) => Err(IpcError::new(*code)),
            Self::Taken => Err(IpcError::new(ErrorCode::Busy)),
        }
    }

    /// The unlocked session for writing, or the error for this state.
    ///
    /// # Errors
    /// As [`Slot::unlocked`].
    pub fn unlocked_mut(&mut self) -> IpcResult<&mut UnlockedSession> {
        match self {
            Self::Unlocked(session) => Ok(session),
            Self::Locked(_) => Err(IpcError::LOCKED),
            Self::Unavailable(code) => Err(IpcError::new(*code)),
            Self::Taken => Err(IpcError::new(ErrorCode::Busy)),
        }
    }
}

/// How account creation picks the Argon2id profile (CRYPTO_SPEC §5.3). The
/// app calibrates on the device; tests pass the cheapest profile.
pub type ProfileChooser = fn() -> mosaic_core::Result<Argon2Profile>;

/// Flags the main window for attention.
pub type Attention = Arc<dyn Fn() + Send + Sync>;

/// Called after every lock (for example to hide the quick-search window).
pub type LockHook = Box<dyn Fn(LockReason) + Send + Sync>;

/// Files chosen for import, behind opaque tokens (`import_choose_file`).
#[derive(Default)]
pub struct FileTokens {
    next: u64,
    paths: BTreeMap<String, PathBuf>,
}

impl FileTokens {
    /// Remembers `path` and returns its token.
    pub fn insert(&mut self, path: PathBuf) -> String {
        self.next = self.next.saturating_add(1);
        let token = format!("file-{}", self.next);
        self.paths.insert(token.clone(), path);
        token
    }

    /// The path behind `token`.
    #[must_use]
    pub fn get(&self, token: &str) -> Option<&PathBuf> {
        self.paths.get(token)
    }

    /// Forgets every token.
    pub fn clear(&mut self) {
        self.paths.clear();
    }
}

/// Everything the commands share.
pub struct Backend {
    session: Mutex<Slot>,
    /// The Secret Key's home on this device (CORE_API: the OS keystore).
    pub secret_keys: Box<dyn SecretKeyStore>,
    pub(crate) stored: Mutex<StoredState>,
    pub(crate) persistence: Persistence,
    /// Backoff between wrong master passwords.
    pub(crate) unlock_throttle: Mutex<UnlockThrottle>,
    /// The clock the throttle reads (a test replaces it).
    pub(crate) unlock_clock: fn() -> Instant,
    /// Wrong Emergency Kit answers since the last unlock.
    pub(crate) kit_wrong: Mutex<u8>,
    /// The settings MAC state (ADR-0030).
    pub(crate) settings_guard: Mutex<SettingsGuard>,
    /// The password `generate_password` returned last, for copying it from
    /// Rust. Dropped (zeroized) on the next generation and on lock.
    pub last_generated: Mutex<Option<SecretString>>,
    /// Events to the webview.
    pub events: Events,
    /// The clipboard worker.
    pub clipboard: ClipboardService,
    /// The last user action, for auto-lock.
    pub activity: Activity,
    /// Argon2id profile choice for new accounts.
    pub choose_profile: ProfileChooser,
    /// Quick-unlock platforms and the re-authentication grant (CRYPTO_SPEC
    /// §6.2, §6.3).
    pub quick_unlock: QuickUnlockService,
    /// Import, insights and SSH agent hooks.
    pub adapters: Adapters,
    /// Native file dialogs.
    pub dialogs: Box<dyn Dialogs>,
    /// Files chosen for import.
    pub import_files: Mutex<FileTokens>,
    /// "Scan screen" (ADR-0031); `NoScreenCapture` until the app sets the
    /// native one.
    pub screen: Box<dyn ScreenCapture>,
    /// Parsed OTP setups waiting for confirmation (ADR-0031 item 2). Cleared
    /// on lock.
    pub(crate) otp_pending: Mutex<PendingOtps>,
    /// Set while an `otp_scan` runs (one at a time).
    pub(crate) otp_scanning: AtomicBool,
    /// Whether the global shortcut is registered (`None` until tried).
    pub shortcut_status: Mutex<Option<ShortcutStatus>>,
    /// Asks the OS to flag the window (taskbar attention) when an SSH
    /// approval arrives. It never focuses the window.
    attention: Option<Attention>,
    /// The OS event sources that started (auto-lock on sleep and screen lock).
    pub os_sources: Mutex<Vec<&'static str>>,
    /// The account whose Emergency Kit was not yet confirmed in this process
    /// (set by `create_account`, cleared by a matching `verify_emergency_kit`
    /// and by every lock). While it matches, the kit saves without the password.
    pub(crate) kit_pending: Mutex<Option<AccountId>>,
    lock_hooks: Mutex<Vec<LockHook>>,
}

impl Backend {
    /// A backend over an opened (locked) session, or an unavailable store,
    /// with in-memory settings, an in-memory clipboard, no dialogs, no
    /// quick unlock and the stub adapters. The app replaces those with the
    /// `with_*` methods.
    #[must_use]
    pub fn new(
        session: Slot,
        secret_keys: Box<dyn SecretKeyStore>,
        choose_profile: ProfileChooser,
    ) -> Self {
        let events = Events::default();
        Self {
            session: Mutex::new(session),
            secret_keys,
            stored: Mutex::new(StoredState::default()),
            persistence: Persistence::memory(),
            settings_guard: Mutex::new(SettingsGuard::default()),
            unlock_throttle: Mutex::new(UnlockThrottle::default()),
            unlock_clock: Instant::now,
            kit_wrong: Mutex::new(0),
            last_generated: Mutex::new(None),
            clipboard: ClipboardService::spawn(
                MemoryClipboard::default().factory(),
                events.clone(),
            ),
            events,
            activity: Activity::default(),
            choose_profile,
            quick_unlock: QuickUnlockService::default(),
            adapters: Adapters::default(),
            dialogs: Box::new(NoDialogs),
            import_files: Mutex::new(FileTokens::default()),
            screen: Box::new(NoScreenCapture),
            otp_pending: Mutex::new(PendingOtps::default()),
            otp_scanning: AtomicBool::new(false),
            shortcut_status: Mutex::new(None),
            attention: None,
            os_sources: Mutex::new(Vec::new()),
            kit_pending: Mutex::new(None),
            lock_hooks: Mutex::new(Vec::new()),
        }
    }

    /// Loads and keeps the settings and account in `persistence`.
    #[must_use]
    pub fn with_persistence(mut self, persistence: Persistence) -> Self {
        let (state, file) = persistence.load_checked();
        self.stored = Mutex::new(state);
        self.settings_guard = Mutex::new(SettingsGuard::new(file));
        self.persistence = persistence;
        self
    }

    /// Uses this clipboard backend.
    #[must_use]
    pub fn with_clipboard(mut self, factory: BackendFactory) -> Self {
        self.clipboard = ClipboardService::spawn(factory, self.events.clone());
        self
    }

    /// Uses these dialogs.
    #[must_use]
    pub fn with_dialogs(mut self, dialogs: Box<dyn Dialogs>) -> Self {
        self.dialogs = dialogs;
        self
    }

    /// Uses this screen capture.
    #[must_use]
    pub fn with_screen_capture(mut self, screen: Box<dyn ScreenCapture>) -> Self {
        self.screen = screen;
        self
    }

    /// Uses these adapters.
    #[must_use]
    pub fn with_adapters(mut self, adapters: Adapters) -> Self {
        self.adapters = adapters;
        self
    }

    /// Uses these quick-unlock platforms.
    #[must_use]
    pub fn with_quick_unlock(mut self, quick_unlock: QuickUnlockService) -> Self {
        self.quick_unlock = quick_unlock;
        self
    }

    /// Calls `hook` when an SSH approval request arrives.
    #[must_use]
    pub fn with_attention(mut self, hook: Attention) -> Self {
        self.attention = Some(hook);
        self
    }

    /// Runs `hook` after every lock.
    pub fn on_lock(&self, hook: LockHook) {
        if let Ok(mut hooks) = self.lock_hooks.lock() {
            hooks.push(hook);
        }
    }

    pub(crate) fn run_lock_hooks(&self, reason: LockReason) {
        if let Ok(hooks) = self.lock_hooks.lock() {
            for hook in hooks.iter() {
                hook(reason);
            }
        }
    }

    /// Locks the session slot for one operation.
    ///
    /// # Errors
    /// `internal` if a previous holder panicked (debug builds; release
    /// builds abort on panic).
    pub fn slot(&self) -> IpcResult<MutexGuard<'_, Slot>> {
        self.session.lock().map_err(|_| IpcError::INTERNAL)
    }

    /// Whether the vault is unlocked right now (no wait if a command holds
    /// the session: then it is busy, which counts as not idle).
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.session
            .try_lock()
            .is_ok_and(|slot| matches!(*slot, Slot::Unlocked(_)))
    }

    /// Sets the event channel (the latest subscriber wins).
    ///
    /// # Errors
    /// `internal` on a poisoned mutex.
    pub fn subscribe(&self, channel: Channel<AppEvent>) -> IpcResult<()> {
        self.events.subscribe(channel)
    }

    /// Sends an event to the webview, if it subscribed.
    pub fn emit(&self, event: &AppEvent) {
        self.events.emit(event);
    }

    /// Records a user action (auto-lock).
    pub fn touch(&self) {
        self.activity.touch();
    }

    /// The current settings.
    #[must_use]
    pub fn settings_snapshot(&self) -> Settings {
        self.stored
            .lock()
            .map(|s| s.settings.clone())
            .unwrap_or_default()
    }

    /// Everything in the state file.
    #[must_use]
    pub fn stored(&self) -> StoredState {
        self.stored.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Sends events to the webview from background threads (the SSH agent).
    #[must_use]
    pub fn event_sink(&self) -> EventSink {
        let events = self.events.clone();
        let attention = self.attention.clone();
        Arc::new(move |event| {
            if let (AppEvent::SshApprovalRequested { .. }, Some(attention)) = (&event, &attention) {
                attention();
            }
            events.emit(&event);
        })
    }

    /// Tells the SSH agent the items changed. Call it after every
    /// successful item change, while the session is still held.
    pub(crate) fn items_changed(&self, session: &UnlockedSession) {
        self.adapters.ssh.keys_changed(session);
    }

    /// The account of this install, as recorded in the state file.
    #[must_use]
    pub fn account(&self) -> Option<AccountInfo> {
        self.stored.lock().ok().and_then(|s| s.account.clone())
    }

    /// Changes and saves the stored state. Takes the session slot itself:
    /// callers that already hold it use [`Backend::update_stored_in`].
    ///
    /// # Errors
    /// `internal` on a poisoned mutex; `storeUnavailable` if the file cannot
    /// be written (the change stays in memory).
    pub fn update_stored<F: FnOnce(&mut StoredState)>(&self, change: F) -> IpcResult<StoredState> {
        let mut slot = self.slot()?;
        self.update_stored_in(&mut slot, change)
    }

    /// [`Backend::update_stored`] for a caller that holds the slot. The MAC'd
    /// write replaces the settings nonce in the store, which needs the
    /// session, and holding the slot also serializes concurrent saves.
    ///
    /// # Errors
    /// As [`Backend::update_stored`].
    pub(crate) fn update_stored_in<F: FnOnce(&mut StoredState)>(
        &self,
        slot: &mut Slot,
        change: F,
    ) -> IpcResult<StoredState> {
        let mut stored = guard(&self.stored)?;
        change(&mut stored);
        let snapshot = stored.clone();
        drop(stored);
        self.save_state(slot, &snapshot)?;
        Ok(snapshot)
    }
}

/// Locks a mutex, mapping poisoning to `internal`.
///
/// # Errors
/// `internal` if a previous holder panicked.
pub fn guard<T>(mutex: &Mutex<T>) -> IpcResult<MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| IpcError::INTERNAL)
}

/// The Tauri-managed state.
#[derive(Clone)]
pub struct AppState(Arc<Backend>);

impl AppState {
    /// Wraps a backend.
    #[must_use]
    pub fn new(backend: Backend) -> Self {
        Self(Arc::new(backend))
    }

    /// The backend, for synchronous work.
    #[must_use]
    pub fn backend(&self) -> &Backend {
        &self.0
    }

    /// A weak handle for background threads (they end with the app).
    #[must_use]
    pub fn downgrade(&self) -> std::sync::Weak<Backend> {
        Arc::downgrade(&self.0)
    }

    /// Runs `work` on the blocking pool.
    ///
    /// # Errors
    /// What `work` returns; `internal` if the task could not run.
    pub async fn run<T, F>(&self, work: F) -> IpcResult<T>
    where
        F: FnOnce(&Backend) -> IpcResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let backend = Arc::clone(&self.0);
        tauri::async_runtime::spawn_blocking(move || work(&backend))
            .await
            .map_err(|_| IpcError::INTERNAL)?
    }
}
