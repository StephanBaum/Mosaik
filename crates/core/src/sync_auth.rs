//! Keys and MACs of the sync layer (CRYPTO_SPEC §9.2, §10, §10.1, ADR-0043).
//!
//! ```text
//! K_cp   = HKDF(AK, "", ctx("checkpoint-key"), 32)
//! K_fold = HKDF(AK, "", ctx("sync/folder-mac-key"), 32)
//! K_dev  = HKDF(AK, "", ctx("sync/device-info-key"), 32)
//!
//! checkpoint mac = HMAC-SHA-256(K_cp, ctx("checkpoint") || device_id || counter:u64 || seen_digest)
//! folder mac     = HMAC-SHA-256(K_fold, ctx(label(purpose)) || 0x00 || msg)
//! session token  = HKDF(session_key, "", ctx("session"), 32)
//! ```
//!
//! Every derivation happens here, so `AK` never leaves `mosaic-core`: the
//! vault session holds the keys, `mosaic-sync` builds the byte formats and
//! asks the session for MACs. Device info (the name and platform of an
//! install) is sealed under `K_dev` with XChaCha20-Poly1305.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::aead;
use crate::encoding::{Aad, Label, ctx, label, take};
use crate::error::{Error, Result};
use crate::ids::{AccountId, InstallId};
use crate::kdf::hkdf_ctx;
use crate::secret::{
    AccountKey, CheckpointKey, DeviceInfoKey, FolderMacKey, SecretBytes, SecretVec, SessionToken,
};

/// Length of a checkpoint or folder MAC in bytes.
pub const SYNC_MAC_LEN: usize = 32;

/// The magic of a sealed device-info file.
const DEVICE_INFO_MAGIC: &[u8; 4] = b"MDEV";
/// The format version of a sealed device-info file.
const DEVICE_INFO_VERSION: u8 = 1;
/// Longest device-info plaintext in bytes (§10.1).
pub const DEVICE_INFO_MAX_PLAINTEXT: usize = 512;
/// `magic || version || device_id`, the cleartext header.
const DEVICE_INFO_HEADER_LEN: usize = 4 + 1 + 16;

impl AccountKey {
    /// `K_cp = HKDF(AK, "", ctx("checkpoint-key"), 32)` (§10).
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn checkpoint_key(&self) -> Result<CheckpointKey> {
        Ok(CheckpointKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::CHECKPOINT_KEY,
        )?))
    }

    /// `K_fold = HKDF(AK, "", ctx("sync/folder-mac-key"), 32)` (§10.1).
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn folder_mac_key(&self) -> Result<FolderMacKey> {
        Ok(FolderMacKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::SYNC_FOLDER_MAC_KEY,
        )?))
    }

    /// `K_dev = HKDF(AK, "", ctx("sync/device-info-key"), 32)` (§10.1).
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn device_info_key(&self) -> Result<DeviceInfoKey> {
        Ok(DeviceInfoKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::SYNC_DEVICE_INFO_KEY,
        )?))
    }
}

/// An HMAC-SHA-256 keyed with a 32-byte secret (any key length is valid for
/// HMAC, so the error cannot occur; it is mapped, never ignored).
fn keyed(key: &SecretBytes<32>) -> Result<Hmac<Sha256>> {
    <Hmac<Sha256> as KeyInit>::new_from_slice(key.expose_secret()).map_err(|_| Error::SizeLimit)
}

fn finish(mac: Hmac<Sha256>) -> [u8; SYNC_MAC_LEN] {
    mac.finalize().into_bytes().into()
}

/// `HMAC-SHA-256(K_cp, ctx("checkpoint") || device_id || counter:u64 || seen_digest)`
/// (§10). The caller computes `seen_digest` over the checkpoint body.
///
/// # Errors
/// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
pub fn checkpoint_mac(
    key: &CheckpointKey,
    device: &InstallId,
    counter: u64,
    seen_digest: &[u8; 32],
) -> Result<[u8; SYNC_MAC_LEN]> {
    let mut mac = keyed(&key.0)?;
    mac.update(&ctx(label::CHECKPOINT.as_str()));
    mac.update(device.as_bytes());
    mac.update(&counter.to_be_bytes());
    mac.update(seen_digest);
    Ok(finish(mac))
}

