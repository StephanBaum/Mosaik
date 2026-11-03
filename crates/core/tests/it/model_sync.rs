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
//!
//! Half of the histories sync with `Item::merge_and_repair`, as stores do.
//! There, in addition: a replica that receives a delete it had not seen,
//! racing a concurrent edit, keeps every field it held; the repair only
//! adds fresh entries of the receiving device; and without a concurrent
//! delete the result is exactly the pure merge.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::panic,
    clippy::similar_names,
    reason = "test helpers outside #[test] functions"
)]

use std::collections::BTreeSet;

use mosaic_core::model::{
    AttachmentRef, Causality, Item, ItemType, LoginUrl, MAX_HISTORY, SecretField, Stamp, Text,
    TextField, Timestamp,
};
use mosaic_core::{AttachmentId, AttachmentKey, DeviceId, ItemId, KeyGeneration, VaultKey};
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
    if item.is_deleted() {
        // A deleted item takes no edits; the user undeletes it first.
        assert_eq!(
            item.edit(d, at).set_title("refused").unwrap_err(),
            mosaic_core::Error::ItemDeleted
        );
        if !matches!(op, Op::DismissDeletion(_)) {
            item.undelete(&ItemType::Login, d, at).unwrap();
        }
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

/// The repairing merge of a store on device `to`, with its checks.
fn repairing_sync(local: &Item, received: &Item, to: usize) -> Item {
    let plain = local.merge(received);
    let out = local.merge_and_repair(received, dev(to)).unwrap();
    assert!(!out.stamp_collision, "honest histories never collide");
    let missed_deletion = plain.deleted_concurrently().is_some()
        && plain
            .entries("deleted")
            .any(|e| !local.version().covers(&e.stamp()));
    if missed_deletion {
        for k in local.keys().filter(|k| *k != "deleted") {
            assert!(out.item.keys().any(|r| r == k), "repair kept {k}");
            // Every value of a field the deletion removed is still there,
            // conflict copies included (review M2 c1).
            if received.entries(k).next().is_none() {
                for e in local.entries(k) {
                    assert!(
                        out.item.entries(k).any(|o| o.value() == e.value()),
                        "repair lost a value of {k}"
                    );
                }
                // ... and the value this replica showed is still shown.
                assert_eq!(
                    out.item
                        .entries(k)
                        .max_by_key(|e| e.stamp())
                        .map(mosaic_core::model::Entry::value),
                    local
                        .entries(k)
                        .max_by_key(|e| e.stamp())
                        .map(mosaic_core::model::Entry::value),
                    "repair changed the shown value of {k}"
                );
            }
        }
    } else {
        assert_eq!(out.restored_fields, 0);
        assert_eq!(out.item, plain, "no repair: exactly the pure merge");
    }
    // The repair adds only fresh entries of this device.
    let added: EntrySet = entries(&out.item)
        .difference(&entries(&plain))
        .cloned()
        .collect();
    for (_, s) in &added {
        assert_eq!(s.device(), dev(to));
        assert!(!plain.version().covers(s));
    }
    assert!(entries(&plain).is_subset(&entries(&out.item)));
    out.item
}

fn run(ops: &[Op], separate_origin: bool, repair: bool) -> Run {
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
            let merged = if repair {
                repairing_sync(&replicas[*to], &replicas[*from], *to)
            } else {
                replicas[*to].merge(&replicas[*from])
            };
            let after = entries(&merged);
            let fresh: EntrySet = after.difference(&before).cloned().collect();
            assert!(
                fresh.iter().all(|(_, s)| repair && s.device() == dev(*to)),
                "a merge invents no entries; a repair adds only its own"
            );
            written.extend(fresh);
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
        repair in any::<bool>(),
        triples in proptest::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>(), any::<prop::sample::Index>()), 1..24),
    ) {
        let r = run(&ops, separate_origin, repair);

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

    /// A delete races a burst of edits on another device, and the three
    /// devices sync in a random order with the repairing merge: all
    /// devices converge, and if the burst wrote anything, the item stays
    /// (flagged) with every field the editing device had. A burst that
    /// only removed fields does not conflict with the delete.
    #[test]
    fn delete_racing_edits_keeps_the_editors_fields(
        edits in proptest::collection::vec(edit_op(), 1..8),
        syncs in proptest::collection::vec((0..3usize, 0..3usize), 0..12),
    ) {
        let mut base = Item::new(&ItemType::Login, dev(0), Timestamp::from_unix_millis(0)).unwrap();
        {
            let mut e = base.edit(dev(0), Timestamp::from_unix_millis(0));
            e.set_title("base").unwrap();
            e.set_text(TextField::Username, "user").unwrap();
            e.set_secret(SecretField::Password, "pw").unwrap();
            e.add_url(&LoginUrl::new("https://base.example")).unwrap();
            e.add_tag("tag").unwrap();
        }
        // Device 0 deletes, device 1 edits, device 2 only relays.
        let mut replicas: Vec<Item> = (0..3).map(|_| copy(&base)).collect();
        replicas[0].delete(dev(0), Timestamp::from_unix_millis(1)).unwrap();
        for (clock, op) in (2i64..).zip(&edits) {
            apply(&mut replicas[1], op, clock);
        }
        let editor_keys: Vec<String> = replicas[1].keys().map(str::to_owned).collect();
        let raced = !replicas[1].merge(&replicas[0]).is_deleted();
        for (from, to) in syncs {
            replicas[to] = repairing_sync(&replicas[to], &replicas[from], to);
        }
        for _ in 0..2 {
            for to in 0..3 {
                for from in 0..3 {
                    replicas[to] = repairing_sync(&replicas[to], &replicas[from], to);
                }
            }
        }
        for r in &replicas {
            prop_assert_eq!(r, &replicas[0], "converged");
        }
        let fin = &replicas[0];
        if !raced {
            prop_assert!(fin.is_deleted());
            return Ok(());
        }
        prop_assert!(fin.deleted_concurrently().is_some());
        for k in &editor_keys {
            prop_assert!(fin.keys().any(|f| f == k), "editor lost {}", k);
        }
        prop_assert_eq!(fin.item_type(), Some(ItemType::Login));
    }
}

fn edit_op() -> impl Strategy<Value = Op> {
    let v = 0u8..4;
    prop_oneof![
        v.clone().prop_map(|v| Op::Title(1, v)),
        v.clone().prop_map(|v| Op::Password(1, v)),
        v.clone().prop_map(|v| Op::Username(1, v)),
        Just(Op::ClearUsername(1)),
        v.clone().prop_map(|v| Op::AddTag(1, v)),
        v.clone().prop_map(|v| Op::RemoveTag(1, v)),
        v.prop_map(|v| Op::AddUrl(1, v)),
        Just(Op::RemoveFirstUrl(1)),
        Just(Op::Favorite(1, true)),
        Just(Op::Trash(1)),
    ]
}

