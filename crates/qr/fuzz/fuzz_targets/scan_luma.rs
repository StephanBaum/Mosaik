//! Grey levels straight into QR detection and decoding, without the image
//! decoders. The first two bytes are the width (little endian, clamped to
//! 21..=1024), the rest are the pixels.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_qr::{Frame, PixelLayout};

fuzz_target!(|data: &[u8]| {
    let Some((head, pixels)) = data.split_at_checked(2) else {
        return;
    };
    let width = usize::from(u16::from_le_bytes([head[0], head[1]])).clamp(21, 1024);
    let height = pixels.len() / width;
    if height < 21 {
        return;
    }
    let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height)) else {
        return;
    };
    if let Ok(frame) = Frame::new(w, h, width, PixelLayout::Luma8, pixels) {
        let _ = mosaic_qr::scan_frames(&[frame]);
    }
});
