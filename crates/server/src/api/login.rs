//! `POST /v1/login/start`, `/login/finish`, `/login/totp` and `/logout`
//! (CRYPTO_SPEC §9.2, §9.3).
//!
//! The server derives the session token from the OPAQUE session key exactly
//! as the client does and stores only its SHA-256; it never sends the token.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use mosaic_core::sync_auth::session_token;
use mosaic_sync::wire::json::{
    B64, B64Fixed, CodeRequest, Empty, LoginFinishRequest, LoginResponse, LoginStartRequest,
    LoginStartResponse, LoginStatus, codes,
};
use mosaic_sync::wire::opaque::{
    OsRng, Suite,
    ke::{CredentialFinalization, CredentialRequest, ServerLogin, ServerRegistration},
};

use super::prelogin::normalized_email;
use super::{Gated, LoginGate, Reply, parse_body, valid_device_name};
use crate::AppState;
use crate::auth::opaque::login_params;
use crate::auth::{self, AnySession, prelogin, token_hash, totp};
use crate::db::{accounts, audit, devices, sessions};
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};
use zeroize::Zeroize as _;

const MAX_BODY: usize = 16 * 1024;
/// The OPAQUE login is fresh for five minutes (CRYPTO_SPEC §9.4).
const FRESH_MS: u64 = 300_000;
/// Wrong TOTP codes allowed on one pending session.
const TOTP_ATTEMPTS: u32 = 5;
/// A pending session (login done, TOTP code missing) lives this long.
const PENDING_MS: u64 = 300_000;
/// Sessions of one account; the ones idle longest are evicted beyond it.
const MAX_SESSIONS_PER_ACCOUNT: usize = 64;

fn login_failed() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::LOGIN_FAILED)
}

fn two_factor_invalid() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::TWO_FACTOR_INVALID)
}

pub async fn start(
    State(st): State<AppState>,
    Gated(ip, _): Gated<LoginGate>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: LoginStartRequest = parse_body(&body, MAX_BODY)?;
    let now = st.clock.now_ms();
    let email = normalized_email(&req.email)?;
    st.rate
        .check(
            now,
            Scope::LoginEmail,
            email.as_bytes(),
            ratelimit::LOGIN_EMAIL,
        )
        .map_err(ApiError::rate_limited)?;
    let request = CredentialRequest::<Suite>::deserialize(&req.credential_request.0)
        .map_err(|_| ApiError::bad_request())?;

    let account = st.db.run(|c| accounts::by_email(c, &email))?;
    // An unknown email gets a fake credential response bound to the fake
    // account ID of the pre-login (CRYPTO_SPEC §9.2).
    let (real, credential_id, record) = match account {
        Some(a) => {
            let record = ServerRegistration::<Suite>::deserialize(&a.opaque_record)
                .map_err(|_| ApiError::internal())?;
            (
                Some((a.account_id, a.account_version)),
                a.account_id,
                Some(record),
            )
        }
        None => (
            None,
            prelogin::fake_account_id(
                &*auth::prelogin_key(&st.keys.server_secret).ok_or_else(ApiError::internal)?,
                &email,
            ),
            None,
        ),
    };
    let started = ServerLogin::<Suite>::start(
        &mut OsRng,
        st.setup(),
        record,
        request,
        &credential_id,
        login_params(&credential_id, &st.origin),
    )
    .map_err(|_| ApiError::bad_request())?;
    let mut login_id = [0u8; 16];
    getrandom::fill(&mut login_id).map_err(|_| ApiError::internal())?;
    st.logins
        .insert(ip.key(), login_id, started.state, real, credential_id, now);
    Reply::ok(&LoginStartResponse {
        login_id: B64Fixed(login_id),
        credential_response: B64(started.message.serialize().to_vec()),
    })
}

