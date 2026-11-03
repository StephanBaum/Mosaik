//! Tests of quick unlock, hardware-key wrappings and re-authentication
//! (CRYPTO_SPEC §6.2, §6.3, ADR-0028).

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

use core::time::Duration;
use std::path::Path;

use mosaic_core::model::ItemType;
use mosaic_core::unlock::{PrfOutput, prf_unlock_key};
use mosaic_core::{Argon2Profile, Kek, MasterPassword, SecretBytes, SecretKey, UnlockMethod};
use proptest::prelude::*;

use super::unlock_methods::{decode_hw_params, encode_hw_params};
use super::{
    DEFAULT_RP_ID, Error, LockedSession, MAX_FAILED_QUICK_ATTEMPTS, MAX_HARDWARE_KEYS,
    NewHardwareKey, QUICK_UNLOCK_FUTURE_SKEW, QuickMethod, QuickUnlockEnrollment,
    QuickUnlockPolicy, REAUTH_GRANT, Reauthenticated, UnlockedSession,
};
use crate::storage::{Error as StorageError, Store};

const DAY_MS: i64 = 86_400_000;

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(Store::open_in_memory().unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

fn file_account(path: &Path) -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(Store::open(path).unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

/// A fresh proof for `s` without the Argon2 round trip.
fn ra(s: &UnlockedSession) -> Reauthenticated {
    Reauthenticated::for_test(s.proof_id)
}

fn prf(fill: u8) -> PrfOutput {
    PrfOutput::from_secret(SecretBytes::from_array(&mut [fill; 32]))
}

fn hw_key(fill: u8) -> NewHardwareKey {
    NewHardwareKey::new(vec![fill; 32], DEFAULT_RP_ID.to_owned(), [fill ^ 0x5A; 32]).unwrap()
}

fn quick_rows(store: &Store) -> usize {
    store
        .unlock_methods()
        .unwrap()
        .iter()
        .filter(|r| matches!(r.method, UnlockMethod::QuickUnlock(_)))
        .count()
}

#[test]
fn verify_master_password_ok_and_wrong() {
    let (mut s, sk) = account();
    let vault = s.create_vault("v").unwrap();
    drop(s.verify_master_password(&pw(), &sk).unwrap());
    let other = MasterPassword::new("not the password".to_owned());
    assert!(matches!(
        s.verify_master_password(&other, &sk),
        Err(Error::UnlockFailed)
    ));
    let other_sk = SecretKey::from_secret(SecretBytes::from_array(&mut [7; 16]));
    assert!(matches!(
        s.verify_master_password(&pw(), &other_sk),
        Err(Error::UnlockFailed)
    ));
    assert!(matches!(
        s.verify_master_password(&MasterPassword::new(String::new()), &sk),
        Err(Error::Core(mosaic_core::Error::EmptyPassword))
    ));
    // Still usable.
    s.add_item(vault, &ItemType::Login, |e| e.set_title("after"))
        .unwrap();
}

#[test]
fn quick_unlock_round_trip() {
    let (mut s, _) = account();
    let vault = s.create_vault("v").unwrap();
    let item = s
        .add_item(vault, &ItemType::Login, |e| e.set_title("kept"))
        .unwrap();
    let w = Kek::generate().unwrap();
    let (enrolled, replaced) = s.enroll_quick_unlock(&w, &[1, 2, 3], &ra(&s)).unwrap();
    assert!(replaced.is_none());
    assert_eq!(enrolled.platform_params, [1, 2, 3]);
    assert_eq!(s.quick_unlock_enrollment().unwrap(), Some(enrolled.clone()));
    let first_writer = s.device_id();
    let locked = s.lock();
    assert_eq!(
        locked.quick_unlock_enrollment().unwrap(),
        Some(enrolled.clone())
    );
    assert!(!locked.password_required(QuickMethod::Biometric));
    let s = locked
        .unlock_quick(
            &w,
            &QuickUnlockPolicy::default(),
            enrolled.enrolled_at_ms + 1000,
        )
        .unwrap();
    assert_eq!(s.item(vault, item).unwrap().title(), Some("kept"));
    assert_ne!(s.device_id(), first_writer);
}

#[test]
fn quick_unlock_needs_password_after_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = file_account(&path);
    let w = Kek::generate().unwrap();
    let (enrolled, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    drop(s.lock().into_store());

    let locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
    assert!(!locked.password_required(QuickMethod::HardwareKey));
    let err = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), enrolled.enrolled_at_ms)
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    // The enrollment is kept; the refusal is not a failed attempt.
    assert_eq!(
        err.session.quick_unlock_enrollment().unwrap(),
        Some(enrolled.clone())
    );
    let s = err.session.unlock(&pw(), &sk).unwrap();
    let locked = s.lock();
    assert!(!locked.password_required(QuickMethod::Biometric));
    locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), enrolled.enrolled_at_ms)
        .unwrap();
}

