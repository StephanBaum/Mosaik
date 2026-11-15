//! Request validation and single-slot injection (agent access design §7.2, steps 5 to 8).
//!
//! [`validate`] checks everything the agent supplied against the binding **before** any secret
//! value is read: the URL's exact origin, the methods and paths if the binding narrows them, the
//! headers, the sizes. [`ValidatedRequest::inject`] then places the value in exactly one slot of
//! the request: the `Authorization` header, one named header, or one query parameter. The URL
//! that was checked is the URL that is sent (one parse, never re-parsed from text).
//!
//! A `mosaic://` string in the agent's input is plain text: nothing here expands references.

use core::fmt;

use base64ct::{Base64, Encoding as _};
use mosaic_core::SecretString;
use mosaic_origin::{agent_destination_allowed, parse_agent_url};
use serde::{Deserialize, Serialize};
use url::Url;
use zeroize::Zeroizing;

use crate::binding::{BasicUser, Binding, Inject, Method, is_forbidden_header, is_token};
use crate::error::AgentErrorCode;

/// Most headers an agent may send.
pub const MAX_AGENT_HEADERS: usize = 32;
/// Longest value of one agent header, in bytes.
pub const MAX_AGENT_HEADER_VALUE: usize = 8 * 1024;
/// Most bytes of all agent header names and values together.
pub const MAX_AGENT_HEADERS_TOTAL: usize = 32 * 1024;
/// Largest request body, in bytes.
pub const MAX_REQUEST_BODY: usize = 1024 * 1024;
/// The default `maxBytes` of a response.
pub const DEFAULT_MAX_BYTES: u32 = 256 * 1024;
/// The largest `maxBytes` of a response.
pub const MAX_MAX_BYTES: u32 = 1024 * 1024;
/// The longest path the log keeps, in characters.
pub const MAX_LOG_PATH_CHARS: usize = 512;

/// What the agent asked for (the `http_request` tool's input). It is not a vault secret but may be
/// large, so `Debug` prints sizes only.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestInput {
    /// The method, upper case (`GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`).
    pub method: String,
    /// The absolute `https` URL.
    pub url: String,
    /// Extra headers, in order.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// The body, UTF-8 text.
    #[serde(default)]
    pub body: Option<String>,
    /// Cap of the response body returned.
    #[serde(default)]
    pub max_bytes: Option<u32>,
}

impl fmt::Debug for HttpRequestInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequestInput")
            .field("method", &self.method)
            .field("headers", &self.headers.len())
            .field("body_bytes", &self.body.as_ref().map(String::len))
            .field("max_bytes", &self.max_bytes)
            .finish_non_exhaustive()
    }
}

/// A request that passed every check that needs no secret.
pub struct ValidatedRequest {
    /// The method.
    pub method: Method,
    /// The checked URL; this exact value is sent.
    pub url: Url,
    /// The origin of the URL (`https://host[:port]`).
    pub origin: String,
    /// The URL path only (no query, no fragment), at most 512 characters, for the log.
    pub path: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    /// The response body cap, in bytes.
    pub max_bytes: usize,
}

impl fmt::Debug for ValidatedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// A request with the secret in place, ready to send. Not `Clone`; `Debug` is redacted.
#[cfg_attr(
    all(not(feature = "http"), not(test)),
    expect(
        dead_code,
        reason = "only the HTTPS sender (feature `http`) reads the parts"
    )
)]
pub struct PreparedRequest {
    pub(crate) method: Method,
    pub(crate) url: Url,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) secret_header: Option<(String, Zeroizing<String>)>,
    pub(crate) body: Option<String>,
    /// The response body cap, in bytes.
    pub max_bytes: usize,
}

impl fmt::Debug for PreparedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

fn method_allowed(binding: &Binding, method: Option<Method>) -> bool {
    match (&binding.methods, method) {
        (None, _) => true,
        (Some(list), Some(m)) => list.contains(&m),
        (Some(_), None) => false,
    }
}

/// Whether `path` is `prefix` or below it, at a segment boundary: `/v1` allows `/v1` and
/// `/v1/x`, not `/v10`.
fn path_under(prefix: &str, path: &str) -> bool {
    match path.strip_prefix(prefix) {
        None => false,
        Some("") => true,
        Some(rest) => prefix.ends_with('/') || rest.starts_with('/'),
    }
}

/// An encoded slash or backslash in a path could be decoded by the server into a segment
/// boundary the prefix check did not see, so a binding that narrows paths refuses them.
fn hides_a_separator(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("%2f") || lower.contains("%5c")
}

fn header_value_ok(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b == b'\t' || (b >= 0x20 && b != 0x7f))
}

fn injection_header(inject: &Inject) -> Option<&str> {
    match inject {
        Inject::Bearer | Inject::Basic(_) => Some("authorization"),
        Inject::Header { name, .. } => Some(name),
        Inject::Query { .. } => None,
    }
}

fn check_headers(
    binding: &Binding,
    headers: &[(String, String)],
) -> Result<Vec<(String, String)>, AgentErrorCode> {
    if headers.len() > MAX_AGENT_HEADERS {
        return Err(AgentErrorCode::RequestTooLarge);
    }
    let slot = injection_header(&binding.inject);
    let mut total = 0usize;
    let mut out = Vec::with_capacity(headers.len() + 1);
    for (name, value) in headers {
        if value.len() > MAX_AGENT_HEADER_VALUE {
            return Err(AgentErrorCode::RequestTooLarge);
        }
        total = total.saturating_add(name.len()).saturating_add(value.len());
        if total > MAX_AGENT_HEADERS_TOTAL {
            return Err(AgentErrorCode::RequestTooLarge);
        }
        if !is_token(name)
            || !header_value_ok(value)
            || is_forbidden_header(name)
            || slot.is_some_and(|s| s.eq_ignore_ascii_case(name))
        {
            return Err(AgentErrorCode::HeaderNotAllowed);
        }
        // The client asks for plain bytes; an agent cannot make it ask for anything else.
        if name.eq_ignore_ascii_case("accept-encoding") {
            continue;
        }
        out.push((name.clone(), value.clone()));
    }
    out.push(("accept-encoding".to_owned(), "identity".to_owned()));
    Ok(out)
}

