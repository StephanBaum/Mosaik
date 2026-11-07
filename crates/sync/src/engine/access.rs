//! How the engine reaches the session, the jitter source and the retry
//! schedule.

use mosaic_vault::session::UnlockedSession;

use super::SyncError;

/// Short, locked steps on the unlocked session. The engine never holds the
/// session across a transport call: it calls [`SessionAccess::with`] for
/// each step. `Err(SyncError::Locked)` when the session is locked.
pub trait SessionAccess {
    /// Runs `f` on the unlocked session.
    ///
    /// # Errors
    /// [`SyncError::Locked`].
    fn with<R>(&mut self, f: impl FnOnce(&mut UnlockedSession) -> R) -> Result<R, SyncError>;
}

/// The random factor of the retry delay (±20 %).
pub trait Jitter: Send {
    /// A factor in per mille, 800 to 1200.
    fn permille(&mut self) -> u32;
}

/// Jitter from the operating system's random source.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemJitter;

impl Jitter for SystemJitter {
    fn permille(&mut self) -> u32 {
        let mut b = [0u8; 2];
        // The fallback is acceptable: the jitter only spreads retries out, it
        // is not secret and protects nothing. A failing random source gives
        // the plain delay instead of an error.
        if getrandom::fill(&mut b).is_err() {
            return 1000;
        }
        800 + u32::from(u16::from_be_bytes(b)) % 401
    }
}

/// Exponential backoff after transient failures.
#[derive(Clone, Copy, Debug, Default)]
pub struct Backoff {
    failures: u32,
    /// The floor of the last delay a server asked for (ms).
    floor_ms: u64,
    /// When the next attempt may start (ms since the Unix epoch).
    pub retry_at: Option<u64>,
}

impl Backoff {
    /// Forgets all failures.
    pub fn reset(&mut self) {
        self.failures = 0;
        self.floor_ms = 0;
        self.retry_at = None;
    }

    /// Whether the scheduled retry lies further ahead of `now` than the
    /// longest delay the backoff can set (`max_ms` plus the jitter of 20 %,
    /// or the floor a server asked for, whichever is more): the clock was set
    /// back since the failure.
    #[must_use]
    pub fn is_beyond_any_delay(&self, now: u64, max_ms: u64) -> bool {
        let longest = (max_ms.saturating_mul(12) / 10).max(self.floor_ms);
        self.retry_at
            .is_some_and(|at| at.saturating_sub(now) > longest)
    }

    /// Notes a failure at `now`: the delay doubles from `initial_ms` up to
    /// `max_ms`, scaled by `permille` / 1000, and is at least `floor_s`
    /// seconds.
    pub fn fail(&mut self, now: u64, floor_s: u32, permille: u32, initial_ms: u64, max_ms: u64) {
        let base = initial_ms
            .saturating_mul(1u64 << self.failures.min(20))
            .min(max_ms);
        let jittered = base.saturating_mul(u64::from(permille.clamp(800, 1200))) / 1000;
        let floor = u64::from(floor_s).saturating_mul(1000);
        self.floor_ms = floor;
        self.retry_at = Some(now.saturating_add(jittered.max(floor)));
        self.failures = self.failures.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_is_capped_and_honors_the_floor() {
        let mut b = Backoff::default();
        b.fail(1_000, 0, 1000, 5_000, 900_000);
        assert_eq!(b.retry_at, Some(6_000));
        b.fail(10_000, 0, 1000, 5_000, 900_000);
        assert_eq!(b.retry_at, Some(20_000));
        for _ in 0..40 {
            b.fail(0, 0, 1000, 5_000, 900_000);
        }
        assert_eq!(b.retry_at, Some(900_000));
        // A server's retry-after is a floor, and the jitter is clamped.
        let mut b = Backoff::default();
        b.fail(0, 60, 5000, 5_000, 900_000);
        assert_eq!(b.retry_at, Some(60_000));
        b.reset();
        assert_eq!(b.retry_at, None);
        b.fail(0, 0, 0, 5_000, 900_000);
        assert_eq!(b.retry_at, Some(4_000));
    }

    #[test]
    fn a_retry_beyond_any_delay_is_a_clock_set_back() {
        let mut b = Backoff::default();
        b.fail(500_000_000, 0, 1200, 5_000, 900_000);
        assert!(!b.is_beyond_any_delay(500_000_000, 900_000));
        // The clock goes back two days.
        assert!(b.is_beyond_any_delay(500_000_000 - 172_800_000, 900_000));
        // A long floor from the server is still honoured.
        let mut b = Backoff::default();
        b.fail(10_000_000_000, 7_200, 1000, 5_000, 900_000);
        assert!(!b.is_beyond_any_delay(10_000_000_000, 900_000));
        assert!(b.is_beyond_any_delay(1_000_000_000, 900_000));
    }

    #[test]
    fn system_jitter_stays_within_twenty_percent() {
        let mut j = SystemJitter;
        for _ in 0..200 {
            let p = j.permille();
            assert!((800..=1200).contains(&p), "{p}");
        }
    }
}
