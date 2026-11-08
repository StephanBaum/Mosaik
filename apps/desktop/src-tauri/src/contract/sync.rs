//! Sync (M6, `docs/DESKTOP_IPC.md` §3 "Sync"): the settings group, the join
//! of a new device and the status the status bar shows. The webview never
//! sends a folder path (ADR-0034): `sync_pick_folder` returns a `choiceId`,
//! and the connect and join commands take it back. Passwords, the Secret Key,
//! second-factor codes and the invite code travel as [`SecretIn`]; the second
//! factor's setup secret leaves as [`SecretOut`].

use serde::{Deserialize, Serialize};

use super::events::SyncState;
use super::ids::{UnixMillis, WireAccountId, WireInstallId};
use super::macros::wire_enum;
use super::secret::{SecretIn, SecretOut};
use super::sharing::SharingFindingView;

wire_enum!(
    /// Who may create an account on a server (`GET /v1/info`).
    pub enum RegistrationMode {
        /// Anyone.
        Open,
        /// Only with an invite code.
        Invite,
        /// Nobody.
        Closed,
    }
);

wire_enum!(
    /// What `sync_hosted_connect` does on the server.
    pub enum HostedMode {
        /// Creates the sync account for this device's account.
        Register,
        /// Signs in to an existing sync account of this account.
        SignIn,
    }
);

wire_enum!(
    /// What a chosen folder holds.
    pub enum FolderState {
        /// Nothing (Mosaic can use it).
        Empty,
        /// A Mosaic sync folder (it may hold accounts to join).
        Mosaic,
        /// Other files: Mosaic refuses to write there.
        Other,
    }
);

wire_enum!(
    /// A device's operating system.
    pub enum DevicePlatform {
        /// Windows.
        Windows,
        /// macOS.
        Macos,
        /// Linux.
        Linux,
    }
);

/// The configured backend, without secrets.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SyncConfigView {
    /// The Mosaic server.
    Hosted {
        /// The server's canonical origin.
        server_url: String,
        /// The sync account's email.
        email: String,
        /// This device's name.
        device_name: String,
        /// Whether the sync account has a second factor.
        totp_enabled: bool,
    },
    /// A folder.
    Folder {
        /// The folder's path, for display only.
        display_path: String,
        /// This device's name.
        device_name: String,
    },
}

/// Something the user should know about, with what to do in the UI text.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SyncWarningView {
    /// Another device's changes have not arrived for longer than the grace
    /// (hosted).
    FreezeSuspected {
        /// The device's name (`null` if unknown).
        device_name: Option<String>,
        /// Since when.
        since: UnixMillis,
    },
    /// Files of another device are missing from the folder.
    MissingFiles {
        /// The device's name (`null` if unknown).
        device_name: Option<String>,
        /// Since when.
        since: UnixMillis,
    },
    /// This device and another have not seen the same server data for two
    /// days.
    NoCommonView {
        /// The other device's name (`null` if unknown).
        device_name: Option<String>,
        /// Since when.
        since: UnixMillis,
    },
    /// The backend's data is older than this device's own checkpoint.
    RollbackDetected,
    /// Another install writes under this device's identity.
    IdentityDuplicated,
    /// The server showed this device different data than another device.
    ServerInconsistent {
        /// The vault's name (`null` if unknown).
        vault_name: Option<String>,
    },
    /// The server's copy of a vault is older than this device's.
    ServerRollback {
        /// The vault's name (`null` if unknown).
        vault_name: Option<String>,
    },
    /// Some received items could not be decrypted.
    Undecryptable {
        /// How many.
        count: u32,
    },
    /// Hardware-protected vaults stay on this device.
    HardwareVaultsLocal {
        /// How many.
        count: u32,
    },
    /// Attachment files on the backend are damaged and could not be
    /// replaced from this device.
    AttachmentsNotRepaired {
        /// How many.
        count: u32,
    },
    /// A shared vault's key could not be changed after a removal (this device
    /// owns the vault): the removed person still holds the old key.
    RotationBlocked {
        /// The vault's name (`null` if unknown).
        vault_name: Option<String>,
    },
}

