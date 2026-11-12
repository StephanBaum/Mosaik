//! The fixes of the G1 crypto review of the mobile core (M10 T3-fix): one test
//! per finding that needs a core to run.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use mosaic_mobile::quick_unlock::params::quick_unlock_aad;
use mosaic_mobile::{
    AndroidCaller, AndroidPasskeyRequest, AssetLinksStatus, CoreContext, FillTarget,
    FillTargetKind, IosAssertRequest, IosRegisterRequest, ItemFilter, ItemKey, JoinHostedRequest,
    LoginDraft, MobileError, OtpEdit, Platform, QuickUnlockAvailability, QuickUnlockPlatform,
    ResetConfirm, ServiceIdentifier, ServiceIdentifierKind, SessionState, SettingsUpdate, UrlDraft,
};
use mosaic_passkey::b64url;
use mosaic_vault::storage::Store;

use crate::common::{BOOT, Desktop, LoopbackServer, PASSWORD, Phone};

const DAY_MS: i64 = 86_400_000;
const CERT: [u8; 32] = [0x11; 32];
const OTHER_CERT: [u8; 32] = [0x22; 32];

fn get_json() -> String {
    format!(
        "{{\"challenge\":\"{}\",\"rpId\":\"example.com\",\"userVerification\":\"required\"}}",
        b64url(&[7u8; 32])
    )
}

fn assetlinks(package: &str, cert: &[u8; 32]) -> Vec<u8> {
    let fingerprint = cert
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");
    format!(
        "[{{\"relation\":[\"delegate_permission/common.get_login_creds\"],         \"target\":{{\"namespace\":\"android_app\",\"package_name\":\"{package}\",         \"sha256_cert_fingerprints\":[\"{fingerprint}\"]}}}}]"
    )
    .into_bytes()
}

fn android_body() -> Vec<u8> {
    vec![0u8; 61]
}

