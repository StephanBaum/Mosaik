//! How the sync thread reaches the vault session: over the backend's session
//! slot, one closure at a time. The slot is locked for that closure only and
//! never across a network call (the engine and the transports guarantee it),
//! and a handle belongs to one service generation: after a lock, a
//! disconnect or a restart the older handles fail closed.

use std::sync::{Arc, Weak};

use mosaic_sync::engine::{SessionAccess, SyncError};
use mosaic_sync::wire::SessionHandle;
use mosaic_vault::session::UnlockedSession;

use crate::state::{Backend, Slot};

/// The session of one sync service generation. Cheap to clone.
#[derive(Clone)]
pub struct BackendSession {
    backend: Weak<Backend>,
    generation: u64,
}

impl BackendSession {
    /// A handle for `generation`.
    #[must_use]
    pub const fn new(backend: Weak<Backend>, generation: u64) -> Self {
        Self {
            backend,
            generation,
        }
    }

    /// The backend, if it lives and this generation is still the current
    /// one.
    pub(crate) fn current(&self) -> Option<Arc<Backend>> {
        let backend = self.backend.upgrade()?;
        backend.sync.is_current(self.generation).then_some(backend)
    }

    /// Runs `f` on the unlocked session; `None` if the app is locked, the
    /// generation ended or the slot is busy for good (poisoned).
    fn run<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Option<R> {
        let backend = self.current()?;
        let mut slot = backend.slot().ok()?;
        let out = match &mut *slot {
            Slot::Unlocked(session) => f(session),
            Slot::Unavailable(_) | Slot::Locked(_) | Slot::Taken => return None,
        };
        // A batch whose commit failed and whose reload failed too leaves a
        // session that may disagree with the store: lock it, so the next
        // unlock rebuilds it from disk.
        if matches!(&*slot, Slot::Unlocked(session) if session.needs_relock())
            && let Slot::Unlocked(session) = std::mem::replace(&mut *slot, Slot::Taken)
        {
            *slot = Slot::Locked((*session).lock());
        }
        Some(out)
    }
}

impl SessionHandle for BackendSession {
    fn with_session<R>(&self, f: impl FnOnce(&UnlockedSession) -> R) -> Option<R> {
        self.run(|session| f(session))
    }

    fn with_session_mut<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Option<R> {
        self.run(f)
    }
}

impl SessionAccess for BackendSession {
    fn with<R>(&mut self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Result<R, SyncError> {
        self.run(f).ok_or(SyncError::Locked)
    }
}
