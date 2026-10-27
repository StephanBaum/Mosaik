//! The sync entry points: `UnlockedSession::apply_remote_record`,
//! `apply_remote_vault_key` and `apply_remote_vault_meta` with
//! server-controlled input.
//!
//! Every input builds two fresh sessions of one account (no Argon2id: the
//! account key is generated and copied through a wrapping): a producer
//! that writes valid records, keys and metadata as another device would,
//! and the target under test. The input is `n` and then `n` 4-byte steps
//! `[op, a, b, c]` (at most 64); the bytes after them are raw record bytes
//! for the raw-delivery step. Deliveries take a produced record and mutate
//! it (bit flips, another record or vault ID, another `server_seq`, the
//! other record kind) or decode raw bytes. Either device can rotate the
//! vault key, so concurrent rotations (refused, ADR-0013) happen too.
//!
//! Checked after every delivery:
//! - nothing panics;
//! - a record whose authenticated content was changed (a bit flip,
//!   another record or vault ID, the other record kind) is rejected, with
//!   `ItemCorrupted` for that record, `UnsupportedVersion` (a changed
//!   `schema_version` in the header), a generation error, `UnknownVault`
//!   or `VaultUnavailable`: an accepted one would be an AAD or
//!   authentication bypass;
//! - a rejected record leaves the target unchanged (item, stored row,
//!   outbox, corrupt list);
//! - an accepted record is readable, and applying it again is `Ignored`;
//! - every corrupt record is unreadable and still stored.
//!
//! At the end of each input, unless the two devices hold different keys
//! (a refused concurrent rotation, or a rotation only the target made),
//! every unmutated record is delivered in order, and then the target
//! holds every item of the producer: exactly, or, for an item the target
//! edited itself, a state that contains the producer's.
//!
//! Reproducibility: account, keys, vault, item and device IDs and nonces
//! come from the OS CSPRNG (the session has no seeded mode, and the crypto
//! core takes no injected randomness), so two runs of one input differ in
//! those bytes and in the order of IDs. A failure that depends on them may
//! not reproduce from the saved input; rerun it a few times. The IDs that
//! mutations substitute are derived from the input.
#![no_main]

// `mosaic_vault::session::fuzzing` exists only under `cfg(fuzzing)`, which
// cargo-fuzz sets. Say so instead of leaving only an E0433.
#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;

use mosaic_core::model::{Causality, ItemType, SecretField, TextField};
use mosaic_core::{
    AccountId, AccountKey, Item, ItemId, Kek, SecretBytes, UnlockMethod, VaultId,
    VaultMetaCiphertext,
};
use mosaic_vault::session::{ApplyOutcome, Error, RemoteRecord, UnlockedSession, fuzzing};
use mosaic_vault::storage::{RecordBody, RecordKind, Store, StoredRecord};

/// At most this many items, so states stay small.
const MAX_ITEMS: usize = 4;
/// At most this many rotations per input.
const MAX_ROTATIONS: usize = 3;

struct World {
    producer: UnlockedSession,
    target: UnlockedSession,
    vault: VaultId,
    items: Vec<ItemId>,
    records: Vec<RemoteRecord>,
    seq: u64,
    rotations: usize,
    /// The target refused one of the producer's keys (a concurrent
    /// rotation, ADR-0013).
    key_conflict: bool,
    /// Items the target edited itself.
    target_edited: BTreeSet<ItemId>,
}

