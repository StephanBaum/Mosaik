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
    let done = b.secret_key_rotate_commit(&commit()).unwrap();
    assert!(done.finished);
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
    let done = b.rekey_commit(&rekey_commit(None)).unwrap();
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
