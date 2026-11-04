//! Tests of the one-time-code setup commands (ADR-0031) on an in-memory
//! store with a fake screen, an in-memory clipboard and a fixed file dialog.
//! Each test that creates an account runs Argon2id, so it holds the KDF permit.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use mosaic_core::Argon2Profile;
use mosaic_qr::testing::{SAMPLE_URI, canvas, jpeg, png, qr_frame, rgba};
use mosaic_qr::{OwnedFrame, PixelLayout};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::clipboard::MemoryClipboard;
use crate::commands::otp::check_clipboard_pixels;
use crate::contract::errors::{ErrorCode, IpcResult};
use crate::contract::events::LockReason;
use crate::contract::items::{ItemDetail, OtpChange, OtpScheme};
use crate::contract::otp::{
    OtpAlgorithm, OtpParseRequest, OtpPending, OtpPendingRequest, OtpScanRequest, OtpScanResult,
    OtpScanSource, OtpSecretStrength,
};
use crate::contract::secret::SecretIn;
use crate::contract::session::{CreateAccountRequest, UnlockRequest};
use crate::dialogs::{Dialogs, FileFilter};
use crate::keystore::MemoryKeystore;
use crate::state::{Backend, Slot};
use crate::test_support::FakeScreenCapture;

const PASSWORD: &str = "correct horse battery staple";
const SECRET: &str = "JBSWY3DPEHPK3PXP";
const OTHER_URI: &str =
    "otpauth://totp/Other:bob?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=Other";

/// Answers every dialog with the current path (or a cancel).
#[derive(Clone, Default)]
struct PathDialog(Arc<Mutex<Option<PathBuf>>>);

impl PathDialog {
    fn set(&self, path: Option<PathBuf>) {
        *self.0.lock().unwrap() = path;
    }
}

impl Dialogs for PathDialog {
    fn save_file(&self, _: &str, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(None)
    }
    fn open_file(&self, _: FileFilter) -> IpcResult<Option<PathBuf>> {
        Ok(self.0.lock().unwrap().clone())
    }
}

struct Rig {
    backend: Arc<Backend>,
    screen: FakeScreenCapture,
    clipboard: MemoryClipboard,
    dialog: PathDialog,
}

fn rig_with(supported: bool, frames: Vec<OwnedFrame>) -> Rig {
    let screen = FakeScreenCapture::new(supported, frames);
    let clipboard = MemoryClipboard::default();
    let dialog = PathDialog::default();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_clipboard(clipboard.factory())
    .with_dialogs(Box::new(dialog.clone()))
    .with_screen_capture(Box::new(screen.clone()));
    backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    Rig {
        backend: Arc::new(backend),
        screen,
        clipboard,
        dialog,
    }
}

fn rig() -> Rig {
    rig_with(true, Vec::new())
}

fn unlock(b: &Backend) {
    b.unlock(UnlockRequest {
        password: SecretIn::test(PASSWORD),
        secret_key: None,
    })
    .unwrap();
}

fn typed<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn scan(b: &Backend, source: OtpScanSource) -> IpcResult<OtpScanResult> {
    b.otp_scan(&OtpScanRequest { source })
}

fn found(result: IpcResult<OtpScanResult>) -> OtpPending {
    match result.unwrap() {
        OtpScanResult::Found { pending, .. } => pending,
        OtpScanResult::Cancelled => panic!("cancelled"),
    }
}

fn scan_code(b: &Backend, source: OtpScanSource) -> ErrorCode {
    scan(b, source).unwrap_err().code
}

/// A screen showing the given codes side by side.
fn screen_of(codes: &[&str]) -> Vec<OwnedFrame> {
    let frames: Vec<OwnedFrame> = codes.iter().map(|c| qr_frame(c, 5, 4)).collect();
    let place: Vec<(&OwnedFrame, u32, u32)> = frames
        .iter()
        .enumerate()
        .map(|(i, f)| (f, 40 + 480 * u32::try_from(i).unwrap(), 60))
        .collect();
    vec![canvas(1400, 640, PixelLayout::Rgbx8, 240, &place)]
}

fn pending_id(pending: &OtpPending) -> String {
    pending.pending_id.clone()
}

fn pending_req(id: &str) -> OtpPendingRequest {
    OtpPendingRequest {
        pending_id: id.to_owned(),
    }
}

