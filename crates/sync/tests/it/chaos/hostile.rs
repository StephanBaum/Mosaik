//! A hostile or faulty folder: rolled-back directories, replayed and hidden
//! checkpoints, withheld vaults, files planted by an outsider or by a
//! holder of the account key, and writes that fail half-way.

use std::fs;

use mosaic_core::{AccountKey, AttachmentId, InstallId, KeyGeneration, VaultId, VaultKey};
use mosaic_sync::engine::{EngineConfig, SyncEngine, SyncError};
use mosaic_sync::testutil::{FixedJitter, MaliciousTransport, SimConfig};
use mosaic_sync::transport::{Transport, TransportKind};
use mosaic_sync::wire::KeyedCrypto;
use mosaic_sync::wire::blob_delete::BlobDelete;
use mosaic_sync::wire::checkpoint::{Checkpoint, CheckpointEntry};
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_sync::wire::folder_names::{blob_delete_name, blob_name, dir_name, segment_name};
use mosaic_sync::wire::segment::{Segment, SegmentObject};

use super::util::{
    DAY_MS, Faulty, HOUR_MS, add_login, max_seq, password_of, relock, segment_needle, seq_of,
    set_password, shows, warns_missing,
};
use crate::common::{World, account_dir, device_dir, is_idle, log_dir, open_transport, segments};

/// Three devices at rest sharing one vault with one item.
fn three_devices(seed: u64) -> (World, VaultId, mosaic_core::ItemId) {
    let mut w = World::new(seed, 3);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    (w, vault, item)
}

fn install(w: &World, d: usize) -> InstallId {
    w.devs[d].s().install_id()
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------

/// The provider rolls back A's log directory (a restore of an older folder
/// state, or anyone with write access deleting A's newest segments). A
/// persisted its sequence numbers before it wrote them, so after a relock it
/// notices that segments it wrote are gone and halts before it pushes
/// anything or rewrites its checkpoint. Once the user acknowledges, it
/// numbers its next segments above everything it ever used: the readers
/// that marked the lost numbers read still get A's new edits.
#[test]
fn rolled_back_device_log_reuses_sequence_numbers() {
    let (mut w, vault, item) = three_devices(301);
    for i in 0..4 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
    }
    w.converge(0);
    for d in 1..3 {
        assert_eq!(password_of(&w, d, vault, item).as_deref(), Some("v3"));
    }
    // A's three newest segments vanish from the folder.
    let dir = log_dir(&w, 0, 0, vault);
    let top = max_seq(&dir);
    let segs = segments(&dir);
    for p in &segs[segs.len() - 3..] {
        fs::remove_file(p).unwrap();
    }
    w.cloud.settle().unwrap();
    relock(&mut w, 0);
    set_password(&mut w, 0, vault, item, "after the rollback");
    let head_before = fs::read(device_dir(&w, 0, 0).join("head.mck")).unwrap();
    let r = w.try_sync(0);
    assert!(matches!(r, Err(SyncError::RollbackDetected)), "{r:?}");
    assert!(max_seq(&dir) < top, "nothing was written");
    assert_eq!(
        fs::read(device_dir(&w, 0, 0).join("head.mck")).unwrap(),
        head_before,
        "the checkpoint was not rewritten"
    );
    // The halt survives a relock.
    relock(&mut w, 0);
    let r = w.try_sync(0);
    assert!(matches!(r, Err(SyncError::RollbackDetected)), "{r:?}");
    // The user acknowledges.
    {
        let dev = &mut w.devs[0];
        assert!(dev.engine.halted(&mut dev.access).unwrap().is_some());
        dev.engine.clear_halt(&mut dev.access).unwrap();
    }
    w.sync(0);
    assert!(
        segments(&dir)
            .iter()
            .all(|p| seq_of(p) <= top - 3 || seq_of(p) > top),
        "no sequence number of the lost segments was used again"
    );
    assert!(max_seq(&dir) > top);
    w.converge(1);
    w.advance(25 * HOUR_MS);
    let reports = w.sync_all();
    for (d, report) in reports.iter().enumerate().skip(1) {
        assert!(
            shows(&w, d, vault, item, "after the rollback")
                || warns_missing(report, install(&w, 0)),
            "device {d}: A's edit after the rollback neither arrived nor was reported: {:?} / {report:?}",
            password_of(&w, d, vault, item),
        );
    }
}

