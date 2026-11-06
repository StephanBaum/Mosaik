//! Races between devices: blob delete, re-assert and purge (with a clock
//! that runs ahead, and a stale device that purges), a partition of three
//! devices, and three concurrent rotations over a flaky client.

use mosaic_core::model::SecretField;
use mosaic_core::{AttachmentId, ItemId, KeyGeneration, VaultId};
use mosaic_sync::testutil::{ManualClock, SimConfig};
use mosaic_sync::wire::folder_names::{blob_name, dir_name};
use mosaic_vault::session::AttachmentFileState;

use super::util::{
    DAY_MS, HOUR_MS, add_login, password_of, relock_with_clock, set_password, shows, sync_with,
    warns_missing,
};
use crate::common::{World, account_dir, everything};

fn rounds(w: &mut World, n: usize) {
    w.cloud.set_config(SimConfig::CALM);
    for _ in 0..n {
        w.cloud.round().unwrap();
    }
}

fn blob_exists(w: &World, replica: usize, vault: VaultId, id: AttachmentId) -> bool {
    account_dir(w, replica)
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("blobs")
        .join(blob_name(id.as_bytes()))
        .exists()
}

fn readable_somewhere(w: &World, vault: VaultId, item: ItemId, id: AttachmentId) -> bool {
    (0..w.devs.len()).any(|d| {
        w.devs[d]
            .s()
            .attachment(vault, item, id)
            .is_ok_and(|p| p.expose_secret() == b"attachment body")
    })
}

/// C's clock runs 31 days ahead. A deletes an item with an attachment
/// while B, offline, edits it; neither B nor C ever had the file, so A was
/// its only holder (and C writes no marker of its own when it takes the
/// delete). C reads A's fresh delete marker as 31 days old and purges the
/// blob at once. B's edit wins against the delete and brings the item back
/// with its attachment reference, but no device and no folder holds the
/// file any more: the retention period was meant to cover exactly this.
/// (A device that held the file is safe: it writes its own marker with its
/// own clock when it drops the file, and the newest marker counts.)
#[test]
#[ignore = "M6-T2 chaos finding 9: blob purge trusts the marker's wall-clock time; a device clock 31 days ahead purges a just-deleted blob a concurrent edit restores"]
fn clock_ahead_does_not_purge_a_blob_a_concurrent_edit_restores() {
    purge_race(31 * DAY_MS);
}

/// Control of finding 9: the same schedule with C's clock right. The blob
/// stays for the retention period and B fetches it for the restored item.
#[test]
fn blob_of_a_restored_item_survives_when_clocks_agree() {
    purge_race(0);
}

fn purge_race(c_ahead_ms: u64) {
    let mut w = World::new(601, 3);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.sync(0);
    w.cloud.set_config(SimConfig::CALM);
    // Neither B nor C gets the file in time (a slow provider, or the
    // download budget): A is its only holder.
    w.cloud.withhold(1, &blob_name(id.as_bytes()), 7);
    w.cloud.withhold(2, &blob_name(id.as_bytes()), 7);
    rounds(&mut w, 3);
    let (b, c) = (w.join(), w.join());
    w.sync(b);
    w.sync(c);
    w.sync(0);
    rounds(&mut w, 3);
    w.sync(b);
    w.sync(c);
    assert_eq!(
        w.devs[b].s().attachment_file_state(id).unwrap(),
        AttachmentFileState::Missing
    );
    assert_eq!(
        w.devs[c].s().attachment_file_state(id).unwrap(),
        AttachmentFileState::Missing
    );
    let ahead = ManualClock::new();
    ahead.set_ms(mosaic_sync::clock::Clock::now_ms(&w.clock) + c_ahead_ms);
    relock_with_clock(&mut w, c, &ahead);

    set_password(&mut w, b, vault, item, "B offline");
    w.devs[0].s_mut().purge_item(vault, item).unwrap();
    w.sync(0);
    rounds(&mut w, 3);
    sync_with(&mut w, c, &ahead);
    rounds(&mut w, 3);
    w.sync(b);
    for _ in 0..4 {
        w.cloud.settle().unwrap();
        w.sync(0);
        w.sync(b);
        sync_with(&mut w, c, &ahead);
    }
    for d in 0..3 {
        assert!(shows(&w, d, vault, item, "B offline"), "device {d}");
        assert!(
            w.devs[d]
                .s()
                .item(vault, item)
                .unwrap()
                .attachments()
                .iter()
                .any(|a| a.attachment_id == id),
            "device {d}: the reference came back"
        );
    }
    assert!(
        readable_somewhere(&w, vault, item, id),
        "the attachment of a restored item is gone for good (blob in replica 0: {})",
        blob_exists(&w, 0, vault, id)
    );
}

