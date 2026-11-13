//! The `inject` template scanner: any text splits into segments without a
//! panic, every reference is a `mosaic://` substring of the input, and the
//! scan is deterministic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_cli::refs::{Segment, scan_template};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let segments = scan_template(text);
    for segment in &segments {
        if let Segment::Reference(r) = segment {
            assert!(r.starts_with("mosaic://"));
            assert!(text.contains(r), "a reference is a slice of the input");
        }
    }
    assert_eq!(segments, scan_template(text), "deterministic");
});
