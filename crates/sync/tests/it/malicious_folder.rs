//! A hostile folder: whoever can write to the synced directory (a
//! compromised cloud account, a malicious provider, a stale restore) must
//! change nothing without the account key, and what it withholds or rolls
//! back must show (CRYPTO_SPEC §10, §10.1, THREAT_MODEL).

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration tests"
)]

use std::fs;
use std::path::{Path, PathBuf};

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{
    AccountKey, AttachmentId, ItemCiphertext, ItemId, KeyGeneration, TombstoneCiphertext, VaultId,
    VaultKey,
};
use mosaic_sync::engine::{EngineConfig, SyncEngine, SyncError, SyncWarning};
use mosaic_sync::testutil::{FixedJitter, MaliciousTransport};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::folder_names::segment_name;
use mosaic_sync::wire::segment::{Segment, SegmentObject, SegmentPath};
use mosaic_sync::wire::{KeyedCrypto, WireError};

use crate::common::{World, account_dir, device_dir, is_idle, log_dir, open_transport, segments};

const HOUR_MS: u64 = 3_600_000;

fn add_login(w: &mut World, d: usize, vault: VaultId, title: &str) -> ItemId {
    w.devs[d]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, "pw")
        })
        .unwrap()
}

fn password_of(w: &World, d: usize, vault: VaultId, item: ItemId) -> String {
    w.devs[d]
        .s()
        .item(vault, item)
        .unwrap()
        .secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

/// What an item shows: its title and password.
fn fingerprint(w: &World, d: usize, vault: VaultId, item: ItemId) -> (String, String) {
    let it = w.devs[d].s().item(vault, item).unwrap();
    (
        it.title().unwrap_or_default().to_owned(),
        password_of(w, d, vault, item),
    )
}

/// Two devices at rest sharing one vault with one item.
fn two_devices(seed: u64) -> (World, VaultId, ItemId) {
    let mut w = World::new(seed, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.converge(0);
    (w, vault, item)
}

fn next_seq(dir: &Path) -> u64 {
    segments(dir).last().map_or(0, |p| {
        let n = p.file_name().unwrap().to_string_lossy().into_owned();
        u64::from_str_radix(n.trim_end_matches(".mseg"), 16).unwrap()
    }) + 1
}

/// A segment as an attacker without the account key writes it.
fn forged_segment(
    w: &World,
    vault: VaultId,
    writer: usize,
    seq: u64,
    objects: Vec<SegmentObject>,
    crypto: &KeyedCrypto,
) -> Vec<u8> {
    Segment {
        snapshot: None,
        account_id: w.account,
        vault_id: vault,
        device_id: w.devs[writer].s().install_id(),
        seq,
        created_at_ms: 1,
        objects,
    }
    .encode(crypto)
    .unwrap()
}

#[test]
fn replayed_old_segment_changes_nothing() {
    let (mut w, vault, item) = two_devices(31);
    let dir = log_dir(&w, 1, 0, vault);
    let old = segments(&dir).pop().unwrap();
    let old_bytes = fs::read(&old).unwrap();
    // A newer version of the item, which B takes.
    w.devs[0]
        .s_mut()
        .update_item(vault, item, |e| {
            e.set_secret(SecretField::Password, "newer")
        })
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.sync(1);
    assert_eq!(password_of(&w, 1, vault, item), "newer");
    let before = fingerprint(&w, 1, vault, item);

    // The old file is put back (same name): already read.
    fs::write(&old, &old_bytes).unwrap();
    // And the old bytes under a new sequence number: refused, because the
    // file says it is another sequence.
    let replayed = dir.join(segment_name(next_seq(&dir)));
    fs::write(&replayed, &old_bytes).unwrap();
    let r = w.sync(1);
    assert_eq!(r.pulled, 0, "{r:?}");
    assert_eq!(fingerprint(&w, 1, vault, item), before);

    // Even if B forgets what it read, the old version changes nothing.
    w.devs[1].s_mut().set_sync_cursor(vault, &[]).unwrap();
    let r = w.sync(1);
    assert_eq!(r.pulled, 0, "an old version is ignored: {r:?}");
    assert_eq!(fingerprint(&w, 1, vault, item), before);
    assert_eq!(password_of(&w, 1, vault, item), "newer");
    assert_eq!(w.devs[1].s().corrupt_records().len(), 0);
}

#[test]
fn segment_copied_to_other_device_dir_rejected() {
    let (mut w, vault, _item) = two_devices(32);
    let new_item = add_login(&mut w, 0, vault, "secret new item");
    w.sync(0);
    w.cloud.settle().unwrap();
    // The attacker moves A's newest segment into B's own log directory
    // (and removes it from A's).
    let a_dir = log_dir(&w, 1, 0, vault);
    let b_dir = log_dir(&w, 1, 1, vault);
    let moved = segments(&a_dir).pop().unwrap();
    let bytes = fs::read(&moved).unwrap();
    fs::create_dir_all(&b_dir).unwrap();
    fs::remove_file(&moved).unwrap();
    let name = moved.file_name().unwrap().to_owned();
    fs::write(b_dir.join(&name), &bytes).unwrap();

    // The file says it is A's, filed under B's directory: refused.
    let keys = KeyedCrypto::from_keys(w.devs[1].s().sync_keys().unwrap());
    let seq = u64::from_str_radix(name.to_string_lossy().trim_end_matches(".mseg"), 16).unwrap();
    let decoded = Segment::decode(
        &bytes,
        &SegmentPath {
            account_id: w.account,
            vault_id: vault,
            device_id: w.devs[1].s().install_id(),
            seq,
        },
        &keys,
    );
    assert_eq!(decoded.unwrap_err(), WireError::Path);

    // B's own log is not read by B; a third reader (A's view) rejects it too,
    // and nothing of the item reaches B.
    let r = w.sync(1);
    assert_eq!(r.pulled, 0);
    assert!(w.devs[1].s().item(vault, new_item).is_err());
    let r = w.sync(0);
    assert_eq!(r.pulled, 0);
}

#[test]
fn segment_without_ak_rejected() {
    let (mut w, vault, item) = two_devices(33);
    let dir = log_dir(&w, 1, 0, vault);
    let seq = next_seq(&dir);
    // An attacker with write access but another key: a well-formed segment
    // with a new item, a bogus vault key and a metadata object.
    let attacker_ak = AccountKey::generate().unwrap();
    let crypto = KeyedCrypto::new(&attacker_ak, w.account, w.devs[0].s().install_id()).unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let evil = ItemId::new_v7().unwrap();
    let wrapped = vk.wrap(&attacker_ak, &w.account, &vault).unwrap();
    let bytes = forged_segment(
        &w,
        vault,
        0,
        seq,
        vec![
            SegmentObject::VaultKey {
                generation: KeyGeneration::FIRST.next().unwrap(),
                wrapped,
            },
            SegmentObject::Item {
                record_id: evil,
                ciphertext: ItemCiphertext::seal(&vk, &vault, &evil, 1, b"{}").unwrap(),
            },
        ],
        &crypto,
    );
    fs::write(dir.join(segment_name(seq)), bytes).unwrap();
    let r = w.sync(1);
    assert!(is_idle(&r), "{r:?}");
    assert_eq!(r.warnings.len(), 0);
    assert!(w.devs[1].s().item(vault, evil).is_err());
    assert_eq!(
        w.devs[1]
            .s()
            .vaults()
            .iter()
            .find(|v| v.vault_id == vault)
            .unwrap()
            .key_generation,
        KeyGeneration::FIRST
    );
    assert_eq!(password_of(&w, 1, vault, item), "pw");
}

#[test]
fn swapped_record_between_vaults_fails_auth() {
    let (mut w, vault_a, item) = two_devices(34);
    let vault_b = w.new_vault(0, "Other");
    add_login(&mut w, 0, vault_b, "in other");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.sync(1);

    // Someone with the folder key material (a rogue device) files the
    // ciphertext of a vault-A item in vault B's log, and a vault-A item's
    // ciphertext under another item's ID in vault A's log. The MAC verifies,
    // the AEAD does not.
    let ciphertext = match w.devs[0].s().store().records(&vault_a).unwrap()[0]
        .as_ref()
        .unwrap()
        .body
        .clone()
    {
        mosaic_vault::storage::RecordBody::Item(c) => c,
        mosaic_vault::storage::RecordBody::Tombstone(_) => panic!("an item"),
    };
    let crypto = KeyedCrypto::from_keys(w.devs[0].s().sync_keys().unwrap());
    let dir_b = log_dir(&w, 1, 0, vault_b);
    let seq_b = next_seq(&dir_b);
    fs::write(
        dir_b.join(segment_name(seq_b)),
        forged_segment(
            &w,
            vault_b,
            0,
            seq_b,
            vec![SegmentObject::Item {
                record_id: item,
                ciphertext: ciphertext.clone(),
            }],
            &crypto,
        ),
    )
    .unwrap();
    let other_id = ItemId::new_v7().unwrap();
    let dir_a = log_dir(&w, 1, 0, vault_a);
    let seq_a = next_seq(&dir_a);
    fs::write(
        dir_a.join(segment_name(seq_a)),
        forged_segment(
            &w,
            vault_a,
            0,
            seq_a,
            vec![SegmentObject::Item {
                record_id: other_id,
                ciphertext,
            }],
            &crypto,
        ),
    )
    .unwrap();

    let r = w.sync(1);
    assert_eq!(r.pulled, 0, "{r:?}");
    assert_eq!(r.undecryptable, 2, "{r:?}");
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::Undecryptable { count: 2 })),
        "{r:?}"
    );
    assert!(w.devs[1].s().item(vault_b, item).is_err());
    assert!(w.devs[1].s().item(vault_a, other_id).is_err());
    assert_eq!(password_of(&w, 1, vault_a, item), "pw");
}

