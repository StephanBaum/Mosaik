//! The settings file and its integrity.

use mosaic_mobile::{CoreContext, MobileError, MobileSettings, Platform, SettingsUpdate};

use crate::common::{BOOT, PASSWORD, Phone};

fn update() -> SettingsUpdate {
    SettingsUpdate::default()
}

#[test]
fn settings_ranges_are_clamped() {
    let (phone, _) = Phone::with_account();
    assert_eq!(phone.core.settings(), MobileSettings::default());
    let mut u = update();
    u.auto_lock_background_seconds = Some(45); // down to the option 30
    u.idle_lock_minutes = Some(999);
    u.clipboard_clear_seconds = Some(1);
    let s = phone.core.update_settings(u).unwrap();
    assert_eq!(s.auto_lock_background_seconds, 30);
    assert_eq!(s.idle_lock_minutes, 30);
    assert_eq!(s.clipboard_clear_seconds, 10);
    assert_eq!(s.quick_unlock_max_age_days, 14);
    // A longer quick-unlock age and the password identity store need the
    // master password.
    let mut u = update();
    u.quick_unlock_max_age_days = Some(99);
    assert_eq!(
        phone.core.update_settings(u).err(),
        Some(MobileError::ReauthRequired)
    );
    let mut u = update();
    u.quick_unlock_max_age_days = Some(99);
    u.reauth_password = Some(b"wrong".to_vec());
    assert_eq!(
        phone.core.update_settings(u).err(),
        Some(MobileError::WrongCredentials)
    );
    let mut u = update();
    u.quick_unlock_max_age_days = Some(99);
    u.password_identities = Some(true);
    u.reauth_password = Some(PASSWORD.as_bytes().to_vec());
    let s = phone.core.update_settings(u).unwrap();
    assert_eq!(s.quick_unlock_max_age_days, 30);
    assert!(s.password_identities);
    // A shorter one needs nothing, and 0 is not an option for the idle lock.
    let mut u = update();
    u.quick_unlock_max_age_days = Some(0);
    u.idle_lock_minutes = Some(0);
    let s = phone.core.update_settings(u).unwrap();
    assert_eq!(s.quick_unlock_max_age_days, 1);
    assert_eq!(s.idle_lock_minutes, 1);
    // Locked: no changes.
    phone.core.lock();
    assert_eq!(
        phone.core.update_settings(update()).err(),
        Some(MobileError::Locked)
    );
}

#[test]
fn settings_mac_rejects_tampered_file() {
    let (mut phone, _) = Phone::with_account();
    let mut u = update();
    u.idle_lock_minutes = Some(20);
    u.clipboard_clear_seconds = Some(20);
    phone.core.update_settings(u).unwrap();
    phone.core.close_store();
    let path = phone.dir.path().join("mosaic").join("settings.json");
    let original = std::fs::read_to_string(&path).unwrap();

    // The honest file: while locked only the strict values apply, after the
    // unlock the stored ones.
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    assert_eq!(phone.core.settings().idle_lock_minutes, 5);
    assert_eq!(phone.core.settings().clipboard_clear_seconds, 20);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(phone.core.settings().idle_lock_minutes, 20);
    assert_eq!(phone.core.settings().clipboard_clear_seconds, 20);
    phone.core.close_store();

    // A tampered file (longer clipboard time, MAC kept): rejected.
    let tampered = original.replace(
        "\"clipboardClearSeconds\": 20",
        "\"clipboardClearSeconds\": 300",
    );
    assert_ne!(tampered, original);
    std::fs::write(&path, tampered).unwrap();
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    let s = phone.core.settings();
    assert_eq!(
        s.clipboard_clear_seconds, 30,
        "the strict value, not the tampered one"
    );
    assert_eq!(
        s.idle_lock_minutes, 5,
        "the strict value, not the stored one"
    );
    // The file was rewritten with a fresh nonce and MAC: the next run agrees.
    phone.core.close_store();
    let rewritten = std::fs::read_to_string(&path).unwrap();
    assert_ne!(rewritten, original);
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(phone.core.settings().clipboard_clear_seconds, 30);

    // An older copy of the file (a restored backup) fails too: the nonce moved.
    phone
        .core
        .update_settings({
            let mut u = update();
            u.idle_lock_minutes = Some(10);
            u
        })
        .unwrap();
    phone.core.close_store();
    std::fs::write(&path, original).unwrap();
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(phone.core.settings().idle_lock_minutes, 5);
    // Garbage is ignored: the defaults.
    phone.core.close_store();
    std::fs::write(&path, b"{ not json").unwrap();
    phone.reopen_as(Platform::Ios, CoreContext::App, &BOOT);
    assert_eq!(phone.core.settings(), MobileSettings::default());
}
