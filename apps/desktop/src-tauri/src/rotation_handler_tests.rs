//! Tests of the account keys commands (M7 T5): every command locked, the
//! Secret Key rotation and the profile upgrade on a local account, the journal
//! (abandon, resume), the re-key on a folder and on the real server with a
//! second device adopting it, and the refusal of a Secret Key the device has
//! held. The hosted test runs the real server on `127.0.0.1`.

use mosaic_vault::session::RotationPhase;

use crate::contract::errors::ErrorCode;
use crate::contract::events::SyncStatus;
use crate::contract::rotation::{
    DismissFileRequest, ProfileUpgradeRequest, RekeyBackend, RekeyCommitRequest,
    RekeyPrepareRequest, RotateCommitRequest, RotatePrepareRequest, RotationKind, RotationNeeds,
    RotationPhaseName, RotationProfile, RotationResumeRequest,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{UnlockRequest, VerifyKitRequest};
use crate::contract::sync::{
    JoinHostedRequest, JoinOutcome, RecoverPasswordRequest, SyncWarningView,
};
use crate::keystore::SecretKeyStore as _;
use crate::sync_handler_tests::loopback::LoopbackServer;
use crate::sync_handler_tests::{Env, PASSWORD, wait_for};
use crate::test_support::kdf_permit;

const NEW_PASSWORD: &str = "a brand new master password";

pub(crate) fn now_ms() -> u64 {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    u64::try_from(since.as_millis()).unwrap()
}

/// A CLI session of `account` as `mosaic unlock` would have left it (the app
/// never creates one; the binding does not matter to the sweep and the delete).
pub(crate) fn plant_cli_session(
    b: &crate::state::Backend,
    account: mosaic_core::AccountId,
    start: u64,
) {
    use mosaic_core::{AccountKey, AccountSalt, CliSessionBinding, InstallId, seal_cli_session};
    let binding = CliSessionBinding {
        account_id: account,
        install_id: InstallId::new_v7().unwrap(),
        ak_epoch: 1,
        salt: AccountSalt::generate().unwrap(),
    };
    let (key, file) =
        seal_cli_session(&AccountKey::generate().unwrap(), &binding, start, 60_000).unwrap();
    b.cli_sessions.save(account, &key, &file).unwrap();
}

pub(crate) fn has_cli_session(b: &crate::state::Backend, account: mosaic_core::AccountId) -> bool {
    b.cli_sessions.load(account).unwrap().is_some()
}

fn prepare() -> RotatePrepareRequest {
    RotatePrepareRequest {
        password: SecretIn::test(PASSWORD),
    }
}

fn commit() -> RotateCommitRequest {
    RotateCommitRequest {
        password: SecretIn::test(PASSWORD),
        code: None,
    }
}

fn rekey_commit(code: Option<&str>) -> RekeyCommitRequest {
    RekeyCommitRequest {
        password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
        code: code.map(SecretIn::test),
    }
}

/// The last group of the staged Secret Key, as the user types it from the new kit.
fn pending_group(e: &Env) -> String {
    let key = e
        .b()
        .pending_secret_keys
        .load(e.account())
        .unwrap()
        .expect("a staged Secret Key");
    let display = key.to_display().unwrap();
    display
        .expose_secret()
        .rsplit('-')
        .next()
        .unwrap()
        .to_owned()
}

fn verify_new_kit(e: &Env, group: &str) -> bool {
    e.b()
        .verify_emergency_kit(&VerifyKitRequest {
            last_group: SecretIn::test(group),
            pending: true,
        })
        .unwrap()
        .matches
}

#[test]
fn every_command_is_locked() {
    let e = Env::new();
    let b = e.b();
    let locked = |code: ErrorCode| assert_eq!(code, ErrorCode::Locked);
    locked(b.rotation_status().unwrap_err().code);
    locked(b.secret_key_rotate_prepare(&prepare()).unwrap_err().code);
    locked(b.secret_key_rotate_commit(&commit()).unwrap_err().code);
    locked(
        b.profile_upgrade(&ProfileUpgradeRequest {
            password: SecretIn::test(PASSWORD),
            profile: RotationProfile::Strong,
            code: None,
            force: true,
        })
        .unwrap_err()
        .code,
    );
    locked(b.rekey_review_view().unwrap_err().code);
    locked(
        b.rekey_prepare(&RekeyPrepareRequest {
            password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
        })
        .unwrap_err()
        .code,
    );
    locked(b.rekey_commit(&rekey_commit(None)).unwrap_err().code);
    locked(
        b.rotation_resume(&RotationResumeRequest {
            password: None,
            kit_group: None,
            code: None,
        })
        .unwrap_err()
        .code,
    );
    locked(b.rotation_abandon().unwrap_err().code);
    locked(b.rotation_abandon_committed().unwrap_err().code);
    locked(b.sync_rekey_markers().unwrap_err().code);
    locked(b.sync_ignored_rekey_markers().unwrap_err().code);
    locked(
        b.sync_restore_rekey_marker(&DismissFileRequest {
            name: "rekey-00000002.mrk".into(),
            sha256: "cd".repeat(32),
        })
        .unwrap_err()
        .code,
    );
    locked(
        b.sync_dismiss_rekey_marker(&DismissFileRequest {
            name: "rekey-00000002.mrk".into(),
            sha256: "cd".repeat(32),
        })
        .unwrap_err()
        .code,
    );
    locked(
        b.sync_dismiss_account_file(&DismissFileRequest {
            name: "account-00000002.macct".into(),
            sha256: "ab".repeat(32),
        })
        .unwrap_err()
        .code,
    );
}

#[test]
fn secret_key_rotation_replaces_the_key_only_after_a_verified_kit() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let old = e.create();
    let b = e.b();
    assert_eq!(
        b.secret_key_rotate_commit(&commit()).unwrap_err().code,
        ErrorCode::KitNotVerified
    );
    let wrong = b.secret_key_rotate_prepare(&RotatePrepareRequest {
        password: SecretIn::test("not the password"),
    });
    assert_eq!(wrong.unwrap_err().code, ErrorCode::WrongCredentials);
    assert!(b.rotation_status().unwrap().job.is_none());

    b.secret_key_rotate_prepare(&prepare()).unwrap();
    let job = b.rotation_status().unwrap().job.unwrap();
    assert_eq!(job.kind, RotationKind::SecretKey);
    assert_eq!(job.phase, RotationPhaseName::Prepared);
    assert_eq!(job.needs, RotationNeeds::Password);
    // The same prepare again keeps the staged key; another kind is refused.
    let staged = pending_group(&e);
    b.secret_key_rotate_prepare(&prepare()).unwrap();
    assert_eq!(pending_group(&e), staged);
    let other = b.profile_upgrade(&ProfileUpgradeRequest {
        password: SecretIn::test(PASSWORD),
        profile: RotationProfile::Standard,
        code: None,
        force: true,
    });
    assert_eq!(other.unwrap_err().code, ErrorCode::RotationInProgress);

    // The backend (here: nothing) learns nothing before the kit is verified.
    assert_eq!(
        b.secret_key_rotate_commit(&commit()).unwrap_err().code,
        ErrorCode::KitNotVerified
    );
    assert!(!verify_new_kit(&e, "ZZZZ"));
    assert_eq!(
        b.secret_key_rotate_commit(&commit()).unwrap_err().code,
        ErrorCode::KitNotVerified
    );
    assert!(verify_new_kit(&e, &staged));
    plant_cli_session(b, e.account(), now_ms());
    let done = b.secret_key_rotate_commit(&commit()).unwrap();
    assert!(done.finished);
    assert!(
        !has_cli_session(b, e.account()),
        "a new Secret Key ends CLI sessions"
    );
    assert!(b.rotation_status().unwrap().job.is_none());
    assert!(b.pending_secret_keys.load(e.account()).unwrap().is_none());

    let stored = e.keys.load(e.account()).unwrap().unwrap();
    assert_ne!(stored.to_display().unwrap().expose_secret(), old);
    // The old key no longer opens the account; the new one does.
    b.lock(crate::contract::events::LockReason::Manual).unwrap();
    let with_old = b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test(&old)),
    });
    assert_eq!(with_old.unwrap_err().code, ErrorCode::WrongCredentials);
    e.unlock().unwrap();
}

