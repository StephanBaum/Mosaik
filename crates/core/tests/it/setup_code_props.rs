//! Property tests of the setup code (CRYPTO_SPEC §6.9.7, ADR-0096): the
//! build/parse round trip and strict rejection of every mutated code.

use mosaic_core::setup_code::{MAX_SETUP_CODE_LEN, SETUP_CODE_PREFIX, SetupCode, build, parse};
use mosaic_core::{Error, SecretBytes, SecretKey};
use proptest::prelude::*;

/// A stand-in for the sync crate's rules: only lowercase ASCII is canonical.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the parser takes a function returning an Option"
)]
fn lower(server: &str, email: &str) -> Option<(String, String)> {
    Some((server.to_ascii_lowercase(), email.to_ascii_lowercase()))
}

fn key(bytes: [u8; 16]) -> SecretKey {
    let mut b = bytes;
    SecretKey::from_secret(SecretBytes::from_array(&mut b))
}

fn host() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,12}(\\.[a-z][a-z0-9]{1,8}){1,3}".prop_map(|h| format!("https://{h}"))
}

fn email() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9._+-]{0,20}@[a-z][a-z0-9]{0,10}\\.[a-z]{2,6}"
}

fn code_of(server: String, email: String, k: [u8; 16]) -> SetupCode {
    SetupCode {
        server,
        email,
        secret_key: key(k),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn build_then_parse_returns_the_same_fields(
        server in host(), mail in email(), k in any::<[u8; 16]>(),
    ) {
        let code = code_of(server.clone(), mail.clone(), k);
        let text = build(&code).unwrap();
        prop_assert!(text.starts_with(SETUP_CODE_PREFIX));
        prop_assert!(text.len() <= MAX_SETUP_CODE_LEN);
        let back = parse(text.as_bytes(), lower).unwrap();
        prop_assert_eq!(&back.server, &server);
        prop_assert_eq!(&back.email, &mail);
        let shown_back = back.secret_key.to_display().unwrap();
        let shown = code.secret_key.to_display().unwrap();
        prop_assert_eq!(shown_back.expose_secret(), shown.expose_secret());
        // Idempotent: building the parsed code gives the same bytes.
        let rebuilt = build(&back).unwrap();
        prop_assert_eq!(rebuilt.as_bytes(), text.as_bytes());
    }

    /// Changing any one byte of a valid code either fails or yields a code
    /// that builds to exactly the mutated text (nothing is accepted that is
    /// not canonical), and a changed key or fields never goes unnoticed.
    #[test]
    fn a_mutated_code_is_rejected_or_canonical(
        server in host(), mail in email(), k in any::<[u8; 16]>(),
        pos in any::<prop::sample::Index>(), delta in 1u8..=255,
    ) {
        let text = build(&code_of(server, mail, k)).unwrap();
        let mut bytes = text.as_bytes().to_vec();
        let i = pos.index(bytes.len());
        bytes[i] ^= delta;
        if let Ok(parsed) = parse(&bytes, lower) {
            let rebuilt = build(&parsed).unwrap();
            prop_assert_eq!(rebuilt.as_bytes(), bytes.as_slice());
            // Only possible when the flip landed on a different but still
            // canonical code, so it cannot be the original.
            prop_assert_ne!(bytes.as_slice(), text.as_bytes());
        }
    }

    /// A flip inside the prefix is always refused.
    #[test]
    fn a_mutated_prefix_is_always_refused(
        k in any::<[u8; 16]>(), pos in 0usize..SETUP_CODE_PREFIX.len(), delta in 1u8..=255,
    ) {
        let text = build(&code_of("https://a.example".into(), "a@b.example".into(), k)).unwrap();
        let mut bytes = text.as_bytes().to_vec();
        bytes[pos] ^= delta;
        prop_assert_eq!(parse(&bytes, lower).err(), Some(Error::Malformed));
    }

    /// Any truncation or extension is refused.
    #[test]
    fn truncated_or_extended_codes_are_refused(
        k in any::<[u8; 16]>(), cut in 1usize..40, extra in "[A-Za-z0-9_-]{1,8}",
    ) {
        let text = build(&code_of("https://a.example".into(), "a@b.example".into(), k)).unwrap();
        let short = &text.as_bytes()[..text.len() - cut];
        prop_assert!(parse(short, lower).is_err());
        let longer = format!("{}{extra}", text.as_str());
        prop_assert!(parse(longer.as_bytes(), lower).is_err());
    }

    /// Arbitrary bytes never panic and an accepted code rebuilds identically.
    #[test]
    fn arbitrary_bytes_never_panic(data in prop::collection::vec(any::<u8>(), 0..1100)) {
        if let Ok(code) = parse(&data, lower) {
            let rebuilt = build(&code).unwrap();
            prop_assert_eq!(rebuilt.as_bytes(), data.as_slice());
        }
    }

    /// Fields that cannot sit in the JSON unescaped are refused on build.
    #[test]
    fn build_refuses_unescaped_characters(
        bad in prop::sample::select(vec!['"', '\\', '\n', '\u{0}', '\u{7f}']),
        in_server in any::<bool>(),
    ) {
        let (s, e) = if in_server {
            (format!("https://a{bad}.example"), "a@b.example".to_owned())
        } else {
            ("https://a.example".to_owned(), format!("a{bad}@b.example"))
        };
        prop_assert_eq!(build(&code_of(s, e, [1; 16])).err(), Some(Error::Malformed));
    }
}
