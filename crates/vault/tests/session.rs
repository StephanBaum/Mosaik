//! End-to-end tests of the session layer: account lifecycle, vaults,
//! items, search, attachments, moves, key generations, OTP codes and
//! applying received records.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

mod common;

use common::{PASSWORD, Server, join, new_account, new_account_in, pw};
use mosaic_core::model::{
    AttachmentRef, ItemType, LoginUrl, OtpConfig, OtpKind, SecretField, Text, TextField, VaultMeta,
};
use mosaic_core::{
    Argon2Profile, AttachmentId, AttachmentKey, DeviceId, ItemId, KeyGeneration,
    MAX_ATTACHMENT_LEN, SecretKey, SecretVec, VaultKey,
};
use mosaic_vault::session::{
    ApplyOutcome, Error, ItemFilter, LockedSession, MAX_ACCOUNT_ATTACHMENT_BYTES, RemoteRecord,
    TrashFilter, UnlockedSession,
};
use mosaic_vault::storage::{Error as StorageError, RecordBody, Store};

fn login(
    s: &mut UnlockedSession,
    vault: mosaic_core::VaultId,
    title: &str,
    password: &str,
) -> ItemId {
    s.add_item(vault, &ItemType::Login, |e| {
        e.set_title(title)?;
        e.set_text(TextField::Username, "alice@example.com")?;
        e.set_secret(SecretField::Password, password)?;
        e.add_url(&LoginUrl::new("https://login.example.com/start"))?;
        e.add_tag("Work")?;
        Ok(())
    })
    .unwrap()
}

fn password_of(s: &UnlockedSession, vault: mosaic_core::VaultId, item: ItemId) -> String {
    s.item(vault, item)
        .unwrap()
        .secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

#[test]
fn account_create_lock_unlock_and_emergency_kit() {
    let (mut s, created) = new_account();
    let kit = &created.emergency_kit;
    assert_eq!(kit.account_id, s.account_id());
    assert_eq!(created.params.account_id, s.account_id());
    assert!(kit.secret_key.expose_secret().starts_with("M1-"));
    // The displayed Secret Key parses back to the same key.
    let parsed = SecretKey::parse(kit.secret_key.expose_secret()).unwrap();
    assert_eq!(parsed, created.secret_key);
    // Debug output never shows the Secret Key.
    assert!(!format!("{created:?}").contains(kit.secret_key.expose_secret()));

    let vault = s.create_vault("Personal").unwrap();
    let item = login(&mut s, vault, "Example", "hunter2");
    let device = s.device_id();

    let locked = s.lock();
    assert!(locked.has_account().unwrap());

    // A wrong password or Secret Key fails and gives the session back.
    let err = locked
        .unlock(&pw("wrong password"), &created.secret_key)
        .unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    assert!(!format!("{err:?}").contains("wrong password"));
    let other_key = SecretKey::generate().unwrap();
    let err = err.session.unlock(&pw(PASSWORD), &other_key).unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));

    let s = err
        .session
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap();
    // Every unlock writes under a fresh writer ID (ADR-0015), recorded in
    // the store as the last one used.
    assert_ne!(s.device_id(), device, "a fresh writer ID per unlock");
    assert_eq!(s.store().device_id().unwrap(), Some(s.device_id()));
    assert_eq!(password_of(&s, vault, item), "hunter2");
    assert_eq!(s.vaults().len(), 1);
    assert_eq!(s.vaults()[0].name, Some("Personal"));
    assert_eq!(s.corrupt_records().len(), 0);
}

#[test]
fn a_store_holds_one_account() {
    let (s, _) = new_account();
    let store = s.lock().into_store();
    let err = LockedSession::open(store)
        .create_account(&pw("another"), Argon2Profile::Baseline)
        .unwrap_err();
    assert!(matches!(
        err.error,
        Error::Store(StorageError::AccountExists)
    ));
    let empty = LockedSession::open(Store::open_in_memory().unwrap());
    assert!(!empty.has_account().unwrap());
    let created = SecretKey::generate().unwrap();
    let err = empty.unlock(&pw(PASSWORD), &created).unwrap_err();
    assert!(matches!(err.error, Error::NoAccount));
    // An empty password is refused and nothing is stored.
    let err = err
        .session
        .create_account(&pw(""), Argon2Profile::Baseline)
        .unwrap_err();
    assert!(matches!(
        err.error,
        Error::Core(mosaic_core::Error::EmptyPassword)
    ));
    assert!(!err.session.has_account().unwrap());
}

#[test]
fn change_master_password() {
    let (mut s, created) = new_account();
    let vault = s.create_vault("Personal").unwrap();
    let item = login(&mut s, vault, "Example", "hunter2");
    let old_params = s.store().account().unwrap().unwrap();

    let err = s
        .change_master_password(&pw("not it"), &pw("new password"), &created.secret_key)
        .unwrap_err();
    assert!(matches!(err, Error::UnlockFailed));

    let changed = s
        .change_master_password(&pw(PASSWORD), &pw("new password"), &created.secret_key)
        .unwrap();
    assert_ne!(changed.params.salt, old_params.salt, "a new salt");
    assert_eq!(changed.params.account_id, s.account_id());

    let locked = s.lock();
    let err = locked
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    let s = err
        .session
        .unlock(&pw("new password"), &created.secret_key)
        .unwrap();
    assert_eq!(password_of(&s, vault, item), "hunter2");
}

#[test]
fn multiple_vaults_and_rename() {
    let (mut s, created) = new_account();
    let personal = s.create_vault("Personal").unwrap();
    let work = s.create_vault("Work").unwrap();
    assert_ne!(personal, work);
    s.rename_vault(work, "Job").unwrap();
    assert_eq!(
        s.vault_meta(work).unwrap().and_then(VaultMeta::name),
        Some("Job")
    );
    login(&mut s, work, "Intranet", "pw");

    // The name is not in the store in cleartext; it survives a relock.
    let s = s.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    let mut names: Vec<_> = s
        .vaults()
        .iter()
        .map(|v| v.name.unwrap().to_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["Job", "Personal"]);
    let summary = s.vaults().into_iter().find(|v| v.vault_id == work).unwrap();
    assert_eq!(summary.item_count, 1);
    assert!(summary.available);
    assert_eq!(summary.key_generation, KeyGeneration::FIRST);

    let unknown = mosaic_core::VaultId::new_v7().unwrap();
    let mut s = s;
    assert!(matches!(
        s.rename_vault(unknown, "x"),
        Err(Error::UnknownVault)
    ));
    assert!(matches!(s.vault_meta(unknown), Err(Error::UnknownVault)));
}

