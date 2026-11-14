//! The platform parameters of a quick-unlock enrolment (CRYPTO_SPEC §6.2,
//! ADR-0090): `0x02 || platform:u8 || key_nonce[16] || body || max_age_days:u8`.
//! The vault stores them as opaque bytes; the decoder here is strict: version
//! `0x02`, a known platform byte that matches the configured platform, exact
//! body lengths and a maximum age of 1 to 30 days. The maximum age is the
//! verified setting at the time of the enrolment (G1-10): the enrolment carries
//! it, so an unverified settings file cannot shorten it while the phone is
//! locked after a restart.

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
/// The longest maximum age an enrolment may carry, in days (CRYPTO_SPEC §6.2).
pub const MAX_AGE_DAYS_LIMIT: u8 = 30;
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
    /// The maximum age of the enrolment in days, 1 to 30: the verified
    /// setting when it was made.
    pub max_age_days: u8,
}

impl Params {
    /// Checks the body and builds the parameters.
    ///
    /// # Errors
    /// `invalidInput` for a body of the wrong length or with unknown flag bits,
    /// or a maximum age outside 1 to 30 days.
    pub fn new(
        platform: QuickUnlockPlatform,
        key_nonce: [u8; KEY_NONCE_LEN],
        body: Vec<u8>,
        max_age_days: u8,
    ) -> Result<Self> {
        let ok = match platform {
            QuickUnlockPlatform::IosSecureEnclave => body.len() == IOS_BODY_LEN,
            QuickUnlockPlatform::AndroidKeystore { strongbox } => {
                body.len() == ANDROID_BODY_LEN
                    && body.first().copied() == Some(if strongbox { FLAG_STRONGBOX } else { 0x00 })
            }
        };
        if ok && (1..=MAX_AGE_DAYS_LIMIT).contains(&max_age_days) {
            Ok(Self {
                platform,
                key_nonce,
                body,
                max_age_days,
            })
        } else {
            Err(MobileError::InvalidInput)
        }
    }

