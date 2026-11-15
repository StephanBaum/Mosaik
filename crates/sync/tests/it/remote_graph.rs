//! The Microsoft Graph store (and, from the sign-in tests on, its OAuth)
//! against the fake server.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::fs;

use mosaic_sync::folder::remote::credential::Provider;
use mosaic_sync::folder::remote::graph::{GRAPH_CHUNK, GraphAuth, GraphStore};
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::location::FolderLocation;
use mosaic_sync::folder::remote::oauth::Endpoints;
use mosaic_sync::folder::{FolderStore, RelPath};
use mosaic_sync::testutil::{DRIVE_ID, FakeGraph, FakeHttp, FakeResponse, Faults, id_of};
use mosaic_sync::transport::TransportError;

use crate::common::store_contract;

pub const TOKEN: &str = "EwB.graph-access-1";

pub fn location(item_id: &str) -> FolderLocation {
    FolderLocation {
        provider: Provider::OneDrive,
        root: "/Mosaic".to_owned(),
        server: None,
        drive_id: Some(DRIVE_ID.to_owned()),
        item_id: Some(item_id.to_owned()),
        path_root: None,
    }
}

pub fn store_with(server: &FakeGraph, item_id: &str, auth: GraphAuth) -> GraphStore {
    GraphStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &Endpoints::loopback(server.origin()),
        &location(item_id),
        auth,
    )
    .unwrap()
}

pub fn store_at(server: &FakeGraph, item_id: &str) -> GraphStore {
    store_with(server, item_id, GraphAuth::fixed_for_test(TOKEN))
}

/// A fake server over a fresh directory that holds the folder `Mosaic`.
fn graph() -> (tempfile::TempDir, FakeGraph, GraphStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = FakeGraph::serve(dir.path().to_path_buf(), TOKEN);
    let store = store_at(&server, &id_of("Mosaic"));
    (dir, server, store)
}

fn rel(names: &[&str]) -> RelPath {
    names
        .iter()
        .fold(RelPath::root(), |p, n| p.child(n).unwrap())
}