/// A checkpoint write fails twice in a row (on Windows a sync client that
/// holds `head.mck` open makes the rename fail). The engine persisted each
/// counter before the write, so the next run sees its own checkpoint below
/// its counter for the second time and halts with `RollbackDetected`,
/// although nothing was rolled back.
#[test]
fn two_failed_checkpoint_writes_are_not_a_rollback() {
    let (mut w, vault, item) = three_devices(302);
    let mut engine = SyncEngine::with_jitter(
        Faulty::new(open_transport(
            &w.devs[0].access,
            &w.replica(0),
            &w.clock,
            "device 0",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    engine.transport_mut().faults.fail_put_checkpoint = 2;
    for attempt in 0..2 {
        set_password(&mut w, 0, vault, item, &format!("try {attempt}"));
        let dev = &mut w.devs[0];
        let r = engine.sync_once(&mut dev.access, &w.clock);
        assert!(
            matches!(r, Err(SyncError::Transport(_))),
            "attempt {attempt}: {r:?}"
        );
        w.advance(20 * 60 * 1000);
    }
    // The folder works again.
    let dev = &mut w.devs[0];
    let r = engine.sync_once(&mut dev.access, &w.clock);
    assert!(r.is_ok(), "no rollback happened, the writes failed: {r:?}");
}

/// B saw A's checkpoint ahead of what it received (lag started), then the
/// folder serves A's older, valid `head.mck` again while it keeps
/// withholding A's segment. The older checkpoint replaces the newer one, the
/// lag is forgotten and the grace never runs out.
#[test]
fn replayed_old_checkpoint_does_not_reset_freeze_detection() {
    let (mut w, vault, item) = three_devices(303);
    let a = install(&w, 0);
    let old_head = fs::read(device_dir(&w, 1, 0).join("head.mck")).unwrap();
    let mut engine = SyncEngine::with_jitter(
        MaliciousTransport::new(open_transport(
            &w.devs[1].access,
            &w.replica(1),
            &w.clock,
            "device 1",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    set_password(&mut w, 0, vault, item, "withheld");
    w.sync(0);
    let seq = seq_of(segments(&log_dir(&w, 0, 0, vault)).last().unwrap());
    w.cloud.set_config(SimConfig::CALM);
    w.cloud
        .withhold(1, &segment_needle(&w, 0, vault, seq), 10_000);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let b = &mut w.devs[1];
    let r = engine.sync_once(&mut b.access, &w.clock).unwrap();
    assert!(r.warnings.is_empty(), "{r:?}");
    w.advance(12 * HOUR_MS);
    let b = &mut w.devs[1];
    engine.sync_once(&mut b.access, &w.clock).unwrap();
    // From now on the folder serves A's older head.
    engine.transport_mut().replace_checkpoints = vec![(a, old_head)];
    let mut warned = false;
    for _ in 0..4 {
        w.advance(6 * HOUR_MS);
        let b = &mut w.devs[1];
        let r = engine.sync_once(&mut b.access, &w.clock).unwrap();
        warned |= warns_missing(&r, a);
    }
    assert_ne!(password_of(&w, 1, vault, item).as_deref(), Some("withheld"));
    assert!(warned, "A's segment has been missing for 36 hours");
}

/// The lag of another device is kept in the engine's memory only, so a
/// device that relocks (or restarts) more often than the 24-hour grace never
/// reports files that stay missing.
#[test]
fn missing_files_are_reported_across_relocks() {
    let (mut w, vault, item) = three_devices(304);
    let a = install(&w, 0);
    set_password(&mut w, 0, vault, item, "withheld");
    w.sync(0);
    let seq = seq_of(segments(&log_dir(&w, 0, 0, vault)).last().unwrap());
    w.cloud.set_config(SimConfig::CALM);
    w.cloud
        .withhold(1, &segment_needle(&w, 0, vault, seq), 10_000);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let mut warned = false;
    for _ in 0..8 {
        warned |= warns_missing(&w.sync(1), a);
        // The user locks the app every evening and unlocks it the next day.
        w.advance(8 * HOUR_MS);
        relock(&mut w, 1);
    }
    assert!(warned, "A's segment has been missing for 64 hours");
}

/// A creates a vault; the folder withholds the whole vault directory from
/// B. A's checkpoint lists the vault, but B only compares entries of vaults
/// it already holds, so nothing is ever reported.
#[test]
fn withheld_new_vault_is_reported() {
    let (mut w, _vault, _item) = three_devices(305);
    let a = install(&w, 0);
    let fresh = w.new_vault(0, "Fresh");
    add_login(&mut w, 0, fresh, "in the fresh vault");
    w.sync(0);
    w.cloud.set_config(SimConfig::CALM);
    w.cloud.withhold(1, &dir_name(fresh.as_bytes()), 10_000);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let mut warned = false;
    for _ in 0..4 {
        warned |= warns_missing(&w.sync(1), a);
        w.advance(9 * HOUR_MS);
    }
    warned |= warns_missing(&w.sync(1), a);
    assert!(
        !w.devs[1].s().vaults().iter().any(|v| v.vault_id == fresh),
        "withheld"
    );
    assert!(warned, "a whole vault of A has been missing for 36 hours");
}

/// Two devices rotate concurrently. The loser learns of the winner's key
/// and its run fails while it rotates again (a transport error); then it
/// relocks. The conflict was kept in memory only and the winner's key
/// segment is marked read, so the loser never rotates: it keeps the losing
/// key for good, and each side's records stay unreadable to the other,
/// without a warning (the run that saw it failed; the others count the
/// loser's records as an expected conflict).
#[test]
fn key_conflict_survives_a_failed_run_and_relock() {
    let (mut w, vault, item) = three_devices(306);
    let other = add_login(&mut w, 0, vault, "second");
    w.converge(0);
    let (winner, loser) = if install(&w, 0).as_bytes() < install(&w, 1).as_bytes() {
        (0, 1)
    } else {
        (1, 0)
    };
    set_password(&mut w, winner, vault, item, "by the winner");
    let g = w.devs[winner].s_mut().rotate_vault_key(vault).unwrap();
    set_password(&mut w, loser, vault, other, "by the loser");
    assert_eq!(w.devs[loser].s_mut().rotate_vault_key(vault).unwrap(), g);
    w.sync(winner);
    w.cloud.settle().unwrap();
    // The loser's run: its own queued key goes out (call 1), it reads the
    // winner's key, and the rotation it starts fails (call 2).
    let mut engine = SyncEngine::with_jitter(
        Faulty::new(open_transport(
            &w.devs[loser].access,
            &w.replica(loser),
            &w.clock,
            "loser",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    engine.transport_mut().faults.fail_push_key_call = Some(2);
    let dev = &mut w.devs[loser];
    let r = engine.sync_once(&mut dev.access, &w.clock);
    assert!(matches!(r, Err(SyncError::Transport(_))), "{r:?}");
    drop(engine);
    relock(&mut w, loser);
    w.advance(60_000);
    w.converge(2);
    let current = |d: usize| {
        let keys = w.devs[d].s().store().vault_keys(&vault).unwrap();
        keys.iter()
            .max_by_key(|(g, _)| *g)
            .map(|(g, k)| (*g, k.as_bytes().to_vec()))
            .unwrap()
    };
    // (generation, same key as the winner, winner's edit, loser's edit,
    // warnings of the last run)
    let state: Vec<_> = (0..3)
        .map(|d| {
            (
                current(d).0.get(),
                current(d) == current(winner),
                shows(&w, d, vault, item, "by the winner"),
                shows(&w, d, vault, other, "by the loser"),
                w.devs[d].last.warnings.len(),
            )
        })
        .collect();
    assert!(
        state.iter().all(|s| s.1 && s.2 && s.3),
        "winner {winner}, loser {loser}: per device (generation, winner's key, winner's edit, loser's edit, warnings) = {state:?}"
    );
}

/// Somebody without the account key drops a junk file named
/// `ffffffffffffffff.mseg` into A's log directory. A numbers its next
/// segment after the highest name on disk, which overflows: every push of A
/// fails with `Io` from then on, and A cannot write again until a human
/// removes the file (A never removes unverifiable files from its own log).
#[test]
fn junk_file_at_the_top_of_the_sequence_range_does_not_stop_the_writer() {
    let (mut w, vault, item) = three_devices(319);
    let dir = log_dir(&w, 1, 0, vault);
    fs::write(dir.join("ffffffffffffffff.mseg"), b"junk").unwrap();
    w.cloud.settle().unwrap();
    set_password(&mut w, 0, vault, item, "after the junk");
    let r = w.try_sync(0);
    assert!(r.is_ok(), "A can still write: {r:?}");
    w.converge(0);
    for d in 0..3 {
        assert!(shows(&w, d, vault, item, "after the junk"), "device {d}");
    }
}

// ---------------------------------------------------------------------------
// Passing scenarios
// ---------------------------------------------------------------------------

/// Provider conflict copies of `head.mck`: an older and a truncated copy in
/// the owner's directory, and in a reader's replica a copy that holds the
/// newest checkpoint while `head.mck` holds an older one (the provider kept
/// the wrong file). Nobody halts or warns, the owner removes its copies,
/// and the reader uses the newest checkpoint.
#[test]
fn conflict_copies_of_head_confuse_nobody() {
    let (mut w, vault, item) = three_devices(311);
    let a = install(&w, 0);
    let old = fs::read(device_dir(&w, 0, 0).join("head.mck")).unwrap();
    for i in 0..3 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.advance(1000);
        w.sync(0);
    }
    w.converge(0);
    let own = device_dir(&w, 0, 0);
    fs::write(own.join("head (conflicted copy 1).mck"), &old).unwrap();
    fs::write(own.join("head-DESKTOP-0001.mck"), &old[..old.len() / 2]).unwrap();
    // In replica 2 the provider kept the old file as head.mck and the newest
    // one as a conflict copy.
    let theirs = device_dir(&w, 2, 0);
    let newest = fs::read(theirs.join("head.mck")).unwrap();
    fs::write(theirs.join("head.mck"), &old).unwrap();
    fs::write(
        theirs.join("head.sync-conflict-20260101-000001-ABCDEFG.mck"),
        &newest,
    )
    .unwrap();
    for d in [0, 2, 1] {
        let r = w.sync(d);
        assert!(r.warnings.is_empty(), "device {d}: {r:?}");
    }
    let left: Vec<_> = fs::read_dir(&own)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| {
            std::path::Path::new(n)
                .extension()
                .is_some_and(|e| e == "mck")
        })
        .collect();
    assert_eq!(
        left,
        vec!["head.mck".to_owned()],
        "own conflict copies removed"
    );
    // The reader in replica 2 takes the newest checkpoint: A is not behind.
    let mut t = open_transport(&w.devs[2].access, &w.replica(2), &w.clock, "probe");
    let newest_counter = t
        .checkpoints()
        .unwrap()
        .into_iter()
        .filter(|(d, _)| *d == a)
        .filter_map(|(d, b)| Checkpoint::decode(&b, d, &*w.devs[2].s()).ok())
        .map(|c| c.counter)
        .max()
        .unwrap();
    assert!(
        newest_counter
            > Checkpoint::decode(&old, a, &*w.devs[2].s())
                .unwrap()
                .counter
    );
    w.advance(25 * HOUR_MS);
    for r in w.sync_all() {
        assert!(r.warnings.is_empty(), "{r:?}");
    }
    for d in 0..3 {
        assert_eq!(password_of(&w, d, vault, item).as_deref(), Some("v2"));
    }
}

/// Another device's `head.mck` arrives truncated, then whole: no error, no
/// warning either way.
#[test]
fn truncated_then_full_head_of_another_device() {
    let (mut w, vault, item) = three_devices(312);
    set_password(&mut w, 0, vault, item, "new");
    w.sync(0);
    w.cloud.settle().unwrap();
    let head = device_dir(&w, 1, 0).join("head.mck");
    let full = fs::read(&head).unwrap();
    fs::write(&head, &full[..full.len() - 7]).unwrap();
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "{r:?}");
    assert_eq!(password_of(&w, 1, vault, item).as_deref(), Some("new"));
    w.advance(25 * HOUR_MS);
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "{r:?}");
    fs::write(&head, &full).unwrap();
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "{r:?}");
}

/// `info.mdev` is lost everywhere: syncing is unaffected, the device list
/// shows the device without a name, and the next open writes it again.
#[test]
fn lost_device_info_changes_nothing_and_is_rewritten_on_open() {
    let (mut w, vault, item) = three_devices(313);
    for r in 0..3 {
        fs::remove_file(device_dir(&w, r, 0).join("info.mdev")).unwrap();
    }
    w.cloud.settle().unwrap();
    set_password(&mut w, 0, vault, item, "still syncing");
    w.converge(0);
    for d in 0..3 {
        assert!(w.devs[d].last.warnings.is_empty(), "{:?}", w.devs[d].last);
        assert_eq!(
            password_of(&w, d, vault, item).as_deref(),
            Some("still syncing")
        );
    }
    let a = install(&w, 0);
    let listed = w.devs[1].engine.transport_mut().devices().unwrap();
    let entry = listed.iter().find(|x| x.device_id == a).unwrap();
    assert_eq!(entry.name, None);
    assert!(entry.last_seen_ms.is_some());
    relock(&mut w, 0);
    w.cloud.settle().unwrap();
    let listed = w.devs[1].engine.transport_mut().devices().unwrap();
    let entry = listed.iter().find(|x| x.device_id == a).unwrap();
    assert_eq!(entry.name.as_deref(), Some("device 0"));
}

/// Somebody without the account key plants a whole fake device (head,
/// info, a log segment) and a backdated delete marker for an attachment:
/// nothing changes, nothing warns, the blob survives its "retention".
#[test]
fn outsider_planted_device_and_marker_change_nothing() {
    let (mut w, vault, item) = three_devices(314);
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.converge(0);
    let before: Vec<_> = (0..3).map(|d| password_of(&w, d, vault, item)).collect();
    let fake = InstallId::new_v7().unwrap();
    let outsider = KeyedCrypto::new(&AccountKey::generate().unwrap(), w.account, fake).unwrap();
    let acct = account_dir(&w, 1);
    let ddir = acct.join("d").join(dir_name(fake.as_bytes()));
    fs::create_dir_all(&ddir).unwrap();
    let cp = Checkpoint {
        device_id: fake,
        counter: 9,
        issued_at_ms: 1,
        entries: vec![CheckpointEntry {
            vault_id: vault,
            generation: KeyGeneration::FIRST,
            high_seq: 50,
        }],
    };
    fs::write(ddir.join("head.mck"), cp.encode(&outsider).unwrap()).unwrap();
    let info = DeviceInfo {
        created_at_ms: 1,
        name: "evil".to_owned(),
        platform: Platform::Linux,
    };
    fs::write(ddir.join("info.mdev"), info.seal(&outsider).unwrap()).unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let ldir = acct
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("log")
        .join(dir_name(fake.as_bytes()));
    fs::create_dir_all(&ldir).unwrap();
    let seg = Segment {
        snapshot: None,
        account_id: w.account,
        vault_id: vault,
        device_id: fake,
        seq: 1,
        created_at_ms: 1,
        objects: vec![SegmentObject::Tombstone {
            record_id: item,
            ciphertext: mosaic_core::TombstoneCiphertext::seal(&vk, &vault, &item, b"{}").unwrap(),
        }],
    };
    fs::write(ldir.join(segment_name(1)), seg.encode(&outsider).unwrap()).unwrap();
    let marker = BlobDelete {
        attachment_id: id,
        vault_id: vault,
        device_id: fake,
        deleted_at_ms: 1,
    };
    let bdir = acct
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("blobs");
    fs::write(
        bdir.join(blob_delete_name(id.as_bytes(), fake.as_bytes())),
        marker.encode(&outsider).unwrap(),
    )
    .unwrap();
    for _ in 0..3 {
        w.advance(31 * DAY_MS);
        for r in w.sync_all() {
            assert!(r.warnings.is_empty(), "{r:?}");
            assert_eq!(r.undecryptable, 0, "{r:?}");
        }
    }
    for (d, shown) in before.iter().enumerate() {
        assert_eq!(&password_of(&w, d, vault, item), shown);
        assert_eq!(
            w.devs[d]
                .s()
                .attachment(vault, item, id)
                .unwrap()
                .expose_secret(),
            b"attachment body"
        );
        let blob = account_dir(&w, d)
            .join("v")
            .join(dir_name(vault.as_bytes()))
            .join("blobs")
            .join(blob_name(id.as_bytes()));
        assert!(blob.exists(), "replica {d}");
    }
}

/// A holder of the account key writes a valid delete marker dated 1970
/// under a made-up device ID for an attachment every device still uses: no
/// device purges it, because each still refers to it.
#[test]
fn backdated_valid_marker_cannot_purge_a_referenced_blob() {
    let (mut w, vault, item) = three_devices(315);
    let id: AttachmentId = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.converge(0);
    let fake = InstallId::new_v7().unwrap();
    let marker = BlobDelete {
        attachment_id: id,
        vault_id: vault,
        device_id: fake,
        deleted_at_ms: 1,
    };
    let bdir = account_dir(&w, 0)
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("blobs");
    fs::write(
        bdir.join(blob_delete_name(id.as_bytes(), fake.as_bytes())),
        // The holder's session MACs with the real folder key.
        marker.encode(&*w.devs[0].s()).unwrap(),
    )
    .unwrap();
    for _ in 0..2 {
        w.advance(31 * DAY_MS);
        w.sync_all();
    }
    for d in 0..3 {
        assert_eq!(
            w.devs[d]
                .s()
                .attachment(vault, item, id)
                .unwrap()
                .expose_secret(),
            b"attachment body"
        );
    }
    assert!(bdir.join(blob_name(id.as_bytes())).exists());
}

/// A crash between the segment write and the commit of the upload: the
/// record stays queued and goes out again under a new sequence number. The
/// duplicate changes nothing anywhere.
#[test]
fn crash_between_push_and_commit_duplicates_harmlessly() {
    let (mut w, vault, item) = three_devices(316);
    let mut engine = SyncEngine::with_jitter(
        Faulty::new(open_transport(
            &w.devs[0].access,
            &w.replica(0),
            &w.clock,
            "device 0",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    engine.transport_mut().faults.lose_push_records = 1;
    set_password(&mut w, 0, vault, item, "once");
    let before = segments(&log_dir(&w, 0, 0, vault)).len();
    let dev = &mut w.devs[0];
    assert!(engine.sync_once(&mut dev.access, &w.clock).is_err());
    w.advance(60_000);
    let dev = &mut w.devs[0];
    let r = engine.sync_once(&mut dev.access, &w.clock).unwrap();
    assert_eq!(r.pushed, 1, "sent again: {r:?}");
    assert_eq!(segments(&log_dir(&w, 0, 0, vault)).len(), before + 2);
    assert_eq!(w.devs[0].s().pending_uploads(1).unwrap().len(), 0);
    drop(engine);
    w.converge(1);
    for d in 0..3 {
        assert_eq!(password_of(&w, d, vault, item).as_deref(), Some("once"));
        assert!(w.devs[d].last.warnings.is_empty(), "{:?}", w.devs[d].last);
        assert_eq!(w.devs[d].s().items(&crate::common::everything()).len(), 1);
    }
}

/// The provider renames a segment of A into a conflict copy in B's replica
/// (the canonical name disappears). Readers ignore conflict names, so the
/// segment counts as missing and is reported after the grace; once the
/// canonical name is back it is read and the warning goes.
#[test]
fn segment_renamed_to_a_conflict_copy_is_reported_until_it_returns() {
    let (mut w, vault, item) = three_devices(317);
    let a = install(&w, 0);
    set_password(&mut w, 0, vault, item, "renamed away");
    w.sync(0);
    w.cloud.settle().unwrap();
    let seg = segments(&log_dir(&w, 1, 0, vault)).pop().unwrap();
    let copy = seg.with_file_name(format!(
        "{} (1).mseg",
        seg.file_stem().unwrap().to_string_lossy()
    ));
    fs::rename(&seg, &copy).unwrap();
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "{r:?}");
    w.advance(25 * HOUR_MS);
    let r = w.sync(1);
    assert!(warns_missing(&r, a), "{r:?}");
    assert_ne!(
        password_of(&w, 1, vault, item).as_deref(),
        Some("renamed away")
    );
    fs::copy(&copy, &seg).unwrap();
    let r = w.sync(1);
    assert!(r.warnings.is_empty(), "{r:?}");
    assert_eq!(
        password_of(&w, 1, vault, item).as_deref(),
        Some("renamed away")
    );
    assert!(is_idle(&w.sync(1)));
}

/// The same `head.mck` bytes of A replayed by the folder for a long time
/// while A keeps syncing: B never warns while A's segments arrive, and B's
/// view of A is never a reason to halt B.
#[test]
fn old_head_of_another_device_without_withheld_data_is_quiet() {
    let (mut w, vault, item) = three_devices(318);
    let a = install(&w, 0);
    let old_head = fs::read(device_dir(&w, 1, 0).join("head.mck")).unwrap();
    let mut engine = SyncEngine::with_jitter(
        MaliciousTransport::new(open_transport(
            &w.devs[1].access,
            &w.replica(1),
            &w.clock,
            "device 1",
        )),
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    engine.transport_mut().replace_checkpoints = vec![(a, old_head)];
    for i in 0..4 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
        w.cloud.settle().unwrap();
        w.advance(10 * HOUR_MS);
        let b = &mut w.devs[1];
        let r = engine.sync_once(&mut b.access, &w.clock).unwrap();
        assert!(r.warnings.is_empty(), "{r:?}");
    }
    assert_eq!(password_of(&w, 1, vault, item).as_deref(), Some("v3"));
}
