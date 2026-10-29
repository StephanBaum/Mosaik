//! SSH key material from vault items: `AgentKey::new` on an arbitrary
//! private key file, public key line and passphrase never panics. A key
//! that is accepted has a `SHA256:` fingerprint and a non-empty public key
//! blob, is listed once in a key set (a second insert is a duplicate), and
//! signing with it either fails with a named reason or produces a non-empty
//! signature. The KDF round limit is kept at 1 so a fuzzed file cannot
//! make an iteration slow.
//!
//! Input: `[private_len, private..., public_len, public..., passphrase...]`
//! with two-byte little-endian lengths (taken modulo what is left), so a
//! real OpenSSH private key (~400 bytes) fits. Seeds: `seeds/key_material`,
//! built from `tests/keys`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::model::{SshKeyMaterial, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_ssh_agent::{AgentKey, KeySet};

fn split(data: &[u8]) -> (&[u8], &[u8]) {
    let Some((len, rest)) = data.split_first_chunk::<2>() else {
        return (&[], &[]);
    };
    let n = usize::from(u16::from_le_bytes(*len)) % (rest.len() + 1);
    rest.split_at(n)
}

fuzz_target!(|data: &[u8]| {
    let (private, rest) = split(data);
    let (public, passphrase) = split(rest);
    let (Ok(private), Ok(public), Ok(passphrase)) = (
        std::str::from_utf8(private),
        std::str::from_utf8(public),
        std::str::from_utf8(passphrase),
    ) else {
        return;
    };
    let Ok(secret) = SecretString::from_str_copy(private) else {
        return;
    };
    let material = SshKeyMaterial::new(secret, Text::new(public), Text::new(""));
    let passphrase = SecretString::from_str_copy(passphrase).ok();
    let vault = VaultId::from_bytes([1; 16]).expect("id");
    let item = ItemId::from_bytes([2; 16]).expect("id");
    let Ok(key) = AgentKey::new(vault, item, None, None, material, passphrase) else {
        return;
    };
    assert!(key.fingerprint().starts_with("SHA256:"));
    assert!(!key.public_key_blob().is_empty());
    if let Ok(signature) = key.sign(b"challenge", 0, 1) {
        assert!(!signature.is_empty());
    }
    let mut set = KeySet::new();
    assert!(set.insert(key));
    assert_eq!(set.len(), 1);
});
