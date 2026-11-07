//! The M2 multi-device simulation over the hosted backend (brief §5,
//! CRYPTO_SPEC §9, §10): three devices of one account edit two vaults while
//! offline, then reconcile only through the sync engine and the real server
//! on 127.0.0.1, through a chaos transport that fails calls, loses responses
//! after the effect and repeats requests.
//!
//! Besides field edits, trash and deletes, a round can rotate one vault's
//! key (offline, queued, or online through the engine), move items between
//! the vaults, and add or remove attachments (all three devices have file
//! stores).
//!
//! After every round the devices must converge:
//!
//! - identical items, keys, generations and attachment usage everywhere;
//! - every password and tag written in the round is still visible in some
//!   live item (as the value, a conflict copy or a password history entry;
//!   a moved item carries it to its copy), unless the writing device
//!   deleted that item afterwards;
//! - a delete (or a move, which deletes the source) loses against a
//!   concurrent write by another device, and wins otherwise;
//! - every stored record and the metadata are sealed under the vault's
//!   accepted key generation;
//! - every outbox and upload queue is empty and no report warns;
//! - every referenced attachment is present on every device, and every
//!   attachment file has its row;
//! - the merged state survives a lock and unlock, which queues nothing.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::model::{ItemType, SecretField, Value};
use mosaic_core::{AttachmentId, Item, ItemId, KeyGeneration, VaultId};
use mosaic_sync::engine::{RotationOutcome, SyncError};
use mosaic_sync::testutil::SimRng;
use mosaic_vault::session::{Error, UnlockedSession};

use crate::hosted::{FLAKY, PASSWORD, World, everything, pw};

const DEVICES: usize = 3;

/// What a device did to an item in a round.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Password(String),
    Tag(String),
    /// A write that is not checked for its value (a title, the copy a move
    /// created).
    Write,
    /// Moved to the trash: writes `trashed_at`.
    Trash,
    /// Restored from the trash: removes `trashed_at` only, so it does not
    /// beat a delete, and it undoes a `Trash` of the same round.
    Restore,
    /// Added an attachment (a write).
    AddAttachment(AttachmentId),
    /// Removed an attachment: a removal only, which undoes an
    /// `AddAttachment` of the same attachment in the same round.
    RemoveAttachment(AttachmentId),
    Delete,
    /// Moved to another vault as the given new item: deletes the source.
    Move(ItemId),
}

impl Op {
    /// Whether the op writes a value that a concurrent delete must not
    /// remove.
    const fn writes(&self) -> bool {
        matches!(
            self,
            Self::Password(_) | Self::Tag(_) | Self::Write | Self::Trash | Self::AddAttachment(_)
        )
    }

    /// Whether `later`, by the same device on the same item, removes what
    /// this op wrote again (ADR-0009 item 5).
    fn undone_by(&self, later: &Self) -> bool {
        match (self, later) {
            (Self::Trash, Self::Restore) => true,
            (Self::AddAttachment(a), Self::RemoveAttachment(b)) => a == b,
            _ => false,
        }
    }
}

struct Sim {
    w: World,
    rng: SimRng,
    vaults: [VaultId; 2],
    counter: usize,
    /// Copy -> source, for every move so far.
    moved_from: BTreeMap<ItemId, ItemId>,
    /// Creation order of every item, so a seed picks the same items
    /// whatever the (random, UUIDv7) item IDs are.
    order: BTreeMap<ItemId, usize>,
    /// Every record ID that was ever written, with its vault.
    records: BTreeMap<ItemId, VaultId>,
}

