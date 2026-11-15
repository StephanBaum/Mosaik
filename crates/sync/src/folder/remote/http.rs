//! The HTTP layer of the provider stores: one blocking client (TLS 1.3 only,
//! the trust of [`crate::http::TlsTrust`], no automatic redirects), the URL
//! rules, a response cap on every request and the mapping of a status to a
//! [`TransportError`].
//!
//! An error never carries what the provider sent. The one redirect that is
//! followed (a download link) is followed without the `Authorization` header.

use std::error::Error as StdError;
use std::time::Duration;

use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue, RETRY_AFTER};
use reqwest::redirect::Policy;
use zeroize::Zeroizing;

use crate::http::client::{
    CONNECT_TIMEOUT, DEFAULT_RETRY_AFTER_S, map_reqwest, parse_retry_after, read_capped, tls_config,
};
use crate::transport::TransportError;

/// The longest URL.
const MAX_URL_LEN: usize = 8 * 1024;
/// The most response headers kept.
const MAX_HEADERS: usize = 64;
/// The longest response header value kept.
const MAX_HEADER_LEN: usize = 8 * 1024;
/// The most of an error body that is read.
const MAX_ERROR_BODY: usize = 64 * 1024;

/// A request body.
pub enum RemoteBody<'a> {
    /// No body.
    None,
    /// These bytes, with this content type.
    Bytes(&'a [u8], &'static str),
    /// An `application/x-www-form-urlencoded` body (it may hold a secret).
    Form(Zeroizing<Vec<u8>>),
}

/// One request.
pub struct RemoteRequest<'a> {
    /// The method.
    pub method: Method,
    /// The absolute URL (checked by [`check_url`]).
    pub url: &'a str,
    /// The headers (a secret one is marked sensitive by its builder).
    pub headers: &'a [(HeaderName, HeaderValue)],
    /// The body.
    pub body: RemoteBody<'a>,
    /// The time the whole request may take.
    pub timeout: Duration,
    /// The most bytes a success answer may have.
    pub cap: u64,
}

/// An answer: the status, `Retry-After` and the (capped) body.
pub struct RemoteReply {
    /// The status code.
    pub status: u16,
    /// `Retry-After` in whole seconds, clamped to one second to one day.
    pub retry_after_s: Option<u32>,
    headers: Vec<(String, String)>,
    /// The body; a success is at most the request's `cap`, a failure is cut at 64 KiB.
    pub body: Vec<u8>,
}

