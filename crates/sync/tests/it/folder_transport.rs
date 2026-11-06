//! The folder transport against the failures of file-sync clients:
//! partial files, out-of-order arrival, conflict copies, lost segments,
//! compaction and cleanup (ADR-0043).

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration tests"
)]

use std::fs;
use std::path::PathBuf;

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{Argon2Profile, ItemId, VaultId};
use mosaic_sync::engine::{SyncError, SyncWarning};
use mosaic_sync::folder::{self, FolderContents, FolderError};
use mosaic_sync::testutil::LocalSession;
use mosaic_sync::transport::{Cursor, Transport, TransportError};
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::checkpoint::Checkpoint;
use mosaic_sync::wire::cursor::FolderCursor;
use mosaic_sync::wire::folder_names::dir_name;
use mosaic_sync::wire::segment::{Segment, SegmentPath};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use crate::common::{
    PASSWORD, World, account_dir, device_dir, files_ending, log_dir, open_transport, pw, segments,
    tree,
};

const HOUR_MS: u64 = 3_600_000;

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

fn password_of(w: &World, d: usize, vault: VaultId, item: ItemId) -> String {
    w.devs[d]
        .s()
        .item(vault, item)
        .unwrap()
        .secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

/// Two devices that both have the vault and one item, at rest. Replica 1's
/// device has synced everything.
fn two_devices() -> (World, VaultId, ItemId) {
    let mut w = World::new(11, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.converge(0);
    (w, vault, item)
}

#[test]
fn writes_are_tmp_then_rename() {
    let (mut w, vault, item) = two_devices();
    set_password(&mut w, 0, vault, item, "second");
    w.sync(0);
    // Nothing of the temp files is left in any directory.
    assert_eq!(files_ending(&w.replica(0), ".mtmp").len(), 0);

    // A write-once name that exists is never overwritten: the push fails,
    // the original stays and no temp file is left.
    let dir = log_dir(&w, 0, 0, vault);
    let last = segments(&dir).pop().unwrap();
    let before = fs::read(&last).unwrap();
    let name = last.file_name().unwrap().to_string_lossy().into_owned();
    let seq = u64::from_str_radix(name.trim_end_matches(".mseg"), 16).unwrap();
    // The next names are taken by non-empty directories this device never
    // wrote: they are removed, the push goes through, nothing is overwritten
    // and no temp file is left.
    let blockers: Vec<PathBuf> = (1..=16)
        .map(|k| dir.join(format!("{:016x}.mseg", seq + k)))
        .collect();
    for b in &blockers {
        fs::create_dir_all(b.join("in")).unwrap();
        fs::write(b.join("in").join("f"), b"x").unwrap();
    }
    set_password(&mut w, 0, vault, item, "third");
    w.sync(0);
    assert_eq!(files_ending(&w.replica(0), ".mtmp").len(), 0);
    assert_eq!(fs::read(&last).unwrap(), before);
    assert!(blockers.iter().all(|b| !b.is_dir()));

    // A checkpoint is replaced whole.
    let head = device_dir(&w, 0, 0).join("head.mck");
    let old = fs::read(&head).unwrap();
    w.advance(1000);
    // Backing off after the failure: move past it.
    w.advance(3_600_000);
    w.sync(0);
    let new = fs::read(&head).unwrap();
    assert_ne!(old, new);
    assert_eq!(new.len(), old.len());
    assert_eq!(files_ending(&w.replica(0), ".mtmp").len(), 0);
}

#[test]
fn ignores_conflict_copies_of_every_provider() {
    let mut w = World::new(12, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();

    // Copies of A's segment and head under the names sync clients make.
    let dir = log_dir(&w, 1, 0, vault);
    let originals = segments(&dir);
    assert_ne!(originals.len(), 0);
    let bytes = fs::read(&originals[0]).unwrap();
    let stem = "0000000000000001";
    for name in [
        format!("{stem} (conflicted copy).mseg"),
        format!("{stem} (1).mseg"),
        format!("{stem} 2.mseg"),
        format!("{stem}-DESKTOP-ABC123.mseg"),
        format!("{stem}.sync-conflict-20260101-000000-ABCDEFG.mseg"),
        format!("{stem} (Case Conflict).mseg"),
        format!("{stem} (conflicted copy 2026-01-01 120000).mseg"),
        format!("{stem}.MSEG"),
        format!("{}.mseg", stem.to_uppercase()),
        "desktop.ini".to_owned(),
        ".DS_Store".to_owned(),
        "~$0000000000000001.mseg".to_owned(),
        ".dropbox.cache".to_owned(),
    ] {
        fs::write(dir.join(name), &bytes).unwrap();
    }
    let report = w.sync(1);
    assert_eq!(report.pulled, 1, "{report:?}");
    assert_eq!(report.warnings.len(), 0);
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("first")
    );
    // Syncing again changes nothing.
    assert!(crate::common::is_idle(&w.sync(1)));

    // A conflict copy of a checkpoint is a candidate: a junk copy is
    // ignored, a verified one counts, and the highest counter wins.
    let head_dir = device_dir(&w, 1, 0);
    fs::write(head_dir.join("head (conflicted copy).mck"), b"junk").unwrap();
    let mut t = open_transport(&w.devs[1].access, &w.replica(1), &w.clock, "probe");
    let cps = t.checkpoints().unwrap();
    assert!(cps.len() >= 3, "head, the junk copy and the reader's own");
    assert!(w.try_sync(1).is_ok());
}

#[test]
fn partial_segment_retried_not_skipped() {
    let mut w = World::new(13, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();

    // Every segment of A arrives truncated.
    let dir = log_dir(&w, 1, 0, vault);
    let full: Vec<(PathBuf, Vec<u8>)> = segments(&dir)
        .into_iter()
        .map(|p| {
            let b = fs::read(&p).unwrap();
            (p, b)
        })
        .collect();
    for (p, b) in &full {
        fs::write(p, &b[..b.len() / 2]).unwrap();
    }
    let report = w.sync(1);
    assert_eq!(report.pulled, 0);
    assert!(w.devs[1].s().item(vault, item).is_err(), "nothing applied");
    assert!(report.warnings.is_empty(), "within the grace");
    // Still partial: still retried, still nothing.
    assert_eq!(w.sync(1).pulled, 0);
    // The full files arrive.
    for (p, b) in &full {
        fs::write(p, b).unwrap();
    }
    let report = w.sync(1);
    assert_eq!(report.pulled, 1);
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("first")
    );
}

#[test]
fn out_of_order_segments_read_once() {
    let (mut w, vault, item) = two_devices();
    let dir = log_dir(&w, 1, 0, vault);
    let base = segments(&dir).len() as u64;

    // A writes two more segments; replica 1 gets the later one first.
    set_password(&mut w, 0, vault, item, "one");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.advance(10 * 60 * 1000);
    w.devs[0]
        .s_mut()
        .update_item(vault, item, |e| e.set_title("two"))
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    let all = segments(&dir);
    assert_eq!(all.len() as u64, base + 2);
    let early = all[all.len() - 2].clone();
    let early_bytes = fs::read(&early).unwrap();
    fs::remove_file(&early).unwrap();

    let a_id = w.devs[0].s().install_id();
    let cursor_of = |w: &World| {
        let raw = w.devs[1].s().sync_cursor(vault).unwrap().unwrap();
        FolderCursor::decode(&raw).unwrap()
    };
    let r = w.sync(1);
    assert_eq!(r.pulled, 1);
    let cur = cursor_of(&w);
    let d = cur.device(&a_id).unwrap();
    assert_eq!(d.contiguous, base);
    assert_eq!(d.above, vec![base + 2], "the later segment is remembered");

    // The earlier one arrives: it alone is read, and the cursor closes up.
    fs::write(&early, early_bytes).unwrap();
    // (The record in it is older than the one already applied: ignored.)
    let r = w.sync(1);
    assert_eq!(r.pulled, 0);
    let cur = cursor_of(&w);
    let d = cur.device(&a_id).unwrap();
    assert_eq!((d.contiguous, d.above.len()), (base + 2, 0));
    assert!(crate::common::is_idle(&w.sync(1)));
    assert_eq!(password_of(&w, 1, vault, item), "one");
    assert_eq!(
        w.devs[1].s().item(vault, item).unwrap().title(),
        Some("two")
    );
}

#[test]
fn gap_reported_as_missing_files_after_24h() {
    let (mut w, vault, item) = two_devices();
    // A writes a segment and its checkpoint; the segment never reaches B.
    set_password(&mut w, 0, vault, item, "lost on the way");
    w.sync(0);
    let seg = segments(&log_dir(&w, 0, 0, vault)).pop().unwrap();
    let seg_name = seg.file_name().unwrap().to_string_lossy().into_owned();
    w.cloud.withhold(1, &seg_name, 10_000);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    assert!(
        !segments(&log_dir(&w, 1, 0, vault))
            .iter()
            .any(|p| p.ends_with(&seg_name))
    );

    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "within the grace: {r:?}");
    w.advance(23 * HOUR_MS);
    assert_eq!(w.sync(1).warnings.len(), 0);
    w.advance(2 * HOUR_MS);
    let r = w.sync(1);
    let a_id = w.devs[0].s().install_id();
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::MissingFiles { device, .. } if *device == a_id)),
        "{r:?}"
    );
}

