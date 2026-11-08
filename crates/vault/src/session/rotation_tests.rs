//! Tests of key rotation in the session (M7, CRYPTO_SPEC §6.4 to §6.7):
//! credential changes, the full re-key and its adoption, the attachment
//! re-seal and the journal.

#![expect(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "accounts a, b, c (and their second devices) read best as one letter"
)]

use mosaic_core::model::ItemType;
use mosaic_core::unlock::PrfOutput;
use mosaic_core::{
    Argon2Profile, CredentialChange, Fingerprint, IdentityPublicKey, ItemId, Kek, KeyGeneration,
    MasterPassword, PasswordParams, SecretBytes, SecretKey, VaultId, WrappedKey,
};

use super::sharing_tests::{fpr, g, pw, trio, user};
use super::{
    DEFAULT_RP_ID, Error, FolderRekeyPlan, LockedSession, NewHardwareKey, PulledShareView,
    RekeyAdoption, RekeyPlan, RekeyRequest, ResealStep, RotationKind, RotationPhase, RotationPlan,
    SyncConfig, SyncKind, UnlockedSession,
};
use crate::storage::{Error as StorageError, Store};

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn pw2() -> MasterPassword {
    MasterPassword::new("a different password".to_owned())
}

fn dup_sk(sk: &SecretKey) -> SecretKey {
    SecretKey::parse(sk.to_display().unwrap().expose_secret()).unwrap()
}

fn hosted() -> SyncConfig {
    SyncConfig {
        kind: SyncKind::Hosted,
        server_url: Some("https://one.example".to_owned()),
        email: Some("a@x".to_owned()),
        folder_path: None,
        device_name: "Laptop".to_owned(),
    }
}

/// An unlocked hosted account with a published identity key.
fn account_in(store: Store) -> (UnlockedSession, SecretKey) {
    let (mut s, created) = LockedSession::open(store)
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    s.set_sync_config(Some(&hosted())).unwrap();
    s.ensure_identity_key().unwrap();
    s.mark_identity_published().unwrap();
    (s, created.secret_key)
}

fn account() -> (UnlockedSession, SecretKey) {
    account_in(Store::open_in_memory().unwrap())
}

fn ra(s: &UnlockedSession) -> super::Reauthenticated {
    super::Reauthenticated::for_test(s.proof_id)
}

fn hw_key(fill: u8) -> NewHardwareKey {
    NewHardwareKey::new(vec![fill; 32], DEFAULT_RP_ID.to_owned(), [fill ^ 0x5A; 32]).unwrap()
}

fn prf(fill: u8) -> PrfOutput {
    PrfOutput::from_secret(SecretBytes::from_array(&mut [fill; 32]))
}

fn login(s: &mut UnlockedSession, vault: VaultId, title: &str) -> ItemId {
    s.add_item(vault, &ItemType::Login, |e| e.set_title(title))
        .unwrap()
}

/// A vault created on the backend, as the hosted precondition wants.
fn synced_vault(s: &mut UnlockedSession, name: &str) -> VaultId {
    let v = s.create_vault(name).unwrap();
    s.mark_vault_created_remotely(v).unwrap();
    v
}

fn prepare(
    s: &UnlockedSession,
    sk: &SecretKey,
    new_sk: &SecretKey,
    plan: RekeyPlan,
) -> Result<super::PreparedRekey, Error> {
    s.prepare_rekey(RekeyRequest {
        current_password: &pw(),
        current_secret_key: sk,
        new_password: &pw2(),
        new_secret_key: new_sk,
        profile: Argon2Profile::Baseline,
        plan,
    })
}

/// What a backend would hold after a re-key.
struct Rekeyed {
    committed: super::RekeyCommitted,
    params: PasswordParams,
    wrapped: WrappedKey,
    identity: Option<(IdentityPublicKey, WrappedKey)>,
    new_sk: SecretKey,
}

fn rekey(s: &mut UnlockedSession, sk: &SecretKey, plan: RekeyPlan) -> Rekeyed {
    let new_sk = SecretKey::generate().unwrap();
    let p = prepare(s, sk, &new_sk, plan).unwrap();
    let (params, wrapped, identity) = (*p.new_params(), p.new_wrapped_ak().clone(), p.identity());
    let committed = s.commit_rekey(p).unwrap();
    Rekeyed {
        committed,
        params,
        wrapped,
        identity,
        new_sk,
    }
}

fn relock(s: UnlockedSession, pw: &MasterPassword, sk: &SecretKey) -> UnlockedSession {
    s.lock().unlock(pw, sk).map_err(|e| e.error).unwrap()
}

// ---- credential changes ---------------------------------------------------------

#[test]
fn secret_key_rotation_keeps_ak_and_removes_quick_unlock() {
    let (mut s, sk) = account();
    let v = s.create_vault("v").unwrap();
    let item = login(&mut s, v, "kept");
    s.enroll_quick_unlock(
        &Kek::from_secret(SecretBytes::random().unwrap()),
        &[],
        &ra(&s),
    )
    .unwrap();
    s.enroll_hardware_key(&hw_key(1), &prf(1), &ra(&s)).unwrap();
    let ak_before = s.store.unlock_methods().unwrap().len();
    assert_eq!(ak_before, 3);
    let account = s.account_id();

    let new_sk = SecretKey::generate().unwrap();
    let change = CredentialChange {
        new_password: None,
        new_secret_key: Some(&new_sk),
        profile: Argon2Profile::Baseline,
    };
    // The same Secret Key again is not a rotation.
    assert!(matches!(
        s.prepare_credential_change(
            &pw(),
            &sk,
            &CredentialChange {
                new_secret_key: Some(&sk),
                ..change
            }
        ),
        Err(Error::SecretKeyUnchanged)
    ));
    // A mistyped current password produces nothing.
    assert!(matches!(
        s.prepare_credential_change(&pw2(), &sk, &change),
        Err(Error::UnlockFailed)
    ));
    let old_salt = s.store.account().unwrap().unwrap().salt;
    let p = s.prepare_credential_change(&pw(), &sk, &change).unwrap();
    let done = s.commit_master_password_change(p).unwrap();
    assert!(done.removed_quick_unlock.is_some());
    assert_ne!(done.params.salt, old_salt);
    assert!(s.salt_retired(&old_salt).unwrap());
    assert_eq!(s.ak_epoch(), 1);
    // The hardware-key wrapping stays; the old Secret Key is retired.
    assert_eq!(s.hardware_keys().unwrap().len(), 1);
    assert_eq!(
        s.retired_secret_keys().unwrap(),
        vec![sk.retired_commitment(&account)]
    );

    let locked = s.lock();
    let err = locked.unlock(&pw(), &sk).unwrap_err();
    assert!(matches!(err.error, Error::UnlockFailed));
    let s = err
        .session
        .unlock(&pw(), &new_sk)
        .map_err(|e| e.error)
        .unwrap();
    assert!(s.item(v, item).is_ok());
}

