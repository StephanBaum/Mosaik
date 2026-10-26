//! The shared generate-then-check engine for random-character passwords.
//!
//! A [`Policy`] is an alphabet of printable ASCII characters, a length and
//! a set of constraints. Generation draws every character independently and
//! uniformly from the alphabet (rejection sampling, see [`crate::rng`]) and
//! then checks the whole candidate. A candidate that fails any constraint
//! is wiped and replaced by a completely fresh one. The output is therefore
//! uniformly distributed over the set of strings that satisfy the
//! constraints: no position is forced, and no class is favored.
//!
//! Before generating, the caller estimates the acceptance probability
//! (`acceptance_*`) and refuses policies below [`MIN_ACCEPTANCE`], so the
//! retry loop finishes quickly and [`MAX_ATTEMPTS`] is not reached in
//! practice.

use zeroize::Zeroizing;

use crate::entropy::Entropy;
use crate::error::{Error, Result};
use crate::rng::{RandomSource, uniform_index};
use crate::secret::GeneratedPassword;

/// Policies whose estimated acceptance probability is below this are
/// refused (expected attempts above 1024).
pub(crate) const MIN_ACCEPTANCE: f64 = 1.0 / 1024.0;

/// Upper bound on candidates per call. With an acceptance probability of
/// at least 2^-10 the chance of reaching it is below e^-48.
pub(crate) const MAX_ATTEMPTS: u32 = 50_000;

/// Most distinct requirement sets the inclusion-exclusion estimate handles
/// (2^12 terms). Real site rules have at most five.
pub(crate) const MAX_OVERLAPPING_REQUIREMENTS: usize = 12;

/// The space character, as a bit in an ASCII mask.
pub(crate) const SPACE_BIT: u128 = 1 << 0x20;

/// A set of ASCII characters as a bit mask: bit `c` is character `c`.
pub(crate) type AsciiMask = u128;

/// The number of characters in `mask`.
pub(crate) fn mask_len(mask: AsciiMask) -> u32 {
    mask.count_ones()
}

/// Whether byte `b` is in `mask`.
pub(crate) fn mask_contains(mask: AsciiMask, b: u8) -> bool {
    b < 128 && mask & (1u128 << b) != 0
}

/// The characters of `mask` in ascending order.
pub(crate) fn mask_bytes(mask: AsciiMask) -> Vec<u8> {
    (0u8..128).filter(|&b| mask_contains(mask, b)).collect()
}

/// "At least `min` characters from `members`".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Requirement {
    pub(crate) members: AsciiMask,
    pub(crate) min: u32,
}

/// Run-length constraints from site rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_field_names,
    reason = "the names mirror the passwordrules properties"
)]
pub(crate) struct Runs {
    /// Longest allowed run of one repeated character (`aaa` is 3).
    pub(crate) max_consecutive: Option<u32>,
    /// Most occurrences of any one character in the whole password.
    pub(crate) max_repeating: Option<u32>,
    /// Longest allowed run of ascending or descending code points
    /// (`abc` and `321` are 3).
    pub(crate) max_sequential: Option<u32>,
}

impl Runs {
    /// Whether any limit can reject a string of `length` characters.
    pub(crate) fn binds(&self, length: u32) -> bool {
        [
            self.max_consecutive,
            self.max_repeating,
            self.max_sequential,
        ]
        .into_iter()
        .flatten()
        .any(|k| k < length)
    }
}

/// An alphabet, a length and constraints, ready to generate.
#[derive(Debug)]
pub(crate) struct Policy {
    alphabet: Vec<u8>,
    length: u32,
    requirements: Vec<Requirement>,
    runs: Runs,
}

impl Policy {
    /// `alphabet` must be a non-empty mask of printable ASCII.
    pub(crate) fn new(
        alphabet: AsciiMask,
        length: u32,
        requirements: Vec<Requirement>,
        runs: Runs,
    ) -> Self {
        Self {
            alphabet: mask_bytes(alphabet),
            length,
            requirements,
            runs,
        }
    }

    #[cfg(test)]
    pub(crate) fn length(&self) -> u32 {
        self.length
    }

    /// `log2(|alphabet|) × length`, an upper bound whenever a constraint
    /// can reject a candidate.
    pub(crate) fn entropy(&self) -> Entropy {
        let constrained =
            self.requirements.iter().any(|r| r.min > 0) || self.runs.binds(self.length);
        let size = u32::try_from(self.alphabet.len()).unwrap_or(u32::MAX);
        Entropy::for_characters(size, self.length, constrained)
    }

