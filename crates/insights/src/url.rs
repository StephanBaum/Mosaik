//! Just enough URL parsing for the dashboard: the scheme and the host of a
//! login URL as the user typed it.
//!
//! This is not a WHATWG URL parser and it never decides what to fill (the
//! extension does that with exact-origin matching, brief §4). It only needs
//! to answer "is this plain HTTP?" and "which host is this?" for URLs that
//! were saved in a vault, so it is deliberately small and strict:
//!
//! - Surrounding whitespace is ignored. A URL without `scheme://` (such as
//!   `example.com/login`) has no scheme, which the dashboard does not treat
//!   as HTTP: browsers and Mosaic's extension upgrade such entries to HTTPS.
//! - The authority ends at the first `/`, `\`, `?` or `#`. User info (up to
//!   the last `@`) and the port are dropped. IPv6 literals keep their
//!   address without brackets.
//! - The host is lowercased (ASCII and Unicode), and one trailing `.` is
//!   removed. A host is rejected if it is empty, longer than 253 bytes, has
//!   an empty label (`a..b`, `.a`, `a..`), or contains whitespace, control
//!   characters or URL delimiters.

/// Longest host name accepted (RFC 1035).
pub const MAX_HOST_LEN: usize = 253;

/// Longest URL the parser looks at; longer input finds nothing.
pub const MAX_URL_LEN: usize = 8192;

/// The parts of a login URL the dashboard uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlParts {
    scheme: Option<String>,
    host: String,
}

impl UrlParts {
    /// The scheme, lowercased (`https`, `http`, ...), if the URL had one.
    #[must_use]
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    /// The host, lowercased, without a trailing `.`, brackets, port or
    /// user info.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Whether the URL uses plain HTTP.
    #[must_use]
    pub fn is_http(&self) -> bool {
        self.scheme.as_deref() == Some("http")
    }
}

/// Splits `url` into its scheme and host; `None` if it has no usable host.
#[must_use]
pub fn parse(url: &str) -> Option<UrlParts> {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_URL_LEN {
        return None;
    }
    let (scheme, rest) = match url.split_once("://") {
        Some((scheme, rest)) if is_scheme(scheme) => (Some(scheme.to_ascii_lowercase()), rest),
        Some(_) => return None,
        None => (None, url),
    };
    let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = if let Some(bracketed) = host_port.strip_prefix('[') {
        let (address, after) = bracketed.split_once(']')?;
        if !(after.is_empty() || after.starts_with(':')) || address.is_empty() {
            return None;
        }
        address
    } else {
        host_port.split(':').next().unwrap_or_default()
    };
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty()
        || host.len() > MAX_HOST_LEN
        || host.split('.').any(str::is_empty)
        || !host.chars().all(is_host_char)
    {
        return None;
    }
    Some(UrlParts {
        scheme,
        host: host.to_lowercase(),
    })
}

/// RFC 3986 `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
fn is_scheme(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

fn is_host_char(c: char) -> bool {
    !(c.is_whitespace()
        || c.is_control()
        || matches!(
            c,
            '/' | '\\' | '?' | '#' | '@' | '[' | ']' | '<' | '>' | '"' | '%' | '|' | '^' | '`'
        ))
}

/// Whether `host` names this machine (`localhost`, `*.localhost`,
/// `127.0.0.0/8`, `::1`). Plain HTTP to it never crosses a network.
#[must_use]
pub fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "::1"
        || host == "0:0:0:0:0:0:0:1"
        || ipv4(host).is_some_and(|[a, ..]| a == 127)
}

/// Whether `host` is on a local network: private and link-local IPv4
/// (RFC 1918, RFC 3927), unique-local and link-local IPv6, single-label
/// names, and the suffixes home networks use (`.local`, `.lan`, `.home`,
/// `.home.arpa`, `.internal`, `.intranet`, `.corp`, `fritz.box`).
#[must_use]
pub fn is_local_network(host: &str) -> bool {
    if let Some([a, b, ..]) = ipv4(host) {
        return a == 10
            || (a == 172 && (16..=31).contains(&b))
            || (a == 192 && b == 168)
            || (a == 169 && b == 254);
    }
    if host.contains(':') {
        let first = host.split(':').next().unwrap_or_default();
        let group = u16::from_str_radix(first, 16).unwrap_or(0);
        return (group & 0xfe00) == 0xfc00 || (group & 0xffc0) == 0xfe80;
    }
    !host.contains('.') || host == "fritz.box" || LOCAL_SUFFIXES.iter().any(|s| host.ends_with(s))
}

