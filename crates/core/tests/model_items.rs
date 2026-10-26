//! Behavior of the item model through the public API: typed fields,
//! password history, collections, conflicts and their resolution,
//! deletion, forward compatibility, and the encryption glue.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    reason = "test helpers outside #[test] functions"
)]

use mosaic_core::model::{
    AttachmentRef, Causality, CustomField, CustomFieldValue, ElementId, Item, ItemType, LoginUrl,
    MAX_HISTORY, MAX_TAG_LEN, OtpConfig, OtpKind, PasskeyCredential, SecretField, SshKeyMaterial,
    Text, TextField, Timestamp, Tombstone, UrlMatch,
};
use mosaic_core::{
    AttachmentId, AttachmentKey, DeviceId, Error, ItemCiphertext, ItemId, KeyGeneration,
    SecretBytes, SecretString, SecretVec, TombstoneCiphertext, VaultId, VaultKey,
};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 16]).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn copy(item: &Item) -> Item {
    Item::from_json(item.to_json().unwrap().expose_secret()).unwrap()
}

fn login() -> Item {
    let mut item = Item::new(&ItemType::Login, dev(1), at(100)).unwrap();
    let mut e = item.edit(dev(1), at(100));
    e.set_title("GitHub").unwrap();
    e.set_text(TextField::Username, "octocat").unwrap();
    e.set_secret(SecretField::Password, "pw-1").unwrap();
    item
}

fn password(item: &Item) -> String {
    item.secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

fn json_string(item: &Item) -> String {
    String::from_utf8(item.to_json().unwrap().expose_secret().to_vec()).unwrap()
}

#[test]
fn new_item_and_text_fields() {
    let mut item = Item::new(&ItemType::Identity, dev(1), at(5)).unwrap();
    assert_eq!(item.item_type(), Some(ItemType::Identity));
    assert_eq!(item.created_at(), Some(at(5)));
    assert_eq!(item.modified_at(), Some(at(5)));
    assert_eq!(item.version().get(&dev(1)), 1);
    for (i, f) in TextField::ALL.iter().enumerate() {
        item.edit(dev(1), at(6))
            .set_text(*f, &format!("v{i}"))
            .unwrap();
    }
    for (i, f) in TextField::ALL.iter().enumerate() {
        assert_eq!(item.text(*f), Some(format!("v{i}").as_str()));
        assert_eq!(TextField::from_key(f.key()), Some(*f));
    }
    for f in SecretField::ALL {
        item.edit(dev(1), at(7)).set_secret(*f, f.key()).unwrap();
        assert_eq!(item.secret(*f).unwrap().unwrap().expose_secret(), f.key());
        assert_eq!(SecretField::from_key(f.key()), Some(*f));
    }
    assert_eq!(item.modified_at(), Some(at(7)));
    assert_eq!(item.title(), Some("v0"));
    // Unchanged writes are no-ops: no new sync event.
    let v = item.version().clone();
    {
        let mut e = item.edit(dev(2), at(8));
        e.set_title("v0").unwrap();
        e.set_secret(SecretField::Password, "login.password")
            .unwrap();
        e.clear_text(TextField::SshComment).unwrap();
        e.set_favorite(false).unwrap();
        e.set_favorite(false).unwrap();
    }
    assert_ne!(item.version(), &v, "favorite was absent, so it was written");
    let v = item.version().clone();
    item.edit(dev(2), at(9)).set_favorite(false).unwrap();
    item.edit(dev(2), at(9)).remove_tag("none").unwrap();
    assert_eq!(item.version(), &v);
    // Clearing.
    item.edit(dev(1), at(10))
        .clear_text(TextField::Notes)
        .unwrap();
    assert_eq!(item.text(TextField::Notes), None);
    item.edit(dev(1), at(10))
        .clear_secret(SecretField::CardNumber)
        .unwrap();
    assert!(item.secret(SecretField::CardNumber).unwrap().is_none());
    assert!(item.secret(SecretField::Password).unwrap().is_some());
    assert!(SecretField::Password.keeps_history());
    assert!(!SecretField::CardNumber.keeps_history());
    assert_eq!(TextField::from_key("nope"), None);
}

#[test]
fn one_edit_is_one_stamp() {
    let mut item = login();
    let before = item.version().get(&dev(2));
    {
        let mut e = item.edit(dev(2), at(200));
        e.set_title("a").unwrap();
        e.set_text(TextField::Notes, "b").unwrap();
        e.add_tag("x").unwrap();
    }
    assert_eq!(before, 0);
    let stamps: Vec<_> = ["title", "notes", "tag:x"]
        .iter()
        .map(|k| item.entries(k).next().unwrap().stamp())
        .collect();
    assert!(stamps.iter().all(|s| *s == stamps[0]));
    assert_eq!(item.version().get(&dev(2)), stamps[0].counter());
    assert!(format!("{:?}", item.edit(dev(2), at(1))).starts_with("ItemEdit"));
}

#[test]
fn item_types_round_trip() {
    for t in [
        ItemType::Login,
        ItemType::Passkey,
        ItemType::Totp,
        ItemType::SecureNote,
        ItemType::CreditCard,
        ItemType::Identity,
        ItemType::SshKey,
        ItemType::ApiKey,
        ItemType::Wifi,
        ItemType::Document,
        ItemType::Other("bank_account".into()),
    ] {
        assert_eq!(ItemType::parse(t.as_str()), t);
        let item = Item::new(&t, dev(1), at(1)).unwrap();
        assert_eq!(copy(&item).item_type(), Some(t));
    }
}

#[test]
fn password_history() {
    let mut item = login();
    item.edit(dev(1), at(200))
        .set_secret(SecretField::Password, "pw-2")
        .unwrap();
    item.edit(dev(1), at(300))
        .set_secret(SecretField::Password, "pw-3")
        .unwrap();
    let h = item.password_history();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].value.expose_secret(), "pw-2");
    assert_eq!(h[0].changed_at, at(300));
    assert_eq!(h[1].value.expose_secret(), "pw-1");
    assert_eq!(h[0].field, SecretField::Password);
    assert!(!format!("{:?}", h[0]).contains("pw-2"));
    // Two changes in one edit keep both old values.
    {
        let mut e = item.edit(dev(1), at(400));
        e.set_secret(SecretField::Password, "pw-4").unwrap();
        e.set_secret(SecretField::Password, "pw-5").unwrap();
    }
    assert_eq!(item.password_history().len(), 4);
    // Clearing keeps the old value too; fields without history keep none.
    item.edit(dev(1), at(500))
        .clear_secret(SecretField::Password)
        .unwrap();
    assert_eq!(item.password_history()[0].value.expose_secret(), "pw-5");
    item.edit(dev(1), at(500))
        .set_secret(SecretField::CardNumber, "4111")
        .unwrap();
    item.edit(dev(1), at(501))
        .set_secret(SecretField::CardNumber, "4222")
        .unwrap();
    assert_eq!(item.password_history().len(), 5);
    // Trimmed to MAX_HISTORY, oldest first.
    for i in 0..40 {
        item.edit(dev(1), at(1000 + i))
            .set_secret(SecretField::WifiPassword, &format!("w{i}"))
            .unwrap();
    }
    let h = item.password_history();
    assert_eq!(h.len(), MAX_HISTORY);
    assert_eq!(h[0].value.expose_secret(), "w38");
    item.edit(dev(1), at(2000))
        .clear_password_history()
        .unwrap();
    assert!(item.password_history().is_empty());
}

