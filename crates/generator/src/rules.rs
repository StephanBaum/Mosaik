//! Apple's `passwordrules` language: parsing, canonical display,
//! validation, and generation of passwords that satisfy a rule.
//!
//! The parser follows Apple's reference implementation
//! (`tools/PasswordRulesParser/PasswordRulesParser.js` in
//! apple/password-manager-resources, MIT), including its leniency, so the
//! vendored quirks data and `passwordrules` attributes mean the same here
//! as in Safari:
//!
//! - Properties are `name: value` separated by `;`. Names are
//!   case-sensitive: `required`, `allowed`, `max-consecutive`,
//!   `minlength`, `maxlength`, plus the reference parser's
//!   `max-repeating` and `max-sequential`.
//! - `required`/`allowed` take a `,`-separated list of the named classes
//!   `upper`, `lower`, `digit`, `special`, `ascii-printable`, `unicode`
//!   (case-insensitive) and custom classes `[...]`. In a custom class `-`
//!   is literal only as the first character (elsewhere it is ignored), a
//!   trailing `]]` is a literal `]`, and characters outside printable ASCII
//!   are ignored.
//! - Each `required` property means "at least one character from this
//!   set". `allowed` is the union of every `allowed` and `required` set;
//!   without any it is `ascii-printable`.
//! - Repeated limits combine to the strictest: the largest `minlength`, the
//!   smallest `maxlength` and run limits. Zero values are ignored.
//! - Malformed input degrades as in the reference: a property with an
//!   unusable value is dropped, and a stray token after a property discards
//!   the whole rule set (leaving the default `allowed: ascii-printable`).
//!   Parsing never fails.
//! - Repeated identical `required` sets count once.
//!
//! Unlike the reference, input size is bounded, because a hostile page
//! supplies the `passwordrules` attribute: a rule string longer than
//! [`MAX_RULES_LEN`] characters, or one with more than
//! [`MAX_REQUIRED_CLASSES`] distinct `required` sets, is treated as
//! malformed (no rules). Real rules are far smaller (the longest vendored
//! one has 204 characters, and none has more than 8 `required` sets). The
//! bounds keep parsing, checking and generation linear in the input, and
//! they are chosen so the canonical display of any accepted rule set is
//! itself accepted.
//!
//! The reference parser does not define `max-repeating` and
//! `max-sequential`. This crate enforces conservative readings that imply
//! any plausible weaker one: `max-repeating: n` allows no character more
//! than `n` times in the whole password, and `max-sequential: n` allows no
//! run of more than `n` characters with ascending or descending code points
//! (`abc`, `321`).

use core::fmt;

use crate::entropy::Entropy;
use crate::error::{Error, Result};
use zeroize::Zeroizing;

use crate::password::{DEFAULT_LENGTH, DIGITS, LOWER, MAX_LENGTH, MIN_LENGTH, UPPER};
use crate::policy::{
    AsciiMask, MIN_ACCEPTANCE, Overlapping, Policy, Requirement, Runs, RunsAcceptance, SPACE_BIT,
    acceptance_repeating, mask_contains, mask_len, runs_ok,
};
use crate::rng::{OsRandom, RandomSource};
use crate::secret::GeneratedPassword;

/// Longest `passwordrules` string [`PasswordRules::parse`] reads, in
/// characters. Longer input is treated as malformed.
pub const MAX_RULES_LEN: usize = 4096;

/// Most distinct `required` sets [`PasswordRules::parse`] accepts. More are
/// treated as malformed. A canonical `required` property is at most 105
/// characters, so the display of an accepted rule set stays below
/// [`MAX_RULES_LEN`].
pub const MAX_REQUIRED_CLASSES: usize = 32;

/// Printable ASCII, `' '..='~'`.
const PRINTABLE: AsciiMask = {
    let mut m = 0u128;
    let mut c = 0x20u32;
    while c <= 0x7E {
        m |= 1 << c;
        c += 1;
    }
    m
};
/// Apple's `special`: every printable ASCII character that is not a letter
/// or digit, including the space.
const SPECIAL: AsciiMask = PRINTABLE & !(UPPER | LOWER | DIGITS);

