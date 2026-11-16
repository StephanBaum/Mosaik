//! The folder engine over `WebDAV`: a phone that reads and writes its replica
//! of the folder through the provider store while a desktop works on a local
//! directory, with a file-sync client between them, a listing that lags, a
//! rate limit and a hostile folder (ADR-0162, review focus 1, 2 and 4).

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

use std::fs;
use std::path::Path;

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{ItemId, VaultId};
use mosaic_sync::engine::{SyncError, SyncReport};
use mosaic_sync::folder::remote::credential::ProviderCredential;
use mosaic_sync::folder::remote::dropbox::DropboxAuth;
use mosaic_sync::folder::remote::http::RemoteHttp;
use mosaic_sync::folder::remote::webdav::WebDavStore;
use mosaic_sync::folder::{self, FolderStore, RelPath};
use mosaic_sync::testutil::{FakeDav, FakeDropbox, FakeGraph, Faults, LocalSession, SimConfig};
use mosaic_sync::transport::TransportError;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use zeroize::Zeroizing;

use crate::common::{
    Dev, PASSWORD, World, is_idle, log_dir, make_engine_over, open_remote_transport, pw,
};
use crate::common::{device_dir, segments};
use crate::remote_dropbox::{APP_KEY, refreshing_auth, store_with};
use crate::remote_graph::store_at as graph_store_at;

const DAV_PASSWORD: &str = "phone-app-password";
const DROPBOX_TOKEN: &str = "sl.access-token-1";

fn add_login(w: &mut World, d: usize, vault: VaultId, title: &str) -> ItemId {
    w.devs[d]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, "pw")
        })
        .unwrap()
}

fn set_password(w: &mut World, d: usize, vault: VaultId, item: ItemId, value: &str) {
    w.devs[d]
        .s_mut()
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, value))
        .unwrap();
}

fn password_of(w: &World, d: usize, vault: VaultId, item: ItemId) -> Option<String> {
    let session = w.devs[d].s();
    let it = session.item(vault, item).ok()?;
    Some(
        it.secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret()
            .to_owned(),
    )
}

fn webdav(dav: &FakeDav) -> WebDavStore {
    WebDavStore::new(
        RemoteHttp::new_loopback().unwrap(),
        &dav.base_url(),
        "",
        ProviderCredential::WebDav {
            login: "alice".to_owned(),
            app_password: Zeroizing::new(DAV_PASSWORD.to_owned()),
        },
    )
    .unwrap()
}