#[test]
fn tags_favorite_trash() {
    let mut item = login();
    {
        let mut e = item.edit(dev(1), at(1));
        e.add_tag("work").unwrap();
        e.add_tag("Ünïcode tag").unwrap();
        e.add_tag("work").unwrap();
        e.set_favorite(true).unwrap();
        e.trash().unwrap();
        e.trash().unwrap();
    }
    assert_eq!(item.tags(), vec!["work", "Ünïcode tag"]);
    assert!(item.favorite());
    assert_eq!(item.trashed_at(), Some(at(1)));
    for bad in ["", "a\nb", &"t".repeat(MAX_TAG_LEN + 1)] {
        assert_eq!(
            item.edit(dev(1), at(2)).add_tag(bad).unwrap_err(),
            Error::Malformed
        );
    }
    let mut e = item.edit(dev(1), at(3));
    e.remove_tag("work").unwrap();
    e.restore_from_trash().unwrap();
    assert_eq!(item.tags(), vec!["Ünïcode tag"]);
    assert_eq!(item.trashed_at(), None);
    // Trash vs. concurrent edit: both apply.
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(4)).trash().unwrap();
    b.edit(dev(3), at(4)).set_title("edited").unwrap();
    let m = a.merge(&b);
    assert_eq!(m.trashed_at(), Some(at(4)));
    assert_eq!(m.title(), Some("edited"));
    // Concurrent add and remove of the same tag: the add wins.
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(5)).remove_tag("Ünïcode tag").unwrap();
    b.edit(dev(3), at(5)).add_tag("Ünïcode tag").unwrap();
    assert!(a.merge(&b).tags().is_empty(), "b's add was a no-op");
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(5)).add_tag("new").unwrap();
    a.edit(dev(2), at(6)).remove_tag("new").unwrap();
    b.edit(dev(3), at(5)).add_tag("new").unwrap();
    assert_eq!(a.merge(&b).tags(), vec!["new", "Ünïcode tag"]);
}

#[test]
fn urls_and_custom_fields() {
    let mut item = login();
    let mut e = item.edit(dev(1), at(1));
    let first = e.add_url(&LoginUrl::new("https://github.com")).unwrap();
    let mut u = LoginUrl::new("https://gist.github.com");
    u.match_mode = UrlMatch::Etld1;
    u.allow_http = true;
    let second = e.add_url(&u).unwrap();
    let f = e
        .add_custom_field(&CustomField::new(
            Text::new("PIN"),
            CustomFieldValue::Hidden(SecretString::from_str_copy("1234").unwrap()),
        ))
        .unwrap();
    assert_ne!(first, second);
    let urls = item.urls();
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0].0, first);
    assert_eq!(urls[1].1.match_mode, UrlMatch::Etld1);
    assert!(urls[1].1.allow_http);
    assert!(!urls[0].1.allow_http);
    assert_eq!(first.to_string(), first.as_str());
    assert_eq!(ElementId::from_stored(first.as_str()).unwrap(), first);
    // Replace and remove.
    let mut e = item.edit(dev(1), at(2));
    e.set_url(&first, &LoginUrl::new("https://github.com/login"))
        .unwrap();
    e.remove_url(&second).unwrap();
    let urls = item.urls();
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].1.url.as_str(), "https://github.com/login");
    let fields = item.custom_fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].0, f);
    match &fields[0].1.value {
        CustomFieldValue::Hidden(s) => assert_eq!(s.expose_secret(), "1234"),
        _ => panic!("wrong kind"),
    }
    let mut bad_otp = OtpConfig::totp(SecretVec::from_slice(b"k").unwrap());
    bad_otp.digits = 4;
    let bad = CustomField::new(Text::new("otp"), CustomFieldValue::Otp(bad_otp));
    assert_eq!(
        item.edit(dev(1), at(3)).add_custom_field(&bad).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        item.edit(dev(1), at(3))
            .set_custom_field(&f, &bad)
            .unwrap_err(),
        Error::Malformed
    );
    let good = CustomField::new(Text::new("note"), CustomFieldValue::Text(Text::new("x")));
    item.edit(dev(1), at(3))
        .set_custom_field(&f, &good)
        .unwrap();
    assert!(matches!(
        item.custom_fields()[0].1.value,
        CustomFieldValue::Text(_)
    ));
    item.edit(dev(1), at(4)).remove_custom_field(&f).unwrap();
    assert!(item.custom_fields().is_empty());
    // Concurrent additions of different URLs never conflict.
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(5))
        .add_url(&LoginUrl::new("https://a"))
        .unwrap();
    b.edit(dev(3), at(5))
        .add_url(&LoginUrl::new("https://b"))
        .unwrap();
    let m = a.merge(&b);
    assert_eq!(m.urls().len(), 3);
    assert!(m.conflicts().is_empty());
}

