//! The settings MAC (CRYPTO_SPEC §11.1, ADR-0030).
//!
//! ```text
//! K_settings = HKDF(AK, "", ctx("settings-mac"), 32)
//! mac        = HMAC-SHA-256(K_settings, account_id[16] || settings_nonce[16] || canonical)
//! ```
//!
//! The canonical form of the settings is the desktop app's business; this
//! module derives the key, draws the nonce and computes and checks tags over
//! the message `account_id || settings_nonce || canonical`. The nonce is a
//! random value that the caller keeps in the local-only row of its store and
//! replaces with every write (rollback binding, ADR-0030).

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::encoding::label;
use crate::ids::AccountId;
use crate::error::{Error, Result};
use crate::kdf::hkdf_ctx;
use crate::secret::{AccountKey, SettingsMacKey};

/// Length of a settings MAC tag in bytes.
pub const SETTINGS_MAC_LEN: usize = 32;

/// Length of the settings nonce in bytes.
pub const SETTINGS_NONCE_LEN: usize = 16;

/// A fresh `random(16)` settings nonce (§11.1).
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_settings_nonce() -> Result<[u8; SETTINGS_NONCE_LEN]> {
    crate::rng::array()
}

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
    fn mac(
        &self,
        account: &AccountId,
        nonce: &[u8; SETTINGS_NONCE_LEN],
        canonical: &[u8],
    ) -> Result<Hmac<Sha256>> {
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(self.0.expose_secret())
            .map_err(|_| Error::SizeLimit)?;
        mac.update(account.as_bytes());
        mac.update(nonce);
        mac.update(canonical);
        Ok(mac)
    }

    /// `HMAC-SHA-256(K_settings, account_id || settings_nonce || canonical)`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
    pub fn tag(
        &self,
        account: &AccountId,
        nonce: &[u8; SETTINGS_NONCE_LEN],
        canonical: &[u8],
    ) -> Result<[u8; SETTINGS_MAC_LEN]> {
        Ok(self.mac(account, nonce, canonical)?.finalize().into_bytes().into())
    }

    /// Whether `tag` is the MAC of the message, compared in constant time.
    /// A tag of the wrong length is not valid.
    #[must_use]
    pub fn verify(
        &self,
        account: &AccountId,
        nonce: &[u8; SETTINGS_NONCE_LEN],
        canonical: &[u8],
        tag: &[u8],
    ) -> bool {
        self.tag(account, nonce, canonical)
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

    fn id(byte: u8) -> AccountId {
        AccountId::from_bytes([byte; 16]).unwrap()
    }

    #[test]
    fn tags_verify_and_nothing_else_does() {
        let key = ak(1).settings_mac_key().unwrap();
        let (a, n) = (id(9), [3u8; 16]);
        let tag = key.tag(&a, &n, b"settings").unwrap();
        assert!(key.verify(&a, &n, b"settings", &tag));
        assert!(!key.verify(&a, &n, b"settingz", &tag));
        assert!(!key.verify(&a, &n, b"settings", &tag[..31]));
        assert!(!key.verify(&a, &n, b"settings", &[tag.as_slice(), &[0]].concat()));
        assert!(!key.verify(&a, &n, b"settings", &[]));
        let mut flipped = tag;
        flipped[0] ^= 1;
        assert!(!key.verify(&a, &n, b"settings", &flipped));
        let other = ak(2).settings_mac_key().unwrap();
        assert!(!other.verify(&a, &n, b"settings", &tag), "another account's key");
        assert!(!key.verify(&id(8), &n, b"settings", &tag), "another account ID");
        assert!(!key.verify(&a, &[4u8; 16], b"settings", &tag), "another nonce");
    }

    #[test]
    fn nonces_are_random_and_16_bytes() {
        let (a, b) = (new_settings_nonce().unwrap(), new_settings_nonce().unwrap());
        assert_eq!(a.len(), SETTINGS_NONCE_LEN);
        assert_ne!(a, b);
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
        manual.update(&[5u8; 16]);
        manual.update(&[6u8; 16]);
        manual.update(b"m");
        let manual: [u8; 32] = manual.finalize().into_bytes().into();
        assert_eq!(key.tag(&id(5), &[6u8; 16], b"m").unwrap(), manual);
    }
}
