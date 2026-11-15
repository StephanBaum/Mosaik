//! The Microsoft Graph store (and, from the sign-in tests on, its OAuth)
//! against the fake server.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_sync::folder::remote::credential::{CredentialSink, Provider, ProviderCredential};
use mosaic_sync::folder::remote::graph::{
    GRAPH_CHUNK, GraphAuth, GraphStore, graph_authorize_url, graph_browse, graph_exchange,
    graph_resolve_root,
};
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::location::FolderLocation;
use mosaic_sync::folder::remote::oauth::{AuthState, Endpoints, Pkce};
use mosaic_sync::folder::{FolderStore, RelPath};
use mosaic_sync::testutil::{
    DRIVE_ID, FakeGraph, FakeHttp, FakeResponse, Faults, GRAPH_AUTHORIZATION_CODE, id_of,
};
use mosaic_sync::transport::TransportError;

use zeroize::Zeroizing;

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
    let blob: Vec<u8> = (0u32..7 * 1024 * 1024)
        .map(|i| u8::try_from(i % 251).unwrap())
        .collect();
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
    // Write-once is always an upload session (`conflictBehavior: fail` is documented there),
    // even for a small blob; a small replace is one simple `PUT`.
    let chunks = server.upload_chunks().len();
    store.write_atomic(&d, "small.bin", b"x", false).unwrap();
    assert_eq!(server.upload_chunks().len(), chunks + 1);
    assert_eq!(
        store.write_atomic(&d, "small.bin", b"y", false),
        Err(TransportError::Io)
    );
    let chunks = server.upload_chunks().len();
    store.write_atomic(&d, "small.bin", b"z", true).unwrap();
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

// ---- OAuth ---------------------------------------------------------------------------------

pub const CLIENT_ID: &str = "00000000-0000-0000-0000-00000000aaaa";

/// Keeps the refresh tokens it is given, and can be made to fail. Each call
/// also records how many requests already used the access token that goes with
/// the new refresh token (it must be none).
pub struct RecordingSink {
    pub stored: Mutex<Vec<String>>,
    pub fail: AtomicBool,
    server: Arc<FakeGraph>,
    /// For each call: the number of requests that carried the newest access
    /// token at that moment.
    pub used_before_store: Mutex<Vec<usize>>,
}

impl RecordingSink {
    pub fn new(server: &Arc<FakeGraph>) -> Arc<Self> {
        Arc::new(Self {
            stored: Mutex::new(Vec::new()),
            fail: AtomicBool::new(false),
            server: Arc::clone(server),
            used_before_store: Mutex::new(Vec::new()),
        })
    }
}

fn requests_with_access(server: &FakeGraph, access: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|r| r.header("authorization") == Some(format!("Bearer {access}").as_str()))
        .count()
}

impl CredentialSink for RecordingSink {
    fn rotated(&self, credential: &ProviderCredential) -> Result<(), TransportError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(TransportError::Io);
        }
        let ProviderCredential::Graph { refresh_token } = credential else {
            return Err(TransportError::Protocol);
        };
        self.stored.lock().unwrap().push(refresh_token.to_string());
        // The newest access token is the one the answer carried.
        let used = requests_with_access(&self.server, &self.server.access_token());
        self.used_before_store.lock().unwrap().push(used);
        Ok(())
    }
}

pub fn refreshing_auth(server: &FakeGraph, sink: Arc<RecordingSink>) -> GraphAuth {
    GraphAuth::new(
        CLIENT_ID.to_owned(),
        ProviderCredential::Graph {
            refresh_token: Zeroizing::new(server.refresh_token()),
        },
        sink,
    )
}

