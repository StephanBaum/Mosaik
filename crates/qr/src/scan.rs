//! QR search over frames and payload classification.

use mosaic_totp::{OtpAuth, SecretText};
use zeroize::Zeroizing;

use crate::decode::{ImageLimits, decode_image};
use crate::error::ScanError;
use crate::frame::{Frame, Luma, OwnedFrame, to_luma};

/// The start of a Mosaic setup code (CRYPTO_SPEC §6.9.7), matched ASCII
/// case-insensitively like the other prefixes here; the setup code parser
/// itself is strict.
const SETUP_PREFIX: &str = "mosaic-setup:";
/// The longest setup code kept, in bytes (`mosaic_core::setup_code::MAX_SETUP_CODE_LEN`).
const MAX_SETUP_LEN: usize = 1024;

/// Frames examined per scan; more are ignored.
pub const MAX_FRAMES: usize = 8;
/// QR codes decoded per frame.
pub const MAX_GRIDS_PER_FRAME: usize = 16;
/// Grey-level pixels per frame handed to QR detection (2^23, at least 3840 by 2160).
pub const MAX_DECODE_PIXELS: u64 = 8_388_608;

/// What a QR payload is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payload {
    /// An `otpauth://` URI.
    OtpAuth,
    /// An `otpauth-migration://` URI (Google Authenticator export).
    Migration,
    /// Anything else.
    Other,
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// Classifies a payload after trimming ASCII whitespace; prefixes compare ASCII
/// case-insensitively: `otpauth-migration://` is `Migration` (checked first),
/// `otpauth://` is `OtpAuth`, anything else `Other`.
#[must_use]
pub fn classify(payload: &str) -> Payload {
    let text = payload.trim_ascii();
    if starts_with_ignore_case(text, "otpauth-migration://") {
        Payload::Migration
    } else if starts_with_ignore_case(text, "otpauth://") {
        Payload::OtpAuth
    } else {
        Payload::Other
    }
}

/// What a scan is looking for.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Want {
    /// One `otpauth://` URI.
    #[default]
    Otp,
    /// One Mosaic setup code.
    Setup,
}

/// What the grids of the frames held so far.
#[derive(Default)]
struct Found {
    want: Want,
    otp: Option<SecretText>,
    setup: Option<Zeroizing<String>>,
    setup_distinct: bool,
    migration: bool,
    other: bool,
}

impl Found {
    /// Records one decoded payload.
    ///
    /// # Errors
    /// `Multiple` if it is an `otpauth://` URI different from the one kept.
    fn record(&mut self, payload: &[u8]) -> Result<(), ScanError> {
        let Ok(text) = core::str::from_utf8(payload) else {
            self.other = true;
            return Ok(());
        };
        if self.want == Want::Setup {
            self.record_setup(text);
            return Ok(());
        }
        match classify(text) {
            Payload::OtpAuth => {
                let trimmed = text.trim_ascii();
                match &self.otp {
                    Some(kept) if kept.expose_secret() != trimmed => Err(ScanError::Multiple),
                    Some(_) => Ok(()),
                    None => {
                        self.otp = Some(SecretText::new(trimmed));
                        Ok(())
                    }
                }
            }
            Payload::Migration => {
                self.migration = true;
                Ok(())
            }
            Payload::Other => {
                self.other = true;
                Ok(())
            }
        }
    }

    /// Setup-code mode: keeps the one setup code, drops everything else unread.
    fn record_setup(&mut self, text: &str) {
        let trimmed = text.trim_ascii();
        if starts_with_ignore_case(trimmed, SETUP_PREFIX) && trimmed.len() <= MAX_SETUP_LEN {
            match &self.setup {
                Some(kept) if kept.as_str() != trimmed => self.setup_distinct = true,
                Some(_) => {}
                None => self.setup = Some(Zeroizing::new(trimmed.to_owned())),
            }
        } else {
            self.other = true;
        }
    }

    fn finish_setup(self) -> Result<Zeroizing<String>, ScanError> {
        if self.setup_distinct {
            return Err(ScanError::Multiple);
        }
        match self.setup {
            Some(code) => Ok(code),
            None if self.other => Err(ScanError::NotSetupCode),
            None => Err(ScanError::NotFound),
        }
    }

    fn finish(self) -> Result<OtpAuth, ScanError> {
        if let Some(text) = self.otp {
            return OtpAuth::parse(text.expose_secret()).map_err(|_| ScanError::InvalidOtp);
        }
        if self.migration {
            Err(ScanError::Migration)
        } else if self.other {
            Err(ScanError::NotOtp)
        } else {
            Err(ScanError::NotFound)
        }
    }
}

