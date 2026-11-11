//! Unit tests for `mosaic-qr` (fixtures from `testing`).

use image::{ImageBuffer, ImageEncoder, Luma};
use mosaic_totp::{Algorithm, OtpAuth, OtpKind, SecretText};
use proptest::prelude::*;
use zeroize::Zeroizing;

use crate::testing::{SAMPLE_URI, canvas, inverted, jpeg, png, qr_frame, qr_frame_transparent};
use crate::{
    Frame, ImageLimits, OwnedFrame, Payload, PixelLayout, ScanError, classify, scan_frames,
    scan_image, scan_setup_frames,
};

const DEFAULT: ImageLimits = ImageLimits::DEFAULT;
const OTHER_URI: &str =
    "otpauth://totp/Other:bob?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=Other";

fn assert_sample(auth: &OtpAuth) {
    assert_eq!(
        auth.issuer.as_ref().map(SecretText::expose_secret),
        Some("Example")
    );
    assert_eq!(auth.account_name.expose_secret(), "alice@example.com");
    assert_eq!(
        auth.otp.secret.to_base32().expose_secret(),
        "JBSWY3DPEHPK3PXP"
    );
    assert_eq!(auth.otp.algorithm, Algorithm::Sha1);
    assert_eq!(auth.otp.kind, OtpKind::DEFAULT_TOTP);
}

fn scan_one(frame: &OwnedFrame) -> Result<OtpAuth, ScanError> {
    scan_frames(&[frame.frame()])
}

/// A screen-like canvas with the given codes at fixed spots.
fn screen(codes: &[&str], layout: PixelLayout) -> OwnedFrame {
    let frames: Vec<OwnedFrame> = codes.iter().map(|c| qr_frame(c, 5, 4)).collect();
    let place: Vec<(&OwnedFrame, u32, u32)> = frames
        .iter()
        .enumerate()
        .map(|(i, f)| (f, 40 + 480 * u32::try_from(i).unwrap_or(0), 60))
        .collect();
    canvas(1400, 640, layout, 240, &place)
}

#[test]
fn png_round_trip() {
    let auth = scan_image(&png(&qr_frame(SAMPLE_URI, 4, 4)), &DEFAULT).unwrap();
    assert_sample(&auth);
}

#[test]
fn jpeg_q75() {
    let auth = scan_image(&jpeg(&qr_frame(SAMPLE_URI, 6, 4), 75), &DEFAULT).unwrap();
    assert_sample(&auth);
}

#[test]
fn transparent_png() {
    let auth = scan_image(&png(&qr_frame_transparent(SAMPLE_URI, 4)), &DEFAULT).unwrap();
    assert_sample(&auth);
}

#[test]
fn inverted_code() {
    let auth = scan_image(&png(&inverted(&qr_frame(SAMPLE_URI, 4, 4))), &DEFAULT).unwrap();
    assert_sample(&auth);
}

#[test]
fn screenshot_rgbx_alpha_zero() {
    let qr = qr_frame(SAMPLE_URI, 5, 4);
    let c = canvas(1920, 1080, PixelLayout::Rgbx8, 240, &[(&qr, 700, 300)]);
    assert_sample(&scan_one(&c).unwrap());
}

#[test]
fn screenshot_bgrx() {
    let qr = qr_frame(SAMPLE_URI, 5, 4);
    let c = canvas(1920, 1080, PixelLayout::Bgrx8, 240, &[(&qr, 700, 300)]);
    assert_sample(&scan_one(&c).unwrap());
}

#[test]
fn large_canvas_downscaled() {
    let qr = qr_frame(SAMPLE_URI, 10, 4);
    let c = canvas(7680, 4320, PixelLayout::Rgbx8, 255, &[(&qr, 100, 100)]);
    assert_sample(&scan_one(&c).unwrap());
}

#[test]
fn two_frames_second_has_code() {
    let blank = canvas(800, 600, PixelLayout::Rgbx8, 200, &[]);
    let qr = qr_frame(SAMPLE_URI, 5, 4);
    let with = canvas(800, 600, PixelLayout::Rgbx8, 200, &[(&qr, 50, 50)]);
    assert_sample(&scan_frames(&[blank.frame(), with.frame()]).unwrap());
}

