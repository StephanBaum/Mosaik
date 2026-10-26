//! Golden-file tests for every serialized format (CRYPTO_SPEC §14).
//!
//! Each object is produced from fixed keys, IDs and nonces and compared with
//! `crates/core/tests/golden/<name>.hex`, so any format change shows up as a
//! diff in review. The golden bytes are also decoded and opened, so old data
//! stays readable. To regenerate after an intended, spec-approved change:
//!
//! ```sh
//! MOSAIC_BLESS=1 cargo test -p mosaic-core golden
//! ```
//!
//! These tests live inside the crate because they need the deterministic
//! nonce paths, which are not public.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::attachment::{CHUNK_LEN, open_attachment, reference, seal_attachment_with_prefix};
use crate::envelope::{
    ItemCiphertext, TombstoneCiphertext, UnlockMethod, VaultMetaCiphertext, WrappedKey,
};
use crate::ids::{AccountId, AttachmentId, DeviceId, ItemId, KeyGeneration, VaultId};
use crate::kdf::{self, AccountSalt, TEST_COST};
use crate::secret::{
    AccountKey, AttachmentKey, Kek, MasterPassword, SecretBytes, SecretKey, VaultKey,
};

const ACCOUNT: [u8; 16] = [0x0A; 16];
const VAULT: [u8; 16] = [0x0B; 16];
const ITEM: [u8; 16] = [0x0C; 16];
const ATTACHMENT: [u8; 16] = [0x0D; 16];
const DEVICE: [u8; 16] = [0x0E; 16];

fn nonce() -> [u8; 24] {
    core::array::from_fn(|i| 0x50 + u8::try_from(i).unwrap())
}

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}.hex"))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}

fn render(description: &str, bytes: &[u8]) -> String {
    let mut out = String::new();
    for line in description.lines() {
        writeln!(out, "# {line}").unwrap();
    }
    let hex = to_hex(bytes);
    for chunk in hex.as_bytes().chunks(64) {
        writeln!(out, "{}", std::str::from_utf8(chunk).unwrap()).unwrap();
    }
    out
}

fn parse(text: &str) -> Vec<u8> {
    let hex: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.chars().filter(|c| !c.is_whitespace()))
        .collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// Compares `bytes` with the golden file and returns the golden bytes.
fn check(name: &str, description: &str, bytes: &[u8]) -> Vec<u8> {
    let p = path(name);
    if std::env::var_os("MOSAIC_BLESS").is_some() {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, render(description, bytes)).unwrap();
    }
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("missing golden file {}: {e}", p.display()));
    let golden = parse(&text);
    assert_eq!(
        to_hex(&golden),
        to_hex(bytes),
        "serialized format of `{name}` changed; if intended (spec change + ADR), \
         re-run with MOSAIC_BLESS=1 and review the diff"
    );
    golden
}

fn ids() -> (AccountId, VaultId, ItemId, AttachmentId) {
    (
        AccountId::from_bytes(ACCOUNT).unwrap(),
        VaultId::from_bytes(VAULT).unwrap(),
        ItemId::from_bytes(ITEM).unwrap(),
        AttachmentId::from_bytes(ATTACHMENT).unwrap(),
    )
}

/// Effective AADs written out literally from CRYPTO_SPEC (§3 header binding
/// and `aad(label, ...)` = `"mosaic/v1/" || label || 0x00 || fields`, §6,
/// §7.1, §7.2), byte by byte, without the crate's `Aad` builder or labels.
/// The golden objects must open with them under plain XChaCha20-Poly1305.
mod spec_aad {
    fn start(header: &[u8], label: &[u8]) -> Vec<u8> {
        let mut a = header.to_vec();
        a.extend_from_slice(b"mosaic/v1/");
        a.extend_from_slice(label);
        a.push(0x00);
        a
    }

    /// §6: `01 01 || aad("wrap/ak", account_id, method:u8, method_id[16])`.
    pub(super) fn wrap_ak(account: &[u8; 16], method: u8, method_id: &[u8; 16]) -> Vec<u8> {
        let mut a = start(&[0x01, 0x01], b"wrap/ak");
        a.extend_from_slice(account);
        a.push(method);
        a.extend_from_slice(method_id);
        a
    }

