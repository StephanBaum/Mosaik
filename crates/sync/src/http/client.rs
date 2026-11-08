//! The HTTP layer under the hosted transport and the hosted client functions:
//! the server URL rules, the TLS 1.3 only configuration, one request with a
//! size cap on the response, and the mapping of an error response to a
//! [`TransportError`].
//!
//! Nothing here logs. The session token is sent as a sensitive header value
//! and never appears in an error. The `Authorization` value is built in a
//! zeroizing string and kept as one sensitive [`Bearer`] per transport; the
//! copies that `reqwest` and `hyper` make while they send it are outside this
//! crate's reach (`THREAT_MODEL` R57).

use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{KeyGeneration, SessionToken};
use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use reqwest::redirect::Policy;
use rustls::ClientConfig;
use rustls_platform_verifier::BuilderVerifierExt;
use serde::Serialize;
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::transport::{Rejection, TransportError};
use crate::wire::json::sharing::{
    ConflictDetail, CooldownDetail, InviteStaleDetail, WaitingDetail,
};
use crate::wire::json::{
    self, AccountVersionDetail, ErrorBody, GenerationDetail, InfoResponse, MAX_RESPONSE_LEN,
    PROTOCOL_VERSION, codes,
};
use crate::wire::origin;

/// Connecting to the server.
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// An ordinary request.
pub(super) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// The longest long poll.
pub(super) const POLL_TIMEOUT: Duration = Duration::from_secs(40);
/// An attachment transfer.
pub(super) const ATTACHMENT_TIMEOUT: Duration = Duration::from_secs(600);
/// The most of an error body that is read.
const MAX_ERROR_LEN: usize = 64 * 1024;
/// `Retry-After` when the server sends none (or something unreadable).
const DEFAULT_RETRY_AFTER_S: u32 = 60;
/// The most a `Retry-After` can ask for (one day).
const MAX_RETRY_AFTER_S: u32 = 86_400;

/// The canonical `https://host[:port]` origin of a server URL
/// ([`origin::canonical_origin`], shared with the server).
///
/// Lowercase, no path, query, fragment or credentials, the default port
/// removed. `http` is accepted only for `127.0.0.1`, `[::1]` and
/// `localhost` (M6 spec §6.1).
///
/// # Errors
/// [`TransportError::InsecureUrl`] for `http` to another host or any other
/// scheme than `https`; [`TransportError::InvalidUrl`] for anything that is
/// not `scheme://host[:port]`.
pub fn canonical_origin(base: &str) -> Result<String, TransportError> {
    origin::canonical_origin(base).map_err(|e| match e {
        origin::OriginError::Insecure => TransportError::InsecureUrl,
        _ => TransportError::InvalidUrl,
    })
}

/// A session token as 32 bytes. [`SessionToken`] cannot be rebuilt from
/// bytes, so a client that keeps the token between runs (the desktop's
/// keystore) holds a `StoredToken`: zeroizing, redacted in `Debug`, not
/// `Clone`. Both types are accepted wherever a token is.
pub struct StoredToken(Zeroizing<[u8; 32]>);

impl StoredToken {
    /// Moves `bytes` in and zeroizes the source array.
    #[must_use]
    pub fn from_array(bytes: &mut [u8; 32]) -> Self {
        let token = Self(Zeroizing::new(*bytes));
        zeroize::Zeroize::zeroize(bytes);
        token
    }

    /// A copy of a token the session just granted, for storing it.
    #[must_use]
    pub fn from_session(token: &SessionToken) -> Self {
        Self(Zeroizing::new(*token.expose()))
    }
}

impl core::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("StoredToken([REDACTED])")
    }
}

/// A session token's bytes, for the `Authorization` header. Never log them.
pub trait Token {
    /// The 32 token bytes.
    fn expose_bytes(&self) -> &[u8; 32];
}

impl Token for SessionToken {
    fn expose_bytes(&self) -> &[u8; 32] {
        self.expose()
    }
}

