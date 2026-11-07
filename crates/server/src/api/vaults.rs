//! `GET /v1/vaults`, `PUT /v1/vaults/{id}` and the change feed.

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use mosaic_core::{InstallId, ItemId, KeyGeneration, VaultId, VaultMetaCiphertext, WrappedKey};
use mosaic_sync::wire::json::{
    B64, ChangeKey, ChangeMeta, ChangeRecord, ChangesResponse, CreateVaultRequest, HeadResponse,
    RecordKind, VaultEntry, VaultsResponse,
};
use serde::Deserialize;

use super::{Reply, parse_body, parse_uuid};
use crate::AppState;
use crate::auth::Session;
use crate::db::vaults::{self as dbv, Created, NewVault};
use crate::error::ApiError;

/// Most records in one page of changes.
const MAX_PAGE: usize = 500;

fn generation(g: u32) -> Result<KeyGeneration, ApiError> {
    KeyGeneration::new(g).map_err(|_| ApiError::internal())
}

pub async fn list(State(st): State<AppState>, Session(s): Session) -> Result<Reply, ApiError> {
    let rows = st.db.run(|c| dbv::list(c, &s.account_id))?;
    let vaults = rows
        .into_iter()
        .map(|(id, g, head)| {
            Ok(VaultEntry {
                vault_id: VaultId::from_bytes(id).map_err(|_| ApiError::internal())?,
                generation: generation(g)?,
                head_seq: head,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::ok(&VaultsResponse { vaults })
}

pub async fn create(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let req: CreateVaultRequest = parse_body(
        &body,
        usize::try_from(st.limits.max_request_bytes).unwrap_or(usize::MAX),
    )?;
    let meta = VaultMetaCiphertext::from_bytes(&req.meta.0).map_err(|_| ApiError::bad_request())?;
    if meta.key_generation() != req.generation {
        return Err(ApiError::bad_request());
    }
    // A vault is created by the device that signed in.
    if *req.device_id.as_bytes() != s.device_id {
        return Err(ApiError::forbidden());
    }
    if u64::from(req.generation.get()) > st.limits.max_key_generations {
        return Err(crate::db::devices::quota("keyGenerations"));
    }
    let now = st.clock.now_ms();
    let created = st.db.run_tx(|tx| {
        dbv::create(
            tx,
            &s.account_id,
            &NewVault {
                vault: &vault,
                generation: req.generation.get(),
                wrapped: req.wrapped_key.as_bytes(),
                device: req.device_id.as_bytes(),
                meta: &req.meta.0,
            },
            now,
        )
    })?;
    match created {
        Created::New(head_seq) => {
            st.polls.bump(s.account_id, now);
            Reply::created(&HeadResponse { head_seq })
        }
        Created::Existing(head_seq) => Reply::ok(&HeadResponse { head_seq }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesQuery {
    since: Option<u64>,
    limit: Option<usize>,
}

pub async fn changes(
    State(st): State<AppState>,
    Session(s): Session,
    Path(vault_id): Path<String>,
    query: Result<Query<ChangesQuery>, QueryRejection>,
) -> Result<Reply, ApiError> {
    let vault = parse_uuid(&vault_id)?;
    let Query(q) = query.map_err(|_| ApiError::bad_request())?;
    let limit = q.limit.unwrap_or(MAX_PAGE);
    if limit == 0 || limit > MAX_PAGE {
        return Err(ApiError::bad_request());
    }
    let since = q.since.unwrap_or(0);
    let page = st
        .db
        .run(|c| dbv::changes(c, &s.account_id, &vault, since, limit))?;
    let next_since = if page.more {
        page.records.last().map_or(page.head_seq, |r| r.server_seq)
    } else {
        page.head_seq
    };
    let keys = page
        .keys
        .into_iter()
        .map(|k| {
            Ok(ChangeKey {
                generation: generation(k.generation)?,
                wrapped_key: WrappedKey::from_bytes(&k.wrapped)
                    .map_err(|_| ApiError::internal())?,
                device_id: InstallId::from_bytes(k.device_id).map_err(|_| ApiError::internal())?,
                server_seq: k.server_seq,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let records = page
        .records
        .into_iter()
        .map(|r| {
            Ok(ChangeRecord {
                record_id: ItemId::from_bytes(r.record_id).map_err(|_| ApiError::internal())?,
                kind: if r.kind == 1 {
                    RecordKind::Item
                } else {
                    RecordKind::Tombstone
                },
                ciphertext: B64(r.ciphertext),
                server_seq: r.server_seq,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Reply::json(
        StatusCode::OK,
        &ChangesResponse {
            head_seq: page.head_seq,
            generation: generation(page.generation)?,
            keys,
            meta: page.meta.map(|(ciphertext, server_seq)| ChangeMeta {
                ciphertext: B64(ciphertext),
                server_seq,
            }),
            records,
            more: page.more,
            next_since,
        },
    )
}
