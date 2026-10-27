//! Password strength estimate: the generator's entropy formula, lowered by
//! the patterns people actually use.
//!
//! # Method (a documented heuristic, in the spirit of zxcvbn)
//!
//! Mosaic does not depend on a zxcvbn crate: the Rust ports copy the
//! password into many ordinary `String`s (lowercased, de-leeted,
//! substrings), which would leave copies of every vault password on the heap
//! that nobody zeroizes, and they embed large frequency lists. This module
//! keeps every copy in a zeroizing buffer of exactly the right size and uses
//! a short hand-written word list ([`crate::words`]).
//!
//! 1. The first [`MAX_ANALYZED_CHARS`] characters are analyzed; the rest
//!    adds nothing (a conservative choice: a longer tail can only make the
//!    true strength higher).
//! 2. **Brute force**: each character costs `log2(pool)` bits, where `pool`
//!    is the sum of the character classes the password uses (26 lowercase,
//!    26 uppercase, 10 digits, 33 ASCII symbols and space, 100 for anything
//!    else). This is the generator's formula (CRYPTO_SPEC §13) and an upper
//!    bound.
//! 3. **Patterns** found in the password, each with a cost in bits:
//!    - a word from [`crate::words::COMMON_WORDS`] (also reversed, and
//!      after undoing common substitutions such as `@`→`a`, `0`→`o`,
//!      `1`→`i`/`l`): `log2(rank + 2)`, plus bits for its capitalization,
//!      one bit per substituted character and one bit if reversed;
//!    - an alphabetic or numeric sequence of three or more (`abc`, `9876`):
//!      `log2(start choices) + log2(length)`, plus one bit if descending;
//!    - a run along one row of a QWERTY or QWERTZ keyboard (`qwert`,
//!      `asdf`, `!"§$%`): `log2(94) + 1 + log2(length)`;
//!    - a block repeated two or more times (`aaaa`, `abcabc`): the block's
//!      own estimate plus `log2(repetitions)`;
//!    - a year from 1900 to 2099 (`log2(200)` bits) or a date (day, month
//!      and year in the usual orders, with or without one separator):
//!      `log2(31 × 12 × years)`, plus two bits for a separator.
//! 4. The estimate is the cheapest way to cover the whole password with
//!    brute-force characters and patterns (dynamic programming over the
//!    positions), so it is never above the brute-force bound.
//!
//! The result maps to the generator's [`Strength`] buckets. The dashboard
//! calls a password weak below [`Strength::Fair`] (60 bits), the same line
//! the generator draws.

use mosaic_generator::Strength;
use zeroize::Zeroizing;

use crate::words::COMMON_WORDS;

/// How many characters of a password are analyzed.
pub const MAX_ANALYZED_CHARS: usize = 128;

/// Shortest pattern (sequence, keyboard run) that counts.
const MIN_PATTERN_LEN: usize = 3;

/// Keys on a keyboard that produce printable ASCII, the start choices of a
/// keyboard run.
const KEYBOARD_KEYS: f64 = 94.0;

/// Keyboard rows (lowercase, unshifted and shifted), US QWERTY and German
/// QWERTZ.
const KEYBOARD_ROWS: &[&str] = &[
    "`1234567890-=",
    "~!@#$%^&*()_+",
    "qwertyuiop[]\\",
    "asdfghjkl;'",
    "zxcvbnm,./",
    "^1234567890ß´",
    "°!\"§$%&/()=?`",
    "qwertzuiopü+",
    "asdfghjklöä#",
    "<yxcvbnm,.-",
    "789",
    "456",
    "123",
];

/// An estimate of how hard a password is to guess.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrengthEstimate {
    bits: f64,
}

impl StrengthEstimate {
    /// The estimate in bits (never negative, never above the brute-force
    /// bound).
    #[must_use]
    pub fn bits(&self) -> f64 {
        self.bits
    }

    /// The generator's strength bucket for [`StrengthEstimate::bits`].
    #[must_use]
    pub fn strength(&self) -> Strength {
        Strength::from_bits(self.bits)
    }

    /// Whether the dashboard calls this weak: below [`Strength::Fair`].
    #[must_use]
    pub fn is_weak(&self) -> bool {
        self.strength() < Strength::Fair
    }
}

