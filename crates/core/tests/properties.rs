//! Property tests over the public API (CRYPTO_SPEC §14): round trips for
//! every object type, single-bit tampering anywhere in an object fails, and
//! opening under any other context (IDs, schema version, key generation,
//! unlock method) fails.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test helpers outside #[test] functions"
)]

use mosaic_core::encoding::{pad, padded_len, unpad};
use mosaic_core::{
    AccountId, AccountKey, AttachmentId, AttachmentKey, DeviceId, Error, ItemCiphertext, ItemId,
    Kek, KeyGeneration, SecretBytes, TombstoneCiphertext, UnlockMethod, VaultId, VaultKey,
    VaultMetaCiphertext, WrappedKey, open_attachment, seal_attachment,
};
use proptest::prelude::*;

fn id16() -> impl Strategy<Value = [u8; 16]> {
    any::<[u8; 16]>().prop_filter("nil UUID is reserved", |b| *b != [0u8; 16])
}

fn generation() -> impl Strategy<Value = KeyGeneration> {
    (1u32..=u32::MAX).prop_map(|g| KeyGeneration::new(g).unwrap())
}

/// Flips one bit and returns the tampered bytes and the index of the
/// flipped byte.
fn flip(bytes: &[u8], bit: usize) -> (Vec<u8>, usize) {
    let mut v = bytes.to_vec();
    let bit = bit % (v.len() * 8);
    v[bit / 8] ^= 1 << (bit % 8);
    (v, bit / 8)
}

/// The exact error a single-bit flip at `byte` must cause when the object
/// is decoded and opened under its correct key and context. `gen` is the
/// position of the header's `key_generation:u32`, if it has one. Every flip
/// past the strictly checked header is an authentication failure, which
/// must be [`Error::Decryption`] and nothing else.
fn tamper_error(tampered: &[u8], byte: usize, gen_at: Option<usize>) -> Error {
    match (byte, gen_at) {
        (0, _) => Error::UnsupportedVersion,
        (1, _) => Error::UnsupportedAlgorithm,
        (b, Some(g)) if (g..g + 4).contains(&b) => {
            let generation = u32::from_be_bytes(tampered[g..g + 4].try_into().unwrap());
            if generation == 0 {
                Error::InvalidKeyGeneration
            } else {
                Error::KeyGenerationMismatch
            }
        }
        _ => Error::Decryption,
    }
}

/// Item header: `version alg schema:u16 key_generation:u32`.
const ITEM_GEN_AT: Option<usize> = Some(4);
/// Vault-metadata and tombstone header: `version alg key_generation:u32`.
const GEN_GEN_AT: Option<usize> = Some(2);

/// Attachment header: `version alg chunk_log2`; `chunk_log2` is checked like
/// the algorithm.
fn attachment_tamper_error(byte: usize) -> Error {
    match byte {
        0 => Error::UnsupportedVersion,
        1 | 2 => Error::UnsupportedAlgorithm,
        _ => Error::Decryption,
    }
}

fn other_id(id: [u8; 16], tweak: u8) -> [u8; 16] {
    let mut o = id;
    o[usize::from(tweak % 16)] ^= 1 << (tweak % 8);
    if o == [0u8; 16] {
        o[0] = 1;
    }
    o
}