fn wait_for_busy(core: &mosaic_mobile::Core) {
    let start = Instant::now();
    while core.state() != SessionState::Busy {
        assert!(start.elapsed() < Duration::from_secs(20), "never busy");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn login(phone: &Phone, title: &str, url: &str) -> ItemKey {
    phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: title.to_owned(),
            username: "alice".to_owned(),
            password: Some(b"hunter2-hunter2".to_vec()),
            urls: vec![UrlDraft {
                url: url.to_owned(),
                match_etld1: false,
                allow_http: false,
            }],
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap()
}

fn caller(package: &str, signers: Vec<[u8; 32]>) -> AndroidCaller {
    AndroidCaller {
        package: package.to_owned(),
        current_signers: signers.into_iter().map(|c| c.to_vec()).collect(),
        web_domain: None,
        web_scheme: None,
    }
}

fn web_target(origin: &str) -> FillTarget {
    FillTarget {
        kind: FillTargetKind::Web {
            origin: origin.to_owned(),
        },
        display: String::new(),
        warning: None,
    }
}

fn enroll(phone: &Phone, platform: QuickUnlockPlatform, body: Vec<u8>) -> Vec<u8> {
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    let w = setup.w.expose_bytes();
    phone
        .core
        .quick_unlock_enroll_finish(platform, body)
        .unwrap();
    w
}

// ---- G1-1 ----------------------------------------------------------------------------------------

#[test]
fn lock_during_reauth_is_not_lost() {
    let (phone, _) = Phone::android_with_account();
    phone.core.test_hooks().set_work_delay_ms(400);
    let result = std::thread::scope(|scope| {
        let begin = scope.spawn(|| {
            phone
                .core
                .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        });
        wait_for_busy(&phone.core);
        phone.core.lock();
        begin.join().unwrap()
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    assert!(
        matches!(phone.core.state(), SessionState::Locked { .. }),
        "the lock was kept"
    );
    // The flag is spent: the next unlock stays unlocked and a re-authentication
    // works.
    phone.core.test_hooks().set_work_delay_ms(0);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert!(matches!(phone.core.state(), SessionState::Unlocked { .. }));
    phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert!(matches!(phone.core.state(), SessionState::Unlocked { .. }));
}

// ---- G1-2 ----------------------------------------------------------------------------------------

#[test]
fn state_and_overview_never_read_a_secret_to_test_presence() {
    let (mut phone, _) = Phone::with_account();
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    // The open asked for presence; no secret was read, no unlock happened.
    assert!(
        phone
            .storage
            .contains_calls
            .lock()
            .unwrap()
            .iter()
            .any(|n| n.starts_with("secret-key/"))
    );
    for _ in 0..3 {
        match phone.core.state() {
            SessionState::Locked {
                secret_key_stored, ..
            } => assert!(secret_key_stored),
            other => panic!("{other:?}"),
        }
        let overview = phone.core.sync_overview();
        assert!(!overview.signed_in);
    }
    assert_eq!(phone.storage.reads_of("secret-key/"), 0);
    assert_eq!(phone.storage.reads_of("sync-token/"), 0);
    let asked = phone.storage.contains_calls.lock().unwrap().len();
    let _ = phone.core.state();
    let _ = phone.core.sync_overview();
    assert_eq!(
        phone.storage.contains_calls.lock().unwrap().len(),
        asked,
        "state() and sync_overview() do not call the platform at all"
    );
}

// ---- G1-3 ----------------------------------------------------------------------------------------

#[test]
fn reauth_five_wrong_locks_requiring_password() {
    let (mut phone, _) = Phone::android_with_account();
    enroll(
        &phone,
        QuickUnlockPlatform::AndroidKeystore { strongbox: false },
        android_body(),
    );
    let wrong = |phone: &Phone| {
        phone
            .core
            .quick_unlock_enroll_begin(b"not the password".to_vec())
            .err()
    };
    // Four wrong, one right (the count starts again), four wrong: still open.
    for _ in 0..4 {
        assert_eq!(wrong(&phone), Some(MobileError::WrongCredentials));
    }
    phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    for _ in 0..4 {
        assert_eq!(wrong(&phone), Some(MobileError::WrongCredentials));
    }
    assert!(matches!(phone.core.state(), SessionState::Unlocked { .. }));
    // The fifth, through the settings screen, ends the session.
    let settings = phone.core.update_settings(SettingsUpdate {
        quick_unlock_max_age_days: Some(30),
        reauth_password: Some(b"not the password".to_vec()),
        ..SettingsUpdate::default()
    });
    assert_eq!(settings.err(), Some(MobileError::Locked));
    match phone.core.state() {
        SessionState::Locked { quick_unlock, .. } => {
            assert_eq!(quick_unlock, QuickUnlockAvailability::PasswordRequired);
        }
        other => panic!("{other:?}"),
    }
    // The boot marker went with it: a new process cannot adopt it.
    phone.core.close_store();
    assert!(
        Store::open(phone.store_path())
            .unwrap()
            .boot_marker()
            .unwrap()
            .is_none()
    );
    phone.reopen_as(Platform::Android, CoreContext::App, &BOOT);
    match phone.core.state() {
        SessionState::Locked { quick_unlock, .. } => {
            assert_eq!(quick_unlock, QuickUnlockAvailability::PasswordRequired);
        }
        other => panic!("{other:?}"),
    }
    // A password unlock ends the refusal and the count.
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    for _ in 0..4 {
        assert_eq!(wrong(&phone), Some(MobileError::WrongCredentials));
    }
    assert!(matches!(phone.core.state(), SessionState::Unlocked { .. }));
}

// ---- G1-4 ----------------------------------------------------------------------------------------

fn ios_register_request(phone: &Phone, user_verified: bool) -> IosRegisterRequest {
    IosRegisterRequest {
        rp_id: "example.com".to_owned(),
        user_name: "alice".to_owned(),
        user_handle: vec![7; 16],
        client_data_hash: vec![9; 32],
        algorithms: vec![-7],
        exclude_credential_ids: Vec::new(),
        vault_id: phone.vault_id(),
        user_verified,
    }
}

#[test]
fn ios_register_without_uv_is_refused() {
    let (phone, _) = Phone::with_account();
    assert_eq!(
        phone
            .core
            .passkey_ios_register(ios_register_request(&phone, false))
            .err(),
        Some(MobileError::ReauthRequired)
    );
    // No item was stored.
    assert!(phone.core.passkey_identities().unwrap().is_empty());
    assert!(phone.core.items(ItemFilter::default()).unwrap().is_empty());
    phone
        .core
        .passkey_ios_register(ios_register_request(&phone, true))
        .unwrap();
}

#[test]
fn ios_assert_without_uv_is_refused() {
    let (phone, _) = Phone::with_account();
    let made = phone
        .core
        .passkey_ios_register(ios_register_request(&phone, true))
        .unwrap();
    let request = |user_verified| IosAssertRequest {
        rp_id: "example.com".to_owned(),
        client_data_hash: vec![3; 32],
        credential_id: made.credential_id.clone(),
        user_verified,
    };
    let before = phone.core.items(ItemFilter::default()).unwrap();
    assert_eq!(
        phone.core.passkey_ios_assert(request(false)).err(),
        Some(MobileError::ReauthRequired)
    );
    assert_eq!(phone.core.items(ItemFilter::default()).unwrap(), before);
    let assertion = phone.core.passkey_ios_assert(request(true)).unwrap();
    assert_eq!(assertion.authenticator_data[32] & 0x04, 0x04, "UV flag set");
}

#[test]
fn ios_extension_session_comes_from_quick_unlock() {
    // The extension never unlocks with a password, so a session in it is one
    // `W` opened; signing there needs the user verification the same way.
    let (mut phone, _) = Phone::with_account();
    let w = enroll(&phone, QuickUnlockPlatform::IosSecureEnclave, vec![5; 113]);
    phone.core.lock();
    phone.reopen_as(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert_eq!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), None)
            .err(),
        Some(MobileError::PasswordInApp)
    );
    phone.core.unlock_quick(w).unwrap();
    let hash = vec![4; 32];
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: hash,
                credential_id: vec![1; 16],
                user_verified: false,
            })
            .err(),
        Some(MobileError::ReauthRequired)
    );
}

