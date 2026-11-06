//! Clock skew of days between three devices, in model-checker style: random
//! seeded schedules of edits, tags, trash, deletes, moves between vaults
//! and key rotations made offline, delivered by a flaky file-sync client, with each device's
//! engine and folder transport on its own clock. After every round the
//! devices must agree, no write may be lost, no report may warn, and the
//! outcome must not depend on the clocks or the delivery order: two runs of
//! one schedule with mirrored skews end with the same values.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::model::SecretField;
use mosaic_core::{ItemId, VaultId};
use mosaic_sync::testutil::{ManualClock, SimConfig, SimRng};
use mosaic_vault::session::UnlockedSession;

use super::util::{add_login, has_password, password_values, relock_with_clock, sync_with};
use crate::common::{World, everything, is_idle};

const DEVICES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Password(String),
    Tag(String),
    Trash,
    Restore,
    Delete,
    /// Moved to the other vault as the given copy (deletes the source).
    Move(ItemId),
    /// The copy a move made (only its mover touches it this round).
    Copy,
}

impl Op {
    const fn writes(&self) -> bool {
        matches!(
            self,
            Self::Password(_) | Self::Tag(_) | Self::Trash | Self::Copy
        )
    }
}

struct Run {
    w: World,
    clocks: Vec<ManualClock>,
    vaults: [VaultId; 2],
    rng: SimRng,
    counter: usize,
    order: BTreeMap<ItemId, usize>,
    records: BTreeMap<ItemId, VaultId>,
    moved_from: BTreeMap<ItemId, ItemId>,
}

/// What one item ended as, independent of IDs and of which concurrent value
/// a random writer ID made the shown one.
type Outcome = BTreeMap<usize, (bool, Vec<String>, Vec<String>)>;

impl Run {
    /// Three devices whose clocks are off by `skews_ms` from the shared
    /// time (negative: behind).
    fn new(seed: u64, skews_ms: [i64; DEVICES]) -> Self {
        let mut w = World::new(seed, DEVICES);
        let vaults = [w.new_vault(0, "Shared"), w.new_vault(0, "Other")];
        let mut order = BTreeMap::new();
        let mut records = BTreeMap::new();
        for i in 0..4 {
            let vault = vaults[i % 2];
            let id = add_login(&mut w, 0, vault, &format!("item {i}"));
            order.insert(id, i);
            records.insert(id, vault);
        }
        w.sync(0);
        w.cloud.settle().unwrap();
        for _ in 1..DEVICES {
            w.join();
        }
        let base = mosaic_sync::clock::Clock::now_ms(&w.clock);
        let clocks: Vec<ManualClock> = skews_ms
            .iter()
            .map(|s| {
                let c = ManualClock::new();
                c.set_ms(base.saturating_add_signed(*s));
                c
            })
            .collect();
        for (d, c) in clocks.iter().enumerate() {
            relock_with_clock(&mut w, d, c);
        }
        let mut run = Self {
            w,
            clocks,
            vaults,
            rng: SimRng::new(seed),
            counter: 0,
            order,
            records,
            moved_from: BTreeMap::new(),
        };
        run.converge(0);
        run
    }