/// A set of characters from a `required` or `allowed` property.
///
/// Either a subset of printable ASCII, or `unicode` (any character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterClass {
    ascii: AsciiMask,
    unicode: bool,
}

impl CharacterClass {
    const EMPTY: Self = Self {
        ascii: 0,
        unicode: false,
    };
    const ASCII_PRINTABLE: Self = Self {
        ascii: PRINTABLE,
        unicode: false,
    };
    const UNICODE: Self = Self {
        ascii: PRINTABLE,
        unicode: true,
    };

    fn union(self, other: Self) -> Self {
        if self.unicode || other.unicode {
            Self::UNICODE
        } else {
            Self {
                ascii: self.ascii | other.ascii,
                unicode: false,
            }
        }
    }

    fn is_empty(self) -> bool {
        !self.unicode && self.ascii == 0
    }

    /// Whether `c` belongs to the class.
    #[must_use]
    pub fn contains(&self, c: char) -> bool {
        self.unicode || (c.is_ascii() && mask_contains(self.ascii, u8::try_from(c).unwrap_or(0)))
    }

    /// Whether this is the `unicode` class (any character).
    #[must_use]
    pub fn is_unicode(&self) -> bool {
        self.unicode
    }

    /// The printable ASCII characters of the class (all of them for
    /// `unicode`).
    pub(crate) fn ascii(self) -> AsciiMask {
        self.ascii
    }
}

impl fmt::Display for CharacterClass {
    /// The canonical form: named classes for complete ranges, then one
    /// custom class for the rest with `-` first and `]` last, so the output
    /// parses back to the same set.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unicode {
            return f.write_str("unicode");
        }
        if self.ascii == PRINTABLE {
            return f.write_str("ascii-printable");
        }
        let mut rest = self.ascii;
        let mut parts: Vec<String> = Vec::new();
        for (name, mask) in [
            ("upper", UPPER),
            ("lower", LOWER),
            ("digit", DIGITS),
            ("special", SPECIAL),
        ] {
            if self.ascii & mask == mask {
                parts.push(name.to_owned());
                rest &= !mask;
            }
        }
        if rest != 0 {
            let mut custom = String::from("[");
            let mut push = |b: u8| custom.push(char::from(b));
            if mask_contains(rest, b'-') {
                push(b'-');
            }
            let ordered = [UPPER, LOWER, DIGITS, SPECIAL];
            for class in ordered {
                for b in 0x20u8..=0x7E {
                    if b != b'-' && b != b']' && mask_contains(rest & class, b) {
                        push(b);
                    }
                }
            }
            if mask_contains(rest, b']') {
                push(b']');
            }
            custom.push(']');
            parts.push(custom);
        }
        f.write_str(&parts.join(", "))
    }
}

/// A parsed, canonicalized `passwordrules` rule set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordRules {
    required: Vec<CharacterClass>,
    allowed: CharacterClass,
    max_consecutive: Option<u32>,
    max_repeating: Option<u32>,
    max_sequential: Option<u32>,
    min_length: Option<u32>,
    max_length: Option<u32>,
}

impl Default for PasswordRules {
    /// No constraints: `allowed: ascii-printable`.
    fn default() -> Self {
        Self {
            required: Vec::new(),
            allowed: CharacterClass::ASCII_PRINTABLE,
            max_consecutive: None,
            max_repeating: None,
            max_sequential: None,
            min_length: None,
            max_length: None,
        }
    }
}

