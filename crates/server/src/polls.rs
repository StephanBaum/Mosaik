//! Long-poll change notification (M6 spec §6.2, `GET /v1/poll`).
//!
//! Every account has a change counter in memory. A write to one of the
//! account's vaults bumps it. A poll returns at once when the counter differs
//! from the client's `since`, else when it changes or after 25 s. A session
//! has at most one poll; a second one gets `too_many_polls` and the older
//! poll is answered first, so a client that restarted does not wait for its
//! stale poll.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::{Notify, watch};

/// How long a poll waits.
pub const POLL_TIMEOUT: Duration = Duration::from_secs(25);
/// Polls waiting at once, server-wide.
pub const MAX_POLLS: usize = 10_000;

/// Why a poll was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollRefused {
    /// The session already has a poll, or the server is full.
    TooMany,
}

struct Active {
    id: u64,
    release: std::sync::Arc<Notify>,
}

/// Counters and waiting polls.
#[derive(Default)]
pub struct Polls {
    counters: Mutex<HashMap<[u8; 16], watch::Sender<u64>>>,
    active: Mutex<HashMap<[u8; 32], Active>>,
    next_id: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Polls {
    /// No counters, no polls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Announces a change to `account`'s data.
    pub fn bump(&self, account: [u8; 16], now_ms: u64) {
        let mut map = lock(&self.counters);
        let tx = map
            .entry(account)
            .or_insert_with(|| watch::channel(now_ms).0);
        tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    /// Waits for `account`'s counter to differ from `since`, at most
    /// `timeout`. `session` identifies the session (its token hash).
    ///
    /// # Errors
    /// [`PollRefused::TooMany`].
    pub async fn wait(
        &self,
        account: [u8; 16],
        session: [u8; 32],
        since: u64,
        now_ms: u64,
        timeout: Duration,
    ) -> Result<u64, PollRefused> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let release = std::sync::Arc::new(Notify::new());
        {
            let mut active = lock(&self.active);
            if let Some(old) = active.get(&session) {
                // The older poll is answered; this request is refused.
                old.release.notify_one();
                return Err(PollRefused::TooMany);
            }
            if active.len() >= MAX_POLLS {
                return Err(PollRefused::TooMany);
            }
            active.insert(
                session,
                Active {
                    id,
                    release: release.clone(),
                },
            );
        }
        let _guard = ActiveGuard {
            polls: self,
            session,
            id,
        };
        let mut rx = {
            let mut map = lock(&self.counters);
            map.entry(account)
                .or_insert_with(|| watch::channel(now_ms).0)
                .subscribe()
        };
        let wait = async {
            loop {
                if *rx.borrow_and_update() != since {
                    break;
                }
                tokio::select! {
                    changed = rx.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    () = release.notified() => break,
                }
            }
        };
        let _ = tokio::time::timeout(timeout, wait).await;
        let value = *rx.borrow();
        Ok(value)
    }

    /// Drops counters nobody waits on. A recreated counter starts at the
    /// clock, which is above any value this one reached.
    pub fn prune(&self) {
        lock(&self.counters).retain(|_, tx| tx.receiver_count() > 0);
    }
}

struct ActiveGuard<'a> {
    polls: &'a Polls,
    session: [u8; 32],
    id: u64,
}

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        let mut active = lock(&self.polls.active);
        if active.get(&self.session).is_some_and(|a| a.id == self.id) {
            active.remove(&self.session);
        }
    }
}