// ---- G1-5 ----------------------------------------------------------------------------------------

#[test]
fn ios_entry_points_refuse_an_android_core() {
    let (phone, _) = Phone::android_with_account();
    assert_eq!(
        phone
            .core
            .fill_target_ios(ServiceIdentifier {
                kind: ServiceIdentifierKind::Domain,
                value: "example.com".to_owned(),
            })
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_candidates("example.com".to_owned(), Vec::new())
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_register(ios_register_request(&phone, true))
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .passkey_ios_assert(IosAssertRequest {
                rp_id: "example.com".to_owned(),
                client_data_hash: vec![3; 32],
                credential_id: vec![1; 16],
                user_verified: true,
            })
            .err(),
        Some(MobileError::InvalidInput)
    );
    // The Android calls work there.
    assert!(
        phone
            .core
            .fill_target_android(caller("com.example.app", vec![CERT]))
            .is_ok()
    );
}

#[test]
fn android_entry_points_refuse_an_ios_core() {
    let (phone, _) = Phone::with_account();
    let key = login(&phone, "Example", "https://example.com");
    assert_eq!(
        phone
            .core
            .fill_target_android(caller("com.example.app", vec![CERT]))
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .link_android_app(
                &key.vault_id,
                &key.item_id,
                caller("com.example.app", vec![CERT])
            )
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .asset_links_status(
                &key.vault_id,
                &key.item_id,
                caller("com.example.app", vec![CERT])
            )
            .err(),
        Some(MobileError::InvalidInput)
    );
    let request = AndroidPasskeyRequest {
        request_json: "{}".to_owned(),
        caller: caller("com.example.app", vec![CERT]),
        platform_origin: None,
        client_data_hash: None,
    };
    assert_eq!(
        phone.core.passkey_android_get_begin(request.clone()).err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone.core.passkey_android_create_begin(request).err(),
        Some(MobileError::InvalidInput)
    );
    assert!(
        phone
            .core
            .fill_target_ios(ServiceIdentifier {
                kind: ServiceIdentifierKind::Domain,
                value: "example.com".to_owned(),
            })
            .is_ok()
    );
}

