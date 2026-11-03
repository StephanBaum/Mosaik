//! Tests of the quick-unlock flows (CRYPTO_SPEC §6.2; ADR-0028) over fake
//! platforms: no OS prompt, no device. Each test that creates an account or
//! checks a password runs Argon2id, so they hold the KDF permit.
#![allow(clippy::needless_pass_by_value)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::{AccountId, Argon2Profile, MasterPassword, SecretKey};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::quick_unlock::{
    BiometricPlatformKind, EnrollBiometricRequest, EnrollHardwareKeyRequest, QuickUnlockStatus,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{
    CreateAccountRequest, QuickUnlockKind, QuickUnlockRequest, SaveKitRequest, SessionState,
    UnlockRequest,
};
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::contract::vaults::VaultProtection;
use crate::events::testing::{Log, recording_channel};
use crate::keystore::{MemoryKeystore, SecretKeyStore};
use crate::persist::Persistence;
use crate::quick_unlock::fakes::{FAKE_PIN, FakeBiometric, FakeHardwareKey};
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

    let state = b
        .unlock_quick(&mut quick(QuickUnlockKind::Biometric))
        .unwrap();
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
                .unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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
                .unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(on.fake.prompts(), 0);

    // A PIN belongs to security keys only.
    enroll_then_lock(&on);
    let mut with_pin = QuickUnlockRequest {
        method: QuickUnlockKind::Biometric,
        pin: Some(SecretIn::test("1234")),
    };
    assert_eq!(
        code(on.backend.unlock_quick(&mut with_pin).unwrap_err()),
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
                .unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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
        .unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockExpired
    );
    assert_eq!(r.fake.removals(), 1);
    assert!(!r.fake.has_key());
    assert_eq!(r.fake.prompts(), 1, "no prompt for an expired enrollment");
    assert_eq!(
        code(
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable,
        "the wrapping is gone"
    );
}