#[test]
fn item_lifecycle_history_favorite_trash_purge() {
    let (mut s, created) = new_account();
    let vault = s.create_vault("Personal").unwrap();
    let item = login(&mut s, vault, "Example", "first");
    let note = s
        .add_item(vault, &ItemType::SecureNote, |e| {
            e.set_title("Note")?;
            e.set_text(TextField::Notes, "remember the milk")
        })
        .unwrap();

    // A password change pushes the old value into the history.
    s.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "second")
    })
    .unwrap();
    s.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "third")
    })
    .unwrap();
    assert_eq!(password_of(&s, vault, item), "third");
    let history: Vec<String> = s
        .item(vault, item)
        .unwrap()
        .password_history()
        .iter()
        .map(|h| h.value.expose_secret().to_owned())
        .collect();
    assert!(history.contains(&"first".to_owned()));
    assert!(history.contains(&"second".to_owned()));

    // An edit that changes nothing is not a new version.
    let outbox_before = s.pending_uploads(usize::MAX).unwrap().len();
    s.update_item(vault, item, |_| Ok(())).unwrap();
    assert_eq!(s.pending_uploads(usize::MAX).unwrap().len(), outbox_before);
    // A failing edit leaves the item as it was.
    let before = s.item(vault, item).unwrap().version().clone();
    let failed = s.update_item(vault, item, |e| {
        e.set_title("changed")?;
        e.add_tag("\u{7}")
    });
    assert!(matches!(
        failed,
        Err(Error::Core(mosaic_core::Error::Malformed))
    ));
    assert_eq!(s.item(vault, item).unwrap().version(), &before);
    assert_eq!(s.item(vault, item).unwrap().title(), Some("Example"));

    // Favorites.
    s.set_favorite(vault, note, true).unwrap();
    let mut favorites = ItemFilter::default();
    favorites.favorites_only = true;
    let favs = s.items(&favorites);
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].item_id, note);

    // Trash and restore.
    s.trash_item(vault, item).unwrap();
    assert!(s.item(vault, item).unwrap().trashed_at().is_some());
    assert_eq!(s.items(&ItemFilter::default()).len(), 1);
    let mut trashed = ItemFilter::default();
    trashed.trash = TrashFilter::Only;
    let trash = s.items(&trashed);
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].item_id, item);
    let mut whole_vault = ItemFilter::default();
    whole_vault.trash = TrashFilter::Include;
    whole_vault.vault = Some(vault);
    assert_eq!(s.items(&whole_vault).len(), 2);
    assert!(
        s.search("example").is_empty(),
        "trashed items are not found"
    );
    s.restore_item(vault, item).unwrap();
    assert!(s.item(vault, item).unwrap().trashed_at().is_none());
    assert_eq!(s.search("example").len(), 1);

    // Purge: a tombstone that syncs, not a missing row.
    s.purge_item(vault, item).unwrap();
    assert!(s.item(vault, item).unwrap().is_deleted());
    assert!(matches!(
        s.store().record(&vault, &item).unwrap().unwrap().body,
        RecordBody::Tombstone(_)
    ));
    assert_eq!(s.search("example").len(), 0);
    assert_eq!(s.items(&ItemFilter::default()).len(), 1);
    assert!(matches!(
        s.update_item(vault, item, |e| e.set_title("x")),
        Err(Error::ItemDeleted)
    ));
    assert!(matches!(s.purge_item(vault, item), Err(Error::ItemDeleted)));
    assert!(matches!(
        s.update_item(vault, ItemId::new_v7().unwrap(), |e| e.set_title("x")),
        Err(Error::UnknownItem)
    ));

    // Everything survives a relock: the tombstone stays a tombstone.
    let s = s.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert!(s.item(vault, item).unwrap().is_deleted());
    assert!(s.item(vault, note).unwrap().favorite());
    assert_eq!(
        s.item(vault, note).unwrap().text(TextField::Notes),
        Some("remember the milk")
    );
}

#[test]
fn search_covers_titles_users_urls_and_tags() {
    let (mut s, created) = new_account();
    let personal = s.create_vault("Personal").unwrap();
    let work = s.create_vault("Work").unwrap();
    let mail = login(&mut s, personal, "Mailbox", "a");
    let bank = s
        .add_item(work, &ItemType::Login, |e| {
            e.set_title("Bank of Example")?;
            e.set_text(TextField::Username, "BOB")?;
            e.add_url(&LoginUrl::new("https://bank.test/login"))?;
            e.add_tag("finance")
        })
        .unwrap();
    let ids = |s: &UnlockedSession, q: &str| -> Vec<ItemId> {
        s.search(q).iter().map(|r| r.item_id).collect()
    };
    assert_eq!(ids(&s, "MAILBOX"), [mail]);
    assert_eq!(ids(&s, "bob"), [bank]);
    assert_eq!(ids(&s, "bank.test"), [bank]);
    assert_eq!(ids(&s, "finance"), [bank]);
    assert_eq!(ids(&s, "work"), [mail], "tags of the first item");
    assert_eq!(ids(&s, "bank finance"), [bank], "every term must match");
    assert_eq!(ids(&s, "bank work").len(), 0);
    assert_eq!(ids(&s, "   ").len(), 0);
    // A title prefix ranks first.
    let mut both = ids(&s, "example");
    both.sort();
    let mut expected = vec![mail, bank];
    expected.sort();
    assert_eq!(both, expected, "alice@example.com and Bank of Example");
    assert_eq!(s.search("bank")[0].vault_id, work);
    // Secrets and notes are not indexed.
    s.update_item(personal, mail, |e| e.set_text(TextField::Notes, "zebra"))
        .unwrap();
    assert_eq!(ids(&s, "zebra").len(), 0);
    assert!(ids(&s, "a").len() >= 2);

    // The index is rebuilt from the decrypted items on unlock.
    let s = s.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert_eq!(ids(&s, "bob"), [bank]);
}

#[test]
fn attachments_round_trip_with_limits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, created) = new_account_in(Store::open(dir.path().join("vault.db")).unwrap());
    let vault = s.create_vault("Personal").unwrap();
    let item = s
        .add_item(vault, &ItemType::Document, |e| e.set_title("Passport scan"))
        .unwrap();
    let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let att = s
        .add_attachment(vault, item, "scan.pdf", "application/pdf", &data)
        .unwrap();
    assert_eq!(
        s.attachment(vault, item, att).unwrap().expose_secret(),
        &data[..]
    );
    assert_eq!(s.attachment_usage(), 200_000);
    let refs = s.item(vault, item).unwrap().attachments();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].file_name.as_str(), "scan.pdf");
    assert_eq!(refs[0].size, 200_000);
    // The ciphertext is queued for upload.
    assert_eq!(s.store().pending_attachment_uploads().unwrap().len(), 1);

    // An empty file works too.
    let empty = s
        .add_attachment(vault, item, "empty.txt", "text/plain", &[])
        .unwrap();
    assert_eq!(
        s.attachment(vault, item, empty)
            .unwrap()
            .expose_secret()
            .len(),
        0
    );

    // Too large: refused before anything is encrypted.
    let big = vec![0u8; MAX_ATTACHMENT_LEN + 1];
    assert!(matches!(
        s.add_attachment(vault, item, "big.bin", "application/octet-stream", &big),
        Err(Error::AttachmentTooLarge)
    ));
    drop(big);
    let unknown = mosaic_core::AttachmentId::new_v7().unwrap();
    assert!(matches!(
        s.attachment(vault, item, unknown),
        Err(Error::UnknownAttachment)
    ));
    assert!(matches!(
        s.remove_attachment(vault, item, unknown),
        Err(Error::UnknownAttachment)
    ));

    // Persisted: readable after a relock.
    let mut s = s.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert_eq!(
        s.attachment(vault, item, att).unwrap().expose_secret(),
        &data[..]
    );

    // Remove: the reference and the file go.
    s.remove_attachment(vault, item, att).unwrap();
    assert_eq!(s.item(vault, item).unwrap().attachments().len(), 1);
    assert!(s.store().attachment_info(&att).unwrap().is_none());
    assert!(matches!(
        s.attachment(vault, item, att),
        Err(Error::UnknownAttachment)
    ));

    // Purging the item removes the remaining file.
    s.purge_item(vault, item).unwrap();
    assert!(s.store().attachment_info(&empty).unwrap().is_none());
    assert_eq!(s.attachment_usage(), 0);
}

#[test]
fn an_in_memory_store_has_no_attachments() {
    let (mut s, _) = new_account();
    let vault = s.create_vault("Personal").unwrap();
    let item = login(&mut s, vault, "Example", "pw");
    assert!(matches!(
        s.add_attachment(vault, item, "a.txt", "text/plain", b"hi"),
        Err(Error::Store(StorageError::AttachmentsUnavailable))
    ));
    // Nothing was added to the item.
    assert_eq!(s.item(vault, item).unwrap().attachments().len(), 0);
}

