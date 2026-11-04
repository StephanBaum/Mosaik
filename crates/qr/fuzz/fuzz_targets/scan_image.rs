//! Image bytes to `otpauth://`: never panics and never allocates past the
//! limits. Small limits keep the exec rate up and still reach the `TooLarge`
//! paths.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_qr::ImageLimits;

fuzz_target!(|data: &[u8]| {
    let limits = ImageLimits {
        max_file_bytes: 4 << 20,
        max_side: 4096,
        max_pixels: 4_000_000,
        max_alloc: 64 << 20,
    };
    let _ = mosaic_qr::scan_image(data, &limits);
});
