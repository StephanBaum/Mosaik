//! The two ways a secret crosses the IPC boundary (`docs/DESKTOP_IPC.md`,
//! "Secrets").
//!
//! - [`SecretIn`]: a secret the user typed into the webview (master
//!   password, Secret Key entry, a new item password). It deserializes
//!   straight into a zeroizing [`SecretString`].
//! - [`SecretOut`]: a secret the webview asked to *see* (an explicit reveal,
//!   a generated password, the current one-time code). It is the only type
//!   in the contract that serializes a secret, so every path that hands a
//!   secret to JavaScript is visible by searching for it.
//!
//! Neither is `Clone`, and `Debug` prints neither value. Copying to the
//! clipboard never uses them: the clipboard commands read the secret in
//! Rust, so it never enters JavaScript.

use core::fmt;

use mosaic_core::SecretString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A secret received from the webview.
pub struct SecretIn(SecretString);

impl SecretIn {
    /// The secret, for handing to a zeroizing core type.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }

    /// The secret as a zeroizing string.
    #[must_use]
    pub fn into_secret(self) -> SecretString {
        self.0
    }

    /// A test value.
    #[cfg(test)]
    pub(crate) fn test(value: &str) -> Self {
        Self(SecretString::from(value.to_owned()))
    }
}

impl<'de> Deserialize<'de> for SecretIn {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The owned `String` moves into the zeroizing buffer without a copy.
        // The IPC request body Tauri parsed it from is not zeroized
        // (`docs/DESKTOP_IPC.md`, "Residual exposure").
        String::deserialize(deserializer).map(|s| Self(SecretString::from(s)))
    }
}

/// Request fixtures are generated from Rust values, so tests can serialize
/// a `SecretIn`. Release builds cannot.
#[cfg(test)]
impl Serialize for SecretIn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.expose_secret())
    }
}

impl fmt::Debug for SecretIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretIn(<redacted>)")
    }
}

/// A secret sent to the webview on explicit request.
pub struct SecretOut(SecretString);

impl SecretOut {
    /// Wraps a secret for one response.
    #[must_use]
    pub fn new(secret: SecretString) -> Self {
        Self(secret)
    }

    /// A test value.
    #[cfg(test)]
    pub(crate) fn test(value: &str) -> Self {
        Self(SecretString::from(value.to_owned()))
    }
}

impl Serialize for SecretOut {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.expose_secret())
    }
}

impl fmt::Debug for SecretOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretOut(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_prints_the_value() {
        let input: SecretIn = serde_json::from_str("\"hunter2\"").unwrap();
        assert_eq!(input.expose_secret(), "hunter2");
        assert!(!format!("{input:?}").contains("hunter2"));
        let output = SecretOut::test("hunter2");
        assert!(!format!("{output:?}").contains("hunter2"));
        assert_eq!(serde_json::to_string(&output).unwrap(), "\"hunter2\"");
    }
}
