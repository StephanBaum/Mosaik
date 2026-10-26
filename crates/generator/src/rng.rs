//! Randomness for the generators (CRYPTO_SPEC §13).
//!
//! Production code draws only from the operating system's CSPRNG
//! ([`OsRandom`], via `getrandom`). The [`RandomSource`] trait exists so the
//! unit tests (and, under `cfg(fuzzing)`, the fuzz targets) can inject a
//! seeded generator and check distributions deterministically. It is
//! crate-private: callers of this crate cannot supply their own RNG.
//!
//! [`uniform_below`] maps 32-bit draws to `0..n` by rejection sampling:
//! the draws `x < 2^32 mod n` are rejected, which leaves an accepted range
//! of `2^32 - (2^32 mod n)` values, an exact multiple of `n`, so `x % n` is
//! exactly uniform. There is no modulo bias.

use core::num::NonZeroU32;

use crate::error::{Error, Result};

/// A source of uniformly distributed 32-bit values.
pub(crate) trait RandomSource {
    /// The next uniformly distributed value.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the source fails.
    fn next_u32(&mut self) -> Result<u32>;
}

/// The operating system's CSPRNG. Stateless: every draw goes to `getrandom`,
/// so no random state is buffered in this process.
pub(crate) struct OsRandom;

impl RandomSource for OsRandom {
    fn next_u32(&mut self) -> Result<u32> {
        getrandom::u32().map_err(|_| Error::Randomness)
    }
}

/// Consecutive rejections after which [`uniform_below`] gives up. For every
/// `n` a single draw is rejected with probability below 1/2 (below
/// `7776 / 2^32` for the sizes this crate uses), so 64 rejections in a row
/// mean the source is broken, not unlucky.
const MAX_CONSECUTIVE_REJECTIONS: u32 = 64;

/// The rejection threshold `2^32 mod n`: draws below it are rejected.
pub(crate) fn rejection_threshold(n: NonZeroU32) -> u32 {
    // (2^32 - n) mod n == 2^32 mod n, computed without 64-bit arithmetic.
    n.get().wrapping_neg() % n
}

/// A uniformly distributed value in `0..n`.
///
/// # Errors
/// [`Error::Randomness`] if the source fails or rejects
/// [`MAX_CONSECUTIVE_REJECTIONS`] draws in a row.
pub(crate) fn uniform_below<R: RandomSource + ?Sized>(rng: &mut R, n: NonZeroU32) -> Result<u32> {
    let threshold = rejection_threshold(n);
    for _ in 0..MAX_CONSECUTIVE_REJECTIONS {
        let x = rng.next_u32()?;
        if x >= threshold {
            return Ok(x % n);
        }
    }
    Err(Error::Randomness)
}

/// A uniform index into a collection of `len` elements.
///
/// # Errors
/// [`Error::Randomness`] as for [`uniform_below`], and also if `len` is 0
/// or does not fit in `u32` (callers never pass such lengths).
pub(crate) fn uniform_index<R: RandomSource + ?Sized>(rng: &mut R, len: usize) -> Result<usize> {
    let n = u32::try_from(len)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or(Error::Randomness)?;
    let v = uniform_below(rng, n)?;
    usize::try_from(v).map_err(|_| Error::Randomness)
}

/// `SplitMix64`, a small deterministic generator for tests and fuzzing only.
/// It is **not** cryptographically secure and is compiled only under
/// `cfg(test)` or `cfg(fuzzing)`.
#[cfg(any(test, fuzzing))]
pub(crate) struct SplitMix64(u64);

#[cfg(any(test, fuzzing))]
impl SplitMix64 {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }
}

#[cfg(any(test, fuzzing))]
impl RandomSource for SplitMix64 {
    fn next_u32(&mut self) -> Result<u32> {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // The high half has the best statistical quality.
        Ok(u32::try_from(z >> 32).unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss, reason = "test statistics")]
pub(crate) mod testing {
    //! Test helpers: scripted sources and a chi-squared check.

    use super::*;

    /// Replays a fixed list of values, then fails with `Randomness`.
    pub(crate) struct Scripted {
        values: Vec<u32>,
        pos: usize,
        pub(crate) draws: usize,
    }

    impl Scripted {
        pub(crate) fn new(values: &[u32]) -> Self {
            Self {
                values: values.to_vec(),
                pos: 0,
                draws: 0,
            }
        }
    }

    impl RandomSource for Scripted {
        fn next_u32(&mut self) -> Result<u32> {
            let v = self
                .values
                .get(self.pos)
                .copied()
                .ok_or(Error::Randomness)?;
            self.pos += 1;
            self.draws += 1;
            Ok(v)
        }
    }