impl Token for StoredToken {
    fn expose_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// The `Authorization: Bearer` header value of a session token: built in a
/// zeroizing string, marked sensitive (reqwest keeps it out of its debug
/// output and hpack tables) and kept for the life of the transport.
pub(super) struct Bearer(HeaderValue);

impl Bearer {
    pub(super) fn new(token: &impl Token) -> Result<Self, TransportError> {
        // Straight into zeroizing buffers: no temporary `String`.
        let mut text: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(64));
        text.extend_from_slice(b"Bearer ");
        let mut encoded = Zeroizing::new([0u8; 64]);
        let n = Base64UrlUnpadded::encode(token.expose_bytes(), encoded.as_mut_slice())
            .map_err(|_| TransportError::Io)?
            .len();
        text.extend_from_slice(encoded.get(..n).ok_or(TransportError::Io)?);
        let mut value = HeaderValue::from_bytes(&text).map_err(|_| TransportError::Io)?;
        value.set_sensitive(true);
        Ok(Self(value))
    }
}

/// The TLS versions of the client: 1.3 only (ADR-0046).
static TLS_VERSIONS: [&rustls::SupportedProtocolVersion; 1] = [&rustls::version::TLS13];

pub(super) fn tls_versions() -> &'static [&'static rustls::SupportedProtocolVersion] {
    &TLS_VERSIONS
}

/// The client's TLS configuration: the `ring` provider, TLS 1.3 only, the
/// operating system's trust store and no client certificate. (`rustls` is
/// built with its `tls12` feature, which hyper-rustls switches on; this
/// configuration offers TLS 1.3 only whatever the features say.)
pub(super) fn tls_config() -> Result<ClientConfig, TransportError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    Ok(ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(tls_versions())
        .map_err(|_| TransportError::Io)?
        .with_platform_verifier()
        .map_err(|_| TransportError::Io)?
        .with_no_client_auth())
}

/// A request body.
pub(super) enum Body<'a> {
    None,
    Json(Vec<u8>),
    Bytes(&'a [u8]),
}

/// An answer: status, `Retry-After` and the (capped) body.
pub(super) struct Reply {
    pub(super) status: u16,
    retry_after_s: Option<u32>,
    pub(super) body: Vec<u8>,
}

impl Reply {
    fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The error body of a failure, if it is one.
    pub(super) fn error_body(&self) -> Option<ErrorBody> {
        json::parse(&self.body, MAX_ERROR_LEN).ok()
    }

    /// The error of a failure answer.
    pub(super) fn error(&self) -> TransportError {
        map_failure(self.status, self.retry_after_s, self.error_body().as_ref())
    }

    /// The JSON body of a success with one of the `ok` statuses; any other
    /// answer is an error.
    pub(super) fn json<T: DeserializeOwned>(&self, ok: &[u16]) -> Result<(u16, T), TransportError> {
        if !ok.contains(&self.status) {
            return Err(self.failure());
        }
        let value =
            json::parse(&self.body, MAX_RESPONSE_LEN).map_err(|_| TransportError::Protocol)?;
        Ok((self.status, value))
    }

    /// A 2xx answer that is not one of the expected ones is a protocol
    /// error; anything else maps by its code.
    pub(super) fn failure(&self) -> TransportError {
        if self.is_success() {
            TransportError::Protocol
        } else {
            self.error()
        }
    }
}

/// `Retry-After` in whole seconds; the HTTP-date form is not honoured.
fn parse_retry_after(value: &HeaderValue) -> Option<u32> {
    let s = value.to_str().ok()?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<u32>().ok().map(|n| n.clamp(1, MAX_RETRY_AFTER_S))
}