#[test]
fn attachments_otp_passkey_ssh() {
    let mut item = Item::new(&ItemType::Document, dev(1), at(1)).unwrap();
    let id = AttachmentId::from_bytes([0x0D; 16]).unwrap();
    let key = AttachmentKey::generate().unwrap();
    let a = AttachmentRef::new(
        id,
        Text::new("scan.pdf"),
        Text::new("application/pdf"),
        42,
        key,
    );
    item.edit(dev(1), at(2)).set_attachment(&a).unwrap();
    let got = item.attachments();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].key, a.key);
    let huge = AttachmentRef::new(
        id,
        Text::new("big"),
        Text::new("x"),
        u64::MAX,
        AttachmentKey::generate().unwrap(),
    );
    assert_eq!(
        item.edit(dev(1), at(3)).set_attachment(&huge).unwrap_err(),
        Error::SizeLimit
    );
    item.edit(dev(1), at(3)).remove_attachment(&id).unwrap();
    assert!(item.attachments().is_empty());

    let mut otp = OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap());
    otp.kind = OtpKind::Hotp;
    item.edit(dev(1), at(4)).set_otp(&otp).unwrap();
    assert_eq!(item.otp().unwrap().kind, OtpKind::Hotp);
    otp.period = 0;
    assert_eq!(
        item.edit(dev(1), at(4)).set_otp(&otp).unwrap_err(),
        Error::Malformed
    );
    assert_eq!(item.otp_counter(), 0);
    item.edit(dev(1), at(5)).set_otp_counter(5).unwrap();
    let v = item.version().clone();
    item.edit(dev(1), at(5)).set_otp_counter(3).unwrap();
    assert_eq!(item.version(), &v, "counters never go down");
    assert_eq!(item.otp_counter(), 5);
    // Concurrent counters merge to the maximum without a conflict.
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(6)).set_otp_counter(9).unwrap();
    b.edit(dev(3), at(6)).set_otp_counter(7).unwrap();
    let mut m = a.merge(&b);
    assert_eq!(m.otp_counter(), 9);
    assert!(m.conflicts().is_empty());
    assert_eq!(m.entries("otp.counter").count(), 2);
    m.edit(dev(1), at(7)).set_otp_counter(8).unwrap();
    assert_eq!(m.otp_counter(), 9, "collapsed to the max");
    assert_eq!(m.entries("otp.counter").count(), 1);
    m.edit(dev(1), at(7)).clear_otp().unwrap();
    assert!(m.otp().is_none());

    let mut pk = Item::new(&ItemType::Passkey, dev(1), at(1)).unwrap();
    let cred = PasskeyCredential::new(
        vec![1; 16],
        Text::new("example.com"),
        vec![2; 16],
        SecretVec::from_slice(&[3; 32]).unwrap(),
        -7,
        at(1),
    );
    pk.edit(dev(1), at(1)).set_passkey(&cred).unwrap();
    pk.edit(dev(1), at(2)).set_passkey_sign_count(10).unwrap();
    assert_eq!(pk.passkey().unwrap().private_key, cred.private_key);
    assert_eq!(pk.passkey_sign_count(), 10);
    let bad = PasskeyCredential::new(
        vec![],
        Text::new("example.com"),
        vec![2; 16],
        SecretVec::from_slice(&[3; 32]).unwrap(),
        -7,
        at(1),
    );
    assert_eq!(
        pk.edit(dev(1), at(3)).set_passkey(&bad).unwrap_err(),
        Error::Malformed
    );

    let mut ssh = Item::new(&ItemType::SshKey, dev(1), at(1)).unwrap();
    let key = SshKeyMaterial::new(
        SecretString::from("-----BEGIN".to_owned()),
        Text::new("ssh-ed25519 AAAA"),
        Text::new("SHA256:x"),
    );
    ssh.edit(dev(1), at(1)).set_ssh_key(&key).unwrap();
    ssh.edit(dev(1), at(1))
        .set_text(TextField::SshComment, "me@host")
        .unwrap();
    assert_eq!(
        ssh.ssh_key().unwrap().private_key.expose_secret(),
        "-----BEGIN"
    );
    assert_eq!(ssh.unreadable_fields(), Vec::<&str>::new());
}

