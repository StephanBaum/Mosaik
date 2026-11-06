//! A device's name and platform for the device list (CRYPTO_SPEC §10.1,
//! ADR-0043): sealed in the folder backend's `info.mdev`.
//!
//! The plaintext is canonical JSON, at most 512 bytes, with the keys in this
//! order:
//!
//! ```text
//! {"createdAt":<ms>,"name":"<1..64 characters>","platform":"windows"|"macos"|"linux"}
//! ```
//!
//! [`DeviceInfo::from_plaintext`] accepts only exactly that form, so one
//! device info has one encoding.

use mosaic_core::InstallId;
use serde::Deserialize;

use super::{SyncCrypto, WireError};

/// The longest plaintext in bytes.
pub const MAX_PLAINTEXT_LEN: usize = 512;
/// The longest device name in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// The platform a device runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Windows.
    Windows,
    /// macOS.
    Macos,
    /// Linux.
    Linux,
}

impl Platform {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }
}

/// The info of one device.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceInfo {
    /// When the install was created (ms since the Unix epoch).
    #[serde(rename = "createdAt")]
    pub created_at_ms: u64,
    /// The name the user gave it, 1 to [`MAX_NAME_CHARS`] characters
    /// without control characters.
    pub name: String,
    /// The platform.
    pub platform: Platform,
}

