//! Tests of the sync session API (M6): the upload queue, two-phase
//! rotation, install identity, reset, attachments and the split join and
//! password change.

use mosaic_core::model::ItemType;
use mosaic_core::unlock::PrfOutput;
use mosaic_core::{
    Argon2Profile, Kek, MasterPassword, SecretBytes, SecretKey, UnlockMethod, VaultId, VaultKey,
};

use super::{
    AttachmentFileState, DEFAULT_RP_ID, Error, FolderMacPurpose, KeyOrigin, LockedSession,
    NewHardwareKey, Reauthenticated, SyncConfig, SyncKind, UnlockedSession,
};
use crate::storage::{Error as StorageError, Store};

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account_in(store: Store) -> (UnlockedSession, super::CreatedAccount) {
    LockedSession::open(store)
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap()
}

fn account() -> (UnlockedSession, SecretKey) {
    let (s, created) = account_in(Store::open_in_memory().unwrap());
    (s, created.secret_key)
}

fn file_store(dir: &tempfile::TempDir) -> Store {
    Store::open(dir.path().join("vault.db")).unwrap()
}

fn login(s: &mut UnlockedSession, vault: VaultId, title: &str) -> mosaic_core::ItemId {
    s.add_item(vault, &ItemType::Login, |e| e.set_title(title))
        .unwrap()
}

/// Marks every queued record of `vault` as uploaded with sequence numbers
/// from `seq`.
fn upload_all(s: &mut UnlockedSession, vault: VaultId, mut seq: u64) {
    for entry in s.pending_uploads_in(vault, 100).unwrap() {
        let rec = entry.unwrap();
        s.mark_uploaded(vault, rec.record_id, rec.outbox_seq.unwrap(), seq)
            .unwrap();
        seq += 1;
    }
}

#[test]
fn pending_vault_uploads_after_create_rename_rotate() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    let g1 = mosaic_core::KeyGeneration::FIRST;

    // A new vault: creation carries the first key and the metadata.
    let pending = s.pending_vault_uploads().unwrap();
    assert_eq!(pending.len(), 1);
    let (g, wrapped, meta) = pending[0].create.clone().unwrap();
    assert_eq!(g, g1);
    assert_eq!(wrapped, s.store.vault_key(&v, g1).unwrap().unwrap());
    assert_eq!(meta, s.store.vault_meta(&v).unwrap().unwrap());
    assert!(pending[0].keys.is_empty() && pending[0].meta.is_none());
    s.mark_vault_created_remotely(v).unwrap();
    assert_eq!(s.pending_vault_uploads().unwrap().len(), 0);

    // A rename queues the metadata, with no base sequence number yet.
    s.rename_vault(v, "B").unwrap();
    let pending = s.pending_vault_uploads().unwrap();
    let (renamed, base) = pending[0].meta.clone().unwrap();
    assert_eq!(base, None);
    assert!(pending[0].create.is_none());
    assert_eq!(pending[0].keys.len(), 0);
    assert!(s.mark_vault_meta_uploaded(v, &renamed, 5).unwrap());
    assert_eq!(s.pending_vault_uploads().unwrap().len(), 0);
    s.rename_vault(v, "C").unwrap();
    let pending = s.pending_vault_uploads().unwrap();
    assert_eq!(pending[0].meta.as_ref().unwrap().1, Some(5));
    let (renamed, _) = pending[0].meta.clone().unwrap();
    s.mark_vault_meta_uploaded(v, &renamed, 6).unwrap();

    // An offline rotation queues the new key and the re-sealed metadata.
    let g2 = s.rotate_vault_key(v).unwrap();
    let pending = s.pending_vault_uploads().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].keys.len(), 1);
    assert_eq!(pending[0].keys[0].0, g2);
    assert_eq!(pending[0].meta.as_ref().unwrap().0.key_generation(), g2);
    assert_eq!(s.vault_key_origin(v, g2).unwrap(), Some(KeyOrigin::Local));
    s.mark_vault_key_uploaded(v, g2).unwrap();
    let (ct, base) = pending[0].meta.clone().unwrap();
    assert_eq!(base, Some(6));
    s.mark_vault_meta_uploaded(v, &ct, 7).unwrap();
    assert_eq!(s.pending_vault_uploads().unwrap().len(), 0);
}

/// A rename after the metadata was handed out for creation stays queued.
#[test]
fn a_rename_during_creation_is_not_lost() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    assert!(s.pending_vault_uploads().unwrap()[0].create.is_some());
    s.rename_vault(v, "B").unwrap();
    s.mark_vault_created_remotely(v).unwrap();
    let pending = s.pending_vault_uploads().unwrap();
    assert!(pending[0].create.is_none());
    assert!(pending[0].meta.is_some(), "the rename still has to go");
}

