//! Bounded decoding of untrusted PNG and JPEG bytes.

use std::io::Cursor;

use image::{ColorType, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits};
use zeroize::Zeroizing;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;
use zune_jpeg::errors::DecodeErrors;

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

fn map_jpeg_error(error: &DecodeErrors) -> ScanError {
    if matches!(error, DecodeErrors::LargeDimensions(_)) {
        ScanError::TooLarge
    } else {
        ScanError::Unreadable
    }
}

/// Side, pixel and byte ceilings, checked from the header before any buffer
/// exists.
fn check_size(
    width: u64,
    height: u64,
    total_bytes: u64,
    limits: &ImageLimits,
) -> Result<(), ScanError> {
    if width > u64::from(limits.max_side)
        || height > u64::from(limits.max_side)
        || width.saturating_mul(height) > limits.max_pixels
        || total_bytes > limits.max_alloc
    {
        return Err(ScanError::TooLarge);
    }
    Ok(())
}

/// Reserves exactly `len` zeroed bytes in a buffer that is wiped on drop, so a
/// failed decode cannot leave partial pixels behind. The caller checked `len`
/// against the limits first; an allocation failure is `TooLarge`.
fn zeroing_buffer(len: u64) -> Result<Zeroizing<Vec<u8>>, ScanError> {
    let len = usize::try_from(len).map_err(|_| ScanError::TooLarge)?;
    let mut buffer = Zeroizing::new(Vec::new());
    buffer
        .try_reserve_exact(len)
        .map_err(|_| ScanError::TooLarge)?;
    buffer.resize(len, 0);
    Ok(buffer)
}

/// Moves native-endian 16-bit samples into a zeroizing 8-bit buffer (high
/// byte); the caller's wide buffer is wiped when it drops.
fn narrow(wide: &[u8]) -> Result<Zeroizing<Vec<u8>>, ScanError> {
    let mut out = zeroing_buffer((wide.len() / 2) as u64)?;
    for (dst, src) in out.iter_mut().zip(wide.as_chunks::<2>().0) {
        *dst = (u16::from_ne_bytes(*src) >> 8) as u8;
    }
    Ok(out)
}

/// Decodes a PNG or JPEG into an owned frame.
///
/// 1. More than `max_file_bytes` is `TooLarge`.
/// 2. The format comes from the magic bytes only; anything else is `Unreadable`.
/// 3. The header is read first. Side, `max_pixels` and `max_alloc` are checked
///    from it before any pixel buffer is allocated; a violation is `TooLarge`,
///    any other decoder error `Unreadable`.
/// 4. JPEG runs `zune-jpeg` directly over the borrowed input, so the plaintext
///    is not copied; PNG runs one `image` decoder over the same borrow.
/// 5. The decoder writes into one buffer reserved here that is wiped on drop,
///    also when the decode fails. 16-bit samples are narrowed once into a
///    second zeroizing buffer.
///
/// Decoder scratch (the PNG inflate window and row buffers, the zune-jpeg
/// coefficient planes) is ordinary heap memory that the decoder crates do not
/// wipe.
///
/// # Errors
/// `TooLarge`, `Unreadable`.
pub fn decode_image(bytes: &[u8], limits: &ImageLimits) -> Result<OwnedFrame, ScanError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(ScanError::TooLarge);
    }
    match format_of(bytes).ok_or(ScanError::Unreadable)? {
        ImageFormat::Jpeg => decode_jpeg(bytes, limits),
        format => decode_generic(bytes, format, limits),
    }
}

fn decode_jpeg(bytes: &[u8], limits: &ImageLimits) -> Result<OwnedFrame, ScanError> {
    // zune reports its own side limit as a format error, so the limits here are
    // lifted and `check_size` enforces ours (as `TooLarge`) before any allocation.
    let options = DecoderOptions::default()
        .set_strict_mode(false)
        .set_max_width(usize::MAX)
        .set_max_height(usize::MAX);
    // zune fixes the colour conversion when it reads the headers, so a
    // header-only probe finds the input colorspace first. It copies no pixels.
    let mut probe = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    probe.decode_headers().map_err(|e| map_jpeg_error(&e))?;
    let (probe_width, probe_height) = probe.dimensions().ok_or(ScanError::Unreadable)?;
    check_size(probe_width as u64, probe_height as u64, 0, limits)?;
    let (layout, out_colorspace) = match probe.input_colorspace() {
        Some(ColorSpace::Luma) => (PixelLayout::Luma8, ColorSpace::Luma),
        Some(_) => (PixelLayout::Rgb8, ColorSpace::RGB),
        None => return Err(ScanError::Unreadable),
    };
    drop(probe);

    let options = options.jpeg_set_out_colorspace(out_colorspace);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(|e| map_jpeg_error(&e))?;
    let (width, height) = decoder.dimensions().ok_or(ScanError::Unreadable)?;
    let total = decoder.output_buffer_size().ok_or(ScanError::TooLarge)?;
    check_size(width as u64, height as u64, total as u64, limits)?;
    let mut pixels = zeroing_buffer(total as u64)?;
    decoder
        .decode_into(&mut pixels)
        .map_err(|e| map_jpeg_error(&e))?;
    let width = u32::try_from(width).map_err(|_| ScanError::TooLarge)?;
    let height = u32::try_from(height).map_err(|_| ScanError::TooLarge)?;
    OwnedFrame::new(width, height, layout, pixels)
}

fn decode_generic(
    bytes: &[u8],
    format: ImageFormat,
    limits: &ImageLimits,
) -> Result<OwnedFrame, ScanError> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(decoder_limits(limits));
    let decoder = reader.into_decoder().map_err(|e| map_error(&e))?;
    let (width, height) = decoder.dimensions();
    let total = decoder.total_bytes();
    check_size(u64::from(width), u64::from(height), total, limits)?;
    let color = decoder.color_type();
    let mut raw = zeroing_buffer(total)?;
    decoder.read_image(&mut raw).map_err(|e| map_error(&e))?;
    let (layout, pixels) = match color {
        ColorType::L8 => (PixelLayout::Luma8, raw),
        ColorType::La8 => (PixelLayout::LumaA8, raw),
        ColorType::Rgb8 => (PixelLayout::Rgb8, raw),
        ColorType::Rgba8 => (PixelLayout::Rgba8, raw),
        ColorType::L16 => (PixelLayout::Luma8, narrow(&raw)?),
        ColorType::La16 => (PixelLayout::LumaA8, narrow(&raw)?),
        ColorType::Rgb16 => (PixelLayout::Rgb8, narrow(&raw)?),
        ColorType::Rgba16 => (PixelLayout::Rgba8, narrow(&raw)?),
        _ => return Err(ScanError::Unreadable),
    };
    OwnedFrame::new(width, height, layout, pixels)
}
