//! Contract fixtures (ADR-0018): samples of every request, response, event,
//! error and string enum, written to `apps/desktop/src/ipc/contract.fixtures.ts`.
//!
//! `tsc` checks that file against the TypeScript contract with `satisfies`,
//! so a field, variant or command that differs between the two sides fails
//! `pnpm typecheck`, and the TypeScript parity test compares the command
//! list and every string enum in both directions. The test here fails when
//! the file is stale; regenerate it with
//! `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::attachments::{self, AttachmentAdded, AttachmentSaved};
use super::browser::{
    self, BrowserCloseReason, BrowserEndpointState, BrowserFamily, BrowserHint,
    BrowserPairingRespondRequest, BrowserPasskeyDecision, BrowserPasskeyKind,
    BrowserPasskeyRespondRequest, BrowserRegistryState, BrowserRemovePairingRequest,
    BrowserSaveDecision, BrowserSaveKind, BrowserSaveRespondRequest, PairingDecision,
    PasskeyVerification,
};
use super::clipboard;
use super::errors::ErrorCode;
use super::events::{self, LockReason, SyncStatus};
use super::export;
use super::generator::{self, Strength};
use super::ids::samples as ids;
use super::import::{
    self, ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportEntryWarning,
    ImportFormat, ImportPreviewRequest, ImportResult, ImportSkipReason, ImportWarning,
};
use super::insights::{self, InsightAction, InsightKind, InsightSeverity};
use super::items::{
    self, AddItemRequest, CustomFieldKind, ItemKind, ItemRequest, ListItemsRequest,
    MoveItemRequest, MovedItem, OtpScheme, PasskeyAlgorithm, RevealRequest, RevealedSecret,
    SecretFieldKey, SetFavoriteRequest, TextFieldKey, TrashFilter, UpdateItemRequest, UrlMatchMode,
};
use super::otp::{self, OtpAlgorithm, OtpScanSource, OtpSecretStrength};
use super::quick_unlock::{self, BiometricPlatformKind};
use super::secret::{SecretIn, SecretOut};
use super::session::{
    self, KitVerification, QuickUnlockKind, QuickUnlockRequest, ShortcutState, VerifyKitRequest,
};
use super::settings::{self, Language, Theme, UpdateSettingsRequest};
use super::ssh::{
    self, SshCloseReason, SshDecision, SshRespondRequest, SshSetEnabledRequest, SshSignFailReason,
    SshSkipReason,
};
use super::sync::{
    self, ChangePasswordRequest, CheckServerRequest, CodeRequest, DevicePlatform,
    DeviceRemoveRequest, DeviceRenameRequest, FolderConnectRequest, FolderState,
    HostedConnectRequest, HostedMode, JoinFolderRequest, JoinHostedRequest, JoinOutcome,
    RegistrationMode, TotpBeginRequest, TotpDisableRequest,
};
use super::totp;
use super::vaults::{self, CreateVaultRequest, RenameVaultRequest};
use crate::commands::names::COMMANDS;

/// Commands whose argument is not a JSON request (a `Channel`).
const CHANNEL_COMMANDS: &[&str] = &["subscribe_events"];

/// Commands whose response is binary (an `ArrayBuffer` in the webview), so it
/// has no JSON sample. `attachment_preview` returns
/// `[width u32 LE][height u32 LE][width * height * 4 bytes of straight RGBA]`
/// (at most 2048 x 2048 and 4194304 pixels), as one `tauri::ipc::Response`.
const BINARY_RESPONSES: &[&str] = &["attachment_preview"];

struct Fixtures {
    requests: Map<String, Value>,
    responses: Map<String, Value>,
}

impl Fixtures {
    /// Adds a command's request and response samples. Each request sample
    /// must deserialize back into the request type, as the command will.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the samples are consumed by value at every call site"
    )]
    fn command<Req, Res>(&mut self, name: &str, requests: Vec<Req>, responses: Vec<Res>)
    where
        Req: Serialize + DeserializeOwned,
        Res: Serialize,
    {
        let requests: Vec<Value> = requests
            .iter()
            .map(|request| {
                let value = serde_json::to_value(request).unwrap();
                serde_json::from_value::<Req>(value.clone())
                    .unwrap_or_else(|e| panic!("{name}: request does not round-trip: {e}"));
                value
            })
            .collect();
        assert!(!requests.is_empty(), "{name}: no request sample");
        assert!(!responses.is_empty(), "{name}: no response sample");
        let previous = self
            .requests
            .insert(name.to_owned(), Value::Array(requests));
        assert!(previous.is_none(), "{name} listed twice");
        self.responses
            .insert(name.to_owned(), serde_json::to_value(responses).unwrap());
    }
}