#[test]
fn abandon_works_before_the_backend_knows_and_resume_needs_its_input() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    assert_eq!(
        b.rotation_abandon().unwrap_err().code,
        ErrorCode::RotationNone
    );
    b.secret_key_rotate_prepare(&prepare()).unwrap();
    // Phase 1 is wizard territory, not a resume.
    let resume = RotationResumeRequest {
        password: None,
        kit_group: None,
        code: None,
    };
    assert_eq!(
        b.rotation_resume(&resume).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    b.rotation_abandon().unwrap();
    assert!(b.pending_secret_keys.load(e.account()).unwrap().is_none());
    assert!(b.rotation_status().unwrap().job.is_none());

    // Once the change may have reached the backend, it cannot be discarded.
    b.secret_key_rotate_prepare(&prepare()).unwrap();
    b.with_unlocked(|s| Ok(s.set_rotation_phase(RotationPhase::BackendStarted)?))
        .unwrap();
    assert_eq!(
        b.rotation_status().unwrap().job.unwrap().needs,
        RotationNeeds::KitGroup
    );
    assert_eq!(
        b.rotation_abandon().unwrap_err().code,
        ErrorCode::RotationTooFar
    );
    assert!(b.pending_secret_keys.load(e.account()).unwrap().is_some());
    // The kit group is checked before anything else.
    let group_missing = b.rotation_resume(&RotationResumeRequest {
        password: Some(SecretIn::test(PASSWORD)),
        kit_group: None,
        code: None,
    });
    assert_eq!(group_missing.unwrap_err().code, ErrorCode::InvalidInput);
    let wrong_group = b.rotation_resume(&RotationResumeRequest {
        password: Some(SecretIn::test(PASSWORD)),
        kit_group: Some(SecretIn::test("ZZZZ")),
        code: None,
    });
    assert_eq!(wrong_group.unwrap_err().code, ErrorCode::KitGroupMismatch);
}

