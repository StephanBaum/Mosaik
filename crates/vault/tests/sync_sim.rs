//! Offline multi-device simulation (brief §5, CRYPTO_SPEC §10): three
//! devices of one account edit two vaults while offline, then reconcile
//! only through `apply_remote_record` and `apply_remote_vault_key`, with
//! the server's records and keys delivered in a shuffled order and with
//! duplicates and replays.
//!
//! Besides field edits, trash and deletes, a round can rotate one vault's
//! key (the new wrapped key reaches the other devices at a random point
//! among the records, so records of the new generation can arrive before
//! their key and older ones after it), move items between the vaults, and
//! add or remove attachments (device 0 has a file store; the others are in
//! memory and never hold attachment files).
//!
//! After every round the devices must converge to identical items, and no
//! write may be lost:
//!
//! - every password and tag written in the round is still visible in some
//!   live item (as the value, a conflict copy or a password history entry;
//!   a moved item carries it to its copy), unless the writing device
//!   deleted that item afterwards;
//! - a delete (or a move, which deletes the source) loses against a
//!   concurrent write by another device, and wins otherwise; a moved copy
//!   stays unless its mover deleted it;
//! - every stored record and the metadata are sealed under the vault's
//!   accepted key generation, which all devices agree on;
//! - attachment usage is the same everywhere, and device 0 can open every
//!   attachment file it holds;
//! - the merged state survives a lock and unlock, which queues nothing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::cast_possible_truncation,
    reason = "integration tests"
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{PASSWORD, Server, Shuffler, join, new_account_in, pw};
use mosaic_core::model::{ItemType, SecretField, Value};
use mosaic_core::{AttachmentId, Item, ItemId, KeyGeneration, SecretKey, VaultId, WrappedKey};
use mosaic_vault::session::{Error, ItemFilter, RemoteRecord, TrashFilter, UnlockedSession};
use mosaic_vault::storage::Store;

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
    /// this op wrote again (ADR-0009 item 5: the net edit is then a removal
    /// only, which does not beat a delete).
    fn undone_by(&self, later: &Self) -> bool {
        match (self, later) {
            (Self::Trash, Self::Restore) => true,
            (Self::AddAttachment(a), Self::RemoveAttachment(b)) => a == b,
            _ => false,
        }
    }
}

/// One thing the server hands a device.
#[derive(Clone)]
enum Delivery {
    Record(RemoteRecord),
    Key(VaultId, KeyGeneration, WrappedKey),
}

/// Every item of every vault, trash included.
fn everything() -> ItemFilter {
    let mut filter = ItemFilter::default();
    filter.trash = TrashFilter::Include;
    filter
}

struct Sim {
    _dir: tempfile::TempDir,
    devices: Vec<UnlockedSession>,
    server: Server,
    /// Wrapped vault keys published by rotations, in order.
    keys: Vec<(VaultId, KeyGeneration, WrappedKey)>,
    published: BTreeMap<VaultId, KeyGeneration>,
    cursors: Vec<usize>,
    key_cursors: Vec<usize>,
    /// Records that arrived before their generation's key.
    waiting: Vec<Vec<RemoteRecord>>,
    rng: Shuffler,
    vaults: [VaultId; 2],
    secret_key: SecretKey,
    counter: usize,
    /// Copy -> source, for every move so far.
    moved_from: BTreeMap<ItemId, ItemId>,
    /// Creation order of every item, so a seed picks the same items
    /// whatever the (random, UUIDv7) item IDs are.
    order: BTreeMap<ItemId, usize>,
}

