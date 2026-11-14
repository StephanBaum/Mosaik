//! Quick unlock: the enrolment flow and the platform rows.

use mosaic_mobile::quick_unlock::params::quick_unlock_aad;
use mosaic_mobile::{
    MobileError, Platform, QuickUnlockAvailability, QuickUnlockPlatform, SessionState,
};

use crate::common::{PASSWORD, Phone};

fn android_body(strongbox: bool) -> Vec<u8> {
    let mut body = vec![3u8; 61];
    body[0] = u8::from(strongbox);
    body
}

#[test]
fn quick_unlock_enroll_requires_password_and_returns_aad() {
    let mut phone = Phone::android();
    phone
        .core
        .test_hooks()
        .create_local_account("user@example.com", PASSWORD)
        .unwrap();
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_begin(b"wrong password".to_vec())
            .err(),
        Some(MobileError::WrongCredentials)
    );
    assert_eq!(
        phone.core.quick_unlock_enroll_begin(Vec::new()).err(),
        Some(MobileError::EmptyPassword)
    );
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert_eq!(setup.w.len(), 32);
    assert_eq!(setup.key_nonce.len(), 16);
    assert_eq!(
        setup.key_alias,
        format!(
            "mosaic-quick-unlock-{}",
            setup
                .key_nonce
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    );
    let nonce: [u8; 16] = setup.key_nonce.clone().try_into().unwrap();
    assert_eq!(setup.aad, quick_unlock_aad(&nonce));
    // A second begin draws a new W and nonce and replaces the first.
    let again = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert_ne!(again.key_nonce, setup.key_nonce);
    assert_ne!(again.w.expose_bytes(), setup.w.expose_bytes());
    // The wrong platform and the wrong body fit nothing.
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![0; 113])
            .err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(
                QuickUnlockPlatform::AndroidKeystore { strongbox: false },
                vec![0; 60]
            )
            .err(),
        Some(MobileError::InvalidInput)
    );
    // A wrong body does not spend the pending enrolment.
    let enrolled = phone
        .core
        .quick_unlock_enroll_finish(
            QuickUnlockPlatform::AndroidKeystore { strongbox: true },
            android_body(true),
        )
        .unwrap();
    assert_eq!(enrolled.replaced_key_alias, None);
    assert!(matches!(
        phone.core.state(),
        SessionState::Unlocked {
            quick_unlock_enrolled: true,
            ..
        }
    ));
    // The enrolment survives a lock and carries the AAD for the unlock.
    phone.core.lock();
    phone.reopen_as(
        Platform::Android,
        mosaic_mobile::CoreContext::App,
        &crate::common::BOOT,
    );
    match phone.core.state() {
        SessionState::Locked {
            quick_unlock:
                QuickUnlockAvailability::Available {
                    platform,
                    key_alias,
                    aad,
                    body,
                },
            ..
        } => {
            assert_eq!(
                platform,
                QuickUnlockPlatform::AndroidKeystore { strongbox: true }
            );
            assert_eq!(key_alias, again.key_alias);
            assert_eq!(
                aad,
                quick_unlock_aad(&again.key_nonce.clone().try_into().unwrap())
            );
            assert_eq!(body, android_body(true));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn quick_unlock_pending_slot_expires_and_is_dropped_on_lock() {
    let (phone, _) = Phone::with_account();
    let ios = QuickUnlockPlatform::IosSecureEnclave;
    // Expires after 120 seconds.
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert!(setup.expires_at_ms > 0);
    phone.core.test_hooks().set_clock_offset_ms(121_000);
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(ios, vec![1; 113])
            .err(),
        Some(MobileError::ReauthRequired)
    );
    // And the slot is gone: a finish without a begin is refused too.
    phone.core.test_hooks().set_clock_offset_ms(0);
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(ios, vec![1; 113])
            .err(),
        Some(MobileError::ReauthRequired)
    );
    // A lock drops the slot.
    phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone.core.lock();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(ios, vec![1; 113])
            .err(),
        Some(MobileError::ReauthRequired)
    );
}

/// A failed platform half cancels the pending enrolment at once: the slot (and
/// the `W` it holds) is gone, so a finish is refused, and a cancel with nothing
/// pending is harmless.
#[test]
fn quick_unlock_enroll_cancel_drops_the_pending_slot() {
    let (phone, _) = Phone::with_account();
    let ios = QuickUnlockPlatform::IosSecureEnclave;
    phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone.core.quick_unlock_enroll_cancel();
    assert_eq!(
        phone
            .core
            .quick_unlock_enroll_finish(ios, vec![1; 113])
            .err(),
        Some(MobileError::ReauthRequired)
    );
    // Nothing pending: a no-op.
    phone.core.quick_unlock_enroll_cancel();
}

#[test]
fn quick_unlock_platform_failed_deletes_wrapping_and_returns_alias() {
    let (phone, _) = Phone::with_account();
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![7; 113])
        .unwrap();
    phone.core.lock();
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::Available { .. },
            ..
        }
    ));
    // The OS invalidated the key (a new fingerprint, say).
    assert_eq!(
        phone.core.quick_unlock_platform_failed().unwrap(),
        Some(setup.key_alias)
    );
    assert!(matches!(
        phone.core.state(),
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::NotEnrolled,
            ..
        }
    ));
    assert_eq!(
        phone.core.unlock_quick(vec![0; 32]),
        Err(MobileError::PasswordRequired)
    );
    // Nothing left to delete.
    assert_eq!(phone.core.quick_unlock_platform_failed().unwrap(), None);
}

#[test]
fn quick_unlock_replaced_returns_old_alias() {
    let (phone, _) = Phone::with_account();
    let first = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![1; 113])
        .unwrap();
    let second = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    let enrolled = phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![2; 113])
        .unwrap();
    assert_eq!(enrolled.replaced_key_alias, Some(first.key_alias));
    assert_ne!(second.key_alias, enrolled.replaced_key_alias.unwrap());
    // Removing returns the live enrolment's key.
    assert_eq!(
        phone.core.quick_unlock_remove().unwrap(),
        Some(second.key_alias)
    );
    assert_eq!(phone.core.quick_unlock_remove().unwrap(), None);
}

#[test]
fn quick_unlock_expiry_reports_the_key_to_delete() {
    let (phone, _) = Phone::with_account();
    let setup = phone
        .core
        .quick_unlock_enroll_begin(PASSWORD.as_bytes().to_vec())
        .unwrap();
    phone
        .core
        .quick_unlock_enroll_finish(QuickUnlockPlatform::IosSecureEnclave, vec![1; 113])
        .unwrap();
    phone.core.lock();
    // 15 days later (the default maximum age is 14).
    phone
        .core
        .test_hooks()
        .set_clock_offset_ms(15 * 24 * 3600 * 1000);
    match phone.core.state() {
        SessionState::Locked {
            quick_unlock: QuickUnlockAvailability::Expired { key_alias },
            ..
        } => assert_eq!(key_alias, setup.key_alias),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        phone.core.quick_unlock_remove().unwrap(),
        Some(setup.key_alias)
    );
}