#[test]
fn the_app_sweeps_expired_cli_sessions_at_lock_and_unlock() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    let account = e.account();
    // Started at 20 minutes ago for one minute: long expired.
    plant_cli_session(b, account, now_ms() - 20 * 60_000);
    assert!(has_cli_session(b, account));
    b.lock(crate::contract::events::LockReason::Manual).unwrap();
    assert!(!has_cli_session(b, account), "swept at lock");

    // A live session survives lock and unlock; an expired one goes at unlock.
    plant_cli_session(b, account, now_ms());
    b.lock(crate::contract::events::LockReason::Manual).unwrap();
    assert!(has_cli_session(b, account), "not expired: stays");
    b.cli_sessions.delete_all().unwrap();
    plant_cli_session(b, account, now_ms() - 20 * 60_000);
    e.unlock().unwrap();
    assert!(!has_cli_session(b, account), "swept at unlock");
}

#[test]
fn a_master_password_change_deletes_every_cli_session() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    plant_cli_session(b, e.account(), now_ms());
    b.change_master_password(&crate::contract::sync::ChangePasswordRequest {
        current_password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
        code: None,
    })
    .unwrap();
    assert!(!has_cli_session(b, e.account()));
}

#[test]
fn profile_upgrade_only_goes_up() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    let up = |profile| ProfileUpgradeRequest {
        password: SecretIn::test(PASSWORD),
        profile,
        code: None,
        force: true,
    };
    let done = b.profile_upgrade(&up(RotationProfile::Standard)).unwrap();
    assert!(done.finished);
    assert!(b.rotation_status().unwrap().job.is_none());
    // The same level is not an upgrade.
    assert_eq!(
        b.profile_upgrade(&up(RotationProfile::Standard))
            .unwrap_err()
            .code,
        ErrorCode::ProfileDowngrade
    );
    // The new parameters open the account.
    b.lock(crate::contract::events::LockReason::Manual).unwrap();
    e.unlock().unwrap();
}

