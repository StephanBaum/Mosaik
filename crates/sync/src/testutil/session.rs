//! A session holder for tests that can lock and unlock.

use mosaic_vault::session::UnlockedSession;

use crate::engine::{SessionAccess, SyncError};

/// Holds an unlocked session, or nothing while it is "locked". Implements
/// [`SessionAccess`].
#[derive(Debug)]
pub struct LocalSession(Option<UnlockedSession>);

impl LocalSession {
    /// Wraps an unlocked session.
    #[must_use]
    pub const fn new(session: UnlockedSession) -> Self {
        Self(Some(session))
    }

    /// The session, if it is unlocked.
    #[must_use]
    pub const fn session(&self) -> Option<&UnlockedSession> {
        self.0.as_ref()
    }

    /// The session, if it is unlocked.
    pub const fn session_mut(&mut self) -> Option<&mut UnlockedSession> {
        self.0.as_mut()
    }

    /// Takes the session out (the holder is then locked).
    pub const fn take(&mut self) -> Option<UnlockedSession> {
        self.0.take()
    }

    /// Puts a session in.
    pub fn set(&mut self, session: UnlockedSession) {
        self.0 = Some(session);
    }
}

impl SessionAccess for LocalSession {
    fn with<R>(&mut self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Result<R, SyncError> {
        self.0.as_mut().map(f).ok_or(SyncError::Locked)
    }
}
