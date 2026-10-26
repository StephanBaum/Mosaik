//! Shared helpers for the fuzz targets: fixed keys and the golden objects
//! from `crates/core/tests/golden/`, so every run uses the same keys and a
//! crash reproduces from its saved input. The golden objects are authentic,
//! so the tamper modes reach the code behind the tag check.
#![allow(dead_code, reason = "each target uses a subset")]

// `VaultKey::from_secret_for_fuzzing` exists only under `cfg(fuzzing)`,
// which cargo-fuzz sets. Say so instead of leaving only an E0599.
#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use mosaic_core::{
    AccountId, AccountKey, AttachmentId, AttachmentKey, Error, ItemId, Kek, KeyGeneration,
    SecretBytes, UnlockMethod, VaultId, VaultKey, WrappedKey,
};

/// First input byte selecting a target's tamper mode. Not `0x01`: that is
/// the version byte of every real object, so raw inputs keep reaching the
/// decoders.
pub const TAMPER: u8 = 0xFE;
/// First input byte selecting the structured attachment mode.
pub const STRUCTURED: u8 = 0xFD;

pub const ITEM: &str = include_str!("../../../tests/golden/item.hex");
pub const VAULT_META: &str = include_str!("../../../tests/golden/vault_meta.hex");
pub const TOMBSTONE: &str = include_str!("../../../tests/golden/tombstone.hex");
pub const WRAPPED_AK_PASSWORD: &str = include_str!("../../../tests/golden/wrapped_ak_password.hex");
pub const WRAPPED_AK_QUICK: &str =
    include_str!("../../../tests/golden/wrapped_ak_quick_unlock.hex");
pub const WRAPPED_AK_HARDWARE: &str =
    include_str!("../../../tests/golden/wrapped_ak_hardware_key.hex");
pub const WRAPPED_VK: &str = include_str!("../../../tests/golden/wrapped_vk.hex");
pub const ATTACHMENT_EMPTY: &str = include_str!("../../../tests/golden/attachment_empty.hex");
pub const ATTACHMENT_SMALL: &str = include_str!("../../../tests/golden/attachment_small.hex");

/// The golden-file IDs (see `src/golden.rs`).
pub fn account() -> AccountId {
    AccountId::from_bytes([0x0A; 16]).expect("id")
}
pub fn vault() -> VaultId {
    VaultId::from_bytes([0x0B; 16]).expect("id")
}
pub fn item() -> ItemId {
    ItemId::from_bytes([0x0C; 16]).expect("id")
}
pub fn attachment() -> AttachmentId {
    AttachmentId::from_bytes([0x0D; 16]).expect("id")
}
pub const DEVICE: [u8; 16] = [0x0E; 16];

/// The golden keys: `W = 10*32`, `AK = 20*32`, `VK = 30*32`, `FK = 40*32`.
pub fn w() -> Kek {
    Kek::from_secret(SecretBytes::from_array(&mut [0x10; 32]))
}
pub fn vk(generation: u32) -> VaultKey {
    VaultKey::from_secret_for_fuzzing(
        SecretBytes::from_array(&mut [0x30; 32]),
        KeyGeneration::new(generation).expect("generation"),
    )
}
pub fn fk() -> AttachmentKey {
    AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]))
}
/// `AK = 20*32`, recovered from the golden password wrapping, since the
/// account key has no public constructor.
pub fn ak() -> AccountKey {
    WrappedKey::from_bytes(&hex_file(WRAPPED_AK_PASSWORD))
        .and_then(|k| k.unwrap_account_key(&w(), &account(), &UnlockMethod::Password))
        .expect("golden wrapped AK")
}

/// Decodes a golden `.hex` file (`#` comment lines, then hex).
pub fn hex_file(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.bytes().filter(|b| !b.is_ascii_whitespace()))
        .collect();
    digits
        .chunks(2)
        .map(|p| {
            let s = std::str::from_utf8(p).expect("ascii");
            u8::from_str_radix(s, 16).expect("hex")
        })
        .collect()
}

/// Applies fuzzer-chosen edits to `base`: each 4-byte group
/// `[kind, a, b, v]` XORs, truncates, extends or inserts. Returns the
/// edited bytes.
pub fn mutate(base: &[u8], ops: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    for &[kind, a, b, v] in ops.as_chunks::<4>().0 {
        let pos = usize::from(u16::from_be_bytes([a, b]));
        match kind % 4 {
            0 => {
                if !out.is_empty() {
                    let i = pos % out.len();
                    out[i] ^= v;
                }
            }
            1 => out.truncate(pos % (out.len() + 1)),
            2 => out.extend(std::iter::repeat_n(v, usize::from(a % 64) + 1)),
            _ => {
                let i = pos % (out.len() + 1);
                out.insert(i, v);
            }
        }
    }
    out
}

/// Errors a tampered object may legitimately produce.
pub fn tamper_error_ok(e: Error) -> bool {
    matches!(
        e,
        Error::Decryption
            | Error::Malformed
            | Error::SizeLimit
            | Error::UnsupportedVersion
            | Error::UnsupportedAlgorithm
            | Error::InvalidKeyGeneration
            | Error::KeyGenerationMismatch
    )
}
