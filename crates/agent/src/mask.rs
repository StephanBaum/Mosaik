//! The masking engine (agent access design §8, ADR-0159).
//!
//! A [`Masker`] holds every form in which a secret value could come back in a
//! response (raw, base64, percent, hex, JSON escapes, base64 embedded in
//! larger data) and replaces each match with [`MASK_TEXT`]. A [`MaskStream`]
//! does the same over chunks with a hold-back of the longest pattern minus one
//! byte, so a value split across two chunks is still caught, and its output
//! equals the one-shot output for every way of splitting the input.
//!
//! What masking cannot catch (transformed, reversed or split values) is not
//! hidden: destination binding is the primary control (design §8.3).
//!
//! The patterns and the stream's carry (unmasked bytes) are secrets: they live
//! in zeroizing buffers, `Debug` prints `[REDACTED]`, and the types are not
//! `Clone`.

use core::cmp::Reverse;
use core::fmt;

use zeroize::{Zeroize, Zeroizing};

/// What a match is replaced with.
pub const MASK_TEXT: &str = "[masked by Mosaic]";

/// The shortest embedded base64 core that is kept: shorter runs would mask
/// ordinary text.
const MIN_CORE_CHARS: usize = 16;

const STD_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

struct Pattern {
    bytes: Zeroizing<Vec<u8>>,
    /// Matched ASCII-case-insensitively (the hex form only).
    case_insensitive: bool,
}

impl Pattern {
    fn matches(&self, rest: &[u8]) -> bool {
        rest.get(..self.bytes.len()).is_some_and(|window| {
            if self.case_insensitive {
                window.eq_ignore_ascii_case(&self.bytes)
            } else {
                window == self.bytes.as_slice()
            }
        })
    }
}

/// The set of patterns of one request or run. Not `Clone`; `Debug` is
/// redacted.
pub struct Masker {
    patterns: Vec<Pattern>,
    /// Pattern indexes by first byte, longest first.
    index: Vec<Vec<usize>>,
    longest: usize,
}

impl fmt::Debug for Masker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Masker {
    /// Builds the patterns of every non-empty value (see the module docs and
    /// design §8.1). Values of fewer than 12 bytes must not reach here
    /// ([`crate::binding::check_value`]): they would mask ordinary output.
    #[must_use]
    pub fn new(values: &[&[u8]]) -> Self {
        let mut masker = Self {
            patterns: Vec::new(),
            index: Vec::new(),
            longest: 0,
        };
        for value in values.iter().filter(|v| !v.is_empty()) {
            for (bytes, case_insensitive) in forms(value) {
                masker.push(bytes, case_insensitive);
            }
        }
        masker.rebuild();
        masker
    }

    /// Adds one exact pattern, for example the `base64(user ":" v)` of a Basic
    /// binding, which the caller builds. An empty pattern is dropped.
    pub fn add_exact(&mut self, pattern: &[u8]) {
        self.push(Zeroizing::new(pattern.to_vec()), false);
        self.rebuild();
    }

    fn push(&mut self, bytes: Zeroizing<Vec<u8>>, case_insensitive: bool) {
        if bytes.is_empty() {
            return;
        }
        let known = self
            .patterns
            .iter()
            .any(|p| p.case_insensitive == case_insensitive && p.bytes == bytes);
        if !known {
            self.patterns.push(Pattern {
                bytes,
                case_insensitive,
            });
        }
    }

    fn rebuild(&mut self) {
        let mut index: Vec<Vec<usize>> = vec![Vec::new(); 256];
        for (i, p) in self.patterns.iter().enumerate() {
            let Some(&first) = p.bytes.first() else {
                continue;
            };
            let mut firsts = vec![first];
            if p.case_insensitive {
                firsts.push(first.to_ascii_lowercase());
                firsts.push(first.to_ascii_uppercase());
                firsts.dedup();
            }
            for b in firsts {
                if let Some(bucket) = index.get_mut(usize::from(b))
                    && !bucket.contains(&i)
                {
                    bucket.push(i);
                }
            }
        }
        let patterns = &self.patterns;
        for bucket in &mut index {
            bucket.sort_by_key(|i| Reverse(patterns.get(*i).map_or(0, |p| p.bytes.len())));
        }
        self.longest = self
            .patterns
            .iter()
            .map(|p| p.bytes.len())
            .max()
            .unwrap_or(0);
        self.index = index;
    }

    /// The bytes a stream keeps back: the longest pattern minus one.
    #[must_use]
    pub fn holdback(&self) -> usize {
        self.longest.saturating_sub(1)
    }