#[test]
fn rekey_needs_sync_a_different_password_and_a_verified_kit() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    let request = RekeyPrepareRequest {
        password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
    };
    // Nothing to renew without a backend.
    assert_eq!(
        b.rekey_prepare(&request).unwrap_err().code,
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(b.rekey_review_view().unwrap().backend, RekeyBackend::Local);
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let same = b.rekey_prepare(&RekeyPrepareRequest {
        password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(PASSWORD),
    });
    assert_eq!(same.unwrap_err().code, ErrorCode::PasswordUnchanged);
    assert_eq!(
        b.rekey_commit(&rekey_commit(None)).unwrap_err().code,
        ErrorCode::KitNotVerified
    );
    b.rekey_prepare(&request).unwrap();
    assert_eq!(
        b.rekey_commit(&rekey_commit(None)).unwrap_err().code,
        ErrorCode::KitNotVerified
    );
    // A backend change is refused while the job is open.
    assert!(b.sync_disconnect().is_err());
    b.rotation_abandon().unwrap();
}

#[test]
fn folder_rekey_renews_every_key_and_the_new_credentials_open_the_account() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let old = e.create();
    let b = e.b();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let review = b.rekey_review_view().unwrap();
    assert_eq!(review.backend, RekeyBackend::Folder);
    assert_eq!(review.owned_vaults, 1);
    assert_eq!(review.shared_owned, vec![]);

    b.rekey_prepare(&RekeyPrepareRequest {
        password: SecretIn::test(PASSWORD),
        new_password: SecretIn::test(NEW_PASSWORD),
    })
    .unwrap();
    assert!(verify_new_kit(&e, &pending_group(&e)));
    plant_cli_session(b, e.account(), now_ms());
    let done = b.rekey_commit(&rekey_commit(None)).unwrap();
    assert!(
        !has_cli_session(b, e.account()),
        "a re-key ends CLI sessions"
    );
    assert!(done.finished, "the data followed");
    assert!(b.rotation_status().unwrap().job.is_none());
    assert_eq!(b.with_unlocked(|s| Ok(s.ak_epoch())).unwrap(), 2);
    assert!(b.pending_secret_keys.load(e.account()).unwrap().is_none());
    let stored = e.keys.load(e.account()).unwrap().unwrap();
    assert_ne!(stored.to_display().unwrap().expose_secret(), old);

    // Sync carries on under the new keys.
    b.sync_now().unwrap();
    e.wait_synced();
    // The old password is refused, the new one opens the account.
    b.lock(crate::contract::events::LockReason::Manual).unwrap();
    assert_eq!(e.unlock().unwrap_err().code, ErrorCode::WrongCredentials);
    b.unlock(UnlockRequest {
        password: SecretIn::test(NEW_PASSWORD),
        secret_key: None,
    })
    .unwrap();
}

#[test]
fn a_secret_key_this_device_held_is_refused_with_its_own_message() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let own = e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let refused = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test(&own)),
        code: None,
    });
    assert_eq!(refused.unwrap_err().code, ErrorCode::SecretKeyUnchanged);
    let malformed = e.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test(PASSWORD),
        secret_key: Some(SecretIn::test("not a key")),
        code: None,
    });
    assert_eq!(malformed.unwrap_err().code, ErrorCode::SecretKeyMalformed);
}