impl Fixtures {
    /// Adds a command with a binary response: request samples only.
    fn binary_command<Req>(&mut self, name: &str, requests: &[Req])
    where
        Req: Serialize + DeserializeOwned,
    {
        assert!(BINARY_RESPONSES.contains(&name), "{name} is not binary");
        let requests: Vec<Value> = requests
            .iter()
            .map(|request| {
                let value = serde_json::to_value(request).unwrap();
                serde_json::from_value::<Req>(value.clone())
                    .unwrap_or_else(|e| panic!("{name}: request does not round-trip: {e}"));
                value
            })
            .collect();
        let previous = self
            .requests
            .insert(name.to_owned(), Value::Array(requests));
        assert!(previous.is_none(), "{name} listed twice");
    }
}

fn values<T: Serialize>(all: &[T]) -> Value {
    serde_json::to_value(all).unwrap()
}

/// The session commands.
fn session_commands(f: &mut Fixtures) {
    f.command("session_state", vec![()], session::samples::states());
    f.command(
        "create_account",
        vec![session::samples::create_account_request()],
        vec![session::samples::created_account()],
    );
    f.command(
        "save_emergency_kit",
        vec![
            session::samples::save_kit_request(),
            session::SaveKitRequest {
                password: Some(SecretIn::test("correct horse battery staple")),
                ..session::samples::save_kit_request()
            },
        ],
        vec![session::KitSaved::Saved, session::KitSaved::Cancelled],
    );
    f.command(
        "verify_emergency_kit",
        vec![VerifyKitRequest {
            last_group: SecretIn::test("8VJ2"),
        }],
        vec![
            KitVerification { matches: true },
            KitVerification { matches: false },
        ],
    );
    f.command(
        "unlock",
        vec![
            session::samples::unlock_request(),
            session::UnlockRequest {
                password: SecretIn::test("correct horse battery staple"),
                secret_key: None,
            },
        ],
        session::samples::states(),
    );
    f.command("lock", vec![()], session::samples::states());
    f.command(
        "reauthenticate",
        vec![session::ReauthenticateRequest {
            password: SecretIn::test("correct horse battery staple"),
        }],
        vec![()],
    );
    f.command("app_status", vec![()], vec![session::samples::app_status()]);
}

/// The quick-unlock commands.
fn quick_unlock_commands(f: &mut Fixtures) {
    f.command(
        "unlock_quick",
        QuickUnlockKind::ALL
            .iter()
            .map(|&method| QuickUnlockRequest { method, pin: None })
            .chain([QuickUnlockRequest {
                method: QuickUnlockKind::HardwareKey,
                pin: Some(SecretIn::test("246810")),
            }])
            .collect(),
        session::samples::states(),
    );
    f.command(
        "quick_unlock_status",
        vec![()],
        quick_unlock::samples::statuses(),
    );
    f.command(
        "enroll_biometric",
        quick_unlock::samples::enroll_biometric(),
        quick_unlock::samples::statuses(),
    );
    f.command(
        "remove_biometric",
        vec![()],
        quick_unlock::samples::statuses(),
    );
    f.command(
        "enroll_hardware_key",
        quick_unlock::samples::enroll_hardware_key(),
        quick_unlock::samples::statuses(),
    );
    f.command(
        "remove_hardware_key",
        vec![quick_unlock::RemoveHardwareKeyRequest {
            method_id: "0123456789abcdef0123456789abcdef".into(),
        }],
        quick_unlock::samples::statuses(),
    );
    f.command("hardware_key_cancel", vec![()], vec![()]);
}

