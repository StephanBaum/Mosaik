//! The platform part of a quick-unlock wrapping's public parameters
//! (CRYPTO_SPEC §6.2): `0x01 || platform:u8 || body`.
//!
//! | platform | body |
//! |---|---|
//! | `0x01` Windows Hello | the 32-byte challenge the Hello key signs |
//! | `0x02` Touch ID | the 113-byte ECIES ciphertext of `W` |
//!
//! The vault stores these bytes without looking inside; the decoder is
//! strict (exact lengths, version `0x01`, a known platform byte) because the
//! bytes come from a local file that anyone with write access can edit.

use crate::contract::quick_unlock::BiometricPlatformKind;

/// The only parameter version.
pub const VERSION: u8 = 0x01;
/// Platform byte of Windows Hello.
pub const PLATFORM_WINDOWS_HELLO: u8 = 0x01;
/// Platform byte of Touch ID.
pub const PLATFORM_TOUCH_ID: u8 = 0x02;
/// Length of the Windows Hello challenge.
pub const WINDOWS_HELLO_BODY_LEN: usize = 32;
/// Length of an `ECIES` `X9.63` `SHA-256` `AES-GCM` ciphertext of a 32-byte
/// message over P-256: 65-byte ephemeral key, 32 bytes, 16-byte tag.
pub const TOUCH_ID_BODY_LEN: usize = 113;

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
pub fn encode(platform: BiometricPlatformKind, body: &[u8]) -> Option<Vec<u8>> {
    if body.len() != body_len(platform) {
        return None;
    }
    let mut out = Vec::with_capacity(body.len().saturating_add(2));
    out.push(VERSION);
    out.push(platform_byte(platform));
    out.extend_from_slice(body);
    Some(out)
}

/// Decodes parameters: the platform and its body. `None` for any deviation
/// from the layout.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(BiometricPlatformKind, &[u8])> {
    let [version, platform, body @ ..] = bytes else {
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
    (body.len() == body_len(platform)).then_some((platform, body))
}

/// The platform of well-formed parameters.
#[must_use]
pub fn platform_of(bytes: &[u8]) -> Option<BiometricPlatformKind> {
    decode(bytes).map(|(platform, _)| platform)
}

/// The body of well-formed parameters of exactly `platform`.
#[must_use]
pub fn body_for(platform: BiometricPlatformKind, bytes: &[u8]) -> Option<&[u8]> {
    decode(bytes).and_then(|(found, body)| (found == platform).then_some(body))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn round_trips_both_platforms() {
        for (platform, len) in [
            (BiometricPlatformKind::WindowsHello, 32),
            (BiometricPlatformKind::TouchId, 113),
        ] {
            let body = vec![0xA5; len];
            let bytes = encode(platform, &body).unwrap();
            assert_eq!(bytes.len(), len + 2);
            assert_eq!(decode(&bytes), Some((platform, body.as_slice())));
            assert_eq!(platform_of(&bytes), Some(platform));
            assert_eq!(body_for(platform, &bytes), Some(body.as_slice()));
        }
    }

    #[test]
    fn exact_lengths_only() {
        assert!(encode(BiometricPlatformKind::WindowsHello, &[0; 31]).is_none());
        assert!(encode(BiometricPlatformKind::WindowsHello, &[0; 33]).is_none());
        assert!(encode(BiometricPlatformKind::TouchId, &[0; 112]).is_none());
        assert!(encode(BiometricPlatformKind::TouchId, &[0; 114]).is_none());
        let good = encode(BiometricPlatformKind::WindowsHello, &[7; 32]).unwrap();
        assert!(decode(&good[..good.len() - 1]).is_none(), "truncated");
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_none(), "trailing byte");
        assert!(decode(&[]).is_none());
        assert!(decode(&[VERSION]).is_none());
    }

    #[test]
    fn wrong_version_and_platform_are_rejected() {
        let mut bytes = encode(BiometricPlatformKind::WindowsHello, &[7; 32]).unwrap();
        bytes[0] = 0x02;
        assert!(decode(&bytes).is_none(), "version");
        bytes[0] = VERSION;
        bytes[1] = 0x03;
        assert!(decode(&bytes).is_none(), "platform byte");
        bytes[1] = PLATFORM_TOUCH_ID;
        assert!(decode(&bytes).is_none(), "Touch ID needs 113 bytes");
        let good = encode(BiometricPlatformKind::WindowsHello, &[7; 32]).unwrap();
        assert_eq!(body_for(BiometricPlatformKind::TouchId, &good), None);
    }

    proptest! {
        #[test]
        fn decode_never_panics_and_only_accepts_what_encode_makes(
            bytes in proptest::collection::vec(any::<u8>(), 0..256)
        ) {
            if let Some((platform, body)) = decode(&bytes) {
                prop_assert_eq!(encode(platform, body), Some(bytes));
            }
        }
    }
}