impl Sim {
    fn new(seed: u64) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let (mut a, created) = new_account_in(Store::open(dir.path().join("device-0.db")).unwrap());
        let vaults = [
            a.create_vault("Shared").unwrap(),
            a.create_vault("Other").unwrap(),
        ];
        let b = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
        let c = join(&a, &created.secret_key, Store::open_in_memory().unwrap());
        let published = vaults.iter().map(|v| (*v, KeyGeneration::FIRST)).collect();
        let mut sim = Self {
            _dir: dir,
            devices: vec![a, b, c],
            server: Server::default(),
            keys: Vec::new(),
            published,
            cursors: vec![0; DEVICES],
            key_cursors: vec![0; DEVICES],
            waiting: vec![Vec::new(); DEVICES],
            rng: Shuffler::new(seed),
            vaults,
            secret_key: created.secret_key,
            counter: 0,
            moved_from: BTreeMap::new(),
            order: BTreeMap::new(),
        };
        for i in 0..6 {
            let vault = vaults[i % 2];
            let id = sim.devices[0]
                .add_item(vault, &ItemType::Login, |e| {
                    e.set_title(&format!("item {i}"))?;
                    e.set_secret(SecretField::Password, &format!("initial {i}"))
                })
                .unwrap();
            sim.created(id);
        }
        sim.sync();
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

