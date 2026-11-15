//! The masking engine (agent access design §8).

use core::fmt::Write as _;

use base64ct::{Base64, Base64Unpadded, Base64Url, Base64UrlUnpadded, Encoding};

use crate::mask::{MASK_TEXT, Masker};

/// At least 12 bytes, printable, nothing that needs escaping.
const V: &str = "sk-test-CANARY-0123456789abcdef";
/// Characters that percent, form and JSON encodings change: `/ + = space "`
/// backslash, a control character and non-ASCII.
const SPECIAL: &str = "sk/te+st=CAN ARY\"q\\r\u{7}-\u{e9}\u{2713}-0123456789";

pub(crate) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn hex(bytes: &[u8], upper: bool) -> String {
    let mut out = String::new();
    for b in bytes {
        let _ = if upper {
            write!(out, "{b:02X}")
        } else {
            write!(out, "{b:02x}")
        };
    }
    out
}

fn percent(value: &str, upper: bool, form: bool) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else if form && b == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&hex(&[b], upper));
        }
    }
    out
}

/// A backslash-u escape for every non-ASCII character of an already
/// JSON-escaped string.
fn unicode_escapes(json: &str, upper: bool) -> String {
    let mut s = String::new();
    for c in json.chars() {
        if c.is_ascii() {
            s.push(c);
            continue;
        }
        let mut units = [0u16; 2];
        for u in c.encode_utf16(&mut units) {
            s.push_str("\\u");
            s.push_str(&hex(&u.to_be_bytes(), upper));
        }
    }
    s
}

/// Every encoding the design lists, built with independent encoders
/// (`base64ct`, `serde_json`, plain formatting).
pub(crate) fn encodings_of(value: &str) -> Vec<String> {
    let b = value.as_bytes();
    let json = serde_json::to_string(value).unwrap();
    let json = json[1..json.len() - 1].to_owned();
    let mut forms = vec![
        value.to_owned(),
        Base64::encode_string(b),
        Base64Unpadded::encode_string(b),
        Base64Url::encode_string(b),
        Base64UrlUnpadded::encode_string(b),
        percent(value, true, false),
        percent(value, false, false),
        percent(value, true, true),
        hex(b, false),
        hex(b, true),
        json.clone(),
        json.replace('/', "\\/"),
        unicode_escapes(&json, false),
        unicode_escapes(&json, true),
    ];
    forms.sort();
    forms.dedup();
    forms
}

#[test]
fn every_listed_encoding_is_masked() {
    for value in [V, SPECIAL] {
        let masker = Masker::new(&[value.as_bytes()]);
        let forms = encodings_of(value);
        assert!(forms.len() >= 5, "the test covers many forms");
        for form in forms {
            let (out, n) = masker.mask(format!("a{form}b").as_bytes());
            assert_eq!(n, 1, "one replacement");
            assert!(!contains(&out, form.as_bytes()), "encoding survived");
            assert_eq!(
                out,
                format!("a{MASK_TEXT}b").into_bytes(),
                "encoding not replaced whole"
            );
        }
    }
}

#[test]
fn embedded_base64_cores_are_masked() {
    let masker = Masker::new(&[V.as_bytes()]);
    let encoders: [fn(&[u8]) -> String; 4] = [
        |b| Base64::encode_string(b),
        |b| Base64Unpadded::encode_string(b),
        |b| Base64Url::encode_string(b),
        |b| Base64UrlUnpadded::encode_string(b),
    ];
    // Prefix lengths 0 to 5 cover the three alignments twice, as
    // base64("user:" + V) (5) and base64("xy" + V + "z") (2) do.
    for prefix_len in 0..6 {
        for suffix in ["", "z", "zz", "zzz"] {
            for encode in encoders {
                let mut data = "x".repeat(prefix_len).into_bytes();
                data.extend_from_slice(V.as_bytes());
                data.extend_from_slice(suffix.as_bytes());
                let text = encode(&data);
                let (out, n) = masker.mask(text.as_bytes());
                assert_eq!(n, 1, "prefix {prefix_len}, suffix {suffix:?}");
                // At most one partial group at each end plus the alignment
                // remains: the core covers all but ~4 characters of the value.
                let removed = text.len() + MASK_TEXT.len() - out.len();
                assert!(removed >= V.len() * 8 / 6 - 4, "core too short: {removed}");
            }
        }
    }
}

#[test]
fn add_exact_masks_the_basic_header_value() {
    let mut masker = Masker::new(&[V.as_bytes()]);
    let basic = Base64::encode_string(format!("user:{V}").as_bytes());
    let secret_is_user = Base64::encode_string(format!("{V}:").as_bytes());
    let (_, before) = masker.mask(format!("Authorization: Basic {basic}").as_bytes());
    assert_eq!(before, 1, "the embedded core already catches most of it");
    masker.add_exact(basic.as_bytes());
    masker.add_exact(secret_is_user.as_bytes());
    for form in [&basic, &secret_is_user] {
        let (out, n) = masker.mask(format!("Basic {form} end").as_bytes());
        assert_eq!(n, 1);
        assert_eq!(out, format!("Basic {MASK_TEXT} end").into_bytes());
    }
    masker.add_exact(b"");
    assert_eq!(masker.mask(b"plain").1, 0);
}

