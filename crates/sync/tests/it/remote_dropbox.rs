//! The Dropbox store (and, from the sign-in tests on, its OAuth) against the
//! fake server.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::fs;

use mosaic_sync::folder::remote::credential::Provider;
use mosaic_sync::folder::remote::dropbox::{DropboxAuth, DropboxStore};
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::location::FolderLocation;
use mosaic_sync::folder::remote::oauth::Endpoints;
use mosaic_sync::folder::{FolderStore, RelPath};
use mosaic_sync::testutil::{FakeDropbox, Faults};
use mosaic_sync::transport::TransportError;

use crate::common::store_contract;

const TOKEN: &str = "sl.access-token-1";

pub fn location(root: &str, path_root: Option<&str>) -> FolderLocation {
    FolderLocation {
        provider: Provider::Dropbox,
        root: root.to_owned(),
        server: None,
        drive_id: None,
        item_id: None,
        path_root: path_root.map(str::to_owned),
    }
}

pub fn store_at(server: &FakeDropbox, root: &str, path_root: Option<&str>) -> DropboxStore {
    DropboxStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &Endpoints::loopback(server.origin()),
        &location(root, path_root),
        DropboxAuth::fixed_for_test(TOKEN),
    )
    .unwrap()
}

/// A fake server over a fresh directory that holds the folder `root`.
fn dropbox(root: &str) -> (tempfile::TempDir, FakeDropbox, DropboxStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(root)).unwrap();
    let server = FakeDropbox::serve(dir.path().to_path_buf(), TOKEN);
    let store = store_at(&server, root, None);
    (dir, server, store)
}

fn rel(names: &[&str]) -> RelPath {
    names
        .iter()
        .fold(RelPath::root(), |p, n| p.child(n).unwrap())
}