/// A stale device purges a blob that a device whose news it has not seen
/// yet (segments and head held back) brought back: the holder uploads it
/// again from its own copy, and everyone ends up with the file.
#[test]
fn blob_purged_by_a_stale_device_is_reuploaded_by_the_holder() {
    let mut w = World::new(602, 3);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    let (b, d) = (w.join(), w.join());
    w.converge(0);
    for x in 0..3 {
        assert!(w.devs[x].s().attachment(vault, item, id).is_ok());
    }
    // B goes offline and edits; A deletes the item; D takes the delete.
    set_password(&mut w, b, vault, item, "B offline");
    w.devs[0].s_mut().purge_item(vault, item).unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.sync(d);
    w.advance(31 * DAY_MS);
    // B returns: its edit wins and the item is back.
    let b_hex = dir_name(w.devs[b].s().install_id().as_bytes());
    w.sync(b);
    assert!(shows(&w, b, vault, item, "B offline"));
    // D sees none of B's news and purges the expired blob.
    w.cloud.withhold(d, &format!("log/{b_hex}"), 4);
    w.cloud.withhold(d, &format!("d/{b_hex}"), 4);
    rounds(&mut w, 2);
    w.sync(0);
    w.sync(d);
    rounds(&mut w, 1);
    // B re-asserts its file; then everything is delivered.
    w.converge(1);
    for x in 0..3 {
        assert!(shows(&w, x, vault, item, "B offline"), "device {x}");
        assert_eq!(
            w.devs[x]
                .s()
                .attachment(vault, item, id)
                .unwrap()
                .expose_secret(),
            b"attachment body",
            "device {x}"
        );
        assert!(
            w.devs[x].last.warnings.is_empty(),
            "{:?}",
            w.devs[x].last.warnings
        );
    }
    assert!(blob_exists(&w, 0, vault, id));
}

/// Devices A and B on one side, C on the other: each side gets the other's
/// checkpoints but not its segments for over a day while all three edit,
/// one item is deleted on one side and edited on the other. Both sides
/// report the other's files missing during the partition; after it heals,
/// everything converges, nothing is lost and nobody warns.
#[test]
fn three_device_partition_warns_then_heals() {
    let mut w = World::new(603, 3);
    let vault = w.new_vault(0, "Shared");
    let items: Vec<ItemId> = (0..3)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    let hex = |w: &World, d: usize| dir_name(w.devs[d].s().install_id().as_bytes());
    let (ha, hb, hc) = (hex(&w, 0), hex(&w, 1), hex(&w, 2));
    w.cloud.set_config(SimConfig::CALM);
    for r in [0, 1] {
        w.cloud.withhold(r, &format!("log/{hc}"), 12);
    }
    w.cloud.withhold(2, &format!("log/{ha}"), 12);
    w.cloud.withhold(2, &format!("log/{hb}"), 12);
    let mut warned = [false; 3];
    for i in 0..7 {
        set_password(&mut w, 0, vault, items[0], &format!("a{i}"));
        set_password(&mut w, 1, vault, items[1], &format!("b{i}"));
        set_password(&mut w, 2, vault, items[0], &format!("c{i}"));
        if i == 1 {
            w.devs[0].s_mut().purge_item(vault, items[2]).unwrap();
            set_password(&mut w, 2, vault, items[2], "C keeps it");
        }
        for (d, flag) in warned.iter_mut().enumerate() {
            let r = w.sync(d);
            let peer = if d == 2 { 0 } else { 2 };
            *flag |= warns_missing(&r, w.devs[peer].s().install_id());
        }
        w.cloud.round().unwrap();
        w.advance(8 * HOUR_MS);
    }
    assert_eq!(
        warned,
        [true, true, true],
        "the partition is reported on both sides"
    );
    w.converge(1);
    for d in 0..3 {
        assert!(shows(&w, d, vault, items[0], "a6"), "device {d}");
        assert!(shows(&w, d, vault, items[0], "c6"), "device {d}");
        assert!(shows(&w, d, vault, items[1], "b6"), "device {d}");
        assert!(shows(&w, d, vault, items[2], "C keeps it"), "device {d}");
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "device {d}: {:?}",
            w.devs[d].last.warnings
        );
        assert_eq!(
            w.devs[d].s().items(&everything()).len(),
            w.devs[0].s().items(&everything()).len()
        );
    }
}