/// Review M2 (sync, c1): a delete raced an edit of a field that held
/// conflict copies. The editor's write superseded the shown value and kept
/// the other copy on purpose; the field survives the merge through that
/// write, but the deletion's vector dropped the copy. The repair restores
/// every entry the deletion removed, not only whole fields.
#[test]
fn c1_conflict_copy_lost_in_delete_race() {
    let at = Timestamp::from_unix_millis;
    let mut base = Item::new(&ItemType::Login, dev(0), at(0)).unwrap();
    base.edit(dev(0), at(0)).set_title("base").unwrap();
    let (mut x, mut y) = (copy(&base), copy(&base));
    x.edit(dev(5), at(1)).set_title("from-x").unwrap();
    y.edit(dev(6), at(1)).set_title("from-y").unwrap();
    let both = x.merge(&y);
    assert_eq!(both.conflicts().len(), 1);
    // Device 7 saw both values and deletes.
    let mut deleter = copy(&both);
    deleter.delete(dev(7), at(2)).unwrap();
    // Device 5 saw both too and sets the title; the shown value is
    // superseded, the other copy stays.
    let mut editor = copy(&both);
    editor.edit(dev(5), at(3)).set_title("edited").unwrap();
    let held: Vec<String> = titles(&editor);
    assert_eq!(held.len(), 2, "the edit kept a conflict copy");

    let out = editor.merge_and_repair(&deleter, dev(5)).unwrap();
    assert!(out.item.deleted_concurrently().is_some());
    assert_eq!(titles(&out.item), held, "no conflict copy is lost");
    assert_eq!(
        out.restored_fields, 3,
        "type, created_at and the title copy"
    );
    assert_eq!(out.item.title(), Some("edited"));
    assert_eq!(out.item.conflicts().len(), 1);
    // The deleting device converges to the same state.
    let back = deleter.merge_and_repair(&out.item, dev(7)).unwrap();
    assert_eq!(back.item, out.item);
}

/// All distinct values of `key`, sorted (shown value and conflict copies).
fn values(item: &Item, key: &str) -> Vec<String> {
    let mut out: Vec<String> = item
        .entries(key)
        .filter_map(|e| e.value().as_str().map(str::to_owned))
        .collect();
    out.sort();
    out.dedup();
    out
}

fn password(item: &Item) -> Option<String> {
    item.secret(SecretField::Password)
        .unwrap()
        .map(|p| p.expose_secret().to_owned())
}

/// Review M2, third round (sync, high): two writers with one device ID
/// (a restored backup or a copied profile) each made one edit from the same
/// state, in different fields. Both edits get the stamp (X, 3), the vectors
/// are equal, and each side's vector covers the other's stamp, so the pure
/// merge drops the title and the password of both sides. Equal vectors
/// with different contents are a certain collision: every value either
/// side held comes back, on a third device and on the writers themselves.
#[test]
#[allow(clippy::many_single_char_names, reason = "writers and devices")]
fn reused_device_id_edits_of_different_fields_lose_nothing() {
    let at = Timestamp::from_unix_millis;
    let x = dev(0);
    let mut base = Item::new(&ItemType::Login, x, at(0)).unwrap();
    base.edit(x, at(1)).set_title("t0").unwrap();
    base.edit(x, at(2))
        .set_secret(SecretField::Password, "p0")
        .unwrap();
    let (mut w1, mut w2) = (copy(&base), copy(&base));
    w1.edit(x, at(10))
        .set_secret(SecretField::Password, "p1")
        .unwrap();
    w2.edit(x, at(20)).set_title("t1").unwrap();
    assert_eq!(w1.compare(&w2), Causality::Equal);
    // The pure merge loses both edits and both older values.
    let pure = w1.merge(&w2);
    assert_eq!(pure.title(), None);
    assert_eq!(password(&pure), None);

    // A third device that already holds W1's version.
    let y = dev(1);
    let third = copy(&base).merge_and_repair(&w1, y).unwrap().item;
    assert_eq!(password(&third).as_deref(), Some("p1"));
    let out = third.merge_and_repair(&w2, y).unwrap();
    assert!(out.stamp_collision, "the collision is reported");
    let item = out.item;
    assert_eq!(values(&item, "title"), ["t0", "t1"]);
    assert_eq!(values(&item, "login.password"), ["p0", "p1"]);
    // The newest write by the wall clock is shown, the other is a
    // conflict copy (shown to the user).
    assert_eq!(item.title(), Some("t1"));
    assert_eq!(password(&item).as_deref(), Some("p1"));
    assert_eq!(item.conflicts().len(), 2);

    // Either writer sees the reuse of its own ID, and after rotating it
    // keeps both values as well.
    assert_eq!(
        w1.merge_and_repair(&w2, x).unwrap_err(),
        mosaic_core::Error::DeviceIdReused
    );
    let z = dev(2);
    let rotated = w1.merge_and_repair_after_reuse(&w2, z, &[x]).unwrap();
    assert!(rotated.stamp_collision);
    assert_eq!(values(&rotated.item, "title"), ["t0", "t1"]);
    assert_eq!(values(&rotated.item, "login.password"), ["p0", "p1"]);

    // Every replica converges once the repaired versions are exchanged.
    let a = item.merge_and_repair(&rotated.item, y).unwrap().item;
    let b = rotated.item.merge_and_repair(&item, z).unwrap().item;
    assert_eq!(a.merge(&b), b.merge(&a));
    assert_eq!(values(&a, "title"), ["t0", "t1"]);
    assert_eq!(values(&b, "login.password"), ["p0", "p1"]);
}

/// Review M2, third round (sync, high), restored-backup variant: the old
/// install edits the password (X, 3) and then the user name (X, 4); the
/// restored install edits the title offline (X, 3). The restored install
/// detects the reuse (a counter for its own ID above its state), rotates,
/// and merges again with the old ID as reused: no value of either writer
/// is lost, and the newest password stays the one shown.
#[test]
fn restored_backup_with_a_reused_id_loses_nothing_after_rotation() {
    let at = Timestamp::from_unix_millis;
    let x = dev(0);
    let mut base = Item::new(&ItemType::Login, x, at(0)).unwrap();
    base.edit(x, at(1)).set_title("t0").unwrap();
    base.edit(x, at(2))
        .set_secret(SecretField::Password, "p0")
        .unwrap();
    let (mut old, mut restored) = (copy(&base), copy(&base));
    old.edit(x, at(10))
        .set_secret(SecretField::Password, "p1")
        .unwrap();
    old.edit(x, at(11))
        .set_text(TextField::Username, "u1")
        .unwrap();
    restored.edit(x, at(20)).set_title("t1").unwrap();

    assert_eq!(
        restored.merge_and_repair(&old, x).unwrap_err(),
        mosaic_core::Error::DeviceIdReused
    );
    // The documented recovery used to lose the title and the password:
    // the plain repairing merge under the new ID.
    let y = dev(1);
    let plain = restored.merge_and_repair(&old, y).unwrap().item;
    assert_eq!(plain.title(), None);
    assert_eq!(password(&plain), None);

    let out = restored
        .merge_and_repair_after_reuse(&old, y, &[x])
        .unwrap();
    assert!(out.stamp_collision);
    let item = out.item;
    assert_eq!(values(&item, "title"), ["t0", "t1"]);
    assert_eq!(values(&item, "login.password"), ["p0", "p1"]);
    assert_eq!(item.text(TextField::Username), Some("u1"));
    assert_eq!(item.title(), Some("t1"));
    assert_eq!(password(&item).as_deref(), Some("p1"));
    // The result dominates both writers' versions, so they converge on it.
    assert_eq!(item.compare(&old), Causality::Dominates);
    assert_eq!(item.compare(&restored), Causality::Dominates);
    let at_old = old.merge_and_repair(&item, dev(3)).unwrap().item;
    assert_eq!(at_old, item);
    // A reused ID that does not collide in a record changes nothing.
    let unrelated = copy(&base);
    let mut newer = copy(&base);
    newer.edit(dev(4), at(30)).set_title("t2").unwrap();
    let plain = unrelated.merge_and_repair(&newer, y).unwrap();
    let after = unrelated
        .merge_and_repair_after_reuse(&newer, y, &[x])
        .unwrap();
    assert_eq!(after.item, plain.item);
    assert!(!after.stamp_collision);
}