// ---- G1-6 ----------------------------------------------------------------------------------------

#[test]
fn secret_read_before_a_lock_is_not_handed_out() {
    let (phone, _) = Phone::with_account();
    let key = login(&phone, "Example", "https://example.com");
    phone.core.test_hooks().set_work_delay_ms(400);
    let result = std::thread::scope(|scope| {
        let reveal = scope.spawn(|| {
            phone
                .core
                .reveal(&key.vault_id, &key.item_id, "login.password")
        });
        // `reveal` has read the value and waits before it hands it out.
        std::thread::sleep(Duration::from_millis(150));
        phone.core.lock();
        reveal.join().unwrap()
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    phone.core.test_hooks().set_work_delay_ms(0);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(
        phone
            .core
            .reveal(&key.vault_id, &key.item_id, "login.password")
            .unwrap()
            .expose_bytes(),
        b"hunter2-hunter2"
    );
}

// ---- G1-8 ----------------------------------------------------------------------------------------

#[test]
fn extension_context_never_fetches_asset_links() {
    let phone = Phone::with(Platform::Android, CoreContext::IosAutofillExtension);
    phone
        .core
        .test_hooks()
        .create_local_account("user@example.com", PASSWORD)
        .unwrap();
    let key = login(&phone, "Example", "https://example.com");
    let fetches = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&fetches);
    phone.core.test_hooks().set_asset_links_fetch(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(b"[]".to_vec())
    });
    assert_eq!(
        phone
            .core
            .asset_links_status(
                &key.vault_id,
                &key.item_id,
                caller("com.example.app", vec![CERT])
            )
            .err(),
        Some(MobileError::InvalidInput)
    );
    let request = AndroidPasskeyRequest {
        request_json: get_json(),
        caller: caller("com.example.app", vec![CERT]),
        platform_origin: None,
        client_data_hash: None,
    };
    assert!(phone.core.passkey_android_get_begin(request).is_err());
    assert_eq!(
        fetches.load(Ordering::SeqCst),
        0,
        "no network in the extension"
    );
}

// ---- G1-9 ----------------------------------------------------------------------------------------

#[test]
fn more_than_one_current_signer_is_refused() {
    let (phone, _) = Phone::android_with_account();
    let key = login(&phone, "Example", "https://example.com");
    let two = caller("com.example.app", vec![OTHER_CERT, CERT]);
    assert_eq!(
        phone
            .core
            .link_android_app(&key.vault_id, &key.item_id, two.clone())
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .asset_links_status(&key.vault_id, &key.item_id, two.clone())
            .err(),
        Some(MobileError::InvalidInput)
    );
    let none = caller("com.example.app", Vec::new());
    assert_eq!(
        phone
            .core
            .link_android_app(&key.vault_id, &key.item_id, none)
            .err(),
        Some(MobileError::InvalidInput)
    );
    // The site delegates the app to the certificate: with one current signer
    // the passkey request passes, with the old one beside it it does not.
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks("com.example.app", &CERT)));
    let request = |c| AndroidPasskeyRequest {
        request_json: get_json(),
        caller: c,
        platform_origin: None,
        client_data_hash: None,
    };
    assert!(
        phone
            .core
            .passkey_android_get_begin(request(caller("com.example.app", vec![CERT])))
            .is_ok()
    );
    assert_eq!(
        phone.core.passkey_android_get_begin(request(two)).err(),
        Some(MobileError::InvalidInput)
    );
    // One current signer still links.
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Err(mosaic_sync::TransportError::NotFound));
    phone
        .core
        .link_android_app(
            &key.vault_id,
            &key.item_id,
            caller("com.example.app", vec![CERT]),
        )
        .unwrap();
    assert_eq!(
        phone
            .core
            .asset_links_status(
                &key.vault_id,
                &key.item_id,
                caller("com.example.app", vec![CERT])
            )
            .unwrap(),
        AssetLinksStatus::NotListed
    );
}

// ---- G1-10 ---------------------------------------------------------------------------------------

