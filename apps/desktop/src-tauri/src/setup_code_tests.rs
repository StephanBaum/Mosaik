//! Tests of `setup_code_show` (M10 T13, ADR-0096): the re-authentication, the
//! hosted and folder forms, the 60 s expiry and that the rendered modules decode to
//! the account's setup code.

use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_core::SecretKey;
use mosaic_qr::{OwnedFrame, PixelLayout, scan_setup_frames};
use mosaic_sync::setup_code::{FolderProviderHint, ParsedSetupCode};
use mosaic_vault::session::{SyncConfig, SyncKind};
use zeroize::Zeroizing;

use crate::contract::errors::ErrorCode;
use crate::contract::secret::SecretIn;
use crate::contract::setup_code::{SetupCodeShowRequest, SetupCodeShown};
use crate::sync_handler_tests::{DEVICE, Env, PASSWORD};
use crate::test_support::kdf_permit;

const SERVER: &str = "https://sync.example.com";
const EMAIL: &str = "alex@example.com";

fn ask(password: &str) -> SetupCodeShowRequest {
    SetupCodeShowRequest {
        password: SecretIn::test(password),
    }
}

fn hosted(e: &Env) {
    e.b()
        .with_unlocked(|session| {
            session.set_sync_config(Some(&SyncConfig {
                kind: SyncKind::Hosted,
                server_url: Some(SERVER.to_owned()),
                email: Some(EMAIL.to_owned()),
                folder_path: None,
                device_name: DEVICE.to_owned(),
            }))?;
            Ok(())
        })
        .unwrap();
}

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

/// The modules as a camera would see them: black on white, four modules of
/// quiet zone, six pixels a module.
fn frame_of(shown: &SetupCodeShown) -> OwnedFrame {
    let (quiet, px) = (4usize, 6usize);
    let modules = usize::try_from(shown.size).unwrap();
    let side = (modules + 2 * quiet) * px;
    let dark = |x: usize, y: usize| {
        let (mx, my) = (x / px, y / px);
        mx >= quiet
            && my >= quiet
            && shown
                .modules
                .get(my - quiet)
                .and_then(|row| row.get(mx - quiet))
                .copied()
                .unwrap_or(false)
    };
    let pixels: Vec<u8> = (0..side * side)
        .map(|i| if dark(i % side, i / side) { 0 } else { 255 })
        .collect();
    let width = u32::try_from(side).unwrap();
    OwnedFrame::new(width, width, PixelLayout::Luma8, Zeroizing::new(pixels)).unwrap()
}

#[test]
fn setup_code_requires_reauth() {
    let _kdf = kdf_permit();
    let e = Env::new();
    // Locked.
    assert_eq!(
        e.b().setup_code_show(ask(PASSWORD)).unwrap_err().code,
        ErrorCode::Locked
    );
    e.create();
    hosted(&e);
    for (typed, expected) in [
        ("", ErrorCode::EmptyPassword),
        ("not the password", ErrorCode::WrongCredentials),
    ] {
        assert_eq!(
            e.b().setup_code_show(ask(typed)).unwrap_err().code,
            expected,
            "{typed:?}"
        );
    }
    e.b().setup_code_show(ask(PASSWORD)).unwrap();
    // The right password earns no two-minute grant for other commands.
    assert_eq!(
        e.b()
            .require_reauth(
                None,
                crate::contract::session::ReauthPurpose::QuickUnlockEnroll
            )
            .unwrap_err()
            .code,
        ErrorCode::ReauthRequired
    );
}

#[test]
fn setup_code_for_folder_account_carries_account_and_hint() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let kit_key = e.create();
    // No sync at all.
    assert_eq!(
        e.b().setup_code_show(ask(PASSWORD)).unwrap_err().code,
        ErrorCode::SetupCodeNeedsServer
    );
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    let account = e.b().with_unlocked(|s| Ok(s.account_id())).unwrap();
    let shown = e.b().setup_code_show(ask(PASSWORD)).unwrap();
    let frame = frame_of(&shown);
    let text = scan_setup_frames(&[frame.frame()]).unwrap();
    assert!(text.starts_with("mosaic-setup:2:"));
    let ParsedSetupCode::Folder(code) =
        mosaic_sync::setup_code::parse_any(text.as_bytes()).unwrap()
    else {
        panic!("expected a folder code");
    };
    assert_eq!(&code.account, account.as_bytes());
    assert_eq!(code.secret_key, SecretKey::parse(&kit_key).unwrap());
    // A temporary folder is under no provider client's root.
    assert_eq!(code.provider, FolderProviderHint::Other);
    assert_eq!(code.server, "");
    assert_eq!(code.path, "");
}

#[test]
fn setup_code_folder_still_requires_reauth() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    let dir = tempfile::tempdir().unwrap();
    e.connect_folder(dir.path());
    for (typed, expected) in [
        ("", ErrorCode::EmptyPassword),
        ("not the password", ErrorCode::WrongCredentials),
    ] {
        assert_eq!(
            e.b().setup_code_show(ask(typed)).unwrap_err().code,
            expected,
            "{typed:?}"
        );
    }
    e.b().setup_code_show(ask(PASSWORD)).unwrap();
}

#[test]
fn setup_code_expires_after_60_s() {
    let _kdf = kdf_permit();
    let e = Env::new();
    e.create();
    hosted(&e);
    let before = now_ms();
    let shown = e.b().setup_code_show(ask(PASSWORD)).unwrap();
    let after = now_ms();
    assert!((before + 60_000..=after + 60_000).contains(&shown.expires_at));
}

#[test]
fn setup_code_modules_decode_to_payload() {
    let _kdf = kdf_permit();
    let e = Env::new();
    let kit_key = e.create();
    hosted(&e);
    let shown = e.b().setup_code_show(ask(PASSWORD)).unwrap();
    let size = usize::try_from(shown.size).unwrap();
    assert_eq!(shown.modules.len(), size);
    assert!(shown.modules.iter().all(|row| row.len() == size));

    let frame = frame_of(&shown);
    let text = scan_setup_frames(&[frame.frame()]).unwrap();
    let code = mosaic_sync::setup_code::parse(text.as_bytes()).unwrap();
    assert_eq!(code.server, SERVER);
    assert_eq!(code.email, EMAIL);
    assert_eq!(code.secret_key, SecretKey::parse(&kit_key).unwrap());
}