#[test]
fn reconcile_removes_what_expired_or_lost_its_key() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll(b, Some(PASSWORD)).unwrap();

    // Enrollment without a platform key.
    r.fake.lose_key();
    b.reconcile_quick_unlock();
    assert!(!b.quick_unlock_status().unwrap().biometric.enrolled);

    // Expired: a shorter maximum age takes effect at once.
    enroll(b, Some(PASSWORD)).unwrap();
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
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::Cancelled
    );
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
    assert!(r.fake.has_key());
    r.fake.fail_next(PlatformError::Failed);
    assert_eq!(
        code(
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::Internal
    );
    b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
        .unwrap();
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
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::QuickUnlockUnavailable
    );
    assert_eq!(methods(b), vec![]);
    assert_eq!(
        code(
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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

    // A right password opens the grant for one enrollment.
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let first = enroll(b, None).unwrap();
    assert!(first.biometric.enrolled);
    assert_eq!(r.fake.prompts(), 1);
    // The success used the grant up.
    assert_eq!(
        code(enroll(b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    assert_eq!(r.fake.prompts(), 1, "no prompt without a grant");
    // A re-enrollment replaces the enrollment and deletes the old key.
    let again = enroll(b, Some(PASSWORD)).unwrap();
    assert!(again.biometric.enrolled);
    assert_eq!(r.fake.prompts(), 2);
    assert_eq!(r.fake.key_count(), 1, "the replaced key is gone");

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
    enroll(b, Some(PASSWORD)).unwrap();
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
    b.platform_key_after_removal(changed.removed_quick_unlock.as_ref());
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
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
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
            b.unlock_quick(&mut quick(QuickUnlockKind::HardwareKey))
                .unwrap_err()
        ),
        ErrorCode::HardwareKeyUnsupported
    );
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
}

// ---------------------------------------------------------------------------
// Security keys and hardware-protected vaults
// ---------------------------------------------------------------------------

struct HwRig {
    backend: Backend,
    key: Arc<FakeHardwareKey>,
    events: Log,
}

/// A fresh unlocked account with `physical` security keys (key 0 plugged in)
/// and no biometric platform.
fn hw_rig(physical: usize) -> HwRig {
    let key = FakeHardwareKey::new(physical);
    let service = QuickUnlockService::new(None, Box::new(Arc::clone(&key)));
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_quick_unlock(service);
    let (channel, events) = recording_channel();
    backend.subscribe(channel).unwrap();
    create(&backend);
    HwRig {
        backend,
        key,
        events,
    }
}

fn enroll_key(
    b: &Backend,
    password: Option<&str>,
    pin: Option<&str>,
) -> IpcResult<QuickUnlockStatus> {
    b.enroll_hardware_key(EnrollHardwareKeyRequest {
        password: password.map(SecretIn::test),
        pin: pin.map(SecretIn::test),
    })
}

fn pin_request(pin: Option<&str>) -> QuickUnlockRequest {
    QuickUnlockRequest {
        method: QuickUnlockKind::HardwareKey,
        pin: pin.map(SecretIn::test),
    }
}

fn typed<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

#[test]
fn unsupported_security_keys_answer_without_touching_a_device() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let b = &r.backend;
    r.key.supported.store(false, Ordering::SeqCst);
    let vault = b.list_vaults().unwrap()[0].vault_id.0.to_string();
    let unsupported = ErrorCode::HardwareKeyUnsupported;
    assert_eq!(
        code(enroll_key(b, Some(PASSWORD), None).unwrap_err()),
        unsupported
    );
    assert_eq!(
        code(
            b.create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
                .unwrap_err()
        ),
        unsupported
    );
    assert_eq!(
        code(
            b.open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        unsupported
    );
    assert_eq!(
        code(
            b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        unsupported
    );
    let status = b.quick_unlock_status().unwrap();
    assert!(!status.hardware_keys.supported);
    assert_eq!(status.hardware_keys.keys, vec![]);
    assert_eq!(status.hardware_keys.max_keys, 8);
    b.lock(LockReason::Manual).unwrap();
    assert_eq!(
        code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
        unsupported
    );
    assert_eq!(methods(b), vec![]);
    assert_eq!(r.key.calls(), 0, "no device was touched");

    // Removing needs no device and validates its input.
    unlock_with_password(b);
    assert_eq!(
        code(
            b.remove_hardware_key(&typed(json!({ "methodId": "xyz" })))
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(
            b.remove_hardware_key(&typed(
                json!({ "methodId": "00112233445566778899aabbccddeeff" })
            ))
            .unwrap_err()
        ),
        ErrorCode::HardwareKeyNotRecognized
    );
}

#[test]
fn a_security_key_unlocks_without_the_password_after_a_start() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let b = &r.backend;
    let status = enroll_key(b, Some(PASSWORD), None).unwrap();
    assert!(status.hardware_keys.supported);
    assert_eq!(status.hardware_keys.keys.len(), 1);
    assert_eq!(status.hardware_keys.keys[0].method_id.len(), 32);
    assert_eq!(r.key.calls(), 1);
    assert_eq!(r.key.salts.lock().unwrap().len(), 1, "the raw PRF salt");

    b.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(b), vec![QuickUnlockKind::HardwareKey]);
    let state = b.unlock_quick(&mut pin_request(None)).unwrap();
    assert!(matches!(state, SessionState::Unlocked { .. }));
    assert_eq!(r.key.calls(), 2);
    assert_eq!(b.quick_unlock_status().unwrap().hardware_keys.keys.len(), 1);
}

#[test]
fn the_pin_is_checked_by_the_key_and_five_failures_need_the_password() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let b = &r.backend;
    r.key.needs_pin.store(true, Ordering::SeqCst);
    assert_eq!(
        code(enroll_key(b, Some(PASSWORD), None).unwrap_err()),
        ErrorCode::HardwareKeyPinRequired
    );
    assert_eq!(
        code(enroll_key(b, None, Some("000000")).unwrap_err()),
        ErrorCode::HardwareKeyPinInvalid,
        "the grant of the right password still holds"
    );
    enroll_key(b, None, Some(FAKE_PIN)).unwrap();
    b.lock(LockReason::Manual).unwrap();

    assert_eq!(
        code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
        ErrorCode::HardwareKeyPinRequired,
        "a missing PIN is not a failed attempt"
    );
    for _ in 0..5 {
        assert_eq!(
            code(
                b.unlock_quick(&mut pin_request(Some("000000")))
                    .unwrap_err()
            ),
            ErrorCode::HardwareKeyPinInvalid
        );
    }
    let calls = r.key.calls();
    assert_eq!(
        code(
            b.unlock_quick(&mut pin_request(Some(FAKE_PIN)))
                .unwrap_err()
        ),
        ErrorCode::PasswordRequired,
        "even the right PIN needs the password now"
    );
    assert_eq!(r.key.calls(), calls, "refused before the device");
    assert_eq!(methods(b), vec![]);

    unlock_with_password(b);
    b.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(b), vec![QuickUnlockKind::HardwareKey]);
    b.unlock_quick(&mut pin_request(Some(FAKE_PIN))).unwrap();
}

