//! The site part of a login URL, for duplicate detection and for titles of
//! entries that have none.
//!
//! This is not a URL parser for matching (the extension matches exact
//! origins, brief §4.2). It only answers "is this the same site": the host
//! in lowercase without a leading `www.` or trailing dot, plus the port if
//! it is not the scheme's default. A missing scheme counts as `https`.

/// The site key of `url`, or `None` if it has no host.
pub(crate) fn site(url: &str) -> Option<String> {
    let url = url.trim();
    let (scheme, rest) = match url.find("://") {
        Some(i) => {
            let scheme = url.get(..i)?;
            let valid = !scheme.is_empty()
                && scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
            if !valid {
                return None;
            }
            (scheme.to_ascii_lowercase(), url.get(i.saturating_add(3)..)?)
        }
        None => ("https".to_owned(), url),
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = rest.get(..end)?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let (host, port) = split_port(host_port)?;
    let host = host.trim_end_matches('.').to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host.is_empty() || host.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let default_port = match scheme.as_str() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    Some(match port {
        Some(p) if Some(p) != default_port => format!("{host}:{p}"),
        _ => host.to_owned(),
    })
}

/// Splits `host[:port]` (with bracketed IPv6 hosts). `None` for a port
/// that is not a number.
fn split_port(s: &str) -> Option<(&str, Option<u16>)> {
    if let Some(rest) = s.strip_prefix('[') {
        let close = rest.find(']')?;
        let host = rest.get(..close)?;
        let after = rest.get(close.saturating_add(1)..)?;
        return match after.strip_prefix(':') {
            Some(p) if !p.is_empty() => Some((host, Some(p.parse().ok()?))),
            Some(_) => Some((host, None)),
            None if after.is_empty() => Some((host, None)),
            None => None,
        };
    }
    match s.rsplit_once(':') {
        Some((h, "")) => Some((h, None)),
        Some((h, p)) => Some((h, Some(p.parse().ok()?))),
        None => Some((s, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sites() {
        for (url, want) in [
            ("https://www.Example.com/login?x=1", Some("example.com")),
            ("http://example.com:80/", Some("example.com")),
            ("https://example.com:443", Some("example.com")),
            ("https://example.com:8443/a", Some("example.com:8443")),
            ("http://example.com:443", Some("example.com:443")),
            ("example.com/path", Some("example.com")),
            ("  https://user:pw@Example.COM./#frag ", Some("example.com")),
            ("ftp://files.example.com:21", Some("files.example.com:21")),
            ("androidapp://com.example.app", Some("com.example.app")),
            ("https://[::1]:8080/", Some("::1:8080")),
            ("https://[::1]/", Some("::1")),
            ("https://[::1]:", Some("::1")),
            ("https://example.com:", Some("example.com")),
            ("https://[::1", None),
            ("https://[::1]x", None),
            ("https://[::1]:x", None),
            ("https://example.com:99999", None),
            ("https://", None),
            ("://example.com", None),
            ("ht tp://example.com", None),
            ("https://exa mple.com", None),
            ("", None),
        ] {
            assert_eq!(site(url).as_deref(), want, "{url}");
        }
    }
}