#[test]
fn prepare_rotation_changes_nothing_until_commit() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "x");
    upload_all(&mut s, v, 1);
    let keys_before = s.store.vault_keys(&v).unwrap();
    let state_before = s.store.vault(&v).unwrap().unwrap();
    let record_before = s.store.record(&v, &item).unwrap().unwrap();

    let p = s.prepare_vault_rotation(v).unwrap();
    assert_eq!(p.vault_id(), v);
    assert_eq!(p.generation().get(), 2);
    assert!(
        p.wrapped()
            .unwrap_vault_key(&s.account_key, &s.account_id, &v, p.generation())
            .is_ok()
    );
    assert_eq!(s.store.vault_keys(&v).unwrap(), keys_before);
    assert_eq!(s.store.vault(&v).unwrap().unwrap(), state_before);
    assert_eq!(s.store.record(&v, &item).unwrap().unwrap(), record_before);
    assert_eq!(s.vaults()[0].key_generation.get(), 1);
    assert_eq!(s.pending_uploads_in(v, 10).unwrap().len(), 0);
    // Preparing again gives another key, still without a change.
    let again = s.prepare_vault_rotation(v).unwrap();
    assert_ne!(again.wrapped(), p.wrapped());
    assert_eq!(s.store.vault_keys(&v).unwrap(), keys_before);
}

#[test]
fn commit_rotation_reseals_and_queues() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "x");
    upload_all(&mut s, v, 1);
    s.mark_vault_created_remotely(v).unwrap();
    assert_eq!(s.pending_vault_uploads().unwrap().len(), 0);

    let p = s.prepare_vault_rotation(v).unwrap();
    let wrapped = p.wrapped().clone();
    let g = s.commit_vault_rotation(p).unwrap();
    assert_eq!(g.get(), 2);
    assert_eq!(s.vaults()[0].key_generation, g);
    // The record is sealed under the new key and queued; the item is intact.
    let stored = s.store.record(&v, &item).unwrap().unwrap();
    assert_eq!(stored.body.key_generation(), g);
    assert!(stored.outbox_seq.is_some());
    assert_eq!(s.item(v, item).unwrap().title(), Some("x"));
    // The key was uploaded before the commit: queued metadata only.
    assert_eq!(s.store.vault_key(&v, g).unwrap().unwrap(), wrapped);
    assert_eq!(s.vault_key_origin(v, g).unwrap(), Some(KeyOrigin::Local));
    let pending = s.pending_vault_uploads().unwrap();
    assert_eq!(pending[0].keys.len(), 0);
    assert_eq!(pending[0].meta.as_ref().unwrap().0.key_generation(), g);
}

#[test]
fn commit_rotation_refuses_stale_generation() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    let first = s.prepare_vault_rotation(v).unwrap();
    let second = s.prepare_vault_rotation(v).unwrap();
    s.commit_vault_rotation(first).unwrap();
    assert!(matches!(
        s.commit_vault_rotation(second),
        Err(Error::KeyGenerationRollback {
            highest_accepted: 2,
            offered: 2
        })
    ));
    assert_eq!(s.vaults()[0].key_generation.get(), 2);

    // A rotation accepted from another device in between is refused as well.
    let late = s.prepare_vault_rotation(v).unwrap();
    let remote = VaultKey::generate(late.generation()).unwrap();
    let wrapped = remote.wrap(&s.account_key, &s.account_id, &v).unwrap();
    s.apply_remote_vault_key(v, late.generation(), &wrapped)
        .unwrap();
    assert_eq!(
        s.vault_key_origin(v, late.generation()).unwrap(),
        Some(KeyOrigin::Remote)
    );
    assert!(matches!(
        s.commit_vault_rotation(late),
        Err(Error::KeyGenerationRollback { .. })
    ));
}

/// L4: what a session prepared dies with it. Prepare, lock, unlock, commit:
/// refused, and nothing changes.
#[test]
fn prepared_changes_do_not_survive_a_lock() {
    let (mut s, created) = account_in(Store::open_in_memory().unwrap());
    let sk = created.secret_key;
    let v = s.create_vault("A").unwrap();
    let rotation = s.prepare_vault_rotation(v).unwrap();
    let new = MasterPassword::new("a brand new password".to_owned());
    let change = s.prepare_master_password_change(&pw(), &new, &sk).unwrap();
    let before = s.account_bootstrap().unwrap();

    // The same session still commits what it prepared (the happy path is
    // covered elsewhere); a later session does not.
    let stale_rotation = s.prepare_vault_rotation(v).unwrap();
    let mut s = s.lock().unlock(&pw(), &sk).unwrap();
    assert!(matches!(
        s.commit_vault_rotation(rotation),
        Err(Error::PreparedStale)
    ));
    assert!(matches!(
        s.commit_vault_rotation(stale_rotation),
        Err(Error::PreparedStale)
    ));
    assert!(matches!(
        s.commit_master_password_change(change),
        Err(Error::PreparedStale)
    ));
    assert_eq!(s.vaults()[0].key_generation.get(), 1);
    let after = s.account_bootstrap().unwrap();
    assert_eq!(
        (after.params, after.wrapped_ak),
        (before.params, before.wrapped_ak)
    );

    // Preparing again in the new session works.
    let fresh = s.prepare_vault_rotation(v).unwrap();
    assert_eq!(s.commit_vault_rotation(fresh).unwrap().get(), 2);
}