#[test]
fn withheld_device_log_freeze_after_grace() {
    let (mut w, vault, item) = two_devices(35);
    // B reads through a transport that withholds every pull while A's
    // checkpoints keep arriving.
    let mut engine = SyncEngine::with_jitter(
        MaliciousTransport::new(open_transport(
            w.devs[1].s(),
            &w.replica(1),
            &w.clock,
            "device 1",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    engine.transport_mut().withhold_pulls = true;
    w.devs[0]
        .s_mut()
        .update_item(vault, item, |e| {
            e.set_secret(SecretField::Password, "hidden")
        })
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();

    let b = &mut w.devs[1];
    let r = engine.sync_once(&mut b.access, &w.clock).unwrap();
    assert!(r.warnings.is_empty(), "within the grace: {r:?}");
    w.advance(23 * HOUR_MS);
    let b = &mut w.devs[1];
    assert_eq!(
        engine
            .sync_once(&mut b.access, &w.clock)
            .unwrap()
            .warnings
            .len(),
        0
    );
    w.advance(2 * HOUR_MS);
    let b = &mut w.devs[1];
    let r = engine.sync_once(&mut b.access, &w.clock).unwrap();
    let a = w.devs[0].s().install_id();
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::MissingFiles { device, .. } if *device == a)),
        "{r:?}"
    );
    assert_eq!(password_of(&w, 1, vault, item), "pw", "nothing leaked in");
}

