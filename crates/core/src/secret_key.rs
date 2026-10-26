//! The Secret Key `SK` and its `M1-` display format (CRYPTO_SPEC §5.2).
//!
//! `SK = random(16)`. The display string is `M1-` followed by 28 Crockford
//! Base32 characters in 7 groups of 4, separated by `-`. They encode
//! `SK || c` with `c = SHA-256("mosaic/v1/secret-key-checksum" || SK)[0]`:
//! 17 bytes, zero-padded to 140 bits.
//!
//! Parsing is case-insensitive, ignores `-` and whitespace, and accepts the
//! Crockford aliases `O -> 0` and `I`/`L -> 1`. A wrong checksum yields
//! [`Error::SecretKeyMistyped`]; anything that is not the format at all
//! (wrong length, invalid character, missing prefix, non-zero pad bits)
//! yields [`Error::SecretKeyMalformed`]. The checksum only catches typos.

use sha2::{Digest, Sha256};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, ConstantTimeLess, CtOption};
use zeroize::Zeroize;

use crate::encoding::{CTX_PREFIX, label};
use crate::error::{Error, Result};
use crate::secret::{SecretBytes, SecretKey, SecretString};

/// Crockford Base32 alphabet (no I, L, O, U).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Display prefix (format version 1).
const PREFIX: &str = "M1";
/// Number of Base32 data characters.
const DATA_CHARS: usize = 28;
/// Characters per group.
const GROUP: usize = 4;
/// `M1-` + 28 characters + 6 separators.
const DISPLAY_LEN: usize = PREFIX.len() + 1 + DATA_CHARS + DATA_CHARS / GROUP - 1;
/// Bytes encoded: `SK || c`.
const PAYLOAD_LEN: usize = 17;

/// `c = SHA-256(ctx("secret-key-checksum") || SK)[0]`.
fn checksum(sk: &[u8; 16]) -> u8 {
    let mut digest = Sha256::new()
        .chain_update(CTX_PREFIX.as_bytes())
        .chain_update(label::SECRET_KEY_CHECKSUM.as_bytes())
        .chain_update(sk)
        .finalize();
    let c = digest.first().copied().unwrap_or(0);
    // The rest of the digest is a function of SK; do not leave it behind.
    digest.zeroize();
    c
}

/// The low byte of `x` (no branch, unlike a checked conversion).
const fn low_byte(x: u32) -> u8 {
    let [b, _, _, _] = x.to_le_bytes();
    b
}

/// Maps one input character to its 5-bit Crockford value, applying the
/// case-insensitivity and alias rules. Returns `None` for invalid input.
///
/// The characters of a Secret Key are secret, so the mapping neither
/// branches nor indexes on the character: case folding is a masked select,
/// and the value is selected by a constant-time comparison against all 32
/// alphabet entries and the aliases. Only the final validity flag becomes a
/// branch (in the caller), and invalid input is not secret.
fn decode_symbol(c: char) -> Option<u8> {
    let code = u32::from(c);
    let is_ascii = code.ct_lt(&0x80);
    let b = low_byte(code) & 0x7F;
    // ASCII upper-casing: clear bit 5 for 'a'..='z' only.
    let is_lower = !b.ct_lt(&b'a') & b.ct_lt(&(b'z' + 1));
    let b = u8::conditional_select(&b, &(b & !0x20), is_lower);

    let mut value = 0u8;
    let mut found = Choice::from(0);
    for (v, &a) in (0u8..).zip(ALPHABET.iter()) {
        let hit = b.ct_eq(&a);
        value.conditional_assign(&v, hit);
        found |= hit;
    }
    // Crockford aliases: O -> 0, I and L -> 1.
    let zero_alias = b.ct_eq(&b'O');
    let one_alias = b.ct_eq(&b'I') | b.ct_eq(&b'L');
    value.conditional_assign(&0, zero_alias);
    value.conditional_assign(&1, one_alias);
    found |= zero_alias | one_alias;
    CtOption::new(value, found & is_ascii).into()
}

