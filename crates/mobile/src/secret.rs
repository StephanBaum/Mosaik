//! Secrets that leave Rust (CRYPTO_SPEC §6.9.5): a [`SecretValue`] is the only
//! carrier. The platform calls [`SecretValue::expose_bytes`] at the last
//! moment, wipes its copy, then calls [`SecretValue::wipe`]. Drop zeroizes too.

use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::error::{MobileError, Result};

/// A secret value handed to the platform. Not `Clone`; `Debug` is redacted.
pub struct SecretValue {
    bytes: Mutex<Zeroizing<Vec<u8>>>,
}

impl SecretValue {
    /// Wraps bytes; the caller's buffer moves in.
    #[must_use]
    pub fn new(bytes: Zeroizing<Vec<u8>>) -> Self {
        Self {
            bytes: Mutex::new(bytes),
        }
    }

    /// Wraps a copy of `text`.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::new(Zeroizing::new(text.as_bytes().to_vec()))
    }

    /// A copy of the bytes (UTF-8 text, or raw bytes for `W`). The caller
    /// wipes the copy. A poisoned value reads as empty.
    #[must_use]
    pub fn expose_bytes(&self) -> Vec<u8> {
        self.bytes
            .lock()
            .map_or_else(|_| Vec::new(), |b| b.to_vec())
    }

    /// The number of bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.lock().map_or(0, |b| b.len())
    }

    /// Whether the value is empty (also after [`SecretValue::wipe`]).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Zeroizes and empties the value.
    pub fn wipe(&self) {
        let mut guard = self
            .bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        zeroize::Zeroize::zeroize(&mut *guard);
        guard.clear();
    }
}

impl core::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

/// UTF-8 text from a secret input, moved without a copy. A buffer that is not
/// UTF-8 is zeroized and refused.
pub(crate) fn text_of(bytes: Vec<u8>) -> Result<Zeroizing<String>> {
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Zeroizing::new(text)),
        Err(e) => {
            drop(Zeroizing::new(e.into_bytes()));
            Err(MobileError::InvalidInput)
        }
    }
}