pub async fn finish(State(st): State<AppState>, body: Bytes) -> Result<Reply, ApiError> {
    let req: LoginFinishRequest = parse_body(&body, MAX_BODY)?;
    let now = st.clock.now_ms();
    // Every failure before the key exchange is verified is the same answer.
    let entry = st
        .logins
        .take(&req.login_id.0, now)
        .ok_or_else(login_failed)?;
    let account_id = entry.account_id.ok_or_else(login_failed)?;
    let finalization = CredentialFinalization::<Suite>::deserialize(&req.credential_finalization.0)
        .map_err(|_| login_failed())?;
    let mut finished = entry
        .state
        .finish(finalization, login_params(&entry.credential_id, &st.origin))
        .map_err(|_| login_failed())?;
    // The OPAQUE session key is wiped as soon as the token is derived.
    let token = session_token(&finished.session_key);
    finished.session_key.as_mut_slice().zeroize();
    let token = token.map_err(|_| ApiError::internal())?;
    if !valid_device_name(&req.device_name) {
        return Err(ApiError::bad_request());
    }
    let hash = token_hash(token.expose());
    let device = *req.device_id.as_bytes();
    let idle = st.limits.session_idle_days.saturating_mul(86_400_000);
    let idle_expiry = now.saturating_add(idle);

    let (pending, expires_at) = st.db.run_tx(|tx| {
        let account = accounts::by_id(tx, &account_id)?.ok_or_else(login_failed)?;
        // A password change since `login/start` ends this login.
        if account.account_version != entry.account_version {
            return Err(login_failed());
        }
        devices::upsert_on_login(tx, &account_id, &device, &req.device_name, now)?;
        let pending =
            account.totp_sealed.is_some() || crate::db::webauthn::count(tx, &account_id)? > 0;
        // A pending session ends after five minutes.
        let expires_at = if pending {
            now.saturating_add(PENDING_MS)
        } else {
            idle_expiry
        };
        // One session per device: a new login ends the device's older ones.
        sessions::delete_for_device(tx, &account_id, &device)?;
        sessions::insert(
            tx,
            &sessions::NewSession {
                token_hash: &hash,
                account: &account_id,
                device: &device,
                now,
                expires_at,
                fresh_until: now.saturating_add(FRESH_MS),
                pending,
                account_version: account.account_version,
            },
        )?;
        // A pending session is not a login yet; it is audited at the code.
        if !pending {
            audit::record(tx, &account_id, now, audit::Kind::Login, Some(&device))?;
        }
        sessions::evict_over_cap(tx, &account_id, &hash, MAX_SESSIONS_PER_ACCOUNT)?;
        Ok((pending, expires_at))
    })?;
    Reply::ok(&LoginResponse {
        status: if pending {
            LoginStatus::TwoFactorRequired
        } else {
            LoginStatus::Ok
        },
        expires_at,
    })
}

pub async fn totp(
    State(st): State<AppState>,
    AnySession(session): AnySession,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: CodeRequest = parse_body(&body, 1024)?;
    if !session.pending {
        return Err(ApiError::bad_request());
    }
    let now = st.clock.now_ms();
    // This attempt is counted before the code is looked at (atomically, so
    // parallel guesses cannot exceed the cap), on the session and on the
    // account.
    let Some(attempt) = st
        .db
        .run(|c| sessions::claim_attempt(c, &session.token_hash, TOTP_ATTEMPTS))?
    else {
        st.db.run(|c| sessions::delete(c, &session.token_hash))?;
        return Err(ApiError::unauthorized());
    };
    if let Err(retry) = st
        .db
        .run(|c| accounts::claim_second_factor_slot(c, &session.account_id, now))?
    {
        return Err(ApiError::rate_limited(retry));
    }
    let account = st
        .db
        .run(|c| accounts::by_id(c, &session.account_id))?
        .ok_or_else(ApiError::unauthorized)?;
    let secret = account
        .totp_sealed
        .as_deref()
        .and_then(|s| totp::open(&st.keys.server_secret, &session.account_id, s));
    let step = secret
        .as_ref()
        .and_then(|s| totp::verify(s, req.code.expose_secret(), now, account.totp_last_step));
    let Some(step) = step else {
        if attempt >= TOTP_ATTEMPTS {
            st.db.run(|c| sessions::delete(c, &session.token_hash))?;
        }
        return Err(two_factor_invalid());
    };
    let idle = st.limits.session_idle_days.saturating_mul(86_400_000);
    let expires_at = st.db.run_tx(|tx| {
        // The step is stored conditionally: of two requests with one code,
        // only one wins.
        if !accounts::set_last_step(tx, &session.account_id, step)? {
            return Err(two_factor_invalid());
        }
        let expires_at = now.saturating_add(idle);
        sessions::complete_2fa(tx, &session.token_hash, expires_at)?;
        accounts::refund_totp_slot(tx, &session.account_id)?;
        audit::record(
            tx,
            &session.account_id,
            now,
            audit::Kind::Login,
            Some(&session.device_id),
        )?;
        Ok(expires_at)
    })?;
    Reply::ok(&LoginResponse {
        status: LoginStatus::Ok,
        expires_at,
    })
}

pub async fn logout(
    State(st): State<AppState>,
    AnySession(session): AnySession,
) -> Result<Reply, ApiError> {
    st.db.run(|c| sessions::delete(c, &session.token_hash))?;
    st.handoffs.drop_session(&session.token_hash);
    Reply::ok(&Empty {})
}