impl PasswordRules {
    /// Parses a `passwordrules` string. Never fails: malformed input
    /// degrades exactly as in Apple's reference parser, and input beyond
    /// [`MAX_RULES_LEN`] characters or [`MAX_REQUIRED_CLASSES`] distinct
    /// `required` sets counts as malformed (see the module documentation).
    #[must_use]
    pub fn parse(input: &str) -> Self {
        if input.chars().nth(MAX_RULES_LEN).is_some() {
            return Self::default();
        }
        let chars: Vec<char> = input.chars().collect();
        let raw = parse_rules_internal(&chars).unwrap_or_default();
        let mut rules = Self {
            allowed: CharacterClass::EMPTY,
            ..Self::default()
        };
        for rule in raw {
            match rule {
                RawRule::Required(values) => {
                    let class = canonical_class(&values);
                    rules.allowed = rules.allowed.union(class);
                    // An empty required set constrains nothing; the
                    // reference keeps it as `required: ` with no value. A
                    // repeated set adds nothing either. The list stays at
                    // most MAX_REQUIRED_CLASSES long, so `contains` is cheap.
                    if !class.is_empty() && !rules.required.contains(&class) {
                        if rules.required.len() == MAX_REQUIRED_CLASSES {
                            return Self::default();
                        }
                        rules.required.push(class);
                    }
                }
                RawRule::Allowed(values) => {
                    rules.allowed = rules.allowed.union(canonical_class(&values));
                }
                RawRule::MaxConsecutive(n) => {
                    rules.max_consecutive = Some(min_of(rules.max_consecutive, n));
                }
                RawRule::MaxRepeating(n) => {
                    rules.max_repeating = Some(min_of(rules.max_repeating, n));
                }
                RawRule::MaxSequential(n) => {
                    rules.max_sequential = Some(min_of(rules.max_sequential, n));
                }
                RawRule::MinLength(n) => {
                    rules.min_length = Some(rules.min_length.map_or(n, |m| m.max(n)));
                }
                RawRule::MaxLength(n) => rules.max_length = Some(min_of(rules.max_length, n)),
            }
        }
        if rules.allowed.is_empty() {
            rules.allowed = CharacterClass::ASCII_PRINTABLE;
        }
        rules
    }

    /// The distinct `required` sets, in input order (at most
    /// [`MAX_REQUIRED_CLASSES`]).
    #[must_use]
    pub fn required(&self) -> &[CharacterClass] {
        &self.required
    }

    /// The allowed characters (the union of `allowed` and `required`).
    #[must_use]
    pub fn allowed(&self) -> CharacterClass {
        self.allowed
    }

    /// `minlength`, if any.
    #[must_use]
    pub fn min_length(&self) -> Option<u32> {
        self.min_length
    }

    /// `maxlength`, if any.
    #[must_use]
    pub fn max_length(&self) -> Option<u32> {
        self.max_length
    }

    /// `max-consecutive`: the longest allowed run of one repeated
    /// character.
    #[must_use]
    pub fn max_consecutive(&self) -> Option<u32> {
        self.max_consecutive
    }

    /// `max-repeating` (see the module documentation for its meaning here).
    #[must_use]
    pub fn max_repeating(&self) -> Option<u32> {
        self.max_repeating
    }

    /// `max-sequential` (see the module documentation for its meaning here).
    #[must_use]
    pub fn max_sequential(&self) -> Option<u32> {
        self.max_sequential
    }

    fn runs(&self) -> Runs {
        Runs {
            max_consecutive: self.max_consecutive,
            max_repeating: self.max_repeating,
            max_sequential: self.max_sequential,
        }
    }

    /// Whether `password` satisfies every rule (for checking existing or
    /// user-chosen passwords).
    ///
    /// Streams over `password` without copying it: the only derived state
    /// is the set of ASCII characters it contains and the run counters, all
    /// in zeroizing storage (see `runs_ok`). Linear in the password and the
    /// number of `required` sets.
    #[must_use]
    pub fn is_satisfied_by(&self, password: &str) -> bool {
        let mut len = 0u32;
        let mut present: Zeroizing<AsciiMask> = Zeroizing::new(0);
        for c in password.chars() {
            if !self.allowed.contains(c) {
                return false;
            }
            len = len.saturating_add(1);
            if let Ok(b) = u8::try_from(c)
                && b.is_ascii()
            {
                *present |= 1u128 << b;
            }
        }
        self.min_length.is_none_or(|m| len >= m)
            && self.max_length.is_none_or(|m| len <= m)
            && self.required.iter().all(|r| {
                // `unicode` matches any character; other sets only ASCII.
                if r.is_unicode() {
                    len > 0
                } else {
                    r.ascii() & *present != 0
                }
            })
            && runs_ok(&self.runs(), password.chars().map(u32::from))
    }

