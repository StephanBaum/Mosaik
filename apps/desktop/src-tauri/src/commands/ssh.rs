//! SSH agent commands: not built yet (SSH agent track, on top of
//! `mosaic-ssh-agent`).

use tauri::State;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::ssh::{SshAgentStatus, SshRespondRequest, SshSetEnabledRequest};
use crate::state::AppState;

/// `ssh_agent_status`.
#[tauri::command]
pub async fn ssh_agent_status(state: State<'_, AppState>) -> IpcResult<SshAgentStatus> {
    let _ = state;
    Ok(SshAgentStatus {
        enabled: false,
        running: false,
        endpoint: None,
        key_count: 0,
    })
}

/// `ssh_agent_set_enabled`.
#[tauri::command]
pub async fn ssh_agent_set_enabled(
    state: State<'_, AppState>,
    request: SshSetEnabledRequest,
) -> IpcResult<SshAgentStatus> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}

/// `ssh_respond`.
#[tauri::command]
pub async fn ssh_respond(state: State<'_, AppState>, request: SshRespondRequest) -> IpcResult<()> {
    let _ = (state, request);
    Err(IpcError::NOT_IMPLEMENTED)
}