/// All three devices rotate the same vault offline, each with an edit under
/// its new key, and a flaky client delivers: the canonical winner holds,
/// the losers rotate on until one key is left, and every edit is readable
/// everywhere.
#[test]
fn three_concurrent_rotations_over_a_flaky_client_converge() {
    let mut w = World::new(604, 3);
    let vault = w.new_vault(0, "Shared");
    let items: Vec<ItemId> = (0..3)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    let mut generation = None;
    for (d, item) in items.iter().enumerate() {
        w.devs[d]
            .s_mut()
            .update_item(vault, *item, |e| {
                e.set_secret(SecretField::Password, &format!("by {d}"))
            })
            .unwrap();
        let g = w.devs[d].s_mut().rotate_vault_key(vault).unwrap();
        assert!(generation.is_none_or(|x: KeyGeneration| x == g));
        generation = Some(g);
    }
    w.converge(4);
    let current = |w: &World, d: usize| {
        let keys = w.devs[d].s().store().vault_keys(&vault).unwrap();
        keys.iter()
            .max_by_key(|(g, _)| *g)
            .map(|(g, k)| (*g, k.as_bytes().to_vec()))
            .unwrap()
    };
    for d in 0..3 {
        assert_eq!(current(&w, d), current(&w, 0), "device {d}");
        for (i, item) in items.iter().enumerate() {
            assert_eq!(
                password_of(&w, d, vault, *item).as_deref(),
                Some(format!("by {i}").as_str()),
                "device {d}"
            );
        }
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "{:?}",
            w.devs[d].last.warnings
        );
    }
}

/// A rotates online; C takes the new key and edits under it; A, which has
/// not seen C's edit yet, rotates once more. Everyone reads C's segment
/// only after the second rotation, so the record is a generation behind
/// for all of them; C re-seals and sends it again under the newest key
/// when it takes that key, and the edit reaches everyone.
#[test]
fn edit_under_an_intermediate_generation_survives_a_second_rotation() {
    let mut w = World::new(605, 3);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "item");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    let rotate = |w: &mut World| {
        let clock = w.clock.clone();
        let dev = &mut w.devs[0];
        dev.engine
            .rotate_vault(&mut dev.access, &clock, vault)
            .unwrap()
    };
    rotate(&mut w);
    w.cloud.settle().unwrap();
    w.sync(2);
    set_password(&mut w, 2, vault, item, "by C under the middle key");
    w.sync(2);
    let c_hex = dir_name(w.devs[2].s().install_id().as_bytes());
    for r in [0, 1] {
        w.cloud.withhold(r, &format!("log/{c_hex}"), 6);
    }
    rounds(&mut w, 2);
    rotate(&mut w);
    w.sync(1);
    w.converge(1);
    let g = w.devs[0]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    assert_eq!(g.get(), 3);
    for d in 0..3 {
        assert!(
            shows(&w, d, vault, item, "by C under the middle key"),
            "device {d}"
        );
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "{:?}",
            w.devs[d].last.warnings
        );
    }
}