#[test]
fn concurrent_edit_conflict_and_resolution() {
    let base = login();
    let mut a = copy(&base);
    let mut b = copy(&base);
    let mut c = copy(&base);
    a.edit(dev(2), at(10))
        .set_secret(SecretField::Password, "from-a")
        .unwrap();
    b.edit(dev(3), at(5))
        .set_secret(SecretField::Password, "from-b")
        .unwrap();
    c.edit(dev(4), at(1))
        .set_text(TextField::Username, "other")
        .unwrap();
    assert_eq!(a.compare(&b), Causality::Concurrent);
    let m = a.merge(&b).merge(&c);
    assert_eq!(m, c.merge(&b).merge(&a));
    // Deterministic winner by Lamport stamp (same counter, larger device),
    // independent of the wall clock (a wrote later by its clock).
    assert_eq!(password(&m), "from-b");
    let conflicts = m.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].key, SecretField::Password.key());
    assert_eq!(conflicts[0].losers.len(), 1);
    assert_eq!(conflicts[0].losers[0].value().as_str(), Some("from-a"));
    assert_eq!(m.text(TextField::Username), Some("other"));
    assert_eq!(m.compare(&a), Causality::Dominates);
    assert_eq!(a.compare(&m), Causality::DominatedBy);

    // A plain edit keeps the conflict copy.
    let mut edited = copy(&m);
    edited
        .edit(dev(2), at(20))
        .set_secret(SecretField::Password, "newer")
        .unwrap();
    assert_eq!(password(&edited), "newer");
    assert_eq!(
        edited.conflicts()[0].losers[0].value().as_str(),
        Some("from-a")
    );

    // Resolving to the loser: the discarded values go into the history.
    let mut resolved = copy(&m);
    let loser = m.conflicts()[0].losers[0].stamp();
    resolved
        .edit(dev(1), at(30))
        .resolve_conflict(SecretField::Password.key(), loser)
        .unwrap();
    assert_eq!(password(&resolved), "from-a");
    assert!(resolved.conflicts().is_empty());
    assert!(
        resolved
            .password_history()
            .iter()
            .any(|h| h.value.expose_secret() == "from-b")
    );
    // The resolution propagates: merging the old conflicted state back in
    // does not bring the conflict back.
    assert!(resolved.merge(&m).conflicts().is_empty());
    assert_eq!(password(&resolved.merge(&m)), "from-a");

    // Dismissing keeps the winner.
    let mut dismissed = copy(&m);
    dismissed
        .edit(dev(1), at(30))
        .dismiss_conflict(SecretField::Password.key())
        .unwrap();
    assert_eq!(password(&dismissed), "from-b");
    assert!(dismissed.merge(&m).conflicts().is_empty());
    // Resolving a field without a conflict is a no-op; unknown stamps fail.
    let v = dismissed.version().clone();
    dismissed
        .edit(dev(1), at(31))
        .dismiss_conflict("title")
        .unwrap();
    assert_eq!(dismissed.version(), &v);
    assert_eq!(
        dismissed
            .edit(dev(1), at(31))
            .dismiss_conflict("nope")
            .unwrap_err(),
        Error::Malformed
    );
    assert_eq!(
        dismissed
            .edit(dev(1), at(31))
            .resolve_conflict("title", loser)
            .unwrap_err(),
        Error::Malformed
    );
    // Equal concurrent values are not a conflict.
    let mut x = copy(&base);
    let mut y = copy(&base);
    x.edit(dev(2), at(1)).set_title("same").unwrap();
    y.edit(dev(3), at(1)).set_title("same").unwrap();
    let xy = x.merge(&y);
    assert!(xy.conflicts().is_empty());
    assert_eq!(xy.entries("title").count(), 2);
}

