//! Test utilities for the sync engine and the transports (feature
//! `test-util`): a manual clock, a fixed jitter, a seeded random source, a
//! session holder that can lock, a simulated file-sync client
//! ([`CloudSim`]), and transports that fail ([`ChaosTransport`]) or lie
//! ([`MaliciousTransport`]).
//!
//! Everything here is deterministic for a seed and never sleeps. It is not
//! part of the production API.

#![expect(
    clippy::indexing_slicing,
    reason = "test utilities: a failure here is a failed test"
)]

mod chaos;
mod cloudsim;
mod malicious;
mod session;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub use chaos::{ChaosConfig, ChaosTransport};
pub use cloudsim::{CloudSim, RoundStats, SimConfig};
pub use malicious::MaliciousTransport;
pub use session::LocalSession;

use crate::clock::{Clock, SystemClock};
use crate::engine::Jitter;

/// A clock that only moves when a test moves it. Clones share the time.
#[derive(Clone, Debug)]
pub struct ManualClock(Arc<AtomicU64>);

impl ManualClock {
    /// A clock set to the real current time (so file modification times
    /// and the clock agree), which then stands still.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(SystemClock.now_ms())))
    }

    /// Moves the time forward.
    pub fn advance_ms(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }

    /// Sets the time.
    pub fn set_ms(&self, ms: u64) {
        self.0.store(ms, Ordering::SeqCst);
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// A jitter source that always answers the same factor.
#[derive(Clone, Copy, Debug)]
pub struct FixedJitter(pub u32);

impl Jitter for FixedJitter {
    fn permille(&mut self) -> u32 {
        self.0
    }
}

/// A small deterministic generator (xorshift64*). Not for anything secret.
#[derive(Clone, Debug)]
pub struct SimRng(u64);

impl SimRng {
    /// A generator for `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number below `n` (0 if `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        match u64::try_from(n) {
            Ok(0) | Err(_) => 0,
            Ok(n) => usize::try_from(self.next_u64() % n).unwrap_or(0),
        }
    }

    /// `true` with the given chance in per mille.
    pub fn chance(&mut self, permille: u32) -> bool {
        self.next_u64() % 1000 < u64::from(permille)
    }

    /// Shuffles a slice.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}
