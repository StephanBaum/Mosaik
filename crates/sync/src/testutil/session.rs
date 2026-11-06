//! A shared session holder for tests that can lock and unlock.

use std::sync::{Arc, Mutex};

use mosaic_vault::session::UnlockedSession;

use crate::engine::{SessionAccess, SyncError};
use crate::wire::SessionHandle;

#[expect(
    clippy::panic,
    clippy::unwrap_used,
    reason = "test utilities: a failure here is a failed test"
)]
mod guards {
    use std::ops::{Deref, DerefMut};
    use std::sync::{Mutex, MutexGuard, TryLockError};

    use mosaic_vault::session::UnlockedSession;

    /// A read guard on the session of a [`LocalSession`].
    pub struct SessionRef<'a>(pub(super) MutexGuard<'a, Option<UnlockedSession>>);

    impl Deref for SessionRef<'_> {
        type Target = UnlockedSession;

        fn deref(&self) -> &UnlockedSession {
            self.0.as_ref().unwrap()
        }
    }

    /// A write guard on the session of a [`LocalSession`].
    pub struct SessionMut<'a>(pub(super) MutexGuard<'a, Option<UnlockedSession>>);

    impl Deref for SessionMut<'_> {
        type Target = UnlockedSession;

        fn deref(&self) -> &UnlockedSession {
            self.0.as_ref().unwrap()
        }
    }

    impl DerefMut for SessionMut<'_> {
        fn deref_mut(&mut self) -> &mut UnlockedSession {
            self.0.as_mut().unwrap()
        }
    }

    pub fn lock(lock: &Mutex<Option<UnlockedSession>>) -> MutexGuard<'_, Option<UnlockedSession>> {
        match lock.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => {
                panic!("the session is in use: a lock was held across a call that needs it")
            }
            Err(TryLockError::Poisoned(_)) => panic!("session lock poisoned"),
        }
    }
}

pub use guards::{SessionMut, SessionRef};

/// Holds an unlocked session, or nothing while it is "locked". Clones share
/// the session, like the desktop's `Arc<Mutex<Option<UnlockedSession>>>`:
/// the folder transport gets a clone as its [`SessionHandle`]
/// ([`SessionCrypto`](crate::wire::SessionCrypto)), the engine uses the
/// holder as its [`SessionAccess`], and locking (`take`) ends both.
///
/// A step that needs the session while another guard is alive is a bug of
/// the engine (it must never hold the session across a transport call) or of
/// the test (two guards of one device in one expression), and it panics
/// instead of waiting forever.
#[derive(Clone, Debug)]
pub struct LocalSession(Arc<Mutex<Option<UnlockedSession>>>);

impl LocalSession {
    /// Wraps an unlocked session.
    #[must_use]
    pub fn new(session: UnlockedSession) -> Self {
        Self(Arc::new(Mutex::new(Some(session))))
    }

    /// The session, if it is unlocked.
    #[must_use]
    pub fn session(&self) -> Option<SessionRef<'_>> {
        let guard = guards::lock(&self.0);
        guard.is_some().then_some(SessionRef(guard))
    }

    /// The session with write access, if it is unlocked.
    #[must_use]
    pub fn session_mut(&self) -> Option<SessionMut<'_>> {
        let guard = guards::lock(&self.0);
        guard.is_some().then_some(SessionMut(guard))
    }

    /// Takes the session out (the holder is then locked, for every clone).
    #[must_use]
    pub fn take(&self) -> Option<UnlockedSession> {
        guards::lock(&self.0).take()
    }

    /// Puts a session in.
    pub fn set(&self, session: UnlockedSession) {
        *guards::lock(&self.0) = Some(session);
    }
}

impl SessionAccess for LocalSession {
    fn with<R>(&mut self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Result<R, SyncError> {
        guards::lock(&self.0)
            .as_mut()
            .map(f)
            .ok_or(SyncError::Locked)
    }
}

impl SessionHandle for LocalSession {
    fn with_session<R>(&self, f: impl FnOnce(&UnlockedSession) -> R) -> Option<R> {
        guards::lock(&self.0).as_ref().map(f)
    }

    fn with_session_mut<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Option<R> {
        guards::lock(&self.0).as_mut().map(f)
    }
}