#[test]
fn snapshot_heals_lost_segment() {
    let (mut w, vault, item) = two_devices();
    // A writes a segment that is lost for good (B never gets it), then many
    // more that B reads.
    set_password(&mut w, 0, vault, item, "v0");
    w.sync(0);
    // (Deleting it from A's own replica would be a rollback of A's log: A
    // halts. The file is lost on the way to B instead.)
    let lost = segments(&log_dir(&w, 0, 0, vault)).pop().unwrap();
    let a_dir = dir_name(w.devs[0].s().install_id().as_bytes());
    let needle = format!(
        "{}/log/{a_dir}/{}",
        dir_name(vault.as_bytes()),
        lost.file_name().unwrap().to_string_lossy()
    );
    w.cloud.set_config(mosaic_sync::testutil::SimConfig::CALM);
    w.cloud.withhold(1, &needle, 1_000_000);
    for i in 1..=12 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
    }
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    w.sync(1);
    w.advance(25 * HOUR_MS);
    let r = w.sync(1);
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::MissingFiles { .. })),
        "the gap is reported: {r:?}"
    );

    // A keeps writing until its log is compacted into a snapshot.
    for i in 13..=70 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
    }
    w.sync(0);
    let files = segments(&log_dir(&w, 0, 0, vault));
    assert!(
        files.len() < 10,
        "older segments were deleted: {}",
        files.len()
    );
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "the snapshot closed the gap: {r:?}");
    assert_eq!(password_of(&w, 1, vault, item), "v70");
}