#[test]
fn not_otp() {
    let qr = qr_frame("https://example.com", 5, 4);
    assert_eq!(scan_one(&qr).unwrap_err(), ScanError::NotOtp);
}

#[test]
fn migration() {
    let qr = qr_frame("otpauth-migration://offline?data=CgYKBAAAAAA%3D", 5, 4);
    assert_eq!(scan_one(&qr).unwrap_err(), ScanError::Migration);
}

#[test]
fn migration_wins_over_other() {
    let c = screen(
        &[
            "otpauth-migration://offline?data=CgYKBAAAAAA%3D",
            "https://example.com",
        ],
        PixelLayout::Rgbx8,
    );
    assert_eq!(scan_one(&c).unwrap_err(), ScanError::Migration);
}

#[test]
fn multiple_distinct() {
    let c = screen(&[SAMPLE_URI, OTHER_URI], PixelLayout::Rgbx8);
    assert_eq!(scan_one(&c).unwrap_err(), ScanError::Multiple);
}

#[test]
fn duplicates_count_once() {
    let c = screen(&[SAMPLE_URI, SAMPLE_URI], PixelLayout::Rgbx8);
    assert_sample(&scan_one(&c).unwrap());
}

#[test]
fn otp_beside_other_qr() {
    let c = screen(&[SAMPLE_URI, "https://example.com"], PixelLayout::Rgbx8);
    assert_sample(&scan_one(&c).unwrap());
}

#[test]
fn invalid_otp() {
    for uri in [
        "otpauth://totp/x?secret=1",
        "otpauth://totp/x?secret=JBSWY3DP&algorithm=MD5",
    ] {
        let qr = qr_frame(uri, 5, 4);
        assert_eq!(scan_one(&qr).unwrap_err(), ScanError::InvalidOtp, "{uri}");
    }
}

#[test]
fn no_qr() {
    let blank = canvas(200, 200, PixelLayout::Luma8, 255, &[]);
    assert_eq!(
        scan_image(&png(&blank), &DEFAULT).unwrap_err(),
        ScanError::NotFound
    );
}

#[test]
fn tiny_frame() {
    let tiny = canvas(10, 10, PixelLayout::Luma8, 255, &[]);
    assert_eq!(scan_one(&tiny).unwrap_err(), ScanError::NotFound);
}

#[test]
fn file_too_large() {
    let bytes = png(&qr_frame(SAMPLE_URI, 4, 4));
    let limits = ImageLimits {
        max_file_bytes: 100,
        ..DEFAULT
    };
    assert_eq!(
        scan_image(&bytes, &limits).unwrap_err(),
        ScanError::TooLarge
    );
}

#[test]
fn side_too_large() {
    let bytes = png(&canvas(300, 1, PixelLayout::Luma8, 255, &[]));
    let limits = ImageLimits {
        max_side: 256,
        ..DEFAULT
    };
    assert_eq!(
        scan_image(&bytes, &limits).unwrap_err(),
        ScanError::TooLarge
    );
}

#[test]
fn pixels_too_many() {
    let bytes = png(&canvas(200, 200, PixelLayout::Luma8, 255, &[]));
    let limits = ImageLimits {
        max_pixels: 10_000,
        ..DEFAULT
    };
    assert_eq!(
        scan_image(&bytes, &limits).unwrap_err(),
        ScanError::TooLarge
    );
}

#[test]
fn alloc_limit() {
    let bytes = png(&canvas(200, 200, PixelLayout::Rgba8, 255, &[]));
    let limits = ImageLimits {
        max_alloc: 1024,
        ..DEFAULT
    };
    assert_eq!(
        scan_image(&bytes, &limits).unwrap_err(),
        ScanError::TooLarge
    );
}

#[test]
fn unsupported_format() {
    for bytes in [&b"GIF89a\x01\x00\x01\x00"[..], b"BM\x00\x00\x00\x00", b""] {
        assert_eq!(
            scan_image(bytes, &DEFAULT).unwrap_err(),
            ScanError::Unreadable
        );
    }
}