impl RemoteReply {
    /// The first header called `name` (any case); only the first 64 headers
    /// of at most 8 KiB each are kept.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Whether the status is 2xx.
    #[must_use]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    #[cfg(test)]
    pub(crate) fn for_test(status: u16, retry_after_s: Option<u32>) -> Self {
        Self {
            status,
            retry_after_s,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

/// A blocking client for provider APIs.
pub struct RemoteHttp {
    client: Client,
    allow_loopback: bool,
}

impl RemoteHttp {
    /// The production client: redirects off, TLS 1.3 only, `https` only.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the TLS configuration or the client cannot be built.
    pub fn new() -> Result<Self, TransportError> {
        Self::build(false)
    }

    /// A client that also accepts `http://127.0.0.1` and `http://[::1]` and
    /// uses no proxy. Tests only: it exists in test builds and with the
    /// debug-only `test-util` feature, never in a release build.
    ///
    /// # Errors
    /// [`TransportError::Io`] if the TLS configuration or the client cannot be built.
    #[cfg(any(test, feature = "test-util"))]
    pub fn new_loopback() -> Result<Self, TransportError> {
        Self::build(true)
    }

    fn build(allow_loopback: bool) -> Result<Self, TransportError> {
        let mut builder = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(Policy::none())
            .tls_backend_preconfigured(tls_config()?);
        builder = if allow_loopback {
            builder.no_proxy()
        } else {
            builder.https_only(true)
        };
        let client = builder.build().map_err(|_| TransportError::Io)?;
        Ok(Self {
            client,
            allow_loopback,
        })
    }

    /// Whether loopback `http` is accepted.
    #[must_use]
    pub const fn allow_loopback(&self) -> bool {
        self.allow_loopback
    }

    /// Checks a URL against this client's rules.
    ///
    /// # Errors
    /// See [`check_url`].
    pub fn check(&self, url: &str) -> Result<(), TransportError> {
        check_url(url, self.allow_loopback)
    }

    /// Sends one request and reads the answer.
    ///
    /// # Errors
    /// [`TransportError::InsecureUrl`] or [`TransportError::InvalidUrl`] for a
    /// refused URL; [`TransportError::TlsVersionUnsupported`], `Timeout`,
    /// `Unreachable` or `Io` for a failed exchange; [`TransportError::Protocol`]
    /// for a success body over the cap. A failure *status* is not an error
    /// here: map it with [`status_error`].
    pub fn send(&self, req: RemoteRequest<'_>) -> Result<RemoteReply, TransportError> {
        self.check(req.url)?;
        let mut request = self
            .client
            .request(req.method, req.url)
            .timeout(req.timeout);
        for (name, value) in req.headers {
            request = request.header(name.clone(), value.clone());
        }
        request = match req.body {
            RemoteBody::None => request,
            RemoteBody::Bytes(bytes, content_type) => request
                .header(CONTENT_TYPE, content_type)
                .body(bytes.to_vec()),
            RemoteBody::Form(bytes) => request
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(bytes.to_vec()),
        };
        let response = request.send().map_err(|e| map_remote_error(&e))?;
        let status = response.status();
        let retry_after_s = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(parse_retry_after);
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                let v = v.to_str().ok()?;
                (v.len() <= MAX_HEADER_LEN).then(|| (k.as_str().to_owned(), v.to_owned()))
            })
            .take(MAX_HEADERS)
            .collect();
        let ok = status.is_success();
        let cap = usize::try_from(req.cap).unwrap_or(usize::MAX);
        let body = read_capped(response, if ok { cap } else { MAX_ERROR_BODY }, ok)?;
        Ok(RemoteReply {
            status: status.as_u16(),
            retry_after_s,
            headers,
            body,
        })
    }

    /// `GET url` (with `auth`, if any); when the answer is a redirect, `GET`
    /// its `Location` once, **without** the `Authorization` header. A second
    /// redirect, or a target that fails the URL rules, is
    /// [`TransportError::Protocol`].
    ///
    /// # Errors
    /// As [`send`](Self::send), and `Protocol` as above.
    pub fn get_following_one_redirect(
        &self,
        url: &str,
        auth: Option<&HeaderValue>,
        timeout: Duration,
        cap: u64,
    ) -> Result<RemoteReply, TransportError> {
        let mut headers = Vec::new();
        if let Some(value) = auth {
            headers.push((AUTHORIZATION, value.clone()));
        }
        let first = self.send(RemoteRequest {
            method: Method::GET,
            url,
            headers: &headers,
            body: RemoteBody::None,
            timeout,
            cap,
        })?;
        if !is_redirect(first.status) {
            return Ok(first);
        }
        let target = first.header("location").ok_or(TransportError::Protocol)?;
        check_url(target, self.allow_loopback).map_err(|_| TransportError::Protocol)?;
        let second = self.send(RemoteRequest {
            method: Method::GET,
            url: target,
            headers: &[],
            body: RemoteBody::None,
            timeout,
            cap,
        })?;
        if is_redirect(second.status) {
            return Err(TransportError::Protocol);
        }
        Ok(second)
    }
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Whether `authority` is `127.0.0.1` or `[::1]` with an optional port.
fn is_loopback_authority(authority: &str) -> bool {
    let rest = authority
        .strip_prefix("127.0.0.1")
        .or_else(|| authority.strip_prefix("[::1]"));
    match rest {
        Some("") => true,
        Some(port) => port.strip_prefix(':').is_some_and(|p| {
            !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit())
        }),
        None => false,
    }
}

/// The URL rules: `https://` only (`http://127.0.0.1` and `http://[::1]` when
/// `allow_loopback`), no user information, no fragment, at most 8 KiB, no
/// control character, space or backslash.
///
/// # Errors
/// [`TransportError::InsecureUrl`] for `http` to another host or another
/// scheme; [`TransportError::InvalidUrl`] for the rest.
pub(crate) fn check_url(url: &str, allow_loopback: bool) -> Result<(), TransportError> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err(TransportError::InsecureUrl);
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if !(secure || allow_loopback && is_loopback_authority(authority)) {
        return Err(TransportError::InsecureUrl);
    }
    if url.len() > MAX_URL_LEN
        || authority.is_empty()
        || authority.contains('@')
        || url.contains('#')
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return Err(TransportError::InvalidUrl);
    }
    Ok(())
}

