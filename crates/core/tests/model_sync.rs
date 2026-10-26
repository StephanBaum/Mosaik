//! Property tests for the sync merge (brief §5 "Sync", CRYPTO_SPEC §10):
//! random concurrent edit histories across four devices, with random
//! syncs between them.
//!
//! Checked on every history:
//! - the merge is commutative, associative and idempotent on every state
//!   the history reached (and on unrelated items);
//! - all replicas converge to the same state whatever the merge order;
//! - nothing is silently lost: an entry disappears from a merge result
//!   only if some device removed it by a local edit, and every value that
//!   was written and never removed is in the converged state;
//! - a replica's later states dominate its earlier ones (rollback check);
//! - plaintext and tombstone encodings round-trip every reached state.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test helpers outside #[test] functions"
)]

use std::collections::BTreeSet;

use mosaic_core::DeviceId;
use mosaic_core::model::{
    Causality, Item, ItemType, LoginUrl, SecretField, Stamp, TextField, Timestamp,
};
use proptest::prelude::*;

const DEVICES: usize = 4;

fn dev(i: usize) -> DeviceId {
    DeviceId::from_bytes([u8::try_from(i).unwrap() + 1; 16]).unwrap()
}

fn copy(item: &Item) -> Item {
    let back = Item::from_json(item.to_json().unwrap().expose_secret()).unwrap();
    assert_eq!(&back, item, "plaintext round trip");
    if let Some(t) = item.to_tombstone() {
        let t2 =
            mosaic_core::model::Tombstone::from_json(t.to_json().unwrap().expose_secret()).unwrap();
        assert_eq!(&Item::from(t2), item, "tombstone round trip");
    }
    back
}

#[derive(Clone, Debug)]
enum Op {
    Title(usize, u8),
    Password(usize, u8),
    Username(usize, u8),
    ClearUsername(usize),
    AddTag(usize, u8),
    RemoveTag(usize, u8),
    Favorite(usize, bool),
    Trash(usize),
    Restore(usize),
    AddUrl(usize, u8),
    RemoveFirstUrl(usize),
    Counter(usize, u8),
    Delete(usize),
    DismissDeletion(usize),
    Resolve(usize, bool),
    Sync(usize, usize),
}

fn op() -> impl Strategy<Value = Op> {
    let d = 0..DEVICES;
    let v = 0u8..4;
    prop_oneof![
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::Title(d, v)),
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::Password(d, v)),
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::Username(d, v)),
        d.clone().prop_map(Op::ClearUsername),
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::AddTag(d, v)),
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::RemoveTag(d, v)),
        (d.clone(), any::<bool>()).prop_map(|(d, b)| Op::Favorite(d, b)),
        d.clone().prop_map(Op::Trash),
        d.clone().prop_map(Op::Restore),
        (d.clone(), v.clone()).prop_map(|(d, v)| Op::AddUrl(d, v)),
        d.clone().prop_map(Op::RemoveFirstUrl),
        (d.clone(), v).prop_map(|(d, v)| Op::Counter(d, v)),
        d.clone().prop_map(Op::Delete),
        d.clone().prop_map(Op::DismissDeletion),
        (d.clone(), any::<bool>()).prop_map(|(d, b)| Op::Resolve(d, b)),
        // Syncs are frequent so histories mix concurrency and causality.
        (d.clone(), d.clone()).prop_map(|(a, b)| Op::Sync(a, b)),
        (d.clone(), d).prop_map(|(a, b)| Op::Sync(a, b)),
    ]
}

type EntrySet = BTreeSet<(String, Stamp)>;

fn entries(item: &Item) -> EntrySet {
    item.keys()
        .flat_map(|k| item.entries(k).map(move |e| (k.to_owned(), e.stamp())))
        .collect()
}

