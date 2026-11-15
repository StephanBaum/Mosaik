//! The Dropbox store (and, from the sign-in tests on, its OAuth) against the
//! fake server.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::fs;
use std::sync::Arc;

use mosaic_sync::folder::remote::credential::{CredentialSink, Provider, ProviderCredential};
use mosaic_sync::folder::remote::dropbox::{
    DropboxAuth, DropboxStore, dropbox_authorize_url, dropbox_exchange, dropbox_revoke,
};
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::location::FolderLocation;
use mosaic_sync::folder::remote::oauth::{AuthState, Endpoints, Pkce, REDIRECT_URI};
use mosaic_sync::folder::{FolderStore, RelPath};
use mosaic_sync::testutil::{AUTHORIZATION_CODE, FakeDropbox, Faults};
use mosaic_sync::transport::TransportError;

use zeroize::Zeroizing;

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
    store_with(server, root, path_root, DropboxAuth::fixed_for_test(TOKEN))
}

pub fn store_with(
    server: &FakeDropbox,
    root: &str,
    path_root: Option<&str>,
    auth: DropboxAuth,
) -> DropboxStore {
    DropboxStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &Endpoints::loopback(server.origin()),
        &location(root, path_root),
        auth,
    )
    .unwrap()
}

/// A sink that keeps nothing (Dropbox does not rotate its refresh token).
pub struct NullSink;

impl CredentialSink for NullSink {
    fn rotated(&self, _: &ProviderCredential) -> Result<(), TransportError> {
        Ok(())
    }
}

pub const APP_KEY: &str = "app-key-123";

/// An authorisation that signs in with the fake's current refresh token.
pub fn refreshing_auth(server: &FakeDropbox) -> DropboxAuth {
    DropboxAuth::new(
        APP_KEY.to_owned(),
        ProviderCredential::Dropbox {
            refresh_token: Zeroizing::new(server.refresh_token()),
        },
        Arc::new(NullSink),
    )
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

// ---- OAuth ---------------------------------------------------------------------------------

fn query_of(url: &str) -> Vec<(String, String)> {
    url.split_once('?')
        .unwrap()
        .1
        .split('&')
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap();
            (k.to_owned(), v.to_owned())
        })
        .collect()
}

#[test]
fn authorize_url_has_pkce_offline_and_exact_redirect() {
    let endpoints = Endpoints::production();
    let pkce = Pkce::new().unwrap();
    let state = AuthState::new().unwrap();
    let url = dropbox_authorize_url(&endpoints, APP_KEY, &pkce, &state);
    assert!(
        url.starts_with("https://www.dropbox.com/oauth2/authorize?"),
        "{url}"
    );
    let q = query_of(&url);
    let get = |k: &str| {
        q.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap()
    };
    assert_eq!(get("client_id"), APP_KEY);
    assert_eq!(get("response_type"), "code");
    assert_eq!(get("code_challenge"), pkce.challenge());
    assert_eq!(get("code_challenge_method"), "S256");
    assert_eq!(get("token_access_type"), "offline");
    assert_eq!(get("state"), state.as_param());
    assert_eq!(
        get("scope"),
        "files.metadata.read%20files.content.read%20files.content.write%20account_info.read"
    );
    assert_eq!(
        get("redirect_uri"),
        "io.github.stephanbaum.mosaic.oauth%3A%2Fcb",
        "exactly the registered redirect, encoded"
    );
    assert_eq!(REDIRECT_URI, "io.github.stephanbaum.mosaic.oauth:/cb");
    // The verifier never leaves the phone with the authorization request.
    assert!(!url.contains(pkce.verifier()));
    assert_eq!(q.len(), 8, "no other parameter: {url}");
}

#[test]
fn exchange_returns_refresh_credential_and_team_namespace() {
    let (_dir, server, _store) = dropbox("Mosaic");
    server.expect_app_key(APP_KEY);
    let http = RemoteHttp::new_loopback().unwrap();
    let endpoints = Endpoints::loopback(server.origin());
    let pkce = Pkce::new().unwrap();
    server.expect_challenge(&pkce.challenge());
    let (credential, team) =
        dropbox_exchange(&http, &endpoints, APP_KEY, AUTHORIZATION_CODE, &pkce).unwrap();
    assert_eq!(team, None);
    let ProviderCredential::Dropbox { refresh_token } = credential else {
        panic!("not a Dropbox credential");
    };
    assert_eq!(*refresh_token, server.refresh_token());
    // A team member's root is the team space.
    server.set_team_root("ns-team-7");
    let (_, team) =
        dropbox_exchange(&http, &endpoints, APP_KEY, AUTHORIZATION_CODE, &pkce).unwrap();
    assert_eq!(team.as_deref(), Some("ns-team-7"));
    // A wrong code, and a code with another verifier, are refused.
    assert_eq!(
        dropbox_exchange(&http, &endpoints, APP_KEY, "other", &pkce).err(),
        Some(TransportError::Unauthorized)
    );
    assert_eq!(
        dropbox_exchange(
            &http,
            &endpoints,
            APP_KEY,
            AUTHORIZATION_CODE,
            &Pkce::new().unwrap()
        )
        .err(),
        Some(TransportError::Unauthorized)
    );
}

#[test]
fn revoke_ends_the_grant() {
    let (_dir, server, _store) = dropbox("Mosaic");
    server.expect_app_key(APP_KEY);
    let http = RemoteHttp::new_loopback().unwrap();
    let endpoints = Endpoints::loopback(server.origin());
    let store = store_with(&server, "Mosaic", None, refreshing_auth(&server));
    assert!(!server.revoked());
    dropbox_revoke(&http, &endpoints, &DropboxAuth::fixed_for_test(TOKEN)).unwrap();
    assert!(server.revoked());
    // The grant is gone: the refresh token is refused.
    assert_eq!(
        store.list(&RelPath::root()).err(),
        Some(TransportError::Unauthorized)
    );
}

#[test]
fn token_never_in_debug_or_error() {
    let (_dir, server, _store) = dropbox("Mosaic");
    server.expect_app_key(APP_KEY);
    let refresh = server.refresh_token();
    let auth = refreshing_auth(&server);
    assert!(!format!("{auth:?}").contains(&refresh));
    let store = store_with(&server, "Mosaic", None, auth);
    store.list(&RelPath::root()).unwrap();
    assert!(!format!("{store:?}").contains(&refresh));
    // A refresh with a token the server does not know: refused, and the
    // error shows nothing of it.
    let stale = DropboxAuth::new(
        APP_KEY.to_owned(),
        ProviderCredential::Dropbox {
            refresh_token: Zeroizing::new("refresh-stale-secret".to_owned()),
        },
        Arc::new(NullSink),
    );
    let bad = store_with(&server, "Mosaic", None, stale);
    let err = bad.list(&RelPath::root()).err().unwrap();
    assert_eq!(err, TransportError::Unauthorized);
    assert!(!format!("{err:?} {err}").contains("stale-secret"));
    // No secret in any URL; the form bodies carry them, not the URLs.
    assert!(server.requests().iter().all(|r| {
        !r.path_and_query.contains(&refresh) && !r.path_and_query.contains("stale-secret")
    }));
    let credential = ProviderCredential::Dropbox {
        refresh_token: Zeroizing::new(refresh.clone()),
    };
    assert!(!format!("{credential:?}").contains(&refresh));
}
