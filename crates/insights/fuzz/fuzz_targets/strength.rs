//! The strength estimate on arbitrary text.
//!
//! - It never panics and is deterministic.
//! - The estimate is finite, never negative and never above the
//!   brute-force bound.
//! - Appending text beyond `MAX_ANALYZED_CHARS` changes nothing.
#![no_main]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use mosaic_insights::{MAX_ANALYZED_CHARS, brute_force_bits, estimate_strength};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let estimate = estimate_strength(&text);
    let bits = estimate.bits();
    assert!(bits.is_finite() && bits >= 0.0, "{bits}");
    assert!(bits <= brute_force_bits(&text) + 1e-9, "{bits}");
    assert_eq!(estimate, estimate_strength(&text));
    assert_eq!(estimate.is_weak(), estimate.strength() < mosaic_insights::Strength::Fair);

    if text.chars().count() >= MAX_ANALYZED_CHARS {
        let longer = format!("{text}tail");
        assert_eq!(estimate_strength(&longer), estimate);
    }
});