#[test]
fn quick_unlock_keeps_the_age_verified_at_enrolment() {
    let (mut phone, _) = Phone::android_with_account();
    // A verified 30 days (raising the age needs the password).
    phone
        .core
        .update_settings(SettingsUpdate {
            quick_unlock_max_age_days: Some(30),
            reauth_password: Some(PASSWORD.as_bytes().to_vec()),
            ..SettingsUpdate::default()
        })
        .unwrap();
    let w = enroll(
        &phone,
        QuickUnlockPlatform::AndroidKeystore { strongbox: false },
        android_body(),
    );
    // A new process: the settings file is not verified yet, so its strict
    // stand-in values (14 days) must not shorten the enrolment.
    phone.reopen_as(Platform::Android, CoreContext::App, &BOOT);
    phone.core.test_hooks().set_clock_offset_ms(20 * DAY_MS);
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::Available { .. },
            ..
        }
    ));
    assert!(matches!(
        phone.core.unlock_quick(w.clone()).unwrap(),
        SessionState::Unlocked { .. }
    ));
    // Day 31: past the cap, deleted.
    phone.core.lock();
    phone.reopen_as(Platform::Android, CoreContext::App, &BOOT);
    phone
        .core
        .test_hooks()
        .set_clock_offset_ms(31 * DAY_MS + 1_000);
    // The state read inside the unlock finds it expired and deletes it first.
    assert_eq!(
        phone.core.unlock_quick(w).err(),
        Some(MobileError::PasswordRequired)
    );
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::Expired { .. },
            ..
        }
    ));
}

#[test]
fn quick_unlock_age_follows_a_verified_lower_setting() {
    let (phone, _) = Phone::android_with_account();
    let w = enroll(
        &phone,
        QuickUnlockPlatform::AndroidKeystore { strongbox: false },
        android_body(),
    );
    // Lowering the age needs no password and applies at once (verified).
    phone
        .core
        .update_settings(SettingsUpdate {
            quick_unlock_max_age_days: Some(1),
            ..SettingsUpdate::default()
        })
        .unwrap();
    phone.core.lock();
    phone.core.test_hooks().set_clock_offset_ms(2 * DAY_MS);
    assert_eq!(
        phone.core.unlock_quick(w).err(),
        Some(MobileError::PasswordRequired)
    );
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::Expired { .. },
            ..
        }
    ));
}

// ---- G1-11 ---------------------------------------------------------------------------------------

fn pending_join(email: &str) -> (LoopbackServer, Desktop, String, Phone) {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, email);
    let secret = desktop.enable_totp(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    phone
        .core
        .join_hosted(JoinHostedRequest {
            server_url: server.base.clone(),
            email: email.to_owned(),
            password: PASSWORD.as_bytes().to_vec(),
            secret_key: desktop.sk_text().into_bytes(),
            device_name: None,
        })
        .unwrap();
    (server, desktop, secret, phone)
}

#[test]
fn expired_pending_join_is_dropped_by_lock_state_and_close() {
    for how in ["lock", "state", "close_store"] {
        let (_server, _desktop, _secret, phone) = pending_join(&format!("{how}@example.com"));
        phone.core.test_hooks().set_clock_offset_ms(6 * 60_000);
        match how {
            "lock" => phone.core.lock(),
            "state" => {
                let _ = phone.core.state();
            }
            _ => phone.core.close_store(),
        }
        // Dropped, not merely expired: `notFound`, not `syncHandoffExpired`.
        assert_eq!(
            phone.core.join_totp(b"123456".to_vec()).err(),
            Some(MobileError::NotFound),
            "{how}"
        );
    }
}

#[test]
fn a_lock_drops_the_pending_join() {
    let (_server, _desktop, _secret, phone) = pending_join("lockdrops@example.com");
    phone.core.lock();
    assert_eq!(
        phone.core.join_totp(b"123456".to_vec()).err(),
        Some(MobileError::NotFound)
    );
}

// ---- G1-12 ---------------------------------------------------------------------------------------

