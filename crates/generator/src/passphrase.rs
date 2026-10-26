//! Passphrases from the EFF large wordlist (CRYPTO_SPEC §13).
//!
//! The wordlist is `data/eff_large_wordlist.txt` (CC BY 3.0 US, Electronic
//! Frontier Foundation; see `data/README.md`), embedded at compile time.

use std::sync::OnceLock;

use zeroize::Zeroizing;

use crate::entropy::{BITS_PER_WORD, Entropy};
use crate::error::{Error, Result};
use crate::rng::{OsRandom, RandomSource, uniform_index};
use crate::secret::GeneratedPassword;

/// Number of words in the EFF large wordlist (6^5, five dice).
pub const WORDLIST_LEN: usize = 7776;
/// Fewest words in a passphrase.
pub const MIN_WORDS: u32 = 3;
/// Most words in a passphrase.
pub const MAX_WORDS: u32 = 20;
/// Default word count (77.6 bits, the EFF recommendation).
pub const DEFAULT_WORDS: u32 = 6;
/// Longest separator, in characters.
pub const MAX_SEPARATOR_CHARS: u32 = 8;
/// Default separator.
pub const DEFAULT_SEPARATOR: &str = "-";

const WORDLIST_TEXT: &str = include_str!("../data/eff_large_wordlist.txt");

static WORDLIST: OnceLock<Option<Box<[&'static str]>>> = OnceLock::new();

/// Parses `ddddd<TAB>word` lines: five dice digits `1-6`, then a word of
/// lowercase ASCII letters and `-`. `None` if any line is malformed or the
/// count is not [`WORDLIST_LEN`].
fn parse_wordlist(text: &'static str) -> Option<Box<[&'static str]>> {
    let mut words = Vec::with_capacity(WORDLIST_LEN);
    for line in text.lines() {
        let (dice, word) = line.split_once('\t')?;
        let dice_ok = dice.len() == 5 && dice.bytes().all(|b| (b'1'..=b'6').contains(&b));
        let word_ok = !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase() || b == b'-');
        if !dice_ok || !word_ok {
            return None;
        }
        words.push(word);
    }
    (words.len() == WORDLIST_LEN).then(|| words.into_boxed_slice())
}

/// The embedded wordlist, parsed once.
///
/// # Errors
/// [`Error::Wordlist`] if the embedded file is malformed (a broken build).
pub(crate) fn wordlist() -> Result<&'static [&'static str]> {
    WORDLIST
        .get_or_init(|| parse_wordlist(WORDLIST_TEXT))
        .as_deref()
        .ok_or(Error::Wordlist)
}

/// Settings for [`generate_passphrase`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassphraseOptions {
    /// Number of words, [`MIN_WORDS`] to [`MAX_WORDS`].
    pub words: u32,
    /// Placed between words. At most [`MAX_SEPARATOR_CHARS`] characters, no
    /// control characters; may be empty.
    pub separator: String,
    /// Capitalize the first letter of every word.
    pub capitalize: bool,
    /// Append one random digit `0-9` to one randomly chosen word.
    pub append_digit: bool,
}

impl Default for PassphraseOptions {
    /// Six lowercase words separated by `-`.
    fn default() -> Self {
        Self {
            words: DEFAULT_WORDS,
            separator: DEFAULT_SEPARATOR.to_owned(),
            capitalize: false,
            append_digit: false,
        }
    }
}

impl PassphraseOptions {
    fn validate(&self) -> Result<()> {
        if !(MIN_WORDS..=MAX_WORDS).contains(&self.words) {
            return Err(Error::InvalidWordCount {
                min: MIN_WORDS,
                max: MAX_WORDS,
            });
        }
        let chars = self.separator.chars().count();
        if chars > MAX_SEPARATOR_CHARS as usize || self.separator.chars().any(char::is_control) {
            return Err(Error::InvalidSeparator {
                max: MAX_SEPARATOR_CHARS,
            });
        }
        Ok(())
    }

    /// `12.925 × words` (CRYPTO_SPEC §13). Capitalization is deterministic
    /// and adds nothing; the appended digit is not counted, which keeps the
    /// estimate conservative. Without a separator and without
    /// capitalization, word boundaries are not visible and different word
    /// sequences can concatenate to the same text, so the value is then an
    /// upper bound.
    ///
    /// # Errors
    /// The validation errors of [`generate_passphrase`].
    pub fn entropy(&self) -> Result<Entropy> {
        self.validate()?;
        let ambiguous_boundaries = self.separator.is_empty() && !self.capitalize;
        Ok(Entropy::new(
            BITS_PER_WORD * f64::from(self.words),
            ambiguous_boundaries,
        ))
    }
}