    /// §7.1: `01 01 || aad("wrap/vault", account_id, vault_id, key_generation:u32)`.
    pub(super) fn wrap_vault(account: &[u8; 16], vault: &[u8; 16], generation: u32) -> Vec<u8> {
        let mut a = start(&[0x01, 0x01], b"wrap/vault");
        a.extend_from_slice(account);
        a.extend_from_slice(vault);
        a.extend_from_slice(&generation.to_be_bytes());
        a
    }

    /// §7.1: header `01 01 || key_generation:u32`, then
    /// `aad("vault-meta", vault_id, key_generation:u32)`.
    pub(super) fn vault_meta(vault: &[u8; 16], generation: u32) -> Vec<u8> {
        let g = generation.to_be_bytes();
        let mut a = start(&[0x01, 0x01, g[0], g[1], g[2], g[3]], b"vault-meta");
        a.extend_from_slice(vault);
        a.extend_from_slice(&g);
        a
    }

    /// §7.2: header `01 01 || key_generation:u32`, then
    /// `aad("tombstone", vault_id, item_id, key_generation:u32)`.
    pub(super) fn tombstone(vault: &[u8; 16], item: &[u8; 16], generation: u32) -> Vec<u8> {
        let g = generation.to_be_bytes();
        let mut a = start(&[0x01, 0x01, g[0], g[1], g[2], g[3]], b"tombstone");
        a.extend_from_slice(vault);
        a.extend_from_slice(item);
        a.extend_from_slice(&g);
        a
    }

    /// §7.2: header `01 01 || schema_version:u16 || key_generation:u32`, then
    /// `aad("item", vault_id, item_id, schema_version:u16, key_generation:u32)`.
    pub(super) fn item(vault: &[u8; 16], item: &[u8; 16], schema: u16, generation: u32) -> Vec<u8> {
        let s = schema.to_be_bytes();
        let g = generation.to_be_bytes();
        let mut a = start(&[0x01, 0x01, s[0], s[1], g[0], g[1], g[2], g[3]], b"item");
        a.extend_from_slice(vault);
        a.extend_from_slice(item);
        a.extend_from_slice(&s);
        a.extend_from_slice(&g);
        a
    }
}

/// Splits a golden object into `header || nonce[24] || ciphertext || tag`,
/// checks the header is the AAD's prefix and the nonce is the fixed test
/// nonce, and opens it with plain XChaCha20-Poly1305 under `aad`. The same
/// object must not open without the header in the AAD.
fn open_with_spec_aad(
    golden: &[u8],
    header_len: usize,
    key: &[u8; 32],
    aad: &[u8],
) -> crate::secret::SecretVec {
    let (header, rest) = golden.split_at(header_len);
    let (n, body) = rest.split_at(24);
    assert_eq!(header, &aad[..header_len], "header is not the AAD prefix");
    assert_eq!(n, nonce(), "nonce position");
    let n: [u8; 24] = n.try_into().unwrap();
    assert_eq!(
        crate::aead::open(key, &n, &aad[header_len..], body).unwrap_err(),
        crate::Error::Decryption,
        "opened without header binding"
    );
    crate::aead::open(key, &n, aad, body).expect("golden object opens with the spec AAD")
}

fn vk(generation: u32) -> VaultKey {
    VaultKey {
        key: SecretBytes::from_array(&mut [0x30; 32]),
        generation: KeyGeneration::new(generation).unwrap(),
    }
}

#[test]
fn golden_wrapped_account_key() {
    let (account, ..) = ids();
    let w = Kek(SecretBytes::from_array(&mut [0x10; 32]));
    let ak = AccountKey(SecretBytes::from_array(&mut [0x20; 32]));
    let hw_method_id: [u8; 16] = {
        use sha2::{Digest, Sha256};
        Sha256::digest(b"credential-id")[..16].try_into().unwrap()
    };
    for (name, method, spec_aad) in [
        (
            "wrapped_ak_password",
            UnlockMethod::Password,
            spec_aad::wrap_ak(&ACCOUNT, 0x01, &ACCOUNT),
        ),
        (
            "wrapped_ak_quick_unlock",
            UnlockMethod::QuickUnlock(DeviceId::from_bytes(DEVICE).unwrap()),
            spec_aad::wrap_ak(&ACCOUNT, 0x02, &DEVICE),
        ),
        (
            "wrapped_ak_hardware_key",
            UnlockMethod::hardware_key(b"credential-id"),
            spec_aad::wrap_ak(&ACCOUNT, 0x03, &hw_method_id),
        ),
    ] {
        let wrapped = ak.wrap_with_nonce(&w, &account, &method, &nonce()).unwrap();
        let desc = format!(
            "WrappedKey (CRYPTO_SPEC 6), label wrap/ak, method {:#04x}\n\
             W = 10*32, AK = 20*32, account_id = 0a*16, nonce = 50..67\n\
             device_id = 0e*16; hardware key method_id = SHA-256(\"credential-id\")[..16]\n\
             layout: version(01) alg(01) nonce[24] ciphertext[32] tag[16]",
            method.id()
        );
        let golden = check(name, &desc, wrapped.as_bytes());
        let pt = open_with_spec_aad(&golden, 2, &[0x10; 32], &spec_aad);
        assert_eq!(pt.expose_secret(), &[0x20; 32]);
        let decoded = WrappedKey::from_bytes(&golden).unwrap();
        assert_eq!(
            decoded.unwrap_account_key(&w, &account, &method).unwrap(),
            ak
        );
    }
}

