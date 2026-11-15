//! The masking engine (agent access design §8): a stream gives the one-shot
//! output for every split, and the output of a masked buffer holds no raw
//! value and no padded standard base64 of it.
//!
//! Input layout: byte 0 is the value length minus 12 (so 12 to 267 bytes),
//! then the value, then two split points (one byte each, as a fraction of the
//! data), then the data to mask.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_agent::mask::{MASK_TEXT, Masker};

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// Standard base64 with padding, written out so the target has no extra
/// dependency.
fn base64_padded(data: &[u8]) -> Vec<u8> {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let sextet = |shift: u32| A[((n >> shift) & 0x3f) as usize];
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { b'=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { b'=' });
    }
    out
}

fuzz_target!(|input: &[u8]| {
    let Some((&len_byte, rest)) = input.split_first() else {
        return;
    };
    let len = usize::from(len_byte) + 12;
    if rest.len() < len + 2 {
        return;
    }
    let (value, rest) = rest.split_at(len);
    let (cuts, data) = rest.split_at(2);
    let a = data.len() * usize::from(cuts[0]) / 255;
    let b = data.len() * usize::from(cuts[1]) / 255;
    let (i, j) = if a <= b { (a, b) } else { (b, a) };

    let masker = Masker::new(&[value]);
    let (once, count) = masker.mask(data);

    let mut stream = masker.stream();
    let mut streamed = stream.push(&data[..i]);
    streamed.extend(stream.push(&data[i..j]));
    streamed.extend(stream.push(&data[j..]));
    let (tail, streamed_count) = stream.finish_counted();
    streamed.extend(tail);
    assert_eq!(streamed, once, "the stream equals the one-shot output");
    assert_eq!(streamed_count, count, "and counts the same replacements");

    // A raw value can only come back if it shares bytes with the replacement
    // text; otherwise no occurrence survives the leftmost-longest scan.
    let shares_mask_bytes = value.iter().any(|b| MASK_TEXT.as_bytes().contains(b));
    if !shares_mask_bytes {
        assert!(!contains(&once, value), "raw value survived");
        let b64 = base64_padded(value);
        if !b64.iter().any(|b| MASK_TEXT.as_bytes().contains(b)) {
            assert!(!contains(&once, &b64), "padded base64 survived");
        }
    }
});