    /// Always returns the same value.
    pub(crate) struct Constant(pub(crate) u32);

    impl RandomSource for Constant {
        fn next_u32(&mut self) -> Result<u32> {
            Ok(self.0)
        }
    }

    /// Always fails, like an unavailable OS CSPRNG.
    pub(crate) struct Failing;

    impl RandomSource for Failing {
        fn next_u32(&mut self) -> Result<u32> {
            Err(Error::Randomness)
        }
    }

    /// Pearson's chi-squared statistic of `counts` against a uniform
    /// expectation.
    pub(crate) fn chi_squared(counts: &[u64]) -> f64 {
        let total: u64 = counts.iter().sum();
        let expected = total as f64 / counts.len() as f64;
        counts
            .iter()
            .map(|&c| {
                let d = c as f64 - expected;
                d * d / expected
            })
            .sum()
    }

    /// The chi-squared critical value for `df` degrees of freedom at
    /// significance 1e-6 (z = 4.753), by the Wilson-Hilferty approximation,
    /// which is accurate to well under 1 % for df >= 2.
    pub(crate) fn chi_squared_critical(df: usize) -> f64 {
        let k = df as f64;
        let z = 4.753_424_3;
        let a = 2.0 / (9.0 * k);
        k * (1.0 - a + z * a.sqrt()).powi(3)
    }

    /// Asserts that `counts` is consistent with a uniform distribution.
    pub(crate) fn assert_uniform(counts: &[u64], what: &str) {
        assert!(counts.len() >= 2);
        let chi2 = chi_squared(counts);
        let crit = chi_squared_critical(counts.len() - 1);
        assert!(
            chi2 < crit,
            "{what}: chi^2 = {chi2:.1} exceeds the critical value {crit:.1} (df = {})",
            counts.len() - 1
        );
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use proptest::prelude::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn threshold_is_two_pow_32_mod_n() {
        for n in [
            1u32,
            2,
            3,
            7,
            10,
            26,
            62,
            94,
            95,
            7776,
            1 << 31,
            (1 << 31) + 1,
            u32::MAX,
        ] {
            let expected = u32::try_from((1u64 << 32) % u64::from(n)).unwrap();
            assert_eq!(rejection_threshold(nz(n)), expected, "n = {n}");
        }
        // Powers of two divide 2^32: nothing is rejected.
        assert_eq!(rejection_threshold(nz(64)), 0);
    }

    #[test]
    fn rejection_boundary_n3() {
        // 2^32 = 3 * 1431655765 + 1, so exactly the draw 0 is rejected.
        assert_eq!(rejection_threshold(nz(3)), 1);
        let mut rng = Scripted::new(&[0, 1]);
        assert_eq!(uniform_below(&mut rng, nz(3)).unwrap(), 1);
        assert_eq!(
            rng.draws, 2,
            "the draw below the threshold must be rejected"
        );

        let mut rng = Scripted::new(&[1]);
        assert_eq!(uniform_below(&mut rng, nz(3)).unwrap(), 1);
        assert_eq!(rng.draws, 1, "the threshold itself is accepted");

        let mut rng = Scripted::new(&[u32::MAX]);
        assert_eq!(uniform_below(&mut rng, nz(3)).unwrap(), u32::MAX % 3);
    }

    #[test]
    fn rejection_boundary_wordlist_and_alphabet_sizes() {
        for n in [7u32, 10, 62, 94, 7776] {
            let t = rejection_threshold(nz(n));
            assert!(t > 0, "n = {n} does not divide 2^32");
            // Just below the threshold: rejected, the next draw is used.
            let mut rng = Scripted::new(&[t - 1, t]);
            assert_eq!(uniform_below(&mut rng, nz(n)).unwrap(), t % n);
            assert_eq!(rng.draws, 2, "n = {n}: t - 1 must be rejected");
            // The accepted range [t, 2^32) has a multiple of n elements,
            // so each residue is hit equally often.
            assert_eq!((u64::from(u32::MAX) + 1 - u64::from(t)) % u64::from(n), 0);
        }
    }

    #[test]
    fn n1_and_powers_of_two_never_reject() {
        let mut rng = Scripted::new(&[0]);
        assert_eq!(uniform_below(&mut rng, nz(1)).unwrap(), 0);
        let mut rng = Scripted::new(&[0]);
        assert_eq!(uniform_below(&mut rng, nz(64)).unwrap(), 0);
    }

    #[test]
    fn broken_source_is_an_error_not_a_hang() {
        // 0 is always below the threshold for n = 3.
        assert_eq!(
            uniform_below(&mut Constant(0), nz(3)),
            Err(Error::Randomness)
        );
        assert_eq!(uniform_below(&mut Failing, nz(3)), Err(Error::Randomness));
    }

    #[test]
    fn uniform_index_rejects_impossible_lengths() {
        assert_eq!(uniform_index(&mut Constant(5), 0), Err(Error::Randomness));
        assert_eq!(uniform_index(&mut Constant(15), 10).unwrap(), 5);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            uniform_index(&mut Constant(5), usize::MAX),
            Err(Error::Randomness)
        );
    }

