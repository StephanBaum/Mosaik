//! Regression tests of the crypto review of M6 T2 (C-H1 and C-M1 to C-L6 of
//! the fix list): each one fails on the engine as it was before the fixes.

use std::fs;

use mosaic_core::{
    AccountKey, AttachmentId, InstallId, ItemId, KeyGeneration, SecretKey, VaultId, VaultKey,
    VaultMetaCiphertext,
};
use mosaic_sync::engine::{EngineConfig, RotationOutcome, SyncEngine, SyncError};
use mosaic_sync::testutil::FixedJitter;
use mosaic_sync::transport::{Cursor, MaintainContext, Transport, TransportError, TransportKind};
use mosaic_sync::wire::checkpoint::Checkpoint;
use mosaic_sync::wire::folder_names::{account_name, blob_name, dir_name, segment_name};
use mosaic_sync::wire::segment::SegmentObject;
use mosaic_vault::storage::MarkerStamp;

use super::compaction::{BIG, big_vault, edit_until_compacted, rounds};
use super::util::{
    DAY_MS, Faulty, HOUR_MS, add_login, password_of, relock, set_password, shows, snapshot_parts,
    warns_missing,
};
use crate::common::{
    PASSWORD, World, account_dir, device_dir, files_ending, log_dir, open_transport, pw, segments,
    tree,
};

