//! In-memory token buckets (M6 spec §6.4). Nothing here is ever written to
//! disk: client IP addresses exist only as SHA-256 keys in this map
//! (CRYPTO_SPEC §12).

use std::collections::HashMap;
use std::sync::Mutex;

use sha2::{Digest, Sha256};

/// Which limit a bucket belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Scope {
    /// `POST /v1/prelogin`, per IP.
    PreloginIp,
    /// `POST /v1/prelogin`, per email.
    PreloginEmail,
    /// `POST /v1/login/start`, per IP.
    LoginIp,
    /// `POST /v1/login/start`, per email.
    LoginEmail,
    /// `POST /v1/register/start`, per IP.
    RegisterStartIp,
    /// `POST /v1/register/finish`, per IP.
    RegisterFinishIp,
    /// Authenticated routes, per session.
    Session,
    /// Attachment uploads, per session.
    AttachmentPut,
}

/// A bucket's size and refill rate.
#[derive(Clone, Copy, Debug)]
pub struct Rule {
    /// Tokens the bucket holds when full (the burst).
    pub capacity: u32,
    /// Milliseconds to earn one token.
    pub ms_per_token: u64,
}

impl Rule {
    /// `n` per minute with a burst of `capacity`.
    #[must_use]
    pub const fn per_minute(n: u32, capacity: u32) -> Self {
        Self {
            capacity,
            ms_per_token: 60_000 / n as u64,
        }
    }

    /// `n` per hour with a burst of `n`.
    #[must_use]
    pub const fn per_hour(n: u32) -> Self {
        Self {
            capacity: n,
            ms_per_token: 3_600_000 / n as u64,
        }
    }
}

/// Prelogin: 10/min per IP (burst 20).
pub const PRELOGIN_IP: Rule = Rule::per_minute(10, 20);
/// Prelogin: 30/h per email.
pub const PRELOGIN_EMAIL: Rule = Rule::per_hour(30);
/// Login start: 10/min per IP.
pub const LOGIN_IP: Rule = Rule::per_minute(10, 10);
/// Login start: 5/min per email.
pub const LOGIN_EMAIL: Rule = Rule::per_minute(5, 5);
/// Registration: 3/h per IP.
pub const REGISTER_IP: Rule = Rule::per_hour(3);
/// Authenticated routes: 600/min per session.
pub const SESSION: Rule = Rule::per_minute(600, 600);
/// Attachment uploads: 60/min per session.
pub const ATTACHMENT_PUT: Rule = Rule::per_minute(60, 60);

/// Most buckets of the IP scopes.
const MAX_IP_BUCKETS: usize = 100_000;
/// Most buckets of the email and session scopes (a separate map, so a flood
/// of IP keys cannot crowd them out).
const MAX_KEY_BUCKETS: usize = 200_000;
/// Seconds a request must wait when the map is full of live buckets.
const FULL_RETRY_S: u32 = 60;

#[derive(Clone, Copy)]
struct Bucket {
    /// Milliseconds worth of tokens held (so refill is integer arithmetic).
    level: u64,
    last: u64,
    full: u64,
}

type Buckets = HashMap<(Scope, [u8; 32]), Bucket>;

const fn is_full(b: &Bucket, now_ms: u64) -> bool {
    b.level.saturating_add(now_ms.saturating_sub(b.last)) >= b.full
}

/// The rate limiter. Buckets live in two maps with their own caps. A bucket
/// is dropped only when it is full again (dropping it then forgets nothing);
/// when a map is full of live buckets, a new key is refused.
pub struct RateLimiter {
    ip: Mutex<Buckets>,
    keyed: Mutex<Buckets>,
    ip_cap: usize,
    key_cap: usize,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::with_caps(MAX_IP_BUCKETS, MAX_KEY_BUCKETS)
    }
}

const fn is_ip_scope(scope: Scope) -> bool {
    matches!(
        scope,
        Scope::PreloginIp | Scope::LoginIp | Scope::RegisterStartIp | Scope::RegisterFinishIp
    )
}