#[test]
fn delete_tombstone_and_concurrent_edit() {
    let vault = VaultId::from_bytes([0x0B; 16]).unwrap();
    let id = ItemId::from_bytes([0x0C; 16]).unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let base = login();
    let mut deleted = copy(&base);
    deleted.delete(dev(2), at(50)).unwrap();
    assert!(deleted.is_deleted());
    assert!(deleted.deleted_concurrently().is_none());
    assert!(base.to_tombstone().is_none());
    let tomb = deleted.to_tombstone().unwrap();
    assert_eq!(tomb.deleted_at(), at(50));
    assert_eq!(tomb.deleted_by(), vec![dev(2)]);
    assert_eq!(tomb.version(), deleted.version());
    assert!(format!("{tomb:?}").starts_with("Tombstone"));
    let ct = tomb.seal(&vk, &vault, &id).unwrap();
    let ct = TombstoneCiphertext::from_bytes(ct.as_bytes()).unwrap();
    let back = Tombstone::open(&ct, &vk, &vault, &id).unwrap();
    assert_eq!(back, tomb);
    let other = ItemId::from_bytes([0x0D; 16]).unwrap();
    assert_eq!(
        Tombstone::open(&ct, &vk, &vault, &other).unwrap_err(),
        Error::ItemCorrupted { item_id: other }
    );
    let as_item = Item::from(back);
    assert_eq!(as_item, deleted);

    // Stale replicas do not resurrect a deleted item.
    assert!(as_item.merge(&base).is_deleted());
    assert_eq!(as_item.compare(&base), Causality::Dominates);

    // A concurrent edit survives and the item is flagged.
    let mut editor = copy(&base);
    editor
        .edit(dev(3), at(40))
        .set_secret(SecretField::Password, "changed offline")
        .unwrap();
    let m = as_item.merge(&editor);
    assert_eq!(m, editor.merge(&as_item));
    assert!(!m.is_deleted());
    let info = m.deleted_concurrently().unwrap();
    assert_eq!(info.deleted_at, at(50));
    assert_eq!(info.deleted_by, vec![dev(2)]);
    assert_eq!(password(&m), "changed offline");
    assert_eq!(m.title(), None, "fields the deleter had seen are removed");
    assert!(m.to_tombstone().is_none());

    // The editing device merges with repair and keeps every field it had.
    let out = editor.merge_and_repair(&as_item, dev(3)).unwrap();
    assert_eq!(out.restored_fields, 4, "type, created_at, title, username");
    assert!(!out.stamp_collision);
    let mut restored = out.item;
    assert!(restored.deleted_concurrently().is_some(), "still flagged");
    restored.edit(dev(3), at(60)).dismiss_deletion().unwrap();
    assert_eq!(restored.title(), Some("GitHub"));
    assert_eq!(restored.created_at(), Some(at(100)));
    assert_eq!(restored.item_type(), Some(ItemType::Login));
    assert_eq!(password(&restored), "changed offline");
    assert!(restored.deleted_concurrently().is_none());
    // Dismissal propagates.
    assert!(restored.merge(&m).deleted_concurrently().is_none());

    // Deleting again wins over the flagged state.
    let mut again = copy(&m);
    again.delete(dev(3), at(70)).unwrap();
    assert!(again.merge(&m).is_deleted());
    again.delete(dev(3), at(71)).unwrap();
    assert_eq!(again.to_tombstone().unwrap().deleted_at(), at(70));

    // Concurrent deletes merge into one tombstone with both deletions.
    let mut d1 = copy(&base);
    let mut d2 = copy(&base);
    d1.delete(dev(2), at(1)).unwrap();
    d2.delete(dev(3), at(2)).unwrap();
    let both = d1.merge(&d2);
    assert!(both.is_deleted());
    let t = both.to_tombstone().unwrap();
    assert_eq!(t.deleted_by(), vec![dev(2), dev(3)]);
    assert_eq!(t.deleted_at(), at(2));

    // A deleted item takes no edits; undelete brings it back with a type.
    let mut undo = copy(&deleted);
    let v = undo.version().clone();
    assert_eq!(
        undo.edit(dev(2), at(80)).set_title("back").unwrap_err(),
        Error::ItemDeleted
    );
    assert_eq!(
        undo.edit(dev(2), at(80))
            .add_url(&LoginUrl::new("https://x"))
            .unwrap_err(),
        Error::ItemDeleted
    );
    assert_eq!(undo.version(), &v, "a refused edit changes nothing");
    undo.undelete(&ItemType::Login, dev(2), at(80)).unwrap();
    undo.edit(dev(2), at(80)).set_title("back").unwrap();
    assert!(!undo.is_deleted());
    assert!(undo.deleted_concurrently().is_none());
    assert_eq!(undo.title(), Some("back"));
    assert_eq!(undo.item_type(), Some(ItemType::Login));
    assert_eq!(undo.created_at(), Some(at(80)));
    let v = undo.version().clone();
    undo.undelete(&ItemType::Wifi, dev(2), at(81)).unwrap();
    assert_eq!(undo.version(), &v, "undelete of a live item is a no-op");
    // The undelete wins over the tombstone it saw.
    assert!(!undo.merge(&deleted).is_deleted());
    // Dismissing on a deleted item does nothing.
    let mut still = copy(&deleted);
    let v = still.version().clone();
    still.edit(dev(2), at(81)).dismiss_deletion().unwrap();
    assert_eq!(still.version(), &v);
}

#[test]
fn seal_and_open() {
    let vault = VaultId::from_bytes([0x0B; 16]).unwrap();
    let id = ItemId::from_bytes([0x0C; 16]).unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let item = login();
    let ct = item.seal(&vk, &vault, &id).unwrap();
    assert_eq!(ct.schema_version(), 1);
    let ct = ItemCiphertext::from_bytes(ct.as_bytes()).unwrap();
    assert_eq!(Item::open(&ct, &vk, &vault, &id).unwrap(), item);
    let other = ItemId::from_bytes([0x0D; 16]).unwrap();
    assert_eq!(
        Item::open(&ct, &vk, &vault, &other).unwrap_err(),
        Error::ItemCorrupted { item_id: other }
    );
    let vk2 = VaultKey::generate(KeyGeneration::new(2).unwrap()).unwrap();
    assert_eq!(
        Item::open(&ct, &vk2, &vault, &id).unwrap_err(),
        Error::KeyGenerationMismatch
    );
    // A newer schema version in the header is refused before decryption.
    let pt = item.to_json().unwrap();
    let newer = ItemCiphertext::seal(&vk, &vault, &id, 2, pt.expose_secret()).unwrap();
    assert_eq!(
        Item::open(&newer, &vk, &vault, &id).unwrap_err(),
        Error::UnsupportedVersion
    );
    // Authentic but undecodable plaintext.
    let junk = ItemCiphertext::seal(&vk, &vault, &id, 1, b"not json").unwrap();
    assert_eq!(
        Item::open(&junk, &vk, &vault, &id).unwrap_err(),
        Error::Malformed
    );
    // Size limits.
    let mut big = Item::new(&ItemType::SecureNote, dev(1), at(1)).unwrap();
    big.edit(dev(1), at(1))
        .set_text(TextField::Notes, &"x".repeat(1024 * 1024))
        .unwrap();
    assert_eq!(big.to_json().unwrap_err(), Error::SizeLimit);
    assert_eq!(big.seal(&vk, &vault, &id).unwrap_err(), Error::SizeLimit);
    assert_eq!(
        Item::from_json(&vec![b' '; 1024 * 1024]).unwrap_err(),
        Error::SizeLimit
    );
}

