//! The store never writes plaintext or key material to disk (CRYPTO_SPEC
//! §11): fill a file-backed store with real `mosaic-core` ciphertext whose
//! plaintexts carry a marker string, then scan every file the store wrote
//! (database, WAL, shared memory, attachment directory) for the marker and
//! for the raw bytes of the keys whose values the test knows.

#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    reason = "test helpers outside #[test] functions"
)]

use std::fs;
use std::path::Path;

use mosaic_core::{
    AccountId, AccountKey, AccountSalt, Argon2Profile, AttachmentId, AttachmentKey, DeviceId,
    ItemCiphertext, ItemId, Kek, KeyGeneration, PasswordParams, SecretBytes, TombstoneCiphertext,
    UnlockMethod, VaultId, VaultKey, VaultMetaCiphertext, open_attachment, seal_attachment,
};
use mosaic_vault::storage::{Origin, PendingPolicy, RecordBody, Store};

const MARKER: &[u8] = b"MOSAIC-PLAINTEXT-MARKER-c0ffee";

fn with_marker(label: &str) -> Vec<u8> {
    let mut v = format!("{label}:").into_bytes();
    v.extend_from_slice(MARKER);
    v.extend_from_slice(b":password=hunter2");
    v
}

/// Every file below `dir`, recursively, as `(path, bytes)`.
fn all_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(all_files(&path));
        } else if path.to_string_lossy().ends_with("-lock") {
            // The store's lock file: always empty, and on Windows not
            // readable while the store holds its lock.
            assert_eq!(fs::metadata(&path).unwrap().len(), 0);
        } else {
            out.push((path.display().to_string(), fs::read(&path).unwrap()));
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Asserts that no file contains any needle, and returns how many bytes
/// were scanned.
fn assert_absent(dir: &Path, needles: &[(&str, &[u8])]) -> usize {
    let files = all_files(dir);
    assert!(files.len() >= 2, "expected the database and attachments");
    let mut scanned = 0;
    for (path, bytes) in &files {
        scanned += bytes.len();
        for (what, needle) in needles {
            assert!(!contains(bytes, needle), "{what} found in {path}");
        }
    }
    scanned
}

fn any_file_contains(dir: &Path, needle: &[u8]) -> bool {
    all_files(dir).iter().any(|(_, b)| contains(b, needle))
}

/// §6.2: quick-unlock expiry deletes the wrapping, and §6.4 / §6.6 replace
/// old wrappings. Once the deleting call returns, the old wrapping's bytes
/// are gone from every file, including the WAL, while the store is still
/// open (a crash right after the call must not leave them behind).
#[test]
fn removed_wrappings_leave_no_bytes_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("v.db");
    let mut store = Store::open(&path).unwrap();
    let kek = Kek::from_secret(SecretBytes::random().unwrap());
    let params = PasswordParams {
        account_id: AccountId::new_v7().unwrap(),
        salt: AccountSalt::generate().unwrap(),
        profile: Argon2Profile::DEFAULT,
    };
    let ak = AccountKey::generate().unwrap();
    let old_password = ak
        .wrap(&kek, &params.account_id, &UnlockMethod::Password)
        .unwrap();
    store.init_account(&params, &old_password).unwrap();
    let quick = UnlockMethod::QuickUnlock(DeviceId::new_v7().unwrap());
    let quick_wrapping = ak.wrap(&kek, &params.account_id, &quick).unwrap();
    store
        .put_unlock_method(&quick, &quick_wrapping, &[])
        .unwrap();
    // A vault so that the store holds more than the removed rows.
    let vault = VaultId::new_v7().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let old_vk_wrapping = vk.wrap(&ak, &params.account_id, &vault).unwrap();
    store
        .create_vault(&vault, KeyGeneration::FIRST, &old_vk_wrapping)
        .unwrap();
    // The scan is real: the bytes are on disk before the removal.
    assert!(any_file_contains(tmp.path(), quick_wrapping.as_bytes()));
    assert!(any_file_contains(tmp.path(), old_password.as_bytes()));

    assert!(store.remove_unlock_method(&quick).unwrap());
    assert!(
        !any_file_contains(tmp.path(), quick_wrapping.as_bytes()),
        "removed quick-unlock wrapping still on disk"
    );

    let new_password = ak
        .wrap(&kek, &params.account_id, &UnlockMethod::Password)
        .unwrap();
    store.update_password(&params, &new_password).unwrap();
    assert!(
        !any_file_contains(tmp.path(), old_password.as_bytes()),
        "replaced password wrapping still on disk"
    );
    assert!(any_file_contains(tmp.path(), new_password.as_bytes()));

    let new_vk_wrapping = vk.wrap(&ak, &params.account_id, &vault).unwrap();
    store
        .rewrap_vault_key(&vault, KeyGeneration::FIRST, &new_vk_wrapping)
        .unwrap();
    assert!(
        !any_file_contains(tmp.path(), old_vk_wrapping.as_bytes()),
        "replaced vault-key wrapping still on disk"
    );
    drop(store);
}

