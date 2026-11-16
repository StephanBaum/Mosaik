//! What the OAuth providers share: reading a token answer and building a
//! bearer header. Every token lives in a zeroizing buffer; the header value is
//! marked sensitive. Nothing here keeps or reports what the provider sent
//! beyond the tokens.

use reqwest::Method;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use zeroize::Zeroizing;

use super::MAX_SMALL_JSON;
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest};
use super::oauth::form;
use crate::http::client::REQUEST_TIMEOUT;
use crate::transport::TransportError;

/// The longest token that is accepted.
const MAX_TOKEN: usize = 8 * 1024;

/// The tokens of a token answer.
pub(crate) struct Tokens {
    pub(crate) access: Zeroizing<String>,
    /// A refresh token, if the answer carried one.
    pub(crate) refresh: Option<Zeroizing<String>>,
}

#[derive(Deserialize)]
struct Wire {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    token_type: String,
}

fn token_ok(t: &str) -> bool {
    !t.is_empty() && t.len() <= MAX_TOKEN && t.bytes().all(|b| (b' '..=b'~').contains(&b))
}

/// Reads a successful token answer.
///
/// # Errors
/// [`TransportError::Protocol`] for anything that is not a bearer token
/// answer with a usable access token.
pub(crate) fn parse_tokens(body: Vec<u8>) -> Result<Tokens, TransportError> {
    let body = Zeroizing::new(body);
    let mut wire: Wire = serde_json::from_slice(&body).map_err(|_| TransportError::Protocol)?;
    let access = Zeroizing::new(std::mem::take(&mut wire.access_token));
    let refresh = wire.refresh_token.take().map(Zeroizing::new);
    if !wire.token_type.eq_ignore_ascii_case("bearer")
        || !token_ok(&access)
        || refresh.as_ref().is_some_and(|r| !token_ok(r))
    {
        return Err(TransportError::Protocol);
    }
    Ok(Tokens { access, refresh })
}

/// `Bearer <token>`, marked sensitive.
///
/// # Errors
/// [`TransportError::Io`] for a token that cannot be a header value.
pub(crate) fn bearer(token: &str) -> Result<HeaderValue, TransportError> {
    let mut text = Zeroizing::new(String::from("Bearer "));
    text.push_str(token);
    let mut value = HeaderValue::from_bytes(text.as_bytes()).map_err(|_| TransportError::Io)?;
    value.set_sensitive(true);
    Ok(value)
}

/// `POST url` with a form of `pairs` (which may hold a secret) to a token
/// endpoint.
///
/// # Errors
/// As [`RemoteHttp::send`].
pub(crate) fn post_form(
    http: &RemoteHttp,
    url: &str,
    pairs: &[(&str, &str)],
) -> Result<RemoteReply, TransportError> {
    http.send(RemoteRequest {
        method: Method::POST,
        url,
        headers: &[],
        body: RemoteBody::Form(form(pairs)),
        timeout: REQUEST_TIMEOUT,
        cap: MAX_SMALL_JSON,
    })
}

/// Whether a failure answer of a token endpoint says the grant is gone
/// (`invalid_grant`, or a 401).
pub(crate) fn grant_is_gone(reply: &RemoteReply) -> bool {
    reply.status == 401
        || (reply.status == 400 && String::from_utf8_lossy(&reply.body).contains("invalid_grant"))
}
