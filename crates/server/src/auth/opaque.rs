//! Server-side OPAQUE helpers (CRYPTO_SPEC §9): the in-memory state between
//! `login/start` and `login/finish`, and the parameters both steps share.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

use mosaic_sync::wire::opaque::{
    LOGIN_CONTEXT, Suite, identifiers,
    ke::{ServerLogin, ServerLoginParameters},
};

/// How long a login state lives.
pub const LOGIN_TTL_MS: u64 = 60_000;
/// Login states held at once; the oldest is evicted beyond it.
pub const MAX_LOGIN_STATES: usize = 10_000;

/// Pending logins one IP prefix may hold; its oldest is evicted beyond it.
pub const MAX_LOGINS_PER_IP: usize = 16;

/// A login in progress.
pub struct LoginEntry {
    /// The OPAQUE server state (KE2 sent, KE3 awaited).
    pub state: ServerLogin<Suite>,
    /// The account, `None` for a fake (unknown email).
    pub account_id: Option<[u8; 16]>,
    /// The account version when the login started: `login/finish` refuses
    /// if a password change moved it meanwhile.
    pub account_version: u64,
    /// The identifier the OPAQUE messages were bound to.
    pub credential_id: [u8; 16],
    created_ms: u64,
    order: (u64, u64),
    ip: Vec<u8>,
}

#[derive(Default)]
struct Inner {
    map: HashMap<[u8; 16], LoginEntry>,
    /// Oldest first: `(created_ms, sequence)`.
    by_time: BTreeMap<(u64, u64), [u8; 16]>,
    per_ip: HashMap<Vec<u8>, BTreeSet<(u64, u64)>>,
    seq: u64,
}

impl Inner {
    fn remove(&mut self, id: &[u8; 16]) -> Option<LoginEntry> {
        let entry = self.map.remove(id)?;
        self.by_time.remove(&entry.order);
        if let Some(set) = self.per_ip.get_mut(&entry.ip) {
            set.remove(&entry.order);
            if set.is_empty() {
                self.per_ip.remove(&entry.ip);
            }
        }
        Some(entry)
    }

    fn evict_oldest(&mut self) {
        if let Some(id) = self.by_time.values().next().copied() {
            self.remove(&id);
        }
    }

    fn expire(&mut self, now_ms: u64) {
        while let Some((&(created, _), &id)) = self.by_time.iter().next() {
            if now_ms.saturating_sub(created) < LOGIN_TTL_MS {
                break;
            }
            self.remove(&id);
        }
    }
}

/// The pending logins, in memory only.
pub struct LoginStates {
    inner: Mutex<Inner>,
    cap: usize,
    per_ip_cap: usize,
}

impl Default for LoginStates {
    fn default() -> Self {
        Self::with_caps(MAX_LOGIN_STATES, MAX_LOGINS_PER_IP)
    }
}

