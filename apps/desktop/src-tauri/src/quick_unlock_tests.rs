//! Tests of the quick-unlock flows (CRYPTO_SPEC §6.2; ADR-0028) over fake
//! platforms: no OS prompt, no device. Each test that creates an account or
//! checks a password runs Argon2id, so they hold the KDF permit.
#![allow(clippy::needless_pass_by_value)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use mosaic_core::{AccountId, Argon2Profile, MasterPassword, SecretKey};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::quick_unlock::{
    BiometricPlatformKind, EnrollBiometricRequest, QuickUnlockStatus,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{
    CreateAccountRequest, QuickUnlockKind, QuickUnlockRequest, SaveKitRequest, SessionState,
    UnlockRequest,
};
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::{MemoryKeystore, SecretKeyStore};
use crate::persist::Persistence;
use crate::quick_unlock::fakes::FakeBiometric;
use crate::quick_unlock::{NoHardwareKeys, PlatformError, QuickUnlockService, system_now_ms};
use crate::state::{Backend, Slot};

const PASSWORD: &str = "correct horse battery staple";
const DAY_MS: i64 = 86_400_000;

fn code(error: IpcError) -> ErrorCode {
    error.code
}

/// A keystore the test keeps a handle to (survives a "restart").
#[derive(Clone, Default)]
struct SharedKeystore(Arc<MemoryKeystore>);

impl SecretKeyStore for SharedKeystore {
    fn load(&self, account: AccountId) -> IpcResult<Option<SecretKey>> {
        self.0.load(account)
    }
    fn save(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        self.0.save(account, key)
    }
    fn delete(&self, account: AccountId) -> IpcResult<()> {
        self.0.delete(account)
    }
}

struct Rig {
    backend: Backend,
    fake: Arc<FakeBiometric>,
    /// Added to the wall clock the service sees (fake clock).
    skew: Arc<AtomicI64>,
    events: Log,
}

impl Rig {
    fn advance(&self, ms: i64) {
        self.skew.fetch_add(ms, Ordering::SeqCst);
    }
}

fn service(fake: &Arc<FakeBiometric>, skew: &Arc<AtomicI64>) -> QuickUnlockService {
    let skew = Arc::clone(skew);
    QuickUnlockService::new(Some(Box::new(Arc::clone(fake))), Box::new(NoHardwareKeys)).with_clock(
        Box::new(move || system_now_ms() + skew.load(Ordering::SeqCst)),
    )
}

fn backend_over(
    store: Store,
    keystore: SharedKeystore,
    persistence: Persistence,
    fake: &Arc<FakeBiometric>,
    skew: &Arc<AtomicI64>,
) -> (Backend, Log) {
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(store)),
        Box::new(keystore),
        || Ok(Argon2Profile::Baseline),
    )
    .with_persistence(persistence)
    .with_quick_unlock(service(fake, skew));
    let (channel, log) = recording_channel();
    backend.subscribe(channel).unwrap();
    (backend, log)
}

fn create(b: &Backend) {
    b.create_account(CreateAccountRequest {
        email: "alex@example.com".into(),
        password: SecretIn::test(PASSWORD),
        default_vault_name: "Personal".into(),
    })
    .unwrap();
}

/// A fresh in-memory account, unlocked, with a biometric platform.
fn rig(available: bool) -> Rig {
    let fake = FakeBiometric::new(available);
    let skew = Arc::new(AtomicI64::new(0));
    let (backend, events) = backend_over(
        Store::open_in_memory().unwrap(),
        SharedKeystore::default(),
        Persistence::memory(),
        &fake,
        &skew,
    );
    create(&backend);
    Rig {
        backend,
        fake,
        skew,
        events,
    }
}

fn enroll(b: &Backend, password: Option<&str>) -> IpcResult<QuickUnlockStatus> {
    b.enroll_biometric(EnrollBiometricRequest {
        password: password.map(SecretIn::test),
    })
}

fn quick(method: QuickUnlockKind) -> QuickUnlockRequest {
    QuickUnlockRequest { method, pin: None }
}

fn methods(b: &Backend) -> Vec<QuickUnlockKind> {
    match b.session_state().unwrap() {
        SessionState::Locked { quick_unlock, .. } => quick_unlock,
        other => panic!("not locked: {other:?}"),
    }
}

fn unlock_with_password(b: &Backend) {
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
}

fn events(log: &Log) -> Vec<Value> {
    log.lock()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect()
}

fn locked_event() -> Value {
    json!({ "type": "locked", "reason": "manual" })
}

