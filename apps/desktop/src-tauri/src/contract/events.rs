//! Events from Rust to the webview. They travel over a Tauri `Channel` that
//! the webview hands to `subscribe_events`, so the capability needs no
//! `core:event` permission (ADR-0019). The latest subscription wins (one
//! main window; a reload subscribes again).

use serde::Serialize;

use super::browser::{
    BrowserCloseReason, BrowserPairingRequest, BrowserPasskeyRequest, BrowserSaveRequest,
};
use super::errors::ErrorCode;
use super::ids::UnixMillis;
use super::ids::{WireItemId, WireVaultId};
use super::macros::wire_enum;
use super::rotation::{RotationKind, RotationPhaseName};
use super::ssh::{SshApprovalRequest, SshCloseReason, SshSignFailReason};

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
    /// Sync status for the status bar.
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
        /// The server ended this device's session; sign in again.
        SignedOut,
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
    /// A sign request failed after it was approved.
    SshSignFailed {
        /// The request.
        request_id: String,
        /// The key's vault.
        vault_id: WireVaultId,
        /// The key's item.
        item_id: WireItemId,
        /// Why.
        reason: SshSignFailReason,
    },
    /// The global shortcut was pressed while unlocked: open the palette.
    QuickSearchRequested,
    /// A hardware-protected vault closed by itself after its five minutes
    /// (a manual close and a lock do not send it).
    HardwareVaultClosed {
        /// The vault.
        vault_id: WireVaultId,
    },
    /// The settings file did not carry a valid integrity check (it was
    /// edited outside Mosaic, or is from an older version) and weaker
    /// protection settings in it were reset to safe defaults (ADR-0030).
    SettingsReset,
    /// A browser asks to be paired. The UI shows the code and waits for the
    /// user (`browser_pairing_respond`).
    BrowserPairingRequested {
        /// The request.
        request: BrowserPairingRequest,
    },
    /// A pairing request is no longer open.
    BrowserPairingClosed {
        /// The request.
        request_id: String,
        /// Why.
        reason: BrowserCloseReason,
    },
    /// The extension offers to save a login. The UI shows what would be
    /// written (`browser_save_respond`); the password stays in Rust.
    BrowserSaveRequested {
        /// The request.
        request: BrowserSaveRequest,
    },
    /// A save request is no longer open.
    BrowserSaveClosed {
        /// The request.
        request_id: String,
        /// Why.
        reason: BrowserCloseReason,
    },
    /// The extension asks to create or use a passkey. The UI shows the
    /// request and waits for the user (`browser_passkey_respond`).
    BrowserPasskeyRequested {
        /// The request.
        request: BrowserPasskeyRequest,
    },
    /// A passkey request is no longer open.
    BrowserPasskeyClosed {
        /// The request.
        request_id: String,
        /// Why.
        reason: BrowserCloseReason,
    },
    /// A browser was paired or removed, or connected or disconnected: read
    /// `browser_status` again.
    BrowserPairingsChanged,
    /// A sync run changed these vaults: read their items again.
    SyncDataChanged {
        /// The vaults.
        vault_ids: Vec<WireVaultId>,
    },
    /// The invites waiting for this account or the emergency requests waiting
    /// for its answer changed (after a sync run): read `invites_list` or
    /// `emergency_overview` again.
    SharingChanged {
        /// Invites waiting for this account.
        invites: u32,
        /// Emergency requests waiting for this account's answer.
        emergency_requests: u32,
    },
    /// A rotation job (Secret Key, key derivation, re-key) moved on: read
    /// `rotation_status` again (M7).
    RotationProgress {
        /// What is renewed.
        kind: RotationKind,
        /// Where it stands.
        phase: RotationPhaseName,
        /// Steps done.
        done: u32,
        /// Steps in all.
        total: u32,
    },
    /// A sync run merged or repaired something the user may want to look at.
    SyncAttention {
        /// Records merged with a local change.
        conflicts: u32,
        /// Fields written back after a delete raced an edit.
        restored: u32,
        /// Unreadable local records replaced by the received version.
        replaced_unreadable: u32,
    },
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::browser::samples as browser;
    use crate::contract::ids::samples as ids;
    use crate::contract::ssh::samples as ssh;

    #[expect(clippy::too_many_lines, reason = "one sample per event")]
    pub(crate) fn events() -> Vec<AppEvent> {
        let mut all = vec![
            AppEvent::Locked {
                reason: LockReason::Idle,
            },
            AppEvent::ClipboardCleared,
            AppEvent::SshApprovalRequested {
                request: ssh::approval(),
            },
            AppEvent::SshApprovalClosed {
                request_id: "ssh-1".into(),
                reason: SshCloseReason::Expired,
            },
            AppEvent::SshSignFailed {
                request_id: "ssh-1".into(),
                vault_id: ids::vault(1),
                item_id: ids::item(3),
                reason: SshSignFailReason::WrongPassphrase,
            },
            AppEvent::QuickSearchRequested,
            AppEvent::HardwareVaultClosed {
                vault_id: ids::vault(3),
            },
            AppEvent::SettingsReset,
            AppEvent::BrowserPairingRequested {
                request: browser::pairing_request(),
            },
            AppEvent::BrowserPairingClosed {
                request_id: browser::request_id(),
                reason: BrowserCloseReason::Answered,
            },
            AppEvent::BrowserSaveRequested {
                request: browser::save_request(),
            },
            AppEvent::BrowserSaveRequested {
                request: browser::save_request_new(),
            },
            AppEvent::BrowserSaveClosed {
                request_id: browser::request_id(),
                reason: BrowserCloseReason::Locked,
            },
            AppEvent::BrowserPasskeyRequested {
                request: browser::passkey_get_request(),
            },
            AppEvent::BrowserPasskeyRequested {
                request: browser::passkey_create_request(),
            },
            AppEvent::BrowserPasskeyRequested {
                request: browser::passkey_excluded_request(),
            },
            AppEvent::BrowserPasskeyClosed {
                request_id: browser::request_id(),
                reason: BrowserCloseReason::AbortedByPage,
            },
            AppEvent::BrowserPairingsChanged,
            AppEvent::SharingChanged {
                invites: 1,
                emergency_requests: 0,
            },
            AppEvent::SharingChanged {
                invites: 0,
                emergency_requests: 2,
            },
            AppEvent::RotationProgress {
                kind: RotationKind::Rekey,
                phase: RotationPhaseName::Attachments,
                done: 3,
                total: 12,
            },
        ];
        all.extend(crate::contract::sync::samples::events());
        for event in &all {
            // Adding a variant fails to compile here until it has a sample.
            match event {
                AppEvent::Locked { .. }
                | AppEvent::SyncStateChanged { .. }
                | AppEvent::ClipboardCleared
                | AppEvent::SshApprovalRequested { .. }
                | AppEvent::SshApprovalClosed { .. }
                | AppEvent::SshSignFailed { .. }
                | AppEvent::QuickSearchRequested
                | AppEvent::HardwareVaultClosed { .. }
                | AppEvent::SettingsReset
                | AppEvent::BrowserPairingRequested { .. }
                | AppEvent::BrowserPairingClosed { .. }
                | AppEvent::BrowserSaveRequested { .. }
                | AppEvent::BrowserSaveClosed { .. }
                | AppEvent::BrowserPasskeyRequested { .. }
                | AppEvent::BrowserPasskeyClosed { .. }
                | AppEvent::BrowserPairingsChanged
                | AppEvent::SharingChanged { .. }
                | AppEvent::RotationProgress { .. }
                | AppEvent::SyncDataChanged { .. }
                | AppEvent::SyncAttention { .. } => {}
            }
        }
        all
    }
}
