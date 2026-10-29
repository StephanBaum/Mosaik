//! Hooks for the crates of the other M3 tracks: import (`mosaic-import`),
//! security insights (`mosaic-insights`) and the SSH agent
//! (`mosaic-ssh-agent`). The shell owns what is shell work (the file
//! dialog and the file tokens, the session lock, events, lock handling);
//! the adapters own the format parsing, the analysis and the agent.
//!
//! Each track replaces the stub ([`NotWired`]) with an implementation of
//! its trait in `lib.rs` (`Adapters`). The contracts every implementation
//! keeps:
//!
//! - Errors are `IpcError` codes only (`docs/DESKTOP_IPC.md` §5).
//! - Nothing secret leaves through a return value except what the IPC
//!   contract allows (none of these responses carries a secret).
//! - `on_lock` drops every decrypted or parsed secret the adapter holds
//!   (import previews, SSH approvals in flight, the agent's identities).

use std::path::Path;

use mosaic_core::SecretString;
use mosaic_vault::session::UnlockedSession;

use crate::contract::errors::{IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::ids::UnixMillis;
use crate::contract::import::{ImportCommitRequest, ImportFormat, ImportPreview, ImportResult};
use crate::contract::insights::SecurityReport;
use crate::contract::ssh::{SshAgentStatus, SshRespondRequest};

/// Import (`mosaic-import`).
pub trait ImportAdapter: Send + Sync {
    /// Reads and parses `file` and returns the preview. The adapter keeps
    /// the parsed entries (zeroizing) under the preview ID.
    ///
    /// # Errors
    /// `importFileUnreadable`, `importFormatInvalid`, `wrongCredentials`.
    fn preview(
        &self,
        file: &Path,
        format: ImportFormat,
        password: Option<SecretString>,
        session: &UnlockedSession,
    ) -> IpcResult<ImportPreview>;

    /// Writes the previewed entries (minus `skip`) into the target vault
    /// and drops the preview.
    ///
    /// # Errors
    /// `importPreviewExpired`, `vaultNotFound`, session errors.
    fn commit(
        &self,
        request: &ImportCommitRequest,
        session: &mut UnlockedSession,
    ) -> IpcResult<ImportResult>;

    /// Drops a preview (no-op if it is gone).
    fn cancel(&self, preview_id: &str);

    /// Drops every preview (the vault locked).
    fn on_lock(&self);
}

/// Security insights (`mosaic-insights`).
pub trait InsightsAdapter: Send + Sync {
    /// The report over the unlocked session's items at `now`.
    ///
    /// # Errors
    /// Session errors.
    fn report(&self, session: &UnlockedSession, now: UnixMillis) -> IpcResult<SecurityReport>;
}

/// Emits an event to the webview (for the SSH agent's approvals).
pub type EventSink = std::sync::Arc<dyn Fn(AppEvent) + Send + Sync>;

/// The SSH agent (`mosaic-ssh-agent`).
pub trait SshAgentAdapter: Send + Sync {
    /// The agent's state; `unlocked` says whether keys can be served.
    fn status(&self, unlocked: bool) -> SshAgentStatus;

    /// Turns the agent on or off. Approval requests go out through
    /// `events` as `sshApprovalRequested` / `sshApprovalClosed`.
    ///
    /// # Errors
    /// `sshAgentUnavailable`.
    fn set_enabled(&self, enabled: bool, events: EventSink) -> IpcResult<SshAgentStatus>;

    /// The user's answer to an approval request.
    ///
    /// # Errors
    /// `sshRequestExpired`.
    fn respond(&self, request: &SshRespondRequest) -> IpcResult<()>;

    /// Refuses every open request (`sshApprovalClosed` with `locked`) and
    /// forgets the served keys.
    fn on_lock(&self);
}

/// The stub until a track wires its crate: commands answer
/// `notImplemented`, and the SSH agent reports "off".
pub struct NotWired;

impl ImportAdapter for NotWired {
    fn preview(
        &self,
        _: &Path,
        _: ImportFormat,
        _: Option<SecretString>,
        _: &UnlockedSession,
    ) -> IpcResult<ImportPreview> {
        Err(IpcError::NOT_IMPLEMENTED)
    }

    fn commit(&self, _: &ImportCommitRequest, _: &mut UnlockedSession) -> IpcResult<ImportResult> {
        Err(IpcError::NOT_IMPLEMENTED)
    }

    fn cancel(&self, _: &str) {}

    fn on_lock(&self) {}
}

impl InsightsAdapter for NotWired {
    fn report(&self, _: &UnlockedSession, _: UnixMillis) -> IpcResult<SecurityReport> {
        Err(IpcError::NOT_IMPLEMENTED)
    }
}

impl SshAgentAdapter for NotWired {
    fn status(&self, _: bool) -> SshAgentStatus {
        SshAgentStatus {
            enabled: false,
            running: false,
            endpoint: None,
            key_count: 0,
        }
    }

    fn set_enabled(&self, _: bool, _: EventSink) -> IpcResult<SshAgentStatus> {
        Err(IpcError::NOT_IMPLEMENTED)
    }

    fn respond(&self, _: &SshRespondRequest) -> IpcResult<()> {
        Err(IpcError::NOT_IMPLEMENTED)
    }

    fn on_lock(&self) {}
}

/// The adapters the backend calls.
pub struct Adapters {
    /// Import.
    pub import: Box<dyn ImportAdapter>,
    /// Security insights.
    pub insights: Box<dyn InsightsAdapter>,
    /// SSH agent.
    pub ssh: Box<dyn SshAgentAdapter>,
}

impl Default for Adapters {
    fn default() -> Self {
        Self {
            import: Box::new(NotWired),
            insights: Box::new(NotWired),
            ssh: Box::new(NotWired),
        }
    }
}