#[test]
fn authorize_url_has_pkce_and_scopes() {
    let pkce = Pkce::new().unwrap();
    let state = AuthState::new().unwrap();
    let url = graph_authorize_url(&Endpoints::production(), CLIENT_ID, &pkce, &state);
    assert!(
        url.starts_with("https://login.microsoftonline.com/common/oauth2/v2.0/authorize?"),
        "{url}"
    );
    let q: Vec<(&str, &str)> = url
        .split_once('?')
        .unwrap()
        .1
        .split('&')
        .map(|p| p.split_once('=').unwrap())
        .collect();
    let get = |k: &str| q.iter().find(|(n, _)| *n == k).map(|(_, v)| *v).unwrap();
    assert_eq!(get("client_id"), CLIENT_ID);
    assert_eq!(get("response_type"), "code");
    assert_eq!(get("response_mode"), "query");
    assert_eq!(get("scope"), "Files.ReadWrite%20offline_access");
    assert_eq!(
        get("redirect_uri"),
        "io.github.stephanbaum.mosaic.oauth%3A%2Fcb"
    );
    assert_eq!(get("code_challenge"), pkce.challenge());
    assert_eq!(get("code_challenge_method"), "S256");
    assert_eq!(get("state"), state.as_param());
    assert_eq!(q.len(), 8, "{url}");
    assert!(!url.contains(pkce.verifier()));
}

#[test]
fn exchange_returns_a_refresh_credential() {
    let (_dir, server, _store) = graph();
    server.expect_client_id(CLIENT_ID);
    let pkce = Pkce::new().unwrap();
    server.expect_challenge(&pkce.challenge());
    let http = RemoteHttp::new_loopback().unwrap();
    let endpoints = Endpoints::loopback(server.origin());
    let credential = graph_exchange(
        &http,
        &endpoints,
        CLIENT_ID,
        GRAPH_AUTHORIZATION_CODE,
        &pkce,
    )
    .unwrap();
    let ProviderCredential::Graph { refresh_token } = credential else {
        panic!("not a Graph credential");
    };
    assert_eq!(*refresh_token, server.refresh_token());
    assert_eq!(
        graph_exchange(&http, &endpoints, CLIENT_ID, "other", &pkce).err(),
        Some(TransportError::Unauthorized)
    );
    assert_eq!(
        graph_exchange(
            &http,
            &endpoints,
            CLIENT_ID,
            GRAPH_AUTHORIZATION_CODE,
            &Pkce::new().unwrap()
        )
        .err(),
        Some(TransportError::Unauthorized)
    );
}

/// Review focus 3: a rotated refresh token is stored before it matters.
#[test]
fn rotated_refresh_token_is_stored_before_use() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = Arc::new(FakeGraph::serve(dir.path().to_path_buf(), TOKEN));
    server.expect_client_id(CLIENT_ID);
    server.strict_rotation(true);
    let sink = RecordingSink::new(&server);
    let old = server.refresh_token();
    let store = store_with(
        &server,
        &id_of("Mosaic"),
        refreshing_auth(&server, Arc::clone(&sink)),
    );
    // The first request signs in: the answer rotates the refresh token.
    store.list(&RelPath::root()).unwrap();
    let stored = sink.stored.lock().unwrap().clone();
    assert_eq!(stored, [server.refresh_token()]);
    assert_ne!(stored[0], old);
    // When the sink was called, no request had used the new access token.
    assert_eq!(*sink.used_before_store.lock().unwrap(), [0]);
    assert!(requests_with_access(&server, &server.access_token()) >= 1);
    // The next sign-in (the server expires the token) uses the stored one.
    let again = store_with(
        &server,
        &id_of("Mosaic"),
        refreshing_auth(&server, Arc::clone(&sink)),
    );
    again.list(&RelPath::root()).unwrap();
    assert_eq!(sink.stored.lock().unwrap().len(), 2);
    // The old token is dead on a strict server; the stored one is not.
    let stale = GraphAuth::new(
        CLIENT_ID.to_owned(),
        ProviderCredential::Graph {
            refresh_token: Zeroizing::new(old),
        },
        sink,
    );
    let bad = store_with(&server, &id_of("Mosaic"), stale);
    assert_eq!(
        bad.list(&RelPath::root()).err(),
        Some(TransportError::Unauthorized)
    );
}

