//! Unlocking: password, Secret Key, boot marker, extension context.

use mosaic_mobile::{
    CoreContext, ItemFilter, MobileError, Platform, QuickUnlockAvailability, QuickUnlockPlatform,
    ResetConfirm, SessionState, SettingsUpdate,
};
use mosaic_vault::storage::Store;

use crate::common::{BOOT, PASSWORD, Phone};

#[test]
fn unlock_password_uses_stored_secret_key() {
    let (phone, _) = Phone::with_account();
    phone.core.lock();
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            secret_key_stored: true,
            ..
        }
    ));
    let wrong = phone
        .core
        .unlock_password(b"not the password".to_vec(), None);
    assert_eq!(wrong, Err(MobileError::WrongCredentials));
    assert_eq!(
        phone.core.unlock_password(Vec::new(), None),
        Err(MobileError::EmptyPassword)
    );
    assert!(matches!(phone.core.state(), SessionState::Locked { .. }));
    let state = phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert!(matches!(
        state,
        SessionState::Unlocked {
            quick_unlock_enrolled: false,
            ..
        }
    ));
    assert!(phone.core.items(ItemFilter::default()).is_ok());
}

#[test]
fn unlock_password_records_boot_marker() {
    let (mut phone, _) = Phone::with_account();
    phone.core.close_store();
    {
        let store = Store::open(phone.store_path()).unwrap();
        let marker = store.boot_marker().unwrap().unwrap();
        assert_eq!(marker.boot_id, BOOT);
    }
    // A new boot: the marker is replaced by the next password unlock.
    phone.reopen_as(Platform::Ios, CoreContext::App, &[0, 0, 0, 8]);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    phone.core.close_store();
    let store = Store::open(phone.store_path()).unwrap();
    assert_eq!(store.boot_marker().unwrap().unwrap().boot_id, [0, 0, 0, 8]);
}

#[test]
fn wrong_secret_key_checksum_is_mistyped() {
    let (phone, sk) = Phone::with_account();
    phone.core.lock();
    // Change the first data character: the key changes, the checksum stays.
    let mut typed = sk.clone().into_bytes();
    typed[3] = if typed[3] == b'0' { b'2' } else { b'0' };
    assert_eq!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), Some(typed)),
        Err(MobileError::SecretKeyMistyped)
    );
    assert_eq!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), Some(b"not a key".to_vec())),
        Err(MobileError::SecretKeyMalformed)
    );
    // A typed Secret Key that is right works even when the phone holds none.
    phone.storage.map.lock().unwrap().clear();
    assert_eq!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), None),
        Err(MobileError::SecretKeyRequired)
    );
    assert!(
        phone
            .core
            .unlock_password(PASSWORD.as_bytes().to_vec(), Some(sk.into_bytes()))
            .is_ok()
    );
}

#[test]
fn extension_context_refuses_password_unlock_join_and_sync() {
    let (mut phone, _) = Phone::with_account();
    phone.reopen_as(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    let core = &phone.core;
    assert_eq!(
        core.unlock_password(PASSWORD.as_bytes().to_vec(), None),
        Err(MobileError::PasswordInApp)
    );
    assert_eq!(core.sync_now().err(), Some(MobileError::InvalidInput));
    assert_eq!(
        core.recover_password(b"new".to_vec()).err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.prelogin_check("https://example.com".to_owned()).err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.reset_device(ResetConfirm {
            typed_email: "user@example.com".to_owned()
        })
        .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![0; 113])
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.update_settings(SettingsUpdate::default()).err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        core.join_hosted(mosaic_mobile::JoinHostedRequest {
            server_url: "https://example.com".to_owned(),
            email: "a@example.com".to_owned(),
            password: b"x".to_vec(),
            secret_key: b"y".to_vec(),
            device_name: None,
        })
        .err(),
        Some(MobileError::InvalidInput)
    );
    // The state is still plainly locked.
    assert!(matches!(core.state(), SessionState::Locked { .. }));
}

fn enroll_ios(phone: &Phone) -> (String, Vec<u8>) {
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    let w = setup.w.expose_bytes();
    let enrolled = phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![9; 113])
        .unwrap();
    assert_eq!(enrolled.replaced_key_alias, None);
    (setup.key_alias, w)
}

#[test]
fn extension_quick_unlock_after_app_password_unlock_same_boot() {
    let (mut phone, _) = Phone::with_account();
    let (alias, w) = enroll_ios(&phone);
    phone.core.lock();
    // The extension is a new process of the same boot.
    phone.reopen_as(Platform::Ios, CoreContext::IosAutofillExtension, &BOOT);
    match phone.core.state() {
        SessionState::Locked { quick_unlock, .. } => match quick_unlock {
            QuickUnlockAvailability::Available {
                key_alias,
                body,
                aad,
                ..
            } => {
                assert_eq!(key_alias, alias);
                assert_eq!(body, vec![9; 113]);
                assert!(aad.is_empty());
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    let state = phone.core.unlock_quick(w).unwrap();
    assert!(matches!(
        state,
        SessionState::Unlocked {
            quick_unlock_enrolled: true,
            ..
        }
    ));
    assert!(phone.core.items(ItemFilter::default()).is_ok());
}

#[test]
fn quick_unlock_refused_after_reboot() {
    let (mut phone, _) = Phone::with_account();
    let (_, w) = enroll_ios(&phone);
    phone.core.lock();
    // The device booted again: no marker matches.
    phone.reopen_as(
        Platform::Ios,
        CoreContext::IosAutofillExtension,
        &[0, 0, 0, 9],
    );
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::PasswordRequired,
            ..
        }
    ));
    assert_eq!(
        phone.core.unlock_quick(w),
        Err(MobileError::PasswordRequired)
    );
    // An empty boot ID fails safe the same way.
    phone.reopen_as(Platform::Ios, CoreContext::IosAutofillExtension, &[]);
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::PasswordRequired,
            ..
        }
    ));
}

#[test]
fn required_unlock_memory_matches_profile_plus_margin() {
    let (phone, _) = Phone::with_account();
    // Baseline: 128 MiB, plus the 64 MiB margin.
    let expected = (128 + 64) * 1024 * 1024;
    assert_eq!(phone.core.required_unlock_memory_bytes(), Some(expected));
    phone.core.lock();
    assert_eq!(phone.core.required_unlock_memory_bytes(), Some(expected));
}