/// L3: a damaged local-origin key row replaced by the backend's wrapping is
/// no longer queued for upload and no longer counts as generated here.
#[test]
fn a_damaged_local_key_replaced_by_a_remote_wrapping_is_not_queued() {
    let (mut s, _) = account();
    let g1 = mosaic_core::KeyGeneration::FIRST;
    let v = s.create_vault("A").unwrap();
    let intact = s.store.vault_key(&v, g1).unwrap().unwrap();
    // The new vault's key is a local one waiting for upload.
    assert_eq!(s.vault_key_origin(v, g1).unwrap(), Some(KeyOrigin::Local));
    assert_eq!(s.store.pending_vault_keys(&v).unwrap().len(), 1);
    s.store
        .conn_for_tests()
        .execute(
            "UPDATE vault_keys SET wrapped_vk = zeroblob(74) WHERE vault_id = ?1",
            [v.as_bytes()],
        )
        .unwrap();
    let mut s = s.reload();
    assert!(!s.vaults()[0].available);
    s.apply_remote_vault_key(v, g1, &intact).unwrap();
    assert!(s.vaults()[0].available);
    assert_eq!(s.store.vault_key(&v, g1).unwrap(), Some(intact));
    assert_eq!(s.vault_key_origin(v, g1).unwrap(), Some(KeyOrigin::Remote));
    assert_eq!(s.store.pending_vault_keys(&v).unwrap().len(), 0);
}

#[test]
fn install_id_stable_across_unlocks_and_distinct_per_store() {
    let dir = tempfile::tempdir().unwrap();
    let (s, created) = account_in(file_store(&dir));
    let id = s.install_id();
    let writer = s.device_id();
    let locked = s.lock();
    let s = locked.unlock(&pw(), &created.secret_key).unwrap();
    assert_eq!(s.install_id(), id, "stable across unlocks");
    assert_ne!(s.device_id(), writer, "the writer ID is fresh per unlock");
    drop(s);
    let other = tempfile::tempdir().unwrap();
    let (t, _) = account_in(file_store(&other));
    assert_ne!(t.install_id(), id, "distinct per store");
}

