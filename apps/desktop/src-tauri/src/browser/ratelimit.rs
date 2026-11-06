//! Sliding-window rate limits of the browser endpoint (`docs/NATIVE_MESSAGING.md`
//! §6): per pairing for the ops, global for `focus` and pairing requests.
//! The windows read an injected clock, so the tests move time by hand.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// What is limited.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bucket {
    /// `fill`.
    Fill,
    /// `copy`.
    Copy,
    /// `listForOrigin` and `search` together.
    Lookup,
    /// `generate`.
    Generate,
    /// `checkSave`.
    CheckSave,
    /// `proposeSave`.
    ProposeSave,
    /// `passkeyCreate`.
    PasskeyCreate,
    /// `passkeyGet`.
    PasskeyGet,
    /// `passkeyCancel`.
    PasskeyCancel,
    /// `focus` (global).
    Focus,
    /// Pairing requests (global).
    PairRequest,
}

impl Bucket {
    /// Requests allowed per window, and the window.
    #[must_use]
    pub const fn limit(self) -> (usize, Duration) {
        match self {
            Self::Fill => (10, Duration::from_secs(10)),
            Self::Copy | Self::PasskeyCancel => (20, Duration::from_secs(60)),
            Self::Lookup => (60, Duration::from_secs(60)),
            Self::Generate => (30, Duration::from_secs(60)),
            Self::CheckSave | Self::PasskeyGet => (10, Duration::from_secs(60)),
            Self::ProposeSave | Self::PairRequest | Self::PasskeyCreate => {
                (3, Duration::from_secs(60))
            }
            Self::Focus => (1, Duration::from_secs(2)),
        }
    }

    const fn is_global(self) -> bool {
        matches!(self, Self::Focus | Self::PairRequest)
    }
}

/// The key of one window: a bucket, and for per-pairing buckets the pairing.
type Key = (Bucket, Option<[u8; 16]>);

/// Every window.
#[derive(Default)]
pub struct RateLimits {
    windows: HashMap<Key, VecDeque<Instant>>,
}

impl RateLimits {
    /// Counts one request of `pairing` in `bucket` at `now`. `false`: over
    /// the limit (the request is not counted).
    pub fn allow(&mut self, bucket: Bucket, pairing: Option<[u8; 16]>, now: Instant) -> bool {
        let (max, window) = bucket.limit();
        let key = (bucket, if bucket.is_global() { None } else { pairing });
        let times = self.windows.entry(key).or_default();
        while times
            .front()
            .is_some_and(|&t| now.saturating_duration_since(t) >= window)
        {
            times.pop_front();
        }
        if times.len() >= max {
            return false;
        }
        times.push_back(now);
        true
    }

    /// Forgets the windows of a removed pairing.
    pub fn forget(&mut self, pairing: &[u8; 16]) {
        self.windows
            .retain(|(_, owner), _| owner.as_ref() != Some(pairing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eleventh_fill_in_ten_seconds_is_refused() {
        let mut limits = RateLimits::default();
        let start = Instant::now();
        let id = Some([1; 16]);
        for i in 0..10 {
            assert!(limits.allow(Bucket::Fill, id, start + Duration::from_millis(i * 100)));
        }
        assert!(!limits.allow(Bucket::Fill, id, start + Duration::from_secs(9)));
        // The window slides: the first request has aged out after 10 s.
        assert!(limits.allow(Bucket::Fill, id, start + Duration::from_secs(10)));
        // Another pairing has its own window.
        assert!(limits.allow(Bucket::Fill, Some([2; 16]), start + Duration::from_secs(9)));
    }

    #[test]
    fn global_buckets_ignore_the_pairing() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        assert!(limits.allow(Bucket::Focus, Some([1; 16]), now));
        assert!(!limits.allow(Bucket::Focus, Some([2; 16]), now));
        assert!(limits.allow(Bucket::Focus, None, now + Duration::from_secs(2)));
        for _ in 0..3 {
            assert!(limits.allow(Bucket::PairRequest, None, now));
        }
        assert!(!limits.allow(Bucket::PairRequest, None, now));
    }

    #[test]
    fn passkey_buckets_have_their_own_limits() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        let id = Some([1; 16]);
        for _ in 0..3 {
            assert!(limits.allow(Bucket::PasskeyCreate, id, now));
        }
        assert!(!limits.allow(Bucket::PasskeyCreate, id, now));
        for _ in 0..10 {
            assert!(limits.allow(Bucket::PasskeyGet, id, now));
        }
        assert!(!limits.allow(Bucket::PasskeyGet, id, now));
        for _ in 0..20 {
            assert!(limits.allow(Bucket::PasskeyCancel, id, now));
        }
        assert!(!limits.allow(Bucket::PasskeyCancel, id, now));
        // The windows are separate from each other and slide after 60 s.
        assert!(limits.allow(Bucket::PasskeyCreate, id, now + Duration::from_secs(60)));
    }

    #[test]
    fn forgetting_a_pairing_clears_its_windows() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        for _ in 0..3 {
            assert!(limits.allow(Bucket::ProposeSave, Some([1; 16]), now));
        }
        assert!(!limits.allow(Bucket::ProposeSave, Some([1; 16]), now));
        limits.forget(&[1; 16]);
        assert!(limits.allow(Bucket::ProposeSave, Some([1; 16]), now));
    }
}