impl RateLimiter {
    /// A limiter with no buckets.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A limiter with the given caps for the IP map and the other map.
    #[must_use]
    pub fn with_caps(ip_cap: usize, key_cap: usize) -> Self {
        Self {
            ip: Mutex::new(HashMap::new()),
            keyed: Mutex::new(HashMap::new()),
            ip_cap,
            key_cap,
        }
    }

    /// Takes one token from the bucket for `(scope, key)`.
    ///
    /// # Errors
    /// The seconds to wait until a token is available.
    pub fn check(&self, now_ms: u64, scope: Scope, key: &[u8], rule: Rule) -> Result<(), u32> {
        let key: [u8; 32] = Sha256::digest(key).into();
        let full = u64::from(rule.capacity).saturating_mul(rule.ms_per_token);
        let (mutex, cap) = if is_ip_scope(scope) {
            (&self.ip, self.ip_cap)
        } else {
            (&self.keyed, self.key_cap)
        };
        let mut map = mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if map.len() >= cap && !map.contains_key(&(scope, key)) {
            map.retain(|_, b| !is_full(b, now_ms));
            if map.len() >= cap {
                return Err(FULL_RETRY_S);
            }
        }
        let bucket = map.entry((scope, key)).or_insert(Bucket {
            level: full,
            last: now_ms,
            full,
        });
        let elapsed = now_ms.saturating_sub(bucket.last);
        bucket.last = now_ms.max(bucket.last);
        bucket.level = bucket.level.saturating_add(elapsed).min(bucket.full);
        if bucket.level >= rule.ms_per_token {
            bucket.level -= rule.ms_per_token;
            Ok(())
        } else {
            let missing = rule.ms_per_token - bucket.level;
            Err(u32::try_from(missing.div_ceil(1000))
                .unwrap_or(u32::MAX)
                .max(1))
        }
    }

    /// Drops buckets that are full again.
    pub fn prune(&self, now_ms: u64) {
        for m in [&self.ip, &self.keyed] {
            m.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retain(|_, b| !is_full(b, now_ms));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_then_refill() {
        let l = RateLimiter::new();
        let rule = Rule::per_minute(5, 5);
        for _ in 0..5 {
            assert!(l.check(0, Scope::LoginEmail, b"a", rule).is_ok());
        }
        let wait = l.check(0, Scope::LoginEmail, b"a", rule).unwrap_err();
        assert!((1..=12).contains(&wait));
        // Another key is independent.
        assert!(l.check(0, Scope::LoginEmail, b"b", rule).is_ok());
        // One token after 12 s.
        assert!(l.check(12_000, Scope::LoginEmail, b"a", rule).is_ok());
        assert!(l.check(12_000, Scope::LoginEmail, b"a", rule).is_err());
        l.prune(10_000_000);
        assert!(l.keyed.lock().unwrap().is_empty());
    }

    /// A flood of IP keys fills only the IP map: a live bucket of another
    /// scope keeps its state and the flood's new keys are refused.
    #[test]
    fn ip_flood_leaves_email_bucket_intact() {
        let l = RateLimiter::with_caps(10, 10);
        let rule = Rule::per_minute(5, 5);
        // The email bucket has spent one token (4 left).
        assert!(l.check(0, Scope::LoginEmail, b"victim", rule).is_ok());
        let mut refused = 0;
        for i in 0..100u32 {
            if l.check(0, Scope::LoginIp, &i.to_be_bytes(), LOGIN_IP)
                .is_err()
            {
                refused += 1;
            }
        }
        assert_eq!(
            refused, 90,
            "the IP map holds 10 live buckets, the rest is refused"
        );
        // A key already in the map is still served.
        assert!(
            l.check(0, Scope::LoginIp, &0u32.to_be_bytes(), LOGIN_IP)
                .is_ok()
        );
        // The victim keeps exactly its 4 remaining tokens.
        for _ in 0..4 {
            assert!(l.check(0, Scope::LoginEmail, b"victim", rule).is_ok());
        }
        assert!(l.check(0, Scope::LoginEmail, b"victim", rule).is_err());
        // Idle buckets are dropped once full again, so new keys fit later.
        assert!(
            l.check(3_600_000, Scope::LoginIp, &500u32.to_be_bytes(), LOGIN_IP)
                .is_ok()
        );
    }
}