#[test]
fn reset_sync_state_requeues_everything_except_hardware_vaults() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = account_in(file_store(&dir));
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "x");
    let att = s
        .add_attachment(v, item, "f.txt", "text/plain", b"hello")
        .unwrap();
    let hw = s
        .create_hardware_vault(
            "Codes",
            &NewHardwareKey::new(vec![1; 32], DEFAULT_RP_ID.to_owned(), [2; 32]).unwrap(),
            &PrfOutput::from_secret(SecretBytes::from_array(&mut [0xA1; 32])),
        )
        .unwrap();
    let hw_item = s
        .add_item(hw, &ItemType::Totp, |e| {
            e.set_title("t")?;
            e.set_otp(&mosaic_core::model::OtpConfig::totp(
                mosaic_core::SecretVec::from_slice(b"12345678901234567890").unwrap(),
            ))
        })
        .unwrap();

    // Everything is "synced".
    upload_all(&mut s, v, 10);
    s.mark_vault_created_remotely(v).unwrap();
    let rec = s.store.record(&hw, &hw_item).unwrap().unwrap();
    s.store
        .mark_uploaded(&hw, &hw_item, rec.outbox_seq.unwrap(), 9)
        .unwrap();
    s.mark_attachment_uploaded(att).unwrap();
    s.set_sync_cursor(v, b"cursor").unwrap();
    s.allocate_local_seqs(v, 5).unwrap();
    s.remove_attachment(v, item, att).unwrap();
    assert_eq!(s.pending_attachment_deletions().unwrap().len(), 1);
    upload_all(&mut s, v, 20);
    assert_eq!(s.pending_vault_uploads().unwrap().len(), 0);
    assert_eq!(s.synced_vaults(), vec![v], "the hardware vault never syncs");
    let att2 = s
        .add_attachment(v, item, "g.txt", "text/plain", b"again")
        .unwrap();
    s.mark_attachment_uploaded(att2).unwrap();
    upload_all(&mut s, v, 30);
    assert_eq!(s.pending_uploads_in(v, 10).unwrap().len(), 0);
    let counter = s.next_checkpoint_counter().unwrap();

    s.reset_sync_state().unwrap();

    // The vault's record is queued again without a server sequence.
    let entries = s.pending_uploads_in(v, 10).unwrap();
    assert_eq!(entries.len(), 1);
    let rec = entries[0].clone().unwrap();
    assert_eq!((rec.record_id, rec.server_seq), (item, None));
    assert_eq!(s.pending_attachment_uploads().unwrap().len(), 1);
    assert_eq!(s.pending_attachment_deletions().unwrap().len(), 0);
    assert_eq!(s.sync_cursor(v).unwrap(), None);
    let pending = s.pending_vault_uploads().unwrap();
    assert_eq!(pending.len(), 1, "only the synced vault");
    assert_eq!(pending[0].vault_id, v);
    assert!(pending[0].create.is_some());
    assert_eq!(s.allocate_local_seqs(v, 1).unwrap(), 1, "blocks restart");
    assert_eq!(s.checkpoint_counter().unwrap(), counter, "kept");
    // The hardware vault is untouched: uploaded state kept, nothing queued.
    let hw_rec = s.store.record(&hw, &hw_item).unwrap().unwrap();
    assert_eq!((hw_rec.server_seq, hw_rec.outbox_seq), (Some(9), None));
    assert_eq!(s.pending_uploads_in(hw, 10).unwrap().len(), 0);
    assert!(matches!(
        s.prepare_vault_rotation(hw),
        Err(Error::WrongVaultProtection)
    ));
}

#[test]
fn store_remote_attachment_requires_reference() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = account_in(file_store(&dir));
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "x");
    let other = login(&mut s, v, "y");
    let att = s
        .add_attachment(v, item, "f.txt", "text/plain", b"hello")
        .unwrap();
    assert_eq!(
        s.attachment_file_state(att).unwrap(),
        AttachmentFileState::Present {
            upload_pending: true
        }
    );
    let refs = s.attachment_references();
    assert_eq!(refs.len(), 1);
    assert_eq!(
        (refs[0].item_id, refs[0].attachment_id, refs[0].live),
        (item, att, true)
    );
    s.trash_item(v, item).unwrap();
    assert!(!s.attachment_references()[0].live);

    // The file is gone locally (a device that has not downloaded it yet).
    let ct = s.attachment_ciphertext(att).unwrap();
    s.store.delete_attachment(&att).unwrap();
    assert_eq!(
        s.attachment_file_state(att).unwrap(),
        AttachmentFileState::Missing
    );
    // An item that does not refer to it, or an unknown item, is refused.
    assert!(matches!(
        s.store_remote_attachment(v, other, att, &ct),
        Err(Error::UnknownAttachmentReference)
    ));
    assert!(matches!(
        s.store_remote_attachment(v, mosaic_core::ItemId::new_v7().unwrap(), att, &ct),
        Err(Error::UnknownAttachmentReference)
    ));
    s.store_remote_attachment(v, item, att, &ct).unwrap();
    assert_eq!(
        s.attachment_file_state(att).unwrap(),
        AttachmentFileState::Present {
            upload_pending: false
        }
    );
    assert_eq!(s.pending_attachment_uploads().unwrap().len(), 0);
    assert!(matches!(
        s.store_remote_attachment(v, item, att, &ct),
        Err(Error::Store(StorageError::AttachmentExists))
    ));
    assert_eq!(
        s.attachment(v, item, att).unwrap().expose_secret(),
        b"hello"
    );
}

/// A deleted attachment that was uploaded reaches the backend's deletion
/// queue; one that was not uploaded does not.
#[test]
fn attachment_deletion_recorded_only_after_upload() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = account_in(file_store(&dir));
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "x");
    let sent = s.add_attachment(v, item, "a", "text/plain", b"1").unwrap();
    let unsent = s.add_attachment(v, item, "b", "text/plain", b"2").unwrap();
    s.mark_attachment_uploaded(sent).unwrap();
    s.remove_attachment(v, item, unsent).unwrap();
    assert_eq!(s.pending_attachment_deletions().unwrap().len(), 0);
    s.remove_attachment(v, item, sent).unwrap();
    assert_eq!(s.pending_attachment_deletions().unwrap(), vec![(v, sent)]);
    s.mark_attachment_deletion_sent(sent).unwrap();
    assert_eq!(s.pending_attachment_deletions().unwrap().len(), 0);
}