    /// Whether `candidate` meets every constraint.
    pub(crate) fn accepts(&self, candidate: &[u8]) -> bool {
        let requirements_met = self.requirements.iter().all(|r| {
            let count = candidate
                .iter()
                .filter(|&&b| mask_contains(r.members, b))
                .count();
            u32::try_from(count).unwrap_or(u32::MAX) >= r.min
        });
        requirements_met && runs_ok(&self.runs, candidate)
    }

    /// Generates a password: fresh uniform candidates until one is accepted.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the source fails, [`Error::RetryLimit`]
    /// after [`MAX_ATTEMPTS`] rejected candidates.
    pub(crate) fn generate<R: RandomSource + ?Sized>(
        &self,
        rng: &mut R,
    ) -> Result<GeneratedPassword> {
        let len = usize::try_from(self.length).map_err(|_| Error::RetryLimit)?;
        let mut candidate = Zeroizing::new(vec![0u8; len]);
        for _ in 0..MAX_ATTEMPTS {
            for slot in candidate.iter_mut() {
                let i = uniform_index(rng, self.alphabet.len())?;
                *slot = self.alphabet.get(i).copied().ok_or(Error::Randomness)?;
            }
            if self.accepts(&candidate) {
                let mut out = GeneratedPassword::with_capacity(len, self.entropy());
                for &b in candidate.iter() {
                    out.push(char::from(b));
                }
                return Ok(out);
            }
        }
        Err(Error::RetryLimit)
    }
}

/// Checks the run-length limits on `s` (a sequence of code points).
pub(crate) fn runs_ok<T: Copy + Into<u32>>(runs: &Runs, s: &[T]) -> bool {
    if let Some(k) = runs.max_consecutive
        && longest_run(s, |a, b| a == b) > k
    {
        return false;
    }
    if let Some(k) = runs.max_sequential
        && (longest_run(s, |a, b| b.checked_sub(a) == Some(1)) > k
            || longest_run(s, |a, b| a.checked_sub(b) == Some(1)) > k)
    {
        return false;
    }
    if let Some(k) = runs.max_repeating {
        let mut seen: Vec<(u32, u32)> = Vec::new();
        for c in s.iter().map(|&c| c.into()) {
            if let Some(entry) = seen.iter_mut().find(|(v, _)| *v == c) {
                entry.1 += 1;
                if entry.1 > k {
                    return false;
                }
            } else {
                if k == 0 {
                    return false;
                }
                seen.push((c, 1));
            }
        }
    }
    true
}

/// The longest run in which every adjacent pair satisfies `linked`.
fn longest_run<T: Copy + Into<u32>>(s: &[T], linked: impl Fn(u32, u32) -> bool) -> u32 {
    let mut best = 0u32;
    let mut current = 0u32;
    let mut prev: Option<u32> = None;
    for c in s.iter().map(|&c| c.into()) {
        current = match prev {
            Some(p) if linked(p, c) => current.saturating_add(1),
            _ => 1,
        };
        best = best.max(current);
        prev = Some(c);
    }
    best
}

/// `0! ..= n!` as `f64` (exact enough; 128! is about 3.9e215).
fn factorials(n: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0).saturating_add(1));
    let mut acc = 1.0f64;
    out.push(acc);
    for i in 1..=n {
        acc *= f64::from(i);
        out.push(acc);
    }
    out
}

/// Exact probability that a uniform string of `length` characters over an
/// alphabet of `alphabet` characters meets every requirement, when the
/// requirement sets are pairwise **disjoint** (random-character mode).
///
/// Multinomial DP over the classes: `P = n! Σ Π (p_i^k_i / k_i!) ·
/// p_rest^r / r!` over all counts with `k_i ≥ min_i`.
pub(crate) fn acceptance_disjoint(alphabet: u32, requirements: &[Requirement], length: u32) -> f64 {
    let a = f64::from(alphabet);
    let fact = factorials(length);
    let n = usize::try_from(length).unwrap_or(0);
    let mut dp = vec![0.0f64; n + 1];
    if let Some(first) = dp.first_mut() {
        *first = 1.0;
    }
    let mut covered = 0u32;
    for r in requirements {
        let size = mask_len(r.members);
        covered = covered.saturating_add(size);
        let p = f64::from(size) / a;
        let min = usize::try_from(r.min).unwrap_or(usize::MAX);
        let mut next = vec![0.0f64; n + 1];
        for (j, &mass) in dp.iter().enumerate() {
            if mass == 0.0 {
                continue;
            }
            for k in min..=n.saturating_sub(j) {
                let term = p.powi(i32::try_from(k).unwrap_or(i32::MAX))
                    / fact.get(k).copied().unwrap_or(f64::INFINITY);
                if let Some(slot) = next.get_mut(j + k) {
                    *slot += mass * term;
                }
            }
        }
        dp = next;
    }
    let p_rest = f64::from(alphabet.saturating_sub(covered)) / a;
    let total: f64 = dp
        .iter()
        .enumerate()
        .map(|(j, &mass)| {
            let r = n - j;
            mass * p_rest.powi(i32::try_from(r).unwrap_or(i32::MAX))
                / fact.get(r).copied().unwrap_or(f64::INFINITY)
        })
        .sum();
    (total * fact.get(n).copied().unwrap_or(0.0)).clamp(0.0, 1.0)
}

