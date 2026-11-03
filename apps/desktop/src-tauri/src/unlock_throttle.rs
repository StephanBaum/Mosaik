//! Backoff between wrong master passwords at the unlock screen.
//!
//! The Argon2id cost already slows a guesser, but nothing stopped a local
//! process (or someone at the keyboard) from calling `unlock` in a loop.
//! After the third wrong password in a row the backend refuses the next
//! attempts for an increasing time (1 s, 2 s, 4 s ... up to 30 s) with
//! `unlockDelayed`, before any key derivation runs. A refused attempt is not
//! counted and does not block other commands (nothing sleeps). The state is
//! in memory: a right password resets it, and so does an app restart (a
//! process that can restart the app is beyond this limit, which is about the
//! unlock screen).

use std::time::{Duration, Instant};

/// Wrong passwords in a row that are free.
pub const FREE_ATTEMPTS: u32 = 3;
/// The first delay, after the [`FREE_ATTEMPTS`]th wrong password.
pub const FIRST_DELAY: Duration = Duration::from_secs(1);
/// The longest delay.
pub const MAX_DELAY: Duration = Duration::from_secs(30);

/// The failure count and when the next attempt is allowed.
#[derive(Default, Debug)]
pub struct UnlockThrottle {
    failures: u32,
    blocked_until: Option<Instant>,
}

impl UnlockThrottle {
    /// Whether an attempt at `now` may proceed.
    ///
    /// # Errors
    /// The time left to wait.
    pub fn check(&self, now: Instant) -> Result<(), Duration> {
        match self.blocked_until {
            Some(until) if now < until => Err(until.saturating_duration_since(now)),
            _ => Ok(()),
        }
    }

    /// Records a wrong password at `now`; the delay applies from the third on.
    pub fn failed(&mut self, now: Instant) {
        self.failures = self.failures.saturating_add(1);
        self.blocked_until = delay_after(self.failures).map(|d| now + d);
    }

    /// A right password resets everything.
    pub fn succeeded(&mut self) {
        *self = Self::default();
    }

    /// Wrong passwords in a row.
    #[must_use]
    pub const fn failures(&self) -> u32 {
        self.failures
    }
}

/// The delay after `failures` wrong passwords in a row.
#[must_use]
pub fn delay_after(failures: u32) -> Option<Duration> {
    let over = failures.checked_sub(FREE_ATTEMPTS)?;
    // 1 s, 2 s, 4 s, ...; the shift is capped so it cannot overflow.
    let factor = 1u32.checked_shl(over.min(16)).unwrap_or(u32::MAX);
    Some((FIRST_DELAY * factor).min(MAX_DELAY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delay_starts_after_the_third_wrong_password_and_doubles_to_a_cap() {
        let secs = |n| delay_after(n).map(|d| d.as_secs());
        assert_eq!(secs(0), None);
        assert_eq!(secs(2), None);
        assert_eq!(secs(3), Some(1));
        assert_eq!(secs(4), Some(2));
        assert_eq!(secs(5), Some(4));
        assert_eq!(secs(6), Some(8));
        assert_eq!(secs(7), Some(16));
        assert_eq!(secs(8), Some(30));
        assert_eq!(secs(1000), Some(30));
        assert_eq!(secs(u32::MAX), Some(30));
    }

    #[test]
    fn attempts_wait_for_the_delay_and_a_success_resets() {
        let t0 = Instant::now();
        let mut t = UnlockThrottle::default();
        for _ in 0..2 {
            assert_eq!(t.check(t0), Ok(()));
            t.failed(t0);
        }
        assert_eq!(t.check(t0), Ok(()), "two failures are free");
        t.failed(t0);
        assert_eq!(t.check(t0), Err(Duration::from_secs(1)));
        assert_eq!(
            t.check(t0 + Duration::from_millis(400)),
            Err(Duration::from_millis(600))
        );
        assert_eq!(t.check(t0 + Duration::from_secs(1)), Ok(()));
        let t1 = t0 + Duration::from_secs(1);
        t.failed(t1);
        assert_eq!(
            t.check(t1 + Duration::from_secs(1)),
            Err(Duration::from_secs(1))
        );
        assert_eq!(t.check(t1 + Duration::from_secs(2)), Ok(()));
        t.succeeded();
        assert_eq!(t.failures(), 0);
        assert_eq!(t.check(t1), Ok(()));
        t.failed(t1);
        assert_eq!(t.check(t1), Ok(()), "the count started over");
    }
}