fn map_failure(
    status: u16,
    retry_after_s: Option<u32>,
    body: Option<&ErrorBody>,
) -> TransportError {
    let rate_limited = TransportError::RateLimited {
        retry_after_s: retry_after_s.unwrap_or(DEFAULT_RETRY_AFTER_S),
    };
    let rejected = TransportError::Rejected;
    match body.map(|b| b.error.as_str()) {
        Some(codes::UNAUTHORIZED) => TransportError::Unauthorized,
        Some(codes::FRESH_LOGIN_REQUIRED) => TransportError::FreshLoginRequired,
        Some(codes::FORBIDDEN | codes::VAULT_EXISTS) => TransportError::Forbidden,
        Some(codes::NOT_FOUND) => TransportError::NotFound,
        Some(codes::RATE_LIMITED) => rate_limited,
        Some(codes::QUOTA_EXCEEDED) => TransportError::QuotaExceeded,
        Some(codes::GENERATION_MISMATCH) => body
            .and_then(ErrorBody::detail_as::<GenerationDetail>)
            .and_then(|d| KeyGeneration::new(d.current_generation).ok())
            .map_or(TransportError::Protocol, |current| {
                TransportError::GenerationMismatch { current }
            }),
        Some(codes::ACCOUNT_VERSION_MISMATCH) => body
            .and_then(ErrorBody::detail_as::<AccountVersionDetail>)
            .map_or(TransportError::Protocol, |d| {
                rejected(Rejection::AccountVersionMismatch {
                    account_version: d.account_version,
                })
            }),
        Some(codes::LOGIN_FAILED) => rejected(Rejection::LoginFailed),
        Some(codes::DEVICE_REVOKED) => rejected(Rejection::DeviceRevoked),
        Some(codes::TWO_FACTOR_INVALID) => rejected(Rejection::TwoFactorInvalid),
        Some(codes::ACCOUNT_EXISTS) => rejected(Rejection::AccountExists),
        Some(codes::REGISTRATION_CLOSED) => rejected(Rejection::RegistrationClosed),
        Some(codes::INVALID_INVITE) => rejected(Rejection::InvalidInvite),
        Some(codes::INVALID_EMAIL) => rejected(Rejection::InvalidEmail),
        Some(codes::BAD_REQUEST) => rejected(Rejection::BadRequest),
        Some(codes::TOO_LARGE) => rejected(Rejection::TooLarge),
        Some(codes::ATTACHMENT_EXISTS) => rejected(Rejection::AttachmentExists),
        Some(codes::CHECKPOINT_STALE) => rejected(Rejection::CheckpointStale),
        Some(codes::TOO_MANY_POLLS) => rejected(Rejection::TooManyPolls),
        Some(codes::DEVICE_IS_CURRENT) => rejected(Rejection::DeviceIsCurrent),
        Some(codes::ROLE_FORBIDDEN) => rejected(Rejection::RoleForbidden),
        Some(codes::MEMBERS_MISMATCH) => rejected(Rejection::MembersMismatch),
        Some(codes::IDENTITY_EXISTS) => rejected(Rejection::IdentityExists),
        Some(codes::CONFLICT) => rejected(Rejection::Conflict {
            current_seq: body
                .and_then(ErrorBody::detail_as::<ConflictDetail>)
                .map(|d| d.current_seq),
        }),
        Some(codes::INVITE_EXISTS) => rejected(Rejection::InviteExists),
        Some(codes::INVITE_STALE) => body
            .and_then(ErrorBody::detail_as::<InviteStaleDetail>)
            .map_or(TransportError::Protocol, |d| {
                rejected(Rejection::InviteStale {
                    current_generation: d.current_generation,
                })
            }),
        Some(codes::INVITE_EXPIRED) => rejected(Rejection::InviteExpired),
        Some(codes::MEMBERS_LIMIT | codes::INVITE_LIMIT) => rejected(Rejection::InviteLimit),
        Some(codes::ALREADY_MEMBER) => rejected(Rejection::AlreadyMember),
        Some(codes::REINVITE_BLOCKED) => rejected(Rejection::ReinviteBlocked),
        Some(codes::OWNER_CANNOT_LEAVE) => rejected(Rejection::OwnerCannotLeave),
        Some(codes::NO_SHARE) => rejected(Rejection::NoShare),
        Some(codes::EMERGENCY_WAITING) => rejected(Rejection::EmergencyWaiting {
            release_at: body
                .and_then(ErrorBody::detail_as::<WaitingDetail>)
                .map(|d| d.release_at),
        }),
        Some(codes::EMERGENCY_COOLDOWN) => body
            .and_then(ErrorBody::detail_as::<CooldownDetail>)
            .map_or(TransportError::Protocol, |d| {
                rejected(Rejection::EmergencyCooldown { until: d.until })
            }),
        Some(codes::EMERGENCY_RELEASED) => rejected(Rejection::EmergencyReleased),
        Some(codes::EMERGENCY_NOT_REQUESTED) => rejected(Rejection::EmergencyNotRequested),
        Some(codes::EMERGENCY_OWNER_ONLY) => rejected(Rejection::EmergencyOwnerOnly),
        _ => match status {
            401 => TransportError::Unauthorized,
            408 => TransportError::Timeout,
            429 => rate_limited,
            502..=504 => TransportError::Unreachable,
            500..=599 => TransportError::Io,
            _ => TransportError::Protocol,
        },
    }
}