#[test]
fn no_plaintext_or_key_bytes_on_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vault.db");
    let mut store = Store::open(&path).unwrap();

    // Keys whose raw bytes the test knows. AK and VK cannot even be
    // exported from mosaic-core.
    let kek_bytes = [0x5Au8; 32];
    let fk_bytes = [0xA7u8; 32];
    let kek = Kek::from_secret(SecretBytes::from_array(&mut kek_bytes.clone()));
    let fk = AttachmentKey::from_secret(SecretBytes::from_array(&mut fk_bytes.clone()));

    let params = PasswordParams {
        account_id: AccountId::new_v7().unwrap(),
        salt: AccountSalt::generate().unwrap(),
        profile: Argon2Profile::DEFAULT,
    };
    let ak = AccountKey::generate().unwrap();
    let wrapped_ak = ak
        .wrap(&kek, &params.account_id, &UnlockMethod::Password)
        .unwrap();
    store.init_account(&params, &wrapped_ak).unwrap();
    let quick = UnlockMethod::QuickUnlock(DeviceId::new_v7().unwrap());
    store
        .put_unlock_method(
            &quick,
            &ak.wrap(&kek, &params.account_id, &quick).unwrap(),
            &[],
        )
        .unwrap();

    let vault = VaultId::new_v7().unwrap();
    let vk1 = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    store
        .create_vault(
            &vault,
            KeyGeneration::FIRST,
            &vk1.wrap(&ak, &params.account_id, &vault).unwrap(),
        )
        .unwrap();
    store
        .set_vault_meta(
            &vault,
            &VaultMetaCiphertext::seal(&vk1, &vault, &with_marker("vault-name")).unwrap(),
        )
        .unwrap();

    let mut items = Vec::new();
    for i in 0..20u64 {
        let id = ItemId::new_v7().unwrap();
        let pt = with_marker(&format!("item-{i}"));
        let ct = ItemCiphertext::seal(&vk1, &vault, &id, 1, &pt).unwrap();
        let origin = if i % 2 == 0 {
            Origin::Local
        } else {
            Origin::Remote { server_seq: i }
        };
        store
            .upsert_record(&vault, &id, &RecordBody::Item(ct.clone()), origin)
            .unwrap();
        items.push((id, ct));
    }
    // Overwrite and delete some, so freed pages and WAL frames exist too.
    for (id, _) in items.iter().take(5) {
        let t = TombstoneCiphertext::seal(&vk1, &vault, id, &with_marker("tombstone")).unwrap();
        store
            .upsert_record(&vault, id, &RecordBody::Tombstone(t), Origin::Local)
            .unwrap();
    }
    for (id, _) in items.iter().skip(5).take(3) {
        assert!(
            store
                .delete_record(&vault, id, PendingPolicy::Discard)
                .unwrap()
        );
    }

    // A three-chunk attachment full of markers.
    let (owner, _) = items.last().unwrap().clone();
    let attachment = AttachmentId::new_v7().unwrap();
    let content: Vec<u8> = MARKER.iter().copied().cycle().take(150_000).collect();
    let att_ct = seal_attachment(&fk, &vault, &owner, &attachment, &content).unwrap();
    store
        .put_attachment(&vault, &owner, &attachment, &att_ct, true)
        .unwrap();

    // Rotation: a second generation with fresh items.
    let vk2 = VaultKey::generate(KeyGeneration::FIRST.next().unwrap()).unwrap();
    store
        .accept_vault_key(
            &vault,
            vk2.generation(),
            &vk2.wrap(&ak, &params.account_id, &vault).unwrap(),
        )
        .unwrap();
    let id2 = ItemId::new_v7().unwrap();
    let ct2 = ItemCiphertext::seal(&vk2, &vault, &id2, 1, &with_marker("gen-2")).unwrap();
    store
        .upsert_record(&vault, &id2, &RecordBody::Item(ct2.clone()), Origin::Local)
        .unwrap();

    let needles: [(&str, &[u8]); 4] = [
        ("plaintext marker", MARKER),
        ("plaintext password", b"hunter2"),
        ("KEK bytes", &kek_bytes),
        ("attachment key bytes", &fk_bytes),
    ];
    // While open (data partly in the WAL)...
    let scanned = assert_absent(tmp.path(), &needles);
    assert!(scanned > 150_000);
    // ...the test does find the ciphertext it wrote, so the scan is real.
    assert!(any_file_contains(tmp.path(), &ct2.as_bytes()[..64]));
    assert!(any_file_contains(tmp.path(), &att_ct[..64]));

    // ...and after a checkpoint and close.
    store.close().unwrap();
    assert_absent(tmp.path(), &needles);

    // The data is all there: reopen and decrypt with the keys.
    let store = Store::open(&path).unwrap();
    assert_eq!(store.integrity_check().unwrap(), Vec::<String>::new());
    let (owner_id, owner_ct) = items.last().unwrap();
    let r = store.record(&vault, owner_id).unwrap().unwrap();
    assert_eq!(r.body, RecordBody::Item(owner_ct.clone()));
    let RecordBody::Item(ct) = r.body else {
        unreachable!()
    };
    let pt = ct.open(&vk1, &vault, owner_id).unwrap();
    assert!(contains(pt.expose_secret(), MARKER));
    let meta = store.vault_meta(&vault).unwrap().unwrap();
    assert!(contains(
        meta.open(&vk1, &vault).unwrap().expose_secret(),
        MARKER
    ));
    let att = store.attachment(&attachment).unwrap();
    let opened = open_attachment(&fk, &vault, &owner, &attachment, &att).unwrap();
    assert_eq!(opened.expose_secret(), content.as_slice());
    let unwrapped = store
        .unlock_method(&UnlockMethod::Password)
        .unwrap()
        .unwrap()
        .wrapped_account_key
        .unwrap_account_key(&kek, &params.account_id, &UnlockMethod::Password)
        .unwrap();
    assert_eq!(unwrapped, ak);
}