fn world() -> World {
    let account = AccountId::new_v7().expect("id");
    let ak = AccountKey::generate().expect("account key");
    let kek = Kek::from_secret(SecretBytes::random().expect("random"));
    let copy = ak
        .wrap(&kek, &account, &UnlockMethod::Password)
        .expect("wrap")
        .unwrap_account_key(&kek, &account, &UnlockMethod::Password)
        .expect("unwrap");
    let mut producer =
        fuzzing::unlocked(Store::open_in_memory().expect("store"), account, ak).expect("producer");
    let target =
        fuzzing::unlocked(Store::open_in_memory().expect("store"), account, copy).expect("target");
    let vault = producer.create_vault("Shared").expect("vault");
    let mut w = World {
        producer,
        target,
        vault,
        items: Vec::new(),
        records: Vec::new(),
        seq: 0,
        rotations: 0,
        key_conflict: false,
        target_edited: BTreeSet::new(),
    };
    w.share_keys();
    let meta = w.producer.store().vault_meta(&vault).expect("meta");
    w.target
        .apply_remote_vault_meta(vault, &meta.expect("present"))
        .expect("meta applies");
    w
}

impl World {
    /// Uploads the producer's outbox (as M6 would) and keeps the records.
    fn upload(&mut self) {
        for entry in self.producer.pending_uploads(usize::MAX).expect("outbox") {
            let rec = entry.expect("producer rows are valid");
            self.seq += 1;
            self.producer
                .mark_uploaded(
                    rec.vault_id,
                    rec.record_id,
                    rec.outbox_seq.expect("queued"),
                    self.seq,
                )
                .expect("mark");
            self.records.push(RemoteRecord {
                vault_id: rec.vault_id,
                record_id: rec.record_id,
                body: rec.body,
                server_seq: self.seq,
            });
        }
    }

    fn share_keys(&mut self) {
        let keys = self.producer.store().vault_keys(&self.vault).expect("keys");
        for (g, wrapped) in keys {
            match self.target.apply_remote_vault_key(self.vault, g, &wrapped) {
                Ok(()) => {}
                Err(Error::KeyGenerationRollback { .. }) => {}
                // Both rotated to the same generation; each keeps its own.
                Err(Error::VaultKeyConflict { generation }) => {
                    assert_eq!(generation, g.get());
                    assert!(
                        self.target
                            .store()
                            .vault_key(&self.vault, g)
                            .expect("key")
                            .is_some()
                    );
                    self.key_conflict = true;
                }
                Err(e) => panic!("a valid key was refused: {e:?}"),
            }
        }
    }

    fn item(&self, a: u8) -> Option<ItemId> {
        (!self.items.is_empty()).then(|| self.items[usize::from(a) % self.items.len()])
    }

    fn step(&mut self, op: u8, a: u8, b: u8, c: u8, raw: &[u8]) {
        match op % 9 {
            0 => {
                if let Some(id) = self.item(a) {
                    let _ = self.producer.update_item(self.vault, id, |e| {
                        e.set_secret(SecretField::Password, &format!("p{b}"))
                    });
                }
            }
            1 if self.items.len() < MAX_ITEMS => {
                let id = self
                    .producer
                    .add_item(self.vault, &ItemType::Login, |e| {
                        e.set_title(&format!("t{a}"))?;
                        e.set_secret(SecretField::Password, &format!("p{b}"))
                    })
                    .expect("add");
                self.items.push(id);
            }
            2 => {
                if let Some(id) = self.item(a) {
                    let _ = self.producer.purge_item(self.vault, id);
                }
            }
            3 => {
                // A concurrent local edit on the target.
                if let Some(id) = self.item(a)
                    && self
                        .target
                        .update_item(self.vault, id, |e| {
                            e.set_text(TextField::Username, &format!("u{b}"))
                        })
                        .is_ok()
                {
                    self.target_edited.insert(id);
                }
            }
            4 => {
                self.upload();
                if !self.records.is_empty() {
                    let rec = self.records[usize::from(a) % self.records.len()].clone();
                    if let Some((rec, tampered)) = self.mutate(rec, b, c) {
                        self.deliver(&rec, tampered);
                    }
                }
            }
            5 => {
                let kind = if a % 2 == 0 {
                    RecordKind::Item
                } else {
                    RecordKind::Tombstone
                };
                if let (Ok(body), Some(id)) = (RecordBody::decode(kind, raw), self.item(b)) {
                    let rec = RemoteRecord {
                        vault_id: self.vault,
                        record_id: id,
                        body,
                        server_seq: u64::from(c),
                    };
                    self.deliver(&rec, false);
                }
            }
            6 if self.rotations < MAX_ROTATIONS => {
                self.rotations += 1;
                if a % 4 == 0 {
                    // The target rotates too: possibly a concurrent rotation.
                    self.target.rotate_vault_key(self.vault).expect("rotate");
                } else {
                    self.producer.rotate_vault_key(self.vault).expect("rotate");
                }
                if c % 2 == 0 {
                    self.share_keys();
                }
            }
            7 => self.share_keys(),
            8 => self.deliver_meta(b, c),
            _ => {}
        }
    }

