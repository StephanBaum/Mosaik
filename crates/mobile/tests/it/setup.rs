//! The setup code: typed text and camera frames.

use mosaic_core::SecretKey;
use mosaic_core::setup_code::{SETUP_CODE_PREFIX, SetupCode, build};
use mosaic_mobile::{LumaFrame, MobileError};
use mosaic_passkey::{b64url, from_b64url};
use mosaic_qr::testing::qr_frame;

use crate::common::Phone;

fn code(server: &str, email: &str) -> (String, SecretKey) {
    let secret_key = SecretKey::generate().unwrap();
    let text = build(&SetupCode {
        server: server.to_owned(),
        email: email.to_owned(),
        secret_key: SecretKey::parse(secret_key.to_display().unwrap().expose_secret()).unwrap(),
    })
    .unwrap();
    (text.to_string(), secret_key)
}

fn luma(payload: &str) -> LumaFrame {
    let owned = qr_frame(payload, 4, 4);
    let frame = owned.frame();
    LumaFrame {
        width: frame.width(),
        height: frame.height(),
        stride: frame.width(),
        pixels: owned.pixels().to_vec(),
    }
}

#[test]
fn scan_setup_qr_returns_code_and_drops_others() {
    let phone = Phone::new();
    let (text, secret_key) = code("https://sync.example.com", "alice@example.com");
    let found = phone.core.scan_setup_qr(luma(&text)).unwrap().unwrap();
    assert_eq!(found.server, "https://sync.example.com");
    assert_eq!(found.email, "alice@example.com");
    assert_eq!(
        found.secret_key.expose_bytes(),
        secret_key.to_display().unwrap().expose_secret().as_bytes()
    );
    found.secret_key.wipe();
    assert_eq!(found.secret_key.len(), 0);

    // Other QR codes are dropped unread: no result and no error.
    for other in [
        "https://example.com/?q=1",
        "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP",
        "just some text",
    ] {
        assert!(
            phone.core.scan_setup_qr(luma(other)).unwrap().is_none(),
            "{other}"
        );
    }
    // No QR code at all.
    let blank = LumaFrame {
        width: 64,
        height: 64,
        stride: 64,
        pixels: vec![255; 64 * 64],
    };
    assert!(phone.core.scan_setup_qr(blank).unwrap().is_none());
    // A setup code that does not parse (a server that is not canonical).
    let (bad, _) = code("https://Sync.example.com/", "alice@example.com");
    assert_eq!(
        phone.core.scan_setup_qr(luma(&bad)).err(),
        Some(MobileError::SetupCodeInvalid)
    );
    // A frame that is too short for its size.
    let short = LumaFrame {
        width: 64,
        height: 64,
        stride: 64,
        pixels: vec![0; 10],
    };
    assert_eq!(
        phone.core.scan_setup_qr(short).err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn parse_setup_code_is_strict() {
    let phone = Phone::new();
    let (text, _) = code("https://sync.example.com", "alice@example.com");
    let parsed = phone
        .core
        .parse_setup_code(format!("  {text}\n").into_bytes())
        .unwrap();
    assert_eq!(parsed.server, "https://sync.example.com");
    for bad in [
        String::new(),
        "mosaic-setup:2:AAAA".to_owned(),
        format!("{text}x"),
        format!("{SETUP_CODE_PREFIX}!!!!"),
        "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP".to_owned(),
    ] {
        assert_eq!(
            phone.core.parse_setup_code(bad.clone().into_bytes()).err(),
            Some(MobileError::SetupCodeInvalid),
            "{bad}"
        );
    }
    assert_eq!(
        phone.core.parse_setup_code(vec![0xff, 0xfe]).err(),
        Some(MobileError::InvalidInput)
    );
    // A mistyped Secret Key (checksum) is named as such.
    let json = from_b64url(text.strip_prefix(SETUP_CODE_PREFIX).unwrap(), 1024).unwrap();
    let json = String::from_utf8(json).unwrap();
    let at = json.find("M1-").unwrap() + 3;
    let mut bytes = json.into_bytes();
    bytes[at] = if bytes[at] == b'0' { b'2' } else { b'0' };
    let mistyped = format!("{SETUP_CODE_PREFIX}{}", b64url(&bytes));
    assert_eq!(
        phone.core.parse_setup_code(mistyped.into_bytes()).err(),
        Some(MobileError::SecretKeyMistyped)
    );
}