impl LoginStates {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty set with the given caps (all pending logins, per IP prefix).
    #[must_use]
    pub fn with_caps(cap: usize, per_ip_cap: usize) -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            cap,
            per_ip_cap,
        }
    }

    /// Stores a login under `id` for the client with the rate-limit key `ip`.
    pub fn insert(
        &self,
        ip: Vec<u8>,
        id: [u8; 16],
        state: ServerLogin<Suite>,
        account: Option<([u8; 16], u64)>,
        credential_id: [u8; 16],
        now_ms: u64,
    ) {
        let (account_id, account_version) = account.map_or((None, 0), |(a, v)| (Some(a), v));
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.expire(now_ms);
        // This IP prefix keeps its newest logins only.
        while inner
            .per_ip
            .get(&ip)
            .is_some_and(|s| s.len() >= self.per_ip_cap)
        {
            let oldest = inner
                .per_ip
                .get(&ip)
                .and_then(|s| s.iter().next().copied())
                .and_then(|o| inner.by_time.get(&o).copied());
            match oldest {
                Some(old) => inner.remove(&old),
                None => break,
            };
        }
        while inner.map.len() >= self.cap && !inner.map.is_empty() {
            inner.evict_oldest();
        }
        inner.seq += 1;
        let order = (now_ms, inner.seq);
        inner.by_time.insert(order, id);
        inner.per_ip.entry(ip.clone()).or_default().insert(order);
        if let Some(old) = inner.map.insert(
            id,
            LoginEntry {
                state,
                account_id,
                account_version,
                credential_id,
                created_ms: now_ms,
                order,
                ip,
            },
        ) {
            inner.by_time.remove(&old.order);
        }
    }

    /// Takes the login `id` out (each state is used once); `None` if it is
    /// unknown or expired.
    pub fn take(&self, id: &[u8; 16], now_ms: u64) -> Option<LoginEntry> {
        let entry = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id)?;
        (now_ms.saturating_sub(entry.created_ms) < LOGIN_TTL_MS).then_some(entry)
    }

    /// Drops every pending login of `account` (a password change ends them:
    /// a login started against the old password must not complete).
    pub fn drop_account(&self, account: &[u8; 16]) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ids: Vec<[u8; 16]> = inner
            .map
            .iter()
            .filter(|(_, e)| e.account_id.as_ref() == Some(account))
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            inner.remove(&id);
        }
    }

    /// Drops expired states.
    pub fn prune(&self, now_ms: u64) {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .expire(now_ms);
    }

    /// How many logins are pending (tests).
    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().map.len()
    }
}

/// The server-side parameters of login start and finish.
#[must_use]
pub fn login_params<'a>(
    credential_id: &'a [u8; 16],
    origin: &'a str,
) -> ServerLoginParameters<'static, 'a> {
    ServerLoginParameters {
        context: Some(LOGIN_CONTEXT.as_slice()),
        identifiers: identifiers(credential_id.as_slice(), origin.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use mosaic_sync::wire::opaque::{
        OsRng,
        ke::{ClientLogin, ServerSetup},
    };

    use super::*;

    fn state(setup: &ServerSetup<Suite>) -> ServerLogin<Suite> {
        let request = ClientLogin::<Suite>::start(&mut OsRng, b"pw")
            .unwrap()
            .message;
        let cred = [1u8; 16];
        ServerLogin::start(
            &mut OsRng,
            setup,
            None,
            request,
            &cred,
            login_params(&cred, "https://x.example"),
        )
        .unwrap()
        .state
    }

    /// An IP prefix holds 16 logins (its oldest goes first) and the global cap
    /// evicts in time order.
    #[test]
    fn per_ip_and_global_caps_evict_oldest_first() {
        let setup = ServerSetup::<Suite>::new(&mut OsRng);
        let states = LoginStates::with_caps(20, 16);
        for i in 0..17u8 {
            states.insert(vec![1], [i; 16], state(&setup), None, [0; 16], u64::from(i));
        }
        assert_eq!(states.len(), 16);
        assert!(
            states.take(&[0; 16], 20).is_none(),
            "the oldest of the prefix was evicted"
        );
        assert!(states.take(&[1; 16], 20).is_some());
        // Another prefix is unaffected.
        states.insert(vec![2], [100; 16], state(&setup), None, [0; 16], 30);
        assert!(states.take(&[100; 16], 31).is_some());

        // Global cap: the oldest of all goes, whoever owns it.
        let states = LoginStates::with_caps(3, 16);
        for i in 0..5u8 {
            states.insert(vec![i], [i; 16], state(&setup), None, [0; 16], u64::from(i));
        }
        assert_eq!(states.len(), 3);
        assert!(states.take(&[0; 16], 10).is_none() && states.take(&[1; 16], 10).is_none());
        assert!(states.take(&[4; 16], 10).is_some());
        // Expiry follows the index too.
        states.insert(vec![9], [9; 16], state(&setup), None, [0; 16], 20);
        states.prune(20 + LOGIN_TTL_MS);
        assert_eq!(states.len(), 0);
    }
}