#[test]
fn compaction_deletes_only_own_older_segments() {
    let (mut w, vault, item) = two_devices();
    // B writes a few segments of its own.
    for i in 0..3 {
        w.devs[1]
            .s_mut()
            .update_item(vault, item, |e| e.add_tag(&format!("b{i}")))
            .unwrap();
        w.sync(1);
    }
    w.cloud.settle().unwrap();
    w.sync(0);
    let b_dir = log_dir(&w, 0, 1, vault);
    let b_before = tree(&b_dir);
    assert_eq!(b_before.len(), 3);
    for i in 0..70 {
        set_password(&mut w, 0, vault, item, &format!("a{i}"));
        w.sync(0);
    }
    w.sync(0);
    let a_dir = log_dir(&w, 0, 0, vault);
    let a_files = segments(&a_dir);
    assert!(a_files.len() < 10, "A compacted: {}", a_files.len());
    assert_eq!(tree(&b_dir), b_before, "B's segments are untouched");

    // The newest segments are a complete snapshot set.
    let keys = SessionCrypto::new(w.devs[0].access.clone()).unwrap();
    let install = w.devs[0].s().install_id();
    let mut snapshots = 0;
    for p in &a_files {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let seq = u64::from_str_radix(name.trim_end_matches(".mseg"), 16).unwrap();
        let segment = Segment::decode(
            &fs::read(p).unwrap(),
            &SegmentPath {
                account_id: w.account,
                vault_id: vault,
                device_id: install,
                seq,
            },
            &keys,
        )
        .unwrap();
        if let Some(info) = segment.snapshot {
            snapshots += 1;
            assert!(info.base <= seq && seq < info.base + u64::from(info.parts));
        }
    }
    assert!(snapshots >= 1);
}

