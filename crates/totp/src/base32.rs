//! RFC 4648 §6 base32 (alphabet `A-Z2-7`), hand-rolled.
//!
//! A crate would not save much: the codec is about 60 lines, and it has to
//! zeroize its buffers and map symbols without secret-dependent table
//! lookups, which general-purpose crates do not promise. Symbol mapping uses
//! `subtle` comparisons and range arithmetic (as `mosaic-core`'s Secret Key
//! codec does, ADR-0007 item 12). Whether a character is whitespace,
//! padding or invalid is an ordinary branch: separators carry no secret
//! bits, and invalid input is rejected (compare PROGRESS M1-R2).

use subtle::{Choice, ConditionallySelectable, ConstantTimeGreater};
use zeroize::Zeroize;

use crate::error::{Error, Result};

/// `lo <= c <= hi` in constant time.
fn in_range(c: u8, lo: u8, hi: u8) -> Choice {
    !lo.ct_gt(&c) & !c.ct_gt(&hi)
}

/// Maps one character to its 5-bit value; the `Choice` is set if the
/// character is in the alphabet (either case).
fn decode_symbol(c: u8) -> (u8, Choice) {
    let lower = c | 0x20;
    let is_alpha = in_range(lower, b'a', b'z');
    let is_digit = in_range(c, b'2', b'7');
    let alpha = u8::conditional_select(&0, &lower.wrapping_sub(b'a'), is_alpha);
    let digit = u8::conditional_select(&0, &c.wrapping_sub(b'2').wrapping_add(26), is_digit);
    (alpha | digit, is_alpha | is_digit)
}

/// Maps a 5-bit value to its upper-case character.
fn encode_symbol(v: u8) -> char {
    let v = v & 0x1F;
    let is_alpha = !v.ct_gt(&25);
    let c = u8::conditional_select(
        &v.wrapping_sub(26).wrapping_add(b'2'),
        &v.wrapping_add(b'A'),
        is_alpha,
    );
    char::from(c)
}

fn is_skippable(c: u8) -> bool {
    c.is_ascii_whitespace()
}

/// Decodes leniently; see [`crate::OtpSecret::from_base32`]. The result may
/// be empty; the caller checks lengths.
pub(crate) fn decode(text: &[u8]) -> Result<Vec<u8>> {
    // Reserve the final size up front so the secret is never reallocated.
    let mut out = Vec::with_capacity(text.len() * 5 / 8);
    let mut acc: u16 = 0;
    let mut bits: u8 = 0;
    let mut padded = false;
    let mut result = Ok(());
    for &c in text {
        if is_skippable(c) {
            continue;
        }
        if c == b'=' {
            padded = true;
            continue;
        }
        let (v, valid) = decode_symbol(c);
        if padded || !bool::from(valid) {
            result = Err(Error::InvalidSecret);
            break;
        }
        acc = (acc << 5) | u16::from(v);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((acc >> bits) & 0xFF).unwrap_or(0));
        }
        acc &= (1u16 << bits) - 1;
    }
    acc.zeroize();
    match result {
        Ok(()) => Ok(out),
        Err(e) => {
            out.zeroize();
            Err(e)
        }
    }
}

/// Length of the unpadded encoding of `n` bytes.
pub(crate) fn encoded_len(n: usize) -> usize {
    (n * 8).div_ceil(5)
}