fn kek() -> Kek {
    Kek::from_secret(SecretBytes::random().unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn pad_unpad_round_trip(msg in proptest::collection::vec(any::<u8>(), 0..3000)) {
        let p = pad(&msg).unwrap();
        prop_assert_eq!(p.len(), padded_len(msg.len()).unwrap());
        prop_assert!(p.len() >= 512 && p.len().is_multiple_of(256));
        prop_assert_eq!(unpad(p.expose_secret()).unwrap(), msg.as_slice());
    }

    #[test]
    fn unpad_rejects_corruption(
        msg in proptest::collection::vec(any::<u8>(), 0..1000),
        pos in any::<usize>(),
        val in 1u8..=255,
    ) {
        let p = pad(&msg).unwrap();
        let mut bad = p.expose_secret().to_vec();
        // Corrupt one byte of the padding region (marker or filler).
        let region = bad.len() - msg.len();
        let i = msg.len() + pos % region;
        bad[i] ^= val;
        // Either rejected, or (only when the marker byte moved) not the
        // original message.
        if let Ok(out) = unpad(&bad) {
            prop_assert_ne!(out, msg.as_slice());
        }
    }

    #[test]
    fn unpad_rejects_arbitrary_input_or_is_canonical(data in proptest::collection::vec(any::<u8>(), 0..1600)) {
        if let Ok(msg) = unpad(&data) {
            prop_assert_eq!(pad(msg).unwrap().expose_secret().to_vec(), data.as_slice());
        }
    }

    #[test]
    fn wrapped_account_key(
        account in id16(),
        device in id16(),
        cred in proptest::collection::vec(any::<u8>(), 1..64),
        method_sel in 0u8..3,
        bit in any::<usize>(),
        tweak in any::<u8>(),
    ) {
        let account_id = AccountId::from_bytes(account).unwrap();
        let method = match method_sel {
            0 => UnlockMethod::Password,
            1 => UnlockMethod::QuickUnlock(DeviceId::from_bytes(device).unwrap()),
            _ => UnlockMethod::hardware_key(&cred),
        };
        let w = kek();
        let ak = AccountKey::generate().unwrap();
        let wrapped = ak.wrap(&w, &account_id, &method).unwrap();
        prop_assert_eq!(wrapped.as_bytes().len(), WrappedKey::LEN);

        // Round trip through the serialized form.
        let decoded = WrappedKey::from_bytes(wrapped.as_bytes()).unwrap();
        prop_assert_eq!(&decoded.unwrap_account_key(&w, &account_id, &method).unwrap(), &ak);

        // Any single bit flip (header, nonce, ciphertext, tag) fails, with
        // the header error for the two header bytes and `Decryption` for
        // everything else.
        let (tampered, byte) = flip(wrapped.as_bytes(), bit);
        let r = WrappedKey::from_bytes(&tampered)
            .and_then(|k| k.unwrap_account_key(&w, &account_id, &method));
        prop_assert_eq!(r, Err(tamper_error(&tampered, byte, None)));

        // Wrong account, method byte or method_id fails.
        let other_account = AccountId::from_bytes(other_id(account, tweak)).unwrap();
        prop_assert_eq!(
            decoded.unwrap_account_key(&w, &other_account, &method).unwrap_err(),
            Error::Decryption
        );
        let other_methods = [
            UnlockMethod::Password,
            UnlockMethod::QuickUnlock(DeviceId::from_bytes(other_id(device, tweak)).unwrap()),
            UnlockMethod::QuickUnlock(DeviceId::from_bytes(device).unwrap()),
            UnlockMethod::HardwareKey(method.method_id(&account_id)),
            UnlockMethod::HardwareKey(other_id(method.method_id(&account_id), tweak)),
        ];
        for m in other_methods.iter().filter(|m| **m != method) {
            // Password and a method with the same id bytes as account_id still
            // differ in the method byte, so every distinct method fails.
            prop_assert_eq!(
                decoded.unwrap_account_key(&w, &account_id, m).unwrap_err(),
                Error::Decryption
            );
        }
        prop_assert_eq!(
            decoded.unwrap_account_key(&kek(), &account_id, &method).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn wrapped_vault_key(
        account in id16(),
        vault in id16(),
        kg in generation(),
        bit in any::<usize>(),
        tweak in any::<u8>(),
        gen_delta in 1u32..1000,
    ) {
        let account_id = AccountId::from_bytes(account).unwrap();
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let ak = AccountKey::generate().unwrap();
        let vk = VaultKey::generate(kg).unwrap();
        let wrapped = WrappedKey::from_bytes(vk.wrap(&ak, &account_id, &vault_id).unwrap().as_bytes()).unwrap();
        let back = wrapped.unwrap_vault_key(&ak, &account_id, &vault_id, kg).unwrap();
        prop_assert_eq!(&back, &vk);
        prop_assert_eq!(back.generation(), kg);

        let (tampered, byte) = flip(wrapped.as_bytes(), bit);
        prop_assert_eq!(
            WrappedKey::from_bytes(&tampered)
                .and_then(|k| k.unwrap_vault_key(&ak, &account_id, &vault_id, kg)),
            Err(tamper_error(&tampered, byte, None))
        );

        let other_vault = VaultId::from_bytes(other_id(vault, tweak)).unwrap();
        prop_assert_eq!(
            wrapped.unwrap_vault_key(&ak, &account_id, &other_vault, kg),
            Err(Error::Decryption)
        );
        let other_account = AccountId::from_bytes(other_id(account, tweak)).unwrap();
        prop_assert_eq!(
            wrapped.unwrap_vault_key(&ak, &other_account, &vault_id, kg),
            Err(Error::Decryption)
        );
        prop_assert_eq!(
            wrapped.unwrap_vault_key(&AccountKey::generate().unwrap(), &account_id, &vault_id, kg),
            Err(Error::Decryption)
        );
        let other_gen = KeyGeneration::new(kg.get().wrapping_add(gen_delta).max(1)).unwrap();
        prop_assume!(other_gen != kg);
        prop_assert_eq!(
            wrapped.unwrap_vault_key(&ak, &account_id, &vault_id, other_gen).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn item_ciphertext(
        vault in id16(),
        item in id16(),
        schema in any::<u16>(),
        kg in generation(),
        pt in proptest::collection::vec(any::<u8>(), 0..1500),
        bit in any::<usize>(),
        tweak in any::<u8>(),
    ) {
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let item_id = ItemId::from_bytes(item).unwrap();
        let vk = VaultKey::generate(kg).unwrap();
        let ct = ItemCiphertext::seal(&vk, &vault_id, &item_id, schema, &pt).unwrap();
        let bytes = ct.as_bytes();
        prop_assert_eq!(bytes.len(), 8 + 24 + padded_len(pt.len()).unwrap() + 16);

        let decoded = ItemCiphertext::from_bytes(bytes).unwrap();
        prop_assert_eq!(decoded.schema_version(), schema);
        prop_assert_eq!(decoded.key_generation(), kg);
        prop_assert_eq!(decoded.open(&vk, &vault_id, &item_id).unwrap().expose_secret().to_vec(), pt.as_slice());

        // Single bit flips anywhere fail (decode or open), each with its
        // exact error.
        let (tampered, byte) = flip(bytes, bit);
        prop_assert_eq!(
            ItemCiphertext::from_bytes(&tampered).and_then(|c| c.open(&vk, &vault_id, &item_id)),
            Err(tamper_error(&tampered, byte, ITEM_GEN_AT))
        );

        // Swapped context fails authentication.
        let other_vault = VaultId::from_bytes(other_id(vault, tweak)).unwrap();
        let other_item = ItemId::from_bytes(other_id(item, tweak)).unwrap();
        prop_assert_eq!(decoded.open(&vk, &other_vault, &item_id).unwrap_err(), Error::Decryption);
        prop_assert_eq!(decoded.open(&vk, &vault_id, &other_item).unwrap_err(), Error::Decryption);
        // vault_id and item_id swapped.
        if vault != item {
            prop_assert_eq!(
                decoded.open(&vk, &VaultId::from_bytes(item).unwrap(), &ItemId::from_bytes(vault).unwrap()),
                Err(Error::Decryption)
            );
        }

        // A different schema_version in the header fails, even though the
        // header is well-formed.
        let mut reschema = bytes.to_vec();
        let other_schema = schema ^ (1 << (tweak % 16));
        reschema[2..4].copy_from_slice(&other_schema.to_be_bytes());
        let c = ItemCiphertext::from_bytes(&reschema).unwrap();
        prop_assert_eq!(c.open(&vk, &vault_id, &item_id).unwrap_err(), Error::Decryption);

        // A header claiming another generation is refused for this key, and
        // a key of another generation is refused for this header.
        let other_gen = kg.get() ^ (1 << (tweak % 32));
        prop_assume!(other_gen != 0);
        let mut regen = bytes.to_vec();
        regen[4..8].copy_from_slice(&other_gen.to_be_bytes());
        let c = ItemCiphertext::from_bytes(&regen).unwrap();
        prop_assert_eq!(c.open(&vk, &vault_id, &item_id).unwrap_err(), Error::KeyGenerationMismatch);
        let other_vk = VaultKey::generate(KeyGeneration::new(other_gen).unwrap()).unwrap();
        prop_assert_eq!(decoded.open(&other_vk, &vault_id, &item_id).unwrap_err(), Error::KeyGenerationMismatch);
        // Same generation, different key.
        let same_gen = VaultKey::generate(kg).unwrap();
        prop_assert_eq!(decoded.open(&same_gen, &vault_id, &item_id).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn vault_meta(
        vault in id16(),
        kg in generation(),
        meta in proptest::collection::vec(any::<u8>(), 0..800),
        bit in any::<usize>(),
        tweak in any::<u8>(),
    ) {
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let vk = VaultKey::generate(kg).unwrap();
        let ct = VaultMetaCiphertext::seal(&vk, &vault_id, &meta).unwrap();
        let decoded = VaultMetaCiphertext::from_bytes(ct.as_bytes()).unwrap();
        prop_assert_eq!(decoded.key_generation(), kg);
        prop_assert_eq!(decoded.open(&vk, &vault_id).unwrap().expose_secret().to_vec(), meta.as_slice());

        let (tampered, byte) = flip(ct.as_bytes(), bit);
        prop_assert_eq!(
            VaultMetaCiphertext::from_bytes(&tampered).and_then(|c| c.open(&vk, &vault_id)),
            Err(tamper_error(&tampered, byte, GEN_GEN_AT))
        );
        let other_vault = VaultId::from_bytes(other_id(vault, tweak)).unwrap();
        prop_assert_eq!(decoded.open(&vk, &other_vault).unwrap_err(), Error::Decryption);
        let other_gen = kg.get() ^ (1 << (tweak % 32));
        prop_assume!(other_gen != 0);
        let other_vk = VaultKey::generate(KeyGeneration::new(other_gen).unwrap()).unwrap();
        prop_assert_eq!(decoded.open(&other_vk, &vault_id).unwrap_err(), Error::KeyGenerationMismatch);
    }

    #[test]
    fn tombstone(
        vault in id16(),
        item in id16(),
        kg in generation(),
        body in proptest::collection::vec(any::<u8>(), 0..300),
        bit in any::<usize>(),
        tweak in any::<u8>(),
    ) {
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let item_id = ItemId::from_bytes(item).unwrap();
        let vk = VaultKey::generate(kg).unwrap();
        let ct = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, &body).unwrap();
        prop_assert_eq!(ct.as_bytes().len(), 6 + 24 + body.len() + 16);
        let decoded = TombstoneCiphertext::from_bytes(ct.as_bytes()).unwrap();
        prop_assert_eq!(decoded.key_generation(), kg);
        prop_assert_eq!(decoded.open(&vk, &vault_id, &item_id).unwrap().expose_secret().to_vec(), body.as_slice());

        let (tampered, byte) = flip(ct.as_bytes(), bit);
        prop_assert_eq!(
            TombstoneCiphertext::from_bytes(&tampered).and_then(|c| c.open(&vk, &vault_id, &item_id)),
            Err(tamper_error(&tampered, byte, GEN_GEN_AT))
        );
        let other_vault = VaultId::from_bytes(other_id(vault, tweak)).unwrap();
        let other_item = ItemId::from_bytes(other_id(item, tweak)).unwrap();
        prop_assert_eq!(decoded.open(&vk, &other_vault, &item_id).unwrap_err(), Error::Decryption);
        prop_assert_eq!(decoded.open(&vk, &vault_id, &other_item).unwrap_err(), Error::Decryption);
        let other_gen = kg.get() ^ (1 << (tweak % 32));
        prop_assume!(other_gen != 0);
        let other_vk = VaultKey::generate(KeyGeneration::new(other_gen).unwrap()).unwrap();
        prop_assert_eq!(decoded.open(&other_vk, &vault_id, &item_id).unwrap_err(), Error::KeyGenerationMismatch);
    }

    #[test]
    fn item_tombstone_and_meta_are_not_interchangeable(
        vault in id16(),
        item in id16(),
        schema in any::<u16>(),
        pt in proptest::collection::vec(any::<u8>(), 0..100),
        tomb_pt in proptest::collection::vec(any::<u8>(), 512),
    ) {
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let item_id = ItemId::from_bytes(item).unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        let meta = VaultMetaCiphertext::seal(&vk, &vault_id, &pt).unwrap();
        // A 512-byte tombstone has a 528-byte body, which is also a valid
        // padded body, so every cross-decode below parses and only the
        // label in the AAD keeps the types apart.
        let tomb = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, &tomb_pt).unwrap();
        let as_tomb = TombstoneCiphertext::from_bytes(meta.as_bytes()).unwrap();
        prop_assert_eq!(as_tomb.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
        let as_meta = VaultMetaCiphertext::from_bytes(tomb.as_bytes()).unwrap();
        prop_assert_eq!(as_meta.open(&vk, &vault_id), Err(Error::Decryption));

        // Item <-> tombstone: remove or insert the schema_version so the
        // bytes parse with the other header layout.
        let item_ct = ItemCiphertext::seal(&vk, &vault_id, &item_id, schema, &pt).unwrap();
        let mut stripped = item_ct.as_bytes().to_vec();
        stripped.drain(2..4);
        let as_tomb = TombstoneCiphertext::from_bytes(&stripped).unwrap();
        prop_assert_eq!(as_tomb.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
        let mut widened = tomb.as_bytes().to_vec();
        widened.splice(2..2, schema.to_be_bytes());
        let as_item = ItemCiphertext::from_bytes(&widened).unwrap();
        prop_assert_eq!(as_item.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
        // Item <-> vault metadata, the same way.
        let as_meta = VaultMetaCiphertext::from_bytes(&stripped).unwrap();
        prop_assert_eq!(as_meta.open(&vk, &vault_id), Err(Error::Decryption));
        let mut meta_widened = meta.as_bytes().to_vec();
        meta_widened.splice(2..2, schema.to_be_bytes());
        let as_item = ItemCiphertext::from_bytes(&meta_widened).unwrap();
        prop_assert_eq!(as_item.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
    }
}

/// Inputs for the decoders that get past the length and header checks: a
/// valid header prefix for each object type, then a random tail whose length
/// lands on or next to the boundaries the decoders check.
fn decoder_input() -> impl Strategy<Value = Vec<u8>> {
    let prefix = prop_oneof![
        Just(vec![0x01, 0x01]),                       // wrapped key
        Just(vec![0x01, 0x01, 0, 0, 0, 1]),           // meta, tombstone
        Just(vec![0x01, 0x01, 0, 1, 0, 0, 0, 1]),     // item
        Just(vec![0x01, 0x02, 0x10]),                 // attachment
        proptest::collection::vec(any::<u8>(), 0..8), // anything
    ];
    // Bytes after the header: nonce (24) + body, or prefix (19) + chunks.
    let tail_len = prop_oneof![
        Just(24 + 48),
        Just(24 + 511),
        Just(24 + 512),
        Just(24 + 527),
        Just(24 + 528),
        Just(24 + 784),
        Just(24 + 1040),
        Just(19 + 15),
        Just(19 + 16),
        Just(19 + 65_552),
        Just(19 + 65_553),
        0usize..2000,
    ];
    (prefix, tail_len).prop_flat_map(|(prefix, len)| {
        proptest::collection::vec(any::<u8>(), len).prop_map(move |tail| {
            let mut v = prefix.clone();
            v.extend(tail);
            v
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn attachment(
        vault in id16(),
        item in id16(),
        attachment in id16(),
        len in prop_oneof![0usize..200, 65_530usize..65_545, 131_060usize..131_080],
        seed in any::<u8>(),
        bit in any::<usize>(),
        tweak in any::<u8>(),
    ) {
        let vault_id = VaultId::from_bytes(vault).unwrap();
        let item_id = ItemId::from_bytes(item).unwrap();
        let att_id = AttachmentId::from_bytes(attachment).unwrap();
        let fk = AttachmentKey::generate().unwrap();
        let pt: Vec<u8> = (0..len).map(|i| seed.wrapping_add(u8::try_from(i % 256).unwrap())).collect();
        let ct = seal_attachment(&fk, &vault_id, &item_id, &att_id, &pt).unwrap();
        prop_assert_eq!(
            ct.len(),
            mosaic_core::attachment::ciphertext_len(len).unwrap()
        );
        prop_assert_eq!(
            open_attachment(&fk, &vault_id, &item_id, &att_id, &ct).unwrap().expose_secret().to_vec(),
            pt.as_slice()
        );

        let (tampered, byte) = flip(&ct, bit);
        prop_assert_eq!(
            open_attachment(&fk, &vault_id, &item_id, &att_id, &tampered),
            Err(attachment_tamper_error(byte))
        );

        let ov = VaultId::from_bytes(other_id(vault, tweak)).unwrap();
        let oi = ItemId::from_bytes(other_id(item, tweak)).unwrap();
        let oa = AttachmentId::from_bytes(other_id(attachment, tweak)).unwrap();
        prop_assert_eq!(open_attachment(&fk, &ov, &item_id, &att_id, &ct).unwrap_err(), Error::Decryption);
        prop_assert_eq!(open_attachment(&fk, &vault_id, &oi, &att_id, &ct).unwrap_err(), Error::Decryption);
        prop_assert_eq!(open_attachment(&fk, &vault_id, &item_id, &oa, &ct).unwrap_err(), Error::Decryption);
        prop_assert_eq!(
            open_attachment(&AttachmentKey::generate().unwrap(), &vault_id, &item_id, &att_id, &ct).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn decoders_never_panic(data in decoder_input()) {
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        let ak = AccountKey::generate().unwrap();
        let ids = (VaultId::from_bytes([1; 16]).unwrap(), ItemId::from_bytes([2; 16]).unwrap());
        let account = AccountId::from_bytes([3; 16]).unwrap();
        // Random bytes never authenticate. With generation-1 headers and a
        // generation-1 key, a parsed object reaches the AEAD, so the only
        // possible open error is `Decryption`.
        if let Ok(w) = WrappedKey::from_bytes(&data) {
            prop_assert_eq!(w.as_bytes().as_slice(), data.as_slice());
            prop_assert_eq!(
                w.unwrap_account_key(&kek(), &account, &UnlockMethod::Password),
                Err(Error::Decryption)
            );
            prop_assert_eq!(
                w.unwrap_vault_key(&ak, &account, &ids.0, KeyGeneration::FIRST),
                Err(Error::Decryption)
            );
        }
        let open_err = |g: KeyGeneration| {
            if g == KeyGeneration::FIRST { Error::Decryption } else { Error::KeyGenerationMismatch }
        };
        if let Ok(c) = ItemCiphertext::from_bytes(&data) {
            prop_assert_eq!(c.as_bytes(), data.as_slice());
            prop_assert_eq!(c.open(&vk, &ids.0, &ids.1), Err(open_err(c.key_generation())));
        }
        if let Ok(c) = VaultMetaCiphertext::from_bytes(&data) {
            prop_assert_eq!(c.as_bytes(), data.as_slice());
            prop_assert_eq!(c.open(&vk, &ids.0), Err(open_err(c.key_generation())));
        }
        if let Ok(c) = TombstoneCiphertext::from_bytes(&data) {
            prop_assert_eq!(c.as_bytes(), data.as_slice());
            prop_assert_eq!(c.open(&vk, &ids.0, &ids.1), Err(open_err(c.key_generation())));
        }
        let fk = AttachmentKey::generate().unwrap();
        let att = AttachmentId::from_bytes([4; 16]).unwrap();
        let r = open_attachment(&fk, &ids.0, &ids.1, &att, &data);
        prop_assert!(matches!(
            r,
            Err(Error::Decryption | Error::Malformed | Error::UnsupportedVersion | Error::UnsupportedAlgorithm)
        ));
    }
}

/// The decoder strategy really reaches the AEAD for every object type (the
/// property above would otherwise pass vacuously).
#[test]
fn decoder_input_reaches_open() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = decoder_input();
    let mut hits = [0usize; 5];
    for _ in 0..2000 {
        let data = strategy.new_tree(&mut runner).unwrap().current();
        hits[0] += usize::from(WrappedKey::from_bytes(&data).is_ok());
        hits[1] += usize::from(ItemCiphertext::from_bytes(&data).is_ok());
        hits[2] += usize::from(VaultMetaCiphertext::from_bytes(&data).is_ok());
        hits[3] += usize::from(TombstoneCiphertext::from_bytes(&data).is_ok());
        let fk = AttachmentKey::generate().unwrap();
        let (v, i, a) = (
            VaultId::from_bytes([1; 16]).unwrap(),
            ItemId::from_bytes([2; 16]).unwrap(),
            AttachmentId::from_bytes([4; 16]).unwrap(),
        );
        hits[4] += usize::from(open_attachment(&fk, &v, &i, &a, &data) == Err(Error::Decryption));
    }
    assert!(hits.iter().all(|&h| h >= 20), "decoder coverage {hits:?}");
}

#[test]
fn decoders_are_strict() {
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let vault_id = VaultId::new_v7().unwrap();
    let item_id = ItemId::new_v7().unwrap();
    let ct = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, b"x").unwrap();
    let b = ct.as_bytes();
    // Version and algorithm.
    let mut v = b.to_vec();
    v[0] = 0x02;
    assert_eq!(
        ItemCiphertext::from_bytes(&v),
        Err(Error::UnsupportedVersion)
    );
    let mut v = b.to_vec();
    v[1] = 0x02;
    assert_eq!(
        ItemCiphertext::from_bytes(&v),
        Err(Error::UnsupportedAlgorithm)
    );
    // Key generation 0 is invalid.
    let mut v = b.to_vec();
    v[4..8].copy_from_slice(&[0; 4]);
    assert_eq!(
        ItemCiphertext::from_bytes(&v),
        Err(Error::InvalidKeyGeneration)
    );
    // Lengths: truncated, trailing byte, short.
    assert_eq!(
        ItemCiphertext::from_bytes(&b[..b.len() - 1]),
        Err(Error::Malformed)
    );
    let mut v = b.to_vec();
    v.push(0);
    assert_eq!(ItemCiphertext::from_bytes(&v), Err(Error::Malformed));
    assert_eq!(ItemCiphertext::from_bytes(&b[..1]), Err(Error::Malformed));
    assert_eq!(ItemCiphertext::from_bytes(&b[..31]), Err(Error::Malformed));
    assert_eq!(ItemCiphertext::from_bytes(&[]), Err(Error::Malformed));
    // Size limit.
    let big = vec![0u8; mosaic_core::envelope::MAX_ITEM_PLAINTEXT_LEN + 1];
    assert_eq!(
        ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, &big),
        Err(Error::SizeLimit)
    );
    let mut huge = b[..32].to_vec();
    huge.resize(
        32 + mosaic_core::envelope::MAX_ITEM_PADDED_LEN + 256 + 16,
        0,
    );
    assert_eq!(ItemCiphertext::from_bytes(&huge), Err(Error::SizeLimit));
    let max = vec![0u8; mosaic_core::envelope::MAX_ITEM_PLAINTEXT_LEN];
    let ct = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, &max).unwrap();
    let back = ItemCiphertext::from_bytes(ct.as_bytes()).unwrap();
    assert_eq!(
        back.open(&vk, &vault_id, &item_id).unwrap().len(),
        max.len()
    );
    // Exactly at the limit decodes; the zero body then fails the tag.
    let mut at_limit = b[..32].to_vec();
    at_limit.resize(32 + mosaic_core::envelope::MAX_ITEM_PADDED_LEN + 16, 0);
    let c = ItemCiphertext::from_bytes(&at_limit).unwrap();
    assert_eq!(c.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
    // Padded-body structure: a 256-byte block below the 512 minimum, one
    // byte past a block boundary, and a body shorter than a tag.
    for body in [256 + 16, 512 + 16 + 1, 15] {
        let mut v = b[..32].to_vec();
        v.resize(32 + body, 0);
        assert_eq!(
            ItemCiphertext::from_bytes(&v),
            Err(Error::Malformed),
            "{body}"
        );
    }

    // Wrapped keys are exactly 74 bytes.
    let ak = AccountKey::generate().unwrap();
    let account = AccountId::new_v7().unwrap();
    let w = ak.wrap(&kek(), &account, &UnlockMethod::Password).unwrap();
    let wb = w.as_bytes();
    assert_eq!(WrappedKey::from_bytes(&wb[..73]), Err(Error::Malformed));
    let mut v = wb.to_vec();
    v.push(0);
    assert_eq!(WrappedKey::from_bytes(&v), Err(Error::Malformed));
    let mut v = wb.to_vec();
    v[0] = 0;
    assert_eq!(WrappedKey::from_bytes(&v), Err(Error::UnsupportedVersion));
    let mut v = wb.to_vec();
    v[1] = 2;
    assert_eq!(WrappedKey::from_bytes(&v), Err(Error::UnsupportedAlgorithm));
    assert_eq!(WrappedKey::from_bytes(&[]), Err(Error::Malformed));
    assert_eq!(WrappedKey::from_bytes(&[1]), Err(Error::Malformed));
    let mut exact = vec![1, 1];
    exact.extend_from_slice(&[0u8; 72]);
    let w = WrappedKey::from_bytes(&exact).unwrap();
    assert_eq!(
        w.unwrap_account_key(&kek(), &account, &UnlockMethod::Password),
        Err(Error::Decryption)
    );

    // Vault metadata and tombstones.
    let meta = VaultMetaCiphertext::seal(&vk, &vault_id, b"m").unwrap();
    let mb = meta.as_bytes();
    assert_eq!(
        VaultMetaCiphertext::from_bytes(&mb[..mb.len() - 1]),
        Err(Error::Malformed)
    );
    for body in [256 + 16, 512 + 16 + 1, 15] {
        let mut v = mb[..30].to_vec();
        v.resize(30 + body, 0);
        assert_eq!(
            VaultMetaCiphertext::from_bytes(&v),
            Err(Error::Malformed),
            "{body}"
        );
    }
    let mut at_limit = mb[..30].to_vec();
    at_limit.resize(
        30 + mosaic_core::envelope::MAX_VAULT_META_PADDED_LEN + 16,
        0,
    );
    let c = VaultMetaCiphertext::from_bytes(&at_limit).unwrap();
    assert_eq!(c.open(&vk, &vault_id), Err(Error::Decryption));
    // Header checks for both 6-byte-header types.
    let tomb = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, b"t").unwrap();
    for bytes in [mb, tomb.as_bytes()] {
        let mut v = bytes.to_vec();
        v[0] = 2;
        assert_eq!(
            VaultMetaCiphertext::from_bytes(&v),
            Err(Error::UnsupportedVersion)
        );
        assert_eq!(
            TombstoneCiphertext::from_bytes(&v),
            Err(Error::UnsupportedVersion)
        );
        let mut v = bytes.to_vec();
        v[1] = 2;
        assert_eq!(
            VaultMetaCiphertext::from_bytes(&v),
            Err(Error::UnsupportedAlgorithm)
        );
        assert_eq!(
            TombstoneCiphertext::from_bytes(&v),
            Err(Error::UnsupportedAlgorithm)
        );
        let mut v = bytes.to_vec();
        v[2..6].copy_from_slice(&[0; 4]);
        // The generation is checked before the body structure.
        assert_eq!(
            VaultMetaCiphertext::from_bytes(&v),
            Err(Error::InvalidKeyGeneration)
        );
        assert_eq!(
            TombstoneCiphertext::from_bytes(&v),
            Err(Error::InvalidKeyGeneration)
        );
        assert_eq!(
            TombstoneCiphertext::from_bytes(&v[..1]),
            Err(Error::Malformed)
        );
        assert_eq!(
            VaultMetaCiphertext::from_bytes(&v[..1]),
            Err(Error::Malformed)
        );
    }
    let big = vec![0u8; mosaic_core::envelope::MAX_VAULT_META_PLAINTEXT_LEN + 1];
    assert_eq!(
        VaultMetaCiphertext::seal(&vk, &vault_id, &big),
        Err(Error::SizeLimit)
    );
    let mut huge = mb[..30].to_vec();
    huge.resize(
        30 + mosaic_core::envelope::MAX_VAULT_META_PADDED_LEN + 256 + 16,
        0,
    );
    assert_eq!(
        VaultMetaCiphertext::from_bytes(&huge),
        Err(Error::SizeLimit)
    );
    let tomb = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, b"").unwrap();
    let tb = tomb.as_bytes();
    assert_eq!(tb.len(), 46);
    assert_eq!(
        TombstoneCiphertext::from_bytes(&tb[..45]),
        Err(Error::Malformed)
    );
    let big = vec![0u8; mosaic_core::envelope::MAX_TOMBSTONE_PLAINTEXT_LEN + 1];
    assert_eq!(
        TombstoneCiphertext::seal(&vk, &vault_id, &item_id, &big),
        Err(Error::SizeLimit)
    );
    let mut huge = tb[..30].to_vec();
    huge.resize(
        30 + mosaic_core::envelope::MAX_TOMBSTONE_PLAINTEXT_LEN + 1 + 16,
        0,
    );
    assert_eq!(
        TombstoneCiphertext::from_bytes(&huge),
        Err(Error::SizeLimit)
    );
}

/// Two seals of the same input under the same key and context must use
/// different, non-zero nonces (or STREAM nonce prefixes) at `range`, and so
/// produce different ciphertexts. A constant nonce would pass every round
/// trip, tamper and golden test, so this is the only guard against nonce
/// reuse in each public seal path.
fn assert_fresh_nonce(what: &str, a: &[u8], b: &[u8], range: core::ops::Range<usize>) {
    let (na, nb) = (&a[range.clone()], &b[range]);
    assert!(na.iter().any(|&x| x != 0), "{what}: all-zero nonce");
    assert!(nb.iter().any(|&x| x != 0), "{what}: all-zero nonce");
    assert_ne!(na, nb, "{what}: nonce reused");
    assert_ne!(a, b, "{what}: identical ciphertexts");
}

#[test]
fn every_seal_uses_a_fresh_nonce() {
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let ak = AccountKey::generate().unwrap();
    let w = kek();
    let account = AccountId::new_v7().unwrap();
    let vault_id = VaultId::new_v7().unwrap();
    let item_id = ItemId::new_v7().unwrap();
    let att_id = AttachmentId::new_v7().unwrap();

    let a = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, b"same").unwrap();
    let b = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, b"same").unwrap();
    assert_fresh_nonce("item", a.as_bytes(), b.as_bytes(), 8..32);

    let a = ak.wrap(&w, &account, &UnlockMethod::Password).unwrap();
    let b = ak.wrap(&w, &account, &UnlockMethod::Password).unwrap();
    assert_fresh_nonce("wrapped AK", a.as_bytes(), b.as_bytes(), 2..26);

    let a = vk.wrap(&ak, &account, &vault_id).unwrap();
    let b = vk.wrap(&ak, &account, &vault_id).unwrap();
    assert_fresh_nonce("wrapped VK", a.as_bytes(), b.as_bytes(), 2..26);

    let a = VaultMetaCiphertext::seal(&vk, &vault_id, b"same").unwrap();
    let b = VaultMetaCiphertext::seal(&vk, &vault_id, b"same").unwrap();
    assert_fresh_nonce("vault metadata", a.as_bytes(), b.as_bytes(), 6..30);

    let a = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, b"same").unwrap();
    let b = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, b"same").unwrap();
    assert_fresh_nonce("tombstone", a.as_bytes(), b.as_bytes(), 6..30);

    let fk = AttachmentKey::generate().unwrap();
    let a = seal_attachment(&fk, &vault_id, &item_id, &att_id, b"same").unwrap();
    let b = seal_attachment(&fk, &vault_id, &item_id, &att_id, b"same").unwrap();
    assert_fresh_nonce("attachment", &a, &b, 3..22);
}

/// Each size limit is inclusive: exactly the limit seals, decodes and opens
/// (or reaches the AEAD), one byte more is `SizeLimit`. Off-by-one changes
/// to any of the comparisons fail here.
#[test]
fn size_limits_are_inclusive() {
    use mosaic_core::envelope::{
        MAX_TOMBSTONE_PLAINTEXT_LEN, MAX_VAULT_META_PADDED_LEN, MAX_VAULT_META_PLAINTEXT_LEN,
    };
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let vault_id = VaultId::new_v7().unwrap();
    let item_id = ItemId::new_v7().unwrap();

    // Tombstone seal at the limit round-trips.
    let max = vec![0x5Au8; MAX_TOMBSTONE_PLAINTEXT_LEN];
    let t = TombstoneCiphertext::seal(&vk, &vault_id, &item_id, &max).unwrap();
    assert_eq!(t.as_bytes().len(), 30 + MAX_TOMBSTONE_PLAINTEXT_LEN + 16);
    let back = TombstoneCiphertext::from_bytes(t.as_bytes()).unwrap();
    assert_eq!(
        back.open(&vk, &vault_id, &item_id).unwrap().expose_secret(),
        max.as_slice()
    );
    // Tombstone decode at the limit: a zero body parses and fails the tag;
    // one more byte is over the limit.
    let mut at_limit = t.as_bytes()[..30].to_vec();
    at_limit.resize(30 + MAX_TOMBSTONE_PLAINTEXT_LEN + 16, 0);
    let c = TombstoneCiphertext::from_bytes(&at_limit).unwrap();
    assert_eq!(c.open(&vk, &vault_id, &item_id), Err(Error::Decryption));
    at_limit.push(0);
    assert_eq!(
        TombstoneCiphertext::from_bytes(&at_limit),
        Err(Error::SizeLimit)
    );
    assert_eq!(
        TombstoneCiphertext::seal(
            &vk,
            &vault_id,
            &item_id,
            &vec![0; MAX_TOMBSTONE_PLAINTEXT_LEN + 1]
        ),
        Err(Error::SizeLimit)
    );

    // Vault metadata seal at the limit fills the largest padded body.
    let max = vec![0x5Au8; MAX_VAULT_META_PLAINTEXT_LEN];
    let m = VaultMetaCiphertext::seal(&vk, &vault_id, &max).unwrap();
    assert_eq!(m.as_bytes().len(), 30 + MAX_VAULT_META_PADDED_LEN + 16);
    let back = VaultMetaCiphertext::from_bytes(m.as_bytes()).unwrap();
    assert_eq!(
        back.open(&vk, &vault_id).unwrap().expose_secret(),
        max.as_slice()
    );
    assert_eq!(
        VaultMetaCiphertext::seal(&vk, &vault_id, &vec![0; MAX_VAULT_META_PLAINTEXT_LEN + 1]),
        Err(Error::SizeLimit)
    );
}