#[test]
fn quick_unlock_wrong_w_deletes_enrollment() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let at = s.quick_unlock_enrollment().unwrap().unwrap().enrolled_at_ms;
    let other = Kek::generate().unwrap();
    let err = s
        .lock()
        .unlock_quick(&other, &QuickUnlockPolicy::default(), at)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockInvalidated));
    assert_eq!(err.session.quick_unlock_enrollment().unwrap(), None);
    assert_eq!(quick_rows(err.session.store()), 0);
    // The right W cannot bring it back.
    let err = err
        .session
        .unlock_quick(&w, &QuickUnlockPolicy::default(), at)
        .unwrap_err();
    assert!(matches!(err.error, Error::NoQuickUnlock));
}

#[test]
fn quick_unlock_expiry() {
    let policy = QuickUnlockPolicy::default();
    let at = 1_700_000_000_000_i64;
    let mut id = *mosaic_core::DeviceId::new_v7().unwrap().as_bytes();
    id[..6].copy_from_slice(&at.to_be_bytes()[2..]);
    let e = QuickUnlockEnrollment {
        enrollment_id: mosaic_core::DeviceId::from_bytes(id).unwrap(),
        enrolled_at_ms: at,
        platform_params: vec![],
    };
    assert_eq!(e.expires_at_ms(&policy), at + 14 * DAY_MS);
    assert!(!e.is_expired(&policy, at + 14 * DAY_MS - 1));
    assert!(!e.is_expired(&policy, at + 14 * DAY_MS));
    assert!(e.is_expired(&policy, at + 14 * DAY_MS + 1));
    // The skew allowance is exactly five minutes.
    let skew = i64::try_from(QUICK_UNLOCK_FUTURE_SKEW.as_millis()).unwrap();
    assert!(!e.is_expired(&policy, at - skew));
    assert!(e.is_expired(&policy, at - skew - 1));
    // The policy clamps to 1..=30 days.
    assert_eq!(
        QuickUnlockPolicy::new(Duration::from_hours(60 * 24)).max_age(),
        QuickUnlockPolicy::MAX_MAX_AGE
    );
    assert_eq!(
        QuickUnlockPolicy::new(Duration::from_secs(60)).max_age(),
        QuickUnlockPolicy::MIN_MAX_AGE
    );
    let long = QuickUnlockPolicy::new(Duration::from_hours(60 * 24));
    assert!(!e.is_expired(&long, at + 30 * DAY_MS));
    assert!(e.is_expired(&long, at + 30 * DAY_MS + 1));
    // A non-v7 ID carries no enrollment time and counts as expired.
    let mut v4 = id;
    v4[6] = 0x40 | (v4[6] & 0x0F);
    let e4 = QuickUnlockEnrollment {
        enrollment_id: mosaic_core::DeviceId::from_bytes(v4).unwrap(),
        ..e
    };
    assert!(e4.is_expired(&policy, at));
}