/// Device 0 (the desktop, on a local directory) owns a vault with one item
/// and has synced it; replica 1 holds the folder.
fn desktop(seed: u64) -> (World, VaultId, ItemId) {
    let mut w = World::new(seed, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    (w, vault, item)
}

/// Device 1 (the phone) joins replica 1 through a `WebDAV` server that
/// serves it.
fn join_phone(w: &mut World) -> FakeDav {
    let dav = FakeDav::serve(w.replica(1), "alice", DAV_PASSWORD);
    let store = Store::open(w.tmp.path().join("device-1.db")).unwrap();
    let session = folder::join_from_folder_in(
        &webdav(&dav),
        w.account,
        LockedSession::open(store),
        &pw(PASSWORD),
        &w.secret_key,
    )
    .unwrap_or_else(|f| panic!("join failed: {:?}", f.error));
    let access = LocalSession::new(session);
    let transport = open_remote_transport(&access, Box::new(webdav(&dav)), &w.clock, "phone");
    w.devs.push(Dev {
        engine: make_engine_over(transport),
        access,
        last: SyncReport::default(),
    });
    dav
}

/// Device 1 (the phone) joins replica 1 through a fake Dropbox that serves it.
fn join_phone_dropbox(w: &mut World) -> FakeDropbox {
    join_phone_dropbox_with(w, |_| DropboxAuth::fixed_for_test(DROPBOX_TOKEN))
}

/// As [`join_phone_dropbox`], with the authorisation `auth` makes for each store.
fn join_phone_dropbox_with(
    w: &mut World,
    auth: impl Fn(&FakeDropbox) -> DropboxAuth,
) -> FakeDropbox {
    let server = FakeDropbox::serve(w.replica(1), DROPBOX_TOKEN);
    server.expect_app_key(APP_KEY);
    let store = Store::open(w.tmp.path().join("device-1.db")).unwrap();
    let session = folder::join_from_folder_in(
        &store_with(&server, "", None, auth(&server)),
        w.account,
        LockedSession::open(store),
        &pw(PASSWORD),
        &w.secret_key,
    )
    .unwrap_or_else(|f| panic!("join failed: {:?}", f.error));
    let access = LocalSession::new(session);
    let transport = open_remote_transport(
        &access,
        Box::new(store_with(&server, "", None, auth(&server))),
        &w.clock,
        "phone",
    );
    w.devs.push(Dev {
        engine: make_engine_over(transport),
        access,
        last: SyncReport::default(),
    });
    server
}

/// Device 1 (the phone) joins replica 1 through a fake Graph server that serves it.
fn join_phone_graph(w: &mut World) -> FakeGraph {
    let server = FakeGraph::serve(w.replica(1), crate::remote_graph::TOKEN);
    let store = Store::open(w.tmp.path().join("device-1.db")).unwrap();
    let session = folder::join_from_folder_in(
        &graph_store_at(&server, "root"),
        w.account,
        LockedSession::open(store),
        &pw(PASSWORD),
        &w.secret_key,
    )
    .unwrap_or_else(|f| panic!("join failed: {:?}", f.error));
    let access = LocalSession::new(session);
    let transport = open_remote_transport(
        &access,
        Box::new(graph_store_at(&server, "root")),
        &w.clock,
        "phone",
    );
    w.devs.push(Dev {
        engine: make_engine_over(transport),
        access,
        last: SyncReport::default(),
    });
    server
}

/// Device 1 joins replica 1 as a local directory (the reference).
fn join_local_phone(w: &mut World) {
    w.join();
}

/// One run of device `d`, started the way the phone starts it.
fn run(w: &mut World, d: usize) -> Result<SyncReport, SyncError> {
    // The run's listing is part of the run: a refusal there ends it too.
    w.devs[d]
        .engine
        .transport_mut()
        .begin_run()
        .map_err(SyncError::Transport)?;
    w.try_sync(d)
}

/// Delivers everything and runs both devices until two passes in a row
/// change nothing.
fn converge(w: &mut World) {
    let mut idle = 0;
    for _ in 0..40 {
        w.cloud.settle().unwrap();
        let a = w.sync(0);
        let b = run(w, 1).unwrap();
        w.advance(60_000);
        if is_idle(&a) && is_idle(&b) {
            idle += 1;
            if idle == 2 {
                return;
            }
        } else {
            idle = 0;
        }
    }
    panic!("no convergence after 40 passes");
}

#[test]
fn phone_joins_and_pulls_desktop_items() {
    let (mut w, vault, item) = desktop(21);
    let _dav = join_phone(&mut w);
    let report = run(&mut w, 1).unwrap();
    assert!(report.pulled >= 1, "{report:?}");
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("first")
    );
    // The phone's own writes (its device info, checkpoint) reached the folder.
    assert!(device_dir(&w, 1, 1).read_dir().unwrap().next().is_some());
    assert!(is_idle(&run(&mut w, 1).unwrap()));
}

#[test]
fn phone_joins_and_pulls_desktop_items_graph() {
    let (mut w, vault, item) = desktop(35);
    let _server = join_phone_graph(&mut w);
    let report = run(&mut w, 1).unwrap();
    assert!(report.pulled >= 1, "{report:?}");
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("first")
    );
    assert!(device_dir(&w, 1, 1).read_dir().unwrap().next().is_some());
    set_password(&mut w, 1, vault, item, "from the phone");
    run(&mut w, 1).unwrap();
    converge(&mut w);
    assert_eq!(
        password_of(&w, 0, vault, item).as_deref(),
        Some("from the phone")
    );
    assert!(is_idle(&run(&mut w, 1).unwrap()));
}