fn apply(item: &mut Item, op: &Op, clock: i64) {
    let at = Timestamp::from_unix_millis(clock);
    let d = match op {
        Op::Title(d, _)
        | Op::Password(d, _)
        | Op::Username(d, _)
        | Op::ClearUsername(d)
        | Op::AddTag(d, _)
        | Op::RemoveTag(d, _)
        | Op::Favorite(d, _)
        | Op::Trash(d)
        | Op::Restore(d)
        | Op::AddUrl(d, _)
        | Op::RemoveFirstUrl(d)
        | Op::Counter(d, _)
        | Op::Delete(d)
        | Op::DismissDeletion(d)
        | Op::Resolve(d, _) => dev(*d),
        Op::Sync(..) => unreachable!(),
    };
    if let Op::Delete(_) = op {
        item.delete(d, at).unwrap();
        return;
    }
    let first_conflict = item
        .conflicts()
        .first()
        .map(|c| (c.key.to_owned(), c.winner.stamp(), c.losers[0].stamp()));
    let first_url = item.urls().first().map(|(id, _)| id.clone());
    let mut e = item.edit(d, at);
    match op {
        Op::Title(_, v) => e.set_title(&format!("title {v}")).unwrap(),
        Op::Password(_, v) => e
            .set_secret(SecretField::Password, &format!("pw {v}"))
            .unwrap(),
        Op::Username(_, v) => e
            .set_text(TextField::Username, &format!("user {v}"))
            .unwrap(),
        Op::ClearUsername(_) => e.clear_text(TextField::Username).unwrap(),
        Op::AddTag(_, v) => e.add_tag(&format!("tag {v}")).unwrap(),
        Op::RemoveTag(_, v) => e.remove_tag(&format!("tag {v}")).unwrap(),
        Op::Favorite(_, b) => e.set_favorite(*b).unwrap(),
        Op::Trash(_) => e.trash().unwrap(),
        Op::Restore(_) => e.restore_from_trash().unwrap(),
        Op::AddUrl(_, v) => {
            e.add_url(&LoginUrl::new(&format!("https://{v}.example")))
                .unwrap();
        }
        Op::RemoveFirstUrl(_) => {
            if let Some(id) = first_url {
                e.remove_url(&id).unwrap();
            }
        }
        Op::Counter(_, v) => e.set_otp_counter(u64::from(*v)).unwrap(),
        Op::DismissDeletion(_) => e.dismiss_deletion().unwrap(),
        Op::Resolve(_, pick_loser) => {
            if let Some((key, winner, loser)) = first_conflict {
                let keep = if *pick_loser { loser } else { winner };
                e.resolve_conflict(&key, keep).unwrap();
            }
        }
        Op::Delete(_) | Op::Sync(..) => unreachable!(),
    }
}

fn check_laws(a: &Item, b: &Item, c: &Item) {
    let ab = a.merge(b);
    assert_eq!(ab, b.merge(a), "commutative");
    assert_eq!(a.merge(a), *a, "idempotent");
    assert_eq!(ab.merge(&ab), ab, "idempotent on results");
    assert_eq!(ab.merge(c), a.merge(&b.merge(c)), "associative");
    // The merge is an upper bound in the causal order.
    assert!(matches!(
        ab.compare(a),
        Causality::Dominates | Causality::Equal
    ));
}

struct Run {
    replicas: Vec<Item>,
    snapshots: Vec<Item>,
    written: EntrySet,
    removed: EntrySet,
}

fn run(ops: &[Op], separate_origin: bool) -> Run {
    let mut base = Item::new(&ItemType::Login, dev(0), Timestamp::from_unix_millis(0)).unwrap();
    base.edit(dev(0), Timestamp::from_unix_millis(0))
        .set_title("base")
        .unwrap();
    let mut replicas: Vec<Item> = (0..DEVICES).map(|_| copy(&base)).collect();
    if separate_origin {
        // An unrelated item on the last device: the laws must hold anyway.
        replicas[DEVICES - 1] = Item::new(
            &ItemType::SecureNote,
            dev(DEVICES - 1),
            Timestamp::from_unix_millis(1),
        )
        .unwrap();
    }
    let mut written: EntrySet = replicas.iter().flat_map(entries).collect();
    let mut removed = EntrySet::new();
    let mut snapshots: Vec<Item> = replicas.iter().map(copy).collect();
    for (clock, op) in (1i64..).zip(ops) {
        // Wall clocks run backwards on purpose: merges must not use them.
        let wall = 1_000_000 - clock;
        let touched = if let Op::Sync(from, to) = op {
            let before: EntrySet = entries(&replicas[*to])
                .union(&entries(&replicas[*from]))
                .cloned()
                .collect();
            let merged = replicas[*to].merge(&replicas[*from]);
            let after = entries(&merged);
            assert!(after.is_subset(&before), "a merge invents no entries");
            for lost in before.difference(&after) {
                assert!(
                    removed.contains(lost),
                    "merge dropped {lost:?}, which no device removed"
                );
            }
            replicas[*to] = merged;
            *to
        } else {
            let i = match op {
                Op::Title(d, _)
                | Op::Password(d, _)
                | Op::Username(d, _)
                | Op::ClearUsername(d)
                | Op::AddTag(d, _)
                | Op::RemoveTag(d, _)
                | Op::Favorite(d, _)
                | Op::Trash(d)
                | Op::Restore(d)
                | Op::AddUrl(d, _)
                | Op::RemoveFirstUrl(d)
                | Op::Counter(d, _)
                | Op::Delete(d)
                | Op::DismissDeletion(d)
                | Op::Resolve(d, _) => *d,
                Op::Sync(..) => unreachable!(),
            };
            let before_state = copy(&replicas[i]);
            let before = entries(&replicas[i]);
            apply(&mut replicas[i], op, wall);
            let after = entries(&replicas[i]);
            written.extend(after.difference(&before).cloned());
            removed.extend(before.difference(&after).cloned());
            let rel = replicas[i].compare(&before_state);
            assert!(
                matches!(rel, Causality::Dominates | Causality::Equal),
                "a local edit moves forward: {rel:?}"
            );
            assert_eq!(
                rel == Causality::Equal,
                before == after && replicas[i] == before_state
            );
            i
        };
        snapshots.push(copy(&replicas[touched]));
    }
    Run {
        replicas,
        snapshots,
        written,
        removed,
    }
}

