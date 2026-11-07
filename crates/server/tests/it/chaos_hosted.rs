//! Hosted sync-chaos scenarios (M6 T4 red team): the sync engine, the HTTP
//! transport and the real server on 127.0.0.1, with one fault or one hostile
//! answer placed at an exact call (`chaos_net::Faulty`).
//!
//! Every scenario is deterministic (fixed seeds, manual engine and server
//! clocks, no sleeps) and checks the sync invariants after full delivery:
//! no silent loss, convergence, no warning once everything is delivered,
//! and the right warning (`FreezeSuspected`, `RollbackDetected`) while a
//! server misbehaves.
//!
//! A scenario that breaks an invariant is `#[ignore]`d with the finding's
//! number ("M6-T4 chaos finding N"); it is a regression test for the fix.

#![allow(
    clippy::many_single_char_names,
    reason = "w, d, e, x, y: world, device, engine and the items of a scenario"
)]

use core::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_core::model::SecretField;
use mosaic_core::{AccountKey, InstallId, ItemId, VaultId, VaultKey};
use mosaic_sync::clock::Clock;
use mosaic_sync::engine::consts::BACKOFF_MAX_MS;
use mosaic_sync::engine::{RotationOutcome, SyncError, SyncWarning};
use mosaic_sync::testutil::ManualClock;
use mosaic_sync::transport::{
    Cursor, PullPage, PulledKey, PulledRecord, PushResult, Transport, TransportError,
};

use crate::chaos_net::{
    DAY_MS, FEngine, Fault, MINUTE_MS, Op, anywhere, assert_converged, cursor_of, faulty, frun,
    frun_at, fsync, fsync_at, restore_vault, server_head, settle, snapshot_vault, warns_freeze,
};
use crate::hosted::{Dev, World, everything, is_idle};
use crate::scenarios::{add_login, generation, has_pw, set_password, three_devices, two_devices};

const ALL: [usize; 3] = [0, 1, 2];

fn head_of(next: &Cursor) -> u64 {
    u64::from_be_bytes(<[u8; 8]>::try_from(next.0.as_slice()).unwrap())
}

/// Rotates `vault` on device `d` through its own engine (online).
fn rotate_online(w: &mut World, d: usize, vault: VaultId) -> RotationOutcome {
    w.server.advance_ms(MINUTE_MS);
    let clock = w.clock.clone();
    let dev = &mut w.devs[d];
    dev.engine
        .rotate_vault(&mut dev.access, &clock, vault)
        .unwrap()
}

/// Runs `e` until a run succeeds and every scripted fault was used,
/// whatever the faults made the earlier runs fail with (the app retries;
/// after a 401 it would sign in again, here the session is still valid).
fn drive(w: &mut World, e: &mut FEngine, d: usize) {
    for _ in 0..60 {
        match frun(w, e, d) {
            Ok(_) if e.transport_mut().faults.is_empty() => return,
            Ok(_) | Err(SyncError::Transport(_)) => {}
            Err(SyncError::Backoff { retry_at_ms }) => {
                let now = w.clock.now_ms();
                w.clock.advance_ms(retry_at_ms.saturating_sub(now) + 1);
            }
            Err(other) => panic!("device {d}: {other:?}"),
        }
    }
    panic!("device {d} never got through its faults");
}

// ---- lost responses and repeated requests at every mutating endpoint --------------

/// The pending work device 0 holds before the faulty run.
struct Busy {
    vault: VaultId,
    items: Vec<ItemId>,
    second: VaultId,
    in_second: ItemId,
    kept: mosaic_core::AttachmentId,
    removed: mosaic_core::AttachmentId,
}

/// Three devices at rest, then device 0 queues work for every mutating
/// endpoint in one run: a new vault (create), an offline rotation of the
/// shared vault (key push), edits and a delete (records), a rename
/// (metadata), a new attachment (put), a removed one (delete) and its
/// checkpoint. Device 1 edits concurrently, so a merge happens too.
fn busy(seed: u64) -> (World, Busy) {
    let (mut w, vault, items) = three_devices(seed);
    let removed = w.devs[0]
        .s_mut()
        .add_attachment(
            vault,
            items[0],
            "a.txt",
            "text/plain",
            b"attachment removed",
        )
        .unwrap();
    settle(&mut w, &ALL);
    let second = w.new_vault(0, "Second");
    let in_second = add_login(&mut w, 0, second, "in second");
    set_password(&mut w, 0, vault, items[1], "edit by 0");
    w.devs[0].s_mut().purge_item(vault, items[2]).unwrap();
    w.devs[0].s_mut().rename_vault(vault, "Renamed").unwrap();
    let kept = w.devs[0]
        .s_mut()
        .add_attachment(vault, items[1], "b.txt", "text/plain", b"attachment kept")
        .unwrap();
    w.devs[0]
        .s_mut()
        .remove_attachment(vault, items[0], removed)
        .unwrap();
    w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    w.devs[1]
        .s_mut()
        .update_item(vault, items[1], |e| e.set_title("title by 1"))
        .unwrap();
    (
        w,
        Busy {
            vault,
            items,
            second,
            in_second,
            kept,
            removed,
        },
    )
}

/// Everything device 0 queued arrived everywhere, exactly once.
fn check_busy(w: &mut World, b: &Busy) {
    assert_converged(w, &ALL);
    for d in ALL {
        let s = w.devs[d].s();
        let v = s.vaults();
        let shared = v.iter().find(|x| x.vault_id == b.vault).unwrap();
        assert_eq!(shared.name, Some("Renamed"), "device {d}");
        assert!(shared.key_generation.get() >= 2, "device {d}");
        assert!(v.iter().any(|x| x.vault_id == b.second), "device {d}");
        assert!(!s.item(b.second, b.in_second).unwrap().is_deleted());
        let edited = s.item(b.vault, b.items[1]).unwrap();
        assert!(crate::sim_hosted::has_password(edited, "edit by 0"));
        assert_eq!(edited.title(), Some("title by 1"), "device {d}");
        assert!(s.item(b.vault, b.items[2]).unwrap().is_deleted());
        let plain = s.attachment(b.vault, b.items[1], b.kept).unwrap();
        assert_eq!(plain.expose_secret(), b"attachment kept");
        assert!(
            s.item(b.vault, b.items[0])
                .unwrap()
                .attachments()
                .iter()
                .all(|a| a.attachment_id != b.removed)
        );
        assert_eq!(s.items(&everything()).len(), 3, "device {d}: no duplicate");
    }
}

const MUTATING: [Op; 7] = [
    Op::CreateVault,
    Op::PushKey,
    Op::PushRecords,
    Op::PushMeta,
    Op::PutAttachment,
    Op::DeleteAttachment,
    Op::PutCheckpoint,
];

/// Every mutating endpoint loses its first response (the server committed,
/// the client saw a timeout) in the runs of one device, one after another;
/// then every endpoint answers twice to one request. Nothing is lost,
/// nothing is duplicated, nothing warns.
#[test]
fn lost_and_repeated_responses_at_every_endpoint() {
    for (seed, fault) in [(1001, Fault::LoseResponse), (1002, Fault::Duplicate)] {
        let (mut w, b) = busy(seed);
        let mut e = faulty(&w, 0);
        for op in MUTATING {
            e.transport_mut().faults.push((op, 0, fault));
        }
        drive(&mut w, &mut e, 0);
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        check_busy(&mut w, &b);
    }
}

/// Requests that never reach the server and 401 answers, at every call of
/// the first runs.
#[test]
fn failed_requests_and_401_at_every_endpoint() {
    for (seed, fault) in [(1003, Fault::FailBefore), (1004, Fault::Unauthorized)] {
        let (mut w, b) = busy(seed);
        let mut e = faulty(&w, 0);
        for op in MUTATING
            .into_iter()
            .chain([Op::Pull, Op::Checkpoints, Op::ListVaults])
        {
            e.transport_mut().faults.push((op, 0, fault));
        }
        drive(&mut w, &mut e, 0);
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        check_busy(&mut w, &b);
    }
}

