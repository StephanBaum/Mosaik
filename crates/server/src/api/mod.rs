//! The HTTP API (M6 spec §6.2): routes, request parsing and the middleware
//! shared by every route.

mod account;
mod attachments;
mod checkpoints;
mod devices;
mod directory;
mod emergency;
mod info;
mod login;
mod page;
mod poll;
mod prelogin;
mod records;
mod register;
mod rekey;
mod sharing;
mod vaults;
mod webauthn;
mod wire;

use std::marker::PhantomData;
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
use tower::limit::ConcurrencyLimitLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::AppState;
use crate::error::ApiError;
use crate::ratelimit::{self, Rule, Scope};

/// Every error code: the M6 set plus the sharing and emergency-access codes
/// of M6b (`docs/SERVER.md`).
pub mod codes {
    pub use mosaic_sync::wire::json::codes::*;

    /// The caller's role does not allow the request (403).
    pub const ROLE_FORBIDDEN: &str = "role_forbidden";
    /// `PUT keys/{g}` on a shared vault, or a rotation that does not cover
    /// exactly the members and open invites (409).
    pub const MEMBERS_MISMATCH: &str = "members_mismatch";
    /// The identity key is already set to another pair (409).
    pub const IDENTITY_EXISTS: &str = "identity_exists";
    /// A compare-and-swap failed (409, detail `currentSeq`).
    pub const CONFLICT: &str = "conflict";
    /// The recipient already has an open invite to the vault (409).
    pub const INVITE_EXISTS: &str = "invite_exists";
    /// The invite was sealed for an older key generation (409).
    pub const INVITE_STALE: &str = "invite_stale";
    /// The invite ran out (410).
    pub const INVITE_EXPIRED: &str = "invite_expired";
    /// The vault is at its member limit (409).
    pub const MEMBERS_LIMIT: &str = "members_limit";
    /// The account has too many open invites (409).
    pub const INVITE_LIMIT: &str = "invite_limit";
    /// The recipient is a member already (409).
    pub const ALREADY_MEMBER: &str = "already_member";
    /// A removed account can't be invited again before the next rotation (409).
    pub const REINVITE_BLOCKED: &str = "reinvite_blocked";
    /// The owner can't leave (409).
    pub const OWNER_CANNOT_LEAVE: &str = "owner_cannot_leave";
    /// No share or emergency seal backs the claim (403).
    pub const NO_SHARE: &str = "no_share";
    /// A request is open; the wait can't change, or the wait is not over (409).
    pub const EMERGENCY_WAITING: &str = "emergency_waiting";
    /// The grantee asked less than 24 h after a refusal (409, detail `until`).
    pub const EMERGENCY_COOLDOWN: &str = "emergency_cooldown";
    /// The grant was approved or released (409).
    pub const EMERGENCY_RELEASED: &str = "emergency_released";
    /// There is no open request (409).
    pub const EMERGENCY_NOT_REQUESTED: &str = "emergency_not_requested";
    /// Emergency seals cover only vaults the grantor owns (403).
    pub const EMERGENCY_OWNER_ONLY: &str = "emergency_owner_only";
    /// The re-key target epoch is not the stored epoch plus one (409, detail
    /// `currentEpoch`).
    pub const EPOCH_MISMATCH: &str = "epoch_mismatch";
    /// The re-key would lower the Argon2 profile (409).
    pub const PROFILE_DOWNGRADE: &str = "profile_downgrade";
    /// A re-key list is not exactly the set the server holds (409, detail
    /// `set`: `vaults`, `memberRewraps` or `emergencySeals`).
    pub const REKEY_MISMATCH: &str = "rekey_mismatch";
}