/// Whether `mac` is the checkpoint MAC of the message, compared in constant
/// time.
///
/// # Errors
/// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
pub fn verify_checkpoint_mac(
    key: &CheckpointKey,
    device: &InstallId,
    counter: u64,
    seen_digest: &[u8; 32],
    mac: &[u8; SYNC_MAC_LEN],
) -> Result<bool> {
    let expected = checkpoint_mac(key, device, counter, seen_digest)?;
    Ok(bool::from(expected.ct_eq(mac)))
}

/// What a folder MAC protects; each purpose has its own label (§10.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderMacPurpose {
    /// A segment file (`.mseg`).
    Segment,
    /// An account file (`.macct`).
    Account,
    /// A blob delete marker (`.mdel`).
    BlobDelete,
}

impl FolderMacPurpose {
    const fn label(self) -> Label {
        match self {
            Self::Segment => label::SYNC_FOLDER_SEGMENT,
            Self::Account => label::SYNC_FOLDER_ACCOUNT,
            Self::BlobDelete => label::SYNC_FOLDER_BLOB_DELETE,
        }
    }
}

/// `HMAC-SHA-256(K_fold, ctx(label) || 0x00 || msg)` for the label of
/// `purpose` (§10.1).
///
/// # Errors
/// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
pub fn folder_mac(
    key: &FolderMacKey,
    purpose: FolderMacPurpose,
    msg: &[u8],
) -> Result<[u8; SYNC_MAC_LEN]> {
    let mut mac = keyed(&key.0)?;
    mac.update(&ctx(purpose.label().as_str()));
    mac.update(&[0x00]);
    mac.update(msg);
    Ok(finish(mac))
}

/// Whether `mac` is the folder MAC of the message, compared in constant time.
///
/// # Errors
/// [`Error::SizeLimit`] if the MAC cannot be keyed (it cannot for 32 bytes).
pub fn verify_folder_mac(
    key: &FolderMacKey,
    purpose: FolderMacPurpose,
    msg: &[u8],
    mac: &[u8; SYNC_MAC_LEN],
) -> Result<bool> {
    let expected = folder_mac(key, purpose, msg)?;
    Ok(bool::from(expected.ct_eq(mac)))
}

fn device_info_header(device: &InstallId) -> [u8; DEVICE_INFO_HEADER_LEN] {
    let mut header = [0u8; DEVICE_INFO_HEADER_LEN];
    let (magic, rest) = header.split_at_mut(4);
    magic.copy_from_slice(DEVICE_INFO_MAGIC);
    if let Some((version, id)) = rest.split_first_mut() {
        *version = DEVICE_INFO_VERSION;
        id.copy_from_slice(device.as_bytes());
    }
    header
}

fn device_info_aad(header: &[u8], account: &AccountId, device: &InstallId) -> Aad {
    Aad::with_header(header, label::SYNC_DEVICE_INFO)
        .fixed(account.as_bytes())
        .fixed(device.as_bytes())
}

/// Seals device info (`MDEV || 1 || device_id || nonce[24] || ciphertext || tag`)
/// under `K_dev`, bound to the account and the device (§10.1). The caller
/// builds the canonical JSON `plaintext` (at most
/// [`DEVICE_INFO_MAX_PLAINTEXT`] bytes).
///
/// # Errors
/// [`Error::SizeLimit`] for a longer plaintext, [`Error::Randomness`] if the
/// CSPRNG fails.
pub fn seal_device_info(
    key: &DeviceInfoKey,
    account: &AccountId,
    device: &InstallId,
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    if plaintext.len() > DEVICE_INFO_MAX_PLAINTEXT {
        return Err(Error::SizeLimit);
    }
    let header = device_info_header(device);
    let nonce = aead::random_nonce()?;
    let aad = device_info_aad(&header, account, device);
    aead::seal_with_prefix(
        key.0.expose_secret(),
        &nonce,
        aad.as_bytes(),
        &[&header, &nonce],
        plaintext,
    )
}