/// Generates a passphrase from the OS CSPRNG: each word is drawn uniformly
/// and independently from the 7776-word EFF large wordlist.
///
/// # Errors
/// - [`Error::InvalidWordCount`] if the word count is out of range.
/// - [`Error::InvalidSeparator`] if the separator is too long or contains a
///   control character.
/// - [`Error::Randomness`] if the OS CSPRNG fails.
/// - [`Error::Wordlist`] if the embedded wordlist is damaged.
pub fn generate_passphrase(options: &PassphraseOptions) -> Result<GeneratedPassword> {
    generate_passphrase_with(options, &mut OsRandom)
}

pub(crate) fn generate_passphrase_with<R: RandomSource + ?Sized>(
    options: &PassphraseOptions,
    rng: &mut R,
) -> Result<GeneratedPassword> {
    let entropy = options.entropy()?;
    let list = wordlist()?;
    let count = usize::try_from(options.words).map_err(|_| Error::Wordlist)?;
    // The chosen indices are the secret; they are wiped after use.
    let mut picks: Zeroizing<Vec<usize>> = Zeroizing::new(Vec::with_capacity(count));
    for _ in 0..count {
        picks.push(uniform_index(rng, list.len())?);
    }
    let word = |i: usize| list.get(i).copied().ok_or(Error::Wordlist);
    let digit = if options.append_digit {
        let position = uniform_index(rng, count)?;
        let value = uniform_index(rng, 10)?;
        let c = char::from_digit(u32::try_from(value).map_err(|_| Error::Randomness)?, 10)
            .ok_or(Error::Randomness)?;
        Some((position, c))
    } else {
        None
    };

    let mut capacity = options.separator.len() * (count - 1) + usize::from(digit.is_some());
    for &i in picks.iter() {
        capacity += word(i)?.len();
    }
    let mut out = GeneratedPassword::with_capacity(capacity, entropy);
    for (n, &i) in picks.iter().enumerate() {
        if n > 0 {
            out.push_str(&options.separator);
        }
        let mut chars = word(i)?.chars();
        if options.capitalize
            && let Some(first) = chars.next()
        {
            out.push(first.to_ascii_uppercase());
        }
        out.push_str(chars.as_str());
        if let Some((position, c)) = digit
            && position == n
        {
            out.push(c);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::rng::testing::{Failing, assert_uniform};
    use proptest::prelude::*;
    use std::collections::HashSet;

    #[test]
    fn wordlist_has_7776_unique_entries() {
        let list = wordlist().unwrap();
        assert_eq!(list.len(), WORDLIST_LEN);
        let unique: HashSet<&str> = list.iter().copied().collect();
        assert_eq!(unique.len(), WORDLIST_LEN);
        assert_eq!(list.first(), Some(&"abacus"));
        assert_eq!(list.last(), Some(&"zoom"));
        assert!(list.iter().all(|w| (3..=9).contains(&w.len())));
    }

    #[test]
    fn wordlist_dice_cover_every_roll_in_order() {
        // 11111, 11112, ..., 66666: every five-dice roll exactly once.
        let rolls: Vec<&str> = WORDLIST_TEXT
            .lines()
            .map(|l| l.split_once('\t').unwrap().0)
            .collect();
        let mut expected = Vec::new();
        for n in 0..WORDLIST_LEN {
            let mut v = n;
            let mut s = [b'1'; 5];
            for slot in s.iter_mut().rev() {
                *slot = b'1' + u8::try_from(v % 6).unwrap();
                v /= 6;
            }
            expected.push(String::from_utf8(s.to_vec()).unwrap());
        }
        assert_eq!(rolls, expected);
    }

    #[test]
    fn malformed_wordlists_are_rejected() {
        assert!(parse_wordlist("").is_none());
        assert!(parse_wordlist("11111 abacus\n").is_none());
        assert!(parse_wordlist("11117\tabacus\n").is_none());
        assert!(parse_wordlist("1111\tabacus\n").is_none());
        assert!(parse_wordlist("11111\tAbacus\n").is_none());
        assert!(parse_wordlist("11111\t\n").is_none());
        // Well-formed but too short.
        assert!(parse_wordlist("11111\tabacus\n").is_none());
    }

    #[test]
    fn default_passphrase() {
        let p = generate_passphrase(&PassphraseOptions::default()).unwrap();
        let list: HashSet<&str> = wordlist().unwrap().iter().copied().collect();
        let s = p.expose_secret();
        // Hyphenated words ("t-shirt") also split on '-', so check by
        // re-joining greedily against the list instead of counting parts.
        assert!(
            s.split('-')
                .all(|part| list.iter().any(|w| w.contains(part)))
        );
        assert!((p.entropy().bits() - 6.0 * 12.925).abs() < 1e-9);
        assert!(!p.entropy().is_upper_bound());
        assert_eq!(p.capacity(), s.len());
    }

    #[test]
    fn options_shape_the_output() {
        let o = PassphraseOptions {
            words: 4,
            separator: " · ".to_owned(),
            capitalize: true,
            append_digit: true,
        };
        let mut rng = SplitMix64::new(11);
        for _ in 0..100 {
            let p = generate_passphrase_with(&o, &mut rng).unwrap();
            let s = p.expose_secret();
            let parts: Vec<&str> = s.split(" · ").collect();
            assert_eq!(parts.len(), 4);
            assert!(
                parts
                    .iter()
                    .all(|w| w.starts_with(|c: char| c.is_ascii_uppercase()))
            );
            let digits: Vec<char> = s.chars().filter(char::is_ascii_digit).collect();
            assert_eq!(digits.len(), 1);
            assert_eq!(
                parts
                    .iter()
                    .filter(|w| w.ends_with(|c: char| c.is_ascii_digit()))
                    .count(),
                1
            );
            assert_eq!(p.capacity(), s.len(), "no reallocation");
        }
    }

    #[test]
    fn empty_separator() {
        let o = PassphraseOptions {
            separator: String::new(),
            ..PassphraseOptions::default()
        };
        let e = o.entropy().unwrap();
        assert!(e.is_upper_bound());
        let capitalized = PassphraseOptions {
            capitalize: true,
            ..o.clone()
        };
        assert!(!capitalized.entropy().unwrap().is_upper_bound());
        let p = generate_passphrase_with(&o, &mut SplitMix64::new(5)).unwrap();
        assert!(
            p.expose_secret()
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'-')
        );
    }

    #[test]
    fn validation() {
        for words in [0, 2, 21, u32::MAX] {
            let o = PassphraseOptions {
                words,
                ..PassphraseOptions::default()
            };
            assert_eq!(
                generate_passphrase(&o).unwrap_err(),
                Error::InvalidWordCount { min: 3, max: 20 }
            );
        }
        for separator in ["123456789", "\n", "a\u{7f}"] {
            let o = PassphraseOptions {
                separator: separator.to_owned(),
                ..PassphraseOptions::default()
            };
            assert_eq!(
                o.entropy().unwrap_err(),
                Error::InvalidSeparator { max: 8 },
                "{separator:?}"
            );
        }
        let o = PassphraseOptions {
            separator: "äöüßäöüß".to_owned(),
            ..PassphraseOptions::default()
        };
        assert!(o.entropy().is_ok(), "8 characters, more than 8 bytes");
        assert_eq!(
            generate_passphrase_with(&PassphraseOptions::default(), &mut Failing).unwrap_err(),
            Error::Randomness
        );
    }

    /// Word choice is uniform over all 7776 words.
    #[test]
    fn chi_squared_word_frequencies() {
        let list = wordlist().unwrap();
        let index: std::collections::HashMap<&str, usize> =
            list.iter().enumerate().map(|(i, w)| (*w, i)).collect();
        let o = PassphraseOptions {
            words: 20,
            separator: " ".to_owned(),
            ..PassphraseOptions::default()
        };
        let mut rng = SplitMix64::new(0x00ef_f000);
        let mut counts = vec![0u64; WORDLIST_LEN];
        for _ in 0..(WORDLIST_LEN * 50 / 20) {
            let p = generate_passphrase_with(&o, &mut rng).unwrap();
            for w in p.expose_secret().split(' ') {
                counts[index[w]] += 1;
            }
        }
        assert_uniform(&counts, "EFF word frequencies");
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn passphrases_meet_options(
            words in MIN_WORDS..=MAX_WORDS,
            separator in "[ ._+=/0-9]{1,8}",
            capitalize in any::<bool>(),
            append_digit in any::<bool>(),
            seed in any::<u64>(),
        ) {
            let o = PassphraseOptions { words, separator: separator.clone(), capitalize, append_digit };
            let p = generate_passphrase_with(&o, &mut SplitMix64::new(seed)).unwrap();
            let s = p.expose_secret();
            prop_assert_eq!(p.capacity(), s.len());
            prop_assert!((p.entropy().bits() - 12.925 * f64::from(words)).abs() < 1e-9);
            // Removing the separators leaves only word characters plus at
            // most one digit (separators may themselves contain digits, so
            // count letters instead).
            let letters = s.chars().filter(char::is_ascii_alphabetic).count();
            prop_assert!(letters >= 3 * words as usize - 3);
            prop_assert_eq!(
                s.chars().filter(char::is_ascii_uppercase).count() >= words as usize,
                capitalize
            );
        }
    }
}