#[test]
fn phone_joins_and_pulls_desktop_items_dropbox() {
    let (mut w, vault, item) = desktop(31);
    let _server = join_phone_dropbox(&mut w);
    let report = run(&mut w, 1).unwrap();
    assert!(report.pulled >= 1, "{report:?}");
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("first")
    );
    // The phone's own writes reached the folder; the desktop sees them.
    assert!(device_dir(&w, 1, 1).read_dir().unwrap().next().is_some());
    set_password(&mut w, 1, vault, item, "from the phone");
    run(&mut w, 1).unwrap();
    converge(&mut w);
    assert_eq!(
        password_of(&w, 0, vault, item).as_deref(),
        Some("from the phone")
    );
    assert!(is_idle(&run(&mut w, 1).unwrap()));
}

/// Review focus 3: the access token runs out in the middle of a run.
#[test]
fn expired_access_token_mid_run_refreshes_once() {
    let (mut w, vault, item) = desktop(33);
    let server = join_phone_dropbox_with(&mut w, refreshing_auth);
    run(&mut w, 1).unwrap();
    // Work in both directions, so the run makes plenty of requests.
    set_password(&mut w, 0, vault, item, "from the desktop");
    w.sync(0);
    w.cloud.settle().unwrap();
    set_password(&mut w, 1, vault, item, "from the phone");
    w.advance(60_000);
    let (before, refreshed) = (server.requests().len(), server.refreshes());
    server.expire_access_after(3);
    run(&mut w, 1).expect("the run finishes after one refresh");
    assert_eq!(server.refreshes(), refreshed + 1, "exactly one refresh");
    let window: Vec<_> = server.requests()[before..]
        .iter()
        .filter(|r| r.path_and_query.starts_with("/2/"))
        .cloned()
        .collect();
    assert!(window.len() > 4, "the run made {} requests", window.len());
    let key = |r: &mosaic_sync::testutil::FakeRequest| {
        (
            r.path_and_query.clone(),
            r.body.clone(),
            r.header("dropbox-api-arg").map(str::to_owned),
        )
    };
    // The fourth request was answered 401 and sent again, once.
    assert_eq!(key(&window[3]), key(&window[4]));
    for pair in window.windows(2).enumerate() {
        if pair.0 != 3 {
            assert_ne!(key(&pair.1[0]), key(&pair.1[1]), "request {} twice", pair.0);
        }
    }
    converge(&mut w);
    assert_eq!(
        password_of(&w, 0, vault, item),
        password_of(&w, 1, vault, item)
    );
}

/// Review focus 3: the user revokes the app in the middle of a run.
#[test]
fn revoked_refresh_token_ends_run_signed_out() {
    let (mut w, vault, item) = desktop(34);
    let server = join_phone_dropbox_with(&mut w, refreshing_auth);
    run(&mut w, 1).unwrap();
    set_password(&mut w, 0, vault, item, "from the desktop");
    w.sync(0);
    w.cloud.settle().unwrap();
    set_password(&mut w, 1, vault, item, "from the phone");
    w.advance(60_000);
    server.revoke_after(4);
    assert!(matches!(
        run(&mut w, 1),
        Err(SyncError::Transport(TransportError::Unauthorized))
    ));
    // No half-written name, no temp file: an upload is whole or absent.
    let replica = w.replica(1);
    assert_eq!(
        crate::common::files_ending(&replica, ".mtmp"),
        Vec::<std::path::PathBuf>::new()
    );
    for file in crate::common::files_ending(&replica, "") {
        assert!(fs::metadata(&file).unwrap().len() > 0, "{}", file.display());
    }
    // It stays signed out.
    assert!(matches!(
        run(&mut w, 1),
        Err(SyncError::Transport(TransportError::Unauthorized))
    ));
}

