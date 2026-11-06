//! Sliding-window rate limits of the browser endpoint (`docs/NATIVE_MESSAGING.md`
//! §6): per pairing for the ops, global for `focus` and pairing requests, and
//! per (pairing, origin) for `passkeyCreate` and `passkeyGet`, so one site
//! cannot spend the pairing's passkey windows for another.
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

    /// The smaller allowance of one origin, for the buckets that have one.
    #[must_use]
    pub const fn origin_limit(self) -> Option<(usize, Duration)> {
        match self {
            Self::PasskeyCreate => Some((2, Duration::from_secs(60))),
            Self::PasskeyGet => Some((6, Duration::from_secs(60))),
            _ => None,
        }
    }

    const fn is_global(self) -> bool {
        matches!(self, Self::Focus | Self::PairRequest)
    }
}

/// The most origins one pairing keeps a window for, per bucket. The oldest
/// goes first. The per-pairing windows allow 13 passkey requests a minute, so
/// a site cannot push another site's live window out.
pub const MAX_ORIGINS_PER_PAIRING: usize = 256;

/// The key of one per-origin window: a bucket, a pairing and the canonical origin.
type OriginKey = (Bucket, [u8; 16], String);

/// The key of one window: a bucket, and for per-pairing buckets the pairing.
type Key = (Bucket, Option<[u8; 16]>);

/// Every window.
#[derive(Default)]
pub struct RateLimits {
    windows: HashMap<Key, VecDeque<Instant>>,
    origin_windows: HashMap<OriginKey, VecDeque<Instant>>,
}

/// Drops the requests that have aged out of `window`.
fn trim(times: &mut VecDeque<Instant>, window: Duration, now: Instant) {
    while times
        .front()
        .is_some_and(|&t| now.saturating_duration_since(t) >= window)
    {
        times.pop_front();
    }
}

impl RateLimits {
    /// Counts one request of `pairing` in `bucket` at `now`. `false`: over
    /// the limit (the request is not counted).
    pub fn allow(&mut self, bucket: Bucket, pairing: Option<[u8; 16]>, now: Instant) -> bool {
        let (max, window) = bucket.limit();
        let key = (bucket, if bucket.is_global() { None } else { pairing });
        let times = self.windows.entry(key).or_default();
        trim(times, window, now);
        if times.len() >= max {
            return false;
        }
        times.push_back(now);
        true
    }

    /// Counts one passkey request of `pairing` for `origin` in `bucket` at
    /// `now`: it must fit the origin's own window first and then the
    /// pairing's, and a request refused by either is counted in neither, so a
    /// site that spent its own window spends nothing of the pairing's. Without
    /// an origin (the body did not parse, the handler refuses it) only the
    /// pairing's window counts.
    pub fn allow_for_origin(
        &mut self,
        bucket: Bucket,
        pairing: [u8; 16],
        origin: Option<&str>,
        now: Instant,
    ) -> bool {
        let Some((origin, (max, window))) = origin.zip(bucket.origin_limit()) else {
            return self.allow(bucket, Some(pairing), now);
        };
        let key: OriginKey = (bucket, pairing, origin.to_owned());
        let has_room = match self.origin_windows.get_mut(&key) {
            Some(times) => {
                trim(times, window, now);
                times.len() < max
            }
            None => true,
        };
        if !has_room || !self.allow(bucket, Some(pairing), now) {
            return false;
        }
        if !self.origin_windows.contains_key(&key) {
            self.make_room_for_origin(bucket, &pairing, window, now);
        }
        self.origin_windows.entry(key).or_default().push_back(now);
        true
    }