fn names_in(store: &GraphStore, dir: &RelPath) -> Vec<String> {
    store
        .list(dir)
        .unwrap()
        .iter()
        .map(|e| e.name.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn graph_store_passes_the_store_contract() {
    let (_dir, _server, store) = graph();
    store_contract(&store, false);
    let (_dir, _server, store) = graph();
    store_contract(&store, true);
}

#[test]
fn listing_entry_outside_the_root_is_dropped() {
    let (_dir, server, store) = graph();
    let d = rel(&["d"]);
    store.write_atomic(&d, "ok.mseg", b"x", false).unwrap();
    // A shortcut into another drive and a name with a slash are not entries.
    server.set_faults(Faults {
        outside_path: true,
        ..Faults::default()
    });
    assert_eq!(names_in(&store, &d), ["ok.mseg"]);
    // A next link that leaves the drive ends the listing in a protocol error.
    server.set_faults(Faults {
        foreign_next_link: true,
        ..Faults::default()
    });
    assert_eq!(store.list(&d).err(), Some(TransportError::Protocol));
    assert!(
        server
            .requests()
            .iter()
            .all(|r| !r.path_and_query.contains("127.0.0.2"))
    );
}

#[test]
fn listing_follows_pages() {
    let (dir, server, store) = graph();
    fs::create_dir(dir.path().join("Mosaic/many")).unwrap();
    for i in 0..450 {
        fs::write(dir.path().join(format!("Mosaic/many/f{i:04}.bin")), b"x").unwrap();
    }
    let before = server.requests().len();
    assert_eq!(names_in(&store, &rel(&["many"])).len(), 450);
    assert_eq!(server.requests().len() - before, 3, "three pages of 200");
}

#[test]
fn graph_download_redirect_drops_authorization() {
    let (_dir, server, store) = graph();
    store
        .write_atomic(&rel(&["d"]), "a.mseg", b"secret bytes", false)
        .unwrap();
    assert_eq!(
        store.read_capped(&rel(&["d", "a.mseg"]), 100),
        Ok(Some(b"secret bytes".to_vec()))
    );
    let content: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path_and_query.contains("/content"))
        .filter(|r| r.method == "GET")
        .collect();
    assert_eq!(content.len(), 1);
    assert!(content[0].header("authorization").is_some());
    let downloads = server.download_requests();
    assert_eq!(downloads.len(), 1);
    assert!(downloads[0].header("authorization").is_none());
}

#[test]
fn large_blob_uses_upload_session_with_320k_multiple_chunks() {
    let (dir, server, store) = graph();
    assert_eq!(GRAPH_CHUNK, 3_276_800);
    let blob: Vec<u8> = (0..7 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let d = rel(&["d"]);
    store.write_atomic(&d, "big.bin", &blob, false).unwrap();
    assert_eq!(server.upload_chunks(), [3_276_800, 3_276_800, 786_432]);
    assert_eq!(fs::read(dir.path().join("Mosaic/d/big.bin")).unwrap(), blob);
    // Write-once: the session is opened with `fail` and a taken name is `Io`.
    assert_eq!(
        store.write_atomic(&d, "big.bin", &blob, false),
        Err(TransportError::Io)
    );
    store.write_atomic(&d, "big.bin", &blob, true).unwrap();
    let sessions: Vec<String> = server
        .requests()
        .into_iter()
        .filter(|r| r.path_and_query.contains("createUploadSession"))
        .map(|r| String::from_utf8(r.body).unwrap())
        .collect();
    assert_eq!(sessions.len(), 3);
    assert!(sessions[0].contains("\"fail\""), "{}", sessions[0]);
    assert!(sessions[2].contains("\"replace\""), "{}", sessions[2]);
    // A small blob is one simple `PUT`.
    let chunks = server.upload_chunks().len();
    store.write_atomic(&d, "small.bin", b"x", false).unwrap();
    assert_eq!(server.upload_chunks().len(), chunks);
}

#[test]
fn upload_url_must_be_secure() {
    let evil = FakeHttp::start(|_| {
        FakeResponse::new(
            200,
            &[("Content-Type", "application/json")],
            br#"{"uploadUrl":"http://evil.example/upload"}"#.to_vec(),
        )
    });
    let store = GraphStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &Endpoints::loopback(evil.origin()),
        &location("root"),
        GraphAuth::fixed_for_test(TOKEN),
    )
    .unwrap();
    let blob = vec![1u8; 5 * 1024 * 1024];
    assert_eq!(
        store.write_atomic(&RelPath::root(), "big.bin", &blob, false),
        Err(TransportError::InsecureUrl)
    );
    // Only the session request was made; nothing went to the evil host.
    assert_eq!(evil.requests().len(), 1);
}

#[test]
fn graph_507_is_quota_exceeded() {
    let (_dir, server, store) = graph();
    server.set_faults(Faults {
        fail_nth: Some((1, 507, None)),
        ..Faults::default()
    });
    assert_eq!(
        store.write_atomic(&RelPath::root(), "a.bin", b"x", false),
        Err(TransportError::QuotaExceeded)
    );
    server.set_faults(Faults {
        fail_nth: Some((1, 429, Some(7))),
        ..Faults::default()
    });
    assert_eq!(
        store.list(&RelPath::root()).err(),
        Some(TransportError::RateLimited { retry_after_s: 7 })
    );
}

#[test]
fn remove_never_deletes_a_folder_or_a_changed_file() {
    let (dir, server, store) = graph();
    let planted = dir.path().join("Mosaic/d/one.mseg");
    fs::create_dir_all(planted.join("inner")).unwrap();
    fs::write(planted.join("inner/keep.txt"), b"keep").unwrap();
    let writes = server.writes();
    store.remove(&rel(&["d", "one.mseg"])).unwrap();
    assert!(planted.join("inner/keep.txt").is_file());
    assert_eq!(server.writes(), writes);
    // A folder is no file for a read either.
    assert_eq!(store.read_capped(&rel(&["d", "one.mseg"]), 10), Ok(None));
    // A file that changed since it was listed is kept.
    store
        .write_atomic(&rel(&["d"]), "f.mseg", b"x", false)
        .unwrap();
    store.begin_run().unwrap();
    store.list(&rel(&["d"])).unwrap();
    fs::write(dir.path().join("Mosaic/d/f.mseg"), b"changed elsewhere").unwrap();
    assert_eq!(
        store.remove(&rel(&["d", "f.mseg"])),
        Err(TransportError::Io)
    );
    assert!(dir.path().join("Mosaic/d/f.mseg").is_file());
}

#[test]
fn wrong_token_is_unauthorized_and_never_in_a_url() {
    let (_dir, server, _store) = graph();
    let wrong = store_with(
        &server,
        &id_of("Mosaic"),
        GraphAuth::fixed_for_test("EwB.wrong"),
    );
    assert_eq!(
        wrong.list(&RelPath::root()).err(),
        Some(TransportError::Unauthorized)
    );
    assert!(
        server
            .requests()
            .iter()
            .all(|r| !r.path_and_query.contains("EwB."))
    );
    assert!(!format!("{wrong:?} {:?}", GraphAuth::fixed_for_test(TOKEN)).contains("EwB."));
}

#[test]
fn locations_are_checked() {
    let http = || RemoteHttp::new_loopback().unwrap();
    let e = Endpoints::loopback("http://127.0.0.1:1");
    let auth = || GraphAuth::fixed_for_test(TOKEN);
    let mut other = location("root");
    other.provider = Provider::Dropbox;
    let mut no_drive = location("root");
    no_drive.drive_id = None;
    let mut no_item = location("root");
    no_item.item_id = None;
    let mut slash = location("a/b");
    slash.drive_id = Some("d".to_owned());
    for bad in [other, no_drive, no_item, slash] {
        assert_eq!(
            GraphStore::new(http(), &e, &bad, auth()).err(),
            Some(TransportError::Protocol)
        );
    }
    assert_eq!(
        GraphStore::new(RemoteHttp::new().unwrap(), &e, &location("root"), auth()).err(),
        Some(TransportError::InsecureUrl)
    );
}
