//! Tests of the CLI session API (CRYPTO_SPEC §6.8) and the encrypted export
//! writer (§11.2). Accounts use the baseline profile; exports the standard
//! one (the lowest §11.2 accepts), so each export costs one 256 MiB Argon2id.

use core::time::Duration;
use std::io::Read;

use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_core::{
    Argon2Profile, ExportPassphrase, ExportReader, MasterPassword, SecretKey, SecretString,
};

use super::{Error, ExportSelection, LockedSession, UnlockedSession};
use crate::storage::Store;

const NOW: u64 = 1_800_000_000_000;
const MIN5: Duration = Duration::from_mins(5);

fn pw() -> MasterPassword {
    MasterPassword::new("cli session test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    account_in(Store::open_in_memory().unwrap())
}

fn account_in(store: Store) -> (UnlockedSession, SecretKey) {
    let (mut s, created) = LockedSession::open(store)
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    let vault = s.create_vault("Personal").unwrap();
    s.add_item(vault, &ItemType::Login, |e| {
        e.set_title("Example")?;
        e.set_secret(SecretField::Password, "hunter2")
    })
    .unwrap();
    (s, created.secret_key)
}

fn expect_rejected(
    locked: LockedSession,
    key: &mosaic_core::CliSessionKey,
    file: &mosaic_core::CliSessionFile,
    now: u64,
) -> LockedSession {
    match locked.unlock_cli_session(key, file, now) {
        Err(e) => {
            assert!(
                matches!(e.error, Error::CliSessionRejected),
                "unexpected {:?}",
                e.error
            );
            e.session
        }
        Ok(_) => panic!("the session was accepted"),
    }
}

#[test]
fn unlock_cli_session_round_trip() {
    let (s, _sk) = account();
    let (key, file) = s.create_cli_session(MIN5, NOW).unwrap();
    let vault = s.vaults()[0].vault_id;
    let locked = s.lock();
    assert!(locked.account_id().is_ok());
    let opened = locked
        .unlock_cli_session(&key, &file, NOW + 1_000)
        .map_err(|e| e.error)
        .unwrap();
    let items = opened.items(&super::ItemFilter::default());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item.title(), Some("Example"));
    assert_eq!(items[0].vault_id, vault);
    assert_eq!(
        items[0]
            .item
            .secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret(),
        "hunter2"
    );
    // The same session opens again until it expires.
    let locked = opened.lock();
    let again = locked
        .unlock_cli_session(&key, &file, NOW + 299_000)
        .unwrap();
    // Absolute expiry, not sliding: refused after 5 minutes whatever was done.
    let locked = again.lock();
    let locked = expect_rejected(locked, &key, &file, NOW + 300_001);
    // A key part of another session does not open it.
    let (other_key, _) = account().0.create_cli_session(MIN5, NOW).unwrap();
    expect_rejected(locked, &other_key, &file, NOW + 1_000);
}

#[test]
fn cli_session_needs_password_unlock() {
    let (s, sk) = account();
    let (key, file) = s.create_cli_session(MIN5, NOW).unwrap();
    // L3: a session opened from a CLI session offers no way to seal another.
    let locked = s.lock();
    let opened = locked
        .unlock_cli_session(&key, &file, NOW + 1_000)
        .map_err(|e| e.error)
        .unwrap();
    assert!(matches!(
        opened.create_cli_session(MIN5, NOW + 1_000),
        Err(Error::ReauthenticationRequired)
    ));
    // Not after a lock and another session open either.
    let locked = opened.lock();
    let opened = locked
        .unlock_cli_session(&key, &file, NOW + 2_000)
        .map_err(|e| e.error)
        .unwrap();
    assert!(matches!(
        opened.create_cli_session(MIN5, NOW + 2_000),
        Err(Error::ReauthenticationRequired)
    ));
    // Only the master password does.
    let locked = opened.lock();
    let fresh = locked.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    assert!(fresh.create_cli_session(MIN5, NOW + 3_000).is_ok());
}

#[test]
fn cli_session_ttl_is_one_to_thirty_minutes() {
    let (s, _sk) = account();
    for secs in [0, 30, 31 * 60, 3600] {
        assert!(matches!(
            s.create_cli_session(Duration::from_secs(secs), NOW),
            Err(Error::Core(mosaic_core::Error::InvalidTtl))
        ));
    }
    assert!(s.create_cli_session(Duration::from_secs(60), NOW).is_ok());
    assert!(s.create_cli_session(Duration::from_mins(30), NOW).is_ok());
}