#[test]
fn golden_wrapped_vault_key() {
    let (account, vault, ..) = ids();
    let ak = AccountKey(SecretBytes::from_array(&mut [0x20; 32]));
    let key = vk(1);
    let wrapped = key
        .wrap_with_nonce(&ak, &account, &vault, &nonce())
        .unwrap();
    let golden = check(
        "wrapped_vk",
        "WrappedKey (CRYPTO_SPEC 7.1), label wrap/vault\n\
         AK = 20*32, VK = 30*32, key_generation = 1, account_id = 0a*16, vault_id = 0b*16, nonce = 50..67\n\
         layout: version(01) alg(01) nonce[24] ciphertext[32] tag[16]",
        wrapped.as_bytes(),
    );
    let pt = open_with_spec_aad(
        &golden,
        2,
        &[0x20; 32],
        &spec_aad::wrap_vault(&ACCOUNT, &VAULT, 1),
    );
    assert_eq!(pt.expose_secret(), &[0x30; 32]);
    let decoded = WrappedKey::from_bytes(&golden).unwrap();
    assert_eq!(
        decoded
            .unwrap_vault_key(&ak, &account, &vault, KeyGeneration::FIRST)
            .unwrap(),
        key
    );
}

#[test]
fn golden_item() {
    let (_, vault, item, _) = ids();
    let key = vk(7);
    let pt = b"{\"title\":\"golden\"}";
    let ct = ItemCiphertext::seal_with_nonce(&key, &vault, &item, 3, pt, &nonce()).unwrap();
    let golden = check(
        "item",
        "ItemCiphertext (CRYPTO_SPEC 7.2)\n\
         VK = 30*32, key_generation = 7, schema_version = 3, vault_id = 0b*16, item_id = 0c*16, nonce = 50..67\n\
         plaintext = {\"title\":\"golden\"} padded to 512 bytes\n\
         layout: version(01) alg(01) schema_version:u16 key_generation:u32 nonce[24] ciphertext[512] tag[16]",
        ct.as_bytes(),
    );
    let padded = open_with_spec_aad(
        &golden,
        8,
        &[0x30; 32],
        &spec_aad::item(&VAULT, &ITEM, 3, 7),
    );
    assert_eq!(padded.len(), 512);
    assert!(padded.expose_secret().starts_with(pt));
    let decoded = ItemCiphertext::from_bytes(&golden).unwrap();
    assert_eq!(decoded.schema_version(), 3);
    assert_eq!(decoded.key_generation().get(), 7);
    assert_eq!(
        decoded.open(&key, &vault, &item).unwrap().expose_secret(),
        pt
    );
}

#[test]
fn golden_vault_meta() {
    let (_, vault, ..) = ids();
    let key = vk(1);
    let ct = VaultMetaCiphertext::seal_with_nonce(&key, &vault, b"Personal", &nonce()).unwrap();
    let golden = check(
        "vault_meta",
        "VaultMeta (CRYPTO_SPEC 7.1)\n\
         VK = 30*32, key_generation = 1, vault_id = 0b*16, nonce = 50..67, meta = \"Personal\" padded to 512\n\
         layout: version(01) alg(01) key_generation:u32 nonce[24] ciphertext[512] tag[16]",
        ct.as_bytes(),
    );
    let padded = open_with_spec_aad(&golden, 6, &[0x30; 32], &spec_aad::vault_meta(&VAULT, 1));
    assert_eq!(padded.len(), 512);
    let decoded = VaultMetaCiphertext::from_bytes(&golden).unwrap();
    assert_eq!(
        decoded.open(&key, &vault).unwrap().expose_secret(),
        b"Personal"
    );
}

