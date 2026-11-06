//! The HTTP API (M6 spec §6.2): routes, request parsing and the middleware
//! shared by every route.

mod account;
mod attachments;
mod checkpoints;
mod devices;
mod info;
mod login;
mod poll;
mod prelogin;
mod records;
mod register;
mod vaults;

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{ConnectInfo, FromRequestParts, MatchedPath, Request};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use mosaic_sync::wire::json::{self, JsonError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::AppState;
use crate::error::ApiError;

/// Request timeout of the JSON routes.
const JSON_TIMEOUT: Duration = Duration::from_secs(60);
/// Request timeout of the attachment routes (client: 600 s).
const ATTACHMENT_TIMEOUT: Duration = Duration::from_secs(600);

/// A successful response: a status and a JSON (or binary) body.
pub struct Reply {
    status: StatusCode,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Reply {
    /// 200 with a JSON body.
    pub fn ok<T: Serialize>(value: &T) -> Result<Self, ApiError> {
        Self::json(StatusCode::OK, value)
    }

    /// 201 with a JSON body.
    pub fn created<T: Serialize>(value: &T) -> Result<Self, ApiError> {
        Self::json(StatusCode::CREATED, value)
    }

    /// A JSON body with a status.
    pub fn json<T: Serialize>(status: StatusCode, value: &T) -> Result<Self, ApiError> {
        let body = json::to_vec(value).map_err(|_| ApiError::internal())?;
        Ok(Self {
            status,
            content_type: "application/json",
            body,
        })
    }

    /// 200 `{}`.
    pub fn empty() -> Result<Self, ApiError> {
        Self::ok(&json::Empty {})
    }

    /// A binary body.
    pub const fn bytes(body: Vec<u8>) -> Self {
        Self {
            status: StatusCode::OK,
            content_type: "application/octet-stream",
            body,
        }
    }
}

impl IntoResponse for Reply {
    fn into_response(self) -> Response {
        let mut r = (self.status, self.body).into_response();
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(self.content_type),
        );
        r
    }
}

/// Parses a JSON request body.
pub fn parse_body<T: DeserializeOwned>(body: &[u8], max: usize) -> Result<T, ApiError> {
    json::parse(body, max).map_err(|e| match e {
        JsonError::TooLarge => ApiError::too_large(),
        _ => ApiError::bad_request(),
    })
}

/// A device name: 1 to 64 characters, no control characters, not blank.
pub fn valid_device_name(name: &str) -> bool {
    let n = name.chars().count();
    (1..=64).contains(&n) && !name.chars().any(char::is_control) && !name.trim().is_empty()
}

/// A lowercase hyphenated, non-nil UUID path segment as 16 bytes.
pub fn parse_uuid(s: &str) -> Result<[u8; 16], ApiError> {
    let b = s.as_bytes();
    if b.len() != 36 {
        return Err(ApiError::bad_request());
    }
    let mut out = [0u8; 16];
    let mut n = 0usize;
    let mut i = 0usize;
    while i < 36 {
        if matches!(i, 8 | 13 | 18 | 23) {
            if b.get(i) != Some(&b'-') {
                return Err(ApiError::bad_request());
            }
            i += 1;
            continue;
        }
        let hi = nibble(b.get(i).copied())?;
        let lo = nibble(b.get(i + 1).copied())?;
        *out.get_mut(n).ok_or_else(ApiError::bad_request)? = (hi << 4) | lo;
        n += 1;
        i += 2;
    }
    if out == [0u8; 16] {
        return Err(ApiError::bad_request());
    }
    Ok(out)
}

fn nibble(c: Option<u8>) -> Result<u8, ApiError> {
    match c {
        Some(c @ b'0'..=b'9') => Ok(c - b'0'),
        Some(c @ b'a'..=b'f') => Ok(c - b'a' + 10),
        _ => Err(ApiError::bad_request()),
    }
}

/// The client address for rate limiting: the peer, or, when the peer is a
/// trusted proxy, the last hop of `X-Forwarded-For`. Never stored.
pub struct ClientIp(pub IpAddr);

impl ClientIp {
    /// The bytes that key the rate limiter: IPv4 as is, IPv6 by its /64.
    pub fn key(&self) -> Vec<u8> {
        match self.0 {
            IpAddr::V4(v4) => v4.octets().to_vec(),
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => v4.octets().to_vec(),
                None => v6.octets().get(..8).unwrap_or(&[]).to_vec(),
            },
        }
    }
}

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or(IpAddr::from([0, 0, 0, 0]), |c| c.0.ip());
        if state.trusted_proxies.contains(&peer)
            && let Some(last) = parts
                .headers
                .get_all("x-forwarded-for")
                .iter()
                .next_back()
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
            && let Ok(ip) = last.trim().parse::<IpAddr>()
        {
            return std::future::ready(Ok(Self(ip)));
        }
        std::future::ready(Ok(Self(peer)))
    }
}