#[test]
fn own_conflicted_head_with_higher_counter_is_identity_duplicated() {
    let (mut w, _vault, _item) = two_devices();
    w.sync(0);
    let counter = w.devs[0].s().checkpoint_counter().unwrap();
    // A verified checkpoint of this install with a counter this device never
    // reached, filed as a conflict copy: another profile writes under the
    // same install ID.
    let own_install = w.devs[0].s().install_id();
    let forged = Checkpoint {
        device_id: own_install,
        counter: counter + 5,
        issued_at_ms: 1,
        entries: Vec::new(),
    }
    .encode(&*w.devs[0].s())
    .unwrap();
    let dir = device_dir(&w, 0, 0);
    fs::write(dir.join("head (conflicted copy).mck"), forged).unwrap();
    let err = w.try_sync(0).unwrap_err();
    assert!(matches!(err, SyncError::IdentityDuplicated), "{err:?}");
    // Automatic sync stays stopped until the user acts.
    assert!(matches!(
        w.try_sync(0).unwrap_err(),
        SyncError::IdentityDuplicated
    ));
    let dev = &mut w.devs[0];
    dev.engine.clear_halt(&mut dev.access).unwrap();
    fs::remove_file(dir.join("head (conflicted copy).mck")).unwrap();
    assert!(w.try_sync(0).is_ok());
}

#[test]
fn temp_and_conflict_files_cleaned_in_own_dirs_only() {
    let (mut w, vault, _item) = two_devices();
    w.sync(0);
    let (own_dev, own_log) = (device_dir(&w, 0, 0), log_dir(&w, 0, 0, vault));
    let (other_dev, other_log) = (device_dir(&w, 0, 1), log_dir(&w, 0, 1, vault));
    fs::create_dir_all(&other_log).unwrap();
    let junk = [
        (&own_dev, ".0123456789abcdef.mtmp"),
        (&own_dev, "head (conflicted copy).mck"),
        (&own_dev, "info (1).mdev"),
        (&own_log, ".fedcba9876543210.mtmp"),
        (&other_dev, ".0123456789abcdef.mtmp"),
        (&other_dev, "head (conflicted copy).mck"),
        (&other_dev, "info (1).mdev"),
        (&other_log, ".fedcba9876543210.mtmp"),
    ];
    for (dir, name) in junk {
        fs::write(dir.join(name), b"x").unwrap();
    }
    // A young temp file is left alone (it may be a write in progress).
    w.sync(0);
    assert!(own_dev.join(".0123456789abcdef.mtmp").exists());
    assert!(!own_dev.join("head (conflicted copy).mck").exists());
    assert!(!own_dev.join("info (1).mdev").exists());

    w.advance(25 * HOUR_MS);
    w.sync(0);
    assert!(!own_dev.join(".0123456789abcdef.mtmp").exists());
    assert!(!own_log.join(".fedcba9876543210.mtmp").exists());
    // The other device's directories are not ours to clean.
    for (dir, name) in junk.iter().skip(4) {
        assert!(dir.join(name).exists(), "{name} of the other device");
    }
}

#[test]
fn other_account_dirs_untouched() {
    let mut w = World::new(14, 2);
    let vault = w.new_vault(0, "Mine");
    add_login(&mut w, 0, vault, "x");
    w.sync(0);

    // Another account in the same folder.
    let dir_b = tempfile::tempdir().unwrap();
    let (session_b, _created) =
        LockedSession::open(Store::open(dir_b.path().join("b.db")).unwrap())
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
    let access_b = LocalSession::new(session_b);
    let mut transport_b = open_transport(&access_b, &w.replica(0), &w.clock, "other");
    let boot = access_b.session().unwrap().account_bootstrap().unwrap();
    transport_b
        .write_account_file(&boot.params, &boot.wrapped_ak, 1)
        .unwrap();
    let other = w.replica(0).join("a").join(dir_name(
        access_b.session().unwrap().account_id().as_bytes(),
    ));
    let before = tree(&other);
    assert!(!before.is_empty());

    for _ in 0..3 {
        w.advance(25 * HOUR_MS);
        w.sync(0);
    }
    assert_eq!(tree(&other), before);
    // And the other account is not seen as ours.
    assert_eq!(folder::list_accounts(&w.replica(0)).len(), 2);
}