#[test]
fn join_with_keys_equals_join_account() {
    let (a, created) = account_in(Store::open_in_memory().unwrap());
    let bootstrap = a.account_bootstrap().unwrap();
    let derived =
        LockedSession::derive_join_keys(&bootstrap.params, &pw(), &created.secret_key).unwrap();
    // The OPAQUE password of the join is the one of the account.
    assert_eq!(derived.opaque_password(), &created.opaque_password);
    assert_eq!(
        a.opaque_password(&pw(), &created.secret_key).unwrap(),
        created.opaque_password
    );

    // The keys carry the parameters they were derived under, and the join
    // stores exactly those.
    assert_eq!(derived.params(), &bootstrap.params);
    let split = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account_with_keys(&bootstrap.wrapped_ak, derived)
        .unwrap();
    let whole = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account(
            &bootstrap.params,
            &bootstrap.wrapped_ak,
            &pw(),
            &created.secret_key,
        )
        .unwrap();
    assert!(split.account_key == whole.account_key && split.account_key == a.account_key);
    assert_eq!(split.account_id(), a.account_id());
    for s in [&split, &whole] {
        let b = s.account_bootstrap().unwrap();
        assert_eq!(b.params, bootstrap.params);
        assert_eq!(b.wrapped_ak, bootstrap.wrapped_ak);
    }

    // A wrong password stores nothing and gives the locked session back.
    let wrong = MasterPassword::new("not the password".to_owned());
    let keys =
        LockedSession::derive_join_keys(&bootstrap.params, &wrong, &created.secret_key).unwrap();
    let err = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account_with_keys(&bootstrap.wrapped_ak, keys)
        .unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    assert!(!err.session.has_account().unwrap());

    // Keys derived under other parameters (another salt) do not open the
    // wrapped key, and nothing is stored.
    let mut other_params = bootstrap.params;
    other_params.salt = mosaic_core::AccountSalt::from_bytes([0x5A; 16]);
    assert_ne!(other_params, bootstrap.params);
    let keys = LockedSession::derive_join_keys(&other_params, &pw(), &created.secret_key).unwrap();
    assert_eq!(keys.params(), &other_params);
    let err = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account_with_keys(&bootstrap.wrapped_ak, keys)
        .unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    assert!(!err.session.has_account().unwrap());
}

#[test]
fn opaque_password_wrong_password_fails_unlock_failed() {
    let (s, created) = account_in(Store::open_in_memory().unwrap());
    let wrong = MasterPassword::new("not the password".to_owned());
    assert!(matches!(
        s.opaque_password(&wrong, &created.secret_key),
        Err(Error::UnlockFailed)
    ));
    let other_sk = SecretKey::from_secret(SecretBytes::from_array(&mut [7; 16]));
    assert!(matches!(
        s.opaque_password(&pw(), &other_sk),
        Err(Error::UnlockFailed)
    ));
    assert!(matches!(
        s.opaque_password(&MasterPassword::new(String::new()), &created.secret_key),
        Err(Error::Core(mosaic_core::Error::EmptyPassword))
    ));
    assert_eq!(
        s.opaque_password(&pw(), &created.secret_key).unwrap(),
        created.opaque_password
    );
}

#[test]
fn prepare_commit_password_change_equals_change_master_password() {
    let (mut s, created) = account_in(Store::open_in_memory().unwrap());
    let sk = created.secret_key;
    let new = MasterPassword::new("a brand new password".to_owned());
    let before = s.account_bootstrap().unwrap();
    // A quick-unlock wrapping exists before the change (CRYPTO_SPEC 6.2).
    let proof = Reauthenticated::for_test(s.proof_id);
    let (enrolled, _) = s
        .enroll_quick_unlock(&Kek::generate().unwrap(), &[9], &proof)
        .unwrap();
    let quick_rows = |s: &UnlockedSession| {
        s.store
            .unlock_methods()
            .unwrap()
            .iter()
            .filter(|r| matches!(r.method, UnlockMethod::QuickUnlock(_)))
            .count()
    };
    assert_eq!(quick_rows(&s), 1);

    let prepared = s.prepare_master_password_change(&pw(), &new, &sk).unwrap();
    // Nothing is stored by the preparation: the wrapping is still there.
    assert_eq!(quick_rows(&s), 1);
    let unchanged = s.account_bootstrap().unwrap();
    assert_eq!(unchanged.params, before.params);
    assert_eq!(unchanged.wrapped_ak, before.wrapped_ak);
    assert_eq!(prepared.old_opaque_password(), &created.opaque_password);
    assert_ne!(prepared.new_opaque_password(), &created.opaque_password);
    assert_eq!(prepared.new_params().account_id, before.params.account_id);
    assert_ne!(prepared.new_params().salt, before.params.salt, "a new salt");
    assert!(matches!(
        s.prepare_master_password_change(&new, &new, &sk),
        Err(Error::UnlockFailed)
    ));
    assert!(matches!(
        s.prepare_master_password_change(&pw(), &MasterPassword::new(String::new()), &sk),
        Err(Error::Core(mosaic_core::Error::EmptyPassword))
    ));

    let wrapped = prepared.new_wrapped_ak().clone();
    let params = *prepared.new_params();
    let changed = s.commit_master_password_change(prepared).unwrap();
    assert_eq!(changed.params, params);
    // The commit deleted the quick-unlock wrapping and returned it, so the
    // app deletes its platform key.
    assert_eq!(changed.removed_quick_unlock, Some(enrolled));
    assert_eq!(quick_rows(&s), 0);
    assert_eq!(s.quick_unlock_enrollment().unwrap(), None);
    let after = s.account_bootstrap().unwrap();
    assert_eq!((after.params, after.wrapped_ak), (params, wrapped));
    assert_eq!(
        changed.opaque_password,
        s.opaque_password(&new, &sk).unwrap()
    );

    // The new password unlocks, the old one does not.
    let account_id = s.account_id();
    let err = s.lock().unlock(&pw(), &sk).unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    let s = err.session.unlock(&new, &sk).unwrap();
    assert_eq!(s.account_id(), account_id);
}