#[test]
fn golden_tombstone() {
    let (_, vault, item, _) = ids();
    let key = vk(1);
    let ct = TombstoneCiphertext::seal_with_nonce(&key, &vault, &item, b"tombstone v1", &nonce())
        .unwrap();
    let golden = check(
        "tombstone",
        "Tombstone (CRYPTO_SPEC 7.2), not padded\n\
         VK = 30*32, key_generation = 1, vault_id = 0b*16, item_id = 0c*16, nonce = 50..67, plaintext = \"tombstone v1\"\n\
         layout: version(01) alg(01) key_generation:u32 nonce[24] ciphertext[12] tag[16]",
        ct.as_bytes(),
    );
    let pt = open_with_spec_aad(
        &golden,
        6,
        &[0x30; 32],
        &spec_aad::tombstone(&VAULT, &ITEM, 1),
    );
    assert_eq!(pt.expose_secret(), b"tombstone v1");
    let decoded = TombstoneCiphertext::from_bytes(&golden).unwrap();
    assert_eq!(
        decoded.open(&key, &vault, &item).unwrap().expose_secret(),
        b"tombstone v1"
    );
}

#[test]
fn golden_attachment() {
    let (_, vault, item, attachment) = ids();
    let fk = AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]));
    let prefix: [u8; 19] = core::array::from_fn(|i| 0x70 + u8::try_from(i).unwrap());
    for (name, pt) in [
        ("attachment_empty", &b""[..]),
        ("attachment_small", &b"attachment bytes"[..]),
    ] {
        let ct = seal_attachment_with_prefix(&fk, &vault, &item, &attachment, pt, &prefix).unwrap();
        let desc = format!(
            "Attachment (CRYPTO_SPEC 7.3), STREAM-BE32 over XChaCha20-Poly1305, one (last) chunk\n\
             FK = 40*32, vault_id = 0b*16, item_id = 0c*16, attachment_id = 0d*16, nonce_prefix = 70..82\n\
             plaintext = {:?}\n\
             layout: version(01) alg(02) chunk_log2(10) nonce_prefix[19] chunk(ciphertext || tag[16])",
            String::from_utf8_lossy(pt)
        );
        let golden = check(name, &desc, &ct);
        let back = open_attachment(&fk, &vault, &item, &attachment, &golden).unwrap();
        assert_eq!(back.expose_secret(), pt);
        // The golden bytes also open with plain XChaCha20-Poly1305, nonce
        // `prefix || BE32(0) || 01`, independent of `aead-stream`.
        let aad = reference::aad(vault.as_bytes(), item.as_bytes(), attachment.as_bytes());
        assert_eq!(reference::open(fk.expose_secret(), &aad, &golden), pt);
    }
}

/// `len:u64 || SHA-256(ct) || ct[..64] || ct[len-64..]`: pins a large
/// ciphertext without storing all of it.
fn summary(ct: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut out = u64::try_from(ct.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(&Sha256::digest(ct));
    out.extend_from_slice(&ct[..64]);
    out.extend_from_slice(&ct[ct.len() - 64..]);
    out
}

/// Multi-chunk attachments: counters above 0, the last-block flag off on
/// every chunk but the last, an exact multiple of the chunk size (no empty
/// tail) and a 1-byte last chunk.
#[test]
fn golden_attachment_multi() {
    let (_, vault, item, attachment) = ids();
    let fk = AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]));
    let prefix: [u8; 19] = core::array::from_fn(|i| 0x70 + u8::try_from(i).unwrap());
    let aad = reference::aad(vault.as_bytes(), item.as_bytes(), attachment.as_bytes());
    let mut both = Vec::new();
    for len in [2 * CHUNK_LEN, 2 * CHUNK_LEN + 1] {
        let pt: Vec<u8> = (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect();
        let ct =
            seal_attachment_with_prefix(&fk, &vault, &item, &attachment, &pt, &prefix).unwrap();
        assert_eq!(
            summary(&reference::seal(fk.expose_secret(), &aad, &prefix, &pt)),
            summary(&ct)
        );
        let back = open_attachment(&fk, &vault, &item, &attachment, &ct).unwrap();
        assert_eq!(back.expose_secret(), pt.as_slice());
        both.extend(summary(&ct));
    }
    check(
        "attachment_multi",
        "Attachments (CRYPTO_SPEC 7.3) of 2*65536 and 2*65536+1 bytes, plaintext byte i = i mod 251\n\
         FK = 40*32, vault_id = 0b*16, item_id = 0c*16, attachment_id = 0d*16, nonce_prefix = 70..82\n\
         per attachment: ciphertext_len:u64 || SHA-256(ciphertext)[32] || first 64 bytes || last 64 bytes",
        &both,
    );
}

