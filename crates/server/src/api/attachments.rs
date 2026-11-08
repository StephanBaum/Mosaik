//! Attachments: `/v1/vaults/{v}/attachments[/{a}]` (CRYPTO_SPEC §7.3).
//! The server stores the STREAM ciphertext as is after the same structural
//! check the clients make; soft deletes are purged after the retention.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use mosaic_core::AttachmentId;
use mosaic_sync::wire::json::{AttachmentEntry, AttachmentsResponse, Empty, RestoreResponse};
use mosaic_vault::storage::{self, validate_attachment_ciphertext};

use super::sharing::notify_vault;
use super::{Reply, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::attachments::{self, Put};
use crate::db::vaults as dbv;
use crate::error::ApiError;
use crate::ratelimit::{self, Scope};

pub async fn list(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let rows = st.db.run(|c| attachments::list(c, &s.account_id, &vault))?;
    let attachments = rows
        .into_iter()
        .map(|(id, size, deleted_at)| {
            Ok(AttachmentEntry {
                attachment_id: AttachmentId::from_bytes(id).map_err(|_| ApiError::internal())?,
                size,
                deleted_at,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&AttachmentsResponse { attachments })
}

pub async fn put(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, attachment_id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let id = parse_uuid(&attachment_id)?;
    let now = st.clock.now_ms();
    st.rate
        .check(
            now,
            Scope::AttachmentPut,
            &s.token_hash,
            ratelimit::ATTACHMENT_PUT,
        )
        .map_err(ApiError::rate_limited)?;
    validate_attachment_ciphertext(&body).map_err(|e| match e {
        storage::Error::AttachmentTooLarge => ApiError::too_large(),
        _ => ApiError::bad_request(),
    })?;
    let size = u64::try_from(body.len()).unwrap_or(u64::MAX);
    let put = st.db.run_tx(|tx| {
        attachments::put(
            tx,
            &s.account_id,
            &vault,
            &id,
            size,
            st.limits.account_attachment_bytes,
            now,
        )
    })?;
    match put {
        Put::Created => {
            if let Err(e) = st.blobs.write(&id, &body) {
                // Do not leave a row without a file.
                st.db.run(|c| attachments::remove_row(c, &id))?;
                return Err(e.into());
            }
            notify_vault(&st, &vault, now);
            Reply::created(&Empty {})
        }
        Put::Existing => {
            if !st.blobs.exists(&id) {
                st.blobs.write(&id, &body)?;
            }
            notify_vault(&st, &vault, now);
            Reply::empty()
        }
    }
}

pub async fn get(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, attachment_id)): Path<(String, String)>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let id = parse_uuid(&attachment_id)?;
    let row = st.db.run(|c| {
        dbv::access(c, &s.account_id, &vault)?;
        attachments::get(c, &id)
    })?;
    // Served also while soft-deleted, until the purge.
    let row = row
        .filter(|r| r.vault_id == vault)
        .ok_or_else(ApiError::not_found)?;
    let bytes = st.blobs.read(&id)?.ok_or_else(ApiError::not_found)?;
    if u64::try_from(bytes.len()).ok() != Some(row.size) {
        return Err(ApiError::internal());
    }
    Ok(Reply::bytes(bytes))
}

pub async fn delete(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, attachment_id)): Path<(String, String)>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let id = parse_uuid(&attachment_id)?;
    let now = st.clock.now_ms();
    st.db
        .run(|c| attachments::soft_delete(c, &s.account_id, &vault, &id, now))?;
    notify_vault(&st, &vault, now);
    Reply::empty()
}

pub async fn restore(
    State(st): State<AppState>,
    Session(s): Session,
    Path((vault_id, attachment_id)): Path<(String, String)>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let id = parse_uuid(&attachment_id)?;
    let restored = st
        .db
        .run(|c| attachments::restore(c, &s.account_id, &vault, &id))?;
    if restored {
        notify_vault(&st, &vault, st.clock.now_ms());
    }
    Reply::json(StatusCode::OK, &RestoreResponse { restored })
}