fn map_reqwest(e: &reqwest::Error) -> TransportError {
    if e.is_timeout() {
        TransportError::Timeout
    } else if e.is_connect() {
        TransportError::Unreachable
    } else {
        TransportError::Io
    }
}

/// `localhost` and its port, if the origin names it: it is pinned to
/// `127.0.0.1` (an IP literal origin needs no resolution).
fn pinned_loopback(origin: &str) -> Option<(&'static str, u16)> {
    let (scheme, rest) = origin.split_once("://")?;
    let (host, port) = match rest.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().ok()?),
        None => (rest, if scheme == "https" { 443 } else { 80 }),
    };
    (host == "localhost").then_some(("localhost", port))
}

/// A blocking client for one server origin.
pub(super) struct Http {
    client: Client,
    origin: String,
}

impl Http {
    /// A client for `origin` (a canonical origin from [`canonical_origin`]).
    pub(super) fn new(origin: &str) -> Result<Self, TransportError> {
        let secure = origin.starts_with("https://");
        let mut builder = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(Policy::none())
            .tls_backend_preconfigured(tls_config()?);
        builder = if secure {
            builder.https_only(true)
        } else {
            // A loopback server is never behind a proxy.
            builder.no_proxy()
        };
        if let Some((host, port)) = pinned_loopback(origin) {
            // `localhost` is looked up by the system resolver otherwise,
            // and a plain-http loopback connection must not depend on it.
            builder = builder.resolve(host, SocketAddr::from((Ipv4Addr::LOCALHOST, port)));
        }
        let client = builder.build().map_err(|_| TransportError::Io)?;
        Ok(Self {
            client,
            origin: origin.to_owned(),
        })
    }

    /// Sends one request and reads the answer, at most `cap` bytes of it
    /// when it is a success.
    pub(super) fn send(
        &self,
        method: Method,
        path: &str,
        auth: Option<&Bearer>,
        body: Body<'_>,
        timeout: Duration,
        cap: usize,
    ) -> Result<Reply, TransportError> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.origin))
            .timeout(timeout);
        if let Some(bearer) = auth {
            request = request.header(AUTHORIZATION, bearer.0.clone());
        }
        request = match body {
            Body::None => request,
            Body::Json(bytes) => request.header(CONTENT_TYPE, "application/json").body(bytes),
            Body::Bytes(bytes) => request
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(bytes.to_vec()),
        };
        let response = request.send().map_err(|e| map_reqwest(&e))?;
        let status = response.status();
        let retry_after_s = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(parse_retry_after);
        let ok = status.is_success();
        let limit = if ok { cap } else { MAX_ERROR_LEN };
        let bytes = read_capped(response, limit, ok)?;
        Ok(Reply {
            status: status.as_u16(),
            retry_after_s,
            body: bytes,
        })
    }

    /// A JSON request with a JSON answer.
    pub(super) fn call<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        auth: Option<&Bearer>,
        request: Option<&Req>,
        ok: &[u16],
    ) -> Result<(u16, Resp), TransportError> {
        self.send(
            method,
            path,
            auth,
            json_body(request)?,
            REQUEST_TIMEOUT,
            MAX_RESPONSE_LEN,
        )?
        .json(ok)
    }

    /// `GET /v1/info`, checked: the protocol is ours and the server names
    /// the origin the client asked for.
    pub(super) fn info(&self) -> Result<InfoResponse, TransportError> {
        let (_, info): (u16, InfoResponse) =
            self.call(Method::GET, "/v1/info", None, None::<&json::Empty>, &[200])?;
        if info.protocol != PROTOCOL_VERSION {
            return Err(TransportError::Incompatible);
        }
        if info.server_identity != self.origin {
            return Err(TransportError::ServerMismatch);
        }
        Ok(info)
    }
}