/// Opens sealed device info. The device ID in the header must be `device`
/// (the one in the file path).
///
/// # Errors
/// [`Error::Malformed`] for a wrong length or magic or another device ID,
/// [`Error::UnsupportedVersion`], and [`Error::Decryption`] for a wrong key,
/// account or tampered bytes.
pub fn open_device_info(
    key: &DeviceInfoKey,
    account: &AccountId,
    device: &InstallId,
    sealed: &[u8],
) -> Result<SecretVec> {
    let max = DEVICE_INFO_HEADER_LEN + aead::NONCE_LEN + DEVICE_INFO_MAX_PLAINTEXT + aead::TAG_LEN;
    if sealed.len() < DEVICE_INFO_HEADER_LEN + aead::NONCE_LEN + aead::TAG_LEN || sealed.len() > max
    {
        return Err(Error::Malformed);
    }
    let (header, rest) = sealed.split_at(DEVICE_INFO_HEADER_LEN);
    let (magic, rest_header) = take::<4>(header)?;
    let (version, id) = rest_header.split_first().ok_or(Error::Malformed)?;
    if magic != DEVICE_INFO_MAGIC {
        return Err(Error::Malformed);
    }
    if *version != DEVICE_INFO_VERSION {
        return Err(Error::UnsupportedVersion);
    }
    if id != device.as_bytes().as_slice() {
        return Err(Error::Malformed);
    }
    let (nonce, body) = take::<{ aead::NONCE_LEN }>(rest)?;
    let aad = device_info_aad(header, account, device);
    aead::open(key.0.expose_secret(), nonce, aad.as_bytes(), body)
}

/// `session_token = HKDF(session_key, "", ctx("session"), 32)` (§9.2). The
/// client sends it as a bearer token, the server stores its SHA-256.
///
/// # Errors
/// [`Error::Malformed`] for an empty `session_key`.
pub fn session_token(session_key: &[u8]) -> Result<SessionToken> {
    if session_key.is_empty() {
        return Err(Error::Malformed);
    }
    Ok(SessionToken(hkdf_ctx(session_key, &[], label::SESSION)?))
}