/// The `sync_status` response, and what most sync commands return. (The spec
/// calls it `SyncStatus`; that name is the status enum of [`SyncState`].)
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOverview {
    /// The backend (`null` if sync is off).
    pub config: Option<SyncConfigView>,
    /// The state of the last run.
    pub state: SyncState,
    /// What the user should know.
    pub warnings: Vec<SyncWarningView>,
    /// What sharing wants the user to look at (`sync_acknowledge` clears the
    /// ones that are findings, not states).
    pub sharing_findings: Vec<SharingFindingView>,
    /// The server (or the folder) holds a master password this device does
    /// not know: `sync_recover_password` brings it in (CRYPTO_SPEC §9.4).
    pub password_changed: bool,
}

/// `sync_check_server`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckServerRequest {
    /// The server, `https://host[:port]` (or `http` on this computer).
    pub server_url: String,
}

/// The `sync_check_server` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCheck {
    /// The server's canonical origin.
    pub server_url: String,
    /// Who may register.
    pub registration: RegistrationMode,
}

/// `sync_hosted_connect`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostedConnectRequest {
    /// The server.
    pub server_url: String,
    /// Create the sync account or sign in.
    pub mode: HostedMode,
    /// The master password.
    pub password: SecretIn,
    /// The invite code, if the server needs one.
    pub invite_code: Option<SecretIn>,
    /// This device's name.
    pub device_name: String,
}

/// A request that carries one code (`sync_hosted_two_factor`,
/// `sync_totp_confirm`, `join_hosted_two_factor`).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeRequest {
    /// The six digits from the authenticator app.
    pub code: SecretIn,
}

/// The `sync_pick_folder` response (`null` when the user cancels).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderChoice {
    /// Names this choice in the following commands; it expires after five
    /// minutes.
    pub choice_id: String,
    /// The folder's path, for display.
    pub display_path: String,
    /// What it holds.
    pub contents: FolderState,
    /// The Mosaic accounts in it (to join one).
    pub accounts: Vec<WireAccountId>,
}

/// `sync_folder_connect`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FolderConnectRequest {
    /// From `sync_pick_folder`.
    pub choice_id: String,
    /// This device's name.
    pub device_name: String,
}

/// A device of the sync account.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevice {
    /// The device's install ID.
    pub device_id: WireInstallId,
    /// Its name (`null` if the backend has none).
    pub name: Option<String>,
    /// Its operating system (known for this device only).
    pub platform: Option<DevicePlatform>,
    /// Whether this is the device the app runs on.
    pub current: bool,
    /// When it last synced (`null` if unknown).
    pub last_seen_at: Option<UnixMillis>,
    /// Whether the user can remove it (hosted, not this device).
    pub can_remove: bool,
}

/// The `sync_devices` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevices {
    /// The devices.
    pub devices: Vec<SyncDevice>,
}

/// `sync_device_rename`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceRenameRequest {
    /// The device.
    pub device_id: WireInstallId,
    /// The new name.
    pub name: String,
}

/// `sync_device_remove`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceRemoveRequest {
    /// The device.
    pub device_id: WireInstallId,
    /// The master password (a fresh login).
    pub password: SecretIn,
    /// A current second-factor code, when the account has one.
    pub code: Option<SecretIn>,
}

/// `sync_totp_begin`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TotpBeginRequest {
    /// The master password (a fresh login).
    pub password: SecretIn,
}

/// The `sync_totp_begin` response: the fifth secret response (DESKTOP_IPC §6).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpSetupInfo {
    /// The setup key, to type into an authenticator app.
    pub secret: SecretOut,
    /// The `otpauth://` link (for a QR code).
    pub otpauth_uri: SecretOut,
}

/// `sync_totp_disable`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TotpDisableRequest {
    /// The master password (a fresh login).
    pub password: SecretIn,
    /// A current code.
    pub code: SecretIn,
}

