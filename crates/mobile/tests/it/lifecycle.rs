//! Open, close, lock and concurrency.

use std::time::{Duration, Instant};

use mosaic_mobile::{
    CoreContext, ItemFilter, LoginDraft, MobileError, OtpEdit, Platform, QuickUnlockAvailability,
    SessionState, UrlDraft,
};

use crate::common::{BOOT, Desktop, LoopbackServer, PASSWORD, Phone};

fn wait_for_busy(core: &mosaic_mobile::Core) {
    let start = Instant::now();
    while core.state() != SessionState::Busy {
        assert!(start.elapsed() < Duration::from_secs(20), "never busy");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn open_new_dir_reports_no_account() {
    let phone = Phone::new();
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    assert_eq!(phone.core.required_unlock_memory_bytes(), None);
    assert_eq!(
        phone.core.items(ItemFilter::default()),
        Err(MobileError::NoAccount)
    );
}

#[test]
fn close_store_releases_the_file_lock() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    // Another process (the extension) cannot open the store while it is open.
    let other = phone.second_core(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert_eq!(other.state(), SessionState::StoreBusy);
    drop(other);
    phone.core.close_store();
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
    let other = phone.second_core(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert!(matches!(other.state(), SessionState::Locked { .. }));
    // And now the app finds it busy until the extension lets go.
    assert_eq!(phone.core.reopen_store().unwrap(), SessionState::StoreBusy);
    other.close_store();
    drop(other);
    assert!(matches!(
        phone.core.reopen_store().unwrap(),
        SessionState::Locked { .. }
    ));
}

#[test]
fn reopen_after_close_restores_locked_state() {
    let (phone, _) = Phone::with_account();
    phone.core.close_store();
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
    assert_eq!(
        phone.core.items(ItemFilter::default()),
        Err(MobileError::StoreUnavailable)
    );
    match phone.core.reopen_store().unwrap() {
        SessionState::Locked {
            email,
            secret_key_stored,
            quick_unlock,
        } => {
            assert_eq!(email.as_deref(), Some("user@example.com"));
            assert!(secret_key_stored);
            assert_eq!(quick_unlock, QuickUnlockAvailability::NotEnrolled);
        }
        other => panic!("{other:?}"),
    }
    // It unlocks again.
    assert!(matches!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), None)
            .unwrap(),
        SessionState::Unlocked { .. }
    ));
}

fn login_draft(vault: &str, password: &str) -> LoginDraft {
    LoginDraft {
        vault_id: vault.to_owned(),
        item_id: None,
        title: "Example".to_owned(),
        username: "alice".to_owned(),
        password: Some(password.as_bytes().to_vec()),
        urls: vec![UrlDraft {
            url: "https://example.com".to_owned(),
            match_etld1: false,
            allow_http: false,
        }],
        notes: None,
        otp: OtpEdit::Keep,
        favorite: false,
    }
}

#[test]
fn lock_drops_session_slots_and_secret_values() {
    let (phone, _) = Phone::with_account();
    let vault = phone.vault_id();
    let key = phone
        .core
        .save_login(login_draft(&vault, "hunter2-hunter2"))
        .unwrap();
    let revealed = phone
        .core
        .reveal(&key.vault_id, &key.item_id, "login.password")
        .unwrap();
    assert_eq!(revealed.expose_bytes(), b"hunter2-hunter2");
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert_eq!(setup.w.len(), 32);
    let pending = phone
        .core
        .otp_from_text(b"JBSWY3DPEHPK3PXP".to_vec())
        .unwrap();

    phone.core.lock();

    // Every secret value handed out is wiped.
    assert_eq!(revealed.len(), 0);
    assert!(revealed.expose_bytes().is_empty());
    assert_eq!(setup.w.len(), 0);
    // Everything that needs the session is refused.
    assert_eq!(
        phone
            .core
            .reveal(&key.vault_id, &key.item_id, "login.password")
            .err(),
        Some(MobileError::Locked)
    );
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    // The pending slots did not survive the lock.
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(
                mosaic_mobile::QuickUnlockPlatform::IosSecureEnclave,
                vec![0; 113]
            )
            .err(),
        Some(MobileError::ReauthRequired)
    );
    let mut draft = login_draft(&vault, "x");
    draft.otp = OtpEdit::SetPending {
        id: pending.pending_id,
    };
    assert_eq!(
        phone.core.save_login(draft).err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn lock_during_unlock_ends_locked() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    phone.core.test_hooks().set_work_delay_ms(400);
    let result = std::thread::scope(|scope| {
        let unlock = scope.spawn(|| {
            phone
                .core
                .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        });
        wait_for_busy(&phone.core);
        phone.core.lock();
        unlock.join().unwrap()
    });
    assert!(
        matches!(result, Ok(SessionState::Locked { .. })),
        "{result:?}"
    );
    assert!(matches!(phone.core.state(), SessionState::Locked { .. }));
    assert_eq!(
        phone.core.items(ItemFilter::default()).err(),
        Some(MobileError::Locked)
    );
    // The next unlock works.
    phone.core.test_hooks().set_work_delay_ms(0);
    assert!(matches!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), None)
            .unwrap(),
        SessionState::Unlocked { .. }
    ));
}

#[test]
fn concurrent_calls_during_unlock_answer_busy() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    phone.core.test_hooks().set_work_delay_ms(400);
    std::thread::scope(|scope| {
        let unlock = scope.spawn(|| {
            phone
                .core
                .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        });
        wait_for_busy(&phone.core);
        assert_eq!(
            phone.core.items(ItemFilter::default()).err(),
            Some(MobileError::Busy)
        );
        assert_eq!(
            phone
                .core
                .unlock_password(PASSWORD.as_bytes().to_vec(), None)
                .err(),
            Some(MobileError::Busy)
        );
        assert_eq!(phone.core.state(), SessionState::Busy);
        assert!(matches!(
            unlock.join().unwrap(),
            Ok(SessionState::Unlocked { .. })
        ));
    });
}

#[test]
fn lock_during_sync_fails_sync_without_deadlock() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "sync-lock@example.com");
    desktop.add_login("Desk", "bob", "pw-1234", "https://desk.example");
    desktop.sync(&server);
    let phone = Phone::new();
    phone.join(&server, "sync-lock@example.com", &desktop.sk_text());
    phone.core.test_hooks().set_work_delay_ms(1500);
    let result = std::thread::scope(|scope| {
        let sync = scope.spawn(|| phone.core.sync_now());
        std::thread::sleep(Duration::from_millis(300));
        phone.core.lock();
        sync.join().unwrap()
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    assert!(matches!(phone.core.state(), SessionState::Locked { .. }));
}