#[test]
fn move_item_reencrypts_under_the_target_vault() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, created) = new_account_in(Store::open(dir.path().join("vault.db")).unwrap());
    let from = s.create_vault("Personal").unwrap();
    let to = s.create_vault("Work").unwrap();
    let item = login(&mut s, from, "Example", "hunter2");
    s.update_item(from, item, |e| e.set_secret(SecretField::Password, "newer"))
        .unwrap();
    let att = s
        .add_attachment(from, item, "key.txt", "text/plain", b"attachment body")
        .unwrap();

    assert!(matches!(
        s.move_item(from, item, from),
        Err(Error::SameVault)
    ));
    let moved = s.move_item(from, item, to).unwrap();
    assert_ne!(moved, item);

    // The source is a tombstone; the copy keeps every field and the history.
    assert!(s.item(from, item).unwrap().is_deleted());
    let copy = s.item(to, moved).unwrap();
    assert_eq!(copy.title(), Some("Example"));
    assert_eq!(copy.password_history().len(), 1);
    assert_eq!(password_of(&s, to, moved), "newer");
    // The attachment got a fresh ID and key in the target vault.
    let refs = copy.attachments();
    assert_eq!(refs.len(), 1);
    assert_ne!(refs[0].attachment_id, att);
    let new_att = refs[0].attachment_id;
    assert_eq!(
        s.attachment(to, moved, new_att).unwrap().expose_secret(),
        b"attachment body"
    );
    assert!(s.store().attachment_info(&att).unwrap().is_none());
    let search: Vec<_> = s
        .search("example")
        .iter()
        .map(|r| (r.vault_id, r.item_id))
        .collect();
    assert_eq!(search, [(to, moved)]);

    // The stored record is sealed for the target vault: it authenticates
    // only with the target vault's ID.
    let rec = s.store().record(&to, &moved).unwrap().unwrap();
    assert!(matches!(rec.body, RecordBody::Item(_)));
    let s = s.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert_eq!(password_of(&s, to, moved), "newer");
    assert_eq!(
        s.attachment(to, moved, new_att).unwrap().expose_secret(),
        b"attachment body"
    );
    assert_eq!(s.corrupt_records().len(), 0);
}

#[test]
fn otp_codes_for_totp_and_hotp_items() {
    let (mut s, _) = new_account();
    let vault = s.create_vault("Personal").unwrap();
    let rfc = SecretVec::from_slice(b"12345678901234567890").unwrap();
    let mut totp = OtpConfig::totp(rfc);
    totp.digits = 8;
    let item = s
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("With 2FA")?;
            e.set_otp(&totp)
        })
        .unwrap();
    // RFC 6238 Appendix B, SHA-1, T = 59.
    let code = s.otp_code(vault, item, 59).unwrap();
    assert_eq!(code.code.expose_secret(), "94287082");
    assert_eq!(code.seconds_remaining, Some(1));
    // TOTP does not change the item.
    let v = s.item(vault, item).unwrap().version().clone();
    s.otp_code(vault, item, 60).unwrap();
    assert_eq!(s.item(vault, item).unwrap().version(), &v);

    let mut hotp = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
    hotp.kind = OtpKind::Hotp;
    let counter_item = s
        .add_item(vault, &ItemType::Totp, |e| {
            e.set_otp(&hotp)?;
            e.set_otp_counter(1)
        })
        .unwrap();
    // RFC 4226 Appendix D: counter 1, then 2; never the same code twice.
    let first = s.otp_code(vault, counter_item, 0).unwrap();
    assert_eq!(first.code.expose_secret(), "287082");
    assert_eq!(first.seconds_remaining, None);
    let second = s.otp_code(vault, counter_item, 0).unwrap();
    assert_eq!(second.code.expose_secret(), "359152");
    assert_eq!(s.item(vault, counter_item).unwrap().otp_counter(), 3);

    let plain = login(&mut s, vault, "No 2FA", "pw");
    assert!(matches!(s.otp_code(vault, plain, 0), Err(Error::NoOtp)));
    // A secret mosaic-totp cannot use is refused when it is set, so no
    // stored item has an OTP configuration that cannot produce codes.
    let too_long =
        OtpConfig::totp(SecretVec::from_slice(&[1u8; mosaic_totp::MAX_SECRET_LEN + 1]).unwrap());
    let before = s.items(&ItemFilter::default()).len();
    assert!(matches!(
        s.add_item(vault, &ItemType::Totp, |e| e.set_otp(&too_long)),
        Err(Error::Core(mosaic_core::Error::Malformed))
    ));
    assert_eq!(s.items(&ItemFilter::default()).len(), before);
    let longest =
        OtpConfig::totp(SecretVec::from_slice(&[1u8; mosaic_totp::MAX_SECRET_LEN]).unwrap());
    let long_item = s
        .add_item(vault, &ItemType::Totp, |e| e.set_otp(&longest))
        .unwrap();
    assert_eq!(
        s.otp_code(vault, long_item, 0)
            .unwrap()
            .code
            .expose_secret()
            .len(),
        6
    );
}

#[test]
fn key_rotation_raises_the_generation_and_rejects_rollback() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    for r in &server.log {
        b.apply_remote_record(r).unwrap();
    }
    let old_record = server.log[0].clone();
    assert_eq!(password_of(&b, vault, item), "v1");

    // A rotates; every record is re-sealed under generation 2.
    let generation = a.rotate_vault_key(vault).unwrap();
    assert_eq!(generation.get(), 2);
    a.update_item(vault, item, |e| e.set_secret(SecretField::Password, "v2"))
        .unwrap();
    let start = server.log.len();
    server.upload(&mut a);
    let rotated: Vec<RemoteRecord> = server.log[start..].to_vec();
    assert!(
        rotated
            .iter()
            .all(|r| r.body.key_generation() == generation)
    );

    // B cannot accept generation 2 records before it accepts the key.
    assert!(matches!(
        b.apply_remote_record(&rotated[0]),
        Err(Error::KeyGenerationNotAccepted {
            highest_accepted: 1,
            offered: 2
        })
    ));
    // B has a pending change under generation 1; accepting the new key
    // re-seals it under generation 2.
    let other = login(&mut b, vault, "Other", "b");
    let (gen2, wrapped2) = a
        .store()
        .vault_keys(&vault)
        .unwrap()
        .into_iter()
        .find(|(g, _)| *g == generation)
        .unwrap();
    b.apply_remote_vault_key(vault, gen2, &wrapped2).unwrap();
    let pending = b.pending_uploads(usize::MAX).unwrap();
    assert!(
        pending
            .iter()
            .all(|r| r.as_ref().unwrap().body.key_generation() == generation)
    );
    assert!(
        pending
            .iter()
            .any(|r| r.as_ref().unwrap().record_id == other)
    );
    for r in &rotated {
        b.apply_remote_record(r).unwrap();
    }
    assert_eq!(password_of(&b, vault, item), "v2");
    assert_eq!(b.vaults()[0].key_generation, generation);

    // The old generation-1 record is a rollback now.
    assert!(matches!(
        b.apply_remote_record(&old_record),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    // So is the generation-1 vault key, with the same session error
    // (review M2: it used to surface as a store error).
    let (gen1, wrapped1) = a
        .store()
        .vault_keys(&vault)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(gen1, KeyGeneration::FIRST);
    assert!(matches!(
        b.apply_remote_vault_key(vault, gen1, &wrapped1),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    // The same key again is a no-op.
    b.apply_remote_vault_key(vault, gen2, &wrapped2).unwrap();
    // A key wrapped for another vault ID does not open.
    let foreign = mosaic_core::VaultId::new_v7().unwrap();
    assert!(matches!(
        b.apply_remote_vault_key(foreign, gen2, &wrapped2),
        Err(Error::Core(mosaic_core::Error::Decryption))
    ));
    assert!(b.vaults().iter().all(|v| v.vault_id != foreign));

    // Rotation and its records survive a relock.
    let a = a.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert_eq!(a.vaults()[0].key_generation, generation);
    assert_eq!(password_of(&a, vault, item), "v2");
}

#[test]
fn received_records_are_merged_and_replays_ignored() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    assert_eq!(b.vaults()[0].name, Some("Personal"));
    let mut server = Server::default();
    server.upload(&mut a);
    let v1 = server.log[0].clone();
    assert_eq!(b.apply_remote_record(&v1).unwrap(), ApplyOutcome::New);
    assert_eq!(b.apply_remote_record(&v1).unwrap(), ApplyOutcome::Ignored);
    assert!(
        b.pending_uploads(10).unwrap().is_empty(),
        "nothing to upload"
    );

    a.update_item(vault, item, |e| e.set_secret(SecretField::Password, "v2"))
        .unwrap();
    server.upload(&mut a);
    let v2 = server.log[1].clone();
    assert_eq!(
        b.apply_remote_record(&v2).unwrap(),
        ApplyOutcome::FastForward
    );
    // Replaying the older version is a rollback attempt: ignored.
    assert_eq!(b.apply_remote_record(&v1).unwrap(), ApplyOutcome::Ignored);
    assert_eq!(password_of(&b, vault, item), "v2");

    // Concurrent edits: both values survive, one as a conflict copy.
    a.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "from-a")
    })
    .unwrap();
    b.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "from-b")
    })
    .unwrap();
    server.upload(&mut a);
    let from_a = server.log.last().unwrap().clone();
    let outcome = b.apply_remote_record(&from_a).unwrap();
    assert!(matches!(
        outcome,
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: false
        }
    ));
    let merged = b.item(vault, item).unwrap();
    let conflicts = merged.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].key, "login.password");
    // The merge is queued for upload, so A learns B's value.
    server.upload(&mut b);
    let from_b = server.log.last().unwrap().clone();
    a.apply_remote_record(&from_b).unwrap();
    assert_eq!(a.item(vault, item).unwrap(), b.item(vault, item).unwrap());

    // A record filed under another ID does not authenticate.
    let mut moved = from_b.clone();
    moved.record_id = ItemId::new_v7().unwrap();
    assert!(matches!(
        a.apply_remote_record(&moved),
        Err(Error::Core(mosaic_core::Error::ItemCorrupted { item_id }))
            if item_id == moved.record_id
    ));
    assert!(matches!(
        a.item(vault, moved.record_id),
        Err(Error::UnknownItem)
    ));
    let mut unknown_vault = from_b;
    unknown_vault.vault_id = mosaic_core::VaultId::new_v7().unwrap();
    assert!(matches!(
        a.apply_remote_record(&unknown_vault),
        Err(Error::UnknownVault)
    ));
}