    /// The entropy [`PasswordRules::generate`] would report for
    /// `preferred_length`, without generating anything.
    ///
    /// # Errors
    /// As for [`PasswordRules::generate`], except randomness errors.
    pub fn entropy(&self, preferred_length: u32) -> Result<Entropy> {
        Ok(self.policy(preferred_length)?.entropy())
    }

    /// Generates a password that satisfies these rules, from the OS CSPRNG.
    ///
    /// Generation alphabet: the allowed set, limited to printable ASCII
    /// (`unicode` generates from printable ASCII, which it includes), and
    /// without the space unless a `required` set needs it.
    ///
    /// Length: `preferred_length` **clamped** to the allowed range, which
    /// is `minlength..=maxlength`, at most [`MAX_LENGTH`], at most
    /// `max-repeating × |alphabet|`, and at least [`MIN_LENGTH`] unless the
    /// site's `maxlength` is shorter (so `0` asks for the shortest
    /// reasonable password, and a site with `maxlength: 2` gets 2
    /// characters). If the rules are too unlikely to hold at that length
    /// (estimated acceptance below 1 in 1024), the shortest longer length
    /// that works is used, else the longest shorter one. The chosen length
    /// is the length of the result, and its [`Entropy`] reflects it, so a
    /// short site-imposed length shows up as a weak strength bucket.
    ///
    /// Candidates are uniform over the alphabet and are checked and retried
    /// as a whole, so the result is uniform over all passwords of that
    /// length that satisfy the rules. The entropy is
    /// `log2(|alphabet|) × length`, flagged as an upper bound when the rules
    /// can reject a candidate.
    ///
    /// # Errors
    /// - [`Error::RulesUnsatisfiable`] if the rules contradict each other
    ///   (no allowed length can satisfy them) or need more than
    ///   [`MAX_LENGTH`] characters.
    /// - [`Error::ConstraintsTooStrict`] if too few candidates would pass at
    ///   every allowed length.
    /// - [`Error::RetryLimit`] if no candidate passed. Each kind of
    ///   constraint is estimated exactly, but combinations are estimated
    ///   as if independent, so a few strongly correlated rule sets end here.
    /// - [`Error::Randomness`] if the OS CSPRNG fails.
    pub fn generate(&self, preferred_length: u32) -> Result<GeneratedPassword> {
        self.generate_with(preferred_length, &mut OsRandom::new())
    }

    pub(crate) fn generate_with<R: RandomSource + ?Sized>(
        &self,
        preferred_length: u32,
        rng: &mut R,
    ) -> Result<GeneratedPassword> {
        self.policy(preferred_length)?.generate(rng)
    }

    /// Generates with [`DEFAULT_LENGTH`] as the preferred length.
    ///
    /// # Errors
    /// As for [`PasswordRules::generate`].
    pub fn generate_default(&self) -> Result<GeneratedPassword> {
        self.generate(DEFAULT_LENGTH)
    }