/// Estimates the strength of `password` (see the module documentation).
/// The empty password has 0 bits.
#[must_use]
pub fn estimate_strength(password: &str) -> StrengthEstimate {
    let n = password.chars().count().min(MAX_ANALYZED_CHARS);
    let mut original: Zeroizing<Vec<char>> = Zeroizing::new(Vec::with_capacity(n));
    original.extend(password.chars().take(n));
    let lower = mapped(&original, lowercase);
    let leet_i = mapped(&lower, |c| unleet(c, 'i'));
    let leet_l = mapped(&lower, |c| unleet(c, 'l'));
    let text = Text {
        original: &original,
        lower: &lower,
        leet: [&leet_i, &leet_l],
    };
    StrengthEstimate {
        bits: min_bits(&text, true),
    }
}

/// Upper bound of [`estimate_strength`]: `log2(pool) × length` over the
/// analyzed characters.
#[must_use]
pub fn brute_force_bits(password: &str) -> f64 {
    let n = password.chars().count().min(MAX_ANALYZED_CHARS);
    let mut chars: Zeroizing<Vec<char>> = Zeroizing::new(Vec::with_capacity(n));
    chars.extend(password.chars().take(n));
    per_char_bits(&chars) * count_f64(n)
}

/// `src` mapped character by character into a new zeroizing buffer of the
/// same length (no reallocation, so no stray copies).
fn mapped(src: &[char], f: impl Fn(char) -> char) -> Zeroizing<Vec<char>> {
    let mut out = Zeroizing::new(Vec::with_capacity(src.len()));
    out.extend(src.iter().copied().map(f));
    out
}

fn lowercase(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_lowercase();
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

/// Undoes a common substitution; `one` is what `1`, `!` and `|` stand for.
fn unleet(c: char, one: char) -> char {
    match c {
        '4' | '@' => 'a',
        '8' => 'b',
        '(' | '{' | '[' | '<' => 'c',
        '3' => 'e',
        '6' | '9' => 'g',
        '1' | '!' | '|' => one,
        '0' => 'o',
        '5' | '$' => 's',
        '7' | '+' => 't',
        '2' => 'z',
        '%' => 'x',
        _ => c,
    }
}

/// The views of one password (or of a block of it) that matching uses. All
/// slices have the same length.
struct Text<'a> {
    original: &'a [char],
    lower: &'a [char],
    leet: [&'a [char]; 2],
}

impl Text<'_> {
    fn len(&self) -> usize {
        self.original.len()
    }

    /// The same views restricted to `start..end`.
    fn slice(&self, start: usize, end: usize) -> Option<Text<'_>> {
        let [a, b] = self.leet;
        Some(Text {
            original: self.original.get(start..end)?,
            lower: self.lower.get(start..end)?,
            leet: [a.get(start..end)?, b.get(start..end)?],
        })
    }
}

/// A pattern covering `start..end` at a cost of `bits`.
struct Match {
    start: usize,
    end: usize,
    bits: f64,
}

/// The cheapest cover of `text` by brute-force characters and patterns.
fn min_bits(text: &Text<'_>, with_repeats: bool) -> f64 {
    let n = text.len();
    let per_char = per_char_bits(text.original);
    let mut matches = Vec::new();
    dictionary_matches(text, &mut matches);
    sequence_matches(text.lower, &mut matches);
    keyboard_matches(text.lower, &mut matches);
    date_matches(text.original, &mut matches);
    if with_repeats {
        repeat_matches(text, &mut matches);
    }

    let mut best: Vec<f64> = Vec::with_capacity(n.saturating_add(1));
    best.push(0.0);
    for end in 1..=n {
        let mut cost = best.last().copied().unwrap_or(0.0) + per_char;
        for m in matches.iter().filter(|m| m.end == end) {
            if let Some(before) = best.get(m.start) {
                cost = cost.min(before + m.bits);
            }
        }
        best.push(cost);
    }
    best.last().copied().unwrap_or(0.0).max(0.0)
}