#[test]
fn delete_racing_an_edit_keeps_the_edit() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();

    // A deletes while B (offline) changes the password.
    a.purge_item(vault, item).unwrap();
    b.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "edited")
    })
    .unwrap();
    server.upload(&mut a);
    let tombstone = server.log.last().unwrap().clone();
    let outcome = b.apply_remote_record(&tombstone).unwrap();
    let ApplyOutcome::Merged {
        restored_fields, ..
    } = outcome
    else {
        panic!("expected a merge, got {outcome:?}");
    };
    assert!(restored_fields > 0);
    assert_eq!(password_of(&b, vault, item), "edited");
    assert_eq!(b.item(vault, item).unwrap().title(), Some("Example"));
    assert!(
        b.item(vault, item)
            .unwrap()
            .deleted_concurrently()
            .is_some()
    );

    // The repaired item reaches A, which shows it again.
    server.upload(&mut b);
    a.apply_remote_record(server.log.last().unwrap()).unwrap();
    assert!(!a.item(vault, item).unwrap().is_deleted());
    assert_eq!(password_of(&a, vault, item), "edited");
    assert_eq!(a.search("example").len(), 1);
}

#[test]
fn a_moved_item_reaches_a_device_that_edited_it() {
    // Regression: the moved copy carries the source's version vector, so a
    // device that edited the item sees its own counter on a record it has
    // never stored. That is not a reused device ID.
    let (mut a, created) = new_account();
    let from = a.create_vault("Personal").unwrap();
    let to = a.create_vault("Work").unwrap();
    let item = login(&mut a, from, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();
    b.update_item(from, item, |e| e.set_secret(SecretField::Password, "v2"))
        .unwrap();
    server.upload(&mut b);
    a.apply_remote_record(server.log.last().unwrap()).unwrap();

    let moved = a.move_item(from, item, to).unwrap();
    let start = server.log.len();
    server.upload(&mut a);
    for r in &server.log[start..] {
        b.apply_remote_record(r).unwrap();
    }
    assert!(b.item(from, item).unwrap().is_deleted());
    assert_eq!(password_of(&b, to, moved), "v2");
    assert_eq!(b.item(to, moved).unwrap(), a.item(to, moved).unwrap());
}

#[test]
fn vault_metadata_merges_and_keys_arrive_for_new_vaults() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let meta_v1 = a.store().vault_meta(&vault).unwrap().unwrap();
    assert_eq!(
        b.apply_remote_vault_meta(vault, &meta_v1).unwrap(),
        ApplyOutcome::Ignored
    );

    a.rename_vault(vault, "Home").unwrap();
    let meta_v2 = a.store().vault_meta(&vault).unwrap().unwrap();
    assert_eq!(
        b.apply_remote_vault_meta(vault, &meta_v2).unwrap(),
        ApplyOutcome::FastForward
    );
    assert_eq!(b.vaults()[0].name, Some("Home"));
    assert_eq!(
        b.apply_remote_vault_meta(vault, &meta_v1).unwrap(),
        ApplyOutcome::Ignored
    );

    // Concurrent renames keep both names (one as a conflict).
    a.rename_vault(vault, "From A").unwrap();
    b.rename_vault(vault, "From B").unwrap();
    let from_a = a.store().vault_meta(&vault).unwrap().unwrap();
    assert!(matches!(
        b.apply_remote_vault_meta(vault, &from_a).unwrap(),
        ApplyOutcome::Merged { .. }
    ));
    let meta = b.vault_meta(vault).unwrap().unwrap();
    assert_eq!(meta.conflicts().len(), 1);
    let from_b = b.store().vault_meta(&vault).unwrap().unwrap();
    a.apply_remote_vault_meta(vault, &from_b).unwrap();
    assert_eq!(a.vault_meta(vault).unwrap(), b.vault_meta(vault).unwrap());

    // Metadata sealed under a key generation that is not accepted.
    let vk2 = VaultKey::generate(KeyGeneration::new(2).unwrap()).unwrap();
    let forged = VaultMeta::new(
        "x",
        DeviceId::new_v7().unwrap(),
        mosaic_core::Timestamp::from_unix_millis(0),
    )
    .unwrap()
    .seal(&vk2, &vault)
    .unwrap();
    assert!(matches!(
        b.apply_remote_vault_meta(vault, &forged),
        Err(Error::KeyGenerationNotAccepted { .. })
    ));

    // A vault created on A later reaches B through its wrapped key.
    let work = a.create_vault("Work").unwrap();
    let (g, wrapped) = a
        .store()
        .vault_keys(&work)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    b.apply_remote_vault_key(work, g, &wrapped).unwrap();
    assert!(b.vaults().iter().any(|v| v.vault_id == work && v.available));
    // Its name arrives with its metadata.
    let meta = a.store().vault_meta(&work).unwrap().unwrap();
    assert_eq!(
        b.apply_remote_vault_meta(work, &meta).unwrap(),
        ApplyOutcome::New
    );
    assert!(b.vaults().iter().any(|v| v.name == Some("Work")));
}

#[test]
fn join_checks_the_password_before_storing_anything() {
    let (a, created) = new_account();
    let params = a.store().account().unwrap().unwrap();
    let wrapped = a
        .store()
        .unlock_method(&mosaic_core::UnlockMethod::Password)
        .unwrap()
        .unwrap()
        .wrapped_account_key;
    let err = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account(&params, &wrapped, &pw("wrong"), &created.secret_key)
        .unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    assert!(!err.session.has_account().unwrap());
    let b = err
        .session
        .join_account(&params, &wrapped, &pw(PASSWORD), &created.secret_key)
        .unwrap();
    assert_eq!(b.account_id(), a.account_id());
    assert_ne!(b.device_id(), a.device_id());
}

