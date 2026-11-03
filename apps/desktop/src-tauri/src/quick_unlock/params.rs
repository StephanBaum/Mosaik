//! The platform part of a quick-unlock wrapping's public parameters
//! (CRYPTO_SPEC §6.2): `0x02 || platform:u8 || key_nonce:[u8;16] || body`.
//!
//! | platform | body |
//! |---|---|
//! | `0x01` Windows Hello | the 32-byte challenge the Hello key signs |
//! | `0x02` Touch ID | the 113-byte ECIES ciphertext of `W` |
//!
//! The `key_nonce` is random per enrollment and names the OS key
//! ([`key_name`]), so two stores of the same account (a copied store) never
//! share or delete each other's platform key, and the name cannot be
//! predicted from the account ID (ADR-0028 item 3).
//!
//! The vault stores these bytes without looking inside; the decoder is
//! strict (exact lengths, version `0x02`, a known platform byte) because the
//! bytes come from a local file that anyone with write access can edit.
//! Version `0x01` (no nonce, key named by account) never shipped; it decodes
//! to `None`, so an enrollment of that form counts as stale and the user
//! enrolls again.

use mosaic_core::SecretBytes;

use crate::contract::quick_unlock::BiometricPlatformKind;

/// The only parameter version.
pub const VERSION: u8 = 0x02;
/// Platform byte of Windows Hello.
pub const PLATFORM_WINDOWS_HELLO: u8 = 0x01;
/// Platform byte of Touch ID.
pub const PLATFORM_TOUCH_ID: u8 = 0x02;
/// Length of the Windows Hello challenge.
pub const WINDOWS_HELLO_BODY_LEN: usize = 32;
/// Length of an `ECIES` `X9.63` `SHA-256` `AES-GCM` ciphertext of a 32-byte
/// message over P-256: 65-byte ephemeral key, 32 bytes, 16-byte tag.
pub const TOUCH_ID_BODY_LEN: usize = 113;
/// Length of the per-enrollment key nonce.
pub const KEY_NONCE_LEN: usize = 16;
/// Bytes before the body: version, platform, nonce.
const HEADER_LEN: usize = 2 + KEY_NONCE_LEN;

/// A random key nonce; `None` if the system RNG fails.
#[must_use]
pub fn new_key_nonce() -> Option<[u8; KEY_NONCE_LEN]> {
    SecretBytes::<KEY_NONCE_LEN>::random()
        .ok()
        .map(|nonce| *nonce.expose_secret())
}

/// The OS key name for a nonce: `Mosaic quick unlock <32 hex digits>`.
#[must_use]
pub fn key_name(nonce: &[u8; KEY_NONCE_LEN]) -> String {
    use std::fmt::Write as _;
    nonce
        .iter()
        .fold(String::from("Mosaic quick unlock "), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

const fn body_len(platform: BiometricPlatformKind) -> usize {
    match platform {
        BiometricPlatformKind::WindowsHello => WINDOWS_HELLO_BODY_LEN,
        BiometricPlatformKind::TouchId => TOUCH_ID_BODY_LEN,
    }
}

const fn platform_byte(platform: BiometricPlatformKind) -> u8 {
    match platform {
        BiometricPlatformKind::WindowsHello => PLATFORM_WINDOWS_HELLO,
        BiometricPlatformKind::TouchId => PLATFORM_TOUCH_ID,
    }
}

/// Encodes the parameters of `platform`; `None` if `body` has the wrong
/// length.
#[must_use]
pub fn encode(
    platform: BiometricPlatformKind,
    nonce: &[u8; KEY_NONCE_LEN],
    body: &[u8],
) -> Option<Vec<u8>> {
    if body.len() != body_len(platform) {
        return None;
    }
    let mut out = Vec::with_capacity(body.len().saturating_add(HEADER_LEN));
    out.push(VERSION);
    out.push(platform_byte(platform));
    out.extend_from_slice(nonce);
    out.extend_from_slice(body);
    Some(out)
}

/// Decoded parameters.
#[derive(Debug, PartialEq, Eq)]
pub struct Decoded<'a> {
    /// The platform.
    pub platform: BiometricPlatformKind,
    /// The key nonce.
    pub nonce: [u8; KEY_NONCE_LEN],
    /// The platform body.
    pub body: &'a [u8],
}

/// Decodes parameters. `None` for any deviation from the layout.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Decoded<'_>> {
    let [version, platform, rest @ ..] = bytes else {
        return None;
    };
    if *version != VERSION {
        return None;
    }
    let platform = match *platform {
        PLATFORM_WINDOWS_HELLO => BiometricPlatformKind::WindowsHello,
        PLATFORM_TOUCH_ID => BiometricPlatformKind::TouchId,
        _ => return None,
    };
    let (nonce, body) = rest.split_first_chunk::<KEY_NONCE_LEN>()?;
    (body.len() == body_len(platform)).then_some(Decoded {
        platform,
        nonce: *nonce,
        body,
    })
}