/// The same, one endpoint and one fault per world (slow).
#[test]
#[ignore = "long-running: one world per endpoint and fault"]
fn each_endpoint_each_fault_isolated() {
    let mut seed = 1100;
    for fault in [Fault::LoseResponse, Fault::Duplicate, Fault::FailBefore] {
        for op in MUTATING {
            seed += 1;
            let (mut w, b) = busy(seed);
            let mut e = faulty(&w, 0);
            e.transport_mut().faults.push((op, 0, fault));
            drive(&mut w, &mut e, 0);
            drop(e);
            w.relogin(0);
            settle(&mut w, &ALL);
            check_busy(&mut w, &b);
        }
    }
}

/// An attachment that a delete removed and a concurrent edit brought back is
/// restored on the server; the restore's response is lost, then repeated.
#[test]
fn attachment_restore_lost_and_repeated() {
    for (seed, fault) in [(1201, Fault::LoseResponse), (1202, Fault::Duplicate)] {
        let (mut w, vault, item) = two_devices(seed);
        let att = w.devs[0]
            .s_mut()
            .add_attachment(vault, item, "a.txt", "text/plain", b"attachment restored")
            .unwrap();
        settle(&mut w, &[0, 1]);
        w.devs[0].s_mut().purge_item(vault, item).unwrap();
        w.sync(0);
        set_password(&mut w, 1, vault, item, "edit keeps the file");
        let mut e = faulty(&w, 1);
        e.transport_mut()
            .faults
            .push((Op::RestoreAttachment, 0, fault));
        drive(&mut w, &mut e, 1);
        assert!(
            e.transport_mut().count(Op::RestoreAttachment) > 0,
            "the scenario restores the attachment"
        );
        drop(e);
        w.relogin(1);
        settle(&mut w, &[0, 1]);
        assert_converged(&mut w, &[0, 1]);
        for d in [0, 1] {
            let s = w.devs[d].s();
            assert!(!s.item(vault, item).unwrap().is_deleted());
            let plain = s.attachment(vault, item, att).unwrap();
            assert_eq!(plain.expose_secret(), b"attachment restored");
        }
    }
}

// ---- rotations -------------------------------------------------------------------

/// An online rotation whose key upload is committed but unanswered, or
/// answered twice (the second answer is `GenerationTaken` for the device's
/// own key); the app retries the rotation. A concurrent offline edit under
/// the old key is kept, every device ends at one generation.
#[test]
fn rotation_key_push_lost_or_repeated() {
    for (seed, fault) in [(1301, Fault::LoseResponse), (1302, Fault::Duplicate)] {
        let (mut w, vault, items) = three_devices(seed);
        set_password(&mut w, 1, vault, items[1], "offline under the old key");
        set_password(&mut w, 0, vault, items[0], "rotator's edit");
        let mut e = faulty(&w, 0);
        e.transport_mut().faults.push((Op::PushKey, 0, fault));
        let mut outcome = None;
        for _ in 0..5 {
            w.server.advance_ms(MINUTE_MS);
            match e.rotate_vault(&mut w.devs[0].access, &w.clock, vault) {
                Ok(o) => {
                    outcome = Some(o);
                    break;
                }
                Err(SyncError::Transport(_)) => {}
                Err(other) => panic!("{fault:?}: {other:?}"),
            }
        }
        assert!(outcome.is_some(), "{fault:?}: the rotation went through");
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            assert!(generation(&w, d, vault).get() >= 2);
            assert!(has_pw(&w, d, vault, items[1], "offline under the old key"));
            assert!(has_pw(&w, d, vault, items[0], "rotator's edit"));
            assert_eq!(w.devs[d].s().items(&everything()).len(), 3);
        }
    }
}

/// The head moves under a rotation (another device's edit lands between the
/// pull and the key upload), and the retried key upload then loses its
/// response.
#[test]
fn rotation_head_moved_then_retry_lost() {
    let (mut w, vault, items) = three_devices(1303);
    let b: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(1)));
    let fired = Arc::new(AtomicBool::new(false));
    let mut e = faulty(&w, 0);
    {
        let (b, fired, clock) = (b.clone(), fired.clone(), w.clock.clone());
        let item = items[1];
        e.transport_mut().before_next(Op::PushKey, move || {
            let mut dev = b.lock().unwrap();
            dev.s_mut()
                .update_item(vault, item, |e| {
                    e.set_secret(SecretField::Password, "lands first")
                })
                .unwrap();
            let dev = &mut *dev;
            dev.engine.sync_once(&mut dev.access, &clock).unwrap();
            fired.store(true, Ordering::SeqCst);
        });
    }
    e.transport_mut()
        .faults
        .push((Op::PushKey, 1, Fault::LoseResponse));
    w.server.advance_ms(MINUTE_MS);
    let first = e.rotate_vault(&mut w.devs[0].access, &w.clock, vault);
    assert!(fired.load(Ordering::SeqCst));
    assert!(
        matches!(first, Err(SyncError::Transport(TransportError::Timeout))),
        "{first:?}"
    );
    drop(e);
    w.devs
        .insert(1, Arc::try_unwrap(b).ok().unwrap().into_inner().unwrap());
    w.relogin(0);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        assert_eq!(generation(&w, d, vault).get(), 2, "one rotation landed");
        assert!(has_pw(&w, d, vault, items[1], "lands first"));
    }
}

/// Three devices rotate one vault online at once: while device 0 offers its
/// key, devices 2 and 1 complete their rotations. Device 0's offer is
/// refused, it takes the newest key; every edit survives.
#[test]
fn three_online_rotations_race() {
    let (mut w, vault, items) = three_devices(1304);
    for d in ALL {
        set_password(&mut w, d, vault, items[d], &format!("edit by {d}"));
    }
    let c: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(2)));
    let b: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(1)));
    let mut e = faulty(&w, 0);
    {
        let (b, c, clock) = (b.clone(), c.clone(), w.clock.clone());
        e.transport_mut().before_next(Op::PushKey, move || {
            for dev in [c, b] {
                let mut dev = dev.lock().unwrap();
                let dev = &mut *dev;
                let o = dev
                    .engine
                    .rotate_vault(&mut dev.access, &clock, vault)
                    .unwrap();
                assert!(matches!(o, RotationOutcome::Rotated(_)), "{o:?}");
            }
        });
    }
    w.server.advance_ms(MINUTE_MS);
    let outcome = e
        .rotate_vault(&mut w.devs[0].access, &w.clock, vault)
        .unwrap();
    assert!(
        matches!(outcome, RotationOutcome::RotatedByAnother(g) if g.get() == 3),
        "{outcome:?}"
    );
    drop(e);
    w.devs
        .insert(1, Arc::try_unwrap(b).ok().unwrap().into_inner().unwrap());
    w.devs
        .insert(2, Arc::try_unwrap(c).ok().unwrap().into_inner().unwrap());
    w.relogin(0);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        assert_eq!(generation(&w, d, vault).get(), 3);
        for j in ALL {
            assert!(has_pw(&w, d, vault, items[j], &format!("edit by {j}")));
        }
    }
}

