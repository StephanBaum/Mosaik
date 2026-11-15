//! Joining a hosted account (against the real server on `127.0.0.1`).

use mosaic_mobile::{
    CoreContext, HandoffStatus, ItemFilter, JoinHostedRequest, JoinOutcome, MobileError, Platform,
    SecondFactorMethods, SessionState,
};
use mosaic_sync::wire::json::RegistrationMode;

use crate::common::{BOOT, Desktop, LoopbackServer, PASSWORD, Phone};

fn wait_for_busy(core: &mosaic_mobile::Core) {
    let start = std::time::Instant::now();
    while core.state() != SessionState::Busy {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(20),
            "never busy"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn request(server: &LoopbackServer, email: &str, sk: &str, password: &str) -> JoinHostedRequest {
    JoinHostedRequest {
        server_url: server.base.clone(),
        email: email.to_owned(),
        password: password.as_bytes().to_vec(),
        secret_key: sk.as_bytes().to_vec(),
        device_name: None,
    }
}

#[test]
fn join_hosted_unlocks_and_stores_secret_key_and_token() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "join@example.com");
    desktop.add_login("Desk", "bob", "pw-1234", "https://desk.example");
    desktop.sync(&server);

    let phone = Phone::new();
    let info = phone.core.prelogin_check(server.base.clone()).unwrap();
    assert_eq!(info.origin, server.base);
    assert!(!info.webauthn, "an IP origin cannot run the ceremony");
    assert_eq!(
        phone
            .core
            .prelogin_check("http://example.com".to_owned())
            .err(),
        Some(MobileError::SyncServerInsecure)
    );

    server.advance_ms(60_000);
    let outcome = phone
        .core
        .join_hosted(request(
            &server,
            "Join@Example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    let JoinOutcome::Unlocked(SessionState::Unlocked {
        account_id,
        email,
        quick_unlock_enrolled,
    }) = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(account_id, desktop.account.to_string());
    assert_eq!(email.as_deref(), Some("join@example.com"));
    assert!(!quick_unlock_enrolled);
    // The Secret Key and the session token are in secure storage, under the
    // names of CRYPTO_SPEC 6.9.1, and nowhere else.
    let hex = account_id.replace('-', "");
    assert_eq!(
        phone.storage.names(),
        vec![format!("secret-key/{hex}"), format!("sync-token/{hex}")]
    );
    let map = phone.storage.map.lock().unwrap();
    assert_eq!(map[&format!("secret-key/{hex}")].len(), 16);
    assert_eq!(map[&format!("sync-token/{hex}")].len(), 32);
    drop(map);
    // The phone can sync and sees the vault; a lock and an unlock with the
    // stored key work.
    phone.core.sync_now().unwrap();
    assert_eq!(phone.core.items(ItemFilter::default()).unwrap().len(), 1);
    phone.core.lock();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let overview = phone.core.sync_overview();
    assert!(overview.configured && overview.signed_in);
    assert_eq!(overview.server_url.as_deref(), Some(server.base.as_str()));
}

#[test]
fn join_hosted_wrong_password_is_wrong_credentials() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "wrong@example.com");
    let phone = Phone::new();
    server.advance_ms(60_000);
    assert_eq!(
        phone
            .core
            .join_hosted(request(
                &server,
                "wrong@example.com",
                &desktop.sk_text(),
                "a wrong password"
            ))
            .err(),
        Some(MobileError::WrongCredentials)
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    assert!(phone.storage.names().is_empty());
    // An empty password and a mistyped Secret Key fail before any request.
    assert_eq!(
        phone
            .core
            .join_hosted(request(
                &server,
                "wrong@example.com",
                &desktop.sk_text(),
                ""
            ))
            .err(),
        Some(MobileError::EmptyPassword)
    );
    assert_eq!(
        phone
            .core
            .join_hosted(request(
                &server,
                "wrong@example.com",
                "M1-nonsense",
                PASSWORD
            ))
            .err(),
        Some(MobileError::SecretKeyMalformed)
    );
}

#[test]
fn join_refused_when_account_exists() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "exists@example.com");
    let (phone, _) = Phone::with_account();
    assert_eq!(
        phone
            .core
            .join_hosted(request(
                &server,
                "exists@example.com",
                &desktop.sk_text(),
                PASSWORD
            ))
            .err(),
        Some(MobileError::AccountExists)
    );
    // Still unlocked and untouched.
    assert!(matches!(phone.core.state(), SessionState::Unlocked { .. }));
}