fn count_f64(n: usize) -> f64 {
    // Lengths here are at most MAX_ANALYZED_CHARS, far below 2^52.
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// `log2(pool)` for the character classes `chars` uses; 0 for no
/// characters.
fn per_char_bits(chars: &[char]) -> f64 {
    let (mut lower, mut upper, mut digit, mut symbol, mut other) =
        (false, false, false, false, false);
    for &c in chars {
        if c.is_ascii_lowercase() {
            lower = true;
        } else if c.is_ascii_uppercase() {
            upper = true;
        } else if c.is_ascii_digit() {
            digit = true;
        } else if c.is_ascii_graphic() || c == ' ' {
            symbol = true;
        } else {
            other = true;
        }
    }
    let pool: u32 = [
        (lower, 26),
        (upper, 26),
        (digit, 10),
        (symbol, 33),
        (other, 100),
    ]
    .iter()
    .filter(|(used, _)| *used)
    .map(|(_, size)| size)
    .sum();
    if pool == 0 {
        0.0
    } else {
        f64::from(pool).log2()
    }
}

fn dictionary_matches(text: &Text<'_>, out: &mut Vec<Match>) {
    let n = text.len();
    let variants = [text.lower, text.leet[0], text.leet[1]];
    for (rank, word) in (0u32..).zip(COMMON_WORDS.iter()) {
        let len = word.chars().count();
        let base = f64::from(rank.saturating_add(2)).log2();
        for start in 0..n.saturating_sub(len.saturating_sub(1)) {
            let end = start + len;
            let (Some(original), Some(lower)) =
                (text.original.get(start..end), text.lower.get(start..end))
            else {
                continue;
            };
            let mut cheapest: Option<f64> = None;
            for variant in variants {
                let Some(span) = variant.get(start..end) else {
                    continue;
                };
                let substituted = span.iter().zip(lower).filter(|(a, b)| a != b).count();
                if span.iter().copied().eq(word.chars()) {
                    let bits = base + count_f64(substituted);
                    cheapest = Some(cheapest.map_or(bits, |c| c.min(bits)));
                }
                if span.iter().rev().copied().eq(word.chars()) {
                    let bits = base + count_f64(substituted) + 1.0;
                    cheapest = Some(cheapest.map_or(bits, |c| c.min(bits)));
                }
            }
            if let Some(bits) = cheapest {
                out.push(Match {
                    start,
                    end,
                    bits: bits + uppercase_bits(original),
                });
            }
        }
    }
}

/// Bits for how a word is capitalized (zxcvbn's rule): none for all
/// lowercase, one for first-letter, last-letter or all caps, otherwise the
/// number of ways to place that many capitals.
fn uppercase_bits(span: &[char]) -> f64 {
    let upper = span.iter().filter(|c| c.is_uppercase()).count();
    let lower = span.iter().filter(|c| c.is_lowercase()).count();
    if upper == 0 {
        return 0.0;
    }
    let first = span.first().is_some_and(|c| c.is_uppercase());
    let last = span.last().is_some_and(|c| c.is_uppercase());
    if lower == 0 || (upper == 1 && (first || last)) {
        return 1.0;
    }
    let total = upper + lower;
    let ways: f64 = (1..=upper.min(lower)).map(|k| choose(total, k)).sum();
    ways.log2()
}

fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * count_f64(n - i) / count_f64(i + 1))
}

fn sequence_matches(lower: &[char], out: &mut Vec<Match>) {
    let n = lower.len();
    for start in 0..n {
        let Some(&first) = lower.get(start) else {
            continue;
        };
        let Some(&second) = lower.get(start + 1) else {
            continue;
        };
        let Some(delta) = step(first, second) else {
            continue;
        };
        let mut end = start + 2;
        while let (Some(&prev), Some(&next)) = (lower.get(end - 1), lower.get(end)) {
            if step(prev, next) != Some(delta) {
                break;
            }
            end += 1;
        }
        let len = end - start;
        if len < MIN_PATTERN_LEN {
            continue;
        }
        let start_choices: f64 = if matches!(first, 'a' | 'z' | '0' | '1' | '9') {
            4.0
        } else if first.is_ascii_digit() {
            10.0
        } else {
            26.0
        };
        let descending = if delta < 0 { 1.0 } else { 0.0 };
        out.push(Match {
            start,
            end,
            bits: start_choices.log2() + count_f64(len).log2() + descending,
        });
    }
}

