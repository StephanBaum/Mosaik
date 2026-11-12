//! Tests of the boot marker (CRYPTO_SPEC §6.9.2, ADR-0090).

use std::path::Path;

use mosaic_core::{Argon2Profile, Kek, MasterPassword, SecretBytes, SecretKey};

use super::{
    BootId, DEFAULT_RP_ID, Error, LockedSession, MAX_FAILED_QUICK_ATTEMPTS, NewHardwareKey,
    QuickMethod, QuickUnlockPolicy, Reauthenticated, UnlockedSession,
};
use crate::storage::Store;
use mosaic_core::unlock::PrfOutput;

const DAY_MS: i64 = 86_400_000;

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account_in(store: Store) -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(store)
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

fn account() -> (UnlockedSession, SecretKey) {
    account_in(Store::open_in_memory().unwrap())
}

fn ra(s: &UnlockedSession) -> Reauthenticated {
    Reauthenticated::for_test(s.proof_id)
}

fn boot(fill: u8) -> BootId {
    BootId::new(&[fill; 4]).unwrap()
}

fn w() -> Kek {
    Kek::from_secret(SecretBytes::random().unwrap())
}

fn marker(s: &UnlockedSession) -> Option<Vec<u8>> {
    s.store().boot_marker().unwrap().map(|m| m.boot_id)
}

fn locked_marker(s: &LockedSession) -> Option<Vec<u8>> {
    s.store().boot_marker().unwrap().map(|m| m.boot_id)
}

/// A file account with a marker and a quick-unlock enrollment, closed.
fn closed_phone(path: &Path, boot_id: &BootId, w: &Kek) -> SecretKey {
    let (mut s, sk) = account_in(Store::open(path).unwrap());
    s.record_boot_marker(boot_id, 1_000).unwrap();
    s.enroll_quick_unlock(w, &[4, 1], &ra(&s)).unwrap();
    drop(s.lock().into_store());
    sk
}

#[test]
fn boot_marker_lets_quick_unlock_in_a_new_session_same_boot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let w = w();
    let _sk = closed_phone(&path, &boot(7), &w);

    // A new process: the app-start rule would ask for the password.
    let mut locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
    let at = locked
        .quick_unlock_enrollment()
        .unwrap()
        .unwrap()
        .enrolled_at_ms;
    assert!(locked.adopt_boot_marker(&boot(7)).unwrap());
    assert!(!locked.password_required(QuickMethod::Biometric));
    let s = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), at + 1000)
        .unwrap();
    // The quick-unlocked session keeps the marker, and cannot refresh it.
    assert_eq!(marker(&s), Some(vec![7; 4]));
}

#[test]
fn boot_marker_from_another_boot_requires_password() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let w = w();
    let sk = closed_phone(&path, &boot(7), &w);

    let mut locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(!locked.adopt_boot_marker(&boot(8)).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
    let at = locked
        .quick_unlock_enrollment()
        .unwrap()
        .unwrap()
        .enrolled_at_ms;
    let err = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), at)
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    // A different length is a different boot too.
    let mut locked = err.session;
    assert!(
        !locked
            .adopt_boot_marker(&BootId::new(&[7; 5]).unwrap())
            .unwrap()
    );
    // Neither refusal deleted anything; the password still works.
    assert_eq!(locked_marker(&locked), Some(vec![7; 4]));
    assert!(locked.quick_unlock_enrollment().unwrap().is_some());
    locked.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
}

#[test]
fn quick_unlock_without_boot_marker_requires_password() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, _) = account_in(Store::open(&path).unwrap());
    let w = w();
    s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    assert_eq!(marker(&s), None);
    drop(s.lock().into_store());

    let mut locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(!locked.adopt_boot_marker(&boot(1)).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
}

