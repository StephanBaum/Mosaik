//! Tests of the M9 UI retargets that need a backend: the global shortcut
//! setting (M3-R10) and the revocation reason in the sync overview (M7-R3).

use std::sync::{Arc, Mutex};

use mosaic_sync::transport::RevokedReason;

use crate::contract::errors::ErrorCode;
use crate::contract::events::SyncStatus;
use crate::contract::settings::{Settings, UpdateSettingsRequest};
use crate::contract::sync::RevokedReasonView;
use crate::shortcut::{Accelerator, Registrar, ShortcutStatus};
use crate::sync_handler_tests::Env;
use crate::test_support::kdf_permit;

fn with_shortcut(b: &crate::state::Backend, text: &str) -> UpdateSettingsRequest {
    UpdateSettingsRequest {
        settings: Settings {
            global_shortcut: text.into(),
            ..b.settings_snapshot()
        },
        password: None,
    }
}

fn registrar(answer: ShortcutStatus, seen: &Arc<Mutex<Vec<Accelerator>>>) -> Registrar {
    let seen = Arc::clone(seen);
    Arc::new(move |accelerator| {
        seen.lock().unwrap().push(accelerator);
        answer
    })
}

#[test]
fn a_changed_global_shortcut_is_validated_and_registered() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let b = e.b();
    let seen = Arc::new(Mutex::new(Vec::new()));
    *b.shortcut_registrar.lock().unwrap() = Some(registrar(ShortcutStatus::Registered, &seen));
    *b.shortcut_status.lock().unwrap() = Some(ShortcutStatus::Registered);

    // Text that is no accelerator never reaches the registrar.
    for bad in ["", "Ctrl+", "Shift+K", "Ctrl+Banana"] {
        let error = b.update_settings(with_shortcut(b, bad)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{bad:?}");
    }
    assert!(seen.lock().unwrap().is_empty());

    let stored = b.update_settings(with_shortcut(b, "Ctrl+Alt+K")).unwrap();
    assert_eq!(stored.global_shortcut, "Ctrl+Alt+K");
    assert_eq!(b.settings_snapshot().global_shortcut, "Ctrl+Alt+K");
    assert_eq!(seen.lock().unwrap().len(), 1);

    // The same value again registers nothing.
    b.update_settings(with_shortcut(b, "Ctrl+Alt+K")).unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn a_shortcut_another_program_holds_is_shortcut_unavailable_and_not_stored() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let b = e.b();
    let before = b.settings_snapshot().global_shortcut;
    let seen = Arc::new(Mutex::new(Vec::new()));
    *b.shortcut_registrar.lock().unwrap() = Some(registrar(ShortcutStatus::Conflict, &seen));
    *b.shortcut_status.lock().unwrap() = Some(ShortcutStatus::Registered);
    let error = b
        .update_settings(with_shortcut(b, "Ctrl+Alt+K"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ShortcutUnavailable);
    assert_eq!(b.settings_snapshot().global_shortcut, before);
    assert_eq!(
        b.app_status().shortcut,
        crate::contract::session::ShortcutState::Conflict
    );
}

#[test]
fn on_wayland_the_shortcut_is_shortcut_unsupported() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let b = e.b();
    *b.shortcut_status.lock().unwrap() = Some(ShortcutStatus::Unsupported);
    let error = b
        .update_settings(with_shortcut(b, "Ctrl+Alt+K"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ShortcutUnsupported);
    assert_eq!(
        b.app_status().shortcut,
        crate::contract::session::ShortcutState::Unsupported
    );
    // Other settings still change.
    let mut same = b.settings_snapshot();
    same.rotation_reminders = !same.rotation_reminders;
    b.update_settings(UpdateSettingsRequest {
        settings: same,
        password: None,
    })
    .unwrap();
}

#[test]
fn the_revocation_reason_reaches_the_overview_until_the_next_sign_in() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let b = e.b();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    e.wait_synced();
    assert_eq!(e.status().revoked, None);

    b.sync_signed_out(b.sync.generation(), None, Some(RevokedReason::Rekey));
    let overview = e.status();
    assert_eq!(overview.state.status, SyncStatus::SignedOut);
    assert_eq!(overview.revoked, Some(RevokedReasonView::Rekey));

    // A sign-out without a reason (an old server) shows none.
    b.sync_signed_out(b.sync.generation(), None, None);
    assert_eq!(e.status().revoked, None);
    b.sync_signed_out(
        b.sync.generation(),
        None,
        Some(RevokedReason::DeviceRemoved),
    );
    assert_eq!(e.status().revoked, Some(RevokedReasonView::DeviceRemoved));
    b.sync_force_signed_out();
    assert_eq!(e.status().revoked, None);
}
