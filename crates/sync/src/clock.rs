//! The clock the engine and the transports read, so tests never sleep.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of the current time.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
}

/// The system clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    fn now_ms(&self) -> u64 {
        (**self).now_ms()
    }
}

impl<C: Clock + ?Sized> Clock for &C {
    fn now_ms(&self) -> u64 {
        (**self).now_ms()
    }
}