/// `join_hosted`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JoinHostedRequest {
    /// The server.
    pub server_url: String,
    /// The sync account's email.
    pub email: String,
    /// The master password.
    pub password: SecretIn,
    /// The Secret Key from the Emergency Kit.
    pub secret_key: SecretIn,
    /// This device's name.
    pub device_name: String,
}

/// `join_folder`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JoinFolderRequest {
    /// From `sync_pick_folder`.
    pub choice_id: String,
    /// The account to join (one of the choice's `accounts`).
    pub account_id: WireAccountId,
    /// The master password.
    pub password: SecretIn,
    /// The Secret Key from the Emergency Kit.
    pub secret_key: SecretIn,
    /// This device's name.
    pub device_name: String,
}

/// What a join did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum JoinOutcome {
    /// Joined and unlocked.
    Unlocked,
    /// The account has a second factor: `join_hosted_two_factor` is next.
    TwoFactorRequired,
}

/// `sync_recover_password`: the master password the server or the folder
/// holds now (changed by an earlier run of this device that did not finish).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoverPasswordRequest {
    /// The new master password.
    pub new_password: SecretIn,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `sync_acknowledge`: no fields. Rust owns the native confirmation and its
/// texts (English or German, by the language setting), so the webview cannot
/// skip, reword or answer it.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields)]
pub struct AcknowledgeRequest {}

