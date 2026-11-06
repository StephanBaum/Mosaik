//! Email normalization (CRYPTO_SPEC §9.2), shared by the client and the
//! server so both derive the same account and the same fake pre-login
//! values from the same address.
//!
//! Steps: trim whitespace; apply NFC; split at the last `@`; the local part
//! is 1 to 64 characters, lowercased (Unicode lowercase), without control
//! characters or whitespace; the domain is converted to its lowercase IDNA
//! A-label (strict UTS 46), at most 253 characters, with at least one dot
//! and no empty label; the whole address is at most 320 characters.

use unicode_normalization::UnicodeNormalization;

/// The longest normalized address in characters.
pub const MAX_EMAIL_LEN: usize = 320;
/// The longest local part in characters.
pub const MAX_LOCAL_LEN: usize = 64;
/// The longest domain in characters.
pub const MAX_DOMAIN_LEN: usize = 253;

/// Why an address was refused. No variant carries the address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EmailError {
    /// There is no `@`, or the local part or the domain is empty.
    #[error("not an email address")]
    Shape,
    /// The local part is too long, or has a control character or
    /// whitespace.
    #[error("invalid local part")]
    Local,
    /// The domain is not a valid, dotted, IDNA-convertible host name.
    #[error("invalid domain")]
    Domain,
    /// The address is longer than 320 characters.
    #[error("address too long")]
    TooLong,
}

/// Normalizes an email address.
///
/// # Errors
/// [`EmailError`] for anything that is not a syntactically valid address.
pub fn normalize(input: &str) -> Result<String, EmailError> {
    // A bound before any work: nothing longer can normalize to 320
    // characters except by combining marks, which NFC never lengthens past
    // the input's own length in bytes.
    if input.len() > MAX_EMAIL_LEN * 4 + 64 {
        return Err(EmailError::TooLong);
    }
    let nfc: String = input.trim().nfc().collect();
    let (local, domain) = nfc.rsplit_once('@').ok_or(EmailError::Shape)?;
    if local.is_empty() || domain.is_empty() {
        return Err(EmailError::Shape);
    }
    if local.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(EmailError::Local);
    }
    let local: String = local.to_lowercase().nfc().collect();
    let local_chars = local.chars().count();
    if local_chars == 0 || local_chars > MAX_LOCAL_LEN {
        return Err(EmailError::Local);
    }
    let domain = idna::domain_to_ascii_strict(domain).map_err(|_| EmailError::Domain)?;
    let domain = domain.to_ascii_lowercase();
    if domain.len() > MAX_DOMAIN_LEN
        || !domain.contains('.')
        || domain.split('.').any(str::is_empty)
    {
        return Err(EmailError::Domain);
    }
    let out = format!("{local}@{domain}");
    if out.chars().count() > MAX_EMAIL_LEN {
        return Err(EmailError::TooLong);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalization_vectors() {
        for (input, expected) in [
            ("alice@example.com", "alice@example.com"),
            ("  Alice@Example.COM \n", "alice@example.com"),
            ("A.B+tag@Sub.Example.Org", "a.b+tag@sub.example.org"),
            // The split is at the last @.
            ("a@b@example.com", "a@b@example.com"),
            // NFC: a decomposed e-acute becomes the composed one.
            ("Jose\u{301}@example.com", "jos\u{e9}@example.com"),
            ("JOS\u{c9}@example.com", "jos\u{e9}@example.com"),
            // IDNA A-labels, lowercase.
            ("user@B\u{fc}cher.example", "user@xn--bcher-kva.example"),
            ("user@xn--bcher-kva.EXAMPLE", "user@xn--bcher-kva.example"),
            ("\u{e4}@m\u{fc}nchen.de", "\u{e4}@xn--mnchen-3ya.de"),
            // Full-width and ideographic dot mapped by UTS 46.
            ("user@example\u{3002}com", "user@example.com"),
            ("user@\u{ff25}xample.com", "user@example.com"),
            // Limits: 64-character local part, 253-character domain.
            (
                &format!("{}@example.com", "a".repeat(64)),
                &format!("{}@example.com", "a".repeat(64)),
            ),
        ] {
            assert_eq!(normalize(input).as_deref(), Ok(expected), "{input:?}");
        }
        // Idempotent.
        let once = normalize("  Jose\u{301}@B\u{fc}cher.EXAMPLE").unwrap();
        assert_eq!(normalize(&once).unwrap(), once);
    }

    #[test]
    fn invalid_addresses() {
        let long_label = "a".repeat(63);
        let domain_254 = format!(
            "{long_label}.{long_label}.{long_label}.{}.com",
            "a".repeat(58)
        );
        assert!(domain_254.len() > 253);
        for (input, error) in [
            ("", EmailError::Shape),
            ("   ", EmailError::Shape),
            ("no-at-sign", EmailError::Shape),
            ("@example.com", EmailError::Shape),
            ("user@", EmailError::Shape),
            ("user@localhost", EmailError::Domain),
            ("user@example.com.", EmailError::Domain),
            ("user@.example.com", EmailError::Domain),
            ("user@exa mple.com", EmailError::Domain),
            ("user@exam_ple.com", EmailError::Domain),
            ("user@[127.0.0.1]", EmailError::Domain),
            ("user@-bad.example.com", EmailError::Domain),
            ("us er@example.com", EmailError::Local),
            ("us\ter@example.com", EmailError::Local),
            ("us\u{0}er@example.com", EmailError::Local),
            ("us\u{7f}er@example.com", EmailError::Local),
            ("us\u{a0}er@example.com", EmailError::Local),
            ("a\u{2028}b@example.com", EmailError::Local),
        ] {
            assert_eq!(normalize(input), Err(error), "{input:?}");
        }
        assert_eq!(
            normalize(&format!("{}@example.com", "a".repeat(65))),
            Err(EmailError::Local)
        );
        assert_eq!(
            normalize(&format!("user@{domain_254}")),
            Err(EmailError::Domain)
        );
        assert_eq!(
            normalize(&format!("{}@{}", "a".repeat(64), "b".repeat(300))),
            Err(EmailError::Domain)
        );
        assert_eq!(normalize(&"a".repeat(5000)), Err(EmailError::TooLong));
        // 64 + 1 + 253 = 318 characters is within the total.
        let local = "l".repeat(64);
        let domain = format!(
            "{long_label}.{long_label}.{long_label}.{}.com",
            "d".repeat(57)
        );
        assert!(domain.len() <= 253);
        assert!(normalize(&format!("{local}@{domain}")).is_ok());
    }

    /// A normalized address is a fixed point for every character in the
    /// local part and in a domain label (the fuzz target asserts it too).
    #[test]
    fn normalization_is_idempotent_for_every_character() {
        for code in (0u32..=0x2FFFF).chain(0xE0000..=0xE01EF) {
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            for input in [format!("a{c}b@example.com"), format!("ab@ex{c}ample.com")] {
                if let Ok(once) = normalize(&input) {
                    assert_eq!(
                        normalize(&once).as_deref(),
                        Ok(once.as_str()),
                        "U+{code:04X}"
                    );
                }
            }
        }
    }

    #[test]
    fn lowercasing_that_lengthens_is_bounded() {
        // U+0130 lowercases to two characters; 40 of them are 80 > 64.
        let local = "\u{130}".repeat(40);
        assert_eq!(
            normalize(&format!("{local}@example.com")),
            Err(EmailError::Local)
        );
        let ok = "\u{130}".repeat(10);
        assert!(normalize(&format!("{ok}@example.com")).is_ok());
    }
}