#[test]
fn truncated_png() {
    let bytes = png(&qr_frame(SAMPLE_URI, 4, 4));
    assert_eq!(
        scan_image(&bytes[..60], &DEFAULT).unwrap_err(),
        ScanError::Unreadable
    );
}

#[test]
fn sixteen_bit_png() {
    let qr = qr_frame(SAMPLE_URI, 4, 4);
    let wide: Vec<u16> = qr.pixels().iter().map(|&v| u16::from(v) * 257).collect();
    let buf: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_raw(qr.width(), qr.height(), wide).unwrap();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            bytes_of(buf.as_raw()).as_slice(),
            buf.width(),
            buf.height(),
            image::ExtendedColorType::L16,
        )
        .unwrap();
    assert_sample(&scan_image(&out, &DEFAULT).unwrap());
}

/// Native-endian bytes of 16-bit samples, as `ImageEncoder` expects them.
fn bytes_of(samples: &[u16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_ne_bytes()).collect()
}

/// A `w` x `h` RGB frame with a smooth gradient (JPEG-friendly).
fn rgb_frame(w: u32, h: u32) -> OwnedFrame {
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            px.extend([
                u8::try_from(x * 3 % 256).unwrap(),
                u8::try_from(y * 5 % 256).unwrap(),
                128,
            ]);
        }
    }
    OwnedFrame::new(w, h, PixelLayout::Rgb8, Zeroizing::new(px)).unwrap()
}

#[test]
fn decode_layouts_and_dimensions() {
    use crate::decode_image;
    let grey = qr_frame(SAMPLE_URI, 4, 4);
    let (w, h) = (grey.width(), grey.height());

    let f = decode_image(&png(&grey), &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (w, h, PixelLayout::Luma8)
    );
    assert_eq!(f.pixels(), grey.pixels());

    let f = decode_image(&jpeg(&grey, 90), &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (w, h, PixelLayout::Luma8)
    );
    assert_eq!(f.pixels().len(), (w * h) as usize);

    let rgb = rgb_frame(40, 24);
    let f = decode_image(&png(&rgb), &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (40, 24, PixelLayout::Rgb8)
    );
    assert_eq!(f.pixels(), rgb.pixels());

    let f = decode_image(&jpeg(&rgb, 90), &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (40, 24, PixelLayout::Rgb8)
    );
    assert_eq!(f.pixels().len(), 40 * 24 * 3);

    let f = decode_image(&png(&crate::testing::rgba(&grey)), &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (w, h, PixelLayout::Rgba8)
    );
    let f = decode_image(&png(&qr_frame_transparent(SAMPLE_URI, 4)), &DEFAULT).unwrap();
    assert_eq!(f.layout(), PixelLayout::LumaA8);
}

#[test]
fn decode_sixteen_bit_png_narrows_to_high_byte() {
    use crate::decode_image;
    let samples: Vec<u16> = vec![0x0000, 0x1234, 0xABCD, 0xFFFF, 0x8001, 0x00FF];
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&bytes_of(&samples), 3, 2, image::ExtendedColorType::L16)
        .unwrap();
    let f = decode_image(&out, &DEFAULT).unwrap();
    assert_eq!(
        (f.width(), f.height(), f.layout()),
        (3, 2, PixelLayout::Luma8)
    );
    assert_eq!(f.pixels(), [0x00, 0x12, 0xAB, 0xFF, 0x80, 0x00]);
}

#[test]
fn truncated_jpeg() {
    use crate::decode_image;
    let bytes = jpeg(&qr_frame(SAMPLE_URI, 4, 4), 80);
    // Cut inside the headers.
    assert_eq!(
        decode_image(&bytes[..60], &DEFAULT).unwrap_err(),
        ScanError::Unreadable
    );
    // Only the magic bytes.
    assert_eq!(
        decode_image(&bytes[..3], &DEFAULT).unwrap_err(),
        ScanError::Unreadable
    );
}

