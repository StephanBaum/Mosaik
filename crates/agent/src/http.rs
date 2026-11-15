//! The HTTPS sender of the request path (agent access design §7.2, ADR-0046, ADR-0159).
//!
//! Blocking `reqwest` over `rustls` with the `ring` provider, TLS 1.3 only, the operating
//! system's trust store and no client certificate (the settings of the sync client). Redirects are
//! never followed, no proxy is used, only `https` is accepted, there is no cookie store, and the
//! response is capped: the headers at 64 KiB, the body at the request's `max_bytes` plus the
//! masker's hold-back, the whole exchange at 60 s (10 s to connect).
//!
//! The secret header is a sensitive `HeaderValue` and the [`PreparedRequest`] is consumed and
//! dropped before the body is read. Copies of the value inside `reqwest` and `hyper` buffers are a
//! recorded residual (ADR-0159).

use std::io::Read as _;
use std::net::SocketAddr;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, RootCertStore};

pub use crate::response::{RawResponse, SendError};

use crate::binding::Method;
use crate::request::PreparedRequest;

/// Time allowed to connect.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Time allowed for the whole exchange, body included.
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);
/// Most bytes of response header names and values together.
pub const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;

/// TLS 1.3 only (ADR-0046, ADR-0159 Q2).
static TLS_VERSIONS: [&rustls::SupportedProtocolVersion; 1] = [&rustls::version::TLS13];

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// The production TLS configuration: `ring`, TLS 1.3 only, the OS trust, no client certificate.
fn platform_config() -> Result<ClientConfig, SendError> {
    use rustls_platform_verifier::BuilderVerifierExt as _;
    Ok(ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&TLS_VERSIONS)
        .map_err(|_| SendError::Other)?
        .with_platform_verifier()
        .map_err(|_| SendError::Other)?
        .with_no_client_auth())
}

/// A configuration that trusts only `anchors` (tests).
#[cfg(feature = "test-util")]
fn anchors_config(anchors: &[&[u8]]) -> Result<ClientConfig, SendError> {
    let provider = provider();
    let mut roots = RootCertStore::empty();
    for der in anchors {
        roots
            .add(CertificateDer::from(der.to_vec()))
            .map_err(|_| SendError::Other)?;
    }
    let verifier =
        WebPkiServerVerifier::builder_with_provider(Arc::new(roots), Arc::clone(&provider))
            .build()
            .map_err(|_| SendError::Other)?;
    Ok(ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&TLS_VERSIONS)
        .map_err(|_| SendError::Other)?
        .with_webpki_verifier(verifier)
        .with_no_client_auth())
}

/// Sends the prepared requests of agents.
pub struct HttpsSender {
    client: Client,
    total: Duration,
}

impl core::fmt::Debug for HttpsSender {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("HttpsSender")
    }
}

fn build(
    config: ClientConfig,
    timeout: Duration,
    resolve: &[(&str, SocketAddr)],
) -> Result<HttpsSender, SendError> {
    let mut builder = Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(timeout)
        .redirect(Policy::none())
        .no_proxy()
        .https_only(true)
        .referer(false)
        .tls_backend_preconfigured(config);
    for (host, addr) in resolve {
        builder = builder.resolve(host, *addr);
    }
    Ok(HttpsSender {
        client: builder.build().map_err(|_| SendError::Other)?,
        total: timeout,
    })
}

impl HttpsSender {
    /// The production sender.
    ///
    /// # Errors
    /// [`SendError::Other`] if the TLS configuration or the client cannot be built.
    pub fn new() -> Result<Self, SendError> {
        build(platform_config()?, TOTAL_TIMEOUT, &[])
    }

    /// A sender that trusts only `ca_der` and resolves the given hosts to the given addresses
    /// (the port of a URL is the one in the URL). Everything else is as in production.
    ///
    /// # Errors
    /// [`SendError::Other`].
    #[cfg(feature = "test-util")]
    pub fn for_test(ca_der: &[u8], resolve: &[(&str, SocketAddr)]) -> Result<Self, SendError> {
        build(anchors_config(&[ca_der])?, TOTAL_TIMEOUT, resolve)
    }

    /// [`Self::for_test`] with another total time limit.
    ///
    /// # Errors
    /// [`SendError::Other`].
    #[cfg(feature = "test-util")]
    pub fn for_test_with_timeout(
        ca_der: &[u8],
        resolve: &[(&str, SocketAddr)],
        total: Duration,
    ) -> Result<Self, SendError> {
        build(anchors_config(&[ca_der])?, total, resolve)
    }