/// The vault commands.
fn vault_commands(f: &mut Fixtures) {
    f.command("list_vaults", vec![()], vec![vaults::samples::summaries()]);
    f.command(
        "create_vault",
        vec![CreateVaultRequest {
            name: "Work".into(),
        }],
        vaults::samples::summaries(),
    );
    f.command(
        "rename_vault",
        vec![RenameVaultRequest {
            vault_id: ids::vault(1),
            name: "Family".into(),
        }],
        vaults::samples::summaries(),
    );

    f.command(
        "create_hardware_vault",
        vec![quick_unlock::CreateHardwareVaultRequest {
            name: "Authenticators".into(),
            pin: Some(SecretIn::test("246810")),
        }],
        vaults::samples::summaries(),
    );
    f.command(
        "open_hardware_vault",
        vec![quick_unlock::samples::vault_request()],
        vaults::samples::summaries(),
    );
    f.command(
        "close_hardware_vault",
        vec![vaults::VaultRequest {
            vault_id: ids::vault(3),
        }],
        vec![()],
    );
    f.command(
        "hardware_vault_keys",
        vec![vaults::VaultRequest {
            vault_id: ids::vault(3),
        }],
        quick_unlock::samples::key_summaries(),
    );
    f.command(
        "add_hardware_vault_key",
        vec![quick_unlock::samples::vault_request()],
        quick_unlock::samples::key_summaries(),
    );
    f.command(
        "remove_hardware_vault_key",
        vec![quick_unlock::HardwareVaultKeyRequest {
            vault_id: ids::vault(3),
            method_id: "0123456789abcdef0123456789abcdef".into(),
        }],
        quick_unlock::samples::key_summaries(),
    );
}

/// The item commands.
fn item_commands(f: &mut Fixtures, item_request: ItemRequest) {
    f.command(
        "list_items",
        vec![
            ListItemsRequest {
                vault_id: Some(ids::vault(1)),
                trash: TrashFilter::Exclude,
                favorites_only: true,
                query: Some("git".into()),
            },
            ListItemsRequest {
                vault_id: None,
                trash: TrashFilter::Only,
                favorites_only: false,
                query: None,
            },
        ],
        vec![items::samples::summaries()],
    );
    f.command(
        "get_item",
        vec![item_request],
        vec![
            items::samples::detail(),
            items::samples::ssh_detail(),
            items::samples::passkey_detail(),
        ],
    );
    f.command(
        "reveal_secret",
        items::samples::secret_refs()
            .into_iter()
            .map(|secret| RevealRequest {
                vault_id: ids::vault(1),
                item_id: ids::item(1),
                secret,
            })
            .collect(),
        vec![RevealedSecret {
            value: SecretOut::test("hunter2"),
            hide_after_seconds: 60,
        }],
    );
    f.command(
        "add_item",
        vec![AddItemRequest {
            vault_id: ids::vault(1),
            kind: ItemKind::Login,
            draft: items::samples::draft(),
        }],
        vec![items::samples::detail()],
    );
    f.command(
        "update_item",
        items::samples::otp_changes()
            .into_iter()
            .map(|otp| UpdateItemRequest {
                vault_id: ids::vault(1),
                item_id: ids::item(1),
                draft: items::ItemDraft {
                    otp: Some(otp),
                    ..items::samples::draft()
                },
            })
            .collect(),
        vec![items::samples::detail()],
    );
    f.command(
        "set_favorite",
        vec![SetFavoriteRequest {
            vault_id: ids::vault(1),
            item_id: ids::item(1),
            favorite: true,
        }],
        vec![items::samples::summary()],
    );
    f.command("trash_item", vec![item_request], vec![()]);
    f.command("restore_item", vec![item_request], vec![()]);
    f.command("purge_item", vec![item_request], vec![()]);
    f.command(
        "move_item",
        vec![MoveItemRequest {
            vault_id: ids::vault(1),
            item_id: ids::item(1),
            target_vault_id: ids::vault(2),
        }],
        vec![MovedItem {
            vault_id: ids::vault(2),
            item_id: ids::item(9),
        }],
    );
}

