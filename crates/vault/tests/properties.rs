//! Property tests of the store's invariants against simple models:
//! monotonic key generations (CRYPTO_SPEC §7.1) and record upsert / outbox
//! semantics (§10).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test helpers outside #[test] functions"
)]

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{
    AccountId, AccountKey, ItemCiphertext, ItemId, KeyGeneration, VaultId, VaultKey,
    VaultMetaCiphertext,
};
use mosaic_vault::storage::{Error, Origin, OutboxSeq, PendingPolicy, RecordBody, Store};
use proptest::prelude::*;

fn g(n: u32) -> KeyGeneration {
    KeyGeneration::new(n).unwrap()
}

// ---------------------------------------------------------------------------
// Key generations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum GenOp {
    Accept(u32),
    WriteRecord(u8, u32),
    WriteMeta(u32),
    Prune(u32),
    /// Delete the vault and add it again at a generation.
    Recreate(u32),
}

fn gen_op() -> impl Strategy<Value = GenOp> {
    prop_oneof![
        (1..8u32).prop_map(GenOp::Accept),
        (0..3u8, 1..8u32).prop_map(|(r, n)| GenOp::WriteRecord(r, n)),
        (1..8u32).prop_map(GenOp::WriteMeta),
        (1..8u32).prop_map(GenOp::Prune),
        (1..8u32).prop_map(GenOp::Recreate),
    ]
}

struct GenModel {
    current: u32,
    highest: u32,
    keys: BTreeSet<u32>,
    records: BTreeMap<u8, u32>,
    meta: Option<u32>,
}

impl GenModel {
    /// The §7.1 write rule.
    fn write_rule(&self, n: u32) -> Result<(), (u32, u32, bool)> {
        match n.cmp(&self.highest) {
            std::cmp::Ordering::Equal => Ok(()),
            std::cmp::Ordering::Less => Err((self.highest, n, true)),
            std::cmp::Ordering::Greater => Err((self.highest, n, false)),
        }
    }
}

