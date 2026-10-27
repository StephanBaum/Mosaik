//! Session unit tests: the paths the end-to-end tests in `tests/` cannot
//! reach through the public API alone (corrupt and damaged rows,
//! hardware-protected vaults, interrupted re-seals, the generation rule,
//! search ranking).

#![allow(clippy::unwrap_used, reason = "tests")]

use mosaic_core::model::{Item, ItemType, SecretField, TextField, VaultMeta};
use mosaic_core::{
    AccountKey, Argon2Profile, DeviceId, ItemCiphertext, ItemId, KeyGeneration, MasterPassword,
    SecretKey, Timestamp, VaultId, VaultKey,
};

use super::sync::RemoteRecord;
use super::{ApplyOutcome, Error, LockedSession, UnlockedSession, search::SearchIndex};
use crate::storage::{Error as StorageError, Origin, RecordBody, Store, VaultKeyWrap};

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(Store::open_in_memory().unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

#[test]
fn generation_rule_is_exact() {
    let g1 = KeyGeneration::FIRST;
    let g2 = g1.next().unwrap();
    assert!(super::sync::check_generation(g2, g2).is_ok());
    assert!(matches!(
        super::sync::check_generation(g2, g1),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    assert!(matches!(
        super::sync::check_generation(g1, g2),
        Err(Error::KeyGenerationNotAccepted {
            highest_accepted: 1,
            offered: 2
        })
    ));
}

#[test]
fn search_ranks_title_prefixes_first() {
    let device = DeviceId::new_v7().unwrap();
    let at = Timestamp::from_unix_millis(0);
    let vault = VaultId::new_v7().unwrap();
    let mk = |title: &str, user: &str| {
        let mut item = Item::new(&ItemType::Login, device, at).unwrap();
        {
            let mut e = item.edit(device, at);
            e.set_title(title).unwrap();
            e.set_text(TextField::Username, user).unwrap();
        }
        item
    };
    let mut index = SearchIndex::default();
    let (a, b) = (ItemId::new_v7().unwrap(), ItemId::new_v7().unwrap());
    // `a` sorts first in the index but matches only through its user name.
    index.update(vault, a, &mk("Mail", "github-user"));
    index.update(vault, b, &mk("GitHub", "x"));
    assert_eq!(index.search("git"), [b, a].map(|i| (vault, i)));
    // Every term must match, each in any field.
    assert_eq!(index.search("GIT HUB").len(), 2);
    assert_eq!(index.search("hub X"), [b].map(|i| (vault, i)));
    index.remove(vault, b);
    assert_eq!(index.search("git"), [a].map(|i| (vault, i)));
    // A deleted item leaves the index.
    let mut gone = mk("Mail", "github-user");
    gone.delete(device, at).unwrap();
    index.update(vault, a, &gone);
    assert_eq!(index.search("git").len(), 0);
}

#[test]
fn corrupt_rows_are_listed_and_replaced_by_a_valid_version() {
    let (mut s, sk) = account();
    let vault = s.create_vault("Personal").unwrap();
    let good = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("ok"))
        .unwrap();
    let account_id = s.account_id();
    let mut store = s.lock().into_store();

    // A record sealed under a key that is not the vault's key.
    let bad = ItemId::new_v7().unwrap();
    let wrong = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let body = RecordBody::Item(ItemCiphertext::seal(&wrong, &vault, &bad, 1, b"{}").unwrap());
    store
        .upsert_record(&vault, &bad, &body, Origin::Remote { server_seq: 1 })
        .unwrap();
    let mut s = LockedSession::open(store).unlock(&pw(), &sk).unwrap();
    assert_eq!(s.account_id(), account_id);
    assert_eq!(s.corrupt_records(), [(vault, bad)]);
    assert!(matches!(s.item(vault, bad), Err(Error::UnknownItem)));
    assert!(s.item(vault, good).is_ok());

    // A valid received version of the same record replaces the bad row.
    let device = DeviceId::new_v7().unwrap();
    let mut item = Item::new(
        &ItemType::SecureNote,
        device,
        Timestamp::from_unix_millis(1),
    )
    .unwrap();
    item.edit(device, Timestamp::from_unix_millis(1))
        .set_title("repaired")
        .unwrap();
    let vk = s.vaults.get(&vault).unwrap().current_key().unwrap();
    let rec = RemoteRecord {
        vault_id: vault,
        record_id: bad,
        body: RecordBody::Item(item.seal(vk, &vault, &bad).unwrap()),
        server_seq: 2,
    };
    assert_eq!(
        s.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::ReplacedUnreadable
    );
    assert_eq!(s.corrupt_records().len(), 0);
    assert_eq!(s.item(vault, bad).unwrap().title(), Some("repaired"));
}

#[test]
fn a_hardware_protected_vault_is_listed_but_unavailable() {
    let (mut s, _) = account();
    let vault = VaultId::new_v7().unwrap();
    // Its VK is wrapped under a TK, which this session never holds.
    let tk_like = AccountKey::generate().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let wrapped = vk.wrap(&tk_like, &s.account_id(), &vault).unwrap();
    s.store
        .create_vault_with(
            &vault,
            KeyGeneration::FIRST,
            VaultKeyWrap::TotpKey,
            &wrapped,
        )
        .unwrap();
    let mut s = s.reload();
    let summary = s
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .unwrap();
    assert!(!summary.available);
    assert!(summary.name.is_none());
    assert!(matches!(
        s.add_item(vault, &ItemType::Totp, |_| Ok(())),
        Err(Error::VaultUnavailable)
    ));
    assert!(matches!(
        s.rename_vault(vault, "x"),
        Err(Error::VaultUnavailable)
    ));
    assert!(matches!(
        s.rotate_vault_key(vault),
        Err(Error::VaultUnavailable)
    ));
    // A wrapping that does not open under AK fails authentication ...
    assert!(matches!(
        s.apply_remote_vault_key(vault, KeyGeneration::FIRST, &wrapped),
        Err(Error::Core(mosaic_core::Error::Decryption))
    ));
    // ... and one that does open is still refused: the vault's keys live
    // under its TK, which this session does not hold.
    let under_ak = vk.wrap(&s.account_key, &s.account_id(), &vault).unwrap();
    assert!(matches!(
        s.apply_remote_vault_key(vault, KeyGeneration::FIRST, &under_ak),
        Err(Error::VaultUnavailable)
    ));
    assert_eq!(
        s.store.vault_key(&vault, KeyGeneration::FIRST).unwrap(),
        Some(wrapped)
    );
}

#[test]
fn a_vault_key_that_does_not_open_leaves_the_vault_unavailable() {
    let (mut s, _) = account();
    let vault = VaultId::new_v7().unwrap();
    let other_ak = AccountKey::generate().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let wrapped = vk.wrap(&other_ak, &s.account_id(), &vault).unwrap();
    s.store
        .create_vault(&vault, KeyGeneration::FIRST, &wrapped)
        .unwrap();
    let mut s = s.reload();
    assert!(!s.vaults()[0].available);
    assert!(matches!(
        s.add_item(vault, &ItemType::Login, |e| e
            .set_secret(SecretField::Password, "x")),
        Err(Error::VaultUnavailable)
    ));
    let other = ItemId::new_v7().unwrap();
    assert!(matches!(s.item(vault, other), Err(Error::UnknownItem)));
}

/// A record of `vault` as another device would send it: a secure note
/// with `title`, sealed under the vault's current key.
fn remote_note(s: &UnlockedSession, vault: VaultId, id: ItemId, title: &str) -> RemoteRecord {
    let device = DeviceId::new_v7().unwrap();
    let at = Timestamp::from_unix_millis(1);
    let mut item = Item::new(&ItemType::SecureNote, device, at).unwrap();
    item.edit(device, at).set_title(title).unwrap();
    let vk = s.vaults.get(&vault).unwrap().current_key().unwrap();
    RemoteRecord {
        vault_id: vault,
        record_id: id,
        body: RecordBody::Item(item.seal(vk, &vault, &id).unwrap()),
        server_seq: 7,
    }
}

fn pending_ids(s: &UnlockedSession) -> Vec<ItemId> {
    s.pending_uploads(usize::MAX)
        .unwrap()
        .into_iter()
        .map(|r| r.unwrap().record_id)
        .collect()
}

/// Uploads everything (as M6 would), so nothing is pending.
fn mark_all_uploaded(s: &mut UnlockedSession) {
    for (seq, r) in (1..).zip(s.pending_uploads(usize::MAX).unwrap()) {
        let r = r.unwrap();
        s.mark_uploaded(r.vault_id, r.record_id, r.outbox_seq.unwrap(), seq)
            .unwrap();
    }
}

/// Another device rotated the vault to generation 2: the store accepts the
/// key and the cache follows, but nothing is re-sealed, as when the
/// re-seal was interrupted.
fn accept_without_reseal(s: &mut UnlockedSession, vault: VaultId) -> VaultKey {
    let g2 = KeyGeneration::FIRST.next().unwrap();
    let vk = VaultKey::generate(g2).unwrap();
    let wrapped = vk.wrap(&s.account_key, &s.account_id, &vault).unwrap();
    s.store.accept_vault_key(&vault, g2, &wrapped).unwrap();
    let copy = wrapped
        .unwrap_vault_key(&s.account_key, &s.account_id, &vault, g2)
        .unwrap();
    let ov = s.vaults.get_mut(&vault).unwrap();
    ov.keys.insert(g2, copy);
    ov.generation = g2;
    vk
}

#[test]
fn a_pending_corrupt_row_is_replaced_and_stays_queued() {
    // Review M2 (tests): a corrupt row that is in the outbox is replaced
    // by a valid received version with `Origin::Merged`, so the upload
    // intent is kept.
    let (mut s, sk) = account();
    let vault = s.create_vault("Personal").unwrap();
    let bad = ItemId::new_v7().unwrap();
    let wrong = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let body = RecordBody::Item(ItemCiphertext::seal(&wrong, &vault, &bad, 1, b"{}").unwrap());
    s.store
        .upsert_record(&vault, &bad, &body, Origin::Local)
        .unwrap();
    let mut s = LockedSession::open(s.lock().into_store())
        .unlock(&pw(), &sk)
        .unwrap();
    assert_eq!(s.corrupt_records(), [(vault, bad)]);
    assert_eq!(pending_ids(&s), [bad]);
    let rec = remote_note(&s, vault, bad, "valid");
    assert_eq!(
        s.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::ReplacedUnreadable
    );
    assert_eq!(s.corrupt_records(), []);
    assert_eq!(pending_ids(&s), [bad], "the upload intent is kept");
    let row = s.store.record(&vault, &bad).unwrap().unwrap();
    assert_eq!(row.body, rec.body);
    assert_eq!(row.server_seq, Some(7));
    assert_eq!(s.item(vault, bad).unwrap().title(), Some("valid"));
}

#[test]
fn a_row_the_store_rejects_is_listed_and_replaced() {
    // A row that fails the store's own checks (here an undecodable
    // ciphertext) is listed after unlock, and a valid received version
    // replaces it instead of failing on it forever.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let good = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("ok"))
        .unwrap();
    let bad = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("soon damaged"))
        .unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE records SET ciphertext = x'0102' WHERE record_id = ?1",
            [bad.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    assert_eq!(s.corrupt_records(), [(vault, bad)]);
    assert!(matches!(s.item(vault, bad), Err(Error::UnknownItem)));
    assert_eq!(s.item(vault, good).unwrap().title(), Some("ok"));
    assert!(matches!(
        s.store.record(&vault, &bad),
        Err(StorageError::Corrupt)
    ));

    let rec = remote_note(&s, vault, bad, "replacement");
    assert_eq!(
        s.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::ReplacedUnreadable
    );
    assert_eq!(s.corrupt_records(), []);
    assert_eq!(s.item(vault, bad).unwrap().title(), Some("replacement"));
    // The damaged row was a queued local change; the replacement stays
    // queued so nothing pending is dropped.
    assert!(pending_ids(&s).contains(&bad));
    assert_eq!(
        s.store.record(&vault, &bad).unwrap().unwrap().body,
        rec.body
    );
}

#[test]
fn damaged_key_or_metadata_rows_degrade_instead_of_failing_the_unlock() {
    let (mut s, _) = account();
    let named = s.create_vault("Named").unwrap();
    let keyless = s.create_vault("Keyless").unwrap();
    let before_damage = s.store.vault_meta(&named).unwrap().unwrap();
    let conn = s.store.conn_for_tests();
    conn.execute(
        "UPDATE vaults SET meta = x'0102' WHERE vault_id = ?1",
        [named.as_bytes()],
    )
    .unwrap();
    conn.execute(
        "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1",
        [keyless.as_bytes()],
    )
    .unwrap();
    let mut s = s.reload();
    assert!(summary(&s, named).available);
    assert_eq!(summary(&s, named).name, None);
    assert!(!summary(&s, keyless).available);

    // Review M2 (tests): renaming a vault whose metadata is unreadable
    // writes new metadata at the current generation, which survives a
    // reload ...
    s.rename_vault(named, "Renamed").unwrap();
    assert_eq!(summary(&s, named).name, Some("Renamed"));
    let mut s = s.reload();
    assert_eq!(summary(&s, named).name, Some("Renamed"));
    assert_eq!(
        s.store
            .vault_meta(&named)
            .unwrap()
            .unwrap()
            .key_generation(),
        summary(&s, named).key_generation
    );
    // ... and merges with the metadata from before the damage without
    // losing either name: the rename was written under a one-off writer
    // ID, so it does not claim to have seen the lost metadata. That
    // metadata carries an edit of an earlier session of this device; every
    // unlock writes under a fresh ID (ADR-0015), so it is merged like any
    // other device's edit.
    assert_eq!(
        s.apply_remote_vault_meta(named, &before_damage).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: false
        }
    );
    let meta = s.vault_meta(named).unwrap().unwrap();
    let conflicts = meta.conflicts();
    assert_eq!(conflicts.len(), 1);
    let mut vault_names: Vec<&str> = conflicts[0]
        .losers
        .iter()
        .chain([&conflicts[0].winner])
        .filter_map(|e| e.value().as_str())
        .collect();
    vault_names.sort_unstable();
    assert_eq!(vault_names, ["Named", "Renamed"]);
}

#[test]
fn received_metadata_replacing_an_unreadable_row_is_reported() {
    // Review M2 (crypto, fourth round): a damaged metadata row, or one
    // sealed under a generation whose key is not held, gives no rollback
    // anchor; the received metadata that replaces it is reported as
    // `ReplacedUnreadable` like a record (THREAT_MODEL R17), not as `New`.
    let (mut s, _) = account();
    let damaged = s.create_vault("Damaged").unwrap();
    let foreign = s.create_vault("Foreign key").unwrap();
    let absent = s.create_vault("Absent").unwrap();
    let from_server = |s: &UnlockedSession, vault: VaultId, name: &str| {
        let vk = s.vaults.get(&vault).unwrap().current_key().unwrap();
        VaultMeta::new(
            name,
            DeviceId::new_v7().unwrap(),
            Timestamp::from_unix_millis(1),
        )
        .unwrap()
        .seal(vk, &vault)
        .unwrap()
    };
    // Metadata that authenticates under no held key (another key of the
    // same generation, as after a lost rotation race).
    let wrong = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let sealed_wrong = VaultMeta::new(
        "x",
        DeviceId::new_v7().unwrap(),
        Timestamp::from_unix_millis(1),
    )
    .unwrap()
    .seal(&wrong, &foreign)
    .unwrap();
    s.store.set_vault_meta(&foreign, &sealed_wrong).unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vaults SET meta = x'0102' WHERE vault_id = ?1",
            [damaged.as_bytes()],
        )
        .unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vaults SET meta = NULL, meta_generation = NULL WHERE vault_id = ?1",
            [absent.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    for vault in [damaged, foreign, absent] {
        assert_eq!(summary(&s, vault).name, None);
    }
    for (vault, expected) in [
        (damaged, ApplyOutcome::ReplacedUnreadable),
        (foreign, ApplyOutcome::ReplacedUnreadable),
        (absent, ApplyOutcome::New),
    ] {
        let meta = from_server(&s, vault, "From the server");
        assert_eq!(s.apply_remote_vault_meta(vault, &meta).unwrap(), expected);
        assert_eq!(summary(&s, vault).name, Some("From the server"));
        // Now there is an anchor: the same metadata again is ignored.
        assert_eq!(
            s.apply_remote_vault_meta(vault, &meta).unwrap(),
            ApplyOutcome::Ignored
        );
    }
}

fn summary(s: &UnlockedSession, vault: VaultId) -> super::VaultSummary<'_> {
    s.vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
}

