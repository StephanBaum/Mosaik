//! Lenient base32 secret decoding: never panics; agrees with a plain
//! reference decoder on every input; accepted secrets re-encode
//! canonically and decode to the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_totp::{Error, MAX_SECRET_LEN, MAX_SECRET_TEXT_LEN, OtpSecret};

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// The documented leniency written out with a table lookup: skip ASCII
/// whitespace, ignore case, allow only `=` (and whitespace) after the first
/// `=`, drop bits that do not fill a byte.
fn reference(text: &[u8]) -> Result<Vec<u8>, Error> {
    if text.len() > MAX_SECRET_TEXT_LEN {
        return Err(Error::SizeLimit);
    }
    let mut bits = Vec::new();
    let mut padded = false;
    for &c in text {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padded = true;
            continue;
        }
        let v = ALPHABET
            .iter()
            .position(|&a| a == c.to_ascii_uppercase())
            .ok_or(Error::InvalidSecret)?;
        if padded {
            return Err(Error::InvalidSecret);
        }
        for i in (0..5).rev() {
            bits.push((v >> i) & 1 == 1);
        }
    }
    let bytes: Vec<u8> = bits
        .chunks_exact(8)
        .map(|byte| byte.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b)))
        .collect();
    if bytes.is_empty() {
        Err(Error::InvalidSecret)
    } else if bytes.len() > MAX_SECRET_LEN {
        Err(Error::SizeLimit)
    } else {
        Ok(bytes)
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let got = OtpSecret::from_base32(s);
    match (&got, reference(data)) {
        (Ok(secret), Ok(want)) => assert_eq!(secret.expose_secret(), want.as_slice()),
        (Err(e), Err(want)) => assert_eq!(*e, want),
        _ => panic!("decoder disagrees with the reference"),
    }
    if let Ok(secret) = got {
        let text = secret.to_base32();
        let again = OtpSecret::from_base32(text.expose_secret()).expect("re-decode");
        assert!(again == secret, "base32 round trip");
        let raw = OtpSecret::from_bytes(secret.expose_secret()).expect("from_bytes");
        assert_eq!(raw.to_base32().expose_secret(), text.expose_secret());
    }
});
