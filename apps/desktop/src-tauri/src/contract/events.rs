//! Events from Rust to the webview. They travel over a Tauri `Channel` that
//! the webview hands to `subscribe_events`, so the capability needs no
//! `core:event` permission (ADR-0019). The latest subscription wins (one
//! main window; a reload subscribes again).

use serde::Serialize;

use super::errors::ErrorCode;
use super::ids::UnixMillis;
use super::macros::wire_enum;
use super::ssh::{SshApprovalRequest, SshCloseReason};

wire_enum!(
    /// Why the vault locked.
    pub enum LockReason {
        /// The user locked it.
        Manual,
        /// Auto-lock after the idle timeout.
        Idle,
        /// The system went to sleep.
        Sleep,
        /// The screen locked.
        ScreenLock,
    }
);

wire_enum!(
    /// Sync status for the status bar (sync itself is M6).
    pub enum SyncStatus {
        /// No sync server set up.
        NotConfigured,
        /// Server unreachable; changes stay on this device.
        Offline,
        /// Up- or downloading.
        Syncing,
        /// Everything is in sync.
        UpToDate,
        /// The last sync failed (see `error`).
        Error,
    }
);

/// The sync state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    /// Status.
    pub status: SyncStatus,
    /// Last successful sync (`null` if never).
    pub last_synced_at: Option<UnixMillis>,
    /// Why the last sync failed (`null` unless `status` is `error`).
    pub error: Option<ErrorCode>,
}

/// An event.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AppEvent {
    /// The vault locked (any reason). The UI drops every revealed secret
    /// and returns to the unlock screen.
    Locked {
        /// Why.
        reason: LockReason,
    },
    /// The sync state changed.
    SyncStateChanged {
        /// The new state.
        state: SyncState,
    },
    /// Rust cleared the clipboard after the timeout.
    ClipboardCleared,
    /// An SSH client asks to sign with a vault key.
    SshApprovalRequested {
        /// The request.
        request: SshApprovalRequest,
    },
    /// An approval request is no longer open.
    SshApprovalClosed {
        /// The request.
        request_id: String,
        /// Why.
        reason: SshCloseReason,
    },
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ssh::samples as ssh;

    pub(crate) fn events() -> Vec<AppEvent> {
        let all = vec![
            AppEvent::Locked {
                reason: LockReason::Idle,
            },
            AppEvent::SyncStateChanged {
                state: SyncState {
                    status: SyncStatus::Error,
                    last_synced_at: Some(1_761_480_000_000),
                    error: Some(ErrorCode::Internal),
                },
            },
            AppEvent::SyncStateChanged {
                state: SyncState {
                    status: SyncStatus::NotConfigured,
                    last_synced_at: None,
                    error: None,
                },
            },
            AppEvent::ClipboardCleared,
            AppEvent::SshApprovalRequested {
                request: ssh::approval(),
            },
            AppEvent::SshApprovalClosed {
                request_id: "ssh-1".into(),
                reason: SshCloseReason::Expired,
            },
        ];
        for event in &all {
            // Adding a variant fails to compile here until it has a sample.
            match event {
                AppEvent::Locked { .. }
                | AppEvent::SyncStateChanged { .. }
                | AppEvent::ClipboardCleared
                | AppEvent::SshApprovalRequested { .. }
                | AppEvent::SshApprovalClosed { .. } => {}
            }
        }
        all
    }
}
