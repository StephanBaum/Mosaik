//! A hostile server (M6 T4, CRYPTO_SPEC §10, `THREAT_MODEL`): the real
//! server's answers rewritten on their way to one device by a
//! `MaliciousTransport`. The device must keep what it has, refuse what does
//! not authenticate, notice a server that withholds or rolls back, and never
//! take a key it cannot open.

use mosaic_core::{
    AccountId, AccountKey, ItemId, KeyGeneration, TombstoneCiphertext, VaultId, VaultKey,
};
use mosaic_sync::engine::{SyncEngine, SyncError, SyncReport, SyncWarning};
use mosaic_sync::testutil::MaliciousTransport;
use mosaic_sync::transport::{PulledKey, PulledRecord, Transport};
use mosaic_vault::storage::RecordBody;

use crate::hosted::{LoopbackTransport, World, everything};
use crate::scenarios::{add_login, engine_for, has_pw, set_password, two_devices};

const MINUTE_MS: u64 = 60_000;

type Mal = SyncEngine<MaliciousTransport<LoopbackTransport>>;

/// Device 1 reading through a transport the test controls (its own session
/// on the server).
///
/// A login ends the other sessions of the same device, so device 1's own
/// engine needs `World::relogin(1)` before it is used again.
fn malicious_b(w: &World) -> Mal {
    let (opw, device) = (w.opw(1), w.devs[1].device);
    engine_for(MaliciousTransport::new(w.login_transport(
        device,
        "device 1, hostile path",
        &opw,
    )))
}

fn run(w: &mut World, engine: &mut Mal) -> SyncReport {
    w.server.advance_ms(MINUTE_MS);
    let dev = &mut w.devs[1];
    engine.sync_once(&mut dev.access, &w.clock).unwrap()
}

fn head_of(next: &mosaic_sync::transport::Cursor) -> u64 {
    u64::from_be_bytes(<[u8; 8]>::try_from(next.0.as_slice()).unwrap())
}

fn stored_body(w: &World, d: usize, vault: VaultId, item: ItemId) -> RecordBody {
    w.devs[d]
        .s()
        .store()
        .record(&vault, &item)
        .unwrap()
        .unwrap()
        .body
}

#[test]
fn rollback_to_old_ciphertext_ignored() {
    let (mut w, vault, item) = two_devices(71);
    let old = stored_body(&w, 0, vault, item);
    set_password(&mut w, 0, vault, item, "v2");
    w.sync(0);
    w.sync(1);
    assert!(has_pw(&w, 1, vault, item, "v2"));
    // Something else changes, so there is a page; the server adds the old
    // version of the item to it, under a newer sequence number.
    let other = add_login(&mut w, 0, vault, "other");
    w.sync(0);
    let mut engine = malicious_b(&w);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        let seq = head_of(&page.next) + 1;
        page.records.push(PulledRecord {
            record_id: item,
            body: old.clone(),
            seq: Some(seq),
        });
    }));
    let report = run(&mut w, &mut engine);
    assert!(report.pulled >= 1, "the real change arrived: {report:?}");
    assert!(has_pw(&w, 1, vault, other, "pw"));
    assert!(
        has_pw(&w, 1, vault, item, "v2"),
        "the old version replaced nothing"
    );
    assert_eq!(report.undecryptable, 0);
}

#[test]
fn record_swapped_between_items_or_vaults_fails_auth() {
    let (mut w, vault_a, item) = two_devices(72);
    let vault_b = w.new_vault(0, "Other");
    w.sync(0);
    w.sync(1);
    let body = stored_body(&w, 0, vault_a, item);
    // New changes in both vaults give the hostile server pages to alter.
    add_login(&mut w, 0, vault_a, "more in A");
    add_login(&mut w, 0, vault_b, "more in B");
    w.sync(0);
    let forged = ItemId::new_v7().unwrap();
    let mut engine = malicious_b(&w);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        if page.records.is_empty() {
            return;
        }
        let seq = head_of(&page.next) + 1;
        // One vault's ciphertext under another item's ID, and the same item
        // ID in the other vault: the AEAD binds vault and item.
        for id in [forged, item] {
            page.records.push(PulledRecord {
                record_id: id,
                body: body.clone(),
                seq: Some(seq),
            });
        }
    }));
    let report = run(&mut w, &mut engine);
    assert!(report.undecryptable >= 2, "{report:?}");
    assert!(
        report
            .warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::Undecryptable { .. })),
        "{report:?}"
    );
    let s = w.devs[1].s();
    assert!(s.item(vault_a, forged).is_err());
    assert!(s.item(vault_b, forged).is_err());
    assert!(s.item(vault_b, item).is_err());
    assert_eq!(s.item(vault_a, item).unwrap().title(), Some("first"));
}