#[test]
fn profile_upgrade_only_increases() {
    let (mut s, sk) = account();
    let change = |profile| CredentialChange {
        new_password: None,
        new_secret_key: None,
        profile,
    };
    let p = s
        .prepare_credential_change(&pw(), &sk, &change(Argon2Profile::Standard))
        .unwrap();
    assert_eq!(p.new_params().profile, Argon2Profile::Standard);
    s.commit_master_password_change(p).unwrap();
    assert_eq!(
        s.store.account().unwrap().unwrap().profile,
        Argon2Profile::Standard
    );
    // Checked before any Argon2id work.
    assert!(matches!(
        s.prepare_credential_change(&pw(), &sk, &change(Argon2Profile::Baseline)),
        Err(Error::Core(mosaic_core::Error::ProfileDowngrade))
    ));
}

// ---- re-key preconditions ---------------------------------------------------------

#[test]
fn rekey_refuses_same_password() {
    let (s, sk) = account();
    let new_sk = SecretKey::generate().unwrap();
    let err = s
        .prepare_rekey(RekeyRequest {
            current_password: &pw(),
            current_secret_key: &sk,
            new_password: &pw(),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan: RekeyPlan::default(),
        })
        .unwrap_err();
    assert!(matches!(err, Error::PasswordUnchanged));
    // A wrong current password is an unlock failure.
    let err = s
        .prepare_rekey(RekeyRequest {
            current_password: &pw2(),
            current_secret_key: &sk,
            new_password: &pw2(),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan: RekeyPlan::default(),
        })
        .unwrap_err();
    assert!(matches!(err, Error::UnlockFailed));
}

#[test]
fn rekey_refuses_same_or_retired_secret_key() {
    let (mut s, sk) = account();
    let err = prepare(&s, &sk, &dup_sk(&sk), RekeyPlan::default()).unwrap_err();
    assert!(matches!(err, Error::SecretKeyUnchanged));
    // After a rotation the old key is retired and cannot come back.
    let mid = SecretKey::generate().unwrap();
    let p = s
        .prepare_credential_change(
            &pw(),
            &sk,
            &CredentialChange {
                new_password: None,
                new_secret_key: Some(&mid),
                profile: Argon2Profile::Baseline,
            },
        )
        .unwrap();
    s.commit_master_password_change(p).unwrap();
    let err = prepare(&s, &mid, &dup_sk(&sk), RekeyPlan::default()).unwrap_err();
    assert!(matches!(err, Error::SecretKeyUnchanged));
}

#[test]
fn rekey_needs_clean_sync() {
    let (mut s, sk) = account();
    let new_sk = SecretKey::generate().unwrap();
    // An owned vault the backend does not have yet.
    let v = s.create_vault("v").unwrap();
    assert!(matches!(
        prepare(&s, &sk, &new_sk, RekeyPlan::default()),
        Err(Error::RekeyNotClean)
    ));
    s.mark_vault_created_remotely(v).unwrap();
    prepare(&s, &sk, &new_sk, RekeyPlan::default()).unwrap();
    // A halt blocks it, except the one a verified re-key marker sets.
    s.set_sync_halt(crate::storage::SyncHalt::Rollback).unwrap();
    assert!(matches!(
        prepare(&s, &sk, &new_sk, RekeyPlan::default()),
        Err(Error::RekeyNotClean)
    ));
    s.clear_sync_halt().unwrap();
    // The re-key-pending flag is no halt: a re-key is still allowed.
    s.set_rekey_pending(true).unwrap();
    assert!(s.rekey_pending().unwrap());
    prepare(&s, &sk, &new_sk, RekeyPlan::default()).unwrap();
    assert!(s.rekey_review().is_ok());
}

// ---- the commit -------------------------------------------------------------------

/// One account with a synced vault, an item, a contact book and an identity
/// key: what every row of the commit touches.
fn busy() -> (UnlockedSession, SecretKey, VaultId, ItemId) {
    let (mut s, sk) = account();
    let v = synced_vault(&mut s, "Personal");
    let item = login(&mut s, v, "kept");
    (s, sk, v, item)
}

type Snapshot = (
    Option<PasswordParams>,
    usize,
    Vec<(KeyGeneration, WrappedKey)>,
    u32,
    Option<crate::storage::StoredIdentity>,
    Option<super::RotationJob>,
    usize,
);

fn snapshot(s: &UnlockedSession, v: VaultId) -> Snapshot {
    (
        s.store.account().unwrap(),
        s.store.unlock_methods().unwrap().len(),
        s.store.vault_keys(&v).unwrap(),
        s.store.ak_epoch().unwrap(),
        s.store.identity_key().unwrap(),
        s.store.rotation_job().unwrap(),
        s.store.outbox(100).unwrap().len(),
    )
}