#[test]
fn join_hosted_totp_second_factor() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "totp@example.com");
    let secret = desktop.enable_totp(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    let outcome = phone
        .core
        .join_hosted(request(
            &server,
            "totp@example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    assert_eq!(
        outcome,
        JoinOutcome::SecondFactor {
            methods: SecondFactorMethods {
                totp: true,
                webauthn: false
            }
        }
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    // A wrong code is refused and the join stays pending.
    assert_eq!(
        phone.core.join_totp(b"000000".to_vec()).err(),
        Some(MobileError::SyncTwoFactorInvalid)
    );
    server.advance_ms(30_000);
    let code = Desktop::code_now(&server, &secret);
    let state = phone.core.join_totp(code.into_bytes()).unwrap();
    assert!(matches!(state, SessionState::Unlocked { .. }));
    assert_eq!(phone.storage.names().len(), 2);
    // Nothing waits any more.
    assert_eq!(
        phone.core.join_totp(b"123456".to_vec()).err(),
        Some(MobileError::NotFound)
    );
}

fn add_key_row(server: &LoopbackServer, desktop: &Desktop) {
    let account = desktop.account;
    server
        .state
        .db()
        .run(|c| {
            c.execute(
                "INSERT INTO webauthn_credentials (account_id, credential_id, public_key, \
                 sign_count, label, created_at) VALUES (?1, ?2, ?3, 0, ?4, 0)",
                rusqlite::params![
                    account.as_bytes().as_slice(),
                    [7u8; 16].as_slice(),
                    [1u8; 8].as_slice(),
                    "YubiKey"
                ],
            )?;
            Ok::<_, mosaic_server::ApiError>(())
        })
        .unwrap();
}

/// A server whose identity is a host name (`localhost`), so it can run the
/// security-key ceremony.
fn host_name_server() -> LoopbackServer {
    let mut server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
    let port = server.base.rsplit(':').next().unwrap().to_owned();
    server.base = format!("http://localhost:{port}");
    server
}

#[test]
fn join_hosted_webauthn_begin_returns_handoff_url_and_waits() {
    let server = host_name_server();
    let desktop = Desktop::register(&server, "key@example.com");
    desktop.enable_totp(&server);
    add_key_row(&server, &desktop);
    let phone = Phone::new();
    assert!(
        phone
            .core
            .prelogin_check(server.base.clone())
            .unwrap()
            .webauthn
    );
    server.advance_ms(60_000);
    let outcome = phone
        .core
        .join_hosted(request(
            &server,
            "key@example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    assert_eq!(
        outcome,
        JoinOutcome::SecondFactor {
            methods: SecondFactorMethods {
                totp: true,
                webauthn: true
            }
        }
    );
    // No handoff yet.
    assert_eq!(
        phone.core.join_webauthn_status().err(),
        Some(MobileError::NotFound)
    );
    let info = phone.core.join_webauthn_begin().unwrap();
    assert!(
        info.url.starts_with(&format!("{}/webauthn/#", server.base)),
        "{}",
        info.url
    );
    assert_eq!(info.match_code.len(), 6);
    assert!(info.match_code.bytes().all(|b| b.is_ascii_digit()));
    assert!(info.expires_at_ms > 0);
    assert_eq!(
        phone.core.join_webauthn_status().unwrap(),
        HandoffStatus::Waiting
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    phone.core.join_cancel();
    assert_eq!(
        phone.core.join_webauthn_status().err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn pending_join_holds_no_store() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "nostore@example.com");
    let secret = desktop.enable_totp(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    phone
        .core
        .join_hosted(request(
            &server,
            "nostore@example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    // The app goes to the background (iOS): the store is closed, and another
    // process (the extension) can open it, because the join holds no store.
    phone.core.close_store();
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
    let other = phone.second_core(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    assert_eq!(other.state(), SessionState::NoAccount);
    drop(other);
    // Back in the foreground the join is still pending and finishes.
    assert_eq!(phone.core.reopen_store().unwrap(), SessionState::NoAccount);
    phone.core.close_store();
    server.advance_ms(30_000);
    let code = Desktop::code_now(&server, &secret);
    let state = phone.core.join_totp(code.into_bytes()).unwrap();
    assert!(matches!(state, SessionState::Unlocked { .. }), "{state:?}");
}

#[test]
fn pending_join_expires_after_five_minutes() {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "expire@example.com");
    let secret = desktop.enable_totp(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    phone
        .core
        .join_hosted(request(
            &server,
            "expire@example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    phone.core.test_hooks().set_clock_offset_ms(301_000);
    server.advance_ms(30_000);
    let code = Desktop::code_now(&server, &secret);
    assert_eq!(
        phone.core.join_totp(code.into_bytes()).err(),
        Some(MobileError::SyncHandoffExpired)
    );
    // And it is gone.
    assert_eq!(
        phone.core.join_totp(b"123456".to_vec()).err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
}

#[test]
fn folder_setup_code_or_config_is_folder_account_unsupported() {
    // A store whose account syncs through a folder (a copied desktop store, say).
    let (phone, _) = Phone::with_account();
    phone.core.close_store();
    {
        let mut store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
        store
            .set_sync_config(Some(&mosaic_vault::storage::SyncConfig {
                kind: mosaic_vault::storage::SyncKind::Folder,
                server_url: None,
                email: None,
                folder_path: Some(phone.dir.path().join("folder")),
                device_name: "Phone".to_owned(),
            }))
            .unwrap();
    }
    phone.core.reopen_store().unwrap();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(
        phone.core.sync_now().err(),
        Some(MobileError::FolderAccountUnsupported)
    );
    assert_eq!(
        phone
            .core
            .recover_password(b"another password".to_vec())
            .err(),
        Some(MobileError::FolderAccountUnsupported)
    );
    let overview = phone.core.sync_overview();
    assert!(!overview.configured);
}

#[test]
fn a_lock_during_join_without_a_second_factor_ends_locked() {
    // M10-G2 round 2, N3: the lock came while the join call ran.
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "joinlock@example.com");
    desktop.sync(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    phone.core.test_hooks().set_work_delay_ms(400);
    let outcome = std::thread::scope(|scope| {
        let join = scope.spawn(|| {
            phone.core.join_hosted(request(
                &server,
                "joinlock@example.com",
                &desktop.sk_text(),
                PASSWORD,
            ))
        });
        wait_for_busy(&phone.core);
        phone.core.lock();
        join.join().unwrap()
    });
    assert!(outcome.is_ok(), "{outcome:?}");
    assert!(
        matches!(phone.core.state(), SessionState::Locked { .. }),
        "the lock was kept"
    );
}

/// A phone with a TOTP join pending, then `act` runs while `finish_join`
/// fetches the account (the work delay holds it there). Returns the phone and
/// what the join call returned.
fn totp_join_with(act: impl FnOnce(&Phone)) -> (Phone, Result<SessionState, MobileError>) {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, "joinflight@example.com");
    let secret = desktop.enable_totp(&server);
    let phone = Phone::new();
    server.advance_ms(60_000);
    phone
        .core
        .join_hosted(request(
            &server,
            "joinflight@example.com",
            &desktop.sk_text(),
            PASSWORD,
        ))
        .unwrap();
    server.advance_ms(30_000);
    let code = Desktop::code_now(&server, &secret);
    phone.core.test_hooks().set_work_delay_ms(400);
    let outcome = std::thread::scope(|scope| {
        let join = scope.spawn(|| phone.core.join_totp(code.into_bytes()));
        let start = std::time::Instant::now();
        while !phone.core.test_hooks().join_in_flight() {
            assert!(start.elapsed() < std::time::Duration::from_secs(20));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        act(&phone);
        join.join().unwrap()
    });
    phone.core.test_hooks().set_work_delay_ms(0);
    (phone, outcome)
}

#[test]
fn a_lock_during_finish_joins_fetch_ends_locked() {
    let (phone, outcome) = totp_join_with(|phone| phone.core.lock());
    assert!(outcome.is_ok(), "{outcome:?}");
    assert!(
        matches!(phone.core.state(), SessionState::Locked { .. }),
        "the lock was kept, not cleared"
    );
}

#[test]
fn a_close_during_finish_joins_fetch_ends_closed() {
    let (phone, outcome) = totp_join_with(|phone| phone.core.close_store());
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(phone.core.state(), SessionState::StoreClosed);
}

#[test]
fn a_join_cancel_during_finish_joins_fetch_keeps_the_lock() {
    let (phone, outcome) = totp_join_with(|phone| {
        phone.core.lock();
        phone.core.join_cancel();
    });
    assert!(outcome.is_ok(), "{outcome:?}");
    assert!(
        matches!(phone.core.state(), SessionState::Locked { .. }),
        "join_cancel in flight neither unlocked nor dropped the lock"
    );
}