fn log_path(url: &Url) -> String {
    url.path().chars().take(MAX_LOG_PATH_CHARS).collect()
}

/// Checks the agent's request against the binding. Reads no secret value.
///
/// # Errors
/// [`AgentErrorCode::DestinationNotAllowed`] (not an exact `https` origin of the binding, an
/// unparsable or over-long URL, or a path outside the binding's paths),
/// [`AgentErrorCode::MethodNotAllowed`], [`AgentErrorCode::HeaderNotAllowed`],
/// [`AgentErrorCode::RequestTooLarge`], [`AgentErrorCode::InvalidRequest`] (`maxBytes`).
pub fn validate(
    binding: &Binding,
    input: &HttpRequestInput,
) -> Result<ValidatedRequest, AgentErrorCode> {
    let url = parse_agent_url(&input.url).map_err(|_| AgentErrorCode::DestinationNotAllowed)?;
    if !binding
        .origins
        .iter()
        .any(|o| agent_destination_allowed(o, &url).is_ok())
    {
        return Err(AgentErrorCode::DestinationNotAllowed);
    }

    let method = Method::parse(&input.method);
    if !method_allowed(binding, method) {
        return Err(AgentErrorCode::MethodNotAllowed);
    }
    if let Some(paths) = &binding.paths
        && (hides_a_separator(url.path()) || !paths.iter().any(|p| path_under(p, url.path())))
    {
        return Err(AgentErrorCode::DestinationNotAllowed);
    }
    let method = method.ok_or(AgentErrorCode::MethodNotAllowed)?;

    let headers = check_headers(binding, &input.headers)?;

    if let Inject::Query { name } = &binding.inject
        && url.query_pairs().any(|(k, _)| k == name.as_str())
    {
        return Err(AgentErrorCode::HeaderNotAllowed);
    }

    if input
        .body
        .as_ref()
        .is_some_and(|b| b.len() > MAX_REQUEST_BODY)
    {
        return Err(AgentErrorCode::RequestTooLarge);
    }
    let max_bytes = match input.max_bytes {
        None => DEFAULT_MAX_BYTES,
        Some(n) if (1..=MAX_MAX_BYTES).contains(&n) => n,
        Some(_) => return Err(AgentErrorCode::InvalidRequest),
    };

    Ok(ValidatedRequest {
        method,
        origin: url.origin().ascii_serialization(),
        path: log_path(&url),
        url,
        headers,
        body: input.body.clone(),
        max_bytes: usize::try_from(max_bytes).map_err(|_| AgentErrorCode::InvalidRequest)?,
    })
}

/// `Basic base64(user ":" value)` in zeroizing buffers (the intermediate `user:value` and the
/// encoded text are both wiped on drop).
fn basic_header(user: &str, value: &str) -> Zeroizing<String> {
    let mut plain = Zeroizing::new(Vec::with_capacity(user.len() + 1 + value.len()));
    plain.extend_from_slice(user.as_bytes());
    plain.push(b':');
    plain.extend_from_slice(value.as_bytes());
    let mut text = Zeroizing::new(vec![0u8; Base64::encoded_len(&plain)]);
    let mut out = Zeroizing::new(String::from("Basic "));
    if let Ok(encoded) = Base64::encode(&plain, &mut text) {
        out.push_str(encoded);
    }
    out
}

impl ValidatedRequest {
    /// Places `value` in the one slot `inject` names and returns the request to send.
    /// `basic_user` is the user name of a Basic binding (the caller reads it from the binding or
    /// the item); a [`BasicUser::SecretIsUser`] binding sends `base64(value ":")` and ignores it.
    #[must_use]
    pub fn inject(
        self,
        inject: &Inject,
        value: &SecretString,
        basic_user: Option<&str>,
    ) -> PreparedRequest {
        let Self {
            method,
            mut url,
            headers,
            body,
            max_bytes,
            ..
        } = self;
        let secret = value.expose_secret();
        let mut secret_header = None;
        match inject {
            Inject::Bearer => {
                let mut v = Zeroizing::new(String::with_capacity(7 + secret.len()));
                v.push_str("Bearer ");
                v.push_str(secret);
                secret_header = Some(("authorization".to_owned(), v));
            }
            Inject::Header { name, prefix } => {
                let mut v = Zeroizing::new(String::with_capacity(prefix.len() + secret.len()));
                v.push_str(prefix);
                v.push_str(secret);
                secret_header = Some((name.clone(), v));
            }
            Inject::Basic(BasicUser::SecretIsUser) => {
                secret_header = Some(("authorization".to_owned(), basic_header(secret, "")));
            }
            Inject::Basic(_) => {
                secret_header = Some((
                    "authorization".to_owned(),
                    basic_header(basic_user.unwrap_or(""), secret),
                ));
            }
            Inject::Query { name } => {
                url.query_pairs_mut().append_pair(name, secret);
            }
        }
        PreparedRequest {
            method,
            url,
            headers,
            secret_header,
            body,
            max_bytes,
        }
    }
}