/// Name suffixes that home and office networks resolve locally.
const LOCAL_SUFFIXES: [&str; 7] = [
    ".local",
    ".lan",
    ".home",
    ".home.arpa",
    ".internal",
    ".intranet",
    ".corp",
];

/// The four octets of a dotted-decimal IPv4 address.
fn ipv4(host: &str) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let mut parts = host.split('.');
    for octet in &mut out {
        let part = parts.next()?;
        if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *octet = part.parse().ok()?;
    }
    parts.next().is_none().then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(url: &str) -> Option<String> {
        parse(url).map(|p| p.host().to_owned())
    }

    #[test]
    fn schemes_and_hosts() {
        let p = parse("https://Login.Example.COM./path?q#f").unwrap();
        assert_eq!(p.scheme(), Some("https"));
        assert_eq!(p.host(), "login.example.com");
        assert!(!p.is_http());

        let p = parse("  HTTP://user:pw@example.com:8080/x ").unwrap();
        assert!(p.is_http());
        assert_eq!(p.host(), "example.com");

        let p = parse("example.com/login").unwrap();
        assert_eq!(p.scheme(), None);
        assert!(!p.is_http());
        assert_eq!(p.host(), "example.com");

        assert_eq!(host("http://[::1]:8080/").as_deref(), Some("::1"));
        assert_eq!(host("http://[FE80::1]").as_deref(), Some("fe80::1"));
        assert_eq!(host("http://a@b@c.example/").as_deref(), Some("c.example"));
        assert_eq!(
            host("https://example.com\\evil").as_deref(),
            Some("example.com")
        );
        assert_eq!(host("https://bücher.de").as_deref(), Some("bücher.de"));
        assert_eq!(
            host("android://com.example.app").as_deref(),
            Some("com.example.app")
        );
    }

    #[test]
    fn rejects_unusable_input() {
        for url in [
            "",
            "   ",
            "https://",
            "https:///path",
            "https://.",
            "1http://example.com",
            "ht tp://example.com",
            "://example.com",
            "https://[::1",
            "https://[]",
            "https://[::1]x",
            "https://exa mple.com",
            "https://exa%41mple.com",
            "https://user@",
            "https://:443",
        ] {
            assert_eq!(parse(url), None, "{url:?}");
        }
        assert_eq!(parse(&"a".repeat(MAX_URL_LEN + 1)), None);
        assert_eq!(
            parse(&format!("https://{}", "a".repeat(MAX_HOST_LEN + 1))),
            None
        );
        assert!(parse(&format!("https://{}", "a".repeat(MAX_HOST_LEN))).is_some());
    }

    /// Fuzz finding (CI, insights `url_host`): "2.." parsed to the host "2.",
    /// which still ended with a dot. Hosts with an empty label are invalid.
    #[test]
    fn rejects_empty_labels() {
        for url in [
            "2..",
            "https://2..",
            "https://a..b",
            "https://.a",
            "https://a...",
            "a..b.com/x",
        ] {
            assert_eq!(parse(url), None, "{url:?}");
        }
        assert_eq!(
            host("https://example.com./x").as_deref(),
            Some("example.com")
        );
        assert_eq!(host("https://[::1]:8080").as_deref(), Some("::1"));
    }

    #[test]
    fn loopback() {
        for h in [
            "localhost",
            "app.localhost",
            "127.0.0.1",
            "127.255.1.2",
            "::1",
            "0:0:0:0:0:0:0:1",
        ] {
            assert!(is_loopback(h), "{h}");
        }
        for h in [
            "localhost.example",
            "128.0.0.1",
            "127.0.0",
            "127.0.0.1.5",
            "::2",
            "example.com",
        ] {
            assert!(!is_loopback(h), "{h}");
        }
    }

    #[test]
    fn local_network() {
        for h in [
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.178.1",
            "169.254.1.1",
            "fd00::1",
            "fc12::1",
            "fe80::1",
            "router",
            "fritz.box",
            "nas.local",
            "printer.lan",
            "pi.home.arpa",
            "wiki.internal",
        ] {
            assert!(is_local_network(h), "{h}");
        }
        for h in [
            "172.15.0.1",
            "172.32.0.1",
            "192.169.0.1",
            "8.8.8.8",
            "2001:db8::1",
            "example.com",
            "fritz.box.example.com",
            "256.1.1.1",
            "1.2.3.04444",
        ] {
            assert!(!is_local_network(h), "{h}");
        }
    }
}
