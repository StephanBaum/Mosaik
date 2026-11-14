//! Scan errors.

/// Why a scan found no usable code. Carries no input data (pixels or QR content
/// may hold the secret).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ScanError {
    /// No QR code in the image.
    #[error("no QR code found")]
    NotFound,
    /// A QR code, but not an `otpauth://` URI.
    #[error("QR code is not an otpauth URI")]
    NotOtp,
    /// A Google Authenticator export code.
    #[error("Google Authenticator export")]
    Migration,
    /// QR codes, but none of them a Mosaic setup code (M10).
    #[error("QR code is not a Mosaic setup code")]
    NotSetupCode,
    /// Several different `otpauth://` codes (or setup codes).
    #[error("several different otpauth codes")]
    Multiple,
    /// An `otpauth://` URI that `mosaic-totp` refuses.
    #[error("otpauth URI not usable")]
    InvalidOtp,
    /// Not a readable PNG or JPEG, or a malformed frame.
    #[error("image unreadable")]
    Unreadable,
    /// Over one of the size limits.
    #[error("image too large")]
    TooLarge,
}