#[test]
fn sink_failure_keeps_old_credential_and_fails_io() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = Arc::new(FakeGraph::serve(dir.path().to_path_buf(), TOKEN));
    server.expect_client_id(CLIENT_ID);
    let sink = RecordingSink::new(&server);
    let original = server.refresh_token();
    let store = store_with(
        &server,
        &id_of("Mosaic"),
        refreshing_auth(&server, Arc::clone(&sink)),
    );
    sink.fail.store(true, Ordering::SeqCst);
    assert_eq!(store.list(&RelPath::root()).err(), Some(TransportError::Io));
    // Nothing was stored, and no request used the new access token.
    assert!(sink.stored.lock().unwrap().is_empty());
    assert_eq!(requests_with_access(&server, &server.access_token()), 0);
    // The old refresh token is still the one in use: a retry presents it.
    sink.fail.store(false, Ordering::SeqCst);
    assert!(store.list(&RelPath::root()).unwrap().is_empty());
    let token_calls: Vec<String> = server
        .requests()
        .into_iter()
        .filter(|r| r.path_and_query.ends_with("/token"))
        .map(|r| String::from_utf8(r.body).unwrap())
        .collect();
    assert_eq!(token_calls.len(), 2);
    assert!(
        token_calls
            .iter()
            .all(|b| b.contains(&format!("refresh_token={original}")))
    );
}

#[test]
fn resolve_root_by_path_returns_drive_and_item_ids() {
    let (dir, server, _store) = graph();
    fs::create_dir_all(dir.path().join("Mosaic/Family")).unwrap();
    fs::write(dir.path().join("note.txt"), b"x").unwrap();
    let http = RemoteHttp::new_loopback().unwrap();
    let endpoints = Endpoints::loopback(server.origin());
    let auth = GraphAuth::fixed_for_test(TOKEN);
    let location = graph_resolve_root(&http, &endpoints, &auth, "/Mosaic/Family").unwrap();
    assert_eq!(location.provider, Provider::OneDrive);
    assert_eq!(location.drive_id.as_deref(), Some(DRIVE_ID));
    assert_eq!(location.item_id, Some(id_of("Mosaic/Family")));
    assert_eq!(location.root, "/Mosaic/Family");
    // The ids address the folder: a store on them sees what is in it.
    fs::write(dir.path().join("Mosaic/Family/x.bin"), b"x").unwrap();
    let store = store_with(
        &server,
        location.item_id.as_deref().unwrap(),
        GraphAuth::fixed_for_test(TOKEN),
    );
    assert_eq!(names_in(&store, &RelPath::root()), ["x.bin"]);
    // Missing, a file, and a path with a bad component are refused.
    for missing in ["/Nope", "/note.txt"] {
        assert_eq!(
            graph_resolve_root(&http, &endpoints, &auth, missing).err(),
            Some(TransportError::FolderUnavailable),
            "{missing}"
        );
    }
    assert_eq!(
        graph_resolve_root(&http, &endpoints, &auth, "/a/../b").err(),
        Some(TransportError::Protocol)
    );
    // Browsing lists folders only, with their ids.
    let top = graph_browse(&http, &endpoints, &auth, None, None).unwrap();
    assert_eq!(top, [("Mosaic".to_owned(), id_of("Mosaic"))]);
    let below = graph_browse(
        &http,
        &endpoints,
        &auth,
        Some(DRIVE_ID),
        Some(&id_of("Mosaic")),
    )
    .unwrap();
    assert_eq!(below, [("Family".to_owned(), id_of("Mosaic/Family"))]);
}

#[test]
fn token_never_in_debug_or_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("Mosaic")).unwrap();
    let server = Arc::new(FakeGraph::serve(dir.path().to_path_buf(), TOKEN));
    server.expect_client_id(CLIENT_ID);
    let sink = RecordingSink::new(&server);
    let refresh = server.refresh_token();
    let auth = refreshing_auth(&server, Arc::clone(&sink));
    assert!(!format!("{auth:?}").contains(&refresh));
    let stale = GraphAuth::new(
        CLIENT_ID.to_owned(),
        ProviderCredential::Graph {
            refresh_token: Zeroizing::new("ms-stale-secret".to_owned()),
        },
        sink,
    );
    let bad = store_with(&server, &id_of("Mosaic"), stale);
    let err = bad.list(&RelPath::root()).err().unwrap();
    assert_eq!(err, TransportError::Unauthorized);
    assert!(!format!("{err:?} {err}").contains("stale-secret"));
    assert!(server.requests().iter().all(|r| {
        !r.path_and_query.contains("stale-secret") && !r.path_and_query.contains(&refresh)
    }));
}