/// The JPEG with its SOF0 dimensions rewritten to `w` x `h`.
fn jpeg_with_dimensions(mut bytes: Vec<u8>, w: u16, h: u16) -> Vec<u8> {
    let sof = bytes
        .windows(2)
        .position(|m| m == [0xFF, 0xC0])
        .expect("baseline SOF0");
    bytes[sof + 5..sof + 7].copy_from_slice(&h.to_be_bytes());
    bytes[sof + 7..sof + 9].copy_from_slice(&w.to_be_bytes());
    bytes
}

#[test]
fn jpeg_oversize_header_refused_before_allocation() {
    use crate::decode_image;
    let base = jpeg(&qr_frame(SAMPLE_URI, 4, 4), 80);
    // 65535 x 65535 would need 4 GiB; the header alone must be refused.
    let huge = jpeg_with_dimensions(base.clone(), u16::MAX, u16::MAX);
    assert_eq!(
        decode_image(&huge, &DEFAULT).unwrap_err(),
        ScanError::TooLarge
    );
    // Within the side limit but over the pixel budget.
    let wide = jpeg_with_dimensions(base.clone(), 10_000, 10_000);
    assert_eq!(
        decode_image(&wide, &DEFAULT).unwrap_err(),
        ScanError::TooLarge
    );
    // Side, pixel and allocation limits on a genuine image.
    for limits in [
        ImageLimits {
            max_side: 16,
            ..DEFAULT
        },
        ImageLimits {
            max_pixels: 100,
            ..DEFAULT
        },
        ImageLimits {
            max_alloc: 100,
            ..DEFAULT
        },
    ] {
        assert_eq!(
            decode_image(&base, &limits).unwrap_err(),
            ScanError::TooLarge
        );
    }
}

#[test]
fn frame_new_checks() {
    let px = [0u8; 64];
    let l = PixelLayout::Luma8;
    assert!(Frame::new(8, 8, 8, l, &px).is_ok());
    assert!(Frame::new(8, 8, 7, l, &px).is_err(), "stride too small");
    assert!(Frame::new(9, 8, 9, l, &px).is_err(), "buffer short");
    assert!(Frame::new(0, 8, 8, l, &px).is_err(), "zero width");
    assert!(Frame::new(8, 0, 8, l, &px).is_err(), "zero height");
    assert!(
        Frame::new(u32::MAX, u32::MAX, usize::MAX, PixelLayout::Rgba8, &px).is_err(),
        "overflowing dimensions"
    );
    assert!(OwnedFrame::new(8, 9, l, Zeroizing::new(px.to_vec())).is_err());
}

#[test]
fn classify_table() {
    assert_eq!(classify(" OTPAUTH://totp/a?secret=A\n"), Payload::OtpAuth);
    assert_eq!(classify("otpauth-migration://x"), Payload::Migration);
    assert_eq!(classify("OtPaUtH-MiGrAtIoN://x"), Payload::Migration);
    assert_eq!(classify("https://x"), Payload::Other);
    assert_eq!(classify(""), Payload::Other);
}

#[test]
fn debug_redacts() {
    let frame = OwnedFrame::new(2, 1, PixelLayout::Luma8, Zeroizing::new(vec![171, 205])).unwrap();
    let text = format!("{frame:?} {:?}", frame.frame());
    assert!(text.contains("width: 2"));
    assert!(!text.contains("171") && !text.contains("205"));
}