    /// Keeps one pairing's origins in a bucket below [`MAX_ORIGINS_PER_PAIRING`]:
    /// expired windows go first, then the one whose latest request is the oldest.
    fn make_room_for_origin(
        &mut self,
        bucket: Bucket,
        pairing: &[u8; 16],
        window: Duration,
        now: Instant,
    ) {
        let ours = |key: &OriginKey| key.0 == bucket && key.1 == *pairing;
        let live = |times: &VecDeque<Instant>| {
            times
                .back()
                .is_some_and(|&t| now.saturating_duration_since(t) < window)
        };
        let count = self.origin_windows.keys().filter(|key| ours(key)).count();
        if count < MAX_ORIGINS_PER_PAIRING {
            return;
        }
        self.origin_windows
            .retain(|key, times| !ours(key) || live(times));
        let kept = self.origin_windows.keys().filter(|key| ours(key)).count();
        if kept < MAX_ORIGINS_PER_PAIRING {
            return;
        }
        let oldest = self
            .origin_windows
            .iter()
            .filter(|(key, _)| ours(key))
            .min_by_key(|(_, times)| times.back().copied())
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            self.origin_windows.remove(&oldest);
        }
    }

    /// Forgets the windows of a removed pairing.
    pub fn forget(&mut self, pairing: &[u8; 16]) {
        self.windows
            .retain(|(_, owner), _| owner.as_ref() != Some(pairing));
        self.origin_windows
            .retain(|(_, owner, _), _| owner != pairing);
    }

    /// How many origins have a window (tests).
    #[cfg(test)]
    fn origin_count(&self, pairing: &[u8; 16]) -> usize {
        self.origin_windows
            .keys()
            .filter(|key| key.1 == *pairing)
            .count()
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
    fn a_site_has_smaller_passkey_windows_of_its_own() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        let pairing = [1; 16];
        let a = Some("https://a.test");
        for _ in 0..2 {
            assert!(limits.allow_for_origin(Bucket::PasskeyCreate, pairing, a, now));
        }
        assert!(!limits.allow_for_origin(Bucket::PasskeyCreate, pairing, a, now));
        // The refusal cost the pairing nothing: 2 of its 3 are spent.
        assert!(limits.allow_for_origin(
            Bucket::PasskeyCreate,
            pairing,
            Some("https://b.test"),
            now
        ));
        assert!(!limits.allow_for_origin(
            Bucket::PasskeyCreate,
            pairing,
            Some("https://c.test"),
            now
        ));
        for _ in 0..6 {
            assert!(limits.allow_for_origin(Bucket::PasskeyGet, pairing, a, now));
        }
        assert!(!limits.allow_for_origin(Bucket::PasskeyGet, pairing, a, now));
        // Another pairing has windows of its own, and they slide after 60 s.
        assert!(limits.allow_for_origin(Bucket::PasskeyCreate, [2; 16], a, now));
        let later = now + Duration::from_secs(60);
        assert!(limits.allow_for_origin(Bucket::PasskeyCreate, pairing, a, later));
        assert!(limits.allow_for_origin(Bucket::PasskeyGet, pairing, a, later));
    }

    #[test]
    fn the_pairings_window_still_limits_many_sites_and_a_missing_origin() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        let pairing = [1; 16];
        for n in 0..3 {
            let origin = format!("https://site{n}.test");
            assert!(limits.allow_for_origin(Bucket::PasskeyCreate, pairing, Some(&origin), now));
        }
        assert!(!limits.allow_for_origin(
            Bucket::PasskeyCreate,
            pairing,
            Some("https://x.test"),
            now
        ));
        // No origin (the handler refuses the body): only the pairing's window counts.
        assert!(!limits.allow_for_origin(Bucket::PasskeyCreate, pairing, None, now));
        let mut limits = RateLimits::default();
        for _ in 0..10 {
            assert!(limits.allow_for_origin(Bucket::PasskeyGet, pairing, None, now));
        }
        assert!(!limits.allow_for_origin(Bucket::PasskeyGet, pairing, None, now));
        assert_eq!(limits.origin_count(&pairing), 0);
        // Buckets without an origin allowance ignore the origin.
        for _ in 0..20 {
            assert!(limits.allow_for_origin(
                Bucket::PasskeyCancel,
                pairing,
                Some("https://a.test"),
                now
            ));
        }
        assert!(!limits.allow_for_origin(
            Bucket::PasskeyCancel,
            pairing,
            Some("https://b.test"),
            now
        ));
        assert_eq!(limits.origin_count(&pairing), 0);
    }

    #[test]
    fn the_origin_map_is_bounded_and_the_oldest_origin_goes_first() {
        let mut limits = RateLimits::default();
        let start = Instant::now();
        let pairing = [1; 16];
        // One request a minute from ever new sites, as hours of browsing would
        // make: each window has expired when the cap is reached.
        for n in 0..MAX_ORIGINS_PER_PAIRING + 40 {
            let at = start + Duration::from_secs(n as u64 * 61);
            let origin = format!("https://site{n}.test");
            assert!(limits.allow_for_origin(Bucket::PasskeyGet, pairing, Some(&origin), at));
            assert!(limits.origin_count(&pairing) <= MAX_ORIGINS_PER_PAIRING);
        }
        // Expired windows went first; the map is far below the cap here.
        assert!(limits.origin_count(&pairing) < MAX_ORIGINS_PER_PAIRING);
    }

    #[test]
    fn at_the_cap_the_origin_with_the_oldest_request_is_evicted() {
        let mut limits = RateLimits::default();
        let start = Instant::now();
        let pairing = [1; 16];
        // Fill the cap with live windows, each origin's request a millisecond
        // after the one before (inserted directly: the pairing's own window
        // would not let that many requests through a minute).
        for n in 0..MAX_ORIGINS_PER_PAIRING {
            let key = (Bucket::PasskeyGet, pairing, format!("https://site{n}.test"));
            limits
                .origin_windows
                .entry(key)
                .or_default()
                .push_back(start + Duration::from_millis(n as u64));
        }
        assert_eq!(limits.origin_count(&pairing), MAX_ORIGINS_PER_PAIRING);
        let now = start + Duration::from_secs(1);
        assert!(limits.allow_for_origin(
            Bucket::PasskeyGet,
            pairing,
            Some("https://new.test"),
            now
        ));
        assert_eq!(limits.origin_count(&pairing), MAX_ORIGINS_PER_PAIRING);
        let has = |limits: &RateLimits, origin: &str| {
            limits
                .origin_windows
                .contains_key(&(Bucket::PasskeyGet, pairing, origin.to_owned()))
        };
        // site0 had the oldest request; the newest and the new one stay.
        assert!(!has(&limits, "https://site0.test"));
        assert!(has(&limits, "https://site1.test"));
        assert!(has(&limits, "https://new.test"));
        // Another pairing's origins do not count against this one's cap.
        assert!(limits.allow_for_origin(
            Bucket::PasskeyGet,
            [2; 16],
            Some("https://new.test"),
            now
        ));
        assert_eq!(limits.origin_count(&pairing), MAX_ORIGINS_PER_PAIRING);
    }

    #[test]
    fn forgetting_a_pairing_clears_its_origin_windows_too() {
        let mut limits = RateLimits::default();
        let now = Instant::now();
        let origin = Some("https://a.test");
        for _ in 0..2 {
            assert!(limits.allow_for_origin(Bucket::PasskeyCreate, [1; 16], origin, now));
        }
        assert!(!limits.allow_for_origin(Bucket::PasskeyCreate, [1; 16], origin, now));
        limits.forget(&[1; 16]);
        assert_eq!(limits.origin_count(&[1; 16]), 0);
        assert!(limits.allow_for_origin(Bucket::PasskeyCreate, [1; 16], origin, now));
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
