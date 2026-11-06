//! `POST /v1/prelogin`: the salt and parameters a new device needs before it
//! can derive its OPAQUE password. Unknown emails get deterministic fakes
//! (CRYPTO_SPEC §9.2).

use axum::body::Bytes;
use axum::extract::State;
use mosaic_core::{AccountId, Argon2Profile};
use mosaic_sync::wire::email;
use mosaic_sync::wire::json::{B64Fixed, PreloginRequest, PreloginResponse, codes};

use super::{ClientIp, Reply, parse_body};
use crate::AppState;
use crate::auth::prelogin::{self, DIST_TTL_MS, ProfileDist};
use crate::db::accounts;
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

const MAX_BODY: usize = 4096;

/// The real profile distribution, recomputed hourly.
fn distribution(st: &AppState, now: u64) -> Result<ProfileDist, ApiError> {
    {
        let cached = st
            .dist
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, d)) = cached.as_ref()
            && now.saturating_sub(*at) < DIST_TTL_MS
        {
            return Ok(d.clone());
        }
    }
    let counts = st.db.run(accounts::profile_counts)?;
    let d = ProfileDist::from_counts(&counts);
    *st.dist
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((now, d.clone()));
    Ok(d)
}

/// Normalizes the email of a request or answers `invalid_email`.
pub fn normalized_email(raw: &str) -> Result<String, ApiError> {
    email::normalize(raw)
        .map_err(|_| ApiError::new(axum::http::StatusCode::BAD_REQUEST, codes::INVALID_EMAIL))
}

pub async fn post(
    State(st): State<AppState>,
    ip: ClientIp,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: PreloginRequest = parse_body(&body, MAX_BODY)?;
    let now = st.clock.now_ms();
    st.rate
        .check(now, Scope::PreloginIp, &ip.key(), ratelimit::PRELOGIN_IP)
        .map_err(ApiError::rate_limited)?;
    let email = normalized_email(&req.email)?;
    st.rate
        .check(
            now,
            Scope::PreloginEmail,
            email.as_bytes(),
            ratelimit::PRELOGIN_EMAIL,
        )
        .map_err(ApiError::rate_limited)?;

    let account = st.db.run(|c| accounts::by_email(c, &email))?;
    let (id, salt, profile) = if let Some(a) = account {
        let profile = Argon2Profile::from_id(a.profile).map_err(|_| ApiError::internal())?;
        (a.account_id, a.salt, profile)
    } else {
        let secret = &*st.keys.server_secret;
        let dist = distribution(&st, now)?;
        (
            prelogin::fake_account_id(secret, st.keys.init_ms, &email),
            prelogin::fake_salt(secret, &email),
            prelogin::fake_profile(secret, &dist, &email),
        )
    };
    Reply::ok(&PreloginResponse {
        account_id: AccountId::from_bytes(id).map_err(|_| ApiError::internal())?,
        salt: B64Fixed(salt),
        profile,
    })
}
