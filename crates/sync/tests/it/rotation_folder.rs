//! Concurrent vault key rotations over the folder backend: there is no
//! server to order them, so the canonical-winner rule does (CRYPTO_SPEC
//! §10.1, M6 spec §5.5).

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration tests"
)]

use std::fs;

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{ItemId, KeyGeneration, VaultId};
use mosaic_sync::engine::RotationOutcome;
use mosaic_sync::wire::folder_names::dir_name;
use mosaic_vault::session::KeyOrigin;

use crate::common::{World, everything};

/// Three devices at rest, one vault with three items.
fn three_devices() -> (World, VaultId, [ItemId; 3]) {
    let mut w = World::new(21, 3);
    let vault = w.new_vault(0, "Shared");
    let items = [0, 1, 2].map(|i| {
        w.devs[0]
            .s_mut()
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(&format!("item {i}"))?;
                e.set_secret(SecretField::Password, &format!("initial {i}"))
            })
            .unwrap()
    });
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.join();
    w.converge(0);
    (w, vault, items)
}

fn generation(w: &World, d: usize, vault: VaultId) -> KeyGeneration {
    w.devs[d]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation
}

fn set_password(w: &mut World, d: usize, vault: VaultId, item: ItemId, value: &str) {
    w.devs[d]
        .s_mut()
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, value))
        .unwrap();
}

fn has_password(w: &World, d: usize, vault: VaultId, item: ItemId, value: &str) -> bool {
    let session = w.devs[d].s();
    let it = session.item(vault, item).unwrap();
    it.secret(SecretField::Password)
        .unwrap()
        .is_some_and(|s| s.expose_secret() == value)
        || it
            .password_history()
            .iter()
            .any(|h| h.value.expose_secret() == value)
}

/// The two devices of `rotators` ordered as (winner, loser): the winner has
/// the lower install ID.
fn winner_and_loser(w: &World, a: usize, b: usize) -> (usize, usize) {
    let ia = *w.devs[a].s().install_id().as_bytes();
    let ib = *w.devs[b].s().install_id().as_bytes();
    if ia < ib { (a, b) } else { (b, a) }
}

fn all_agree(w: &World, vault: VaultId) {
    let keys0 = w.devs[0].s().store().vault_keys(&vault).unwrap();
    let items0 = w.devs[0].s().items(&everything()).len();
    for d in 1..w.devs.len() {
        assert_eq!(generation(w, d, vault), generation(w, 0, vault));
        let s = w.devs[d].s();
        // The key of the current generation is the same bytes everywhere.
        let current = |keys: &[(KeyGeneration, mosaic_core::WrappedKey)]| {
            keys.iter()
                .max_by_key(|(g, _)| *g)
                .map(|(g, k)| (*g, k.as_bytes().to_vec()))
        };
        assert_eq!(
            current(&s.store().vault_keys(&vault).unwrap()),
            current(&keys0),
            "device {d} holds another current key"
        );
        assert_eq!(s.items(&everything()).len(), items0);
    }
}

#[test]
fn concurrent_rotations_converge_by_canonical_winner() {
    let (mut w, vault, items) = three_devices();
    // Devices 0 and 1 rotate offline, at the same time, each with an edit of
    // its own under its new key.
    set_password(&mut w, 0, vault, items[0], "edit by 0");
    let g0 = w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    set_password(&mut w, 1, vault, items[1], "edit by 1");
    let g1 = w.devs[1].s_mut().rotate_vault_key(vault).unwrap();
    assert_eq!(g0, g1, "both took the next generation");
    set_password(&mut w, 2, vault, items[2], "edit by 2");

    w.converge(1);

    // The loser rotated once more; everyone is on that generation.
    let (winner, loser) = winner_and_loser(&w, 0, 1);
    let g = generation(&w, 0, vault);
    assert_eq!(g.get(), g0.get() + 1, "exactly one more rotation");
    all_agree(&w, vault);
    let origin = |d: usize, g: KeyGeneration| w.devs[d].s().vault_key_origin(vault, g).unwrap();
    assert_eq!(
        origin(loser, g),
        Some(KeyOrigin::Local),
        "the loser's new key"
    );
    assert_eq!(
        origin(winner, g0),
        Some(KeyOrigin::Local),
        "the winner kept its key"
    );
    for d in 0..3 {
        assert!(has_password(&w, d, vault, items[0], "edit by 0"));
        assert!(has_password(&w, d, vault, items[1], "edit by 1"));
        assert!(has_password(&w, d, vault, items[2], "edit by 2"));
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "{:?}",
            w.devs[d].last.warnings
        );
    }
}