/// Clipboard, generator and one-time codes, settings and insights.
/// The attachment commands.
fn attachment_commands(f: &mut Fixtures, item_request: ItemRequest) {
    let request = attachments::samples::request();
    f.command(
        "attachment_add",
        vec![item_request],
        vec![
            AttachmentAdded::Added {
                item: Box::new(items::samples::detail()),
            },
            AttachmentAdded::Cancelled,
        ],
    );
    f.binary_command("attachment_preview", &[request]);
    f.command(
        "attachment_save",
        vec![request],
        vec![AttachmentSaved::Saved, AttachmentSaved::Cancelled],
    );
    f.command(
        "attachment_remove",
        vec![request],
        vec![items::samples::detail()],
    );
}

fn tool_commands(f: &mut Fixtures, item_request: ItemRequest) {
    f.command(
        "copy_to_clipboard",
        clipboard::samples::requests(),
        clipboard::samples::receipts(),
    );
    f.command("clear_clipboard", vec![()], vec![()]);

    f.command(
        "generate_password",
        generator::samples::requests(),
        vec![generator::samples::generated()],
    );
    f.command("totp_code", vec![item_request], totp::samples::codes());
    otp_commands(f);

    f.command("get_settings", vec![()], settings::samples::settings());
    f.command(
        "update_settings",
        settings::samples::settings()
            .into_iter()
            .zip([None, Some(SecretIn::test("correct horse battery staple"))])
            .map(|(settings, password)| UpdateSettingsRequest { settings, password })
            .collect(),
        settings::samples::settings(),
    );

    f.command(
        "security_report",
        vec![()],
        vec![insights::samples::report()],
    );
}

/// The one-time code setup commands (ADR-0031).
fn otp_commands(f: &mut Fixtures) {
    f.command(
        "otp_scan",
        otp::samples::scan_requests(),
        otp::samples::scan_results(),
    );
    f.command(
        "otp_parse",
        otp::samples::parse_requests(),
        otp::samples::previews(),
    );
    f.command(
        "otp_pending_code",
        otp::samples::pending_requests(),
        totp::samples::codes(),
    );
    f.command(
        "otp_pending_discard",
        otp::samples::pending_requests(),
        vec![()],
    );
}

/// The import commands.
fn import_commands(f: &mut Fixtures) {
    f.command(
        "import_choose_file",
        ImportFormat::ALL
            .iter()
            .map(|&format| ImportChooseRequest { format })
            .collect(),
        import::samples::choices(),
    );
    f.command(
        "import_preview",
        vec![
            ImportPreviewRequest {
                file_token: "f-1".into(),
                format: ImportFormat::Keepass,
                password: Some(SecretIn::test("kdbx password")),
            },
            ImportPreviewRequest {
                file_token: "f-1".into(),
                format: ImportFormat::Enpass,
                password: None,
            },
        ],
        vec![import::samples::preview()],
    );
    f.command(
        "import_commit",
        vec![ImportCommitRequest {
            preview_id: "p-1".into(),
            target_vault_id: ids::vault(1),
            skip: vec![0],
        }],
        vec![ImportResult {
            imported: 1,
            skipped: 1,
            failed: 0,
            attachments: 1,
            attachments_failed: 0,
        }],
    );
    f.command(
        "import_cancel",
        vec![ImportCancelRequest {
            preview_id: "p-1".into(),
        }],
        vec![()],
    );
}

/// The export commands.
fn export_commands(f: &mut Fixtures) {
    f.command(
        "export_passkeys_cxf",
        export::samples::requests(),
        export::samples::results(),
    );
}

/// The SSH agent commands.
fn ssh_commands(f: &mut Fixtures) {
    f.command("ssh_agent_status", vec![()], ssh::samples::statuses());
    f.command(
        "ssh_agent_set_enabled",
        vec![SshSetEnabledRequest { enabled: true }],
        ssh::samples::statuses(),
    );
    f.command(
        "ssh_respond",
        SshDecision::ALL
            .iter()
            .map(|&decision| SshRespondRequest {
                request_id: "ssh-1".into(),
                decision,
            })
            .collect(),
        vec![()],
    );
}