/// Appends `s` as a JSON string (RFC 8259: `"` and `\` escaped, U+0000 to
/// U+001F as `\b \t \n \f \r` or `\u00xx` in lowercase hex, everything else
/// as raw UTF-8).
fn push_json_string(out: &mut String, s: &str) {
    use core::fmt::Write as _;
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if c < '\u{20}' => {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl DeviceInfo {
    fn check_name(&self) -> Result<(), WireError> {
        let chars = self.name.chars().count();
        if (1..=MAX_NAME_CHARS).contains(&chars) && !self.name.chars().any(char::is_control) {
            Ok(())
        } else {
            Err(WireError::Field)
        }
    }

    /// The canonical plaintext.
    ///
    /// # Errors
    /// [`WireError::Field`] for a name outside 1 to [`MAX_NAME_CHARS`]
    /// characters or with control characters; [`WireError::Length`] above
    /// [`MAX_PLAINTEXT_LEN`] bytes.
    pub fn to_plaintext(&self) -> Result<Vec<u8>, WireError> {
        self.check_name()?;
        let mut out = String::new();
        out.push_str("{\"createdAt\":");
        out.push_str(&self.created_at_ms.to_string());
        out.push_str(",\"name\":");
        push_json_string(&mut out, &self.name);
        out.push_str(",\"platform\":\"");
        out.push_str(self.platform.as_str());
        out.push_str("\"}");
        if out.len() > MAX_PLAINTEXT_LEN {
            return Err(WireError::Length);
        }
        Ok(out.into_bytes())
    }

    /// Parses a plaintext, which must be exactly the canonical encoding.
    ///
    /// # Errors
    /// [`WireError::Length`] above [`MAX_PLAINTEXT_LEN`] bytes;
    /// [`WireError::Field`] for anything that is not canonical device info.
    pub fn from_plaintext(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() > MAX_PLAINTEXT_LEN {
            return Err(WireError::Length);
        }
        let info: Self = serde_json::from_slice(bytes).map_err(|_| WireError::Field)?;
        if info.to_plaintext()? == bytes {
            Ok(info)
        } else {
            Err(WireError::Field)
        }
    }

    /// Seals this device's info for its `info.mdev`.
    ///
    /// # Errors
    /// As [`DeviceInfo::to_plaintext`]; [`WireError::Crypto`].
    pub fn seal(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        crypto.seal_device_info(&self.to_plaintext()?)
    }

    /// Opens the `info.mdev` of install `device`.
    ///
    /// # Errors
    /// [`WireError::Crypto`] for a forged or damaged file; as
    /// [`DeviceInfo::from_plaintext`].
    pub fn open(
        sealed: &[u8],
        device: InstallId,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        let plaintext = crypto.open_device_info(device, sealed)?;
        Self::from_plaintext(plaintext.expose_secret())
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountId, AccountKey};

    use super::*;
    use crate::wire::KeyedCrypto;

    fn info(name: &str) -> DeviceInfo {
        DeviceInfo {
            created_at_ms: 1_700_000_000_000,
            name: name.to_owned(),
            platform: Platform::Linux,
        }
    }

    #[test]
    fn canonical_plaintext() {
        assert_eq!(
            info("Laptop").to_plaintext().unwrap(),
            br#"{"createdAt":1700000000000,"name":"Laptop","platform":"linux"}"#
        );
        // Escapes, in the same form as the settings canonical JSON.
        let tricky = info("a\"b\\c d\u{e9}");
        let text = String::from_utf8(tricky.to_plaintext().unwrap()).unwrap();
        assert!(text.contains(r#""a\"b\\c d"#) && text.contains('\u{e9}'));
        assert_eq!(DeviceInfo::from_plaintext(text.as_bytes()).unwrap(), tricky);
    }

    #[test]
    fn name_and_size_bounds() {
        assert_eq!(info("").to_plaintext(), Err(WireError::Field));
        assert_eq!(info(&"x".repeat(65)).to_plaintext(), Err(WireError::Field));
        assert!(info(&"x".repeat(64)).to_plaintext().is_ok());
        assert_eq!(info("bell\u{7}").to_plaintext(), Err(WireError::Field));
        assert_eq!(info("tab\there").to_plaintext(), Err(WireError::Field));
        // 64 four-byte characters stay within 512 bytes.
        assert!(info(&"\u{1F600}".repeat(64)).to_plaintext().is_ok());
        assert_eq!(
            DeviceInfo::from_plaintext(&[b' '; MAX_PLAINTEXT_LEN + 1]),
            Err(WireError::Length)
        );
    }

    #[test]
    fn only_the_canonical_form_parses() {
        let good = br#"{"createdAt":1,"name":"x","platform":"macos"}"#;
        assert_eq!(
            DeviceInfo::from_plaintext(good).unwrap().platform,
            Platform::Macos
        );
        for bad in [
            &br#"{"name":"x","createdAt":1,"platform":"macos"}"#[..],
            br#"{"createdAt":1,"name":"x","platform":"macos"} "#,
            br#"{ "createdAt":1,"name":"x","platform":"macos"}"#,
            br#"{"createdAt":1,"name":"x","platform":"macos","extra":1}"#,
            br#"{"createdAt":1,"name":"x"}"#,
            br#"{"createdAt":-1,"name":"x","platform":"macos"}"#,
            br#"{"createdAt":1,"name":"x","platform":"beos"}"#,
            br#"{"createdAt":1,"name":"","platform":"macos"}"#,
            br#"{"createdAt":01,"name":"x","platform":"macos"}"#,
            br#"{"createdAt":1,"name":"a\/b","platform":"macos"}"#,
            b"",
            b"null",
        ] {
            assert!(
                DeviceInfo::from_plaintext(bad).is_err(),
                "{}",
                String::from_utf8_lossy(bad)
            );
        }
    }

    #[test]
    fn seal_and_open() {
        let ak = AccountKey::generate().unwrap();
        let id = InstallId::new_v7().unwrap();
        let c = KeyedCrypto::new(&ak, AccountId::new_v7().unwrap(), id).unwrap();
        let sealed = info("Laptop").seal(&c).unwrap();
        assert_eq!(DeviceInfo::open(&sealed, id, &c).unwrap(), info("Laptop"));
        assert_eq!(
            DeviceInfo::open(&sealed, InstallId::new_v7().unwrap(), &c),
            Err(WireError::Crypto)
        );
        let mut bad = sealed;
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert_eq!(DeviceInfo::open(&bad, id, &c), Err(WireError::Crypto));
    }
}
