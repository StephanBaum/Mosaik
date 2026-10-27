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
///
/// `#[non_exhaustive]`: start from [`PassphraseOptions::default`] and set
/// the fields you need, so a later option is not a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PassphraseOptions {
    /// Number of words, [`MIN_WORDS`] to [`MAX_WORDS`].
    pub words: u32,
    /// Placed between words. At most [`MAX_SEPARATOR_CHARS`] printable
    /// ASCII characters (`' '..='~'`); may be empty. Printable ASCII only,
    /// like the symbol set of random-character passwords: no invisible or
    /// bidirectional-control characters (`U+200B`, `U+202E`), no combining
    /// marks, nothing that displays differently from what is typed.
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
        let printable_ascii = self.separator.bytes().all(|b| (b' '..=b'~').contains(&b));
        if self.separator.len() > MAX_SEPARATOR_CHARS as usize || !printable_ascii {
            return Err(Error::InvalidSeparator {
                max: MAX_SEPARATOR_CHARS,
            });
        }
        Ok(())
    }

    /// Whether different word sequences can produce the same text, so that
    /// the words cannot be read back from the passphrase.
    ///
    /// Words are lowercase ASCII letters and `-`; an appended digit never
    /// hides a boundary, because words contain no digits. Conservatively:
    /// - With capitalization, every uppercase letter starts a word, so the
    ///   boundaries are visible unless the separator has uppercase letters.
    /// - Without it, an empty separator, or one with a lowercase letter or
    ///   `-`, may hide boundaries. The exception is exactly `-` (the
    ///   default): no hyphenated word in the list begins with a word
    ///   followed by `-` (`drop-down`, `felt-tip`, `t-shirt`, `yo-yo`; tested
    ///   below), so the list is a prefix code at the `-`-separated level and
    ///   decodes uniquely.
    fn boundaries_ambiguous(&self) -> bool {
        let sep = self.separator.as_bytes();
        if self.capitalize {
            sep.iter().any(u8::is_ascii_uppercase)
        } else {
            sep.is_empty()
                || (sep != b"-" && sep.iter().any(|&b| b.is_ascii_lowercase() || b == b'-'))
        }
    }

    /// `12.925 × words` (CRYPTO_SPEC §13). Capitalization is deterministic
    /// and adds nothing; the appended digit is not counted, which keeps the
    /// estimate conservative. When the separator can hide word boundaries
    /// (an empty separator, or one made of word characters; see
    /// `boundaries_ambiguous`), different word sequences can produce the
    /// same text, so the value is then an upper bound.
    ///
    /// # Errors
    /// The validation errors of [`generate_passphrase`].
    pub fn entropy(&self) -> Result<Entropy> {
        self.validate()?;
        Ok(Entropy::new(
            BITS_PER_WORD * f64::from(self.words),
            self.boundaries_ambiguous(),
        ))
    }
}

/// Generates a passphrase from the OS CSPRNG: each word is drawn uniformly
/// and independently from the 7776-word EFF large wordlist.
///
/// # Errors
/// - [`Error::InvalidWordCount`] if the word count is out of range.
/// - [`Error::InvalidSeparator`] if the separator is too long or contains
///   anything other than printable ASCII.
/// - [`Error::Randomness`] if the OS CSPRNG fails.
/// - [`Error::Wordlist`] if the embedded wordlist is damaged.
pub fn generate_passphrase(options: &PassphraseOptions) -> Result<GeneratedPassword> {
    generate_passphrase_with(options, &mut OsRandom::new())
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

    /// The embedded list is byte-for-byte the file whose SHA-256
    /// `data/README.md` records (`.gitattributes` keeps LF endings on every
    /// checkout), so a substituted word cannot slip in.
    #[test]
    fn wordlist_matches_the_recorded_sha256() {
        use sha2::{Digest, Sha256};
        assert_eq!(
            hex(&Sha256::digest(WORDLIST_TEXT.as_bytes())),
            "addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        use core::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
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
            separator: " + ".to_owned(),
            capitalize: true,
            append_digit: true,
        };
        let mut rng = SplitMix64::new(11);
        for _ in 0..100 {
            let p = generate_passphrase_with(&o, &mut rng).unwrap();
            let s = p.expose_secret();
            let parts: Vec<&str> = s.split(" + ").collect();
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

    /// Separators made of word characters can hide word boundaries, so the
    /// entropy is then only an upper bound.
    #[test]
    fn separators_that_hide_boundaries_give_an_upper_bound() {
        let bound = |separator: &str, capitalize: bool| {
            PassphraseOptions {
                separator: separator.to_owned(),
                capitalize,
                ..PassphraseOptions::default()
            }
            .entropy()
            .unwrap()
            .is_upper_bound()
        };
        for sep in ["", "a", "s", "x-", "--", " - ", "ab", "1a"] {
            assert!(bound(sep, false), "{sep:?} without capitals");
        }
        for sep in ["-", " ", ".", "_", "1", "X", "+=+", "12345678"] {
            assert!(!bound(sep, false), "{sep:?} without capitals");
        }
        // Capitals mark every word start, unless the separator has capitals.
        for sep in ["", "a", "-", "s", "--", " "] {
            assert!(!bound(sep, true), "{sep:?} with capitals");
        }
        for sep in ["X", "aB", "Q-"] {
            assert!(bound(sep, true), "{sep:?} with capitals");
        }
    }

    /// Why `-` counts as an unambiguous separator: no hyphenated word
    /// starts with another word followed by `-`, so splitting a passphrase
    /// at `-` and regrouping the pieces into words has only one solution
    /// (a prefix code over the pieces).
    #[test]
    fn hyphenated_words_do_not_start_with_a_word() {
        let list = wordlist().unwrap();
        let words: HashSet<&str> = list.iter().copied().collect();
        let hyphenated: Vec<&str> = list.iter().copied().filter(|w| w.contains('-')).collect();
        assert_eq!(hyphenated, ["drop-down", "felt-tip", "t-shirt", "yo-yo"]);
        for w in hyphenated {
            for (i, _) in w.match_indices('-') {
                assert!(!words.contains(&w[..i]), "{w}: {} is a word", &w[..i]);
            }
        }
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
        // Too long, control characters, and anything outside printable
        // ASCII: invisible (U+200B), bidi override (U+202E, which makes the
        // passphrase display reversed), combining marks, other letters.
        for separator in [
            "123456789",
            "\n",
            "\t",
            "a\u{7f}",
            "\u{200b}",
            "\u{202e}",
            "\u{2066}",
            "\u{feff}",
            "e\u{301}",
            "\u{301}",
            "\u{a0}",
            "·",
            "ä",
        ] {
            let o = PassphraseOptions {
                separator: separator.to_owned(),
                ..PassphraseOptions::default()
            };
            assert_eq!(
                o.entropy().unwrap_err(),
                Error::InvalidSeparator { max: 8 },
                "{separator:?}"
            );
            assert_eq!(
                generate_passphrase_with(&o, &mut SplitMix64::new(1)).unwrap_err(),
                Error::InvalidSeparator { max: 8 },
                "{separator:?}"
            );
        }
        for separator in ["", " ", "~", "12345678", " !\"#$%&'", "()*+,./:"] {
            let o = PassphraseOptions {
                separator: separator.to_owned(),
                ..PassphraseOptions::default()
            };
            assert!(o.entropy().is_ok(), "{separator:?}");
        }
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