/// Three devices rotate the same vault offline (each queues its own key of
/// generation 2) and edit under it; they reach the server in several
/// orders. One key per generation wins on the server, the losers rotate
/// again, every edit survives.
#[test]
fn three_offline_rotations_converge_in_any_order() {
    for (i, order) in [[0, 1, 2], [2, 1, 0], [1, 2, 0]].into_iter().enumerate() {
        let (mut w, vault, items) = three_devices(1310 + i as u64);
        for d in ALL {
            set_password(&mut w, d, vault, items[d], &format!("edit by {d}"));
            w.devs[d].s_mut().rotate_vault_key(vault).unwrap();
        }
        for d in order {
            w.sync(d);
        }
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            for j in ALL {
                assert!(
                    has_pw(&w, d, vault, items[j], &format!("edit by {j}")),
                    "order {order:?}: device {d} lacks the edit by {j}"
                );
            }
        }
    }
}

/// Finding 1. Device 1 rotated offline (its own key of generation 2) and
/// edited; device 0 rotated online (its key of generation 2 took the slot),
/// edited and re-sealed every record under it, then never synced again
/// (lost, wiped: the device a rotation is often about). Device 1 cannot
/// open the generation-2 records, skips them, rotates to generation 3, and
/// its re-sealed records conflict forever with the generation-2 versions it
/// never read. A device that joins afterwards takes generation 3 and drops
/// every generation-2 record as a rollback: it sees an empty vault.
#[test]
#[ignore = "M6-T4 chaos finding 1: offline-rotation loser skips the winner's records; vault unreadable once the winner is gone"]
fn offline_rotation_loser_after_winner_left() {
    let (mut w, vault, x) = two_devices(1320);
    let y = add_login(&mut w, 0, vault, "second");
    settle(&mut w, &[0, 1]);
    set_password(&mut w, 1, vault, x, "loser's edit");
    w.devs[1].s_mut().rotate_vault_key(vault).unwrap();
    set_password(&mut w, 0, vault, y, "winner's edit");
    assert!(matches!(
        rotate_online(&mut w, 0, vault),
        RotationOutcome::Rotated(g) if g.get() == 2
    ));
    w.sync(0);
    assert_eq!(w.devs[0].s().pending_uploads(1).unwrap().len(), 0);
    // Device 0 is gone from here on. A new device joins.
    let d = w.join();
    settle(&mut w, &[1, d]);
    let state = |w: &World, dev: usize| {
        let s = w.devs[dev].s();
        let shows = |id: ItemId, value: &str| {
            s.item(vault, id)
                .is_ok_and(|it| crate::sim_hosted::has_password(it, value))
        };
        (
            s.items(&everything()).len(),
            shows(y, "winner's edit"),
            shows(x, "loser's edit"),
            s.pending_uploads(64).unwrap().len(),
        )
    };
    let facts: Vec<String> = [1, d]
        .iter()
        .map(|&dev| {
            let (live, winner, loser, queued) = state(&w, dev);
            format!(
                "device {dev}: {live} live items, winner's edit {winner}, loser's edit {loser}, {queued} queued, generation {}, last report {:?}",
                generation(&w, dev, vault).get(),
                w.devs[dev].last,
            )
        })
        .collect();
    let complete = [1, d]
        .iter()
        .all(|&dev| state(&w, dev) == (2, true, true, 0));
    assert!(complete, "{facts:#?}");
    assert_converged(&mut w, &[1, d]);
}

// ---- deletes and moves with lost responses ---------------------------------------

/// Device 0 deletes an item; the server stores the tombstone but the answer
/// is lost. Device 1 edits the item without having seen the delete, device 2
/// deletes it as well. The write wins everywhere and is flagged.
#[test]
fn tombstone_lost_response_vs_concurrent_edit() {
    let (mut w, vault, items) = three_devices(1401);
    let x = items[0];
    w.devs[0].s_mut().purge_item(vault, x).unwrap();
    let mut e = faulty(&w, 0);
    e.transport_mut().next(Op::PushRecords, Fault::LoseResponse);
    assert!(frun(&mut w, &mut e, 0).is_err());
    set_password(&mut w, 1, vault, x, "edit after delete");
    w.devs[2].s_mut().purge_item(vault, x).unwrap();
    w.sync(1);
    w.sync(2);
    fsync(&mut w, &mut e, 0);
    drop(e);
    w.relogin(0);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        let s = w.devs[d].s();
        let it = s.item(vault, x).unwrap();
        assert!(!it.is_deleted(), "device {d}: the write wins");
        assert!(crate::sim_hosted::has_password(it, "edit after delete"));
        assert!(it.deleted_concurrently().is_some());
    }
    // A delete with nothing concurrent wins.
    w.devs[2].s_mut().purge_item(vault, x).unwrap();
    settle(&mut w, &ALL);
    for d in ALL {
        assert!(w.devs[d].s().item(vault, x).unwrap().is_deleted());
    }
}

/// A move's copy and the source's tombstone go out in two requests; either
/// answer is lost, and another device edits the source in between. The copy
/// exists exactly once, the edit survives (on the source, which comes back
/// flagged), nothing is lost.
#[test]
fn move_with_lost_response_between_copy_and_tombstone() {
    for lost in [0usize, 1] {
        let (mut w, vault, items) = three_devices(1410 + lost as u64);
        let other = w.new_vault(0, "Other");
        settle(&mut w, &ALL);
        let x = items[0];
        let copy = w.devs[0].s_mut().move_item(vault, x, other).unwrap();
        let mut e = faulty(&w, 0);
        e.transport_mut()
            .faults
            .push((Op::PushRecords, lost, Fault::LoseResponse));
        assert!(frun(&mut w, &mut e, 0).is_err(), "lost {lost}");
        set_password(&mut w, 1, vault, x, "edit during move");
        w.sync(1);
        fsync(&mut w, &mut e, 0);
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            let s = w.devs[d].s();
            assert!(!s.item(other, copy).unwrap().is_deleted(), "lost {lost}");
            let source = s.item(vault, x).unwrap();
            assert!(!source.is_deleted(), "lost {lost}: the edit wins");
            assert!(crate::sim_hosted::has_password(source, "edit during move"));
            assert_eq!(s.items(&everything()).len(), 4, "lost {lost}: one copy");
        }
    }
}

// ---- answers out of order across devices -----------------------------------------

/// Device 1 runs a whole sync at the worst moments of device 0's run: after
/// device 0's pull answered (device 0 applies a stale page), between its
/// pull and its push, and after its push landed but before device 0 sees
/// the answer.
#[test]
fn responses_reordered_across_devices() {
    for (i, (op, after)) in [
        (Op::Pull, true),
        (Op::PushRecords, false),
        (Op::PushRecords, true),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut w, vault, items) = three_devices(1501 + i as u64);
        set_password(&mut w, 0, vault, items[0], "from 0");
        set_password(&mut w, 1, vault, items[0], "from 1");
        set_password(&mut w, 1, vault, items[1], "only 1");
        w.devs[0].s_mut().purge_item(vault, items[2]).unwrap();
        set_password(&mut w, 1, vault, items[2], "1 edits what 0 deletes");
        let b: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(1)));
        let fired = Arc::new(AtomicBool::new(false));
        let mut e = faulty(&w, 0);
        let hook = {
            let (b, fired, clock) = (b.clone(), fired.clone(), w.clock.clone());
            move || {
                let mut dev = b.lock().unwrap();
                let dev = &mut *dev;
                dev.engine.sync_once(&mut dev.access, &clock).unwrap();
                fired.store(true, Ordering::SeqCst);
            }
        };
        if after {
            e.transport_mut().after_next(op, hook);
        } else {
            e.transport_mut().before_next(op, hook);
        }
        fsync(&mut w, &mut e, 0);
        assert!(fired.load(Ordering::SeqCst));
        drop(e);
        w.devs
            .insert(1, Arc::try_unwrap(b).ok().unwrap().into_inner().unwrap());
        w.relogin(0);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            assert!(has_pw(&w, d, vault, items[0], "from 0"), "{op:?} {after}");
            assert!(has_pw(&w, d, vault, items[0], "from 1"), "{op:?} {after}");
            assert!(has_pw(&w, d, vault, items[1], "only 1"), "{op:?} {after}");
            assert!(has_pw(&w, d, vault, items[2], "1 edits what 0 deletes"));
        }
    }
}

