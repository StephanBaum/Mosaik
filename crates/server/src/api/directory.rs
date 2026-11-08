//! The key directory (M6b spec §5.4). `POST /v1/directory/lookup` answers
//! every valid email with `200 {accountId, ipk}`: unknown emails and accounts
//! without a published key get values derived from the server secret, so the
//! answer does not reveal who is registered (the account ID is the same fake
//! the pre-login route gives). `GET /v1/directory/{id}` serves only accounts
//! the caller already shares something with.

use axum::body::Bytes;
use axum::extract::{Path, State};
use mosaic_core::encoding::label;
use mosaic_sync::wire::json::B64Fixed;

use super::wire::{DirectoryEntry, Id, LookupRequest};
use super::{Reply, parse_body, parse_uuid, prelogin};
use crate::AppState;
use crate::auth::{self, Session, prelogin as fakes};
use crate::db::directory as dbd;
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

pub async fn lookup(
    State(st): State<AppState>,
    Session(s): Session,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let now = st.clock.now_ms();
    st.rate
        .check(
            now,
            Scope::DirectorySession,
            &s.token_hash,
            ratelimit::DIRECTORY_SESSION,
        )
        .map_err(ApiError::rate_limited)?;
    st.rate
        .check(
            now,
            Scope::DirectoryAccount,
            &s.account_id,
            ratelimit::DIRECTORY_ACCOUNT,
        )
        .map_err(ApiError::rate_limited)?;
    let req: LookupRequest = parse_body(&body, 4096)?;
    let email = prelogin::normalized_email(&req.email)?;
    // The fakes are always computed, so a real and an unknown address cost
    // the same.
    let prelogin_key = auth::prelogin_key(&st.keys.server_secret).ok_or_else(ApiError::internal)?;
    let dir_key = auth::subkey(&st.keys.server_secret, label::SERVER_DIRECTORY_KEY)
        .ok_or_else(ApiError::internal)?;
    let fake_id = fakes::fake_account_id(&prelogin_key, &email);
    let fake_ipk = fakes::fake_ipk(&dir_key, &email);
    let found = st.db.run(|c| dbd::by_email(c, &email))?;
    let (id, ipk) = match found {
        Some((id, Some(ipk))) => (id, ipk),
        Some((id, None)) => (id, fake_ipk),
        None => (fake_id, fake_ipk),
    };
    Reply::ok(&DirectoryEntry {
        account_id: Id(id),
        ipk: B64Fixed(ipk),
    })
}

pub async fn by_id(
    State(st): State<AppState>,
    Session(s): Session,
    Path(account_id): Path<String>,
) -> Result<Reply, ApiError> {
    let target = parse_uuid(&account_id)?;
    let ipk = st.db.run(|c| {
        if dbd::may_look_up(c, &s.account_id, &target)? {
            dbd::ipk_of(c, &target)
        } else {
            Ok(None)
        }
    })?;
    let ipk = ipk.ok_or_else(ApiError::not_found)?;
    Reply::ok(&DirectoryEntry {
        account_id: Id(target),
        ipk: B64Fixed(ipk),
    })
}
