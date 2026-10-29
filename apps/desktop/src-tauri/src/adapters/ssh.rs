//! The SSH agent (`mosaic-ssh-agent`, ADR-0025). Off by default, running
//! only while the vault is unlocked: it starts after unlock (when the user
//! turned it on), stops before the `locked` event and at quit, and its key
//! set is rebuilt after every item change.

use std::str::FromStr as _;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_ssh_agent::{
    AgentConfig, AgentHandle, ApprovalEvent, ApprovalPrompts, CloseReason, DEFAULT_GRANT, Decision,
    Endpoint, KeyError, KeySet, RequestId, SignFailure, SkipReason,
};
use mosaic_vault::session::{ItemFilter, UnlockedSession};

use super::EventSink;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::ids::UnixMillis;
use crate::contract::ssh::{
    SshAgentStatus, SshApprovalRequest, SshClient, SshCloseReason, SshDecision, SshRespondRequest,
    SshSignFailReason, SshSkipReason, SshSkippedKey,
};
use crate::state::guard;

type Notify = Box<dyn Fn(ApprovalEvent) + Send + Sync>;
type Prompts = ApprovalPrompts<Notify>;

struct Running {
    handle: AgentHandle<Prompts>,
    prompts: Arc<Prompts>,
}

/// The agent behind the desktop IPC.
pub struct SshAgentService {
    endpoint: Option<Endpoint>,
    running: Mutex<Option<Running>>,
    skipped: Mutex<Vec<SshSkippedKey>>,
    last_error: Mutex<Option<ErrorCode>>,
}

impl SshAgentService {
    /// A service that listens on `endpoint` (`None`: the agent cannot start).
    #[must_use]
    pub const fn new(endpoint: Option<Endpoint>) -> Self {
        Self {
            endpoint,
            running: Mutex::new(None),
            skipped: Mutex::new(Vec::new()),
            last_error: Mutex::new(None),
        }
    }

    /// The endpoint of this OS. Windows: the OpenSSH pipe. Linux:
    /// `$XDG_RUNTIME_DIR/mosaic/agent.sock`, else
    /// `<app_local_data_dir>/ssh/agent.sock`. macOS:
    /// `<app_local_data_dir>/ssh/agent.sock`. The directory is created
    /// private (mode 0700) on Unix.
    #[must_use]
    pub fn for_this_os(app_local_data_dir: &std::path::Path) -> Self {
        if cfg!(windows) {
            return Self::new(Some(Endpoint::platform_default(app_local_data_dir)));
        }
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .filter(|p| cfg!(target_os = "linux") && p.is_absolute());
        let dir = runtime.map_or_else(|| app_local_data_dir.join("ssh"), |p| p.join("mosaic"));
        if create_private_dir(&dir).is_err() {
            return Self::new(None);
        }
        Self::new(Some(Endpoint::platform_default(&dir)))
    }

    /// The agent's state. `enabled` is the stored setting.
    #[must_use]
    pub fn status(&self, enabled: bool) -> SshAgentStatus {
        let running = self.running.lock().ok();
        let running = running.as_deref().and_then(Option::as_ref);
        SshAgentStatus {
            enabled,
            running: running.is_some(),
            endpoint: running.map(|r| r.handle.endpoint().auth_sock()),
            key_count: running.map_or(0, |r| {
                u32::try_from(r.handle.core().key_count()).unwrap_or(u32::MAX)
            }),
            skipped: self.skipped.lock().map(|s| s.clone()).unwrap_or_default(),
            error: self.last_error.lock().ok().and_then(|e| *e),
        }
    }

    /// Starts the agent on the session's keys (no-op if it runs).
    /// Approval requests go out through `events`.
    ///
    /// # Errors
    /// `sshAgentEndpointInUse`, `sshAgentUnavailable`. The code is also kept
    /// for [`SshAgentService::status`].
    pub fn start(&self, session: &UnlockedSession, events: EventSink) -> IpcResult<()> {
        let mut running = guard(&self.running)?;
        if running.is_some() {
            return Ok(());
        }
        let result = self.start_new(session, events);
        let error = result.as_ref().err().map(|e| e.code);
        if let Ok(mut last) = self.last_error.lock() {
            *last = error;
        }
        *running = Some(result?);
        Ok(())
    }