impl SessionToken {
    /// Borrows the 32 token bytes to send them or hash them. Never log them.
    #[must_use]
    pub fn expose(&self) -> &[u8; 32] {
        self.0.expose_secret()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// `crates/core/tests/vectors/sync_kat.txt`, written by the independent
    /// script `sync_kat.py` (Python `hmac` and `hashlib`, HKDF from RFC 5869).
    fn kat() -> BTreeMap<String, Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("vectors")
            .join("sync_kat.txt");
        let text = std::fs::read_to_string(path).unwrap();
        text.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let (name, hex) = l.split_once('=').unwrap();
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect();
                (name.to_owned(), bytes)
            })
            .collect()
    }

    fn ak() -> AccountKey {
        AccountKey(SecretBytes::from_array(&mut [0x11; 32]))
    }

    fn account() -> AccountId {
        AccountId::from_bytes([0x0A; 16]).unwrap()
    }

    fn install(byte: u8) -> InstallId {
        InstallId::from_bytes([byte; 16]).unwrap()
    }

    #[test]
    fn derived_keys_match_the_script() {
        let k = kat();
        assert_eq!(
            ak().checkpoint_key().unwrap().0.expose_secret().as_slice(),
            k["k_cp"].as_slice()
        );
        assert_eq!(
            ak().folder_mac_key().unwrap().0.expose_secret().as_slice(),
            k["k_fold"].as_slice()
        );
        assert_eq!(
            ak().device_info_key().unwrap().0.expose_secret().as_slice(),
            k["k_dev"].as_slice()
        );
    }

    #[test]
    fn checkpoint_mac_kat() {
        let k = kat();
        let key = ak().checkpoint_key().unwrap();
        let digest: [u8; 32] = k["seen_digest"].as_slice().try_into().unwrap();
        let mac = checkpoint_mac(&key, &install(0x0E), 7, &digest).unwrap();
        assert_eq!(mac.as_slice(), k["checkpoint_mac"].as_slice());
        assert!(verify_checkpoint_mac(&key, &install(0x0E), 7, &digest, &mac).unwrap());
        assert!(!verify_checkpoint_mac(&key, &install(0x0F), 7, &digest, &mac).unwrap());
        assert!(!verify_checkpoint_mac(&key, &install(0x0E), 8, &digest, &mac).unwrap());
        assert!(!verify_checkpoint_mac(&key, &install(0x0E), 7, &[0; 32], &mac).unwrap());
        let mut flipped = mac;
        flipped[31] ^= 1;
        assert!(!verify_checkpoint_mac(&key, &install(0x0E), 7, &digest, &flipped).unwrap());
        let other = AccountKey(SecretBytes::from_array(&mut [0x12; 32]))
            .checkpoint_key()
            .unwrap();
        assert!(!verify_checkpoint_mac(&other, &install(0x0E), 7, &digest, &mac).unwrap());
    }

    #[test]
    fn folder_mac_kat_each_purpose() {
        let k = kat();
        let key = ak().folder_mac_key().unwrap();
        let msg = b"folder message";
        let mut seen = Vec::new();
        for (purpose, name) in [
            (FolderMacPurpose::Segment, "folder_mac_segment"),
            (FolderMacPurpose::Account, "folder_mac_account"),
            (FolderMacPurpose::BlobDelete, "folder_mac_blob_delete"),
        ] {
            let mac = folder_mac(&key, purpose, msg).unwrap();
            assert_eq!(mac.as_slice(), k[name].as_slice(), "{name}");
            assert!(verify_folder_mac(&key, purpose, msg, &mac).unwrap());
            assert!(!verify_folder_mac(&key, purpose, b"folder messagf", &mac).unwrap());
            seen.push(mac);
        }
        assert_ne!(seen[0], seen[1]);
        assert_ne!(seen[1], seen[2]);
        assert_ne!(seen[0], seen[2]);
        let mac = folder_mac(&key, FolderMacPurpose::Segment, msg).unwrap();
        assert!(
            !verify_folder_mac(&key, FolderMacPurpose::Account, msg, &mac).unwrap(),
            "a MAC is bound to its purpose"
        );
    }

    #[test]
    fn session_token_kat() {
        let k = kat();
        let token = session_token(&[0x22; 64]).unwrap();
        assert_eq!(token.expose().as_slice(), k["session_token"].as_slice());
        assert_ne!(
            session_token(&[0x23; 64]).unwrap().expose(),
            token.expose(),
            "another session key"
        );
        assert_eq!(session_token(&[]).unwrap_err(), Error::Malformed);
    }

    #[test]
    fn device_info_seal_open_and_aad_binding() {
        let key = ak().device_info_key().unwrap();
        let plaintext = br#"{"createdAt":1,"name":"Laptop","platform":"linux"}"#;
        let sealed = seal_device_info(&key, &account(), &install(0x0E), plaintext).unwrap();
        assert_eq!(&sealed[..4], b"MDEV");
        assert_eq!(sealed[4], 1);
        assert_eq!(&sealed[5..21], &[0x0E; 16]);
        assert_eq!(sealed.len(), 21 + 24 + plaintext.len() + 16);
        let opened = open_device_info(&key, &account(), &install(0x0E), &sealed).unwrap();
        assert_eq!(opened.expose_secret(), plaintext);
        // A fresh nonce for every seal.
        let again = seal_device_info(&key, &account(), &install(0x0E), plaintext).unwrap();
        assert_ne!(sealed, again);

        // Bound to the account, the device (path and header) and the key.
        let other_account = AccountId::from_bytes([0x0B; 16]).unwrap();
        assert_eq!(
            open_device_info(&key, &other_account, &install(0x0E), &sealed).unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            open_device_info(&key, &account(), &install(0x0F), &sealed).unwrap_err(),
            Error::Malformed
        );
        let other_key = AccountKey(SecretBytes::from_array(&mut [0x12; 32]))
            .device_info_key()
            .unwrap();
        assert_eq!(
            open_device_info(&other_key, &account(), &install(0x0E), &sealed).unwrap_err(),
            Error::Decryption
        );
        // Every byte is authenticated: the header through the AAD, the rest
        // through the tag. A device ID swapped in the header fails the path
        // check; the other bytes fail the tag.
        for i in 0..sealed.len() {
            let mut bad = sealed.clone();
            bad[i] ^= 1;
            assert!(
                open_device_info(&key, &account(), &install(0x0E), &bad).is_err(),
                "byte {i}"
            );
        }
    }

    #[test]
    fn device_info_bounds_and_versions() {
        let key = ak().device_info_key().unwrap();
        let (a, d) = (account(), install(0x0E));
        assert_eq!(
            seal_device_info(&key, &a, &d, &[0; DEVICE_INFO_MAX_PLAINTEXT + 1]).unwrap_err(),
            Error::SizeLimit
        );
        let max = seal_device_info(&key, &a, &d, &[7; DEVICE_INFO_MAX_PLAINTEXT]).unwrap();
        assert_eq!(
            open_device_info(&key, &a, &d, &max).unwrap().len(),
            DEVICE_INFO_MAX_PLAINTEXT
        );
        let empty = seal_device_info(&key, &a, &d, &[]).unwrap();
        assert_eq!(open_device_info(&key, &a, &d, &empty).unwrap().len(), 0);
        assert_eq!(
            open_device_info(&key, &a, &d, &max[..max.len() - 1]).unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            open_device_info(&key, &a, &d, &empty[..60]).unwrap_err(),
            Error::Malformed
        );
        let mut long = max.clone();
        long.push(0);
        assert_eq!(
            open_device_info(&key, &a, &d, &long).unwrap_err(),
            Error::Malformed
        );
        let mut bad_magic = empty.clone();
        bad_magic[0] = b'X';
        assert_eq!(
            open_device_info(&key, &a, &d, &bad_magic).unwrap_err(),
            Error::Malformed
        );
        let mut bad_version = empty;
        bad_version[4] = 2;
        assert_eq!(
            open_device_info(&key, &a, &d, &bad_version).unwrap_err(),
            Error::UnsupportedVersion
        );
    }

    #[test]
    fn keys_are_redacted_and_distinct() {
        let ak = ak();
        let (cp, fold, dev) = (
            ak.checkpoint_key().unwrap(),
            ak.folder_mac_key().unwrap(),
            ak.device_info_key().unwrap(),
        );
        assert_ne!(cp.0.expose_secret(), fold.0.expose_secret());
        assert_ne!(fold.0.expose_secret(), dev.0.expose_secret());
        assert_ne!(cp.0.expose_secret(), dev.0.expose_secret());
        assert_ne!(cp.0.expose_secret(), &[0x11; 32]);
        assert_eq!(format!("{cp:?}"), "[REDACTED]");
        assert_eq!(format!("{fold:?}"), "[REDACTED]");
        assert_eq!(format!("{dev:?}"), "[REDACTED]");
        assert_eq!(
            format!("{:?}", session_token(&[1; 64]).unwrap()),
            "[REDACTED]"
        );
        assert_eq!(cp, ak.checkpoint_key().unwrap());
        assert_ne!(
            cp,
            AccountKey(SecretBytes::from_array(&mut [3; 32]))
                .checkpoint_key()
                .unwrap()
        );
    }
}
