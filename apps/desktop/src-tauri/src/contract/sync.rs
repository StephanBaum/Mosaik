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
    /// Why the server ended this device's session (CRYPTO_SPEC §9.5, M7-R3).
    /// The server's claim: it only selects the text of the recovery card.
    pub enum RevokedReasonView {
        /// The account was re-keyed on another device.
        Rekey,
        /// The master password was changed on another device.
        PasswordChange,
        /// This device was removed from the account.
        DeviceRemoved,
        /// The server operator ended the session.
        Operator,
    }
);

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
    /// Where a browser handoff stands (M9, DESKTOP_IPC §3).
    pub enum HandoffState {
        /// Waiting for the browser page.
        Waiting,
        /// The security key was used; the step is finished.
        Done,
        /// Too many wrong codes or failed checks; start again.
        Failed,
        /// It ran out of time (or none is running).
        Expired,
    }
);

wire_enum!(
    /// Which browser step an account handoff is at.
    pub enum HandoffStep {
        /// Signing in with a security key (a fresh login of an account that has them).
        Login,
        /// Adding a security key.
        Register,
        /// Confirming a second factor so keys can be removed.
        Confirm,
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
        /// Whether the sync account has an authenticator-app second factor.
        totp_enabled: bool,
        /// How many security keys the sync account has (M9).
        webauthn_keys: u32,
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
    /// An account file in the sync folder cannot be verified with this
    /// device's keys and names this device's account-key epoch or a higher
    /// one (M7). It may be another device's re-key made at the same time
    /// (`conflict`), or a stray file. Dismissing it never halts anything.
    UnverifiedAccountFile {
        /// The file's name (for `sync_dismiss_account_file`).
        name: String,
        /// The file's SHA-256, 64 hex digits (for `sync_dismiss_account_file`).
        sha256: String,
        /// Whether it names this device's own epoch: two devices renewed the
        /// account's keys at the same time.
        conflict: bool,
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
    /// A verified re-key marker in the folder says the account's keys were
    /// renewed on another device: `sync_recover_password` with the new Secret
    /// Key adopts them (sync stays paused until then).
    pub rekey_pending: bool,
    /// Why the server ended this device's session (`null` unless `state.status`
    /// is `signedOut` after a `401` with a revocation reason).
    pub revoked: Option<RevokedReasonView>,
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

/// The `sync_login_methods` response: the second factors the waiting login can use.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginMethods {
    /// An authenticator-app code.
    pub totp: bool,
    /// A security key, through the browser (the server can run it and the account has one).
    pub webauthn: bool,
}

/// What `sync_hosted_webauthn_begin` and `join_hosted_webauthn_begin` return. The code is shown
/// to the user, who types it on the server's page; it authorizes nothing by itself (it is not a
/// `SecretOut`, DESKTOP_IPC §3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnBegin {
    /// Six digits.
    pub match_code: String,
    /// When the handoff runs out (ms since the Unix epoch).
    pub expires_at: UnixMillis,
}

/// The `sync_webauthn_status` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnStatus {
    /// Where the handoff stands.
    pub state: HandoffState,
}

/// What `sync_webauthn_add_begin` and `sync_webauthn_confirm_begin` return.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnAccountBegin {
    /// The browser step that starts: `login` first when the fresh login needs a security key.
    pub step: HandoffStep,
    /// Six digits.
    pub match_code: String,
    /// When the handoff runs out.
    pub expires_at: UnixMillis,
}

/// The `sync_webauthn_add_status` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnAccountStatus {
    /// Where the current step stands.
    pub state: HandoffState,
    /// The current step (after a finished login step: the next one).
    pub step: HandoffStep,
    /// The new step's code, when a new handoff started.
    pub match_code: Option<String>,
    /// The new step's expiry.
    pub expires_at: Option<UnixMillis>,
}

/// One security key of the sync account.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnKeyView {
    /// The key's reference, 32 hex digits (for `sync_webauthn_remove`).
    pub credential_ref: String,
    /// What the user called it.
    pub label: String,
    /// When it was added.
    pub created_at: UnixMillis,
    /// When it was last used.
    pub last_used_at: Option<UnixMillis>,
}

/// The `sync_webauthn_list` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebauthnKeys {
    /// Whether the account also has an authenticator-app second factor.
    pub totp: bool,
    /// The keys, oldest first.
    pub keys: Vec<WebauthnKeyView>,
}

/// `sync_webauthn_add_begin`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebauthnAddRequest {
    /// The master password (a fresh login).
    pub password: SecretIn,
    /// A current authenticator-app code, when the account has one (else `null`).
    pub code: Option<SecretIn>,
    /// The key's name, 1 to 64 characters.
    pub label: String,
}