#[test]
fn quick_unlock_expiry_deletes_the_enrollment() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    let (e, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let policy = QuickUnlockPolicy::default();
    let s = s
        .lock()
        .unlock_quick(&w, &policy, e.expires_at_ms(&policy))
        .unwrap();
    // Enrolled again, then found expired while locked.
    let mut s = s;
    let (e2, replaced) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    assert_eq!(replaced.map(|r| r.enrollment_id), Some(e.enrollment_id));
    let err = s
        .lock()
        .unlock_quick(&w, &policy, e2.expires_at_ms(&policy) + 1)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockExpired));
    assert_eq!(quick_rows(err.session.store()), 0);
    // Expiry is not a failed attempt.
    assert!(!err.session.password_required(QuickMethod::Biometric));

    // An enrollment dated in the future (beyond the skew) is expired too.
    let (mut s, _) = account();
    s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let at = s.quick_unlock_enrollment().unwrap().unwrap().enrolled_at_ms;
    let err = s
        .lock()
        .unlock_quick(&w, &policy, at - 10 * 60 * 1000)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockExpired));
    assert_eq!(quick_rows(err.session.store()), 0);
}

#[test]
fn enrollment_time_is_authenticated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, _) = file_account(&path);
    let w = Kek::generate().unwrap();
    let (e, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let locked = s.lock();

    // Rewrite the row's method_id to a newer UUIDv7 (extends the expiry).
    let mut forged = *e.enrollment_id.as_bytes();
    forged[..6].copy_from_slice(&(e.enrolled_at_ms + 13 * DAY_MS).to_be_bytes()[2..]);
    assert_ne!(forged, *e.enrollment_id.as_bytes());
    let raw = rusqlite::Connection::open(&path).unwrap();
    let n = raw
        .execute(
            "UPDATE unlock_methods SET method_id = ?1 WHERE method = 2",
            [forged.as_slice()],
        )
        .unwrap();
    assert_eq!(n, 1);
    drop(raw);

    let err = locked
        .unlock_quick(
            &w,
            &QuickUnlockPolicy::default(),
            e.enrolled_at_ms + 20 * DAY_MS,
        )
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockInvalidated));
    assert_eq!(quick_rows(err.session.store()), 0);
}

#[test]
fn re_enroll_replaces() {
    let (mut s, _) = account();
    let w1 = Kek::generate().unwrap();
    let w2 = Kek::generate().unwrap();
    let (first, none) = s.enroll_quick_unlock(&w1, &[1], &ra(&s)).unwrap();
    assert!(none.is_none());
    let (second, replaced) = s.enroll_quick_unlock(&w2, &[2], &ra(&s)).unwrap();
    assert_eq!(replaced, Some(first.clone()));
    assert_ne!(first.enrollment_id, second.enrollment_id);
    assert_eq!(quick_rows(s.store()), 1);
    assert_eq!(s.quick_unlock_enrollment().unwrap(), Some(second.clone()));
    // Only the new W opens it.
    let policy = QuickUnlockPolicy::default();
    let err = s
        .lock()
        .unlock_quick(&w1, &policy, second.enrolled_at_ms)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockInvalidated));
    // Removing returns the enrollment.
    let (mut s, _) = account();
    let (e, _) = s.enroll_quick_unlock(&w1, &[], &ra(&s)).unwrap();
    assert_eq!(s.remove_quick_unlock().unwrap(), Some(e));
    assert_eq!(s.remove_quick_unlock().unwrap(), None);
    let mut locked = s.lock();
    assert_eq!(locked.remove_quick_unlock().unwrap(), None);
}

#[test]
fn password_change_removes_quick_unlock() {
    let (mut s, sk) = account();
    let w = Kek::generate().unwrap();
    let (e, _) = s.enroll_quick_unlock(&w, &[9], &ra(&s)).unwrap();
    s.enroll_hardware_key(&hw_key(3), &prf(4), &ra(&s)).unwrap();
    let new_pw = MasterPassword::new("a new password".to_owned());
    let changed = s.change_master_password(&pw(), &new_pw, &sk).unwrap();
    assert_eq!(changed.removed_quick_unlock, Some(e));
    assert_eq!(quick_rows(s.store()), 0);
    assert_eq!(s.quick_unlock_enrollment().unwrap(), None);
    // The hardware-key wrapping (AK unchanged) still opens.
    let s = s
        .lock()
        .unlock_with_hardware_key(&[3; 32], &prf(4))
        .unwrap();
    // A wrong current password changes nothing.
    let mut s = s;
    assert!(matches!(
        s.change_master_password(&pw(), &new_pw, &sk),
        Err(Error::UnlockFailed)
    ));
}

