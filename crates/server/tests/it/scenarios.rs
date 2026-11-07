//! Named hosted scenarios (M6 T4): the rotation, delete, move, join and
//! password change cases of the milestone's review log, driven through the
//! sync engine, the HTTP transport and the real server on 127.0.0.1.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mosaic_core::model::{ItemType, SecretField};
use mosaic_core::{ItemId, KeyGeneration, VaultId};
use mosaic_sync::TransportError;
use mosaic_sync::engine::{EngineConfig, RotationOutcome, SyncEngine, SyncError};
use mosaic_sync::http::hosted;
use mosaic_sync::testutil::{ChaosConfig, FixedJitter};
use mosaic_sync::transport::{Rejection, Transport, TransportKind};
use mosaic_vault::session::KeyOrigin;
use mosaic_vault::storage::RecordKind;

use crate::hosted::{Dev, Intercept, PASSWORD, World, everything, pw};
use crate::sim_hosted::has_password;

fn calm(seed: u64) -> World {
    World::new(seed, ChaosConfig::default())
}

pub(crate) fn add_login(w: &mut World, d: usize, vault: VaultId, title: &str) -> ItemId {
    w.devs[d]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, "pw")
        })
        .unwrap()
}

pub(crate) fn set_password(w: &mut World, d: usize, vault: VaultId, item: ItemId, value: &str) {
    w.devs[d]
        .s_mut()
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, value))
        .unwrap();
}

pub(crate) fn has_pw(w: &World, d: usize, vault: VaultId, item: ItemId, value: &str) -> bool {
    has_password(w.devs[d].s().item(vault, item).unwrap(), value)
}

pub(crate) fn generation(w: &World, d: usize, vault: VaultId) -> KeyGeneration {
    w.devs[d]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation
}

/// Two devices at rest sharing one vault with one item.
pub(crate) fn two_devices(seed: u64) -> (World, VaultId, ItemId) {
    let mut w = calm(seed);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    w.join();
    w.converge(0);
    (w, vault, item)
}

/// Three devices at rest sharing one vault with three items.
pub(crate) fn three_devices(seed: u64) -> (World, VaultId, Vec<ItemId>) {
    let mut w = calm(seed);
    let vault = w.new_vault(0, "Shared");
    let items: Vec<ItemId> = (0..3)
        .map(|i| add_login(&mut w, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0);
    w.join();
    w.join();
    w.converge(0);
    (w, vault, items)
}

pub(crate) fn engine_for<T: Transport>(t: T) -> SyncEngine<T> {
    SyncEngine::with_jitter(
        t,
        EngineConfig::for_kind(TransportKind::Hosted),
        Box::new(FixedJitter(1000)),
    )
}

/// M2-R12: a rotation offers its key on top of the vault's head; another
/// device's edit that lands in between moves the head, the key is refused,
/// the engine pulls and pushes again and offers the same key once more. The
/// edit is neither lost nor sealed under a stale key.
#[test]
fn rotation_head_cas_loses_no_edit() {
    let (mut w, vault, item) = two_devices(11);
    let fired = Arc::new(AtomicBool::new(false));
    let b: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(1)));
    let hook = {
        let (b, fired, clock) = (b.clone(), fired.clone(), w.clock.clone());
        move || {
            let mut dev = b.lock().unwrap();
            dev.s_mut()
                .update_item(vault, item, |e| {
                    e.set_secret(SecretField::Password, "from-B")
                })
                .unwrap();
            let dev = &mut *dev;
            dev.engine.sync_once(&mut dev.access, &clock).unwrap();
            fired.store(true, Ordering::SeqCst);
        }
    };
    let (opw, device) = (w.opw(0), w.devs[0].device);
    let mut transport = Intercept::new(w.login_transport(device, "device 0", &opw));
    transport.before_key_push = Some(Box::new(hook));
    let mut engine = engine_for(transport);
    let outcome = engine
        .rotate_vault(&mut w.devs[0].access, &w.clock, vault)
        .unwrap();
    assert!(
        fired.load(Ordering::SeqCst),
        "the other device acted in between"
    );
    let RotationOutcome::Rotated(g) = outcome else {
        panic!("the key won: {outcome:?}");
    };
    assert_eq!(g.get(), 2);
    drop(engine);
    let b = Arc::try_unwrap(b).ok().unwrap().into_inner().unwrap();
    w.devs.insert(1, b);
    w.relogin(0);
    w.converge(0);
    for d in 0..2 {
        assert_eq!(generation(&w, d, vault), g);
        assert!(has_pw(&w, d, vault, item, "from-B"), "device {d}");
        assert_eq!(w.devs[d].s().corrupt_records().len(), 0);
        assert_eq!(w.devs[d].s().pending_uploads(1).unwrap().len(), 0);
        assert_eq!(w.devs[d].last.warnings.len(), 0);
    }
}