    /// The live items of a device's view (trash included), both vaults, in
    /// creation order. Item IDs are random, so ordering by ID would make a
    /// seed pick different items on every run. (Replay is still not exact:
    /// device IDs are random too and break ties between concurrent writes,
    /// which later ops can depend on; see PROGRESS M2-R14.)
    fn live(&self, d: usize) -> Vec<(VaultId, ItemId)> {
        let mut live: Vec<(VaultId, ItemId)> = self.devices[d]
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
    /// vault's key at some point among them.
    fn offline_edits(&mut self, d: usize, rotate: bool, log: &mut Vec<(usize, ItemId, Op)>) {
        let n_ops = 1 + self.rng.below(3);
        let rotate_at = rotate.then(|| self.rng.below(n_ops + 1));
        for i in 0..=n_ops {
            if rotate_at == Some(i) {
                let vault = self.vaults[self.rng.below(2)];
                self.devices[d].rotate_vault_key(vault).unwrap();
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
            let id = self.devices[d]
                .add_item(vault, &ItemType::Login, |e| {
                    e.set_title(&title)?;
                    e.set_secret(SecretField::Password, &password)
                })
                .unwrap();
            self.created(id);
            log.push((d, id, Op::Password(password)));
            return;
        }
        let (vault, item) = live[self.rng.below(live.len())];
        let to = self.other_vault(vault);
        let n = self.counter + 1;
        self.counter += 1;
        let s = &mut self.devices[d];
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
                    let next = self.order.len();
                    self.order.entry(copy).or_insert(next);
                    // The copy is new; only its mover can touch it this
                    // round, and it must survive unless the mover deletes it.
                    log.push((d, copy, Op::Write));
                    Op::Move(copy)
                }
                // An attachment file this store does not hold (devices 1
                // and 2 hold none): nothing was changed.
                Err(Error::Store(_)) => {
                    assert!(!attachments.is_empty());
                    return;
                }
                Err(e) => panic!("move failed: {e:?}"),
            },
            86..93 if d == 0 => {
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

    /// Publishes the wrapped keys of a rotation `d` made.
    fn publish_keys(&mut self, d: usize) {
        for v in self.vaults {
            for (g, wrapped) in self.devices[d].store().vault_keys(&v).unwrap() {
                if g > self.published[&v] {
                    self.published.insert(v, g);
                    self.keys.push((v, g, wrapped));
                }
            }
        }
    }

    /// Applies one record: deferred until its key arrives, dropped as a
    /// rollback once a newer key is accepted (`CORE_API.md`: never retried).
    fn apply_record(&mut self, d: usize, r: &RemoteRecord) {
        match self.devices[d].apply_remote_record(r) {
            Ok(_) => {}
            Err(Error::KeyGenerationNotAccepted { .. }) => self.waiting[d].push(r.clone()),
            Err(Error::KeyGenerationRollback {
                highest_accepted,
                offered,
            }) => assert!(offered < highest_accepted),
            Err(e) => panic!("device {d} failed to apply a record: {e:?}"),
        }
    }

    fn deliver(&mut self, d: usize, delivery: Delivery) {
        match delivery {
            Delivery::Record(r) => self.apply_record(d, &r),
            Delivery::Key(v, g, wrapped) => {
                match self.devices[d].apply_remote_vault_key(v, g, &wrapped) {
                    // Accepted, or a device that already took a later key.
                    Ok(()) | Err(Error::KeyGenerationRollback { .. }) => {}
                    Err(e) => panic!("device {d} failed to apply a key: {e:?}"),
                }
                for r in std::mem::take(&mut self.waiting[d]) {
                    self.apply_record(d, &r);
                }
            }
        }
    }

    /// Uploads and delivers until no device has anything left to send.
    fn sync(&mut self) {
        for _ in 0..20 {
            let mut order: Vec<usize> = (0..DEVICES).collect();
            for i in (1..order.len()).rev() {
                let j = self.rng.below(i + 1);
                order.swap(i, j);
            }
            let mut uploaded = 0;
            for &d in &order {
                uploaded += self.server.upload(&mut self.devices[d]);
                self.publish_keys(d);
            }
            let caught_up = self.cursors.iter().all(|&c| c == self.server.log.len())
                && self.key_cursors.iter().all(|&c| c == self.keys.len());
            if uploaded == 0 && caught_up {
                assert!(self.waiting.iter().all(Vec::is_empty));
                return;
            }
            for &d in &order {
                let mut batch: Vec<Delivery> = self
                    .rng
                    .deliveries(&self.server.log[self.cursors[d]..])
                    .into_iter()
                    .map(Delivery::Record)
                    .collect();
                // Old versions arrive again now and then (replays).
                for _ in 0..2 {
                    if self.cursors[d] > 0 {
                        let old = self.rng.below(self.cursors[d]);
                        batch.push(Delivery::Record(self.server.log[old].clone()));
                    }
                }
                // New keys arrive at a random point among the records.
                for (v, g, wrapped) in &self.keys[self.key_cursors[d]..] {
                    let at = self.rng.below(batch.len() + 1);
                    batch.insert(at, Delivery::Key(*v, *g, wrapped.clone()));
                }
                for delivery in batch {
                    self.deliver(d, delivery);
                }
                self.cursors[d] = self.server.log.len();
                self.key_cursors[d] = self.keys.len();
            }
        }
        panic!("no convergence after 20 sync rounds");
    }

    /// Every record ID the server has seen, with its vault.
    fn records(&self) -> BTreeMap<ItemId, VaultId> {
        self.server
            .log
            .iter()
            .map(|r| (r.record_id, r.vault_id))
            .collect()
    }

    /// Every device holds the same items, keys and attachments.
    fn assert_converged(&self) -> BTreeMap<ItemId, bool> {
        let live = |d: &UnlockedSession| -> BTreeSet<(VaultId, ItemId)> {
            d.items(&everything())
                .iter()
                .map(|r| (r.vault_id, r.item_id))
                .collect()
        };
        let reference = &self.devices[0];
        let mut state = BTreeMap::new();
        for (id, vault) in self.records() {
            let x = reference.item(vault, id).unwrap();
            for other in &self.devices[1..] {
                let y = other.item(vault, id).unwrap();
                assert_eq!(x, y, "devices disagree on {id:?}");
            }
            assert_eq!(x.unreadable_fields().len(), 0);
            state.insert(id, x.is_deleted());
        }
        let generations = |d: &UnlockedSession| -> Vec<(VaultId, KeyGeneration)> {
            d.vaults()
                .iter()
                .map(|v| (v.vault_id, v.key_generation))
                .collect()
        };
        for d in &self.devices {
            assert_eq!(live(d), live(reference));
            assert_eq!(d.pending_uploads(1).unwrap().len(), 0);
            assert_eq!(d.corrupt_records().len(), 0);
            assert_eq!(generations(d), generations(reference));
            assert_eq!(d.attachment_usage(), reference.attachment_usage());
            for v in d.vaults() {
                for r in d.store().records(&v.vault_id).unwrap() {
                    assert_eq!(r.unwrap().body.key_generation(), v.key_generation);
                }
                let meta = d.store().vault_meta(&v.vault_id).unwrap().unwrap();
                assert_eq!(meta.key_generation(), v.key_generation);
            }
        }
        // Device 0 can open every attachment file it holds.
        for r in reference.items(&everything()) {
            for a in r.item.attachments() {
                if reference
                    .store()
                    .attachment_info(&a.attachment_id)
                    .unwrap()
                    .is_some()
                {
                    let plain = reference
                        .attachment(r.vault_id, r.item_id, a.attachment_id)
                        .unwrap();
                    assert!(plain.expose_secret().starts_with(b"attachment "));
                }
            }
        }
        // Review M2, third round (tests): every attachment device 0 stores
        // is referenced by a live item, and every file has its row. The
        // usage check above counts references only, so orphaned ciphertext
        // left by a missing prune would go unnoticed.
        let mut rows = 0usize;
        for v in reference.vaults() {
            for r in reference.store().records(&v.vault_id).unwrap() {
                let id = r.unwrap().record_id;
                let referenced: Vec<AttachmentId> = match reference.item(v.vault_id, id) {
                    Ok(item) if !item.is_deleted() => {
                        item.attachments().iter().map(|a| a.attachment_id).collect()
                    }
                    _ => Vec::new(),
                };
                for info in reference
                    .store()
                    .attachments_for_record(&v.vault_id, &id)
                    .unwrap()
                {
                    assert!(
                        referenced.contains(&info.attachment_id),
                        "orphaned attachment {:?} of {id:?}",
                        info.attachment_id
                    );
                    rows += 1;
                }
            }
        }
        let files = std::fs::read_dir(reference.store().attachment_dir().unwrap())
            .unwrap()
            .count();
        assert_eq!(files, rows, "attachment files without a row");
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
        self.devices[0]
            .items(&everything())
            .iter()
            .any(|r| pred(r.item))
    }
}

/// Whether `value` is the item's password, a conflict copy of it, or in
/// its password history.
fn has_password(item: &Item, value: &str) -> bool {
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
        // later in the round (trash then restore, or adding then removing
        // one attachment): the net edit is then a removal only, which does
        // not beat a delete (ADR-0009 item 5).
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
        // The seed and the round go into the failure message, so a failure
        // of the long run can be replayed with `run(seed, round + 1)`.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ops = Vec::new();
            // At most one rotation per round: concurrent rotations of one
            // vault are refused (ADR-0013) and resolved by the M6 server.
            let rotator = (sim.rng.below(3) == 0).then(|| sim.rng.below(DEVICES));
            for d in 0..DEVICES {
                sim.offline_edits(d, rotator == Some(d), &mut ops);
            }
            sim.sync();
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
    let a = sim.devices.remove(0);
    let a = a.lock().unlock(&pw(PASSWORD), &sim.secret_key).unwrap();
    assert_eq!(a.pending_uploads(1).unwrap().len(), 0);
    let b = &sim.devices[0];
    for (id, vault) in sim.records() {
        assert!(
            a.item(vault, id).unwrap() == b.item(vault, id).unwrap(),
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

/// Regression seed of the long run: in round 9, device 0 trashed and
/// restored an item (or added and removed one attachment) while device 1
/// changed its password and deleted it. Device 0's net edit only removed
/// what it had written, so the delete wins (ADR-0009 item 5); the oracle
/// used to count device 0's first write as a surviving concurrent write.
#[test]
fn regression_seed_229651() {
    run(229_651, 10);
}

/// Review M2, third round (tests): the scenario of seed 229651 replayed
/// deterministically (the seed itself is not an exact replay, see `live`).
/// ADR-0009 item 5: a concurrent edit whose net effect only removed what
/// it wrote itself does not beat a delete; a real write does.
#[test]
fn a_net_removal_does_not_beat_a_concurrent_delete() {
    // Device 0 trashes and restores, device 1 changes the password and
    // then deletes: the delete wins everywhere.
    let mut sim = Sim::new(5);
    let (vault, item) = sim.live(0)[0];
    sim.devices[0].trash_item(vault, item).unwrap();
    sim.devices[0].restore_item(vault, item).unwrap();
    sim.devices[1]
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, "late"))
        .unwrap();
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.sync();
    assert!(sim.assert_converged()[&item], "the delete wins");

    // Without the restore, device 0's trash is a real write: the item
    // survives (in the trash) and carries the deletion notice.
    let mut sim = Sim::new(5);
    let (vault, item) = sim.live(0)[0];
    sim.devices[0].trash_item(vault, item).unwrap();
    sim.devices[1]
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, "late"))
        .unwrap();
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.sync();
    assert!(!sim.assert_converged()[&item], "the trash write wins");
    let it = sim.devices[2].item(vault, item).unwrap();
    assert!(it.trashed_at().is_some());
    assert!(it.deleted_concurrently().is_some());
}

/// The same with an attachment that device 0 adds and removes again.
#[test]
fn an_attachment_added_and_removed_does_not_beat_a_concurrent_delete() {
    let mut sim = Sim::new(6);
    let (vault, item) = sim.live(0)[0];
    let att = sim.devices[0]
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment x")
        .unwrap();
    sim.devices[0].remove_attachment(vault, item, att).unwrap();
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.sync();
    assert!(sim.assert_converged()[&item], "the delete wins");

    // Keeping the attachment is a real write: the item survives with it.
    let mut sim = Sim::new(6);
    let (vault, item) = sim.live(0)[0];
    let att = sim.devices[0]
        .add_attachment(vault, item, "a.txt", "text/plain", b"attachment x")
        .unwrap();
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.sync();
    assert!(!sim.assert_converged()[&item], "the added attachment wins");
    let it = sim.devices[0].item(vault, item).unwrap();
    assert_eq!(it.attachments()[0].attachment_id, att);
    assert!(it.deleted_concurrently().is_some());
}

#[test]
fn concurrent_deletes_and_edits_of_one_item() {
    // Deterministic: all three devices touch the same item in one round.
    let mut sim = Sim::new(99);
    let (vault, item) = sim.live(0)[0];
    sim.devices[0].purge_item(vault, item).unwrap();
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.devices[2]
        .update_item(vault, item, |e| {
            e.set_secret(SecretField::Password, "survivor")
        })
        .unwrap();
    sim.sync();
    let state = sim.assert_converged();
    assert!(!state[&item]);
    let it = sim.devices[0].item(vault, item).unwrap();
    assert!(has_password(it, "survivor"));
    assert_eq!(
        it.title(),
        Some("item 0"),
        "the editing device restored the title"
    );
    let info = it.deleted_concurrently().unwrap();
    assert_eq!(info.deleted_by.len(), 2);

    // Deleting again, with no concurrent write, wins everywhere.
    sim.devices[1].purge_item(vault, item).unwrap();
    sim.sync();
    assert!(sim.assert_converged()[&item]);
}

#[test]
fn a_rotation_during_offline_edits_reaches_everyone() {
    // Deterministic: device 1 rotates while 0 and 2 edit under the old
    // key; their edits are re-sealed when the key arrives (before or after
    // the rotated records) and nothing is lost.
    let mut sim = Sim::new(7);
    let vault = sim.vaults[0];
    let items: Vec<ItemId> = sim
        .live(0)
        .into_iter()
        .filter(|(v, _)| *v == vault)
        .map(|(_, i)| i)
        .collect();
    sim.devices[0]
        .update_item(vault, items[0], |e| {
            e.set_secret(SecretField::Password, "from-0")
        })
        .unwrap();
    let g = sim.devices[1].rotate_vault_key(vault).unwrap();
    sim.devices[2]
        .update_item(vault, items[1], |e| {
            e.set_secret(SecretField::Password, "from-2")
        })
        .unwrap();
    sim.devices[1]
        .update_item(vault, items[2], |e| {
            e.set_secret(SecretField::Password, "from-1")
        })
        .unwrap();
    sim.sync();
    sim.assert_converged();
    for d in &sim.devices {
        assert!(
            d.vaults()
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
            sim.devices[0].item(vault, item).unwrap(),
            value
        ));
    }
}