#[test]
fn failed_attempt_limit() {
    let (mut s, sk) = account();
    s.enroll_hardware_key(&hw_key(1), &prf(2), &ra(&s)).unwrap();
    let mut locked = s.lock();
    for _ in 0..MAX_FAILED_QUICK_ATTEMPTS {
        assert!(!locked.password_required(QuickMethod::HardwareKey));
        let err = locked
            .unlock_with_hardware_key(&[1; 32], &prf(99))
            .unwrap_err();
        assert!(matches!(err.error, Error::HardwareKeyRejected));
        locked = err.session;
    }
    assert!(locked.password_required(QuickMethod::HardwareKey));
    assert!(locked.password_required(QuickMethod::Biometric));
    // Even the right PRF is refused now, and the wrapping is kept.
    let err = locked
        .unlock_with_hardware_key(&[1; 32], &prf(2))
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    assert_eq!(err.session.hardware_keys().unwrap().len(), 1);
    // A password unlock resets the counter.
    let s = err.session.unlock(&pw(), &sk).unwrap();
    let locked = s.lock();
    assert!(!locked.password_required(QuickMethod::HardwareKey));
    locked.unlock_with_hardware_key(&[1; 32], &prf(2)).unwrap();
}

#[test]
fn failed_quick_unlocks_count_too() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let at = s.quick_unlock_enrollment().unwrap().unwrap().enrolled_at_ms;
    let policy = QuickUnlockPolicy::default();
    let err = s
        .lock()
        .unlock_quick(&Kek::generate().unwrap(), &policy, at)
        .unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockInvalidated));
    // One failure does not lock the hardware path.
    assert!(!err.session.password_required(QuickMethod::HardwareKey));
}

#[test]
fn hardware_key_round_trip() {
    let (mut s, _) = account();
    let vault = s.create_vault("v").unwrap();
    let key = hw_key(1);
    let enrolled = s.enroll_hardware_key(&key, &prf(2), &ra(&s)).unwrap();
    assert_eq!(enrolled.credential_id, key.credential_id);
    assert_eq!(enrolled.rp_id, DEFAULT_RP_ID);
    assert_eq!(enrolled.prf_salt, key.prf_salt);
    assert_eq!(s.hardware_keys().unwrap(), std::slice::from_ref(&enrolled));
    let locked = s.lock();
    assert_eq!(
        locked.hardware_keys().unwrap(),
        std::slice::from_ref(&enrolled)
    );

    // Unknown credential, then a wrong PRF.
    let err = locked
        .unlock_with_hardware_key(&[8; 32], &prf(2))
        .unwrap_err();
    assert!(matches!(err.error, Error::UnknownHardwareKey));
    let err = err
        .session
        .unlock_with_hardware_key(&[1; 32], &prf(3))
        .unwrap_err();
    assert!(matches!(err.error, Error::HardwareKeyRejected));
    let err = err
        .session
        .unlock_with_hardware_key(&[1; 15], &prf(2))
        .unwrap_err();
    assert!(matches!(err.error, Error::InvalidHardwareKey));
    let mut s = err
        .session
        .unlock_with_hardware_key(&[1; 32], &prf(2))
        .unwrap();
    assert_eq!(s.vaults().len(), 1);
    let _ = vault;

    // Eight keys at most; replacing one does not count.
    for i in 2..=u8::try_from(MAX_HARDWARE_KEYS).unwrap() {
        s.enroll_hardware_key(&hw_key(i), &prf(i), &ra(&s)).unwrap();
    }
    assert_eq!(s.hardware_keys().unwrap().len(), MAX_HARDWARE_KEYS);
    assert!(matches!(
        s.enroll_hardware_key(&hw_key(9), &prf(9), &ra(&s)),
        Err(Error::HardwareKeyLimit)
    ));
    s.enroll_hardware_key(&hw_key(2), &prf(2), &ra(&s)).unwrap();
    assert!(s.remove_hardware_key(&enrolled.method_id).unwrap());
    assert!(!s.remove_hardware_key(&enrolled.method_id).unwrap());
    s.enroll_hardware_key(&hw_key(9), &prf(9), &ra(&s)).unwrap();
}

