//! Sync, recovery and the device reset (two cores or a desktop device and a
//! phone, over the real server on `127.0.0.1`).

use mosaic_core::{Argon2Profile, SecretKey};
use mosaic_mobile::{
    ItemFilter, LoginDraft, MobileError, OtpEdit, QuickUnlockAvailability, QuickUnlockPlatform,
    ResetConfirm, SessionState, UrlDraft,
};
use mosaic_sync::http::hosted::{self, RekeyOutcome};
use mosaic_vault::session::{RekeyPlan, RekeyRequest, RotationKind};

use crate::common::{Desktop, LoopbackServer, PASSWORD, Phone, pw};

const NEW_PASSWORD: &str = "a completely different passphrase";

/// A desktop with one login that a phone joined.
fn world(email: &str) -> (LoopbackServer, Desktop, Phone) {
    let server = LoopbackServer::start();
    let desktop = Desktop::register(&server, email);
    desktop.add_login("Desk login", "bob", "desk-pw-1234", "https://desk.example");
    desktop.sync(&server);
    let phone = Phone::new();
    phone.join(&server, email, &desktop.sk_text());
    (server, desktop, phone)
}

fn titles(phone: &Phone) -> Vec<String> {
    phone
        .core
        .items(ItemFilter::default())
        .unwrap()
        .into_iter()
        .map(|i| i.title)
        .collect()
}

#[test]
fn sync_now_pushes_edit_and_pulls_remote() {
    let (server, desktop, phone) = world("sync@example.com");
    // Pull: the desktop's login arrives.
    let overview = phone.core.sync_now().unwrap();
    assert_eq!(titles(&phone), vec!["Desk login".to_owned()]);
    assert!(overview.last_synced_at_ms.is_some());
    assert_eq!(overview.last_error, None);
    assert!(!overview.halted);

    // Push: a login saved on the phone reaches the desktop.
    let vault = phone.vault_id();
    let key = phone
        .core
        .save_login(LoginDraft {
            vault_id: vault,
            item_id: None,
            title: "Phone login".to_owned(),
            username: "carol".to_owned(),
            password: Some(b"phone-pw-9876".to_vec()),
            urls: vec![UrlDraft {
                url: "https://phone.example".to_owned(),
                match_etld1: false,
                allow_http: false,
            }],
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap();
    assert!(phone.core.sync_overview().pending_uploads > 0);
    let overview = phone.core.sync_now().unwrap();
    assert_eq!(overview.pending_uploads, 0);
    desktop.sync(&server);
    let desktop_titles: Vec<String> = desktop.with(|s| {
        s.items(&mosaic_vault::session::ItemFilter::default())
            .iter()
            .map(|r| r.item.title().unwrap().to_owned())
            .collect()
    });
    assert!(
        desktop_titles.contains(&"Phone login".to_owned()),
        "{desktop_titles:?}"
    );

    // An edit on the desktop reaches the phone.
    let vault_id = desktop.vault();
    let item_id: mosaic_core::ItemId = desktop.with(|s| {
        s.items(&mosaic_vault::session::ItemFilter::default())
            .iter()
            .find(|r| r.item.title() == Some("Desk login"))
            .unwrap()
            .item_id
    });
    desktop.with(|s| {
        s.update_item(vault_id, item_id, |e| e.set_title("Desk login v2"))
            .unwrap();
    });
    desktop.sync(&server);
    phone.core.sync_now().unwrap();
    let mut titles = titles(&phone);
    titles.sort();
    assert_eq!(
        titles,
        vec!["Desk login v2".to_owned(), "Phone login".to_owned()]
    );
    let revealed = phone
        .core
        .reveal(&key.vault_id, &key.item_id, "login.password")
        .unwrap();
    assert_eq!(revealed.expose_bytes(), b"phone-pw-9876");
}

/// The desktop changes the master password on the server.
fn change_password_on_desktop(server: &LoopbackServer, desktop: &Desktop) {
    let prepared = desktop
        .with(|s| {
            s.prepare_master_password_change(&pw(PASSWORD), &pw(NEW_PASSWORD), &desktop.secret_key)
        })
        .unwrap();
    server.advance_ms(60_000);
    hosted::password_change(
        &server.base,
        &desktop.email,
        &prepared,
        desktop.install,
        "Desktop",
        None,
    )
    .unwrap();
    desktop
        .with(|s| s.commit_master_password_change(prepared))
        .unwrap();
}

#[test]
fn sync_password_changed_elsewhere_then_recover() {
    let (server, desktop, phone) = world("recover@example.com");
    phone.core.sync_now().unwrap();
    change_password_on_desktop(&server, &desktop);
    assert_eq!(
        phone.core.sync_now().err(),
        Some(MobileError::SyncPasswordChanged)
    );
    let overview = phone.core.sync_overview();
    assert!(!overview.signed_in, "a revoked session token is deleted");
    assert_eq!(overview.last_error, Some(MobileError::SyncPasswordChanged));
    // The old password still unlocks the phone; the new one recovers.
    assert_eq!(
        phone.core.recover_password(Vec::new()).err(),
        Some(MobileError::EmptyPassword)
    );
    server.advance_ms(60_000);
    assert_eq!(
        phone
            .core
            .recover_password(b"not the new password".to_vec())
            .err(),
        Some(MobileError::WrongCredentials)
    );
    server.advance_ms(60_000);
    let overview = phone
        .core
        .recover_password(NEW_PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert!(overview.signed_in);
    assert_eq!(overview.last_error, None);
    // Syncing works again, and the new password is the one that unlocks.
    phone.core.sync_now().unwrap();
    phone.core.lock();
    assert_eq!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), None),
        Err(MobileError::WrongCredentials)
    );
    assert!(
        phone
            .core
            .unlock_password(NEW_PASSWORD.as_bytes().to_vec(), None)
            .is_ok()
    );
}