#[test]
fn withheld_vault_freeze_suspected_after_grace() {
    let (mut w, vault, item) = two_devices(73);
    let mut engine = malicious_b(&w);
    engine.transport_mut().withhold_pulls = true;
    set_password(&mut w, 0, vault, item, "hidden");
    // A's checkpoint names what A has read: its own upload counts from the
    // run after the one that sent it.
    w.sync(0);
    w.sync(0);

    let r = run(&mut w, &mut engine);
    assert!(r.warnings.is_empty(), "within the grace: {r:?}");
    w.advance(5 * MINUTE_MS);
    let r = run(&mut w, &mut engine);
    assert!(r.warnings.is_empty(), "five minutes: {r:?}");
    w.advance(6 * MINUTE_MS);
    let r = run(&mut w, &mut engine);
    let a = w.devs[0].device;
    assert!(
        r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::FreezeSuspected { device, .. } if *device == a)),
        "{r:?}"
    );
    assert!(has_pw(&w, 1, vault, item, "pw"), "nothing leaked in");
    // An honest path ends the suspicion.
    w.relogin(1);
    w.sync(1);
    assert!(has_pw(&w, 1, vault, item, "hidden"));
    assert_eq!(w.devs[1].last.warnings.len(), 0);
}

#[test]
fn whole_server_rollback_detected_via_own_checkpoint() {
    let (mut w, _vault, _item) = two_devices(74);
    let b = w.devs[1].device;
    let own = |w: &mut World| {
        w.devs[1]
            .engine
            .transport_mut()
            .inner_mut()
            .checkpoints()
            .unwrap()
            .into_iter()
            .find(|(d, _)| *d == b)
            .map(|(_, bytes)| bytes)
            .unwrap()
    };
    let old = own(&mut w);
    for _ in 0..3 {
        w.advance(1000);
        w.sync(1);
    }
    // The server comes back from a backup: this device's checkpoint is the
    // old one.
    let mut engine = malicious_b(&w);
    engine.transport_mut().hide_checkpoints_of.insert(b);
    engine.transport_mut().replace_checkpoints.push((b, old));
    // One lost upload is tolerated.
    run(&mut w, &mut engine);
    w.server.advance_ms(MINUTE_MS);
    let err = engine
        .sync_once(&mut w.devs[1].access, &w.clock)
        .unwrap_err();
    assert!(matches!(err, SyncError::RollbackDetected), "{err:?}");
    // Stopped until the user acts, whatever the path.
    let err = w.try_sync(1).unwrap_err();
    assert!(matches!(err, SyncError::RollbackDetected), "{err:?}");
}

#[test]
fn forged_tombstone_rejected() {
    let (mut w, vault, item) = two_devices(75);
    add_login(&mut w, 0, vault, "more");
    w.sync(0);
    // Sealed under a key the attacker made up, with a header that fits.
    let generation = w.devs[1]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    let attacker = VaultKey::generate(generation).unwrap();
    let tombstone = TombstoneCiphertext::seal(&attacker, &vault, &item, b"{}").unwrap();
    let mut engine = malicious_b(&w);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        let seq = head_of(&page.next) + 1;
        page.records.push(PulledRecord {
            record_id: item,
            body: RecordBody::Tombstone(tombstone.clone()),
            seq: Some(seq),
        });
    }));
    let report = run(&mut w, &mut engine);
    assert!(report.undecryptable >= 1, "{report:?}");
    let s = w.devs[1].s();
    assert!(!s.item(vault, item).unwrap().is_deleted());
    drop(s);
    assert!(has_pw(&w, 1, vault, item, "pw"));
    assert_eq!(w.devs[1].s().items(&everything()).len(), 2);
}

#[test]
fn stale_checkpoint_ignored() {
    let (mut w, vault, item) = two_devices(76);
    let a = w.devs[0].device;
    let a_checkpoint = |w: &mut World| {
        w.devs[1]
            .engine
            .transport_mut()
            .inner_mut()
            .checkpoints()
            .unwrap()
            .into_iter()
            .find(|(d, _)| *d == a)
            .map(|(_, bytes)| bytes)
            .unwrap()
    };
    let old = a_checkpoint(&mut w);
    for i in 0..2 {
        set_password(&mut w, 0, vault, item, &format!("edit {i}"));
        w.advance(1000);
        w.sync(0);
    }
    w.sync(1);
    let counter = |w: &World| {
        w.devs[1]
            .s()
            .peer_checkpoints()
            .unwrap()
            .into_iter()
            .find(|c| c.device_id == a)
            .map(|c| c.counter)
            .unwrap()
    };
    let newest = counter(&w);
    // The server serves the old checkpoint of A in place of the new one.
    let mut engine = malicious_b(&w);
    engine.transport_mut().hide_checkpoints_of.insert(a);
    engine.transport_mut().replace_checkpoints.push((a, old));
    let report = run(&mut w, &mut engine);
    assert!(report.warnings.is_empty(), "{report:?}");
    assert_eq!(counter(&w), newest, "an older checkpoint replaces nothing");
}