#[test]
fn a_damaged_current_key_is_repaired_by_a_wrapping_that_opens() {
    // Review M2 (crypto, tests): ADR-0010 item 9 makes such a vault
    // unavailable; the server's intact wrapping of the same generation
    // replaces the unreadable row and brings the vault back with its
    // metadata and records, also when the row is missing altogether.
    let (mut s, _) = account();
    let g1 = KeyGeneration::FIRST;
    let damaged = s.create_vault("Damaged").unwrap();
    let missing = s.create_vault("Missing").unwrap();
    let id = s
        .add_item(damaged, &ItemType::Login, |e| e.set_title("kept"))
        .unwrap();
    mark_all_uploaded(&mut s);
    let intact = s.store.vault_key(&damaged, g1).unwrap().unwrap();
    let intact_missing = s.store.vault_key(&missing, g1).unwrap().unwrap();
    let conn = s.store.conn_for_tests();
    conn.execute(
        "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1",
        [damaged.as_bytes()],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM vault_keys WHERE vault_id = ?1",
        [missing.as_bytes()],
    )
    .unwrap();
    let mut s = s.reload();
    assert!(!summary(&s, damaged).available);
    assert!(!summary(&s, missing).available);
    assert!(matches!(s.item(damaged, id), Err(Error::UnknownItem)));

    s.apply_remote_vault_key(damaged, g1, &intact).unwrap();
    s.apply_remote_vault_key(missing, g1, &intact_missing)
        .unwrap();
    for (vault, name) in [(damaged, "Damaged"), (missing, "Missing")] {
        let v = summary(&s, vault);
        assert!(v.available);
        assert_eq!(v.name, Some(name));
        assert_eq!(v.key_generation, g1);
    }
    assert_eq!(summary(&s, damaged).item_count, 1);
    assert_eq!(s.item(damaged, id).unwrap().title(), Some("kept"));
    assert_eq!(s.search("kept").len(), 1);
    assert_eq!(s.store.vault_key(&damaged, g1).unwrap(), Some(intact));
    assert_eq!(
        s.store.vault_key(&missing, g1).unwrap(),
        Some(intact_missing)
    );
    assert!(pending_ids(&s).is_empty(), "nothing to re-seal or upload");
    let s = s.reload();
    assert!(summary(&s, damaged).available && summary(&s, missing).available);
    assert_eq!(s.item(damaged, id).unwrap().title(), Some("kept"));
}

