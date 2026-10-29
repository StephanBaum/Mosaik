//! The crates of the other M3 tracks, as services the backend owns:
//! import ([`ImportService`], `mosaic-import`) and the SSH agent
//! ([`SshAgentService`], `mosaic-ssh-agent`). Security insights need no
//! service: `commands::insights` calls `mosaic-insights` directly.
//!
//! The shell owns what is shell work (the file dialog and the file tokens,
//! the session lock, events, lock handling); the services own the format
//! parsing and the agent. The contracts they keep:
//!
//! - Errors are `IpcError` codes only (`docs/DESKTOP_IPC.md` section 5).
//! - Nothing secret leaves through a return value except what the IPC
//!   contract allows (none of these responses carries a secret).
//! - On lock they drop every decrypted or parsed secret they hold
//!   (`ImportService::clear`, `SshAgentService::stop`).
//!
//! The desktop fuzz crate includes `src/contract/` by path, so the contract
//! modules never import these crates: every conversion lives here and in
//! `commands/` (`docs/DESKTOP_IPC.md` section 8).

mod import;
mod ssh;

pub use import::ImportService;
pub use ssh::SshAgentService;

use crate::contract::events::AppEvent;

/// Emits an event to the webview (for the SSH agent's approvals).
pub type EventSink = std::sync::Arc<dyn Fn(AppEvent) + Send + Sync>;

/// The services the backend calls.
pub struct Adapters {
    /// Import.
    pub import: ImportService,
    /// The SSH agent.
    pub ssh: SshAgentService,
}

impl Default for Adapters {
    fn default() -> Self {
        Self {
            import: ImportService::default(),
            ssh: SshAgentService::new(None),
        }
    }
}
