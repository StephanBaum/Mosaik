//! Account re-key on the folder backend (M7 spec §7.5, CRYPTO_SPEC §6.6,
//! §6.7, §10.1): three devices, offline edits, the retention of the
//! pre-re-key own segments, objects of the old epoch, a deleted marker and
//! concurrent re-keys.

use mosaic_core::{Argon2Profile, ItemId, SecretKey, VaultId};
use mosaic_sync::engine::{RekeyError, RekeySecrets, SyncError, SyncWarning};
use mosaic_vault::session::{RekeyAdopted, SyncConfig, SyncKind};

use super::util::{DAY_MS, add_login, password_of, set_password};
use crate::common::{PASSWORD, World, files_ending, log_dir, pw, segments};

const NEW_PASSWORD: &str = "a different horse battery staple";

/// A copy of a Secret Key (it is not `Clone`).
fn copy_key(sk: &SecretKey) -> SecretKey {
    SecretKey::parse(sk.to_display().unwrap().expose_secret()).unwrap()
}

/// A world of `devices` devices that share one vault with one item and
/// converged, every device configured for the folder backend.
fn world(seed: u64, devices: usize) -> (World, VaultId, ItemId) {
    world_with(seed, devices, true)
}

fn world_with(seed: u64, devices: usize, with_vault: bool) -> (World, VaultId, ItemId) {
    let mut w = World::new(seed, devices);
    let (vault, item) = if with_vault {
        let vault = w.new_vault(0, "Shared");
        (vault, add_login(&mut w, 0, vault, "first"))
    } else {
        (
            VaultId::from_bytes([1; 16]).unwrap(),
            ItemId::from_bytes([2; 16]).unwrap(),
        )
    };
    w.sync(0);
    w.cloud.settle().unwrap();
    for _ in 1..devices {
        w.join();
    }
    for d in 0..devices {
        let config = SyncConfig {
            kind: SyncKind::Folder,
            server_url: None,
            email: None,
            folder_path: Some(w.replica(d)),
            device_name: format!("device {d}"),
        };
        w.devs[d].s_mut().set_sync_config(Some(&config)).unwrap();
    }
    w.converge(0);
    (w, vault, item)
}

/// Device `d` re-keys: from `(current_pw, current_sk)` to `(new_pw, new_sk)`.
fn rekey_as(
    w: &mut World,
    d: usize,
    current: (&str, &SecretKey),
    new: (&str, &SecretKey),
) -> Result<(), RekeyError> {
    let (current_password, new_password) = (pw(current.0), pw(new.0));
    let secrets = RekeySecrets {
        current_password: &current_password,
        current_secret_key: current.1,
        new_password: &new_password,
        new_secret_key: new.1,
        profile: Argon2Profile::Baseline,
    };
    let World { devs, clock, .. } = w;
    let dev = &mut devs[d];
    dev.engine
        .rekey_folder(&mut dev.access, clock, &secrets)
        .map(|_| ())
}

/// Finishes the job of device `d`: attachments, uploads, close.
fn finish_job(w: &mut World, d: usize) {
    let World { devs, clock, .. } = w;
    let dev = &mut devs[d];
    let progress = dev.engine.rekey_continue(&mut dev.access, clock).unwrap();
    assert!(progress.finished, "{progress:?}");
    assert!(
        dev.access
            .session()
            .unwrap()
            .rotation_job()
            .unwrap()
            .is_none()
    );
}

fn rekey(w: &mut World, d: usize, new_sk: &SecretKey) -> Result<(), RekeyError> {
    let current = copy_key(&w.secret_key);
    rekey_as(w, d, (PASSWORD, &current), (NEW_PASSWORD, new_sk))
}

/// Device `d` adopts the re-key with the typed `(password, secret key)`; its
/// own current Secret Key is `current_sk`.
fn adopt_as(
    w: &mut World,
    d: usize,
    typed: (&str, &SecretKey),
    current_sk: &SecretKey,
) -> Result<RekeyAdopted, RekeyError> {
    let password = pw(typed.0);
    let World { devs, clock, .. } = w;
    let dev = &mut devs[d];
    dev.engine
        .adopt_folder_rekey(&mut dev.access, clock, &password, typed.1, current_sk)
}

