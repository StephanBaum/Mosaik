//! Sync commands (M6): thin wrappers over the `Backend` methods in
//! `sync/`. The work runs on the blocking pool: it talks to servers, folders
//! and native dialogs. Every command first drops what expired (pending
//! logins and joins with their keys, folder choices).

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::sync::{
    AcknowledgeRequest, ChangePasswordRequest, CheckServerRequest, CodeRequest,
    DeviceRemoveRequest, DeviceRenameRequest, FolderChoice, FolderConnectRequest,
    HostedConnectRequest, JoinFolderRequest, JoinHostedRequest, JoinOutcome, LoginMethods,
    RecoverPasswordRequest, ServerCheck, SyncDevices, SyncOverview, TotpBeginRequest,
    TotpDisableRequest, TotpSetupInfo, WebauthnAccountBegin, WebauthnAccountStatus,
    WebauthnAddRequest, WebauthnBegin, WebauthnConfirmRequest, WebauthnKeys, WebauthnRemoveRequest,
    WebauthnStatus,
};
use crate::state::{AppState, Backend};

/// Runs `work` on the blocking pool after the sweep of expired flows.
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

/// `sync_status`.
#[tauri::command]
pub async fn sync_status(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    run(&state, Backend::sync_status).await
}

/// `sync_check_server`.
#[tauri::command]
pub async fn sync_check_server(
    state: State<'_, AppState>,
    request: CheckServerRequest,
) -> IpcResult<ServerCheck> {
    run(&state, move |b| b.sync_check_server(&request)).await
}

/// `sync_hosted_connect`.
#[tauri::command]
pub async fn sync_hosted_connect(
    state: State<'_, AppState>,
    request: HostedConnectRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_hosted_connect(request)).await
}

/// `sync_hosted_two_factor`.
#[tauri::command]
pub async fn sync_hosted_two_factor(
    state: State<'_, AppState>,
    request: CodeRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_hosted_two_factor(&request)).await
}

/// `sync_pick_folder`.
#[tauri::command]
pub async fn sync_pick_folder(state: State<'_, AppState>) -> IpcResult<Option<FolderChoice>> {
    run(&state, Backend::sync_pick_folder).await
}

/// `sync_folder_connect`.
#[tauri::command]
pub async fn sync_folder_connect(
    state: State<'_, AppState>,
    request: FolderConnectRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_folder_connect(&request)).await
}

/// `sync_now`.
#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    run(&state, Backend::sync_now).await
}

/// `sync_disconnect`.
#[tauri::command]
pub async fn sync_disconnect(state: State<'_, AppState>) -> IpcResult<SyncOverview> {
    run(&state, Backend::sync_disconnect).await
}

/// `sync_acknowledge`.
#[tauri::command]
pub async fn sync_acknowledge(
    state: State<'_, AppState>,
    request: AcknowledgeRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_acknowledge(&request)).await
}

/// `sync_recover_password`.
#[tauri::command]
pub async fn sync_recover_password(
    state: State<'_, AppState>,
    request: RecoverPasswordRequest,
) -> IpcResult<SyncOverview> {
    run(&state, move |b| b.sync_recover_password(&request)).await
}

/// `sync_devices`.
#[tauri::command]
pub async fn sync_devices(state: State<'_, AppState>) -> IpcResult<SyncDevices> {
    run(&state, Backend::sync_devices).await
}

/// `sync_device_rename`.
#[tauri::command]
pub async fn sync_device_rename(
    state: State<'_, AppState>,
    request: DeviceRenameRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sync_device_rename(&request)).await
}

/// `sync_device_remove`.
#[tauri::command]
pub async fn sync_device_remove(
    state: State<'_, AppState>,
    request: DeviceRemoveRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sync_device_remove(&request)).await
}

/// `sync_totp_begin`.
#[tauri::command]
pub async fn sync_totp_begin(
    state: State<'_, AppState>,
    request: TotpBeginRequest,
) -> IpcResult<TotpSetupInfo> {
    run(&state, move |b| b.sync_totp_begin(&request)).await
}