impl Sim {
    fn new(seed: u64) -> Self {
        let mut w = World::new(seed, FLAKY);
        let vaults = [w.new_vault(0, "Shared"), w.new_vault(0, "Other")];
        let mut order = BTreeMap::new();
        let mut records = BTreeMap::new();
        for i in 0..6 {
            let vault = vaults[i % 2];
            let id = w.devs[0]
                .s_mut()
                .add_item(vault, &ItemType::Login, |e| {
                    e.set_title(&format!("item {i}"))?;
                    e.set_secret(SecretField::Password, &format!("initial {i}"))
                })
                .unwrap();
            order.insert(id, i);
            records.insert(id, vault);
        }
        w.sync(0);
        for _ in 1..DEVICES {
            w.join();
        }
        let mut sim = Self {
            w,
            rng: SimRng::new(seed),
            vaults,
            counter: 0,
            moved_from: BTreeMap::new(),
            order,
            records,
        };
        sim.sync(0);
        sim
    }

    fn unique(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}-{}", self.counter)
    }

    fn created(&mut self, id: ItemId) {
        let next = self.order.len();
        self.order.entry(id).or_insert(next);
    }

    fn device(&self, d: usize) -> mosaic_sync::testutil::SessionRef<'_> {
        self.w.devs[d].s()
    }

    /// The live items of a device's view (trash included), both vaults, in
    /// creation order. Item IDs are random, so ordering by ID would make a
    /// seed pick different items on every run.
    fn live(&self, d: usize) -> Vec<(VaultId, ItemId)> {
        let mut live: Vec<(VaultId, ItemId)> = self
            .device(d)
            .items(&everything())
            .iter()
            .map(|r| (r.vault_id, r.item_id))
            .collect();
        live.sort_by_key(|(_, id)| (self.order.get(id).copied().unwrap_or(usize::MAX), *id));
        live
    }

    fn other_vault(&self, v: VaultId) -> VaultId {
        if v == self.vaults[0] {
            self.vaults[1]
        } else {
            self.vaults[0]
        }
    }

    /// One device's offline edits for a round; `rotate` makes it rotate a
    /// vault's key (queued, offline) at some point among them.
    fn offline_edits(&mut self, d: usize, rotate: bool, log: &mut Vec<(usize, ItemId, Op)>) {
        let n_ops = 1 + self.rng.below(3);
        let rotate_at = rotate.then(|| self.rng.below(n_ops + 1));
        for i in 0..=n_ops {
            if rotate_at == Some(i) {
                let vault = self.vaults[self.rng.below(2)];
                self.w.devs[d].s_mut().rotate_vault_key(vault).unwrap();
            }
            if i < n_ops {
                self.one_edit(d, log);
            }
        }
    }

    fn one_edit(&mut self, d: usize, log: &mut Vec<(usize, ItemId, Op)>) {
        let live = self.live(d);
        let roll = self.rng.below(100);
        if live.is_empty() || roll >= 96 {
            let vault = self.vaults[self.rng.below(2)];
            let title = self.unique("new");
            let password = self.unique("pw");
            let id = self.w.devs[d]
                .s_mut()
                .add_item(vault, &ItemType::Login, |e| {
                    e.set_title(&title)?;
                    e.set_secret(SecretField::Password, &password)
                })
                .unwrap();
            self.created(id);
            self.records.insert(id, vault);
            log.push((d, id, Op::Password(password)));
            return;
        }
        let (vault, item) = live[self.rng.below(live.len())];
        let to = self.other_vault(vault);
        let n = self.counter + 1;
        self.counter += 1;
        let mut s = self.w.devs[d].s_mut();
        let attachments = s.item(vault, item).unwrap().attachments();
        let op = match roll {
            0..36 => {
                let v = format!("pw-{d}-{n}");
                s.update_item(vault, item, |e| e.set_secret(SecretField::Password, &v))
                    .unwrap();
                Op::Password(v)
            }
            36..52 => {
                let t = format!("tag{d}x{n}");
                s.update_item(vault, item, |e| e.add_tag(&t)).unwrap();
                Op::Tag(t)
            }
            52..60 => {
                let t = format!("title-{d}-{n}");
                s.update_item(vault, item, |e| e.set_title(&t)).unwrap();
                Op::Write
            }
            60..68 => {
                if s.item(vault, item).unwrap().trashed_at().is_some() {
                    s.restore_item(vault, item).unwrap();
                    Op::Restore
                } else {
                    s.trash_item(vault, item).unwrap();
                    Op::Trash
                }
            }
            68..76 => {
                s.purge_item(vault, item).unwrap();
                Op::Delete
            }
            76..86 => match s.move_item(vault, item, to) {
                Ok(copy) => {
                    self.moved_from.insert(copy, item);
                    self.records.insert(copy, to);
                    let next = self.order.len();
                    self.order.entry(copy).or_insert(next);
                    // The copy is new; only its mover can touch it this
                    // round, and it must survive unless the mover deletes it.
                    log.push((d, copy, Op::Write));
                    Op::Move(copy)
                }
                // An attachment file this store does not hold: nothing was
                // changed.
                Err(Error::Store(_)) => {
                    assert!(!attachments.is_empty());
                    return;
                }
                Err(e) => panic!("move failed: {e:?}"),
            },
            86..93 => {
                let body = format!("attachment {n} of {item}");
                let id = s
                    .add_attachment(vault, item, "a.txt", "text/plain", body.as_bytes())
                    .unwrap();
                Op::AddAttachment(id)
            }
            86..96 if !attachments.is_empty() => {
                let id = attachments[self.rng.below(attachments.len())].attachment_id;
                s.remove_attachment(vault, item, id).unwrap();
                Op::RemoveAttachment(id)
            }
            _ => {
                let t = format!("title-{d}-{n}");
                s.update_item(vault, item, |e| e.set_title(&t)).unwrap();
                Op::Write
            }
        };
        log.push((d, item, op));
    }

    /// Syncs until every device is at rest (a flaky pass, then calm ones).
    fn sync(&mut self, flaky: usize) {
        self.w.converge(flaky);
    }

    /// Every device holds the same items, keys and attachments, with no
    /// queue left and no warning.
    fn assert_converged(&mut self) -> BTreeMap<ItemId, bool> {
        let live = |d: &UnlockedSession| -> BTreeSet<(VaultId, ItemId)> {
            d.items(&everything())
                .iter()
                .map(|r| (r.vault_id, r.item_id))
                .collect()
        };
        let generations = |d: &UnlockedSession| -> Vec<(VaultId, KeyGeneration)> {
            d.vaults()
                .iter()
                .map(|v| (v.vault_id, v.key_generation))
                .collect()
        };
        let mut state = BTreeMap::new();
        {
            let reference = self.device(0);
            for (id, vault) in &self.records {
                let x = reference.item(*vault, *id).unwrap();
                for d in 1..DEVICES {
                    let other = self.device(d);
                    let y = other.item(*vault, *id).unwrap();
                    assert!(
                        x == y,
                        "devices 0 and {d} disagree on {id:?}: {:?} vs {:?}; moved_from {:?};                          0: {:?} 2: {:?}",
                        x.version(),
                        y.version(),
                        self.moved_from.get(id),
                        reference.store().record(vault, id).unwrap().map(|r| (
                            r.server_seq,
                            r.outbox_seq.map(mosaic_vault::storage::OutboxSeq::get),
                            r.body.key_generation(),
                            r.body.kind()
                        )),
                        other.store().record(vault, id).unwrap().map(|r| (
                            r.server_seq,
                            r.outbox_seq.map(mosaic_vault::storage::OutboxSeq::get),
                            r.body.key_generation(),
                            r.body.kind()
                        )),
                    );
                }
                assert_eq!(x.unreadable_fields().len(), 0);
                state.insert(*id, x.is_deleted());
            }
        }
        let (ref_live, ref_generations, ref_usage) = {
            let reference = self.device(0);
            (
                live(&reference),
                generations(&reference),
                reference.attachment_usage(),
            )
        };
        for d in 0..DEVICES {
            let s = self.device(d);
            assert_eq!(live(&s), ref_live);
            assert_eq!(s.pending_uploads(1).unwrap().len(), 0, "device {d} outbox");
            assert_eq!(s.corrupt_records().len(), 0);
            assert_eq!(generations(&s), ref_generations);
            assert_eq!(s.attachment_usage(), ref_usage);
            for v in s.vaults() {
                for r in s.store().records(&v.vault_id).unwrap() {
                    assert_eq!(r.unwrap().body.key_generation(), v.key_generation);
                }
                let meta = s.store().vault_meta(&v.vault_id).unwrap().unwrap();
                assert_eq!(meta.key_generation(), v.key_generation);
            }
            assert_attachments_complete(&s, d);
            assert!(
                s.pending_attachment_uploads().unwrap().is_empty(),
                "device {d} attachment uploads"
            );
            assert!(
                s.pending_attachment_deletions().unwrap().is_empty(),
                "device {d} attachment deletions"
            );
        }
        for d in 0..DEVICES {
            let pending = self.w.devs[d].s_mut().pending_vault_uploads().unwrap();
            assert!(pending.is_empty(), "device {d} vault uploads: {pending:?}");
            assert!(
                self.w.devs[d].last.warnings.is_empty(),
                "device {d} warns: {:?}",
                self.w.devs[d].last.warnings
            );
        }
        state
    }

    /// The item a chain of moves started from.
    fn root(&self, mut id: ItemId) -> ItemId {
        while let Some(source) = self.moved_from.get(&id) {
            id = *source;
        }
        id
    }

    /// Whether `pred` holds for some live item on device 0.
    fn any_live(&self, pred: impl Fn(&Item) -> bool) -> bool {
        self.device(0)
            .items(&everything())
            .iter()
            .any(|r| pred(r.item))
    }

    /// Rotates `vault` on device `d` online, through the engine. A call that
    /// fails (a fault of the chaos transport) is repeated, as the app would
    /// on its next attempt.
    fn rotate_online(&mut self, d: usize, vault: VaultId) -> KeyGeneration {
        for _ in 0..200 {
            self.w.server.advance_ms(60_000);
            let dev = &mut self.w.devs[d];
            match dev
                .engine
                .rotate_vault(&mut dev.access, &self.w.clock, vault)
            {
                Ok(RotationOutcome::Rotated(g) | RotationOutcome::RotatedByAnother(g)) => {
                    return g;
                }
                Err(SyncError::Transport(_)) => {}
                Err(e) => panic!("rotation failed: {e:?}"),
            }
        }
        panic!("rotation never got through");
    }
}