/// Three devices at rest sharing one vault with one item.
fn three_devices(seed: u64) -> (World, VaultId, ItemId) {
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

fn blob_path(w: &World, replica: usize, vault: VaultId, id: AttachmentId) -> std::path::PathBuf {
    account_dir(w, replica)
        .join("v")
        .join(dir_name(vault.as_bytes()))
        .join("blobs")
        .join(blob_name(id.as_bytes()))
}

fn same_secret_key(w: &World) -> SecretKey {
    let text = w.secret_key.to_display().unwrap();
    SecretKey::parse(text.expose_secret()).unwrap()
}

// ---------------------------------------------------------------------------
// C-M1: the transport holds no key
// ---------------------------------------------------------------------------

/// The folder transport gets its keys from the shared session for every
/// operation: once the session locks, the transport fails closed (and does
/// not remember a file as bad because the session could not verify it); the
/// engine reports `Locked`; after the unlock everything works again.
#[test]
fn transport_fails_closed_once_the_session_locks() {
    let (mut w, vault, _item) = three_devices(501);
    let mut t = open_transport(&w.devs[1].access, &w.replica(1), &w.clock, "probe");
    let first = t.pull(vault, &Cursor::default(), 100).unwrap();
    assert_ne!(first.units.len(), 0);

    let session = w.devs[1].access.take().unwrap();
    assert!(matches!(
        t.pull(vault, &Cursor::default(), 100),
        Err(TransportError::Locked)
    ));
    let key = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let meta = VaultMetaCiphertext::seal(&key, &vault, b"{}").unwrap();
    assert!(matches!(
        t.push_meta(vault, &meta, None),
        Err(TransportError::Locked)
    ));
    assert!(matches!(
        t.own_logs_rolled_back(&[vault]),
        Err(TransportError::Locked)
    ));
    let r = w.try_sync(1);
    assert!(matches!(r, Err(SyncError::Locked)), "{r:?}");

    // Unlocked again: the files are read, none was written off.
    w.devs[1].access.set(session);
    let again = t.pull(vault, &Cursor::default(), 100).unwrap();
    assert_eq!(again.units.len(), first.units.len());
    assert!(t.list_vaults().unwrap().iter().any(|v| v.vault_id == vault));
    w.sync(1);
}

// ---------------------------------------------------------------------------
// C-M2: planted names in the own log
// ---------------------------------------------------------------------------

fn plant_and_write(seed: u64, name: &str) {
    let (mut w, vault, item) = three_devices(seed);
    let a = install(&w, 0);
    fs::write(log_dir(&w, 1, 0, vault).join(name), b"junk").unwrap();
    w.cloud.settle().unwrap();
    set_password(&mut w, 0, vault, item, "after the junk");
    w.try_sync(0).unwrap();
    w.converge(0);
    w.advance(25 * HOUR_MS);
    for report in w.sync_all() {
        assert!(!warns_missing(&report, a), "{report:?}");
    }
    for d in 0..3 {
        assert!(shows(&w, d, vault, item, "after the junk"), "device {d}");
    }
}

#[test]
fn junk_near_the_top_of_the_sequence_range_changes_nothing() {
    plant_and_write(511, "fffffffffffffff0.mseg");
}

#[test]
fn junk_at_the_very_top_of_the_sequence_range_changes_nothing() {
    plant_and_write(512, "ffffffffffffffff.mseg");
}

/// Directories (with content) on the next names, which this device never
/// wrote, are removed at once and do not stop the writer; no peer keeps a
/// warning.
#[test]
fn squatters_on_the_next_names_are_removed_and_do_not_stop_the_writer() {
    let (mut w, vault, item) = three_devices(513);
    let a = install(&w, 0);
    let dir = log_dir(&w, 0, 0, vault);
    let reserved = w.devs[0].s().own_seq(vault).unwrap().reserved;
    let squatters: Vec<_> = (1..=16)
        .map(|k| dir.join(segment_name(reserved + k)))
        .collect();
    for sq in &squatters {
        fs::create_dir_all(sq.join("inside")).unwrap();
        fs::write(sq.join("inside").join("file"), b"x").unwrap();
    }
    set_password(&mut w, 0, vault, item, "past the squatter");
    w.try_sync(0).unwrap();
    assert!(squatters.iter().all(|sq| !sq.is_dir()), "removed");
    w.converge(0);
    w.advance(25 * HOUR_MS);
    for report in w.sync_all() {
        assert!(!warns_missing(&report, a), "{report:?}");
    }
    for d in 0..3 {
        assert!(shows(&w, d, vault, item, "past the squatter"), "device {d}");
    }
}

/// Files in the own log that never verify are removed after the temp age;
/// verified segments are never touched.
#[test]
fn unverifiable_files_in_the_own_log_are_removed_after_a_day() {
    let (mut w, vault, item) = three_devices(514);
    let dir = log_dir(&w, 0, 0, vault);
    let real = segments(&dir).len();
    let junk = dir.join("ffffffffffffff00.mseg");
    fs::write(&junk, b"junk").unwrap();
    w.sync(0);
    assert!(junk.exists(), "too young");
    w.advance(25 * HOUR_MS);
    set_password(&mut w, 0, vault, item, "later");
    w.sync(0);
    assert!(!junk.exists(), "old junk is gone");
    assert!(segments(&dir).len() >= real, "real segments stay");
    w.converge(0);
    assert!(shows(&w, 2, vault, item, "later"));
}

// ---------------------------------------------------------------------------
// C-M3: halts persist
// ---------------------------------------------------------------------------

/// A duplicated identity halts sync; the halt survives a rebuilt engine and
/// a lock and unlock of the session, and only the explicit acknowledgement
/// clears it.
#[test]
fn a_halt_survives_a_new_engine_and_a_new_session() {
    let (mut w, _vault, _item) = three_devices(521);
    w.sync(0);
    let counter = w.devs[0].s().checkpoint_counter().unwrap();
    let own = install(&w, 0);
    let forged = Checkpoint {
        device_id: own,
        counter: counter + 5,
        issued_at_ms: 1,
        entries: Vec::new(),
    }
    .encode(&*w.devs[0].s())
    .unwrap();
    let dir = device_dir(&w, 0, 0);
    let copy = dir.join("head (conflicted copy).mck");
    fs::write(&copy, forged).unwrap();
    assert!(matches!(w.try_sync(0), Err(SyncError::IdentityDuplicated)));
    // A new engine and transport.
    relock(&mut w, 0);
    assert!(matches!(w.try_sync(0), Err(SyncError::IdentityDuplicated)));
    // A new session over the same store.
    let session = w.devs[0].access.take().unwrap();
    let session = session.lock().unlock(&pw(PASSWORD), &w.secret_key).unwrap();
    w.devs[0].access.set(session);
    relock(&mut w, 0);
    assert!(matches!(w.try_sync(0), Err(SyncError::IdentityDuplicated)));
    fs::remove_file(&copy).unwrap();
    {
        let dev = &mut w.devs[0];
        dev.engine.clear_halt(&mut dev.access).unwrap();
    }
    w.sync(0);
}

// ---------------------------------------------------------------------------
// C-M4: a snapshot set completes from the folder, not from memory
// ---------------------------------------------------------------------------

/// A reader gets the later part of a snapshot set first, behind a key it
/// does not hold yet (its unit is deferred); the first part, with the key,
/// follows. The set completes, the gap closes, nothing is reported.
#[test]
fn a_snapshot_part_deferred_behind_its_key_completes_later() {
    let (mut w, vault, items) = big_vault(531);
    let a = install(&w, 0);
    let item = items[BIG - 1];
    // A rotates: the snapshot's first part carries the new key, the later
    // parts hold records sealed under it.
    w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    let last = edit_until_compacted(&mut w, vault, item);
    let parts = snapshot_parts(&w, 0, 0, vault);
    assert!(parts.len() >= 2);
    w.cloud
        .withhold(1, &super::util::segment_needle(&w, 0, vault, parts[0].0), 6);
    rounds(&mut w, 3);
    w.sync(1);
    assert_ne!(
        password_of(&w, 1, vault, item).as_deref(),
        Some(last.as_str()),
        "the records wait for their key"
    );
    for _ in 0..6 {
        w.cloud.round().unwrap();
    }
    w.sync(1);
    assert_eq!(
        password_of(&w, 1, vault, item).as_deref(),
        Some(last.as_str())
    );
    w.advance(25 * HOUR_MS);
    w.sync(0);
    for _ in 0..3 {
        w.cloud.round().unwrap();
    }
    let r = w.sync(1);
    assert!(!warns_missing(&r, a), "{r:?}");
}

// ---------------------------------------------------------------------------
// C-M5: attachment blobs are authenticated
// ---------------------------------------------------------------------------

/// Somebody with write access replaces a blob with a file of valid layout
/// that does not authenticate. A device that fetches it never stores it
/// (the attachment stays missing and the run reports it); the holder
/// replaces it with its own copy, and the fetcher gets the real file.
#[test]
fn a_planted_blob_is_never_stored_and_the_holder_heals_it() {
    let mut w = World::new(541, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    let blob = blob_path(&w, 0, vault, id);
    let real = fs::read(&blob).unwrap();
    let mut planted = real.clone();
    let last = planted.len() - 1;
    planted[last] ^= 1;
    fs::write(&blob, &planted).unwrap();
    relock(&mut w, 0);
    w.cloud.settle().unwrap();

    let b = w.join();
    let r = w.sync(b);
    assert_eq!(r.attachments_rejected, 1, "{r:?}");
    assert_eq!(
        w.devs[b].s().attachment_file_state(id).unwrap(),
        mosaic_vault::session::AttachmentFileState::Missing
    );

    // The holder checks the file of its fresh engine against its own.
    let r = w.sync(0);
    assert_eq!(r.attachments_healed, 1, "{r:?}");
    assert_eq!(fs::read(&blob).unwrap(), real);
    assert_eq!(files_ending(&w.replica(0), ".mtmp").len(), 0);
    w.cloud.settle().unwrap();
    let r = w.sync(b);
    assert_eq!(r.attachments_down, 1, "{r:?}");
    assert_eq!(
        w.devs[b]
            .s()
            .attachment(vault, item, id)
            .unwrap()
            .expose_secret(),
        b"attachment body"
    );
}

// ---------------------------------------------------------------------------
// C-L1: joining with planted account files
// ---------------------------------------------------------------------------

/// The bytes of the real account file of the world's account.
fn real_account_file(w: &World) -> (std::path::PathBuf, Vec<u8>) {
    let dir = account_dir(w, 0).join("account");
    let file = files_ending(&dir, ".macct")
        .into_iter()
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("0000000000000001-"))
        })
        .unwrap();
    let bytes = fs::read(&file).unwrap();
    (file, bytes)
}