#[test]
fn partial_values_are_not_masked() {
    let masker = Masker::new(&[V.as_bytes()]);
    let partial = &V[..V.len() - 1];
    let (out, n) = masker.mask(partial.as_bytes());
    assert_eq!(n, 0);
    assert_eq!(out, partial.as_bytes());
    let (out, n) = masker.mask(&V.as_bytes()[1..]);
    assert_eq!((n, out.as_slice()), (0, &V.as_bytes()[1..]));
    let (_, n) = masker.mask(b"nothing to see here");
    assert_eq!(n, 0);
}

#[test]
fn hex_is_case_insensitive_and_others_are_not() {
    let masker = Masker::new(&[V.as_bytes()]);
    let (out, n) = masker.mask(V.to_uppercase().as_bytes());
    assert_eq!(n, 0, "the raw form is case sensitive");
    assert_eq!(out, V.to_uppercase().as_bytes());
    let lower = hex(V.as_bytes(), false);
    let mixed: String = lower
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    for form in [lower.clone(), lower.to_uppercase(), mixed] {
        let (out, n) = masker.mask(form.as_bytes());
        assert_eq!(n, 1, "{form}");
        assert_eq!(out, MASK_TEXT.as_bytes());
    }
    let b64 = Base64::encode_string(V.as_bytes());
    assert_eq!(
        masker.mask(b64.to_lowercase().as_bytes()).1,
        0,
        "base64 is case sensitive"
    );
}

#[test]
fn longest_match_wins_and_several_values_are_masked() {
    let a = "first-secret-value-AAAA";
    let b = "second-secret-value-BBBB";
    let masker = Masker::new(&[a.as_bytes(), b.as_bytes()]);
    let b64 = Base64::encode_string(a.as_bytes());
    let (out, n) = masker.mask(format!("{a}|{b}|{b64}").as_bytes());
    assert_eq!(n, 3);
    assert_eq!(
        out,
        format!("{MASK_TEXT}|{MASK_TEXT}|{MASK_TEXT}").into_bytes()
    );
    // Back to back: scanning continues after a match.
    let (out, n) = masker.mask(format!("{a}{a}").as_bytes());
    assert_eq!(
        (n, out),
        (2, format!("{MASK_TEXT}{MASK_TEXT}").into_bytes())
    );
}

#[test]
fn stream_equals_one_shot_at_every_split() {
    let masker = Masker::new(&[V.as_bytes()]);
    let b64 = Base64::encode_string(V.as_bytes());
    let body = format!("pre{V}mid{b64}post");
    let body = body.as_bytes();
    let (expected, n) = masker.mask(body);
    assert_eq!(n, 2);
    for i in 0..=body.len() {
        let mut s = masker.stream();
        let mut out = s.push(&body[..i]);
        out.extend(s.push(&body[i..]));
        let (tail, count) = s.finish_counted();
        out.extend(tail);
        assert_eq!(out, expected, "split at {i}");
        assert_eq!(count, 2);
        for j in i..=body.len() {
            let mut s = masker.stream();
            let mut out = s.push(&body[..i]);
            out.extend(s.push(&body[i..j]));
            out.extend(s.push(&body[j..]));
            out.extend(s.finish());
            assert_eq!(out, expected, "splits at {i} and {j}");
        }
    }
    // Byte by byte, and the hold-back is what keeps bytes back.
    let mut s = masker.stream();
    let mut out = Vec::new();
    for b in body {
        out.extend(s.push(&[*b]));
    }
    assert!(!contains(&out, V.as_bytes()));
    out.extend(s.finish());
    assert_eq!(out, expected);
    assert!(masker.holdback() >= V.len() - 1);
}

#[test]
fn a_stream_never_releases_part_of_a_value() {
    let masker = Masker::new(&[V.as_bytes()]);
    let mut s = masker.stream();
    let released = s.push(&V.as_bytes()[..10]);
    assert!(
        released.is_empty(),
        "fewer bytes than the longest pattern are all held back"
    );
    assert!(!contains(&released, &V.as_bytes()[..10]));
}

#[test]
fn an_empty_masker_passes_everything_through() {
    let masker = Masker::new(&[]);
    assert_eq!(masker.holdback(), 0);
    assert_eq!(masker.mask(b"abc"), (b"abc".to_vec(), 0));
    let mut s = masker.stream();
    assert_eq!(s.push(b"ab"), b"ab");
    assert_eq!(s.push(b"c"), b"c");
    assert_eq!(s.finish(), b"");
    assert_eq!(Masker::new(&[b""]).mask(b"abc").1, 0);
}

#[test]
fn a_large_body_is_masked_in_one_pass() {
    let masker = Masker::new(&[V.as_bytes(), SPECIAL.as_bytes()]);
    let mut body = Vec::with_capacity(1 << 20);
    while body.len() < (1 << 20) - 64 {
        body.extend_from_slice(b"{\"id\":12345,\"note\":\"nothing here\"},");
        body.extend_from_slice(V.as_bytes());
    }
    let (out, n) = masker.mask(&body);
    assert!(n > 1000);
    assert!(!contains(&out, V.as_bytes()));
}

#[test]
fn masker_debug_is_redacted() {
    let masker = Masker::new(&[V.as_bytes()]);
    assert_eq!(format!("{masker:?}"), "[REDACTED]");
    assert_eq!(format!("{:?}", masker.stream()), "[REDACTED]");
}