fn vault_key() -> VaultKey {
    VaultKey::generate(KeyGeneration::FIRST).unwrap()
}

fn attachment(name: &str) -> AttachmentRef {
    AttachmentRef::new(
        AttachmentId::new_v7().unwrap(),
        Text::new(name),
        Text::new("text/plain"),
        4,
        AttachmentKey::generate().unwrap(),
    )
}

fn attachment_ids(item: &Item) -> BTreeSet<AttachmentId> {
    item.attachments().iter().map(|a| a.attachment_id).collect()
}

fn has_attachment_key(item: &Item) -> bool {
    item.keys().any(|k| k.starts_with("attachment:"))
}

fn fits(item: &Item) -> bool {
    match item.to_json() {
        Ok(_) => true,
        Err(mosaic_core::Error::SizeLimit) => false,
        Err(e) => panic!("{e:?}"),
    }
}

/// The largest `n` in `lo..=hi` for which `make(n)` fits the item limit.
fn largest_fitting(lo: usize, hi: usize, make: impl Fn(usize) -> Item) -> usize {
    assert!(fits(&make(lo)) && !fits(&make(hi)));
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fits(&make(mid)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The stamp of the entry of `key` that holds `value`.
fn stamp_of(item: &Item, key: &str, value: &str) -> Stamp {
    item.entries(key)
        .find(|e| e.value().as_str() == Some(value))
        .unwrap()
        .stamp()
}

/// Every value of `key` in the record and in its conflict copies.
fn values_with_copies(record: &Item, copies: &[(ItemId, Item)], key: &str) -> Vec<String> {
    let mut out = values(record, key);
    for (_, c) in copies {
        out.extend(values(c, key));
    }
    out.sort();
    out
}

/// Review M2, third round (sync, medium): two concurrent edits of a large
/// note each fit the 1 MiB item limit, but their merge does not. The merge
/// itself succeeds; sealing it fails with `SizeLimit`, and without a rule
/// the devices never converge.
///
/// Review M2, fourth round (sync, high and medium; tests, medium): the
/// split is a pure function of the merge result, not of the pair of
/// versions or of which side is local. The record keeps the value each
/// field shows, with the attachment references of both sides; the other
/// value moves to a conflict copy whose ID and contents every device
/// derives the same way. Here each side also adds an attachment, and one
/// side removes a shared one.
#[test]
fn an_oversized_merge_is_split_deterministically() {
    let at = Timestamp::from_unix_millis;
    let vk = vault_key();
    let big = "n".repeat(600_000);
    let shared = attachment("shared.txt");
    let mut base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();
    {
        let mut e = base.edit(dev(0), at(0));
        e.set_title("Large").unwrap();
        e.set_text(TextField::Notes, &big).unwrap();
        e.set_attachment(&shared).unwrap();
    }
    let (on_a_att, on_b_att) = (attachment("a.txt"), attachment("b.txt"));
    let (note_a, note_b) = (format!("{big}a"), format!("{big}b"));
    let (mut a, mut b) = (copy(&base), copy(&base));
    {
        let mut e = a.edit(dev(1), at(1));
        e.set_text(TextField::Notes, &note_a).unwrap();
        e.set_attachment(&on_a_att).unwrap();
        e.remove_attachment(&shared.attachment_id).unwrap();
    }
    {
        let mut e = b.edit(dev(2), at(2));
        e.set_text(TextField::Notes, &note_b).unwrap();
        e.set_attachment(&on_b_att).unwrap();
    }
    assert!(fits(&a) && fits(&b));
    let merged = a.merge_and_repair(&b, dev(1)).unwrap().item;
    assert!(!fits(&merged), "the union of the two notes does not fit");

    let id = mosaic_core::ItemId::new_v7().unwrap();
    let on_a = a.split_oversized(&b, &id, &vk, dev(1), &[]).unwrap();
    let on_b = b.split_oversized(&a, &id, &vk, dev(2), &[]).unwrap();
    // A third device, merging the other way round, computes the same.
    let third = b.split_oversized(&a, &id, &vk, dev(3), &[]).unwrap();
    // Every device stores the same record and the same copies.
    for other in [&on_b, &third] {
        assert_eq!(on_a.item, other.item);
        assert_eq!(on_a.conflict_copies.len(), other.conflict_copies.len());
        for (x, y) in on_a.conflict_copies.iter().zip(&other.conflict_copies) {
            assert_eq!(x.0, y.0);
            assert_eq!(x.1, y.1);
        }
    }
    // The record dominates both inputs, fits, and shows the winner.
    assert_eq!(on_a.item.compare(&a), Causality::Dominates);
    assert_eq!(on_a.item.compare(&b), Causality::Dominates);
    assert!(fits(&on_a.item));
    assert_eq!(
        on_a.item.text(TextField::Notes),
        merged.text(TextField::Notes)
    );
    assert_eq!(on_a.item.title(), Some("Large"));
    assert!(on_a.item.conflicts().is_empty());
    // The attachment references of both sides are merged: both new ones,
    // and not the one A removed.
    assert_eq!(
        attachment_ids(&on_a.item),
        BTreeSet::from([on_a_att.attachment_id, on_b_att.attachment_id])
    );
    // The other note is the one conflict copy, under its derived ID.
    assert_eq!(on_a.conflict_copies.len(), 1);
    let (copy_id, c) = &on_a.conflict_copies[0];
    let lost = if on_a.item.text(TextField::Notes) == Some(note_a.as_str()) {
        &note_b
    } else {
        &note_a
    };
    let lost_stamp = stamp_of(&merged, "notes", lost);
    assert_eq!(
        *copy_id,
        Item::conflict_copy_id(&vk, &id, "notes", lost_stamp).unwrap()
    );
    assert_ne!(*copy_id, id);
    // The ID depends on the vault key, the record, the field and the value.
    assert_ne!(
        *copy_id,
        Item::conflict_copy_id(&vault_key(), &id, "notes", lost_stamp).unwrap()
    );
    let other_record = mosaic_core::ItemId::new_v7().unwrap();
    assert_ne!(
        *copy_id,
        Item::conflict_copy_id(&vk, &other_record, "notes", lost_stamp).unwrap()
    );
    assert_ne!(
        *copy_id,
        Item::conflict_copy_id(&vk, &id, "title", lost_stamp).unwrap()
    );
    assert_eq!(c.text(TextField::Notes), Some(lost.as_str()));
    // The copy is a function of the value alone: the value and the mark,
    // both under the value's stamp, and a vector of exactly that stamp.
    assert_eq!(c.item_type(), None, "shown with the original's type");
    assert_eq!(c.keys().collect::<Vec<_>>(), ["conflict_copy_of", "notes"]);
    assert_eq!(
        c.version().iter().collect::<Vec<_>>(),
        [(&lost_stamp.device(), lost_stamp.counter())]
    );
    assert_eq!(c.conflict_copy_of(), Some(id));
    assert!(!has_attachment_key(c), "the copy holds no attachment key");
    assert!(fits(c));
    assert!(c.conflicts().is_empty());
    assert_eq!(
        values_with_copies(&on_a.item, &on_a.conflict_copies, "notes"),
        [note_a, note_b]
    );
    let mut dismissed = copy(c);
    dismissed
        .edit(dev(5), at(5))
        .dismiss_conflict_copy()
        .unwrap();
    assert_eq!(dismissed.conflict_copy_of(), None);
}

/// Review M2, fourth round (sync, high): the split lost a committed edit
/// depending on delivery order. A kept its own (longer) version and made no
/// copy; B pulled A's split record first, which dominates B's version,
/// fast-forwarded to it and never split, and B's note was on no device.
/// Whichever device splits now moves the losing value to a copy, so B gets
/// it from the copy.
#[test]
fn a_split_record_delivered_first_to_the_losing_peer_loses_nothing() {
    let at = Timestamp::from_unix_millis;
    let vk = vault_key();
    let big = "n".repeat(600_000);
    let mut base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();
    base.edit(dev(0), at(0))
        .set_text(TextField::Notes, &big)
        .unwrap();
    let (mut a, mut b) = (copy(&base), copy(&base));
    let note_a = format!("{big}{}", "A".repeat(200));
    let note_b = format!("{big}b");
    a.edit(dev(1), at(1))
        .set_text(TextField::Notes, &note_a)
        .unwrap();
    b.edit(dev(2), at(2))
        .set_text(TextField::Notes, &note_b)
        .unwrap();
    let id = mosaic_core::ItemId::new_v7().unwrap();
    let on_a = a.split_oversized(&b, &id, &vk, dev(1), &[]).unwrap();
    // B receives the split record before A's own record: it dominates B's
    // version, so a store fast-forwards and never splits.
    assert_eq!(on_a.item.compare(&b), Causality::Dominates);
    assert_eq!(on_a.item.compare(&a), Causality::Dominates);
    assert_eq!(
        values_with_copies(&on_a.item, &on_a.conflict_copies, "notes"),
        [note_a, note_b]
    );
}

/// Review M2, fourth round (sync, medium): whether a split happened
/// depended on the repaired merge, which differs by device. B, which missed
/// a concurrent purge, split (its write-backs made the merge too large),
/// while A, the purging device, stored the plain merge; both results had
/// the same vector with different contents, A then got `DeviceIdReused`,
/// and the split record had dropped the deletion entry, so the delete was
/// undone without a notice. Schedule (n = 360 000): A and C write notes
/// concurrently and everyone syncs (two values in the register); B writes a
/// third, which keeps the other conflict copy; A purges concurrently.
#[test]
fn a_split_after_a_concurrent_delete_keeps_the_notice_and_converges() {
    let at = Timestamp::from_unix_millis;
    let vk = vault_key();
    let n = 360_000;
    let (a_dev, b_dev, c_dev) = (dev(1), dev(2), dev(3));
    let base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();
    let (mut on_a, mut on_c) = (copy(&base), copy(&base));
    on_a.edit(a_dev, at(1))
        .set_text(TextField::Notes, &"a".repeat(n))
        .unwrap();
    on_c.edit(c_dev, at(2))
        .set_text(TextField::Notes, &"c".repeat(n))
        .unwrap();
    let synced = on_a.merge(&on_c);
    assert_eq!(values(&synced, "notes").len(), 2);
    let mut on_b = copy(&synced);
    on_b.edit(b_dev, at(3))
        .set_text(TextField::Notes, &"b".repeat(n))
        .unwrap();
    assert_eq!(values(&on_b, "notes").len(), 2, "one conflict copy stays");
    let mut tombstone = copy(&synced);
    tombstone.delete(a_dev, at(4)).unwrap();

    // B applies the tombstone: the repaired merge does not fit.
    let repaired = on_b.merge_and_repair(&tombstone, b_dev).unwrap();
    assert!(!fits(&repaired.item));
    let item_id = mosaic_core::ItemId::new_v7().unwrap();
    let split_b = on_b
        .split_oversized(&tombstone, &item_id, &vk, b_dev, &[])
        .unwrap();
    // A applies B's edit: the plain merge fits (A saw its own deletion).
    let merged_a = tombstone.merge_and_repair(&on_b, a_dev).unwrap();
    assert!(fits(&merged_a.item));
    // B's record is its repaired merge reduced: it keeps the deletion
    // notice and the type, shows B's note, and dominates A's result.
    assert_eq!(split_b.item.compare(&merged_a.item), Causality::Dominates);
    assert!(split_b.item.deleted_concurrently().is_some());
    assert_eq!(split_b.item.item_type(), Some(ItemType::SecureNote));
    assert_eq!(
        split_b.item.text(TextField::Notes),
        Some("b".repeat(n).as_str())
    );
    // Every note B held is visible, the restored conflict copy in a copy.
    assert_eq!(
        values_with_copies(&split_b.item, &split_b.conflict_copies, "notes"),
        values(&on_b, "notes")
    );

    // Exchange: A fast-forwards to B's record (no `DeviceIdReused`), and
    // both show the deletion notice.
    assert_eq!(merged_a.item.compare(&split_b.item), Causality::DominatedBy);
    let a_final = merged_a
        .item
        .merge_and_repair(&split_b.item, a_dev)
        .unwrap();
    assert!(!a_final.stamp_collision);
    assert_eq!(a_final.item, split_b.item);
    assert!(a_final.item.deleted_concurrently().is_some());
}

/// Review M2, fourth round (tests, low; sync, low): the fallbacks of the
/// split.
/// 1. Two different large fields edited concurrently: no field has a
///    conflict, so the record moves the larger field out whole.
/// 2. A value at the limit: the mark would push its copy over, so the copy
///    goes unmarked.
/// 3. The attachment references of both sides alone exceed the limit: the
///    split fails with `SizeLimit` instead of dropping references whose
///    files the record's vector would then count as removed.
#[test]
fn split_fallbacks() {
    let at = Timestamp::from_unix_millis;
    let vk = vault_key();
    let id = mosaic_core::ItemId::new_v7().unwrap();
    let base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();

    // 1. Notes on A, the user name on B, 600 kB each.
    let (mut a, mut b) = (copy(&base), copy(&base));
    a.edit(dev(1), at(1))
        .set_text(TextField::Notes, &"a".repeat(600_000))
        .unwrap();
    b.edit(dev(2), at(1))
        .set_text(TextField::Username, &"b".repeat(600_001))
        .unwrap();
    let on_a = a.split_oversized(&b, &id, &vk, dev(1), &[]).unwrap();
    let on_b = b.split_oversized(&a, &id, &vk, dev(2), &[]).unwrap();
    assert_eq!(on_a.item, on_b.item);
    assert!(fits(&on_a.item));
    assert_eq!(on_a.item.compare(&a), Causality::Dominates);
    assert_eq!(on_a.item.compare(&b), Causality::Dominates);
    assert_eq!(on_a.item.text(TextField::Notes), a.text(TextField::Notes));
    assert_eq!(on_a.item.text(TextField::Username), None, "moved out");
    let [(copy_id, c)] = &on_a.conflict_copies[..] else {
        panic!("one copy");
    };
    assert_eq!(*copy_id, on_b.conflict_copies[0].0);
    assert_eq!(c.text(TextField::Username), b.text(TextField::Username));
    assert_eq!(c.conflict_copy_of(), Some(id));

    // 2. A's note at the limit loses against B's small note. A's version
    // holds nothing else (no type, as on a deleting device after a lost
    // race), so the mark would push the copy over the limit.
    let long_a = |n: usize| {
        let d = dev(1);
        let json = format!(
            r#"{{"schema_version":1,"vv":{{"{d}":2}},"fields":{{"notes":[{{"at":1,"c":2,"d":"{d}","v":"{}"}}]}}}}"#,
            "a".repeat(n)
        );
        Item::from_json(json.as_bytes()).ok()
    };
    // The largest such note that decodes (decoding and encoding share the
    // limit).
    let (mut lo, mut hi) = (600_000, 1_100_000);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if long_a(mid).is_some_and(|a| fits(&a)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let a = long_a(lo).unwrap();
    assert_eq!(a.keys().collect::<Vec<_>>(), ["notes"]);
    let mut b = copy(&base);
    b.edit(dev(2), at(1))
        .set_text(TextField::Notes, "small")
        .unwrap();
    let on_a = a.split_oversized(&b, &id, &vk, dev(1), &[]).unwrap();
    assert_eq!(on_a.item.text(TextField::Notes), Some("small"));
    let [(_, c)] = &on_a.conflict_copies[..] else {
        panic!("one copy");
    };
    assert_eq!(c.conflict_copy_of(), None, "the mark would not fit");
    assert!(fits(c));
    assert_eq!(c, &a, "the bare copy is A's version, entry for entry");

    // 3. 2400 attachment references on each side.
    let (mut a, mut b) = (copy(&base), copy(&base));
    for (x, d) in [(&mut a, 1), (&mut b, 2)] {
        let mut e = x.edit(dev(d), at(1));
        for i in 0..2400 {
            e.set_attachment(&attachment(&format!("{d}-{i}"))).unwrap();
        }
    }
    assert!(fits(&a) && fits(&b));
    assert_eq!(
        a.split_oversized(&b, &id, &vk, dev(1), &[]).unwrap_err(),
        mosaic_core::Error::SizeLimit
    );
}

/// Review M2, fourth round (tests, medium): a split of a live version and
/// a deletion. The live version sits at the limit and the deleting
/// replica's vector has more entries, so the merge does not fit; the note
/// moves to a conflict copy, and the record keeps the attachment reference
/// (its file is bound to the record) and the deletion notice. Both devices
/// converge on it.
#[test]
fn a_split_against_a_deletion_keeps_the_attachments_and_the_notice() {
    let at = Timestamp::from_unix_millis;
    let vk = vault_key();
    let base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();
    let mut tombstone = copy(&base);
    for d in 3..7 {
        tombstone.edit(dev(d), at(1)).set_title("t").unwrap();
    }
    tombstone.delete(dev(1), at(2)).unwrap();
    let r = attachment("kept.txt");
    let live = |n: usize| {
        let mut b = copy(&base);
        {
            let mut e = b.edit(dev(2), at(3));
            e.set_text(TextField::Notes, &"b".repeat(n)).unwrap();
            e.set_attachment(&r).unwrap();
        }
        b
    };
    let b = live(largest_fitting(600_000, 1_100_000, live));
    assert!(!fits(&b.merge(&tombstone)), "the merge does not fit");
    let id = mosaic_core::ItemId::new_v7().unwrap();
    let on_b = b
        .split_oversized(&tombstone, &id, &vk, dev(2), &[])
        .unwrap();
    let on_t = tombstone
        .split_oversized(&b, &id, &vk, dev(1), &[])
        .unwrap();
    for s in [&on_b, &on_t] {
        assert!(fits(&s.item));
        assert!(s.item.deleted_concurrently().is_some());
        assert_eq!(attachment_ids(&s.item), BTreeSet::from([r.attachment_id]));
        let [(_, c)] = &s.conflict_copies[..] else {
            panic!("one copy");
        };
        assert_eq!(c.text(TextField::Notes), b.text(TextField::Notes));
        assert!(!has_attachment_key(c));
    }
    // The same copy ID on both; B's record (with its write-backs, such as
    // the type) dominates the deleting device's, and its copy dominates
    // or equals the other.
    assert_eq!(on_b.conflict_copies[0].0, on_t.conflict_copies[0].0);
    assert_eq!(on_b.item.compare(&on_t.item), Causality::Dominates);
    let t_final = on_t.item.merge_and_repair(&on_b.item, dev(1)).unwrap();
    assert_eq!(t_final.item, on_b.item);
    let (cb, ct) = (&on_b.conflict_copies[0].1, &on_t.conflict_copies[0].1);
    assert!(matches!(
        cb.compare(ct),
        Causality::Dominates | Causality::Equal
    ));
    assert_eq!(ct.merge_and_repair(cb, dev(1)).unwrap().item, *cb);
}

fn titles(item: &Item) -> Vec<String> {
    let mut out: Vec<String> = item
        .entries("title")
        .filter_map(|e| e.value().as_str().map(str::to_owned))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The history entries of an item by key, with the stamp of the edit that
/// moved each value into the history: the one in the key
/// (`history:{counter:016x}.{device}.{seq}`), which a delete-race repair
/// keeps when it writes the entry back under a fresh stamp.
fn history(item: &Item) -> std::collections::BTreeMap<String, Stamp> {
    item.keys()
        .filter(|k| k.starts_with("history:"))
        .map(|k| (k.to_owned(), key_stamp(k)))
        .collect()
}

/// The stamp encoded in a history key, parsed independently of the crate.
fn key_stamp(key: &str) -> Stamp {
    let rest = key.strip_prefix("history:").unwrap();
    let mut parts = rest.split('.');
    let counter = u64::from_str_radix(parts.next().unwrap(), 16).unwrap();
    let hex: String = parts
        .next()
        .unwrap()
        .chars()
        .filter(|&c| c != '-')
        .collect();
    let bytes: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    Stamp::new(counter, DeviceId::from_slice(&bytes).unwrap()).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Review M2 (tests): the password history across 2 or 3 replicas with
    /// skewed clocks (days ahead or behind) and random repairing syncs.
    /// After every local password change: at most `MAX_HISTORY` entries,
    /// the replaced value is in the history, and exactly the oldest
    /// entries by edit stamp went. After every sync: no entry is lost that
    /// no edit removed. The bound holds per edit (CRYPTO_SPEC §10): a merge
    /// of concurrent changes can hold more, and the next change on any
    /// replica trims back to `MAX_HISTORY` (checked at the end). Some steps
    /// delete the item (review M2, fourth round): the repairing merges of
    /// the other replicas then write the history back under fresh stamps,
    /// and the trim must still go by the edit that moved each value there.
    #[test]
    fn password_history_trim_by_edit_order(
        replica_count in 2usize..=3,
        offsets_days in proptest::collection::vec(-20i64..=20, 3),
        steps in proptest::collection::vec((0usize..3, 0usize..3, 0u8..6), 0..240),
    ) {
        let day = 24 * 60 * 60 * 1000;
        let mut base = Item::new(&ItemType::Login, dev(0), Timestamp::from_unix_millis(100 * day)).unwrap();
        base.edit(dev(0), Timestamp::from_unix_millis(100 * day))
            .set_secret(SecretField::Password, "initial")
            .unwrap();
        let mut replicas: Vec<Item> = (0..replica_count).map(|_| copy(&base)).collect();
        let mut removed: BTreeSet<(String, Stamp)> = BTreeSet::new();
        let mut changes = vec![0usize; replica_count];
        for (n, (d, from, kind)) in (0i64..).zip(&steps) {
            let (d, from) = (d % replica_count, from % replica_count);
            if *kind == 0 {
                let before: BTreeSet<(String, Stamp)> = history(&replicas[d]).into_iter().collect();
                let merged = replicas[d].merge_and_repair(&replicas[from], dev(d + 1)).unwrap();
                let after: BTreeSet<(String, Stamp)> = history(&merged.item).into_iter().collect();
                let incoming: BTreeSet<(String, Stamp)> = history(&replicas[from]).into_iter().collect();
                for lost in before.union(&incoming).filter(|e| !after.contains(*e)) {
                    prop_assert!(removed.contains(lost), "sync lost history entry {:?}", lost);
                }
                replicas[d] = merged.item;
                continue;
            }
            if replicas[d].is_deleted() || changes[d] >= 80 {
                continue;
            }
            if *kind == 5 {
                // A delete removes every entry; a concurrent replica's
                // repair may write them back.
                let wall = 100 * day + offsets_days[d] * day + n * 1000;
                removed.extend(history(&replicas[d]));
                replicas[d].delete(dev(d + 1), Timestamp::from_unix_millis(wall)).unwrap();
                continue;
            }
            changes[d] += 1;
            let item = &mut replicas[d];
            let shown = item
                .secret(SecretField::Password)
                .unwrap()
                .map(|s| s.expose_secret().to_owned());
            let before = history(item);
            let wall = 100 * day + offsets_days[d] * day + n * 1000;
            item.edit(dev(d + 1), Timestamp::from_unix_millis(wall))
                .set_secret(SecretField::Password, &format!("pw {d} {n}"))
                .unwrap();
            let after = history(item);
            prop_assert!(after.len() <= MAX_HISTORY);
            prop_assert!(item.password_history().len() <= MAX_HISTORY);
            if let Some(shown) = shown {
                prop_assert!(
                    item.password_history().iter().any(|h| h.value.expose_secret() == shown),
                    "the replaced value is in the history"
                );
            }
            let added: Vec<&String> = after.keys().filter(|k| !before.contains_key(*k)).collect();
            prop_assert_eq!(added.len(), 1);
            let gone: BTreeSet<(String, Stamp)> = before
                .iter()
                .filter(|(k, _)| !after.contains_key(*k))
                .map(|(k, s)| (k.clone(), *s))
                .collect();
            // The trim removes the oldest entries by stamp (key as the
            // tie-break), never by wall clock.
            let mut by_age: Vec<(Stamp, String)> = before.iter().map(|(k, s)| (*s, k.clone())).collect();
            by_age.sort();
            let excess = before.len().saturating_sub(MAX_HISTORY - 1);
            let expected: BTreeSet<(String, Stamp)> =
                by_age.into_iter().take(excess).map(|(s, k)| (k, s)).collect();
            prop_assert_eq!(&gone, &expected);
            removed.extend(gone);
        }
        // Converge everything, then one more change re-establishes the
        // bound however many entries the merge held.
        let mut all = copy(&replicas[0]);
        for r in &replicas[1..] {
            all = all.merge_and_repair(r, dev(1)).unwrap().item;
        }
        if !all.is_deleted() {
            all.edit(dev(1), Timestamp::from_unix_millis(200 * day))
                .set_secret(SecretField::Password, "after the sync")
                .unwrap();
            prop_assert!(all.password_history().len() <= MAX_HISTORY);
        }
    }
}

/// Review M2, fourth round (sync): a delete-race repair re-adds every
/// restored history entry under a fresh stamp, so trimming by the
/// register's stamp made the oldest entry look like the newest edit, and the
/// next change dropped the value the user had just replaced. A sets `p0` and
/// then `p1`..`p31` (31 entries) and syncs to B; B sets `pB1`, which moves
/// `p31` into the history (32 entries); A purges concurrently. B applies
/// the tombstone and restores the history, then sets `pB2`: the history
/// must keep `p31` and `pB1` and drop `p0`.
#[test]
fn a_restored_history_is_trimmed_by_the_original_edit_order() {
    let at = Timestamp::from_unix_millis;
    let (a, b) = (dev(1), dev(2));
    let mut on_a = Item::new(&ItemType::Login, a, at(0)).unwrap();
    for i in 0..32 {
        on_a.edit(a, at(i))
            .set_secret(SecretField::Password, &format!("p{i}"))
            .unwrap();
    }
    assert_eq!(on_a.password_history().len(), 31);
    let mut on_b = copy(&on_a);
    on_b.edit(b, at(100))
        .set_secret(SecretField::Password, "pB1")
        .unwrap();
    assert_eq!(on_b.password_history().len(), MAX_HISTORY);
    on_a.delete(a, at(101)).unwrap();
    let repaired = on_b.merge_and_repair(&on_a, b).unwrap();
    assert!(repaired.restored_fields > 0);
    let mut on_b = repaired.item;
    assert_eq!(on_b.password_history().len(), MAX_HISTORY);
    on_b.edit(b, at(102))
        .set_secret(SecretField::Password, "pB2")
        .unwrap();
    let kept: BTreeSet<String> = on_b
        .password_history()
        .iter()
        .map(|h| h.value.expose_secret().to_owned())
        .collect();
    assert_eq!(kept.len(), MAX_HISTORY);
    assert!(kept.contains("pB1"), "the value this edit replaced");
    assert!(kept.contains("p31"), "the newest older value");
    assert!(!kept.contains("p0"), "the oldest value is the one trimmed");
}

/// Review M2, third round (sync): the history bound is per edit. With a
/// full history, three devices that each change the password concurrently
/// leave 34 entries after the merge (each device trimmed the same oldest
/// entry and added its own); the next change trims back to 32.
#[test]
fn concurrent_password_changes_can_exceed_the_history_bound_until_the_next_change() {
    let at = Timestamp::from_unix_millis;
    let mut base = Item::new(&ItemType::Login, dev(0), at(0)).unwrap();
    for i in 0..40 {
        base.edit(dev(0), at(i))
            .set_secret(SecretField::Password, &format!("old {i}"))
            .unwrap();
    }
    assert_eq!(base.password_history().len(), MAX_HISTORY);
    let mut replicas: Vec<Item> = (1..=3).map(|_| copy(&base)).collect();
    for (i, r) in replicas.iter_mut().enumerate() {
        r.edit(dev(i + 1), at(100))
            .set_secret(SecretField::Password, &format!("new {i}"))
            .unwrap();
        assert_eq!(r.password_history().len(), MAX_HISTORY);
    }
    let mut merged = copy(&replicas[0]);
    for r in &replicas[1..] {
        merged = merged.merge_and_repair(r, dev(1)).unwrap().item;
    }
    assert_eq!(merged.password_history().len(), MAX_HISTORY + 2);
    merged
        .edit(dev(1), at(200))
        .set_secret(SecretField::Password, "newest")
        .unwrap();
    assert_eq!(merged.password_history().len(), MAX_HISTORY);
}

/// Characterization of accepted risk M2-R7 / M2-R13 (`THREAT_MODEL` R15),
/// re-verified in the third M2 review round: the delete-race restore still
/// depends on delivery order. Base notes `n0` on A, B and D. B writes `y`
/// and D writes `x` concurrently and they exchange, so both hold {x, y}; A
/// stays offline with `n0`. D deletes (its tombstone covers n0, x and y).
/// A edits the title, receives the tombstone and repairs, writing `n0` back
/// under its own stamps (`R_A`). B edits the user name without seeing the
/// tombstone. If B receives the tombstone first, it keeps {n0, x, y}; if it
/// receives `R_A` first, `R_A` holds `notes`, so the repair leaves that field to
/// the merge and the newer x and y are lost without a notice. The M6
/// redesign (the deleter's pre-delete state) must make both orders agree;
/// this test then flips.
#[test]
fn characterization_delete_race_restore_depends_on_delivery_order() {
    let at = Timestamp::from_unix_millis;
    let (a, b, d) = (dev(1), dev(2), dev(3));
    let mut base = Item::new(&ItemType::SecureNote, dev(0), at(0)).unwrap();
    base.edit(dev(0), at(0))
        .set_text(TextField::Notes, "n0")
        .unwrap();
    let mut on_b = copy(&base);
    let mut on_d = copy(&base);
    on_b.edit(b, at(1)).set_text(TextField::Notes, "y").unwrap();
    on_d.edit(d, at(2)).set_text(TextField::Notes, "x").unwrap();
    let both = on_b.merge(&on_d);
    on_b = copy(&both);
    on_d = copy(&both);
    assert_eq!(values(&on_b, "notes"), ["x", "y"]);
    on_d.delete(d, at(3)).unwrap();
    let tombstone = on_d;

    let mut on_a = copy(&base);
    on_a.edit(a, at(4)).set_title("from A").unwrap();
    let r_a = on_a.merge_and_repair(&tombstone, a).unwrap().item;
    assert_eq!(values(&r_a, "notes"), ["n0"]);
    on_b.edit(b, at(5))
        .set_text(TextField::Username, "from B")
        .unwrap();

    // Order 1: the tombstone, then R_A.
    let first = on_b.merge_and_repair(&tombstone, b).unwrap().item;
    let order_1 = first.merge_and_repair(&r_a, b).unwrap().item;
    // Order 2: R_A, then the tombstone (dominated, ignored by a store).
    let second = on_b.merge_and_repair(&r_a, b).unwrap().item;
    assert_eq!(second.compare(&tombstone), Causality::Dominates);
    let order_2 = second;

    assert_eq!(values(&order_1, "notes"), ["n0", "x", "y"]);
    assert_eq!(
        values(&order_2, "notes"),
        ["n0"],
        "M2-R7: x and y are lost in this order (accepted for M2)"
    );
}

/// Review M2, third round: an ID this replica rotated away from is only
/// treated as a collision where one really shows (a higher counter, or one
/// stamp with two contents in a field). An entry under it that only the
/// received side still holds may simply be superseded here, so an honest
/// concurrent edit merges as usual, without a spurious conflict copy.
#[test]
fn a_retired_id_without_a_collision_merges_normally() {
    let at = Timestamp::from_unix_millis;
    let (x, y, z) = (dev(0), dev(1), dev(4));
    let mut base = Item::new(&ItemType::Login, x, at(0)).unwrap();
    base.edit(x, at(1)).set_title("t1").unwrap();
    let mut mine = copy(&base);
    mine.edit(x, at(2)).set_title("t2").unwrap();
    let mut theirs = copy(&base);
    theirs
        .edit(z, at(3))
        .set_text(TextField::Username, "u")
        .unwrap();
    let plain = mine.merge_and_repair(&theirs, y).unwrap();
    let after = mine.merge_and_repair_after_reuse(&theirs, y, &[x]).unwrap();
    assert!(!after.stamp_collision);
    assert_eq!(after.item, plain.item);
    assert_eq!(after.item.title(), Some("t2"));
    assert!(after.item.conflicts().is_empty());
}
/// One replica of the reuse property test: its state, the writer ID it
/// edits and merges under, and the IDs it rotated away from.
struct Writer {
    item: Item,
    id: DeviceId,
    retired: Vec<DeviceId>,
}

/// A store's handling of a received version (CRYPTO_SPEC §10): ignored if
/// the local state dominates or equals it, otherwise merged with the IDs
/// this replica retired; on `DeviceIdReused` the replica rotates to
/// `fresh()` and merges again. Returns whether a collision was reported.
fn store_apply(local: &mut Writer, received: &Item, fresh: &mut impl FnMut() -> DeviceId) -> bool {
    match local.item.compare(received) {
        Causality::Dominates => return false,
        Causality::Equal if local.item == *received => return false,
        _ => {}
    }
    loop {
        match local
            .item
            .merge_and_repair_after_reuse(received, local.id, &local.retired)
        {
            Ok(out) => {
                local.item = out.item;
                return out.stamp_collision;
            }
            Err(mosaic_core::Error::DeviceIdReused) => {
                local.retired.push(local.id);
                local.id = fresh();
            }
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// Whether `value` of `field` (0 title, 1 password, 2 tag, 3 URL) is
/// visible in `item`: as an entry of the field (a conflict copy counts),
/// and for passwords also in the history.
fn shows(item: &Item, field: u8, value: &str) -> bool {
    match field {
        0 => values(item, "title").iter().any(|v| v == value),
        1 => {
            values(item, "login.password").iter().any(|v| v == value)
                || item
                    .password_history()
                    .iter()
                    .any(|h| h.value.expose_secret() == value)
        }
        2 => item.tags().contains(&value),
        // Two URLs added under one stamp share the element key, so one of
        // them is a conflict copy of that key, which `urls()` does not list.
        _ => {
            let json = item.to_json().unwrap();
            let json = core::str::from_utf8(json.expose_secret())
                .unwrap()
                .to_owned();
            json.contains(&format!("\"url\":\"{value}\""))
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Review M2, fourth round (tests, medium): the ADR-0015 recovery under
    /// random histories. Two writers share the ID X (a restored backup or a
    /// copied profile) and 1 or 2 honest devices edit too, all offline from
    /// one base, across the title, the password, tags and URLs (fields both
    /// a writer and an honest device edit included). The writers then meet
    /// (in either order): each applies the other's version like a store,
    /// rotating on `DeviceIdReused` and merging with the old ID retired.
    /// Then everyone gossips in a random order, followed by full rounds.
    /// Every value that no edit superseded survives, and all replicas are
    /// equal.
    ///
    /// The writers meet before an honest device merges both of their
    /// versions: reuse is detected only where it shows (equal vectors with
    /// different contents, a counter above the replica's own, or one stamp
    /// with two contents in a field), and an honest device that merges two
    /// colliding versions that show none of these drops both writers'
    /// values (`reuse_that_shows_nowhere_is_not_detected`). Fresh writer
    /// IDs per unlock make that case need a forged record (ADR-0015).
    #[test]
    fn reused_writer_ids_lose_nothing_once_the_writers_meet(
        honest in 1usize..=2,
        edits in proptest::collection::vec((0usize..4, 0u8..4), 1..24),
        w2_first in any::<bool>(),
        gossip in proptest::collection::vec((0usize..4, 0usize..4), 0..16),
    ) {
        let n = 2 + honest;
        let at = Timestamp::from_unix_millis;
        let x = dev(0);
        let mut base = Item::new(&ItemType::Login, dev(9), at(0)).unwrap();
        {
            let mut e = base.edit(dev(9), at(0));
            e.set_title("base title").unwrap();
            e.set_secret(SecretField::Password, "base password").unwrap();
        }
        let mut replicas: Vec<Writer> = (0..n)
            .map(|r| Writer {
                item: copy(&base),
                id: if r < 2 { x } else { dev(r + 1) },
                retired: Vec::new(),
            })
            .collect();
        let mut next = 20usize;
        let mut fresh = || {
            next += 1;
            dev(next)
        };
        let mut written: Vec<(u8, String)> = Vec::new();
        let mut superseded: BTreeSet<String> = BTreeSet::new();
        for (step, (r, field)) in (1i64..).zip(&edits) {
            let w = &mut replicas[r % n];
            let value = format!("v{step}");
            if *field == 0 {
                // An edit of the title supersedes the value it showed.
                if let Some(t) = w.item.title() {
                    superseded.insert(t.to_owned());
                }
            }
            let mut e = w.item.edit(w.id, at(step));
            match field {
                0 => e.set_title(&value).unwrap(),
                1 => e.set_secret(SecretField::Password, &value).unwrap(),
                2 => e.add_tag(&value).unwrap(),
                _ => {
                    e.add_url(&LoginUrl::new(&value)).unwrap();
                }
            }
            written.push((*field, value));
        }
        // The writers meet.
        let (first, second) = if w2_first { (1, 0) } else { (0, 1) };
        let other = copy(&replicas[second].item);
        store_apply(&mut replicas[first], &other, &mut fresh);
        let other = copy(&replicas[first].item);
        store_apply(&mut replicas[second], &other, &mut fresh);
        // Gossip in a random order, then full rounds.
        let full = (0..3).flat_map(|_| (0..n).flat_map(move |a| (0..n).map(move |b| (a, b))));
        for (to, from) in gossip.iter().map(|(a, b)| (a % n, b % n)).chain(full) {
            if to != from {
                let received = copy(&replicas[from].item);
                store_apply(&mut replicas[to], &received, &mut fresh);
            }
        }
        for r in &replicas[1..] {
            prop_assert_eq!(&r.item, &replicas[0].item, "replicas converge");
        }
        for (field, value) in &written {
            if !superseded.contains(value) {
                prop_assert!(
                    shows(&replicas[0].item, *field, value),
                    "value {} of field {} was lost", value, field
                );
            }
        }
    }
}

/// The limit of ADR-0015's defence in depth, pinned: two writers with one
/// ID each make one edit of a different field from the same state, and an
/// honest device that made an edit of its own merges both versions. The
/// vectors differ (the honest edit), the counters for X are equal, and no
/// field holds one stamp with two contents, so nothing shows the reuse and
/// the merge drops both writers' values, as the pure join must (each
/// vector covers the other's stamp). ADR-0015 makes writer IDs fresh per
/// unlock so that this needs a forged record; the writers themselves still
/// detect it when they meet each other's versions first (see the property
/// test above).
#[test]
fn reuse_that_shows_nowhere_is_not_detected() {
    let at = Timestamp::from_unix_millis;
    let (x, h) = (dev(0), dev(1));
    let base = Item::new(&ItemType::Login, dev(9), at(0)).unwrap();
    let (mut w1, mut w2, mut honest) = (copy(&base), copy(&base), copy(&base));
    w1.edit(x, at(1)).set_title("from w1").unwrap();
    w2.edit(x, at(2))
        .set_secret(SecretField::Password, "from w2")
        .unwrap();
    honest
        .edit(h, at(3))
        .set_text(TextField::Username, "honest")
        .unwrap();
    let step = honest.merge_and_repair(&w1, h).unwrap().item;
    let out = step.merge_and_repair(&w2, h).unwrap();
    assert!(!out.stamp_collision, "nothing shows the reuse");
    assert_eq!(out.item.title(), None);
    assert_eq!(password(&out.item), None);
    assert_eq!(out.item.text(TextField::Username), Some("honest"));
}

/// Review M2, fourth round (found by the reuse property test): two writers
/// with one ID that each change the password twice from the same state
/// write the same history key (it names the edit's stamp) with different
/// old passwords. The collision repair kept the dropped one as a second
/// entry of that key, where the history (one value per key) never showed
/// it; it now gets a history key of its own.
#[test]
fn a_history_key_written_by_two_writers_keeps_both_passwords() {
    let at = Timestamp::from_unix_millis;
    let x = dev(0);
    let mut base = Item::new(&ItemType::Login, dev(9), at(0)).unwrap();
    base.edit(dev(9), at(0))
        .set_secret(SecretField::Password, "p0")
        .unwrap();
    let (mut w1, mut w2) = (copy(&base), copy(&base));
    for (w, name, t) in [(&mut w1, "w1", 10), (&mut w2, "w2", 20)] {
        for step in ["a", "b"] {
            w.edit(x, at(t))
                .set_secret(SecretField::Password, &format!("{name}{step}"))
                .unwrap();
        }
    }
    assert_eq!(w1.compare(&w2), Causality::Equal);
    let out = w1.merge_and_repair(&w2, dev(1)).unwrap();
    assert!(out.stamp_collision);
    let history: BTreeSet<String> = out
        .item
        .password_history()
        .iter()
        .map(|h| h.value.expose_secret().to_owned())
        .collect();
    assert_eq!(
        history,
        BTreeSet::from(["p0".into(), "w1a".into(), "w2a".into()])
    );
    assert_eq!(values(&out.item, "login.password"), ["w1b", "w2b"]);
    // Every history register holds one entry again.
    for k in out.item.keys().filter(|k| k.starts_with("history:")) {
        assert_eq!(out.item.entries(k).count(), 1, "{k}");
    }
}