#[test]
fn concurrent_rotations_are_refused_and_nothing_becomes_unreadable() {
    // Review M2 (crypto, sync): two devices rotate one vault at the same
    // time and create two keys for generation 2. Accepting the other key
    // used to replace the held one, which made every record sealed under
    // it unreadable for good.
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();

    let g2 = a.rotate_vault_key(vault).unwrap();
    assert_eq!(b.rotate_vault_key(vault).unwrap(), g2);
    b.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "only-on-B")
    })
    .unwrap();
    let key_of = |s: &UnlockedSession, g| s.store().vault_key(&vault, g).unwrap().unwrap();
    let (a_key, b_key) = (key_of(&a, g2), key_of(&b, g2));
    assert_ne!(a_key, b_key);

    // Each side refuses the other's key and keeps its own.
    assert!(matches!(
        b.apply_remote_vault_key(vault, g2, &a_key),
        Err(Error::VaultKeyConflict { generation: 2 })
    ));
    assert!(matches!(
        a.apply_remote_vault_key(vault, g2, &b_key),
        Err(Error::VaultKeyConflict { generation: 2 })
    ));
    assert_eq!(key_of(&b, g2), b_key);
    assert_eq!(key_of(&a, g2), a_key);
    b.apply_remote_vault_key(vault, g2, &b_key).unwrap();

    // B's records under its own key do not authenticate at A and change
    // nothing there.
    let start = server.log.len();
    server.upload(&mut b);
    for r in &server.log[start..] {
        assert!(matches!(
            a.apply_remote_record(r),
            Err(Error::Core(mosaic_core::Error::ItemCorrupted { item_id }))
                if item_id == r.record_id
        ));
    }
    assert_eq!(password_of(&a, vault, item), "v1");
    assert_eq!(a.corrupt_records().len(), 0);
    // Nothing became unreadable on B.
    let mut b = b.lock().unlock(&pw(PASSWORD), &created.secret_key).unwrap();
    assert_eq!(b.corrupt_records().len(), 0);
    assert_eq!(password_of(&b, vault, item), "only-on-B");

    // Resolution (ADR-0013): one side rotates again, past both keys; the
    // other accepts that key, re-seals what it holds, and both converge.
    let g3 = b.rotate_vault_key(vault).unwrap();
    assert_eq!(g3.get(), 3);
    a.apply_remote_vault_key(vault, g3, &key_of(&b, g3))
        .unwrap();
    let start = server.log.len();
    server.upload(&mut b);
    let from_b = server.log[start..].to_vec();
    let start = server.log.len();
    server.upload(&mut a);
    let from_a = server.log[start..].to_vec();
    assert!(
        from_a
            .iter()
            .chain(&from_b)
            .all(|r| r.body.key_generation() == g3)
    );
    for r in &from_b {
        a.apply_remote_record(r).unwrap();
    }
    for r in &from_a {
        b.apply_remote_record(r).unwrap();
    }
    assert_eq!(password_of(&a, vault, item), "only-on-B");
    assert_eq!(a.item(vault, item).unwrap(), b.item(vault, item).unwrap());
}

#[test]
fn accepting_a_rotation_reseals_records_the_rotating_device_never_held() {
    // Review M2 (crypto): only pending records used to be re-sealed when a
    // newer key arrived, so a record uploaded under the old key that the
    // rotating device never received stayed visible on its author only.
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    let item = login(&mut b, vault, "From B", "b-1");
    server.upload(&mut b);
    assert_eq!(b.pending_uploads(10).unwrap().len(), 0);
    let old = server.log.last().unwrap().clone();

    let g2 = a.rotate_vault_key(vault).unwrap();
    assert!(matches!(
        a.apply_remote_record(&old),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    let wrapped = a.store().vault_key(&vault, g2).unwrap().unwrap();
    b.apply_remote_vault_key(vault, g2, &wrapped).unwrap();
    // Re-sealed and queued, although it was not a pending change, and the
    // metadata follows.
    let pending = b.pending_uploads(10).unwrap();
    assert_eq!(pending.len(), 1);
    let queued = pending[0].as_ref().unwrap();
    assert_eq!(queued.record_id, item);
    assert_eq!(queued.body.key_generation(), g2);
    assert_eq!(
        b.store()
            .vault_meta(&vault)
            .unwrap()
            .unwrap()
            .key_generation(),
        g2
    );
    let start = server.log.len();
    server.upload(&mut b);
    let resealed = server.log[start].clone();
    assert_eq!(a.apply_remote_record(&resealed).unwrap(), ApplyOutcome::New);
    assert_eq!(password_of(&a, vault, item), "b-1");
    // A device that joins later gets it too.
    let mut c = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    assert_eq!(c.apply_remote_record(&resealed).unwrap(), ApplyOutcome::New);
    assert_eq!(password_of(&c, vault, item), "b-1");
}

#[test]
fn an_equal_version_resealed_by_a_rotation_is_ignored() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Example", "v1");
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();

    // A rotates without editing; its record is the same version, re-sealed.
    let g2 = a.rotate_vault_key(vault).unwrap();
    let start = server.log.len();
    server.upload(&mut a);
    let rotated = server.log[start].clone();
    assert_eq!(rotated.body.key_generation(), g2);
    let wrapped = a.store().vault_key(&vault, g2).unwrap().unwrap();
    b.apply_remote_vault_key(vault, g2, &wrapped).unwrap();
    assert_eq!(
        b.store()
            .record(&vault, &item)
            .unwrap()
            .unwrap()
            .body
            .key_generation(),
        g2
    );
    let queued = b.pending_uploads(usize::MAX).unwrap().len();
    assert_eq!(
        b.apply_remote_record(&rotated).unwrap(),
        ApplyOutcome::Ignored
    );
    assert_eq!(b.pending_uploads(usize::MAX).unwrap().len(), queued);
    assert_eq!(b.item(vault, item).unwrap(), a.item(vault, item).unwrap());
}

#[test]
fn hotp_codes_on_two_offline_devices() {
    // The counter merges by maximum: two devices offline at counter 1 both
    // show the counter-1 code, and after sync the counter is 2, not 3.
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let mut hotp = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
    hotp.kind = OtpKind::Hotp;
    let item = a
        .add_item(vault, &ItemType::Totp, |e| {
            e.set_otp(&hotp)?;
            e.set_otp_counter(1)
        })
        .unwrap();
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();

    let on_a = a.otp_code(vault, item, 0).unwrap();
    let on_b = b.otp_code(vault, item, 0).unwrap();
    assert_eq!(on_a.code.expose_secret(), "287082");
    assert_eq!(on_b.code.expose_secret(), on_a.code.expose_secret());
    server.upload(&mut a);
    let from_a = server.log.last().unwrap().clone();
    server.upload(&mut b);
    let from_b = server.log.last().unwrap().clone();
    a.apply_remote_record(&from_b).unwrap();
    b.apply_remote_record(&from_a).unwrap();
    assert_eq!(a.item(vault, item).unwrap(), b.item(vault, item).unwrap());
    assert_eq!(a.item(vault, item).unwrap().otp_counter(), 2);
    assert_eq!(
        a.otp_code(vault, item, 0).unwrap().code.expose_secret(),
        "359152"
    );
}

fn attachment_files(store: &Store) -> usize {
    std::fs::read_dir(store.attachment_dir().unwrap())
        .unwrap()
        .count()
}

#[test]
fn attachment_quota_and_a_failed_reference_leave_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = new_account_in(Store::open(dir.path().join("vault.db")).unwrap());
    let vault = s.create_vault("Personal").unwrap();
    let item = s
        .add_item(vault, &ItemType::Document, |e| e.set_title("Scans"))
        .unwrap();
    // References to files held elsewhere (none here) count against the
    // quota: twenty of 50 MiB and one that leaves two bytes free.
    let max = MAX_ACCOUNT_ATTACHMENT_BYTES;
    let each = u64::try_from(MAX_ATTACHMENT_LEN).unwrap();
    let sizes: Vec<u64> = (0..20).map(|_| each).chain([max - 20 * each - 2]).collect();
    s.update_item(vault, item, |e| {
        for size in &sizes {
            e.set_attachment(&AttachmentRef::new(
                AttachmentId::new_v7()?,
                Text::new("remote.bin"),
                Text::new("application/octet-stream"),
                *size,
                AttachmentKey::generate()?,
            ))?;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(s.attachment_usage(), max - 2);

    let files = attachment_files(s.store());
    let version = s.item(vault, item).unwrap().version().clone();
    assert!(matches!(
        s.add_attachment(vault, item, "c.txt", "text/plain", b"abc"),
        Err(Error::AttachmentQuota)
    ));
    assert_eq!(s.attachment_usage(), max - 2);
    assert_eq!(attachment_files(s.store()), files);
    assert_eq!(s.item(vault, item).unwrap().version(), &version);
    // Up to the quota exactly is fine.
    let fits = s
        .add_attachment(vault, item, "b.txt", "text/plain", b"ab")
        .unwrap();
    assert_eq!(s.attachment_usage(), max);
    assert_eq!(attachment_files(s.store()), files + 1);
    s.remove_attachment(vault, item, fits).unwrap();
    assert_eq!(attachment_files(s.store()), files);

    // The file is stored first; when the reference cannot be added (here
    // the item would exceed the record size limit), the file goes again.
    let version = s.item(vault, item).unwrap().version().clone();
    let uploads = s.store().pending_attachment_uploads().unwrap().len();
    let huge_name = "n".repeat(1024 * 1024);
    assert!(matches!(
        s.add_attachment(vault, item, &huge_name, "text/plain", b"x"),
        Err(Error::Core(mosaic_core::Error::SizeLimit))
    ));
    assert_eq!(attachment_files(s.store()), files);
    assert_eq!(
        s.store().pending_attachment_uploads().unwrap().len(),
        uploads
    );
    assert_eq!(
        s.store()
            .attachments_for_record(&vault, &item)
            .unwrap()
            .len(),
        0
    );
    assert_eq!(s.item(vault, item).unwrap().version(), &version);
}

#[test]
fn two_sessions_on_one_store_cannot_lose_an_edit() {
    // Review M2 (sync, t1): two processes unlocked one store file, each
    // edited from its own stale cache, and the second write replaced the
    // first one's password with no conflict and no history. A store is
    // now opened by one session at a time (ADR-0010 item 10).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut p1, created) = new_account_in(Store::open(&path).unwrap());
    let vault = p1.create_vault("Personal").unwrap();
    let item = login(&mut p1, vault, "Title", "old");
    // The second process cannot open the store, so it cannot edit.
    assert!(matches!(Store::open(&path), Err(StorageError::StoreInUse)));
    p1.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "NEW-from-p1")
    })
    .unwrap();
    // A locked session still owns the store.
    let locked = p1.lock();
    assert!(matches!(Store::open(&path), Err(StorageError::StoreInUse)));
    drop(locked);
    let mut p2 = LockedSession::open(Store::open(&path).unwrap())
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap();
    p2.update_item(vault, item, |e| e.set_title("title from p2"))
        .unwrap();
    let item_now = p2.item(vault, item).unwrap();
    assert_eq!(item_now.title(), Some("title from p2"));
    assert_eq!(password_of(&p2, vault, item), "NEW-from-p1");
    assert_eq!(item_now.password_history().len(), 1);
}