/// Body limit of `prelogin`, before any session exists.
const PRELOGIN_BODY: usize = 4 * 1024;
/// Body limit of the register and login routes.
const AUTH_BODY: usize = 16 * 1024;
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

    /// 204 with no body.
    pub const fn no_content() -> Self {
        Self {
            status: StatusCode::NO_CONTENT,
            content_type: "application/json",
            body: Vec::new(),
        }
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

/// A rate limit that is checked before the request body is read.
pub trait Gate {
    /// The bucket scope.
    const SCOPE: Scope;
    /// The bucket's size and rate.
    const RULE: Rule;
}

macro_rules! gate {
    ($name:ident, $scope:expr, $rule:expr) => {
        /// A per-IP gate.
        pub struct $name;
        impl Gate for $name {
            const SCOPE: Scope = $scope;
            const RULE: Rule = $rule;
        }
    };
}

gate!(PreloginGate, Scope::PreloginIp, ratelimit::PRELOGIN_IP);
gate!(LoginGate, Scope::LoginIp, ratelimit::LOGIN_IP);
gate!(HandoffGate, Scope::Handoff, ratelimit::HANDOFF_IP);
gate!(
    RegisterStartGate,
    Scope::RegisterStartIp,
    ratelimit::REGISTER_IP
);
gate!(
    RegisterFinishGate,
    Scope::RegisterFinishIp,
    ratelimit::REGISTER_IP
);

/// The client address after the per-IP rate limit `G` passed. As an
/// extractor placed before the body, it makes a limited client cost no body
/// read and no parsing.
pub struct Gated<G>(pub ClientIp, PhantomData<G>);

impl<G: Gate + Send + Sync> FromRequestParts<AppState> for Gated<G> {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let ip = ClientIp::from_request_parts(parts, state).await?;
        state
            .rate
            .check(state.clock.now_ms(), G::SCOPE, &ip.key(), G::RULE)
            .map_err(ApiError::rate_limited)?;
        Ok(Self(ip, PhantomData))
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

/// A panic in a request becomes a bare `500 internal`; the payload is dropped
/// without being read.
fn panic_response(payload: Box<dyn std::any::Any + Send + 'static>) -> Response {
    drop(payload);
    ApiError::internal().into_response()
}

async fn fallback() -> ApiError {
    ApiError::not_found()
}

/// The router of the whole API.
#[expect(clippy::too_many_lines, reason = "the route table")]
pub fn router(state: AppState) -> Router {
    let max_json = usize::try_from(state.limits.max_request_bytes).unwrap_or(usize::MAX);
    let max_attachment = state.max_attachment;

    // The routes that need no session (or only a pending one) have their own
    // small body limits instead of the global one: limits are not nested
    // (a nested limit hides the length error from the body reader).
    let public_routes: Router<AppState> = Router::new()
        .route("/v1/info", get(info::get))
        .route(
            "/v1/prelogin",
            post(prelogin::post).layer(RequestBodyLimitLayer::new(PRELOGIN_BODY)),
        )
        .route(
            "/v1/register/start",
            post(register::start).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/register/finish",
            post(register::finish).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/login/start",
            post(login::start).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/login/finish",
            post(login::finish).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/login/totp",
            post(login::totp).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route("/v1/logout", post(login::logout))
        .route("/v1/login/methods", get(webauthn::login_methods))
        .route(
            "/v1/login/webauthn/handoff",
            post(webauthn::login_handoff).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/login/webauthn/handoff/state",
            post(webauthn::login_state).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/webauthn/handoff/options",
            post(webauthn::options).layer(RequestBodyLimitLayer::new(AUTH_BODY)),
        )
        .route(
            "/v1/webauthn/handoff/finish",
            post(webauthn::finish).layer(RequestBodyLimitLayer::new(webauthn::FINISH_BODY)),
        )
        .route("/webauthn/", get(page::index))
        .route("/webauthn/page.js", get(page::script))
        .route("/webauthn/page.css", get(page::style))
        .route("/webauthn/strings.json", get(page::strings))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            JSON_TIMEOUT,
        ));

    let json_routes: Router<AppState> = Router::new()
        .route("/v1/account", get(account::get).delete(account::delete))
        .route("/v1/account/identity", put(sharing::put_identity))
        .route(
            "/v1/account/contacts",
            get(sharing::get_contacts).put(sharing::put_contacts),
        )
        .route("/v1/directory/lookup", post(directory::lookup))
        .route("/v1/directory/{account_id}", get(directory::by_id))
        .route("/v1/invites", get(sharing::list_invites))
        .route(
            "/v1/invites/{invite_id}",
            axum::routing::delete(sharing::cancel_invite),
        )
        .route("/v1/invites/{invite_id}/vault", get(sharing::invite_vault))
        .route(
            "/v1/invites/{invite_id}/accept",
            post(sharing::accept_invite),
        )
        .route(
            "/v1/invites/{invite_id}/decline",
            post(sharing::decline_invite),
        )
        .route(
            "/v1/blocks/{account_id}",
            post(sharing::block).delete(sharing::unblock),
        )
        .route("/v1/emergency", get(emergency::overview))
        .route(
            "/v1/emergency/grants/{account_id}",
            put(emergency::put_grant).delete(emergency::delete_grant),
        )
        .route(
            "/v1/emergency/grants/{account_id}/seals",
            put(emergency::put_seals),
        )
        .route(
            "/v1/emergency/grants/{account_id}/request",
            post(emergency::request),
        )
        .route(
            "/v1/emergency/grants/{account_id}/decline",
            post(emergency::decline),
        )
        .route(
            "/v1/emergency/grants/{account_id}/approve",
            post(emergency::approve),
        )
        .route(
            "/v1/emergency/grants/{account_id}/release",
            post(emergency::release),
        )
        .route("/v1/account/rekey/start", post(rekey::start))
        .route("/v1/account/rekey/finish", post(rekey::finish))
        .route("/v1/account/password/start", post(account::password_start))
        .route(
            "/v1/account/password/finish",
            post(account::password_finish),
        )
        .route("/v1/account/totp/setup", post(account::totp_setup))
        .route("/v1/account/totp/confirm", post(account::totp_confirm))
        .route("/v1/account/totp/disable", post(account::totp_disable))
        .route(
            "/v1/account/webauthn/handoff",
            post(webauthn::account_handoff),
        )
        .route(
            "/v1/account/webauthn/handoff/state",
            post(webauthn::account_state),
        )
        .route("/v1/account/webauthn", get(webauthn::list))
        .route("/v1/account/webauthn/remove", post(webauthn::remove))
        .route("/v1/devices", get(devices::list))
        .route(
            "/v1/devices/{device_id}",
            axum::routing::patch(devices::rename).delete(devices::remove),
        )
        .route("/v1/vaults", get(vaults::list))
        .route("/v1/vaults/{vault_id}", put(vaults::create))
        .route("/v1/vaults/{vault_id}/changes", get(vaults::changes))
        .route(
            "/v1/vaults/{vault_id}/invites",
            post(sharing::create_invite),
        )
        .route("/v1/vaults/{vault_id}/members", get(sharing::members))
        .route(
            "/v1/vaults/{vault_id}/members/{account_id}",
            put(sharing::set_role).delete(sharing::remove_member),
        )
        .route("/v1/vaults/{vault_id}/roster", put(sharing::put_roster))
        .route("/v1/vaults/{vault_id}/rotate", post(sharing::rotate))
        .route("/v1/vaults/{vault_id}/leave", post(sharing::leave))
        .route(
            "/v1/vaults/{vault_id}/keys/{generation}/claim",
            put(sharing::claim),
        )
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

    // Long polls wait up to 25 s: they are not counted by the concurrency
    // limit (they have their own caps per session, account and server).
    let limited = json_routes
        .merge(public_routes)
        .merge(attachment_routes)
        .layer(ConcurrencyLimitLayer::new(
            usize::try_from(state.limits.max_concurrent_requests).unwrap_or(usize::MAX),
        ));
    let poll_routes: Router<AppState> =
        Router::new()
            .route("/v1/poll", get(poll::get))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                JSON_TIMEOUT,
            ));

    limited
        .merge(poll_routes)
        .fallback(fallback)
        // Inside `finalize`, so the bare 500 gets the usual headers and log
        // line. The custom handler drops the panic payload unread: the
        // default one would log the message (CRYPTO_SPEC §9.3 "Panics").
        .layer(CatchPanicLayer::custom(panic_response))
        .layer(middleware::from_fn(finalize))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt as _;

    /// A panic in a handler becomes a bare `500 internal`: the body names no
    /// detail and the panic message never reaches the response.
    #[tokio::test]
    async fn a_panic_is_a_bare_500() {
        async fn boom() -> &'static str {
            std::panic::panic_any("secret panic text")
        }
        let router: Router = Router::new()
            .route("/boom", get(boom))
            .layer(CatchPanicLayer::custom(panic_response));
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/boom")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("\"internal\""), "{text}");
        assert!(!text.contains("secret"), "{text}");
    }
}