    #[test]
    fn os_random_produces_varied_values() {
        let mut rng = OsRandom;
        let draws: Vec<u32> = (0..8).map(|_| rng.next_u32().unwrap()).collect();
        assert!(draws.iter().any(|&v| v != draws[0]));
        assert!(uniform_below(&mut rng, nz(94)).unwrap() < 94);
    }

    /// The statistical proof of uniformity: chi-squared over many samples
    /// from a fixed seed, for alphabet and wordlist sizes.
    #[test]
    fn chi_squared_uniform_below() {
        let mut rng = SplitMix64::new(0x6d6f_7361_6963_0001);
        for n in [3u32, 7, 10, 26, 62, 94] {
            let mut counts = vec![0u64; n as usize];
            for _ in 0..(n as usize * 5_000) {
                counts[uniform_below(&mut rng, nz(n)).unwrap() as usize] += 1;
            }
            assert_uniform(&counts, &format!("uniform_below({n})"));
        }
        let mut counts = vec![0u64; 7776];
        for _ in 0..(7776 * 60) {
            counts[uniform_below(&mut rng, nz(7776)).unwrap() as usize] += 1;
        }
        assert_uniform(&counts, "uniform_below(7776)");
    }

    /// The chi-squared check has power: plain `x % n` over a small source
    /// range is visibly biased and is caught, while rejection sampling over
    /// the same source is not.
    #[test]
    fn chi_squared_detects_modulo_bias() {
        // A source of uniform values in 0..256 (one byte), taken mod 3:
        // 0 gets 86/256, 1 and 2 get 85/256 each. Too subtle for 100k
        // samples, so use 0..10 mod 3 (0: 4/10, 1 and 2: 3/10).
        let mut rng = SplitMix64::new(7);
        let mut biased = [0u64; 3];
        let mut fair = [0u64; 3];
        for _ in 0..30_000 {
            let small = uniform_below(&mut rng, nz(10)).unwrap();
            biased[(small % 3) as usize] += 1;
            // Rejection sampling on the same 0..10 source: reject 10 mod 3 = 1
            // value (9), leaving 0..9, a multiple of 3.
            if small < 9 {
                fair[(small % 3) as usize] += 1;
            }
        }
        assert!(chi_squared(&biased) > chi_squared_critical(2), "{biased:?}");
        assert_uniform(&fair, "rejection on a 0..10 source");
    }

    #[test]
    fn critical_values_are_sane() {
        // For df = 2 the exact value is -2 ln(1e-6) = 27.63; Wilson-Hilferty
        // overestimates small df a little, which only makes the uniformity
        // checks more lenient, never flaky. For large df it is accurate.
        let c2 = chi_squared_critical(2);
        assert!((27.0..32.0).contains(&c2), "{c2}");
        let c7775 = chi_squared_critical(7775);
        // Normal approximation: 7775 + 4.753 * sqrt(2 * 7775) = 8368.
        assert!((8340.0..8420.0).contains(&c7775), "{c7775}");
    }

    proptest! {
        #[test]
        fn uniform_below_in_range(n in 1u32.., draws in proptest::collection::vec(any::<u32>(), 1..8)) {
            let mut rng = Scripted::new(&draws);
            match uniform_below(&mut rng, nz(n)) {
                Ok(v) => {
                    prop_assert!(v < n);
                    let used = draws[rng.draws - 1];
                    prop_assert!(used >= rejection_threshold(nz(n)));
                    prop_assert_eq!(v, used % n);
                    // Every earlier draw was below the threshold.
                    for &d in &draws[..rng.draws - 1] {
                        prop_assert!(d < rejection_threshold(nz(n)));
                    }
                }
                Err(e) => {
                    prop_assert_eq!(e, Error::Randomness);
                    prop_assert!(draws.iter().all(|&d| d < rejection_threshold(nz(n))));
                }
            }
        }

        #[test]
        fn accepted_range_is_a_multiple_of_n(n in 1u32..) {
            let t = u64::from(rejection_threshold(nz(n)));
            prop_assert!(t < u64::from(n));
            prop_assert_eq!(((1u64 << 32) - t) % u64::from(n), 0);
        }
    }
}
