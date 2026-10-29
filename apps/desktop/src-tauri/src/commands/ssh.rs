//! SSH agent commands, through the SSH agent adapter
//! (`adapters::SshAgentAdapter`, `mosaic-ssh-agent`). Approval requests
//! reach the webview as events; the adapter refuses them on lock.

use std::sync::Arc;

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::ssh::{SshAgentStatus, SshRespondRequest, SshSetEnabledRequest};
use crate::state::{AppState, Backend};

/// `ssh_agent_status`.
#[tauri::command]
pub async fn ssh_agent_status(state: State<'_, AppState>) -> IpcResult<SshAgentStatus> {
    Ok(state.backend().ssh_agent_status())
}

/// `ssh_agent_set_enabled`.
#[tauri::command]
pub async fn ssh_agent_set_enabled(
    state: State<'_, AppState>,
    request: SshSetEnabledRequest,
) -> IpcResult<SshAgentStatus> {
    state.run(move |b| b.ssh_agent_set_enabled(request)).await
}

/// `ssh_respond`.
#[tauri::command]
pub async fn ssh_respond(state: State<'_, AppState>, request: SshRespondRequest) -> IpcResult<()> {
    state.run(move |b| b.ssh_respond(&request)).await
}

impl Backend {
    /// The agent's state.
    #[must_use]
    pub fn ssh_agent_status(&self) -> SshAgentStatus {
        self.adapters.ssh.status(self.is_unlocked())
    }

    /// Turns the agent on or off.
    ///
    /// # Errors
    /// The adapter's errors (`sshAgentUnavailable`).
    pub fn ssh_agent_set_enabled(
        &self,
        request: SshSetEnabledRequest,
    ) -> IpcResult<SshAgentStatus> {
        self.touch();
        let events = self.events.clone();
        self.adapters
            .ssh
            .set_enabled(request.enabled, Arc::new(move |event| events.emit(&event)))
    }

    /// Answers an approval request.
    ///
    /// # Errors
    /// The adapter's errors (`sshRequestExpired`).
    pub fn ssh_respond(&self, request: &SshRespondRequest) -> IpcResult<()> {
        self.touch();
        self.adapters.ssh.respond(request)
    }
}