#[test]
fn corrupt_hardware_params_are_rejected() {
    let (mut s, _) = account();
    let key = hw_key(1);
    let params = encode_hw_params(&key).unwrap();
    let method = UnlockMethod::hardware_key(&key.credential_id);
    let wrapped = s
        .account_key
        .wrap(
            &prf_unlock_key(&prf(2), &key.credential_id).unwrap(),
            &s.account_id,
            &method,
        )
        .unwrap();
    let other = encode_hw_params(&hw_key(5)).unwrap();
    let mut bad_version = params.clone();
    bad_version[0] = 2;
    let mut trailing = params.clone();
    trailing.push(0);
    let truncated = params[..params.len() - 1].to_vec();
    for bad in [bad_version, trailing, truncated, other, Vec::new()] {
        s.store.put_unlock_method(&method, &wrapped, &bad).unwrap();
        assert!(matches!(
            s.hardware_keys(),
            Err(Error::Store(StorageError::Corrupt))
        ));
    }
    s.store
        .put_unlock_method(&method, &wrapped, &params)
        .unwrap();
    assert_eq!(s.hardware_keys().unwrap().len(), 1);
}

#[test]
fn no_plaintext_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, _) = file_account(&path);
    let w = Kek::generate().unwrap();
    s.enroll_quick_unlock(&w, &[1, 2, 3], &ra(&s)).unwrap();
    s.enroll_hardware_key(&hw_key(1), &prf(0xC3), &ra(&s))
        .unwrap();
    let w_bytes = *w.expose_secret();
    let hw_w = prf_unlock_key(&prf(0xC3), &[1; 32]).unwrap();
    let hw_w_bytes = *hw_w.expose_secret();
    let locked = s.lock();
    let mut files = vec![std::fs::read(&path).unwrap()];
    let mut wal = path.clone().into_os_string();
    wal.push("-wal");
    if let Ok(bytes) = std::fs::read(&wal) {
        files.push(bytes);
    }
    let contains = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w == needle);
    for bytes in &files {
        assert!(!contains(bytes, &w_bytes), "quick-unlock W on disk");
        assert!(!contains(bytes, &hw_w_bytes), "hardware-key W on disk");
        assert!(!contains(bytes, &[0xC3; 32]), "PRF output on disk");
    }
    drop(locked);
}

#[test]
fn removed_and_expired_wrappings_are_scrubbed_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = file_account(&path);
    let w = Kek::generate().unwrap();
    s.enroll_quick_unlock(&w, &[1], &ra(&s)).unwrap();
    let wrapping = |s: &UnlockedSession| quick_wrapping(s.store());
    let first = wrapping(&s);
    let mut wal = path.clone().into_os_string();
    wal.push("-wal");
    let on_disk = |needle: &[u8]| {
        let mut files = vec![std::fs::read(&path).unwrap()];
        if let Ok(bytes) = std::fs::read(&wal) {
            files.push(bytes);
        }
        files
            .iter()
            .any(|f| f.windows(needle.len()).any(|w| w == needle))
    };
    assert!(on_disk(&first), "the test must see the wrapping on disk");
    // Removed while unlocked.
    assert!(s.remove_quick_unlock().unwrap().is_some());
    assert!(!on_disk(&first), "removed wrapping left on disk");

    // Expired while locked.
    s.enroll_quick_unlock(&w, &[2], &ra(&s)).unwrap();
    let second = wrapping(&s);
    assert!(on_disk(&second));
    let enrolled = s.quick_unlock_enrollment().unwrap().unwrap();
    let policy = QuickUnlockPolicy::default();
    let late = enrolled.expires_at_ms(&policy) + 1;
    let locked = s.lock();
    let err = locked.unlock_quick(&w, &policy, late).unwrap_err();
    assert!(matches!(err.error, Error::QuickUnlockExpired));
    assert!(!on_disk(&second), "expired wrapping left on disk");
    drop(err.session.unlock(&pw(), &sk).unwrap());
}

