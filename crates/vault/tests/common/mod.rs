//! Shared helpers for the session tests: accounts on the baseline Argon2id
//! profile (the cheapest real one, 128 MiB) and a minimal in-memory
//! "server" that only orders and replays ciphertext records.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers shared by several test crates"
)]

use mosaic_core::{Argon2Profile, MasterPassword, SecretKey, UnlockMethod};
use mosaic_vault::session::{CreatedAccount, LockedSession, RemoteRecord, UnlockedSession};
use mosaic_vault::storage::Store;

pub const PASSWORD: &str = "correct horse battery staple";

pub fn pw(s: &str) -> MasterPassword {
    MasterPassword::new(s.to_owned())
}

/// A new account in an in-memory store.
pub fn new_account() -> (UnlockedSession, CreatedAccount) {
    new_account_in(Store::open_in_memory().unwrap())
}

pub fn new_account_in(store: Store) -> (UnlockedSession, CreatedAccount) {
    LockedSession::open(store)
        .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
        .unwrap()
}

/// Another device of the same account: joins with the password wrapping
/// and learns every vault of `from` (keys and metadata), as M6 sync would.
pub fn join(from: &UnlockedSession, secret_key: &SecretKey, store: Store) -> UnlockedSession {
    let params = from.store().account().unwrap().unwrap();
    let wrapped = from
        .store()
        .unlock_method(&UnlockMethod::Password)
        .unwrap()
        .unwrap()
        .wrapped_account_key;
    let mut s = LockedSession::open(store)
        .join_account(&params, &wrapped, &pw(PASSWORD), secret_key)
        .unwrap();
    share_vaults(from, &mut s);
    s
}

/// Copies every vault key and every vault's metadata from `from` to `to`.
pub fn share_vaults(from: &UnlockedSession, to: &mut UnlockedSession) {
    for v in from.vaults() {
        for (generation, wrapped) in from.store().vault_keys(&v.vault_id).unwrap() {
            to.apply_remote_vault_key(v.vault_id, generation, &wrapped)
                .unwrap();
        }
        if let Some(meta) = from.store().vault_meta(&v.vault_id).unwrap() {
            to.apply_remote_vault_meta(v.vault_id, &meta).unwrap();
        }
    }
}

/// A server that stores every uploaded record version in order.
#[derive(Default)]
pub struct Server {
    pub log: Vec<RemoteRecord>,
}

impl Server {
    /// Uploads every pending change of `device`.
    pub fn upload(&mut self, device: &mut UnlockedSession) -> usize {
        let pending = device.pending_uploads(usize::MAX).unwrap();
        let n = pending.len();
        for entry in pending {
            let rec = entry.expect("no corrupt rows in tests");
            let seq = u64::try_from(self.log.len()).unwrap() + 1;
            self.log.push(RemoteRecord {
                vault_id: rec.vault_id,
                record_id: rec.record_id,
                body: rec.body,
                server_seq: seq,
            });
            device
                .mark_uploaded(rec.vault_id, rec.record_id, rec.outbox_seq.unwrap(), seq)
                .unwrap();
        }
        n
    }
}

/// A small deterministic generator for shuffles (xorshift64*). Not for
/// anything secret.
pub struct Shuffler(u64);

impl Shuffler {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }

    /// The records in a random order, with some delivered twice.
    pub fn deliveries(&mut self, log: &[RemoteRecord]) -> Vec<RemoteRecord> {
        let mut out: Vec<RemoteRecord> = log.to_vec();
        for r in log {
            if self.next().is_multiple_of(3) {
                out.push(r.clone());
            }
        }
        for i in (1..out.len()).rev() {
            let j = self.below(i + 1);
            out.swap(i, j);
        }
        out
    }
}