fn draft(title: &str, otp: &Value) -> Value {
    json!({
        "text": [{ "field": "title", "value": title }],
        "secrets": [], "urls": [], "customFields": [], "tags": null,
        "otp": otp,
    })
}

fn set_pending(id: &str, issuer: Option<&str>, account: Option<&str>) -> Value {
    json!({ "op": "setPending", "pendingId": id, "issuer": issuer, "accountName": account })
}

fn add_login(b: &Backend, otp: &Value) -> IpcResult<ItemDetail> {
    let vault = b.list_vaults().unwrap()[0].vault_id.0.to_string();
    b.add_item(typed(
        json!({ "vaultId": vault, "kind": "login", "draft": draft("Example", otp) }),
    ))
}

fn totp_now(uri: &str) -> String {
    let auth = mosaic_totp::OtpAuth::parse(uri).unwrap();
    let now = mosaic_totp::unix_time_now().unwrap();
    auth.otp.code_at(now).unwrap().expose_secret().to_owned()
}

#[test]
fn scanning_the_screen_holds_the_setup_in_rust_and_maps_the_errors() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig_with(true, screen_of(&[SAMPLE_URI]));
    let b = &r.backend;
    assert!(b.app_status().otp_screen_scan, "the fake reports support");

    let result = scan(b, OtpScanSource::Screen);
    let text = serde_json::to_string(&result.as_ref().unwrap()).unwrap();
    assert!(!text.contains(SECRET) && !text.to_lowercase().contains("jbswy3dp"));
    let pending = found(result);
    assert_eq!(pending.scheme, OtpScheme::Totp);
    assert_eq!(pending.issuer.as_deref(), Some("Example"));
    assert_eq!(pending.account_name.as_deref(), Some("alice@example.com"));
    assert_eq!(pending.algorithm, OtpAlgorithm::Sha1);
    assert_eq!((pending.digits, pending.period), (6, Some(30)));
    assert_eq!(pending.strength, OtpSecretStrength::BelowRfc4226);
    assert_eq!(pending.expires_in_seconds, 600);
    assert_eq!(pending.pending_id.len(), 32, "16 random bytes in hex");
    assert!(pending.pending_id.bytes().all(|c| c.is_ascii_hexdigit()));
    assert!(!b.otp_scanning.load(Ordering::SeqCst), "flag released");

    // Errors of the scan itself.
    let cases = [
        (screen_of(&["https://example.com"]), ErrorCode::OtpQrNotOtp),
        (
            screen_of(&["otpauth-migration://offline?data=CgYKBAAAAAA%3D"]),
            ErrorCode::OtpQrMigration,
        ),
        (
            screen_of(&[SAMPLE_URI, OTHER_URI]),
            ErrorCode::OtpQrMultiple,
        ),
        (screen_of(&[]), ErrorCode::OtpQrNotFound),
        (
            screen_of(&["otpauth://totp/x?secret=1"]),
            ErrorCode::OtpQrInvalid,
        ),
    ];
    for (frames, expected) in cases {
        r.screen.load(frames);
        assert_eq!(scan_code(b, OtpScanSource::Screen), expected);
        assert!(!b.otp_scanning.load(Ordering::SeqCst), "flag released");
    }

    // One scan at a time.
    b.otp_scanning.store(true, Ordering::SeqCst);
    assert_eq!(scan_code(b, OtpScanSource::Screen), ErrorCode::Busy);
    b.otp_scanning.store(false, Ordering::SeqCst);
}

#[test]
fn a_scan_needs_an_unlocked_vault_and_is_dropped_if_it_locks_meanwhile() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig_with(true, screen_of(&[SAMPLE_URI]));
    let b = &r.backend;
    b.lock(LockReason::Manual).unwrap();
    assert_eq!(scan_code(b, OtpScanSource::Screen), ErrorCode::Locked);
    assert_eq!(r.screen.calls(), 0, "nothing was captured");
    unlock(b);

    let locker = Arc::clone(b);
    r.screen.on_capture(Box::new(move || {
        locker.lock(LockReason::Manual).unwrap();
    }));
    r.screen.load(screen_of(&[SAMPLE_URI]));
    assert_eq!(scan_code(b, OtpScanSource::Screen), ErrorCode::Locked);
    assert_eq!(r.screen.calls(), 1);
    unlock(b);
    assert_eq!(b.otp_pending.lock().unwrap().len(), 0, "nothing was kept");
}

