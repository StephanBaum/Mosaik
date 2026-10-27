//! The sync merge on fuzzed states.
//!
//! - Mode 0: `[0, len_a:u16, a, len_b:u16, b, c]`: three plaintexts. If all
//!   three decode as items, the merge must be commutative, associative and
//!   idempotent on them, and its result must round-trip. The repairing
//!   merge must give a state that dominates the pure merge and round-trips.
//! - Mode 1: `[1, ops...]`: an edit history (up to 32 operations of two
//!   bytes, kind and argument) on three replicas of the small seed item
//!   `unknown_members`, with syncs in between (repairing syncs when the
//!   argument is 128 or more). The laws must hold on every reached state,
//!   all replicas must converge in any merge order, and a merge must never
//!   drop an entry that no device removed.
//! - Mode 2: `[2, order, ops...]`: a reused writer ID (ADR-0015). Two
//!   writers share one ID and an honest device has its own; up to 24
//!   operations of two bytes (field, argument) edit the title, the
//!   password, tags or URLs from one base. The writers then meet in the
//!   order `order` selects, each applying the other's version like a store
//!   (rotating to a fresh ID on `DeviceIdReused` and merging again with the
//!   retired IDs as reused), and everyone gossips (the pairs derived from
//!   the operations, then full rounds). All replicas must converge and
//!   every value no edit superseded must still be visible. In modes 0 and
//!   1 the merge after a reuse also runs with reused IDs derived from the
//!   input, and its result must round-trip.
//!
//! Seeds: small and golden plaintexts and short histories
//! (`seeds/item_merge`).
#![no_main]

use std::collections::BTreeSet;

use libfuzzer_sys::fuzz_target;
use mosaic_core::DeviceId;
use mosaic_core::model::{
    Causality, Item, ItemType, LoginUrl, SecretField, Stamp, TextField, Timestamp,
};

/// A small base item, so a history runs quickly under ASan.
const BASE: &[u8] = include_bytes!("../seeds/item_plaintext/unknown_members");

fn laws(a: &Item, b: &Item, c: &Item) {
    let ab = a.merge(b);
    assert_eq!(ab, b.merge(a), "commutative");
    assert_eq!(a.merge(a), *a, "idempotent");
    assert_eq!(ab.merge(&ab), ab, "idempotent");
    assert_eq!(ab.merge(c), a.merge(&b.merge(c)), "associative");
    if let Ok(json) = ab.to_json() {
        assert_eq!(Item::from_json(json.expose_secret()).expect("decode"), ab);
    }
    if let Ok(out) = a.merge_and_repair(b, dev(0)) {
        assert_eq!(out.item.merge(&ab), out.item, "the repair dominates the merge");
        if let Ok(json) = out.item.to_json() {
            let back = Item::from_json(json.expose_secret()).expect("repair decodes");
            assert_eq!(back, out.item);
        }
    }
    // The merge after a reuse, with the devices of `c`'s vector as the
    // reused IDs (derived from the input).
    let reused: Vec<DeviceId> = c.version().iter().map(|(d, _)| *d).take(4).collect();
    if let Ok(out) = a.merge_and_repair_after_reuse(b, dev(0), &reused)
        && let Ok(json) = out.item.to_json()
    {
        let back = Item::from_json(json.expose_secret()).expect("reuse merge decodes");
        assert_eq!(back, out.item);
    }
}

/// A replica of mode 2: its state, its writer ID and its retired IDs.
struct Replica {
    item: Item,
    id: DeviceId,
    retired: Vec<DeviceId>,
}

