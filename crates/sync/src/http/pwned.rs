//! The HTTPS client of the opt-in breach check (M7b, ADR-0065).
//!
//! [`RangeClient`] is the production [`RangeSource`] of
//! `mosaic_insights::breach`: `GET https://api.pwnedpasswords.com/range/{prefix}`
//! with `Add-Padding: true` and `User-Agent: Mosaic` (no version), nothing else.
//! It goes **directly** to the service. There is no Mosaic relay: a relay would
//! show the server every prefix next to an account (CRYPTO_SPEC §12). The
//! operating system's proxy settings apply as they do for the sync client.
//!
//! The host is fixed in the code; the 5-character prefix, validated by
//! [`RangePrefix`], is the only variable part of a request. TLS is the sync
//! client's configuration (1.3 only, the operating system's trust store,
//! ADR-0046). Redirects are not followed. A body above
//! [`MAX_BODY_LEN`] is **rejected** ([`RangeError::Protocol`]), never cut: a
//! cut body could hide the match. Nothing here logs, and no error carries
//! the prefix.

use std::io::Read;
use std::time::Duration;

use mosaic_insights::breach::{MAX_BODY_LEN, RangeError, RangePrefix, RangeSource};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, RETRY_AFTER, USER_AGENT};
use reqwest::redirect::Policy;
use zeroize::Zeroizing;

use super::client::tls_config;

/// The service's origin.
const ORIGIN: &str = "https://api.pwnedpasswords.com";
/// Connecting to the service.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// One whole request (connect, answer and body).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The only identification sent. No version, no platform.
const USER_AGENT_VALUE: &str = "Mosaic";

/// The blocking HTTPS client of the range API. One request at a time.
pub struct RangeClient {
    client: Client,
    origin: String,
}

impl RangeClient {
    /// A client for `api.pwnedpasswords.com`.
    ///
    /// # Errors
    /// [`RangeError::Protocol`] if the TLS configuration or the client cannot
    /// be built (nothing was sent).
    pub fn new() -> Result<Self, RangeError> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .https_only(true)
            .tls_backend_preconfigured(tls_config().map_err(|_| RangeError::Protocol)?)
            .build()
            .map_err(|_| RangeError::Protocol)?;
        Ok(Self {
            client,
            origin: ORIGIN.to_owned(),
        })
    }

    /// A client for a plain-http origin (a test server on loopback).
    #[cfg(test)]
    fn for_test(origin: &str) -> Result<Self, RangeError> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .no_proxy()
            .tls_backend_preconfigured(tls_config().map_err(|_| RangeError::Protocol)?)
            .build()
            .map_err(|_| RangeError::Protocol)?;
        Ok(Self {
            client,
            origin: origin.to_owned(),
        })
    }
}