#[test]
fn fill_session_is_required_bound_and_dropped_on_lock() {
    let (phone, _) = Phone::with_account();
    let key = login(&phone, "Example", "https://example.com");
    // An unknown session is refused.
    assert_eq!(
        phone.core.fill_values("00".repeat(16), key.clone()).err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        phone.core.fill_otp("00".repeat(16), key.clone()).err(),
        Some(MobileError::NotFound)
    );
    // A real one fills; it is bound to the target it was made for.
    let session = phone
        .core
        .fill_candidates(web_target("https://example.com"))
        .unwrap();
    assert_eq!(session.session_id.len(), 32);
    assert_eq!(
        phone
            .core
            .fill_values(session.session_id.clone(), key.clone())
            .unwrap()
            .password
            .unwrap()
            .expose_bytes(),
        b"hunter2-hunter2"
    );
    let other = phone
        .core
        .fill_candidates(web_target("https://evil.example"))
        .unwrap();
    assert!(other.candidates.is_empty());
    assert_eq!(
        phone.core.fill_values(other.session_id, key.clone()).err(),
        Some(MobileError::NoMatch)
    );
    // It runs out.
    phone.core.test_hooks().set_clock_offset_ms(6 * 60_000);
    assert_eq!(
        phone
            .core
            .fill_values(session.session_id.clone(), key.clone())
            .err(),
        Some(MobileError::NotFound)
    );
    // And a lock drops it.
    phone.core.test_hooks().set_clock_offset_ms(0);
    let session = phone
        .core
        .fill_candidates(web_target("https://example.com"))
        .unwrap();
    phone.core.lock();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(
        phone.core.fill_values(session.session_id, key).err(),
        Some(MobileError::NotFound)
    );
}

// ---- G1-13 ---------------------------------------------------------------------------------------

/// Makes the removal of the store files fail: an open file inside the
/// attachment directory (Windows) or a read-only directory (Unix).
fn block_wipe(phone: &Phone) -> Box<dyn std::any::Any> {
    let dir = phone
        .dir
        .path()
        .join("mosaic")
        .join("store.sqlite-attachments");
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::write(dir.join("blocker"), b"x").unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        Box::new(dir)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // No FILE_SHARE_DELETE: the file cannot be removed while it is open.
        Box::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .share_mode(3)
                .open(dir.join("blocker"))
                .unwrap(),
        )
    }
}

#[test]
fn reset_device_that_cannot_wipe_does_not_stay_busy() {
    let (phone, _) = Phone::with_account();
    let guard = block_wipe(&phone);
    let result = phone.core.reset_device(ResetConfirm {
        typed_email: "user@example.com".to_owned(),
    });
    assert_eq!(result.err(), Some(MobileError::StoreUnavailable));
    assert_ne!(phone.core.state(), SessionState::Busy);
    assert_ne!(phone.core.state(), SessionState::StoreClosed);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = guard.downcast_ref::<std::path::PathBuf>().unwrap();
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    drop(guard);
    // The second try, with nothing in the way, finishes.
    let outcome = phone
        .core
        .reset_device(ResetConfirm {
            typed_email: "user@example.com".to_owned(),
        })
        .map(|o| o.state);
    assert!(
        matches!(
            outcome,
            Ok(SessionState::NoAccount) | Err(MobileError::InvalidInput)
        ),
        "{outcome:?}"
    );
}

// ---- G1-14 ---------------------------------------------------------------------------------------

#[test]
fn mobile_crate_draws_no_randomness_of_its_own() {
    // IDs and nonces come from `mosaic_core::random_public_bytes`: no direct
    // `getrandom` in this crate's sources or dependencies.
    let manifest = include_str!("../../Cargo.toml");
    assert!(!manifest.contains("getrandom"));
    for source in [
        include_str!("../../src/otp.rs"),
        include_str!("../../src/passkeys/mod.rs"),
        include_str!("../../src/quick_unlock/mod.rs"),
        include_str!("../../src/autofill.rs"),
    ] {
        assert!(!source.contains("getrandom::"));
    }
}

#[test]
fn quick_unlock_aad_is_unchanged_by_the_age_byte() {
    // The platform parameters carry the age after the body; the GCM AAD
    // (header, label, nonce) does not.
    let aad = quick_unlock_aad(&[5; 16]);
    assert_eq!(&aad[..2], &[0x02, 0x04]);
}