fn quick_wrapping(store: &Store) -> Vec<u8> {
    store
        .unlock_methods()
        .unwrap()
        .into_iter()
        .find(|r| matches!(r.method, UnlockMethod::QuickUnlock(_)))
        .unwrap()
        .wrapped_account_key
        .as_bytes()
        .to_vec()
}

// ---------------------------------------------------------------------------
// Crypto-review fixes (M1, M2, L1-L3, L7, I3)
// ---------------------------------------------------------------------------

#[test]
fn m1_policy_never_exceeds_thirty_days() {
    // The field is private; every constructor clamps.
    for age in [Duration::ZERO, Duration::from_hours(48), Duration::MAX] {
        let p = QuickUnlockPolicy::new(age);
        assert!(p.max_age() >= QuickUnlockPolicy::MIN_MAX_AGE);
        assert!(p.max_age() <= QuickUnlockPolicy::MAX_MAX_AGE);
        // An enrollment 31 days old is expired under every policy.
        let (mut s, _) = account();
        let w = Kek::generate().unwrap();
        let (e, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
        assert!(e.is_expired(&p, e.enrolled_at_ms + 31 * DAY_MS));
        let max_ms = i64::try_from(p.max_age().as_millis()).unwrap();
        assert!(!e.is_expired(&p, e.enrolled_at_ms + max_ms));
    }
}

#[test]
fn i3_expiry_ignores_the_enrolled_at_field() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    let (mut e, _) = s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    let real = e.enrolled_at_ms;
    e.enrolled_at_ms = real + 100 * DAY_MS;
    let policy = QuickUnlockPolicy::default();
    assert_eq!(e.expires_at_ms(&policy), real + 14 * DAY_MS);
    assert!(e.is_expired(&policy, real + 15 * DAY_MS));
    e.enrolled_at_ms = real - 100 * DAY_MS;
    assert!(!e.is_expired(&policy, real + DAY_MS));
}

#[test]
fn m2_enrollment_needs_a_fresh_proof_of_this_session() {
    let (mut s, sk) = account();
    let w = Kek::generate().unwrap();
    // A real proof works, also just inside the grant.
    let proof = s.verify_master_password(&pw(), &sk).unwrap();
    s.enroll_quick_unlock(&w, &[], &proof).unwrap();
    let almost = ra(&s).aged(REAUTH_GRANT.checked_sub(Duration::from_secs(5)).unwrap());
    s.enroll_hardware_key(&hw_key(1), &prf(1), &almost).unwrap();
    // Expired.
    let old = ra(&s).aged(REAUTH_GRANT + Duration::from_secs(5));
    assert!(matches!(
        s.enroll_quick_unlock(&w, &[], &old),
        Err(Error::ReauthenticationRequired)
    ));
    assert!(matches!(
        s.enroll_hardware_key(&hw_key(2), &prf(2), &old),
        Err(Error::ReauthenticationRequired)
    ));
    // From another session of the same account.
    let (other, other_sk) = account();
    let foreign = other.verify_master_password(&pw(), &other_sk).unwrap();
    assert!(matches!(
        s.enroll_quick_unlock(&w, &[], &foreign),
        Err(Error::ReauthenticationRequired)
    ));
    assert!(matches!(
        s.enroll_hardware_key(&hw_key(2), &prf(2), &foreign),
        Err(Error::ReauthenticationRequired)
    ));
    // After lock and unlock the old session's proof is foreign too.
    let stale = s.verify_master_password(&pw(), &sk).unwrap();
    let mut s = s.lock().unlock(&pw(), &sk).unwrap();
    assert!(matches!(
        s.enroll_quick_unlock(&w, &[], &stale),
        Err(Error::ReauthenticationRequired)
    ));
    // Nothing was written by the refusals.
    assert_eq!(s.hardware_keys().unwrap().len(), 1);
}