/// Review focus 1: the desktop's files arrive late and out of order: after a
/// key rotation, the segments with the new data come before the one that
/// carries the new key.
#[test]
fn phone_waits_for_late_desktop_files_without_halting() {
    let (mut w, vault, item) = desktop(22);
    let _dav = join_phone(&mut w);
    run(&mut w, 1).unwrap();
    let before = sequences(&log_dir(&w, 0, 0, vault))
        .into_iter()
        .max()
        .unwrap();
    {
        let dev = &mut w.devs[0];
        dev.engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
    }
    set_password(&mut w, 0, vault, item, "edited");
    w.sync(0);
    set_password(&mut w, 0, vault, item, "edited twice");
    w.sync(0);
    let seqs = sequences(&log_dir(&w, 0, 0, vault));
    let key_seq = seqs.iter().copied().find(|s| *s > before).unwrap();
    assert!(seqs.iter().any(|s| *s > key_seq), "data follows the key");
    // The segment with the new key arrives three rounds after the rest.
    w.cloud.withhold(1, &format!("{key_seq:016x}.mseg"), 3);
    w.cloud.set_config(SimConfig::FLAKY);
    for round in 0..6 {
        w.cloud.round().unwrap();
        run(&mut w, 1).expect("no halt and no rollback while files are late");
        if round < 2 {
            assert_ne!(
                password_of(&w, 1, vault, item).as_deref(),
                Some("edited twice"),
                "the data cannot be opened before its key arrived"
            );
        }
        w.advance(60_000);
    }
    w.cloud.set_config(SimConfig::CALM);
    converge(&mut w);
    assert_eq!(
        password_of(&w, 1, vault, item).as_deref(),
        Some("edited twice")
    );
    assert_eq!(
        password_of(&w, 0, vault, item).as_deref(),
        Some("edited twice")
    );
}

/// Review focus 1: the provider's listing lags behind its own writes.
#[test]
fn stale_own_listing_does_not_halt() {
    let (mut w, vault, item) = desktop(23);
    let dav = join_phone(&mut w);
    run(&mut w, 1).unwrap();
    set_password(&mut w, 1, vault, item, "phone one");
    run(&mut w, 1).unwrap();
    // From now on listings show the folder as it is now.
    dav.set_faults(Faults {
        stale_listing: true,
        ..Faults::default()
    });
    for i in 0..3 {
        set_password(&mut w, 1, vault, item, &format!("phone {}", i + 2));
        w.advance(60_000);
        let report = run(&mut w, 1).expect("a lagging listing is not a rollback");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }
    // The lag was real: the newest own segment is on disk, not in the listing.
    let own = log_dir(&w, 1, 1, vault);
    let on_disk = segments(&own).len();
    let probe = webdav(&dav);
    let rel = own
        .strip_prefix(w.replica(1))
        .unwrap()
        .components()
        .fold(RelPath::root(), |p, c| {
            p.child(&c.as_os_str().to_string_lossy()).unwrap()
        });
    let listed = probe
        .list(&rel)
        .unwrap()
        .iter()
        .filter(|e| e.name.to_string_lossy().ends_with(".mseg"))
        .count();
    assert!(listed < on_disk, "{listed} listed, {on_disk} on disk");
    // Once the listing catches up, nothing is amiss either.
    dav.set_faults(Faults::default());
    run(&mut w, 1).unwrap();
    converge(&mut w);
    assert_eq!(password_of(&w, 0, vault, item).as_deref(), Some("phone 4"));
}

/// Review focus 2: both devices set the same field in the same round.
#[test]
fn desktop_and_phone_edit_the_same_item_and_converge() {
    let (mut w, vault, item) = desktop(24);
    let _dav = join_phone(&mut w);
    converge(&mut w);
    set_password(&mut w, 0, vault, item, "from the desktop");
    set_password(&mut w, 1, vault, item, "from the phone");
    w.sync(0);
    run(&mut w, 1).unwrap();
    converge(&mut w);
    let (a, b) = (
        password_of(&w, 0, vault, item).unwrap(),
        password_of(&w, 1, vault, item).unwrap(),
    );
    assert_eq!(a, b);
    assert!(a == "from the desktop" || a == "from the phone", "{a}");
    for replica in 0..2 {
        for needle in ["conflict", " (1)", "(2)"] {
            let found = crate::common::files_ending(&w.replica(replica), "")
                .into_iter()
                .filter(|p| p.to_string_lossy().contains(needle))
                .count();
            assert_eq!(found, 0, "replica {replica}: {needle}");
        }
    }
}

/// The sequence numbers of a device's own log, as written.
fn sequences(dir: &Path) -> Vec<u64> {
    segments(dir)
        .iter()
        .map(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            u64::from_str_radix(n.trim_end_matches(".mseg"), 16).unwrap()
        })
        .collect()
}