    /// Masks `data`: the output and the number of replacements.
    #[must_use]
    pub fn mask(&self, data: &[u8]) -> (Vec<u8>, u32) {
        let mut out = Vec::with_capacity(data.len());
        let mut count = 0;
        self.scan(data, true, &mut out, &mut count);
        (out, count)
    }

    /// A stream that masks chunks as they arrive.
    #[must_use]
    pub fn stream(&self) -> MaskStream<'_> {
        MaskStream {
            masker: self,
            carry: Zeroizing::new(Vec::new()),
            count: 0,
        }
    }

    /// The length of the longest pattern that matches `data` at `i`.
    fn longest_at(&self, data: &[u8], i: usize) -> Option<usize> {
        let first = *data.get(i)?;
        let rest = data.get(i..)?;
        self.index
            .get(usize::from(first))?
            .iter()
            .filter_map(|pi| self.patterns.get(*pi))
            .find(|p| p.matches(rest))
            .map(|p| p.bytes.len())
    }

    /// Masks `data` from the start. With `finish` false it stops at the first
    /// position whose decision needs bytes that have not arrived (fewer than
    /// the longest pattern left). Returns how many bytes were consumed.
    fn scan(&self, data: &[u8], finish: bool, out: &mut Vec<u8>, count: &mut u32) -> usize {
        let mut i = 0;
        while i < data.len() && (finish || i.saturating_add(self.longest) <= data.len()) {
            if let Some(len) = self.longest_at(data, i) {
                out.extend_from_slice(MASK_TEXT.as_bytes());
                *count = count.saturating_add(1);
                i = i.saturating_add(len);
            } else {
                if let Some(b) = data.get(i) {
                    out.push(*b);
                }
                i = i.saturating_add(1);
            }
        }
        i
    }
}

/// Masks a sequence of chunks. The output of all `push` calls and the final
/// `finish` equals [`Masker::mask`] of the whole input, for any split.
pub struct MaskStream<'a> {
    masker: &'a Masker,
    /// Bytes that may be the start of a pattern: unmasked, so zeroized.
    carry: Zeroizing<Vec<u8>>,
    count: u32,
}

impl fmt::Debug for MaskStream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl MaskStream<'_> {
    /// Takes the next chunk and returns the output that is decided.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<u8> {
        let mut buf = Zeroizing::new(Vec::with_capacity(
            self.carry.len().saturating_add(chunk.len()),
        ));
        buf.extend_from_slice(&self.carry);
        buf.extend_from_slice(chunk);
        self.carry.zeroize();
        let mut out = Vec::with_capacity(buf.len());
        let consumed = self.masker.scan(&buf, false, &mut out, &mut self.count);
        if let Some(rest) = buf.get(consumed..) {
            self.carry.extend_from_slice(rest);
        }
        out
    }

    /// Replacements so far (without the final flush of [`MaskStream::finish`]).
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// Ends the stream and returns the rest of the output.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.finish_counted().0
    }

    /// Like [`MaskStream::finish`], with the total number of replacements.
    #[must_use]
    pub fn finish_counted(mut self) -> (Vec<u8>, u32) {
        let mut out = Vec::with_capacity(self.carry.len());
        let carry = core::mem::take(&mut *self.carry);
        let carry = Zeroizing::new(carry);
        self.masker.scan(&carry, true, &mut out, &mut self.count);
        (out, self.count)
    }
}

// ---------------------------------------------------------------------------
// The forms of one value
// ---------------------------------------------------------------------------

type Form = (Zeroizing<Vec<u8>>, bool);

/// Every pattern of `value` with whether it is matched case-insensitively.
fn forms(value: &[u8]) -> Vec<Form> {
    let mut out: Vec<Form> = vec![(Zeroizing::new(value.to_vec()), false)];
    for alphabet in [STD_ALPHABET, URL_ALPHABET] {
        for pad in [true, false] {
            out.push((base64(value, alphabet, pad), false));
        }
        for offset in 0..3 {
            let core = base64_core(value, alphabet, offset);
            if core.len() >= MIN_CORE_CHARS {
                out.push((core, false));
            }
        }
    }
    for upper in [true, false] {
        out.push((percent(value, upper, false), false));
    }
    out.push((percent(value, true, true), false));
    out.push((hex_lower(value), true));
    if let Ok(text) = core::str::from_utf8(value) {
        for slash in [false, true] {
            for unicode in [Unicode::Off, Unicode::Lower, Unicode::Upper] {
                out.push((json_escape(text, slash, unicode), false));
            }
        }
    }
    out
}