fn adopt(w: &mut World, d: usize, new_sk: &SecretKey) -> Result<RekeyAdopted, RekeyError> {
    let current = copy_key(&w.secret_key);
    adopt_as(w, d, (NEW_PASSWORD, new_sk), &current)
}

fn epoch(w: &World, d: usize) -> u32 {
    w.devs[d].s().ak_epoch()
}

/// The unverifiable-account-file findings of a report.
fn findings(w: &mut World, d: usize) -> Vec<(String, [u8; 32], u32)> {
    w.sync(d)
        .warnings
        .into_iter()
        .filter_map(|x| match x {
            SyncWarning::UnverifiedAccountFile {
                name,
                sha256,
                epoch,
            } => Some((name, sha256, epoch)),
            _ => None,
        })
        .collect()
}

#[test]
fn folder_rekey_three_devices_offline_peer_edits_survive() {
    let (mut w, vault, item) = world(11, 3);
    // Device 2 edits while it is offline; device 1 adds an item and syncs.
    set_password(&mut w, 2, vault, item, "edited offline by device 2");
    let extra = add_login(&mut w, 1, vault, "added by device 1");
    w.sync(1);
    w.cloud.settle().unwrap();

    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    assert_eq!(epoch(&w, 0), 2);
    assert!(w.devs[0].s().rotation_job().unwrap().is_some());
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();

    // A device of the old epoch stops at the verified marker, before any write.
    assert!(matches!(w.try_sync(1), Err(SyncError::AccountRekeyed)));
    assert!(w.devs[1].s().rekey_pending().unwrap());
    assert!(matches!(w.try_sync(1), Err(SyncError::AccountRekeyed)));

    adopt(&mut w, 1, &new_sk).unwrap();
    adopt(&mut w, 2, &new_sk).unwrap();
    assert!(!w.devs[1].s().rekey_pending().unwrap());
    w.converge(0);
    // The job of the re-keying device closes once everything is uploaded.
    w.sync(0);
    w.converge(0);

    for d in 0..3 {
        assert_eq!(epoch(&w, d), 2, "device {d}");
        assert_eq!(
            password_of(&w, d, vault, item).as_deref(),
            Some("edited offline by device 2"),
            "device {d} keeps the edit made offline"
        );
        assert!(w.devs[d].s().item(vault, extra).is_ok(), "device {d}");
    }
    // Every device holds the same single key generation per vault.
    let g = w.devs[0].s().vaults()[0].key_generation;
    assert_eq!(g.get(), 2);
    for d in 1..3 {
        assert_eq!(w.devs[d].s().vaults()[0].key_generation, g);
    }
}

#[test]
fn folder_pre_rekey_own_segments_kept_until_snapshot_and_retention() {
    let (mut w, vault, item) = world(12, 2);
    set_password(&mut w, 0, vault, item, "before the re-key");
    w.sync(0);
    let dir = log_dir(&w, 0, 0, vault);
    let before: Vec<_> = segments(&dir);
    assert_ne!(before.len(), 0);

    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    let floors = w.devs[0].s().folder_rekey_floors().unwrap();
    assert_eq!(floors.len(), 1);
    assert!(floors[0].snapshot_done, "the snapshot set was written");
    let all = segments(&dir);
    assert!(all.len() > before.len(), "the new snapshot set is there");

    // Two days later: the 24-hour rule for unverifiable files does not apply
    // to the pre-re-key segments.
    w.advance(2 * DAY_MS);
    w.sync(0);
    for f in &before {
        assert!(f.exists(), "{f:?} stays within the retention");
    }
    // After the retention (30 days) they go, with the snapshot complete.
    w.advance(31 * DAY_MS);
    w.sync(0);
    for f in &before {
        assert!(!f.exists(), "{f:?} is removed after the retention");
    }
    assert!(!segments(&dir).is_empty(), "the new epoch's segments stay");
}

#[test]
fn folder_old_epoch_forged_segment_ignored_after_rekey() {
    let (mut w, vault, _item) = world(13, 2);
    // A device that still holds the old keys writes after the re-key (here:
    // its segment is in the folder before the re-key, delivered after).
    let stale = add_login(&mut w, 1, vault, "written under the old epoch");
    w.sync(1);
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    w.cloud.settle().unwrap();
    w.sync(0);
    w.sync(0);
    assert!(
        w.devs[0].s().item(vault, stale).is_err(),
        "an object of the old epoch is never accepted at the new one"
    );
    assert_eq!(epoch(&w, 0), 2);
}