#[test]
fn pending_uploads_in_filters_by_vault() {
    let (mut s, _) = account();
    let (a, b) = (s.create_vault("A").unwrap(), s.create_vault("B").unwrap());
    let (ia, ib1) = (login(&mut s, a, "a"), login(&mut s, b, "b1"));
    let ib2 = login(&mut s, b, "b2");
    let ids = |s: &UnlockedSession, v, limit| -> Vec<_> {
        s.pending_uploads_in(v, limit)
            .unwrap()
            .into_iter()
            .map(|e| e.unwrap().record_id)
            .collect()
    };
    assert_eq!(ids(&s, a, 10), vec![ia]);
    assert_eq!(ids(&s, b, 10), vec![ib1, ib2], "oldest change first");
    assert_eq!(ids(&s, b, 1), vec![ib1]);
    assert_eq!(ids(&s, VaultId::new_v7().unwrap(), 10).len(), 0);
    assert_eq!(s.pending_uploads(10).unwrap().len(), 3);
}

#[test]
fn mark_vault_meta_uploaded_only_clears_matching_bytes() {
    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    s.mark_vault_created_remotely(v).unwrap();
    s.rename_vault(v, "B").unwrap();
    let first = s.store.vault_meta(&v).unwrap().unwrap();
    s.rename_vault(v, "C").unwrap();
    let second = s.store.vault_meta(&v).unwrap().unwrap();
    assert_ne!(first, second);

    // An acknowledgement for the older bytes keeps the newer queued.
    assert!(!s.mark_vault_meta_uploaded(v, &first, 3).unwrap());
    let state = s.store.vault_sync_state(&v).unwrap();
    assert!(state.meta_upload_pending);
    assert_eq!(state.meta_server_seq, Some(3), "the sequence is stored");
    // The matching bytes clear it; a lower sequence never lowers the stored one.
    assert!(s.mark_vault_meta_uploaded(v, &second, 2).unwrap());
    let state = s.store.vault_sync_state(&v).unwrap();
    assert!(!state.meta_upload_pending);
    assert_eq!(state.meta_server_seq, Some(3));
    s.note_vault_meta_seq(v, 7).unwrap();
    s.note_vault_meta_seq(v, 5).unwrap();
    assert_eq!(
        s.store.vault_sync_state(&v).unwrap().meta_server_seq,
        Some(7)
    );
    // A merged remote metadata is queued, an adopted one is not.
    assert!(!s.store.vault_sync_state(&v).unwrap().meta_upload_pending);
}

