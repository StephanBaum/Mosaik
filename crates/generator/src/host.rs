//! Host-name handling shared by the per-site lookups: the generator's
//! password-rules quirks and the 2FA directory in `mosaic-insights`.
//!
//! A host is normalized (ASCII lowercase, one trailing `.` removed, plain
//! DNS labels only), and then looked up as itself and as each parent domain
//! down to two labels ([`candidates`]).

/// The longest host name accepted, per RFC 1035.
pub const MAX_HOST_LEN: usize = 253;

/// Lowercases and validates `host`; `None` if it is not a DNS name
/// (`a-z`, `0-9`, `-`, non-empty labels; internationalized names must be in
/// their ASCII form).
#[must_use]
pub fn normalize(host: &str) -> Option<String> {
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() || host.len() > MAX_HOST_LEN {
        return None;
    }
    let host = host.to_ascii_lowercase();
    let valid = host.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    valid.then_some(host)
}

/// Whether `host` ends in an all-digit label, as IPv4 addresses do. Such a
/// host has no parent domains.
#[must_use]
pub fn is_numeric(host: &str) -> bool {
    host.rsplit('.')
        .next()
        .is_some_and(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_digit()))
}

/// `host` (already normalized), then its parent domains while they still
/// contain a `.` (so never a bare top-level label). A numeric host has no
/// parents.
pub fn candidates(host: &str) -> impl Iterator<Item = &str> {
    let numeric = is_numeric(host);
    let mut next = Some(host);
    core::iter::from_fn(move || {
        let current = next?;
        next = if numeric {
            None
        } else {
            current
                .split_once('.')
                .map(|(_, parent)| parent)
                .filter(|parent| parent.contains('.'))
        };
        Some(current)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_addresses_have_no_parents() {
        assert!(is_numeric("10.0.0.1"));
        assert!(is_numeric("10.0.0"));
        assert!(!is_numeric("1.0.0.a"));
        assert!(!is_numeric("163.com"));
        assert!(!is_numeric(""));
        assert_eq!(candidates("10.0.0.1").collect::<Vec<_>>(), ["10.0.0.1"]);
    }

    #[test]
    fn candidates_stop_at_two_labels() {
        assert_eq!(
            candidates("a.b.example.co.uk").collect::<Vec<_>>(),
            [
                "a.b.example.co.uk",
                "b.example.co.uk",
                "example.co.uk",
                "co.uk"
            ]
        );
        assert_eq!(
            candidates("example.com").collect::<Vec<_>>(),
            ["example.com"]
        );
        assert_eq!(candidates("localhost").collect::<Vec<_>>(), ["localhost"]);
    }

    #[test]
    fn normalize_lowercases_and_validates() {
        assert_eq!(normalize("Example.COM.").as_deref(), Some("example.com"));
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("a..b"), None);
        assert_eq!(normalize("bücher.de"), None);
        assert!(normalize(&"a".repeat(MAX_HOST_LEN)).is_some());
        assert_eq!(normalize(&"a".repeat(MAX_HOST_LEN + 1)), None);
    }
}