#[test]
fn a_system_without_screen_capture_says_so_without_capturing() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig_with(false, screen_of(&[SAMPLE_URI]));
    assert!(!r.backend.app_status().otp_screen_scan);
    assert_eq!(
        scan_code(&r.backend, OtpScanSource::Screen),
        ErrorCode::OtpScanUnsupported
    );
    assert_eq!(r.screen.calls(), 0);
}

#[test]
fn a_clipboard_screenshot_is_read_and_the_clipboard_cleared() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    assert_eq!(
        scan_code(b, OtpScanSource::Clipboard),
        ErrorCode::OtpClipboardNoImage
    );
    r.clipboard.put("something copied");
    r.clipboard.put_image(rgba(&qr_frame(SAMPLE_URI, 5, 4)));
    match scan(b, OtpScanSource::Clipboard).unwrap() {
        OtpScanResult::Found {
            pending,
            clipboard_cleared,
        } => {
            assert!(clipboard_cleared);
            assert_eq!(pending.issuer.as_deref(), Some("Example"));
        }
        OtpScanResult::Cancelled => panic!("cancelled"),
    }
    assert!(r.clipboard.content().is_none() && !r.clipboard.has_image());

    // A failed read leaves the clipboard alone.
    r.clipboard
        .put_image(rgba(&qr_frame("https://example.com", 5, 4)));
    assert_eq!(
        scan_code(b, OtpScanSource::Clipboard),
        ErrorCode::OtpQrNotOtp
    );
}

#[test]
fn the_clipboard_image_has_a_pixel_limit() {
    assert!(check_clipboard_pixels(6324, 6324).is_ok());
    assert_eq!(
        check_clipboard_pixels(6325, 6325).unwrap_err().code,
        ErrorCode::OtpImageTooLarge
    );
}

#[test]
fn image_files_are_read_within_the_limits() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    };
    let qr = qr_frame(SAMPLE_URI, 5, 4);

    r.dialog.set(Some(write("code.png", &png(&qr))));
    assert_eq!(
        found(scan(b, OtpScanSource::File)).issuer.as_deref(),
        Some("Example")
    );
    r.dialog.set(Some(write("code.jpg", &jpeg(&qr, 80))));
    assert_eq!(
        found(scan(b, OtpScanSource::File)).issuer.as_deref(),
        Some("Example")
    );

    r.dialog.set(None);
    assert!(matches!(
        scan(b, OtpScanSource::File).unwrap(),
        OtpScanResult::Cancelled
    ));

    r.dialog.set(Some(write("notes.png", b"just some text")));
    assert_eq!(
        scan_code(b, OtpScanSource::File),
        ErrorCode::OtpImageUnreadable
    );

    // Over 20 MiB: refused from the file size, nothing is read.
    let big = dir.path().join("big.png");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(20 * 1024 * 1024 + 1).unwrap();
    r.dialog.set(Some(big));
    assert_eq!(
        scan_code(b, OtpScanSource::File),
        ErrorCode::OtpImageTooLarge
    );

    r.dialog.set(Some(dir.path().join("missing.png")));
    assert_eq!(
        scan_code(b, OtpScanSource::File),
        ErrorCode::OtpImageUnreadable
    );
}

fn parse_uri(b: &Backend, uri: &str) -> IpcResult<OtpPending> {
    b.otp_parse(OtpParseRequest::Uri {
        uri: SecretIn::test(uri),
    })
}

fn parse_key(
    b: &Backend,
    secret: &str,
    account: Option<&str>,
    digits: u8,
    period: u32,
) -> IpcResult<OtpPending> {
    b.otp_parse(OtpParseRequest::Key {
        secret: SecretIn::test(secret),
        issuer: Some("GitHub".into()),
        account_name: account.map(str::to_owned),
        algorithm: OtpAlgorithm::Sha256,
        digits,
        period,
    })
}