#[test]
fn rekey_commit_is_atomic() {
    // A failure after any of the statements leaves the store as it was.
    for trigger in [
        "BEFORE UPDATE ON records",
        "BEFORE UPDATE OF ak_epoch ON account",
        "BEFORE UPDATE ON identity_key",
        "BEFORE UPDATE ON contacts_doc",
        "BEFORE INSERT ON retired_secret_keys",
        "BEFORE UPDATE OF current_generation ON vaults",
        "BEFORE DELETE ON unlock_methods",
        "BEFORE INSERT ON rotation_job",
        "BEFORE UPDATE OF meta ON vaults",
    ] {
        let (mut s, sk, v, item) = busy();
        s.enroll_hardware_key(&hw_key(1), &prf(1), &ra(&s)).unwrap();
        let before = snapshot(&s, v);
        let new_sk = SecretKey::generate().unwrap();
        let p = prepare(&s, &sk, &new_sk, RekeyPlan::default()).unwrap();
        s.store
            .exec_raw(&format!(
                "CREATE TEMP TRIGGER boom {trigger} BEGIN SELECT RAISE(ABORT, 'boom'); END;"
            ))
            .unwrap();
        let err = s.commit_rekey(p).unwrap_err();
        assert!(
            matches!(err, Error::Store(StorageError::Database(_))),
            "{trigger}: {err:?}"
        );
        assert_eq!(snapshot(&s, v), before, "{trigger}");
        assert_eq!(s.ak_epoch(), 1);
        assert!(s.item(v, item).is_ok());
        // Without the fault the same re-key goes through.
        s.store.exec_raw("DROP TRIGGER boom;").unwrap();
        let p = prepare(&s, &sk, &new_sk, RekeyPlan::default()).unwrap();
        s.commit_rekey(p).unwrap();
        assert_eq!(s.ak_epoch(), 2);
    }
}

#[test]
fn rekey_rewraps_every_generation() {
    let (mut s, sk, v, item) = busy();
    s.rotate_vault_key(v).unwrap();
    assert_eq!(s.store.vault_keys(&v).unwrap().len(), 2);
    let done = rekey(&mut s, &sk, RekeyPlan::default());
    // Generations 1 and 2 re-wrapped, 3 new: all open under the new AK.
    let s = relock(s, &pw2(), &done.new_sk);
    assert_eq!(s.vaults.get(&v).unwrap().keys.len(), 3);
    assert_eq!(s.ak_epoch(), 2);
    assert!(s.item(v, item).is_ok());
    assert_eq!(s.corrupt_records().len(), 0);
    // The old Secret Key and password do not open the new wrapping.
    let locked = s.lock();
    let e = locked.unlock(&pw(), &sk).unwrap_err();
    assert!(matches!(e.error, Error::UnlockFailed));
}

#[test]
fn rekey_reseals_records_meta_roster_under_next_generation() {
    let (mut a, b, c, v) = trio();
    a.s.mark_vault_created_remotely(v).unwrap();
    let item = login(&mut a.s, v, "team item");
    let plan = RekeyPlan {
        shared: vec![(
            v,
            RotationPlan {
                remaining: vec![(b.id, b.ipk), (c.id, c.ipk)],
                ..RotationPlan::default()
            },
        )],
        heads: vec![(v, 7)],
        folder: None,
    };
    let new_sk = SecretKey::generate().unwrap();
    let p =
        a.s.prepare_rekey(RekeyRequest {
            current_password: &pw(),
            current_secret_key: &a.secret,
            new_password: &pw2(),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan,
        })
        .unwrap();
    assert_eq!(p.to_epoch(), 2);
    assert_eq!(p.vault_keys().len(), 1);
    let (vault, generation, _, head) = p.vault_keys()[0].clone();
    assert_eq!((vault, generation, head), (v, g(2), 7));
    let out = &p.shared_rotations()[0];
    let mut sealed_to: Vec<_> = out.seals().iter().map(|(acc, _)| *acc).collect();
    sealed_to.sort();
    let mut want = vec![b.id, c.id];
    want.sort();
    assert_eq!(sealed_to, want);
    let (new_ipk, _) = p.identity().unwrap();
    let seals = out.seals().to_vec();
    let roster_ct = out.roster().clone();
    a.s.commit_rekey(p).unwrap();

    // Records, metadata and roster are all at generation 2.
    assert_eq!(a.s.vaults.get(&v).unwrap().generation, g(2));
    for entry in a.s.store.records(&v).unwrap() {
        assert_eq!(entry.ok().unwrap().body.key_generation(), g(2));
    }
    assert_eq!(
        a.s.store.vault_meta(&v).unwrap().unwrap().key_generation(),
        g(2)
    );
    let stored_roster = a.s.store.vault_roster(&v).unwrap().unwrap();
    assert_eq!(stored_roster.generation, g(2));
    assert!(!stored_roster.pending, "the request carried it");
    let owner = a.s.vault_roster(v).unwrap().unwrap();
    let own = owner.iter().find(|m| m.account_id == a.id).unwrap();
    assert_eq!(own.fingerprint, Fingerprint::of(&a.id, &new_ipk));
    assert!(a.s.item(v, item).is_ok());
    assert!(a.s.vault_key_confirmed(v, g(2)).unwrap());

    // A member refuses the owner's share until the new code is accepted
    // (THREAT_MODEL R84), then claims the new key.
    let mut b = b;
    let share = |sender| PulledShareView {
        generation: g(2),
        sender,
        seal: seals
            .iter()
            .find(|(acc, _)| *acc == b.id)
            .unwrap()
            .1
            .clone(),
    };
    let ipks = |acc: &mosaic_core::AccountId| (*acc == a.id).then_some(new_ipk);
    let refused =
        b.s.open_share_chain(v, &[share(a.id)], &[(g(2), roster_ct.clone())], &ipks);
    assert!(refused.is_err());
    b.s.accept_key_change(&a.id, &new_ipk, "a@x").unwrap();
    b.s.verify_contact(&a.id, &Fingerprint::of(&a.id, &new_ipk))
        .unwrap();
    let chain =
        b.s.open_share_chain(v, &[share(a.id)], &[(g(2), roster_ct)], &ipks)
            .unwrap();
    assert_eq!(chain.generation(), g(2));
    let _ = fpr(&b);
}

