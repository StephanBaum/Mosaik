//! Item, vault-metadata and tombstone decoding and opening.
//!
//! - Mode 0: seal the rest of the input under the fixed key and check every
//!   round trip.
//! - Mode `0xFE`: take an authentic golden object (`data[1]` picks item, vault
//!   metadata or tombstone), apply the fuzzer's edits (XOR, truncate,
//!   extend, insert), and require the original plaintext if and only if
//!   the bytes are unchanged, otherwise one of the documented errors. This
//!   is the only way the fuzzer reaches the code after the tag check
//!   (unpadding) with authentic data.
//! - Otherwise: arbitrary input decodes strictly, re-encodes to itself, and
//!   never opens, except the exact golden objects (the keys are fixed).
//!
//! Keys are fixed (the golden keys), so a crash reproduces from its input.
#![no_main]

mod common;

use common::TAMPER;
use libfuzzer_sys::fuzz_target;
use mosaic_core::{Error, ItemCiphertext, TombstoneCiphertext, VaultMetaCiphertext};

fuzz_target!(|data: &[u8]| {
    let vault = common::vault();
    let item = common::item();

    match data.split_first() {
        Some((&0, pt)) => {
            let vk = common::vk(1);
            let schema = u16::from(pt.first().copied().unwrap_or(0));
            let ct = ItemCiphertext::seal(&vk, &vault, &item, schema, pt).expect("seal");
            let back = ItemCiphertext::from_bytes(ct.as_bytes()).expect("decode");
            assert_eq!(
                back.open(&vk, &vault, &item).expect("open").expose_secret(),
                pt
            );
            let meta = VaultMetaCiphertext::seal(&vk, &vault, pt).expect("seal");
            let back = VaultMetaCiphertext::from_bytes(meta.as_bytes()).expect("decode");
            assert_eq!(back.open(&vk, &vault).expect("open").expose_secret(), pt);
            let tomb = TombstoneCiphertext::seal(&vk, &vault, &item, pt).expect("seal");
            let back = TombstoneCiphertext::from_bytes(tomb.as_bytes()).expect("decode");
            assert_eq!(
                back.open(&vk, &vault, &item).expect("open").expose_secret(),
                pt
            );
        }
        Some((&TAMPER, rest)) => {
            let Some((&which, ops)) = rest.split_first() else {
                return;
            };
            let (golden, pt, generation): (_, &[u8], u32) = match which % 3 {
                0 => (common::ITEM, b"{\"title\":\"golden\"}", 7),
                1 => (common::VAULT_META, b"Personal", 1),
                _ => (common::TOMBSTONE, b"tombstone v1", 1),
            };
            let base = common::hex_file(golden);
            let bytes = common::mutate(&base, ops);
            let vk = common::vk(generation);
            let r = match which % 3 {
                0 => ItemCiphertext::from_bytes(&bytes).and_then(|c| c.open(&vk, &vault, &item)),
                1 => VaultMetaCiphertext::from_bytes(&bytes).and_then(|c| c.open(&vk, &vault)),
                _ => {
                    TombstoneCiphertext::from_bytes(&bytes).and_then(|c| c.open(&vk, &vault, &item))
                }
            };
            if bytes == base {
                assert_eq!(r.expect("unchanged object opens").expose_secret(), pt);
            } else {
                let e = r.expect_err("a modified object must not open");
                assert!(common::tamper_error_ok(e), "unexpected error {e:?}");
            }
        }
        _ => {
            // With fixed keys, the only inputs that may open are the exact
            // golden objects, to their golden plaintexts.
            let vk = common::vk(1);
            let vk7 = common::vk(7);
            let only_golden =
                |r: Result<mosaic_core::SecretVec, Error>, golden: &str, pt: &[u8]| match r {
                    Ok(out) => assert!(
                        data == common::hex_file(golden) && out.expose_secret() == pt,
                        "only the golden object may open"
                    ),
                    Err(e) => assert!(
                        matches!(e, Error::Decryption | Error::KeyGenerationMismatch),
                        "unexpected error {e:?}"
                    ),
                };
            if let Ok(c) = ItemCiphertext::from_bytes(data) {
                assert_eq!(c.as_bytes(), data);
                let r = c.open(&vk, &vault, &item);
                assert!(r.is_err(), "no generation-1 item is authentic");
                only_golden(r, common::ITEM, b"");
                only_golden(
                    c.open(&vk7, &vault, &item),
                    common::ITEM,
                    b"{\"title\":\"golden\"}",
                );
            }
            if let Ok(c) = VaultMetaCiphertext::from_bytes(data) {
                assert_eq!(c.as_bytes(), data);
                only_golden(c.open(&vk, &vault), common::VAULT_META, b"Personal");
            }
            if let Ok(c) = TombstoneCiphertext::from_bytes(data) {
                assert_eq!(c.as_bytes(), data);
                only_golden(
                    c.open(&vk, &vault, &item),
                    common::TOMBSTONE,
                    b"tombstone v1",
                );
            }
        }
    }
});