/// `+1` or `-1` if `b` follows `a` in the alphabet or among the digits.
fn step(a: char, b: char) -> Option<i8> {
    let same_class = (a.is_ascii_lowercase() && b.is_ascii_lowercase())
        || (a.is_ascii_digit() && b.is_ascii_digit());
    if !same_class {
        return None;
    }
    match u32::from(b).checked_sub(u32::from(a)) {
        Some(1) => Some(1),
        _ if u32::from(a).checked_sub(u32::from(b)) == Some(1) => Some(-1),
        _ => None,
    }
}

fn keyboard_matches(lower: &[char], out: &mut Vec<Match>) {
    // Public data, not secret: ordinary vectors are fine here.
    let rows: Vec<Vec<char>> = KEYBOARD_ROWS
        .iter()
        .flat_map(|row| [row.chars().collect(), row.chars().rev().collect()])
        .collect();
    for (start, first) in lower.iter().enumerate() {
        let rest = lower.get(start..).unwrap_or_default();
        let mut longest = 0;
        for keys in &rows {
            for (pos, _) in keys.iter().enumerate().filter(|(_, k)| *k == first) {
                let run = keys
                    .get(pos..)
                    .unwrap_or_default()
                    .iter()
                    .zip(rest)
                    .take_while(|(k, c)| k == c)
                    .count();
                longest = longest.max(run);
            }
        }
        if longest >= MIN_PATTERN_LEN {
            out.push(Match {
                start,
                end: start + longest,
                bits: KEYBOARD_KEYS.log2() + 1.0 + count_f64(longest).log2(),
            });
        }
    }
}

fn repeat_matches(text: &Text<'_>, out: &mut Vec<Match>) {
    let n = text.len();
    for start in 0..n {
        let Some(rest) = text.original.get(start..) else {
            continue;
        };
        let Some(block_len) = (1..=rest.len() / 2).find(|&l| rest.get(..l) == rest.get(l..2 * l))
        else {
            continue;
        };
        let block = rest.get(..block_len).unwrap_or_default();
        let repetitions = rest
            .chunks_exact(block_len)
            .take_while(|chunk| *chunk == block)
            .count();
        let Some(base) = text.slice(start, start + block_len) else {
            continue;
        };
        out.push(Match {
            start,
            end: start + block_len * repetitions,
            bits: min_bits(&base, false) + count_f64(repetitions).log2(),
        });
    }
}

/// Bits of a four-digit year in 1900..=2099.
const YEAR_BITS: f64 = 7.643_856_189_774_724; // log2(200)

fn date_matches(original: &[char], out: &mut Vec<Match>) {
    let n = original.len();
    for start in 0..n {
        for len in 4..=10 {
            let Some(span) = original.get(start..start + len) else {
                break;
            };
            if let Some(bits) = date_bits(span) {
                out.push(Match {
                    start,
                    end: start + len,
                    bits,
                });
            }
        }
    }
}

/// The cost of `span` if it is a year or a date.
fn date_bits(span: &[char]) -> Option<f64> {
    if span.iter().all(char::is_ascii_digit) {
        return match span.len() {
            4 => is_year(number(span)?).then_some(YEAR_BITS),
            6 => digit_date(span, 2),
            8 => digit_date(span, 4),
            _ => None,
        };
    }
    let separator = *span.iter().find(|c| !c.is_ascii_digit())?;
    if !matches!(separator, '.' | '-' | '/' | ' ' | '_') {
        return None;
    }
    let mut parts = span.split(|&ch| ch == separator);
    let (first, second, third) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || [first, second, third]
            .iter()
            .any(|p| !p.iter().all(char::is_ascii_digit))
    {
        return None;
    }
    let short = |p: &[char]| (1..=2).contains(&p.len());
    let bits = if short(first) && short(second) && matches!(third.len(), 2 | 4) {
        let year = year_choices(third)?;
        let (one, two) = (number(first)?, number(second)?);
        (valid_day_month(one, two) || valid_day_month(two, one)).then(|| date_cost(year))?
    } else if first.len() == 4 && short(second) && short(third) {
        let year = year_choices(first)?;
        valid_day_month(number(third)?, number(second)?).then(|| date_cost(year))?
    } else {
        return None;
    };
    Some(bits + 2.0)
}