/// Exact probability that a uniform string over `alphabet` contains at
/// least one character from each of some (possibly overlapping) sets, by
/// inclusion-exclusion: `P(n) = Σ_{S ⊆ sets} (-1)^|S| ((|A| - |∪S|) / |A|)^n`.
/// The terms do not depend on the length, so they are computed once.
pub(crate) struct Overlapping {
    /// `(fraction of the alphabet outside ∪S, |S| is odd)` per subset `S`.
    terms: Vec<(f64, bool)>,
}

impl Overlapping {
    /// `None` if there are more than [`MAX_OVERLAPPING_REQUIREMENTS`] sets.
    pub(crate) fn new(alphabet: AsciiMask, sets: &[AsciiMask]) -> Option<Self> {
        if sets.len() > MAX_OVERLAPPING_REQUIREMENTS {
            return None;
        }
        let a = f64::from(mask_len(alphabet));
        let terms = (0u32..(1u32 << sets.len()))
            .map(|subset| {
                let union = sets
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| subset & (1 << i) != 0)
                    .fold(0, |acc, (_, &m)| acc | (m & alphabet));
                let outside = f64::from(mask_len(alphabet & !union)) / a;
                (outside, subset.count_ones() % 2 == 1)
            })
            .collect();
        Some(Self { terms })
    }

    /// The acceptance probability for strings of `length` characters.
    pub(crate) fn probability(&self, length: u32) -> f64 {
        let n = i32::try_from(length).unwrap_or(i32::MAX);
        self.terms
            .iter()
            .map(|&(outside, odd)| {
                let term = outside.powi(n);
                if odd { -term } else { term }
            })
            .sum::<f64>()
            .clamp(0.0, 1.0)
    }
}

