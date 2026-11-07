//! `GET /v1/poll?since=<u64>`: the long-poll change feed. Returns at once if
//! the account's change counter differs from `since`, else after a change or
//! 25 s.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use mosaic_sync::wire::json::{PollResponse, codes};
use serde::Deserialize;

use super::Reply;
use crate::AppState;
use crate::auth::Session;
use crate::error::ApiError;
use crate::polls::POLL_TIMEOUT;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PollQuery {
    since: Option<u64>,
}

pub async fn get(
    State(st): State<AppState>,
    Session(s): Session,
    query: Result<Query<PollQuery>, QueryRejection>,
) -> Result<Reply, ApiError> {
    let Query(q) = query.map_err(|_| ApiError::bad_request())?;
    let value = st
        .polls
        .wait(
            s.account_id,
            s.token_hash,
            q.since.unwrap_or(0),
            st.clock.now_ms(),
            POLL_TIMEOUT,
        )
        .await
        .map_err(|_| ApiError::new(StatusCode::TOO_MANY_REQUESTS, codes::TOO_MANY_POLLS))?;
    Reply::ok(&PollResponse {
        change_counter: value,
    })
}