/// A store applying a received version (CRYPTO_SPEC §10): ignored if the
/// local state dominates or equals it, otherwise merged; on
/// `DeviceIdReused` the replica rotates to a fresh ID and merges again.
fn store_apply(local: &mut Replica, received: &Item, fresh: &mut u8) {
    match local.item.compare(received) {
        Causality::Dominates => return,
        Causality::Equal if local.item == *received => return,
        _ => {}
    }
    loop {
        match local
            .item
            .merge_and_repair_after_reuse(received, local.id, &local.retired)
        {
            Ok(out) => {
                local.item = out.item;
                return;
            }
            Err(mosaic_core::Error::DeviceIdReused) => {
                *fresh = fresh.wrapping_add(1);
                local.retired.push(local.id);
                local.id = DeviceId::from_bytes([0x50, *fresh, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
                    .expect("id");
            }
            Err(e) => panic!("{e:?}"),
        }
    }
}

/// Whether `value` of `field` (0 title, 1 password, 2 tag, 3 URL) is
/// visible: an entry of the field (a conflict copy counts), a password
/// history entry, or a URL under any element key.
fn shows(item: &Item, field: u8, value: &str) -> bool {
    let entry_of = |key: &str| {
        item.entries(key)
            .any(|e| e.value().as_str() == Some(value))
    };
    match field {
        0 => entry_of("title"),
        1 => {
            entry_of("login.password")
                || item
                    .password_history()
                    .iter()
                    .any(|h| h.value.expose_secret() == value)
        }
        2 => item.tags().contains(&value),
        _ => {
            let json = item.to_json().expect("encode");
            core::str::from_utf8(json.expose_secret())
                .expect("utf-8")
                .contains(&format!("\"url\":\"{value}\""))
        }
    }
}

fn reuse(order: u8, ops: &[u8]) {
    let x = DeviceId::from_bytes([0x40; 16]).expect("id");
    let mut base = Item::new(&ItemType::Login, dev(2), Timestamp::from_unix_millis(0)).expect("new");
    base.edit(dev(2), Timestamp::from_unix_millis(0))
        .set_secret(SecretField::Password, "base")
        .expect("edit");
    let mut replicas: Vec<Replica> = [x, x, dev(1)]
        .into_iter()
        .map(|id| Replica {
            item: copy(&base),
            id,
            retired: Vec::new(),
        })
        .collect();
    let mut fresh = 0u8;
    let mut written: Vec<(u8, String)> = Vec::new();
    let mut superseded: BTreeSet<String> = BTreeSet::new();
    let ops = ops.as_chunks::<2>().0;
    for (step, &[field, arg]) in (1i64..).zip(ops.iter().take(24)) {
        let field = field % 4;
        let r = &mut replicas[usize::from(arg % 3)];
        let value = format!("v{step}");
        if field == 0
            && let Some(t) = r.item.title()
        {
            superseded.insert(t.to_owned());
        }
        let mut e = r.item.edit(r.id, Timestamp::from_unix_millis(step));
        match field {
            0 => e.set_title(&value).expect("edit"),
            1 => e.set_secret(SecretField::Password, &value).expect("edit"),
            2 => e.add_tag(&value).expect("edit"),
            _ => {
                e.add_url(&LoginUrl::new(&value)).expect("edit");
            }
        }
        written.push((field, value));
    }
    let (first, second) = if order % 2 == 0 { (0, 1) } else { (1, 0) };
    let other = copy(&replicas[second].item);
    store_apply(&mut replicas[first], &other, &mut fresh);
    let other = copy(&replicas[first].item);
    store_apply(&mut replicas[second], &other, &mut fresh);
    let pairs = ops
        .iter()
        .map(|&[a, b]| (usize::from(a % 3), usize::from(b % 3)));
    let full = (0..3).flat_map(|_| (0..3).flat_map(|a| (0..3).map(move |b| (a, b))));
    for (to, from) in pairs.chain(full) {
        if to != from {
            let received = copy(&replicas[from].item);
            store_apply(&mut replicas[to], &received, &mut fresh);
        }
    }
    for r in &replicas[1..] {
        assert!(r.item == replicas[0].item, "replicas converge");
    }
    for (field, value) in &written {
        if !superseded.contains(value) {
            assert!(shows(&replicas[0].item, *field, value), "lost {value}");
        }
    }
}

fn take<'a>(data: &mut &'a [u8]) -> Option<&'a [u8]> {
    let (len, rest) = data.split_first_chunk::<2>()?;
    let len = usize::from(u16::from_be_bytes(*len)).min(rest.len());
    let (part, rest) = rest.split_at(len);
    *data = rest;
    Some(part)
}