/// `change_master_password`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangePasswordRequest {
    /// The current master password.
    pub current_password: SecretIn,
    /// The new master password.
    pub new_password: SecretIn,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::errors::ErrorCode;
    use crate::contract::events::SyncStatus;
    use crate::contract::ids::samples as ids;

    const T: UnixMillis = 1_761_480_000_000;

    fn state(status: SyncStatus) -> SyncState {
        SyncState {
            status,
            last_synced_at: Some(T),
            error: None,
        }
    }

    pub(crate) fn warnings() -> Vec<SyncWarningView> {
        let all = vec![
            SyncWarningView::FreezeSuspected {
                device_name: Some("Mosaic on macOS".into()),
                since: T,
            },
            SyncWarningView::MissingFiles {
                device_name: None,
                since: T,
            },
            SyncWarningView::NoCommonView {
                device_name: Some("Mosaic on Linux".into()),
                since: T,
            },
            SyncWarningView::RollbackDetected,
            SyncWarningView::IdentityDuplicated,
            SyncWarningView::ServerInconsistent {
                vault_name: Some("Personal".into()),
            },
            SyncWarningView::ServerRollback { vault_name: None },
            SyncWarningView::Undecryptable { count: 3 },
            SyncWarningView::HardwareVaultsLocal { count: 1 },
            SyncWarningView::AttachmentsNotRepaired { count: 2 },
            SyncWarningView::RotationBlocked {
                vault_name: Some("Family".into()),
            },
        ];
        for warning in &all {
            // Adding a variant fails to compile here until it has a sample.
            match warning {
                SyncWarningView::FreezeSuspected { .. }
                | SyncWarningView::MissingFiles { .. }
                | SyncWarningView::NoCommonView { .. }
                | SyncWarningView::RollbackDetected
                | SyncWarningView::IdentityDuplicated
                | SyncWarningView::ServerInconsistent { .. }
                | SyncWarningView::ServerRollback { .. }
                | SyncWarningView::Undecryptable { .. }
                | SyncWarningView::HardwareVaultsLocal { .. }
                | SyncWarningView::AttachmentsNotRepaired { .. }
                | SyncWarningView::RotationBlocked { .. } => {}
            }
        }
        all
    }

    pub(crate) fn overviews() -> Vec<SyncOverview> {
        vec![
            SyncOverview {
                config: None,
                state: SyncState {
                    status: SyncStatus::NotConfigured,
                    last_synced_at: None,
                    error: None,
                },
                warnings: vec![],
                sharing_findings: vec![],
                password_changed: false,
            },
            SyncOverview {
                config: Some(SyncConfigView::Hosted {
                    server_url: "https://sync.example.com".into(),
                    email: "alex@example.com".into(),
                    device_name: "Mosaic on Windows".into(),
                    totp_enabled: true,
                }),
                state: state(SyncStatus::UpToDate),
                warnings: vec![],
                sharing_findings: vec![],
                password_changed: false,
            },
            SyncOverview {
                config: Some(SyncConfigView::Folder {
                    display_path: "C:\\Users\\alex\\Dropbox\\Mosaic".into(),
                    device_name: "Mosaic on Windows".into(),
                }),
                state: state(SyncStatus::Syncing),
                warnings: warnings(),
                sharing_findings: crate::contract::sharing::samples::findings(),
                password_changed: false,
            },
            SyncOverview {
                config: Some(SyncConfigView::Hosted {
                    server_url: "https://sync.example.com".into(),
                    email: "alex@example.com".into(),
                    device_name: "Mosaic on Windows".into(),
                    totp_enabled: false,
                }),
                state: SyncState {
                    status: SyncStatus::SignedOut,
                    last_synced_at: Some(T),
                    error: Some(ErrorCode::SyncSignedOut),
                },
                warnings: vec![],
                sharing_findings: vec![],
                password_changed: true,
            },
        ]
    }

    pub(crate) fn checks() -> Vec<ServerCheck> {
        RegistrationMode::ALL
            .iter()
            .map(|&registration| ServerCheck {
                server_url: "https://sync.example.com".into(),
                registration,
            })
            .collect()
    }

    pub(crate) fn choices() -> Vec<Option<FolderChoice>> {
        vec![
            None,
            Some(FolderChoice {
                choice_id: "choice-1".into(),
                display_path: "C:\\Users\\alex\\Dropbox\\Mosaic".into(),
                contents: FolderState::Mosaic,
                accounts: vec![ids::account()],
            }),
            Some(FolderChoice {
                choice_id: "choice-2".into(),
                display_path: "D:\\Sync".into(),
                contents: FolderState::Empty,
                accounts: vec![],
            }),
            Some(FolderChoice {
                choice_id: "choice-3".into(),
                display_path: "D:\\Documents".into(),
                contents: FolderState::Other,
                accounts: vec![],
            }),
        ]
    }

    pub(crate) fn devices() -> Vec<SyncDevices> {
        vec![SyncDevices {
            devices: vec![
                SyncDevice {
                    device_id: ids::install(1),
                    name: Some("Mosaic on Windows".into()),
                    platform: Some(DevicePlatform::Windows),
                    current: true,
                    last_seen_at: Some(T),
                    can_remove: false,
                },
                SyncDevice {
                    device_id: ids::install(2),
                    name: Some("Mosaic on macOS".into()),
                    platform: None,
                    current: false,
                    last_seen_at: Some(T - 3_600_000),
                    can_remove: true,
                },
                SyncDevice {
                    device_id: ids::install(3),
                    name: None,
                    platform: None,
                    current: false,
                    last_seen_at: None,
                    can_remove: false,
                },
            ],
        }]
    }

    pub(crate) fn totp_setups() -> Vec<TotpSetupInfo> {
        vec![TotpSetupInfo {
            secret: SecretOut::test("JBSWY3DPEHPK3PXP"),
            otpauth_uri: SecretOut::test(
                "otpauth://totp/Mosaic%20sync:alex@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Mosaic%20sync",
            ),
        }]
    }

    pub(crate) fn events() -> Vec<crate::contract::events::AppEvent> {
        use crate::contract::events::AppEvent;
        vec![
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
            AppEvent::SyncDataChanged {
                vault_ids: vec![ids::vault(1), ids::vault(2)],
            },
            AppEvent::SyncAttention {
                conflicts: 2,
                restored: 1,
                replaced_unreadable: 0,
            },
        ]
    }

    pub(crate) fn pw() -> SecretIn {
        SecretIn::test("correct horse battery staple")
    }
}