/// The JSON body of an optional request value.
pub(super) fn json_body<Req: Serialize>(
    request: Option<&Req>,
) -> Result<Body<'static>, TransportError> {
    match request {
        Some(r) => Ok(Body::Json(json::to_vec(r).map_err(|_| TransportError::Io)?)),
        None => Ok(Body::None),
    }
}

/// Reads a response body of at most `cap` bytes. A success that is longer
/// is a protocol error; a failure that is longer is cut (its body is not
/// needed to map it).
fn read_capped(
    response: reqwest::blocking::Response,
    cap: usize,
    strict: bool,
) -> Result<Vec<u8>, TransportError> {
    let cap64 = u64::try_from(cap).unwrap_or(u64::MAX);
    if strict
        && let Some(len) = response.content_length()
        && len > cap64
    {
        return Err(TransportError::Protocol);
    }
    let mut out = Vec::new();
    if let Err(e) = response.take(cap64.saturating_add(1)).read_to_end(&mut out) {
        return Err(if e.kind() == std::io::ErrorKind::TimedOut {
            TransportError::Timeout
        } else {
            TransportError::Io
        });
    }
    if out.len() > cap {
        if strict {
            return Err(TransportError::Protocol);
        }
        out.truncate(cap);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_config_is_tls13_only() {
        let versions = tls_versions();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, rustls::ProtocolVersion::TLSv1_3);
        // The builder accepts the platform verifier without a process default.
        assert!(tls_config().is_ok());
    }

    #[test]
    fn origins_are_canonical_and_https_only() {
        assert_eq!(
            canonical_origin("https://Sync.Example.com").as_deref(),
            Ok("https://sync.example.com")
        );
        assert_eq!(
            canonical_origin("https://sync.example.com:8443").as_deref(),
            Ok("https://sync.example.com:8443")
        );
        for ok in [
            "http://127.0.0.1:8743",
            "http://localhost",
            "http://[::1]:1",
        ] {
            assert!(canonical_origin(ok).is_ok(), "{ok}");
        }
        for insecure in [
            "http://sync.example.com",
            "http://127.0.0.1.example.com",
            "ftp://sync.example.com",
        ] {
            assert_eq!(
                canonical_origin(insecure),
                Err(TransportError::InsecureUrl),
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
            "https://sync.example.com:99999",
            "https://sync.example.com:",
            "https://.example.com",
            "https://exämple.com",
            "https://sync example.com",
        ] {
            assert_eq!(
                canonical_origin(bad),
                Err(TransportError::InvalidUrl),
                "{bad}"
            );
        }
    }

    #[test]
    fn localhost_is_pinned_and_nothing_else() {
        assert_eq!(
            pinned_loopback("http://localhost:8743"),
            Some(("localhost", 8743))
        );
        assert_eq!(pinned_loopback("http://localhost"), Some(("localhost", 80)));
        assert_eq!(
            pinned_loopback("https://localhost"),
            Some(("localhost", 443))
        );
        assert_eq!(pinned_loopback("http://127.0.0.1:1"), None);
        assert_eq!(pinned_loopback("http://[::1]:1"), None);
        assert_eq!(pinned_loopback("https://sync.example.com"), None);
    }

    #[test]
    fn the_bearer_value_is_sensitive() {
        let token = mosaic_core::sync_auth::session_token(&[7; 64]).unwrap();
        let bearer = Bearer::new(&token).unwrap();
        assert!(bearer.0.is_sensitive());
        assert!(bearer.0.as_bytes().starts_with(b"Bearer "));
        assert_eq!(format!("{:?}", bearer.0), "Sensitive");
    }

    #[test]
    fn retry_after_is_whole_seconds_clamped() {
        let h = |s: &str| HeaderValue::from_str(s).unwrap();
        assert_eq!(parse_retry_after(&h("7")), Some(7));
        assert_eq!(parse_retry_after(&h("0")), Some(1));
        assert_eq!(parse_retry_after(&h("999999")), Some(MAX_RETRY_AFTER_S));
        assert_eq!(parse_retry_after(&h("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(parse_retry_after(&h("-3")), None);
    }
}