/// Detects and decodes the grids of one luma image, optionally inverted.
fn scan_luma(luma: &Luma, invert: bool, found: &mut Found) -> Result<bool, ScanError> {
    let (w, h) = (luma.width, luma.height);
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
        let v = luma.pixels.get(y * w + x).copied().unwrap_or(255);
        if invert { 255 - v } else { v }
    });
    let grids = prepared.detect_grids();
    for grid in grids.iter().take(MAX_GRIDS_PER_FRAME) {
        let mut buf: Zeroizing<Vec<u8>> =
            Zeroizing::new(Vec::with_capacity(rqrr::MAX_PAYLOAD_SIZE));
        if grid.decode_to(&mut *buf).is_ok() {
            found.record(&buf)?;
        }
    }
    Ok(!grids.is_empty())
}

/// Finds the one `otpauth://` code in up to [`MAX_FRAMES`] frames (more are
/// ignored).
///
/// Per frame: grey levels (downscaled to [`MAX_DECODE_PIXELS`]), a first pass,
/// and, if it finds no grid, a second pass on the inverted levels
/// (white-on-black codes). At most [`MAX_GRIDS_PER_FRAME`] grids are decoded.
/// An `otpauth://` payload is kept in a zeroizing text; a later one that
/// differs is `Multiple`, an equal one counts once. Other payloads are
/// remembered as seen but never kept. The result is the kept URI parsed, else
/// `Migration`, `NotOtp` or `NotFound`.
///
/// # Errors
/// `NotFound`, `NotOtp`, `Migration`, `Multiple`, `InvalidOtp`.
pub fn scan_frames(frames: &[Frame<'_>]) -> Result<OtpAuth, ScanError> {
    let lumas: Vec<Luma> = frames
        .iter()
        .take(MAX_FRAMES)
        .filter_map(|frame| to_luma(frame, MAX_DECODE_PIXELS).ok())
        .collect();
    scan_lumas(&lumas)
}

/// [`scan_frames`] on frames it owns: each is dropped (zeroized) as soon as
/// its grey levels exist, so the raw screen pixels are gone before the search
/// starts.
///
/// # Errors
/// As [`scan_frames`].
pub fn scan_owned_frames(frames: Vec<OwnedFrame>) -> Result<OtpAuth, ScanError> {
    let mut lumas = Vec::new();
    for frame in frames.into_iter().take(MAX_FRAMES) {
        if let Ok(luma) = to_luma(&frame.frame(), MAX_DECODE_PIXELS) {
            lumas.push(luma);
        }
    }
    scan_lumas(&lumas)
}

fn scan_lumas(lumas: &[Luma]) -> Result<OtpAuth, ScanError> {
    search(lumas, Found::default())?.finish()
}

fn search(lumas: &[Luma], mut found: Found) -> Result<Found, ScanError> {
    for luma in lumas {
        if !scan_luma(luma, false, &mut found)? {
            scan_luma(luma, true, &mut found)?;
        }
    }
    Ok(found)
}

/// Finds the one Mosaic setup code (`mosaic-setup:` ...) in up to
/// [`MAX_FRAMES`] frames, the same way [`scan_frames`] finds an `otpauth://`
/// code. The mobile scanner hands it camera frames (grey levels).
///
/// Every other payload is dropped unread: it is never kept, parsed or
/// reported, only counted as "something else was there". The code is kept in
/// a zeroizing string and is **not** parsed here (`mosaic_core::setup_code`
/// does that); two different setup codes are `Multiple`.
///
/// # Errors
/// `NotFound` (no QR code), `NotSetupCode` (QR codes, none of them a setup
/// code), `Multiple`.
pub fn scan_setup_frames(frames: &[Frame<'_>]) -> Result<Zeroizing<String>, ScanError> {
    let lumas: Vec<Luma> = frames
        .iter()
        .take(MAX_FRAMES)
        .filter_map(|frame| to_luma(frame, MAX_DECODE_PIXELS).ok())
        .collect();
    search(
        &lumas,
        Found {
            want: Want::Setup,
            ..Found::default()
        },
    )?
    .finish_setup()
}

/// [`decode_image`] then [`scan_frames`] on the one frame; the decoded frame is
/// dropped (zeroized) before returning.
///
/// # Errors
/// As both.
pub fn scan_image(bytes: &[u8], limits: &ImageLimits) -> Result<OtpAuth, ScanError> {
    let owned = decode_image(bytes, limits)?;
    scan_frames(&[owned.frame()])
}
