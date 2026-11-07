//! Sync commands (M6): thin wrappers over the `Backend` methods in
//! `sync/`. The work runs on the blocking pool: it talks to servers, folders
//! and native dialogs.

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::sync::{
    ChangePasswordRequest, CheckServerRequest, CodeRequest, DeviceRemoveRequest,
    DeviceRenameRequest, FolderChoice, FolderConnectRequest, HostedConnectRequest,
    JoinFolderRequest, JoinHostedRequest, JoinOutcome, ServerCheck, SyncDevices, SyncOverview,
    TotpBeginRequest, TotpDisableRequest, TotpSetupInfo,
};
use crate::state::{AppState, Backend};

/// `sync_status`.
#[tauri::command]
pub async fn sync_status(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    state.run(Backend::sync_status).await
}

/// `sync_check_server`.
#[tauri::command]
pub async fn sync_check_server(
    state: State<'_, AppState>,
    request: CheckServerRequest,
) -> IpcResult<ServerCheck> {
    state.run(move |b| b.sync_check_server(&request)).await
}

/// `sync_hosted_connect`.
#[tauri::command]
pub async fn sync_hosted_connect(
    state: State<'_, AppState>,
    request: HostedConnectRequest,
) -> IpcResult<SyncOverview> {
    state.run(move |b| b.sync_hosted_connect(request)).await
}

/// `sync_hosted_two_factor`.
#[tauri::command]
pub async fn sync_hosted_two_factor(
    state: State<'_, AppState>,
    request: CodeRequest,
) -> IpcResult<SyncOverview> {
    state.run(move |b| b.sync_hosted_two_factor(&request)).await
}

/// `sync_pick_folder`.
#[tauri::command]
pub async fn sync_pick_folder(state: State<'_, AppState>) -> IpcResult<Option<FolderChoice>> {
    state.run(Backend::sync_pick_folder).await
}

/// `sync_folder_connect`.
#[tauri::command]
pub async fn sync_folder_connect(
    state: State<'_, AppState>,
    request: FolderConnectRequest,
) -> IpcResult<SyncOverview> {
    state.run(move |b| b.sync_folder_connect(&request)).await
}

/// `sync_now`.
#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    state.run(Backend::sync_now).await
}

/// `sync_disconnect`.
#[tauri::command]
pub async fn sync_disconnect(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    state.run(Backend::sync_disconnect).await
}

/// `sync_acknowledge`.
#[tauri::command]
pub async fn sync_acknowledge(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    state.run(Backend::sync_acknowledge).await
}

/// `sync_devices`.
#[tauri::command]
pub async fn sync_devices(state: State<'_, AppState>) -> IpcResult<SyncDevices> {
    state.run(Backend::sync_devices).await
}

/// `sync_device_rename`.
#[tauri::command]
pub async fn sync_device_rename(
    state: State<'_, AppState>,
    request: DeviceRenameRequest,
) -> IpcResult<()> {
    state.run(move |b| b.sync_device_rename(&request)).await
}

/// `sync_device_remove`.
#[tauri::command]
pub async fn sync_device_remove(
    state: State<'_, AppState>,
    request: DeviceRemoveRequest,
) -> IpcResult<()> {
    state.run(move |b| b.sync_device_remove(&request)).await
}

/// `sync_totp_begin`.
#[tauri::command]
pub async fn sync_totp_begin(
    state: State<'_, AppState>,
    request: TotpBeginRequest,
) -> IpcResult<TotpSetupInfo> {
    state.run(move |b| b.sync_totp_begin(&request)).await
}

/// `sync_totp_confirm`.
#[tauri::command]
pub async fn sync_totp_confirm(state: State<'_, AppState>, request: CodeRequest) -> IpcResult<()> {
    state.run(move |b| b.sync_totp_confirm(&request)).await
}

/// `sync_totp_disable`.
#[tauri::command]
pub async fn sync_totp_disable(
    state: State<'_, AppState>,
    request: TotpDisableRequest,
) -> IpcResult<()> {
    state.run(move |b| b.sync_totp_disable(&request)).await
}

/// `join_hosted`.
#[tauri::command]
pub async fn join_hosted(
    state: State<'_, AppState>,
    request: JoinHostedRequest,
) -> IpcResult<JoinOutcome> {
    state.run(move |b| b.join_hosted(request)).await
}

/// `join_hosted_two_factor`.
#[tauri::command]
pub async fn join_hosted_two_factor(
    state: State<'_, AppState>,
    request: CodeRequest,
) -> IpcResult<JoinOutcome> {
    state.run(move |b| b.join_hosted_two_factor(&request)).await
}

/// `join_folder`.
#[tauri::command]
pub async fn join_folder(
    state: State<'_, AppState>,
    request: JoinFolderRequest,
) -> IpcResult<JoinOutcome> {
    state.run(move |b| b.join_folder(request)).await
}

/// `change_master_password`.
#[tauri::command]
pub async fn change_master_password(
    state: State<'_, AppState>,
    request: ChangePasswordRequest,
) -> IpcResult<()> {
    state.run(move |b| b.change_master_password(&request)).await
}
