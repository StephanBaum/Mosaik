//! A session the caller owns for the length of one sync, for a command-line
//! tool that opens a store, syncs it as its own device and closes it
//! (M8, ADR-0071).
//!
//! The desktop keeps its session behind its own state and gives the engine a
//! `SessionAccess` over that. A caller without such a state wraps the
//! unlocked session in an [`OwnedSession`] instead: the engine uses the
//! holder as its [`SessionAccess`], the transport gets a clone as its
//! [`SessionHandle`] (for the hosted and folder transports' crypto), and
//! [`OwnedSession::take`] hands the session back when the sync is over.
//! Clones share one session; taking it out locks every clone.

use std::sync::{Arc, Mutex};

use mosaic_vault::session::UnlockedSession;

use super::{SessionAccess, SyncError};
use crate::wire::SessionHandle;

/// An unlocked session shared between the engine and its transport for the
/// length of one sync. See the [module documentation](self).
#[derive(Clone)]
pub struct OwnedSession(Arc<Mutex<Option<UnlockedSession>>>);

impl core::fmt::Debug for OwnedSession {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OwnedSession").finish_non_exhaustive()
    }
}

impl OwnedSession {
    /// Wraps an unlocked session.
    #[must_use]
    pub fn new(session: UnlockedSession) -> Self {
        Self(Arc::new(Mutex::new(Some(session))))
    }

    /// Takes the session out. The holder is locked from then on, for every
    /// clone: the engine and the transport fail with [`SyncError::Locked`]
    /// instead of using a session the caller has taken back. `None` if it
    /// was taken before (or the holder is poisoned: fail closed).
    #[must_use]
    pub fn take(&self) -> Option<UnlockedSession> {
        self.0.lock().ok()?.take()
    }

    /// Whether the session is still inside.
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.0.lock().is_ok_and(|g| g.is_some())
    }
}

impl SessionAccess for OwnedSession {
    fn with<R>(&mut self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Result<R, SyncError> {
        // A poisoned lock counts as locked: fail closed.
        let mut guard = self.0.lock().map_err(|_| SyncError::Locked)?;
        guard.as_mut().map(f).ok_or(SyncError::Locked)
    }
}

impl SessionHandle for OwnedSession {
    fn with_session<R>(&self, f: impl FnOnce(&UnlockedSession) -> R) -> Option<R> {
        self.0.lock().ok()?.as_ref().map(f)
    }

    fn with_session_mut<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Option<R> {
        self.0.lock().ok()?.as_mut().map(f)
    }
}