    /// Builds the generation policy (see [`PasswordRules::generate`]).
    pub(crate) fn policy(&self, preferred_length: u32) -> Result<Policy> {
        let allowed = self.allowed.ascii() & PRINTABLE;
        let ascii_required: Vec<AsciiMask> = self
            .required
            .iter()
            .filter(|r| !r.is_unicode())
            .map(|r| r.ascii() & allowed)
            .collect();
        // Leave out the space (sites often trim it) unless that would make
        // the alphabet or a required set empty.
        let without_space = allowed & !SPACE_BIT;
        let alphabet =
            if without_space != 0 && ascii_required.iter().all(|&r| r & without_space != 0) {
                without_space
            } else {
                allowed
            };
        if alphabet == 0 {
            return Err(Error::RulesUnsatisfiable);
        }

        let mut sets: Vec<AsciiMask> = Vec::new();
        for r in ascii_required {
            let r = r & alphabet;
            if r == 0 {
                return Err(Error::RulesUnsatisfiable);
            }
            if r != alphabet && !sets.contains(&r) {
                sets.push(r);
            }
        }
        // A set that contains another set is implied by it.
        let pruned: Vec<AsciiMask> = sets
            .iter()
            .copied()
            .filter(|&s| !sets.iter().any(|&t| t != s && t & s == t))
            .collect();

        let size = mask_len(alphabet);
        let site_lo = self.min_length.unwrap_or(1).max(1);
        let mut hi = self.max_length.unwrap_or(MAX_LENGTH).min(MAX_LENGTH);
        // No character may occur more than k times: at most k × |alphabet|
        // characters fit.
        if let Some(k) = self.max_repeating {
            hi = hi.min(k.saturating_mul(size));
        }
        if site_lo > hi {
            return Err(Error::RulesUnsatisfiable);
        }
        // Prefer at least MIN_LENGTH, as in random-character mode, unless
        // the site's maxlength is shorter.
        let lo = site_lo.max(MIN_LENGTH.min(hi));
        let runs = self.runs();
        let required = Overlapping::new(alphabet, &pruned).ok_or(Error::ConstraintsTooStrict)?;
        let mut p_runs =
            RunsAcceptance::new(alphabet, runs.max_consecutive, runs.max_sequential, hi);
        let p_repeating = runs
            .max_repeating
            .map(|k| acceptance_repeating(size, k, hi));
        let at = |p: &[f64], n: u32| {
            usize::try_from(n)
                .ok()
                .and_then(|i| p.get(i))
                .copied()
                .unwrap_or(0.0)
        };
        // The estimates are exact for each kind of constraint; their product
        // is an approximation (see `crate::policy`).
        let mut possible = false;
        let mut likely_enough = |n: u32| {
            let p = required.probability(n)
                * p_runs.probability(n)
                * p_repeating.as_deref().map_or(1.0, |p| at(p, n));
            possible |= p > 0.0;
            p >= MIN_ACCEPTANCE
        };
        // The preferred length if it works, else the shortest longer one,
        // else the longest shorter one down to MIN_LENGTH, else down to the
        // site's own minimum.
        let start = preferred_length.clamp(lo, hi);
        let length = (start..=hi)
            .chain((lo..start).rev())
            .chain((site_lo..lo).rev())
            .find(|&n| likely_enough(n));
        let Some(length) = length else {
            // Zero means the rules exclude every length: a contradiction,
            // not just an unlikely combination.
            return Err(if possible {
                Error::ConstraintsTooStrict
            } else {
                Error::RulesUnsatisfiable
            });
        };
        let requirements = pruned
            .into_iter()
            .map(|members| Requirement { members, min: 1 })
            .collect();
        Ok(Policy::new(alphabet, length, requirements, runs))
    }
}

fn min_of(current: Option<u32>, n: u32) -> u32 {
    current.map_or(n, |m| m.min(n))
}

impl fmt::Display for PasswordRules {
    /// The canonical form in the reference parser's order: `required`
    /// properties, `allowed`, `max-consecutive`, `max-repeating`,
    /// `max-sequential`, `minlength`, `maxlength`. It parses back to an
    /// equal value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = self
            .required
            .iter()
            .map(|r| format!("required: {r}"))
            .collect();
        parts.push(format!("allowed: {}", self.allowed));
        for (name, value) in [
            ("max-consecutive", self.max_consecutive),
            ("max-repeating", self.max_repeating),
            ("max-sequential", self.max_sequential),
            ("minlength", self.min_length),
            ("maxlength", self.max_length),
        ] {
            if let Some(v) = value {
                parts.push(format!("{name}: {v}"));
            }
        }
        f.write_str(&parts.join("; "))
    }
}

// ---------------------------------------------------------------------------
// Parser: a direct port of the reference parser's functions. Positions are
// indices into the input's chars. Every function returns the position where
// it stopped, like the reference's `[value, position]` pairs.
// ---------------------------------------------------------------------------

