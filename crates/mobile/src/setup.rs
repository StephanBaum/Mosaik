//! The setup code (CRYPTO_SPEC §6.9.7, ADR-0096): the QR code a desktop shows
//! so a phone can join without typing. The format and its strict parser are in
//! `mosaic_core::setup_code` (with the sync origin and email rules in
//! `mosaic_sync::setup_code`); this module reads it from typed text or from a
//! camera frame and hands the Secret Key out as a [`SecretValue`].

use std::sync::Arc;

use mosaic_core::Error as CoreError;
use mosaic_qr::{Frame, PixelLayout, ScanError};
use zeroize::Zeroizing;

use crate::Core;
use crate::error::{MobileError, Result};
use crate::otp::LumaFrame;
use crate::secret::{SecretValue, text_of};

/// A parsed setup code.
pub struct SetupCode {
    /// The server's canonical origin.
    pub server: String,
    /// The account email, normalized.
    pub email: String,
    /// The Secret Key in its display form (`M1-…`), as text bytes.
    pub secret_key: Arc<SecretValue>,
}

fn setup_error(e: CoreError) -> MobileError {
    match e {
        CoreError::SecretKeyMistyped => MobileError::SecretKeyMistyped,
        _ => MobileError::SetupCodeInvalid,
    }
}

impl Core {
    fn setup_code_of(&self, bytes: &[u8]) -> Result<SetupCode> {
        let code = mosaic_sync::setup_code::parse(bytes).map_err(setup_error)?;
        let display = code.secret_key.to_display()?;
        Ok(SetupCode {
            server: code.server,
            email: code.email,
            secret_key: self.issue(SecretValue::from_text(display.expose_secret())),
        })
    }

    /// Parses the text of a setup code (typed, pasted or read by the OS
    /// scanner), strictly.
    ///
    /// # Errors
    /// `setupCodeInvalid` (any deviation from the format),
    /// `secretKeyMistyped` (the checksum of the Secret Key is wrong),
    /// `invalidInput` (not UTF-8).
    pub fn parse_setup_code(&self, text: Vec<u8>) -> Result<SetupCode> {
        let text = text_of(text)?;
        self.setup_code_of(text.trim_ascii().as_bytes())
    }

    /// Looks for the one setup code in a camera frame. `None`: no QR code, or
    /// only others (they are dropped unread). The pixels are wiped.
    ///
    /// # Errors
    /// `setupCodeInvalid` (a setup code that does not parse, or two different
    /// ones), `secretKeyMistyped`, `invalidInput` (a malformed frame).
    pub fn scan_setup_qr(&self, frame: LumaFrame) -> Result<Option<SetupCode>> {
        let LumaFrame {
            width,
            height,
            stride,
            pixels,
        } = frame;
        let pixels = Zeroizing::new(pixels);
        let stride = usize::try_from(stride).map_err(|_| MobileError::InvalidInput)?;
        let frame = Frame::new(width, height, stride, PixelLayout::Luma8, &pixels)
            .map_err(|_| MobileError::InvalidInput)?;
        match mosaic_qr::scan_setup_frames(&[frame]) {
            Ok(text) => self.setup_code_of(text.as_bytes()).map(Some),
            Err(ScanError::NotFound | ScanError::NotSetupCode) => Ok(None),
            Err(ScanError::Multiple) => Err(MobileError::SetupCodeInvalid),
            Err(_) => Err(MobileError::InvalidInput),
        }
    }
}