// ---- long poll -------------------------------------------------------------------

/// A change is never missed by the long poll: a wait whose answer was lost,
/// and a change that lands while the device syncs, both wake the next wait
/// at once (a missed wakeup would wait the two seconds and return the old
/// token).
#[test]
fn long_poll_wakeups_not_lost() {
    let (mut w, vault, item) = two_devices(1601);
    let mut e = faulty(&w, 1);
    let wait = Duration::from_secs(2);
    let t0 = e.transport_mut().wait_for_change(None, wait).unwrap();
    fsync(&mut w, &mut e, 1);
    set_password(&mut w, 0, vault, item, "wake up");
    w.sync(0);
    // The wakeup is answered but the answer is lost; the engine keeps no
    // token and the next wait answers at once.
    e.transport_mut().next(Op::Wait, Fault::LoseResponse);
    assert!(e.wait_for_change(wait).is_err());
    let t1 = e.transport_mut().wait_for_change(Some(&t0), wait).unwrap();
    assert_ne!(t1, t0, "the change woke the poll");
    // A change between this device's wakeup and its sync is in the sync, and
    // the next wait still answers at once.
    set_password(&mut w, 0, vault, item, "during the sync");
    w.sync(0);
    fsync(&mut w, &mut e, 1);
    assert!(has_pw(&w, 1, vault, item, "during the sync"));
    let t2 = e.transport_mut().wait_for_change(Some(&t1), wait).unwrap();
    assert_ne!(t2, t1);
}

// ---- sessions --------------------------------------------------------------------

/// The device signs in again elsewhere (one session per device) while a run
/// is in flight, at several points of the run: the run fails with 401, the
/// app signs in again, nothing is lost, no halt, no warning.
#[test]
fn session_evicted_mid_run_then_relogin() {
    for (i, op) in [Op::Pull, Op::PushRecords, Op::PushMeta, Op::PutCheckpoint]
        .into_iter()
        .enumerate()
    {
        let (mut w, vault, items) = three_devices(1701 + i as u64);
        set_password(&mut w, 0, vault, items[0], "before the 401");
        w.devs[0].s_mut().rename_vault(vault, "Renamed").unwrap();
        set_password(&mut w, 1, vault, items[1], "other device");
        w.sync(1);
        let mut e = faulty(&w, 0);
        let (base, email, account, device) = (
            w.server.base.clone(),
            w.email.clone(),
            w.account,
            w.devs[0].device,
        );
        let opw = w.opw(0);
        let server_clock = w.server.clock.clone();
        e.transport_mut().before_next(op, move || {
            server_clock.advance_ms(MINUTE_MS);
            let grant = mosaic_sync::http::hosted::login(
                &base,
                &email,
                account,
                &opw,
                device,
                "device 0 elsewhere",
            )
            .unwrap();
            drop(grant);
        });
        let err = frun(&mut w, &mut e, 0).unwrap_err();
        assert!(
            matches!(err, SyncError::Transport(TransportError::Unauthorized)),
            "{op:?}: {err:?}"
        );
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            assert!(has_pw(&w, d, vault, items[0], "before the 401"), "{op:?}");
            assert!(has_pw(&w, d, vault, items[1], "other device"), "{op:?}");
            let s = w.devs[d].s();
            let name = s
                .vaults()
                .iter()
                .find(|v| v.vault_id == vault)
                .unwrap()
                .name;
            assert_eq!(name, Some("Renamed"));
        }
    }
}

/// Every session expires during forty days offline (server and device
/// clocks both move); the devices sign in again and converge without a
/// warning.
#[test]
fn session_expiry_after_long_offline() {
    let (mut w, vault, items) = three_devices(1710);
    set_password(&mut w, 0, vault, items[0], "written offline");
    set_password(&mut w, 2, vault, items[0], "also offline");
    w.server.advance_ms(40 * DAY_MS);
    w.advance(40 * DAY_MS);
    let err = w.try_sync(0).unwrap_err();
    assert!(
        matches!(err, SyncError::Transport(TransportError::Unauthorized)),
        "{err:?}"
    );
    for d in ALL {
        w.relogin(d);
    }
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        assert!(has_pw(&w, d, vault, items[0], "written offline"));
        assert!(has_pw(&w, d, vault, items[0], "also offline"));
    }
}

// ---- clocks ----------------------------------------------------------------------

fn clock_at(offset_ms: i64) -> ManualClock {
    let c = ManualClock::new();
    let now = c.now_ms();
    c.set_ms(now.saturating_add_signed(offset_ms));
    c
}

/// Device clocks three days behind, right and three days ahead: concurrent
/// edits of one field, a delete racing an edit and a rename converge to the
/// same state with every value kept and no warning, also after the grace.
#[test]
fn clock_skew_of_days_changes_nothing() {
    let (mut w, vault, items) = three_devices(1801);
    let day = i64::try_from(DAY_MS).unwrap();
    let clocks = [clock_at(-3 * day), clock_at(0), clock_at(3 * day)];
    let mut es: Vec<FEngine> = ALL.iter().map(|&d| faulty(&w, d)).collect();
    for d in ALL {
        set_password(&mut w, d, vault, items[0], &format!("skewed {d}"));
    }
    w.devs[2].s_mut().purge_item(vault, items[1]).unwrap();
    set_password(&mut w, 0, vault, items[1], "edit vs delete");
    w.devs[1].s_mut().rename_vault(vault, "Skewed").unwrap();
    for round in 0..6 {
        if round == 3 {
            for c in &clocks {
                c.advance_ms(11 * MINUTE_MS);
            }
        }
        for d in [2, 0, 1] {
            fsync_at(&mut w, &mut es[d], d, &clocks[d]);
            clocks[d].advance_ms(MINUTE_MS);
        }
    }
    for d in ALL {
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "device {d}: {:?}",
            w.devs[d].last
        );
    }
    assert_converged(&mut w, &ALL);
    for d in ALL {
        for j in ALL {
            assert!(
                anywhere(&w, d, &format!("skewed {j}")),
                "device {d}, value {j}"
            );
        }
        assert!(has_pw(&w, d, vault, items[1], "edit vs delete"));
    }
}

/// The clock of a device jumps forward and back by days between runs (the
/// user fixes it); honest devices never warn.
#[test]
fn clock_jumps_cause_no_false_warning() {
    let (mut w, vault, items) = three_devices(1802);
    for (k, jump) in [3 * DAY_MS, 0, 7 * DAY_MS].into_iter().enumerate() {
        set_password(&mut w, k % 3, vault, items[0], &format!("jump {k}"));
        w.advance(jump);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        let now = w.clock.now_ms();
        w.clock.set_ms(now - 5 * DAY_MS);
        set_password(&mut w, (k + 1) % 3, vault, items[1], &format!("back {k}"));
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
    }
}

/// Finding 2. A failed run schedules the retry 5 s ahead; then the clock
/// goes back two days (it ran fast and was corrected). The engine refuses to
/// sync for two days.
#[test]
#[ignore = "M6-T4 chaos finding 2: backoff retry time is absolute; a backward clock jump stalls sync for days"]
fn backoff_survives_backward_clock_jump() {
    let (mut w, _vault, _item) = two_devices(1803);
    let mut e = faulty(&w, 0);
    e.transport_mut().next(Op::Checkpoints, Fault::FailBefore);
    let clock = clock_at(0);
    assert!(matches!(
        frun_at(&mut w, &mut e, 0, &clock),
        Err(SyncError::Transport(TransportError::Unreachable))
    ));
    let now = clock.now_ms();
    clock.set_ms(now - 2 * DAY_MS);
    match frun_at(&mut w, &mut e, 0, &clock) {
        Ok(_) => {}
        Err(SyncError::Backoff { retry_at_ms }) => {
            let wait = retry_at_ms.saturating_sub(clock.now_ms());
            assert!(
                wait <= BACKOFF_MAX_MS * 12 / 10,
                "sync waits {} hours after the clock went back",
                wait / 3_600_000
            );
        }
        Err(other) => panic!("{other:?}"),
    }
}

