//! Compaction under chaos: snapshot sets that arrive in pieces, across a
//! relock, after a failed part write, and devices that come back after the
//! log they last read was compacted away.

use std::fs;

use mosaic_core::{ItemId, VaultId};
use mosaic_sync::engine::SyncError;
use mosaic_sync::testutil::SimConfig;

use super::util::{
    DAY_MS, HOUR_MS, add_login, max_seq, password_of, relock, segment_needle, set_password, shows,
    snapshot_parts, warns_missing,
};
use crate::common::{World, log_dir, segments};

/// More records than one segment holds (256 objects), so a snapshot of the
/// vault needs two parts.
pub(super) const BIG: usize = 260;

/// Two devices at rest sharing a vault of `BIG` items; returns the items in
/// creation order.
pub(super) fn big_vault(seed: u64) -> (World, VaultId, Vec<ItemId>) {
    let mut w = World::new(seed, 2);
    let vault = w.new_vault(0, "Big");
    let items: Vec<ItemId> = (0..BIG)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.converge(0);
    (w, vault, items)
}

/// Device 0 edits `item` and syncs until its log holds a snapshot set;
/// returns the last value written.
pub(super) fn edit_until_compacted(w: &mut World, vault: VaultId, item: ItemId) -> String {
    for i in 0..90 {
        let value = format!("edit {i}");
        set_password(w, 0, vault, item, &value);
        w.sync(0);
        if !snapshot_parts(w, 0, 0, vault).is_empty() {
            return value;
        }
    }
    panic!("device 0 never compacted");
}

/// Runs `rounds` rounds of a calm cloud (holds count down per round).
pub(super) fn rounds(w: &mut World, n: usize) {
    w.cloud.set_config(SimConfig::CALM);
    for _ in 0..n {
        w.cloud.round().unwrap();
    }
}

/// The snapshot parts reach B in two syncs of one engine: the set completes
/// and the device is caught up. (Control of finding 7.)
#[test]
fn snapshot_parts_across_two_syncs_close_the_gap() {
    snapshot_parts_in_two_pieces(401, false);
}

/// The same with a relock between the two pieces: the new transport's
/// snapshot tracker never saw the first part (already marked read), so the
/// set never completes, the deleted older segments stay a gap in the cursor
/// and B reports A's files missing although it holds everything.
#[test]
fn snapshot_parts_across_a_relock_close_the_gap() {
    snapshot_parts_in_two_pieces(402, true);
}

fn snapshot_parts_in_two_pieces(seed: u64, relock_between: bool) {
    let (mut w, vault, items) = big_vault(seed);
    let a = w.devs[0].s().install_id();
    let item = items[BIG - 1];
    // B stays away while A edits until its log is compacted.
    let last = edit_until_compacted(&mut w, vault, item);
    let parts = snapshot_parts(&w, 0, 0, vault);
    assert!(parts.len() >= 2, "a set of several parts: {}", parts.len());
    let first_part = parts[0].0;
    assert!(
        segments(&log_dir(&w, 0, 0, vault))
            .iter()
            .all(|p| super::util::seq_of(p) >= first_part),
        "older segments deleted"
    );
    // Every part but the first is held back from B for a while.
    for (seq, _, _) in &parts[1..] {
        w.cloud.withhold(1, &segment_needle(&w, 0, vault, *seq), 5);
    }
    rounds(&mut w, 3);
    assert!(
        parts[1..].iter().any(|(_, _, ids)| ids.contains(&item)),
        "the edited item sits in a later part"
    );
    w.sync(1);
    assert_ne!(
        password_of(&w, 1, vault, item).as_deref(),
        Some(last.as_str()),
        "the part with the edit is still held back"
    );
    if relock_between {
        relock(&mut w, 1);
    }
    w.cloud.settle().unwrap();
    w.sync(1);
    assert_eq!(
        password_of(&w, 1, vault, item).as_deref(),
        Some(last.as_str())
    );
    w.advance(25 * HOUR_MS);
    w.sync(0);
    w.cloud.settle().unwrap();
    let r = w.sync(1);
    assert!(
        !warns_missing(&r, a),
        "every file was delivered, nothing is missing: {r:?}"
    );
}