#[test]
fn checkpoint_folder_and_device_info_use_the_session_keys() {
    let (mut s, _) = account();
    let digest = [9u8; 32];
    let mac = s.checkpoint_mac(4, &digest).unwrap();
    assert!(
        s.verify_checkpoint_mac(s.install_id(), 4, &digest, &mac)
            .unwrap()
    );
    let other = mosaic_core::InstallId::new_v7().unwrap();
    assert!(!s.verify_checkpoint_mac(other, 4, &digest, &mac).unwrap());
    assert!(
        !s.verify_checkpoint_mac(s.install_id(), 5, &digest, &mac)
            .unwrap()
    );

    for purpose in [
        FolderMacPurpose::Segment,
        FolderMacPurpose::Account,
        FolderMacPurpose::BlobDelete,
    ] {
        let mac = s.folder_mac(purpose, b"m").unwrap();
        assert!(s.verify_folder_mac(purpose, b"m", &mac).unwrap());
        assert!(!s.verify_folder_mac(purpose, b"n", &mac).unwrap());
    }
    // The plaintext is exactly 512 bytes (the caller pads it).
    let mut plaintext = [0u8; 512];
    plaintext[..12].copy_from_slice(b"{\"name\":\"x\"}");
    let sealed = s.seal_device_info(&plaintext).unwrap();
    assert_eq!(
        s.open_device_info(s.install_id(), &sealed)
            .unwrap()
            .expose_secret(),
        plaintext.as_slice()
    );
    assert!(s.open_device_info(other, &sealed).is_err());
    assert!(matches!(
        s.seal_device_info(b"{\"name\":\"x\"}"),
        Err(Error::Core(mosaic_core::Error::Malformed))
    ));
    assert!(matches!(
        s.seal_device_info(&[0u8; 513]),
        Err(Error::Core(mosaic_core::Error::SizeLimit))
    ));

    assert_eq!(s.checkpoint_counter().unwrap(), 0);
    assert_eq!(s.next_checkpoint_counter().unwrap(), 1);
    assert_eq!(s.checkpoint_counter().unwrap(), 1);

    // The checkpoint key is the account's: another account's MAC differs.
    let (t, _) = account();
    assert_ne!(
        t.checkpoint_mac(4, &digest).unwrap(),
        s.checkpoint_mac(4, &digest).unwrap()
    );
}

#[test]
fn sync_config_and_cursors_through_the_session() {
    let (mut s, _) = account();
    assert_eq!(s.sync_config().unwrap(), None);
    let config = SyncConfig {
        kind: SyncKind::Hosted,
        server_url: Some("https://sync.example".to_owned()),
        email: Some("a@b.example".to_owned()),
        folder_path: None,
        device_name: "Laptop".to_owned(),
    };
    s.set_sync_config(Some(&config)).unwrap();
    assert_eq!(s.sync_config().unwrap(), Some(config.clone()));
    let locked = s.lock();
    assert_eq!(locked.sync_config().unwrap(), Some(config));
}

/// A downloaded attachment is stored only if every chunk opens under the file
/// key of the item that refers to it; a file of valid layout with a flipped
/// byte is refused, and so is a file sealed under another key.
#[test]
fn remote_attachments_must_authenticate() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = account_in(file_store(&dir));
    let v = s.create_vault("A").unwrap();
    let item = login(&mut s, v, "with file");
    let id = s
        .add_attachment(v, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    let ct = s.attachment_ciphertext(id).unwrap();
    assert!(s.remote_attachment_authentic(v, item, id, &ct).unwrap());
    let mut flipped = ct.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 1;
    assert!(
        !s.remote_attachment_authentic(v, item, id, &flipped)
            .unwrap()
    );
    // Same layout, other key.
    let other = mosaic_core::seal_attachment(
        &mosaic_core::AttachmentKey::generate().unwrap(),
        &v,
        &item,
        &id,
        b"attachment body",
    )
    .unwrap();
    assert_eq!(other.len(), ct.len());
    assert!(!s.remote_attachment_authentic(v, item, id, &other).unwrap());
    assert!(matches!(
        s.store_remote_attachment(v, item, id, &other),
        Err(Error::Core(_))
    ));
    // An attachment the item does not refer to.
    let stranger = mosaic_core::AttachmentId::new_v7().unwrap();
    assert!(matches!(
        s.remote_attachment_authentic(v, item, stranger, &ct),
        Err(Error::UnknownAttachmentReference)
    ));
}