/// Review focus 4: a 429 comes in the middle of a run. Whatever request it hits
/// the run ends as rate limited, and the next one completes.
#[test]
fn rate_limit_mid_run_then_next_run_completes() {
    let mut hit_a_run = false;
    for nth in [2, 4, 5, 7, 9, 12] {
        let (mut w, vault, item) = desktop(25);
        let dav = join_phone(&mut w);
        run(&mut w, 1).unwrap();
        // The phone has an edit to push, and the desktop two to deliver.
        set_password(&mut w, 1, vault, item, "phone");
        let second = add_login(&mut w, 0, vault, "second");
        w.sync(0);
        w.cloud.settle().unwrap();
        dav.set_faults(Faults {
            fail_nth: Some((nth, 429, Some(30))),
            ..Faults::default()
        });
        match run(&mut w, 1) {
            Err(SyncError::Transport(TransportError::RateLimited { retry_after_s })) => {
                assert_eq!(retry_after_s, 30);
                hit_a_run = true;
                // Backing off: nothing more is asked of the provider yet.
                assert!(matches!(run(&mut w, 1), Err(SyncError::Backoff { .. })));
            }
            Ok(_) => {}
            Err(e) => panic!("request {nth}: {e:?}"),
        }
        dav.set_faults(Faults::default());
        w.advance(25 * 60 * 60 * 1000);
        run(&mut w, 1).unwrap();
        // The phone's own log has no duplicate and no skipped number
        // (reserved gaps aside, a gap is at most one number).
        let seqs = sequences(&log_dir(&w, 1, 1, vault));
        assert!(seqs.windows(2).all(|p| p[0] < p[1]), "{seqs:?}");
        assert!(seqs.windows(2).all(|p| p[1] - p[0] <= 2), "{seqs:?}");
        converge(&mut w);
        assert_eq!(password_of(&w, 0, vault, item).as_deref(), Some("phone"));
        assert!(password_of(&w, 1, vault, second).is_some());
    }
    assert!(hit_a_run, "no request number hit a run");
}

// ---- A hostile folder, on disk and over WebDAV ---------------------------------------------

/// What a run of the victim looked like, comparable between backends.
fn trace(label: &str, r: &Result<SyncReport, SyncError>) -> String {
    match r {
        Ok(rep) => format!(
            "{label}: ok pulled={} pushed={} warnings={} undecryptable={}",
            rep.pulled,
            rep.pushed,
            rep.warnings.len(),
            rep.undecryptable
        ),
        Err(e) => format!("{label}: {e:?}"),
    }
}

/// One run of the victim, noted in `out`.
fn step(w: &mut World, out: &mut Vec<String>, label: &str) -> Result<SyncReport, SyncError> {
    let r = run(w, 1);
    out.push(trace(label, &r));
    r
}