/// M2-R7, R13: a delete and an edit race; whichever reaches the server second
/// finds the other's version (its base sequence number is stale), pulls,
/// merges and pushes the merge: the write wins, in both orders and when the
/// other device acts between the pull and the push.
#[test]
fn delete_race_linearized_by_base_seq() {
    // The edit lands between the deleting device's pull and its push.
    {
        let (mut w, vault, item) = two_devices(21);
        w.devs[0].s_mut().purge_item(vault, item).unwrap();
        set_password(&mut w, 1, vault, item, "edit");
        let fired = Arc::new(AtomicBool::new(false));
        let b: Arc<Mutex<Dev>> = Arc::new(Mutex::new(w.devs.remove(1)));
        let hook = {
            let (b, fired, clock) = (b.clone(), fired.clone(), w.clock.clone());
            move || {
                let mut dev = b.lock().unwrap();
                let dev = &mut *dev;
                dev.engine.sync_once(&mut dev.access, &clock).unwrap();
                fired.store(true, Ordering::SeqCst);
            }
        };
        let (opw, device) = (w.opw(0), w.devs[0].device);
        let mut transport = Intercept::new(w.login_transport(device, "device 0", &opw));
        transport.before_push_records = Some(Box::new(hook));
        let mut engine = engine_for(transport);
        let report = engine.sync_once(&mut w.devs[0].access, &w.clock).unwrap();
        assert!(fired.load(Ordering::SeqCst));
        assert!(report.merged >= 1, "the delete met the edit: {report:?}");
        drop(engine);
        w.devs
            .insert(1, Arc::try_unwrap(b).ok().unwrap().into_inner().unwrap());
        w.relogin(0);
        w.converge(0);
        for d in 0..2 {
            let s = w.devs[d].s();
            let it = s.item(vault, item).unwrap();
            assert!(!it.is_deleted(), "device {d}: the write wins");
            assert!(has_password(it, "edit"));
            assert!(it.deleted_concurrently().is_some());
        }
    }
    // The deleting device syncs first, or last.
    for order in [[0, 1], [1, 0]] {
        let (mut w, vault, item) = two_devices(22);
        w.devs[0].s_mut().purge_item(vault, item).unwrap();
        set_password(&mut w, 1, vault, item, "edit");
        for d in order {
            w.sync(d);
        }
        w.converge(0);
        for d in 0..2 {
            let s = w.devs[d].s();
            let it = s.item(vault, item).unwrap();
            assert!(!it.is_deleted(), "order {order:?}, device {d}");
            assert!(has_password(it, "edit"));
        }
    }
    // A delete without a concurrent write wins.
    let (mut w, vault, item) = two_devices(23);
    w.devs[0].s_mut().purge_item(vault, item).unwrap();
    w.converge(0);
    assert!(w.devs[1].s().item(vault, item).unwrap().is_deleted());
}

/// M2-R12: a moved item's copy reaches the server before the source's
/// tombstone, so a crash or a lost connection in between loses neither.
#[test]
fn move_copy_uploaded_before_tombstone() {
    let (mut w, vault, item) = two_devices(31);
    let other = w.new_vault(0, "Other");
    w.converge(0);
    let copy = w.devs[0].s_mut().move_item(vault, item, other).unwrap();
    let (opw, device) = (w.opw(0), w.devs[0].device);
    let transport = Intercept::new(w.login_transport(device, "device 0", &opw));
    let pushes = transport.pushes.clone();
    let mut engine = engine_for(transport);
    engine.sync_once(&mut w.devs[0].access, &w.clock).unwrap();
    let log = pushes.lock().unwrap().clone();
    let copy_at = log
        .iter()
        .position(|p| p.vault == other && p.kinds.contains(&RecordKind::Item))
        .expect("the copy was pushed");
    let tombstone_at = log
        .iter()
        .position(|p| p.vault == vault && p.kinds.contains(&RecordKind::Tombstone))
        .expect("the source tombstone was pushed");
    assert!(copy_at < tombstone_at, "{log:?}");
    drop(engine);
    w.relogin(0);
    w.converge(0);
    let s = w.devs[1].s();
    assert!(!s.item(other, copy).unwrap().is_deleted());
    assert!(s.item(vault, item).unwrap().is_deleted());
}