/// Six or eight digits as day-month-year, month-day-year or
/// year-month-day, with a year of `year_len` digits.
fn digit_date(span: &[char], year_len: usize) -> Option<f64> {
    let tail = span.len().checked_sub(year_len)?;
    let (day_month, year) = (span.get(..tail)?, span.get(tail..)?);
    let (year_first, month_day) = (span.get(..year_len)?, span.get(year_len..)?);
    let pair = |p: &[char]| Some((number(p.get(..2)?)?, number(p.get(2..)?)?));
    let leading = year_choices(year).is_some()
        && pair(day_month)
            .is_some_and(|(one, two)| valid_day_month(one, two) || valid_day_month(two, one));
    let trailing = year_choices(year_first).is_some()
        && pair(month_day).is_some_and(|(month, day)| valid_day_month(day, month));
    (leading || trailing).then(|| date_cost(if year_len == 4 { 200.0 } else { 100.0 }))
}

fn number(digits: &[char]) -> Option<u32> {
    digits.iter().try_fold(0u32, |acc, c| {
        acc.checked_mul(10)?.checked_add(c.to_digit(10)?)
    })
}

fn is_year(y: u32) -> bool {
    (1900..=2099).contains(&y)
}

/// How many years a year field of this form stands for: 100 for two
/// digits, 200 for 1900..=2099; `None` otherwise.
fn year_choices(digits: &[char]) -> Option<f64> {
    match digits.len() {
        2 => number(digits).map(|_| 100.0),
        4 => is_year(number(digits)?).then_some(200.0),
        _ => None,
    }
}

fn valid_day_month(day: u32, month: u32) -> bool {
    (1..=31).contains(&day) && (1..=12).contains(&month)
}

