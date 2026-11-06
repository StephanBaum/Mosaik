//! Account names from relying parties, imports and sync: cleaned before they
//! are stored or shown.

use mosaic_core::strip_spoofing;

/// The longest account name kept and shown, in bytes (WebAuthn L3 6.4.1).
pub const MAX_ACCOUNT_NAME_BYTES: usize = 64;

/// Removes control (Cc) and format (Cf, default-ignorable) characters from a
/// name and cuts it to [`MAX_ACCOUNT_NAME_BYTES`] at a character boundary
/// (WebAuthn L3 6.4.1: the authenticator truncates and the text must not
/// hide or reorder itself).
#[must_use]
pub fn sanitize_name(name: &str) -> String {
    let (stripped, _) = strip_spoofing(name);
    let mut out = String::with_capacity(stripped.len().min(MAX_ACCOUNT_NAME_BYTES));
    for c in stripped.chars().filter(|c| !c.is_control()) {
        if out.len() + c.len_utf8() > MAX_ACCOUNT_NAME_BYTES {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_truncates_at_a_character_boundary() {
        assert_eq!(sanitize_name("alex@example.com"), "alex@example.com");
        assert_eq!(sanitize_name("a\u{202e}b\u{200b}c\u{0007}d\n"), "abcd");
        let long = "x".repeat(100);
        assert_eq!(sanitize_name(&long).len(), MAX_ACCOUNT_NAME_BYTES);
        let wide = "\u{20ac}".repeat(30);
        assert_eq!(sanitize_name(&wide), "\u{20ac}".repeat(21));
    }
}