#[test]
fn a_wrong_key_or_answer_is_not_recognized() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(2);
    let b = &r.backend;
    enroll_key(b, Some(PASSWORD), None).unwrap();
    b.lock(LockReason::Manual).unwrap();

    r.key.plug(Some(1));
    assert_eq!(
        code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
        ErrorCode::HardwareKeyNotRecognized,
        "another physical key"
    );
    r.key.plug(None);
    assert_eq!(
        code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
        ErrorCode::HardwareKeyNotFound
    );
    r.key.plug(Some(0));
    r.key.corrupt_prf.store(true, Ordering::SeqCst);
    for _ in 0..5 {
        assert_eq!(
            code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
            ErrorCode::HardwareKeyNotRecognized,
            "the vault rejects the answer"
        );
    }
    r.key.corrupt_prf.store(false, Ordering::SeqCst);
    assert_eq!(
        code(b.unlock_quick(&mut pin_request(None)).unwrap_err()),
        ErrorCode::PasswordRequired
    );
    assert!(!b.is_unlocked());
}

#[test]
fn locking_cancels_a_running_device_call_and_only_one_runs_at_a_time() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let key = Arc::clone(&r.key);
    let b = Arc::new(r.backend);
    let cell: Arc<OnceLock<Weak<Backend>>> = Arc::default();
    cell.set(Arc::downgrade(&b)).ok();

    // A second device operation while one runs answers busy.
    let inner: Arc<Mutex<Option<ErrorCode>>> = Arc::default();
    let (weak, seen) = (Arc::clone(&cell), Arc::clone(&inner));
    *key.on_call.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            let error = b
                .create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
                .unwrap_err();
            *seen.lock().unwrap() = Some(error.code);
        }
    }));
    enroll_key(&b, Some(PASSWORD), None).unwrap();
    assert_eq!(*inner.lock().unwrap(), Some(ErrorCode::Busy));

    // A lock during the touch cancels it.
    let weak = Arc::clone(&cell);
    *key.on_call.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.get().and_then(Weak::upgrade) {
            b.lock(LockReason::Manual).unwrap();
        }
    }));
    assert_eq!(
        code(enroll_key(&b, Some(PASSWORD), None).unwrap_err()),
        ErrorCode::Cancelled
    );
    assert!(!b.is_unlocked());

    // The cancel command sets the flag (nothing runs, nothing breaks).
    b.hardware_key_cancel();
    assert!(b.quick_unlock.cancel_flag().is_cancelled());
    unlock_with_password(&b);
    assert_eq!(b.quick_unlock_status().unwrap().hardware_keys.keys.len(), 1);
}

#[test]
fn at_most_eight_security_keys_can_be_registered() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let b = &r.backend;
    enroll_key(b, Some(PASSWORD), None).unwrap();
    for _ in 1..8 {
        enroll_key(b, Some(PASSWORD), None).unwrap();
    }
    let calls = r.key.calls();
    assert_eq!(
        code(enroll_key(b, Some(PASSWORD), None).unwrap_err()),
        ErrorCode::HardwareKeyLimit
    );
    assert_eq!(r.key.calls(), calls, "refused before the device");
    let keys = b.quick_unlock_status().unwrap().hardware_keys.keys;
    assert_eq!(keys.len(), 8);
    let after = b
        .remove_hardware_key(&typed(json!({ "methodId": keys[0].method_id })))
        .unwrap();
    assert_eq!(after.hardware_keys.keys.len(), 7);
    enroll_key(b, Some(PASSWORD), None).unwrap();
}

fn totp_draft(title: &str) -> Value {
    json!({
        "text": [{ "field": "title", "value": title }],
        "secrets": [],
        "urls": [],
        "customFields": [],
        "tags": null,
        "otp": { "op": "set", "uri": "otpauth://totp/Bank:alex?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=Bank" }
    })
}