#[test]
fn rekey_plan_must_cover_owned_shared_vaults() {
    let (mut a, b, c, v) = trio();
    a.s.mark_vault_created_remotely(v).unwrap();
    let new_sk = SecretKey::generate().unwrap();
    let go = |a: &super::sharing_tests::User, plan| {
        a.s.prepare_rekey(RekeyRequest {
            current_password: &pw(),
            current_secret_key: &a.secret,
            new_password: &pw2(),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan,
        })
    };
    // No entry for the shared vault; a member left out; a revocation.
    assert!(matches!(
        go(&a, RekeyPlan::default()),
        Err(Error::PlanMismatch)
    ));
    let only_b = RekeyPlan {
        shared: vec![(
            v,
            RotationPlan {
                remaining: vec![(b.id, b.ipk)],
                ..RotationPlan::default()
            },
        )],
        ..RekeyPlan::default()
    };
    assert!(matches!(go(&a, only_b), Err(Error::PlanMismatch)));
    let revoking = RekeyPlan {
        shared: vec![(
            v,
            RotationPlan {
                remaining: vec![(b.id, b.ipk)],
                revoked: vec![c.id],
                ..RotationPlan::default()
            },
        )],
        ..RekeyPlan::default()
    };
    assert!(matches!(go(&a, revoking), Err(Error::PlanMismatch)));
    // A served key that is not the roster's.
    let (_, other_ipk) = mosaic_core::IdentitySecret::generate().unwrap();
    let wrong_key = RekeyPlan {
        shared: vec![(
            v,
            RotationPlan {
                remaining: vec![(b.id, other_ipk), (c.id, c.ipk)],
                ..RotationPlan::default()
            },
        )],
        ..RekeyPlan::default()
    };
    assert!(matches!(go(&a, wrong_key), Err(Error::MemberKeyMismatch)));
}

#[test]
fn rekey_deletes_quick_and_hardware_unlock_keeps_tk_vaults() {
    let (mut s, sk, v, _) = busy();
    s.enroll_quick_unlock(
        &Kek::from_secret(SecretBytes::random().unwrap()),
        &[7],
        &ra(&s),
    )
    .unwrap();
    s.enroll_hardware_key(&hw_key(1), &prf(1), &ra(&s)).unwrap();
    let tk = s
        .create_hardware_vault("codes", &hw_key(2), &prf(2))
        .unwrap();
    let tk_keys = s.store.vault_keys(&tk).unwrap();
    let tk_wraps = s.store.tk_wrappings(&tk).unwrap();
    let review = s.rekey_review().unwrap();
    assert_eq!(review.tk_vaults, vec![tk]);
    assert_eq!(review.owned_vaults, vec![v]);
    assert!(review.quick_unlock);
    assert_eq!(review.hardware_keys, 1);

    let done = rekey(&mut s, &sk, RekeyPlan::default());
    assert_eq!(
        done.committed.removed_quick_unlock.unwrap().platform_params,
        [7]
    );
    assert_eq!(done.committed.removed_hardware_keys.len(), 1);
    assert_eq!(s.store.unlock_methods().unwrap().len(), 1, "password only");
    assert_eq!(s.store.vault_keys(&tk).unwrap(), tk_keys);
    assert_eq!(s.store.tk_wrappings(&tk).unwrap(), tk_wraps);
    // Still there after a lock, closed.
    let s = relock(s, &pw2(), &done.new_sk);
    assert!(s.vaults.contains_key(&tk));
}

#[test]
fn rekey_replaces_identity_and_reseals_contacts() {
    let (mut s, sk, _, _) = busy();
    let other = user("b@x");
    s.pin_contact(&other.id, &other.ipk, "b@x").unwrap();
    s.verify_contact(&other.id, &fpr(&other)).unwrap();
    let (old_ipk, old_wrapped, published) = s.identity().unwrap().unwrap();
    assert!(published);
    let old_doc = s.store.contacts_doc().unwrap().unwrap().ciphertext;

    let done = rekey(&mut s, &sk, RekeyPlan::default());
    let (new_ipk, new_wrapped, published) = s.identity().unwrap().unwrap();
    assert_ne!(new_ipk, old_ipk);
    assert_ne!(new_wrapped, old_wrapped);
    assert!(published, "hosted: the request carried it");
    assert_eq!(done.identity.as_ref().unwrap().0, new_ipk);
    // The book is sealed under the new K_contacts, queued, and keeps the pin.
    let doc = s.store.contacts_doc().unwrap().unwrap();
    assert!(doc.pending);
    assert_ne!(doc.ciphertext, old_doc);
    let mut s = relock(s, &pw2(), &done.new_sk);
    assert_eq!(s.contacts().unwrap().len(), 1);
    assert_eq!(s.ensure_identity_key().unwrap(), new_ipk);
    // Epoch and identity counters only rise.
    assert!(s.store.exec_raw("UPDATE account SET ak_epoch = 1").is_err());
}