#[test]
fn golden_aad() {
    let (account, vault, item, _) = ids();
    let header = [0x01, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x07];
    let aad = item_aad(&header, &vault, &item);
    assert_eq!(aad, spec_aad::item(&VAULT, &ITEM, 3, 7));
    assert_eq!(
        WrappedKey::ak_aad(&account, &UnlockMethod::Password).as_bytes(),
        spec_aad::wrap_ak(&ACCOUNT, 0x01, &ACCOUNT)
    );
    check(
        "aad_item",
        "Effective AAD of an item (CRYPTO_SPEC 3, 7.2): header || ctx(\"item\") || 00 || vault_id || item_id || schema_version:u16 || key_generation:u32\n\
         header = 01 01 0003 00000007, vault_id = 0b*16, item_id = 0c*16",
        &aad,
    );
    let ak_aad = WrappedKey::ak_aad(&account, &UnlockMethod::Password);
    check(
        "aad_wrap_ak",
        "Effective AAD of a password wrapping of AK (CRYPTO_SPEC 3, 6.1): 01 01 || ctx(\"wrap/ak\") || 00 || account_id || 01 || account_id\n\
         account_id = 0a*16",
        ak_aad.as_bytes(),
    );
}

fn item_aad(header: &[u8], vault: &VaultId, item: &ItemId) -> Vec<u8> {
    crate::encoding::Aad::with_header(header, crate::encoding::label::ITEM)
        .fixed(vault.as_bytes())
        .fixed(item.as_bytes())
        .u16(3)
        .u32(7)
        .as_bytes()
        .to_vec()
}

#[test]
fn golden_secret_key_display() {
    let sk = SecretKey(SecretBytes::from_array(&mut core::array::from_fn(|i| {
        u8::try_from(i).unwrap()
    })));
    let shown = sk.to_display().unwrap();
    let p = path("secret_key_display").with_extension("txt");
    if std::env::var_os("MOSAIC_BLESS").is_some() {
        std::fs::write(
            &p,
            format!(
                "# Secret Key display string (CRYPTO_SPEC 5.2) for SK = 00 01 02 .. 0f
{}
",
                shown.expose_secret()
            ),
        )
        .unwrap();
    }
    let text = std::fs::read_to_string(&p).unwrap();
    let golden = text
        .lines()
        .find(|l| !l.starts_with('#'))
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        golden,
        shown.expose_secret(),
        "Secret Key display format changed"
    );
    assert_eq!(SecretKey::parse(&golden).unwrap(), sk);
}

/// Also reproduced independently (argon2-cffi plus a stdlib HKDF) by
/// `tests/vectors/derive_keys_baseline.py`, line `test-cost`.
#[test]
fn golden_derived_keys() {
    let account = AccountId::from_bytes(ACCOUNT).unwrap();
    let salt = AccountSalt::from_bytes([0x5A; 16]);
    let sk = SecretKey(SecretBytes::from_array(&mut [0x0F; 16]));
    let prepared =
        kdf::prepare_password(&MasterPassword::new("correct horse battery staple".into())).unwrap();
    let p = kdf::stretch_with_cost(&prepared, &salt, TEST_COST).unwrap();
    let keys = kdf::derive_keys(p, &sk, &account).unwrap();
    let mut both = keys.kek.0.expose_secret().to_vec();
    both.extend_from_slice(keys.opaque_password.expose_secret());
    check(
        "derived_keys",
        "KEK || OPW (CRYPTO_SPEC 5.4) with TEST-ONLY Argon2id cost m = 64 KiB, t = 1, p = 1\n\
         MP = \"correct horse battery staple\", account_salt = 5a*16, SK = 0f*16, account_id = 0a*16",
        &both,
    );
}
