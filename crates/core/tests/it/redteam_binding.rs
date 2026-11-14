//! Red team (RT-core): AAD / label binding of the sharing and attachment
//! objects, which `properties.rs` does not cover (it tests item, tombstone
//! and vault metadata). A malicious server or folder provider holds the
//! ciphertext of every object and can file any blob under any name; these
//! tests confirm that swapping a roster for metadata, a contact book across
//! accounts, or an attachment across its (vault, item, attachment) context
//! is caught by the AEAD, exactly as CRYPTO_SPEC §3 (header binding, the
//! label registry) and §14 ("Changing `vault_id` or `item_id` causes
//! decryption to fail") require.
//!
//! Every attack here is **defended today**, so these are regression tests
//! (no `#[ignore]`). They would start failing if a label were reused or an
//! ID dropped from an AAD.

#![expect(clippy::unwrap_used, reason = "test helpers")]

use mosaic_core::{
    AccountId, AccountKey, AttachmentId, AttachmentKey, ContactsCiphertext, Error, ItemId,
    KeyGeneration, SecretBytes, VaultId, VaultKey, VaultMetaCiphertext, VaultRosterCiphertext,
    open_attachment, seal_attachment,
};

fn vk(g: u32) -> VaultKey {
    // A random key of the given generation. Tests that need the *same* key
    // for two objects generate it once and reuse it; tests of the
    // generation binding rely only on the generation number, which
    // `check_generation` compares before any crypto.
    VaultKey::generate(KeyGeneration::new(g).unwrap()).unwrap()
}

fn vault(a: u8) -> VaultId {
    VaultId::from_bytes([a; 16]).unwrap()
}

fn item(a: u8) -> ItemId {
    ItemId::from_bytes([a; 16]).unwrap()
}

/// A roster and vault metadata share the §7.1 construction (same header
/// `version || alg || key_generation`, same padded-body shape) and differ
/// only in the AAD label (`vault-roster` vs `vault-meta`). Each must refuse
/// to open as the other: otherwise a server could serve a roster where a
/// client expects metadata, or the reverse.
#[test]
fn roster_and_meta_are_not_interchangeable() {
    let v = vault(1);
    let key = vk(1); // one key, reused to seal both objects
    // 512-byte plaintexts so both bodies are valid padded bodies that parse.
    let body = vec![0xA5u8; 512];
    let roster = VaultRosterCiphertext::seal(&key, &v, &body).unwrap();
    let meta = VaultMetaCiphertext::seal(&key, &v, &body).unwrap();

    // Roster bytes, decoded and opened as metadata: parses, label mismatch
    // fails the tag.
    let as_meta = VaultMetaCiphertext::from_bytes(roster.as_bytes()).unwrap();
    assert_eq!(as_meta.open(&key, &v).unwrap_err(), Error::Decryption);

    // Metadata bytes opened as a roster: the same.
    let as_roster = VaultRosterCiphertext::from_bytes(meta.as_bytes()).unwrap();
    assert_eq!(as_roster.open(&key, &v).unwrap_err(), Error::Decryption);

    // The genuine pairings still open.
    assert_eq!(roster.open(&key, &v).unwrap().expose_secret(), &body[..]);
    assert_eq!(meta.open(&key, &v).unwrap().expose_secret(), &body[..]);
}

/// A roster is bound to its vault and key generation: a roster served for
/// another vault, or opened under a key of another generation, fails.
#[test]
fn roster_is_bound_to_vault_and_generation() {
    let body = vec![0x11u8; 300];
    let key = vk(2); // the sealing key, reused for the genuine open
    let roster = VaultRosterCiphertext::seal(&key, &vault(7), &body).unwrap();
    let decoded = VaultRosterCiphertext::from_bytes(roster.as_bytes()).unwrap();

    // Another vault: the vault ID is in the AAD.
    assert_eq!(
        decoded.open(&key, &vault(8)).unwrap_err(),
        Error::Decryption
    );
    // A key of another generation: the generation is in the header and the
    // AAD, and `check_generation` rejects it before any crypto (so a fresh
    // gen-3 key suffices).
    assert_eq!(
        decoded.open(&vk(3), &vault(7)).unwrap_err(),
        Error::KeyGenerationMismatch
    );
    // The right context opens.
    assert_eq!(
        decoded.open(&key, &vault(7)).unwrap().expose_secret(),
        &body[..]
    );
}

/// The contact book is bound to its account (`aad("contacts", account_id)`):
/// a book served to another account of the same user's devices fails, and a
/// book decoded under a different key fails.
#[test]
fn contacts_are_bound_to_account() {
    // K_contacts is derived from the account key (HKDF(AK, ctx("contacts-key"))):
    // a different account key gives a different book key, as a re-key does.
    let key = AccountKey::generate().unwrap().contacts_key().unwrap();
    let other = AccountKey::generate().unwrap().contacts_key().unwrap();
    let a = AccountId::from_bytes([1; 16]).unwrap();
    let b = AccountId::from_bytes([2; 16]).unwrap();
    let body = vec![0x7eu8; 400];
    let book = ContactsCiphertext::seal(&key, &a, &body).unwrap();
    let decoded = ContactsCiphertext::from_bytes(book.as_bytes()).unwrap();

    assert_eq!(decoded.open(&key, &a).unwrap().expose_secret(), &body[..]);
    // Wrong account.
    assert_eq!(decoded.open(&key, &b).unwrap_err(), Error::Decryption);
    // Wrong key (K_contacts is derived from AK; a re-key changes it).
    assert_eq!(decoded.open(&other, &a).unwrap_err(), Error::Decryption);
}

/// An attachment's every chunk binds `(vault_id, item_id, attachment_id)` in
/// its AAD (CRYPTO_SPEC §7.3). A ciphertext served under any other context,
/// or opened with the file key of another item, fails: the server cannot
/// move an attachment between items, vaults or attachment slots.
#[test]
fn attachment_is_bound_to_its_context() {
    let fk = AttachmentKey::from_secret(SecretBytes::from_slice(&[0x9c; 32]).unwrap());
    let (v, it, at) = (
        vault(1),
        item(2),
        AttachmentId::from_bytes([3; 16]).unwrap(),
    );
    // Two chunks so chunk indexing and the final-block flag both participate.
    let pt = vec![0x42u8; 70_000];
    let ct = seal_attachment(&fk, &v, &it, &at, &pt).unwrap();

    assert_eq!(
        open_attachment(&fk, &v, &it, &at, &ct)
            .unwrap()
            .expose_secret(),
        &pt[..]
    );

    let foreign_vault = vault(9);
    let foreign_item = item(9);
    let foreign_attachment = AttachmentId::from_bytes([9; 16]).unwrap();
    for (vv, ii, aa) in [
        (foreign_vault, it, at),
        (v, foreign_item, at),
        (v, it, foreign_attachment),
        // vault and item swapped.
        (
            VaultId::from_bytes([2; 16]).unwrap(),
            ItemId::from_bytes([1; 16]).unwrap(),
            at,
        ),
    ] {
        assert_eq!(
            open_attachment(&fk, &vv, &ii, &aa, &ct).unwrap_err(),
            Error::Decryption
        );
    }

    // Another file key fails too.
    let other_fk = AttachmentKey::from_secret(SecretBytes::from_slice(&[0x00; 32]).unwrap());
    assert_eq!(
        open_attachment(&other_fk, &v, &it, &at, &ct).unwrap_err(),
        Error::Decryption
    );
}