/// A rotation made offline (queued) by two devices at once: the server takes
/// one key per generation, the later device's key is refused, it adopts the
/// server's key for that generation (no further generation), re-seals what it
/// holds under it and nothing readable is lost, whichever device is first and
/// whichever has the lower install ID.
#[test]
fn legacy_offline_rotation_loser_adopts_server_key() {
    for first in [0usize, 1] {
        let second = 1 - first;
        let (mut w, vault, items) = three_devices(41 + first as u64);
        let g0 = generation(&w, 0, vault);
        set_password(&mut w, 0, vault, items[0], "edit by 0");
        w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
        set_password(&mut w, 1, vault, items[1], "edit by 1");
        w.devs[1].s_mut().rotate_vault_key(vault).unwrap();
        set_password(&mut w, 2, vault, items[2], "edit by 2");
        // The first device's key takes the slot; the second one's is refused.
        w.sync(first);
        w.sync(second);
        w.converge(0);

        let g = generation(&w, 0, vault);
        assert_eq!(g.get(), g0.get() + 1, "first {first}: no further rotation");
        for d in 0..3 {
            assert_eq!(generation(&w, d, vault), g);
            assert!(has_pw(&w, d, vault, items[0], "edit by 0"), "device {d}");
            assert!(has_pw(&w, d, vault, items[1], "edit by 1"), "device {d}");
            assert!(has_pw(&w, d, vault, items[2], "edit by 2"), "device {d}");
            assert_eq!(w.devs[d].s().corrupt_records().len(), 0);
            assert_eq!(w.devs[d].s().pending_uploads(1).unwrap().len(), 0);
            assert!(
                w.devs[d].last.warnings.is_empty(),
                "{:?}",
                w.devs[d].last.warnings
            );
        }
        let origin = |d: usize| w.devs[d].s().vault_key_origin(vault, g).unwrap();
        assert_eq!(
            origin(first),
            Some(KeyOrigin::Local),
            "the winner's own key"
        );
        assert_eq!(
            origin(second),
            Some(KeyOrigin::Remote),
            "the loser adopted the server's key"
        );
        let current_key = |d: usize| {
            w.devs[d]
                .s()
                .store()
                .vault_keys(&vault)
                .unwrap()
                .into_iter()
                .max_by_key(|(g, _)| *g)
                .map(|(_, k)| k.as_bytes().to_vec())
        };
        assert_eq!(current_key(0), current_key(1));
        assert_eq!(current_key(0), current_key(2));
    }
}

/// A new device joins over the wire with the email, the password and the
/// Secret Key, converges and edits; a wrong password does not join.
#[test]
fn join_new_device_hosted() {
    let (mut w, vault, item) = two_devices(51);
    assert_eq!(
        w.try_join_with("not the password").err(),
        Some(TransportError::Rejected(Rejection::LoginFailed))
    );
    let d = w.join();
    assert_eq!(d, 2);
    assert_eq!(w.devs[2].s().items(&everything()).len(), 0, "nothing yet");
    w.converge(0);
    assert!(has_pw(&w, 2, vault, item, "pw"));
    assert_eq!(generation(&w, 2, vault), generation(&w, 0, vault));
    set_password(&mut w, 2, vault, item, "from the new device");
    w.converge(0);
    for d in 0..3 {
        assert!(has_pw(&w, d, vault, item, "from the new device"));
    }
    // The server lists the three devices.
    let devices = w.devs[0].engine.transport_mut().devices().unwrap();
    assert_eq!(devices.len(), 3);
}