#[test]
fn l1_noted_hardware_failures_count_toward_the_limit() {
    let (mut s, sk) = account();
    s.enroll_hardware_key(&hw_key(1), &prf(2), &ra(&s)).unwrap();
    let mut locked = s.lock();
    for _ in 0..MAX_FAILED_QUICK_ATTEMPTS - 1 {
        locked.note_failed_hardware_attempt();
        assert!(!locked.password_required(QuickMethod::HardwareKey));
    }
    locked.note_failed_hardware_attempt();
    assert!(locked.password_required(QuickMethod::HardwareKey));
    assert!(locked.password_required(QuickMethod::Biometric));
    let err = locked
        .unlock_with_hardware_key(&[1; 32], &prf(2))
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    // The password resets it.
    drop(err.session.unlock(&pw(), &sk).unwrap());
}

#[test]
fn l2_failed_quick_enrollment_keeps_the_previous_one() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    let (first, _) = s.enroll_quick_unlock(&w, &[1], &ra(&s)).unwrap();
    let big = vec![0u8; crate::storage::MAX_UNLOCK_PARAMS_LEN + 1];
    assert!(s.enroll_quick_unlock(&w, &big, &ra(&s)).is_err());
    assert_eq!(s.quick_unlock_enrollment().unwrap(), Some(first.clone()));
    assert_eq!(quick_rows(s.store()), 1);
    // A good one replaces it in one step and reports it.
    let (second, replaced) = s.enroll_quick_unlock(&w, &[2], &ra(&s)).unwrap();
    assert_eq!(replaced, Some(first));
    assert_eq!(quick_rows(s.store()), 1);
    assert_eq!(s.quick_unlock_enrollment().unwrap(), Some(second));
}

#[test]
fn l3_prf_salt_is_never_reused_across_wrappings() {
    let (mut s, _) = account();
    let same_salt = |fill: u8, salt: [u8; 32]| {
        NewHardwareKey::new(vec![fill; 32], DEFAULT_RP_ID.to_owned(), salt).unwrap()
    };
    // Unlock first, then a vault wrapping with the same salt: refused.
    s.enroll_hardware_key(&same_salt(1, [7; 32]), &prf(1), &ra(&s))
        .unwrap();
    assert!(matches!(
        s.create_hardware_vault("v", &same_salt(1, [7; 32]), &prf(2)),
        Err(Error::InvalidHardwareKey)
    ));
    // A salt of its own works, and so does another credential's salt.
    let v = s
        .create_hardware_vault("v", &same_salt(1, [8; 32]), &prf(2))
        .unwrap();
    assert!(
        s.create_hardware_vault("w", &same_salt(2, [7; 32]), &prf(3))
            .is_ok()
    );
    // Vault first, then unlock with the vault's salt: refused.
    assert!(matches!(
        s.enroll_hardware_key(&same_salt(1, [8; 32]), &prf(4), &ra(&s)),
        Err(Error::InvalidHardwareKey)
    ));
    // Adding a key to a vault: reusing the unlock salt, or another vault's.
    assert!(matches!(
        s.add_hardware_vault_key(v, &same_salt(1, [7; 32]), &prf(5)),
        Err(Error::InvalidHardwareKey)
    ));
    // Re-registering the same credential with the salt it replaces is fine.
    s.enroll_hardware_key(&same_salt(1, [7; 32]), &prf(1), &ra(&s))
        .unwrap();
    s.add_hardware_vault_key(v, &same_salt(1, [8; 32]), &prf(2))
        .unwrap();
}

#[test]
fn l7_downgrade_invariant_hardware_unlock_does_not_enable_biometrics() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    let (mut s, sk) = file_account(&path);
    let w = Kek::generate().unwrap();
    s.enroll_quick_unlock(&w, &[], &ra(&s)).unwrap();
    s.enroll_hardware_key(&hw_key(1), &prf(2), &ra(&s)).unwrap();
    // Open the store afresh: no password unlock in this app run.
    drop(s.lock().into_store());
    let locked = LockedSession::open(Store::open(&path).unwrap());
    assert!(locked.password_required(QuickMethod::Biometric));
    let s = locked.unlock_with_hardware_key(&[1; 32], &prf(2)).unwrap();
    // Locking again keeps "password still required" for biometrics.
    let locked = s.lock();
    assert!(locked.password_required(QuickMethod::Biometric));
    let err = locked
        .unlock_quick(&w, &QuickUnlockPolicy::default(), i64::MAX / 2)
        .unwrap_err();
    assert!(matches!(err.error, Error::PasswordRequired));
    // The enrollment survived the refusal (it is not deleted).
    assert!(err.session.quick_unlock_enrollment().unwrap().is_some());
    // The password unlock enables it.
    let s = err.session.unlock(&pw(), &sk).unwrap();
    assert!(!s.lock().password_required(QuickMethod::Biometric));
}