#[test]
fn rekey_on_a_folder_builds_the_marker_and_floors() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, sk) = account_in(Store::open(dir.path().join("v.db")).unwrap());
    s.set_sync_config(Some(&SyncConfig {
        kind: SyncKind::Folder,
        server_url: None,
        email: None,
        folder_path: Some(dir.path().join("folder")),
        device_name: "Laptop".to_owned(),
    }))
    .unwrap();
    let v = s.create_vault("v").unwrap();
    let item = login(&mut s, v, "x");
    s.reserve_own_seq(v, 5).unwrap();
    let plan = RekeyPlan {
        folder: Some(FolderRekeyPlan {
            to_account_version: 4,
            created_at_ms: 99,
        }),
        ..RekeyPlan::default()
    };
    let new_sk = SecretKey::generate().unwrap();
    let p = prepare(&s, &sk, &new_sk, plan).unwrap();
    let marker = *p.folder_marker().unwrap();
    assert_eq!(&marker[..5], b"MREK\x01");
    assert_eq!(&marker[21..25], &1u32.to_be_bytes());
    assert_eq!(&marker[25..29], &2u32.to_be_bytes());
    assert_eq!(&marker[29..37], &4u64.to_be_bytes());
    // MAC'd under the OLD K_fold: the old key verifies it, the new one cannot.
    let old_fold = s.folder_key().unwrap();
    assert!(
        mosaic_core::sync_auth::verify_folder_rekey_mac(
            &old_fold,
            &marker[..61],
            marker[61..].try_into().unwrap()
        )
        .unwrap()
    );
    s.commit_rekey(p).unwrap();
    let new_fold = s.folder_key().unwrap();
    assert!(
        !mosaic_core::sync_auth::verify_folder_rekey_mac(
            &new_fold,
            &marker[..61],
            marker[61..].try_into().unwrap()
        )
        .unwrap()
    );
    let job = s.rotation_job().unwrap().unwrap();
    assert_eq!(job.kind, RotationKind::Rekey);
    assert_eq!(job.phase, RotationPhase::LocalCommitted);
    assert_eq!(job.folder_marker, Some(marker));
    assert_eq!(job.target_version, Some(4));
    // New keys wait for upload on a folder.
    assert!(s.store.pending_vault_keys(&v).unwrap().len() >= 2);
    let floors = s.folder_rekey_floors().unwrap();
    assert_eq!(floors.len(), 1);
    assert_eq!((floors[0].vault_id, floors[0].floor_seq), (v, 5));
    assert!(!floors[0].snapshot_done);
    s.mark_folder_rekey_snapshot(v).unwrap();
    assert!(s.folder_rekey_floors().unwrap()[0].snapshot_done);
    assert!(s.item(v, item).is_ok());
    // A backend change resets the floors, never the epoch.
    s.set_rotation_phase(RotationPhase::Uploading).unwrap();
    s.finish_rotation_job().unwrap();
    s.set_sync_config(None).unwrap();
    s.reset_sync_state().unwrap();
    assert_eq!(s.folder_rekey_floors().unwrap().len(), 0);
    assert_eq!(s.ak_epoch(), 2);
}

// ---- attachments --------------------------------------------------------------------

#[test]
fn reseal_attachments_resume_after_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = account_in(Store::open(&path).unwrap());
    let v = synced_vault(&mut s, "v");
    let item = login(&mut s, v, "with files");
    let a1 = s
        .add_attachment(v, item, "a.txt", "text/plain", b"first")
        .unwrap();
    let a2 = s
        .add_attachment(v, item, "b.txt", "text/plain", b"second")
        .unwrap();
    // Both blobs reached the backend, so it has to be told to drop the old ones.
    s.mark_attachment_uploaded(a1).unwrap();
    s.mark_attachment_uploaded(a2).unwrap();
    let done = rekey(&mut s, &sk, RekeyPlan::default());
    assert_eq!(done.committed.attachments_to_reseal, 2);
    assert_eq!(s.rotation_job().unwrap().unwrap().total, 2);

    // One step, then a crash: a fresh unlock continues.
    assert!(matches!(
        s.reseal_next_attachment(None).unwrap(),
        ResealStep::Resealed { .. }
    ));
    let mut s = relock(s, &pw2(), &done.new_sk);
    let job = s.rotation_job().unwrap().unwrap();
    assert_eq!((job.done, job.total), (1, 2));
    assert!(matches!(
        s.finish_rotation_job(),
        Err(Error::Store(StorageError::RotationNotFinished))
    ));
    assert!(matches!(
        s.reseal_next_attachment(None).unwrap(),
        ResealStep::Resealed { .. }
    ));
    assert_eq!(s.reseal_next_attachment(None).unwrap(), ResealStep::Done);

    // The item refers to fresh IDs, the files open, the old ones are gone and
    // queued for deletion once the item is accepted.
    let refs: Vec<_> = s
        .item(v, item)
        .unwrap()
        .attachments()
        .into_iter()
        .map(|a| a.attachment_id)
        .collect();
    assert_eq!(refs.len(), 2);
    assert!(!refs.contains(&a1) && !refs.contains(&a2));
    let mut plain: Vec<Vec<u8>> = refs
        .iter()
        .map(|id| s.attachment(v, item, *id).unwrap().expose_secret().to_vec())
        .collect();
    plain.sort();
    assert_eq!(plain, vec![b"first".to_vec(), b"second".to_vec()]);
    assert!(s.store.attachment_info(&a1).unwrap().is_none());
    let deletions = s.pending_attachment_deletions().unwrap();
    assert!(deletions.iter().any(|(_, id)| *id == a1));
    assert!(deletions.iter().any(|(_, id)| *id == a2));
    assert!(s.pending_attachment_uploads().unwrap().len() >= 2);
    s.set_rotation_phase(RotationPhase::Uploading).unwrap();
    s.finish_rotation_job().unwrap();
    assert!(s.rotation_job().unwrap().is_none());
}

