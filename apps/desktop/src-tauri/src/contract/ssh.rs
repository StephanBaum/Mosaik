//! SSH agent (mosaic-ssh-agent): status, on/off, and the per-use approval.
//! A sign request arrives as the `sshApprovalRequested` event; the UI
//! answers with `ssh_respond`. Unanswered requests are refused when they
//! expire or the vault locks (`sshApprovalClosed`).

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::macros::wire_enum;

wire_enum!(
    /// The user's answer to a sign request.
    pub enum SshDecision {
        /// Sign this one request.
        AllowOnce,
        /// Sign requests for this key from this client until the vault locks.
        AllowUntilLock,
        /// Refuse.
        Deny,
    }
);

wire_enum!(
    /// Why an approval request closed.
    pub enum SshCloseReason {
        /// The user answered (possibly in another window).
        Answered,
        /// Nobody answered in time; refused.
        Expired,
        /// The vault locked; refused.
        Locked,
    }
);

/// The `ssh_agent_status` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshAgentStatus {
    /// The user turned the agent on.
    pub enabled: bool,
    /// It is listening.
    pub running: bool,
    /// The socket path or pipe name for `SSH_AUTH_SOCK` (`null` if not
    /// running).
    pub endpoint: Option<String>,
    /// SSH key items it serves (0 while locked).
    pub key_count: u32,
}

/// `ssh_agent_set_enabled`.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SshSetEnabledRequest {
    /// On or off.
    pub enabled: bool,
}

/// The process asking to sign, as far as the OS tells.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshClient {
    /// Executable name (`null` if unknown).
    pub process_name: Option<String>,
    /// Process ID (`null` if unknown).
    pub pid: Option<u32>,
}

/// A sign request waiting for approval.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshApprovalRequest {
    /// Handle for `ssh_respond`.
    pub request_id: String,
    /// The key's vault.
    pub vault_id: WireVaultId,
    /// The key's item.
    pub item_id: WireItemId,
    /// The key's title (`null` if empty).
    pub key_title: Option<String>,
    /// The public key fingerprint, `SHA256:…`.
    pub fingerprint: String,
    /// Who asks.
    pub client: SshClient,
    /// When it is refused without an answer.
    pub expires_at: UnixMillis,
}

/// `ssh_respond`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SshRespondRequest {
    /// From the event.
    pub request_id: String,
    /// The answer.
    pub decision: SshDecision,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn statuses() -> Vec<SshAgentStatus> {
        vec![
            SshAgentStatus {
                enabled: true,
                running: true,
                endpoint: Some(r"\\.\pipe\openssh-ssh-agent".into()),
                key_count: 2,
            },
            SshAgentStatus {
                enabled: false,
                running: false,
                endpoint: None,
                key_count: 0,
            },
        ]
    }

    pub(crate) fn approval() -> SshApprovalRequest {
        SshApprovalRequest {
            request_id: "ssh-1".into(),
            vault_id: ids::vault(1),
            item_id: ids::item(3),
            key_title: Some("Deploy key".into()),
            fingerprint: "SHA256:uNiVztksCsDhcc0u9e8BujQXVUpKZIDTMczCvj3tD2s".into(),
            client: SshClient {
                process_name: Some("git".into()),
                pid: Some(4242),
            },
            expires_at: 1_761_480_060_000,
        }
    }
}
