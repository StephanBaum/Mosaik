//! Test fixtures: the throwaway keys in `tests/keys/` and items built from
//! them.

use mosaic_core::model::{Item, ItemType, SecretField, SshKeyMaterial, Text, TextField, Timestamp};
use mosaic_core::{DeviceId, ItemId, SecretString, VaultId};
use ssh_key::{LineEnding, PrivateKey};

use crate::keys::AgentKey;

/// The passphrase of every encrypted fixture.
pub(crate) const PASSPHRASE: &str = "correct horse battery staple";

/// `(private key, public key line)` of a fixture.
pub(crate) fn fixture(name: &str) -> (&'static str, &'static str) {
    macro_rules! key {
        ($n:literal) => {
            (
                include_str!(concat!("../tests/keys/", $n)),
                include_str!(concat!("../tests/keys/", $n, ".pub")),
            )
        };
    }
    match name {
        "ed25519" => key!("ed25519"),
        "ed25519_encrypted" => key!("ed25519_encrypted"),
        "ed25519_encrypted_chacha" => key!("ed25519_encrypted_chacha"),
        "ed25519_encrypted_gcm" => key!("ed25519_encrypted_gcm"),
        "ecdsa_p256" => key!("ecdsa_p256"),
        "ecdsa_p256_encrypted" => key!("ecdsa_p256_encrypted"),
        "ecdsa_p384" => key!("ecdsa_p384"),
        "rsa" => key!("rsa"),
        _ => panic!("no fixture {name}"),
    }
}

pub(crate) fn secret(s: &str) -> SecretString {
    SecretString::from_str_copy(s).unwrap()
}

pub(crate) fn material(private: &str, public: &str) -> SshKeyMaterial {
    SshKeyMaterial::new(
        secret(private),
        Text::new(public),
        Text::new("SHA256:ignored"),
    )
}

pub(crate) fn ids() -> (VaultId, ItemId) {
    (VaultId::new_v7().unwrap(), ItemId::new_v7().unwrap())
}

/// A key from a fixture, with the fixture passphrase for encrypted ones.
pub(crate) fn key(name: &str) -> AgentKey {
    let (private, public) = fixture(name);
    let (vault, item) = ids();
    let pass = name.contains("encrypted").then(|| secret(PASSPHRASE));
    AgentKey::new(
        vault,
        item,
        Some(name),
        None,
        material(private, public),
        pass,
    )
    .unwrap()
}

/// An unencrypted OpenSSH private key file for `key`.
pub(crate) fn openssh(key: impl Into<ssh_key::private::KeypairData>) -> String {
    PrivateKey::new(key.into(), "kat")
        .unwrap()
        .to_openssh(LineEnding::LF)
        .unwrap()
        .to_string()
}

/// A blob of SSH strings (each with its u32 length prefix).
pub(crate) fn strings(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(&s(p));
    }
    out
}

/// One SSH string.
pub(crate) fn s(bytes: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(bytes.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(bytes);
    out
}

/// An SSH key item.
pub(crate) fn ssh_item(
    private: &str,
    public: &str,
    passphrase: Option<&str>,
    comment: Option<&str>,
) -> Item {
    let device = DeviceId::new_v7().unwrap();
    let at = Timestamp::from_unix_millis(1_761_480_000_000);
    let mut item = Item::new(&ItemType::SshKey, device, at).unwrap();
    {
        let mut e = item.edit(device, at);
        e.set_text(TextField::Title, "Deploy key").unwrap();
        e.set_ssh_key(&material(private, public)).unwrap();
        if let Some(p) = passphrase {
            e.set_secret(SecretField::Password, p).unwrap();
        }
        if let Some(c) = comment {
            e.set_text(TextField::SshComment, c).unwrap();
        }
    }
    item
}