#[test]
fn to_luma_values() {
    use crate::frame::to_luma;
    let one = |layout: PixelLayout, px: &[u8]| {
        // 21x21 repeat so the smallest-QR check passes.
        let bpp = layout.bytes_per_pixel();
        let all: Vec<u8> = px.iter().copied().cycle().take(21 * 21 * bpp).collect();
        let frame = Frame::new(21, 21, 21 * bpp, layout, &all).unwrap();
        let luma = to_luma(&frame, 1 << 20).unwrap();
        luma.pixels[0]
    };
    for layout in [PixelLayout::Rgb8, PixelLayout::Rgba8] {
        let white: &[u8] = if layout == PixelLayout::Rgb8 {
            &[255, 255, 255]
        } else {
            &[255, 255, 255, 255]
        };
        let black: &[u8] = if layout == PixelLayout::Rgb8 {
            &[0, 0, 0]
        } else {
            &[0, 0, 0, 255]
        };
        assert_eq!(one(layout, white), 255);
        assert_eq!(one(layout, black), 0);
    }
    assert_eq!(one(PixelLayout::Rgbx8, &[255, 255, 255, 0]), 255);
    assert_eq!(one(PixelLayout::Bgrx8, &[0, 0, 0, 0]), 0);
    let half = one(PixelLayout::LumaA8, &[0, 128]);
    assert!((127..=129).contains(&half), "{half}");
    let half = one(PixelLayout::Rgba8, &[0, 0, 0, 128]);
    assert!((127..=129).contains(&half), "{half}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn prop_round_trip(
        secret in proptest::collection::vec(any::<u8>(), 10..=64),
        issuer in "[A-Za-z0-9._@-][A-Za-z0-9 ._@-]{0,18}[A-Za-z0-9._@-]",
        account in "[A-Za-z0-9._@-][A-Za-z0-9 ._@-]{0,18}[A-Za-z0-9._@-]",
        digits in 6u8..=8,
        long_period in any::<bool>(),
    ) {
        use mosaic_totp::{Digits, Otp, OtpSecret, Period};
        let auth = OtpAuth {
            otp: Otp {
                secret: OtpSecret::from_bytes(&secret).unwrap(),
                algorithm: Algorithm::Sha1,
                kind: OtpKind::Totp {
                    digits: Digits::new(digits).unwrap(),
                    period: Period::new(if long_period { 60 } else { 30 }).unwrap(),
                },
            },
            issuer: Some(SecretText::new(&issuer)),
            account_name: SecretText::new(&account),
        };
        let uri = auth.to_uri().unwrap();
        let bytes = png(&qr_frame(uri.expose_secret(), 4, 4));
        let scanned = scan_image(&bytes, &DEFAULT).unwrap();
        prop_assert_eq!(scanned, auth);
    }
}

#[test]
#[ignore = "writes fuzz seeds; run with MOSAIC_UPDATE_FIXTURES=1"]
fn write_fuzz_seeds() {
    if std::env::var("MOSAIC_UPDATE_FIXTURES").as_deref() != Ok("1") {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds");
    let image_dir = root.join("scan_image");
    let luma_dir = root.join("scan_luma");
    std::fs::create_dir_all(&image_dir).unwrap();
    std::fs::create_dir_all(&luma_dir).unwrap();
    let write = |dir: &std::path::Path, name: &str, bytes: &[u8]| {
        assert!(bytes.len() <= 8 * 1024, "{name} too large");
        std::fs::write(dir.join(name), bytes).unwrap();
    };
    write(&image_dir, "otp.png", &png(&qr_frame(SAMPLE_URI, 4, 4)));
    write(
        &image_dir,
        "otp.jpg",
        &jpeg(&qr_frame(SAMPLE_URI, 3, 4), 70),
    );
    write(
        &image_dir,
        "url.png",
        &png(&qr_frame("https://example.com", 4, 4)),
    );
    write(
        &image_dir,
        "migration.png",
        &png(&qr_frame(
            "otpauth-migration://offline?data=CgYKBAAAAAA%3D",
            4,
            4,
        )),
    );
    write(
        &image_dir,
        "blank.png",
        &png(&canvas(64, 64, PixelLayout::Luma8, 255, &[])),
    );
    let small = qr_frame(SAMPLE_URI, 2, 4);
    // Layout byte 0 is Luma8, then the width.
    let mut raw = vec![0];
    raw.extend_from_slice(&u16::try_from(small.width()).unwrap().to_le_bytes());
    raw.extend_from_slice(small.pixels());
    write(&luma_dir, "otp.luma", &raw);
}

/// A white greyscale frame tiled with `n` x `n` finder squares (7 modules of
/// `module` px) and no timing pattern, so no grid is valid.
fn finder_field(n: u32, module: u32, gap: u32) -> OwnedFrame {
    let pitch = 7 * module + gap;
    let side = n * pitch + gap;
    let mut pixels = vec![255_u8; (side * side) as usize];
    for y in 0..side {
        for x in 0..side {
            let (lx, ly) = (x.wrapping_sub(gap) % pitch, y.wrapping_sub(gap) % pitch);
            if x < gap || y < gap || lx >= 7 * module || ly >= 7 * module {
                continue;
            }
            let (mx, my) = (lx / module, ly / module);
            let outer = mx == 0 || mx == 6 || my == 0 || my == 6;
            let core = (2..=4).contains(&mx) && (2..=4).contains(&my);
            if outer || core {
                pixels[(y * side + x) as usize] = 0;
            }
        }
    }
    OwnedFrame::new(side, side, PixelLayout::Luma8, Zeroizing::new(pixels)).unwrap()
}

#[test]
fn fake_finder_field_is_bounded() {
    // 900 capstones: over the patched rqrr's capstone cap.
    // 8 x 8 = 64: under it, so the attempt cap is what ends the search.
    for n in [30, 8] {
        let started = std::time::Instant::now();
        let result = scan_one(&finder_field(n, 4, 10));
        assert_eq!(result.unwrap_err(), ScanError::NotFound);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(15),
            "scan of {n}x{n} fake finders took {:?}",
            started.elapsed()
        );
    }
}