fn sextet(alphabet: &[u8; 64], n: u32) -> u8 {
    let idx = usize::try_from(n & 0x3f).unwrap_or(0);
    alphabet.get(idx).copied().unwrap_or(b'=')
}

/// Base64 of `data`, padded or not.
fn base64(data: &[u8], alphabet: &[u8; 64], pad: bool) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(data.len().div_ceil(3).saturating_mul(4)));
    for chunk in data.chunks(3) {
        let mut it = chunk.iter().copied();
        let b0 = it.next().unwrap_or(0);
        let b1 = it.next();
        let b2 = it.next();
        let n =
            (u32::from(b0) << 16) | (u32::from(b1.unwrap_or(0)) << 8) | u32::from(b2.unwrap_or(0));
        out.push(sextet(alphabet, n >> 18));
        out.push(sextet(alphabet, n >> 12));
        match (b1, b2) {
            (Some(_), Some(_)) => {
                out.push(sextet(alphabet, n >> 6));
                out.push(sextet(alphabet, n));
            }
            (Some(_), None) => {
                out.push(sextet(alphabet, n >> 6));
                if pad {
                    out.push(b'=');
                }
            }
            (None, _) => {
                if pad {
                    out.extend_from_slice(b"==");
                }
            }
        }
    }
    out
}

/// The stable core of the base64 of a larger buffer that holds `value` at
/// byte `offset`: the characters whose 6-bit groups lie entirely inside the
/// value's bytes, whatever surrounds them.
fn base64_core(value: &[u8], alphabet: &[u8; 64], offset: usize) -> Zeroizing<Vec<u8>> {
    let mut buf = Zeroizing::new(Vec::with_capacity(offset + value.len() + 2));
    buf.resize(offset, 0);
    buf.extend_from_slice(value);
    buf.extend_from_slice(&[0, 0]);
    let first = (8 * offset).div_ceil(6);
    let last = 8 * (offset + value.len()) / 6;
    let mut out = Zeroizing::new(Vec::with_capacity(last.saturating_sub(first)));
    for i in first..last {
        let bit = 6 * i;
        let hi = buf.get(bit / 8).copied().unwrap_or(0);
        let lo = buf.get(bit / 8 + 1).copied().unwrap_or(0);
        let word = (u32::from(hi) << 8) | u32::from(lo);
        let shift = 10 - (bit % 8);
        out.push(sextet(alphabet, word >> shift));
    }
    out
}

fn hex_digit(nibble: u8, upper: bool) -> u8 {
    let digits: &[u8; 16] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    digits
        .get(usize::from(nibble & 0x0f))
        .copied()
        .unwrap_or(b'0')
}

fn hex_lower(value: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(value.len().saturating_mul(2)));
    for b in value {
        out.push(hex_digit(b >> 4, false));
        out.push(hex_digit(*b, false));
    }
    out
}

/// Percent-encoding of every byte outside `A-Za-z0-9-._~`; with `form`, a
/// space is `+` (`application/x-www-form-urlencoded`).
fn percent(value: &[u8], upper: bool, form: bool) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(value.len()));
    for b in value {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(*b);
        } else if form && *b == b' ' {
            out.push(b'+');
        } else {
            out.push(b'%');
            out.push(hex_digit(b >> 4, upper));
            out.push(hex_digit(*b, upper));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unicode {
    /// Non-ASCII characters stay as UTF-8; control characters use `\u00xx`
    /// in lower case, as `serde_json` writes them.
    Off,
    /// Non-ASCII characters and controls as `\uXXXX` with lower-case hex.
    Lower,
    /// The same with upper-case hex.
    Upper,
}

/// A JSON string body (no quotes). `Off` and `slash` false is `serde_json`'s
/// escaping.
fn json_escape(text: &str, slash: bool, unicode: Unicode) -> Zeroizing<Vec<u8>> {
    let upper = unicode == Unicode::Upper;
    let mut out = Zeroizing::new(Vec::with_capacity(text.len()));
    let mut units = [0u16; 2];
    for c in text.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '/' if slash => out.extend_from_slice(b"\\/"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if u32::from(c) < 0x20 || (!c.is_ascii() && unicode != Unicode::Off) => {
                for unit in c.encode_utf16(&mut units) {
                    out.extend_from_slice(b"\\u");
                    for shift in [12u32, 8, 4, 0] {
                        out.push(hex_digit(
                            u8::try_from((*unit >> shift) & 0xf).unwrap_or(0),
                            upper,
                        ));
                    }
                }
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out
}