    fn start_new(&self, session: &UnlockedSession, events: EventSink) -> IpcResult<Running> {
        let endpoint = self
            .endpoint
            .clone()
            .ok_or(IpcError::new(ErrorCode::SshAgentUnavailable))?;
        let (keys, skipped) = key_set(session);
        let notify: Notify = Box::new(move |event| events(to_app_event(event)));
        let prompts = Arc::new(ApprovalPrompts::new(notify));
        let runtime = tauri::async_runtime::handle();
        let _guard = runtime.inner().enter();
        let handle =
            AgentHandle::start(endpoint, keys, Arc::clone(&prompts), AgentConfig::default())
                .map_err(map_start_error)?;
        if let Ok(mut list) = self.skipped.lock() {
            *list = skipped;
        }
        Ok(Running { handle, prompts })
    }

    /// Rebuilds the served keys after an item changed. No-op if the agent
    /// does not run.
    pub fn keys_changed(&self, session: &UnlockedSession) {
        let Ok(running) = self.running.lock() else {
            return;
        };
        let Some(running) = running.as_ref() else {
            return;
        };
        let (keys, skipped) = key_set(session);
        running.handle.core().set_keys(keys);
        if let Ok(mut list) = self.skipped.lock() {
            *list = skipped;
        }
    }

    /// The user's answer to an approval request.
    ///
    /// # Errors
    /// `sshRequestExpired` (unknown or closed request, or no agent).
    pub fn respond(&self, request: &SshRespondRequest) -> IpcResult<()> {
        let expired = || IpcError::new(ErrorCode::SshRequestExpired);
        let id = RequestId::from_str(&request.request_id).map_err(|_| expired())?;
        let running = guard(&self.running)?;
        let running = running.as_ref().ok_or_else(expired)?;
        running
            .prompts
            .respond(id, decision(request.decision))
            .map_err(|_| expired())
    }

    /// Stops the agent: keys, grants and open requests are dropped now
    /// (requests close as locked); the endpoint closes in the background.
    pub fn stop(&self) {
        let taken = self.running.lock().ok().and_then(|mut r| r.take());
        if let Ok(mut list) = self.skipped.lock() {
            list.clear();
        }
        if let Some(running) = taken {
            running.handle.core().stop();
            let handle = running.handle;
            drop(running.prompts);
            tauri::async_runtime::spawn(async move { handle.stop().await });
        }
    }

    /// Stops the agent and waits until its endpoint is closed (quit).
    pub fn shutdown(&self) {
        let taken = self.running.lock().ok().and_then(|mut r| r.take());
        if let Some(running) = taken {
            running.handle.core().stop();
            tauri::async_runtime::block_on(running.handle.stop());
        }
    }
}

#[cfg(unix)]
fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// The agent's keys: every SSH key item of the session.
fn key_set(session: &UnlockedSession) -> (KeySet, Vec<SshSkippedKey>) {
    let items = session.items(&ItemFilter::default());
    let (keys, skipped) = KeySet::from_items(items.iter().map(|r| (r.vault_id, r.item_id, r.item)));
    let skipped = skipped
        .into_iter()
        .map(|s| SshSkippedKey {
            vault_id: s.vault.into(),
            item_id: s.item.into(),
            reason: skip_reason(s.reason),
        })
        .collect();
    (keys, skipped)
}

const fn skip_reason(reason: SkipReason) -> SshSkipReason {
    match reason {
        SkipReason::Key(KeyError::Unsupported) => SshSkipReason::Unsupported,
        SkipReason::Duplicate => SshSkipReason::Duplicate,
        _ => SshSkipReason::Malformed,
    }
}

fn map_start_error(error: mosaic_ssh_agent::Error) -> IpcError {
    IpcError::new(match error {
        mosaic_ssh_agent::Error::EndpointInUse => ErrorCode::SshAgentEndpointInUse,
        _ => ErrorCode::SshAgentUnavailable,
    })
}

const fn decision(decision: SshDecision) -> Decision {
    match decision {
        SshDecision::AllowOnce => Decision::AllowOnce,
        SshDecision::AllowFor15Minutes => Decision::AllowFor(DEFAULT_GRANT),
        SshDecision::Deny => Decision::Deny,
    }
}

fn millis(time: SystemTime) -> UnixMillis {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| {
        UnixMillis::try_from(d.as_millis()).unwrap_or(UnixMillis::MAX)
    })
}