fn plant_account_file(w: &World, version: u64, mutate: impl FnOnce(&mut [u8])) {
    let (_, mut bytes) = real_account_file(w);
    bytes[21..29].copy_from_slice(&version.to_be_bytes());
    mutate(&mut bytes);
    let device: [u8; 16] = bytes[29..45].try_into().unwrap();
    let dir = account_dir(w, 0).join("account");
    fs::write(dir.join(account_name(version, &device)), bytes).unwrap();
}

/// A higher version whose wrapped key does not open, and a copy of the real
/// file with a bumped version that opens but has no valid MAC: neither locks
/// a device out, and neither counts as the highest version.
#[test]
fn join_skips_planted_account_files() {
    let mut w = World::new(551, 2);
    let vault = w.new_vault(0, "Shared");
    w.sync(0);
    plant_account_file(&w, 998, |b| b[45] ^= 1);
    plant_account_file(&w, 999, |_| {});
    w.cloud.settle().unwrap();
    assert_eq!(
        w.devs[0]
            .engine
            .transport_mut()
            .highest_account_version()
            .unwrap(),
        1,
        "planted files do not count"
    );
    assert_eq!(
        w.devs[0]
            .engine
            .transport_mut()
            .next_account_version()
            .unwrap(),
        2
    );
    let b = w.join();
    w.converge(1);
    assert!(w.devs[b].s().vaults().iter().any(|v| v.vault_id == vault));
}