#[test]
fn record_boot_marker_refused_after_quick_unlock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let w = w();
    let sk = closed_phone(&path, &boot(7), &w);
    let mut locked = LockedSession::open(Store::open(&path).unwrap());
    let at = locked
        .quick_unlock_enrollment()
        .unwrap()
        .unwrap()
        .enrolled_at_ms;
    assert!(locked.adopt_boot_marker(&boot(7)).unwrap());
    let mut s = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), at + 1)
        .unwrap();
    assert!(matches!(
        s.record_boot_marker(&boot(9), 5),
        Err(Error::ReauthenticationRequired)
    ));
    assert_eq!(marker(&s), Some(vec![7; 4]), "not refreshed");

    // A lock and a quick unlock again do not make the session fresh.
    let locked = s.lock();
    let mut s = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), at + 2)
        .unwrap();
    assert!(s.record_boot_marker(&boot(9), 5).is_err());

    // A password unlock does, and replaces the marker.
    let mut s = s.lock().unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    s.record_boot_marker(&boot(9), 6).unwrap();
    assert_eq!(marker(&s), Some(vec![9; 4]));
    assert_eq!(s.store().boot_marker().unwrap().unwrap().recorded_at_ms, 6);
}

#[test]
fn boot_marker_deleted_with_every_quick_unlock_deletion() {
    let policy = QuickUnlockPolicy::default();
    let enrolled = |s: &mut UnlockedSession| {
        s.record_boot_marker(&boot(3), 0).unwrap();
        let (e, _) = s.enroll_quick_unlock(&w(), &[], &ra(s)).unwrap();
        assert_eq!(marker(s), Some(vec![3; 4]), "enrolling keeps the marker");
        e
    };

    // remove_quick_unlock, unlocked and locked.
    let (mut s, _) = account();
    enrolled(&mut s);
    assert!(s.remove_quick_unlock().unwrap().is_some());
    assert_eq!(marker(&s), None);
    let (mut s, _) = account();
    enrolled(&mut s);
    let mut locked = s.lock();
    assert!(locked.remove_quick_unlock().unwrap().is_some());
    assert_eq!(locked_marker(&locked), None);

    // Expiry.
    let (mut s, _) = account();
    let e = enrolled(&mut s);
    let err = s
        .lock()
        .unlock_quick(&w(), &policy, e.enrolled_at_ms + 31 * DAY_MS)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockExpired));
    assert_eq!(locked_marker(&err.session), None);

    // Invalidation: a W that does not open the wrapping.
    let (mut s, _) = account();
    let e = enrolled(&mut s);
    let err = s
        .lock()
        .unlock_quick(&w(), &policy, e.enrolled_at_ms)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockInvalidated));
    assert_eq!(locked_marker(&err.session), None);

    // A master password change.
    let (mut s, sk) = account();
    enrolled(&mut s);
    let new_pw = MasterPassword::new("another password".to_owned());
    s.change_master_password(&pw(), &new_pw, &sk).unwrap();
    assert_eq!(marker(&s), None);
    // (A Secret Key rotation and a re-key are checked in `rotation_tests`.)
}

#[test]
fn boot_marker_deleted_by_lock_requiring_password() {
    let (mut s, sk) = account();
    s.record_boot_marker(&boot(3), 0).unwrap();
    let mut locked = s.lock_requiring_password();
    assert_eq!(locked_marker(&locked), None);
    // A new process of the app cannot adopt it, and the password works.
    assert!(!locked.adopt_boot_marker(&boot(3)).unwrap());
    assert!(locked.password_required(QuickMethod::HardwareKey));
    let s = locked.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    drop(s);
}

#[test]
fn undeletable_boot_marker_is_overwritten_so_no_boot_matches() {
    // G1-13: a failed delete must not leave a marker a new process adopts.
    let (mut s, sk) = account();
    s.record_boot_marker(&boot(3), 0).unwrap();
    s.store.block_boot_marker_changes_for_test(true);
    let mut locked = s.lock_requiring_password();
    assert_ne!(locked_marker(&locked), Some(vec![3; 4]), "neutralised");
    assert!(!locked.adopt_boot_marker(&boot(3)).unwrap());
    assert!(locked.password_required(QuickMethod::HardwareKey));
    assert!(locked.enforce_boot_marker_limit().is_ok());
    let s = locked.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    drop(s);
}