    /// The stored bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + KEY_NONCE_LEN + self.body.len() + 1);
        out.push(PARAMS_VERSION);
        out.push(match self.platform {
            QuickUnlockPlatform::IosSecureEnclave => PLATFORM_IOS,
            QuickUnlockPlatform::AndroidKeystore { .. } => PLATFORM_ANDROID,
        });
        out.extend_from_slice(&self.key_nonce);
        out.extend_from_slice(&self.body);
        out.push(self.max_age_days);
        out
    }

    /// Strict decoder.
    ///
    /// # Errors
    /// `invalidInput` for another version, an unknown platform byte, a platform
    /// that is not `expected`, a body of the wrong length, or a maximum age
    /// outside 1 to 30 days.
    pub fn decode(bytes: &[u8], expected: Platform) -> Result<Self> {
        let [version, platform, rest @ ..] = bytes else {
            return Err(MobileError::InvalidInput);
        };
        if *version != PARAMS_VERSION {
            return Err(MobileError::InvalidInput);
        }
        let (nonce, rest) = rest
            .split_first_chunk::<KEY_NONCE_LEN>()
            .ok_or(MobileError::InvalidInput)?;
        let (max_age_days, body) = rest.split_last().ok_or(MobileError::InvalidInput)?;
        let platform = match (*platform, expected) {
            (PLATFORM_IOS, Platform::Ios) => QuickUnlockPlatform::IosSecureEnclave,
            (PLATFORM_ANDROID, Platform::Android) => QuickUnlockPlatform::AndroidKeystore {
                strongbox: body.first().is_some_and(|f| f & FLAG_STRONGBOX != 0),
            },
            _ => return Err(MobileError::InvalidInput),
        };
        Self::new(platform, *nonce, body.to_vec(), *max_age_days)
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
            14,
        )
        .unwrap();
        let bytes = p.encode();
        assert_eq!(&bytes[..2], &[0x02, 0x03]);
        assert_eq!(bytes.len(), 2 + 16 + 113 + 1);
        assert_eq!(bytes.last(), Some(&14));
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
            30,
        )
        .unwrap();
        let bytes = strongbox.encode();
        assert_eq!(&bytes[..2], &[0x02, 0x04]);
        assert_eq!(bytes.len(), 2 + 16 + 61 + 1);
        assert_eq!(
            Params::decode(&bytes, Platform::Android).unwrap(),
            strongbox
        );
        let software = Params::decode(
            &Params::new(
                QuickUnlockPlatform::AndroidKeystore { strongbox: false },
                nonce,
                android_body(0),
                7,
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
                android_body(1),
                7
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
    fn quick_unlock_params_carry_the_verified_max_age() {
        let make = |days| {
            Params::new(
                QuickUnlockPlatform::IosSecureEnclave,
                [1; 16],
                vec![9; IOS_BODY_LEN],
                days,
            )
        };
        // 1 to 30 days; nothing else (the spec cap is 30).
        assert!(make(0).is_err());
        assert!(make(31).is_err());
        let bytes = make(30).unwrap().encode();
        assert_eq!(
            Params::decode(&bytes, Platform::Ios).unwrap().max_age_days,
            30
        );
        for bad in [0u8, 31, 255] {
            let mut tampered = bytes.clone();
            *tampered.last_mut().unwrap() = bad;
            assert!(Params::decode(&tampered, Platform::Ios).is_err(), "{bad}");
        }
        // The old layout (no trailing age) is refused.
        assert!(Params::decode(&bytes[..bytes.len() - 1], Platform::Ios).is_err());
    }

    #[test]
    fn quick_unlock_platform_byte_must_match_platform() {
        let ios = Params::new(
            QuickUnlockPlatform::IosSecureEnclave,
            [3; 16],
            vec![0; IOS_BODY_LEN],
            1,
        )
        .unwrap()
        .encode();
        assert!(Params::decode(&ios, Platform::Android).is_err());
        let android = Params::new(
            QuickUnlockPlatform::AndroidKeystore { strongbox: false },
            [3; 16],
            android_body(0),
            1,
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

/// Property tests (M10-G5): the encoding round-trips, and the decoder is
/// strict about every byte of it.
#[cfg(test)]
mod props {
    use super::*;
    use proptest::prelude::*;

    fn params() -> impl Strategy<Value = Params> {
        (
            any::<[u8; KEY_NONCE_LEN]>(),
            1u8..=MAX_AGE_DAYS_LIMIT,
            0u8..3,
            prop::collection::vec(any::<u8>(), ANDROID_BODY_LEN.max(IOS_BODY_LEN)),
        )
            .prop_map(|(nonce, age, kind, mut raw)| match kind {
                0 => {
                    raw.truncate(IOS_BODY_LEN);
                    Params::new(QuickUnlockPlatform::IosSecureEnclave, nonce, raw, age).unwrap()
                }
                k => {
                    raw.truncate(ANDROID_BODY_LEN);
                    raw[0] = if k == 1 { FLAG_STRONGBOX } else { 0 };
                    Params::new(
                        QuickUnlockPlatform::AndroidKeystore { strongbox: k == 1 },
                        nonce,
                        raw,
                        age,
                    )
                    .unwrap()
                }
            })
    }

    fn platform_of(p: &Params) -> Platform {
        match p.platform {
            QuickUnlockPlatform::IosSecureEnclave => Platform::Ios,
            QuickUnlockPlatform::AndroidKeystore { .. } => Platform::Android,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn encode_then_decode_is_the_identity(p in params()) {
            let bytes = p.encode();
            prop_assert_eq!(Params::decode(&bytes, platform_of(&p)).unwrap(), p.clone());
            // Idempotent.
            prop_assert_eq!(
                Params::decode(&bytes, platform_of(&p)).unwrap().encode(),
                bytes
            );
        }

        #[test]
        fn the_other_platform_never_decodes_it(p in params()) {
            let other = match platform_of(&p) {
                Platform::Ios => Platform::Android,
                Platform::Android => Platform::Ios,
            };
            prop_assert!(Params::decode(&p.encode(), other).is_err());
        }

        #[test]
        fn any_truncation_or_extension_is_refused(
            p in params(), cut in 1usize..8, extra in prop::collection::vec(any::<u8>(), 1..4),
        ) {
            let bytes = p.encode();
            let plat = platform_of(&p);
            prop_assert!(Params::decode(&bytes[..bytes.len() - cut], plat).is_err());
            let mut long = bytes;
            long.extend(extra);
            prop_assert!(Params::decode(&long, plat).is_err());
        }

        /// Any accepted byte string is exactly what its decoded form encodes
        /// to (no two encodings of one value), whatever bytes are thrown at it.
        #[test]
        fn accepted_bytes_are_canonical(
            p in params(), pos in any::<prop::sample::Index>(), delta in 1u8..=255,
        ) {
            let mut bytes = p.encode();
            let i = pos.index(bytes.len());
            bytes[i] ^= delta;
            for plat in [Platform::Ios, Platform::Android] {
                if let Ok(q) = Params::decode(&bytes, plat) {
                    prop_assert_eq!(q.encode(), bytes.clone());
                }
            }
        }

        #[test]
        fn max_age_outside_one_to_thirty_is_refused(age in prop_oneof![Just(0u8), 31u8..=255]) {
            prop_assert!(
                Params::new(
                    QuickUnlockPlatform::IosSecureEnclave,
                    [0; KEY_NONCE_LEN],
                    vec![0; IOS_BODY_LEN],
                    age,
                )
                .is_err()
            );
        }

        #[test]
        fn alias_and_aad_depend_on_the_nonce_only(a in any::<[u8; 16]>(), b in any::<[u8; 16]>()) {
            prop_assume!(a != b);
            prop_assert_ne!(quick_unlock_aad(&a), quick_unlock_aad(&b));
            for plat in [
                QuickUnlockPlatform::IosSecureEnclave,
                QuickUnlockPlatform::AndroidKeystore { strongbox: false },
            ] {
                prop_assert_ne!(key_alias(plat, &a), key_alias(plat, &b));
            }
        }
    }
}
