//! The container for generated passwords and passphrases.

use core::fmt;

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::entropy::Entropy;

/// A freshly generated password or passphrase.
///
/// The text is zeroized on drop. The type is not `Clone`, not `Display`,
/// and its `Debug` output shows only the entropy, never the text. The
/// buffer is reserved at its final size before the first character is
/// written, so building it leaves no reallocated copies behind.
pub struct GeneratedPassword {
    value: String,
    entropy: Entropy,
}

impl GeneratedPassword {
    /// An empty password with room for exactly `capacity` bytes.
    pub(crate) fn with_capacity(capacity: usize, entropy: Entropy) -> Self {
        Self {
            value: String::with_capacity(capacity),
            entropy,
        }
    }

    pub(crate) fn push(&mut self, c: char) {
        self.value.push(c);
    }

    pub(crate) fn push_str(&mut self, s: &str) {
        self.value.push_str(s);
    }

    /// Borrows the generated text. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.value
    }

    /// The entropy estimate for how this value was generated (CRYPTO_SPEC
    /// §13). It describes the generator settings, not this particular
    /// string.
    #[must_use]
    pub fn entropy(&self) -> Entropy {
        self.entropy
    }

    /// The allocated capacity, to check that generation never reallocates.
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.value.capacity()
    }
}

impl Drop for GeneratedPassword {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

impl ZeroizeOnDrop for GeneratedPassword {}

impl fmt::Debug for GeneratedPassword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeneratedPassword")
            .field("value", &"[REDACTED]")
            .field("entropy", &self.entropy)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let mut p = GeneratedPassword::with_capacity(8, Entropy::new(10.0, false));
        p.push_str("hunter");
        p.push('2');
        assert_eq!(p.expose_secret(), "hunter2");
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("hunter"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"));
        assert_eq!(p.capacity(), 8);
        assert!((p.entropy().bits() - 10.0).abs() < f64::EPSILON);
    }
}