const fn fail_reason(failure: SignFailure) -> SshSignFailReason {
    match failure {
        SignFailure::PassphraseRequired => SshSignFailReason::PassphraseRequired,
        SignFailure::WrongPassphrase => SshSignFailReason::WrongPassphrase,
        SignFailure::Malformed => SshSignFailReason::KeyMalformed,
        SignFailure::KdfTooExpensive => SshSignFailReason::KdfTooExpensive,
        SignFailure::PublicKeyMismatch => SshSignFailReason::PublicKeyMismatch,
        SignFailure::Unsupported => SshSignFailReason::Unsupported,
        _ => SshSignFailReason::Internal,
    }
}

const fn close_reason(reason: CloseReason) -> SshCloseReason {
    match reason {
        CloseReason::Answered => SshCloseReason::Answered,
        CloseReason::Expired => SshCloseReason::Expired,
        CloseReason::Locked => SshCloseReason::Locked,
    }
}

fn to_app_event(event: ApprovalEvent) -> AppEvent {
    match event {
        ApprovalEvent::Requested(r) => AppEvent::SshApprovalRequested {
            request: SshApprovalRequest {
                request_id: r.id.to_string(),
                vault_id: r.vault.into(),
                item_id: r.item.into(),
                key_title: r.key_title,
                fingerprint: r.fingerprint,
                client: SshClient {
                    process_name: r.client.process_name,
                    pid: r.client.pid,
                },
                requested_at: millis(r.requested_at),
                expires_at: millis(r.expires_at),
                can_remember: r.can_remember,
            },
        },
        ApprovalEvent::Closed { id, reason } => AppEvent::SshApprovalClosed {
            request_id: id.to_string(),
            reason: close_reason(reason),
        },
        ApprovalEvent::SignFailed(n) => AppEvent::SshSignFailed {
            request_id: n.id.to_string(),
            vault_id: n.vault.into(),
            item_id: n.item.into(),
            reason: fail_reason(n.failure),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_failures_map_to_reasons() {
        let cases = [
            (
                SignFailure::PassphraseRequired,
                SshSignFailReason::PassphraseRequired,
            ),
            (
                SignFailure::WrongPassphrase,
                SshSignFailReason::WrongPassphrase,
            ),
            (SignFailure::Malformed, SshSignFailReason::KeyMalformed),
            (
                SignFailure::KdfTooExpensive,
                SshSignFailReason::KdfTooExpensive,
            ),
            (
                SignFailure::PublicKeyMismatch,
                SshSignFailReason::PublicKeyMismatch,
            ),
            (SignFailure::Unsupported, SshSignFailReason::Unsupported),
            (SignFailure::Internal, SshSignFailReason::Internal),
        ];
        for (failure, reason) in cases {
            assert_eq!(fail_reason(failure), reason);
        }
    }

    #[test]
    fn skip_reasons_map() {
        assert_eq!(
            skip_reason(SkipReason::Key(KeyError::Unsupported)),
            SshSkipReason::Unsupported
        );
        assert_eq!(skip_reason(SkipReason::Duplicate), SshSkipReason::Duplicate);
        assert_eq!(
            skip_reason(SkipReason::Key(KeyError::Malformed)),
            SshSkipReason::Malformed
        );
    }

    #[test]
    fn decisions_map() {
        assert_eq!(decision(SshDecision::AllowOnce), Decision::AllowOnce);
        assert_eq!(decision(SshDecision::Deny), Decision::Deny);
        assert_eq!(
            decision(SshDecision::AllowFor15Minutes),
            Decision::AllowFor(DEFAULT_GRANT)
        );
    }

    #[test]
    fn start_errors_map() {
        assert_eq!(
            map_start_error(mosaic_ssh_agent::Error::EndpointInUse).code,
            ErrorCode::SshAgentEndpointInUse
        );
        assert_eq!(
            map_start_error(mosaic_ssh_agent::Error::NoRuntime).code,
            ErrorCode::SshAgentUnavailable
        );
    }

    #[test]
    fn closed_and_failed_events_map() {
        let id = RequestId::from_str("ssh-1").unwrap();
        assert!(matches!(
            to_app_event(ApprovalEvent::Closed {
                id,
                reason: CloseReason::Expired
            }),
            AppEvent::SshApprovalClosed {
                reason: SshCloseReason::Expired,
                ..
            }
        ));
    }

    fn json_of(event: AppEvent) -> serde_json::Value {
        serde_json::to_value(event).unwrap()
    }

    #[test]
    fn requested_events_carry_what_the_prompt_shows_and_nothing_else() {
        let vault = mosaic_core::VaultId::new_v7().unwrap();
        let item = mosaic_core::ItemId::new_v7().unwrap();
        let request = mosaic_ssh_agent::ApprovalRequest {
            id: RequestId::from_str("ssh-7").unwrap(),
            vault,
            item,
            key_title: Some("deploy key".into()),
            fingerprint: "SHA256:abc".into(),
            algorithm: mosaic_ssh_agent::KeyAlgorithm::Ed25519,
            client: mosaic_ssh_agent::ClientInfo {
                pid: Some(4242),
                process_name: Some("ssh".into()),
                executable: Some("/usr/bin/ssh".into()),
            },
            requested_at: UNIX_EPOCH + std::time::Duration::from_millis(1_500),
            expires_at: UNIX_EPOCH + std::time::Duration::from_millis(31_500),
            can_remember: true,
        };
        let json = json_of(to_app_event(ApprovalEvent::Requested(request)));
        let request = &json["request"];
        assert_eq!(request["requestId"], "ssh-7");
        assert_eq!(request["keyTitle"], "deploy key");
        assert_eq!(request["fingerprint"], "SHA256:abc");
        assert_eq!(request["client"]["processName"], "ssh");
        assert_eq!(request["client"]["pid"], 4242);
        assert_eq!(request["requestedAt"], 1_500);
        assert_eq!(request["expiresAt"], 31_500);
        assert_eq!(request["canRemember"], true);
        // The executable path is for grants inside the agent; it never
        // reaches the webview.
        assert!(!json.to_string().contains("/usr/bin/ssh"));
    }

    #[test]
    fn sign_failed_events_name_the_request_and_the_reason() {
        let id = RequestId::from_str("ssh-9").unwrap();
        let json = json_of(to_app_event(ApprovalEvent::SignFailed(
            mosaic_ssh_agent::SignFailed {
                id,
                vault: mosaic_core::VaultId::new_v7().unwrap(),
                item: mosaic_core::ItemId::new_v7().unwrap(),
                failure: SignFailure::WrongPassphrase,
            },
        )));
        assert_eq!(json["requestId"], "ssh-9");
        assert_eq!(json["reason"], "wrongPassphrase");
    }

    #[test]
    fn every_close_reason_maps_to_its_own_wire_value() {
        assert_eq!(
            close_reason(CloseReason::Answered),
            SshCloseReason::Answered
        );
        assert_eq!(close_reason(CloseReason::Expired), SshCloseReason::Expired);
        assert_eq!(close_reason(CloseReason::Locked), SshCloseReason::Locked);
    }

    #[test]
    fn times_before_the_epoch_clamp_to_zero_and_later_ones_are_milliseconds() {
        assert_eq!(millis(UNIX_EPOCH), 0);
        assert_eq!(millis(UNIX_EPOCH - std::time::Duration::from_secs(5)), 0);
        assert_eq!(
            millis(UNIX_EPOCH + std::time::Duration::from_millis(1_234)),
            1_234
        );
    }

    #[test]
    fn other_start_errors_are_reported_as_unavailable() {
        for error in [
            mosaic_ssh_agent::Error::EndpointInvalid,
            mosaic_ssh_agent::Error::EndpointInsecure,
            mosaic_ssh_agent::Error::Io(std::io::ErrorKind::PermissionDenied),
        ] {
            assert_eq!(map_start_error(error).code, ErrorCode::SshAgentUnavailable);
        }
    }

    #[test]
    fn ids_that_are_not_request_ids_expire_like_unknown_ones() {
        let service = SshAgentService::new(None);
        for bad in ["", "ssh-", "ssh--1", "7", "ssh-1x", "SSH-1"] {
            let error = service
                .respond(&SshRespondRequest {
                    request_id: bad.into(),
                    decision: SshDecision::AllowOnce,
                })
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::SshRequestExpired, "{bad:?}");
        }
    }

    #[test]
    fn a_service_without_an_endpoint_does_not_run_and_rejects_ids() {
        let service = SshAgentService::new(None);
        let status = service.status(true);
        assert!(status.enabled && !status.running);
        assert_eq!(status.key_count, 0);
        let error = service
            .respond(&SshRespondRequest {
                request_id: "x".into(),
                decision: SshDecision::Deny,
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::SshRequestExpired);
        service.stop();
    }
}