#[test]
fn unwritable_boot_marker_is_refused_loudly_and_counted() {
    let (mut s, _) = account();
    s.enroll_hardware_key(&hw_key(1), &prf(2), &ra(&s)).unwrap();
    s.record_boot_marker(&boot(3), 0).unwrap();
    s.store.block_boot_marker_changes_for_test(false);
    let mut locked = s.lock();
    for _ in 0..MAX_FAILED_QUICK_ATTEMPTS - 1 {
        let err = locked
            .unlock_with_hardware_key(&[1; 32], &prf(9))
            .unwrap_err();
        assert!(matches!(err.error, Error::HardwareKeyRejected));
        locked = err.session;
    }
    // The limit is reached and the marker can be neither deleted nor
    // overwritten: the attempt is refused with a store error, not swallowed.
    let err = locked
        .unlock_with_hardware_key(&[1; 32], &prf(9))
        .unwrap_err();
    assert!(matches!(err.error, Error::Store(_)), "{:?}", err.error);
    let mut locked = err.session;
    assert!(locked.password_required(QuickMethod::HardwareKey));
    assert!(locked.enforce_boot_marker_limit().is_err());
    // The same through the infallible paths: the limit still holds.
    locked.note_failed_hardware_attempt();
    assert!(locked.password_required(QuickMethod::Biometric));
}

fn hw_key(fill: u8) -> NewHardwareKey {
    NewHardwareKey::new(vec![fill; 32], DEFAULT_RP_ID.to_owned(), [fill ^ 0x5A; 32]).unwrap()
}

fn prf(fill: u8) -> PrfOutput {
    PrfOutput::from_secret(SecretBytes::from_array(&mut [fill; 32]))
}

#[test]
fn boot_marker_deleted_at_failed_quick_limit() {
    let (mut s, _) = account();
    s.enroll_hardware_key(&hw_key(1), &prf(2), &ra(&s)).unwrap();
    s.record_boot_marker(&boot(3), 0).unwrap();
    let mut locked = s.lock();
    // Rejected hardware-key attempts: the marker stays until the limit.
    for _ in 0..MAX_FAILED_QUICK_ATTEMPTS - 1 {
        let err = locked
            .unlock_with_hardware_key(&[1; 32], &prf(9))
            .unwrap_err();
        assert!(matches!(err.error, Error::HardwareKeyRejected));
        locked = err.session;
        assert_eq!(locked_marker(&locked), Some(vec![3; 4]));
    }
    let err = locked
        .unlock_with_hardware_key(&[1; 32], &prf(9))
        .unwrap_err();
    assert!(matches!(err.error, Error::HardwareKeyRejected));
    assert_eq!(locked_marker(&err.session), None, "limit reached");

    // The attempts the vault cannot see count the same way.
    let (mut s, _) = account();
    s.record_boot_marker(&boot(3), 0).unwrap();
    let mut locked = s.lock();
    for _ in 0..MAX_FAILED_QUICK_ATTEMPTS - 1 {
        locked.note_failed_hardware_attempt();
        assert_eq!(locked_marker(&locked), Some(vec![3; 4]));
    }
    locked.note_failed_hardware_attempt();
    assert_eq!(locked_marker(&locked), None);
}

#[test]
fn boot_id_must_be_1_to_32_bytes() {
    assert!(matches!(BootId::new(&[]), Err(Error::InvalidInput)));
    assert!(BootId::new(&[0]).is_ok());
    assert!(BootId::new(&[0; 32]).is_ok());
    assert!(matches!(BootId::new(&[0; 33]), Err(Error::InvalidInput)));
    assert_eq!(BootId::new(&[1, 2]).unwrap().as_bytes(), [1, 2]);
}

#[test]
fn desktop_app_start_rule_unchanged_without_marker() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = account_in(Store::open(&path).unwrap());
    let w = w();
    let (e, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    drop(s.lock().into_store());

    // A new app start needs the password once, then quick unlock works.
    let locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
    let err = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), e.enrolled_at_ms)
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    let s = err.session.unlock(&pw(), &sk).map_err(|e| e.error).unwrap();
    // The desktop never writes a marker.
    assert_eq!(marker(&s), None);
    let locked = s.lock();
    assert!(!locked.password_required(QuickMethod::Biometric));
    locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), e.enrolled_at_ms)
        .unwrap();
}
