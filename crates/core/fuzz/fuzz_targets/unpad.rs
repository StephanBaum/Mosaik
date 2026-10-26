//! Strict unpadding: never panics, accepts only canonical padding, and
//! pad/unpad round-trips.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::encoding::{pad, unpad};

fuzz_target!(|data: &[u8]| {
    if let Ok(msg) = unpad(data) {
        let repadded = pad(msg).expect("pad");
        assert_eq!(
            repadded.expose_secret(),
            data,
            "unpad accepted non-canonical padding"
        );
    }
    let padded = pad(data).expect("pad");
    assert_eq!(unpad(padded.expose_secret()).expect("unpad"), data);
});