#[test]
fn folder_marker_deleted_stale_device_gets_a_finding_and_can_adopt() {
    let (mut w, vault, item) = world(14, 2);
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    w.cloud.settle().unwrap();
    // Someone with write access deletes the marker.
    for m in files_ending(&w.replica(1), ".mrek") {
        std::fs::remove_file(m).unwrap();
    }
    // No halt: the header names a higher epoch but only a verified marker
    // stops a device. The finding is dismissible.
    let found = findings(&mut w, 1);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].2, 2);
    assert!(!w.devs[1].s().rekey_pending().unwrap());
    // The genuine file still recovers.
    adopt(&mut w, 1, &new_sk).unwrap();
    w.converge(0);
    assert_eq!(epoch(&w, 1), 2);
    assert_eq!(
        password_of(&w, 1, vault, item),
        password_of(&w, 0, vault, item)
    );
}

#[test]
fn a_dismissed_unverifiable_file_never_reports_again() {
    let (mut w, _vault, _item) = world(15, 2);
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    w.cloud.settle().unwrap();
    for m in files_ending(&w.replica(1), ".mrek") {
        std::fs::remove_file(m).unwrap();
    }
    let found = findings(&mut w, 1);
    assert_eq!(found.len(), 1);
    let World { devs, .. } = &mut w;
    let dev = &mut devs[1];
    dev.engine
        .dismiss_account_file(&mut dev.access, &found[0].0, &found[0].1)
        .unwrap();
    assert_eq!(findings(&mut w, 1).len(), 0);
}

#[test]
fn folder_concurrent_rekeys_raise_conflict_and_resolve_by_rekeying_again() {
    // No vault: a device that joined a folder cannot re-key a vault another
    // device created (the vault does not know its owner), see the T4 report.
    let (mut w, _vault, _item) = world_with(16, 2, false);
    let (sk0, sk1) = (
        SecretKey::generate().unwrap(),
        SecretKey::generate().unwrap(),
    );
    let original = copy_key(&w.secret_key);
    // Both devices re-key from epoch 1 without seeing each other.
    rekey_as(
        &mut w,
        0,
        (PASSWORD, &original),
        ("password zero zero", &sk0),
    )
    .unwrap();
    rekey_as(&mut w, 1, (PASSWORD, &original), ("password one one", &sk1)).unwrap();
    finish_job(&mut w, 0);
    finish_job(&mut w, 1);
    w.cloud.settle().unwrap();
    // Each now sees an account file of its own epoch that it cannot verify:
    // a finding, no halt, and no adoption at an equal epoch.
    let f0 = findings(&mut w, 0);
    assert_eq!(f0.len(), 1, "{f0:?}");
    assert_eq!(f0[0].2, 2);
    assert!(!w.devs[0].s().rekey_pending().unwrap());
    assert!(matches!(
        adopt_as(&mut w, 0, ("password one one", &sk1), &sk0),
        Err(RekeyError::NoMatchingAccountFile)
    ));
    // The way out is a third epoch from a device that holds the current AK.
    let sk2 = SecretKey::generate().unwrap();
    rekey_as(
        &mut w,
        1,
        ("password one one", &sk1),
        ("password two two", &sk2),
    )
    .unwrap();
    finish_job(&mut w, 1);
    w.cloud.settle().unwrap();
    adopt_as(&mut w, 0, ("password two two", &sk2), &sk0).unwrap();
    w.converge(0);
    for d in 0..2 {
        assert_eq!(epoch(&w, d), 3, "device {d}");
    }
}