fn entries(item: &Item) -> BTreeSet<(String, Stamp)> {
    item.keys()
        .flat_map(|k| item.entries(k).map(move |e| (k.to_owned(), e.stamp())))
        .collect()
}

fn dev(i: u8) -> DeviceId {
    DeviceId::from_bytes([0x30 + i % 3; 16]).expect("id")
}

fn copy(item: &Item) -> Item {
    Item::from_json(item.to_json().expect("encode").expose_secret()).expect("decode")
}

fn history(ops: &[u8]) {
    let base = Item::from_json(BASE).expect("base item");
    let mut replicas = [copy(&base), copy(&base), copy(&base)];
    let mut states = vec![copy(&base)];
    let mut removed = BTreeSet::new();
    for (clock, &[kind, arg]) in (0i64..).zip(ops.as_chunks::<2>().0.iter().take(32)) {
        let d = usize::from(arg % 3);
        let v = arg / 3 % 4;
        let at = Timestamp::from_unix_millis(clock);
        if kind % 12 == 11 {
            let from = usize::from(arg / 3 % 3);
            let before: BTreeSet<_> = entries(&replicas[d])
                .union(&entries(&replicas[from]))
                .cloned()
                .collect();
            let merged = if arg >= 128 {
                replicas[d]
                    .merge_and_repair(&replicas[from], dev(arg))
                    .expect("honest history")
                    .item
            } else {
                replicas[d].merge(&replicas[from])
            };
            for lost in before.difference(&entries(&merged)) {
                assert!(removed.contains(lost), "merge dropped {lost:?}");
            }
            replicas[d] = merged;
        } else {
            let before = entries(&replicas[d]);
            let item = &mut replicas[d];
            let conflict = item
                .conflicts()
                .first()
                .map(|c| (c.key.to_owned(), c.losers[0].stamp()));
            let url = item.urls().first().map(|(id, _)| id.clone());
            if kind % 12 == 6 {
                item.delete(dev(arg), at).expect("delete");
            } else if item.is_deleted() {
                // A deleted item takes no edits; undelete instead.
                item.undelete(&ItemType::Login, dev(arg), at)
                    .expect("undelete");
            } else {
                let mut e = item.edit(dev(arg), at);
                let r = match kind % 12 {
                    0 => e.set_title(&format!("t{v}")),
                    1 => e.set_secret(SecretField::Password, &format!("p{v}")),
                    2 => e.add_tag(&format!("g{v}")),
                    3 => e.remove_tag(&format!("g{v}")),
                    4 => e.add_url(&LoginUrl::new(&format!("https://{v}"))).map(|_| ()),
                    5 => url.map_or(Ok(()), |id| e.remove_url(&id)),
                    7 => e.dismiss_deletion(),
                    8 => conflict.map_or(Ok(()), |(k, s)| e.resolve_conflict(&k, s)),
                    9 => e.set_otp_counter(u64::from(v)),
                    _ => e.clear_text(TextField::Username),
                };
                r.expect("edit");
            }
            removed.extend(before.difference(&entries(&replicas[d])).cloned());
        }
        states.push(copy(&replicas[d]));
    }
    for w in states.windows(3) {
        laws(&w[0], &w[1], &w[2]);
    }
    let [a, b, c] = &replicas;
    let x = a.merge(b).merge(c);
    assert_eq!(x, c.merge(a).merge(b), "convergence");
    assert_eq!(x, b.merge(&c.merge(a)), "convergence");
    assert!(entries(&x).is_disjoint(&removed), "removals are final");
}

fuzz_target!(|data: &[u8]| {
    match data.split_first() {
        Some((&0, mut rest)) => {
            let (Some(a), Some(b)) = (take(&mut rest), take(&mut rest)) else {
                return;
            };
            if let (Ok(a), Ok(b), Ok(c)) =
                (Item::from_json(a), Item::from_json(b), Item::from_json(rest))
            {
                laws(&a, &b, &c);
                laws(&c, &a, &b);
            }
        }
        Some((&1, ops)) => history(ops),
        Some((&2, rest)) => {
            if let Some((&order, ops)) = rest.split_first() {
                reuse(order, ops);
            }
        }
        _ => {}
    }
});