/// The browser extension commands.
fn browser_commands(f: &mut Fixtures) {
    f.command("browser_status", vec![()], browser::samples::statuses());
    f.command(
        "browser_remove_pairing",
        vec![BrowserRemovePairingRequest {
            pairing_id: browser::samples::pairing_id(),
        }],
        browser::samples::statuses(),
    );
    f.command(
        "browser_pairing_respond",
        vec![
            BrowserPairingRespondRequest {
                request_id: browser::samples::request_id(),
                decision: PairingDecision::Allow,
                label: Some("Work laptop".into()),
            },
            BrowserPairingRespondRequest {
                request_id: browser::samples::request_id(),
                decision: PairingDecision::Deny,
                label: None,
            },
        ],
        vec![()],
    );
    f.command(
        "browser_save_respond",
        vec![
            BrowserSaveRespondRequest {
                request_id: browser::samples::request_id(),
                decision: BrowserSaveDecision::SaveNew {
                    vault_id: ids::vault(1),
                    title: "Example".into(),
                    allow_http: false,
                },
            },
            BrowserSaveRespondRequest {
                request_id: browser::samples::request_id(),
                decision: BrowserSaveDecision::Update {
                    vault_id: ids::vault(1),
                    item_id: ids::item(2),
                },
            },
            BrowserSaveRespondRequest {
                request_id: browser::samples::request_id(),
                decision: BrowserSaveDecision::Cancel,
            },
        ],
        vec![()],
    );
    let passkey = |decision, password: Option<&str>| BrowserPasskeyRespondRequest {
        request_id: browser::samples::request_id(),
        decision,
        password: password.map(SecretIn::test),
    };
    f.command(
        "browser_passkey_respond",
        vec![
            passkey(
                BrowserPasskeyDecision::Create {
                    vault_id: ids::vault(1),
                },
                None,
            ),
            passkey(
                BrowserPasskeyDecision::Assert {
                    vault_id: ids::vault(1),
                    item_id: ids::item(5),
                },
                Some("correct horse battery staple"),
            ),
            passkey(BrowserPasskeyDecision::UseAnotherDevice, None),
            passkey(BrowserPasskeyDecision::Cancel, None),
            passkey(BrowserPasskeyDecision::Acknowledge, None),
        ],
        vec![()],
    );
}

/// The sync commands.
fn sync_commands(f: &mut Fixtures) {
    let url = || "https://sync.example.com".to_owned();
    let device = || "Mosaic on Windows".to_owned();
    f.command("sync_status", vec![()], sync::samples::overviews());
    f.command(
        "sync_check_server",
        vec![CheckServerRequest { server_url: url() }],
        sync::samples::checks(),
    );
    f.command(
        "sync_hosted_connect",
        HostedMode::ALL
            .iter()
            .map(|&mode| HostedConnectRequest {
                server_url: url(),
                mode,
                password: sync::samples::pw(),
                invite_code: (mode == HostedMode::Register)
                    .then(|| SecretIn::test("INVITE-CODE-1234")),
                device_name: device(),
            })
            .collect(),
        sync::samples::overviews(),
    );
    f.command(
        "sync_hosted_two_factor",
        vec![CodeRequest {
            code: SecretIn::test("492039"),
        }],
        sync::samples::overviews(),
    );
    f.command("sync_pick_folder", vec![()], sync::samples::choices());
    f.command(
        "sync_folder_connect",
        vec![FolderConnectRequest {
            choice_id: "choice-1".into(),
            device_name: device(),
        }],
        sync::samples::overviews(),
    );
    f.command("sync_now", vec![()], sync::samples::overviews());
    f.command("sync_disconnect", vec![()], sync::samples::overviews());
    f.command("sync_acknowledge", vec![()], sync::samples::overviews());
    f.command("sync_devices", vec![()], sync::samples::devices());
    f.command(
        "sync_device_rename",
        vec![DeviceRenameRequest {
            device_id: ids::install(2),
            name: "Work laptop".into(),
        }],
        vec![()],
    );
    f.command(
        "sync_device_remove",
        vec![
            DeviceRemoveRequest {
                device_id: ids::install(2),
                password: sync::samples::pw(),
                code: None,
            },
            DeviceRemoveRequest {
                device_id: ids::install(2),
                password: sync::samples::pw(),
                code: Some(SecretIn::test("492039")),
            },
        ],
        vec![()],
    );
    f.command(
        "sync_totp_begin",
        vec![TotpBeginRequest {
            password: sync::samples::pw(),
        }],
        sync::samples::totp_setups(),
    );
    f.command(
        "sync_totp_confirm",
        vec![CodeRequest {
            code: SecretIn::test("492039"),
        }],
        vec![()],
    );
    f.command(
        "sync_totp_disable",
        vec![TotpDisableRequest {
            password: sync::samples::pw(),
            code: SecretIn::test("492039"),
        }],
        vec![()],
    );
    sync_join_commands(f);
}