#[test]
fn rolled_back_own_head_rollback_detected_after_two_syncs() {
    let (mut w, _vault, _item) = two_devices(36);
    let head = device_dir(&w, 0, 0).join("head.mck");
    let old = fs::read(&head).unwrap();
    for _ in 0..3 {
        w.advance(1000);
        w.sync(0);
    }
    // The provider serves an old state of A's head (a restore from backup).
    fs::write(&head, &old).unwrap();
    w.advance(1000);
    w.sync(0); // one lost upload is tolerated
    fs::write(&head, &old).unwrap();
    w.advance(1000);
    let err = w.try_sync(0).unwrap_err();
    assert!(matches!(err, SyncError::RollbackDetected), "{err:?}");
    // Stopped until the user acts.
    assert!(matches!(
        w.try_sync(0).unwrap_err(),
        SyncError::RollbackDetected
    ));
}

#[test]
fn forged_tombstone_rejected() {
    let (mut w, vault, item) = two_devices(37);
    let dir = log_dir(&w, 1, 0, vault);
    // Without the account key: a tombstone for the item in a segment that
    // does not verify.
    let seq = next_seq(&dir);
    let attacker_ak = AccountKey::generate().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let outsider = KeyedCrypto::new(&attacker_ak, w.account, w.devs[0].s().install_id()).unwrap();
    fs::write(
        dir.join(segment_name(seq)),
        forged_segment(
            &w,
            vault,
            0,
            seq,
            vec![SegmentObject::Tombstone {
                record_id: item,
                ciphertext: TombstoneCiphertext::seal(&vk, &vault, &item, b"{}").unwrap(),
            }],
            &outsider,
        ),
    )
    .unwrap();
    // And a MAC-valid one with a key the vault does not use.
    let insider = KeyedCrypto::from_keys(w.devs[0].s().sync_keys().unwrap());
    let seq2 = seq + 1;
    fs::write(
        dir.join(segment_name(seq2)),
        forged_segment(
            &w,
            vault,
            0,
            seq2,
            vec![SegmentObject::Tombstone {
                record_id: item,
                ciphertext: TombstoneCiphertext::seal(&vk, &vault, &item, b"{}").unwrap(),
            }],
            &insider,
        ),
    )
    .unwrap();
    let r = w.sync(1);
    assert_eq!(r.pulled, 0, "{r:?}");
    assert_eq!(
        r.undecryptable, 1,
        "only the second one reaches the AEAD: {r:?}"
    );
    assert!(!w.devs[1].s().item(vault, item).unwrap().is_deleted());
}

