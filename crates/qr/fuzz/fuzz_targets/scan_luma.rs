//! Pixels straight into QR detection and decoding, without the image
//! decoders. The first byte picks the pixel layout, the next two the width
//! (little endian, clamped to 21..=1024), the rest are the pixels.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_qr::{Frame, PixelLayout};

const LAYOUTS: [PixelLayout; 6] = [
    PixelLayout::Luma8,
    PixelLayout::LumaA8,
    PixelLayout::Rgb8,
    PixelLayout::Rgba8,
    PixelLayout::Rgbx8,
    PixelLayout::Bgrx8,
];

fuzz_target!(|data: &[u8]| {
    let Some((head, pixels)) = data.split_at_checked(3) else {
        return;
    };
    let layout = LAYOUTS[usize::from(head[0]) % LAYOUTS.len()];
    let bpp = layout.bytes_per_pixel();
    let width = usize::from(u16::from_le_bytes([head[1], head[2]])).clamp(21, 1024);
    let stride = width * bpp;
    let height = pixels.len() / stride;
    if height < 21 {
        return;
    }
    let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height)) else {
        return;
    };
    if let Ok(frame) = Frame::new(w, h, stride, layout, pixels) {
        let _ = mosaic_qr::scan_frames(&[frame]);
    }
});
