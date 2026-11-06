//! The server's clock. Time in Unix milliseconds. Tests use [`ManualClock`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of Unix time in milliseconds.
pub trait Clock: Send + Sync {
    /// The current time.
    fn now_ms(&self) -> u64;
}

/// The system clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_millis()).ok())
            .unwrap_or(0)
    }
}

/// A clock the test sets and advances by hand.
#[derive(Debug)]
pub struct ManualClock(AtomicU64);

impl ManualClock {
    /// A clock at `now_ms`.
    #[must_use]
    pub const fn new(now_ms: u64) -> Self {
        Self(AtomicU64::new(now_ms))
    }

    /// Moves the clock forward.
    pub fn advance_ms(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }

    /// Sets the clock.
    pub fn set_ms(&self, ms: u64) {
        self.0.store(ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
