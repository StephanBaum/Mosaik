//! App state: the vault session behind a mutex, plus the small pieces of
//! state the commands share. Commands run their work on Tauri's blocking
//! pool ([`AppState::run`]), so Argon2id and SQLite never block the event
//! loop, and they lock the session for the whole operation, so two commands
//! never see a half-changed session.

use std::sync::{Arc, Mutex, MutexGuard};

use mosaic_core::{Argon2Profile, SecretString};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use tauri::ipc::Channel;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::settings::Settings;
use crate::keystore::SecretKeyStore;

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

/// Everything the commands share.
pub struct Backend {
    session: Mutex<Slot>,
    /// The Secret Key's home on this device (CORE_API: the OS keystore).
    pub secret_keys: Box<dyn SecretKeyStore>,
    /// The account email, shown on the unlock screen. Not secret; the
    /// settings track persists it with the settings.
    pub email: Mutex<Option<String>>,
    /// App settings.
    pub settings: Mutex<Settings>,
    /// The password `generate_password` returned last, for copying it from
    /// Rust. Dropped (zeroized) on the next generation and on lock.
    pub last_generated: Mutex<Option<SecretString>>,
    events: Mutex<Option<Channel<AppEvent>>>,
    /// Argon2id profile choice for new accounts.
    pub choose_profile: ProfileChooser,
}

impl Backend {
    /// A backend over an opened (locked) session, or an unavailable store.
    #[must_use]
    pub fn new(
        session: Slot,
        secret_keys: Box<dyn SecretKeyStore>,
        choose_profile: ProfileChooser,
    ) -> Self {
        Self {
            session: Mutex::new(session),
            secret_keys,
            email: Mutex::new(None),
            settings: Mutex::new(Settings::default()),
            last_generated: Mutex::new(None),
            events: Mutex::new(None),
            choose_profile,
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

    /// Sets the event channel (the latest subscriber wins).
    ///
    /// # Errors
    /// `internal` on a poisoned mutex.
    pub fn subscribe(&self, channel: Channel<AppEvent>) -> IpcResult<()> {
        *guard(&self.events)? = Some(channel);
        Ok(())
    }

    /// Sends an event to the webview, if it subscribed. Delivery failures
    /// (a reloading webview) are ignored: events are notifications, and the
    /// webview re-reads the state after subscribing.
    pub fn emit(&self, event: &AppEvent) {
        if let Ok(guard) = self.events.lock()
            && let Some(channel) = guard.as_ref()
        {
            let _ = channel.send(event.clone());
        }
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
