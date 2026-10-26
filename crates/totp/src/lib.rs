//! Mosaic one-time passwords: HOTP (RFC 4226), TOTP (RFC 6238), Steam
//! Guard, and `otpauth://` URIs (CRYPTO_SPEC §13).
//!
//! - **Secrets** ([`OtpSecret`]) and secret-bearing text ([`SecretText`],
//!   [`Code`]) zeroize on drop, are not `Clone` and print as `[REDACTED]`.
//! - **Codes**: [`hotp`], [`totp`], [`steam_guard`], or [`Otp::code_at`]
//!   for a stored configuration. HMAC-SHA-1/256/512 from RustCrypto, 6 to 8
//!   digits ([`Digits`]), period 1 to 300 s ([`Period`]), `T0 = 0`. Steam
//!   Guard codes are 5 characters from [`STEAM_ALPHABET`].
//! - **Verification**: [`Otp::verify`] checks user input in constant time
//!   against a bounded window of steps and rejects steps at or before the
//!   last accepted one (replay guard).
//! - **Time**: callers pass Unix seconds, so codes are deterministic and
//!   testable; [`unix_time_now`] reads the system clock, and
//!   [`Period::seconds_remaining`] / [`Otp::seconds_remaining`] drive a
//!   countdown.
//! - **URIs**: [`OtpAuth::parse`] and [`OtpAuth::to_uri`] (see [`uri`] for
//!   the exact rules and limits).
//!
//! ```
//! use mosaic_totp::{OtpAuth, unix_time_now};
//!
//! # fn main() -> Result<(), mosaic_totp::Error> {
//! let entry = OtpAuth::parse(
//!     "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example",
//! )?;
//! let now = unix_time_now()?;
//! let code = entry.otp.code_at(now)?;
//! assert_eq!(code.len(), 6);
//! assert!(entry.otp.seconds_remaining(now).is_some_and(|s| (1..=30).contains(&s)));
//! # Ok(())
//! # }
//! ```
#![forbid(unsafe_code)]

mod base32;
mod error;
mod otp;
mod secret;
pub mod uri;

pub use error::{Error, Result};
pub use otp::{
    Algorithm, Code, Digits, MAX_VERIFY_WINDOW, Otp, OtpKind, Period, STEAM_ALPHABET,
    STEAM_CODE_LEN, hotp, steam_guard, totp, unix_time_now,
};
pub use secret::{
    COMMON_MIN_SECRET_LEN, MAX_SECRET_LEN, MAX_SECRET_TEXT_LEN, OtpSecret, RFC4226_MIN_SECRET_LEN,
    SecretStrength, SecretText,
};
pub use uri::{MAX_PARAMS, MAX_TEXT_LEN, MAX_URI_LEN, OtpAuth};