/// One parsed class token before canonicalization.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawClass {
    Named(Named),
    Custom(Vec<char>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Named {
    Upper,
    Lower,
    Digit,
    Special,
    AsciiPrintable,
    Unicode,
}

/// A property with a usable value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawRule {
    Required(Vec<RawClass>),
    Allowed(Vec<RawClass>),
    MaxConsecutive(u32),
    MaxRepeating(u32),
    MaxSequential(u32),
    MinLength(u32),
    MaxLength(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleName {
    Required,
    Allowed,
    MaxConsecutive,
    MaxRepeating,
    MaxSequential,
    MinLength,
    MaxLength,
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '-'
}

fn is_printable(c: char) -> bool {
    (' '..='~').contains(&c)
}

fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\x0c' | '\n' | '\r' | '\t')
}

fn at(input: &[char], pos: usize) -> Option<char> {
    input.get(pos).copied()
}

fn skip_whitespace(input: &[char], mut pos: usize) -> usize {
    while at(input, pos).is_some_and(is_whitespace) {
        pos += 1;
    }
    pos
}

fn parse_identifier(input: &[char], mut pos: usize) -> (String, usize) {
    let mut ident = String::new();
    while let Some(c) = at(input, pos).filter(|&c| is_identifier_char(c)) {
        ident.push(c);
        pos += 1;
    }
    (ident, pos)
}

fn named_class(ident: &str) -> Option<Named> {
    match ident.to_ascii_lowercase().as_str() {
        "upper" => Some(Named::Upper),
        "lower" => Some(Named::Lower),
        "digit" => Some(Named::Digit),
        "special" => Some(Named::Special),
        "ascii-printable" => Some(Named::AsciiPrintable),
        "unicode" => Some(Named::Unicode),
        _ => None,
    }
}

/// `_parseCustomCharacterClass`. `pos` is at `[`. `None` for an unclosed
/// class.
fn parse_custom_class(input: &[char], pos: usize) -> (Option<Vec<char>>, usize) {
    let len = input.len();
    let mut pos = pos + 1;
    if pos >= len {
        return (None, pos);
    }
    let initial = pos;
    let mut result = Vec::new();
    while let Some(c) = at(input, pos) {
        if !is_printable(c) || (c == '-' && pos > initial) {
            pos += 1;
            continue;
        }
        result.push(c);
        pos += 1;
        if c == ']' {
            break;
        }
    }
    let next = at(input, pos);
    let over_consumed = next.is_some_and(|c| c != ']')
        || (next.is_none() && pos.checked_sub(1).and_then(|p| at(input, p)) == Some(']'));
    if over_consumed {
        // The `]` pushed last was the closing bracket.
        result.pop();
        return (Some(result), pos);
    }
    if next == Some(']') {
        // `]]`: the first `]` was literal.
        return (Some(result), pos + 1);
    }
    (None, pos)
}

/// `_parsePasswordRequiredOrAllowedPropertyValue`. `pos < input.len()`.
fn parse_class_list(input: &[char], mut pos: usize) -> (Option<Vec<RawClass>>, usize) {
    let len = input.len();
    let mut values = Vec::new();
    loop {
        match at(input, pos) {
            Some(c) if is_identifier_char(c) => {
                let start = pos;
                let (ident, next) = parse_identifier(input, pos);
                pos = next;
                let Some(named) = named_class(&ident) else {
                    return (None, start);
                };
                values.push(RawClass::Named(named));
            }
            Some('[') => {
                let (custom, next) = parse_custom_class(input, pos);
                pos = next;
                if let Some(chars) = custom.filter(|c| !c.is_empty()) {
                    values.push(RawClass::Custom(chars));
                }
            }
            _ => return (None, pos),
        }
        pos = skip_whitespace(input, pos);
        match at(input, pos) {
            None | Some(';') => break,
            Some(',') => {
                pos = skip_whitespace(input, pos + 1);
                if pos >= len {
                    return (None, pos);
                }
            }
            Some(_) => return (None, pos),
        }
    }
    (Some(values), pos)
}

/// `_parseInteger`. `pos < input.len()`. Saturates at `u32::MAX` (the
/// reference uses unbounded JavaScript numbers).
fn parse_integer(input: &[char], mut pos: usize) -> (Option<u32>, usize) {
    if !at(input, pos).is_some_and(|c| c.is_ascii_digit()) {
        return (None, pos);
    }
    let mut value = 0u32;
    while let Some(d) = at(input, pos).and_then(|c| c.to_digit(10)) {
        value = value.saturating_mul(10).saturating_add(d);
        pos += 1;
    }
    match at(input, pos) {
        None | Some(';') => (Some(value), pos),
        Some(_) => (None, pos),
    }
}

/// The outcome of [`parse_rule`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedRule {
    /// Unrecognized name or missing `:` (the reference's `null` rule).
    NotARule,
    /// A known property with an empty or unusable value (`value: null`).
    NoValue,
    /// A property with a usable value.
    Rule(RawRule),
}