fn converge(replicas: &[Item], order: &[usize]) -> Item {
    let mut acc = copy(&replicas[order[0]]);
    for &i in &order[1..] {
        acc = acc.merge(&replicas[i]);
    }
    acc
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    #[test]
    fn histories_converge_and_lose_nothing(
        ops in proptest::collection::vec(op(), 0..48),
        separate_origin in proptest::bool::weighted(0.1),
        triples in proptest::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>(), any::<prop::sample::Index>()), 1..24),
    ) {
        let r = run(&ops, separate_origin);

        // Laws on reached states.
        for (i, j, k) in &triples {
            let a = &r.snapshots[i.index(r.snapshots.len())];
            let b = &r.snapshots[j.index(r.snapshots.len())];
            let c = &r.snapshots[k.index(r.snapshots.len())];
            check_laws(a, b, c);
        }
        for w in r.snapshots.windows(2) {
            check_laws(&w[0], &w[1], &r.replicas[0]);
        }

        // Convergence in different orders.
        let forward: Vec<usize> = (0..DEVICES).collect();
        let backward: Vec<usize> = (0..DEVICES).rev().collect();
        let mixed = [2, 0, 3, 1];
        let f = converge(&r.replicas, &forward);
        prop_assert_eq!(&f, &converge(&r.replicas, &backward));
        prop_assert_eq!(&f, &converge(&r.replicas, &mixed));
        // Pairwise gossip until stable reaches the same state.
        let mut gossip: Vec<Item> = r.replicas.iter().map(copy).collect();
        for _ in 0..2 {
            for a in 0..DEVICES {
                for b in 0..DEVICES {
                    gossip[a] = gossip[a].merge(&gossip[b]);
                }
            }
        }
        for g in &gossip {
            prop_assert_eq!(g, &f);
        }

        // Nothing silently lost: every written, never removed entry is in
        // the converged state, and nothing else is.
        let fin = entries(&f);
        for e in r.written.difference(&r.removed) {
            prop_assert!(fin.contains(e), "lost {:?}", e);
        }
        prop_assert!(fin.is_subset(&r.written));
        // ... and a removal anywhere is final everywhere (it is never
        // undone by merging an older replica that still holds the value).
        prop_assert!(fin.is_disjoint(&r.removed));

        // The converged state dominates every snapshot.
        for s in &r.snapshots {
            prop_assert!(matches!(f.compare(s), Causality::Dominates | Causality::Equal));
            prop_assert_eq!(&f.merge(s), &f);
        }

        // Every conflict surfaced has a loser with a distinct value, and a
        // deletion entry next to live fields is the concurrent-delete flag.
        for c in f.conflicts() {
            prop_assert!(!c.losers.is_empty());
            for l in &c.losers {
                prop_assert!(l.value() != c.winner.value());
            }
        }
        let has_deleted = f.keys().any(|k| k == "deleted");
        prop_assert_eq!(has_deleted && f.keys().count() > 1, f.deleted_concurrently().is_some());
        prop_assert_eq!(has_deleted && f.keys().count() == 1, f.is_deleted());
    }
}