/// The error of a failure status. 401 `Unauthorized`, 403 `Forbidden`, 404
/// `NotFound`, 408 `Timeout`, 429 `RateLimited`, 503 with `Retry-After`
/// `RateLimited` and without it `Unreachable`, 507 `QuotaExceeded`, 502 and
/// 504 `Unreachable`, other 5xx `Io`, the rest `Protocol`.
#[must_use]
pub(crate) fn status_error(reply: &RemoteReply) -> TransportError {
    match reply.status {
        401 => TransportError::Unauthorized,
        403 => TransportError::Forbidden,
        404 => TransportError::NotFound,
        408 => TransportError::Timeout,
        429 => TransportError::RateLimited {
            retry_after_s: reply.retry_after_s.unwrap_or(DEFAULT_RETRY_AFTER_S),
        },
        503 => reply
            .retry_after_s
            .map_or(TransportError::Unreachable, |retry_after_s| {
                TransportError::RateLimited { retry_after_s }
            }),
        507 => TransportError::QuotaExceeded,
        502 | 504 => TransportError::Unreachable,
        500..=599 => TransportError::Io,
        _ => TransportError::Protocol,
    }
}

/// Whether a TLS error says the server does not speak TLS 1.3: the peer is
/// incompatible with the offer or sent a protocol-version alert.
fn is_version_error(e: &rustls::Error) -> bool {
    matches!(
        e,
        rustls::Error::PeerIncompatible(_)
            | rustls::Error::AlertReceived(rustls::AlertDescription::ProtocolVersion)
    )
}

