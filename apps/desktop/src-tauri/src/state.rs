//! App state: the vault session behind a mutex, plus the pieces the
//! commands share (keystore, settings, clipboard, events, activity,
//! adapters). Commands run their work on Tauri's blocking pool
//! ([`AppState::run`]), so Argon2id, SQLite and OS prompts never block the
//! event loop, and they lock the session for the whole operation, so two
//! commands never see a half-changed session.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use mosaic_core::{AccountId, Argon2Profile, SecretString};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use tauri::ipc::Channel;

use crate::adapters::Adapters;
use crate::autolock::Activity;
use crate::clipboard::{BackendFactory, ClipboardService, MemoryClipboard};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::{AppEvent, LockReason};
use crate::contract::settings::Settings;
use crate::dialogs::{Dialogs, NoDialogs};
use crate::events::Events;
use crate::keystore::SecretKeyStore;
use crate::persist::{AccountInfo, Persistence, StoredState};
use crate::quick_unlock::QuickUnlock;
use crate::shortcut::ShortcutStatus;

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
    stored: Mutex<StoredState>,
    persistence: Persistence,
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
    /// Quick-unlock methods (CRYPTO_SPEC §6.2).
    pub quick_unlock: QuickUnlock,
    /// Import, insights and SSH agent hooks.
    pub adapters: Adapters,
    /// Native file dialogs.
    pub dialogs: Box<dyn Dialogs>,
    /// Files chosen for import.
    pub import_files: Mutex<FileTokens>,
    /// Whether the global shortcut is registered (`None` until tried).
    pub shortcut_status: Mutex<Option<ShortcutStatus>>,
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
            last_generated: Mutex::new(None),
            clipboard: ClipboardService::spawn(
                MemoryClipboard::default().factory(),
                events.clone(),
            ),
            events,
            activity: Activity::default(),
            choose_profile,
            quick_unlock: QuickUnlock::default(),
            adapters: Adapters::default(),
            dialogs: Box::new(NoDialogs),
            import_files: Mutex::new(FileTokens::default()),
            shortcut_status: Mutex::new(None),
            kit_pending: Mutex::new(None),
            lock_hooks: Mutex::new(Vec::new()),
        }
    }

    /// Loads and keeps the settings and account in `persistence`.
    #[must_use]
    pub fn with_persistence(mut self, persistence: Persistence) -> Self {
        self.stored = Mutex::new(persistence.load());
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

    /// Uses these adapters.
    #[must_use]
    pub fn with_adapters(mut self, adapters: Adapters) -> Self {
        self.adapters = adapters;
        self
    }

    /// Uses these quick-unlock methods.
    #[must_use]
    pub fn with_quick_unlock(mut self, quick_unlock: QuickUnlock) -> Self {
        self.quick_unlock = quick_unlock;
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

    /// The account of this install, as recorded in the state file.
    #[must_use]
    pub fn account(&self) -> Option<AccountInfo> {
        self.stored.lock().ok().and_then(|s| s.account.clone())
    }

    /// Changes and saves the stored state.
    ///
    /// # Errors
    /// `internal` on a poisoned mutex; `storeUnavailable` if the file cannot
    /// be written (the change stays in memory).
    pub fn update_stored<F: FnOnce(&mut StoredState)>(&self, change: F) -> IpcResult<StoredState> {
        let mut stored = guard(&self.stored)?;
        change(&mut stored);
        let snapshot = stored.clone();
        drop(stored);
        self.persistence.save(&snapshot)?;
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