#[test]
fn non_empty_foreign_folder_refused() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("notes.txt"), b"my notes").unwrap();
    assert_eq!(folder::inspect(dir.path()), FolderContents::Foreign);
    assert_eq!(folder::init(dir.path()), Err(FolderError::NotEmpty));
    let listing: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(listing.len(), 1, "nothing was written");

    // A marker of another version is not ours either.
    let other = tempfile::tempdir().unwrap();
    fs::write(
        other.path().join("mosaic-sync.json"),
        br#"{"format":"mosaic-folder","version":2}"#,
    )
    .unwrap();
    assert_eq!(folder::init(other.path()), Err(FolderError::Unsupported));
    assert_eq!(fs::read_dir(other.path()).unwrap().count(), 1);

    // Opening a transport on either refuses too.
    let w = World::new(15, 1);
    let access = w.devs[0].access.clone();
    let crypto = SessionCrypto::new(access.clone()).unwrap();
    let account = crypto.account_id();
    let install = mosaic_sync::SyncCrypto::install_id(&crypto);
    let opened = folder::FolderTransport::open(folder::FolderConfig {
        root: dir.path().to_owned(),
        account_id: account,
        install_id: install,
        crypto,
        clock: std::sync::Arc::new(w.clock.clone()),
        device: mosaic_sync::wire::device_info::DeviceInfo {
            created_at_ms: 0,
            name: "x".to_owned(),
            platform: mosaic_sync::wire::device_info::Platform::Linux,
        },
    });
    assert!(matches!(opened, Err(TransportError::FolderUnavailable)));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    // A cursor with garbage is a protocol error, not a panic.
    let mut t = open_transport(&access, &w.replica(0), &w.clock, "x");
    let bad = t.pull(VaultId::new_v7().unwrap(), &Cursor(vec![9, 9, 9]), 10);
    assert!(matches!(bad, Err(TransportError::Protocol)));
}

#[test]
fn expired_deleted_blob_is_purged_only_when_unreferenced_and_caught_up() {
    let (mut w, vault, item) = two_devices();
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.converge(0);
    let blob = account_dir_of(&w, 0)
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("blobs")
        .join(format!("{}.matt", dir_name(id.as_bytes())));
    assert!(blob.exists());

    // Device 0 removes the attachment: a marker is written, the blob stays.
    w.devs[0]
        .s_mut()
        .remove_attachment(vault, item, id)
        .unwrap();
    w.converge(0);
    assert!(blob.exists(), "kept for the retention period");
    w.advance(29 * 24 * HOUR_MS);
    w.sync(0);
    assert!(blob.exists(), "still within 30 days");

    // After 30 days nothing refers to it: the blob and the marker go.
    w.advance(2 * 24 * HOUR_MS);
    w.converge(0);
    w.sync(0);
    assert!(!blob.exists(), "purged");
    let markers = files_ending(&w.replica(0), ".mdel");
    assert!(markers.is_empty(), "{markers:?}");
}

fn account_dir_of(w: &World, replica: usize) -> PathBuf {
    account_dir(w, replica)
}

#[test]
fn waiting_for_a_change_returns_a_new_token_after_a_write() {
    use core::time::Duration;
    use mosaic_sync::transport::Transport;

    let (mut w, vault, item) = two_devices();
    let t = w.devs[1].engine.transport_mut();
    // No sleeping: a zero timeout looks once.
    let first = t.wait_for_change(None, Duration::ZERO).unwrap();
    let same = t.wait_for_change(Some(&first), Duration::ZERO).unwrap();
    assert_eq!(first, same, "nothing changed");
    set_password(&mut w, 0, vault, item, "changed");
    w.sync(0);
    w.cloud.settle().unwrap();
    let t = w.devs[1].engine.transport_mut();
    let after = t.wait_for_change(Some(&first), Duration::ZERO).unwrap();
    assert_ne!(first, after);
    let devices = t.devices().unwrap();
    assert_eq!(devices.len(), 2);
    assert!(
        devices
            .iter()
            .all(|d| d.name.is_some() && d.last_seen_ms.is_some())
    );
}
