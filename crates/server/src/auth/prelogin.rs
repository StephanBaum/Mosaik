//! Pre-login fakes (CRYPTO_SPEC §9.2): for an unknown email the server answers
//! with values that are deterministic per email and look like a real
//! account's, so the response does not reveal whether the email is
//! registered.
//!
//! ```text
//! h(label) = HMAC-SHA-256(K_prelogin, label || normalized_email)
//! account_id: h("prelogin/account-id")[..16] as a UUIDv4 (version nibble
//!             4, variant bits 10), like a real account ID, which is a
//!             random UUIDv4 too; independent of any time
//! salt:       h("prelogin/salt")[..16]
//! profile:    h("prelogin/profile") mapped onto the real profile
//!             distribution, rounded to 5 % buckets
//! ```

use hmac::{Hmac, KeyInit, Mac};
use mosaic_core::Argon2Profile;
use sha2::Sha256;

/// How long a profile distribution is used before it is recomputed.
pub const DIST_TTL_MS: u64 = 3_600_000;

fn h(secret: &[u8; 32], label: &str, email: &str) -> [u8; 32] {
    // A 32-byte key is always valid for HMAC.
    let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(secret) else {
        return [0u8; 32];
    };
    mac.update(label.as_bytes());
    mac.update(email.as_bytes());
    mac.finalize().into_bytes().into()
}

/// The fake account ID of `email`: a `UUIDv4` from the HMAC.
#[must_use]
pub fn fake_account_id(key: &[u8; 32], email: &str) -> [u8; 16] {
    let digest = h(key, "prelogin/account-id", email);
    let mut id = [0u8; 16];
    id.copy_from_slice(digest.get(..16).unwrap_or(&[0u8; 16]));
    if let Some(b) = id.get_mut(6) {
        *b = (*b & 0x0F) | 0x40;
    }
    if let Some(b) = id.get_mut(8) {
        *b = (*b & 0x3F) | 0x80;
    }
    id
}

/// The fake salt of `email`.
#[must_use]
pub fn fake_salt(secret: &[u8; 32], email: &str) -> [u8; 16] {
    let digest = h(secret, "prelogin/salt", email);
    let mut salt = [0u8; 16];
    salt.copy_from_slice(digest.get(..16).unwrap_or(&[0u8; 16]));
    salt
}

/// The weights of the Argon2 profiles over `0..100`, in 5 % buckets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileDist(Vec<(Argon2Profile, u32)>);

impl ProfileDist {
    /// The distribution of `counts` (`profile id`, `accounts`), rounded to
    /// 5 % buckets that add up to 100. With no accounts, only the client's
    /// default profile.
    #[must_use]
    pub fn from_counts(counts: &[(u8, u64)]) -> Self {
        let total: u64 = counts.iter().map(|(_, c)| *c).sum();
        let mut buckets: Vec<(Argon2Profile, u32)> = Vec::new();
        if total > 0 {
            for (id, c) in counts {
                if let Ok(p) = Argon2Profile::from_id(*id) {
                    // Nearest multiple of 5 of the percentage.
                    let pct = c
                        .saturating_mul(100)
                        .saturating_add(total / 2)
                        .checked_div(total)
                        .unwrap_or(0);
                    let rounded = u32::try_from((pct + 2) / 5 * 5).unwrap_or(0);
                    buckets.push((p, rounded));
                }
            }
            buckets.sort_by_key(|(p, _)| p.id());
            let sum: u32 = buckets.iter().map(|(_, w)| *w).sum();
            // Put the rounding remainder on the largest bucket.
            if let Some(max) = buckets.iter_mut().max_by_key(|(_, w)| *w) {
                max.1 = (max.1 + 100).saturating_sub(sum);
            }
            buckets.retain(|(_, w)| *w > 0);
        }
        if buckets.is_empty() {
            buckets.push((Argon2Profile::DEFAULT, 100));
        }
        Self(buckets)
    }

    /// The profile for a value in `0..100`.
    #[must_use]
    pub fn pick(&self, point: u32) -> Argon2Profile {
        let mut acc = 0u32;
        for (p, w) in &self.0 {
            acc += w;
            if point < acc {
                return *p;
            }
        }
        self.0.last().map_or(Argon2Profile::DEFAULT, |(p, _)| *p)
    }
}

/// The fake profile of `email`.
#[must_use]
pub fn fake_profile(secret: &[u8; 32], dist: &ProfileDist, email: &str) -> Argon2Profile {
    let digest = h(secret, "prelogin/profile", email);
    let mut four = [0u8; 4];
    four.copy_from_slice(digest.get(..4).unwrap_or(&[0u8; 4]));
    dist.pick(u32::from_be_bytes(four) % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_id_is_deterministic_uuidv4_and_independent_of_the_setup_time() {
        let key = [5u8; 32];
        let a = fake_account_id(&key, "a@example.com");
        assert_eq!(a, fake_account_id(&key, "a@example.com"));
        assert_ne!(a, fake_account_id(&key, "b@example.com"));
        assert_ne!(a, fake_account_id(&[6u8; 32], "a@example.com"));
        assert_eq!(a[6] >> 4, 4, "UUID version 4");
        assert_eq!(a[8] >> 6, 0b10, "UUID variant");
        // The first bytes are not a timestamp: the IDs of many emails spread
        // over the whole range.
        let firsts: std::collections::BTreeSet<u8> = (0..64)
            .map(|i| fake_account_id(&key, &format!("u{i}@example.com"))[0])
            .collect();
        assert!(firsts.len() > 30);
    }

    #[test]
    fn distribution_rounds_to_five_percent_and_sums_to_100() {
        let d = ProfileDist::from_counts(&[(2, 70), (3, 29), (1, 1)]);
        let sum: u32 = d.0.iter().map(|(_, w)| *w).sum();
        assert_eq!(sum, 100);
        assert!(d.0.iter().all(|(_, w)| w % 5 == 0), "{d:?}");
        assert_eq!(
            ProfileDist::from_counts(&[]).pick(99),
            Argon2Profile::DEFAULT
        );
        assert_eq!(d.pick(0), d.0[0].0);
    }
}