/// Finding 3. A server withholds a vault from device 1; its lag timer starts.
/// Device 1's clock is then set back three days. Eleven minutes later (past
/// the ten-minute grace) it should warn; it stays silent for three days,
/// because the stored start of the lag is in the future.
#[test]
#[ignore = "M6-T4 chaos finding 3: lag start in the future after a backward clock jump suppresses FreezeSuspected for days"]
fn freeze_warning_survives_backward_clock_jump() {
    let (mut w, vault, item) = two_devices(1804);
    let mut e = faulty(&w, 1);
    e.transport_mut().withhold_all = true;
    set_password(&mut w, 0, vault, item, "withheld");
    w.sync(0);
    w.sync(0);
    let clock = clock_at(0);
    let r = fsync_at(&mut w, &mut e, 1, &clock);
    assert!(r.warnings.is_empty(), "{r:?}");
    let now = clock.now_ms();
    clock.set_ms(now - 3 * DAY_MS);
    fsync_at(&mut w, &mut e, 1, &clock);
    clock.advance_ms(11 * MINUTE_MS);
    let r = fsync_at(&mut w, &mut e, 1, &clock);
    assert!(
        warns_freeze(&r),
        "no freeze warning 11 minutes later: {r:?}"
    );
}

// ---- a hostile server ------------------------------------------------------------

/// Finding 4. The server replays an old version of an item (which device 1
/// rightly ignores) under a forged, far higher sequence number (its page
/// claims that head). Device 1 raises the record's upload base to that
/// number, so every later edit of the item is refused as a conflict, on
/// every run, with no warning: it never reaches another device.
#[test]
#[ignore = "M6-T4 chaos finding 4: a replayed record with a forged seq raises the upload base; the item's edits never upload"]
fn replayed_record_with_forged_seq_wedges_uploads() {
    let (mut w, vault, item) = two_devices(1901);
    let old = w.devs[0]
        .s()
        .store()
        .record(&vault, &item)
        .unwrap()
        .unwrap()
        .body;
    set_password(&mut w, 0, vault, item, "v2");
    settle(&mut w, &[0, 1]);
    add_login(&mut w, 0, vault, "gives the server a page");
    w.sync(0);
    let mut e = faulty(&w, 1);
    let mut replay = Some(old);
    e.transport_mut().tamper_pull = Some(Box::new(move |_, page: &mut PullPage| {
        if page.records.is_empty() {
            return;
        }
        if let Some(body) = replay.take() {
            page.records.push(PulledRecord {
                record_id: item,
                body,
                seq: Some(1 << 40),
            });
        }
    }));
    let r = fsync(&mut w, &mut e, 1);
    assert!(
        has_pw(&w, 1, vault, item, "v2"),
        "the old version replaced nothing"
    );
    assert_eq!(r.undecryptable, 0);
    // From here on the server is honest.
    drop(e);
    w.relogin(1);
    set_password(&mut w, 1, vault, item, "edit after the replay");
    settle(&mut w, &[0, 1]);
    assert_eq!(
        w.devs[1].s().pending_uploads(1).unwrap().len(),
        0,
        "device 1's edit is still queued"
    );
    assert!(has_pw(&w, 0, vault, item, "edit after the replay"));
    assert_converged(&mut w, &[0, 1]);
}

/// Finding 5. A hostile server bumps the head it shows device 1 by one and
/// files a replayed old record there. Device 1's cursor moves past a
/// sequence number the server has not used yet; the next real record, which
/// gets exactly that number, is never pulled by device 1. Nothing warns
/// (device 1's cursor is not behind anyone's checkpoint).
#[test]
#[ignore = "M6-T4 chaos finding 5: a forged head moves the cursor past an unused seq; the real record at that seq is skipped silently"]
fn forged_head_skips_next_real_record() {
    let (mut w, vault, item) = two_devices(1902);
    let old = w.devs[0]
        .s()
        .store()
        .record(&vault, &item)
        .unwrap()
        .unwrap()
        .body;
    set_password(&mut w, 0, vault, item, "v2");
    settle(&mut w, &[0, 1]);
    add_login(&mut w, 0, vault, "gives the server a page");
    w.sync(0);
    let mut e = faulty(&w, 1);
    let mut replay = Some(old);
    e.transport_mut().tamper_pull = Some(Box::new(move |_, page: &mut PullPage| {
        if page.records.is_empty() || page.more {
            return;
        }
        if let Some(body) = replay.take() {
            let seq = head_of(&page.next) + 1;
            page.records.push(PulledRecord {
                record_id: item,
                body,
                seq: Some(seq),
            });
            page.next = Cursor(seq.to_be_bytes().to_vec());
        }
    }));
    fsync(&mut w, &mut e, 1);
    drop(e);
    w.relogin(1);
    let late = add_login(&mut w, 0, vault, "the next real record");
    settle(&mut w, &[0, 1]);
    w.advance(11 * MINUTE_MS);
    settle(&mut w, &[0, 1]);
    let warned = warns_freeze(&w.devs[1].last) || warns_freeze(&w.devs[0].last);
    let arrived = w.devs[1].s().item(vault, late).is_ok();
    assert!(
        arrived || warned,
        "device 1 never received the record at the forged head, and nothing warns"
    );
}

/// The server answers an upload with a conflict and a made-up current
/// sequence number, once after storing the upload and once without storing
/// it. The engine ignores the number, pulls, and gets the edit through.
#[test]
fn forged_conflict_current_seq_is_harmless() {
    let (mut w, vault, item) = two_devices(1903);
    set_password(&mut w, 1, vault, item, "stored, answered as conflict");
    let mut e = faulty(&w, 1);
    let mut once = true;
    e.transport_mut().tamper_push_results = Some(Box::new(move |_, results| {
        if std::mem::take(&mut once) {
            for r in results.iter_mut() {
                *r = PushResult::Conflict {
                    current_seq: u64::MAX,
                };
            }
        }
    }));
    fsync(&mut w, &mut e, 1);
    set_password(&mut w, 1, vault, item, "not stored, answered as conflict");
    let mut once = true;
    e.transport_mut().intercept_push = Some(Box::new(move |_, records| {
        std::mem::take(&mut once).then(|| {
            records
                .iter()
                .map(|_| PushResult::Conflict { current_seq: 3 })
                .collect()
        })
    }));
    fsync(&mut w, &mut e, 1);
    fsync(&mut w, &mut e, 1);
    drop(e);
    w.relogin(1);
    settle(&mut w, &[0, 1]);
    assert_converged(&mut w, &[0, 1]);
    for d in [0, 1] {
        assert!(has_pw(&w, d, vault, item, "stored, answered as conflict"));
        assert!(has_pw(
            &w,
            d,
            vault,
            item,
            "not stored, answered as conflict"
        ));
    }
}

