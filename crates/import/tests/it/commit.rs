//! Preview and commit against a real vault session.

#![allow(clippy::unwrap_used, reason = "tests")]

use mosaic_core::model::{ItemType, LoginUrl, SecretField, Text, TextField};
use mosaic_core::{Argon2Profile, MasterPassword, VaultId};
use mosaic_import::{CommitReport, Format, commit, parse};
use mosaic_vault::session::{
    Error as SessionError, ItemFilter, LockedSession, TrashFilter, UnlockedSession,
};
use mosaic_vault::storage::Store;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn session(store: Store) -> (UnlockedSession, VaultId) {
    let (mut s, _) = LockedSession::open(store)
        .create_account(
            &MasterPassword::new("commit test".to_owned()),
            Argon2Profile::Baseline,
        )
        .unwrap();
    let vault = s.create_vault("Imported").unwrap();
    (s, vault)
}

#[test]
fn preview_marks_duplicates_and_commit_adds_items() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, vault) = session(Store::open(dir.path().join("vault.db")).unwrap());
    let existing = s
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("Mail")?;
            e.set_text(TextField::Username, "Alice@Example.com")?;
            e.add_url(&LoginUrl::new("https://mail.example.com/inbox"))?;
            Ok(())
        })
        .unwrap();

    let import = parse(Format::Enpass, &fixture("enpass.json"), None).unwrap();
    let preview = import.preview(&s.items(&ItemFilter::default()));
    let dup = preview.entries.iter().find(|e| e.index == 0).unwrap();
    assert_eq!(dup.duplicate_of.unwrap().item_id, existing);
    assert_eq!(dup.title.as_ref().unwrap().as_str(), "Example Mail");
    assert_eq!(dup.attachments, 1);
    let skip: Vec<u32> = preview
        .entries
        .iter()
        .filter(|e| e.duplicate_of.is_some())
        .map(|e| e.index)
        .collect();
    assert_eq!(skip, [0]);

    let report = commit(import, &mut s, vault, &skip).unwrap();
    let mut want = CommitReport::default();
    want.imported = 8;
    want.skipped = 1;
    assert_eq!(report, want);
    let items = s.items(&ItemFilter::default());
    // 1 existing + 8 imported, minus the card that went to the trash.
    assert_eq!(items.len(), 8);
    let router = items
        .iter()
        .find(|r| r.item.title() == Some("Router admin"))
        .unwrap();
    assert_eq!(
        router
            .item
            .secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret(),
        "router-pw"
    );
    assert!(router.item.tags().contains(&"archived"));
    let mut only_trash = ItemFilter::default();
    only_trash.trash = TrashFilter::Only;
    assert_eq!(s.items(&only_trash).len(), 1);

    // Importing the same file again: everything is a duplicate now.
    let again = parse(Format::Enpass, &fixture("enpass.json"), None).unwrap();
    let mut everything = ItemFilter::default();
    everything.trash = TrashFilter::Include;
    let all = s.items(&everything);
    let p = again.preview(&all);
    assert!(p.entries.iter().all(|e| e.duplicate_of.is_some()), "{p:?}");
}

#[test]
fn attachments_need_a_file_store() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, vault) = session(Store::open(dir.path().join("vault.db")).unwrap());
    let import = parse(Format::Enpass, &fixture("enpass.json"), None).unwrap();
    let report = commit(import, &mut s, vault, &[]).unwrap();
    assert_eq!((report.attachments, report.attachments_failed), (1, 0));
    let mail = s
        .items(&ItemFilter::default())
        .into_iter()
        .find(|r| r.item.title() == Some("Example Mail"))
        .unwrap();
    let a = &mail.item.attachments()[0];
    let data = s
        .attachment(mail.vault_id, mail.item_id, a.attachment_id)
        .unwrap();
    assert_eq!(data.expose_secret(), b"hello attachment");

    let (mut mem, vault) = session(Store::open_in_memory().unwrap());
    let import = parse(Format::Enpass, &fixture("enpass.json"), None).unwrap();
    let report = commit(import, &mut mem, vault, &[]).unwrap();
    assert_eq!((report.attachments, report.attachments_failed), (0, 1));
    assert_eq!(report.imported, 9);
}

#[test]
fn failures_and_unknown_vaults() {
    let (mut s, vault) = session(Store::open_in_memory().unwrap());
    let mut import = parse(Format::Bitwarden, &fixture("bitwarden.json"), None).unwrap();
    // An empty tag fails the item edit: that entry fails, the rest go on.
    import.entries[0].draft.tags.push(Text::new(""));
    let report = commit(import, &mut s, vault, &[]).unwrap();
    assert_eq!(report.failed, 1);
    assert_eq!(report.imported, 8);

    let import = parse(Format::Bitwarden, &fixture("bitwarden.json"), None).unwrap();
    let missing = VaultId::new_v7().unwrap();
    assert!(matches!(
        commit(import, &mut s, missing, &[]),
        Err(SessionError::UnknownVault)
    ));
}
