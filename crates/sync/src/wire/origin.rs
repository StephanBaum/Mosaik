//! The canonical form of a server origin (CRYPTO_SPEC §9.2, ADR-0046): one
//! function, used by the client for the URL it is given and by the server
//! for its configured `canonical_origin`, so both sides compute the same
//! OPAQUE identity bytes from the same text.
//!
//! `scheme://host[:port]`, lowercase, no path, query, fragment or
//! credentials. The default port (443 for `https`, 80 for `http`) is
//! removed. A port is decimal digits only, 1 to 65535: `0`, `+443`, an empty
//! port and any other character are refused. `http` is accepted only for
//! `127.0.0.1`, `[::1]` and `localhost` (M6 spec §6.1).

/// Why an origin was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OriginError {
    /// `http` to a host that is not loopback, or a scheme other than
    /// `https` and `http`.
    #[error("origin is not https")]
    Insecure,
    /// Not `scheme://host[:port]`.
    #[error("invalid origin")]
    Invalid,
}

/// The canonical origin of `base` (see the [module documentation](self)).
///
/// # Errors
/// [`OriginError::Insecure`], [`OriginError::Invalid`].
pub fn canonical_origin(base: &str) -> Result<String, OriginError> {
    let (scheme, rest) = base.split_once("://").ok_or(OriginError::Invalid)?;
    let scheme = scheme.to_ascii_lowercase();
    if rest.is_empty()
        || !rest.is_ascii()
        || rest.contains(['/', '?', '#', '@', ' ', '\\', '%'])
        || rest.chars().any(|c| c.is_ascii_control())
    {
        return Err(OriginError::Invalid);
    }
    let rest = rest.to_ascii_lowercase();
    let (host, port) = match rest.strip_prefix('[') {
        Some(v6) => {
            let (h, p) = v6.split_once(']').ok_or(OriginError::Invalid)?;
            let port = if p.is_empty() {
                None
            } else {
                Some(p.strip_prefix(':').ok_or(OriginError::Invalid)?)
            };
            if h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
                return Err(OriginError::Invalid);
            }
            (format!("[{h}]"), port)
        }
        None => match rest.split_once(':') {
            Some((h, p)) => (h.to_owned(), Some(p)),
            None => (rest.clone(), None),
        },
    };
    let host_ok = !host.is_empty()
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && (host.starts_with('[')
            || host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'));
    if !host_ok {
        return Err(OriginError::Invalid);
    }
    // Digits only: `u16::from_str` would also accept a leading `+`.
    let port = match port {
        None => None,
        Some(p) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => {
            match p.parse::<u16>() {
                Ok(n) if n != 0 => Some(n),
                _ => return Err(OriginError::Invalid),
            }
        }
        Some(_) => return Err(OriginError::Invalid),
    };
    let loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]");
    let default_port = match scheme.as_str() {
        "https" => 443,
        "http" if loopback => 80,
        _ => return Err(OriginError::Insecure),
    };
    Ok(match port {
        Some(p) if p != default_port => format!("{scheme}://{host}:{p}"),
        _ => format!("{scheme}://{host}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_lowercase_without_default_ports() {
        for (given, canonical) in [
            ("https://Sync.Example.com", "https://sync.example.com"),
            ("HTTPS://sync.example.com:443", "https://sync.example.com"),
            (
                "https://sync.example.com:8443",
                "https://sync.example.com:8443",
            ),
            ("https://sync.example.com:0443", "https://sync.example.com"),
            ("http://127.0.0.1:80", "http://127.0.0.1"),
            ("http://127.0.0.1:8743", "http://127.0.0.1:8743"),
            ("http://LOCALHOST", "http://localhost"),
            ("http://[::1]:1", "http://[::1]:1"),
            ("http://[::1]:80", "http://[::1]"),
            // 80 is not the default of https, and 443 not of http.
            ("https://sync.example.com:80", "https://sync.example.com:80"),
            ("http://127.0.0.1:443", "http://127.0.0.1:443"),
        ] {
            assert_eq!(canonical_origin(given).as_deref(), Ok(canonical), "{given}");
            // Canonical forms are fixed points.
            assert_eq!(canonical_origin(canonical).as_deref(), Ok(canonical));
        }
    }

    #[test]
    fn insecure_and_invalid_origins_are_refused() {
        for insecure in [
            "http://sync.example.com",
            "http://127.0.0.1.example.com",
            "ftp://sync.example.com",
        ] {
            assert_eq!(
                canonical_origin(insecure),
                Err(OriginError::Insecure),
                "{insecure}"
            );
        }
        for bad in [
            "sync.example.com",
            "https://",
            "https://sync.example.com/",
            "https://sync.example.com/path",
            "https://user@sync.example.com",
            "https://sync.example.com:0",
            "https://sync.example.com:00",
            "https://sync.example.com:+443",
            "https://sync.example.com:-1",
            "https://sync.example.com:4 43",
            "https://sync.example.com:44a",
            "https://sync.example.com:\u{0664}\u{0664}\u{0663}",
            "https://sync.example.com:99999",
            "https://sync.example.com:",
            "https://[::1]x",
            "https://.example.com",
            "https://exämple.com",
            "https://sync example.com",
        ] {
            assert_eq!(canonical_origin(bad), Err(OriginError::Invalid), "{bad}");
        }
    }
}