/// Finding 6. The server acknowledges device 0's upload with a sequence
/// number and drops it. Device 0 clears its outbox; no other device ever
/// receives the edit, and no device warns, before or after the grace.
#[test]
#[ignore = "M6-T4 chaos finding 6: an acknowledged-then-dropped upload is lost silently (no echo check, no warning)"]
fn acknowledged_but_dropped_upload_is_detected() {
    let (mut w, vault, items) = three_devices(1904);
    set_password(&mut w, 0, vault, items[0], "dropped by the server");
    let fake = cursor_of(&w, 0, vault) + 1;
    let mut e = faulty(&w, 0);
    let mut once = true;
    e.transport_mut().intercept_push = Some(Box::new(move |_, records| {
        std::mem::take(&mut once).then(|| {
            (0u64..)
                .zip(records)
                .map(|(i, _)| PushResult::Accepted {
                    seq: Some(fake + i),
                })
                .collect()
        })
    }));
    fsync(&mut w, &mut e, 0);
    assert_eq!(w.devs[0].s().pending_uploads(1).unwrap().len(), 0);
    drop(e);
    w.relogin(0);
    set_password(&mut w, 1, vault, items[1], "an honest write");
    settle(&mut w, &ALL);
    w.advance(11 * MINUTE_MS);
    settle(&mut w, &ALL);
    let warned = ALL.iter().any(|&d| warns_freeze(&w.devs[d].last));
    let delivered = [1, 2]
        .iter()
        .all(|&d| has_pw(&w, d, vault, items[0], "dropped by the server"));
    assert!(
        delivered || warned,
        "the acknowledged edit reached no other device and nothing warns"
    );
}

/// The server withholds the new key from every page after a rotation:
/// device 1 defers the records sealed under it (its cursor stays before
/// them), keeps its readable state, warns after the grace, and converges
/// without a warning once the key arrives.
#[test]
fn withheld_keys_defer_records_and_warn() {
    let (mut w, vault, items) = three_devices(1905);
    assert!(matches!(
        rotate_online(&mut w, 0, vault),
        RotationOutcome::Rotated(_)
    ));
    set_password(&mut w, 0, vault, items[0], "under the new key");
    w.sync(0);
    w.sync(0);
    let mut e = faulty(&w, 1);
    e.transport_mut().tamper_pull = Some(Box::new(|_, page: &mut PullPage| page.keys.clear()));
    let r = fsync(&mut w, &mut e, 1);
    assert!(r.warnings.is_empty(), "{r:?}");
    assert_eq!(generation(&w, 1, vault).get(), 1);
    assert!(has_pw(&w, 1, vault, items[0], "pw"));
    w.advance(11 * MINUTE_MS);
    let r = fsync(&mut w, &mut e, 1);
    assert!(warns_freeze(&r), "{r:?}");
    assert_eq!(w.devs[1].s().items(&everything()).len(), 3, "nothing lost");
    drop(e);
    w.relogin(1);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        assert!(has_pw(&w, d, vault, items[0], "under the new key"));
    }
}

/// A joining device gets a vault's keys of three generations in reverse
/// order; it takes the newest and reads everything.
#[test]
fn reordered_keys_in_a_page() {
    let (mut w, vault, item) = two_devices(1906);
    for _ in 0..2 {
        rotate_online(&mut w, 0, vault);
        settle(&mut w, &[0, 1]);
    }
    set_password(&mut w, 0, vault, item, "at generation 3");
    settle(&mut w, &[0, 1]);
    let d = w.join();
    let mut e = faulty(&w, d);
    e.transport_mut().tamper_pull = Some(Box::new(|_, page: &mut PullPage| page.keys.reverse()));
    let r = fsync(&mut w, &mut e, d);
    assert_eq!(r.undecryptable, 0, "{r:?}");
    fsync(&mut w, &mut e, d);
    assert_eq!(generation(&w, d, vault).get(), 3);
    assert!(has_pw(&w, d, vault, item, "at generation 3"));
    drop(e);
    w.relogin(d);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
}

/// Finding 7. The server cuts a page short and says there is no more
/// (`more = false`, `nextSince` = head). Device 1 moves its cursor to the
/// head and never asks for the missing records; its cursor is not behind
/// any checkpoint, so nothing warns.
#[test]
#[ignore = "M6-T4 chaos finding 7: a page truncated with more=false loses records silently (high_seq cannot see gaps)"]
fn truncated_page_is_detected() {
    let (mut w, vault, items) = three_devices(1907);
    set_password(&mut w, 0, vault, items[0], "first");
    set_password(&mut w, 0, vault, items[1], "second");
    w.sync(0);
    w.sync(0);
    let mut e = faulty(&w, 1);
    let mut once = true;
    e.transport_mut().tamper_pull = Some(Box::new(move |_, page: &mut PullPage| {
        if page.records.len() >= 2 && std::mem::take(&mut once) {
            page.records.truncate(1);
            page.more = false;
        }
    }));
    fsync(&mut w, &mut e, 1);
    drop(e);
    w.relogin(1);
    settle(&mut w, &ALL);
    w.advance(11 * MINUTE_MS);
    settle(&mut w, &ALL);
    let complete =
        has_pw(&w, 1, vault, items[0], "first") && has_pw(&w, 1, vault, items[1], "second");
    let warned = ALL.iter().any(|&d| warns_freeze(&w.devs[d].last));
    assert!(
        complete || warned,
        "a record was cut from a page and nothing warns"
    );
}

/// The server withholds one vault and serves the other honestly: the device
/// takes what it gets, warns after the grace about the writer of the
/// withheld vault, and stops warning once the vault arrives.
#[test]
fn withholding_one_vault_warns_after_grace() {
    let (mut w, vault, item) = two_devices(1908);
    let second = w.new_vault(0, "Second");
    settle(&mut w, &[0, 1]);
    let mut e = faulty(&w, 1);
    e.transport_mut().withhold.insert(second);
    let hidden = add_login(&mut w, 0, second, "hidden");
    set_password(&mut w, 0, vault, item, "visible");
    w.sync(0);
    w.sync(0);
    let r = fsync(&mut w, &mut e, 1);
    assert!(r.warnings.is_empty(), "{r:?}");
    assert!(has_pw(&w, 1, vault, item, "visible"));
    w.advance(11 * MINUTE_MS);
    let r = fsync(&mut w, &mut e, 1);
    let a = w.devs[0].device;
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::FreezeSuspected { device, .. } if *device == a)),
        "{r:?}"
    );
    assert!(w.devs[1].s().item(second, hidden).is_err());
    drop(e);
    w.relogin(1);
    settle(&mut w, &[0, 1]);
    assert_converged(&mut w, &[0, 1]);
    assert!(w.devs[1].s().item(second, hidden).is_ok());
}

/// While it withholds everything, the server replays device 0's old
/// checkpoint in place of the new one, and files device 0's checkpoint under
/// device 1's ID: the warning stays, no identity alarm, no halt.
#[test]
fn replayed_checkpoint_does_not_end_freeze() {
    let (mut w, vault, item) = two_devices(1909);
    let a = w.devs[0].device;
    let b = w.devs[1].device;
    let checkpoint_of = |w: &mut World, who: InstallId| {
        w.devs[0]
            .engine
            .transport_mut()
            .inner_mut()
            .checkpoints()
            .unwrap()
            .into_iter()
            .find(|(d, _)| *d == who)
            .unwrap()
            .1
    };
    let old = checkpoint_of(&mut w, a);
    set_password(&mut w, 0, vault, item, "withheld");
    w.sync(0);
    w.sync(0);
    let mut e = faulty(&w, 1);
    e.transport_mut().withhold_all = true;
    let r = fsync(&mut w, &mut e, 1);
    assert!(r.warnings.is_empty(), "{r:?}");
    let replay = old.clone();
    e.transport_mut().tamper_checkpoints = Some(Box::new(move |list| {
        list.retain(|(d, _)| *d != a);
        list.push((a, replay.clone()));
        list.push((b, replay.clone()));
    }));
    w.advance(11 * MINUTE_MS);
    let r = fsync(&mut w, &mut e, 1);
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::FreezeSuspected { device, .. } if *device == a)),
        "{r:?}"
    );
    assert!(!r.warnings.contains(&SyncWarning::IdentityDuplicated));
    drop(e);
    w.relogin(1);
    settle(&mut w, &[0, 1]);
    assert_converged(&mut w, &[0, 1]);
    assert!(has_pw(&w, 1, vault, item, "withheld"));
}

