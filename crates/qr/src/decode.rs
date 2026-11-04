//! Bounded decoding of untrusted PNG and JPEG bytes.

use std::io::Cursor;

use image::{DynamicImage, ImageError, ImageFormat, ImageReader, Limits};
use zeroize::Zeroizing;

use crate::error::ScanError;
use crate::frame::{OwnedFrame, PixelLayout};

/// Bounds for untrusted image bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageLimits {
    /// Largest accepted file.
    pub max_file_bytes: usize,
    /// Largest accepted width or height.
    pub max_side: u32,
    /// Largest accepted width times height.
    pub max_pixels: u64,
    /// Decoder allocation ceiling.
    pub max_alloc: u64,
}

impl ImageLimits {
    /// 20 MiB file, 16384 px per side, 40 MP, 512 MiB of decoder memory.
    pub const DEFAULT: Self = Self {
        max_file_bytes: 20 * 1024 * 1024,
        max_side: 16_384,
        max_pixels: 40_000_000,
        max_alloc: 512 * 1024 * 1024,
    };
}

const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];

fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(PNG_MAGIC) {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(JPEG_MAGIC) {
        Some(ImageFormat::Jpeg)
    } else {
        None
    }
}

fn decoder_limits(limits: &ImageLimits) -> Limits {
    let mut l = Limits::default();
    l.max_image_width = Some(limits.max_side);
    l.max_image_height = Some(limits.max_side);
    l.max_alloc = Some(limits.max_alloc);
    l
}

fn map_error(error: &ImageError) -> ScanError {
    if matches!(error, ImageError::Limits(_)) {
        ScanError::TooLarge
    } else {
        ScanError::Unreadable
    }
}

/// Moves 16-bit samples into a zeroizing 8-bit buffer (high byte), wiping the
/// wide one.
fn narrow(raw: Vec<u16>) -> Zeroizing<Vec<u8>> {
    let raw = Zeroizing::new(raw);
    let mut out = Zeroizing::new(Vec::with_capacity(raw.len()));
    out.extend(raw.iter().map(|v| (v >> 8) as u8));
    out
}

/// Decodes a PNG or JPEG into an owned frame.
///
/// 1. More than `max_file_bytes` is `TooLarge`.
/// 2. The format comes from the magic bytes only; anything else is `Unreadable`.
/// 3. The header is read first (`max_side`, `max_alloc`); width times height
///    above `max_pixels` is `TooLarge`.
/// 4. A second reader with the same limits decodes; a limit error is `TooLarge`,
///    any other error `Unreadable`.
/// 5. The pixel `Vec` moves into a zeroizing buffer without a copy; 16-bit
///    samples are narrowed once and the wide buffer is wiped.
///
/// # Errors
/// `TooLarge`, `Unreadable`.
pub fn decode_image(bytes: &[u8], limits: &ImageLimits) -> Result<OwnedFrame, ScanError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(ScanError::TooLarge);
    }
    let format = format_of(bytes).ok_or(ScanError::Unreadable)?;
    let reader = |bytes| {
        let mut r = ImageReader::with_format(Cursor::new(bytes), format);
        r.limits(decoder_limits(limits));
        r
    };
    let (w, h) = reader(bytes).into_dimensions().map_err(|e| map_error(&e))?;
    if u64::from(w) * u64::from(h) > limits.max_pixels {
        return Err(ScanError::TooLarge);
    }
    let image = reader(bytes).decode().map_err(|e| map_error(&e))?;
    let (width, height) = (image.width(), image.height());
    let (layout, pixels) = match image {
        DynamicImage::ImageLuma8(b) => (PixelLayout::Luma8, Zeroizing::new(b.into_raw())),
        DynamicImage::ImageLumaA8(b) => (PixelLayout::LumaA8, Zeroizing::new(b.into_raw())),
        DynamicImage::ImageRgb8(b) => (PixelLayout::Rgb8, Zeroizing::new(b.into_raw())),
        DynamicImage::ImageRgba8(b) => (PixelLayout::Rgba8, Zeroizing::new(b.into_raw())),
        DynamicImage::ImageLuma16(b) => (PixelLayout::Luma8, narrow(b.into_raw())),
        DynamicImage::ImageLumaA16(b) => (PixelLayout::LumaA8, narrow(b.into_raw())),
        DynamicImage::ImageRgb16(b) => (PixelLayout::Rgb8, narrow(b.into_raw())),
        DynamicImage::ImageRgba16(b) => (PixelLayout::Rgba8, narrow(b.into_raw())),
        _ => return Err(ScanError::Unreadable),
    };
    OwnedFrame::new(width, height, layout, pixels)
}