/// A master password change over the wire: a fresh OPAQUE login with the old
/// password, re-registration with the new one, the new wrapped key and salt
/// in one upload. Afterwards only the new password logs in or joins, and the
/// devices keep syncing.
#[test]
fn password_change_hosted_end_to_end() {
    let (mut w, vault, item) = two_devices(61);
    let new_password = "a completely different passphrase";
    let prepared = w.devs[0]
        .s()
        .prepare_master_password_change(&pw(PASSWORD), &pw(new_password), &w.secret_key)
        .unwrap();
    // Nothing is committed locally until the server took the change.
    w.server.advance_ms(60_000);
    let changed = hosted::password_change(
        &w.server.base,
        &w.email,
        &prepared,
        w.devs[0].device,
        "device 0",
        None,
    )
    .unwrap();
    assert_eq!(changed.account_version, 2);
    // The fresh login of the change replaced device 0's session (one per
    // device); device 0 goes on with it, no second login is needed.
    let device0 = w.devs[0].device;
    let transport = crate::hosted::connect(&w.server, &changed.grant, device0);
    w.devs[0].engine = w.engine_over(transport, 0);
    // The change ended every other session of the account: device 1's token
    // is dead, and it signs in again with the new password.
    assert!(matches!(
        w.try_sync(1),
        Err(SyncError::Transport(TransportError::Unauthorized))
    ));
    w.server.advance_ms(60_000);
    let device1 = w.devs[1].device;
    let grant = hosted::login(
        &w.server.base,
        &w.email,
        w.account,
        prepared.new_opaque_password(),
        device1,
        "device 1",
    )
    .unwrap();
    let transport = crate::hosted::connect(&w.server, &grant, device1);
    w.devs[1].engine = w.engine_over(transport, 1);
    w.devs[0]
        .s_mut()
        .commit_master_password_change(prepared)
        .unwrap();

    assert_eq!(
        w.try_join_with(PASSWORD).err(),
        Some(TransportError::Rejected(Rejection::LoginFailed)),
        "the old password no longer opens the account"
    );
    let d = w.try_join_with(new_password).unwrap();
    // Device 1 carries on with its new session.
    set_password(&mut w, 1, vault, item, "after the change");
    w.converge(0);
    for dev in 0..3 {
        assert!(
            has_pw(&w, dev, vault, item, "after the change"),
            "device {dev}"
        );
    }
    // The new device unlocks with the new password later on.
    let secret_key = &w.secret_key;
    let locked = w.devs[d].access.take().unwrap().lock();
    assert!(locked.unlock(&pw(new_password), secret_key).is_ok());
}

/// C-L3. The server stored a password change and its answer was lost. The
/// retry cannot log in with the old password any more; it logs in with the
/// new one, finds the prepared wrapped account key on the server and returns
/// as if the change had just gone through, so the device commits it locally.
/// A change that is not the one on the server stays the original error.
#[test]
fn password_change_lost_answer_is_recovered_on_retry() {
    let (mut w, _vault, _item) = two_devices(62);
    let new_password = "a passphrase for the lost answer";
    let prepared = w.devs[0]
        .s()
        .prepare_master_password_change(&pw(PASSWORD), &pw(new_password), &w.secret_key)
        .unwrap();
    let stale = w.devs[0]
        .s()
        .prepare_master_password_change(&pw(PASSWORD), &pw("a third passphrase"), &w.secret_key)
        .unwrap();
    let device0 = w.devs[0].device;
    let change = |w: &World, prepared: &mosaic_vault::session::PreparedPasswordChange| {
        w.server.advance_ms(60_000);
        hosted::password_change(
            &w.server.base,
            &w.email,
            prepared,
            device0,
            "device 0",
            None,
        )
    };
    // The upload goes through; the device never sees the answer.
    assert_eq!(change(&w, &prepared).unwrap().account_version, 2);
    // The retry recovers: same version, a session for the new password.
    let again = change(&w, &prepared).unwrap();
    assert_eq!(again.account_version, 2);
    let transport = crate::hosted::connect(&w.server, &again.grant, device0);
    w.devs[0].engine = w.engine_over(transport, 0);
    w.devs[0]
        .s_mut()
        .commit_master_password_change(prepared)
        .unwrap();
    let d = w.try_join_with(new_password).unwrap();
    assert!(d >= 2);

    // A change prepared before, for another password, is not what the server
    // holds: neither the old password nor its own new one logs in, and the
    // answer is the original error.
    assert_eq!(
        change(&w, &stale).err(),
        Some(TransportError::Rejected(Rejection::LoginFailed))
    );
}