/// Random schedules (spec §7.5, `chaos_rekey_random`): edits on three devices,
/// flaky delivery, partial runs, one re-key by the device that owns the vault
/// at a random point (its job finished late, as after a crash), stale devices
/// that stop at the marker and adopt. Invariants: every device ends on one
/// epoch and one key generation, shows the same value for the shared item
/// (one that somebody wrote), and an item each device adds after the dust
/// settled reaches every device.
#[expect(clippy::too_many_lines, reason = "one schedule and its invariants")]
fn random_run(seed: u64) {
    use mosaic_sync::testutil::{SimConfig, SimRng};
    use std::collections::BTreeSet;

    let (mut w, vault, item) = world(200 + seed, 3);
    let mut rng = SimRng::new(seed ^ 0x7e4e);
    let new_sk = SecretKey::generate().unwrap();
    let mut written: BTreeSet<String> = BTreeSet::from(["pw".to_owned()]);
    let steps = 14;
    let rekey_step = 2 + rng.below(8);
    let finish_step = rekey_step + 1 + rng.below(4);
    let mut rekeyed = false;
    let mut finished = false;
    w.cloud.set_config(SimConfig::FLAKY);
    for step in 0..steps {
        if step == rekey_step {
            // A partial delivery first: some peers' files are not here yet.
            w.cloud.round().unwrap();
            rekey(&mut w, 0, &new_sk).unwrap();
            rekeyed = true;
            continue;
        }
        if rekeyed && !finished && step == finish_step {
            finish_job(&mut w, 0);
            finished = true;
            continue;
        }
        let d = rng.below(3);
        match rng.below(4) {
            0 | 1 => {
                // Edits of a device that has stopped at the marker stay local
                // until it adopts; a device that does not know yet writes
                // under the old epoch, which nobody at the new one accepts
                // (it re-seals after adopting).
                if !w.devs[d].s().rekey_pending().unwrap() {
                    let value = format!("seed {seed} step {step} device {d}");
                    set_password(&mut w, d, vault, item, &value);
                    written.insert(value);
                }
            }
            2 => {
                let _ = w.try_sync(d);
            }
            _ => {
                w.cloud.round().unwrap();
            }
        }
    }
    if !rekeyed {
        rekey(&mut w, 0, &new_sk).unwrap();
    }
    if !finished {
        finish_job(&mut w, 0);
    }
    // The dust settles: delivery is calm, stale devices stop and adopt.
    w.cloud.set_config(SimConfig::CALM);
    w.cloud.settle().unwrap();
    for d in 1..3 {
        for _ in 0..3 {
            match w.try_sync(d) {
                Ok(_) => {}
                Err(SyncError::AccountRekeyed) => {
                    adopt(&mut w, d, &new_sk).unwrap();
                }
                Err(e) => panic!("device {d}: {e:?}"),
            }
            w.cloud.settle().unwrap();
        }
    }
    w.converge(0);
    w.converge(0);
    let gen0 = w.devs[0].s().vaults()[0].key_generation;
    let shown = password_of(&w, 0, vault, item).unwrap();
    assert!(
        written.contains(&shown),
        "seed {seed}: {shown:?} was never written"
    );
    for d in 0..3 {
        assert_eq!(epoch(&w, d), 2, "seed {seed} device {d}");
        assert_eq!(
            w.devs[d].s().vaults()[0].key_generation,
            gen0,
            "seed {seed} device {d}"
        );
        assert_eq!(
            password_of(&w, d, vault, item),
            Some(shown.clone()),
            "seed {seed} device {d}"
        );
        assert!(!w.devs[d].s().rekey_pending().unwrap());
    }
    // Nothing is lost from now on: each device adds an item, all see all.
    let added: Vec<_> = (0..3)
        .map(|d| add_login(&mut w, d, vault, &format!("after {seed} {d}")))
        .collect();
    w.converge(0);
    for d in 0..3 {
        for id in &added {
            assert!(
                w.devs[d].s().item(vault, *id).is_ok(),
                "seed {seed} device {d}"
            );
        }
    }
}

#[test]
fn chaos_rekey_random() {
    for seed in 1..=8 {
        random_run(seed);
    }
}

// ---- review round (ADR-0063): jumps, thief markers, paging, joins, holds ----------------

/// What a thief of an old device (its profile, so the old keys) can write.
mod thief {
    use std::fs;
    use std::path::PathBuf;

    use mosaic_core::{AccountSalt, Argon2Profile, InstallId, WrappedKey};
    use mosaic_sync::wire::account_file::AccountFile;
    use mosaic_sync::wire::folder_names::{account_name, rekey_marker_name};
    use mosaic_sync::wire::rekey_marker::RekeyMarker;
    use mosaic_sync::wire::{SessionCrypto, SyncCrypto};

    use crate::common::{Crypto, World, account_dir};

    pub struct Base {
        pub wrapped_ak: WrappedKey,
        pub profile: Argon2Profile,
    }

