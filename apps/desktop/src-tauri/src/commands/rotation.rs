//! Account keys commands (M7): thin wrappers over the `Backend` methods in
//! `sync/rotation_flow.rs`. They talk to servers, folders and Argon2id, so
//! they run on the blocking pool; every one first drops what expired (the
//! sync service's pending flows).

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::rotation::{
    DismissFileRequest, ProfileUpgradeRequest, RekeyCommitRequest, RekeyMarkerView,
    RekeyPrepareRequest, RekeyReviewView, RotateCommitRequest, RotatePrepareRequest, RotationDone,
    RotationResumeRequest, RotationStatus,
};
use crate::contract::sync::SyncOverview;
use crate::state::{AppState, Backend};

async fn run<T, F>(state: &State<'_, AppState>, work: F) -> IpcResult<T>
where
    F: FnOnce(&Backend) -> IpcResult<T> + Send + 'static,
    T: Send + 'static,
{
    state
        .run(move |b| {
            b.sync.sweep();
            work(b)
        })
        .await
}

/// `rotation_status`.
#[tauri::command]
pub async fn rotation_status(state: State<'_, AppState>) -> IpcResult<RotationStatus> {
    run(&state, Backend::rotation_status).await
}

/// `secret_key_rotate_prepare`.
#[tauri::command]
pub async fn secret_key_rotate_prepare(
    state: State<'_, AppState>,
    request: RotatePrepareRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.secret_key_rotate_prepare(&request)).await
}

/// `secret_key_rotate_commit`.
#[tauri::command]
pub async fn secret_key_rotate_commit(
    state: State<'_, AppState>,
    request: RotateCommitRequest,
) -> IpcResult<RotationDone> {
    run(&state, move |b| b.secret_key_rotate_commit(&request)).await
}

/// `profile_upgrade`.
#[tauri::command]
pub async fn profile_upgrade(
    state: State<'_, AppState>,
    request: ProfileUpgradeRequest,
) -> IpcResult<RotationDone> {
    run(&state, move |b| b.profile_upgrade(&request)).await
}

/// `rekey_review`.
#[tauri::command]
pub async fn rekey_review(state: State<'_, AppState>) -> IpcResult<RekeyReviewView> {
    run(&state, Backend::rekey_review_view).await
}

/// `rekey_prepare`.
#[tauri::command]
pub async fn rekey_prepare(
    state: State<'_, AppState>,
    request: RekeyPrepareRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.rekey_prepare(&request)).await
}

/// `rekey_commit`.
#[tauri::command]
pub async fn rekey_commit(
    state: State<'_, AppState>,
    request: RekeyCommitRequest,
) -> IpcResult<RotationDone> {
    run(&state, move |b| b.rekey_commit(&request)).await
}

/// `rotation_resume`.
#[tauri::command]
pub async fn rotation_resume(
    state: State<'_, AppState>,
    request: RotationResumeRequest,
) -> IpcResult<RotationDone> {
    run(&state, move |b| b.rotation_resume(&request)).await
}

/// `rotation_abandon`.
#[tauri::command]
pub async fn rotation_abandon(state: State<'_, AppState>) -> IpcResult<()> {
    run(&state, Backend::rotation_abandon).await
}

/// `sync_dismiss_account_file`.
#[tauri::command]
pub async fn sync_dismiss_account_file(
    state: State<'_, AppState>,
    request: DismissFileRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_dismiss_account_file(&request)).await
}

/// `rotation_abandon_committed`.
#[tauri::command]
pub async fn rotation_abandon_committed(state: State<'_, AppState>) -> IpcResult<()> {
    run(&state, Backend::rotation_abandon_committed).await
}

/// `sync_rekey_markers`.
#[tauri::command]
pub async fn sync_rekey_markers(state: State<'_, AppState>) -> IpcResult<Vec<RekeyMarkerView>> {
    run(&state, Backend::sync_rekey_markers).await
}

/// `sync_dismiss_rekey_marker`.
#[tauri::command]
pub async fn sync_dismiss_rekey_marker(
    state: State<'_, AppState>,
    request: DismissFileRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_dismiss_rekey_marker(&request)).await
}