    fn device(&self, d: usize) -> mosaic_sync::testutil::SessionRef<'_> {
        self.w.devs[d].s()
    }

    fn advance(&self, ms: u64) {
        self.w.clock.advance_ms(ms);
        for c in &self.clocks {
            c.advance_ms(ms);
        }
    }

    fn sync_dev(&mut self, d: usize) -> mosaic_sync::engine::SyncReport {
        let clock = self.clocks[d].clone();
        sync_with(&mut self.w, d, &clock)
    }

    /// A flaky pass or more, then calm passes until two in a row are idle.
    fn converge(&mut self, flaky: usize) {
        self.w.cloud.set_config(SimConfig::FLAKY);
        for _ in 0..flaky {
            self.w.cloud.round().unwrap();
            let mut order: Vec<usize> = (0..DEVICES).collect();
            self.rng.shuffle(&mut order);
            for d in order {
                self.sync_dev(d);
            }
            self.advance(10 * 60 * 1000);
        }
        self.w.cloud.set_config(SimConfig::CALM);
        let mut idle = 0;
        for _ in 0..40 {
            self.w.cloud.settle().unwrap();
            let mut order: Vec<usize> = (0..DEVICES).collect();
            self.rng.shuffle(&mut order);
            let mut all_idle = true;
            for d in order {
                all_idle &= is_idle(&self.sync_dev(d));
            }
            if all_idle {
                idle += 1;
                if idle == 2 {
                    return;
                }
            } else {
                idle = 0;
            }
        }
        panic!("no convergence after 40 passes");
    }

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

    fn created(&mut self, id: ItemId, vault: VaultId) {
        let next = self.order.len();
        self.order.entry(id).or_insert(next);
        self.records.insert(id, vault);
    }

    fn one_edit(&mut self, d: usize, log: &mut Vec<(usize, ItemId, Op)>) {
        let live = self.live(d);
        let roll = self.rng.below(100);
        self.counter += 1;
        let n = self.counter;
        if live.is_empty() || roll >= 92 {
            let vault = self.vaults[self.rng.below(2)];
            let password = format!("new-{d}-{n}");
            let id = add_login(&mut self.w, d, vault, &format!("new {n}"));
            self.w.devs[d]
                .s_mut()
                .update_item(vault, id, |e| {
                    e.set_secret(SecretField::Password, &password)
                })
                .unwrap();
            self.created(id, vault);
            log.push((d, id, Op::Password(password)));
            return;
        }
        let (vault, item) = live[self.rng.below(live.len())];
        let to = if vault == self.vaults[0] {
            self.vaults[1]
        } else {
            self.vaults[0]
        };
        let mut s = self.w.devs[d].s_mut();
        let op = match roll {
            0..40 => {
                let v = format!("pw-{d}-{n}");
                s.update_item(vault, item, |e| e.set_secret(SecretField::Password, &v))
                    .unwrap();
                Op::Password(v)
            }
            40..60 => {
                let t = format!("tag{d}x{n}");
                s.update_item(vault, item, |e| e.add_tag(&t)).unwrap();
                Op::Tag(t)
            }
            60..70 => {
                if s.item(vault, item).unwrap().trashed_at().is_some() {
                    s.restore_item(vault, item).unwrap();
                    Op::Restore
                } else {
                    s.trash_item(vault, item).unwrap();
                    Op::Trash
                }
            }
            70..80 => {
                s.purge_item(vault, item).unwrap();
                Op::Delete
            }
            _ => {
                let copy = s.move_item(vault, item, to).unwrap();
                drop(s);
                self.moved_from.insert(copy, item);
                self.created(copy, to);
                log.push((d, copy, Op::Copy));
                Op::Move(copy)
            }
        };
        log.push((d, item, op));
    }

    /// One device's offline edits for a round; `rotate` puts an offline
    /// key rotation of a random vault somewhere among them.
    fn offline_edits(&mut self, d: usize, rotate: bool, log: &mut Vec<(usize, ItemId, Op)>) {
        let n = 1 + self.rng.below(3);
        let rotate_at = rotate.then(|| self.rng.below(n + 1));
        for i in 0..=n {
            if rotate_at == Some(i) {
                let vault = self.vaults[self.rng.below(2)];
                self.w.devs[d].s_mut().rotate_vault_key(vault).unwrap();
            }
            if i < n {
                self.one_edit(d, log);
            }
        }
    }

    fn root(&self, mut id: ItemId) -> ItemId {
        while let Some(source) = self.moved_from.get(&id) {
            id = *source;
        }
        id
    }

    /// The devices agree on every record and nothing is queued or warned.
    fn assert_converged(&self) -> BTreeMap<ItemId, bool> {
        let mut state = BTreeMap::new();
        let reference = self.device(0);
        for (id, vault) in &self.records {
            let x = reference.item(*vault, *id).unwrap();
            for d in 1..DEVICES {
                let other = self.device(d);
                let y = other.item(*vault, *id).unwrap();
                assert!(x == y, "devices 0 and {d} disagree on {id:?}");
            }
            state.insert(*id, x.is_deleted());
        }
        drop(reference);
        let current_keys = |s: &UnlockedSession| -> Vec<(u32, Vec<u8>)> {
            self.vaults
                .iter()
                .map(|v| {
                    let keys = s.store().vault_keys(v).unwrap();
                    keys.iter()
                        .max_by_key(|(g, _)| *g)
                        .map(|(g, k)| (g.get(), k.as_bytes().to_vec()))
                        .unwrap()
                })
                .collect()
        };
        let (reference_keys, reference_items) = {
            let reference = self.device(0);
            (
                current_keys(&reference),
                reference.items(&everything()).len(),
            )
        };
        for d in 0..DEVICES {
            let s = self.device(d);
            assert_eq!(
                current_keys(&s),
                reference_keys,
                "device {d} holds another current key"
            );
            assert_eq!(s.pending_uploads(1).unwrap().len(), 0, "device {d} outbox");
            assert_eq!(s.items(&everything()).len(), reference_items);
            assert!(
                self.w.devs[d].last.warnings.is_empty(),
                "device {d} warns: {:?}",
                self.w.devs[d].last.warnings
            );
        }
        state
    }

    fn check_round(&self, ops: &[(usize, ItemId, Op)], state: &BTreeMap<ItemId, bool>) {
        let mut by_item: BTreeMap<ItemId, Vec<(usize, &Op)>> = BTreeMap::new();
        for (d, item, op) in ops {
            by_item.entry(*item).or_default().push((*d, op));
        }
        let mut deleted_chain: BTreeSet<(usize, ItemId)> = BTreeSet::new();
        for (d, item, op) in ops {
            if *op == Op::Delete {
                deleted_chain.insert((*d, self.root(*item)));
            }
        }
        for (item, item_ops) in &by_item {
            let deleted = state[item];
            let deleters: BTreeSet<usize> = item_ops
                .iter()
                .filter(|(_, op)| matches!(op, Op::Delete | Op::Move(_)))
                .map(|(d, _)| *d)
                .collect();
            let concurrent_write = item_ops.iter().enumerate().any(|(i, (d, op))| {
                !deleters.contains(d)
                    && op.writes()
                    && !(**op == Op::Trash
                        && item_ops[i + 1..]
                            .iter()
                            .any(|(ld, lop)| ld == d && **lop == Op::Restore))
            });
            if deleters.is_empty() {
                assert!(!deleted, "{item:?} vanished without a delete: {item_ops:?}");
            } else if concurrent_write {
                assert!(!deleted, "a delete beat a concurrent write: {item_ops:?}");
            } else {
                assert!(
                    deleted,
                    "a delete without a concurrent write was lost: {item_ops:?}"
                );
            }
        }
        for (d, item, op) in ops {
            if deleted_chain.contains(&(*d, self.root(*item))) {
                continue;
            }
            let reference = self.device(0);
            let rows = reference.items(&everything());
            match op {
                Op::Password(v) => assert!(
                    rows.iter().any(|r| has_password(r.item, v)),
                    "password {v} from device {d} lost"
                ),
                Op::Tag(t) => assert!(
                    rows.iter().any(|r| r.item.tags().contains(&t.as_str())),
                    "tag {t} from device {d} lost"
                ),
                _ => {}
            }
        }
    }

    /// The outcome by creation order: deleted, every password value, tags.
    fn outcome(&self) -> Outcome {
        let mut out = Outcome::new();
        for (id, vault) in &self.records {
            let session = self.device(0);
            let item = session.item(*vault, *id).unwrap();
            let mut tags: Vec<String> = item.tags().iter().map(|t| (*t).to_owned()).collect();
            tags.sort();
            out.insert(
                self.order[id],
                (item.is_deleted(), password_values(item), tags),
            );
        }
        out
    }
}