#[test]
fn injected_foreign_wrapped_key_rejected() {
    let (mut w, vault, _item) = two_devices(77);
    add_login(&mut w, 0, vault, "more");
    w.sync(0);
    let generation = w.devs[1]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    let next = KeyGeneration::new(generation.get() + 1).unwrap();
    // A key of the next generation, wrapped under an account key that is not
    // this account's, with this account's ID (and with another one's).
    let foreign = AccountKey::generate().unwrap();
    let key = VaultKey::generate(next).unwrap();
    let account = w.account;
    let other_account = AccountId::new_v7().unwrap();
    let wrapped = [
        key.wrap(&foreign, &account, &vault).unwrap(),
        key.wrap(&foreign, &other_account, &vault).unwrap(),
    ];
    let device = w.devs[0].device;
    let mut engine = malicious_b(&w);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        let seq = head_of(&page.next) + 1;
        for wrapped in &wrapped {
            page.keys.push(PulledKey {
                generation: next,
                wrapped: wrapped.clone(),
                device_id: device,
                seq: Some(seq),
            });
        }
    }));
    let report = run(&mut w, &mut engine);
    assert!(report.undecryptable >= 2, "{report:?}");
    let current = w.devs[1]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    assert_eq!(current, generation, "no foreign key was taken");
    // The honest path still works.
    w.relogin(1);
    w.sync(1);
    assert_eq!(w.devs[1].last.warnings.len(), 0);
}

/// C-M1. A server that shows device 1 a second valid key for a generation it
/// confirmed (cross-serving the keys of other devices) makes the device
/// rotate by the rank rule, but at most three times a day: then the rotating
/// stops, the warning stays and nothing readable is lost.
#[test]
fn cross_served_keys_warn_and_the_rotations_are_capped() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use mosaic_core::InstallId;

    let (mut w, vault, item) = two_devices(79);
    // Device 0 rotates offline six times and never syncs: it holds valid
    // keys (wrapped under the account key) for generations 2 to 7 that the
    // server never saw.
    for _ in 0..6 {
        w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    }
    let foreign = w.devs[0].s().store().vault_keys(&vault).unwrap();
    let generation_of_1 = |w: &World| {
        w.devs[1]
            .s()
            .vaults()
            .iter()
            .find(|v| v.vault_id == vault)
            .unwrap()
            .key_generation
    };
    // Device 1 rotates online through the hostile path: its key for
    // generation 2 is accepted and confirmed by the server.
    let mut engine = malicious_b(&w);
    w.server.advance_ms(MINUTE_MS);
    {
        let dev = &mut w.devs[1];
        engine
            .rotate_vault(&mut dev.access, &w.clock, vault)
            .unwrap();
    }
    assert_eq!(generation_of_1(&w).get(), 2);

    // From now on the server also shows device 1 a different valid key of the
    // device's current generation, as written by a device with a very low ID.
    let current = Arc::new(AtomicU32::new(2));
    let shown = current.clone();
    let low = InstallId::from_bytes([1; 16]).unwrap();
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        let g = shown.load(Ordering::SeqCst);
        if let Some((generation, wrapped)) = foreign.iter().find(|(gen_, _)| gen_.get() == g) {
            page.keys.push(PulledKey {
                generation: *generation,
                wrapped: wrapped.clone(),
                device_id: low,
                seq: Some(head_of(&page.next)),
            });
        }
    }));
    let mut generations = Vec::new();
    let mut warned = Vec::new();
    for _ in 0..6 {
        current.store(generation_of_1(&w).get(), Ordering::SeqCst);
        let report = run(&mut w, &mut engine);
        warned.push(
            report
                .warnings
                .iter()
                .any(|x| matches!(x, SyncWarning::ServerInconsistent { vault: v } if *v == vault)),
        );
        generations.push(generation_of_1(&w).get());
    }
    // Every run reports the server; the device rotated three times (rank
    // rule) and then stopped, however often the server repeats itself.
    assert!(warned.iter().all(|x| *x), "{warned:?}");
    assert_eq!(generations, [3, 4, 5, 5, 5, 5], "{generations:?}");
    // Nothing readable was lost along the way.
    assert_eq!(w.devs[1].s().corrupt_records().len(), 0);
    assert!(has_pw(&w, 1, vault, item, "pw"));
}