#[test]
fn a_copied_store_never_writes_under_the_original_writer_id() {
    // Review M2, third round (sync, high): a store restored from a backup
    // or copied to another profile used to keep its device ID, and two
    // writers with one ID that edit different fields of an item lost both
    // edits silently (and the documented recovery lost them too). Every
    // unlock now writes under a fresh ID (ADR-0015), so the copy and the
    // original are two writers like any others and nothing is lost.
    let dir = tempfile::tempdir().unwrap();
    let original_path = dir.path().join("vault.db");
    let (mut a, created) = new_account_in(Store::open(&original_path).unwrap());
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "t0", "p0");
    let mut server = Server::default();
    server.upload(&mut a);
    let store = a.lock().into_store();
    store.close().unwrap();
    let copy_path = dir.path().join("copy.db");
    std::fs::copy(&original_path, &copy_path).unwrap();

    let mut a = LockedSession::open(Store::open(&original_path).unwrap())
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap();
    let mut b = LockedSession::open(Store::open(&copy_path).unwrap())
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap();
    assert_ne!(a.device_id(), b.device_id());
    a.update_item(vault, item, |e| e.set_secret(SecretField::Password, "p1"))
        .unwrap();
    b.update_item(vault, item, |e| e.set_title("t1")).unwrap();
    let from_a = server.log.len();
    server.upload(&mut a);
    let from_b = server.log.len();
    server.upload(&mut b);
    let rec_a = server.log[from_a].clone();
    let rec_b = server.log[from_b].clone();
    assert!(matches!(
        a.apply_remote_record(&rec_b).unwrap(),
        ApplyOutcome::Merged {
            stamp_collision: false,
            ..
        }
    ));
    assert!(matches!(
        b.apply_remote_record(&rec_a).unwrap(),
        ApplyOutcome::Merged {
            stamp_collision: false,
            ..
        }
    ));
    for s in [&a, &b] {
        let it = s.item(vault, item).unwrap();
        assert_eq!(it.title(), Some("t1"));
        assert_eq!(password_of(s, vault, item), "p1");
        assert!(it.conflicts().is_empty());
    }
    assert_eq!(a.item(vault, item).unwrap(), b.item(vault, item).unwrap());
}

/// The one conflict copy of `item` a session holds.
fn copy_of(s: &UnlockedSession, vault: mosaic_core::VaultId, item: ItemId) -> ItemId {
    let copies: Vec<ItemId> = s
        .items(&ItemFilter::default())
        .iter()
        .filter(|r| r.vault_id == vault && r.item.conflict_copy_of() == Some(item))
        .map(|r| r.item_id)
        .collect();
    assert_eq!(copies.len(), 1, "{copies:?}");
    copies[0]
}

/// Delivers everything each device has queued to every device, until
/// nothing is queued any more (at most `rounds` rounds).
fn exchange_until_quiet(server: &mut Server, devices: &mut [&mut UnlockedSession], rounds: usize) {
    for _ in 0..rounds {
        let start = server.log.len();
        for d in devices.iter_mut() {
            server.upload(d);
        }
        if server.log.len() == start {
            return;
        }
        let delivered: Vec<RemoteRecord> = server.log[start..].to_vec();
        for rec in &delivered {
            for d in devices.iter_mut() {
                d.apply_remote_record(rec).unwrap();
            }
        }
    }
    panic!("still not quiet after {rounds} rounds");
}

