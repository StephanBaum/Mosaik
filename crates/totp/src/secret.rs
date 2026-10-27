//! Containers for OTP secrets and secret-bearing text (CRYPTO_SPEC §4, §13).
//!
//! Like the containers in `mosaic-core`, every type here zeroizes its memory
//! on drop, is not `Clone`, prints only `"[REDACTED]"` through `Debug` and
//! compares in constant time where equality is offered. Buffers are reserved
//! at their final size up front, so they never reallocate and leave an
//! unzeroized copy behind.

use core::fmt;

use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::base32;
use crate::error::{Error, Result};

const REDACTED: &str = "[REDACTED]";

/// Largest accepted secret, in bytes (1024 bits, the SHA-512 block size).
/// Real-world secrets are 10 to 64 bytes.
pub const MAX_SECRET_LEN: usize = 128;

/// The minimum secret length RFC 4226 §4 R6 requires (128 bits). Shorter
/// secrets (80-bit ones are common) still work; see
/// [`OtpSecret::meets_rfc4226_minimum`].
pub const RFC4226_MIN_SECRET_LEN: usize = 16;

/// The 80-bit secret length that most services (Google, GitHub and many
/// others) issue. Shorter secrets are [`SecretStrength::Weak`].
pub const COMMON_MIN_SECRET_LEN: usize = 10;

/// How a secret's length compares with common practice and RFC 4226, for
/// an import or setup hint. Lengths are not secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecretStrength {
    /// Shorter than [`COMMON_MIN_SECRET_LEN`] (80 bits): a real weakness
    /// worth a warning.
    Weak,
    /// At least 80 bits but below the RFC 4226 minimum of 128 bits
    /// ([`RFC4226_MIN_SECRET_LEN`]); what most services issue, so at most
    /// an informational note.
    BelowRfc4226,
    /// At least 128 bits.
    Rfc4226,
}

/// Longest accepted base32 text for a secret, including whitespace and
/// padding.
pub const MAX_SECRET_TEXT_LEN: usize = 1024;

/// The shared secret `K` of HOTP/TOTP/Steam Guard: 1 to
/// [`MAX_SECRET_LEN`] raw bytes, zeroized on drop.
pub struct OtpSecret(Vec<u8>);

impl OtpSecret {
    /// Copies raw secret bytes. The caller remains responsible for the
    /// source buffer.
    ///
    /// # Errors
    /// [`Error::InvalidSecret`] if `bytes` is empty, [`Error::SizeLimit`] if
    /// it is longer than [`MAX_SECRET_LEN`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        check_len(bytes.len())?;
        let mut v = Vec::with_capacity(bytes.len());
        v.extend_from_slice(bytes);
        Ok(Self(v))
    }

    /// Takes ownership of a buffer that already holds the secret.
    pub(crate) fn from_vec(v: Vec<u8>) -> Result<Self> {
        // Wrap first so the buffer is zeroized on the error path too.
        let s = Self(v);
        check_len(s.0.len())?;
        Ok(s)
    }

    /// Decodes a base32 secret (RFC 4648 §6 alphabet), leniently: ASCII case
    /// is ignored, ASCII whitespace anywhere is skipped, and trailing `=`
    /// padding is optional. Bits that do not fill a whole byte at the end are
    /// dropped, as Google Authenticator does.
    ///
    /// # Errors
    /// [`Error::InvalidSecret`] for a character outside the alphabet, data
    /// after padding, or an empty result. [`Error::SizeLimit`] if the text is
    /// longer than [`MAX_SECRET_TEXT_LEN`] or the secret longer than
    /// [`MAX_SECRET_LEN`].
    pub fn from_base32(text: &str) -> Result<Self> {
        Self::from_base32_bytes(text.as_bytes())
    }

    pub(crate) fn from_base32_bytes(text: &[u8]) -> Result<Self> {
        if text.len() > MAX_SECRET_TEXT_LEN {
            return Err(Error::SizeLimit);
        }
        Self::from_vec(base32::decode(text)?)
    }

    /// Encodes the secret as canonical base32: upper case, no padding, no
    /// whitespace.
    #[must_use]
    pub fn to_base32(&self) -> SecretText {
        let mut s = String::with_capacity(base32::encoded_len(self.0.len()));
        base32::encode_into(&self.0, &mut s);
        SecretText(s)
    }

    /// Borrows the secret bytes. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }

    /// Length in bytes. Lengths are not considered secret.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: an empty secret cannot be constructed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the secret has at least the 128 bits RFC 4226 §4 R6 requires.
    /// Many services issue 80-bit secrets; those still work, and the UI may
    /// point out that the service chose a short secret. A UI warning should
    /// use [`Self::strength`] instead, which singles out truly short
    /// secrets.
    #[must_use]
    pub fn meets_rfc4226_minimum(&self) -> bool {
        self.0.len() >= RFC4226_MIN_SECRET_LEN
    }

    /// The length class of the secret: warn only on
    /// [`SecretStrength::Weak`].
    #[must_use]
    pub fn strength(&self) -> SecretStrength {
        match self.0.len() {
            n if n < COMMON_MIN_SECRET_LEN => SecretStrength::Weak,
            n if n < RFC4226_MIN_SECRET_LEN => SecretStrength::BelowRfc4226,
            _ => SecretStrength::Rfc4226,
        }
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

fn check_len(len: usize) -> Result<()> {
    if len == 0 {
        Err(Error::InvalidSecret)
    } else if len > MAX_SECRET_LEN {
        Err(Error::SizeLimit)
    } else {
        Ok(())
    }
}

impl Drop for OtpSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for OtpSecret {}

impl fmt::Debug for OtpSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl PartialEq for OtpSecret {
    /// Constant time in the contents; lengths are not secret.
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice().ct_eq(other.0.as_slice()).into()
    }
}