fn find_vault(b: &Backend, vault: &str) -> crate::contract::vaults::VaultSummary {
    b.list_vaults()
        .unwrap()
        .into_iter()
        .find(|v| v.vault_id.0.to_string() == vault)
        .unwrap()
}

#[test]
#[allow(clippy::too_many_lines)]
fn a_hardware_vault_opens_for_five_minutes_and_serves_codes_only_while_open() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(2);
    let b = &r.backend;
    let summary = b
        .create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
        .unwrap();
    let vault = summary.vault_id.0.to_string();
    assert_eq!(summary.protection, VaultProtection::HardwareKey);
    assert!(summary.available);
    let until = summary.open_until.expect("open until");
    let now = system_now_ms();
    assert!(
        until > now + 290_000 && until <= now + 300_000,
        "{until} vs {now}"
    );

    // Only one-time-code items live in it.
    let added = b
        .add_item(typed(
            json!({ "vaultId": vault, "kind": "totp", "draft": totp_draft("Bank") }),
        ))
        .unwrap();
    let item = json!({
        "vaultId": added.summary.vault_id.0.to_string(),
        "itemId": added.summary.item_id.0.to_string(),
    });
    assert_eq!(
        code(
            b.add_item(typed(
                json!({ "vaultId": vault, "kind": "login", "draft": totp_draft("Nope") })
            ))
            .unwrap_err()
        ),
        ErrorCode::ItemTypeNotAllowed
    );
    assert_eq!(b.totp_code(typed(item.clone())).unwrap().period, Some(30));

    // Five minutes later the tick closes it and tells the webview.
    b.close_expired_hardware_vaults(
        Instant::now() + Duration::from_secs(301),
        SystemTime::now(),
        false,
    );
    let closed = json!({ "type": "hardwareVaultClosed", "vaultId": vault });
    assert!(events(&r.events).contains(&closed));
    assert_eq!(
        code(b.totp_code(typed(item.clone())).unwrap_err()),
        ErrorCode::VaultUnavailable
    );
    let listed = find_vault(b, &vault);
    assert!(!listed.available && listed.open_until.is_none());
    assert_eq!(listed.protection, VaultProtection::HardwareKey);
    let keys = b
        .hardware_vault_keys(&typed(json!({ "vaultId": vault })))
        .unwrap();
    assert_eq!(keys.keys.len(), 1, "the keys can be listed while closed");
    let first_id = keys.keys[0].method_id.clone();
    assert_eq!(
        code(
            b.remove_hardware_vault_key(&typed(json!({ "vaultId": vault, "methodId": first_id })))
                .unwrap_err()
        ),
        ErrorCode::VaultUnavailable,
        "managing keys needs the vault open"
    );

    // Another physical key cannot open it; the right one can.
    r.key.plug(Some(1));
    assert_eq!(
        code(
            b.open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        ErrorCode::HardwareKeyNotRecognized
    );
    assert!(!find_vault(b, &vault).available, "it stays closed");
    r.key.plug(Some(0));
    let reopened = b
        .open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
        .unwrap();
    assert!(reopened.available && reopened.open_until.is_some());
    assert!(b.totp_code(typed(item.clone())).is_ok());

    // A second key, then the first can go; the last one cannot. Adding a key
    // needs a right password within two minutes (L6).
    assert_eq!(
        code(
            b.add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })))
                .unwrap_err()
        ),
        ErrorCode::ReauthRequired
    );
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    let second = b
        .add_hardware_vault_key(&mut typed(json!({ "vaultId": vault, "pin": null })))
        .unwrap();
    assert_eq!(second.keys.len(), 2);
    let after = b
        .remove_hardware_vault_key(&typed(json!({ "vaultId": vault, "methodId": first_id })))
        .unwrap();
    assert_eq!(after.keys.len(), 1);
    assert_eq!(
        code(
            b.remove_hardware_vault_key(&typed(json!({ "vaultId": vault, "methodId": first_id })))
                .unwrap_err()
        ),
        ErrorCode::HardwareKeyNotRecognized
    );
    assert_eq!(
        code(
            b.remove_hardware_vault_key(&typed(
                json!({ "vaultId": vault, "methodId": after.keys[0].method_id })
            ))
            .unwrap_err()
        ),
        ErrorCode::HardwareKeyLast
    );

    // A manual close sends no event; the account-key vault refuses these
    // commands.
    let before = events(&r.events).len();
    b.close_hardware_vault(&typed(json!({ "vaultId": vault })))
        .unwrap();
    assert_eq!(events(&r.events).len(), before);
    assert_eq!(
        code(b.totp_code(typed(item.clone())).unwrap_err()),
        ErrorCode::VaultUnavailable
    );
    // Any vault but the hardware one (the list order is not specified).
    let plain = b
        .list_vaults()
        .unwrap()
        .into_iter()
        .map(|v| v.vault_id.0.to_string())
        .find(|id| *id != vault)
        .unwrap();
    for error in [
        b.close_hardware_vault(&typed(json!({ "vaultId": plain })))
            .unwrap_err(),
        b.hardware_vault_keys(&typed(json!({ "vaultId": plain })))
            .unwrap_err(),
    ] {
        assert_eq!(code(error), ErrorCode::InvalidInput);
    }

    // A lock closes it too: after the next unlock it is closed.
    b.open_hardware_vault(&mut typed(json!({ "vaultId": vault, "pin": null })))
        .unwrap();
    b.lock(LockReason::Manual).unwrap();
    unlock_with_password(b);
    let listed = find_vault(b, &vault);
    assert!(!listed.available && listed.open_until.is_none());
    assert_eq!(
        code(b.totp_code(typed(item)).unwrap_err()),
        ErrorCode::VaultUnavailable
    );
}