#[test]
fn a_peer_rotation_repairs_a_vault_whose_key_row_is_damaged() {
    // Review M2 (crypto): the newer generation of a peer's rotation is
    // accepted for a vault without its current key, so the peer's records
    // stop failing with `KeyGenerationNotAccepted`. Records sealed under
    // the lost key stay listed as corrupt until the peer's re-sealed
    // versions replace them.
    let (mut s, _) = account();
    let vault = s.create_vault("Named").unwrap();
    let old = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("old"))
        .unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1",
            [vault.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    let g2 = KeyGeneration::FIRST.next().unwrap();
    let wrapped2 = VaultKey::generate(g2)
        .unwrap()
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    s.apply_remote_vault_key(vault, g2, &wrapped2).unwrap();
    let v = summary(&s, vault);
    assert!(v.available);
    assert_eq!(v.key_generation, g2);
    assert_eq!(v.name, None, "the metadata was sealed under the lost key");
    assert_eq!(s.corrupt_records(), [(vault, old)]);

    let rec = remote_note(&s, vault, old, "re-sealed by the peer");
    assert_eq!(
        s.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::ReplacedUnreadable
    );
    assert_eq!(s.corrupt_records(), []);
    // The repaired vault keeps the generation rule, with the session's
    // error for a rollback.
    let wrapped1 = VaultKey::generate(KeyGeneration::FIRST)
        .unwrap()
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    assert!(matches!(
        s.apply_remote_vault_key(vault, KeyGeneration::FIRST, &wrapped1),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    let s = s.reload();
    assert_eq!(summary(&s, vault).key_generation, g2);
    assert_eq!(
        s.item(vault, old).unwrap().title(),
        Some("re-sealed by the peer")
    );
}

#[test]
fn a_repair_by_a_newer_generation_reseals_every_readable_older_record() {
    // Review M2, third round (tests): ADR-0012 item 7 on the repair path.
    // The vault is at generation 2 with readable generation-1 records and
    // metadata (a re-seal that never ran) and an intact generation-1 key
    // row, but its generation-2 row is damaged. A peer's generation-3
    // wrapping repairs it; every readable older record and the metadata
    // must then be re-sealed under generation 3 and queued, or peers that
    // already accepted generation 3 reject them as rollbacks.
    let (mut s, _) = account();
    let vault = s.create_vault("Older").unwrap();
    let a = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("a"))
        .unwrap();
    let b = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("b"))
        .unwrap();
    mark_all_uploaded(&mut s);
    accept_without_reseal(&mut s, vault);
    let g2 = KeyGeneration::FIRST.next().unwrap();
    let g3 = g2.next().unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1 AND generation = 2",
            [vault.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    assert!(!summary(&s, vault).available);
    assert_eq!(
        s.store.records_below_generation(&vault, g2).unwrap().len(),
        2
    );
    assert_eq!(pending_ids(&s), Vec::<ItemId>::new());

    let wrapped3 = VaultKey::generate(g3)
        .unwrap()
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    s.apply_remote_vault_key(vault, g3, &wrapped3).unwrap();
    let v = summary(&s, vault);
    assert!(v.available);
    assert_eq!(v.key_generation, g3);
    assert_eq!(v.name, Some("Older"));
    assert_eq!(
        s.store.records_below_generation(&vault, g3).unwrap().len(),
        0
    );
    let mut queued = pending_ids(&s);
    queued.sort();
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(queued, expected);
    assert_eq!(
        s.store
            .vault_meta(&vault)
            .unwrap()
            .unwrap()
            .key_generation(),
        g3
    );
    let s = s.reload();
    assert_eq!(s.item(vault, a).unwrap().title(), Some("a"));
    assert_eq!(s.item(vault, b).unwrap().title(), Some("b"));
    assert_eq!(s.corrupt_records(), []);
}

#[test]
fn repairing_a_damaged_key_keeps_the_generation_rule() {
    let (mut s, _) = account();
    let vault = s.create_vault("Rotated").unwrap();
    let g1 = KeyGeneration::FIRST;
    let wrapped1 = s.store.vault_key(&vault, g1).unwrap().unwrap();
    s.rotate_vault_key(vault).unwrap();
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1",
            [vault.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    assert!(!summary(&s, vault).available);
    assert!(matches!(
        s.apply_remote_vault_key(vault, g1, &wrapped1),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 1
        })
    ));
    assert!(!summary(&s, vault).available);
}