/// The persisted engine state survives a lock, `reset_sync_state` clears the
/// state of the old backend but keeps a halt and the checkpoint counter, and
/// `clear_sync_halt` keeps the reserved sequence numbers.
#[test]
fn engine_state_persists_and_resets_with_the_backend() {
    use crate::storage::{PeerCheckpoint, PeerEntry, SyncHalt};

    let dir = tempfile::tempdir().unwrap();
    let (mut s, created) = account_in(file_store(&dir));
    let v = s.create_vault("A").unwrap();
    let device = mosaic_core::InstallId::new_v7().unwrap();
    s.reserve_own_seq(v, 7).unwrap();
    s.confirm_own_seq(v, 6).unwrap();
    s.set_last_published_counter(3).unwrap();
    s.set_checkpoint_strikes(1, Some(10), Some(3)).unwrap();
    s.set_sync_halt(SyncHalt::Rollback).unwrap();
    s.set_sync_lag(&[(device, v, 5)]).unwrap();
    let cp = PeerCheckpoint {
        device_id: device,
        counter: 2,
        issued_at_ms: 9,
        entries: vec![PeerEntry {
            vault_id: v,
            generation: mosaic_core::KeyGeneration::FIRST,
            high_seq: 4,
            view_digest: [0; 32],
        }],
        digests_acknowledged: false,
    };
    assert!(s.put_peer_checkpoint(&cp).unwrap());
    s.marker_first_seen(
        v,
        &[crate::storage::MarkerStamp {
            attachment_id: mosaic_core::AttachmentId::new_v7().unwrap(),
            device_id: device,
            stamp_ms: 1,
        }],
        50,
    )
    .unwrap();
    let mut s = s.lock().unlock(&pw(), &created.secret_key).unwrap();
    assert_eq!(s.own_seq(v).unwrap().reserved, 7);
    assert_eq!(s.own_seq(v).unwrap().written, 6);
    assert_eq!(s.peer_checkpoints().unwrap(), vec![cp]);
    assert_eq!(s.sync_lag().unwrap(), vec![(device, v, 5)]);
    let st = s.sync_engine_state().unwrap();
    assert_eq!(st.halt, Some(SyncHalt::Rollback));
    assert_eq!((st.last_published_counter, st.strike_count), (3, 1));

    s.reset_sync_state().unwrap();
    assert_eq!(
        s.own_seq(v).unwrap(),
        own(7, 0),
        "reserved stays, written starts over"
    );
    assert_eq!(s.peer_checkpoints().unwrap(), Vec::new());
    assert_eq!(s.sync_lag().unwrap(), Vec::new());
    let st = s.sync_engine_state().unwrap();
    assert_eq!(st.halt, Some(SyncHalt::Rollback), "the halt stays");
    assert_eq!(st.strike_count, 0);

    s.reserve_own_seq(v, 4).unwrap();
    s.confirm_own_seq(v, 4).unwrap();
    s.clear_sync_halt().unwrap();
    assert_eq!(s.sync_engine_state().unwrap().halt, None);
    assert_eq!(s.own_seq(v).unwrap().reserved, 7);
}

fn own(reserved: u64, written: u64) -> crate::storage::OwnSeq {
    crate::storage::OwnSeq { reserved, written }
}

/// The hosted server accepts a change only on top of the version it holds;
/// the sync engine bases an upload on its served view, not on the stored
/// number. A received version that this device's own record already contains
/// changes nothing here, and the echo of the device's own upload, whose
/// response was lost, acknowledges the pending change.
#[test]
fn received_versions_already_contained_rebase_or_acknowledge_a_pending_change() {
    use super::{ApplyOutcome, RemoteRecord};
    use crate::storage::RecordBody;

    let (mut s, _) = account();
    let v = s.create_vault("A").unwrap();
    let id = login(&mut s, v, "one");
    upload_all(&mut s, v, 5);
    let seq = |s: &UnlockedSession| s.store().record(&v, &id).unwrap().unwrap().server_seq;
    let outbox = |s: &UnlockedSession| s.store().record(&v, &id).unwrap().unwrap().outbox_seq;
    let body =
        |s: &UnlockedSession| -> RecordBody { s.store().record(&v, &id).unwrap().unwrap().body };
    let first = body(&s);

    // A local edit is queued on top of seq 5.
    s.update_item(v, id, |e| e.set_title("two")).unwrap();
    assert_eq!(seq(&s), Some(5));
    assert!(outbox(&s).is_some());

    // The server stored exactly that change and the response was lost: the
    // pull brings it back at seq 9 and the change is acknowledged.
    let echo = RemoteRecord {
        vault_id: v,
        record_id: id,
        body: body(&s),
        server_seq: 9,
    };
    assert_eq!(s.apply_remote_record(&echo).unwrap(), ApplyOutcome::Ignored);
    assert_eq!(seq(&s), Some(9));
    assert_eq!(outbox(&s), None, "nothing left to upload");

    // A later edit is queued on top of 9. An older version of the record
    // (the local state dominates it) arrives at seq 12: the pending change
    // stays queued and the stored number does not move (the base of an
    // upload is the served view the sync engine keeps, which this call does
    // not touch: a forged number must not wedge the record).
    s.update_item(v, id, |e| e.set_title("three")).unwrap();
    assert_eq!(seq(&s), Some(9));
    let older = RemoteRecord {
        vault_id: v,
        record_id: id,
        body: first,
        server_seq: 12,
    };
    assert_eq!(
        s.apply_remote_record(&older).unwrap(),
        ApplyOutcome::Ignored
    );
    assert_eq!(seq(&s), Some(9));
    assert!(outbox(&s).is_some(), "the change is still to be uploaded");
}