#[test]
fn biometric_unlock_happy_path_prompts_once() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    let status = enroll(b, Some(PASSWORD)).unwrap();
    assert_eq!(
        status.biometric.platform,
        Some(BiometricPlatformKind::WindowsHello)
    );
    assert!(status.biometric.available && status.biometric.enrolled);
    let (at, until) = (
        status.biometric.enrolled_at.unwrap(),
        status.biometric.expires_at.unwrap(),
    );
    assert_eq!(until - at, 14 * DAY_MS, "the default maximum age");
    assert_eq!(status.max_age_days, 14);
    assert!(!status.hardware_keys.supported);
    assert_eq!(r.fake.prompts(), 1, "enrolling prompts once");

    b.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
    assert_eq!(r.fake.prompts(), 1, "listing the methods never prompts");

    let state = b.unlock_quick(&quick(QuickUnlockKind::Biometric)).unwrap();
    assert!(matches!(state, SessionState::Unlocked { .. }));
    assert_eq!(r.fake.prompts(), 2, "exactly one prompt per unlock");
    assert!(b.is_unlocked());
    assert!(r.fake.has_key(), "reconcile keeps a good enrollment");
    assert!(b.quick_unlock_status().unwrap().biometric.enrolled);
}

#[test]
fn no_prompt_when_unavailable_or_not_enrolled() {
    let _kdf = crate::test_support::kdf_permit();
    // Platform present but unavailable.
    let off = rig(false);
    assert_eq!(
        code(enroll(&off.backend, Some(PASSWORD)).unwrap_err()),
        ErrorCode::QuickUnlockUnavailable
    );
    let status = off.backend.quick_unlock_status().unwrap();
    assert!(!status.biometric.available && !status.biometric.enrolled);
    assert_eq!(
        status.biometric.platform,
        Some(BiometricPlatformKind::WindowsHello)
    );
    off.backend.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(&off.backend), vec![]);
    assert_eq!(
        code(
            off.backend
                .unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(off.fake.prompts(), 0);

    // Available, but never enrolled.
    let on = rig(true);
    on.backend.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(&on.backend), vec![]);
    assert_eq!(
        code(
            on.backend
                .unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(on.fake.prompts(), 0);

    // A PIN belongs to security keys only.
    enroll_then_lock(&on);
    let with_pin = QuickUnlockRequest {
        method: QuickUnlockKind::Biometric,
        pin: Some(SecretIn::test("1234")),
    };
    assert_eq!(
        code(on.backend.unlock_quick(&with_pin).unwrap_err()),
        ErrorCode::InvalidInput
    );
    assert_eq!(on.fake.prompts(), 1, "only the enrollment prompted");

    // No platform at all (Linux).
    let linux = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    create(&linux);
    let status = linux.quick_unlock_status().unwrap();
    assert_eq!(status.biometric.platform, None);
    assert!(!status.biometric.available && !status.biometric.enrolled);
    assert_eq!(
        code(enroll(&linux, Some(PASSWORD)).unwrap_err()),
        ErrorCode::QuickUnlockUnavailable
    );
}

fn enroll_then_lock(r: &Rig) {
    // Unlocked, within the grant of an earlier right password or not: use
    // the password so the helper works from any state.
    if !r.backend.is_unlocked() {
        unlock_with_password(&r.backend);
    }
    enroll(&r.backend, Some(PASSWORD)).unwrap();
    r.backend.lock(LockReason::Manual).unwrap();
}

#[test]
fn a_restart_requires_the_password_once() {
    let _kdf = crate::test_support::kdf_permit();
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("vault.sqlite3");
    let keystore = SharedKeystore::default();
    let fake = FakeBiometric::new(true);
    let skew = Arc::new(AtomicI64::new(0));
    let (first, _log) = backend_over(
        Store::open(&store_path).unwrap(),
        keystore.clone(),
        Persistence::in_dir(dir.path()),
        &fake,
        &skew,
    );
    create(&first);
    enroll(&first, Some(PASSWORD)).unwrap();
    first.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(&first), vec![QuickUnlockKind::Biometric]);
    drop(first);

    // The app starts again: the platform key and the wrapping survive, but
    // the password is needed once.
    let (second, _log) = backend_over(
        Store::open(&store_path).unwrap(),
        keystore,
        Persistence::in_dir(dir.path()),
        &fake,
        &skew,
    );
    second.reconcile_quick_unlock();
    assert!(fake.has_key(), "reconcile keeps a good enrollment");
    assert_eq!(methods(&second), vec![]);
    assert_eq!(
        code(
            second
                .unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::PasswordRequired
    );
    assert_eq!(fake.prompts(), 1, "refused before any prompt");

    unlock_with_password(&second);
    assert!(second.quick_unlock_status().unwrap().biometric.enrolled);
    second.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(&second), vec![QuickUnlockKind::Biometric]);
    second
        .unlock_quick(&quick(QuickUnlockKind::Biometric))
        .unwrap();
    assert!(second.is_unlocked());
}

#[test]
fn an_expired_enrollment_is_removed_with_its_platform_key() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    // Lowering the maximum age needs no password.
    b.update_settings(UpdateSettingsRequest {
        settings: Settings {
            quick_unlock_max_age_days: 1,
            ..Settings::default()
        },
        password: None,
    })
    .unwrap();
    enroll(b, Some(PASSWORD)).unwrap();
    b.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);

    r.advance(DAY_MS + 60_000);
    assert_eq!(methods(b), vec![], "an expired enrollment is not offered");
    assert!(r.fake.has_key(), "listing never writes");
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockExpired
    );
    assert_eq!(r.fake.removals(), 1);
    assert!(!r.fake.has_key());
    assert_eq!(r.fake.prompts(), 1, "no prompt for an expired enrollment");
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable,
        "the wrapping is gone"
    );
}

