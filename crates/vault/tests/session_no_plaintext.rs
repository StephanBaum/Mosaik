//! A session never writes plaintext to disk (CRYPTO_SPEC §11): fill a
//! file-backed account through the session API with content that carries
//! a marker (titles, user names, passwords, URLs, tags, notes, vault
//! names, OTP secrets, attachment names and contents), use search, rename,
//! move, rotate and lock, then scan every file the store wrote for the
//! marker, the master password and the Secret Key.

#![allow(clippy::unwrap_used, reason = "integration test")]

mod common;

use std::fs;
use std::path::Path;

use common::{PASSWORD, new_account_in, pw};
use mosaic_core::SecretVec;
use mosaic_core::model::{ItemType, LoginUrl, OtpConfig, SecretField, TextField};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

const MARKER: &str = "MosaicMarker7c3f";

fn all_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(all_files(&path));
        } else if path.to_string_lossy().ends_with("-lock") {
            // The store's lock file: always empty, and on Windows not
            // readable while the store holds its lock.
            assert_eq!(fs::metadata(&path).unwrap().len(), 0);
        } else {
            out.push((path.display().to_string(), fs::read(&path).unwrap()));
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn assert_absent(dir: &Path, needles: &[(&str, Vec<u8>)]) {
    let files = all_files(dir);
    assert!(files.len() >= 2, "expected the database and attachments");
    for (path, bytes) in &files {
        for (what, needle) in needles {
            assert!(!contains(bytes, needle), "{what} found in {path}");
        }
    }
}

#[test]
fn nothing_readable_reaches_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("vault.db");
    let (mut s, created) = new_account_in(Store::open(&db).unwrap());
    let m = |what: &str| format!("{what}-{MARKER}");

    let personal = s.create_vault(&m("vault")).unwrap();
    let work = s.create_vault("Work").unwrap();
    let mut otp = OtpConfig::totp(SecretVec::from_slice(MARKER.as_bytes()).unwrap());
    otp.label = Some(mosaic_core::model::Text::new(&m("otp-label")));
    let item = s
        .add_item(personal, &ItemType::Login, |e| {
            e.set_title(&m("title"))?;
            e.set_text(TextField::Username, &m("user"))?;
            e.set_secret(SecretField::Password, &m("password"))?;
            e.set_text(TextField::Notes, &m("notes"))?;
            e.add_url(&LoginUrl::new(&format!("https://{MARKER}.example/login")))?;
            e.add_tag(&m("tag"))?;
            e.set_otp(&otp)
        })
        .unwrap();
    s.update_item(personal, item, |e| {
        e.set_secret(SecretField::Password, &m("second-password"))
    })
    .unwrap();
    s.add_attachment(
        personal,
        item,
        &m("file-name"),
        "text/plain",
        m("file-content").as_bytes(),
    )
    .unwrap();
    assert_eq!(s.search(MARKER).len(), 1);
    s.rename_vault(work, &m("renamed")).unwrap();
    let moved = s.move_item(personal, item, work).unwrap();
    s.rotate_vault_key(work).unwrap();
    s.trash_item(work, moved).unwrap();

    let secret_display = created.emergency_kit.secret_key.expose_secret().to_owned();
    let needles = vec![
        ("marker", MARKER.as_bytes().to_vec()),
        ("master password", PASSWORD.as_bytes().to_vec()),
        ("Secret Key (display)", secret_display.as_bytes().to_vec()),
        (
            "Secret Key (display, no dashes)",
            secret_display.replace('-', "").into_bytes(),
        ),
    ];

    // While the session is open (database and WAL as they are).
    assert_absent(dir.path(), &needles);

    // After locking and closing the store.
    s.lock().into_store().close().unwrap();
    assert_absent(dir.path(), &needles);

    // And the content is all still there.
    let s = LockedSession::open(Store::open(&db).unwrap())
        .unlock(&pw(PASSWORD), &created.secret_key)
        .unwrap();
    let it = s.item(work, moved).unwrap();
    assert_eq!(it.title(), Some(m("title").as_str()));
    assert_eq!(it.password_history().len(), 1);
    assert_eq!(s.search(MARKER).len(), 0, "trashed");
}