#[test]
fn unreachable_attachment_listed() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, sk) = account_in(Store::open(dir.path().join("v.db")).unwrap());
    let v = synced_vault(&mut s, "v");
    let item = login(&mut s, v, "files");
    let held = s
        .add_attachment(v, item, "a.txt", "text/plain", b"held")
        .unwrap();
    let lost = s
        .add_attachment(v, item, "b.txt", "text/plain", b"lost")
        .unwrap();
    let fetched = s
        .add_attachment(v, item, "c.txt", "text/plain", b"fetched")
        .unwrap();
    // Two ciphertexts are not stored here (never fetched); the backend has one.
    let fetched_ct = s.store.attachment(&fetched).unwrap();
    s.store.delete_attachment(&lost).unwrap();
    s.store.delete_attachment(&fetched).unwrap();
    let review = s.rekey_review().unwrap();
    assert_eq!(review.unreachable_attachments.len(), 2);
    let done = rekey(&mut s, &sk, RekeyPlan::default());
    assert_eq!(done.committed.attachments_to_reseal, 3);
    let mut resealed = 0;
    let mut unreachable = Vec::new();
    loop {
        match s.reseal_next_attachment(None).unwrap() {
            ResealStep::Done => break,
            ResealStep::Resealed { .. } => resealed += 1,
            ResealStep::NeedsFetch { attachment_id, .. } if attachment_id == fetched => {
                // Bytes that do not authenticate are refused and change nothing.
                assert!(
                    s.reseal_next_attachment(Some(&fetched_ct[..fetched_ct.len() - 1]))
                        .is_err()
                );
                assert!(matches!(
                    s.reseal_next_attachment(Some(&fetched_ct)).unwrap(),
                    ResealStep::Resealed { .. }
                ));
                resealed += 1;
            }
            ResealStep::NeedsFetch { attachment_id, .. } => {
                assert_eq!(attachment_id, lost);
                assert!(s.mark_attachment_unreachable(attachment_id).unwrap());
                unreachable.push(attachment_id);
            }
            ResealStep::Skipped { .. } => unreachable.push(held),
        }
    }
    assert_eq!(resealed, 2);
    assert_eq!(unreachable, vec![lost]);
    let listed = s.unreachable_attachments().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].old_id, lost);
    // The lost one keeps its old reference, and the job can finish.
    let refs: Vec<_> = s
        .item(v, item)
        .unwrap()
        .attachments()
        .into_iter()
        .map(|a| a.attachment_id)
        .collect();
    assert!(refs.contains(&lost) && !refs.contains(&held) && !refs.contains(&fetched));
    s.finish_rotation_job().unwrap();
    assert_eq!(
        s.unreachable_attachments().unwrap().len(),
        1,
        "the list stays"
    );
}

#[test]
fn vault_attachment_reseal_job() {
    let dir = tempfile::tempdir().unwrap();
    let (mut s, _) = account_in(Store::open(dir.path().join("v.db")).unwrap());
    let v = s.create_vault("v").unwrap();
    let item = login(&mut s, v, "files");
    s.add_attachment(v, item, "a.txt", "text/plain", b"x")
        .unwrap();
    s.rotate_vault_key(v).unwrap();
    assert_eq!(s.queue_vault_attachment_reseal(v).unwrap(), 1);
    let job = s.rotation_job().unwrap().unwrap();
    assert_eq!(
        (job.kind, job.phase),
        (RotationKind::Vault, RotationPhase::Attachments)
    );
    assert!(matches!(
        s.queue_vault_attachment_reseal(v),
        Err(Error::RotationInProgress)
    ));
    assert!(matches!(
        s.reseal_next_attachment(None).unwrap(),
        ResealStep::Resealed { .. }
    ));
    s.finish_rotation_job().unwrap();
}

// ---- adoption -----------------------------------------------------------------------

/// Device A of an account and a second device B that joined it.
fn two_devices() -> (UnlockedSession, SecretKey, UnlockedSession, SecretKey) {
    let (a, sk) = account();
    let boot = a.account_bootstrap().unwrap();
    let b = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account(&boot.params, &boot.wrapped_ak, &pw(), &sk)
        .map_err(|e| e.error)
        .unwrap();
    let b_sk = dup_sk(&sk);
    (a, sk, b, b_sk)
}

fn adoption<'a>(
    done: &'a Rekeyed,
    typed: &'a SecretKey,
    current: &'a SecretKey,
    epoch: u32,
) -> RekeyAdoption<'a> {
    RekeyAdoption {
        keys: LockedSession::derive_join_keys(&done.params, &pw2(), typed).unwrap(),
        typed_secret_key: typed,
        current_secret_key: current,
        wrapped_ak: &done.wrapped,
        params: done.params,
        epoch,
        identity: done.identity.as_ref().map(|(i, w)| (w, i)),
        member_wraps: &[],
    }
}

#[test]
fn adopt_remote_rekey_moves_the_device() {
    let (mut a, sk, mut b, b_sk) = two_devices();
    let done = rekey(&mut a, &sk, RekeyPlan::default());
    let b_old_salt = b.store.account().unwrap().unwrap().salt;
    let adopted = b
        .adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, 2))
        .unwrap();
    assert_eq!(adopted.epoch, 2);
    assert_eq!(b.ak_epoch(), 2);
    assert_eq!(a.account_key, b.account_key);
    assert!(
        b.salt_retired(&b_old_salt).unwrap(),
        "the replaced salt is retired"
    );
    assert_eq!(
        b.retired_secret_keys().unwrap(),
        vec![b_sk.retired_commitment(&b.account_id)]
    );
    // The identity of the backend is the one this device holds now.
    assert_eq!(
        b.identity().unwrap().unwrap().0,
        done.identity.as_ref().unwrap().0
    );
    let b = relock(b, &pw2(), &done.new_sk);
    assert_eq!(b.ak_epoch(), 2);
}

#[test]
fn adopt_remote_rekey_requires_next_epoch() {
    let (mut a, sk, mut b, b_sk) = two_devices();
    let done = rekey(&mut a, &sk, RekeyPlan::default());
    // Equal: no adoption at an equal epoch. Higher by two: not the next one.
    assert!(matches!(
        b.adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, 1)),
        Err(Error::UnlockFailed)
    ));
    assert!(matches!(
        b.adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, 3)),
        Err(Error::EpochNotNext {
            stored: 1,
            offered: 3
        })
    ));
    assert!(matches!(
        b.adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, u32::MAX)),
        Err(Error::EpochNotNext { .. })
    ));
    assert_eq!(b.ak_epoch(), 1, "nothing was stored");
    // A device that is already at 2 sees 1 as a rollback.
    b.adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, 2))
        .unwrap();
    assert!(matches!(
        b.check_backend_epoch(1),
        Err(Error::EpochRollback {
            stored: 2,
            offered: 1
        })
    ));
    assert!(b.check_backend_epoch(2).is_ok());
    assert!(b.check_backend_epoch(3).is_ok());
}