    pub fn base(w: &World) -> Base {
        let boot = w.devs[0].s().account_bootstrap().unwrap();
        Base {
            wrapped_ak: boot.wrapped_ak.clone(),
            profile: boot.params.profile,
        }
    }

    /// The old keys of device `d` (call before it adopts).
    pub fn keys(w: &World, d: usize) -> Crypto {
        SessionCrypto::new(w.devs[d].access.clone()).unwrap()
    }

    pub fn fake(n: u8) -> InstallId {
        let mut b = [1u8; 16];
        b[0] = 0;
        b[1] = n;
        InstallId::from_bytes(b).unwrap()
    }

    fn dir(w: &World, r: usize) -> PathBuf {
        account_dir(w, r).join("account")
    }

    pub fn plant_file(
        w: &World,
        crypto: &impl SyncCrypto,
        b: &Base,
        epoch: u32,
        version: u64,
        device: InstallId,
    ) {
        let file = AccountFile {
            account_id: w.account,
            account_version: version,
            ak_epoch: epoch,
            device_id: device,
            salt: AccountSalt::generate().unwrap(),
            profile: b.profile,
            wrapped_ak: b.wrapped_ak.clone(),
        };
        let bytes = file.encode(crypto).unwrap();
        for r in 0..w.devs.len() {
            fs::write(
                dir(w, r).join(account_name(version, device.as_bytes())),
                &bytes,
            )
            .unwrap();
        }
    }

    pub fn plant_marker(
        w: &World,
        crypto: &impl SyncCrypto,
        from: u32,
        to: u32,
        version: u64,
        device: InstallId,
    ) {
        let m = RekeyMarker {
            account_id: w.account,
            from_epoch: from,
            to_epoch: to,
            to_account_version: version,
            device_id: device,
            created_at_ms: 0,
        };
        let bytes = m.encode(crypto).unwrap();
        for r in 0..w.devs.len() {
            fs::write(
                dir(w, r).join(rekey_marker_name(from, device.as_bytes())),
                &bytes,
            )
            .unwrap();
        }
    }
}

#[test]
fn folder_planted_high_epoch_header_does_not_strand_devices() {
    let (mut w, _vault, _item) = world(21, 3);
    let t = thief::keys(&w, 2);
    let b = thief::base(&w);
    thief::plant_file(&w, &t, &b, 6, 50, thief::fake(1));
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    assert_eq!(epoch(&w, 0), 2, "a planted header does not move the target");
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();
    thief::plant_file(&w, &t, &b, 6, 51, thief::fake(2));
    for d in 1..3 {
        assert!(matches!(w.try_sync(d), Err(SyncError::AccountRekeyed)));
        adopt(&mut w, d, &new_sk).unwrap();
        assert_eq!(epoch(&w, d), 2);
    }
}

#[test]
fn folder_two_rekeys_in_a_row_stale_devices_still_adopt() {
    let (mut w, _v, _i) = world(22, 3);
    let (sk1, sk2) = (
        SecretKey::generate().unwrap(),
        SecretKey::generate().unwrap(),
    );
    rekey(&mut w, 0, &sk1).unwrap();
    finish_job(&mut w, 0);
    rekey_as(
        &mut w,
        0,
        (NEW_PASSWORD, &sk1),
        ("second password here", &sk2),
    )
    .unwrap();
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();
    // Device 1 is two re-keys behind: it adopts the newest in one jump.
    let current = copy_key(&w.secret_key);
    adopt_as(&mut w, 1, ("second password here", &sk2), &current).unwrap();
    assert_eq!(epoch(&w, 1), 3);
    // Device 2 may follow them one by one or jump as well.
    adopt_as(&mut w, 2, ("second password here", &sk2), &current).unwrap();
    assert_eq!(epoch(&w, 2), 3);
}

