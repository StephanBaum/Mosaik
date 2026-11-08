//! Sharing and emergency access commands (M6b): thin wrappers over the
//! `Backend` methods in `sync/sharing_flow.rs`. The work runs on the blocking
//! pool: it talks to the server and opens native dialogs. Every command
//! first drops what expired (pending logins and joins, folder choices).

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::sharing::{
    ContactCodeRequest, ContactList, EmergencyClaimRequest, EmergencyClaimed,
    EmergencyGrantRequest, EmergencyOverview, EmergencyRequestRequest, EmergencyRequested,
    EmergencyRespondRequest, EmergencyRevokeRequest, InviteAcceptRequest, InviteDeclineRequest,
    InviteList, InvitePreviewView, InviteRequest, InviteSent, LookupRequest, LookupResult,
    MemberRequest, RemoveOutcome, SetRoleRequest, ShareVaultRequest, SharingIdentity,
    VaultMembersView,
};
use crate::contract::vaults::VaultRequest;
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

/// `sharing_identity`.
#[tauri::command]
pub async fn sharing_identity(state: State<'_, AppState>) -> IpcResult<SharingIdentity> {
    run(&state, Backend::sharing_identity).await
}

/// `sharing_lookup`.
#[tauri::command]
pub async fn sharing_lookup(
    state: State<'_, AppState>,
    request: LookupRequest,
) -> IpcResult<LookupResult> {
    run(&state, move |b| b.sharing_lookup(&request)).await
}

/// `sharing_verify_contact`.
#[tauri::command]
pub async fn sharing_verify_contact(
    state: State<'_, AppState>,
    request: ContactCodeRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sharing_verify_contact(&request)).await
}

/// `sharing_accept_key_change`.
#[tauri::command]
pub async fn sharing_accept_key_change(
    state: State<'_, AppState>,
    request: ContactCodeRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.sharing_accept_key_change(&request)).await
}

/// `sharing_contacts`.
#[tauri::command]
pub async fn sharing_contacts(state: State<'_, AppState>) -> IpcResult<ContactList> {
    run(&state, Backend::sharing_contacts).await
}

/// `vault_share`.
#[tauri::command]
pub async fn vault_share(
    state: State<'_, AppState>,
    request: ShareVaultRequest,
) -> IpcResult<InviteSent> {
    run(&state, move |b| b.vault_share(&request)).await
}

/// `vault_members`.
#[tauri::command]
pub async fn vault_members(
    state: State<'_, AppState>,
    request: VaultRequest,
) -> IpcResult<VaultMembersView> {
    run(&state, move |b| b.vault_members(&request)).await
}

/// `vault_member_set_role`.
#[tauri::command]
pub async fn vault_member_set_role(
    state: State<'_, AppState>,
    request: SetRoleRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.vault_member_set_role(&request)).await
}

/// `vault_member_remove`.
#[tauri::command]
pub async fn vault_member_remove(
    state: State<'_, AppState>,
    request: MemberRequest,
) -> IpcResult<RemoveOutcome> {
    run(&state, move |b| b.vault_member_remove(&request)).await
}

/// `vault_invite_cancel`.
#[tauri::command]
pub async fn vault_invite_cancel(
    state: State<'_, AppState>,
    request: InviteRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.vault_invite_cancel(&request)).await
}

/// `vault_leave`.
#[tauri::command]
pub async fn vault_leave(state: State<'_, AppState>, request: VaultRequest) -> IpcResult<()> {
    run(&state, move |b| b.vault_leave(&request)).await
}

/// `invites_list`.
#[tauri::command]
pub async fn invites_list(state: State<'_, AppState>) -> IpcResult<InviteList> {
    run(&state, Backend::invites_list).await
}

/// `invite_preview`.
#[tauri::command]
pub async fn invite_preview(
    state: State<'_, AppState>,
    request: InviteRequest,
) -> IpcResult<InvitePreviewView> {
    run(&state, move |b| b.invite_preview(&request)).await
}

/// `invite_accept`.
#[tauri::command]
pub async fn invite_accept(
    state: State<'_, AppState>,
    request: InviteAcceptRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.invite_accept(&request)).await
}

/// `invite_decline`.
#[tauri::command]
pub async fn invite_decline(
    state: State<'_, AppState>,
    request: InviteDeclineRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.invite_decline(&request)).await
}

/// `emergency_overview`.
#[tauri::command]
pub async fn emergency_overview(state: State<'_, AppState>) -> IpcResult<EmergencyOverview> {
    run(&state, Backend::emergency_overview).await
}

/// `emergency_grant`.
#[tauri::command]
pub async fn emergency_grant(
    state: State<'_, AppState>,
    request: EmergencyGrantRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.emergency_grant(&request)).await
}

/// `emergency_revoke`.
#[tauri::command]
pub async fn emergency_revoke(
    state: State<'_, AppState>,
    request: EmergencyRevokeRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.emergency_revoke(&request)).await
}

/// `emergency_request`.
#[tauri::command]
pub async fn emergency_request(
    state: State<'_, AppState>,
    request: EmergencyRequestRequest,
) -> IpcResult<EmergencyRequested> {
    run(&state, move |b| b.emergency_request(&request)).await
}

/// `emergency_respond`.
#[tauri::command]
pub async fn emergency_respond(
    state: State<'_, AppState>,
    request: EmergencyRespondRequest,
) -> IpcResult<()> {
    run(&state, move |b| b.emergency_respond(&request)).await
}

/// `emergency_claim`.
#[tauri::command]
pub async fn emergency_claim(
    state: State<'_, AppState>,
    request: EmergencyClaimRequest,
) -> IpcResult<EmergencyClaimed> {
    run(&state, move |b| b.emergency_claim(&request)).await
}
