//! `otpauth://` parsing: never panics; whatever parses formats to a URI
//! that parses to an equal value and formats to the same text again
//! (formatting is canonical), and every accepted entry generates a code of
//! the right shape.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_totp::{OtpAuth, OtpKind, STEAM_ALPHABET, STEAM_CODE_LEN};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(entry) = OtpAuth::parse(s) else {
        return;
    };
    let uri = entry.to_uri().expect("an accepted entry formats");
    let again = OtpAuth::parse(uri.expose_secret()).expect("formatted URI parses");
    assert!(again == entry, "parse/format round trip");
    let uri2 = again.to_uri().expect("format");
    assert_eq!(uri2.expose_secret(), uri.expose_secret(), "canonical format");

    let code = entry.otp.code_at(1_616_374_841).expect("code");
    match entry.otp.kind {
        OtpKind::Steam { .. } => {
            assert_eq!(code.len(), STEAM_CODE_LEN);
            assert!(code.expose_secret().bytes().all(|c| STEAM_ALPHABET.contains(&c)));
        }
        OtpKind::Totp { digits, .. } | OtpKind::Hotp { digits, .. } => {
            assert_eq!(code.len(), usize::from(digits.get()));
            assert!(code.expose_secret().bytes().all(|c| c.is_ascii_digit()));
        }
    }
});