/// Adds an attachment to the item on device 0 and syncs it everywhere.
fn with_attachment() -> (World, VaultId, ItemId, AttachmentId) {
    let (mut w, vault, item) = two_devices(38);
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.converge(0);
    (w, vault, item, id)
}

fn blob_path(w: &World, replica: usize, vault: VaultId, id: AttachmentId) -> PathBuf {
    account_dir(w, replica)
        .join("v")
        .join(mosaic_sync::wire::folder_names::dir_name(vault.as_bytes()))
        .join("blobs")
        .join(format!(
            "{}.matt",
            mosaic_sync::wire::folder_names::dir_name(id.as_bytes())
        ))
}

#[test]
fn deleted_blob_reasserted_by_holder() {
    let (mut w, vault, item, id) = with_attachment();
    let blob = blob_path(&w, 0, vault, id);
    assert!(blob.exists());
    // Someone deletes the blob from the folder.
    fs::remove_file(&blob).unwrap();
    w.cloud.settle().unwrap();
    assert!(!blob_path(&w, 1, vault, id).exists());
    // The holder still references it and has the file: it uploads it again.
    w.sync(0);
    assert!(blob.exists(), "reasserted by the holder");
    w.cloud.settle().unwrap();
    assert!(blob_path(&w, 1, vault, id).exists());

    // A delete marker written by another device while a reference remains
    // does not make the holders drop the blob, whatever its age.
    let b_install = w.devs[1].s().install_id();
    {
        use mosaic_sync::transport::Transport;
        w.devs[1]
            .engine
            .transport_mut()
            .delete_attachment(vault, id)
            .unwrap();
    }
    w.cloud.settle().unwrap();
    for _ in 0..3 {
        w.advance(31 * 24 * HOUR_MS);
        w.sync_all();
    }
    assert!(blob.exists());
    assert!(w.devs[0].s().attachment(vault, item, id).is_ok());
    assert!(w.devs[1].s().attachment(vault, item, id).is_ok());
    // B references the file too, so its own sync takes its marker away
    // again.
    w.sync_all();
    w.cloud.settle().unwrap();
    let marker = blob.with_file_name(format!(
        "{}.{}.mdel",
        mosaic_sync::wire::folder_names::dir_name(id.as_bytes()),
        mosaic_sync::wire::folder_names::dir_name(b_install.as_bytes())
    ));
    assert!(
        !marker.exists(),
        "the holder's re-assertion removed the marker"
    );
}

#[test]
fn missing_attachment_fetched() {
    let mut w = World::new(40, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.sync(0);
    // The blob is withheld from the joiner's replica for a while: the item
    // arrives, the file does not yet.
    let blob_name = format!(
        "{}.matt",
        mosaic_sync::wire::folder_names::dir_name(id.as_bytes())
    );
    w.cloud.set_config(mosaic_sync::testutil::SimConfig::CALM);
    w.cloud.withhold(1, &blob_name, 6);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let joiner = w.join();
    assert_eq!(joiner, 1);
    let r = w.sync(joiner);
    assert_eq!(r.attachments_down, 0);
    assert!(w.devs[joiner].s().item(vault, item).is_ok());
    for _ in 0..4 {
        w.cloud.round().unwrap();
    }
    let r = w.sync(joiner);
    assert_eq!(r.attachments_down, 1, "{r:?}");
    assert_eq!(
        w.devs[joiner]
            .s()
            .attachment(vault, item, id)
            .unwrap()
            .expose_secret(),
        b"attachment body"
    );
}