#[test]
fn loser_rotates_again_and_nothing_readable_is_lost() {
    let (mut w, vault, items) = three_devices();
    // Both rotate online through the engine, then keep editing under the
    // new key before they see each other.
    for d in [0, 1] {
        let dev = &mut w.devs[d];
        let out = dev
            .engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
        assert!(matches!(out, RotationOutcome::Rotated(_)));
    }
    set_password(&mut w, 0, vault, items[0], "after rotation, device 0");
    w.sync(0);
    set_password(&mut w, 1, vault, items[1], "after rotation, device 1");
    w.sync(1);
    // Device 2 does not rotate and edits under the old key.
    set_password(&mut w, 2, vault, items[2], "old key, device 2");

    w.converge(2);

    all_agree(&w, vault);
    assert!(generation(&w, 0, vault).get() >= 3);
    for d in 0..3 {
        assert!(
            has_password(&w, d, vault, items[0], "after rotation, device 0"),
            "device {d}"
        );
        assert!(
            has_password(&w, d, vault, items[1], "after rotation, device 1"),
            "device {d}"
        );
        assert!(
            has_password(&w, d, vault, items[2], "old key, device 2"),
            "device {d}"
        );
        assert_eq!(w.devs[d].s().corrupt_records().len(), 0);
        assert_eq!(w.devs[d].s().pending_uploads(1).unwrap().len(), 0);
    }
}

#[test]
fn third_device_holding_loser_key_rotates() {
    let (mut w, vault, items) = three_devices();
    let (winner, loser) = winner_and_loser(&w, 0, 1);
    for d in [0, 1] {
        let dev = &mut w.devs[d];
        dev.engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
    }
    // Device 2 sees the loser's key first: the winner's files reach it a few
    // rounds later.
    let winner_dir = dir_name(w.devs[winner].s().install_id().as_bytes());
    w.cloud.withhold(2, &winner_dir, 6);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    w.sync(2);
    let g2 = generation(&w, 2, vault);
    assert_eq!(g2.get(), 2, "device 2 took the loser's key");
    assert_eq!(
        w.devs[2].s().vault_key_origin(vault, g2).unwrap(),
        Some(KeyOrigin::Remote)
    );
    let _ = loser;
    for _ in 0..8 {
        w.cloud.round().unwrap();
    }
    w.sync(2);
    // Device 2 held the losing key: it rotated.
    let g_now = generation(&w, 2, vault);
    assert!(g_now.get() >= 3, "device 2 moved on: {g_now:?}");
    let rotated_here = {
        let session = w.devs[2].s();
        session
            .store()
            .vault_keys(&vault)
            .unwrap()
            .iter()
            .any(|(g, _)| {
                g.get() >= 3
                    && session.vault_key_origin(vault, *g).unwrap() == Some(KeyOrigin::Local)
            })
    };
    assert!(rotated_here, "device 2 generated a key of its own");

    set_password(&mut w, 2, vault, items[2], "edit by 2 after");
    w.converge(2);
    all_agree(&w, vault);
    for d in 0..3 {
        assert!(has_password(&w, d, vault, items[2], "edit by 2 after"));
        assert!(
            w.devs[d].last.warnings.is_empty(),
            "{:?}",
            w.devs[d].last.warnings
        );
    }
}