/// Only a candidate that opens but fails its MAC: `FolderDamaged`, and the
/// session is handed back untouched. Only candidates that do not open:
/// `WrongCredentials`.
#[test]
fn join_reports_damage_only_for_files_that_unlocked() {
    let mut w = World::new(552, 2);
    w.new_vault(0, "Shared");
    w.sync(0);
    let (real, _) = real_account_file(&w);
    plant_account_file(&w, 999, |_| {});
    fs::remove_file(&real).unwrap();
    w.cloud.settle().unwrap();
    let failure = w.try_join(PASSWORD).unwrap_err();
    assert!(
        matches!(failure.error, mosaic_sync::folder::JoinError::FolderDamaged),
        "{:?}",
        failure.error
    );
    assert!(failure.session.is_some());

    let mut w = World::new(553, 2);
    w.new_vault(0, "Shared");
    w.sync(0);
    let (real, _) = real_account_file(&w);
    plant_account_file(&w, 999, |b| b[45] ^= 1);
    fs::remove_file(&real).unwrap();
    w.cloud.settle().unwrap();
    let failure = w.try_join(PASSWORD).unwrap_err();
    assert!(
        matches!(
            failure.error,
            mosaic_sync::folder::JoinError::WrongCredentials
        ),
        "{:?}",
        failure.error
    );
}

// ---------------------------------------------------------------------------
// C-L2: only verified devices are listed
// ---------------------------------------------------------------------------

#[test]
fn a_planted_device_directory_is_not_a_device() {
    let (mut w, _vault, _item) = three_devices(561);
    let phantom = account_dir(&w, 1).join("d").join(dir_name(&[7u8; 16]));
    fs::create_dir_all(&phantom).unwrap();
    fs::write(phantom.join("head.mck"), b"not a checkpoint").unwrap();
    fs::write(phantom.join("info.mdev"), vec![1u8; 573]).unwrap();
    let devices = w.devs[1].engine.transport_mut().devices().unwrap();
    assert_eq!(devices.len(), 3, "{devices:?}");
    assert!(devices.iter().all(|d| d.name.is_some()));
}

// ---------------------------------------------------------------------------
// C-L3: every key is ranked by its lowest writer
// ---------------------------------------------------------------------------

fn low_device(n: u8) -> InstallId {
    let mut b = [0u8; 16];
    b[15] = n;
    InstallId::from_bytes(b).unwrap()
}

/// X generated a key; a lower device (2) carried the same bytes in its own
/// log. A rival key written by a device (3) between the two is outranked by
/// the lower carrier, so X keeps its key. A rival written by a device below
/// the carrier wins, and X rotates.
#[test]
fn the_winner_is_decided_by_the_lowest_writer_of_each_key() {
    let (mut w, vault, _item) = three_devices(571);
    let account = w.account;
    let g = w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    let held = w.devs[0]
        .s()
        .store()
        .vault_keys(&vault)
        .unwrap()
        .into_iter()
        .find(|(k, _)| *k == g)
        .map(|(_, k)| k)
        .unwrap();
    let rival = || {
        VaultKey::generate(g)
            .unwrap()
            .wrap(&AccountKey::generate().unwrap(), &account, &vault)
            .unwrap()
    };
    {
        let mut s = w.devs[0].s_mut();
        s.note_key_observation(vault, g, &held, low_device(2))
            .unwrap();
        s.note_key_observation(vault, g, &rival(), low_device(3))
            .unwrap();
    }
    w.sync(0);
    let current = |w: &World| {
        w.devs[0]
            .s()
            .vaults()
            .iter()
            .find(|v| v.vault_id == vault)
            .unwrap()
            .key_generation
    };
    assert_eq!(current(&w), g, "device 3 ranks below the carrier 2");
    {
        let mut s = w.devs[0].s_mut();
        s.note_key_observation(vault, g, &rival(), low_device(1))
            .unwrap();
    }
    w.sync(0);
    assert!(
        current(&w) > g,
        "a rival from device 1 outranks every carrier"
    );
}

