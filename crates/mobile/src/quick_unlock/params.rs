//! The platform parameters of a quick-unlock enrolment (CRYPTO_SPEC §6.2,
//! ADR-0090): `0x02 || platform:u8 || key_nonce[16] || body`. The vault stores
//! them as opaque bytes; the decoder here is strict: version `0x02`, a known
//! platform byte that matches the configured platform, exact body lengths.

use mosaic_core::encoding::label;

use crate::config::Platform;
use crate::error::{MobileError, Result};
use crate::state::QuickUnlockPlatform;
use crate::storage::hex;

/// The version byte of the parameters.
pub const PARAMS_VERSION: u8 = 0x02;
/// The platform byte of the iOS Secure Enclave row.
pub const PLATFORM_IOS: u8 = 0x03;
/// The platform byte of the Android Keystore row.
pub const PLATFORM_ANDROID: u8 = 0x04;
/// The random per-enrolment nonce that names the platform key.
pub const KEY_NONCE_LEN: usize = 16;
/// The iOS body: the ECIES ciphertext of `W` (`eciesEncryptionCofactorX963SHA256AESGCM`).
pub const IOS_BODY_LEN: usize = 113;
/// The Android body: `flags:u8 || iv[12] || ct[32] || tag[16]`.
pub const ANDROID_BODY_LEN: usize = 61;
/// Bit 0 of the Android flags byte: the key lives in `StrongBox`. The byte is
/// informational and not authenticated.
pub const FLAG_STRONGBOX: u8 = 0x01;

/// Decoded parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    /// The platform row.
    pub platform: QuickUnlockPlatform,
    /// The nonce that names the platform key.
    pub key_nonce: [u8; KEY_NONCE_LEN],
    /// The platform's ciphertext of `W`.
    pub body: Vec<u8>,
}

impl Params {
    /// Checks the body and builds the parameters.
    ///
    /// # Errors
    /// `invalidInput` for a body of the wrong length or with unknown flag bits.
    pub fn new(
        platform: QuickUnlockPlatform,
        key_nonce: [u8; KEY_NONCE_LEN],
        body: Vec<u8>,
    ) -> Result<Self> {
        let ok = match platform {
            QuickUnlockPlatform::IosSecureEnclave => body.len() == IOS_BODY_LEN,
            QuickUnlockPlatform::AndroidKeystore { strongbox } => {
                body.len() == ANDROID_BODY_LEN
                    && body.first().copied() == Some(if strongbox { FLAG_STRONGBOX } else { 0x00 })
            }
        };
        if ok {
            Ok(Self {
                platform,
                key_nonce,
                body,
            })
        } else {
            Err(MobileError::InvalidInput)
        }
    }

    /// The stored bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + KEY_NONCE_LEN + self.body.len());
        out.push(PARAMS_VERSION);
        out.push(match self.platform {
            QuickUnlockPlatform::IosSecureEnclave => PLATFORM_IOS,
            QuickUnlockPlatform::AndroidKeystore { .. } => PLATFORM_ANDROID,
        });
        out.extend_from_slice(&self.key_nonce);
        out.extend_from_slice(&self.body);
        out
    }

    /// Strict decoder.
    ///
    /// # Errors
    /// `invalidInput` for another version, an unknown platform byte, a platform
    /// that is not `expected`, or a body of the wrong length.
    pub fn decode(bytes: &[u8], expected: Platform) -> Result<Self> {
        let [version, platform, rest @ ..] = bytes else {
            return Err(MobileError::InvalidInput);
        };
        if *version != PARAMS_VERSION {
            return Err(MobileError::InvalidInput);
        }
        let (nonce, body) = rest
            .split_first_chunk::<KEY_NONCE_LEN>()
            .ok_or(MobileError::InvalidInput)?;
        let platform = match (*platform, expected) {
            (PLATFORM_IOS, Platform::Ios) => QuickUnlockPlatform::IosSecureEnclave,
            (PLATFORM_ANDROID, Platform::Android) => QuickUnlockPlatform::AndroidKeystore {
                strongbox: body.first().is_some_and(|f| f & FLAG_STRONGBOX != 0),
            },
            _ => return Err(MobileError::InvalidInput),
        };
        Self::new(platform, *nonce, body.to_vec())
    }

    /// The alias of the platform key. Android: `mosaic-quick-unlock-<hex nonce>`.
    /// iOS: `quick-unlock.<hex nonce>`; the app prefixes its bundle ID to get
    /// the `kSecAttrApplicationTag`.
    #[must_use]
    pub fn key_alias(&self) -> String {
        key_alias(self.platform, &self.key_nonce)
    }

    /// The GCM AAD of the Android body; empty on iOS.
    #[must_use]
    pub fn aad(&self) -> Vec<u8> {
        match self.platform {
            QuickUnlockPlatform::IosSecureEnclave => Vec::new(),
            QuickUnlockPlatform::AndroidKeystore { .. } => quick_unlock_aad(&self.key_nonce),
        }
    }
}

/// The alias of the platform key for an enrolment.
#[must_use]
pub fn key_alias(platform: QuickUnlockPlatform, key_nonce: &[u8; KEY_NONCE_LEN]) -> String {
    match platform {
        QuickUnlockPlatform::IosSecureEnclave => format!("quick-unlock.{}", hex(key_nonce)),
        QuickUnlockPlatform::AndroidKeystore { .. } => {
            format!("mosaic-quick-unlock-{}", hex(key_nonce))
        }
    }
}