fn names_in(store: &DropboxStore, dir: &RelPath) -> Vec<String> {
    store
        .list(dir)
        .unwrap()
        .iter()
        .map(|e| e.name.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn dropbox_store_passes_the_store_contract() {
    let (_dir, _server, store) = dropbox("Mosaic");
    store_contract(&store, false);
    let (_dir, _server, store) = dropbox("Mosaic");
    store_contract(&store, true);
}

#[test]
fn listing_entry_outside_the_root_is_dropped() {
    let (dir, server, store) = dropbox("Mosaic");
    let d = rel(&["d"]);
    store.write_atomic(&d, "ok.mseg", b"x", false).unwrap();
    // Real files with the names the lies use must not be what is listed.
    fs::create_dir_all(dir.path().join("Other/a")).unwrap();
    fs::write(dir.path().join("Other/a/x.mseg"), b"x").unwrap();
    fs::create_dir_all(dir.path().join("Mosaic-evil")).unwrap();
    fs::write(dir.path().join("Mosaic-evil/x"), b"x").unwrap();
    server.set_faults(Faults {
        outside_path: true,
        ..Faults::default()
    });
    assert_eq!(names_in(&store, &d), ["ok.mseg"]);
    assert_eq!(names_in(&store, &RelPath::root()), ["d"]);
    // The same in a run (the recursive listing).
    store.begin_run().unwrap();
    assert_eq!(names_in(&store, &d), ["ok.mseg"]);
    assert_eq!(names_in(&store, &RelPath::root()), ["d"]);
}

#[test]
fn upload_args_are_ascii_and_write_once() {
    let (_dir, server, store) = dropbox("Möbius/Mosaic");
    let d = rel(&["d"]);
    store.write_atomic(&d, "a.mseg", b"one", false).unwrap();
    store.write_atomic(&d, "a.mseg", b"two", true).unwrap();
    let uploads: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path_and_query == "/2/files/upload")
        .collect();
    assert_eq!(uploads.len(), 2);
    for r in &uploads {
        let arg = r.header("dropbox-api-arg").unwrap();
        assert!(arg.is_ascii(), "{arg}");
        assert!(arg.contains("\\u00f6"), "{arg}");
        assert!(arg.contains("\"autorename\":false"), "{arg}");
    }
    let add = uploads[0].header("dropbox-api-arg").unwrap();
    assert!(add.contains("\"mode\":\"add\""), "{add}");
    assert!(add.contains("\"strict_conflict\":true"), "{add}");
    let overwrite = uploads[1].header("dropbox-api-arg").unwrap();
    assert!(overwrite.contains("\"mode\":\"overwrite\""), "{overwrite}");
    // The write-once name was taken by the first write.
    assert_eq!(
        store.write_atomic(&d, "a.mseg", b"three", false),
        Err(TransportError::Io)
    );
    assert_eq!(
        store.read_capped(&rel(&["d", "a.mseg"]), 10),
        Ok(Some(b"two".to_vec()))
    );
}

#[test]
fn remove_without_rev_refuses_a_folder() {
    let (dir, server, store) = dropbox("Mosaic");
    let planted = dir.path().join("Mosaic/d/one.mseg");
    fs::create_dir_all(planted.join("inner")).unwrap();
    fs::write(planted.join("inner/keep.txt"), b"keep").unwrap();
    let writes = server.writes();
    store.remove(&rel(&["d", "one.mseg"])).unwrap();
    assert!(planted.join("inner/keep.txt").is_file());
    assert_eq!(server.writes(), writes);
    assert!(
        server
            .requests()
            .iter()
            .all(|r| r.path_and_query != "/2/files/delete_v2")
    );
    // A file whose rev changed since it was looked at is not deleted either.
    store
        .write_atomic(&rel(&["d"]), "f.mseg", b"x", false)
        .unwrap();
    store.begin_run().unwrap();
    fs::write(dir.path().join("Mosaic/d/f.mseg"), b"changed elsewhere").unwrap();
    assert_eq!(
        store.remove(&rel(&["d", "f.mseg"])),
        Err(TransportError::Io)
    );
    assert!(dir.path().join("Mosaic/d/f.mseg").is_file());
}

#[test]
fn dropbox_429_maps_to_rate_limited() {
    let (_dir, server, store) = dropbox("Mosaic");
    server.set_faults(Faults {
        fail_nth: Some((1, 429, Some(30))),
        ..Faults::default()
    });
    assert_eq!(
        store.list(&RelPath::root()).err(),
        Some(TransportError::RateLimited { retry_after_s: 30 })
    );
    // The next request is served.
    assert!(store.list(&RelPath::root()).unwrap().is_empty());
}

#[test]
fn run_listing_over_max_entries_is_protocol() {
    let (_dir, server, store) = dropbox("Mosaic");
    server.set_faults(Faults {
        phantom_entries: 50_001,
        ..Faults::default()
    });
    assert_eq!(store.begin_run(), Err(TransportError::Protocol));
}

#[test]
fn path_root_header_sent_for_team_space() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = FakeDropbox::serve(dir.path().to_path_buf(), TOKEN);
    let team = store_at(&server, "Mosaic", Some("1234567"));
    team.begin_run().unwrap();
    team.write_atomic(&rel(&["d"]), "a.mseg", b"x", false)
        .unwrap();
    team.read_capped(&rel(&["d", "a.mseg"]), 10).unwrap();
    let requests = server.requests();
    assert!(requests.len() >= 3);
    for r in &requests {
        assert_eq!(
            r.header("dropbox-api-path-root"),
            Some("{\".tag\":\"root\",\"root\":\"1234567\"}"),
            "{}",
            r.path_and_query
        );
    }
    // No header for a personal folder.
    let personal = store_at(&server, "Mosaic", None);
    let before = server.requests().len();
    personal.list(&RelPath::root()).unwrap();
    assert!(
        server.requests()[before..]
            .iter()
            .all(|r| r.header("dropbox-api-path-root").is_none())
    );
}

#[test]
fn wrong_token_is_unauthorized_and_never_in_a_url() {
    let (_dir, server, _store) = dropbox("Mosaic");
    let wrong = DropboxStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &Endpoints::loopback(server.origin()),
        &location("Mosaic", None),
        DropboxAuth::fixed_for_test("sl.wrong"),
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
            .all(|r| !r.path_and_query.contains("sl."))
    );
    assert!(!format!("{wrong:?}").contains("sl."));
}

#[test]
fn locations_are_checked() {
    let http = || RemoteHttp::new_loopback().unwrap();
    let e = Endpoints::loopback("http://127.0.0.1:1");
    let auth = || DropboxAuth::fixed_for_test(TOKEN);
    let mut other = location("Mosaic", None);
    other.provider = Provider::OneDrive;
    assert_eq!(
        DropboxStore::new(http(), &e, &other, auth()).err(),
        Some(TransportError::Protocol)
    );
    assert_eq!(
        DropboxStore::new(http(), &e, &location("a/../b", None), auth()).err(),
        Some(TransportError::Protocol)
    );
    assert_eq!(
        DropboxStore::new(RemoteHttp::new().unwrap(), &e, &location("a", None), auth()).err(),
        Some(TransportError::InsecureUrl)
    );
}
