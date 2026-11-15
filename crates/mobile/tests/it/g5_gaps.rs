//! M10-G5: OTP setup from camera frames and typed text, and a join whose storage fails.

use mosaic_mobile::{LumaFrame, MobileError, OtpScheme};
use mosaic_qr::testing::qr_frame;

use crate::common::Phone;

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

const TOTP: &str = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example";

#[test]
fn scan_otp_qr_returns_a_preview_without_the_secret() {
    let (phone, _) = Phone::with_account();
    let preview = phone.core.scan_otp_qr(luma(TOTP)).unwrap().unwrap();
    assert_eq!(preview.scheme, OtpScheme::Totp);
    assert_eq!(preview.issuer.as_deref(), Some("Example"));
    assert_eq!(preview.account_name.as_deref(), Some("alice"));
    assert_eq!(preview.digits, 6);
    assert_eq!(preview.period_s, Some(30));
    assert!(!format!("{preview:?}").contains("JBSWY3DPEHPK3PXP"));
}

#[test]
fn scan_otp_qr_maps_each_refusal_to_its_code() {
    let (phone, _) = Phone::with_account();
    // A QR code that is not an otpauth URI.
    for other in ["https://example.com/", "hello"] {
        assert_eq!(
            phone.core.scan_otp_qr(luma(other)).err(),
            Some(MobileError::OtpQrNotOtp),
            "{other}"
        );
    }
    assert_eq!(
        phone
            .core
            .scan_otp_qr(luma("otpauth-migration://offline?data=AAAA"))
            .err(),
        Some(MobileError::OtpQrMigration)
    );
    // An otpauth URI whose parameters are invalid.
    assert_eq!(
        phone
            .core
            .scan_otp_qr(luma("otpauth://totp/x?secret=!!!!"))
            .err(),
        Some(MobileError::OtpQrInvalid)
    );
    // No QR code in the picture is no result, not an error.
    let blank = LumaFrame {
        width: 64,
        height: 64,
        stride: 64,
        pixels: vec![255; 64 * 64],
    };
    assert!(phone.core.scan_otp_qr(blank).unwrap().is_none());
    // A frame whose buffer is too short, and one with a stride below its width.
    let short = LumaFrame {
        width: 64,
        height: 64,
        stride: 64,
        pixels: vec![0; 10],
    };
    assert_eq!(
        phone.core.scan_otp_qr(short).err(),
        Some(MobileError::InvalidInput)
    );
    let narrow = LumaFrame {
        width: 64,
        height: 4,
        stride: 8,
        pixels: vec![0; 64],
    };
    assert_eq!(
        phone.core.scan_otp_qr(narrow).err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn otp_setup_needs_an_unlocked_core() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    assert_eq!(
        phone.core.scan_otp_qr(luma(TOTP)).err(),
        Some(MobileError::Locked)
    );
    assert_eq!(
        phone.core.otp_from_text(TOTP.as_bytes().to_vec()).err(),
        Some(MobileError::Locked)
    );
}

#[test]
fn otp_from_text_reports_hotp_and_typed_keys() {
    let (phone, _) = Phone::with_account();
    let hotp = phone
        .core
        .otp_from_text(b"otpauth://hotp/Svc:bob?secret=JBSWY3DPEHPK3PXP&counter=3".to_vec())
        .unwrap();
    assert_eq!(hotp.scheme, OtpScheme::Hotp);
    assert_eq!(hotp.period_s, None);
    // A typed base32 key, with the spaces and hyphens people paste.
    let typed = phone
        .core
        .otp_from_text(b" jbsw y3dp-ehpk 3pxp ".to_vec())
        .unwrap();
    assert_eq!(typed.scheme, OtpScheme::Totp);
    assert_eq!(typed.digits, 6);
    // Text that is not UTF-8 is refused, not lossily accepted.
    assert!(phone.core.otp_from_text(vec![0xff, 0xfe, 0x00]).is_err());
}

#[test]
fn a_join_whose_secrets_cannot_be_stored_leaves_no_account() {
    use crate::common::{Desktop, LoopbackServer, PASSWORD};
    use mosaic_mobile::JoinHostedRequest;
    use std::sync::atomic::Ordering;

    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "nostore@example.com");
    let phone = Phone::new();
    // The platform's secure storage refuses every write, but nothing else: the
    // login and the account fetch succeed, so the join reaches the step that
    // stores the Secret Key and the "start over" cleanup runs.
    phone.storage.fail_puts.store(true, Ordering::SeqCst);
    server.advance_ms(60_000);
    let result = phone.core.join_hosted(JoinHostedRequest {
        server_url: server.base.clone(),
        email: "nostore@example.com".to_owned(),
        password: PASSWORD.as_bytes().to_vec(),
        secret_key: desktop.sk_text().into_bytes(),
        device_name: None,
    });
    assert!(result.is_err(), "a join cannot succeed without its storage");
    phone.storage.fail_puts.store(false, Ordering::SeqCst);
    // The cleanup ran: it deleted the Secret Key and the token of the account.
    let deletes = phone.storage.deletes.lock().unwrap().clone();
    assert!(
        deletes.iter().any(|n| n.starts_with("secret-key/")),
        "the start-over cleanup did not delete the Secret Key: {deletes:?}"
    );
    assert!(
        deletes.iter().any(|n| n.starts_with("sync-token/")),
        "the start-over cleanup did not delete the token: {deletes:?}"
    );
    // No half account: the Secret Key and token are not stored, and the phone
    // is a fresh one again (a later join can start over).
    assert!(
        phone.storage.names().is_empty(),
        "{:?}",
        phone.storage.names()
    );
    assert!(
        matches!(
            phone.core.state(),
            mosaic_mobile::SessionState::NoAccount | mosaic_mobile::SessionState::StoreClosed
        ),
        "{:?}",
        phone.core.state()
    );
}
