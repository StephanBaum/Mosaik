//! Server-side OPAQUE helpers (CRYPTO_SPEC §9): the in-memory state between
//! `login/start` and `login/finish`, and the parameters both steps share.

use std::collections::HashMap;
use std::sync::Mutex;

use mosaic_sync::wire::opaque::{
    LOGIN_CONTEXT, Suite, identifiers,
    ke::{ServerLogin, ServerLoginParameters},
};

/// How long a login state lives.
pub const LOGIN_TTL_MS: u64 = 60_000;
/// Login states held at once; the oldest is evicted beyond it.
pub const MAX_LOGIN_STATES: usize = 10_000;

/// A login in progress.
pub struct LoginEntry {
    /// The OPAQUE server state (KE2 sent, KE3 awaited).
    pub state: ServerLogin<Suite>,
    /// The account, `None` for a fake (unknown email).
    pub account_id: Option<[u8; 16]>,
    /// The identifier the OPAQUE messages were bound to.
    pub credential_id: [u8; 16],
    created_ms: u64,
}

/// The pending logins, in memory only.
#[derive(Default)]
pub struct LoginStates {
    map: Mutex<HashMap<[u8; 16], LoginEntry>>,
}

impl LoginStates {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a login under `id`.
    pub fn insert(
        &self,
        id: [u8; 16],
        state: ServerLogin<Suite>,
        account_id: Option<[u8; 16]>,
        credential_id: [u8; 16],
        now_ms: u64,
    ) {
        let mut map = self
            .map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if map.len() >= MAX_LOGIN_STATES {
            map.retain(|_, e| now_ms.saturating_sub(e.created_ms) < LOGIN_TTL_MS);
        }
        while map.len() >= MAX_LOGIN_STATES {
            let oldest = map
                .iter()
                .min_by_key(|(_, e)| e.created_ms)
                .map(|(k, _)| *k);
            match oldest {
                Some(k) => map.remove(&k),
                None => break,
            };
        }
        map.insert(
            id,
            LoginEntry {
                state,
                account_id,
                credential_id,
                created_ms: now_ms,
            },
        );
    }

    /// Takes the login `id` out (each state is used once); `None` if it is
    /// unknown or expired.
    pub fn take(&self, id: &[u8; 16], now_ms: u64) -> Option<LoginEntry> {
        let entry = self
            .map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id)?;
        (now_ms.saturating_sub(entry.created_ms) < LOGIN_TTL_MS).then_some(entry)
    }

    /// Drops expired states.
    pub fn prune(&self, now_ms: u64) {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|_, e| now_ms.saturating_sub(e.created_ms) < LOGIN_TTL_MS);
    }
}

/// The server-side parameters of login start and finish.
#[must_use]
pub const fn login_params<'a>(
    credential_id: &'a [u8; 16],
    origin: &'a str,
) -> ServerLoginParameters<'static, 'a> {
    ServerLoginParameters {
        context: Some(LOGIN_CONTEXT),
        identifiers: identifiers(credential_id.as_slice(), origin.as_bytes()),
    }
}