fn check_write(r: Result<(), Error>, expected: Result<(), (u32, u32, bool)>) {
    match (r, expected) {
        (Ok(()), Ok(())) => {}
        (
            Err(Error::KeyGenerationRollback {
                highest_accepted,
                offered,
            }),
            Err((h, o, true)),
        )
        | (
            Err(Error::KeyGenerationNotAccepted {
                highest_accepted,
                offered,
            }),
            Err((h, o, false)),
        ) => {
            assert_eq!((highest_accepted, offered), (h, o));
        }
        (r, e) => panic!("got {r:?}, expected {e:?}"),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// The highest accepted generation never decreases, not even across
    /// deleting and re-adding the vault; wrapped keys below it are
    /// rejected, and records and metadata are accepted only at exactly the
    /// highest accepted generation.
    #[test]
    fn key_generations_are_monotonic(
        first in 1..4u32,
        ops in proptest::collection::vec(gen_op(), 1..40),
    ) {
        let mut store = Store::open_in_memory().unwrap();
        let account = AccountId::new_v7().unwrap();
        let ak = AccountKey::generate().unwrap();
        let vault = VaultId::new_v7().unwrap();
        let record_ids: Vec<ItemId> = (0..3).map(|_| ItemId::new_v7().unwrap()).collect();
        let wrap = |n: u32| {
            VaultKey::generate(g(n)).unwrap().wrap(&ak, &account, &vault).unwrap()
        };
        store.create_vault(&vault, g(first), &wrap(first)).unwrap();
        let mut model = GenModel {
            current: first,
            highest: first,
            keys: BTreeSet::from([first]),
            records: BTreeMap::new(),
            meta: None,
        };

        for op in ops {
            let highest_before = model.highest;
            match op {
                GenOp::Accept(n) => {
                    let r = store.accept_vault_key(&vault, g(n), &wrap(n));
                    if n < model.highest {
                        prop_assert!(matches!(
                            r,
                            Err(Error::KeyGenerationRollback { highest_accepted, offered })
                                if highest_accepted == model.highest && offered == n
                        ), "{:?}", r);
                    } else {
                        let state = r.unwrap();
                        model.highest = n;
                        model.current = model.current.max(n);
                        model.keys.insert(n);
                        prop_assert_eq!(state.highest_accepted_generation, g(n));
                    }
                }
                GenOp::WriteRecord(i, n) => {
                    let id = record_ids[usize::from(i)];
                    let vk = VaultKey::generate(g(n)).unwrap();
                    let body = RecordBody::Item(
                        ItemCiphertext::seal(&vk, &vault, &id, 1, b"x").unwrap(),
                    );
                    let r = store
                        .upsert_record(&vault, &id, &body, Origin::Local)
                        .map(|_| ());
                    let expected = model.write_rule(n);
                    if expected.is_ok() {
                        model.records.insert(i, n);
                    }
                    check_write(r, expected);
                }
                GenOp::WriteMeta(n) => {
                    let vk = VaultKey::generate(g(n)).unwrap();
                    let meta = VaultMetaCiphertext::seal(&vk, &vault, b"m").unwrap();
                    let r = store.set_vault_meta(&vault, &meta);
                    let expected = model.write_rule(n);
                    if expected.is_ok() {
                        model.meta = Some(n);
                    }
                    check_write(r, expected);
                }
                GenOp::Prune(n) => {
                    let r = store.prune_vault_keys(&vault, g(n));
                    let in_use = n > model.current
                        || model.records.values().any(|&rg| rg < n)
                        || model.meta.is_some_and(|m| m < n);
                    if in_use {
                        prop_assert!(matches!(r, Err(Error::VaultKeyInUse)), "{r:?}");
                    } else {
                        let before = model.keys.len();
                        model.keys.retain(|&k| k >= n);
                        prop_assert_eq!(r.unwrap(), before - model.keys.len());
                    }
                }
                GenOp::Recreate(n) => {
                    prop_assert!(store.delete_vault(&vault, PendingPolicy::Discard).unwrap());
                    prop_assert_eq!(store.generation_floor(&vault).unwrap(), Some(g(model.highest)));
                    let r = store.create_vault(&vault, g(n), &wrap(n));
                    if n < model.highest {
                        prop_assert!(matches!(
                            r,
                            Err(Error::KeyGenerationRollback { highest_accepted, offered })
                                if highest_accepted == model.highest && offered == n
                        ), "{:?}", r);
                        // Put the vault back at the floor to go on.
                        store.create_vault(&vault, g(model.highest), &wrap(model.highest)).unwrap();
                        model.current = model.highest;
                    } else {
                        r.unwrap();
                        model.highest = n;
                        model.current = n;
                    }
                    model.keys = BTreeSet::from([model.current]);
                    model.records.clear();
                    model.meta = None;
                }
            }
            prop_assert!(model.highest >= highest_before);
            let state = store.vault(&vault).unwrap().unwrap();
            prop_assert_eq!(state.highest_accepted_generation, g(model.highest));
            prop_assert_eq!(state.current_generation, g(model.current));
            let held: BTreeSet<u32> = store
                .vault_keys(&vault)
                .unwrap()
                .into_iter()
                .map(|(k, _)| k.get())
                .collect();
            prop_assert_eq!(&held, &model.keys);
            for (i, n) in &model.records {
                let r = store.record(&vault, &record_ids[usize::from(*i)]).unwrap().unwrap();
                prop_assert_eq!(r.body.key_generation(), g(*n));
            }
        }
        prop_assert_eq!(store.integrity_check().unwrap(), Vec::<String>::new());
    }
}

// ---------------------------------------------------------------------------
// Record upsert and outbox
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum RecOp {
    Local(u8),
    Remote(u8, u64),
    Merged(u8, u64),
    /// Acknowledge the `n`-th outbox position ever handed out for the
    /// record (modulo the number handed out), with a server sequence.
    Ack(u8, usize, u64),
    Delete(u8, bool),
}

fn rec_op() -> impl Strategy<Value = RecOp> {
    prop_oneof![
        3 => (0..3u8).prop_map(RecOp::Local),
        2 => (0..3u8, 0..30u64).prop_map(|(r, s)| RecOp::Remote(r, s)),
        1 => (0..3u8, 0..30u64).prop_map(|(r, s)| RecOp::Merged(r, s)),
        2 => (0..3u8, 0..8usize, 0..30u64).prop_map(|(r, i, s)| RecOp::Ack(r, i, s)),
        1 => (0..3u8, any::<bool>()).prop_map(|(r, discard)| RecOp::Delete(r, discard)),
    ]
}

#[derive(Clone, Debug, PartialEq)]
struct RecModel {
    body: RecordBody,
    server_seq: Option<u64>,
    outbox: Option<OutboxSeq>,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Upserts follow the documented rules: local writes queue with fresh,
    /// increasing outbox positions and keep `server_seq`; remote writes set
    /// `server_seq`, never go backwards and never replace a pending local
    /// change; merged writes do both; an upload ack clears only the change
    /// it acknowledges.
    #[test]
    fn record_upserts_follow_the_model(ops in proptest::collection::vec(rec_op(), 1..60)) {
        let mut store = Store::open_in_memory().unwrap();
        let account = AccountId::new_v7().unwrap();
        let ak = AccountKey::generate().unwrap();
        let vault = VaultId::new_v7().unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        store
            .create_vault(&vault, KeyGeneration::FIRST, &vk.wrap(&ak, &account, &vault).unwrap())
            .unwrap();
        let ids: Vec<ItemId> = (0..3).map(|_| ItemId::new_v7().unwrap()).collect();
        let mut model: BTreeMap<u8, RecModel> = BTreeMap::new();
        let mut tokens: BTreeMap<u8, Vec<OutboxSeq>> = BTreeMap::new();
        let mut last_seq: Option<OutboxSeq> = None;

        for (step, op) in ops.into_iter().enumerate() {
            let state = format!("state-{step}").into_bytes();
            let body_for = |i: u8| {
                RecordBody::Item(
                    ItemCiphertext::seal(&vk, &vault, &ids[usize::from(i)], 1, &state).unwrap(),
                )
            };
            match op {
                RecOp::Local(i) => {
                    let body = body_for(i);
                    let seq = store
                        .upsert_record(&vault, &ids[usize::from(i)], &body, Origin::Local)
                        .unwrap()
                        .unwrap();
                    prop_assert!(last_seq.is_none_or(|l| seq > l));
                    last_seq = Some(seq);
                    tokens.entry(i).or_default().push(seq);
                    let server_seq = model.get(&i).and_then(|m| m.server_seq);
                    model.insert(i, RecModel { body, server_seq, outbox: Some(seq) });
                }
                RecOp::Remote(i, s) | RecOp::Merged(i, s) => {
                    let merged = matches!(op, RecOp::Merged(..));
                    let body = body_for(i);
                    let origin = if merged {
                        Origin::Merged { server_seq: s }
                    } else {
                        Origin::Remote { server_seq: s }
                    };
                    let r = store.upsert_record(&vault, &ids[usize::from(i)], &body, origin);
                    let existing = model.get(&i);
                    let stored_seq = existing.and_then(|m| m.server_seq);
                    if stored_seq.is_some_and(|st| st > s) {
                        prop_assert!(matches!(r, Err(Error::StaleServerSeq { .. })), "{r:?}");
                    } else if !merged && existing.is_some_and(|m| m.outbox.is_some()) {
                        prop_assert!(matches!(r, Err(Error::PendingLocalChange)), "{r:?}");
                    } else if merged {
                        let seq = r.unwrap().unwrap();
                        prop_assert!(last_seq.is_none_or(|l| seq > l));
                        last_seq = Some(seq);
                        tokens.entry(i).or_default().push(seq);
                        model.insert(i, RecModel { body, server_seq: Some(s), outbox: Some(seq) });
                    } else {
                        prop_assert_eq!(r.unwrap(), None);
                        model.insert(i, RecModel { body, server_seq: Some(s), outbox: None });
                    }
                }
                RecOp::Ack(i, which, s) => {
                    let issued = tokens.get(&i).cloned().unwrap_or_default();
                    let token = if issued.is_empty() {
                        // Never issued for this record: any other position.
                        let Some(other) = last_seq else { continue };
                        other
                    } else {
                        issued[which % issued.len()]
                    };
                    let r = store.mark_uploaded(&vault, &ids[usize::from(i)], token, s);
                    match model.get_mut(&i) {
                        None => prop_assert!(matches!(r, Err(Error::UnknownRecord)), "{r:?}"),
                        Some(m) if m.server_seq.is_some_and(|st| st > s) => {
                            prop_assert!(matches!(r, Err(Error::StaleServerSeq { .. })), "{r:?}");
                        }
                        Some(m) => {
                            let cleared = m.outbox == Some(token);
                            prop_assert_eq!(r.unwrap(), cleared);
                            m.server_seq = Some(s);
                            if cleared {
                                m.outbox = None;
                            }
                        }
                    }
                }
                RecOp::Delete(i, discard) => {
                    let policy = if discard { PendingPolicy::Discard } else { PendingPolicy::Refuse };
                    let r = store.delete_record(&vault, &ids[usize::from(i)], policy);
                    if !discard && model.get(&i).is_some_and(|m| m.outbox.is_some()) {
                        // A pending local change is never dropped silently.
                        prop_assert!(matches!(r, Err(Error::PendingLocalChange)), "{r:?}");
                    } else {
                        prop_assert_eq!(r.unwrap(), model.remove(&i).is_some());
                    }
                }
            }

            for (i, id) in ids.iter().enumerate() {
                let stored = store.record(&vault, id).unwrap();
                let expected = model.get(&u8::try_from(i).unwrap());
                match (stored, expected) {
                    (None, None) => {}
                    (Some(s), Some(m)) => {
                        prop_assert_eq!(&s.body, &m.body);
                        prop_assert_eq!(s.server_seq, m.server_seq);
                        prop_assert_eq!(s.outbox_seq, m.outbox);
                    }
                    (s, m) => prop_assert!(false, "store {s:?} vs model {m:?}"),
                }
            }
            let mut queued: Vec<(OutboxSeq, ItemId)> = model
                .iter()
                .filter_map(|(i, m)| m.outbox.map(|q| (q, ids[usize::from(*i)])))
                .collect();
            queued.sort();
            let outbox: Vec<(OutboxSeq, ItemId)> = store
                .outbox(usize::MAX)
                .unwrap()
                .into_iter()
                .map(|r| {
                    let r = r.unwrap();
                    (r.outbox_seq.unwrap(), r.record_id)
                })
                .collect();
            prop_assert_eq!(outbox, queued);
        }
    }
}
