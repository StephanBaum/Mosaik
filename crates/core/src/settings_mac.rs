//! The settings MAC (CRYPTO_SPEC §11.1, ADR-0030).
//!
//! ```text
//! K_settings = HKDF(AK, "", ctx("settings-mac"), 32)
//! mac        = HMAC-SHA-256(K_settings, canonical(settings))
//! ```
//!
//! The canonical form of the settings is the desktop app's business; this
//! module only derives the key and computes and checks tags over bytes it is
//! given.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::encoding::label;
use crate::error::{Error, Result};
use crate::kdf::hkdf_ctx;
use crate::secret::{AccountKey, SettingsMacKey};

/// Length of a settings MAC tag in bytes.
pub const SETTINGS_MAC_LEN: usize = 32;

impl AccountKey {
    /// `K_settings = HKDF(AK, "", ctx("settings-mac"), 32)` (§11.1).
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn settings_mac_key(&self) -> Result<SettingsMacKey> {
        Ok(SettingsMacKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::SETTINGS_MAC,
        )?))
    }
}

impl SettingsMacKey {
    /// `HMAC-SHA-256(K_settings, message)`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
    pub fn tag(&self, message: &[u8]) -> Result<[u8; SETTINGS_MAC_LEN]> {
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(self.0.expose_secret())
            .map_err(|_| Error::SizeLimit)?;
        mac.update(message);
        Ok(mac.finalize().into_bytes().into())
    }

    /// Whether `tag` is the MAC of `message`, compared in constant time. A
    /// tag of the wrong length is not valid.
    #[must_use]
    pub fn verify(&self, message: &[u8], tag: &[u8]) -> bool {
        self.tag(message)
            .is_ok_and(|expected| bool::from(expected.as_slice().ct_eq(tag)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::SecretBytes;

    fn ak(byte: u8) -> AccountKey {
        AccountKey(SecretBytes::from_array(&mut [byte; 32]))
    }

    #[test]
    fn tags_verify_and_nothing_else_does() {
        let key = ak(1).settings_mac_key().unwrap();
        let tag = key.tag(b"settings").unwrap();
        assert!(key.verify(b"settings", &tag));
        assert!(!key.verify(b"settingz", &tag));
        assert!(!key.verify(b"settings", &tag[..31]));
        assert!(!key.verify(b"settings", &[tag.as_slice(), &[0]].concat()));
        assert!(!key.verify(b"settings", &[]));
        let mut flipped = tag;
        flipped[0] ^= 1;
        assert!(!key.verify(b"settings", &flipped));
        let other = ak(2).settings_mac_key().unwrap();
        assert!(!other.verify(b"settings", &tag), "another account's key");
    }

    #[test]
    fn the_key_is_not_the_account_key() {
        let key = ak(1).settings_mac_key().unwrap();
        assert_ne!(key.0.expose_secret(), &[1u8; 32]);
        assert_eq!(format!("{key:?}"), format!("{:?}", ak(1)), "redacted");
    }

    /// The derivation and the HMAC against RFC 4231 test case 2 (the key is
    /// "Jefe"): the tag function is plain HMAC-SHA-256.
    #[test]
    fn tag_is_hmac_sha256() {
        let mut inner = Hmac::<Sha256>::new_from_slice(b"Jefe").unwrap();
        inner.update(b"what do ya want for nothing?");
        let expected: [u8; 32] = inner.finalize().into_bytes().into();
        assert_eq!(
            expected,
            hex_literal::hex!("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
        );
        let key = ak(7).settings_mac_key().unwrap();
        let mut manual = Hmac::<Sha256>::new_from_slice(key.0.expose_secret()).unwrap();
        manual.update(b"m");
        let manual: [u8; 32] = manual.finalize().into_bytes().into();
        assert_eq!(key.tag(b"m").unwrap(), manual);
    }
}