/// The AAD of the Android quick-unlock body (CRYPTO_SPEC §6.2):
/// `0x02 || 0x04 || ctx("quick-unlock/android") || 0x00 || key_nonce`, the
/// parameter header bound into the AAD (§3). The flags byte stays outside.
#[must_use]
pub fn quick_unlock_aad(key_nonce: &[u8; KEY_NONCE_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + 32 + 1 + KEY_NONCE_LEN);
    out.push(PARAMS_VERSION);
    out.push(PLATFORM_ANDROID);
    out.extend_from_slice(&label::QUICK_UNLOCK_ANDROID.ctx());
    out.push(0x00);
    out.extend_from_slice(key_nonce);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn android_body(flags: u8) -> Vec<u8> {
        let mut b = vec![7u8; ANDROID_BODY_LEN];
        b[0] = flags;
        b
    }

    #[test]
    fn quick_unlock_params_ios_strict_decode() {
        let nonce = [1u8; 16];
        let p = Params::new(
            QuickUnlockPlatform::IosSecureEnclave,
            nonce,
            vec![9; IOS_BODY_LEN],
        )
        .unwrap();
        let bytes = p.encode();
        assert_eq!(&bytes[..2], &[0x02, 0x03]);
        assert_eq!(bytes.len(), 2 + 16 + 113);
        assert_eq!(Params::decode(&bytes, Platform::Ios).unwrap(), p);
        assert_eq!(p.aad(), Vec::<u8>::new());
        // Version, length and body strictness.
        let mut wrong_version = bytes.clone();
        wrong_version[0] = 0x01;
        assert!(Params::decode(&wrong_version, Platform::Ios).is_err());
        assert!(Params::decode(&bytes[..bytes.len() - 1], Platform::Ios).is_err());
        let mut long = bytes.clone();
        long.push(0);
        assert!(Params::decode(&long, Platform::Ios).is_err());
        assert!(Params::decode(&[], Platform::Ios).is_err());
        assert!(Params::decode(&bytes[..10], Platform::Ios).is_err());
        assert_eq!(p.key_alias(), format!("quick-unlock.{}", "01".repeat(16)));
    }

    #[test]
    fn quick_unlock_params_android_strict_decode() {
        let nonce = [2u8; 16];
        let strongbox = Params::new(
            QuickUnlockPlatform::AndroidKeystore { strongbox: true },
            nonce,
            android_body(1),
        )
        .unwrap();
        let bytes = strongbox.encode();
        assert_eq!(&bytes[..2], &[0x02, 0x04]);
        assert_eq!(bytes.len(), 2 + 16 + 61);
        assert_eq!(
            Params::decode(&bytes, Platform::Android).unwrap(),
            strongbox
        );
        let software = Params::decode(
            &Params::new(
                QuickUnlockPlatform::AndroidKeystore { strongbox: false },
                nonce,
                android_body(0),
            )
            .unwrap()
            .encode(),
            Platform::Android,
        )
        .unwrap();
        assert_eq!(
            software.platform,
            QuickUnlockPlatform::AndroidKeystore { strongbox: false }
        );
        // Unknown flag bits, and a flag that contradicts the platform record.
        let mut bad_flags = bytes.clone();
        bad_flags[2 + 16] = 0x03;
        assert!(Params::decode(&bad_flags, Platform::Android).is_err());
        assert!(
            Params::new(
                QuickUnlockPlatform::AndroidKeystore { strongbox: false },
                nonce,
                android_body(1)
            )
            .is_err()
        );
        assert!(Params::decode(&bytes[..bytes.len() - 1], Platform::Android).is_err());
        assert_eq!(
            strongbox.key_alias(),
            format!("mosaic-quick-unlock-{}", "02".repeat(16))
        );
    }

    #[test]
    fn quick_unlock_platform_byte_must_match_platform() {
        let ios = Params::new(
            QuickUnlockPlatform::IosSecureEnclave,
            [3; 16],
            vec![0; IOS_BODY_LEN],
        )
        .unwrap()
        .encode();
        assert!(Params::decode(&ios, Platform::Android).is_err());
        let android = Params::new(
            QuickUnlockPlatform::AndroidKeystore { strongbox: false },
            [3; 16],
            android_body(0),
        )
        .unwrap()
        .encode();
        assert!(Params::decode(&android, Platform::Ios).is_err());
        let mut unknown = android;
        unknown[1] = 0x05;
        assert!(Params::decode(&unknown, Platform::Android).is_err());
    }

    #[test]
    fn quick_unlock_aad_binds_header_label_and_nonce() {
        let aad = quick_unlock_aad(&[5; 16]);
        assert_eq!(&aad[..2], &[0x02, 0x04]);
        assert_eq!(&aad[2..2 + 30], b"mosaic/v1/quick-unlock/android");
        assert_eq!(aad[2 + 30], 0x00);
        assert_eq!(&aad[2 + 31..], &[5u8; 16]);
        assert_ne!(aad, quick_unlock_aad(&[6; 16]));
    }
}