#[test]
fn adopt_remote_rekey_refuses_current_or_retired_secret_key() {
    let (mut a, sk, mut b, b_sk) = two_devices();
    let done = rekey(&mut a, &sk, RekeyPlan::default());
    // Typing the old Secret Key (an attacker's file under the old key).
    let old_typed = dup_sk(&sk);
    let wrong = RekeyAdoption {
        keys: LockedSession::derive_join_keys(&done.params, &pw2(), &old_typed).unwrap(),
        ..adoption(&done, &old_typed, &b_sk, 2)
    };
    assert!(matches!(
        b.adopt_remote_rekey(wrong),
        Err(Error::SecretKeyUnchanged)
    ));
    // A Secret Key this device retired by an earlier rotation is refused too.
    let mid = SecretKey::generate().unwrap();
    let p = b
        .prepare_credential_change(
            &pw(),
            &b_sk,
            &CredentialChange {
                new_password: None,
                new_secret_key: Some(&mid),
                profile: Argon2Profile::Baseline,
            },
        )
        .unwrap();
    b.commit_master_password_change(p).unwrap();
    let retired_typed = dup_sk(&b_sk);
    let wrong = RekeyAdoption {
        keys: LockedSession::derive_join_keys(&done.params, &pw2(), &retired_typed).unwrap(),
        ..adoption(&done, &retired_typed, &mid, 2)
    };
    assert!(matches!(
        b.adopt_remote_rekey(wrong),
        Err(Error::SecretKeyUnchanged)
    ));
    assert_eq!(b.ak_epoch(), 1);
}

#[test]
fn adopt_remote_rekey_refuses_retired_salt() {
    let (mut a, sk, mut b, b_sk) = two_devices();
    let done = rekey(&mut a, &sk, RekeyPlan::default());
    // Pretend the backend's salt is one this device replaced.
    b.store
        .exec_raw(&format!(
            "INSERT INTO retired_salts (salt, retired_at) VALUES (x'{}', 0)",
            hex(done.params.salt.as_bytes())
        ))
        .unwrap();
    assert!(matches!(
        b.adopt_remote_rekey(adoption(&done, &done.new_sk, &b_sk, 2)),
        Err(Error::PasswordRollback)
    ));
}

#[test]
fn adopt_remote_rekey_refuses_a_forged_wrapping_and_identity() {
    let (mut a, sk, mut b, b_sk) = two_devices();
    let done = rekey(&mut a, &sk, RekeyPlan::default());
    // Keys derived from another password: the wrapping does not open.
    let wrong_pw = RekeyAdoption {
        keys: LockedSession::derive_join_keys(&done.params, &pw(), &done.new_sk).unwrap(),
        ..adoption(&done, &done.new_sk, &b_sk, 2)
    };
    assert!(matches!(
        b.adopt_remote_rekey(wrong_pw),
        Err(Error::UnlockFailed)
    ));
    // Keys derived under other parameters than the ones offered.
    let mut other = done.params;
    other.salt = mosaic_core::AccountSalt::generate().unwrap();
    let mismatched = RekeyAdoption {
        keys: LockedSession::derive_join_keys(&other, &pw2(), &done.new_sk).unwrap(),
        ..adoption(&done, &done.new_sk, &b_sk, 2)
    };
    assert!(matches!(
        b.adopt_remote_rekey(mismatched),
        Err(Error::UnlockFailed)
    ));
    // A served public key that is not the wrapped key's.
    let (_, other_ipk) = mosaic_core::IdentitySecret::generate().unwrap();
    let forged = RekeyAdoption {
        identity: Some((&done.identity.as_ref().unwrap().1, &other_ipk)),
        ..adoption(&done, &done.new_sk, &b_sk, 2)
    };
    assert!(matches!(
        b.adopt_remote_rekey(forged),
        Err(Error::IdentityMismatch)
    ));
    assert_eq!(b.ak_epoch(), 1);
    // At an equal epoch a wrapping that opens to another AK is refused as
    // an unlock failure (the password path of §9.4).
    assert!(matches!(
        b.adopt_remote_password(
            LockedSession::derive_join_keys(&done.params, &pw2(), &done.new_sk).unwrap(),
            &done.wrapped,
            done.params
        ),
        Err(Error::UnlockFailed)
    ));
}

#[test]
fn adoption_stores_the_servers_wrapping_of_a_member_vaults_current_key() {
    // A member's two devices: the re-keying one and one that adopts.
    let (_a, mut b, _c, v) = trio();
    let boot = b.s.account_bootstrap().unwrap();
    let mut b2 = LockedSession::open(Store::open_in_memory().unwrap())
        .join_account(&boot.params, &boot.wrapped_ak, &pw(), &b.secret)
        .map_err(|e| e.error)
        .unwrap();
    let key1 = b.s.store.vault_key(&v, g(1)).unwrap().unwrap();
    b2.apply_remote_vault_key(v, g(1), &key1).unwrap();
    let (book, _) = b.s.pending_contacts_upload().unwrap().unwrap();
    b2.apply_remote_contacts(&book, 1).unwrap();
    assert!(b2.vault_owner(v).is_ok());

    let new_sk = SecretKey::generate().unwrap();
    let p =
        b.s.prepare_rekey(RekeyRequest {
            current_password: &pw(),
            current_secret_key: &b.secret,
            new_password: &pw2(),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan: RekeyPlan::default(),
        })
        .unwrap();
    assert_eq!(p.member_rewraps().len(), 1);
    let served = p.member_rewraps().to_vec();
    let done = Rekeyed {
        params: *p.new_params(),
        wrapped: p.new_wrapped_ak().clone(),
        identity: p.identity(),
        new_sk: dup_sk(&new_sk),
        committed: b.s.commit_rekey(p).unwrap(),
    };
    let b2_sk = dup_sk(&b.secret);
    let mut adopt = adoption(&done, &new_sk, &b2_sk, 2);
    // A wrapping for a key this device does not hold as a member key.
    let stranger = VaultId::new_v7().unwrap();
    let bogus = [(stranger, g(1), served[0].2.clone())];
    adopt.member_wraps = &bogus;
    assert!(matches!(
        b2.adopt_remote_rekey(adopt),
        Err(Error::PlanMismatch)
    ));
    // A wrapping that opens to another key is refused.
    let mut adopt = adoption(&done, &new_sk, &b2_sk, 2);
    let other_vk = mosaic_core::VaultKey::generate(g(1)).unwrap();
    let wrong = [(
        v,
        g(1),
        other_vk
            .wrap(&b.s.account_key, &b.s.account_id, &v)
            .unwrap(),
    )];
    adopt.member_wraps = &wrong;
    assert!(matches!(
        b2.adopt_remote_rekey(adopt),
        Err(Error::UnlockFailed)
    ));
    // The served wrapping is stored byte for byte: the digest does not move.
    let mut adopt = adoption(&done, &new_sk, &b2_sk, 2);
    adopt.member_wraps = &served;
    b2.adopt_remote_rekey(adopt).unwrap();
    assert_eq!(
        b2.store.vault_key(&v, g(1)).unwrap().unwrap(),
        served[0].2,
        "the server's bytes"
    );
    assert_eq!(b.s.store.vault_key(&v, g(1)).unwrap().unwrap(), served[0].2);
}