#[test]
fn manual_entry_parses_links_and_keys() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;

    let ok = parse_uri(b, &format!("  {SAMPLE_URI}\n")).unwrap();
    assert_eq!(ok.issuer.as_deref(), Some("Example"));
    assert_eq!(
        parse_uri(b, "otpauth-migration://offline?data=CgYKBAAAAAA%3D")
            .unwrap_err()
            .code,
        ErrorCode::OtpQrMigration
    );
    assert_eq!(
        parse_uri(b, "https://example.com").unwrap_err().code,
        ErrorCode::OtpUriInvalid
    );
    assert_eq!(
        parse_uri(b, "otpauth://totp/x?secret=1").unwrap_err().code,
        ErrorCode::OtpUriInvalid
    );

    let key = parse_key(b, "jbsw y3dp-ehpk 3pxp", Some("alex"), 8, 60).unwrap();
    assert_eq!(key.scheme, OtpScheme::Totp);
    assert_eq!(key.issuer.as_deref(), Some("GitHub"));
    assert_eq!(key.account_name.as_deref(), Some("alex"));
    assert_eq!(key.algorithm, OtpAlgorithm::Sha256);
    assert_eq!((key.digits, key.period), (8, Some(60)));
    assert_eq!(
        parse_key(b, "0189", None, 6, 30).unwrap_err().code,
        ErrorCode::OtpKeyInvalid
    );
    assert_eq!(
        parse_key(b, SECRET, None, 9, 30).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        parse_key(b, SECRET, None, 6, 301).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        parse_key(b, SECRET, Some("al\u{202e}ex"), 6, 30)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn pending_codes_are_live_discardable_and_gone_after_a_lock() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    let totp = parse_uri(b, SAMPLE_URI).unwrap();
    let hotp = parse_uri(
        b,
        "otpauth://hotp/Example:bob?secret=JBSWY3DPEHPK3PXP&counter=7",
    )
    .unwrap();
    assert_eq!(hotp.scheme, OtpScheme::Hotp);
    assert_eq!(hotp.period, None);

    let before = totp_now(SAMPLE_URI);
    let code = b.otp_pending_code(&pending_req(&totp.pending_id)).unwrap();
    let after = totp_now(SAMPLE_URI);
    let shown = serde_json::to_value(&code).unwrap();
    let text = shown["code"].as_str().unwrap();
    assert!(text == before || text == after, "{text}");
    assert_eq!(shown["period"], 30);

    assert_eq!(
        b.otp_pending_code(&pending_req(&hotp.pending_id))
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput,
        "an HOTP counter must not move before saving"
    );
    assert_eq!(
        b.otp_pending_code(&pending_req("otp-999"))
            .unwrap_err()
            .code,
        ErrorCode::OtpPendingExpired
    );

    b.otp_pending_discard(&pending_req(&totp.pending_id))
        .unwrap();
    b.otp_pending_discard(&pending_req("otp-999")).unwrap();
    assert_eq!(
        b.otp_pending_code(&pending_req(&totp.pending_id))
            .unwrap_err()
            .code,
        ErrorCode::OtpPendingExpired
    );

    b.lock(LockReason::Manual).unwrap();
    assert_eq!(b.otp_pending.lock().unwrap().len(), 0);
    b.otp_pending_discard(&pending_req(&hotp.pending_id))
        .unwrap();
    assert_eq!(
        b.otp_pending_code(&pending_req(&hotp.pending_id))
            .unwrap_err()
            .code,
        ErrorCode::Locked
    );
}

#[test]
fn saving_a_pending_setup_applies_the_edited_labels_once() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    let first = pending_id(&parse_uri(b, SAMPLE_URI).unwrap());

    let detail = add_login(b, &set_pending(&first, Some("Renamed"), Some("me"))).unwrap();
    let otp = detail.otp.as_ref().unwrap();
    assert_eq!(otp.issuer.as_deref(), Some("Renamed"));
    assert_eq!((otp.digits, otp.period), (6, Some(30)));
    assert!(detail.summary.has_otp);
    assert_eq!(
        add_login(b, &set_pending(&first, None, None))
            .unwrap_err()
            .code,
        ErrorCode::OtpPendingExpired,
        "a slot is used once"
    );

    // A draft the model refuses does not use the slot up.
    let second = pending_id(&parse_uri(b, SAMPLE_URI).unwrap());
    let vault = b.list_vaults().unwrap()[0].vault_id.0.to_string();
    let mut bad = draft("Bad", &set_pending(&second, None, None));
    bad["urls"] =
        json!([{ "op": "add", "url": "  ", "matchMode": "exactOrigin", "allowHttp": false }]);
    let error = b
        .add_item(typed(
            json!({ "vaultId": vault, "kind": "login", "draft": bad }),
        ))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    b.otp_pending_code(&pending_req(&second)).unwrap();

    // A bad label takes nothing either.
    let error = add_login(b, &set_pending(&second, Some("bad\u{7}"), None)).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    b.otp_pending_code(&pending_req(&second)).unwrap();

    // update_item replaces an existing code; remove still clears it.
    let item = json!({
        "vaultId": vault,
        "itemId": detail.summary.item_id.0.to_string(),
    });
    let mut update = item.clone();
    update["draft"] = draft("Example", &set_pending(&second, Some("Third"), None));
    let updated = b.update_item(typed(update)).unwrap();
    assert_eq!(
        updated.otp.as_ref().unwrap().issuer.as_deref(),
        Some("Third")
    );
    let mut remove = item;
    remove["draft"] = draft("Example", &json!({ "op": "remove" }));
    assert!(b.update_item(typed(remove)).unwrap().otp.is_none());
}