/// The alphabet character of a 5-bit value, by range arithmetic instead of a
/// table lookup: `'0' + v`, plus 7 to jump from `9` to `A`, plus one for
/// each skipped letter (I, L, O, U) at or below the value.
fn symbol_char(value: u8) -> char {
    let v = value & 0x1F;
    let mut c = b'0' + v;
    for (from, step) in [(10u8, 7u8), (18, 1), (20, 1), (22, 1), (27, 1)] {
        c += u8::conditional_select(&0, &step, !v.ct_lt(&from));
    }
    char::from(c)
}

/// Normalizes `input` into its sequence of symbol values, skipping `-` and
/// whitespace, into `out`. Returns the number of symbols written.
fn normalize(input: &str, out: &mut [u8]) -> Result<usize> {
    let mut n = 0usize;
    for c in input.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let v = decode_symbol(c).ok_or(Error::SecretKeyMalformed)?;
        let slot = out.get_mut(n).ok_or(Error::SecretKeyMalformed)?;
        *slot = v;
        n = n.checked_add(1).ok_or(Error::SecretKeyMalformed)?;
    }
    Ok(n)
}

/// Encodes the 17-byte payload as 28 symbol values (MSB first, zero-padded).
fn encode_payload(payload: &[u8; PAYLOAD_LEN], out: &mut [u8; DATA_CHARS]) {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut slots = out.iter_mut();
    for &byte in payload {
        acc = (acc << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            if let Some(slot) = slots.next() {
                *slot = low_byte((acc >> bits) & 0x1F);
            }
        }
        acc &= (1 << bits) - 1;
    }
    if bits > 0
        && let Some(slot) = slots.next()
    {
        *slot = low_byte((acc << (5 - bits)) & 0x1F);
    }
    acc.zeroize();
}

/// Decodes 28 symbol values into the 17-byte payload, rejecting non-zero
/// pad bits.
fn decode_payload(symbols: &[u8], out: &mut [u8; PAYLOAD_LEN]) -> Result<()> {
    if symbols.len() != DATA_CHARS {
        return Err(Error::SecretKeyMalformed);
    }
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut slots = out.iter_mut();
    for &v in symbols {
        acc = (acc << 5) | u32::from(v & 0x1F);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            let slot = slots.next().ok_or(Error::SecretKeyMalformed)?;
            *slot = low_byte((acc >> bits) & 0xFF);
            acc &= (1 << bits) - 1;
        }
    }
    let pad_zero: bool = acc.ct_eq(&0).into();
    let pad_ok = bits == 4 && pad_zero && slots.next().is_none();
    acc.zeroize();
    if pad_ok {
        Ok(())
    } else {
        Err(Error::SecretKeyMalformed)
    }
}