/// The planted-file scenarios of the hostile-folder tests, played against
/// device 1 of `w`; returns what happened.
fn hostile_scenario(w: &mut World, vault: VaultId, item: ItemId) -> Vec<String> {
    let mut out = Vec::new();
    step(w, &mut out, "first run").unwrap();

    // Conflict copies of the desktop's files under the names sync clients make.
    let dir = log_dir(w, 1, 0, vault);
    let bytes = fs::read(&segments(&dir)[0]).unwrap();
    for name in [
        "0000000000000001 (conflicted copy).mseg",
        "0000000000000001.sync-conflict-20260101-000000-ABCDEFG.mseg",
        "0000000000000001.MSEG",
        "desktop.ini",
    ] {
        fs::write(dir.join(name), &bytes).unwrap();
    }
    fs::write(
        device_dir(w, 1, 0).join("head (conflicted copy).mck"),
        b"junk",
    )
    .unwrap();
    step(w, &mut out, "conflict copies").unwrap();

    // The desktop's next segment arrives truncated, then whole.
    set_password(w, 0, vault, item, "edited");
    w.sync(0);
    w.cloud.settle().unwrap();
    let second = segments(&dir).into_iter().last().unwrap();
    let whole = fs::read(&second).unwrap();
    fs::write(&second, &whole[..whole.len() / 2]).unwrap();
    step(w, &mut out, "truncated").unwrap();
    out.push(format!("password {:?}", password_of(w, 1, vault, item)));
    fs::write(&second, &whole).unwrap();
    step(w, &mut out, "whole").unwrap();
    out.push(format!("password {:?}", password_of(w, 1, vault, item)));

    // A collection on the name of the desktop's next segment.
    let squat = dir.join("00000000000000ff.mseg");
    fs::create_dir_all(squat.join("inner")).unwrap();
    fs::write(squat.join("inner").join("x"), b"x").unwrap();
    step(w, &mut out, "collection on a foreign segment name").unwrap();
    out.push(format!(
        "foreign collection kept {}",
        squat.join("inner/x").is_file()
    ));

    // Squatters and a conflict copy in the victim's own directories.
    let own = log_dir(w, 1, 1, vault);
    let own_squat = own.join("00000000000000fe.mseg");
    fs::create_dir_all(own_squat.join("inner")).unwrap();
    fs::write(own.join(".0123456789abcdef.mtmp"), b"temp").unwrap();
    fs::write(
        device_dir(w, 1, 1).join("head (conflicted copy).mck"),
        b"junk",
    )
    .unwrap();
    let r = step(w, &mut out, "squatters in the own directories");
    out.push(format!("own run ok {}", r.is_ok()));
    out.push(format!("own squatter kept {}", own_squat.exists()));
    out.push(format!(
        "own temp kept {}",
        own.join(".0123456789abcdef.mtmp").exists()
    ));
    out
}

#[test]
fn malicious_folder_suite_over_dav() {
    let (mut w, vault, item) = desktop(26);
    join_local_phone(&mut w);
    let on_disk = hostile_scenario(&mut w, vault, item);

    let (mut w, vault, item) = desktop(26);
    let _dav = join_phone(&mut w);
    let over_dav = hostile_scenario(&mut w, vault, item);

    assert_eq!(over_dav, on_disk);
    // And the planted names did what the scenario says they should.
    assert!(over_dav.iter().any(|l| l.contains("password Some(\"pw\")")));
    assert!(
        over_dav
            .iter()
            .any(|l| l.contains("password Some(\"edited\")"))
    );
}

/// A phone and two desktops; the provider restores an old version of the
/// phone's files (its head and its newest segments are gone again).
#[test]
fn phone_and_two_desktops_roll_back_through_version_history() {
    let mut w = World::new(27, 3);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    let _dav = join_phone(&mut w);
    w.join(); // the second desktop, in replica 2
    let all = |w: &mut World| {
        for _ in 0..30 {
            w.cloud.settle().unwrap();
            let a = w.sync(0);
            let b = run(w, 1).unwrap();
            let c = w.sync(2);
            w.advance(60_000);
            if is_idle(&a) && is_idle(&b) && is_idle(&c) {
                return;
            }
        }
        panic!("no convergence");
    };
    all(&mut w);
    // The provider's old version: the phone's own files now.
    let own = log_dir(&w, 1, 1, vault);
    let head = device_dir(&w, 1, 1).join("head.mck");
    let old_head = fs::read(&head).unwrap();
    let old_segments: Vec<_> = segments(&own);
    for i in 0..3 {
        set_password(&mut w, 1, vault, item, &format!("phone {i}"));
        w.advance(60_000);
        run(&mut w, 1).unwrap();
    }
    all(&mut w);
    assert!(segments(&own).len() > old_segments.len());
    let restore = |w: &World| {
        fs::write(&head, &old_head).unwrap();
        for p in segments(&own) {
            if !old_segments.contains(&p) {
                fs::remove_file(p).unwrap();
            }
        }
        w.advance(1000);
    };
    restore(&w);
    run(&mut w, 1).ok(); // one lost upload is tolerated
    restore(&w);
    let err = run(&mut w, 1).unwrap_err();
    assert!(matches!(err, SyncError::RollbackDetected), "{err:?}");
    // The phone stays stopped until the user acts; the desktops go on.
    assert!(matches!(
        run(&mut w, 1).unwrap_err(),
        SyncError::RollbackDetected
    ));
    w.sync(0);
    w.sync(2);
}