// ---- the journal ----------------------------------------------------------------------

#[test]
fn rotation_job_survives_lock_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = account_in(Store::open(&path).unwrap());
    assert!(s.rotation_job().unwrap().is_none());
    s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
    assert!(matches!(
        s.begin_rotation_job(RotationKind::SecretKey, None),
        Err(Error::RotationInProgress)
    ));
    s.set_rotation_phase(RotationPhase::BackendStarted).unwrap();
    // Kinds 1 and 3 have no phase 4, 6 or 7; a vault job none below 5.
    drop(s.lock().into_store());
    let locked = LockedSession::open(Store::open(&path).unwrap());
    let mut s = locked.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    let job = s.rotation_job().unwrap().unwrap();
    assert_eq!(job.kind, RotationKind::Rekey);
    assert_eq!(job.phase, RotationPhase::BackendStarted);
    assert_eq!((job.from_epoch, job.to_epoch), (Some(1), Some(2)));
    // Only before the backend knows can it be abandoned.
    assert!(matches!(
        s.abandon_rotation_job(),
        Err(Error::Store(StorageError::RotationTooFar))
    ));
    // It cannot finish before the local commit.
    assert!(matches!(
        s.finish_rotation_job(),
        Err(Error::Store(StorageError::RotationNotFinished))
    ));
    s.set_rotation_phase(RotationPhase::Prepared).unwrap();
    s.abandon_rotation_job().unwrap();
    assert!(s.rotation_job().unwrap().is_none());

    s.begin_rotation_job(RotationKind::SecretKey, None).unwrap();
    for bad in [
        RotationPhase::FolderWritten,
        RotationPhase::Attachments,
        RotationPhase::Uploading,
    ] {
        assert!(matches!(
            s.set_rotation_phase(bad),
            Err(Error::Store(StorageError::InvalidRotationPhase))
        ));
    }
    // A credential change moves its job to phase 5 in the same transaction.
    let new_sk = SecretKey::generate().unwrap();
    let p = s
        .prepare_credential_change(
            &pw(),
            &sk,
            &CredentialChange {
                new_password: None,
                new_secret_key: Some(&new_sk),
                profile: Argon2Profile::Baseline,
            },
        )
        .unwrap();
    s.commit_master_password_change(p).unwrap();
    assert_eq!(
        s.rotation_job().unwrap().unwrap().phase,
        RotationPhase::LocalCommitted
    );
    s.finish_rotation_job().unwrap();
    assert!(matches!(
        s.finish_rotation_job(),
        Err(Error::Store(StorageError::NoRotationJob))
    ));
}

#[test]
fn backend_change_refused_during_rotation() {
    let (mut s, _) = account();
    s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
    // The device name may change; the backend may not.
    let mut renamed = hosted();
    renamed.device_name = "Desktop".to_owned();
    s.set_sync_config(Some(&renamed)).unwrap();
    let mut other = hosted();
    other.server_url = Some("https://two.example".to_owned());
    assert!(matches!(
        s.set_sync_config(Some(&other)),
        Err(Error::RotationInProgress)
    ));
    assert!(matches!(
        s.set_sync_config(None),
        Err(Error::RotationInProgress)
    ));
    s.abandon_rotation_job().unwrap();
    s.set_sync_config(Some(&other)).unwrap();
}

#[test]
fn account_versions_never_go_back() {
    let (mut s, _) = account();
    assert!(s.check_account_version(0).is_ok());
    s.note_account_version(5).unwrap();
    assert!(s.check_account_version(5).is_ok());
    assert!(matches!(
        s.check_account_version(4),
        Err(Error::AccountVersionRollback)
    ));
    s.note_account_version(3).unwrap();
    assert_eq!(s.highest_account_version().unwrap(), 5);
}

#[test]
fn dismissed_files_are_remembered() {
    let (mut s, _) = account();
    let sha = [9u8; 32];
    assert!(!s.file_dismissed("a/x/account/account.macct", &sha).unwrap());
    s.dismiss_unverifiable_file("a/x/account/account.macct", &sha)
        .unwrap();
    assert!(s.file_dismissed("a/x/account/account.macct", &sha).unwrap());
    assert!(
        !s.file_dismissed("a/x/account/account.macct", &[8u8; 32])
            .unwrap()
    );
    // A backend change forgets them (they name files of one folder).
    s.set_sync_config(None).unwrap();
    s.reset_sync_state().unwrap();
    assert!(!s.file_dismissed("a/x/account/account.macct", &sha).unwrap());
}