impl SecretKey {
    /// A fresh Secret Key `SK = random(16)`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn generate() -> Result<Self> {
        Ok(Self(SecretBytes::random()?))
    }

    /// The 28 data symbol values of the display string.
    fn symbols(&self) -> SymbolBuf<DATA_CHARS> {
        let mut payload = [0u8; PAYLOAD_LEN];
        let (sk, c) = payload.split_at_mut(16);
        sk.copy_from_slice(self.0.expose_secret());
        if let Some(slot) = c.first_mut() {
            *slot = checksum(self.0.expose_secret());
        }
        let mut buf = SymbolBuf([0u8; DATA_CHARS]);
        encode_payload(&payload, &mut buf.0);
        payload.zeroize();
        buf
    }

    /// The `M1-XXXX-...-XXXX` display string (§5.2), in a zeroizing
    /// container.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the string cannot be allocated.
    pub fn to_display(&self) -> Result<SecretString> {
        let symbols = self.symbols();
        let mut out = SecretString::with_capacity(DISPLAY_LEN)?;
        for c in PREFIX.chars() {
            out.push(c);
        }
        for (i, &v) in symbols.0.iter().enumerate() {
            if i % GROUP == 0 {
                out.push('-');
            }
            out.push(symbol_char(v));
        }
        Ok(out)
    }

    /// The last group of 4 characters, which onboarding asks the user to
    /// type back after saving the Emergency Kit.
    ///
    /// # Errors
    /// [`Error::Allocation`] if the string cannot be allocated.
    pub fn last_group(&self) -> Result<SecretString> {
        let symbols = self.symbols();
        let mut out = SecretString::with_capacity(GROUP)?;
        for &v in symbols.0.iter().skip(DATA_CHARS - GROUP) {
            out.push(symbol_char(v));
        }
        Ok(out)
    }

    /// Whether `input` (with the same leniency as [`SecretKey::parse`])
    /// equals [`SecretKey::last_group`]. Compared in constant time.
    #[must_use]
    pub fn matches_last_group(&self, input: &str) -> bool {
        let expected = self.symbols();
        let mut got = SymbolBuf([0u8; DATA_CHARS]);
        let Ok(n) = normalize(input, &mut got.0) else {
            return false;
        };
        let (Some(got), Some(want)) = (got.0.get(..n), expected.0.get(DATA_CHARS - GROUP..)) else {
            return false;
        };
        n == GROUP && bool::from(got.ct_eq(want))
    }

    /// Parses a Secret Key display string (§5.2).
    ///
    /// # Errors
    /// [`Error::SecretKeyMalformed`] if `input` is not in the `M1-` format;
    /// [`Error::SecretKeyMistyped`] if it is, but the checksum is wrong.
    pub fn parse(input: &str) -> Result<Self> {
        let mut syms = SymbolBuf([0u8; DATA_CHARS + 2]);
        let n = normalize(input, &mut syms.0)?;
        let all = syms.0.get(..n).ok_or(Error::SecretKeyMalformed)?;
        let (prefix, data) = all
            .split_first_chunk::<2>()
            .ok_or(Error::SecretKeyMalformed)?;
        // The prefix is public; comparing it with ordinary branches is fine.
        let expected_prefix = PREFIX.chars().map(decode_symbol);
        if !prefix
            .iter()
            .zip(expected_prefix)
            .all(|(&got, want)| Some(got) == want)
        {
            return Err(Error::SecretKeyMalformed);
        }
        let mut payload = [0u8; PAYLOAD_LEN];
        let decoded = decode_payload(data, &mut payload);
        let result = decoded.and_then(|()| {
            let (sk, c) = payload
                .split_first_chunk::<16>()
                .ok_or(Error::SecretKeyMalformed)?;
            let ok = c
                .first()
                .is_some_and(|&c| bool::from(c.ct_eq(&checksum(sk))));
            if ok {
                Ok(Self(SecretBytes::from_slice(sk)?))
            } else {
                Err(Error::SecretKeyMistyped)
            }
        });
        payload.zeroize();
        result
    }
}

/// A stack buffer of symbol values that is zeroized on drop.
struct SymbolBuf<const N: usize>([u8; N]);

