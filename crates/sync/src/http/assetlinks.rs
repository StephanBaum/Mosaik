//! The fetch of a site's Digital Asset Links file (ADR-0091, ADR-0093,
//! CRYPTO_SPEC §16): `GET https://<host>/.well-known/assetlinks.json`.
//!
//! Android passkey requests from an app that is not a privileged browser are
//! served only if the rpId's file delegates to the app
//! (`mosaic_origin::assetlinks`). This client sits next to [`super::pwned`]
//! (an HTTPS client that is not the sync server) and uses the same TLS trust
//! ([`super::TlsTrust`]).
//!
//! No redirects, no cookies, no credentials, a 10 s timeout, a body of at most
//! [`MAX_BODY_LEN`] bytes that is **rejected**, never cut. Only a `200` is a
//! file; a `404` is [`TransportError::NotFound`] (the site has no file, so
//! nothing is delegated), anything else is [`TransportError::Protocol`] and a
//! failed connection is [`TransportError::Unreachable`] (the caller says
//! "could not check", never "verified").

use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::redirect::Policy;

use super::canonical_origin;
use super::client::tls_config;
use crate::transport::TransportError;

/// The most of the file that is read, in bytes.
pub const MAX_BODY_LEN: usize = 64 * 1024;

/// The whole request (connect, answer and body).
const TIMEOUT: Duration = Duration::from_secs(10);

/// The path of the file.
const PATH: &str = "/.well-known/assetlinks.json";

/// Fetches `https://<host>/.well-known/assetlinks.json`. `host` is a host name
/// with an optional port, nothing else.
///
/// # Errors
/// [`TransportError::InvalidUrl`] for a `host` that is not one,
/// [`TransportError::NotFound`], [`TransportError::Protocol`] or
/// [`TransportError::Unreachable`] as above.
pub fn fetch(host: &str) -> Result<Vec<u8>, TransportError> {
    fetch_from(&format!("https://{host}"), false)
}

fn fetch_from(origin: &str, no_proxy: bool) -> Result<Vec<u8>, TransportError> {
    // `http` passes `canonical_origin` for loopback only (tests).
    let origin = canonical_origin(origin)?;
    let mut builder = Client::builder()
        .connect_timeout(TIMEOUT)
        .timeout(TIMEOUT)
        .redirect(Policy::none());
    if no_proxy {
        builder = builder.no_proxy();
    }
    let client = builder
        .tls_backend_preconfigured(tls_config()?)
        .build()
        .map_err(|_| TransportError::Io)?;
    let response = client
        .get(format!("{origin}{PATH}"))
        .send()
        .map_err(|_| TransportError::Unreachable)?;
    match response.status().as_u16() {
        200 => {}
        404 => return Err(TransportError::NotFound),
        _ => return Err(TransportError::Protocol),
    }
    let cap = u64::try_from(MAX_BODY_LEN).unwrap_or(u64::MAX);
    if response.content_length().is_some_and(|len| len > cap) {
        return Err(TransportError::Protocol);
    }
    let mut body = Vec::with_capacity(MAX_BODY_LEN.min(8 * 1024));
    response
        .take(cap.saturating_add(1))
        .read_to_end(&mut body)
        .map_err(|_| TransportError::Unreachable)?;
    if body.len() > MAX_BODY_LEN {
        return Err(TransportError::Protocol);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use super::*;

    /// Serves one request with `reply` on loopback; returns the origin and the
    /// request head.
    fn serve(reply: Vec<u8>) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let origin = format!("http://{}", listener.local_addr().expect("addr"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut head = Vec::new();
            let mut buf = [0u8; 512];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buf).expect("read");
                if n == 0 {
                    break;
                }
                head.extend_from_slice(&buf[..n]);
            }
            // A client that already gave up is not a test failure here.
            let _ = stream.write_all(&reply);
            let _ = stream.flush();
            String::from_utf8_lossy(&head).into_owned()
        });
        (origin, handle)
    }

    fn http(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn assetlinks_fetch_returns_a_200_body_with_no_credentials() {
        let (origin, server) = serve(http("200 OK", "", b"[]"));
        assert_eq!(fetch_from(&origin, true).unwrap(), b"[]");
        let head = server.join().expect("server").to_ascii_lowercase();
        assert!(
            head.starts_with("get /.well-known/assetlinks.json http/1.1"),
            "{head}"
        );
        assert!(!head.contains("authorization") && !head.contains("cookie"));
    }

    #[test]
    fn assetlinks_fetch_refuses_redirects_and_non_200() {
        for status in [
            "301 Moved Permanently",
            "302 Found",
            "307 Temporary Redirect",
        ] {
            let (origin, server) = serve(http(status, "Location: http://127.0.0.1:9/x\r\n", b""));
            assert_eq!(
                fetch_from(&origin, true).unwrap_err(),
                TransportError::Protocol,
                "{status}"
            );
            let _ = server.join();
        }
        for (status, expected) in [
            ("404 Not Found", TransportError::NotFound),
            ("500 Internal Server Error", TransportError::Protocol),
            ("204 No Content", TransportError::Protocol),
        ] {
            let (origin, server) = serve(http(status, "", b""));
            assert_eq!(fetch_from(&origin, true).unwrap_err(), expected, "{status}");
            let _ = server.join();
        }
        // A closed port is "unreachable", not "verified" and not "not found".
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port();
        assert_eq!(
            fetch_from(&format!("http://127.0.0.1:{port}"), true).unwrap_err(),
            TransportError::Unreachable
        );
    }

    #[test]
    fn assetlinks_fetch_caps_the_body() {
        let at_cap = vec![b' '; MAX_BODY_LEN];
        let (origin, server) = serve(http("200 OK", "", &at_cap));
        assert_eq!(fetch_from(&origin, true).unwrap().len(), MAX_BODY_LEN);
        let _ = server.join();
        let over = vec![b' '; MAX_BODY_LEN + 1];
        let (origin, server) = serve(http("200 OK", "", &over));
        assert_eq!(
            fetch_from(&origin, true).unwrap_err(),
            TransportError::Protocol
        );
        let _ = server.join();
    }

    #[test]
    fn assetlinks_fetch_takes_a_host_and_nothing_else() {
        for bad in [
            "",
            "example.com/x",
            "user@example.com",
            "exa mple.com",
            "https://example.com",
        ] {
            assert!(fetch(bad).is_err(), "{bad:?}");
        }
        // Plain http to a non-loopback host is never made.
        assert_eq!(
            fetch_from("http://example.com", true).unwrap_err(),
            TransportError::InsecureUrl
        );
    }
}