/// Every response: `Cache-Control: no-store`, JSON for the body-limit layer's
/// empty 413, and one log line without any user data.
async fn finalize(req: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = req.method().clone();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |m| m.as_str().to_owned());
    let mut response = next.run(req).await;
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE && !is_json(&response) {
        response = ApiError::too_large().into_response();
    }
    if response.status() == StatusCode::REQUEST_TIMEOUT
        && response.headers().get(header::CONTENT_TYPE).is_none()
    {
        response = ApiError::new(StatusCode::REQUEST_TIMEOUT, "timeout").into_response();
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    tracing::info!(
        method = %method,
        route = %route,
        status = response.status().as_u16(),
        ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "request"
    );
    response
}

fn is_json(r: &Response) -> bool {
    r.headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|v| v.as_bytes().starts_with(b"application/json"))
}

async fn fallback() -> ApiError {
    ApiError::not_found()
}

/// The router of the whole API.
pub fn router(state: AppState) -> Router {
    let max_json = usize::try_from(state.limits.max_request_bytes).unwrap_or(usize::MAX);
    let max_attachment = state.max_attachment;

    let json_routes: Router<AppState> = Router::new()
        .route("/v1/info", get(info::get))
        .route("/v1/prelogin", post(prelogin::post))
        .route("/v1/register/start", post(register::start))
        .route("/v1/register/finish", post(register::finish))
        .route("/v1/login/start", post(login::start))
        .route("/v1/login/finish", post(login::finish))
        .route("/v1/login/totp", post(login::totp))
        .route("/v1/logout", post(login::logout))
        .route("/v1/account", get(account::get).delete(account::delete))
        .route("/v1/account/password/start", post(account::password_start))
        .route(
            "/v1/account/password/finish",
            post(account::password_finish),
        )
        .route("/v1/account/totp/setup", post(account::totp_setup))
        .route("/v1/account/totp/confirm", post(account::totp_confirm))
        .route("/v1/account/totp/disable", post(account::totp_disable))
        .route("/v1/devices", get(devices::list))
        .route(
            "/v1/devices/{device_id}",
            axum::routing::patch(devices::rename).delete(devices::remove),
        )
        .route("/v1/vaults", get(vaults::list))
        .route("/v1/vaults/{vault_id}", put(vaults::create))
        .route("/v1/vaults/{vault_id}/changes", get(vaults::changes))
        .route("/v1/vaults/{vault_id}/records", post(records::push))
        .route("/v1/vaults/{vault_id}/meta", put(records::put_meta))
        .route(
            "/v1/vaults/{vault_id}/keys/{generation}",
            put(records::put_key),
        )
        .route("/v1/vaults/{vault_id}/attachments", get(attachments::list))
        .route(
            "/v1/vaults/{vault_id}/attachments/{attachment_id}",
            axum::routing::delete(attachments::delete),
        )
        .route(
            "/v1/vaults/{vault_id}/attachments/{attachment_id}/restore",
            post(attachments::restore),
        )
        .route(
            "/v1/checkpoints/{device_id}",
            put(checkpoints::put).layer(RequestBodyLimitLayer::new(
                json::MAX_CHECKPOINT_UPLOAD_LEN + 1,
            )),
        )
        .route("/v1/checkpoints", get(checkpoints::list))
        .route("/v1/poll", get(poll::get))
        .layer(RequestBodyLimitLayer::new(max_json))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            JSON_TIMEOUT,
        ));

    let attachment_routes: Router<AppState> = Router::new()
        .route(
            "/v1/vaults/{vault_id}/attachments/{attachment_id}",
            put(attachments::put).get(attachments::get),
        )
        .layer(RequestBodyLimitLayer::new(max_attachment))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            ATTACHMENT_TIMEOUT,
        ));

    json_routes
        .merge(attachment_routes)
        .fallback(fallback)
        .layer(middleware::from_fn(finalize))
        .with_state(state)
}