fn date_cost(years: f64) -> f64 {
    (31.0 * 12.0 * years).log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(p: &str) -> f64 {
        estimate_strength(p).bits()
    }

    fn strength(p: &str) -> Strength {
        estimate_strength(p).strength()
    }

    #[test]
    fn empty_and_tiny() {
        assert_eq!(bits(""), 0.0);
        assert_eq!(brute_force_bits(""), 0.0);
        assert!(estimate_strength("").is_weak());
        assert!((bits("a") - 26f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn random_passwords_keep_their_brute_force_bits() {
        for p in [
            "x7#Qp9!vL2",
            "Kd8$mWq2#Lp9vRt4",
            "hT3%nB8@wQ6^zY1&",
            "Vw9!rT2#kL5$pM8&nQ4*",
        ] {
            let b = bits(p);
            assert!(b <= brute_force_bits(p) + 1e-9, "{p}");
            assert!(b > 0.85 * brute_force_bits(p), "{p}: {b}");
            assert!(!estimate_strength(p).is_weak(), "{p}: {b}");
        }
        assert_eq!(strength("Vw9!rT2#kL5$pM8&nQ4*"), Strength::VeryStrong);
    }

    #[test]
    fn common_patterns_are_weak() {
        for p in [
            "password",
            "Password1",
            "P@ssw0rd!",
            "passwort123",
            "Sommer2024!",
            "Winter2023",
            "qwertz123",
            "qwertyuiop",
            "1q2w3e4r",
            "123456789",
            "abcdefgh",
            "aaaaaaaaaaaa",
            "abcabcabcabc",
            "letmein!",
            "drowssap",
            "iloveyou2",
            "!\"§$%&/()",
            "01.01.1990",
            "19901231",
            "12.31.90",
            "1990-12-31",
            "Michael1985",
            "Hallo123!",
        ] {
            let e = estimate_strength(p);
            assert!(e.is_weak(), "{p}: {} bits", e.bits());
        }
        assert_eq!(strength("password"), Strength::VeryWeak);
        assert_eq!(strength("Sommer2024!"), Strength::VeryWeak);
    }

    #[test]
    fn generated_passphrases_are_not_weak() {
        // Words outside the common list count as brute-force letters.
        assert!(!estimate_strength("tidy-rumble-ascent-pliers-gnarly-velvet").is_weak());
        assert!(!estimate_strength("correct-horse-battery-staple").is_weak());
    }

    #[test]
    fn patterns_lower_the_estimate() {
        assert!(bits("password") < 5.0);
        assert!(bits("Password") < bits("PassWord"));
        assert!(bits("password") < bits("p@ssword"));
        assert!(bits("password") < bits("drowssap"));
        assert!(bits("abcdef") < bits("abcdeg"));
        assert!(bits("fedcba") > bits("abcdef"));
        assert!(bits("456789") < bits("4567890") + 1e-9);
        assert!(bits("asdfgh") < brute_force_bits("asdfgh") / 2.0);
        assert!(bits("Xk#9Xk#9Xk#9") < brute_force_bits("Xk#9Xk#9Xk#9") / 2.0);
        assert!(bits("2019") < brute_force_bits("2019"));
        assert!(bits("1899") > bits("1999"));
    }

    #[test]
    fn capitalization_bits() {
        assert_eq!(uppercase_bits(&['a', 'b']), 0.0);
        assert_eq!(uppercase_bits(&['A', 'b']), 1.0);
        assert_eq!(uppercase_bits(&['a', 'B']), 1.0);
        assert_eq!(uppercase_bits(&['A', 'B']), 1.0);
        // aBcD: two capitals among four letters: C(4,1) + C(4,2) = 10.
        assert!((uppercase_bits(&['a', 'B', 'c', 'D']) - 10f64.log2()).abs() < 1e-9);
        assert!((choose(5, 2) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn dates() {
        let c = |s: &str| date_bits(&s.chars().collect::<Vec<_>>());
        assert_eq!(c("1990"), Some(YEAR_BITS));
        assert_eq!(c("1899"), None);
        assert_eq!(c("2100"), None);
        assert!(c("311290").is_some());
        assert!(c("123190").is_some());
        assert!(c("901231").is_some());
        assert!(c("31121990").is_some());
        assert!(c("19901231").is_some());
        assert!(c("12311990").is_some());
        assert_eq!(c("99999999"), None);
        assert_eq!(c("12345"), None);
        assert_eq!(c("999999"), None);
        assert!(c("1.2.90").is_some());
        assert!(c("31/12/1990").is_some());
        assert!(c("1990-12-31").is_some());
        assert!(c("12_31_90").is_some());
        assert!(c("31 12 90").is_some());
        assert_eq!(c("1990-13-31"), None);
        assert_eq!(c("32.13.90"), None);
        assert_eq!(c("1.2.3"), None);
        assert_eq!(c("1.2.3.4"), None);
        assert_eq!(c("1.2-90"), None);
        assert_eq!(c("1:2:90"), None);
        assert_eq!(c("123.1.90"), None);
        assert_eq!(c("1.2.1800"), None);
        assert!(c("1.2.90").unwrap() > c("1290").unwrap_or(0.0));
    }

    #[test]
    fn sequences_and_keyboard_runs() {
        let mut m = Vec::new();
        sequence_matches(&['x', 'a', 'b', 'c', 'd'], &mut m);
        assert!(m.iter().any(|m| m.start == 1 && m.end == 5));
        let mut m = Vec::new();
        sequence_matches(&['a', 'b', '1', '2'], &mut m);
        assert!(m.is_empty());
        let mut m = Vec::new();
        keyboard_matches(&['p', 'o', 'i', 'u'], &mut m);
        assert!(m.iter().any(|m| m.start == 0 && m.end == 4));
        assert_eq!(step('a', 'b'), Some(1));
        assert_eq!(step('b', 'a'), Some(-1));
        assert_eq!(step('a', 'c'), None);
        assert_eq!(step('9', 'a'), None);
    }

    #[test]
    fn long_and_unicode_input() {
        let long = "Zq8#".repeat(100);
        let e = estimate_strength(&long);
        assert!(e.bits() <= brute_force_bits(&long));
        assert!(e.bits() < 60.0, "a repeated block stays weak");
        let unicode = "ÄöÜß€漢字パスワード🙂";
        assert!(bits(unicode) > 0.0);
        assert!(bits("İ") > 0.0);
        assert_eq!(lowercase('İ'), 'İ');
        assert_eq!(lowercase('Ä'), 'ä');
    }
}
