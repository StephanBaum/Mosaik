//! Authentication: session tokens, the session extractors and the second
//! factor (CRYPTO_SPEC §9).
//!
//! A bearer token is `base64url(session_token)`; the server stores only
//! `SHA-256(session_token)`. A request needs one of
//!
//! - [`Session`]: a session that completed login (and 2FA when enabled),
//! - [`AnySession`]: that, or a pending session waiting for its TOTP code,
//!
//! and routes that change credentials call [`Authed::require_fresh`]: the
//! session's OPAQUE login must be at most five minutes old (§9.4).

pub mod opaque;
pub mod prelogin;
pub mod secret_file;
pub mod totp;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64ct::{Base64UrlUnpadded, Encoding};
use hkdf::Hkdf;
use mosaic_core::encoding::ctx;
use mosaic_sync::wire::json::codes;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::AppState;
use crate::db::sessions;
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

/// A subkey of `server_secret`, one per purpose (CRYPTO_SPEC §9.2, §9.3):
/// `HKDF(server_secret, "", ctx(label), 32)`.
#[must_use]
pub fn subkey(server_secret: &[u8; 32], label: &str) -> Option<Zeroizing<[u8; 32]>> {
    let (mut prk, hkdf) = Hkdf::<Sha256>::extract(Some(&[]), server_secret);
    zeroize::Zeroize::zeroize(prk.as_mut_slice());
    let mut out = Zeroizing::new([0u8; 32]);
    hkdf.expand(&ctx(label), out.as_mut_slice()).ok()?;
    Some(out)
}

/// The key of the pre-login fakes.
#[must_use]
pub fn prelogin_key(server_secret: &[u8; 32]) -> Option<Zeroizing<[u8; 32]>> {
    subkey(server_secret, "server/prelogin-key")
}

/// SHA-256 of a session token: what the database stores.
#[must_use]
pub fn token_hash(token: &[u8; 32]) -> [u8; 32] {
    Sha256::digest(token).into()
}

/// The 32-byte token of an `Authorization: Bearer <base64url>` header.
fn bearer(parts: &Parts) -> Option<[u8; 32]> {
    let value = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let text = value.strip_prefix("Bearer ")?;
    let mut token = [0u8; 32];
    let n = Base64UrlUnpadded::decode(text, &mut token).ok()?.len();
    (n == 32).then_some(token)
}

/// A session found for a request.
pub struct Authed {
    /// The account.
    pub account_id: [u8; 16],
    /// The device the session belongs to.
    pub device_id: [u8; 16],
    /// SHA-256 of the token (also the session's key in the rate limiter).
    pub token_hash: [u8; 32],
    /// Waiting for its TOTP code.
    pub pending: bool,
    fresh_until: u64,
    now: u64,
}

impl Authed {
    /// Requires a login within the last five minutes (CRYPTO_SPEC §9.4).
    ///
    /// # Errors
    /// 403 `fresh_login_required`.
    pub fn require_fresh(&self) -> Result<(), ApiError> {
        if self.fresh_until > self.now {
            Ok(())
        } else {
            Err(ApiError::new(
                StatusCode::FORBIDDEN,
                codes::FRESH_LOGIN_REQUIRED,
            ))
        }
    }
}

fn authenticate(parts: &Parts, state: &AppState) -> Result<Authed, ApiError> {
    let token = bearer(parts).ok_or_else(ApiError::unauthorized)?;
    let hash = token_hash(&token);
    let now = state.clock.now_ms();
    let idle = state.limits.session_idle_days.saturating_mul(86_400_000);
    let max = state.limits.session_max_days.saturating_mul(86_400_000);
    let row = state
        .db
        .run(|c| sessions::lookup_and_touch(c, &hash, now, idle, max))?
        .ok_or_else(ApiError::unauthorized)?;
    state
        .rate
        .check(now, Scope::Session, &hash, ratelimit::SESSION)
        .map_err(ApiError::rate_limited)?;
    Ok(Authed {
        account_id: row.account_id,
        device_id: row.device_id,
        token_hash: hash,
        pending: row.pending_2fa,
        fresh_until: row.fresh_until,
        now,
    })
}

/// A completed session (login and, when enabled, 2FA).
pub struct Session(pub Authed);

/// A session that may still be waiting for its TOTP code.
pub struct AnySession(pub Authed);

impl FromRequestParts<AppState> for Session {
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(authenticate(parts, state).and_then(|a| {
            if a.pending {
                Err(ApiError::unauthorized())
            } else {
                Ok(Self(a))
            }
        }))
    }
}

impl FromRequestParts<AppState> for AnySession {
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(authenticate(parts, state).map(Self))
    }
}