impl<const N: usize> Drop for SymbolBuf<N> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(bytes: [u8; 16]) -> SecretKey {
        SecretKey(SecretBytes::from_slice(&bytes).unwrap())
    }

    #[test]
    fn display_format() {
        let sk = key([0u8; 16]);
        let s = sk.to_display().unwrap();
        let s = s.expose_secret();
        assert_eq!(s.len(), DISPLAY_LEN);
        assert_eq!(s.len(), 37);
        assert!(s.starts_with("M1-"));
        let groups: Vec<&str> = s.split('-').skip(1).collect();
        assert_eq!(groups.len(), 7);
        assert!(groups.iter().all(|g| g.len() == 4));
        // 16 zero bytes -> 25 zero symbols plus the checksum and padding.
        assert!(s.starts_with("M1-0000-0000-0000-0000-0000-0000-0"));
    }

    #[test]
    fn known_encoding() {
        // SK = 00 01 .. 0f; the payload's bit layout is checked against an
        // independent bit-by-bit encoder.
        let bytes: [u8; 16] = core::array::from_fn(|i| u8::try_from(i).unwrap());
        let sk = key(bytes);
        let mut payload = bytes.to_vec();
        payload.push(checksum(&bytes));
        let mut bits: Vec<u8> = payload
            .iter()
            .flat_map(|b| (0..8).rev().map(move |i| (b >> i) & 1))
            .collect();
        bits.extend([0, 0, 0, 0]);
        let expected: String = bits
            .chunks(5)
            .map(|c| {
                let v = c.iter().fold(0u8, |a, &b| (a << 1) | b);
                char::from(ALPHABET[usize::from(v)])
            })
            .collect();
        let shown = sk.to_display().unwrap();
        assert_eq!(shown.expose_secret().replace('-', "")[2..], expected);
        assert_eq!(sk.last_group().unwrap().expose_secret(), &expected[24..]);
    }

    /// The display buffers are reserved at their final size, so building
    /// them never reallocates and leaves no unzeroized copy behind.
    #[test]
    fn display_buffers_are_never_reallocated() {
        for _ in 0..16 {
            let sk = SecretKey::generate().unwrap();
            let shown = sk.to_display().unwrap();
            assert_eq!(shown.expose_secret().len(), DISPLAY_LEN);
            assert_eq!(shown.capacity(), DISPLAY_LEN);
            let last = sk.last_group().unwrap();
            assert_eq!(last.expose_secret().len(), GROUP);
            assert_eq!(last.capacity(), GROUP);
        }
    }

    #[test]
    fn checksum_is_domain_separated_sha256() {
        let sk = [0x42u8; 16];
        let mut h = Sha256::new();
        h.update(b"mosaic/v1/secret-key-checksum");
        h.update(sk);
        assert_eq!(checksum(&sk), h.finalize()[0]);
    }

    #[test]
    fn parse_is_lenient_in_the_allowed_ways() {
        let sk = SecretKey::generate().unwrap();
        let shown = sk.to_display().unwrap();
        let s = shown.expose_secret();
        let variants = [
            s.to_string(),
            s.to_lowercase(),
            s.replace('-', ""),
            s.replace('-', " "),
            format!("  {}\n", s.replace('-', "\t")),
            s.replace('0', "O").replace('1', "l"),
            s.replace('1', "I").replace('0', "o"),
            // ADR-0007 item 8: the `M1` prefix is normalized like the data
            // characters (case, aliases, separators).
            format!("ml{}", &s[2..]),
            format!("MI{}", &s[2..]),
            format!("mi{}", &s[2..]),
            format!("M-1{}", &s[2..]),
            format!("M 1{}", &s[2..]),
        ];
        for v in variants {
            assert_eq!(SecretKey::parse(&v).unwrap(), sk, "{v}");
        }
    }

    #[test]
    fn parse_rejects_malformed() {
        let sk = SecretKey::generate().unwrap();
        let shown = sk.to_display().unwrap();
        let s = shown.expose_secret();
        let malformed = [
            String::new(),
            "M1-".to_string(),
            s[3..].to_string(),                   // missing prefix
            format!("M2{}", &s[2..]),             // wrong version
            format!("X1{}", &s[2..]),             // wrong prefix
            format!("N1{}", &s[2..]),             // wrong prefix
            format!("MO{}", &s[2..]),             // O is an alias of 0, not 1
            s[..s.len() - 1].to_string(),         // too short
            format!("{s}0"),                      // too long
            format!("{s}0000000000"),             // much too long
            s.replace(&s[4..5], "U"),             // U is not Crockford
            format!("{}!{}", &s[..10], &s[11..]), // invalid char
            format!("{}\u{00E9}{}", &s[..10], &s[11..]),
        ];
        for m in malformed {
            assert_eq!(
                SecretKey::parse(&m),
                Err(Error::SecretKeyMalformed),
                "{m:?}"
            );
        }
    }

    #[test]
    fn parse_rejects_nonzero_pad_bits() {
        let sk = key([7u8; 16]);
        let shown = sk.to_display().unwrap();
        let s = shown.expose_secret();
        let last = s.chars().last().unwrap();
        let v = decode_symbol(last).unwrap();
        // The last symbol carries 1 data bit and 4 pad bits.
        assert_eq!(v & 0x0F, 0);
        for pad in 1..16u8 {
            let bad = format!("{}{}", &s[..s.len() - 1], symbol_char(v | pad));
            assert_eq!(SecretKey::parse(&bad), Err(Error::SecretKeyMalformed));
        }
    }

    #[test]
    fn wrong_checksum_is_mistyped() {
        let sk = key([1u8; 16]);
        let shown = sk.to_display().unwrap();
        let s = shown.expose_secret().to_string();
        // Change the first data character: SK changes, checksum does not
        // (unless 1/256 bad luck, which this fixed key does not hit).
        let first = s.chars().nth(3).unwrap();
        let replacement = if first == '0' { '2' } else { '0' };
        let bad = format!("M1-{replacement}{}", &s[4..]);
        assert_eq!(SecretKey::parse(&bad), Err(Error::SecretKeyMistyped));
    }

    #[test]
    fn last_group_check() {
        let sk = SecretKey::generate().unwrap();
        let g = sk.last_group().unwrap();
        let g = g.expose_secret();
        assert_eq!(g.len(), 4);
        assert!(sk.matches_last_group(g));
        assert!(sk.matches_last_group(&g.to_lowercase()));
        assert!(sk.matches_last_group(&format!(" {g} ")));
        assert!(!sk.matches_last_group(&g[..3]));
        assert!(!sk.matches_last_group(&format!("{g}0")));
        assert!(!sk.matches_last_group("!!!!"));
        assert!(!sk.matches_last_group(&"0".repeat(40)));
        let other = if g.starts_with('Z') { "Y" } else { "Z" };
        assert!(!sk.matches_last_group(&format!("{other}{}", &g[1..])));
    }

    /// The branch-free symbol mapping agrees with a plain table lookup and
    /// the alias rules, for every value and for every character in a range
    /// that covers ASCII, Latin-1 and beyond (including characters whose low
    /// byte looks like a valid symbol).
    #[test]
    fn constant_time_symbol_mapping_matches_reference() {
        for v in 0u8..32 {
            assert_eq!(symbol_char(v), char::from(ALPHABET[usize::from(v)]));
            assert_eq!(decode_symbol(symbol_char(v)), Some(v));
        }
        // Only the low 5 bits are used.
        assert_eq!(symbol_char(32 + 5), symbol_char(5));
        let reference = |c: char| -> Option<u8> {
            let c = match c.to_ascii_uppercase() {
                'O' => '0',
                'I' | 'L' => '1',
                other => other,
            };
            let b = u8::try_from(c).ok()?;
            let pos = ALPHABET.iter().position(|&a| a == b)?;
            u8::try_from(pos).ok()
        };
        let extra = [
            '\u{0130}',
            '\u{0141}',
            '\u{0230}',
            '\u{1D160}',
            '\u{10FFFF}',
        ];
        for c in (0u32..0x800).filter_map(char::from_u32).chain(extra) {
            assert_eq!(decode_symbol(c), reference(c), "{c:?}");
        }
    }

    #[test]
    fn decode_payload_rejects_bad_lengths() {
        let mut out = [0u8; PAYLOAD_LEN];
        assert_eq!(
            decode_payload(&[0; 27], &mut out),
            Err(Error::SecretKeyMalformed)
        );
        assert_eq!(symbol_char(31), 'Z');
    }
}
