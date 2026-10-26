//! `RecordBody::decode` and the store's record round trip.
//!
//! The first input byte is the record kind code, the rest the ciphertext.
//! The store's decoder must agree with mosaic-core's strict decoders, keep
//! the bytes unchanged, and every decodable record must survive a write to
//! and a read from an in-memory store unchanged (including the column
//! checks the store makes on the way out).
#![no_main]

use std::cell::RefCell;

use libfuzzer_sys::fuzz_target;
use mosaic_core::{ItemCiphertext, ItemId, TombstoneCiphertext, VaultId, WrappedKey};
use mosaic_vault::storage::{Origin, PendingPolicy, RecordBody, RecordKind, Store};

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::open_in_memory().expect("store"));
}

fuzz_target!(|data: &[u8]| {
    let Some((&code, bytes)) = data.split_first() else {
        return;
    };
    let Some(kind) = RecordKind::from_code(code) else {
        assert!(code != 1 && code != 2);
        return;
    };
    assert_eq!(kind.code(), code);
    let decoded = RecordBody::decode(kind, bytes);
    let core_ok = match kind {
        RecordKind::Item => ItemCiphertext::from_bytes(bytes).is_ok(),
        RecordKind::Tombstone => TombstoneCiphertext::from_bytes(bytes).is_ok(),
    };
    assert_eq!(decoded.is_ok(), core_ok);
    let Ok(body) = decoded else {
        return;
    };
    assert_eq!(body.as_bytes(), bytes);
    assert_eq!(body.kind(), kind);

    STORE.with_borrow_mut(|store| {
        // One throwaway vault per input, at the body's generation, since a
        // vault accepts records only at its accepted generation.
        let generation = body.key_generation();
        let mut vid = [0xB0; 16];
        vid[12..].copy_from_slice(&generation.get().to_be_bytes());
        let vault = VaultId::from_bytes(vid).expect("id");
        // Structurally valid; the store never unwraps it.
        let mut wrapped = [0u8; WrappedKey::LEN];
        wrapped[..2].copy_from_slice(&[0x01, 0x01]);
        let wrapped = WrappedKey::from_bytes(&wrapped).expect("wrapped key");
        store
            .create_vault(&vault, generation, &wrapped)
            .expect("create vault");
        let id = ItemId::from_bytes([0x0C; 16]).expect("id");
        store
            .upsert_record(&vault, &id, &body, Origin::Remote { server_seq: 1 })
            .expect("upsert");
        let back = store.record(&vault, &id).expect("read").expect("present");
        assert_eq!(back.body, body);
        assert_eq!(back.server_seq, Some(1));
        let listed = store.records(&vault).expect("list");
        assert_eq!(listed, vec![Ok(back)]);
        // Nothing is pending (a remote record), so a refusing delete works.
        // The vault keeps its generation floor; the next input with this
        // generation re-creates it at the same generation, which is allowed.
        assert!(store
            .delete_vault(&vault, PendingPolicy::Refuse)
            .expect("delete"));
        assert!(store.vault(&vault).expect("read").is_none());
    });
});