/// Every attachment a device references has its file, every file its row,
/// and the device can open every one.
fn assert_attachments_complete(s: &UnlockedSession, d: usize) {
    use mosaic_vault::session::AttachmentFileState;
    let mut referenced: BTreeSet<AttachmentId> = BTreeSet::new();
    for r in s.attachment_references() {
        referenced.insert(r.attachment_id);
        assert!(
            matches!(
                s.attachment_file_state(r.attachment_id).unwrap(),
                AttachmentFileState::Present { .. }
            ),
            "device {d} lacks a referenced attachment"
        );
    }
    for r in s.items(&everything()) {
        for a in r.item.attachments() {
            let plain = s
                .attachment(r.vault_id, r.item_id, a.attachment_id)
                .unwrap();
            assert!(plain.expose_secret().starts_with(b"attachment "));
        }
    }
    let mut rows = 0usize;
    for v in s.vaults() {
        for r in s.store().records(&v.vault_id).unwrap() {
            let id = r.unwrap().record_id;
            for info in s.store().attachments_for_record(&v.vault_id, &id).unwrap() {
                assert!(
                    referenced.contains(&info.attachment_id),
                    "device {d}: orphaned attachment {:?}",
                    info.attachment_id
                );
                rows += 1;
            }
        }
    }
    let files = std::fs::read_dir(s.store().attachment_dir().unwrap())
        .unwrap()
        .count();
    assert_eq!(files, rows, "device {d}: attachment files without a row");
}