/// The join commands and the master password change.
fn sync_join_commands(f: &mut Fixtures) {
    let url = || "https://sync.example.com".to_owned();
    let device = || "Mosaic on Windows".to_owned();
    let outcomes = || vec![JoinOutcome::Unlocked, JoinOutcome::TwoFactorRequired];
    f.command(
        "join_hosted",
        vec![JoinHostedRequest {
            server_url: url(),
            email: "alex@example.com".into(),
            password: sync::samples::pw(),
            secret_key: SecretIn::test("M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"),
            device_name: device(),
        }],
        outcomes(),
    );
    f.command(
        "join_hosted_two_factor",
        vec![CodeRequest {
            code: SecretIn::test("492039"),
        }],
        outcomes(),
    );
    f.command(
        "join_folder",
        vec![JoinFolderRequest {
            choice_id: "choice-1".into(),
            account_id: ids::account(),
            password: sync::samples::pw(),
            secret_key: SecretIn::test("M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"),
            device_name: device(),
        }],
        outcomes(),
    );
    f.command(
        "change_master_password",
        vec![
            ChangePasswordRequest {
                current_password: sync::samples::pw(),
                new_password: SecretIn::test("a new master password"),
                code: None,
            },
            ChangePasswordRequest {
                current_password: sync::samples::pw(),
                new_password: SecretIn::test("a new master password"),
                code: Some(SecretIn::test("492039")),
            },
        ],
        vec![()],
    );
}

/// Every string enum of the contract, by name.
fn enums() -> Map<String, Value> {
    let mut enums = Map::new();
    for (name, all) in [
        ("errorCode", values(ErrorCode::ALL)),
        ("quickUnlockKind", values(QuickUnlockKind::ALL)),
        ("vaultProtection", values(vaults::VaultProtection::ALL)),
        ("biometricPlatformKind", values(BiometricPlatformKind::ALL)),
        ("itemKind", values(ItemKind::ALL)),
        ("textFieldKey", values(TextFieldKey::ALL)),
        ("secretFieldKey", values(SecretFieldKey::ALL)),
        ("trashFilter", values(TrashFilter::ALL)),
        ("urlMatchMode", values(UrlMatchMode::ALL)),
        ("customFieldKind", values(CustomFieldKind::ALL)),
        ("otpScheme", values(OtpScheme::ALL)),
        ("otpScanSource", values(OtpScanSource::ALL)),
        ("otpAlgorithm", values(OtpAlgorithm::ALL)),
        ("otpSecretStrength", values(OtpSecretStrength::ALL)),
        ("strength", values(Strength::ALL)),
        ("theme", values(Theme::ALL)),
        ("language", values(Language::ALL)),
        ("insightKind", values(InsightKind::ALL)),
        ("insightSeverity", values(InsightSeverity::ALL)),
        ("insightAction", values(InsightAction::ALL)),
        ("importFormat", values(ImportFormat::ALL)),
        ("importWarning", values(ImportWarning::ALL)),
        ("importEntryWarning", values(ImportEntryWarning::ALL)),
        ("importSkipReason", values(ImportSkipReason::ALL)),
        ("shortcutState", values(ShortcutState::ALL)),
        ("sshSkipReason", values(SshSkipReason::ALL)),
        ("sshSignFailReason", values(SshSignFailReason::ALL)),
        ("sshDecision", values(SshDecision::ALL)),
        ("sshCloseReason", values(SshCloseReason::ALL)),
        ("browserEndpointState", values(BrowserEndpointState::ALL)),
        ("browserRegistryState", values(BrowserRegistryState::ALL)),
        ("browserFamily", values(BrowserFamily::ALL)),
        ("browserHint", values(BrowserHint::ALL)),
        ("pairingDecision", values(PairingDecision::ALL)),
        ("browserCloseReason", values(BrowserCloseReason::ALL)),
        ("browserSaveKind", values(BrowserSaveKind::ALL)),
        ("browserPasskeyKind", values(BrowserPasskeyKind::ALL)),
        ("passkeyVerification", values(PasskeyVerification::ALL)),
        ("passkeyAlgorithm", values(PasskeyAlgorithm::ALL)),
        ("lockReason", values(LockReason::ALL)),
        ("syncStatus", values(SyncStatus::ALL)),
        ("registrationMode", values(RegistrationMode::ALL)),
        ("hostedMode", values(HostedMode::ALL)),
        ("folderState", values(FolderState::ALL)),
        ("devicePlatform", values(DevicePlatform::ALL)),
    ] {
        enums.insert(name.to_owned(), all);
    }
    enums
}

