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

use super::clipboard;
use super::errors::ErrorCode;
use super::events::{self, LockReason, SyncStatus};
use super::generator::{self, Strength};
use super::ids::samples as ids;
use super::import::{
    self, ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportEntryWarning,
    ImportFormat, ImportPreviewRequest, ImportResult, ImportSkipReason, ImportWarning,
};
use super::insights::{self, InsightAction, InsightKind, InsightSeverity};
use super::items::{
    self, AddItemRequest, CustomFieldKind, ItemKind, ItemRequest, ListItemsRequest,
    MoveItemRequest, MovedItem, OtpScheme, RevealRequest, RevealedSecret, SecretFieldKey,
    SetFavoriteRequest, TextFieldKey, TrashFilter, UpdateItemRequest, UrlMatchMode,
};
use super::secret::{SecretIn, SecretOut};
use super::session::{
    self, KitVerification, QuickUnlockKind, QuickUnlockRequest, ShortcutState, VerifyKitRequest,
};
use super::settings::{self, Language, Theme, UpdateSettingsRequest};
use super::ssh::{
    self, SshCloseReason, SshDecision, SshRespondRequest, SshSetEnabledRequest, SshSignFailReason,
    SshSkipReason,
};
use super::totp;
use super::vaults::{self, CreateVaultRequest, RenameVaultRequest};
use crate::commands::names::COMMANDS;

/// Commands whose argument is not a JSON request (a `Channel`).
const CHANNEL_COMMANDS: &[&str] = &["subscribe_events"];

struct Fixtures {
    requests: Map<String, Value>,
    responses: Map<String, Value>,
}

impl Fixtures {
    /// Adds a command's request and response samples. Each request sample
    /// must deserialize back into the request type, as the command will.
    #[allow(clippy::needless_pass_by_value)]
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

fn values<T: Serialize>(all: &[T]) -> Value {
    serde_json::to_value(all).unwrap()
}

#[allow(clippy::too_many_lines)]
pub(crate) fn build() -> Value {
    let mut f = Fixtures {
        requests: Map::new(),
        responses: Map::new(),
    };
    let item_request = ItemRequest {
        vault_id: ids::vault(1),
        item_id: ids::item(1),
    };

    // Session
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
    f.command(
        "unlock_quick",
        QuickUnlockKind::ALL
            .iter()
            .map(|&method| QuickUnlockRequest { method })
            .collect(),
        session::samples::states(),
    );
    f.command("lock", vec![()], session::samples::states());
    f.command("app_status", vec![()], vec![session::samples::app_status()]);

    // Vaults
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

    // Items
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
        vec![items::samples::detail()],
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

    // Clipboard
    f.command(
        "copy_to_clipboard",
        clipboard::samples::requests(),
        clipboard::samples::receipts(),
    );
    f.command("clear_clipboard", vec![()], vec![()]);

    // Generator and one-time codes
    f.command(
        "generate_password",
        generator::samples::requests(),
        vec![generator::samples::generated()],
    );
    f.command("totp_code", vec![item_request], totp::samples::codes());

    // Settings
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

    // Security insights
    f.command(
        "security_report",
        vec![()],
        vec![insights::samples::report()],
    );

    // Import
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

    // SSH agent
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

    let mut enums = Map::new();
    for (name, all) in [
        ("errorCode", values(ErrorCode::ALL)),
        ("quickUnlockKind", values(QuickUnlockKind::ALL)),
        ("itemKind", values(ItemKind::ALL)),
        ("textFieldKey", values(TextFieldKey::ALL)),
        ("secretFieldKey", values(SecretFieldKey::ALL)),
        ("trashFilter", values(TrashFilter::ALL)),
        ("urlMatchMode", values(UrlMatchMode::ALL)),
        ("customFieldKind", values(CustomFieldKind::ALL)),
        ("otpScheme", values(OtpScheme::ALL)),
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
        ("lockReason", values(LockReason::ALL)),
        ("syncStatus", values(SyncStatus::ALL)),
    ] {
        enums.insert(name.to_owned(), all);
    }

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
            "create_account" | "reveal_secret" | "generate_password" | "totp_code"
        );
        for secret in ["hunter2", "M1-7KQ4", "492039", "q7#Vt2"] {
            assert!(
                may_hold_secret || !text.contains(secret),
                "{command} response holds a secret sample"
            );
        }
    }
    let events = fixtures["events"].to_string();
    assert!(!events.contains("hunter2"));
}