/// Whether `value` is the item's password, a conflict copy of it, or in
/// its password history.
pub fn has_password(item: &Item, value: &str) -> bool {
    if item
        .secret(SecretField::Password)
        .unwrap()
        .is_some_and(|s| s.expose_secret() == value)
    {
        return true;
    }
    if item
        .password_history()
        .iter()
        .any(|h| h.value.expose_secret() == value)
    {
        return true;
    }
    item.conflicts().iter().any(|c| {
        c.key == SecretField::Password.key()
            && c.losers
                .iter()
                .any(|e| matches!(e.value(), Value::String(t) if t.as_str() == value))
    })
}

fn check_round(sim: &Sim, ops: &[(usize, ItemId, Op)], state: &BTreeMap<ItemId, bool>) {
    let mut by_item: BTreeMap<ItemId, Vec<(usize, &Op)>> = BTreeMap::new();
    for (d, item, op) in ops {
        by_item.entry(*item).or_default().push((*d, op));
    }
    // Devices that deleted some item of a move chain this round.
    let mut deleted_chain: BTreeSet<(usize, ItemId)> = BTreeSet::new();
    for (d, item, op) in ops {
        if *op == Op::Delete {
            deleted_chain.insert((*d, sim.root(*item)));
        }
    }
    for (item_id, item_ops) in &by_item {
        let deleted = state[item_id];
        let deleters: BTreeSet<usize> = item_ops
            .iter()
            .filter(|(_, op)| matches!(op, Op::Delete | Op::Move(_)))
            .map(|(d, _)| *d)
            .collect();
        // A write by a device that did not delete is concurrent with every
        // delete of this round, unless the same device removed it again
        // later in the round (ADR-0009 item 5).
        let concurrent_write = item_ops.iter().enumerate().any(|(i, (d, op))| {
            !deleters.contains(d)
                && op.writes()
                && !item_ops[i + 1..]
                    .iter()
                    .any(|(later_d, later)| later_d == d && op.undone_by(later))
        });
        if deleters.is_empty() {
            assert!(
                !deleted,
                "{item_id:?} vanished without a delete: {item_ops:?}"
            );
        } else if concurrent_write {
            assert!(
                !deleted,
                "a delete beat a concurrent write on {item_id:?}: {item_ops:?}"
            );
        } else {
            assert!(
                deleted,
                "a delete without a concurrent write was lost on {item_id:?}: {item_ops:?}"
            );
        }
    }
    // No write is lost: it is visible in some live item (a moved item
    // carries it to its copy), unless its device deleted that item.
    for (d, item_id, op) in ops {
        if deleted_chain.contains(&(*d, sim.root(*item_id))) {
            continue;
        }
        match op {
            Op::Password(v) => assert!(
                sim.any_live(|i| has_password(i, v)),
                "password {v} from device {d} lost (item {item_id:?})"
            ),
            Op::Tag(t) => assert!(
                sim.any_live(|i| i.tags().contains(&t.as_str())),
                "tag {t} from device {d} lost (item {item_id:?})"
            ),
            Op::Write
            | Op::Trash
            | Op::Restore
            | Op::AddAttachment(_)
            | Op::RemoveAttachment(_)
            | Op::Delete
            | Op::Move(_) => {}
        }
    }
}

