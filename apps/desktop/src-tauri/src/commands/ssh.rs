//! SSH agent commands, through the SSH agent service
//! (`adapters::SshAgentService`, `mosaic-ssh-agent`). Approval requests
//! reach the webview as events; the agent refuses them on lock. The setting
//! is stored on the device; the agent runs only while the vault is unlocked.

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
        self.adapters.ssh.status(self.stored().ssh_agent_enabled)
    }

    /// Turns the agent on or off and stores the choice. Turning it on needs
    /// an unlocked vault; turning it off also works while locked.
    ///
    /// # Errors
    /// `locked` (on), `sshAgentUnavailable`, `sshAgentEndpointInUse`.
    pub fn ssh_agent_set_enabled(
        &self,
        request: SshSetEnabledRequest,
    ) -> IpcResult<SshAgentStatus> {
        self.touch();
        if request.enabled {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            self.adapters.ssh.start(session, self.event_sink())?;
        } else {
            self.adapters.ssh.stop();
        }
        self.update_stored(|s| s.ssh_agent_enabled = request.enabled)?;
        Ok(self.ssh_agent_status())
    }

    /// Answers an approval request.
    ///
    /// # Errors
    /// `sshRequestExpired`.
    pub fn ssh_respond(&self, request: &SshRespondRequest) -> IpcResult<()> {
        self.touch();
        self.adapters.ssh.respond(request)
    }
}