/// Whether `err` or anything in its source chain (an `io::Error` wrapping a
/// TLS error included, which `source()` skips) is a TLS version error.
pub(crate) fn classify_tls(err: &(dyn StdError + 'static)) -> bool {
    let mut current: Option<&(dyn StdError + 'static)> = Some(err);
    for _ in 0..16 {
        let Some(e) = current else {
            return false;
        };
        if e.downcast_ref::<rustls::Error>()
            .is_some_and(is_version_error)
        {
            return true;
        }
        if let Some(inner) = e
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::get_ref)
            && inner
                .downcast_ref::<rustls::Error>()
                .is_some_and(is_version_error)
        {
            return true;
        }
        current = e.source();
    }
    false
}

fn map_remote_error(e: &reqwest::Error) -> TransportError {
    if classify_tls(e) {
        TransportError::TlsVersionUnsupported
    } else {
        map_reqwest(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FakeHttp, FakeResponse};

    const T: Duration = Duration::from_secs(10);

    fn get(http: &RemoteHttp, url: &str, cap: u64) -> Result<RemoteReply, TransportError> {
        http.send(RemoteRequest {
            method: Method::GET,
            url,
            headers: &[],
            body: RemoteBody::None,
            timeout: T,
            cap,
        })
    }

    #[test]
    fn check_url_refuses_http_userinfo_and_fragments() {
        for ok in [
            "https://cloud.example.com/remote.php/dav",
            "https://cloud.example.com:8443/x?y=1",
            "http://127.0.0.1:8080/x",
            "http://[::1]:8080/x",
        ] {
            assert_eq!(check_url(ok, true), Ok(()), "{ok}");
        }
        for (url, err) in [
            ("http://cloud.example.com/x", TransportError::InsecureUrl),
            (
                "http://127.0.0.1.evil.example/x",
                TransportError::InsecureUrl,
            ),
            ("ftp://cloud.example.com/", TransportError::InsecureUrl),
            ("cloud.example.com", TransportError::InsecureUrl),
            (
                "https://user@cloud.example.com/",
                TransportError::InvalidUrl,
            ),
            (
                "https://user:pw@cloud.example.com/",
                TransportError::InvalidUrl,
            ),
            (
                "https://cloud.example.com/x#frag",
                TransportError::InvalidUrl,
            ),
            ("https://cloud.example.com/?q=#", TransportError::InvalidUrl),
            ("https:///x", TransportError::InvalidUrl),
            ("https://cloud.example.com/a b", TransportError::InvalidUrl),
            (
                "https://cloud.example.com/a\r\nb",
                TransportError::InvalidUrl,
            ),
        ] {
            assert_eq!(check_url(url, true), Err(err), "{url}");
        }
        let long = format!("https://cloud.example.com/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(check_url(&long, true), Err(TransportError::InvalidUrl));
        // Loopback http needs the flag.
        assert_eq!(
            check_url("http://127.0.0.1:1/", false),
            Err(TransportError::InsecureUrl)
        );
    }

    #[test]
    fn production_client_refuses_loopback_http() {
        let http = RemoteHttp::new().unwrap();
        assert!(!http.allow_loopback());
        assert_eq!(
            http.check("http://127.0.0.1:1/"),
            Err(TransportError::InsecureUrl)
        );
        assert_eq!(
            http.check("http://[::1]:1/"),
            Err(TransportError::InsecureUrl)
        );
    }

    #[test]
    fn success_body_over_cap_is_protocol() {
        let http = RemoteHttp::new_loopback().unwrap();
        let body = vec![7u8; 1000];
        let announced = {
            let body = body.clone();
            FakeHttp::start(move |_| FakeResponse::new(200, &[], body.clone()))
        };
        let chunked = {
            let body = body.clone();
            FakeHttp::start(move |_| {
                FakeResponse::new(200, &[("Transfer-Encoding", "chunked")], body.clone())
            })
        };
        for server in [&announced, &chunked] {
            let url = format!("{}/f", server.origin());
            assert_eq!(get(&http, &url, 999).err(), Some(TransportError::Protocol));
            assert_eq!(get(&http, &url, 1000).unwrap().body, body);
        }
    }

    #[test]
    fn redirect_is_followed_once_without_authorization() {
        let http = RemoteHttp::new_loopback().unwrap();
        let target = FakeHttp::start(|r| {
            if r.path_and_query == "/dl" {
                FakeResponse::new(200, &[], b"payload".to_vec())
            } else {
                FakeResponse::new(404, &[], Vec::new())
            }
        });
        let location = format!("{}/dl", target.origin());
        let first = {
            let location = location.clone();
            FakeHttp::start(move |_| FakeResponse::new(302, &[("Location", &location)], Vec::new()))
        };
        let mut auth = HeaderValue::from_static("Bearer secret");
        auth.set_sensitive(true);
        let url = format!("{}/start", first.origin());
        let reply = http
            .get_following_one_redirect(&url, Some(&auth), T, 100)
            .unwrap();
        assert_eq!(reply.body, b"payload");
        assert_eq!(
            first.requests()[0].header("authorization"),
            Some("Bearer secret")
        );
        let seen = target.requests();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].header("authorization"), None);

        // A redirect to a redirect, and one to another scheme.
        let again = {
            let location = format!("{}/again", first.origin());
            FakeHttp::start(move |_| FakeResponse::new(302, &[("Location", &location)], Vec::new()))
        };
        let url = format!("{}/x", again.origin());
        assert_eq!(
            http.get_following_one_redirect(&url, None, T, 100).err(),
            Some(TransportError::Protocol)
        );
        let ftp =
            FakeHttp::start(|_| FakeResponse::new(302, &[("Location", "ftp://x/y")], Vec::new()));
        let url = format!("{}/x", ftp.origin());
        assert_eq!(
            http.get_following_one_redirect(&url, None, T, 100).err(),
            Some(TransportError::Protocol)
        );
        // No `Location` at all.
        let none = FakeHttp::start(|_| FakeResponse::new(302, &[], Vec::new()));
        let url = format!("{}/x", none.origin());
        assert_eq!(
            http.get_following_one_redirect(&url, None, T, 100).err(),
            Some(TransportError::Protocol)
        );
    }

    #[test]
    fn status_mapping_table() {
        let r = RemoteReply::for_test;
        let rl = |s| TransportError::RateLimited { retry_after_s: s };
        for (reply, want) in [
            (r(401, None), TransportError::Unauthorized),
            (r(403, None), TransportError::Forbidden),
            (r(404, None), TransportError::NotFound),
            (r(408, None), TransportError::Timeout),
            (r(429, Some(30)), rl(30)),
            (r(429, None), rl(DEFAULT_RETRY_AFTER_S)),
            (r(503, Some(5)), rl(5)),
            (r(503, None), TransportError::Unreachable),
            (r(507, None), TransportError::QuotaExceeded),
            (r(502, None), TransportError::Unreachable),
            (r(504, None), TransportError::Unreachable),
            (r(500, None), TransportError::Io),
            (r(599, None), TransportError::Io),
            (r(400, None), TransportError::Protocol),
            (r(412, None), TransportError::Protocol),
            (r(302, None), TransportError::Protocol),
        ] {
            assert_eq!(status_error(&reply), want, "{}", reply.status);
        }
    }

    #[test]
    fn retry_after_is_clamped() {
        let http = RemoteHttp::new_loopback().unwrap();
        for (sent, want) in [("0", 1), ("999999", 86_400), ("17", 17)] {
            let server = FakeHttp::start(move |_| {
                FakeResponse::new(429, &[("Retry-After", sent)], Vec::new())
            });
            let reply = get(&http, &format!("{}/x", server.origin()), 100).unwrap();
            assert_eq!(
                status_error(&reply),
                TransportError::RateLimited {
                    retry_after_s: want
                }
            );
        }
    }

    #[test]
    fn tls_version_error_is_classified() {
        let version = rustls::Error::AlertReceived(rustls::AlertDescription::ProtocolVersion);
        assert!(classify_tls(&version));
        let incompatible =
            rustls::Error::PeerIncompatible(rustls::PeerIncompatible::Tls12NotOffered);
        assert!(classify_tls(&incompatible));
        // Wrapped in an `io::Error`, as a connector reports it.
        let wrapped = std::io::Error::other(incompatible);
        assert!(classify_tls(&wrapped));
        // Other TLS and I/O errors are not version errors.
        assert!(!classify_tls(&rustls::Error::AlertReceived(
            rustls::AlertDescription::BadCertificate
        )));
        assert!(!classify_tls(&rustls::Error::General("x".to_owned())));
        assert!(!classify_tls(&std::io::Error::other("plain")));
    }

    #[test]
    fn error_bodies_are_cut_and_headers_are_kept() {
        let http = RemoteHttp::new_loopback().unwrap();
        let server = FakeHttp::start(|_| {
            FakeResponse::new(500, &[("X-Thing", "one")], vec![b'x'; MAX_ERROR_BODY * 2])
        });
        let reply = get(&http, &format!("{}/x", server.origin()), 10).unwrap();
        assert_eq!(reply.body.len(), MAX_ERROR_BODY);
        assert_eq!(reply.header("x-thing"), Some("one"));
        assert_eq!(status_error(&reply), TransportError::Io);
    }
}
