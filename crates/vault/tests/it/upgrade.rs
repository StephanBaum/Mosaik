//! A store written by schema v10 (before M10) upgrades to v11 and keeps working.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::path::Path;

use mosaic_core::{MasterPassword, SecretKey};
use mosaic_vault::session::{BootId, LockedSession, QuickMethod};
use mosaic_vault::storage::{LATEST_VERSION, Store};
use rusqlite::Connection;

/// `tests/fixtures/store_v10.sqlite`: written by the v10 code (commit cac40bb):
/// an account on the baseline profile, one vault "Fixture" with one login and a
/// quick-unlock enrollment. Password and Secret Key below.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/store_v10.sqlite"
);
const PASSWORD: &str = "fixture password v10";
const SECRET_KEY: &str = "M1-0HQ6-SW2G-0N1D-66S9-X7D6-XFA1-B8BG";

fn user_version(path: &Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

fn has_table(path: &Path, name: &str) -> bool {
    let n: i64 = Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get(0),
        )
        .unwrap();
    n > 0
}

#[test]
fn schema_v10_fixture_upgrades_to_v11() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    std::fs::copy(FIXTURE, &path).unwrap();
    assert_eq!(user_version(&path), 10);
    assert!(!has_table(&path, "boot_marker"));

    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 11);
    assert_eq!(LATEST_VERSION, 11);
    assert_eq!(store.boot_marker().unwrap(), None);
    let mut locked = LockedSession::open(store);
    // The old data is there: the account and the quick-unlock wrapping.
    assert!(locked.has_account().unwrap());
    assert!(locked.quick_unlock_enrollment().unwrap().is_some());
    // No marker, so a new process needs the password (the desktop rule too).
    assert!(
        !locked
            .adopt_boot_marker(&BootId::new(&[1]).unwrap())
            .unwrap()
    );
    assert!(locked.password_required(QuickMethod::Biometric));

    let secret_key = SecretKey::parse(SECRET_KEY).unwrap();
    let mut s = locked
        .unlock(&MasterPassword::new(PASSWORD.to_owned()), &secret_key)
        .map_err(|e| e.error)
        .unwrap();
    let vaults = s.vaults();
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].name, Some("Fixture"));
    assert_eq!(vaults[0].item_count, 1);
    // The new table works on the upgraded file.
    s.record_boot_marker(&BootId::new(&[7; 8]).unwrap(), 5)
        .unwrap();
    assert_eq!(
        s.store().boot_marker().unwrap().unwrap().boot_id,
        vec![7; 8]
    );
    drop(s.lock().into_store());
    assert_eq!(user_version(&path), 11);
    assert!(has_table(&path, "boot_marker"));
    // The re-authentication count (N2) exists after the upgrade and starts at 0.
    assert!(has_table(&path, "reauth_failures"));
    let store = Store::open(&path).unwrap();
    assert_eq!(store.reauth_failures().unwrap(), 0);
}