/// Appends the unpadded upper-case encoding of `data` to `out`. The caller
/// reserves [`encoded_len`] bytes first.
pub(crate) fn encode_into(data: &[u8], out: &mut String) {
    let mut acc: u16 = 0;
    let mut bits: u8 = 0;
    for &b in data {
        acc = (acc << 8) | u16::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(encode_symbol(
                u8::try_from((acc >> bits) & 0x1F).unwrap_or(0),
            ));
        }
        acc &= (1u16 << bits) - 1;
    }
    if bits > 0 {
        out.push(encode_symbol(
            u8::try_from((acc << (5 - bits)) & 0x1F).unwrap_or(0),
        ));
    }
    acc.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

    fn reference_decode_symbol(c: u8) -> Option<u8> {
        let up = c.to_ascii_uppercase();
        ALPHABET
            .iter()
            .position(|&a| a == up)
            .map(|p| u8::try_from(p).unwrap())
    }

    fn enc(data: &[u8]) -> String {
        let mut s = String::with_capacity(encoded_len(data.len()));
        encode_into(data, &mut s);
        assert_eq!(s.len(), encoded_len(data.len()));
        s
    }

    #[test]
    fn symbol_tables_match_reference() {
        for c in 0..=255u8 {
            let (v, ok) = decode_symbol(c);
            match reference_decode_symbol(c) {
                Some(r) => {
                    assert!(bool::from(ok), "{c}");
                    assert_eq!(v, r, "{c}");
                }
                None => assert!(!bool::from(ok), "{c}"),
            }
        }
        for v in 0..32u8 {
            assert_eq!(encode_symbol(v), char::from(ALPHABET[usize::from(v)]));
        }
    }

    /// RFC 4648 §10 test vectors (padding stripped on output).
    #[test]
    fn rfc4648_vectors() {
        let cases: [(&[u8], &str); 7] = [
            (b"", ""),
            (b"f", "MY======"),
            (b"fo", "MZXQ===="),
            (b"foo", "MZXW6==="),
            (b"foob", "MZXW6YQ="),
            (b"fooba", "MZXW6YTB"),
            (b"foobar", "MZXW6YTBOI======"),
        ];
        for (raw, padded) in cases {
            assert_eq!(decode(padded.as_bytes()).unwrap(), raw);
            assert_eq!(enc(raw), padded.trim_end_matches('='));
            let lower = padded.to_ascii_lowercase();
            assert_eq!(decode(lower.trim_end_matches('=').as_bytes()).unwrap(), raw);
        }
    }

    #[test]
    fn lenient_forms() {
        let want = b"12345678901234567890";
        for text in [
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
            "gezdgnbvgy3tqojqgezdgnbvgy3tqojq",
            "GEZD GNBV GY3T QOJQ GEZD GNBV GY3T QOJQ",
            " gezd\tgnbv\ngy3t\rqojq gezdgnbvgy3tqojq ",
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ========",
            "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ= =",
        ] {
            assert_eq!(decode(text.as_bytes()).unwrap(), want, "{text}");
        }
        // Trailing bits that do not fill a byte are dropped.
        assert_eq!(decode(b"MZXW6YQB").unwrap(), b"foob\x01");
        assert_eq!(decode(b"MZXW6YR").unwrap(), b"foob");
        assert_eq!(decode(b"A").unwrap(), b"");
    }

    #[test]
    fn rejects_bad_input() {
        for text in ["MZXW6YQ=A", "MZ=XW", "MZXW1", "MZXW8", "MZXW-6", "MZ\u{e9}"] {
            assert_eq!(
                decode(text.as_bytes()).err(),
                Some(Error::InvalidSecret),
                "{text}"
            );
        }
    }

    proptest! {
        #[test]
        fn round_trip(data in proptest::collection::vec(any::<u8>(), 0..200)) {
            let s = enc(&data);
            prop_assert_eq!(decode(s.as_bytes()).unwrap(), data.clone());
            // Lower case, spaces every 4 characters and padding decode the same.
            let mut noisy = String::new();
            for (i, ch) in s.chars().enumerate() {
                if i % 4 == 0 { noisy.push(' '); }
                noisy.push(ch.to_ascii_lowercase());
            }
            noisy.push_str("===");
            prop_assert_eq!(decode(noisy.as_bytes()).unwrap(), data);
        }

        #[test]
        fn decode_never_panics_and_reencodes(text in "[A-Za-z2-7 =]{0,64}") {
            if let Ok(bytes) = decode(text.as_bytes()) {
                let again = enc(&bytes);
                prop_assert_eq!(decode(again.as_bytes()).unwrap(), bytes);
            }
        }
    }
}