#[test]
fn folder_forged_marker_is_dismissible_after_the_user_acts() {
    let (mut w, _v, _i) = world(23, 2);
    let t = thief::keys(&w, 1);
    thief::plant_marker(&w, &t, 1, 2, 9, thief::fake(3));
    assert!(matches!(w.try_sync(0), Err(SyncError::AccountRekeyed)));
    assert!(w.devs[0].s().rekey_pending().unwrap());
    let typed = SecretKey::generate().unwrap();
    assert!(matches!(
        adopt(&mut w, 0, &typed),
        Err(RekeyError::NoMatchingAccountFile)
    ));
    let World { devs, .. } = &mut w;
    let dev = &mut devs[0];
    let markers = dev.engine.pending_rekey_markers(&mut dev.access).unwrap();
    assert_eq!(markers.len(), 1);
    let still = dev
        .engine
        .dismiss_rekey_marker(&mut dev.access, &markers[0].name, &markers[0].sha256)
        .unwrap();
    assert!(!still);
    assert!(!w.devs[0].s().rekey_pending().unwrap());
    w.sync(0);
}

#[test]
fn folder_adoption_pages_past_planted_files_and_markers() {
    let (mut w, _v, _i) = world(24, 3);
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();
    let t = thief::keys(&w, 2);
    let b = thief::base(&w);
    // Eight forged markers with low IDs, each naming a planted file with a
    // salt of its own: the genuine file is ninth.
    for n in 0..8u8 {
        let dev = thief::fake(10 + n);
        thief::plant_marker(&w, &t, 1, 2, 100 + u64::from(n), dev);
        thief::plant_file(&w, &t, &b, 2, 100 + u64::from(n), dev);
    }
    assert!(matches!(w.try_sync(1), Err(SyncError::AccountRekeyed)));
    let current = copy_key(&w.secret_key);
    let password = pw(NEW_PASSWORD);
    let World { devs, clock, .. } = &mut w;
    let dev = &mut devs[1];
    let next =
        match dev
            .engine
            .adopt_folder_rekey(&mut dev.access, clock, &password, &new_sk, &current)
        {
            Err(RekeyError::MoreAccountFiles { next_skip }) => next_skip,
            other => panic!("{other:?}"),
        };
    dev.engine
        .adopt_folder_rekey_page(&mut dev.access, clock, &password, &new_sk, &current, next)
        .unwrap();
    assert_eq!(epoch(&w, 1), 2);
}

#[test]
fn folder_join_after_a_rekey_starts_at_the_folders_epoch() {
    let mut w = World::new(25, 2);
    w.sync(0);
    w.cloud.settle().unwrap();
    let config = |w: &World, d: usize| SyncConfig {
        kind: SyncKind::Folder,
        server_url: None,
        email: None,
        folder_path: Some(w.replica(d)),
        device_name: format!("device {d}"),
    };
    let c0 = config(&w, 0);
    w.devs[0].s_mut().set_sync_config(Some(&c0)).unwrap();
    let sk1 = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &sk1).unwrap();
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();
    w.secret_key = copy_key(&sk1);
    let d = w.try_join(NEW_PASSWORD).map_err(|f| f.error).unwrap();
    assert_eq!(epoch(&w, d), 2);
    let c1 = config(&w, d);
    w.devs[d].s_mut().set_sync_config(Some(&c1)).unwrap();
    // Its own re-key targets 3 and the first device adopts it.
    let sk2 = SecretKey::generate().unwrap();
    rekey_as(
        &mut w,
        d,
        (NEW_PASSWORD, &sk1),
        ("third password here", &sk2),
    )
    .unwrap();
    assert_eq!(epoch(&w, d), 3);
    finish_job(&mut w, d);
    w.cloud.settle().unwrap();
    adopt_as(&mut w, 0, ("third password here", &sk2), &copy_key(&sk1)).unwrap();
    assert_eq!(epoch(&w, 0), 3);
}

#[test]
fn folder_adopted_vault_is_held_and_gets_its_snapshot_later() {
    let (mut w, vault, _item) = world(26, 2);
    let new_sk = SecretKey::generate().unwrap();
    rekey(&mut w, 0, &new_sk).unwrap();
    finish_job(&mut w, 0);
    w.cloud.settle().unwrap();
    adopt(&mut w, 1, &new_sk).unwrap();
    assert!(
        w.devs[1].s().rekey_held(vault).unwrap(),
        "held after adopting"
    );
    w.converge(0);
    assert!(
        !w.devs[1].s().rekey_held(vault).unwrap(),
        "the next key arrived"
    );
    assert!(
        w.devs[1]
            .s()
            .folder_rekey_floors()
            .unwrap()
            .iter()
            .all(|f| f.snapshot_done),
        "the snapshot set was written once the hold ended"
    );
}