    /// Sends `req` and reads at most `req.max_bytes + holdback` bytes of the body (`holdback` is
    /// the request's `Masker::holdback()`); `truncated` says there was more. The request, and with
    /// it the secret header, is dropped before the body is read. The whole exchange is bounded by
    /// the total time limit: it runs on a worker thread, and this call stops waiting at the
    /// limit (the worker also checks the limit between reads and ends on its own).
    ///
    /// # Errors
    /// [`SendError`].
    pub fn send(&self, req: PreparedRequest, holdback: usize) -> Result<RawResponse, SendError> {
        let (tx, rx) = mpsc::channel();
        let client = self.client.clone();
        let deadline = Instant::now() + self.total;
        thread::Builder::new()
            .name("mosaic-agent-http".to_owned())
            .spawn(move || {
                // The receiver is gone when the caller timed out; there is nobody to tell.
                let _ = tx.send(exchange(&client, req, holdback, deadline));
            })
            .map_err(|_| SendError::Other)?;
        match rx.recv_timeout(self.total) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(SendError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(SendError::Other),
        }
    }
}

fn exchange(
    client: &Client,
    req: PreparedRequest,
    holdback: usize,
    deadline: Instant,
) -> Result<RawResponse, SendError> {
    let PreparedRequest {
        method,
        url,
        headers,
        secret_header,
        body,
        max_bytes,
    } = req;
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| SendError::Other)?;
        let value = HeaderValue::from_bytes(value.as_bytes()).map_err(|_| SendError::Other)?;
        map.append(name, value);
    }
    if let Some((name, value)) = secret_header {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| SendError::Other)?;
        let mut value = HeaderValue::from_bytes(value.as_bytes()).map_err(|_| SendError::Other)?;
        value.set_sensitive(true);
        map.append(name, value);
    }
    let mut builder = client.request(to_reqwest(method), url).headers(map);
    if let Some(body) = body {
        builder = builder.body(body);
    }
    let response = builder.send().map_err(|e| map_error(&e))?;
    read_response(response, max_bytes.saturating_add(holdback), deadline)
}

const fn to_reqwest(method: Method) -> reqwest::Method {
    match method {
        Method::Get => reqwest::Method::GET,
        Method::Head => reqwest::Method::HEAD,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
    }
}

fn read_response(
    mut response: reqwest::blocking::Response,
    cap: usize,
    deadline: Instant,
) -> Result<RawResponse, SendError> {
    let status = response.status().as_u16();
    let mut header_bytes = 0usize;
    let mut headers = Vec::with_capacity(response.headers().len());
    for (name, value) in response.headers() {
        header_bytes = header_bytes
            .saturating_add(name.as_str().len())
            .saturating_add(value.as_bytes().len());
        if header_bytes > MAX_RESPONSE_HEADER_BYTES {
            return Err(SendError::ResponseTooLarge);
        }
        headers.push((name.as_str().to_owned(), value.as_bytes().to_vec()));
    }
    // One byte past the cap tells whether there was more.
    let limit = cap.saturating_add(1);
    let mut body = Vec::new();
    let mut chunk = [0u8; 8192];
    while body.len() < limit {
        if Instant::now() >= deadline {
            return Err(SendError::Timeout);
        }
        let want = chunk.len().min(limit - body.len());
        let n = response
            .read(chunk.get_mut(..want).unwrap_or_default())
            .map_err(|e| map_io(&e))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(chunk.get(..n).unwrap_or_default());
    }
    let truncated = body.len() > cap;
    body.truncate(cap);
    Ok(RawResponse {
        status,
        headers,
        body,
        truncated,
    })
}

/// The first error in the chain of `err` that satisfies `pick`.
fn find_in_chain<T>(
    err: &(dyn std::error::Error + 'static),
    pick: &dyn Fn(&(dyn std::error::Error + 'static)) -> Option<T>,
) -> Option<T> {
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur {
        if let Some(found) = pick(e) {
            return Some(found);
        }
        if let Some(inner) = e
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::get_ref)
            && let Some(found) = find_in_chain(inner, pick)
        {
            return Some(found);
        }
        cur = e.source();
    }
    None
}

fn is_tls(err: &(dyn std::error::Error + 'static)) -> bool {
    find_in_chain(err, &|e| e.downcast_ref::<rustls::Error>().map(|_| ())).is_some()
}

fn is_timeout(err: &(dyn std::error::Error + 'static)) -> bool {
    find_in_chain(err, &|e| {
        let by_reqwest = e
            .downcast_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout);
        let by_io = e
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::TimedOut);
        (by_reqwest || by_io).then_some(())
    })
    .is_some()
}

fn map_error(err: &reqwest::Error) -> SendError {
    if err.is_timeout() || is_timeout(err) {
        SendError::Timeout
    } else if is_tls(err) {
        SendError::Tls
    } else if err.is_connect() {
        SendError::Connect
    } else {
        SendError::Other
    }
}

fn map_io(err: &std::io::Error) -> SendError {
    if is_timeout(err) {
        SendError::Timeout
    } else if is_tls(err) {
        SendError::Tls
    } else {
        SendError::Other
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn tls13_only_and_platform_roots_in_production_config() {
        assert!(platform_config().is_ok());
        let versions: Vec<_> = TLS_VERSIONS.iter().map(|v| v.version).collect();
        assert_eq!(versions, vec![rustls::ProtocolVersion::TLSv1_3]);
        // The production sender builds with the operating system's verifier.
        assert!(HttpsSender::new().is_ok());
    }
}
