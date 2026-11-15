//! The `WebDAV` store and Nextcloud's Login Flow v2 against the fake server.

#![expect(
    clippy::unwrap_used,
    reason = "integration tests"
)]

use std::fs;

use mosaic_sync::folder::remote::credential::ProviderCredential;
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::webdav::WebDavStore;
use mosaic_sync::folder::{FolderStore, RelPath};
use mosaic_sync::testutil::{FakeDav, FakeHttp, FakeResponse, Faults};
use mosaic_sync::transport::TransportError;
use zeroize::Zeroizing;

use crate::common::{files_ending, store_contract};

const PASSWORD: &str = "app-pass-word-123";

fn credential() -> ProviderCredential {
    ProviderCredential::WebDav {
        login: "alice".to_owned(),
        app_password: Zeroizing::new(PASSWORD.to_owned()),
    }
}

/// A fake server over a fresh directory that holds the folder `Mosaic`.
fn dav() -> (tempfile::TempDir, FakeDav, WebDavStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = FakeDav::serve(dir.path().to_path_buf(), "alice", PASSWORD);
    let store = WebDavStore::new(
        RemoteHttp::new(true).unwrap(),
        &server.base_url(),
        "Mosaic",
        credential(),
    )
    .unwrap();
    (dir, server, store)
}

fn rel(names: &[&str]) -> RelPath {
    names
        .iter()
        .fold(RelPath::root(), |p, n| p.child(n).unwrap())
}

#[test]
fn dav_store_lists_reads_and_writes_like_local() {
    let (_dir, _server, store) = dav();
    store_contract(&store, false);
    let (_dir, _server, store) = dav();
    store_contract(&store, true);
}

#[test]
fn listing_entry_outside_the_root_is_dropped() {
    let (dir, server, store) = dav();
    let d = rel(&["d"]);
    store.write_atomic(&d, "ok.mseg", b"x", false).unwrap();
    // Real files with the names the lies use must not be what is listed.
    fs::create_dir_all(dir.path().join("Other")).unwrap();
    fs::write(dir.path().join("Other").join("x.mseg"), b"x").unwrap();
    server.set_faults(Faults {
        outside_href: true,
        ..Faults::default()
    });
    let names: Vec<String> = store
        .list(&d)
        .unwrap()
        .iter()
        .map(|e| e.name.to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["ok.mseg"]);
}

#[test]
fn remove_never_deletes_a_collection() {
    let (dir, server, store) = dav();
    let planted = dir.path().join("Mosaic").join("d").join("one.mseg");
    fs::create_dir_all(planted.join("inner")).unwrap();
    fs::write(planted.join("inner").join("keep.txt"), b"keep").unwrap();
    let writes = server.writes();
    store.remove(&rel(&["d", "one.mseg"])).unwrap();
    assert!(planted.join("inner").join("keep.txt").is_file());
    assert_eq!(server.writes(), writes, "no DELETE was sent");
    // A file read of a collection is not a file.
    assert_eq!(store.read_capped(&rel(&["d", "one.mseg"]), 10), Ok(None));
}

#[test]
fn write_once_conflict_is_io_and_leaves_no_temp() {
    let (dir, _server, store) = dav();
    let d = rel(&["d"]);
    store.write_atomic(&d, "a.mseg", b"first", false).unwrap();
    assert_eq!(
        store.write_atomic(&d, "a.mseg", b"second", false),
        Err(TransportError::Io)
    );
    assert_eq!(
        fs::read(dir.path().join("Mosaic/d/a.mseg")).unwrap(),
        b"first"
    );
    assert_eq!(
        files_ending(dir.path(), ".mtmp"),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn propfind_over_cap_is_protocol() {
    let big = FakeHttp::start(|_| FakeResponse::new(207, &[], vec![b' '; 5 * 1024 * 1024]));
    let store = WebDavStore::new(
        RemoteHttp::new(true).unwrap(),
        &format!("{}/dav", big.origin()),
        "",
        credential(),
    )
    .unwrap();
    assert_eq!(
        store.list(&RelPath::root()).err(),
        Some(TransportError::Protocol)
    );
}

#[test]
fn password_never_in_debug() {
    let (_dir, _server, store) = dav();
    let text = format!("{store:?}");
    assert!(!text.contains(PASSWORD));
    assert!(!format!("{:?}", credential()).contains(PASSWORD));
    // A wrong password is a 401, and no password is ever in a URL.
    let dir = tempfile::tempdir().unwrap();
    let server = FakeDav::serve(dir.path().to_path_buf(), "alice", PASSWORD);
    let wrong = WebDavStore::new(
        RemoteHttp::new(true).unwrap(),
        &server.base_url(),
        "",
        ProviderCredential::WebDav {
            login: "alice".to_owned(),
            app_password: Zeroizing::new("wrong".to_owned()),
        },
    )
    .unwrap();
    assert_eq!(
        wrong.list(&RelPath::root()).err(),
        Some(TransportError::Unauthorized)
    );
    assert!(
        server
            .requests()
            .iter()
            .all(|r| !r.path_and_query.contains(PASSWORD))
    );
}

#[test]
fn credentials_and_roots_are_checked() {
    let http = || RemoteHttp::new(true).unwrap();
    let other = ProviderCredential::Dropbox {
        refresh_token: Zeroizing::new("t".to_owned()),
    };
    assert_eq!(
        WebDavStore::new(http(), "http://127.0.0.1:1/dav", "", other).err(),
        Some(TransportError::Protocol)
    );
    assert_eq!(
        WebDavStore::new(http(), "http://127.0.0.1:1/dav", "a/../b", credential()).err(),
        Some(TransportError::Protocol)
    );
    assert_eq!(
        WebDavStore::new(
            RemoteHttp::new(false).unwrap(),
            "http://127.0.0.1:1/dav",
            "",
            credential()
        )
        .err(),
        Some(TransportError::InsecureUrl)
    );
}