impl Eq for OtpSecret {}

/// Text that contains a secret (a base32 secret or an `otpauth://` URI) or
/// item content the server must not learn (the issuer and account name of
/// an [`OtpAuth`](crate::OtpAuth), CRYPTO_SPEC §7.5, §12), zeroized on drop.
pub struct SecretText(pub(crate) String);

impl SecretText {
    /// Copies `text` into a new zeroizing buffer of exactly its length.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut s = String::with_capacity(text.len());
        s.push_str(text);
        Self(s)
    }

    /// Borrows the text. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl Drop for SecretText {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for SecretText {}

impl From<String> for SecretText {
    /// Takes ownership of `text`; it is zeroized on drop from now on. Copies
    /// the caller made before (reallocations while building it) are out of
    /// reach.
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl PartialEq for SecretText {
    /// Constant time in the contents; lengths are not secret.
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes().ct_eq(other.0.as_bytes()).into()
    }
}

impl Eq for SecretText {}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_limits() {
        assert_eq!(OtpSecret::from_bytes(&[]).err(), Some(Error::InvalidSecret));
        assert!(OtpSecret::from_bytes(&[1]).is_ok());
        assert!(OtpSecret::from_bytes(&[7; MAX_SECRET_LEN]).is_ok());
        assert_eq!(
            OtpSecret::from_bytes(&[7; MAX_SECRET_LEN + 1]).err(),
            Some(Error::SizeLimit)
        );
        let text = "A".repeat(MAX_SECRET_TEXT_LEN + 1);
        assert_eq!(OtpSecret::from_base32(&text).err(), Some(Error::SizeLimit));
        // 205 base32 characters decode to 128 bytes; 206 to 128 bytes plus
        // dropped bits; 208 to 130 bytes.
        assert!(OtpSecret::from_base32(&"A".repeat(205)).is_ok());
        assert_eq!(
            OtpSecret::from_base32(&"A".repeat(208)).err(),
            Some(Error::SizeLimit)
        );
    }

    #[test]
    fn redacted_and_constant_shape() {
        let s = OtpSecret::from_bytes(b"12345678901234567890").unwrap();
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        let t = s.to_base32();
        assert_eq!(format!("{t:?}"), "[REDACTED]");
        assert_eq!(t.expose_secret(), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(t.capacity(), t.expose_secret().len());
        assert_eq!(s.capacity(), s.len());
        assert!(!s.is_empty());
        assert!(s.meets_rfc4226_minimum());
        assert!(
            !OtpSecret::from_bytes(&[0; 15])
                .unwrap()
                .meets_rfc4226_minimum()
        );
    }

    #[test]
    fn strength_thresholds() {
        let s = |n: usize| OtpSecret::from_bytes(&vec![7; n]).unwrap().strength();
        assert_eq!(s(1), SecretStrength::Weak);
        assert_eq!(s(COMMON_MIN_SECRET_LEN - 1), SecretStrength::Weak);
        // 80-bit secrets (Google, GitHub) are not flagged as weak.
        assert_eq!(s(COMMON_MIN_SECRET_LEN), SecretStrength::BelowRfc4226);
        assert_eq!(s(RFC4226_MIN_SECRET_LEN - 1), SecretStrength::BelowRfc4226);
        assert_eq!(s(RFC4226_MIN_SECRET_LEN), SecretStrength::Rfc4226);
        assert_eq!(s(MAX_SECRET_LEN), SecretStrength::Rfc4226);
        let google = OtpSecret::from_base32("JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(google.strength(), SecretStrength::BelowRfc4226);
        assert!(SecretStrength::Weak < SecretStrength::BelowRfc4226);
    }

    #[test]
    fn equality() {
        let a = OtpSecret::from_bytes(b"abc").unwrap();
        let b = OtpSecret::from_base32("MFRGG").unwrap();
        let c = OtpSecret::from_bytes(b"abd").unwrap();
        let d = OtpSecret::from_bytes(b"ab").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }
}
