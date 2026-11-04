//! Mosaic QR scanning (ADR-0031): images and screen frames to `otpauth://`
//! entries.
//!
//! Pixels (a screen frame, a clipboard image, a decoded PNG or JPEG) are
//! converted to grey levels, searched for QR codes with `rqrr` and the one
//! `otpauth://` payload is parsed by `mosaic-totp`. Everything else found in
//! the image is dropped unread, and errors carry no input data.
//!
//! - **Limits**: [`ImageLimits`] bounds untrusted files (size, side, pixels,
//!   decoder memory); frames are box-downscaled to [`MAX_DECODE_PIXELS`]
//!   before detection; at most [`MAX_FRAMES`] frames and
//!   [`MAX_GRIDS_PER_FRAME`] QR codes per frame are looked at.
//! - **Memory**: pixel, grey-level and payload buffers are zeroizing. The
//!   dependencies (`image`, `rqrr`) keep their own unwiped copies (THREAT_MODEL
//!   R30).
//! - **Several codes**: two different `otpauth://` codes are `Multiple`;
//!   identical ones count once; unrelated QR codes are ignored.
#![forbid(unsafe_code)]

mod decode;
mod error;
mod frame;
mod scan;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod tests;

pub use decode::{ImageLimits, decode_image};
pub use error::ScanError;
pub use frame::{Frame, OwnedFrame, PixelLayout};
pub use scan::{
    MAX_DECODE_PIXELS, MAX_FRAMES, MAX_GRIDS_PER_FRAME, Payload, classify, scan_frames, scan_image,
};