#[test]
fn dismissing_an_account_file_checks_its_input() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let bad = |name: &str, sha: &str| {
        e.b()
            .sync_dismiss_account_file(&DismissFileRequest {
                name: name.into(),
                sha256: sha.into(),
            })
            .unwrap_err()
            .code
    };
    assert_eq!(
        bad("account-00000002.macct", "xyz"),
        ErrorCode::InvalidInput
    );
    assert_eq!(bad("", &"ab".repeat(32)), ErrorCode::InvalidInput);
    assert_eq!(bad("bad\nname", &"ab".repeat(32)), ErrorCode::InvalidInput);
    let ok = e.b().sync_dismiss_account_file(&DismissFileRequest {
        name: "account-00000002.macct".into(),
        sha256: "ab".repeat(32),
    });
    assert!(ok.is_ok());
    assert!(
        !e.status()
            .warnings
            .iter()
            .any(|w| matches!(w, SyncWarningView::UnverifiedAccountFile { .. }))
    );
}

#[test]
fn abandoning_a_committed_job_needs_a_committed_job_and_markers_need_a_folder() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    assert_eq!(
        b.rotation_abandon_committed().unwrap_err().code,
        ErrorCode::RotationNone
    );
    // A job the backend may already know is not abandoned here.
    b.secret_key_rotate_prepare(&prepare()).unwrap();
    assert_eq!(
        b.rotation_abandon_committed().unwrap_err().code,
        ErrorCode::RotationTooFar
    );
    // A local account has no folder, so no notices.
    assert_eq!(b.sync_rekey_markers().unwrap().len(), 0);
    assert_eq!(b.sync_ignored_rekey_markers().unwrap().len(), 0);
    let bad = |name: &str, sha: &str| {
        b.sync_dismiss_rekey_marker(&DismissFileRequest {
            name: name.into(),
            sha256: sha.into(),
        })
        .unwrap_err()
        .code
    };
    assert_eq!(bad("rekey-00000002.mrk", "xyz"), ErrorCode::InvalidInput);
    assert_eq!(bad("", &"cd".repeat(32)), ErrorCode::InvalidInput);
}

/// The account endpoint serves the new epoch, and a device still at epoch 1 reads it as a re-key
/// (an equal or lower one is not, and an unknown local epoch is not guessed at).
fn assert_epoch_served_as_rekey(rekeyed: &Env, base: &str) {
    let token = rekeyed
        .b()
        .sync_tokens
        .load(rekeyed.account())
        .unwrap()
        .unwrap();
    let (_, served) = mosaic_sync::http::hosted::account_epoch(base, &token).unwrap();
    assert_eq!(served, 2);
    let code = |local| crate::sync::password_change_code(local, served);
    assert_eq!(code(Some(1)), ErrorCode::SyncAccountRekeyed);
    assert_eq!(code(Some(2)), ErrorCode::SyncPasswordChanged);
    assert_eq!(code(Some(3)), ErrorCode::SyncPasswordChanged);
    assert_eq!(code(None), ErrorCode::SyncPasswordChanged);
}