impl ParsedRule {
    fn from_value(rule: Option<RawRule>) -> Self {
        rule.map_or(Self::NoValue, Self::Rule)
    }
}

/// `_parsePasswordRule`. `pos` is at an identifier character.
fn parse_rule(input: &[char], pos: usize) -> (ParsedRule, usize) {
    let start = pos;
    let (ident, pos) = parse_identifier(input, pos);
    let name = match ident.as_str() {
        "required" => RuleName::Required,
        "allowed" => RuleName::Allowed,
        "max-consecutive" => RuleName::MaxConsecutive,
        "max-repeating" => RuleName::MaxRepeating,
        "max-sequential" => RuleName::MaxSequential,
        "minlength" => RuleName::MinLength,
        "maxlength" => RuleName::MaxLength,
        _ => return (ParsedRule::NotARule, start),
    };
    if at(input, pos) != Some(':') {
        return (ParsedRule::NotARule, pos);
    }
    let pos = skip_whitespace(input, pos + 1);
    if matches!(at(input, pos), None | Some(';')) {
        return (ParsedRule::NoValue, pos);
    }
    match name {
        RuleName::Required | RuleName::Allowed => {
            let (values, pos) = parse_class_list(input, pos);
            let rule = values.map(|v| {
                if name == RuleName::Required {
                    RawRule::Required(v)
                } else {
                    RawRule::Allowed(v)
                }
            });
            (ParsedRule::from_value(rule), pos)
        }
        _ => {
            let (value, pos) = parse_integer(input, pos);
            // The reference drops 0 (falsy in JavaScript).
            let rule = value.filter(|&n| n != 0).map(|n| match name {
                RuleName::MaxConsecutive => RawRule::MaxConsecutive(n),
                RuleName::MaxRepeating => RawRule::MaxRepeating(n),
                RuleName::MaxSequential => RawRule::MaxSequential(n),
                RuleName::MinLength => RawRule::MinLength(n),
                _ => RawRule::MaxLength(n),
            });
            (ParsedRule::from_value(rule), pos)
        }
    }
}

/// `_parsePasswordRulesInternal`. `None` means "malformed": the caller
/// then uses no rules at all.
fn parse_rules_internal(input: &[char]) -> Option<Vec<RawRule>> {
    let len = input.len();
    let mut rules = Vec::new();
    let mut pos = skip_whitespace(input, 0);
    while pos < len {
        if !at(input, pos).is_some_and(is_identifier_char) {
            // The reference returns what it has so far here.
            return Some(rules);
        }
        let (rule, next) = parse_rule(input, pos);
        if let ParsedRule::Rule(rule) = rule {
            rules.push(rule);
        }
        pos = skip_whitespace(input, next);
        match at(input, pos) {
            None => break,
            Some(';') => {
                pos = skip_whitespace(input, pos + 1);
                if pos >= len {
                    return Some(rules);
                }
            }
            Some(_) => return None,
        }
    }
    Some(rules)
}

/// `_canonicalizedPropertyValues`, as a set: `unicode` wins, then
/// `ascii-printable`, otherwise the union of the classes.
fn canonical_class(values: &[RawClass]) -> CharacterClass {
    let mut ascii = 0u128;
    for value in values {
        match value {
            RawClass::Named(Named::Unicode) => return CharacterClass::UNICODE,
            RawClass::Named(Named::AsciiPrintable) => ascii |= PRINTABLE,
            RawClass::Named(Named::Upper) => ascii |= UPPER,
            RawClass::Named(Named::Lower) => ascii |= LOWER,
            RawClass::Named(Named::Digit) => ascii |= DIGITS,
            RawClass::Named(Named::Special) => ascii |= SPECIAL,
            RawClass::Custom(chars) => {
                for &c in chars {
                    if let Ok(b) = u8::try_from(c) {
                        ascii |= 1u128 << b;
                    }
                }
            }
        }
    }
    CharacterClass {
        ascii: ascii & PRINTABLE,
        unicode: false,
    }
}

#[cfg(test)]
mod tests;
