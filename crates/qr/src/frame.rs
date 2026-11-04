//! Pixel frames and the grey-level conversion.

use core::fmt;
use zeroize::Zeroizing;

use crate::error::ScanError;

/// Pixel byte layout of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelLayout {
    /// 1 byte per pixel.
    Luma8,
    /// 2 bytes, straight alpha: composited over white.
    LumaA8,
    /// 3 bytes.
    Rgb8,
    /// 4 bytes, straight alpha: composited over white (PNG, clipboard).
    Rgba8,
    /// 4 bytes, 4th byte ignored (GDI screen capture: alpha is 0).
    Rgbx8,
    /// 4 bytes, 4th byte ignored (X11 `ZPixmap`, LSB first, depth 24/32).
    Bgrx8,
}

impl PixelLayout {
    /// Bytes per pixel.
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Luma8 => 1,
            Self::LumaA8 => 2,
            Self::Rgb8 => 3,
            Self::Rgba8 | Self::Rgbx8 | Self::Bgrx8 => 4,
        }
    }
}

/// Checks the geometry and returns the minimum buffer length.
fn required_len(
    width: u32,
    height: u32,
    stride: usize,
    layout: PixelLayout,
) -> Result<usize, ScanError> {
    if width == 0 || height == 0 {
        return Err(ScanError::Unreadable);
    }
    let row = usize::try_from(width)
        .ok()
        .and_then(|w| w.checked_mul(layout.bytes_per_pixel()))
        .ok_or(ScanError::Unreadable)?;
    if stride < row {
        return Err(ScanError::Unreadable);
    }
    let rows = usize::try_from(height - 1).map_err(|_| ScanError::Unreadable)?;
    rows.checked_mul(stride)
        .and_then(|n| n.checked_add(row))
        .ok_or(ScanError::Unreadable)
}

/// A borrowed frame. `stride` is bytes per row (at least width times bytes per pixel).
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    width: u32,
    height: u32,
    stride: usize,
    layout: PixelLayout,
    pixels: &'a [u8],
}

impl<'a> Frame<'a> {
    /// Wraps pixels.
    ///
    /// # Errors
    /// `Unreadable` if width or height is 0, stride is below width times bytes
    /// per pixel, or `pixels` is shorter than `stride * (height - 1) + width * bpp`
    /// (checked arithmetic).
    pub fn new(
        width: u32,
        height: u32,
        stride: usize,
        layout: PixelLayout,
        pixels: &'a [u8],
    ) -> Result<Self, ScanError> {
        let need = required_len(width, height, stride, layout)?;
        if pixels.len() < need {
            return Err(ScanError::Unreadable);
        }
        Ok(Self {
            width,
            height,
            stride,
            layout,
            pixels,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

impl fmt::Debug for Frame<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

/// An owned frame in a zeroizing buffer (tightly packed rows).
pub struct OwnedFrame {
    width: u32,
    height: u32,
    layout: PixelLayout,
    pixels: Zeroizing<Vec<u8>>,
}

impl OwnedFrame {
    /// Wraps pixels.
    ///
    /// # Errors
    /// As [`Frame::new`] with `stride = width * bpp`.
    pub fn new(
        width: u32,
        height: u32,
        layout: PixelLayout,
        pixels: Zeroizing<Vec<u8>>,
    ) -> Result<Self, ScanError> {
        let stride = usize::try_from(width)
            .ok()
            .and_then(|w| w.checked_mul(layout.bytes_per_pixel()))
            .ok_or(ScanError::Unreadable)?;
        let need = required_len(width, height, stride, layout)?;
        if pixels.len() < need {
            return Err(ScanError::Unreadable);
        }
        Ok(Self {
            width,
            height,
            layout,
            pixels,
        })
    }

    /// A borrowed view.
    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.width,
            height: self.height,
            stride: usize::try_from(self.width)
                .unwrap_or(0)
                .saturating_mul(self.layout.bytes_per_pixel()),
            layout: self.layout,
            pixels: &self.pixels,
        }
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Pixel layout.
    #[must_use]
    pub const fn layout(&self) -> PixelLayout {
        self.layout
    }

    /// The pixel bytes (fixture generators).
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl fmt::Debug for OwnedFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedFrame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

/// Grey levels, row-major, zeroized on drop.
pub(crate) struct Luma {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Zeroizing<Vec<u8>>,
}

/// Grey level of an RGB triple.
fn grey(r: u8, g: u8, b: u8) -> u32 {
    (77 * u32::from(r) + 150 * u32::from(g) + 29 * u32::from(b) + 128) >> 8
}

/// Grey level `y` with straight alpha `a`, composited over white.
fn over_white(y: u32, a: u32) -> u32 {
    (y * a + 255 * (255 - a) + 127) / 255
}

/// Grey level of one pixel's bytes (`px` is `bytes_per_pixel` long).
fn pixel_luma(layout: PixelLayout, px: &[u8]) -> u32 {
    let at = |i: usize| px.get(i).copied().unwrap_or(0);
    match layout {
        PixelLayout::Luma8 => u32::from(at(0)),
        PixelLayout::LumaA8 => over_white(u32::from(at(0)), u32::from(at(1))),
        PixelLayout::Rgb8 | PixelLayout::Rgbx8 => grey(at(0), at(1), at(2)),
        PixelLayout::Rgba8 => over_white(grey(at(0), at(1), at(2)), u32::from(at(3))),
        PixelLayout::Bgrx8 => grey(at(2), at(1), at(0)),
    }
}

/// Smallest QR code side in modules.
const MIN_SIDE: usize = 21;

/// Converts to grey levels and box-downscales by the smallest integer factor
/// `k >= 1` with `(width / k) * (height / k) <= max_pixels` (integer division;
/// leftover edge columns and rows are dropped). Grey is
/// `y = (77r + 150g + 29b + 128) >> 8`. Alpha layouts composite over white:
/// `y' = (y*a + 255*(255 - a) + 127) / 255`. Each output pixel is the rounded
/// mean of its k by k block. The output buffer is allocated once.
///
/// # Errors
/// `NotFound` if the downscaled frame is smaller than 21 by 21 (smallest QR).
pub(crate) fn to_luma(frame: &Frame<'_>, max_pixels: u64) -> Result<Luma, ScanError> {
    let w = usize::try_from(frame.width).map_err(|_| ScanError::Unreadable)?;
    let h = usize::try_from(frame.height).map_err(|_| ScanError::Unreadable)?;
    let mut k = 1usize;
    while (w / k) as u64 * (h / k) as u64 > max_pixels {
        k += 1;
    }
    let (ow, oh) = (w / k, h / k);
    if ow < MIN_SIDE || oh < MIN_SIDE {
        return Err(ScanError::NotFound);
    }
    let bpp = frame.layout.bytes_per_pixel();
    let block = u32::try_from(k * k).map_err(|_| ScanError::Unreadable)?;
    let mut out = Zeroizing::new(Vec::with_capacity(ow * oh));
    for oy in 0..oh {
        for ox in 0..ow {
            let mut sum = 0u32;
            for dy in 0..k {
                let row = (oy * k + dy) * frame.stride + ox * k * bpp;
                for dx in 0..k {
                    let start = row + dx * bpp;
                    let px = frame.pixels.get(start..start + bpp).unwrap_or(&[]);
                    sum += pixel_luma(frame.layout, px);
                }
            }
            let mean = (sum + block / 2) / block;
            out.push(u8::try_from(mean.min(255)).unwrap_or(255));
        }
    }
    Ok(Luma {
        width: ow,
        height: oh,
        pixels: out,
    })
}