pub(crate) fn build() -> Value {
    let mut f = Fixtures {
        requests: Map::new(),
        responses: Map::new(),
    };
    let item_request = ItemRequest {
        vault_id: ids::vault(1),
        item_id: ids::item(1),
    };
    session_commands(&mut f);
    quick_unlock_commands(&mut f);
    vault_commands(&mut f);
    item_commands(&mut f, item_request);
    attachment_commands(&mut f, item_request);
    tool_commands(&mut f, item_request);
    import_commands(&mut f);
    export_commands(&mut f);
    ssh_commands(&mut f);
    browser_commands(&mut f);
    sync_commands(&mut f);
    let enums = enums();

    let listed: BTreeSet<&str> = f
        .requests
        .keys()
        .map(String::as_str)
        .chain(CHANNEL_COMMANDS.iter().copied())
        .collect();
    let commands: BTreeSet<&str> = COMMANDS.iter().copied().collect();
    assert_eq!(commands.len(), COMMANDS.len(), "COMMANDS has duplicates");
    assert_eq!(listed, commands, "fixtures and COMMANDS differ");

    json!({
        "commands": COMMANDS,
        "enums": enums,
        "errors": ErrorCode::ALL
            .iter()
            .map(|&code| json!({ "code": code }))
            .collect::<Vec<_>>(),
        "events": events::samples::events(),
        "requests": f.requests,
        "responses": f.responses,
        "binaryResponses": BINARY_RESPONSES,
    })
}

fn render() -> String {
    let body = serde_json::to_string_pretty(&build()).unwrap();
    format!(
        "// Generated from the Rust contract (apps/desktop/src-tauri/src/contract) by\n\
         // `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.\n\
         // Do not edit. `tsc` checks it against the TypeScript contract (ADR-0018).\n\
         import type {{ ContractFixtures }} from \"./contract/fixtures.ts\";\n\
         \n\
         export const fixtures = {body} satisfies ContractFixtures;\n"
    )
}

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/ipc/contract.fixtures.ts")
}

#[test]
fn contract_fixtures_are_current() {
    let expected = render();
    let path = fixtures_path();
    if std::env::var_os("MOSAIC_UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &expected).unwrap();
        return;
    }
    let actual = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        actual == expected,
        "{} is stale; run `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`",
        path.display()
    );
}

#[test]
fn fixtures_hold_secrets_only_where_the_contract_allows() {
    // The sample secrets appear only in requests (SecretIn) and in the
    // responses of the commands that return SecretOut.
    let fixtures = build();
    let responses = fixtures["responses"].as_object().unwrap();
    for (command, samples) in responses {
        let text = samples.to_string();
        let may_hold_secret = matches!(
            command.as_str(),
            "create_account"
                | "reveal_secret"
                | "generate_password"
                | "totp_code"
                | "otp_pending_code"
                | "sync_totp_begin"
        );
        for secret in ["hunter2", "M1-7KQ4", "492039", "q7#Vt2", "JBSWY3DPEHPK3PXP"] {
            assert!(
                may_hold_secret || !text.contains(secret),
                "{command} response holds a secret sample"
            );
        }
    }
    let events = fixtures["events"].to_string();
    assert!(!events.contains("hunter2"));
}