/// Exact probability that a uniform string of `length` characters over
/// `alphabet` characters has no run of one repeated character longer than
/// `max_consecutive`.
pub(crate) fn acceptance_consecutive(alphabet: u32, max_consecutive: u32, length: u32) -> f64 {
    if length == 0 || max_consecutive >= length {
        return 1.0;
    }
    if max_consecutive == 0 {
        return 0.0;
    }
    let a = f64::from(alphabet);
    let same = 1.0 / a;
    let differ = (a - 1.0) / a;
    let k = usize::try_from(max_consecutive).unwrap_or(usize::MAX);
    // state[r - 1]: probability mass of surviving strings ending in a run of
    // exactly r equal characters.
    let mut state = vec![0.0f64; k];
    if let Some(first) = state.first_mut() {
        *first = 1.0;
    }
    for _ in 1..length {
        let mass: f64 = state.iter().sum();
        let mut next = vec![0.0f64; k];
        if let Some(first) = next.first_mut() {
            *first = mass * differ;
        }
        for (r, &m) in state.iter().enumerate() {
            if let Some(slot) = next.get_mut(r + 1) {
                *slot += m * same;
            }
        }
        state = next;
    }
    state.iter().sum::<f64>().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::rng::testing::{Constant, Failing, assert_uniform};

    const fn bit(c: u8) -> AsciiMask {
        1u128 << c
    }

    fn mask_of(s: &str) -> AsciiMask {
        s.bytes().fold(0, |m, b| m | bit(b))
    }

    /// Brute force: the fraction of all strings over `alphabet` of length
    /// `n` that `policy` accepts.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "small test sizes"
    )]
    fn brute_force(policy: &Policy) -> f64 {
        let a = policy.alphabet.len();
        let n = policy.length as usize;
        let total = a.pow(n as u32);
        let mut ok = 0usize;
        let mut buf = vec![0u8; n];
        for mut code in 0..total {
            for slot in &mut buf {
                *slot = policy.alphabet[code % a];
                code /= a;
            }
            if policy.accepts(&buf) {
                ok += 1;
            }
        }
        ok as f64 / total as f64
    }

    #[test]
    fn mask_helpers() {
        let m = mask_of("ba");
        assert_eq!(mask_bytes(m), b"ab");
        assert_eq!(mask_len(m), 2);
        assert!(mask_contains(m, b'a'));
        assert!(!mask_contains(m, b'c'));
        assert!(!mask_contains(m, 200));
    }

    #[test]
    fn longest_runs() {
        let s: Vec<u8> = b"aabbbcd".to_vec();
        assert_eq!(longest_run(&s, |a, b| a == b), 3);
        assert_eq!(longest_run(&s, |a, b| b.checked_sub(a) == Some(1)), 3); // b c d
        assert_eq!(longest_run::<u8>(&[], |a, b| a == b), 0);
        let desc: Vec<u8> = b"9876a".to_vec();
        assert_eq!(longest_run(&desc, |a, b| a.checked_sub(b) == Some(1)), 4);
    }

    #[test]
    fn runs_checks() {
        let none = Runs::default();
        assert!(runs_ok(&none, b"aaaa".as_slice()));
        let consecutive = Runs {
            max_consecutive: Some(2),
            ..Runs::default()
        };
        assert!(runs_ok(&consecutive, b"aabaa".as_slice()));
        assert!(!runs_ok(&consecutive, b"abbb".as_slice()));
        let repeating = Runs {
            max_repeating: Some(2),
            ..Runs::default()
        };
        assert!(runs_ok(&repeating, b"abab".as_slice()));
        assert!(!runs_ok(&repeating, b"abaca".as_slice()));
        let zero = Runs {
            max_repeating: Some(0),
            ..Runs::default()
        };
        assert!(!runs_ok(&zero, b"a".as_slice()));
        assert!(runs_ok(&zero, b"".as_slice()));
        let sequential = Runs {
            max_sequential: Some(2),
            ..Runs::default()
        };
        assert!(runs_ok(&sequential, b"abxba".as_slice()));
        assert!(!runs_ok(&sequential, b"xabc".as_slice()));
        assert!(!runs_ok(&sequential, b"321".as_slice()));
        assert!(
            Runs {
                max_consecutive: Some(3),
                ..Runs::default()
            }
            .binds(4)
        );
        assert!(
            !Runs {
                max_consecutive: Some(4),
                ..Runs::default()
            }
            .binds(4)
        );
        assert!(!none.binds(100));
    }

    #[test]
    fn disjoint_estimate_matches_brute_force() {
        let alphabet = mask_of("abcde01");
        for (reqs, n) in [
            (vec![(mask_of("ab"), 1), (mask_of("01"), 1)], 3),
            (vec![(mask_of("ab"), 2), (mask_of("01"), 1)], 4),
            (vec![(mask_of("abcde"), 1), (mask_of("01"), 2)], 4),
            (vec![(mask_of("abcde01"), 5)], 5),
            (vec![(mask_of("a"), 3)], 3),
            (vec![], 3),
        ] {
            let reqs: Vec<Requirement> = reqs
                .into_iter()
                .map(|(members, min)| Requirement { members, min })
                .collect();
            let p = acceptance_disjoint(7, &reqs, n);
            let policy = Policy::new(alphabet, n, reqs, Runs::default());
            let exact = brute_force(&policy);
            assert!((p - exact).abs() < 1e-9, "{p} vs {exact}");
        }
    }

    #[test]
    fn overlapping_estimate_matches_brute_force() {
        let alphabet = mask_of("abcd01");
        for (sets, n) in [
            (vec![mask_of("ab"), mask_of("bc"), mask_of("0")], 3),
            (
                vec![mask_of("a"), mask_of("b"), mask_of("c"), mask_of("d")],
                4,
            ),
            (
                vec![mask_of("a"), mask_of("b"), mask_of("c"), mask_of("d")],
                3,
            ),
            (vec![mask_of("abcd01")], 2),
            (vec![], 2),
        ] {
            let p = Overlapping::new(alphabet, &sets).unwrap().probability(n);
            let reqs = sets
                .iter()
                .map(|&members| Requirement { members, min: 1 })
                .collect();
            let exact = brute_force(&Policy::new(alphabet, n, reqs, Runs::default()));
            assert!((p - exact).abs() < 1e-9, "{p} vs {exact}");
        }
        let too_many = vec![bit(b'a'); MAX_OVERLAPPING_REQUIREMENTS + 1];
        assert!(Overlapping::new(alphabet, &too_many).is_none());
    }

    #[test]
    fn consecutive_estimate_matches_brute_force() {
        let alphabet = mask_of("abc");
        for (k, n) in [(1, 4), (2, 5), (3, 6), (6, 6), (0, 2), (1, 1)] {
            let runs = Runs {
                max_consecutive: Some(k),
                ..Runs::default()
            };
            let exact = brute_force(&Policy::new(alphabet, n, vec![], runs));
            let p = acceptance_consecutive(3, k, n);
            assert!((p - exact).abs() < 1e-9, "k={k} n={n}: {p} vs {exact}");
        }
        assert_eq!(acceptance_consecutive(1, 1, 2), 0.0);
        assert_eq!(acceptance_consecutive(5, 3, 0), 1.0);
    }

    /// Generate-then-check is uniform over the accepted strings: alphabet
    /// `ab01`, length 3, at least one letter and one digit. 48 of the 64
    /// strings qualify and each must be equally likely.
    #[test]
    fn chi_squared_uniform_over_accepted_strings() {
        let alphabet = mask_of("ab01");
        let reqs = vec![
            Requirement {
                members: mask_of("ab"),
                min: 1,
            },
            Requirement {
                members: mask_of("01"),
                min: 1,
            },
        ];
        let policy = Policy::new(alphabet, 3, reqs, Runs::default());
        let accepted: Vec<Vec<u8>> = (0..64usize)
            .map(|code| (0..3).map(|i| b"ab01"[(code >> (2 * i)) & 3]).collect())
            .filter(|s: &Vec<u8>| policy.accepts(s))
            .collect();
        assert_eq!(accepted.len(), 48);
        let mut counts = vec![0u64; 48];
        let mut rng = SplitMix64::new(0x0123_4567_89ab_cdef);
        for _ in 0..48 * 2_000 {
            let p = policy.generate(&mut rng).unwrap();
            let pos = accepted
                .iter()
                .position(|s| s.as_slice() == p.expose_secret().as_bytes())
                .unwrap();
            counts[pos] += 1;
        }
        assert_uniform(&counts, "generate-then-check over 48 strings");
    }

    /// The same check rejects a forced-position generator (put a letter in
    /// position 0 and a digit in position 1, fill the rest freely), which is
    /// the biased technique this crate avoids.
    #[test]
    fn chi_squared_detects_forced_positions() {
        let alphabet = *b"ab01";
        let policy = Policy::new(
            mask_of("ab01"),
            3,
            vec![
                Requirement {
                    members: mask_of("ab"),
                    min: 1,
                },
                Requirement {
                    members: mask_of("01"),
                    min: 1,
                },
            ],
            Runs::default(),
        );
        let accepted: Vec<Vec<u8>> = (0..64usize)
            .map(|code| (0..3).map(|i| alphabet[(code >> (2 * i)) & 3]).collect())
            .filter(|s: &Vec<u8>| policy.accepts(s))
            .collect();
        let mut counts = vec![0u64; 48];
        let mut rng = SplitMix64::new(99);
        for _ in 0..48 * 2_000 {
            let letter = b"ab"[uniform_index(&mut rng, 2).unwrap()];
            let digit = b"01"[uniform_index(&mut rng, 2).unwrap()];
            let free = alphabet[uniform_index(&mut rng, 4).unwrap()];
            let mut s = vec![letter, digit, free];
            // Even shuffling the positions afterwards stays biased.
            let j = uniform_index(&mut rng, 3).unwrap();
            s.swap(0, j);
            let pos = accepted.iter().position(|a| *a == s).unwrap();
            counts[pos] += 1;
        }
        let chi2 = crate::rng::testing::chi_squared(&counts);
        assert!(
            chi2 > crate::rng::testing::chi_squared_critical(47),
            "forced positions should look biased, chi^2 = {chi2}"
        );
    }

    #[test]
    fn retry_limit_and_rng_failure() {
        // The alphabet is sorted ("0a"); a constant odd draw always picks
        // index 1, so every candidate is "aaaa", which never has a digit.
        let policy = Policy::new(
            mask_of("a0"),
            4,
            vec![Requirement {
                members: mask_of("0"),
                min: 1,
            }],
            Runs::default(),
        );
        assert_eq!(
            policy.generate(&mut Constant(1)).unwrap_err(),
            Error::RetryLimit
        );
        assert_eq!(
            policy.generate(&mut Failing).unwrap_err(),
            Error::Randomness
        );
    }

    #[test]
    fn generated_buffer_is_exact_size() {
        let policy = Policy::new(mask_of("abc"), 17, vec![], Runs::default());
        let p = policy.generate(&mut SplitMix64::new(1)).unwrap();
        assert_eq!(p.expose_secret().len(), 17);
        assert_eq!(p.capacity(), 17);
        assert!(!p.entropy().is_upper_bound());
        assert_eq!(policy.length(), 17);
    }
}