#[test]
fn reconcile_removes_what_expired_or_lost_its_key_or_has_no_enrollment() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll(b, Some(PASSWORD)).unwrap();

    // Enrollment without a platform key.
    r.fake.lose_key();
    b.reconcile_quick_unlock();
    assert!(!b.quick_unlock_status().unwrap().biometric.enrolled);

    // A platform key without an enrollment.
    r.fake.plant_key();
    assert!(r.fake.has_key());
    b.reconcile_quick_unlock();
    assert!(!r.fake.has_key());

    // Expired: a shorter maximum age takes effect at once.
    enroll(b, None).unwrap();
    b.update_settings(UpdateSettingsRequest {
        settings: Settings {
            quick_unlock_max_age_days: 1,
            ..Settings::default()
        },
        password: None,
    })
    .unwrap();
    assert!(b.quick_unlock_status().unwrap().biometric.enrolled);
    r.advance(2 * DAY_MS);
    b.update_settings(UpdateSettingsRequest {
        settings: Settings {
            quick_unlock_max_age_days: 1,
            theme: crate::contract::settings::Theme::Dark,
            ..Settings::default()
        },
        password: None,
    })
    .unwrap();
    assert!(!b.quick_unlock_status().unwrap().biometric.enrolled);
    assert!(!r.fake.has_key());
}

#[test]
fn a_cancelled_prompt_keeps_the_enrollment() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll_then_lock(&r);
    r.fake.fail_next(PlatformError::Cancelled);
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::Cancelled
    );
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
    assert!(r.fake.has_key());
    r.fake.fail_next(PlatformError::Failed);
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::Internal
    );
    b.unlock_quick(&quick(QuickUnlockKind::Biometric)).unwrap();
    assert!(b.is_unlocked());
}

#[test]
fn a_changed_platform_key_invalidates_and_deletes_the_enrollment() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll_then_lock(&r);
    r.fake.rotate_key();
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockInvalidated
    );
    assert!(!r.fake.has_key(), "the platform key is deleted");
    assert_eq!(r.fake.removals(), 1);
    assert_eq!(methods(b), vec![]);
    unlock_with_password(b);
    assert!(
        !b.quick_unlock_status().unwrap().biometric.enrolled,
        "the wrapping is gone"
    );
}

#[test]
fn a_key_that_vanished_is_unavailable_and_the_enrollment_goes() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll_then_lock(&r);
    r.fake.lose_key();
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(methods(b), vec![]);
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(r.fake.prompts(), 2, "enrolling and one release");
}