/// Review M2, fourth round (sync): oversized merges under shuffled
/// delivery with duplicates and replays. Every device writes a 400 kB note
/// into one item concurrently (the merge of three does not fit, so the
/// record is split); in every other seed the item already holds a 400 kB
/// note, two devices write 400 kB user names and device 0 purges
/// concurrently, so the writers' repairs bring the note back and their
/// merge does not fit. The devices converge, no value is lost (each is the
/// record's or a conflict copy's), and the purge loses against the writes
/// with its notice kept.
#[test]
fn concurrent_large_notes_split_and_converge_under_shuffled_delivery() {
    large_notes(0..2);
}

/// The scenario of
/// `concurrent_large_notes_split_and_converge_under_shuffled_delivery`
/// for the given seeds (even: three writers; odd: a purge races two).
fn large_notes(seeds: core::ops::Range<u64>) {
    let n = 400_000;
    let has_value = |item: &Item, key: &str, value: &str| {
        item.entries(key)
            .any(|e| matches!(e.value(), Value::String(t) if t.as_str() == value))
    };
    for seed in seeds {
        let mut sim = Sim::new(1000 + seed);
        let (vault, item) = sim.live(0)[0];
        let purge = seed % 2 == 1;
        if purge {
            let old = "o".repeat(n);
            sim.devices[0]
                .update_item(vault, item, |e| {
                    e.set_text(mosaic_core::model::TextField::Notes, &old)
                })
                .unwrap();
            sim.sync();
            sim.assert_converged();
        }
        // With the purge, the writers write the user name, so the large
        // note the purge removed comes back through the repairs.
        let (writers, field, key) = if purge {
            (
                vec![1, 2],
                mosaic_core::model::TextField::Username,
                "login.username",
            )
        } else {
            (vec![0, 1, 2], mosaic_core::model::TextField::Notes, "notes")
        };
        let mut notes = Vec::new();
        for &d in &writers {
            let note = format!("{d}").repeat(n);
            sim.devices[d]
                .update_item(vault, item, |e| e.set_text(field, &note))
                .unwrap();
            notes.push((key, note));
        }
        if purge {
            notes.push(("notes", "o".repeat(n)));
        }
        if purge {
            sim.devices[0].purge_item(vault, item).unwrap();
        }
        sim.sync();
        let state = sim.assert_converged();
        assert!(!state[&item], "seed {seed}: the writes beat the purge");
        for (key, note) in &notes {
            assert!(
                sim.any_live(|i| has_value(i, key, note)),
                "seed {seed}: a large value of {key} was lost"
            );
        }
        let copies = sim.devices[0]
            .items(&everything())
            .iter()
            .filter(|r| r.item.conflict_copy_of() == Some(item))
            .count();
        assert!(copies >= 1, "seed {seed}: the split left a conflict copy");
        if purge {
            assert!(
                sim.devices[0]
                    .item(vault, item)
                    .unwrap()
                    .deleted_concurrently()
                    .is_some(),
                "seed {seed}: the deletion notice stays"
            );
        }
    }
}

/// The long version, run by the gate's `cargo test --release -- --ignored`.
#[test]
#[ignore = "long-running; run with --release -- --ignored"]
fn three_devices_converge_many_seeds() {
    for seed in 1..=60 {
        run(seed * 7919, 12);
    }
    large_notes(2..24);
}