const SETUP_CODE: &str = "mosaic-setup:1:eyJ2IjoxfQ";

fn scan_setup(frame: &OwnedFrame) -> Result<Zeroizing<String>, ScanError> {
    scan_setup_frames(&[frame.frame()])
}

#[test]
fn setup_payload_is_classified_and_returned() {
    // The code comes back as it was read (the parser of `mosaic-core` checks it).
    let qr = qr_frame(SETUP_CODE, 5, 4);
    assert_eq!(scan_setup(&qr).unwrap().as_str(), SETUP_CODE);
    // Prefix case and surrounding whitespace do not matter to the scanner.
    let qr = qr_frame(" MOSAIC-SETUP:1:eyJ2IjoxfQ\n", 5, 4);
    assert_eq!(
        scan_setup(&qr).unwrap().as_str(),
        "MOSAIC-SETUP:1:eyJ2IjoxfQ"
    );
    // A white-on-black code too.
    let qr = inverted(&qr_frame(SETUP_CODE, 5, 4));
    assert_eq!(scan_setup(&qr).unwrap().as_str(), SETUP_CODE);
    // It is not an OTP code: the otpauth scan drops it as something else.
    let qr = qr_frame(SETUP_CODE, 5, 4);
    assert_eq!(scan_one(&qr).unwrap_err(), ScanError::NotOtp);
    // Two different setup codes are not guessed between.
    let two = screen(
        &[SETUP_CODE, "mosaic-setup:1:eyJ2IjoyfQ"],
        PixelLayout::Rgbx8,
    );
    assert_eq!(scan_setup(&two).unwrap_err(), ScanError::Multiple);
}

#[test]
fn scan_drops_other_payloads_unread() {
    for other in [
        SAMPLE_URI,
        "otpauth-migration://offline?data=CgYKBAAAAAA%3D",
        "https://example.com",
        "mosaic-setup",
    ] {
        let qr = qr_frame(other, 5, 4);
        assert_eq!(
            scan_setup(&qr).unwrap_err(),
            ScanError::NotSetupCode,
            "{other}"
        );
    }
    // An otpauth code beside a setup code is dropped, not mixed in.
    let both = screen(&[SAMPLE_URI, SETUP_CODE], PixelLayout::Rgbx8);
    assert_eq!(scan_setup(&both).unwrap().as_str(), SETUP_CODE);
    // A setup code that is too long is something else.
    let long = format!("mosaic-setup:1:{}", "A".repeat(1100));
    let qr = qr_frame(&long, 3, 4);
    assert_eq!(scan_setup(&qr).unwrap_err(), ScanError::NotSetupCode);
    // The otpauth scan still ignores a setup code beside the OTP code.
    assert_sample(&scan_one(&both).unwrap());
}
