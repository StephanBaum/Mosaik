//! Reuse detection: which secrets are equal, without a lookup table keyed by
//! a secret.
//!
//! Each secret is reduced to `HMAC-SHA-256(K, secret)` under a key `K` drawn
//! from the OS CSPRNG for this one call (`mosaic_core::SecretBytes::random`).
//! The tags are sorted and equal tags form a candidate group; within a
//! group, members are then compared byte for byte in constant time
//! (`subtle`), so the groups are exact equivalence classes even in the
//! (cryptographically negligible) case of a tag collision. Sorting compares
//! tags, not secrets, so its timing reveals nothing about a secret to anyone
//! without `K`.
//!
//! `K`, the tags and the order live only for the call: the key is a
//! zeroizing `SecretBytes`, the tags sit in a zeroizing buffer of exactly the
//! right size, and nothing derived from a secret is returned. The result is
//! indices only.

use hmac::{Hmac, KeyInit, Mac};
use mosaic_core::SecretBytes;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::error::{Error, Result};

const TAG_LEN: usize = 32;

/// Groups the indices of `secrets` by equal value. Returns only groups with
/// two or more members, each sorted ascending, and the groups sorted by
/// their first member. Empty secrets are never grouped.
///
/// # Errors
/// [`Error::Random`] if the per-call key cannot be drawn.
pub(crate) fn groups(secrets: &[&[u8]]) -> Result<Vec<Vec<usize>>> {
    let key = SecretBytes::<TAG_LEN>::random().map_err(|_| Error::Random)?;
    groups_with_key(secrets, key.expose_secret())
}

fn groups_with_key(secrets: &[&[u8]], key: &[u8]) -> Result<Vec<Vec<usize>>> {
    let mut tags: Zeroizing<Vec<[u8; TAG_LEN]>> = Zeroizing::new(Vec::with_capacity(secrets.len()));
    for secret in secrets {
        tags.push(tag(key, secret)?);
    }
    Ok(classes(secrets, &tags))
}

/// The exact classes of `secrets`, using `tags` (one per secret) to find
/// candidates.
fn classes(secrets: &[&[u8]], tags: &[[u8; TAG_LEN]]) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..secrets.len())
        .filter(|&i| secrets.get(i).is_some_and(|s| !s.is_empty()))
        .collect();
    order.sort_unstable_by(|&a, &b| tags.get(a).cmp(&tags.get(b)).then(a.cmp(&b)));

    let mut out = Vec::new();
    for run in order.chunk_by(|&a, &b| tags.get(a) == tags.get(b)) {
        // Exact classes inside a run of equal tags: each member joins the
        // first class whose representative it equals in constant time.
        let mut classes: Vec<Vec<usize>> = Vec::new();
        for &i in run {
            let secret = secrets.get(i).copied().unwrap_or_default();
            let home = classes.iter_mut().find(|class| {
                class.first().is_some_and(|&rep| {
                    bool::from(secrets.get(rep).copied().unwrap_or_default().ct_eq(secret))
                })
            });
            match home {
                Some(class) => class.push(i),
                None => classes.push(vec![i]),
            }
        }
        out.extend(classes.into_iter().filter(|c| c.len() >= 2));
    }
    for group in &mut out {
        group.sort_unstable();
    }
    out.sort_unstable();
    out
}

fn tag(key: &[u8], secret: &[u8]) -> Result<[u8; TAG_LEN]> {
    // HMAC takes keys of any length; the error arm is unreachable.
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).map_err(|_| Error::Crypto)?;
    mac.update(secret);
    // `CtOutput` zeroizes on drop (`digest/zeroize`); the copy we return
    // goes straight into the caller's zeroizing buffer.
    let out = mac.finalize();
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(out.as_bytes().as_slice());
    Ok(tag)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn group(values: &[&str]) -> Vec<Vec<usize>> {
        let bytes: Vec<&[u8]> = values.iter().map(|v| v.as_bytes()).collect();
        groups(&bytes).unwrap()
    }

    #[test]
    fn groups_equal_values() {
        assert_eq!(
            group(&["a", "b", "a", "c", "b", "a", "d"]),
            vec![vec![0, 2, 5], vec![1, 4]]
        );
        assert_eq!(group(&[]), Vec::<Vec<usize>>::new());
        assert_eq!(group(&["x"]), Vec::<Vec<usize>>::new());
        assert!(
            group(&["", "", "x"]).is_empty(),
            "empty secrets never group"
        );
        assert_eq!(group(&["Aa", "aA", "Aa"]), vec![vec![0, 2]]);
    }

    #[test]
    fn colliding_tags_still_split_into_exact_classes() {
        // Every tag equal, as if they all collided.
        let values: [&[u8]; 6] = [b"x", b"y", b"x", b"z", b"y", b""];
        let g = classes(&values, &[[0u8; TAG_LEN]; 6]);
        assert_eq!(g, vec![vec![0, 2], vec![1, 4]]);
        // A fixed key gives the same classes as a random one.
        assert_eq!(groups_with_key(&values, &[7u8; 32]).unwrap(), g);
    }

    #[test]
    fn tags_depend_on_the_key() {
        let a = tag(&[1u8; 32], b"secret").unwrap();
        let b = tag(&[2u8; 32], b"secret").unwrap();
        assert_ne!(a, b);
        assert_eq!(a, tag(&[1u8; 32], b"secret").unwrap());
    }

    proptest! {
        /// The groups are exactly the equivalence classes of equal,
        /// non-empty values with two or more members.
        #[test]
        fn groups_are_exact_equivalence_classes(
            values in proptest::collection::vec("[ab]{0,3}", 0..40)
        ) {
            let bytes: Vec<&[u8]> = values.iter().map(String::as_bytes).collect();
            let got = groups(&bytes).unwrap();
            let mut want: Vec<Vec<usize>> = Vec::new();
            for (i, v) in values.iter().enumerate() {
                if v.is_empty() || values[..i].contains(v) {
                    continue;
                }
                let class: Vec<usize> =
                    (i..values.len()).filter(|&j| values[j] == *v).collect();
                if class.len() >= 2 {
                    want.push(class);
                }
            }
            want.sort_unstable();
            prop_assert_eq!(got, want);
        }
    }
}
