//! Email normalization: any input normalizes or is refused without a panic,
//! within the length limits, and a normalized address is a fixed point.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_sync::wire::email::{MAX_EMAIL_LEN, normalize};

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(normal) = normalize(input) {
        assert!(normal.chars().count() <= MAX_EMAIL_LEN);
        assert!(normal.contains('@'));
        assert_eq!(normalize(&normal).as_deref(), Ok(normal.as_str()), "a fixed point");
    }
});