#[test]
fn records_ahead_of_their_key_are_deferred_not_skipped() {
    let (mut w, vault, items) = three_devices();
    let a_dir = w
        .replica(0)
        .join("a")
        .join(dir_name(w.account.as_bytes()))
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("log")
        .join(dir_name(w.devs[0].s().install_id().as_bytes()));
    let before = fs::read_dir(&a_dir).unwrap().count();
    {
        let dev = &mut w.devs[0];
        dev.engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
    }
    set_password(&mut w, 0, vault, items[0], "under the new key");
    w.sync(0);
    let mut names: Vec<String> = fs::read_dir(&a_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    // The first new file is the key; withhold it from device 1.
    let key_segment = names[before].clone();
    w.cloud.withhold(1, &key_segment, 4);
    w.cloud.round().unwrap();
    w.cloud.round().unwrap();

    let report = w.sync(1);
    assert_eq!(report.undecryptable, 0, "{report:?}");
    assert_eq!(report.rollback_dropped, 0);
    assert_eq!(report.warnings.len(), 0);
    assert!(!has_password(&w, 1, vault, items[0], "under the new key"));
    assert_eq!(generation(&w, 1, vault).get(), 1);
    // Still deferred, not skipped: the cursor did not move past the records.
    let again = w.sync(1);
    assert_eq!(again.pulled, 0);
    assert!(!has_password(&w, 1, vault, items[0], "under the new key"));

    // The key arrives; the deferred records are read again and apply.
    for _ in 0..6 {
        w.cloud.round().unwrap();
    }
    let report = w.sync(1);
    assert!(report.pulled >= 1, "{report:?}");
    assert_eq!(generation(&w, 1, vault).get(), 2);
    assert!(has_password(&w, 1, vault, items[0], "under the new key"));
    w.converge(1);
    all_agree(&w, vault);
}

#[test]
fn a_rotation_pushed_but_not_committed_is_recovered_from_the_own_log() {
    use mosaic_sync::transport::Transport;

    let (mut w, vault, items) = three_devices();
    // Device 0 writes its new key to the folder and crashes before it commits
    // the rotation: the prepared secret is gone.
    let prepared = w.devs[0].s().prepare_vault_rotation(vault).unwrap();
    let install = w.devs[0].s().install_id();
    w.devs[0]
        .engine
        .transport_mut()
        .push_vault_key(
            vault,
            prepared.generation(),
            prepared.wrapped(),
            install,
            None,
        )
        .unwrap();
    drop(prepared);
    w.cloud.settle().unwrap();

    // The others take the key and write under it.
    w.sync(1);
    assert_eq!(generation(&w, 1, vault).get(), 2);
    set_password(&mut w, 1, vault, items[1], "written under the new key");
    w.sync(1);
    w.cloud.settle().unwrap();

    // Device 0 still holds generation 1 and learns its own key from its own
    // log, so it can read what the others wrote.
    assert_eq!(generation(&w, 0, vault).get(), 1);
    w.converge(1);
    assert_eq!(generation(&w, 0, vault).get(), 2);
    for d in 0..3 {
        assert!(has_password(
            &w,
            d,
            vault,
            items[1],
            "written under the new key"
        ));
    }
    all_agree(&w, vault);
}

fn attachment_ids(
    w: &World,
    d: usize,
    vault: VaultId,
    item: ItemId,
) -> Vec<mosaic_core::AttachmentId> {
    let s = w.devs[d].s();
    let mut ids: Vec<_> = s
        .item(vault, item)
        .unwrap()
        .attachments()
        .iter()
        .map(|a| a.attachment_id)
        .collect();
    ids.sort_unstable_by_key(|a| *a.as_bytes());
    ids
}

#[test]
fn vault_rotate_key_reseals_attachments_and_resumes() {
    use mosaic_sync::engine::RekeyError;
    use mosaic_vault::session::{ResealStep, RotationKind};

    let (mut w, vault, items) = three_devices();
    for (i, item) in items.iter().take(2).enumerate() {
        w.devs[0]
            .s_mut()
            .add_attachment(
                vault,
                *item,
                "f.bin",
                "application/octet-stream",
                &[i as u8; 64],
            )
            .unwrap();
    }
    w.converge(0);
    let before: Vec<_> = items
        .iter()
        .map(|i| attachment_ids(&w, 0, vault, *i))
        .collect();
    assert_eq!(before[0].len() + before[1].len(), 2);

    // A crash mid re-seal: the key rotated and the queue is open, one
    // attachment is done.
    {
        let dev = &mut w.devs[0];
        dev.engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
        dev.s_mut().queue_vault_attachment_reseal(vault).unwrap();
        let step = dev.s_mut().reseal_next_attachment(None).unwrap();
        assert!(matches!(step, ResealStep::Resealed { .. }), "{step:?}");
        let job = dev.s().rotation_job().unwrap().unwrap();
        assert_eq!(job.kind, RotationKind::Vault);
        // No second rotation starts on top of an open job.
        assert!(matches!(
            dev.engine
                .rotate_vault_key(&mut dev.access, &w.clock, vault, true),
            Err(RekeyError::Session(
                mosaic_vault::session::Error::RotationInProgress
            ))
        ));
    }

    // The resume finishes the rest, uploads and closes the job.
    let progress = {
        let dev = &mut w.devs[0];
        dev.engine
            .vault_rotate_continue(&mut dev.access, &w.clock)
            .unwrap()
    };
    assert!(progress.finished, "{progress:?}");
    assert!(progress.unreachable_attachments.is_empty());
    assert!(w.devs[0].s().rotation_job().unwrap().is_none());
    {
        let dev = &mut w.devs[0];
        assert!(matches!(
            dev.engine.vault_rotate_continue(&mut dev.access, &w.clock),
            Err(RekeyError::NoJob)
        ));
    }
    let after: Vec<_> = items
        .iter()
        .map(|i| attachment_ids(&w, 0, vault, *i))
        .collect();
    for i in 0..2 {
        assert_eq!(after[i].len(), before[i].len());
        assert!(after[i].iter().all(|a| !before[i].contains(a)), "item {i}");
    }

    // The other devices converge on the new IDs and read the files.
    w.converge(1);
    for d in 1..3 {
        for (i, item) in items.iter().take(2).enumerate() {
            let ids = attachment_ids(&w, d, vault, *item);
            assert_eq!(ids, after[i], "device {d}");
            let data = w.devs[d].s().attachment(vault, *item, ids[0]).unwrap();
            assert_eq!(data.expose_secret(), &[i as u8; 64][..], "device {d}");
        }
    }
    all_agree(&w, vault);

    // The one-call path: rotate and re-seal in a single run.
    let outcome = {
        let dev = &mut w.devs[1];
        dev.engine
            .rotate_vault_key(&mut dev.access, &w.clock, vault, true)
            .unwrap()
    };
    assert!(matches!(outcome.outcome, RotationOutcome::Rotated(_)));
    assert!(outcome.progress.unwrap().finished);
    w.converge(1);
    for (i, item) in items.iter().take(2).enumerate() {
        let ids = attachment_ids(&w, 2, vault, *item);
        assert!(ids.iter().all(|a| !after[i].contains(a)));
        let data = w.devs[2].s().attachment(vault, *item, ids[0]).unwrap();
        assert_eq!(data.expose_secret(), &[i as u8; 64][..]);
    }
}
