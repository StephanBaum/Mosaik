//! Decoding the records of a vault at unlock: decryption stays on one thread (the keys never
//! leave it), and the JSON decoding, which dominates, runs on worker threads for big vaults.
//! The plaintext buffers are zeroizing and are dropped in the worker that decoded them.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};
use std::thread;

use mosaic_core::model::{Item, Tombstone};
use mosaic_core::{ItemId, KeyGeneration, SecretVec, VaultId, VaultKey};

use super::{Error, Result};
use crate::storage::RecordBody;

/// Vaults with at most this many records are decoded on the calling thread.
const PARALLEL_MIN_RECORDS: usize = 512;
/// Most worker threads.
const MAX_WORKERS: usize = 8;

/// The decrypted, not yet decoded plaintext of a record.
pub(super) enum Opened {
    /// An item record.
    Item(SecretVec),
    /// A deletion record.
    Tombstone(SecretVec),
}

/// The decryption half of [`super::open_record`]: the same checks and errors, no decoding.
pub(super) fn open_plaintext(
    body: &RecordBody,
    keys: &BTreeMap<KeyGeneration, VaultKey>,
    vault_id: &VaultId,
    item_id: &ItemId,
) -> Result<Opened> {
    let vk = keys
        .get(&body.key_generation())
        .ok_or(Error::VaultUnavailable)?;
    Ok(match body {
        RecordBody::Item(ct) => Opened::Item(Item::open_plaintext(ct, vk, vault_id, item_id)?),
        RecordBody::Tombstone(ct) => {
            Opened::Tombstone(Tombstone::open_plaintext(ct, vk, vault_id, item_id)?)
        }
    })
}

/// Decodes one record; its plaintext is dropped (wiped) here.
fn parse_one(opened: Opened) -> Result<Item> {
    Ok(match opened {
        Opened::Item(pt) => Item::from_json(pt.expose_secret())?,
        Opened::Tombstone(pt) => Item::from(Tombstone::from_json(pt.expose_secret())?),
    })
}

fn parse_chunk(chunk: Vec<(ItemId, Opened)>) -> Vec<(ItemId, Result<Item>)> {
    chunk
        .into_iter()
        .map(|(id, opened)| (id, parse_one(opened)))
        .collect()
}

/// How many workers to decode `records` records with: one (the calling thread) up to
/// [`PARALLEL_MIN_RECORDS`], then one per available core, at most [`MAX_WORKERS`].
pub(super) fn workers_for(records: usize) -> usize {
    if records <= PARALLEL_MIN_RECORDS {
        return 1;
    }
    thread::available_parallelism().map_or(1, |n| n.get().min(MAX_WORKERS))
}

type Chunk = Vec<(ItemId, Opened)>;

fn take_chunk(slot: &Mutex<Option<Chunk>>) -> Chunk {
    slot.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .unwrap_or_default()
}

/// Decodes `opened` with `workers` threads (one means on the calling thread). The result is in
/// the order of the input and holds, per record, exactly what decoding it alone gives, whatever
/// the number of workers.
pub(super) fn parse_records(opened: Chunk, workers: usize) -> Vec<(ItemId, Result<Item>)> {
    let total = opened.len();
    if workers <= 1 || total < 2 {
        return parse_chunk(opened);
    }
    let chunk_len = total.div_ceil(workers);
    let mut slots: Vec<Mutex<Option<Chunk>>> = Vec::new();
    let mut rest = opened.into_iter();
    loop {
        let chunk: Chunk = rest.by_ref().take(chunk_len).collect();
        if chunk.is_empty() {
            break;
        }
        slots.push(Mutex::new(Some(chunk)));
    }
    thread::scope(|scope| {
        let handles: Vec<_> = slots
            .iter()
            .map(|slot| {
                thread::Builder::new()
                    .spawn_scoped(scope, move || parse_chunk(take_chunk(slot)))
                    .ok()
            })
            .collect();
        let mut out = Vec::with_capacity(total);
        for (slot, handle) in slots.iter().zip(handles) {
            match handle {
                Some(h) => match h.join() {
                    Ok(part) => out.extend(part),
                    // A panic while decoding is a bug; it surfaces as it would on one thread.
                    Err(payload) => std::panic::resume_unwind(payload),
                },
                // The thread could not be started: its chunk is still in the slot.
                None => out.extend(parse_chunk(take_chunk(slot))),
            }
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_core::model::ItemType;
    use mosaic_core::{DeviceId, Timestamp};

    fn id(n: usize) -> ItemId {
        let [a, b] = u16::try_from(n + 1).unwrap().to_be_bytes();
        let mut bytes = [1_u8; 16];
        bytes[0] = a;
        bytes[1] = b;
        ItemId::from_bytes(bytes).unwrap()
    }

    /// 600 plaintexts, some unreadable (not JSON, or a newer schema version).
    fn plaintexts() -> Vec<Vec<u8>> {
        let device = DeviceId::from_bytes([9; 16]).unwrap();
        let at = Timestamp::from_unix_millis(5);
        (0..600)
            .map(|n| {
                if n % 7 == 3 {
                    return b"{ not json".to_vec();
                }
                if n % 11 == 5 {
                    return br#"{"schema_version":99}"#.to_vec();
                }
                let mut item = Item::new(&ItemType::Login, device, at).unwrap();
                item.edit(device, at)
                    .set_title(&format!("Item {n}"))
                    .unwrap();
                item.to_json().unwrap().expose_secret().to_vec()
            })
            .collect()
    }

    fn opened(texts: &[Vec<u8>]) -> Chunk {
        texts
            .iter()
            .enumerate()
            .map(|(n, t)| (id(n), Opened::Item(SecretVec::from_slice(t).unwrap())))
            .collect()
    }

    /// The ID and either the re-encoded item or `None` for an unreadable record.
    fn digest(parsed: Vec<(ItemId, Result<Item>)>) -> Vec<(ItemId, Option<Vec<u8>>)> {
        parsed
            .into_iter()
            .map(|(id, r)| {
                (
                    id,
                    r.ok()
                        .map(|i| i.to_json().unwrap().expose_secret().to_vec()),
                )
            })
            .collect()
    }

    #[test]
    fn worker_counts_follow_the_size_of_the_vault() {
        assert_eq!(workers_for(0), 1);
        assert_eq!(workers_for(PARALLEL_MIN_RECORDS), 1);
        let many = workers_for(PARALLEL_MIN_RECORDS + 1);
        assert!((1..=MAX_WORKERS).contains(&many));
    }

    #[test]
    fn parallel_decoding_matches_the_serial_path() {
        let texts = plaintexts();
        let serial = digest(parse_records(opened(&texts), 1));
        assert_eq!(serial.len(), 600);
        let unreadable = serial.iter().filter(|(_, d)| d.is_none()).count();
        assert!(unreadable > 0 && unreadable < 600);
        for workers in [2, 3, 4, 8, 599, 600, 1000] {
            let parallel = digest(parse_records(opened(&texts), workers));
            assert_eq!(parallel, serial, "{workers} workers");
        }
        // The order is the input order.
        assert!(serial.iter().enumerate().all(|(n, (i, _))| *i == id(n)));
    }

    #[test]
    fn tiny_inputs_are_fine() {
        assert!(parse_records(Vec::new(), 4).is_empty());
        let texts = plaintexts();
        let one = parse_records(opened(&texts[..1]), 4);
        assert_eq!(one.len(), 1);
    }
}