#[test]
fn a_repair_never_replaces_a_readable_key() {
    // Defensive path: the session lacks the current key although its row
    // opens. A different key of that generation is a conflict and the row
    // stays; the held key itself only reloads the vault.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let g1 = KeyGeneration::FIRST;
    let held = s.store.vault_key(&vault, g1).unwrap().unwrap();
    s.vaults.get_mut(&vault).unwrap().keys.clear();
    let other = VaultKey::generate(g1)
        .unwrap()
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    assert!(matches!(
        s.apply_remote_vault_key(vault, g1, &other),
        Err(Error::VaultKeyConflict { generation: 1 })
    ));
    assert_eq!(s.store.vault_key(&vault, g1).unwrap(), Some(held.clone()));
    assert!(!summary(&s, vault).available);
    s.apply_remote_vault_key(vault, g1, &held).unwrap();
    assert!(summary(&s, vault).available);
    assert_eq!(summary(&s, vault).name, Some("Personal"));
}

#[test]
fn store_refusals_of_a_rotation_are_session_errors() {
    // Review M2 (crypto): the store's rollback and conflict reach the
    // caller as the session's variants (CORE_API: a rollback is dropped,
    // never retried). Here the store is ahead of the cache.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let g2 = KeyGeneration::FIRST.next().unwrap();
    let g3 = g2.next().unwrap();
    let wrap = |s: &UnlockedSession, g| {
        VaultKey::generate(g)
            .unwrap()
            .wrap(&s.account_key, &s.account_id, &vault)
            .unwrap()
    };
    let other2 = wrap(&s, g2);
    s.store.accept_vault_key(&vault, g2, &other2).unwrap();
    assert!(matches!(
        s.rotate_vault_key(vault),
        Err(Error::VaultKeyConflict { generation: 2 })
    ));
    let other3 = wrap(&s, g3);
    s.store.accept_vault_key(&vault, g3, &other3).unwrap();
    assert!(matches!(
        s.rotate_vault_key(vault),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 3,
            offered: 2
        })
    ));

    // Review M2, third round: a vault deleted locally keeps its generation
    // floor (ADR-0010), and the server re-sharing it below the floor is the
    // session's rollback, not `Error::Store`, on the unknown-vault path too.
    s.store
        .delete_vault(&vault, crate::storage::PendingPolicy::Discard)
        .unwrap();
    let mut s = s.reload();
    assert!(s.vaults().is_empty());
    let stale = wrap(&s, g2);
    assert!(matches!(
        s.apply_remote_vault_key(vault, g2, &stale),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 3,
            offered: 2
        })
    ));
    assert!(s.vaults().is_empty());
    // At the floor it is accepted again.
    let fresh = wrap(&s, g3);
    s.apply_remote_vault_key(vault, g3, &fresh).unwrap();
    assert_eq!(summary(&s, vault).key_generation, g3);
}