/// A wrapped key of the vault's current generation that does not open
/// (wrapped under another account key) changes nothing: no rotation, no
/// key conflict, the honest path converges at the same generation.
#[test]
fn foreign_key_for_current_generation_changes_nothing() {
    let (mut w, vault, _item) = two_devices(1910);
    add_login(&mut w, 0, vault, "gives the server a page");
    w.sync(0);
    let g = generation(&w, 1, vault);
    let foreign = AccountKey::generate().unwrap();
    let wrapped = VaultKey::generate(g)
        .unwrap()
        .wrap(&foreign, &w.account, &vault)
        .unwrap();
    let device = w.devs[0].device;
    let mut e = faulty(&w, 1);
    e.transport_mut().tamper_pull = Some(Box::new(move |_, page: &mut PullPage| {
        let seq = head_of(&page.next);
        page.keys.push(PulledKey {
            generation: g,
            wrapped: wrapped.clone(),
            device_id: device,
            seq: Some(seq),
        });
    }));
    let r = fsync(&mut w, &mut e, 1);
    assert!(r.undecryptable >= 1, "{r:?}");
    assert_eq!(generation(&w, 1, vault), g);
    drop(e);
    w.relogin(1);
    settle(&mut w, &[0, 1]);
    assert_converged(&mut w, &[0, 1]);
    assert_eq!(generation(&w, 0, vault), g, "no rotation was forced");
}

/// Finding 8. The server restores one vault from an older backup (the other
/// vault and the checkpoints stay current), and new uploads then reuse the
/// sequence numbers the devices had already read. A device that joins never
/// sees the edits the backup lost; the old devices skip the new records
/// whose numbers they had read before. No device warns: the joiner's lag
/// ends as soon as the reused numbers pass the old head, and an old
/// device's own checkpoint never went back.
#[test]
#[ignore = "M6-T4 chaos finding 8: one-vault rollback with sequence reuse loses data silently on every device"]
fn vault_rollback_with_seq_reuse_detected() {
    let (mut w, vault, x) = two_devices(1911);
    let other = w.new_vault(0, "Untouched");
    add_login(&mut w, 0, other, "in the other vault");
    settle(&mut w, &[0, 1]);
    snapshot_vault(&w, vault);
    let snap_head = server_head(&w, vault);
    set_password(&mut w, 0, vault, x, "after the backup");
    let y = add_login(&mut w, 0, vault, "after the backup too");
    settle(&mut w, &[0, 1]);
    let read = cursor_of(&w, 0, vault);
    assert!(read > snap_head);
    restore_vault(&w, vault);
    // A new device joins and writes enough that the head passes what the
    // old devices have read.
    let d = w.join();
    w.sync(d);
    let n = usize::try_from(read - snap_head).unwrap() + 2;
    let fresh: Vec<ItemId> = (0..n)
        .map(|i| add_login(&mut w, d, vault, &format!("new {i}")))
        .collect();
    w.sync(d);
    assert!(server_head(&w, vault) > read);
    let mut outcomes = Vec::new();
    for pass in 0..4 {
        if pass == 2 {
            w.advance(11 * MINUTE_MS);
        }
        for dev in [0, 1, d] {
            outcomes.push(match w.try_sync(dev) {
                Ok(r) => warns_freeze(&r),
                Err(SyncError::RollbackDetected) => true,
                Err(other) => panic!("device {dev}: {other:?}"),
            });
        }
    }
    let warned = outcomes.iter().any(|w| *w);
    let joiner_complete =
        has_pw(&w, d, vault, x, "after the backup") && w.devs[d].s().item(vault, y).is_ok();
    let old_complete = fresh
        .iter()
        .all(|id| w.devs[0].s().item(vault, *id).is_ok() && w.devs[1].s().item(vault, *id).is_ok());
    assert!(
        (joiner_complete && old_complete) || warned,
        "rolled-back vault: joiner complete {joiner_complete}, old devices complete \
         {old_complete}, no warning"
    );
}

/// Every run of every device after full delivery is idle and silent, also
/// after many runs with lost and repeated answers at random calls (a seeded
/// schedule over all three devices).
#[test]
fn random_fault_schedule_converges() {
    for seed in [2001u64, 2002] {
        random_schedule(seed, 4);
    }
}

/// The long version (`--ignored`).
#[test]
#[ignore = "long-running: many seeds of the random fault schedule"]
fn random_fault_schedule_many_seeds() {
    for seed in 1..=24u64 {
        random_schedule(seed * 7919, 6);
    }
}

/// Seeded random edits, renames, deletes, moves and offline rotations by
/// three devices, with a random fault placed at random calls of each
/// device's runs, then full delivery; the invariants must hold.
fn random_schedule(seed: u64, rounds: usize) {
    const OPS: [Op; 10] = [
        Op::PushRecords,
        Op::PushMeta,
        Op::PushKey,
        Op::PutAttachment,
        Op::DeleteAttachment,
        Op::PutCheckpoint,
        Op::Pull,
        Op::Checkpoints,
        Op::CreateVault,
        Op::RestoreAttachment,
    ];
    let (mut w, vault, _items) = three_devices(seed);
    let second = w.new_vault(0, "Second");
    add_login(&mut w, 0, second, "item s0");
    add_login(&mut w, 0, second, "item s1");
    settle(&mut w, &ALL);
    let vaults = [vault, second];
    let mut rng = mosaic_sync::testutil::SimRng::new(seed);
    let mut es: Vec<FEngine> = ALL.iter().map(|&d| faulty(&w, d)).collect();
    // (round, value, title of the item it was written to)
    let mut written: Vec<(usize, String, String)> = Vec::new();
    // (round, title) of every purge
    let mut purged: Vec<(usize, String)> = Vec::new();
    let mut rotated = [false; 3];
    let mut n = 0usize;
    for round in 0..rounds {
        for d in ALL {
            for _ in 0..=rng.below(2) {
                n += 1;
                // The device's live items, in an order that does not depend
                // on the (random) IDs.
                let mut live: Vec<(String, usize, VaultId, ItemId)> = w.devs[d]
                    .s()
                    .items(&everything())
                    .iter()
                    .map(|r| {
                        let title = r.item.title().unwrap_or_default().to_owned();
                        let vi = usize::from(r.vault_id != vault);
                        (title, vi, r.vault_id, r.item_id)
                    })
                    .collect();
                live.sort();
                let roll = rng.below(100);
                if live.is_empty() || roll >= 90 {
                    let v = vaults[rng.below(2)];
                    add_login(&mut w, d, v, &format!("new {n}"));
                    continue;
                }
                let (title, vi, v, item) = live[rng.below(live.len())].clone();
                match roll {
                    0..50 => {
                        let value = format!("v{n} d{d}");
                        set_password(&mut w, d, v, item, &value);
                        written.push((round, value, title));
                    }
                    50..62 => {
                        w.devs[d].s_mut().purge_item(v, item).unwrap();
                        purged.push((round, title));
                    }
                    62..74 => {
                        w.devs[d]
                            .s_mut()
                            .move_item(v, item, vaults[1 - vi])
                            .unwrap();
                    }
                    74..82 => {
                        w.devs[d]
                            .s_mut()
                            .rename_vault(v, &format!("name {n}"))
                            .unwrap();
                    }
                    82..90 if !rotated[d] && round > 0 => {
                        rotated[d] = true;
                        w.devs[d].s_mut().rotate_vault_key(v).unwrap();
                    }
                    _ => {
                        w.devs[d]
                            .s_mut()
                            .update_item(v, item, |e| e.add_tag(&format!("tag{n}")))
                            .unwrap();
                    }
                }
            }
            for _ in 0..3 {
                let op = OPS[rng.below(OPS.len())];
                let fault =
                    [Fault::LoseResponse, Fault::Duplicate, Fault::FailBefore][rng.below(3)];
                let at = es[d].transport_mut().count(op) + rng.below(3);
                es[d].transport_mut().faults.push((op, at, fault));
            }
        }
        let mut order = ALL;
        rng.shuffle(&mut order);
        for d in order {
            let _ = frun(&mut w, &mut es[d], d);
        }
    }
    for e in &mut es {
        e.transport_mut().faults.clear();
    }
    for _ in 0..4 {
        for d in ALL {
            fsync(&mut w, &mut es[d], d);
        }
    }
    let mut idle = true;
    for d in ALL {
        idle &= is_idle(&fsync(&mut w, &mut es[d], d));
    }
    assert!(
        idle,
        "seed {seed}: a run after full delivery still moved data"
    );
    assert_converged(&mut w, &ALL);
    // A value survives unless its item was purged in or after the round it
    // was written in (a purge that saw the write wins; one that did not
    // loses, and the value is still there).
    for (round, value, title) in &written {
        if purged.iter().any(|(r, t)| r >= round && t == title) {
            continue;
        }
        assert!(
            ALL.iter().all(|&d| anywhere(&w, d, value)),
            "seed {seed}: {value} (written to {title} in round {round}) lost"
        );
    }
}

