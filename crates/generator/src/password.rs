//! Random-character passwords with configurable classes.

use crate::entropy::Entropy;
use crate::error::{Error, Result};
use crate::policy::{
    AsciiMask, MIN_ACCEPTANCE, Policy, Requirement, Runs, acceptance_disjoint, mask_len,
};
use crate::rng::{OsRandom, RandomSource};
use crate::secret::GeneratedPassword;

/// Shortest password [`generate_password`] produces.
pub const MIN_LENGTH: u32 = 4;
/// Longest password any generator in this crate produces.
pub const MAX_LENGTH: u32 = 128;
/// Default password length.
pub const DEFAULT_LENGTH: u32 = 20;

/// The default symbol set: all 32 ASCII punctuation characters (Apple's
/// `special` class without the space).
pub const DEFAULT_SYMBOLS: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

/// Characters removed by [`PasswordOptions::exclude_ambiguous`]: those
/// commonly confused with each other in many fonts (`0 O o`, `1 I l |`).
pub const AMBIGUOUS_CHARACTERS: &str = "0Oo1Il|";

const fn range(lo: u8, hi: u8) -> AsciiMask {
    let mut m = 0u128;
    let mut c = lo;
    while c <= hi {
        m |= 1 << c;
        c += 1;
    }
    m
}

pub(crate) const LOWER: AsciiMask = range(b'a', b'z');
pub(crate) const UPPER: AsciiMask = range(b'A', b'Z');
pub(crate) const DIGITS: AsciiMask = range(b'0', b'9');

/// The ASCII characters of `s` as a mask, or `None` if `s` has anything
/// outside ASCII.
pub(crate) fn ascii_mask(s: &str) -> Option<AsciiMask> {
    s.chars().try_fold(0u128, |m, c| {
        if c.is_ascii() {
            Some(m | (1u128 << u32::from(c)))
        } else {
            None
        }
    })
}

/// Settings for [`generate_password`].
///
/// Each class is `None` (excluded) or `Some(min)`: included, with at least
/// `min` characters from it. Minimums are met by generating uniformly from
/// the whole alphabet and retrying until they hold, never by forcing
/// positions, so every qualifying password is equally likely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordOptions {
    /// Length in characters, [`MIN_LENGTH`] to [`MAX_LENGTH`].
    pub length: u32,
    /// Lowercase letters `a-z`.
    pub lowercase: Option<u32>,
    /// Uppercase letters `A-Z`.
    pub uppercase: Option<u32>,
    /// Digits `0-9`.
    pub digits: Option<u32>,
    /// Symbols from [`PasswordOptions::symbol_set`].
    pub symbols: Option<u32>,
    /// The symbols to use: ASCII punctuation only. Duplicates are ignored.
    pub symbol_set: String,
    /// Remove [`AMBIGUOUS_CHARACTERS`] from every class.
    pub exclude_ambiguous: bool,
}

impl Default for PasswordOptions {
    /// 20 characters from all four classes, at least one of each.
    fn default() -> Self {
        Self {
            length: DEFAULT_LENGTH,
            lowercase: Some(1),
            uppercase: Some(1),
            digits: Some(1),
            symbols: Some(1),
            symbol_set: DEFAULT_SYMBOLS.to_owned(),
            exclude_ambiguous: false,
        }
    }
}

impl PasswordOptions {
    /// The entropy [`generate_password`] would report for these options,
    /// without generating anything (for a live display).
    ///
    /// # Errors
    /// The validation errors of [`generate_password`].
    pub fn entropy(&self) -> Result<Entropy> {
        Ok(self.policy()?.entropy())
    }

    /// Validates the options and builds the generation policy.
    pub(crate) fn policy(&self) -> Result<Policy> {
        if !(MIN_LENGTH..=MAX_LENGTH).contains(&self.length) {
            return Err(Error::InvalidLength {
                min: MIN_LENGTH,
                max: MAX_LENGTH,
            });
        }
        let symbols = if self.symbols.is_some() {
            let mask = ascii_mask(&self.symbol_set).ok_or(Error::InvalidSymbolSet)?;
            let only_punctuation = self.symbol_set.chars().all(|c| c.is_ascii_punctuation());
            if mask == 0 || !only_punctuation {
                return Err(Error::InvalidSymbolSet);
            }
            mask
        } else {
            0
        };
        let ambiguous = if self.exclude_ambiguous {
            ascii_mask(AMBIGUOUS_CHARACTERS).unwrap_or(0)
        } else {
            0
        };

        let mut alphabet = 0u128;
        let mut requirements = Vec::with_capacity(4);
        let mut min_total = 0u32;
        for (class, members) in [
            (self.lowercase, LOWER),
            (self.uppercase, UPPER),
            (self.digits, DIGITS),
            (self.symbols, symbols),
        ] {
            let Some(min) = class else { continue };
            let members = members & !ambiguous;
            if members == 0 {
                return Err(Error::EmptyCharacterClass);
            }
            alphabet |= members;
            min_total = min_total.saturating_add(min);
            if min > 0 {
                requirements.push(Requirement { members, min });
            }
        }
        if alphabet == 0 {
            return Err(Error::NoCharacterClasses);
        }
        if min_total > self.length {
            return Err(Error::MinimumsExceedLength);
        }
        // The classes are disjoint, so the estimate is exact.
        if acceptance_disjoint(mask_len(alphabet), &requirements, self.length) < MIN_ACCEPTANCE {
            return Err(Error::ConstraintsTooStrict);
        }
        Ok(Policy::new(
            alphabet,
            self.length,
            requirements,
            Runs::default(),
        ))
    }
}