#[test]
fn a_same_value_stamp_collision_is_stored_as_received_and_reported() {
    // Review M2 (tests): `merge_and_repair` reports a stamp collision but
    // has nothing to write back when both writers stored the same value
    // (only the write time differs) and the received payload wins the
    // tie-break. The result equals the received record, so it is stored
    // as received, nothing is queued, and the collision is reported.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let shared = DeviceId::new_v7().unwrap();
    let id = ItemId::new_v7().unwrap();
    let base = Item::new(&ItemType::Login, shared, Timestamp::from_unix_millis(1)).unwrap();
    let copy = || Item::from_json(base.to_json().unwrap().expose_secret()).unwrap();
    let (mut early, mut late) = (copy(), copy());
    early
        .edit(shared, Timestamp::from_unix_millis(10))
        .set_title("same")
        .unwrap();
    late.edit(shared, Timestamp::from_unix_millis(20))
        .set_title("same")
        .unwrap();
    let vk = s.vaults[&vault].current_key().unwrap();
    let record = |item: &Item, seq| RemoteRecord {
        vault_id: vault,
        record_id: id,
        body: RecordBody::Item(item.seal(vk, &vault, &id).unwrap()),
        server_seq: seq,
    };
    let (first, second) = (record(&early, 1), record(&late, 2));
    assert_eq!(s.apply_remote_record(&first).unwrap(), ApplyOutcome::New);
    assert_eq!(
        s.apply_remote_record(&second).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: true
        }
    );
    assert_eq!(s.item(vault, id).unwrap(), &late);
    assert_eq!(
        s.store.record(&vault, &id).unwrap().unwrap().body,
        second.body
    );
    assert!(pending_ids(&s).is_empty(), "nothing is queued");
}

#[test]
fn a_same_value_metadata_stamp_collision_is_stored_as_received_and_reported() {
    // Review M2, third round (tests): the metadata analogue of the item
    // test above. Two writers under one device ID set the same name at
    // t=10 and t=20: the merge equals the received version, so it is stored
    // as received, and the collision is reported as `Merged`, never as
    // `FastForward`.
    let (mut s, _) = account();
    let vault = VaultId::new_v7().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let wrapped = vk.wrap(&s.account_key, &s.account_id, &vault).unwrap();
    s.store
        .create_vault(&vault, KeyGeneration::FIRST, &wrapped)
        .unwrap();
    let mut s = s.reload();
    assert_eq!(summary(&s, vault).name, None);
    let shared = DeviceId::new_v7().unwrap();
    let base = VaultMeta::new("Base", shared, Timestamp::from_unix_millis(1)).unwrap();
    let copy = || VaultMeta::from_json(base.to_json().unwrap().expose_secret()).unwrap();
    let (mut early, mut late) = (copy(), copy());
    early
        .edit(shared, Timestamp::from_unix_millis(10))
        .set_name("same")
        .unwrap();
    late.edit(shared, Timestamp::from_unix_millis(20))
        .set_name("same")
        .unwrap();
    let vk = s.vaults[&vault].current_key().unwrap();
    let (first, second) = (
        early.seal(vk, &vault).unwrap(),
        late.seal(vk, &vault).unwrap(),
    );
    assert_eq!(
        s.apply_remote_vault_meta(vault, &first).unwrap(),
        ApplyOutcome::New
    );
    assert_eq!(
        s.apply_remote_vault_meta(vault, &second).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: true
        }
    );
    assert_eq!(s.vault_meta(vault).unwrap().unwrap(), &late);
    assert_eq!(summary(&s, vault).name, Some("same"));
    assert_eq!(s.store.vault_meta(&vault).unwrap().unwrap(), second);
}

#[test]
fn a_password_check_against_another_account_key_fails() {
    // Review M2 (tests): `change_master_password` compares the key the
    // current password unwraps with the session's account key. A stored
    // wrapping of another key (same password, Secret Key and profile) is
    // refused, and the stored parameters stay as they are.
    let (mut s, sk) = account();
    let params = s.store.account().unwrap().unwrap();
    let other = AccountKey::generate().unwrap();
    let foreign =
        mosaic_core::change_master_password(&other, &pw(), &sk, &params, params.profile).unwrap();
    s.store
        .update_password(&foreign.params, &foreign.wrapped_account_key)
        .unwrap();
    let new_password = MasterPassword::new("another password".to_owned());
    assert!(matches!(
        s.change_master_password(&pw(), &new_password, &sk),
        Err(Error::UnlockFailed)
    ));
    assert_eq!(s.store.account().unwrap().unwrap(), foreign.params);
}

#[test]
fn a_failed_load_returns_the_locked_session() {
    // Review M2 (tests): the account key derived, then loading the state
    // failed; the store must come back as a locked session.
    let (s, _) = account();
    let UnlockedSession {
        store,
        account_id,
        account_key,
        ..
    } = s;
    store
        .conn_for_tests()
        .execute_batch("DROP TABLE local_device;")
        .unwrap();
    let err = UnlockedSession::load(store, account_id, account_key).unwrap_err();
    assert!(matches!(err.error, Error::Store(StorageError::Database(_))));
    assert!(err.session.has_account().unwrap());
    assert_eq!(
        err.session.store().account().unwrap().unwrap().account_id,
        account_id
    );
}