/// Runs `rounds` rounds of offline edits by all three devices, each round
/// with the clocks a few hours further on; returns the outcome.
fn run(seed: u64, rounds: usize, skews_ms: [i64; DEVICES]) -> Outcome {
    let mut r = Run::new(seed, skews_ms);
    r.assert_converged();
    for round in 0..rounds {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut ops = Vec::new();
            // At most one offline key rotation per round, somewhere among
            // the edits of one device.
            let rotator = (r.rng.below(3) == 0).then(|| r.rng.below(DEVICES));
            for d in 0..DEVICES {
                r.offline_edits(d, rotator == Some(d), &mut ops);
            }
            r.converge(1);
            let state = r.assert_converged();
            r.check_round(&ops, &state);
            r.advance(5 * 3_600_000);
        }));
        if let Err(cause) = result {
            let message = cause
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| cause.downcast_ref::<&str>().copied())
                .unwrap_or("(no message)");
            panic!("seed {seed}, skews {skews_ms:?}, round {round}: {message}");
        }
    }
    r.outcome()
}

/// Three days, in ms.
const SKEW: i64 = 3 * 24 * 3_600_000;

fn skewed_runs_agree(seed: u64, rounds: usize) {
    let one = run(seed, rounds, [-SKEW, 0, SKEW]);
    let two = run(seed, rounds, [SKEW, 0, -SKEW]);
    assert_eq!(
        one, two,
        "seed {seed}: the outcome depends on the clocks or the delivery order"
    );
}

#[test]
fn skewed_clocks_do_not_change_the_outcome() {
    skewed_runs_agree(501, 3);
}

/// Long version: more seeds and rounds.
#[test]
#[ignore = "long-running chaos sim; run with --release -- --ignored"]
fn skewed_clocks_do_not_change_the_outcome_many_seeds() {
    for seed in 1..=12 {
        skewed_runs_agree(seed * 104_729, 6);
    }
}
