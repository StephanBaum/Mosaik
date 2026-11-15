//! What the HTTPS sender hands to the policy engine: the raw answer (status, headers and the
//! capped body bytes) or why there is none. No value is masked yet at this point; the engine
//! masks before anything leaves it.

use core::fmt;

/// An answer as received.
#[derive(Clone)]
pub struct RawResponse {
    /// The status code.
    pub status: u16,
    /// The headers in order, values as bytes.
    pub headers: Vec<(String, Vec<u8>)>,
    /// The body: at most `max_bytes` plus the masker's hold-back, nothing decoded.
    pub body: Vec<u8>,
    /// Whether the server had more body than was read.
    pub truncated: bool,
}

impl fmt::Debug for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawResponse")
            .field("status", &self.status)
            .field("headers", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl RawResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, v)| core::str::from_utf8(v).ok())
    }

    /// The `Content-Type` header, if present and text.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// The `Content-Encoding` header, if present and text.
    #[must_use]
    pub fn content_encoding(&self) -> Option<&str> {
        self.header("content-encoding")
    }
}

/// Why no answer arrived.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SendError {
    /// Connect or total time ran out.
    #[error("the request timed out")]
    Timeout,
    /// The TLS handshake or the certificate check failed.
    #[error("the TLS handshake failed")]
    Tls,
    /// The connection could not be made.
    #[error("the connection failed")]
    Connect,
    /// The response headers are over the limit.
    #[error("the response is too large")]
    ResponseTooLarge,
    /// Anything else.
    #[error("the request failed")]
    Other,
}