#[test]
fn a_pending_code_can_be_copied_from_rust() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    let pending = parse_uri(b, SAMPLE_URI).unwrap();
    let before = totp_now(SAMPLE_URI);
    b.copy_to_clipboard(typed(
        json!({ "source": { "kind": "pendingOtp", "pendingId": pending.pending_id } }),
    ))
    .unwrap();
    let copied = r.clipboard.content().unwrap();
    assert!(copied == before || copied == totp_now(SAMPLE_URI));
    assert_eq!(copied.len(), 6);
    assert_eq!(
        b.copy_to_clipboard(typed(
            json!({ "source": { "kind": "pendingOtp", "pendingId": "otp-999" } }),
        ))
        .unwrap_err()
        .code,
        ErrorCode::OtpPendingExpired
    );
}

#[test]
fn a_pending_setup_becomes_a_hardware_protected_item() {
    use crate::quick_unlock::QuickUnlockService;
    let _kdf = crate::test_support::kdf_permit();
    let key = crate::quick_unlock::fakes::FakeHardwareKey::new(1);
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_quick_unlock(QuickUnlockService::new(None, Box::new(Arc::clone(&key))));
    backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    let vault = backend
        .create_hardware_vault(&mut typed(json!({ "name": "Codes", "pin": null })))
        .unwrap()
        .vault_id
        .0
        .to_string();
    let id = pending_id(&parse_uri(&backend, SAMPLE_URI).unwrap());
    let detail = backend
        .add_item(typed(json!({
            "vaultId": vault,
            "kind": "totp",
            "draft": draft("Codes", &set_pending(&id, Some("Example"), None)),
        })))
        .unwrap();
    assert!(detail.otp.unwrap().hardware_protected);
}

#[test]
fn a_clipboard_image_with_a_secret_is_cleared_even_when_the_scan_fails() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    // Migration, several codes and an invalid code all hold secrets.
    let cases = [
        (
            "otpauth-migration://offline?data=CgYKBAAAAAA%3D",
            ErrorCode::OtpQrMigration,
        ),
        ("otpauth://totp/x?secret=1", ErrorCode::OtpQrInvalid),
    ];
    for (uri, expected) in cases {
        r.clipboard.put_image(rgba(&qr_frame(uri, 5, 4)));
        assert_eq!(scan_code(b, OtpScanSource::Clipboard), expected);
        assert!(!r.clipboard.has_image(), "{uri} was cleared");
    }
    // A QR code that is not a setup holds nothing of ours: left alone.
    r.clipboard
        .put_image(rgba(&qr_frame("https://example.com", 5, 4)));
    assert_eq!(
        scan_code(b, OtpScanSource::Clipboard),
        ErrorCode::OtpQrNotOtp
    );
    assert!(r.clipboard.has_image());
}

#[test]
fn a_pending_setup_is_not_restored_into_a_locked_vault() {
    let _kdf = crate::test_support::kdf_permit();
    let r = rig();
    let b = &r.backend;
    let id = pending_id(&parse_uri(b, SAMPLE_URI).unwrap());
    let change = OtpChange::SetPending {
        pending_id: id,
        issuer: None,
        account_name: None,
    };
    let taken = b.take_pending_otp(Some(&change)).unwrap().unwrap();
    b.lock(LockReason::Manual).unwrap();
    b.restore_pending_otp(taken);
    assert_eq!(b.otp_pending.lock().unwrap().len(), 0);
    unlock(b);
    assert_eq!(b.otp_pending.lock().unwrap().len(), 0);
}