#[test]
fn concurrent_edits_of_a_large_note_converge_through_a_split() {
    // Review M2, third round (sync, medium): each device's version of a
    // 600 kB note fits the item limit, their merge does not. Before the
    // split rule `apply_remote_record` failed with `SizeLimit` on both
    // devices forever and neither stored the other's version.
    //
    // Review M2, fourth round: each device adds an attachment before the
    // large edit (the split merges the references of both, and no file may
    // be pruned), and both devices make the copy of the version not kept,
    // under the same derived ID.
    let dir = tempfile::tempdir().unwrap();
    let (mut a, created) = new_account_in(Store::open(dir.path().join("a.db")).unwrap());
    let vault = a.create_vault("Personal").unwrap();
    let big = "n".repeat(600_000);
    let item = a
        .add_item(vault, &ItemType::SecureNote, |e| {
            e.set_title("Large")?;
            e.set_text(TextField::Notes, &big)
        })
        .unwrap();
    let mut b = join(
        &a,
        &created.secret_key,
        Store::open(dir.path().join("b.db")).unwrap(),
    );
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();
    let att_a = a
        .add_attachment(vault, item, "a.txt", "text/plain", b"from a")
        .unwrap();
    let att_b = b
        .add_attachment(vault, item, "b.txt", "text/plain", b"from b")
        .unwrap();
    a.update_item(vault, item, |e| {
        e.set_text(TextField::Notes, &format!("{big}a"))
    })
    .unwrap();
    b.update_item(vault, item, |e| {
        e.set_text(TextField::Notes, &format!("{big}b"))
    })
    .unwrap();
    let from_a = server.log.len();
    server.upload(&mut a);
    let from_b = server.log.len();
    server.upload(&mut b);
    let (rec_a, rec_b) = (server.log[from_a].clone(), server.log[from_b].clone());

    assert_eq!(
        a.apply_remote_record(&rec_b).unwrap(),
        ApplyOutcome::Split { conflict_copies: 1 }
    );
    assert_eq!(
        b.apply_remote_record(&rec_a).unwrap(),
        ApplyOutcome::Split { conflict_copies: 1 }
    );
    // Both devices save the value not kept, under one derived ID.
    let copy = copy_of(&a, vault, item);
    assert_eq!(copy_of(&b, vault, item), copy);
    assert_eq!(a.item(vault, copy).unwrap(), b.item(vault, copy).unwrap());
    let notes = |s: &UnlockedSession, id: ItemId| {
        s.item(vault, id)
            .unwrap()
            .text(TextField::Notes)
            .unwrap()
            .to_owned()
    };
    assert_eq!(notes(&a, item), notes(&b, item));

    // Exchanging what was queued converges both devices: the same record,
    // and one conflict copy, the same on both (built identically, so the
    // exchange changes nothing and queues nothing more).
    exchange_until_quiet(&mut server, &mut [&mut a, &mut b], 2);
    assert_eq!(a.item(vault, item).unwrap(), b.item(vault, item).unwrap());
    assert_eq!(a.item(vault, copy).unwrap(), b.item(vault, copy).unwrap());
    assert_eq!(a.item(vault, copy).unwrap().conflict_copy_of(), Some(item));
    assert!(a.item(vault, copy).unwrap().attachments().is_empty());
    let mut both = [notes(&a, item), notes(&a, copy)];
    both.sort();
    assert_eq!(both, [format!("{big}a"), format!("{big}b")]);
    // Both attachments are referenced on both devices, the usage agrees,
    // and each device still holds the file it added (nothing was pruned).
    for s in [&a, &b] {
        let ids: Vec<_> = s
            .item(vault, item)
            .unwrap()
            .attachments()
            .iter()
            .map(|r| r.attachment_id)
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&att_a) && ids.contains(&att_b));
    }
    assert_eq!(a.attachment_usage(), b.attachment_usage());
    assert_eq!(
        a.attachment(vault, item, att_a).unwrap().expose_secret(),
        b"from a"
    );
    assert_eq!(
        b.attachment(vault, item, att_b).unwrap().expose_secret(),
        b"from b"
    );
    // Nothing is left to upload or merge; a replay changes nothing.
    assert_eq!(a.pending_uploads(usize::MAX).unwrap().len(), 0);
    assert_eq!(b.pending_uploads(usize::MAX).unwrap().len(), 0);
    assert_eq!(
        a.apply_remote_record(&rec_b).unwrap(),
        ApplyOutcome::Ignored
    );
    assert_eq!(
        b.apply_remote_record(&rec_a).unwrap(),
        ApplyOutcome::Ignored
    );
}

#[test]
fn a_split_record_reaching_the_losing_device_first_loses_nothing() {
    // Review M2, fourth round (sync, high): A's version is longer, so A's
    // split keeps it. Before, A then made no copy; B pulled A's split
    // record first (the server's newest), fast-forwarded to it because it
    // dominates B's version, ignored A's original record later, and B's
    // committed note was on no device. A now saves B's version as the
    // conflict copy, and B receives it.
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let big = "n".repeat(600_000);
    let item = a
        .add_item(vault, &ItemType::SecureNote, |e| {
            e.set_text(TextField::Notes, &big)
        })
        .unwrap();
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut server = Server::default();
    server.upload(&mut a);
    b.apply_remote_record(&server.log[0]).unwrap();
    let note_a = format!("{big}{}", "A".repeat(200));
    let note_b = format!("{big}b");
    a.update_item(vault, item, |e| e.set_text(TextField::Notes, &note_a))
        .unwrap();
    b.update_item(vault, item, |e| e.set_text(TextField::Notes, &note_b))
        .unwrap();
    let from_a = server.log.len();
    server.upload(&mut a);
    let from_b = server.log.len();
    server.upload(&mut b);
    let (rec_a, rec_b) = (server.log[from_a].clone(), server.log[from_b].clone());

    assert_eq!(
        a.apply_remote_record(&rec_b).unwrap(),
        ApplyOutcome::Split { conflict_copies: 1 }
    );
    let copy = copy_of(&a, vault, item);
    let start = server.log.len();
    server.upload(&mut a);
    let from_a_split: Vec<RemoteRecord> = server.log[start..].to_vec();
    let split = from_a_split
        .iter()
        .find(|r| r.record_id == item)
        .unwrap()
        .clone();
    // B gets the split record before A's original one.
    assert_eq!(
        b.apply_remote_record(&split).unwrap(),
        ApplyOutcome::FastForward
    );
    assert_eq!(
        b.apply_remote_record(&rec_a).unwrap(),
        ApplyOutcome::Ignored
    );
    for rec in from_a_split.iter().filter(|r| r.record_id == copy) {
        assert_eq!(b.apply_remote_record(rec).unwrap(), ApplyOutcome::New);
    }
    let note = |s: &UnlockedSession, id: ItemId| {
        s.item(vault, id)
            .unwrap()
            .text(TextField::Notes)
            .unwrap()
            .to_owned()
    };
    for s in [&a, &b] {
        let mut both = [note(s, item), note(s, copy)];
        both.sort();
        let mut expected = [note_a.clone(), note_b.clone()];
        expected.sort();
        assert_eq!(both, expected, "both committed edits survive");
    }
}

#[test]
fn received_removals_prune_attachment_files() {
    // Review M2, third round (tests): applying a received record removes
    // the files (and rows) of attachments the result no longer references,
    // on the stored-as-received path (New / FastForward) and on the merged
    // path. Both prune calls survived mutation before this test.
    for merged in [false, true] {
        for delete_item in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let (mut b, created) = new_account_in(Store::open(dir.path().join("b.db")).unwrap());
            let vault = b.create_vault("Personal").unwrap();
            let item = login(&mut b, vault, "Example", "pw");
            let gone = b
                .add_attachment(vault, item, "gone.txt", "text/plain", b"gone")
                .unwrap();
            let kept = b
                .add_attachment(vault, item, "kept.txt", "text/plain", b"kept")
                .unwrap();
            let mut a = join(
                &b,
                &created.secret_key,
                Store::open(dir.path().join("a.db")).unwrap(),
            );
            let mut server = Server::default();
            server.upload(&mut b);
            for rec in server.log.clone() {
                a.apply_remote_record(&rec).unwrap();
            }
            if delete_item {
                a.purge_item(vault, item).unwrap();
            } else {
                a.remove_attachment(vault, item, gone).unwrap();
            }
            if merged {
                b.update_item(vault, item, |e| e.set_title("edited on B"))
                    .unwrap();
            }
            let start = server.log.len();
            server.upload(&mut a);
            let rec = server.log[start..].last().unwrap().clone();
            let outcome = b.apply_remote_record(&rec).unwrap();
            let case = format!("merged={merged} delete={delete_item}");
            let stored: Vec<AttachmentId> = b
                .store()
                .attachments_for_record(&vault, &item)
                .unwrap()
                .into_iter()
                .map(|i| i.attachment_id)
                .collect();
            match (merged, delete_item) {
                (false, false) => {
                    assert_eq!(outcome, ApplyOutcome::FastForward, "{case}");
                    assert_eq!(stored, [kept], "{case}");
                }
                (false, true) => {
                    assert_eq!(outcome, ApplyOutcome::FastForward, "{case}");
                    assert!(stored.is_empty(), "{case}");
                    assert!(matches!(
                        b.store().attachment(&kept),
                        Err(StorageError::UnknownAttachment)
                    ));
                }
                (true, false) => {
                    assert!(matches!(outcome, ApplyOutcome::Merged { .. }), "{case}");
                    assert_eq!(stored, [kept], "{case}");
                }
                (true, true) => {
                    // The delete loses against B's concurrent edit, and the
                    // files the repaired item references stay.
                    assert!(
                        matches!(outcome, ApplyOutcome::Merged { restored_fields, .. } if restored_fields > 0),
                        "{case}"
                    );
                    let mut both = vec![gone, kept];
                    both.sort();
                    assert_eq!(stored, both, "{case}");
                    assert_eq!(
                        b.attachment(vault, item, gone).unwrap().expose_secret(),
                        b"gone"
                    );
                }
            }
            if !(merged && delete_item) {
                assert!(matches!(
                    b.store().attachment(&gone),
                    Err(StorageError::UnknownAttachment)
                ));
            }
            // No file without a row is left behind either.
            assert_eq!(attachment_files(b.store()), stored.len(), "{case}");
        }
    }
}