fn run(seed: u64, rounds: usize) {
    let mut sim = Sim::new(seed);
    sim.assert_converged();
    for round in 0..rounds {
        // The seed and the round go into the failure message.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ops = Vec::new();
            // At most one rotation per round; it is either queued offline or
            // done online through the engine.
            let rotator = (sim.rng.below(3) == 0).then(|| sim.rng.below(DEVICES));
            let online = sim.rng.below(2) == 0;
            for d in 0..DEVICES {
                sim.offline_edits(d, rotator == Some(d) && !online, &mut ops);
            }
            if let (Some(d), true) = (rotator, online) {
                let vault = sim.vaults[sim.rng.below(2)];
                sim.rotate_online(d, vault);
            }
            sim.sync(1);
            let state = sim.assert_converged();
            check_round(&sim, &ops, &state);
        }));
        if let Err(cause) = result {
            let message = cause
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| cause.downcast_ref::<&str>().copied())
                .unwrap_or("(no message)");
            panic!("seed {seed}, round {round}: {message}");
        }
    }
    // The merged state is what was persisted: a relocked device matches,
    // and unlocking finds nothing left to re-seal.
    let secret_key = &sim.w.secret_key;
    let a = sim.w.devs[0].access.take().unwrap();
    let a = a.lock().unlock(&pw(PASSWORD), secret_key).unwrap();
    assert_eq!(a.pending_uploads(1).unwrap().len(), 0);
    let b = sim.w.devs[1].s();
    for (id, vault) in &sim.records {
        assert!(
            a.item(*vault, *id).unwrap() == b.item(*vault, *id).unwrap(),
            "persisted state differs from the in-memory state"
        );
    }
}