/// `sync_webauthn_confirm_begin`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebauthnConfirmRequest {
    /// The master password (a fresh login).
    pub password: SecretIn,
}

/// `sync_webauthn_remove`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebauthnRemoveRequest {
    /// The key's reference from `sync_webauthn_list`.
    pub credential_ref: String,
    /// A current authenticator-app code, or `null` after a security-key confirm.
    pub code: Option<SecretIn>,
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
    /// The Secret Key from the newest Emergency Kit, when the account's keys
    /// were renewed on another device (a Secret Key rotation or a re-key);
    /// `null` to use this device's own.
    pub secret_key: Option<SecretIn>,
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
            SyncWarningView::UnverifiedAccountFile {
                name: "account-00000002.macct".into(),
                sha256: "ab".repeat(32),
                conflict: false,
            },
            SyncWarningView::UnverifiedAccountFile {
                name: "account-00000001.macct".into(),
                sha256: "cd".repeat(32),
                conflict: true,
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
                | SyncWarningView::RotationBlocked { .. }
                | SyncWarningView::UnverifiedAccountFile { .. } => {}
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
                rekey_pending: false,
                revoked: None,
            },
            SyncOverview {
                config: Some(SyncConfigView::Hosted {
                    server_url: "https://sync.example.com".into(),
                    email: "alex@example.com".into(),
                    device_name: "Mosaic on Windows".into(),
                    totp_enabled: true,
                    webauthn_keys: 2,
                }),
                state: state(SyncStatus::UpToDate),
                warnings: vec![],
                sharing_findings: vec![],
                password_changed: false,
                rekey_pending: false,
                revoked: None,
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
                rekey_pending: false,
                revoked: None,
            },
            SyncOverview {
                config: Some(SyncConfigView::Hosted {
                    server_url: "https://sync.example.com".into(),
                    email: "alex@example.com".into(),
                    device_name: "Mosaic on Windows".into(),
                    totp_enabled: false,
                    webauthn_keys: 0,
                }),
                state: SyncState {
                    status: SyncStatus::SignedOut,
                    last_synced_at: Some(T),
                    error: Some(ErrorCode::SyncSignedOut),
                },
                warnings: vec![],
                sharing_findings: vec![],
                password_changed: true,
                rekey_pending: true,
                revoked: Some(RevokedReasonView::Rekey),
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

    pub(crate) fn login_methods() -> Vec<LoginMethods> {
        vec![
            LoginMethods {
                totp: true,
                webauthn: false,
            },
            LoginMethods {
                totp: true,
                webauthn: true,
            },
            LoginMethods {
                totp: false,
                webauthn: true,
            },
        ]
    }

    pub(crate) fn webauthn_begins() -> Vec<WebauthnBegin> {
        vec![WebauthnBegin {
            match_code: "482913".into(),
            expires_at: T + 300_000,
        }]
    }

    pub(crate) fn webauthn_statuses() -> Vec<WebauthnStatus> {
        HandoffState::ALL
            .iter()
            .map(|&state| WebauthnStatus { state })
            .collect()
    }

    pub(crate) fn webauthn_account_begins() -> Vec<WebauthnAccountBegin> {
        [
            HandoffStep::Login,
            HandoffStep::Register,
            HandoffStep::Confirm,
        ]
        .into_iter()
        .map(|step| WebauthnAccountBegin {
            step,
            match_code: "482913".into(),
            expires_at: T + 300_000,
        })
        .collect()
    }

    pub(crate) fn webauthn_account_statuses() -> Vec<WebauthnAccountStatus> {
        vec![
            WebauthnAccountStatus {
                state: HandoffState::Waiting,
                step: HandoffStep::Register,
                match_code: Some("482913".into()),
                expires_at: Some(T + 300_000),
            },
            WebauthnAccountStatus {
                state: HandoffState::Done,
                step: HandoffStep::Confirm,
                match_code: None,
                expires_at: None,
            },
        ]
    }

    pub(crate) fn webauthn_keys() -> Vec<WebauthnKeys> {
        vec![
            WebauthnKeys {
                totp: false,
                keys: vec![],
            },
            WebauthnKeys {
                totp: true,
                keys: vec![
                    WebauthnKeyView {
                        credential_ref: "00112233445566778899aabbccddeeff".into(),
                        label: "YubiKey 5C".into(),
                        created_at: T,
                        last_used_at: Some(T + 60_000),
                    },
                    WebauthnKeyView {
                        credential_ref: "ffeeddccbbaa99887766554433221100".into(),
                        label: "Backup key".into(),
                        created_at: T + 1_000,
                        last_used_at: None,
                    },
                ],
            },
        ]
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