#[test]
fn a_tampered_or_swapped_attachment_file_fails_to_open() {
    // Review M2, third round (tests): the attachment AAD binds vault, item
    // and attachment ID end to end. Item B's file replaced by item A's
    // ciphertext (same size), or one flipped byte, fails with
    // `Decryption`; the items and the other attachment stay readable.
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = new_account_in(Store::open(dir.path().join("vault.db")).unwrap());
    let vault = s.create_vault("Personal").unwrap();
    let item_a = login(&mut s, vault, "A", "pw-a");
    let item_b = login(&mut s, vault, "B", "pw-b");
    let att_a = s
        .add_attachment(vault, item_a, "a.txt", "text/plain", b"contents of A")
        .unwrap();
    let att_b = s
        .add_attachment(vault, item_b, "b.txt", "text/plain", b"contents of B")
        .unwrap();
    let file = |id: AttachmentId| {
        let hex = id.as_bytes().iter().fold(String::new(), |mut hex, b| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02x}");
            hex
        });
        s.store().attachment_dir().unwrap().join(hex)
    };
    let (path_a, path_b) = (file(att_a), file(att_b));
    let original_b = std::fs::read(&path_b).unwrap();
    let bytes_a = std::fs::read(&path_a).unwrap();
    assert_eq!(bytes_a.len(), original_b.len());

    std::fs::write(&path_b, &bytes_a).unwrap();
    assert!(matches!(
        s.attachment(vault, item_b, att_b),
        Err(Error::Core(mosaic_core::Error::Decryption))
    ));
    let mut flipped = original_b.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0x01;
    std::fs::write(&path_b, &flipped).unwrap();
    assert!(matches!(
        s.attachment(vault, item_b, att_b),
        Err(Error::Core(mosaic_core::Error::Decryption))
    ));
    // Everything else is unaffected, and the intact file opens again.
    assert_eq!(
        s.attachment(vault, item_a, att_a).unwrap().expose_secret(),
        b"contents of A"
    );
    assert_eq!(password_of(&s, vault, item_b), "pw-b");
    std::fs::write(&path_b, &original_b).unwrap();
    assert_eq!(
        s.attachment(vault, item_b, att_b).unwrap().expose_secret(),
        b"contents of B"
    );
}

#[test]
fn a_record_in_flight_during_a_rotation_is_refused_by_peers() {
    // Review M2 (sync, t4), accepted risk M2-R12 for M6: B's edit reaches
    // the server under generation 1, A rotates to generation 2 before
    // pulling it, and B never comes back. Every peer that accepts the new
    // key before B's record refuses that record as a rollback, so only a
    // device that applied it in time (and re-sealed it on accepting the
    // key) still carries it. The M6 protocol makes rotations conditional
    // on the vault head (PROGRESS M2-R12).
    let (mut a, created) = new_account();
    let vault = a.create_vault("Personal").unwrap();
    let item = login(&mut a, vault, "Shared", "v1");
    let mut server = Server::default();
    server.upload(&mut a);
    let mut b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut c = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    let mut late = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
    for s in [&mut b, &mut c, &mut late] {
        s.apply_remote_record(&server.log[0]).unwrap();
    }
    b.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "b-edit")
    })
    .unwrap();
    server.upload(&mut b);
    let in_flight = server.log.last().unwrap().clone();
    drop(b);

    let g2 = a.rotate_vault_key(vault).unwrap();
    let wrapped = a.store().vault_key(&vault, g2).unwrap().unwrap();
    // C gets the key first: B's record is now a rollback and is dropped.
    c.apply_remote_vault_key(vault, g2, &wrapped).unwrap();
    assert!(matches!(
        c.apply_remote_record(&in_flight),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    assert_eq!(password_of(&c, vault, item), "v1");
    // A device that gets the record first keeps it and re-seals it.
    late.apply_remote_record(&in_flight).unwrap();
    late.apply_remote_vault_key(vault, g2, &wrapped).unwrap();
    assert_eq!(password_of(&late, vault, item), "b-edit");
    assert!(
        late.pending_uploads(10)
            .unwrap()
            .iter()
            .any(|r| r.as_ref().unwrap().record_id == item)
    );
}

#[test]
fn characterization_a_move_into_a_concurrently_rotated_vault_vanishes_on_the_rotator() {
    // Review M2, fourth round (sync, medium), accepted as part of M2-R12
    // for M6 (PROGRESS): D1 moves X from vault A to vault B while D2
    // rotates B to generation 2. D2 refuses the copy, sealed under B's
    // generation 1, as a rollback; the source's tombstone in A dominates
    // D2's version and fast-forwards, so D2 holds no live X and shows no
    // notice. X comes back on D2 only once D1 accepts generation 2 and
    // re-seals the copy. The M6 fix: upload a move's tombstone only after
    // the server accepted the copy, and flag the source instead of
    // fast-forwarding when the target copy cannot be read. When that
    // lands, this test flips.
    let (mut d1, created) = new_account();
    let a = d1.create_vault("A").unwrap();
    let b = d1.create_vault("B").unwrap();
    let x = login(&mut d1, a, "Moved", "pw");
    let mut server = Server::default();
    server.upload(&mut d1);
    let mut d2 = join(&d1, &created.secret_key, Store::open_in_memory().unwrap());
    for rec in server.log.clone() {
        d2.apply_remote_record(&rec).unwrap();
    }
    let start = server.log.len();
    let copy = d1.move_item(a, x, b).unwrap();
    server.upload(&mut d1);
    let moved: Vec<RemoteRecord> = server.log[start..].to_vec();
    let g2 = d2.rotate_vault_key(b).unwrap();
    assert_eq!(g2.get(), 2);
    let mut refused = 0;
    for rec in &moved {
        match d2.apply_remote_record(rec) {
            Err(Error::KeyGenerationRollback {
                highest_accepted: 2,
                offered: 1,
            }) => {
                assert_eq!(rec.record_id, copy);
                refused += 1;
            }
            Ok(outcome) => {
                assert_eq!(rec.record_id, x);
                assert_eq!(outcome, ApplyOutcome::FastForward);
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert_eq!(refused, 1);
    let live = d2.items(&ItemFilter::default());
    assert!(live.is_empty(), "M2-R12: the moved item is gone on D2");
    assert!(d2.item(a, x).unwrap().is_deleted());
    assert!(d2.item(a, x).unwrap().deleted_concurrently().is_none());

    // Once D1 accepts generation 2 it re-seals the copy, and D2 gets it.
    let wrapped = d2.store().vault_key(&b, g2).unwrap().unwrap();
    d1.apply_remote_vault_key(b, g2, &wrapped).unwrap();
    let start = server.log.len();
    server.upload(&mut d1);
    for rec in &server.log[start..] {
        d2.apply_remote_record(rec).unwrap();
    }
    assert_eq!(d2.item(b, copy).unwrap().title(), Some("Moved"));
}