#[test]
fn three_devices_converge_without_losing_writes() {
    for seed in [1, 2, 3] {
        run(seed, 6);
    }
}

/// Long version, run by the gate's `cargo test --release -- --ignored`.
#[test]
#[ignore = "long-running; run with --release -- --ignored"]
fn three_devices_converge_many_seeds() {
    for seed in 1..=60 {
        run(seed * 7919, 12);
    }
}

#[test]
fn concurrent_deletes_and_edits_of_one_item() {
    // All three devices touch the same item in one round.
    let mut sim = Sim::new(99);
    let (vault, item) = sim.live(0)[0];
    sim.w.devs[0].s_mut().purge_item(vault, item).unwrap();
    sim.w.devs[1].s_mut().purge_item(vault, item).unwrap();
    sim.w.devs[2]
        .s_mut()
        .update_item(vault, item, |e| {
            e.set_secret(SecretField::Password, "survivor")
        })
        .unwrap();
    sim.sync(1);
    let state = sim.assert_converged();
    assert!(!state[&item]);
    {
        let session = sim.device(0);
        let it = session.item(vault, item).unwrap();
        assert!(has_password(it, "survivor"));
        assert_eq!(
            it.title(),
            Some("item 0"),
            "the editing device restored the title"
        );
        assert_eq!(it.deleted_concurrently().unwrap().deleted_by.len(), 2);
    }

    // Deleting again, with no concurrent write, wins everywhere.
    sim.w.devs[1].s_mut().purge_item(vault, item).unwrap();
    sim.sync(1);
    assert!(sim.assert_converged()[&item]);
}

#[test]
fn a_rotation_during_offline_edits_reaches_everyone() {
    // Device 1 rotates (queued, offline) while 0 and 2 edit under the old
    // key; their edits are re-sealed when the key arrives and nothing is
    // lost.
    let mut sim = Sim::new(7);
    let vault = sim.vaults[0];
    let items: Vec<ItemId> = sim
        .live(0)
        .into_iter()
        .filter(|(v, _)| *v == vault)
        .map(|(_, i)| i)
        .collect();
    sim.w.devs[0]
        .s_mut()
        .update_item(vault, items[0], |e| {
            e.set_secret(SecretField::Password, "from-0")
        })
        .unwrap();
    let g = sim.w.devs[1].s_mut().rotate_vault_key(vault).unwrap();
    sim.w.devs[2]
        .s_mut()
        .update_item(vault, items[1], |e| {
            e.set_secret(SecretField::Password, "from-2")
        })
        .unwrap();
    sim.w.devs[1]
        .s_mut()
        .update_item(vault, items[2], |e| {
            e.set_secret(SecretField::Password, "from-1")
        })
        .unwrap();
    sim.sync(1);
    sim.assert_converged();
    for d in 0..DEVICES {
        assert!(
            sim.device(d)
                .vaults()
                .iter()
                .any(|v| v.vault_id == vault && v.key_generation == g)
        );
    }
    for (item, value) in [
        (items[0], "from-0"),
        (items[1], "from-2"),
        (items[2], "from-1"),
    ] {
        assert!(has_password(
            sim.device(0).item(vault, item).unwrap(),
            value
        ));
    }
}