#[test]
fn an_equal_version_under_a_newer_generation_replaces_the_old_ciphertext() {
    // Review M2 (tests): the Equal-and-equal branch. A peer re-sealed an
    // unchanged item under generation 2 while this device's re-seal was
    // interrupted: the received ciphertext replaces the generation-1 row,
    // nothing is queued.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let id = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("same"))
        .unwrap();
    mark_all_uploaded(&mut s);
    let vk2 = accept_without_reseal(&mut s, vault);
    let body = super::seal_record(s.item(vault, id).unwrap(), &vk2, &vault, &id).unwrap();
    let rec = RemoteRecord {
        vault_id: vault,
        record_id: id,
        body,
        server_seq: 9,
    };
    assert_eq!(
        s.store
            .record(&vault, &id)
            .unwrap()
            .unwrap()
            .body
            .key_generation(),
        KeyGeneration::FIRST
    );
    assert_eq!(s.apply_remote_record(&rec).unwrap(), ApplyOutcome::Ignored);
    let row = s.store.record(&vault, &id).unwrap().unwrap();
    assert_eq!(row.body, rec.body);
    assert_eq!(row.server_seq, Some(9));
    assert!(pending_ids(&s).is_empty(), "nothing is queued");
}

#[test]
fn an_equal_version_does_not_replace_a_pending_row() {
    // The `!pending` arm: the local row is a queued change; its ciphertext
    // is kept (it is re-sealed at the next unlock, not replaced).
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let id = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("queued"))
        .unwrap();
    let vk2 = accept_without_reseal(&mut s, vault);
    let before = s.store.record(&vault, &id).unwrap().unwrap();
    let body = super::seal_record(s.item(vault, id).unwrap(), &vk2, &vault, &id).unwrap();
    let rec = RemoteRecord {
        vault_id: vault,
        record_id: id,
        body,
        server_seq: 9,
    };
    assert_eq!(s.apply_remote_record(&rec).unwrap(), ApplyOutcome::Ignored);
    assert_eq!(s.store.record(&vault, &id).unwrap().unwrap(), before);
    assert_eq!(pending_ids(&s), [id]);
}

#[test]
fn an_interrupted_rotation_is_finished_at_unlock() {
    // Review M2 (crypto): records left below the accepted generation (a
    // rotation or key acceptance interrupted before every record was
    // re-sealed) are re-sealed and queued at the next unlock, metadata
    // included.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let a = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("a"))
        .unwrap();
    let b = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("b"))
        .unwrap();
    s.purge_item(vault, b).unwrap();
    mark_all_uploaded(&mut s);
    accept_without_reseal(&mut s, vault);
    let g2 = KeyGeneration::FIRST.next().unwrap();
    assert_eq!(
        s.store.records_below_generation(&vault, g2).unwrap().len(),
        2
    );
    let s = s.reload();
    assert_eq!(
        s.store.records_below_generation(&vault, g2).unwrap().len(),
        0
    );
    let mut queued = pending_ids(&s);
    queued.sort();
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(queued, expected);
    assert!(matches!(
        s.store.record(&vault, &b).unwrap().unwrap().body,
        RecordBody::Tombstone(_)
    ));
    assert_eq!(
        s.store
            .vault_meta(&vault)
            .unwrap()
            .unwrap()
            .key_generation(),
        g2
    );
    assert_eq!(s.vaults()[0].name, Some("Personal"));
    assert_eq!(s.item(vault, a).unwrap().title(), Some("a"));
}

#[test]
fn another_wrapping_of_the_held_key_is_a_no_op() {
    // The session compares the unwrapped key (constant time), so a fresh
    // wrapping of the key it holds is accepted as nothing new, while the
    // store alone, which sees only wrappings, would refuse it.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let g1 = KeyGeneration::FIRST;
    let stored = s.store.vault_key(&vault, g1).unwrap().unwrap();
    let again = s.vaults[&vault].keys[&g1]
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    assert_ne!(again, stored);
    s.apply_remote_vault_key(vault, g1, &again).unwrap();
    assert_eq!(s.store.vault_key(&vault, g1).unwrap(), Some(stored));
    assert!(matches!(
        s.store.accept_vault_key(&vault, g1, &again),
        Err(StorageError::VaultKeyConflict { generation: 1 })
    ));
    // A different key of the held generation is a conflict.
    let other = VaultKey::generate(g1)
        .unwrap()
        .wrap(&s.account_key, &s.account_id, &vault)
        .unwrap();
    assert!(matches!(
        s.apply_remote_vault_key(vault, g1, &other),
        Err(Error::VaultKeyConflict { generation: 1 })
    ));
}

/// Review M2, third round (tests): the hand-written `Debug` impls of the
/// secret-holding session types print no item content, vault name, account
/// key or vault key (CLAUDE.md: secrets are never Debug-printable).
#[test]
fn session_debug_output_holds_no_secrets() {
    let (mut s, sk) = account();
    let vault = s.create_vault("Mark3r Vault Name").unwrap();
    let item = s
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("Mark3r Title")?;
            e.set_text(TextField::Username, "mark3r-user")?;
            e.set_secret(SecretField::Password, "Mark3r-Passw0rd")
        })
        .unwrap();
    s.update_item(vault, item, |e| {
        e.set_secret(SecretField::Password, "Mark3r-Newer")
    })
    .unwrap();
    // The key types are opaque outside mosaic-core; the Secret Key is the
    // one whose bytes are visible here. Every output is also pinned
    // exactly, so no other field (keys, items, the index) can slip in.
    let secret_key_hex = sk.expose_secret().iter().fold(String::new(), |mut hex, b| {
        use core::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
        hex
    });
    let check = |shown: &str| {
        for marker in ["Mark3r", "mark3r", &secret_key_hex, &secret_key_hex[..8]] {
            assert!(!shown.contains(marker), "{shown}");
        }
    };
    let shown = format!("{s:?}");
    check(&shown);
    check(&format!("{s:#?}"));
    assert_eq!(
        shown,
        format!(
            "UnlockedSession {{ account_id: {:?}, device: {:?}, vaults: 1, .. }}",
            s.account_id(),
            s.device_id()
        )
    );

    let locked = s.lock();
    check(&format!("{locked:#?}"));
    assert_eq!(format!("{locked:?}"), "LockedSession { .. }");

    let err = locked
        .unlock(&MasterPassword::new("Mark3r wrong".to_owned()), &sk)
        .unwrap_err();
    check(&format!("{err:#?} {err}"));
    assert_eq!(
        format!("{err:?}"),
        "UnlockError { error: UnlockFailed, .. }"
    );
    assert!(matches!(err.error, Error::UnlockFailed));
}