    /// A produced record, changed as `mode` says, and whether the change
    /// touched authenticated content (the ciphertext, the IDs in the AAD,
    /// the record kind); `None` if the change does not decode. The
    /// `server_seq` (mode 4) is not authenticated.
    fn mutate(&self, mut rec: RemoteRecord, mode: u8, c: u8) -> Option<(RemoteRecord, bool)> {
        let tampered = match mode % 6 {
            0 | 4 => {
                if mode % 6 == 4 {
                    rec.server_seq = u64::from(c) << 56;
                }
                false
            }
            1 => {
                let mut bytes = rec.body.as_bytes().to_vec();
                let bit = usize::from(c) * 7 % (bytes.len() * 8);
                bytes[bit / 8] ^= 1 << (bit % 8);
                rec.body = RecordBody::decode(rec.body.kind(), &bytes).ok()?;
                true
            }
            2 => {
                let original = rec.record_id;
                rec.record_id = match self.item(c) {
                    Some(id) if c % 2 == 0 => id,
                    _ => ItemId::from_bytes([c | 0x80; 16]).expect("id"),
                };
                rec.record_id != original
            }
            3 => {
                rec.vault_id = VaultId::from_bytes([c | 0x80; 16]).expect("id");
                true
            }
            _ => {
                let other = match rec.body.kind() {
                    RecordKind::Item => RecordKind::Tombstone,
                    RecordKind::Tombstone => RecordKind::Item,
                };
                rec.body = RecordBody::decode(other, rec.body.as_bytes()).ok()?;
                true
            }
        };
        Some((rec, tampered))
    }

    fn deliver(&mut self, rec: &RemoteRecord, tampered: bool) {
        let before = Snapshot::of(&self.target, rec);
        let result = self.target.apply_remote_record(rec);
        if tampered {
            assert!(
                matches!(
                    &result,
                    Err(Error::Core(mosaic_core::Error::ItemCorrupted { item_id }))
                        if *item_id == rec.record_id
                ) || matches!(
                    &result,
                    // A flipped `schema_version` in the item header is
                    // refused before decryption.
                    Err(Error::Core(mosaic_core::Error::UnsupportedVersion)
                        | Error::KeyGenerationRollback { .. }
                        | Error::KeyGenerationNotAccepted { .. }
                        | Error::UnknownVault
                        | Error::VaultUnavailable)
                ),
                "a tampered record was not refused: {result:?}"
            );
        }
        match result {
            Ok(_) => {
                assert!(
                    self.target.item(rec.vault_id, rec.record_id).is_ok(),
                    "an accepted record is readable"
                );
                assert!(
                    matches!(
                        self.target.apply_remote_record(rec),
                        Ok(ApplyOutcome::Ignored)
                    ),
                    "applying a record twice is idempotent"
                );
            }
            Err(_) => assert!(
                Snapshot::of(&self.target, rec) == before,
                "a rejected record changed the target"
            ),
        }
        self.check_corrupt();
    }