#[test]
fn cli_session_refused_after_password_change() {
    let (mut s, sk) = account();
    let (key, file) = s.create_cli_session(MIN5, NOW).unwrap();
    let new = MasterPassword::new("a brand new password for this test".to_owned());
    s.change_master_password(&pw(), &new, &sk).unwrap();
    let locked = expect_rejected(s.lock(), &key, &file, NOW + 1_000);
    // The refusal hands the locked session back, which still unlocks.
    assert!(locked.unlock(&new, &sk).is_ok());
}

#[test]
fn cli_session_refused_after_rekey() {
    let (mut s, _sk) = account();
    let (key, file) = s.create_cli_session(MIN5, NOW).unwrap();
    // A re-key raises the epoch (the full flow is in the rotation tests).
    s.raise_epoch_to(2).unwrap();
    expect_rejected(s.lock(), &key, &file, NOW + 1_000);
}

fn passphrase() -> ExportPassphrase {
    ExportPassphrase::new(&SecretString::from_str_copy("an export passphrase").unwrap()).unwrap()
}

fn read_export(bytes: &[u8]) -> serde_json::Value {
    let mut reader = ExportReader::open(bytes, &passphrase()).unwrap();
    let mut plain = Vec::new();
    reader.read_to_end(&mut plain).unwrap();
    serde_json::from_slice(&plain).unwrap()
}

#[test]
fn write_export_skips_attachments_by_default() {
    // Attachments need a file store.
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _sk) = account_in(Store::open(dir.path().join("vault.db")).unwrap());
    let vault = s.vaults()[0].vault_id;
    let doc = s
        .add_item(vault, &ItemType::Document, |e| e.set_title("Scan"))
        .unwrap();
    let data = vec![7u8; 100_000];
    s.add_attachment(vault, doc, "scan.bin", "application/octet-stream", &data)
        .unwrap();
    let trashed = s
        .add_item(vault, &ItemType::SecureNote, |e| {
            e.set_title("Gone")?;
            e.set_text(TextField::Notes, "secret note")
        })
        .unwrap();
    s.trash_item(vault, trashed).unwrap();

    let preview = s.export_preview(&ExportSelection::default()).unwrap();
    assert_eq!(
        (preview.vaults, preview.items),
        (1, 2),
        "trash is not exported"
    );
    assert_eq!(preview.items_with_attachments, vec![(vault, doc)]);
    assert_eq!(
        (preview.attachments, preview.attachment_bytes),
        (1, 100_000)
    );

    let mut file = Vec::new();
    let report = s
        .write_export(&ExportSelection::default(), &mut file, &passphrase())
        .unwrap();
    assert_eq!(
        (
            report.items,
            report.attachments,
            report.attachments_left_out
        ),
        (2, 0, 1)
    );
    let plain = read_export(&file);
    assert_eq!(plain["format"], "mosaic-export");
    assert_eq!(plain["v"], 1);
    let items = plain["vaults"][0]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let scan = items.iter().find(|i| i["title"] == "Scan").unwrap();
    assert_eq!(scan["attachments"][0]["name"], "scan.bin");
    assert_eq!(scan["attachments"][0]["size"], 100_000);
    assert!(scan["attachments"][0]["data"].is_null());
    assert!(!items.iter().any(|i| i["title"] == "Gone"));
    let login = items.iter().find(|i| i["title"] == "Example").unwrap();
    let password = login["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "login.password")
        .unwrap();
    assert_eq!(password["value"], "hunter2");
    assert_eq!(password["secret"], true);
    // No ID of the account, vault or item is exported.
    assert!(plain["vaults"][0].get("id").is_none() && login.get("id").is_none());

    // With attachments the data is in: base64url of the bytes, no padding.
    let mut file = Vec::new();
    let sel = ExportSelection {
        attachments: true,
        ..ExportSelection::default()
    };
    let report = s.write_export(&sel, &mut file, &passphrase()).unwrap();
    assert_eq!((report.attachments, report.attachments_left_out), (1, 0));
    let plain = read_export(&file);
    let scan = plain["vaults"][0]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["title"] == "Scan")
        .unwrap()
        .clone();
    let text = scan["attachments"][0]["data"].as_str().unwrap();
    assert!(!text.contains('='));
    assert_eq!(text.len(), 100_000_usize.div_ceil(3) * 4 - 2);
}

#[test]
fn write_export_names_a_vault_and_refuses_unknown_ones() {
    let (s, _sk) = account();
    let sel = ExportSelection {
        vaults: Some(vec![mosaic_core::VaultId::new_v7().unwrap()]),
        ..ExportSelection::default()
    };
    let mut sink = Vec::new();
    assert!(matches!(
        s.write_export(&sel, &mut sink, &passphrase()),
        Err(Error::UnknownVault)
    ));
    assert!(sink.is_empty(), "nothing is written for a bad selection");
}