#[test]
fn an_unlock_error_that_is_not_a_wrong_secret_keeps_its_variant() {
    // `derive_account_key` maps only `Decryption` to `UnlockFailed`; any
    // other core error (here an empty password) reaches the caller as is.
    let (s, sk) = account();
    let err = s
        .lock()
        .unlock(&MasterPassword::new(String::new()), &sk)
        .unwrap_err();
    assert!(matches!(
        err.error,
        Error::Core(mosaic_core::Error::EmptyPassword)
    ));
    assert!(err.session.has_account().unwrap());
}

#[test]
fn dup_item_copies_a_deleted_item_as_its_tombstone() {
    // Today every caller passes a live item; the tombstone branch keeps
    // `dup_item` total.
    let device = DeviceId::new_v7().unwrap();
    let at = Timestamp::from_unix_millis(5);
    let mut item = Item::new(&ItemType::Login, device, at).unwrap();
    item.edit(device, at).set_title("gone").unwrap();
    item.delete(device, at).unwrap();
    let copy = super::dup_item(&item).unwrap();
    assert!(copy.is_deleted());
    assert_eq!(copy, item);
    let live = Item::new(&ItemType::SecureNote, device, at).unwrap();
    assert_eq!(super::dup_item(&live).unwrap(), live);
}

/// Another device of the same account (in-memory store) that holds every
/// vault key and metadata of `a`, as M6 sync would deliver them.
fn join(a: &UnlockedSession, sk: &SecretKey) -> UnlockedSession {
    let params = a.store.account().unwrap().unwrap();
    let wrapped = a
        .store
        .unlock_method(&mosaic_core::UnlockMethod::Password)
        .unwrap()
        .unwrap()
        .wrapped_account_key;
    let mut b = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account(&params, &wrapped, &pw(), sk)
        .unwrap();
    for v in a.vaults() {
        for (g, w) in a.store.vault_keys(&v.vault_id).unwrap() {
            b.apply_remote_vault_key(v.vault_id, g, &w).unwrap();
        }
        if let Some(meta) = a.store.vault_meta(&v.vault_id).unwrap() {
            b.apply_remote_vault_meta(v.vault_id, &meta).unwrap();
        }
    }
    b
}

/// Every queued record of `s` as the server would deliver it.
fn uploads(s: &mut UnlockedSession) -> Vec<RemoteRecord> {
    let mut out = Vec::new();
    for (seq, r) in (100..).zip(s.pending_uploads(usize::MAX).unwrap()) {
        let r = r.unwrap();
        s.mark_uploaded(r.vault_id, r.record_id, r.outbox_seq.unwrap(), seq)
            .unwrap();
        out.push(RemoteRecord {
            vault_id: r.vault_id,
            record_id: r.record_id,
            body: r.body,
            server_seq: seq,
        });
    }
    out
}

/// Two sessions forced onto one writer ID. Every unlock writes under a
/// fresh ID (ADR-0015), so only a forged or corrupted record (or a UUIDv7
/// collision) can still carry this session's stamps; the detection and the
/// recovery are kept as defence in depth.
#[test]
fn a_reused_device_id_is_detected_and_both_writers_edits_survive() {
    let (mut a, sk) = account();
    let vault = a.create_vault("Personal").unwrap();
    let item = a
        .add_item(vault, &ItemType::Login, |e| e.set_title("From A"))
        .unwrap();
    let first = uploads(&mut a);
    let mut b = join(&a, &sk);
    b.device = a.device;
    b.apply_remote_record(&first[0]).unwrap();
    b.update_item(vault, item, |e| e.set_title("Edited on B"))
        .unwrap();
    let rec = uploads(&mut b).pop().unwrap();

    // A sees an edit under its own ID that it never made.
    assert!(matches!(
        a.apply_remote_record(&rec),
        Err(Error::Core(mosaic_core::Error::DeviceIdReused))
    ));
    assert_eq!(a.item(vault, item).unwrap().title(), Some("From A"));
    let old = a.device_id();
    let new = a.rotate_device_id().unwrap();
    assert_ne!(old, new);
    assert_eq!(a.store.device_id().unwrap(), Some(new));
    // Review M2, third round: the old ID stays suspect, so the merge keeps
    // A's own value too (it may have been a different edit under the same
    // stamp) instead of letting B's vector silently cover it.
    assert_eq!(
        a.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: true
        }
    );
    let merged = a.item(vault, item).unwrap();
    assert_eq!(merged.title(), Some("Edited on B"));
    let conflicts = merged.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].losers[0].value().as_str(), Some("From A"));
}

#[test]
fn one_edit_each_under_a_shared_device_id_is_detected() {
    // Review M2 (crypto): with one edit each from the same state the
    // counters stay equal, so only the colliding stamp shows the reuse.
    let (mut a, sk) = account();
    let vault = a.create_vault("Personal").unwrap();
    let item = a
        .add_item(vault, &ItemType::Login, |e| e.set_title("Base"))
        .unwrap();
    let first = uploads(&mut a);
    let mut b = join(&a, &sk);
    b.device = a.device;
    b.apply_remote_record(&first[0]).unwrap();
    a.update_item(vault, item, |e| e.set_title("title from A"))
        .unwrap();
    b.update_item(vault, item, |e| e.set_title("title from B"))
        .unwrap();
    let rec = uploads(&mut b).pop().unwrap();
    assert!(matches!(
        a.apply_remote_record(&rec),
        Err(Error::Core(mosaic_core::Error::DeviceIdReused))
    ));
    assert_eq!(a.item(vault, item).unwrap().title(), Some("title from A"));
    a.rotate_device_id().unwrap();
    assert_eq!(
        a.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: true
        }
    );
    let conflicts = a.item(vault, item).unwrap().conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].key, "title");
}