#[test]
fn hosted_rekey_signs_the_other_device_out_and_the_new_kit_brings_it_back() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let a = Env::new();
    let old = a.create();
    a.connect_hosted(&server);
    a.wait_synced();
    let b = Env::new();
    let joined = b
        .b()
        .join_hosted(JoinHostedRequest {
            server_url: server.base.clone(),
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            secret_key: SecretIn::test(&old),
            device_name: "Laptop".into(),
        })
        .unwrap();
    assert_eq!(joined, JoinOutcome::Unlocked);
    b.wait_synced();

    a.b()
        .rekey_prepare(&RekeyPrepareRequest {
            password: SecretIn::test(PASSWORD),
            new_password: SecretIn::test(NEW_PASSWORD),
        })
        .unwrap();
    assert!(verify_new_kit(&a, &pending_group(&a)));
    let done = a.b().rekey_commit(&rekey_commit(None)).unwrap();
    assert!(done.finished);
    assert_eq!(a.b().with_unlocked(|s| Ok(s.ak_epoch())).unwrap(), 2);
    a.b().sync_now().unwrap();
    a.wait_synced();

    assert_epoch_served_as_rekey(&a, &server.base);

    // The other device is signed out; the old key is not accepted as the way back.
    b.b().sync_now().ok();
    wait_for("the other device to be signed out", || {
        b.status().state.status == SyncStatus::SignedOut
    });
    let new_key = a
        .keys
        .load(a.account())
        .unwrap()
        .unwrap()
        .to_display()
        .unwrap()
        .expose_secret()
        .to_owned();
    assert_ne!(new_key, old);
    let refused = b.b().sync_recover_password(&RecoverPasswordRequest {
        new_password: SecretIn::test(NEW_PASSWORD),
        secret_key: Some(SecretIn::test(&old)),
        code: None,
    });
    assert_eq!(refused.unwrap_err().code, ErrorCode::SecretKeyUnchanged);
    let adopted = b
        .b()
        .sync_recover_password(&RecoverPasswordRequest {
            new_password: SecretIn::test(NEW_PASSWORD),
            secret_key: Some(SecretIn::test(&new_key)),
            code: None,
        })
        .unwrap();
    assert!(!adopted.password_changed);
    assert!(!adopted.rekey_pending);
    assert_eq!(b.b().with_unlocked(|s| Ok(s.ak_epoch())).unwrap(), 2);
    b.b().sync_now().unwrap();
    b.wait_synced();
    // The adopted keys are the keys in use.
    b.b()
        .lock(crate::contract::events::LockReason::Manual)
        .unwrap();
    b.b()
        .unlock(UnlockRequest {
            password: SecretIn::test(NEW_PASSWORD),
            secret_key: None,
        })
        .unwrap();
}

// ---- vault_rotate_key (M7-R2) ---------------------------------------------------

fn attachment_ids_of(
    b: &crate::state::Backend,
    vault: mosaic_core::VaultId,
    item: mosaic_core::ItemId,
) -> Vec<mosaic_core::AttachmentId> {
    let mut ids: Vec<_> = b
        .with_unlocked(|s| {
            Ok(s.item(vault, item)
                .unwrap()
                .attachments()
                .into_iter()
                .map(|a| a.attachment_id)
                .collect())
        })
        .unwrap();
    ids.sort_unstable_by_key(|a: &mosaic_core::AttachmentId| *a.as_bytes());
    ids
}

fn rotate_request(
    vault: mosaic_core::VaultId,
    reseal: bool,
    proof: Option<&str>,
) -> crate::contract::rotation::VaultRotateKeyRequest {
    crate::contract::rotation::VaultRotateKeyRequest {
        vault_id: vault.into(),
        reseal_attachments: reseal,
        proof: proof.map(SecretIn::test),
    }
}

#[test]
fn vault_rotate_key_needs_the_proof_and_a_backend() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let b = e.b();
    let vault = mosaic_core::VaultId::new_v7().unwrap();
    // Locked, before an account exists.
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, Some(PASSWORD)))
            .unwrap_err()
            .code,
        ErrorCode::Locked
    );
    e.create();
    let vault = b.list_vaults().unwrap()[0].vault_id.get();
    // No proof, then a wrong password: nothing is rotated.
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, None))
            .unwrap_err()
            .code,
        ErrorCode::ReauthRequired
    );
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, Some("not the password")))
            .unwrap_err()
            .code,
        ErrorCode::WrongCredentials
    );
    // A right password on a device without sync: a vault key is offered to a backend.
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, Some(PASSWORD)))
            .unwrap_err()
            .code,
        ErrorCode::SyncNotConfigured
    );
}

