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
//!
//! Seeds: small and golden plaintexts and short histories
//! (`seeds/item_merge`).
#![no_main]

use std::collections::BTreeSet;

use libfuzzer_sys::fuzz_target;
use mosaic_core::DeviceId;
use mosaic_core::model::{Item, ItemType, LoginUrl, SecretField, Stamp, TextField, Timestamp};

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
        _ => {}
    }
});