#[test]
fn edits_of_different_fields_under_a_shared_id_lose_nothing() {
    // Review M2, third round (sync, high), at the session level: the two
    // writers edit different fields from the same state, so the vectors
    // are equal and the pure merge drops both edits. A third device and
    // both writers (after rotating) keep every value.
    let (mut a, sk) = account();
    let vault = a.create_vault("Personal").unwrap();
    let item = a
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("t0")?;
            e.set_secret(SecretField::Password, "p0")
        })
        .unwrap();
    let first = uploads(&mut a);
    let mut b = join(&a, &sk);
    let mut third = join(&a, &sk);
    b.device = a.device;
    for s in [&mut b, &mut third] {
        s.apply_remote_record(&first[0]).unwrap();
    }
    a.update_item(vault, item, |e| e.set_secret(SecretField::Password, "p1"))
        .unwrap();
    b.update_item(vault, item, |e| e.set_title("t1")).unwrap();
    let rec_a = uploads(&mut a).pop().unwrap();
    let rec_b = uploads(&mut b).pop().unwrap();
    let values = |s: &UnlockedSession, key: &str| -> Vec<String> {
        let mut v: Vec<String> = s
            .item(vault, item)
            .unwrap()
            .entries(key)
            .filter_map(|e| e.value().as_str().map(str::to_owned))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    assert_eq!(
        third.apply_remote_record(&rec_a).unwrap(),
        ApplyOutcome::FastForward
    );
    assert_eq!(
        third.apply_remote_record(&rec_b).unwrap(),
        ApplyOutcome::Merged {
            restored_fields: 0,
            stamp_collision: true
        }
    );
    assert_eq!(values(&third, "title"), ["t0", "t1"]);
    assert_eq!(values(&third, "login.password"), ["p0", "p1"]);
    for (s, rec) in [(&mut a, &rec_b), (&mut b, &rec_a)] {
        assert!(matches!(
            s.apply_remote_record(rec),
            Err(Error::Core(mosaic_core::Error::DeviceIdReused))
        ));
        s.rotate_device_id().unwrap();
        assert!(matches!(
            s.apply_remote_record(rec).unwrap(),
            ApplyOutcome::Merged {
                stamp_collision: true,
                ..
            }
        ));
        assert_eq!(values(s, "title"), ["t0", "t1"]);
        assert_eq!(values(s, "login.password"), ["p0", "p1"]);
    }
}

#[test]
fn one_rename_each_under_a_shared_device_id_keeps_both_names() {
    // Review M2 (crypto, sync, tests): vault metadata merges with the
    // repairing merge, so a rename under a shared device ID is detected
    // like an item edit (`DeviceIdReused` for the writer that shares the
    // ID), and both names survive as a conflict.
    let (mut a, sk) = account();
    let vault = a.create_vault("Base").unwrap();
    let mut b = join(&a, &sk);
    b.device = a.device;
    a.rename_vault(vault, "from A").unwrap();
    b.rename_vault(vault, "from B").unwrap();
    let from_a = a.store.vault_meta(&vault).unwrap().unwrap();
    let from_b = b.store.vault_meta(&vault).unwrap().unwrap();
    let names = |s: &UnlockedSession| -> Vec<String> {
        let meta = s.vault_meta(vault).unwrap().unwrap();
        let mut out: Vec<String> = meta
            .conflicts()
            .iter()
            .flat_map(|c| c.losers.iter().chain([&c.winner]))
            .filter_map(|e| e.value().as_str().map(str::to_owned))
            .collect();
        if out.is_empty() {
            out.extend(meta.name().map(str::to_owned));
        }
        out.sort();
        out
    };
    for (s, received, own) in [(&mut a, &from_b, "from A"), (&mut b, &from_a, "from B")] {
        assert!(matches!(
            s.apply_remote_vault_meta(vault, received),
            Err(Error::Core(mosaic_core::Error::DeviceIdReused))
        ));
        assert_eq!(names(s), [own]);
        s.rotate_device_id().unwrap();
        assert_eq!(
            s.apply_remote_vault_meta(vault, received).unwrap(),
            ApplyOutcome::Merged {
                restored_fields: 0,
                stamp_collision: true
            }
        );
        assert_eq!(names(s), ["from A", "from B"]);
    }
}

#[test]
fn a_late_upload_ack_after_a_newer_pull_is_not_an_error() {
    // Review M2, third round (sync): a local edit is uploaded (the server
    // assigns seq 15), and before the ack is processed a concurrent remote
    // version with seq 20 is merged and queued anew. The late ack for the
    // first upload used to fail with `StaleServerSeq`, which an M6 loop
    // could treat as fatal; it now returns `false` and changes nothing.
    let (mut s, _) = account();
    let vault = s.create_vault("Personal").unwrap();
    let id = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("mine"))
        .unwrap();
    mark_all_uploaded(&mut s);
    s.update_item(vault, id, |e| e.set_text(TextField::Username, "local"))
        .unwrap();
    let first = s.pending_uploads(1).unwrap().remove(0).unwrap();
    let first_seq = first.outbox_seq.unwrap();
    // Another device's edit on top of the uploaded version, pulled before
    // the ack: the local change is still pending, so the result is queued
    // anew.
    let mut remote = super::open_record(&first.body, &s.vaults[&vault].keys, &vault, &id).unwrap();
    let other = DeviceId::new_v7().unwrap();
    remote
        .edit(other, Timestamp::from_unix_millis(5))
        .set_title("theirs")
        .unwrap();
    let vk = s.vaults[&vault].current_key().unwrap();
    let rec = RemoteRecord {
        vault_id: vault,
        record_id: id,
        body: RecordBody::Item(remote.seal(vk, &vault, &id).unwrap()),
        server_seq: 20,
    };
    assert!(matches!(
        s.apply_remote_record(&rec).unwrap(),
        ApplyOutcome::Merged { .. } | ApplyOutcome::FastForward
    ));
    let row = s.store.record(&vault, &id).unwrap().unwrap();
    assert_eq!(row.server_seq, Some(20));
    let queued = row.outbox_seq.unwrap();
    assert_ne!(queued, first_seq);
    assert!(!s.mark_uploaded(vault, id, first_seq, 15).unwrap());
    let row = s.store.record(&vault, &id).unwrap().unwrap();
    assert_eq!((row.server_seq, row.outbox_seq), (Some(20), Some(queued)));
}

#[test]
fn vault_summaries_do_not_print_names() {
    let (mut s, _) = account();
    s.create_vault("Very Private Vault Name").unwrap();
    let printed = format!("{:?}", s.vaults());
    assert!(!printed.contains("Very Private"), "{printed}");
    assert!(printed.contains("has_name: true"), "{printed}");
}

impl UnlockedSession {
    /// Locks and unlocks again with the same account key (test shortcut
    /// that skips Argon2id).
    fn reload(self) -> Self {
        let Self {
            store,
            account_id,
            account_key,
            ..
        } = self;
        Self::load(store, account_id, account_key).unwrap()
    }
}
