//! Fixture generators for tests (feature `testing`): QR frames, screen-like
//! canvases and PNG/JPEG encoders. Never linked into the app.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "test fixture generators"
)]

use image::{ExtendedColorType, ImageEncoder};
use qrcodegen::{QrCode, QrCodeEcc};
use zeroize::Zeroizing;

use crate::frame::{OwnedFrame, PixelLayout};

/// A valid TOTP setup URI.
pub const SAMPLE_URI: &str =
    "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";

fn modules(payload: &str) -> QrCode {
    QrCode::encode_text(payload, QrCodeEcc::Medium).expect("payload fits a QR code")
}

fn dim(n: i32) -> u32 {
    u32::try_from(n).expect("non-negative")
}

/// Black modules on white, `quiet` modules of border, `module_px` pixels per
/// module, `Luma8`.
#[must_use]
pub fn qr_frame(payload: &str, module_px: u32, quiet: u32) -> OwnedFrame {
    let code = modules(payload);
    let side = dim(code.size());
    let total = (side + 2 * quiet) * module_px;
    let mut px = Vec::with_capacity((total * total) as usize);
    for y in 0..total {
        for x in 0..total {
            let mx = i64::from(x / module_px) - i64::from(quiet);
            let my = i64::from(y / module_px) - i64::from(quiet);
            let dark = (0..i64::from(side)).contains(&mx)
                && (0..i64::from(side)).contains(&my)
                && code.get_module(
                    i32::try_from(mx).expect("small"),
                    i32::try_from(my).expect("small"),
                );
            px.push(if dark { 0 } else { 255 });
        }
    }
    OwnedFrame::new(total, total, PixelLayout::Luma8, Zeroizing::new(px)).expect("valid frame")
}

/// The same code as `LumaA8`: dark modules opaque black, light modules fully
/// transparent.
#[must_use]
pub fn qr_frame_transparent(payload: &str, module_px: u32) -> OwnedFrame {
    let plain = qr_frame(payload, module_px, 4);
    let mut px = Vec::with_capacity(plain.pixels().len() * 2);
    for &v in plain.pixels() {
        px.push(0);
        px.push(if v == 0 { 255 } else { 0 });
    }
    OwnedFrame::new(
        plain.width(),
        plain.height(),
        PixelLayout::LumaA8,
        Zeroizing::new(px),
    )
    .expect("valid frame")
}

/// An opaque `Rgba8` copy of a `Luma8` frame (what the clipboard hands over).
#[must_use]
pub fn rgba(frame: &OwnedFrame) -> OwnedFrame {
    let px = frame
        .pixels()
        .iter()
        .flat_map(|&v| [v, v, v, 255])
        .collect();
    OwnedFrame::new(
        frame.width(),
        frame.height(),
        PixelLayout::Rgba8,
        Zeroizing::new(px),
    )
    .expect("valid frame")
}

/// White-on-black version of a `Luma8` frame.
#[must_use]
pub fn inverted(frame: &OwnedFrame) -> OwnedFrame {
    let px = frame.pixels().iter().map(|v| 255 - v).collect();
    OwnedFrame::new(
        frame.width(),
        frame.height(),
        frame.layout(),
        Zeroizing::new(px),
    )
    .expect("valid frame")
}

/// A `background`-grey canvas in `layout` (`Rgbx8` or `Bgrx8`, 4th byte 0, or
/// `Luma8`) with `Luma8` frames pasted at (x, y).
#[must_use]
pub fn canvas(
    width: u32,
    height: u32,
    layout: PixelLayout,
    background: u8,
    place: &[(&OwnedFrame, u32, u32)],
) -> OwnedFrame {
    let bpp = layout.bytes_per_pixel();
    let mut px = vec![0u8; (width as usize) * (height as usize) * bpp];
    for p in px.chunks_exact_mut(bpp) {
        for b in p.iter_mut().take(3.min(bpp)) {
            *b = background;
        }
    }
    for (frame, x0, y0) in place {
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let (cx, cy) = (x0 + x, y0 + y);
                if cx >= width || cy >= height {
                    continue;
                }
                let v = frame.pixels()[(y * frame.width() + x) as usize];
                let at = ((cy * width + cx) as usize) * bpp;
                for b in px[at..at + bpp].iter_mut().take(3.min(bpp)) {
                    *b = v;
                }
            }
        }
    }
    OwnedFrame::new(width, height, layout, Zeroizing::new(px)).expect("valid frame")
}

fn color_type(layout: PixelLayout) -> ExtendedColorType {
    match layout {
        PixelLayout::Luma8 => ExtendedColorType::L8,
        PixelLayout::LumaA8 => ExtendedColorType::La8,
        PixelLayout::Rgb8 => ExtendedColorType::Rgb8,
        PixelLayout::Rgba8 | PixelLayout::Rgbx8 | PixelLayout::Bgrx8 => ExtendedColorType::Rgba8,
    }
}

/// PNG bytes of a frame.
#[must_use]
pub fn png(frame: &OwnedFrame) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            frame.pixels(),
            frame.width(),
            frame.height(),
            color_type(frame.layout()),
        )
        .expect("png encode");
    out
}

/// JPEG bytes of a `Luma8` frame.
#[must_use]
pub fn jpeg(frame: &OwnedFrame, quality: u8) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .write_image(
            frame.pixels(),
            frame.width(),
            frame.height(),
            color_type(frame.layout()),
        )
        .expect("jpeg encode");
    out
}