#[test]
fn enrolling_needs_the_password_and_a_right_one_opens_a_two_minute_grant() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    assert_eq!(
        code(enroll(b, None).unwrap_err()),
        ErrorCode::ReauthRequired,
        "no grant yet"
    );
    assert_eq!(
        code(enroll(b, Some("")).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert_eq!(
        code(enroll(b, Some("not the password")).unwrap_err()),
        ErrorCode::WrongCredentials
    );
    assert!(b.is_unlocked(), "a wrong password no longer locks");
    assert!(!events(&r.events).contains(&locked_event()));
    assert_eq!(r.fake.prompts(), 0);

    enroll(b, Some(PASSWORD)).unwrap();
    // Within the grant a null password passes (a re-enrollment replaces).
    let again = enroll(b, None).unwrap();
    assert!(again.biometric.enrolled);
    assert_eq!(r.fake.prompts(), 2);

    // A lock drops the grant.
    b.lock(LockReason::Manual).unwrap();
    unlock_with_password(b);
    assert_eq!(
        code(enroll(b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
}

#[test]
fn five_wrong_reauthentications_in_a_row_lock_and_a_right_one_resets_the_count() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    // Past the onboarding window in which the kit saves without a password.
    *b.kit_pending.lock().unwrap() = None;
    let wrong = |b: &Backend| {
        b.save_emergency_kit(SaveKitRequest {
            labels: crate::emergency_kit::tests::labels(),
            password: Some(SecretIn::test("not the password")),
        })
        .unwrap_err()
    };
    for _ in 0..4 {
        assert_eq!(code(wrong(b)), ErrorCode::WrongCredentials);
        assert!(b.is_unlocked());
    }
    assert!(!events(&r.events).contains(&locked_event()));
    // A right password resets the count.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    for _ in 0..4 {
        assert_eq!(code(wrong(b)), ErrorCode::WrongCredentials);
        assert!(b.is_unlocked());
    }
    assert_eq!(code(wrong(b)), ErrorCode::WrongCredentials);
    assert!(!b.is_unlocked(), "the fifth wrong one in a row locks");
    assert!(events(&r.events).contains(&locked_event()));
    // The count starts again after the lock.
    unlock_with_password(b);
    assert_eq!(code(wrong(b)), ErrorCode::WrongCredentials);
    assert!(b.is_unlocked());
}

#[test]
fn raising_the_maximum_age_needs_the_password_and_lowering_does_not() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    let with_age = |days: u32, password: Option<&str>| UpdateSettingsRequest {
        settings: Settings {
            quick_unlock_max_age_days: days,
            ..Settings::default()
        },
        password: password.map(SecretIn::test),
    };
    assert_eq!(
        code(b.update_settings(with_age(20, None)).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert_eq!(b.settings_snapshot().quick_unlock_max_age_days, 14);
    let stored = b.update_settings(with_age(20, Some(PASSWORD))).unwrap();
    assert_eq!(stored.quick_unlock_max_age_days, 20);
    assert_eq!(b.quick_unlock_status().unwrap().max_age_days, 20);
    // Clamped to the vault's 30 day cap.
    let stored = b.update_settings(with_age(90, Some(PASSWORD))).unwrap();
    assert_eq!(stored.quick_unlock_max_age_days, 30);
    // Lowering is free.
    let stored = b.update_settings(with_age(3, None)).unwrap();
    assert_eq!(stored.quick_unlock_max_age_days, 3);
}

#[test]
fn removing_and_a_password_change_delete_the_platform_key() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll(b, Some(PASSWORD)).unwrap();
    let status = b.remove_biometric().unwrap();
    assert!(!status.biometric.enrolled);
    assert!(!r.fake.has_key());
    assert_eq!(r.fake.removals(), 1);

    // The vault deletes the enrollment on a master password change and
    // reports it; the app deletes the platform key.
    enroll(b, None).unwrap();
    assert!(r.fake.has_key());
    let account = b.account().unwrap().account_id.get();
    let secret_key = b.secret_keys.load(account).unwrap().unwrap();
    let changed = b
        .slot()
        .unwrap()
        .unlocked_mut()
        .unwrap()
        .change_master_password(
            &MasterPassword::new(PASSWORD.to_owned()),
            &MasterPassword::new("a brand new master password".to_owned()),
            &secret_key,
        )
        .unwrap();
    assert!(changed.removed_quick_unlock.is_some());
    assert!(!b.quick_unlock_status().unwrap().biometric.enrolled);
    b.platform_key_after_removal(account, changed.removed_quick_unlock.as_ref());
    assert!(!r.fake.has_key());
}

#[test]
fn a_session_that_changes_during_a_prompt_ends_the_flow_cleanly() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let fake = Arc::clone(&r.fake);
    let b = Arc::new(r.backend);
    let cell: Arc<OnceLock<Weak<Backend>>> = Arc::default();
    cell.set(Arc::downgrade(&b)).ok();

    // The enrollment prompt is open while the vault locks.
    let weak = Arc::clone(&cell);
    *fake.on_enroll.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            b.lock(LockReason::Manual).unwrap();
        }
    }));
    assert_eq!(
        code(enroll(&b, Some(PASSWORD)).unwrap_err()),
        ErrorCode::Locked
    );
    assert!(
        !fake.has_key(),
        "the platform key made meanwhile is deleted"
    );
    unlock_with_password(&b);
    assert!(
        !b.quick_unlock_status().unwrap().biometric.enrolled,
        "nothing was stored"
    );

    // The unlock prompt is open while the password unlocks the vault.
    enroll(&b, Some(PASSWORD)).unwrap();
    b.lock(LockReason::Manual).unwrap();
    let weak = Arc::clone(&cell);
    *fake.on_release.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            unlock_with_password(&b);
        }
    }));
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::AlreadyUnlocked
    );
    assert!(b.is_unlocked(), "the slot stays unlocked");
}

#[test]
fn hardware_keys_are_unsupported_by_default_and_answer_without_a_device() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll_then_lock(&r);
    assert_eq!(
        code(
            b.unlock_quick(&quick(QuickUnlockKind::HardwareKey))
                .unwrap_err()
        ),
        ErrorCode::HardwareKeyUnsupported
    );
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
}
