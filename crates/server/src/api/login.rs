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
use super::{ClientIp, Reply, parse_body, valid_device_name};
use crate::AppState;
use crate::auth::opaque::login_params;
use crate::auth::{AnySession, prelogin, token_hash, totp};
use crate::db::{accounts, audit, devices, sessions};
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

const MAX_BODY: usize = 16 * 1024;
/// The OPAQUE login is fresh for five minutes (CRYPTO_SPEC §9.4).
const FRESH_MS: u64 = 300_000;
/// Wrong TOTP codes allowed on one pending session.
const TOTP_ATTEMPTS: u32 = 5;

fn login_failed() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::LOGIN_FAILED)
}

fn two_factor_invalid() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, codes::TWO_FACTOR_INVALID)
}

pub async fn start(
    State(st): State<AppState>,
    ip: ClientIp,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let req: LoginStartRequest = parse_body(&body, MAX_BODY)?;
    let now = st.clock.now_ms();
    st.rate
        .check(now, Scope::LoginIp, &ip.key(), ratelimit::LOGIN_IP)
        .map_err(ApiError::rate_limited)?;
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
            (Some(a.account_id), a.account_id, Some(record))
        }
        None => (
            None,
            prelogin::fake_account_id(&st.keys.server_secret, st.keys.init_ms, &email),
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
        .insert(login_id, started.state, real, credential_id, now);
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
    let finished = entry
        .state
        .finish(finalization, login_params(&entry.credential_id, &st.origin))
        .map_err(|_| login_failed())?;
    if !valid_device_name(&req.device_name) {
        return Err(ApiError::bad_request());
    }
    let token = session_token(&finished.session_key).map_err(|_| ApiError::internal())?;
    let hash = token_hash(token.expose());
    let device = *req.device_id.as_bytes();
    let idle = st.limits.session_idle_days.saturating_mul(86_400_000);
    let expires_at = now.saturating_add(idle);

    let pending = st.db.run_tx(|tx| {
        let account = accounts::by_id(tx, &account_id)?.ok_or_else(login_failed)?;
        devices::upsert_on_login(tx, &account_id, &device, &req.device_name, now)?;
        let pending = account.totp_sealed.is_some();
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
            },
        )?;
        // A pending session is not a login yet; it is audited at the code.
        if !pending {
            audit::record(tx, &account_id, now, audit::Kind::Login, Some(&device))?;
        }
        Ok(pending)
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
        .and_then(|s| totp::verify(s, &req.code, now, account.totp_last_step));
    let Some(step) = step else {
        let attempts = st
            .db
            .run(|c| sessions::bump_attempts(c, &session.token_hash))?;
        if attempts >= TOTP_ATTEMPTS {
            st.db.run(|c| sessions::delete(c, &session.token_hash))?;
        }
        return Err(two_factor_invalid());
    };
    let expires_at = st.db.run_tx(|tx| {
        accounts::set_last_step(tx, &session.account_id, step)?;
        sessions::complete_2fa(tx, &session.token_hash)?;
        audit::record(
            tx,
            &session.account_id,
            now,
            audit::Kind::Login,
            Some(&session.device_id),
        )?;
        Ok(now.saturating_add(st.limits.session_idle_days.saturating_mul(86_400_000)))
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
    Reply::ok(&Empty {})
}
