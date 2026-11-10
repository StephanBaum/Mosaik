//! The Have I Been Pwned range-response parser (`breach::parse_range`).
//!
//! - It never panics, on any bytes.
//! - It answers once per wanted suffix, or refuses with `Protocol` (nothing
//!   else: no other error can come out of the parser).
//! - When it accepts a body, a suffix is reported found exactly when some
//!   line is that suffix (either case) followed by `:` and a count that is
//!   not all zeros (count 0 is padding), checked here the naive way.
#![no_main]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use mosaic_insights::breach::{RangeError, SUFFIX_LEN, parse_range};

/// The suffixes the seeds contain (`0` and `A` runs, and the SHA-1 of
/// "password").
const WANTED: [[u8; SUFFIX_LEN]; 3] = [
    *b"00000000000000000000000000000000000",
    *b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    *b"1E4C9B93F3F0682250B6CF8331B7EE68FD8",
];

fuzz_target!(|data: &[u8]| {
    match parse_range(data, &WANTED) {
        Ok(hits) => {
            assert_eq!(hits.len(), WANTED.len());
            for (want, hit) in WANTED.iter().zip(hits) {
                assert_eq!(hit, naive(data, want));
            }
        }
        Err(e) => assert_eq!(e, RangeError::Protocol),
    }
});

/// Whether a line of the (already accepted) body is `want:COUNT` with a
/// non-zero count.
fn naive(body: &[u8], want: &[u8; SUFFIX_LEN]) -> bool {
    body.split(|&b| b == b'\n').any(|raw| {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        let Some((suffix, rest)) = line.split_at_checked(SUFFIX_LEN) else {
            return false;
        };
        let Some(count) = rest.strip_prefix(b":") else {
            return false;
        };
        suffix.eq_ignore_ascii_case(want) && count.iter().any(|&d| d != b'0')
    })
}