/// `Retry-After` in whole seconds; the HTTP-date form is not honoured.
fn retry_after(value: &HeaderValue) -> Option<u32> {
    let s = value.to_str().ok()?;
    if s.is_empty() || s.len() > 9 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

impl RangeSource for RangeClient {
    fn range(&mut self, prefix: &RangePrefix) -> Result<Zeroizing<Vec<u8>>, RangeError> {
        let response = self
            .client
            .get(format!("{}/range/{}", self.origin, prefix.as_str()))
            .header(USER_AGENT, USER_AGENT_VALUE)
            .header("Add-Padding", "true")
            .send()
            .map_err(|_| RangeError::Offline)?;
        let status = response.status().as_u16();
        match status {
            200 => {}
            429 => {
                return Err(RangeError::RateLimited {
                    retry_after_s: response.headers().get(RETRY_AFTER).and_then(retry_after),
                });
            }
            _ => return Err(RangeError::Protocol),
        }
        let cap = u64::try_from(MAX_BODY_LEN).unwrap_or(u64::MAX);
        if response.content_length().is_some_and(|len| len > cap) {
            return Err(RangeError::Protocol);
        }
        // Sized once for the largest accepted body and one byte more, so
        // reading never reallocates (a reallocation would leave an
        // unzeroized copy of the answer behind). One byte more than the cap
        // tells an oversize body from one that fits; the oversize one is
        // rejected, not cut.
        let mut body = Zeroizing::new(Vec::with_capacity(MAX_BODY_LEN.saturating_add(1)));
        response
            .take(cap.saturating_add(1))
            .read_to_end(&mut body)
            .map_err(|_| RangeError::Offline)?;
        if body.len() > MAX_BODY_LEN {
            return Err(RangeError::Protocol);
        }
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use super::*;

    /// Serves one request with `reply` and returns the request head.
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

    fn prefix() -> RangePrefix {
        RangePrefix::parse("5BAA6").expect("prefix")
    }

    #[test]
    fn pwned_request_carries_only_the_padding_header_and_the_short_user_agent() {
        let (origin, server) = serve(http("200 OK", "", b"ABC:0\r\n"));
        let mut client = RangeClient::for_test(&origin).expect("client");
        let body = client.range(&prefix()).expect("body");
        assert_eq!(body.as_slice(), b"ABC:0\r\n");
        let head = server.join().expect("server").to_ascii_lowercase();
        assert!(head.starts_with("get /range/5baa6 http/1.1"), "{head}");
        assert!(head.contains("add-padding: true"));
        assert!(head.contains("user-agent: mosaic\r\n"));
        assert!(!head.contains("authorization") && !head.contains("cookie"));
    }

    #[test]
    fn pwned_oversize_body_is_rejected_not_truncated() {
        let body = vec![b'A'; MAX_BODY_LEN + 1];
        let (origin, server) = serve(http("200 OK", "", &body));
        let mut client = RangeClient::for_test(&origin).expect("client");
        assert_eq!(client.range(&prefix()).unwrap_err(), RangeError::Protocol);
        let _ = server.join();
    }

    #[test]
    fn pwned_body_at_the_cap_is_accepted() {
        let body = vec![b'A'; MAX_BODY_LEN];
        let (origin, server) = serve(http("200 OK", "", &body));
        let mut client = RangeClient::for_test(&origin).expect("client");
        let answer = client.range(&prefix()).expect("body");
        assert_eq!(answer.len(), MAX_BODY_LEN);
        // Pre-sized: reading never moved the buffer, so no stale copy of the answer was left.
        assert!(
            answer.capacity() > MAX_BODY_LEN,
            "sized for the cap and one more"
        );
        let _ = server.join();
    }

    #[test]
    fn pwned_429_carries_retry_after() {
        let (origin, server) = serve(http("429 Too Many Requests", "Retry-After: 7\r\n", b""));
        let mut client = RangeClient::for_test(&origin).expect("client");
        assert_eq!(
            client.range(&prefix()).unwrap_err(),
            RangeError::RateLimited {
                retry_after_s: Some(7)
            }
        );
        let _ = server.join();
    }

    #[test]
    fn pwned_other_statuses_and_redirects_are_protocol_errors() {
        for status in ["404 Not Found", "500 Internal Server Error", "302 Found"] {
            let (origin, server) = serve(http(status, "Location: http://127.0.0.1:9/\r\n", b""));
            let mut client = RangeClient::for_test(&origin).expect("client");
            assert_eq!(client.range(&prefix()).unwrap_err(), RangeError::Protocol);
            let _ = server.join();
        }
    }

    #[test]
    fn pwned_unreachable_host_is_offline() {
        // A port that was just free and is closed again.
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port();
        let mut client =
            RangeClient::for_test(&format!("http://127.0.0.1:{port}")).expect("client");
        assert_eq!(client.range(&prefix()).unwrap_err(), RangeError::Offline);
    }

    #[test]
    fn pwned_retry_after_is_whole_seconds() {
        let v = |s: &str| HeaderValue::from_str(s).expect("header");
        assert_eq!(retry_after(&v("30")), Some(30));
        assert_eq!(retry_after(&v("")), None);
        assert_eq!(retry_after(&v("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(retry_after(&v("-1")), None);
        assert_eq!(retry_after(&v("99999999999")), None);
    }

    #[test]
    fn pwned_production_client_builds_for_the_fixed_host() {
        let client = RangeClient::new().expect("client");
        assert_eq!(client.origin, "https://api.pwnedpasswords.com");
    }

    /// Manual check against the real service: `cargo test -p mosaic-sync
    /// --features http pwned_live_smoke -- --ignored`.
    #[test]
    #[ignore = "calls the real api.pwnedpasswords.com"]
    fn pwned_live_smoke() {
        let mut client = RangeClient::new().expect("client");
        let body = client.range(&prefix()).expect("range");
        let suffix = b"1E4C9B93F3F0682250B6CF8331B7EE68FD8";
        let wanted = [<[u8; 35]>::try_from(suffix.as_slice()).expect("35 bytes")];
        let found = mosaic_insights::breach::parse_range(&body, &wanted).expect("parse");
        assert_eq!(found, vec![true], "\"password\" is in the corpus");
    }
}
