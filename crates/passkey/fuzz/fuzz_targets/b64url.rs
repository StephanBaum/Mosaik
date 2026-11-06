//! `from_b64url` with several length limits: it never panics, never returns
//! more than `max_len` bytes, and what it accepts is the canonical encoding
//! (`b64url` of the decoded bytes is the input again). The first input byte
//! picks the limit, the rest is the string.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_passkey::{b64url, from_b64url};

const LIMITS: [usize; 6] = [0, 1, 16, 64, 1024, 65_535];

fuzz_target!(|data: &[u8]| {
    let Some((&pick, rest)) = data.split_first() else {
        return;
    };
    let max_len = LIMITS[usize::from(pick) % LIMITS.len()];
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    if let Ok(bytes) = from_b64url(text, max_len) {
        assert!(bytes.len() <= max_len);
        assert_eq!(b64url(&bytes), text);
        // The same string decodes under any larger limit, to the same bytes.
        assert_eq!(from_b64url(text, usize::MAX / 4).as_ref(), Ok(&bytes));
    }
});