// ---------------------------------------------------------------------------
// Crypto-review fixes (M1, M2, L2, L5, L6, L9)
// ---------------------------------------------------------------------------

#[test]
fn m1_five_wrong_reauthentications_lock_and_need_the_password_afterwards() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll(b, Some(PASSWORD)).unwrap();
    *b.kit_pending.lock().unwrap() = None;
    for _ in 0..5 {
        let error = b
            .save_emergency_kit(SaveKitRequest {
                labels: crate::emergency_kit::tests::labels(),
                password: Some(SecretIn::test("not the password")),
            })
            .unwrap_err();
        assert_eq!(code(error), ErrorCode::WrongCredentials);
    }
    assert!(!b.is_unlocked());
    assert_eq!(methods(b), Vec::new(), "no quick method is offered");
    let prompts = r.fake.prompts();
    assert_eq!(
        code(
            b.unlock_quick(&mut quick(QuickUnlockKind::Biometric))
                .unwrap_err()
        ),
        ErrorCode::PasswordRequired
    );
    assert_eq!(r.fake.prompts(), prompts, "no Hello prompt");
    // A password unlock restores quick unlock for the next lock.
    unlock_with_password(b);
    b.lock(LockReason::Manual).unwrap();
    assert_eq!(methods(b), vec![QuickUnlockKind::Biometric]);
}

#[test]
fn l2_a_failed_enrollment_keeps_the_grant_and_a_success_uses_it_up() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    r.fake.fail_next(PlatformError::Failed);
    assert_eq!(code(enroll(b, None).unwrap_err()), ErrorCode::Internal);
    assert!(!r.fake.has_key());
    enroll(b, None).unwrap();
    assert_eq!(
        code(enroll(b, None).unwrap_err()),
        ErrorCode::ReauthRequired
    );
}

#[test]
fn l5_every_enrollment_gets_its_own_key_and_the_replaced_one_goes() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let b = &r.backend;
    enroll(b, Some(PASSWORD)).unwrap();
    let nonce_of = || {
        let slot = b.slot().unwrap();
        let params = slot
            .unlocked()
            .unwrap()
            .quick_unlock_enrollment()
            .unwrap()
            .unwrap()
            .platform_params;
        crate::quick_unlock::params::nonce_of(&params).unwrap()
    };
    let first = nonce_of();
    enroll(b, Some(PASSWORD)).unwrap();
    assert_ne!(first, nonce_of(), "a new key name per enrollment");
    assert_eq!(r.fake.key_count(), 1);
}

#[test]
fn l9_a_second_enrollment_while_one_prompts_is_busy() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig(true);
    let fake = Arc::clone(&r.fake);
    let b = Arc::new(r.backend);
    let seen: Arc<Mutex<Option<ErrorCode>>> = Arc::default();
    let (weak, out) = (Arc::downgrade(&b), Arc::clone(&seen));
    *fake.on_enroll.lock().unwrap() = Some(Box::new(move || {
        if let Some(b) = weak.upgrade() {
            // Same thread, the mutex is held by the outer call: busy.
            let error = enroll(&b, Some(PASSWORD)).unwrap_err();
            *out.lock().unwrap() = Some(error.code);
        }
    }));
    enroll(&b, Some(PASSWORD)).unwrap();
    assert_eq!(*seen.lock().unwrap(), Some(ErrorCode::Busy));
    assert_eq!(fake.prompts(), 1, "the busy call never prompted");
}