/// Finding 9. One vault is rolled back on the server and a device syncs right
/// away: the page's head is below the device's cursor. The run fails with a
/// protocol error before it evaluates any lag, on every run, so the user
/// never gets `RollbackDetected` or `FreezeSuspected`, and the other vault
/// (pulled after it) stops syncing too.
#[test]
#[ignore = "M6-T4 chaos finding 9: a head below the cursor aborts every run with Protocol; no rollback warning, other vaults stall"]
fn stale_head_below_cursor_reported_and_isolated() {
    let (mut w, vault, x) = two_devices(1912);
    let other = w.new_vault(0, "Untouched");
    settle(&mut w, &[0, 1]);
    snapshot_vault(&w, vault);
    set_password(&mut w, 0, vault, x, "after the backup");
    settle(&mut w, &[0, 1]);
    restore_vault(&w, vault);
    let fresh = add_login(&mut w, 0, other, "written after the rollback");
    let mut outcomes = Vec::new();
    for pass in 0..4 {
        if pass == 2 {
            w.advance(11 * MINUTE_MS);
        }
        for d in [0, 1] {
            outcomes.push(match w.try_sync(d) {
                Ok(r) => format!("ok, warnings {:?}", r.warnings),
                Err(e) => format!("{e:?}"),
            });
        }
    }
    let reported = outcomes
        .iter()
        .any(|o| o.contains("RollbackDetected") || o.contains("FreezeSuspected"));
    let isolated = w.devs[1].s().item(other, fresh).is_ok();
    assert!(
        reported && isolated,
        "reported {reported}, untouched vault still syncs {isolated}: {outcomes:?}"
    );
}

/// Around the stale-upload-base fix: the server stores an upload but the
/// answer is lost; the device edits the item again before its next run, and
/// another device edits it too. The newer edit goes up on top of the echo,
/// nothing is refused forever, nothing is lost.
#[test]
fn edit_after_lost_push_response_rebases() {
    for concurrent in [false, true] {
        let (mut w, vault, items) = three_devices(1913 + u64::from(concurrent));
        let x = items[0];
        set_password(&mut w, 0, vault, x, "first, answer lost");
        let mut e = faulty(&w, 0);
        e.transport_mut().next(Op::PushRecords, Fault::LoseResponse);
        assert!(frun(&mut w, &mut e, 0).is_err());
        set_password(&mut w, 0, vault, x, "second, after the loss");
        if concurrent {
            set_password(&mut w, 2, vault, x, "from device 2");
            w.sync(2);
        }
        fsync(&mut w, &mut e, 0);
        assert_eq!(w.devs[0].s().pending_uploads(1).unwrap().len(), 0);
        drop(e);
        w.relogin(0);
        settle(&mut w, &ALL);
        assert_converged(&mut w, &ALL);
        for d in ALL {
            assert!(has_pw(&w, d, vault, x, "first, answer lost"));
            assert!(has_pw(&w, d, vault, x, "second, after the loss"));
            if concurrent {
                assert!(has_pw(&w, d, vault, x, "from device 2"));
            }
        }
    }
}

/// Finding 10 (cost). After one device rotates a vault online and uploads its
/// records re-sealed under the new key, every other device re-seals its own
/// copies on taking the key, queues them and uploads all of them again
/// (each a byte-different ciphertext of the same version), and every device
/// then downloads every other device's copies. With `D` devices and `N`
/// records a rotation costs about `D * N` uploads instead of `N`.
#[test]
#[ignore = "M6-T4 chaos finding 10: after a remote rotation every device re-uploads every record (D x N uploads)"]
fn remote_rotation_does_not_reupload_everything() {
    let (mut w, vault, items) = three_devices(1915);
    for i in 0..7 {
        add_login(&mut w, 0, vault, &format!("more {i}"));
    }
    settle(&mut w, &ALL);
    assert!(matches!(
        rotate_online(&mut w, 0, vault),
        RotationOutcome::Rotated(_)
    ));
    let mut pushed_by_others = 0;
    let mut idle = 0;
    for _ in 0..20 {
        let mut all_idle = true;
        for d in ALL {
            let r = w.sync(d);
            if d != 0 {
                pushed_by_others += r.pushed;
            }
            all_idle &= is_idle(&r);
        }
        if all_idle {
            idle += 1;
            if idle == 2 {
                break;
            }
        }
    }
    assert_converged(&mut w, &ALL);
    assert!(has_pw(&w, 1, vault, items[0], "pw"));
    assert_eq!(
        pushed_by_others, 0,
        "devices that only took the new key uploaded {pushed_by_others} records"
    );
}

/// A deleted item stays deleted: the server replays its last live
/// ciphertext (inside the page, at a plausible sequence number) to a device
/// that holds the tombstone. The device ignores it and nobody resurrects it.
#[test]
fn replayed_live_version_does_not_resurrect() {
    let (mut w, vault, items) = three_devices(1916);
    let x = items[0];
    let live = w.devs[0]
        .s()
        .store()
        .record(&vault, &x)
        .unwrap()
        .unwrap()
        .body;
    w.devs[0].s_mut().purge_item(vault, x).unwrap();
    settle(&mut w, &ALL);
    add_login(&mut w, 0, vault, "gives the server a page");
    w.sync(0);
    let mut e = faulty(&w, 1);
    let mut replay = Some(live);
    e.transport_mut().tamper_pull = Some(Box::new(move |_, page: &mut PullPage| {
        let Some(last) = page.records.last().and_then(|r| r.seq) else {
            return;
        };
        if let Some(body) = replay.take() {
            page.records.push(PulledRecord {
                record_id: x,
                body,
                seq: Some(last.max(head_of(&page.next))),
            });
        }
    }));
    fsync(&mut w, &mut e, 1);
    assert!(w.devs[1].s().item(vault, x).unwrap().is_deleted());
    drop(e);
    w.relogin(1);
    settle(&mut w, &ALL);
    assert_converged(&mut w, &ALL);
    for d in ALL {
        assert!(
            w.devs[d].s().item(vault, x).unwrap().is_deleted(),
            "device {d}"
        );
    }
}