/// The platform of well-formed parameters.
#[must_use]
pub fn platform_of(bytes: &[u8]) -> Option<BiometricPlatformKind> {
    decode(bytes).map(|d| d.platform)
}

/// The key nonce and body of well-formed parameters of exactly `platform`.
#[must_use]
pub fn body_for(
    platform: BiometricPlatformKind,
    bytes: &[u8],
) -> Option<([u8; KEY_NONCE_LEN], &[u8])> {
    decode(bytes).and_then(|d| (d.platform == platform).then_some((d.nonce, d.body)))
}

/// The key nonce of well-formed parameters of any platform.
#[must_use]
pub fn nonce_of(bytes: &[u8]) -> Option<[u8; KEY_NONCE_LEN]> {
    decode(bytes).map(|d| d.nonce)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const NONCE: [u8; KEY_NONCE_LEN] = [0x5C; KEY_NONCE_LEN];

    #[test]
    fn round_trips_both_platforms() {
        for (platform, len) in [
            (BiometricPlatformKind::WindowsHello, 32),
            (BiometricPlatformKind::TouchId, 113),
        ] {
            let body = vec![0xA5; len];
            let bytes = encode(platform, &NONCE, &body).unwrap();
            assert_eq!(bytes.len(), len + 18);
            assert_eq!(bytes[0], 0x02);
            assert_eq!(&bytes[2..18], &NONCE);
            assert_eq!(
                decode(&bytes),
                Some(Decoded {
                    platform,
                    nonce: NONCE,
                    body: body.as_slice()
                })
            );
            assert_eq!(platform_of(&bytes), Some(platform));
            assert_eq!(nonce_of(&bytes), Some(NONCE));
            assert_eq!(body_for(platform, &bytes), Some((NONCE, body.as_slice())));
        }
    }

    #[test]
    fn exact_lengths_only() {
        let hello = BiometricPlatformKind::WindowsHello;
        assert!(encode(hello, &NONCE, &[0; 31]).is_none());
        assert!(encode(hello, &NONCE, &[0; 33]).is_none());
        assert!(encode(BiometricPlatformKind::TouchId, &NONCE, &[0; 112]).is_none());
        assert!(encode(BiometricPlatformKind::TouchId, &NONCE, &[0; 114]).is_none());
        let good = encode(hello, &NONCE, &[7; 32]).unwrap();
        assert!(decode(&good[..good.len() - 1]).is_none(), "truncated");
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_none(), "trailing byte");
        assert!(decode(&good[..10]).is_none(), "cut inside the nonce");
        assert!(decode(&[]).is_none());
        assert!(decode(&[VERSION]).is_none());
    }

    #[test]
    fn wrong_version_and_platform_are_rejected() {
        let hello = BiometricPlatformKind::WindowsHello;
        let mut bytes = encode(hello, &NONCE, &[7; 32]).unwrap();
        bytes[0] = 0x03;
        assert!(decode(&bytes).is_none(), "version");
        bytes[0] = VERSION;
        bytes[1] = 0x03;
        assert!(decode(&bytes).is_none(), "platform byte");
        bytes[1] = PLATFORM_TOUCH_ID;
        assert!(decode(&bytes).is_none(), "Touch ID needs 113 bytes");
        let good = encode(hello, &NONCE, &[7; 32]).unwrap();
        assert_eq!(body_for(BiometricPlatformKind::TouchId, &good), None);
    }

    #[test]
    fn the_unshipped_version_one_no_longer_decodes() {
        // 0x01 || platform || body, no nonce.
        let mut old = vec![0x01, PLATFORM_WINDOWS_HELLO];
        old.extend_from_slice(&[7; 32]);
        assert!(decode(&old).is_none());
        assert_eq!(platform_of(&old), None);
    }

    #[test]
    fn nonces_are_random_and_name_the_key() {
        let a = new_key_nonce().unwrap();
        let b = new_key_nonce().unwrap();
        assert_ne!(a, b);
        assert_eq!(
            key_name(&[0x01, 0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF]),
            "Mosaic quick unlock 01ab00000000000000000000000000ff"
        );
    }

    proptest! {
        #[test]
        fn decode_never_panics_and_only_accepts_what_encode_makes(
            bytes in proptest::collection::vec(any::<u8>(), 0..256)
        ) {
            if let Some(d) = decode(&bytes) {
                prop_assert_eq!(encode(d.platform, &d.nonce, d.body), Some(bytes));
            }
        }
    }
}