#[test]
fn recover_password_deletes_quick_unlock_and_marker() {
    let (server, desktop, phone) = world("quick-recover@example.com");
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![5; 113])
        .unwrap();
    change_password_on_desktop(&server, &desktop);
    server.advance_ms(60_000);
    phone
        .core
        .recover_password(NEW_PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert!(matches!(
        phone.core.state(),
        SessionState::Unlocked {
            quick_unlock_enrolled: false,
            ..
        }
    ));
    // The platform learns which key to delete from the state after the lock.
    phone.core.lock();
    match phone.core.state() {
        SessionState::Locked { quick_unlock, .. } => assert_eq!(
            quick_unlock,
            QuickUnlockAvailability::Expired {
                key_alias: setup.key_alias.clone()
            }
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        phone.core.unlock_quick(vec![0; 32]),
        Err(MobileError::PasswordRequired)
    );
    // The boot marker went with the wrapping: a new process of this boot
    // must ask for the password, and a password unlock records it again.
    phone.core.close_store();
    {
        let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
        assert!(store.boot_marker().unwrap().is_none());
    }
    phone.core.reopen_store().unwrap();
    phone
        .core
        .unlock_password(NEW_PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    phone.core.close_store();
    let store = mosaic_vault::storage::Store::open(phone.store_path()).unwrap();
    assert!(store.boot_marker().unwrap().is_some());
}

#[test]
fn rekey_elsewhere_reports_account_rekeyed() {
    let (server, desktop, phone) = world("rekey@example.com");
    phone.core.sync_now().unwrap();
    // The desktop re-keys the account (a new password and a new Secret Key).
    let vault = desktop.vault();
    let head = desktop.with(|s| {
        s.sync_cursor(vault)
            .unwrap()
            .and_then(|c| <[u8; 8]>::try_from(c).ok())
            .map_or(0, u64::from_be_bytes)
    });
    let new_sk = SecretKey::generate().unwrap();
    let prepared = desktop.with(|s| {
        s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
        s.prepare_rekey(RekeyRequest {
            current_password: &pw(PASSWORD),
            current_secret_key: &desktop.secret_key,
            new_password: &pw(NEW_PASSWORD),
            new_secret_key: &new_sk,
            profile: Argon2Profile::Baseline,
            plan: RekeyPlan {
                heads: vec![(vault, head)],
                ..RekeyPlan::default()
            },
        })
        .unwrap()
    });
    server.advance_ms(60_000);
    let outcome = hosted::rekey(
        &server.base,
        &desktop.email,
        &prepared,
        desktop.install,
        "Desktop",
        None,
        &mut |_| true,
    )
    .unwrap();
    assert!(matches!(outcome, RekeyOutcome::Done(_)), "{outcome:?}");
    assert_eq!(
        phone.core.sync_now().err(),
        Some(MobileError::SyncAccountRekeyed)
    );
    assert_eq!(
        phone.core.sync_overview().last_error,
        Some(MobileError::SyncAccountRekeyed)
    );
}

#[test]
fn reset_device_wipes_store_secrets_and_returns_aliases() {
    let (server, desktop, mut phone) = world("reset@example.com");
    phone.core.sync_now().unwrap();
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![5; 113])
        .unwrap();
    assert_eq!(phone.storage.names().len(), 2);
    let mut update = mosaic_mobile::SettingsUpdate::default();
    update.clipboard_clear_seconds = Some(20);
    phone.core.update_settings(update).unwrap();
    let settings_file = phone.dir.path().join("mosaic").join("settings.json");
    assert!(settings_file.exists());

    // The wrong email changes nothing.
    assert_eq!(
        phone
            .core
            .reset_device(ResetConfirm {
                typed_email: "someone@else.example".to_owned()
            })
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(phone.storage.names().len(), 2);
    assert!(matches!(phone.core.state(), SessionState::Locked { .. }));

    let outcome = phone
        .core
        .reset_device(ResetConfirm {
            typed_email: "  Reset@Example.com ".to_owned(),
        })
        .unwrap();
    assert_eq!(outcome.state, SessionState::NoAccount);
    assert_eq!(outcome.key_aliases, vec![setup.key_alias]);
    assert!(phone.storage.names().is_empty());
    assert!(!settings_file.exists());
    assert_eq!(
        phone.core.settings(),
        mosaic_mobile::MobileSettings::default()
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    assert_eq!(phone.core.sync_overview().configured, false);
    // The phone onboards again from scratch.
    phone.reopen_as(
        mosaic_mobile::Platform::Ios,
        mosaic_mobile::CoreContext::App,
        &crate::common::BOOT,
    );
    phone.join(&server, "reset@example.com", &desktop.sk_text());
    phone.core.sync_now().unwrap();
    assert_eq!(titles(&phone), vec!["Desk login".to_owned()]);
}

#[test]
fn lock_during_recover_keeps_the_alias_and_the_token() {
    // G1-13: a lock that arrives while the recovery runs wins the phase, but the
    // key alias to delete and the new token are not lost with it.
    let (server, desktop, phone) = world("recover-lock@example.com");
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![5; 113])
        .unwrap();
    change_password_on_desktop(&server, &desktop);
    server.advance_ms(60_000);
    phone.core.test_hooks().set_work_delay_ms(600);
    let result = std::thread::scope(|scope| {
        let recover = scope.spawn(|| {
            phone
                .core
                .recover_password(NEW_PASSWORD.as_bytes().to_vec())
        });
        let start = std::time::Instant::now();
        while phone.core.state() != SessionState::Busy {
            assert!(start.elapsed().as_secs() < 20, "never busy");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        phone.core.lock();
        recover.join().unwrap()
    });
    assert_eq!(result.err(), Some(MobileError::Locked));
    phone.core.test_hooks().set_work_delay_ms(0);
    match phone.core.state() {
        SessionState::Locked { quick_unlock, .. } => assert_eq!(
            quick_unlock,
            QuickUnlockAvailability::Expired {
                key_alias: setup.key_alias
            }
        ),
        other => panic!("{other:?}"),
    }
    assert!(
        phone.core.sync_overview().signed_in,
        "the new token is stored"
    );
}