/// M9-F5: the proof is taken before the rotation runs and spent by a failed
/// call too, so it cannot be used twice.
#[test]
fn vault_rotate_key_spends_the_proof_also_when_it_fails() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    let vault = b.list_vaults().unwrap()[0].vault_id.get();
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    // No backend: the first call fails after the proof was taken.
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, None))
            .unwrap_err()
            .code,
        ErrorCode::SyncNotConfigured
    );
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, None))
            .unwrap_err()
            .code,
        ErrorCode::ReauthRequired,
        "a failed call spent the proof"
    );
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario: rotate, crash, resume, check every attachment")]
fn vault_rotate_key_reseals_attachments_and_resumes() {
    let _kdf = kdf_permit();
    let own = tempfile::tempdir().unwrap();
    let e = Env::on_disk(own.path());
    e.create();
    let b = e.b();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    let vault = b.list_vaults().unwrap()[0].vault_id.get();
    let item = b
        .add_item(
            serde_json::from_value(serde_json::json!({
                "vaultId": vault.to_string(),
                "kind": "document",
                "draft": {
                    "text": [{ "field": "title", "value": "Passport scan" }],
                    "secrets": [],
                    "urls": [],
                    "customFields": [],
                    "tags": null,
                    "otp": null
                }
            }))
            .unwrap(),
        )
        .unwrap()
        .summary
        .item_id
        .get();
    for fill in [1_u8, 2] {
        b.with_unlocked(|s| {
            Ok(s.add_attachment(
                vault,
                item,
                "f.bin",
                "application/octet-stream",
                &[fill; 64],
            )?)
        })
        .unwrap();
    }
    let before = attachment_ids_of(b, vault, item);
    assert_eq!(before.len(), 2);

    // The proof of `reauthenticate` is enough, and is spent on this rotation.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let done = b
        .vault_rotate_key(rotate_request(vault, true, None))
        .unwrap();
    assert!(done.finished, "{done:?}");
    assert_eq!(done.unreachable_attachments, 0);
    assert!(b.rotation_status().unwrap().job.is_none());
    let after = attachment_ids_of(b, vault, item);
    assert_eq!(after.len(), 2);
    assert!(after.iter().all(|a| !before.contains(a)), "new IDs");
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, false, None))
            .unwrap_err()
            .code,
        ErrorCode::ReauthRequired,
        "the proof is spent"
    );

    // An interrupted re-seal: the job is open, no second rotation starts on
    // top of it, and "Continue" (`rotation_resume`) finishes it.
    b.with_unlocked(|s| Ok(s.queue_vault_attachment_reseal(vault)?))
        .unwrap();
    let job = b.rotation_status().unwrap().job.unwrap();
    assert_eq!(job.kind, RotationKind::Vault);
    assert_eq!(
        b.vault_rotate_key(rotate_request(vault, true, Some(PASSWORD)))
            .unwrap_err()
            .code,
        ErrorCode::RotationInProgress
    );
    let resumed = b
        .rotation_resume(&RotationResumeRequest {
            password: None,
            kit_group: None,
            code: None,
        })
        .unwrap();
    assert!(resumed.finished, "{resumed:?}");
    assert!(b.rotation_status().unwrap().job.is_none());
    let last = attachment_ids_of(b, vault, item);
    assert_eq!(last.len(), 2);
    assert!(last.iter().all(|a| !after.contains(a)), "new IDs again");

    // M9-F1: a job opened for an offer whose key never reached the backend
    // (a crash right after it was opened) needs nothing from the user, and
    // the resume drops it without touching the attachments.
    let next = b
        .with_unlocked(|s| {
            Ok(s.vaults()
                .into_iter()
                .find(|v| v.vault_id == vault)
                .unwrap()
                .key_generation
                .next()
                .unwrap())
        })
        .unwrap();
    b.with_unlocked(|s| Ok(s.open_vault_offer_job(vault, next)?))
        .unwrap();
    let job = b.rotation_status().unwrap().job.unwrap();
    assert_eq!(job.kind, RotationKind::Vault);
    let resumed = b
        .rotation_resume(&RotationResumeRequest {
            password: None,
            kit_group: None,
            code: None,
        })
        .unwrap();
    assert!(resumed.finished, "{resumed:?}");
    assert!(b.rotation_status().unwrap().job.is_none());
    assert_eq!(attachment_ids_of(b, vault, item), last);
}