// ---------------------------------------------------------------------------
// C-L4: a stale prepared rotation is dropped
// ---------------------------------------------------------------------------

/// The session relocks between the backend taking the new key and the
/// local commit. The prepared rotation is stale; the engine pulls the key
/// from its own log instead of failing, and everything converges.
#[test]
fn a_rotation_survives_a_relock_between_push_and_commit() {
    let (mut w, vault, item) = three_devices(581);
    let sk = same_secret_key(&w);
    let access = w.devs[0].access.clone();
    let mut faulty = Faulty::new(open_transport(&access, &w.replica(0), &w.clock, "device 0"));
    let hook_access = access.clone();
    faulty.faults.after_key_push = Some(Box::new(move || {
        let session = hook_access.take().unwrap();
        let session = session.lock().unlock(&pw(PASSWORD), &sk).unwrap();
        hook_access.set(session);
    }));
    let mut engine = SyncEngine::with_jitter(
        faulty,
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    let start = w.devs[0]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    let dev = &mut w.devs[0];
    let outcome = engine
        .rotate_vault(&mut dev.access, &w.clock, vault)
        .unwrap();
    assert!(
        matches!(outcome, RotationOutcome::RotatedByAnother(g) if g > start)
            || matches!(outcome, RotationOutcome::Rotated(g) if g > start),
        "{outcome:?}"
    );
    drop(engine);
    set_password(&mut w, 0, vault, item, "after the rotation");
    w.converge(1);
    for d in 0..3 {
        assert!(
            shows(&w, d, vault, item, "after the rotation"),
            "device {d}"
        );
    }
}

// ---------------------------------------------------------------------------
// C-L5: purge gating
// ---------------------------------------------------------------------------

/// A blob that is past retention stays while a key conflict of the vault is
/// open (a record that cannot be read yet may still refer to it), and goes
/// once the conflict is resolved.
#[test]
fn a_blob_is_not_purged_while_a_key_conflict_is_open() {
    let mut w = World::new(591, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.join();
    w.converge(0);
    let blob = blob_path(&w, 0, vault, id);
    assert!(blob.exists());
    w.devs[0]
        .s_mut()
        .remove_attachment(vault, item, id)
        .unwrap();
    w.converge(0);
    // An open conflict at the vault's current generation.
    let g = w.devs[0]
        .s()
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault)
        .unwrap()
        .key_generation;
    let rival = VaultKey::generate(g)
        .unwrap()
        .wrap(&AccountKey::generate().unwrap(), &w.account, &vault)
        .unwrap();
    w.devs[0]
        .s_mut()
        .note_key_observation(vault, g, &rival, InstallId::from_bytes([0xfe; 16]).unwrap())
        .unwrap();
    w.advance(31 * DAY_MS);
    w.sync(0);
    w.sync(1);
    w.sync(0);
    assert!(blob.exists(), "kept while the conflict is open");
    // The conflict is resolved by the generation moving on.
    w.devs[0].s_mut().rotate_vault_key(vault).unwrap();
    w.converge(1);
    w.sync(0);
    w.sync(0);
    assert!(!blob.exists(), "purged once nothing is open");
}

// ---------------------------------------------------------------------------
// C-L6: hostile files
// ---------------------------------------------------------------------------

/// A flood of `*.mck` files in a device's directory costs a bounded number
/// of reads, and the real `head.mck` is still among them.
#[test]
fn a_flood_of_checkpoint_candidates_is_capped() {
    let (mut w, _vault, _item) = three_devices(601);
    let b = install(&w, 1);
    let dir = device_dir(&w, 0, 1);
    let head = fs::read(dir.join("head.mck")).unwrap();
    for i in 0..100 {
        fs::write(dir.join(format!("flood {i}.mck")), vec![0u8; 100]).unwrap();
    }
    let all = w.devs[0].engine.transport_mut().checkpoints().unwrap();
    let of_b: Vec<_> = all.iter().filter(|(d, _)| *d == b).collect();
    assert!(of_b.len() <= 32, "{}", of_b.len());
    assert!(of_b.iter().any(|(_, bytes)| *bytes == head));
}

/// Own conflict copies of `head.mck` are deleted only when the run read them
/// (an unread copy could be the only surviving checkpoint).
#[test]
fn an_unread_conflict_copy_of_head_is_kept() {
    struct Cx(u64);
    impl MaintainContext for Cx {
        fn now_ms(&self) -> u64 {
            self.0
        }
        fn snapshot_objects(&mut self, _: VaultId) -> Result<Vec<SegmentObject>, TransportError> {
            Ok(Vec::new())
        }
        fn attachment_referenced(&mut self, _: AttachmentId) -> Result<bool, TransportError> {
            Ok(false)
        }
        fn caught_up(&self) -> bool {
            false
        }
        fn marker_first_seen(
            &mut self,
            _: VaultId,
            markers: &[MarkerStamp],
            now: u64,
        ) -> Result<Vec<u64>, TransportError> {
            Ok(markers.iter().map(|_| now).collect())
        }
    }
    let (mut w, _vault, _item) = three_devices(602);
    let dir = device_dir(&w, 0, 0);
    let copy = dir.join("head (conflicted copy).mck");
    fs::write(&copy, b"anything").unwrap();
    let now = mosaic_sync::clock::Clock::now_ms(&w.clock);
    let t = w.devs[0].engine.transport_mut();
    t.maintain(&mut Cx(now)).unwrap();
    assert!(copy.exists(), "not read in this run: kept");
    t.checkpoints().unwrap();
    t.maintain(&mut Cx(now)).unwrap();
    assert!(!copy.exists(), "read in this run: removed");
}

/// A device keeps only its newest account file once a verified higher
/// version exists from any device.
#[test]
fn old_own_account_files_go_once_a_higher_version_exists() {
    let mut w = World::new(603, 2);
    w.new_vault(0, "Shared");
    w.sync(0);
    w.cloud.settle().unwrap();
    w.join();
    w.converge(0);
    let boot = w.devs[1].s().account_bootstrap().unwrap();
    w.devs[1]
        .engine
        .transport_mut()
        .write_account_file(&boot.params, &boot.wrapped_ak, 2)
        .unwrap();
    w.cloud.settle().unwrap();
    let dir = account_dir(&w, 0).join("account");
    assert_eq!(files_ending(&dir, ".macct").len(), 2);
    w.sync(0);
    let left = files_ending(&dir, ".macct");
    assert_eq!(left.len(), 1, "{left:?}");
    assert!(
        left[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(&format!("{:016x}-", 2))
    );
}

// ---------------------------------------------------------------------------
// Round 2: join, halts, planted names, holes, heal failures
// ---------------------------------------------------------------------------

/// After a password change, the old password does not join, even while the
/// old account file is present again (or planted again): a file of a higher
/// version that the account key MACs supersedes it.
#[test]
fn a_superseded_password_cannot_join_while_the_old_file_is_present() {
    let mut w = World::new(611, 2);
    w.new_vault(0, "Shared");
    w.sync(0);
    w.cloud.settle().unwrap();
    let (_, v1) = real_account_file(&w);
    let new_password = "a completely different passphrase";
    {
        let dev = &mut w.devs[0];
        let prepared = dev
            .access
            .session_mut()
            .unwrap()
            .prepare_master_password_change(&pw(PASSWORD), &pw(new_password), &w.secret_key)
            .unwrap();
        dev.engine
            .transport_mut()
            .write_account_file(prepared.new_params(), prepared.new_wrapped_ak(), 2)
            .unwrap();
        dev.access
            .session_mut()
            .unwrap()
            .commit_master_password_change(prepared)
            .unwrap();
    }
    // The old file comes back.
    let device: [u8; 16] = v1[29..45].try_into().unwrap();
    let dir = account_dir(&w, 0).join("account");
    fs::write(dir.join(account_name(1, &device)), &v1).unwrap();
    w.cloud.settle().unwrap();
    let failure = w.try_join(PASSWORD).unwrap_err();
    assert!(
        matches!(
            failure.error,
            mosaic_sync::folder::JoinError::WrongCredentials
        ),
        "{:?}",
        failure.error
    );
    assert!(failure.session.is_some());
    drop(failure);
    w.try_join(new_password).unwrap();
}

/// Under a duplicated identity a halted engine does not rotate and writes
/// nothing.
#[test]
fn a_halted_engine_neither_rotates_nor_writes() {
    let (mut w, vault, _item) = three_devices(612);
    w.sync(0);
    let counter = w.devs[0].s().checkpoint_counter().unwrap();
    let own = install(&w, 0);
    let forged = Checkpoint {
        device_id: own,
        counter: counter + 5,
        issued_at_ms: 1,
        entries: Vec::new(),
    }
    .encode(&*w.devs[0].s())
    .unwrap();
    fs::write(
        device_dir(&w, 0, 0).join("head (conflicted copy).mck"),
        forged,
    )
    .unwrap();
    assert!(matches!(w.try_sync(0), Err(SyncError::IdentityDuplicated)));
    let before = tree(&w.replica(0));
    let dev = &mut w.devs[0];
    let r = dev.engine.rotate_vault(&mut dev.access, &w.clock, vault);
    assert!(matches!(r, Err(SyncError::IdentityDuplicated)), "{r:?}");
    assert_eq!(tree(&w.replica(0)), before, "nothing was written");
}

/// The very first run under a duplicated identity writes nothing: the checks
/// that can halt it come before the vault is created on the backend.
#[test]
fn the_first_run_under_a_duplicated_identity_writes_nothing() {
    let mut w = World::new(613, 1);
    let vault = w.new_vault(0, "Fresh");
    add_login(&mut w, 0, vault, "unsynced");
    let own = install(&w, 0);
    let forged = Checkpoint {
        device_id: own,
        counter: 5,
        issued_at_ms: 1,
        entries: Vec::new(),
    }
    .encode(&*w.devs[0].s())
    .unwrap();
    fs::write(
        device_dir(&w, 0, 0).join("head (conflicted copy).mck"),
        forged,
    )
    .unwrap();
    let before = tree(&w.replica(0));
    assert!(matches!(w.try_sync(0), Err(SyncError::IdentityDuplicated)));
    assert_eq!(tree(&w.replica(0)), before, "nothing was written");
}

/// A rollback with junk planted on top of the newest names still halts: a
/// file that fails its MAC does not count as present.
#[test]
fn rollback_with_junk_planted_on_top_still_halts() {
    let (mut w, vault, item) = three_devices(614);
    for i in 0..4 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
    }
    let dir = log_dir(&w, 0, 0, vault);
    let segs = segments(&dir);
    for p in &segs[segs.len() - 3..] {
        fs::remove_file(p).unwrap();
    }
    fs::write(dir.join("ffffffffffffffff.mseg"), b"junk").unwrap();
    relock(&mut w, 0);
    set_password(&mut w, 0, vault, item, "after");
    let r = w.try_sync(0);
    assert!(matches!(r, Err(SyncError::RollbackDetected)), "{r:?}");
}

/// Names above the reserved numbers were never written by this device:
/// non-empty directories, junk and junk with a modification time in the
/// future are removed at once and stop neither pushes nor a multi-part
/// compaction.
#[test]
fn planted_names_above_the_reserved_numbers_stop_nothing() {
    let (mut w, vault, item) = three_devices(615);
    let dir = log_dir(&w, 0, 0, vault);
    // 16 non-empty directories on the next 16 names.
    let reserved = w.devs[0].s().own_seq(vault).unwrap().reserved;
    for k in 1..=16 {
        let d = dir.join(segment_name(reserved + k));
        fs::create_dir_all(d.join("x")).unwrap();
        fs::write(d.join("x").join("f"), b"x").unwrap();
    }
    set_password(&mut w, 0, vault, item, "one");
    w.try_sync(0).unwrap();
    // Junk with a future time.
    let reserved = w.devs[0].s().own_seq(vault).unwrap().reserved;
    let junk = dir.join(segment_name(reserved + 1));
    fs::write(&junk, b"junk").unwrap();
    let future = std::time::SystemTime::now() + std::time::Duration::from_hours(240);
    fs::OpenOptions::new()
        .write(true)
        .open(&junk)
        .unwrap()
        .set_modified(future)
        .unwrap();
    set_password(&mut w, 0, vault, item, "two");
    w.try_sync(0).unwrap();
    assert_ne!(
        fs::read(&junk).unwrap(),
        b"junk",
        "replaced by a real segment"
    );
    w.converge(0);
    for d in 0..3 {
        assert!(shows(&w, d, vault, item, "two"), "device {d}");
    }

    // A multi-part compaction with junk in the middle of its range.
    let (mut w, vault, items) = big_vault(616);
    let item = items[BIG - 1];
    let dir = log_dir(&w, 0, 0, vault);
    for i in 0..90 {
        let reserved = w.devs[0].s().own_seq(vault).unwrap().reserved;
        for k in [2, 3] {
            fs::write(dir.join(segment_name(reserved + k)), b"junk").unwrap();
        }
        set_password(&mut w, 0, vault, item, &format!("edit {i}"));
        w.try_sync(0).unwrap();
        if !snapshot_parts(&w, 0, 0, vault).is_empty() {
            break;
        }
    }
    assert!(snapshot_parts(&w, 0, 0, vault).len() >= 2);
}

/// A rollback that the user acknowledged leaves a hole between the numbers
/// that survived and the next ones; the housekeeping compacts at once, so
/// the readers' lag closes and nobody is warned.
#[test]
fn a_hole_after_an_acknowledged_rollback_heals_without_a_halt() {
    let (mut w, vault, item) = three_devices(617);
    let a = install(&w, 0);
    for i in 0..4 {
        set_password(&mut w, 0, vault, item, &format!("v{i}"));
        w.sync(0);
    }
    w.converge(0);
    let dir = log_dir(&w, 0, 0, vault);
    let segs = segments(&dir);
    for p in &segs[segs.len() - 3..] {
        fs::remove_file(p).unwrap();
    }
    w.cloud.settle().unwrap();
    relock(&mut w, 0);
    set_password(&mut w, 0, vault, item, "after");
    assert!(matches!(w.try_sync(0), Err(SyncError::RollbackDetected)));
    {
        let dev = &mut w.devs[0];
        dev.engine.clear_halt(&mut dev.access).unwrap();
    }
    w.sync(0);
    assert!(
        !snapshot_parts(&w, 0, 0, vault).is_empty(),
        "the hole was closed by a snapshot"
    );
    w.converge(1);
    w.advance(25 * HOUR_MS);
    for (d, report) in w.sync_all().iter().enumerate().skip(1) {
        assert!(!warns_missing(report, a), "device {d}: {report:?}");
        assert!(shows(&w, d, vault, item, "after"), "device {d}");
    }
}

/// A holder that cannot replace a planted blob counts the failure and
/// carries on: the run does not abort.
#[test]
fn a_failed_heal_is_counted_and_never_aborts_the_run() {
    let mut w = World::new(618, 2);
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "with file");
    let id = w.devs[0]
        .s_mut()
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment body")
        .unwrap();
    w.sync(0);
    w.cloud.settle().unwrap();
    let blob = blob_path(&w, 0, vault, id);
    let mut planted = fs::read(&blob).unwrap();
    let last = planted.len() - 1;
    planted[last] ^= 1;
    fs::write(&blob, &planted).unwrap();
    let mut faulty = Faulty::new(open_transport(
        &w.devs[0].access,
        &w.replica(0),
        &w.clock,
        "device 0",
    ));
    faulty.faults.fail_replace_attachment = true;
    let mut engine = SyncEngine::with_jitter(
        faulty,
        EngineConfig::for_kind(TransportKind::Folder),
        Box::new(FixedJitter(1000)),
    );
    let dev = &mut w.devs[0];
    let r = engine.sync_once(&mut dev.access, &w.clock).unwrap();
    assert_eq!(r.attachments_heal_failed, 1, "{r:?}");
    assert_eq!(r.attachments_healed, 0);
    assert_eq!(fs::read(&blob).unwrap(), planted, "still the planted file");
}