    fn deliver_meta(&mut self, mode: u8, c: u8) {
        let Some(meta) = self.producer.store().vault_meta(&self.vault).expect("meta") else {
            return;
        };
        let meta = if mode % 2 == 0 {
            meta
        } else {
            let mut bytes = meta.as_bytes().to_vec();
            let bit = usize::from(c) * 5 % (bytes.len() * 8);
            bytes[bit / 8] ^= 1 << (bit % 8);
            let Ok(m) = VaultMetaCiphertext::from_bytes(&bytes) else {
                return;
            };
            m
        };
        let before = self.target.store().vault_meta(&self.vault).ok().flatten();
        let name = self
            .target
            .vault_meta(self.vault)
            .expect("known vault")
            .and_then(|m| m.name().map(str::to_owned));
        if self
            .target
            .apply_remote_vault_meta(self.vault, &meta)
            .is_err()
        {
            assert_eq!(
                self.target.store().vault_meta(&self.vault).ok().flatten(),
                before
            );
            assert_eq!(
                self.target
                    .vault_meta(self.vault)
                    .expect("known vault")
                    .and_then(|m| m.name().map(str::to_owned)),
                name
            );
        }
    }

    /// Delivers every unmutated record in order and checks that the
    /// target holds every item of the producer.
    fn check_convergence(&mut self) {
        self.share_keys();
        self.upload();
        let generation = |s: &UnlockedSession| {
            s.vaults()
                .iter()
                .find(|v| v.vault_id == self.vault)
                .map(|v| v.key_generation)
        };
        if self.key_conflict || generation(&self.target) != generation(&self.producer) {
            return;
        }
        for rec in self.records.clone() {
            // Older versions are dominated or below the accepted
            // generation now; only the latest of each item matters.
            let _ = self.target.apply_remote_record(&rec);
        }
        for &id in &self.items {
            let Ok(p) = self.producer.item(self.vault, id) else {
                continue;
            };
            let t = self
                .target
                .item(self.vault, id)
                .expect("the target holds every item of the producer");
            if self.target_edited.contains(&id) {
                assert!(
                    matches!(t.compare(p), Causality::Dominates | Causality::Equal),
                    "the target's state contains the producer's"
                );
                assert!(t.merge(p) == *t, "nothing of the producer's is missing");
            } else {
                assert!(t == p, "the target converged to the producer");
            }
        }
    }

    fn check_corrupt(&self) {
        for (v, id) in self.target.corrupt_records() {
            assert!(
                matches!(self.target.item(v, id), Err(Error::UnknownItem)),
                "a corrupt record is not readable"
            );
            assert!(
                !matches!(self.target.store().record(&v, &id), Ok(None)),
                "a corrupt record is still stored"
            );
        }
    }
}

/// What a rejected record must leave unchanged.
#[derive(PartialEq)]
struct Snapshot {
    item: Option<Vec<u8>>,
    row: Option<Option<StoredRecord>>,
    outbox: usize,
    corrupt: Vec<(VaultId, ItemId)>,
}

impl Snapshot {
    fn of(s: &UnlockedSession, rec: &RemoteRecord) -> Self {
        Self {
            item: s.item(rec.vault_id, rec.record_id).ok().map(plaintext),
            row: s.store().record(&rec.vault_id, &rec.record_id).ok(),
            outbox: s.pending_uploads(usize::MAX).map_or(0, |o| o.len()),
            corrupt: s.corrupt_records(),
        }
    }
}

fn plaintext(item: &Item) -> Vec<u8> {
    let json = match item.to_tombstone() {
        Some(t) => t.to_json(),
        None => item.to_json(),
    };
    json.expect("encode").expose_secret().to_vec()
}

fuzz_target!(|data: &[u8]| {
    let Some((&n, rest)) = data.split_first() else {
        return;
    };
    let n = usize::from(n).min(64).min(rest.len() / 4);
    let (steps, raw) = rest.split_at(n * 4);
    let mut w = world();
    for step in steps.as_chunks::<4>().0 {
        let &[op, a, b, c] = step;
        w.step(op, a, b, c, raw);
    }
    w.check_convergence();
});