#[test]
fn invalid_hardware_key_inputs() {
    let ok = |cred: usize, rp: &str| NewHardwareKey::new(vec![1; cred], rp.to_owned(), [0; 32]);
    assert!(ok(16, "a").is_ok());
    assert!(ok(1023, &"a".repeat(253)).is_ok());
    for (cred, rp) in [
        (15, "mosaic.invalid"),
        (1024, "mosaic.invalid"),
        (32, ""),
        (32, &"a".repeat(254)),
        (32, "has space"),
        (32, "bücher.example"),
        (32, "tab\t"),
    ] {
        assert!(
            matches!(ok(cred, rp), Err(Error::InvalidHardwareKey)),
            "{cred} {rp}"
        );
    }
    // A key edited after construction is checked again at enrollment.
    let (mut s, _) = account();
    let mut key = hw_key(1);
    key.credential_id.truncate(4);
    assert!(matches!(
        s.enroll_hardware_key(&key, &prf(1), &ra(&s)),
        Err(Error::InvalidHardwareKey)
    ));
}

#[test]
fn oversized_platform_params_are_refused() {
    let (mut s, _) = account();
    let w = Kek::generate().unwrap();
    let big = vec![0u8; crate::storage::MAX_UNLOCK_PARAMS_LEN + 1];
    assert!(matches!(
        s.enroll_quick_unlock(&w, &big, &ra(&s)),
        Err(Error::Store(StorageError::UnlockParamsTooLarge))
    ));
    assert_eq!(s.quick_unlock_enrollment().unwrap(), None);
}

#[test]
fn unlock_without_account_or_enrollment() {
    let locked = LockedSession::open(Store::open_in_memory().unwrap());
    let err = locked
        .unlock_with_hardware_key(&[1; 32], &prf(1))
        .unwrap_err();
    assert!(matches!(err.error, Error::NoAccount));
    let err = err
        .session
        .unlock_quick(&Kek::generate().unwrap(), &QuickUnlockPolicy::default(), 0)
        .unwrap_err();
    assert!(matches!(err.error, Error::NoAccount));
    // An account without quick unlock.
    let (s, _) = account();
    let err = s
        .lock()
        .unlock_quick(&Kek::generate().unwrap(), &QuickUnlockPolicy::default(), 0)
        .unwrap_err();
    assert!(matches!(err.error, Error::NoQuickUnlock));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn params_codec_round_trips(
        credential_id in proptest::collection::vec(any::<u8>(), 16..=1023),
        rp_id in "[!-~]{1,253}",
        salt in any::<[u8; 32]>(),
    ) {
        let key = NewHardwareKey::new(credential_id.clone(), rp_id, salt).unwrap();
        let bytes = encode_hw_params(&key).unwrap();
        let UnlockMethod::HardwareKey(method_id) = UnlockMethod::hardware_key(&credential_id) else {
            unreachable!()
        };
        prop_assert_eq!(decode_hw_params(&method_id, &bytes).unwrap(), key);
        // Any single appended or removed byte is refused.
        let mut longer = bytes.clone();
        longer.push(0);
        prop_assert!(decode_hw_params(&method_id, &longer).is_err());
        prop_assert!(decode_hw_params(&method_id, &bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn params_decoder_never_panics(
        method_id in any::<[u8; 16]>(),
        bytes in proptest::collection::vec(any::<u8>(), 0..4096),
    ) {
        let _ = decode_hw_params(&method_id, &bytes);
    }
}