/// `sync_totp_confirm`.
#[tauri::command]
pub async fn sync_totp_confirm(state: State<'_, AppState>, request: CodeRequest) -> IpcResult<()> {
    run(&state, move |b| b.sync_totp_confirm(&request)).await
}

/// `sync_totp_disable`.
#[tauri::command]
pub async fn sync_totp_disable(
    state: State<'_, AppState>,
    request: TotpDisableRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sync_totp_disable(&request)).await
}

/// `sync_login_methods`.
#[tauri::command]
pub async fn sync_login_methods(state: State<'_, AppState>) -> IpcResult<LoginMethods> {
    run(&state, Backend::sync_login_methods).await
}

/// `sync_hosted_webauthn_begin`.
#[tauri::command]
pub async fn sync_hosted_webauthn_begin(state: State<'_, AppState>) -> IpcResult<WebauthnBegin> {
    run(&state, Backend::sync_hosted_webauthn_begin).await
}

/// `join_hosted_webauthn_begin`.
#[tauri::command]
pub async fn join_hosted_webauthn_begin(state: State<'_, AppState>) -> IpcResult<WebauthnBegin> {
    run(&state, Backend::join_hosted_webauthn_begin).await
}

/// `sync_webauthn_status`.
#[tauri::command]
pub async fn sync_webauthn_status(state: State<'_, AppState>) -> IpcResult<WebauthnStatus> {
    run(&state, Backend::sync_webauthn_status).await
}

/// `sync_webauthn_reopen`.
#[tauri::command]
pub async fn sync_webauthn_reopen(state: State<'_, AppState>) -> IpcResult<()> {
    run(&state, Backend::sync_webauthn_reopen).await
}

/// `sync_webauthn_list`.
#[tauri::command]
pub async fn sync_webauthn_list(state: State<'_, AppState>) -> IpcResult<WebauthnKeys> {
    run(&state, Backend::sync_webauthn_list).await
}

/// `sync_webauthn_add_begin`.
#[tauri::command]
pub async fn sync_webauthn_add_begin(
    state: State<'_, AppState>,
    request: WebauthnAddRequest,
) -> IpcResult<WebauthnAccountBegin> {
    run(&state, move |b| b.sync_webauthn_add_begin(&request)).await
}

/// `sync_webauthn_add_status`.
#[tauri::command]
pub async fn sync_webauthn_add_status(
    state: State<'_, AppState>,
) -> IpcResult<WebauthnAccountStatus> {
    run(&state, Backend::sync_webauthn_add_status).await
}

/// `sync_webauthn_confirm_begin`.
#[tauri::command]
pub async fn sync_webauthn_confirm_begin(
    state: State<'_, AppState>,
    request: WebauthnConfirmRequest,
) -> IpcResult<WebauthnAccountBegin> {
    run(&state, move |b| b.sync_webauthn_confirm_begin(&request)).await
}

/// `sync_webauthn_remove`.
#[tauri::command]
pub async fn sync_webauthn_remove(
    state: State<'_, AppState>,
    request: WebauthnRemoveRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sync_webauthn_remove(&request)).await
}

/// `join_hosted`.
#[tauri::command]
pub async fn join_hosted(
    state: State<'_, AppState>,
    request: JoinHostedRequest,
) -> IpcResult<JoinOutcome> {
    run(&state, move |b| b.join_hosted(request)).await
}

/// `join_hosted_two_factor`.
#[tauri::command]
pub async fn join_hosted_two_factor(
    state: State<'_, AppState>,
    request: CodeRequest,
) -> IpcResult<JoinOutcome> {
    run(&state, move |b| b.join_hosted_two_factor(&request)).await
}

/// `join_folder`.
#[tauri::command]
pub async fn join_folder(
    state: State<'_, AppState>,
    request: JoinFolderRequest,
) -> IpcResult<JoinOutcome> {
    run(&state, move |b| b.join_folder(request)).await
}

/// `change_master_password`.
#[tauri::command]
pub async fn change_master_password(
    state: State<'_, AppState>,
    request: ChangePasswordRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.change_master_password(&request)).await
}
