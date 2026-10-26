//! The decrypted-plaintext decoders of the item model: item, tombstone and
//! vault metadata.
//!
//! Arbitrary input must never panic. Whatever decodes must re-encode to a
//! canonical form that decodes to the same state and re-encodes to the same
//! bytes, must merge with itself to itself, and every typed accessor must
//! work on it. Seeds: the golden plaintexts in `crates/core/tests/golden/`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::model::{Item, SecretField, TextField, Tombstone, VaultMeta};

fn check_item(item: &Item) {
    // The canonical form can be longer than the input (`1e2` becomes
    // `100.0`), so only the size limit may stop it.
    let json = match item.to_json() {
        Ok(j) => j,
        Err(e) => {
            assert_eq!(e, mosaic_core::Error::SizeLimit);
            return;
        }
    };
    let back = Item::from_json(json.expose_secret()).expect("canonical form decodes");
    assert_eq!(&back, item);
    assert_eq!(
        back.to_json().expect("encode").expose_secret(),
        json.expose_secret(),
        "canonical form is stable"
    );
    assert_eq!(&item.merge(item), item, "idempotent");

    // Typed views never panic, whatever the values are.
    let _ = item.item_type();
    let _ = (item.created_at(), item.modified_at(), item.trashed_at());
    for f in TextField::ALL {
        let _ = item.text(*f);
    }
    for f in SecretField::ALL {
        let _ = item.secret(*f).expect("copy");
    }
    let _ = (item.favorite(), item.tags(), item.urls(), item.custom_fields());
    let _ = (item.attachments(), item.password_history(), item.otp());
    let _ = (item.otp_counter(), item.passkey(), item.passkey_sign_count());
    let _ = (item.ssh_key(), item.unreadable_fields());
    for c in item.conflicts() {
        assert!(!c.losers.is_empty());
    }
    assert!(!(item.is_deleted() && item.deleted_concurrently().is_some()));
    match item.to_tombstone() {
        Some(t) => {
            assert!(item.is_deleted());
            if let Ok(tj) = t.to_json() {
                let t2 = Tombstone::from_json(tj.expose_secret()).expect("tombstone decodes");
                assert_eq!(&Item::from(t2), item);
            }
        }
        None => assert!(!item.is_deleted()),
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(item) = Item::from_json(data) {
        check_item(&item);
    }
    if let Ok(t) = Tombstone::from_json(data) {
        let item = Item::from(t);
        assert!(item.is_deleted());
        check_item(&item);
    }
    if let Ok(meta) = VaultMeta::from_json(data) {
        let json = meta.to_json().expect("re-encodes");
        let back = VaultMeta::from_json(json.expose_secret()).expect("decodes");
        assert_eq!(back, meta);
        assert_eq!(meta.merge(&meta), meta);
        let _ = (meta.name(), meta.icon(), meta.description(), meta.conflicts());
    }
});
