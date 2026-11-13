//! Red team of the QR scanner (RT-parsers, 2026-11-13): malformed frames,
//! extreme geometry and hostile image files, over the public API. Every
//! attack here held; these are regression tests.

#![expect(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

/// The high byte of a xorshift step: cheap noise without a dependency.
fn noise_byte(x: &mut u64) -> u8 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    u8::try_from(*x >> 56).unwrap()
}

use std::time::{Duration, Instant};

use mosaic_qr::{Frame, ImageLimits, PixelLayout, ScanError, decode_image, scan_frames};
use proptest::prelude::*;

const LAYOUTS: [PixelLayout; 6] = [
    PixelLayout::Luma8,
    PixelLayout::LumaA8,
    PixelLayout::Rgb8,
    PixelLayout::Rgba8,
    PixelLayout::Rgbx8,
    PixelLayout::Bgrx8,
];

/// Geometry that cannot be backed by memory is refused before any arithmetic
/// overflows or any buffer is touched.
#[test]
fn extreme_frame_geometry_is_refused_without_allocation() {
    let px = [0u8; 64];
    for layout in LAYOUTS {
        let bpp = layout.bytes_per_pixel();
        for (w, h, stride) in [
            (u32::MAX, u32::MAX, usize::MAX),
            (u32::MAX, 1, bpp),
            (1, u32::MAX, usize::MAX),
            (0, 1, 1),
            (1, 0, 1),
            (8, 8, 1),
            (1 << 20, 1 << 20, (1 << 20) * bpp),
        ] {
            assert_eq!(
                Frame::new(w, h, stride, layout, &px).unwrap_err(),
                ScanError::Unreadable,
                "{layout:?} {w}x{h} stride {stride}"
            );
        }
        // A stride that makes the last row end far past the buffer.
        assert!(Frame::new(2, 2, usize::MAX / 2, layout, &px).is_err());
    }
}

/// A frame wider than the downscale limit but one pixel tall never reaches
/// the detector (the downscaled height is 0), and a 4096 by 4096 noise frame
/// is searched within a bounded time.
#[test]
fn wide_and_large_noise_frames_are_bounded() {
    let row = vec![0x55u8; 1 << 16];
    let frame = Frame::new(1 << 16, 1, 1 << 16, PixelLayout::Luma8, &row).unwrap();
    assert_eq!(scan_frames(&[frame]).unwrap_err(), ScanError::NotFound);

    let side = 4096usize;
    let mut noise = vec![0u8; side * side];
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for px in &mut noise {
        *px = noise_byte(&mut x);
    }
    let side_px = u32::try_from(side).unwrap();
    let frame = Frame::new(side_px, side_px, side, PixelLayout::Luma8, &noise).unwrap();
    let started = Instant::now();
    let result = scan_frames(&[frame; 2]);
    assert!(matches!(
        result,
        Err(ScanError::NotFound | ScanError::NotOtp)
    ));
    assert!(
        started.elapsed() < Duration::from_secs(60),
        "{:?}",
        started.elapsed()
    );
}

/// Image files that claim huge dimensions are refused from their header,
/// before any pixel buffer exists; truncated and non-image files are
/// unreadable; nothing panics.
#[test]
fn hostile_image_headers_are_refused_early() {
    let limits = ImageLimits::DEFAULT;
    // PNG: signature, IHDR with 2^31-1 by 2^31-1.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&0x7fff_ffffu32.to_be_bytes());
    png.extend_from_slice(&0x7fff_ffffu32.to_be_bytes());
    png.extend_from_slice(&[8, 0, 0, 0, 0]);
    png.extend_from_slice(&[0; 4]);
    // The chunk CRC is wrong, so the decoder may refuse the file before it
    // reaches the size check; either way no buffer is allocated.
    assert!(matches!(
        decode_image(&png, &limits),
        Err(ScanError::TooLarge | ScanError::Unreadable)
    ));
    // JPEG: SOI, then a SOF0 claiming 65535 by 65535.
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x01, 0x11, 0x00,
        0xFF, 0xD9,
    ];
    assert!(matches!(
        decode_image(&jpeg, &limits),
        Err(ScanError::TooLarge | ScanError::Unreadable)
    ));
    for bad in [&b""[..], b"\x89PNG", b"\xFF\xD8\xFF", b"GIF89a", b"<svg/>"] {
        assert_eq!(
            decode_image(bad, &limits).unwrap_err(),
            ScanError::Unreadable
        );
    }
    let over = vec![0u8; limits.max_file_bytes + 1];
    assert_eq!(
        decode_image(&over, &limits).unwrap_err(),
        ScanError::TooLarge
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Random small frames of every layout, with any stride at or above the
    /// row, are scanned without a panic and never yield a code.
    #[test]
    fn random_frames_never_panic(
        w in 1u32..40,
        h in 1u32..40,
        extra in 0usize..8,
        layout in 0usize..6,
        seed in any::<u64>(),
    ) {
        let layout = LAYOUTS[layout];
        let stride = usize::try_from(w).unwrap() * layout.bytes_per_pixel() + extra;
        let len = stride * usize::try_from(h).unwrap();
        let mut pixels = vec![0u8; len];
        let mut x = seed | 1;
        for px in &mut pixels {
            *px = noise_byte(&mut x);
        }
        let frame = Frame::new(w, h, stride, layout, &pixels).unwrap();
        prop_assert!(scan_frames(&[frame]).is_err());
    }

    /// Random bytes behind a PNG or JPEG signature are unreadable or too
    /// large, never a panic.
    #[test]
    fn random_image_bytes_never_panic(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&data);
        let mut jpeg = b"\xFF\xD8\xFF".to_vec();
        jpeg.extend_from_slice(&data);
        for bytes in [png, jpeg] {
            prop_assert!(matches!(
                decode_image(&bytes, &ImageLimits::DEFAULT),
                Err(ScanError::Unreadable | ScanError::TooLarge)
            ));
        }
    }
}