/// Creates (and so opens) a hardware vault in a fresh rig.
fn open_hw_vault(r: &HwRig) -> (String, mosaic_core::VaultId) {
    let summary = r
        .backend
        .create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
        .unwrap();
    (summary.vault_id.0.to_string(), summary.vault_id.get())
}

#[test]
fn m2_a_hardware_vault_closes_when_either_clock_passed_or_after_a_sleep() {
    let _kdf = crate::test_support::kdf_permit();
    let long = Duration::from_secs(301);
    // The monotonic clock stood still (a suspend), the wall clock moved on.
    let r = hw_rig(1);
    let (vault, _) = open_hw_vault(&r);
    r.backend
        .close_expired_hardware_vaults(Instant::now(), SystemTime::now(), false);
    assert!(find_vault(&r.backend, &vault).available, "not due yet");
    r.backend
        .close_expired_hardware_vaults(Instant::now(), SystemTime::now() + long, false);
    assert!(!find_vault(&r.backend, &vault).available, "wall clock");

    // The wall clock jumped back, the monotonic clock passed.
    let r = hw_rig(1);
    let (vault, _) = open_hw_vault(&r);
    r.backend
        .close_expired_hardware_vaults(Instant::now() + long, SystemTime::now(), false);
    assert!(!find_vault(&r.backend, &vault).available, "monotonic clock");

    // A sleep closes every hardware vault at once.
    let r = hw_rig(1);
    let (vault, _) = open_hw_vault(&r);
    r.backend
        .close_expired_hardware_vaults(Instant::now(), SystemTime::now(), true);
    assert!(!find_vault(&r.backend, &vault).available, "sleep");
    let closed = json!({ "type": "hardwareVaultClosed", "vaultId": vault });
    assert!(events(&r.events).contains(&closed));
}

#[test]
fn m2_an_open_hardware_vault_without_a_deadline_closes_on_the_next_tick() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let (vault, id) = open_hw_vault(&r);
    r.backend.quick_unlock.note_vault_closed(id);
    assert!(find_vault(&r.backend, &vault).available);
    r.backend
        .close_expired_hardware_vaults(Instant::now(), SystemTime::now(), false);
    assert!(!find_vault(&r.backend, &vault).available);
}

#[test]
fn m2_a_deadline_is_forgotten_only_after_the_close() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let (_, id) = open_hw_vault(&r);
    let q = &r.backend.quick_unlock;
    assert!(q.vault_tracked(id));
    r.backend
        .close_expired_hardware_vaults(Instant::now(), SystemTime::now(), false);
    assert!(q.vault_tracked(id), "not due: kept");
    r.backend.close_expired_hardware_vaults(
        Instant::now() + Duration::from_secs(301),
        SystemTime::now(),
        false,
    );
    assert!(!q.vault_tracked(id), "closed: forgotten");
}

#[test]
fn l6_moving_an_item_out_of_a_hardware_vault_needs_a_fresh_password() {
    let _kdf = crate::test_support::kdf_permit();
    let r = hw_rig(1);
    let b = &r.backend;
    let (vault, _) = open_hw_vault(&r);
    // Any vault but the hardware one (the list order is not specified).
    let plain = b
        .list_vaults()
        .unwrap()
        .into_iter()
        .map(|v| v.vault_id.0.to_string())
        .find(|id| *id != vault)
        .unwrap();
    let added = b
        .add_item(typed(
            json!({ "vaultId": vault, "kind": "totp", "draft": totp_draft("Bank") }),
        ))
        .unwrap();
    let mv = json!({
        "vaultId": vault,
        "itemId": added.summary.item_id.0.to_string(),
        "targetVaultId": plain,
    });
    assert_eq!(
        code(b.move_item(typed(mv.clone())).unwrap_err()),
        ErrorCode::ReauthRequired
    );
    b.reauthenticate(SecretIn::test(PASSWORD)).unwrap();
    b.move_item(typed(mv)).unwrap();
}