/// Generates a random-character password from the OS CSPRNG.
///
/// # Errors
/// - [`Error::InvalidLength`] if the length is outside
///   [`MIN_LENGTH`]..=[`MAX_LENGTH`].
/// - [`Error::NoCharacterClasses`] if every class is `None`.
/// - [`Error::InvalidSymbolSet`] if symbols are enabled and the set is empty
///   or not all ASCII punctuation.
/// - [`Error::EmptyCharacterClass`] if excluding ambiguous characters
///   empties an enabled class.
/// - [`Error::MinimumsExceedLength`] if the minimums add up to more than
///   the length.
/// - [`Error::ConstraintsTooStrict`] if fewer than 1 in 1024 random strings
///   would meet the minimums.
/// - [`Error::Randomness`] if the OS CSPRNG fails.
pub fn generate_password(options: &PasswordOptions) -> Result<GeneratedPassword> {
    generate_password_with(options, &mut OsRandom::new())
}

pub(crate) fn generate_password_with<R: RandomSource + ?Sized>(
    options: &PasswordOptions,
    rng: &mut R,
) -> Result<GeneratedPassword> {
    options.policy()?.generate(rng)
}

#[cfg(test)]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "test statistics"
)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::rng::testing::assert_uniform;
    use proptest::prelude::*;

    fn count(s: &str, mask: AsciiMask) -> u32 {
        s.bytes().filter(|&b| mask & (1u128 << b) != 0).count() as u32
    }

    #[test]
    fn default_password() {
        let p = generate_password(&PasswordOptions::default()).unwrap();
        let s = p.expose_secret();
        assert_eq!(s.len(), 20);
        assert!(count(s, LOWER) >= 1);
        assert!(count(s, UPPER) >= 1);
        assert!(count(s, DIGITS) >= 1);
        assert!(count(s, ascii_mask(DEFAULT_SYMBOLS).unwrap()) >= 1);
        let e = p.entropy();
        assert!((e.bits() - 94f64.log2() * 20.0).abs() < 1e-9);
        assert!(e.is_upper_bound(), "minimums constrain the output");
        assert_eq!(p.capacity(), 20);
    }

    #[test]
    fn default_symbols_are_all_ascii_punctuation() {
        assert_eq!(DEFAULT_SYMBOLS.len(), 32);
        let all: String = (0u8..128)
            .map(char::from)
            .filter(char::is_ascii_punctuation)
            .collect();
        assert_eq!(DEFAULT_SYMBOLS, all);
    }

    #[test]
    fn unconstrained_entropy_is_exact() {
        let o = PasswordOptions {
            length: 16,
            lowercase: Some(0),
            uppercase: None,
            digits: Some(0),
            symbols: None,
            ..PasswordOptions::default()
        };
        let e = o.entropy().unwrap();
        assert!((e.bits() - 36f64.log2() * 16.0).abs() < 1e-9);
        assert!(!e.is_upper_bound());
    }

    #[test]
    fn validation_errors() {
        let base = PasswordOptions::default();
        for length in [0, 3, 129, u32::MAX] {
            let o = PasswordOptions {
                length,
                ..base.clone()
            };
            assert_eq!(
                generate_password(&o).unwrap_err(),
                Error::InvalidLength { min: 4, max: 128 }
            );
        }
        let none = PasswordOptions {
            lowercase: None,
            uppercase: None,
            digits: None,
            symbols: None,
            ..base.clone()
        };
        assert_eq!(none.entropy().unwrap_err(), Error::NoCharacterClasses);
        for set in ["", "abc!", "!!a", "€", " ", "!\t"] {
            let o = PasswordOptions {
                symbol_set: set.to_owned(),
                ..base.clone()
            };
            assert_eq!(o.entropy().unwrap_err(), Error::InvalidSymbolSet, "{set:?}");
        }
        // An invalid symbol set is irrelevant when symbols are off.
        let o = PasswordOptions {
            symbols: None,
            symbol_set: "abc".to_owned(),
            ..base.clone()
        };
        assert!(o.entropy().is_ok());
        let o = PasswordOptions {
            symbol_set: "|".to_owned(),
            exclude_ambiguous: true,
            ..base.clone()
        };
        assert_eq!(o.entropy().unwrap_err(), Error::EmptyCharacterClass);
        let o = PasswordOptions {
            length: 4,
            lowercase: Some(2),
            uppercase: Some(2),
            digits: Some(1),
            ..base.clone()
        };
        assert_eq!(o.entropy().unwrap_err(), Error::MinimumsExceedLength);
        let o = PasswordOptions {
            length: 10,
            digits: Some(10),
            lowercase: Some(0),
            uppercase: Some(0),
            symbols: Some(0),
            ..base.clone()
        };
        assert_eq!(o.entropy().unwrap_err(), Error::ConstraintsTooStrict);
        let o = PasswordOptions {
            lowercase: Some(u32::MAX),
            ..base
        };
        assert_eq!(o.entropy().unwrap_err(), Error::MinimumsExceedLength);
    }

    #[test]
    fn exclude_ambiguous_removes_lookalikes() {
        let o = PasswordOptions {
            length: 128,
            exclude_ambiguous: true,
            ..PasswordOptions::default()
        };
        let e = o.entropy().unwrap();
        // 94 minus 0 O o 1 I l | = 87.
        assert!((e.bits() - 87f64.log2() * 128.0).abs() < 1e-9);
        let mut rng = SplitMix64::new(3);
        for _ in 0..50 {
            let p = generate_password_with(&o, &mut rng).unwrap();
            assert!(
                !p.expose_secret()
                    .chars()
                    .any(|c| AMBIGUOUS_CHARACTERS.contains(c))
            );
        }
    }

    #[test]
    fn custom_symbol_set_with_duplicates() {
        let o = PasswordOptions {
            length: 64,
            lowercase: None,
            uppercase: None,
            digits: None,
            symbols: Some(1),
            symbol_set: "!!@@#".to_owned(),
            exclude_ambiguous: false,
        };
        assert!((o.entropy().unwrap().bits() - 3f64.log2() * 64.0).abs() < 1e-9);
        let p = generate_password_with(&o, &mut SplitMix64::new(4)).unwrap();
        assert!(p.expose_secret().chars().all(|c| "!@#".contains(c)));
    }

    /// Position-by-position uniformity: every alphabet character is equally
    /// likely at every position when no minimums apply.
    #[test]
    fn chi_squared_character_frequencies() {
        let o = PasswordOptions {
            length: 32,
            lowercase: Some(0),
            uppercase: Some(0),
            digits: Some(0),
            symbols: Some(0),
            ..PasswordOptions::default()
        };
        let alphabet: Vec<u8> = (b'!'..=b'~').collect();
        assert_eq!(alphabet.len(), 94);
        let mut rng = SplitMix64::new(0xfeed);
        let mut counts = vec![0u64; 94];
        let mut first = vec![0u64; 94];
        for _ in 0..10_000 {
            let p = generate_password_with(&o, &mut rng).unwrap();
            let bytes = p.expose_secret().as_bytes();
            for &b in bytes {
                counts[(b - b'!') as usize] += 1;
            }
            first[(bytes[0] - b'!') as usize] += 1;
        }
        assert_uniform(&counts, "all positions");
        assert_uniform(&first, "first position");
    }

    fn class() -> impl Strategy<Value = Option<u32>> {
        prop_oneof![Just(None), (0u32..4).prop_map(Some)]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn generated_passwords_meet_options(
            length in MIN_LENGTH..=MAX_LENGTH,
            lowercase in class(),
            uppercase in class(),
            digits in class(),
            symbols in class(),
            symbol_set in "[!-/:-@\\[-`{-~]{1,10}",
            exclude_ambiguous in any::<bool>(),
            seed in any::<u64>(),
        ) {
            let o = PasswordOptions {
                length, lowercase, uppercase, digits, symbols,
                symbol_set: symbol_set.clone(), exclude_ambiguous,
            };
            match generate_password_with(&o, &mut SplitMix64::new(seed)) {
                Ok(p) => {
                    let s = p.expose_secret();
                    prop_assert_eq!(s.len(), length as usize);
                    let amb = if exclude_ambiguous { ascii_mask(AMBIGUOUS_CHARACTERS).unwrap() } else { 0 };
                    let sym = ascii_mask(&symbol_set).unwrap();
                    let mut alphabet = 0u128;
                    for (opt, mask) in [(lowercase, LOWER), (uppercase, UPPER), (digits, DIGITS), (symbols, sym)] {
                        if let Some(min) = opt {
                            let m = mask & !amb;
                            alphabet |= m;
                            prop_assert!(count(s, m) >= min);
                        }
                    }
                    prop_assert_eq!(count(s, alphabet), length);
                    prop_assert_eq!(p.capacity(), length as usize);
                }
                Err(e) => prop_assert!(matches!(
                    e,
                    Error::NoCharacterClasses | Error::EmptyCharacterClass
                        | Error::MinimumsExceedLength | Error::ConstraintsTooStrict
                ), "{e:?}"),
            }
        }
    }
}
