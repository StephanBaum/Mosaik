//! The setup code with the sync rules (CRYPTO_SPEC §6.9.7, ADR-0096): the
//! format lives in `mosaic_core::setup_code`; the server must be a canonical
//! origin and the email must be normalized (§9.2), and those rules are this
//! crate's.

use mosaic_core::Result;
pub use mosaic_core::setup_code::{MAX_SETUP_CODE_LEN, SETUP_CODE_PREFIX, SetupCode, build};

use crate::wire::{email, origin};

/// Parses a setup code strictly: the server passes
/// [`wire::origin::canonical_origin`](crate::wire::origin::canonical_origin)
/// unchanged and the email equals its own normalization.
///
/// # Errors
/// As [`mosaic_core::setup_code::parse`].
pub fn parse(bytes: &[u8]) -> Result<SetupCode> {
    mosaic_core::setup_code::parse(bytes, |server, address| {
        Some((
            origin::canonical_origin(server).ok()?,
            email::normalize(address).ok()?,
        ))
    })
}

#[cfg(test)]
mod tests {
    use mosaic_core::{Error, SecretKey};

    use super::*;

    fn code(server: &str, email: &str) -> SetupCode {
        SetupCode {
            server: server.to_owned(),
            email: email.to_owned(),
            secret_key: SecretKey::generate().unwrap(),
        }
    }

    #[test]
    fn setup_code_uses_the_sync_origin_and_email_rules() {
        let good = code("https://sync.example.com", "alice@example.com");
        let text = build(&good).unwrap();
        let back = parse(text.as_bytes()).unwrap();
        assert_eq!(back.server, good.server);
        assert_eq!(back.email, good.email);
        assert_eq!(back.secret_key, good.secret_key);
        for (server, email) in [
            ("https://Sync.example.com", "alice@example.com"),
            ("https://sync.example.com/", "alice@example.com"),
            ("http://sync.example.com", "alice@example.com"),
            ("https://sync.example.com", "Alice@example.com"),
            ("https://sync.example.com", "alice@EXAMPLE.com"),
            ("https://sync.example.com", "alice"),
        ] {
            let text = build(&code(server, email)).unwrap();
            assert_eq!(
                parse(text.as_bytes()).err(),
                Some(Error::Malformed),
                "{server} {email}"
            );
        }
    }
}