/// A's compaction fails after the first part (a write error); its next run
/// compacts again and numbers the new set right after the first part, so
/// the new set overlaps the range the failed set claimed. A reader that
/// read the failed set's first part refuses every part of the new set for
/// as long as its transport lives: an edit that only the new set still
/// carries (A deleted the segment that had it) never arrives, and A's files
/// are reported missing.
#[test]
fn snapshot_retried_after_a_failed_part_is_accepted() {
    let (mut w, vault, items) = big_vault(403);
    let a_id = w.devs[0].s().install_id();
    let item = items[BIG - 1];
    let dir = log_dir(&w, 0, 0, vault);
    // A directory with the name of the second part's segment makes that
    // write fail (and nothing else: listings skip directories).
    let mut failed_at = None;
    for i in 0..90 {
        let high = max_seq(&dir);
        let blocker = dir.join(format!("{:016x}.mseg", high + 3));
        fs::create_dir_all(&blocker).unwrap();
        set_password(&mut w, 0, vault, item, &format!("edit {i}"));
        let outcome = w.try_sync(0);
        fs::remove_dir(&blocker).unwrap();
        match outcome {
            Ok(_) => {}
            Err(SyncError::Transport(_)) => {
                failed_at = Some(high + 1);
                break;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    let pushed = failed_at.expect("the compaction failed once");
    let first = snapshot_parts(&w, 0, 0, vault);
    assert_eq!(first.len(), 1, "only the first part was written");
    // B gets the first part but not the segment with the last edit.
    w.cloud
        .withhold(1, &segment_needle(&w, 0, vault, pushed), 12);
    rounds(&mut w, 3);
    w.sync(1);
    // A's next run compacts again, then deletes everything older.
    w.advance(60_000);
    w.sync(0);
    assert!(
        snapshot_parts(&w, 0, 0, vault).len() >= 2,
        "a complete set now"
    );
    w.cloud.settle().unwrap();
    for _ in 0..2 {
        w.sync(1);
    }
    w.advance(25 * HOUR_MS);
    w.sync(0);
    w.cloud.settle().unwrap();
    let report = w.sync(1);
    assert_eq!(
        password_of(&w, 1, vault, item),
        password_of(&w, 0, vault, item),
        "B has A's last edit"
    );
    assert!(!warns_missing(&report, a_id), "{report:?}");
}

/// Three devices; C is offline for days while A compacts its log twice and
/// B keeps up. C's offline edits and everything A wrote meet after C
/// returns: no loss, no warning once everything is delivered.
#[test]
fn long_offline_device_returns_after_two_compactions() {
    let mut w = World::new(404, 3);
    let vault = w.new_vault(0, "Shared");
    let items: Vec<ItemId> = (0..4)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    let c = 2;
    // C's offline edits: one item edited, one deleted, one new.
    set_password(&mut w, c, vault, items[1], "C offline");
    w.devs[c].s_mut().purge_item(vault, items[2]).unwrap();
    let fresh = add_login(&mut w, c, vault, "made by C offline");
    // A and B go on; A compacts twice (C's replica still receives files).
    let mut compactions = 0;
    let mut seen_base = 0;
    for i in 0..200 {
        set_password(&mut w, 0, vault, items[0], &format!("a{i}"));
        if i % 7 == 0 {
            set_password(&mut w, 1, vault, items[3], &format!("b{i}"));
            w.sync(1);
        }
        w.sync(0);
        w.cloud.round().unwrap();
        if let Some((seq, _, _)) = snapshot_parts(&w, 0, 0, vault).first()
            && *seq != seen_base
        {
            seen_base = *seq;
            compactions += 1;
            if compactions == 2 {
                break;
            }
        }
        w.advance(HOUR_MS);
    }
    assert_eq!(compactions, 2);
    w.advance(3 * DAY_MS);
    w.converge(2);
    let a_last = password_of(&w, 0, vault, items[0]);
    for d in 0..3 {
        assert_eq!(password_of(&w, d, vault, items[0]), a_last, "device {d}");
        assert!(shows(&w, d, vault, items[1], "C offline"), "device {d}");
        assert!(
            w.devs[d].s().item(vault, items[2]).unwrap().is_deleted(),
            "device {d}: C's delete had no concurrent write"
        );
        assert!(w.devs[d].s().item(vault, fresh).is_ok(), "device {d}");
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "device {d}: {:?}",
            w.devs[d].last.warnings
        );
        assert_eq!(w.devs[d].s().pending_uploads(1).unwrap().len(), 0);
    }
}

/// A snapshot set reaches B only in part, then A compacts again (deleting
/// the first set) before B saw the rest: B ends with A's state and no gap.
#[test]
fn partial_snapshot_superseded_by_the_next_compaction() {
    let (mut w, vault, items) = big_vault(405);
    let a = w.devs[0].s().install_id();
    let item = items[BIG - 1];
    edit_until_compacted(&mut w, vault, item);
    let parts = snapshot_parts(&w, 0, 0, vault);
    for (seq, _, _) in &parts[1..] {
        w.cloud
            .withhold(1, &segment_needle(&w, 0, vault, *seq), 400);
    }
    rounds(&mut w, 3);
    w.sync(1);
    // A compacts once more; the held parts are deleted before B sees them.
    let old_base = parts[0].0;
    for i in 0..90 {
        set_password(&mut w, 0, vault, item, &format!("second {i}"));
        w.sync(0);
        if snapshot_parts(&w, 0, 0, vault)
            .first()
            .is_some_and(|(s, _, _)| *s != old_base)
        {
            break;
        }
        w.cloud.round().unwrap();
    }
    assert!(
        snapshot_parts(&w, 0, 0, vault)
            .first()
            .is_some_and(|(s, _, _)| *s != old_base)
    );
    // Release the holds (their files are gone from the cloud by now).
    w.cloud.settle().unwrap();
    w.sync(1);
    w.advance(25 * HOUR_MS);
    w.sync(0);
    w.cloud.settle().unwrap();
    let r = w.sync(1);
    assert_eq!(
        password_of(&w, 1, vault, item),
        password_of(&w, 0, vault, item)
    );
    assert!(!warns_missing(&r, a), "{r:?}");
}

/// A deletes an item, rotates the vault and compacts its log while C is
/// offline (C's replica receives the files, C does not sync): the segment
/// with the tombstone is gone by the time C returns, so the snapshot must
/// carry the tombstone, re-sealed under the new key. The item stays deleted
/// everywhere.
#[test]
fn tombstone_survives_rotation_and_compaction_for_an_offline_device() {
    let mut w = World::new(406, 3);
    let vault = w.new_vault(0, "Shared");
    let items: Vec<ItemId> = (0..3)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    w.devs[0].s_mut().purge_item(vault, items[1]).unwrap();
    w.sync(0);
    w.cloud.round().unwrap();
    {
        let clock = w.clock.clone();
        let dev = &mut w.devs[0];
        dev.engine
            .rotate_vault(&mut dev.access, &clock, vault)
            .unwrap();
    }
    w.sync(1);
    let last = edit_until_compacted(&mut w, vault, items[0]);
    let base = snapshot_parts(&w, 0, 0, vault)[0].0;
    assert!(
        segments(&log_dir(&w, 0, 0, vault))
            .iter()
            .all(|p| super::util::seq_of(p) >= base),
        "the tombstone's segment is gone"
    );
    w.advance(2 * DAY_MS);
    w.converge(1);
    for d in 0..3 {
        assert!(
            w.devs[d].s().item(vault, items[1]).unwrap().is_deleted(),
            "device {d}: the deleted item came back"
        );
        assert_eq!(
            password_of(&w, d, vault, items[0]).as_deref(),
            Some(last.as_str())
        );
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "{:?}",
            w.devs[d].last.warnings
        );
    }
}