#[test]
fn unknown_fields_survive_an_older_client() {
    // A plaintext from a newer client: an unknown item type field, an
    // unknown member in a known value, an unknown entry member and an
    // unknown top-level member.
    let d = "02020202-0202-0202-0202-020202020202";
    let json = format!(
        concat!(
            r#"{{"schema_version":1,"vv":{{"{d}":3}},"fields":{{"#,
            r#""login.passkey_hint":[{{"at":1,"c":1,"d":"{d}","v":{{"x":[1,2.5]}}}}],"#,
            r#""title":[{{"at":1,"c":2,"d":"{d}","v":"t","author":"bob"}}],"#,
            r#""type":[{{"at":1,"c":1,"d":"{d}","v":"login"}}],"#,
            r#""url:1":[{{"at":1,"c":3,"d":"{d}","v":{{"allow_http":false,"label":"Work","match":"origin","url":"https://a"}}}}],"#,
            r#""url:2":[{{"at":1,"c":3,"d":"{d}","v":{{"match":"prefix","url":"https://b"}}}}]"#,
            r#"}},"sync_hint":{{"y":true}}}}"#
        ),
        d = d
    );
    let mut item = Item::from_json(json.as_bytes()).unwrap();
    assert_eq!(item.to_json().unwrap().expose_secret(), json.as_bytes());
    assert_eq!(
        item.unreadable_fields(),
        vec!["login.passkey_hint", "url:2"]
    );
    assert_eq!(item.urls().len(), 1, "the unreadable URL is skipped");
    // The older client edits the readable URL and the title.
    let (id, mut url) = item.urls().pop().unwrap();
    url.allow_http = true;
    {
        let mut e = item.edit(dev(1), at(5));
        e.set_url(&id, &url).unwrap();
        e.set_title("renamed").unwrap();
    }
    let out = json_string(&item);
    assert!(out.contains(r#""login.passkey_hint":[{"at":1,"c":1,"#));
    assert!(out.contains(r#""label":"Work""#), "unknown member kept");
    assert!(out.contains(r#""match":"prefix""#), "unreadable value kept");
    assert!(out.ends_with(r#","sync_hint":{"y":true}}"#));
    let raw = item.value("login.passkey_hint").unwrap();
    assert_eq!(raw.as_object().unwrap().len(), 1);
    assert!(!item.keys().any(|k| k == "sync_hint"));
    // Unknown items types and bad values of known keys are reported.
    let other = format!(
        r#"{{"schema_version":1,"vv":{{"{d}":1}},"fields":{{"favorite":[{{"at":1,"c":1,"d":"{d}","v":"yes"}}],"type":[{{"at":1,"c":1,"d":"{d}","v":"bank"}}]}}}}"#
    );
    let o = Item::from_json(other.as_bytes()).unwrap();
    assert_eq!(o.item_type(), Some(ItemType::Other("bank".into())));
    assert_eq!(o.unreadable_fields(), vec!["favorite"]);
    assert!(!o.favorite());
    // A newer schema version is refused.
    let newer = json.replace(r#""schema_version":1"#, r#""schema_version":2"#);
    assert_eq!(
        Item::from_json(newer.as_bytes()).unwrap_err(),
        Error::UnsupportedVersion
    );
}

#[test]
fn debug_output_is_redacted() {
    let item = login();
    let dbg = format!("{item:?}");
    assert!(dbg.starts_with("Item"));
    assert!(!dbg.contains("pw-1") && !dbg.contains("GitHub") && !dbg.contains("octocat"));
    let e = item.entries("login.password").next().unwrap();
    assert!(!format!("{e:?}").contains("pw-1"));
    assert!(!format!("{:?}", item.conflicts()).contains("pw-1"));
    let url = LoginUrl::new("https://secret.example");
    assert!(!format!("{url:?}").contains("secret"));
}

#[test]
fn clock_is_independent_of_wall_time() {
    // Device 2's wall clock is far behind, but its edit happened after it
    // saw device 1's edit, so it wins.
    let mut a = login();
    a.edit(dev(1), at(1_000_000)).set_title("first").unwrap();
    let mut b = copy(&a);
    b.edit(dev(2), at(1)).set_title("second").unwrap();
    let m = a.merge(&b);
    assert_eq!(m.title(), Some("second"));
    assert!(m.conflicts().is_empty());
    let e = m.entries("title").next().unwrap();
    assert_eq!(e.at(), at(1));
    assert!(e.stamp().counter() > a.version().max_counter());
    // Device 1 merges and writes again: its stamp is larger still.
    let mut c = copy(&m);
    c.edit(dev(1), at(0)).set_title("third").unwrap();
    assert_eq!(c.merge(&b).title(), Some("third"));
    let _ = SecretBytes::<1>::from_slice(&[0]).unwrap();
}

fn keys(item: &Item) -> Vec<String> {
    item.keys().map(str::to_owned).collect()
}

#[test]
fn reserved_members_cannot_cross_layouts() {
    // Review M2: a tombstone carrying a top-level "fields" member, or an
    // item carrying "deleted", used to decode and then re-encode with a
    // duplicate key that no device could decode again.
    let mut deleted = login();
    deleted.delete(dev(2), at(5)).unwrap();
    let tomb = String::from_utf8(
        deleted
            .to_tombstone()
            .unwrap()
            .to_json()
            .unwrap()
            .expose_secret()
            .to_vec(),
    )
    .unwrap();
    let with_fields = format!(r#"{},"fields":{{}}}}"#, &tomb[..tomb.len() - 1]);
    assert_eq!(
        Tombstone::from_json(with_fields.as_bytes()).unwrap_err(),
        Error::Malformed
    );
    let item = json_string(&login());
    let with_deleted = format!(
        r#"{},"deleted":[{{"at":0,"c":1,"d":"{}","v":null}}]}}"#,
        &item[..item.len() - 1],
        dev(1)
    );
    assert_eq!(
        Item::from_json(with_deleted.as_bytes()).unwrap_err(),
        Error::Malformed
    );
    for member in ["schema_version", "vv"] {
        let dup = format!(r#"{},"{member}":1}}"#, &item[..item.len() - 1]);
        assert_eq!(
            Item::from_json(dup.as_bytes()).unwrap_err(),
            Error::Malformed
        );
    }
    // Other unknown top-level members still survive both layouts.
    let newer = format!(r#"{},"later":[1]}}"#, &tomb[..tomb.len() - 1]);
    let t = Tombstone::from_json(newer.as_bytes()).unwrap();
    assert!(
        String::from_utf8(t.to_json().unwrap().expose_secret().to_vec())
            .unwrap()
            .ends_with(r#","later":[1]}"#)
    );
}

#[test]
fn edit_api_only_writes_keys_that_decode() {
    // Review M2: element IDs from outside used to become field keys the
    // decoder rejects, which made the whole item unreadable once synced.
    for bad in ["a\nb", "tab\there", &"x".repeat(253)] {
        assert_eq!(ElementId::from_stored(bad).unwrap_err(), Error::Malformed);
    }
    let max = ElementId::from_stored(&"x".repeat(252)).unwrap();
    let mut item = login();
    let v = item.version().clone();
    assert_eq!(
        item.edit(dev(1), at(1))
            .set_url(&max, &LoginUrl::new("https://a"))
            .unwrap_err(),
        Error::Malformed,
        "no such URL: not silently created"
    );
    let text = CustomField::new(Text::new("l"), CustomFieldValue::Text(Text::new("v")));
    assert_eq!(
        item.edit(dev(1), at(1))
            .set_custom_field(&max, &text)
            .unwrap_err(),
        Error::Malformed
    );
    assert_eq!(item.version(), &v, "refused edits tick nothing");

    // The longest stored IDs work through the edit API and decode again.
    let d = dev(1);
    let url_id = "u".repeat(252);
    let field_id = "f".repeat(250);
    let json = format!(
        concat!(
            r#"{{"schema_version":1,"vv":{{"{d}":1}},"fields":{{"#,
            r#""field:{f}":[{{"at":1,"c":1,"d":"{d}","v":{{"kind":"text","label":"l","value":"v"}}}}],"#,
            r#""type":[{{"at":1,"c":1,"d":"{d}","v":"login"}}],"#,
            r#""url:{u}":[{{"at":1,"c":1,"d":"{d}","v":{{"url":"https://a"}}}}]"#,
            r#"}}}}"#
        ),
        d = d,
        f = field_id,
        u = url_id
    );
    let mut item = Item::from_json(json.as_bytes()).unwrap();
    let uid = ElementId::from_stored(&url_id).unwrap();
    assert_eq!(item.urls()[0].0, uid);
    let fid = item.custom_fields()[0].0.clone();
    {
        let mut e = item.edit(dev(2), at(2));
        e.set_url(&uid, &LoginUrl::new("https://b")).unwrap();
        e.set_custom_field(&fid, &text).unwrap();
        e.add_tag(&"t".repeat(MAX_TAG_LEN)).unwrap();
        e.add_tag("\u{e9}\u{301} \u{1f511}").unwrap();
        e.add_url(&LoginUrl::new("https://c")).unwrap();
        e.add_custom_field(&text).unwrap();
        e.set_secret(SecretField::Password, "new").unwrap();
    }
    assert_eq!(copy(&item), item, "every key the edit API wrote decodes");
}

#[test]
fn delete_vs_concurrent_edit_keeps_every_field_after_repair() {
    // Review M2 probe: base item with title, username and password; device
    // A deletes, device B concurrently sets the notes.
    let base = login();
    let mut a = copy(&base);
    a.delete(dev(2), at(10)).unwrap();
    let tomb = Item::from(
        Tombstone::from_json(a.to_tombstone().unwrap().to_json().unwrap().expose_secret()).unwrap(),
    );
    let mut b = copy(&base);
    b.edit(dev(3), at(11))
        .set_text(TextField::Notes, "note")
        .unwrap();

    // The pure merge keeps only the concurrent edit (the husk).
    let husk = b.merge(&tomb);
    assert!(husk.deleted_concurrently().is_some());
    assert_eq!(husk.item_type(), None);
    assert_eq!(husk.title(), None);
    assert!(husk.secret(SecretField::Password).unwrap().is_none());

    // The editing device merges with repair and keeps everything.
    let out = b.merge_and_repair(&tomb, dev(3)).unwrap();
    let repaired = out.item;
    assert_eq!(out.restored_fields, 5);
    assert_eq!(repaired.item_type(), Some(ItemType::Login));
    assert_eq!(repaired.created_at(), Some(at(100)));
    assert_eq!(repaired.title(), Some("GitHub"));
    assert_eq!(repaired.text(TextField::Username), Some("octocat"));
    assert_eq!(password(&repaired), "pw-1");
    assert_eq!(repaired.text(TextField::Notes), Some("note"));
    assert!(repaired.deleted_concurrently().is_some(), "still flagged");
    assert!(repaired.conflicts().is_empty());
    for k in keys(&b) {
        assert!(repaired.keys().any(|r| r == k), "{k} kept");
    }
    // Stable: merging the tombstone again repairs nothing more.
    let again = repaired.merge_and_repair(&tomb, dev(3)).unwrap();
    assert_eq!(again.restored_fields, 0);
    assert_eq!(again.item, repaired);

    // The deleting device cannot repair (it saw its own deletion) ...
    let at_a = tomb.merge_and_repair(&b, dev(2)).unwrap();
    assert_eq!(at_a.restored_fields, 0);
    assert_eq!(at_a.item, husk);
    // ... and gets the full item once the repair arrives.
    let at_a = at_a.item.merge_and_repair(&repaired, dev(2)).unwrap();
    assert_eq!(at_a.item.title(), Some("GitHub"));
    assert_eq!(at_a.item.item_type(), Some(ItemType::Login));

    // A third device that held the item but had not seen the deletion
    // repairs too; its values equal B's, so nothing conflicts.
    let c = copy(&base);
    let at_c = c.merge_and_repair(&husk, dev(4)).unwrap();
    assert_eq!(at_c.restored_fields, 5);
    let all = at_c.item.merge(&repaired).merge(&at_a.item);
    assert!(all.conflicts().is_empty());
    assert_eq!(all.title(), Some("GitHub"));
    assert_eq!(all.text(TextField::Notes), Some("note"));

    // Without a concurrent edit the delete wins: nothing is repaired.
    let stale = c.merge_and_repair(&tomb, dev(4)).unwrap();
    assert!(stale.item.is_deleted());
    assert_eq!(stale.restored_fields, 0);

    // Conflict copies come back too.
    let mut x = copy(&base);
    let mut y = copy(&base);
    x.edit(dev(5), at(1)).set_title("from-x").unwrap();
    y.edit(dev(6), at(1)).set_title("from-y").unwrap();
    let conflicted = x.merge(&y);
    let mut deleter = copy(&conflicted);
    deleter.delete(dev(7), at(2)).unwrap();
    let mut editor = copy(&conflicted);
    editor
        .edit(dev(5), at(3))
        .set_text(TextField::Notes, "n")
        .unwrap();
    let fixed = editor.merge_and_repair(&deleter, dev(5)).unwrap().item;
    let c = fixed.conflicts();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].key, "title");
    assert_eq!(fixed.title(), Some("from-y"), "the winner stays the winner");
}

#[test]
fn device_id_reuse_is_detected() {
    // Review M2 probe: desktop and extension writing as the same device.
    let base = login();
    let mut desktop = copy(&base);
    let mut ext = copy(&base);
    desktop
        .edit(dev(4), at(1))
        .set_secret(SecretField::Password, "from-desktop")
        .unwrap();
    ext.edit(dev(4), at(1))
        .set_secret(SecretField::Password, "from-ext")
        .unwrap();
    let s = desktop.entries("login.password").last().unwrap().stamp();
    assert_eq!(s, ext.entries("login.password").last().unwrap().stamp());
    // The pure merge keeps one of them.
    assert!(desktop.merge(&ext).conflicts().is_empty());
    // A third device's repairing merge surfaces both.
    let mut phone = copy(&base);
    phone = phone.merge_and_repair(&desktop, dev(9)).unwrap().item;
    let out = phone.merge_and_repair(&ext, dev(9)).unwrap();
    assert!(out.stamp_collision);
    let c = out.item.conflicts();
    assert_eq!(c.len(), 1);
    let mut values: Vec<&str> = c[0]
        .losers
        .iter()
        .chain([&c[0].winner])
        .filter_map(|e| e.value().as_str())
        .collect();
    values.sort_unstable();
    assert_eq!(values, ["from-desktop", "from-ext"]);
    // The writer whose ID was reused finds out on its next merge.
    ext.edit(dev(4), at(2)).set_title("ext again").unwrap();
    assert_eq!(
        desktop.merge_and_repair(&ext, dev(4)).unwrap_err(),
        Error::DeviceIdReused
    );
    // Its own record coming back is fine.
    assert!(
        !desktop
            .merge_and_repair(&desktop, dev(4))
            .unwrap()
            .stamp_collision
    );
}

#[test]
fn metadata_is_never_a_conflict() {
    let base = login();
    let mut a = copy(&base);
    let mut b = copy(&base);
    {
        let mut e = a.edit(dev(2), at(1));
        e.set_favorite(true).unwrap();
        e.trash().unwrap();
        e.set_title("a").unwrap();
    }
    {
        let mut e = b.edit(dev(3), at(2));
        e.set_favorite(false).unwrap();
        e.trash().unwrap();
        e.set_title("b").unwrap();
    }
    let m = a.merge(&b);
    assert_eq!(m.entries("favorite").count(), 2);
    assert_eq!(m.entries("trashed_at").count(), 2);
    let c = m.conflicts();
    assert_eq!(c.len(), 1, "only the title");
    assert_eq!(c[0].key, "title");
    assert!(!m.favorite(), "largest stamp");
    assert_eq!(m.trashed_at(), Some(at(2)));
}

#[test]
fn counters_never_go_down_through_conflict_resolution() {
    let mut item = Item::new(&ItemType::Passkey, dev(1), at(1)).unwrap();
    item.edit(dev(1), at(1)).set_passkey_sign_count(5).unwrap();
    let mut a = copy(&item);
    let mut b = copy(&item);
    a.edit(dev(2), at(2)).set_passkey_sign_count(9).unwrap();
    b.edit(dev(3), at(2)).set_passkey_sign_count(7).unwrap();
    let m = a.merge(&b);
    let key = "passkey.sign_count";
    let winner = m.entries(key).last().unwrap();
    assert_eq!(winner.value().as_u64(), Some(7), "largest stamp holds 7");
    let low = winner.stamp();
    let high = m
        .entries(key)
        .find(|e| e.value().as_u64() == Some(9))
        .unwrap()
        .stamp();
    let mut r = copy(&m);
    assert_eq!(
        r.edit(dev(1), at(3))
            .resolve_conflict(key, low)
            .unwrap_err(),
        Error::Malformed
    );
    assert_eq!(r.passkey_sign_count(), 9);
    let mut d = copy(&m);
    d.edit(dev(1), at(3)).dismiss_conflict(key).unwrap();
    assert_eq!(d.passkey_sign_count(), 9);
    assert_eq!(d.entries(key).count(), 1);
    r.edit(dev(1), at(3)).resolve_conflict(key, high).unwrap();
    assert_eq!(r.passkey_sign_count(), 9);
    assert_eq!(r.entries(key).count(), 1);
    assert_eq!(
        r.edit(dev(1), at(4))
            .dismiss_conflict("otp.counter")
            .unwrap_err(),
        Error::Malformed
    );
}
