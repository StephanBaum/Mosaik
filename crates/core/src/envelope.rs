//! Serialized AEAD objects (CRYPTO_SPEC §6, §7.1, §7.2).
//!
//! Every object is `header || nonce[24] || AEAD(key, nonce, plaintext, aad)`
//! with `aad = header || aad(label, ...)` (header binding, §3). The context
//! fields of `aad` (account, vault and item IDs, unlock method, generation)
//! are never stored in the object: the opener supplies them from context, so
//! an object moved to another context fails authentication.
//!
//! | Object | Header | Label | AAD fields after the label |
//! |---|---|---|---|
//! | [`WrappedKey`] (AK) | `01 01` | `wrap/ak` | `account_id, method:u8, method_id[16]` |
//! | [`WrappedKey`] (VK) | `01 01` | `wrap/vault` | `account_id, vault_id, key_generation:u32` |
//! | [`VaultMetaCiphertext`] | `01 01 key_generation:u32` | `vault-meta` | `vault_id, key_generation:u32` |
//! | [`ItemCiphertext`] | `01 01 schema_version:u16 key_generation:u32` | `item` | `vault_id, item_id, schema_version:u16, key_generation:u32` |
//! | [`TombstoneCiphertext`] | `01 01 key_generation:u32` | `tombstone` | `vault_id, item_id, key_generation:u32` |
//!
//! Item and vault-metadata plaintexts are padded ([`crate::encoding::pad`]);
//! tombstones are not (§7.2).
//!
//! Decoders are strict: they check the exact length or length structure,
//! the version and the algorithm, and bound the size before any crypto.

use crate::aead::{self, NONCE_LEN, TAG_LEN};
use crate::encoding::{self, Aad, PAD_BLOCK, PAD_MIN, label, take};
use crate::error::{Error, Result};
use crate::ids::{AccountId, DeviceId, ItemId, KeyGeneration, VaultId};
use crate::secret::{SecretBytes, SecretVec, VaultKey};

/// Format version of every v1 object.
pub const FORMAT_VERSION: u8 = 0x01;
/// Algorithm ID of XChaCha20-Poly1305 envelopes.
pub const ALG_XCHACHA20_POLY1305: u8 = 0x01;

/// Largest padded item plaintext: 1 MiB (ADR-0007).
pub const MAX_ITEM_PADDED_LEN: usize = 1024 * 1024;
/// Largest item plaintext that fits [`MAX_ITEM_PADDED_LEN`] once padded.
pub const MAX_ITEM_PLAINTEXT_LEN: usize = MAX_ITEM_PADDED_LEN - 1;
/// Largest padded vault-metadata plaintext: 64 KiB (ADR-0007).
pub const MAX_VAULT_META_PADDED_LEN: usize = 64 * 1024;
/// Largest vault-metadata plaintext.
pub const MAX_VAULT_META_PLAINTEXT_LEN: usize = MAX_VAULT_META_PADDED_LEN - 1;
/// Largest tombstone plaintext: 64 KiB (ADR-0007).
pub const MAX_TOMBSTONE_PLAINTEXT_LEN: usize = 64 * 1024;

const SHORT_HEADER_LEN: usize = 2;
const GEN_HEADER_LEN: usize = 6;
const ITEM_HEADER_LEN: usize = 8;

/// Checks `version || alg` at the start of `input`.
fn check_version_alg(input: &[u8]) -> Result<()> {
    let ([version, alg], _) = take::<2>(input)?;
    if *version != FORMAT_VERSION {
        return Err(Error::UnsupportedVersion);
    }
    if *alg != ALG_XCHACHA20_POLY1305 {
        return Err(Error::UnsupportedAlgorithm);
    }
    Ok(())
}

/// Splits `header || nonce || body` after the header has been validated.
fn split_nonce<const H: usize>(input: &[u8]) -> Result<(&[u8; H], &[u8; NONCE_LEN], &[u8])> {
    let (header, rest) = take::<H>(input)?;
    let (nonce, body) = take::<NONCE_LEN>(rest)?;
    Ok((header, nonce, body))
}

/// Checks the length structure of a padded body: `pad(x) || tag` with the
/// padded part a multiple of 256, at least 512 and at most `max_padded`.
fn check_padded_body(body_len: usize, max_padded: usize) -> Result<()> {
    let padded = body_len.checked_sub(TAG_LEN).ok_or(Error::Malformed)?;
    if padded > max_padded {
        return Err(Error::SizeLimit);
    }
    if padded < PAD_MIN || !padded.is_multiple_of(PAD_BLOCK) {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// Opens a padded body and strips the padding. Padding failures are
/// reported as [`Error::Decryption`], like tag failures (§7.2).
fn open_padded(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    body: &[u8],
) -> Result<SecretVec> {
    let mut pt = aead::open(key, nonce, aad, body)?;
    let len = encoding::unpadded_len(pt.expose_secret()).map_err(|_| Error::Decryption)?;
    pt.as_mut_vec().truncate(len);
    Ok(pt)
}

/// Pads `plaintext` and seals it as `header || nonce || AEAD(...)`.
fn seal_padded(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    header: &[u8],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let padded = encoding::pad(plaintext)?;
    aead::seal_with_prefix(key, nonce, aad, &[header, nonce], padded.expose_secret())
}

// ---------------------------------------------------------------------------
// Wrapped keys (§6, §7.1)
// ---------------------------------------------------------------------------

/// How a wrapping of the account key is unlocked (§6). The method byte and
/// `method_id` are bound into the AAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnlockMethod {
    /// `0x01`: master password + Secret Key (§6.1). `method_id = account_id`.
    Password,
    /// `0x02`: device-local quick unlock (§6.2). `method_id = device_id`.
    QuickUnlock(DeviceId),
    /// `0x03`: hardware key via WebAuthn PRF (§6.3).
    /// `method_id = SHA-256(credential_id)[..16]`.
    HardwareKey([u8; 16]),
}

impl UnlockMethod {
    /// The method byte.
    #[must_use]
    pub const fn id(&self) -> u8 {
        match self {
            Self::Password => 0x01,
            Self::QuickUnlock(_) => 0x02,
            Self::HardwareKey(_) => 0x03,
        }
    }

    /// The hardware-key method for a WebAuthn credential:
    /// `method_id = SHA-256(credential_id)[..16]` (§6.3).
    #[must_use]
    pub fn hardware_key(credential_id: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(credential_id);
        let mut id = [0u8; 16];
        if let Some(prefix) = digest.first_chunk::<16>() {
            id = *prefix;
        }
        Self::HardwareKey(id)
    }

    /// The 16-byte `method_id` for this method on `account_id`.
    #[must_use]
    pub const fn method_id(&self, account_id: &AccountId) -> [u8; 16] {
        match self {
            Self::Password => *account_id.as_bytes(),
            Self::QuickUnlock(device) => *device.as_bytes(),
            Self::HardwareKey(id) => *id,
        }
    }
}

/// A wrapped 256-bit key (§6):
/// `version:u8 (0x01) || alg:u8 (0x01) || nonce[24] || AEAD(W, nonce, key, aad)`.
///
/// The same format wraps the account key under an unlock method's `W`
/// (label `wrap/ak`) and a vault key under the account key (label
/// `wrap/vault`). Always exactly [`WrappedKey::LEN`] bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WrappedKey([u8; WrappedKey::LEN]);

impl WrappedKey {
    /// Encoded length: 2 + 24 + 32 + 16.
    pub const LEN: usize = SHORT_HEADER_LEN + NONCE_LEN + 32 + TAG_LEN;
    const HEADER: [u8; SHORT_HEADER_LEN] = [FORMAT_VERSION, ALG_XCHACHA20_POLY1305];

    /// Strictly decodes a wrapped key.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`] / [`Error::UnsupportedAlgorithm`] for an
    /// unknown header, [`Error::Malformed`] for any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        check_version_alg(bytes)?;
        let arr: [u8; Self::LEN] = bytes.try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(arr))
    }

    /// The encoded bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }

    pub(crate) fn ak_aad(account_id: &AccountId, method: &UnlockMethod) -> Aad {
        Aad::with_header(&Self::HEADER, label::WRAP_AK)
            .fixed(account_id.as_bytes())
            .u8(method.id())
            .fixed(&method.method_id(account_id))
    }

    pub(crate) fn vault_aad(
        account_id: &AccountId,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Aad {
        Aad::with_header(&Self::HEADER, label::WRAP_VAULT)
            .fixed(account_id.as_bytes())
            .fixed(vault_id.as_bytes())
            .u32(generation.get())
    }

    pub(crate) fn seal(
        wrapping_key: &[u8; 32],
        nonce: &[u8; NONCE_LEN],
        aad: &Aad,
        key: &[u8; 32],
    ) -> Result<Self> {
        let out = aead::seal_with_prefix(
            wrapping_key,
            nonce,
            aad.as_bytes(),
            &[&Self::HEADER, nonce],
            key,
        )?;
        let arr: [u8; Self::LEN] = out.as_slice().try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(arr))
    }

    pub(crate) fn open(&self, wrapping_key: &[u8; 32], aad: &Aad) -> Result<SecretBytes<32>> {
        let (_, nonce, body) = split_nonce::<SHORT_HEADER_LEN>(&self.0)?;
        let pt = aead::open(wrapping_key, nonce, aad.as_bytes(), body)?;
        SecretBytes::from_slice(pt.expose_secret()).map_err(|_| Error::Decryption)
    }
}

// ---------------------------------------------------------------------------
// Vault metadata (§7.1)
// ---------------------------------------------------------------------------

/// Encrypted vault metadata (name, icon, description) (§7.1):
/// `version || alg || key_generation:u32 || nonce[24] || AEAD(VK, nonce, pad(meta), aad)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VaultMetaCiphertext {
    bytes: Vec<u8>,
    generation: KeyGeneration,
}

fn gen_header(generation: KeyGeneration) -> [u8; GEN_HEADER_LEN] {
    let [g0, g1, g2, g3] = generation.get().to_be_bytes();
    [FORMAT_VERSION, ALG_XCHACHA20_POLY1305, g0, g1, g2, g3]
}

fn parse_gen_header(header: [u8; GEN_HEADER_LEN]) -> Result<KeyGeneration> {
    let [_, _, g0, g1, g2, g3] = header;
    KeyGeneration::new(u32::from_be_bytes([g0, g1, g2, g3]))
}

fn check_generation(object: KeyGeneration, key: &VaultKey) -> Result<()> {
    if object == key.generation() {
        Ok(())
    } else {
        Err(Error::KeyGenerationMismatch)
    }
}

impl VaultMetaCiphertext {
    fn aad(header: &[u8], vault_id: &VaultId, generation: KeyGeneration) -> Aad {
        Aad::with_header(header, label::VAULT_META)
            .fixed(vault_id.as_bytes())
            .u32(generation.get())
    }

    /// Encrypts vault metadata under `vk` with a fresh random nonce.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_VAULT_META_PLAINTEXT_LEN`];
    /// [`Error::Randomness`] / [`Error::Allocation`].
    pub fn seal(vk: &VaultKey, vault_id: &VaultId, meta: &[u8]) -> Result<Self> {
        Self::seal_with_nonce(vk, vault_id, meta, &aead::random_nonce()?)
    }

    pub(crate) fn seal_with_nonce(
        vk: &VaultKey,
        vault_id: &VaultId,
        meta: &[u8],
        nonce: &[u8; NONCE_LEN],
    ) -> Result<Self> {
        if meta.len() > MAX_VAULT_META_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        let generation = vk.generation();
        let header = gen_header(generation);
        let aad = Self::aad(&header, vault_id, generation);
        let bytes = seal_padded(vk.key.expose_secret(), nonce, &header, aad.as_bytes(), meta)?;
        Ok(Self { bytes, generation })
    }

    /// Strictly decodes encrypted vault metadata.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`], [`Error::UnsupportedAlgorithm`],
    /// [`Error::InvalidKeyGeneration`], [`Error::SizeLimit`] or
    /// [`Error::Malformed`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        check_version_alg(bytes)?;
        let (header, _, body) = split_nonce::<GEN_HEADER_LEN>(bytes)?;
        let generation = parse_gen_header(*header)?;
        check_padded_body(body.len(), MAX_VAULT_META_PADDED_LEN)?;
        let mut v = Vec::new();
        v.try_reserve_exact(bytes.len())
            .map_err(|_| Error::Allocation)?;
        v.extend_from_slice(bytes);
        Ok(Self {
            bytes: v,
            generation,
        })
    }

    /// The encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The vault key generation in the header.
    #[must_use]
    pub const fn key_generation(&self) -> KeyGeneration {
        self.generation
    }

    /// Decrypts the metadata. `vault_id` comes from context.
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`] if `vk` belongs to another
    /// generation; [`Error::Decryption`] for every other failure.
    pub fn open(&self, vk: &VaultKey, vault_id: &VaultId) -> Result<SecretVec> {
        check_generation(self.generation, vk)?;
        let (header, nonce, body) = split_nonce::<GEN_HEADER_LEN>(&self.bytes)?;
        let aad = Self::aad(header, vault_id, self.generation);
        open_padded(vk.key.expose_secret(), nonce, aad.as_bytes(), body)
    }
}

// ---------------------------------------------------------------------------
// Items (§7.2)
// ---------------------------------------------------------------------------

/// An encrypted item record (§7.2):
/// `version || alg || schema_version:u16 || key_generation:u32 || nonce[24] || AEAD(VK, nonce, pad(plaintext), aad)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemCiphertext {
    bytes: Vec<u8>,
    schema_version: u16,
    generation: KeyGeneration,
}

impl ItemCiphertext {
    fn header(schema_version: u16, generation: KeyGeneration) -> [u8; ITEM_HEADER_LEN] {
        let [s0, s1] = schema_version.to_be_bytes();
        let [g0, g1, g2, g3] = generation.get().to_be_bytes();
        [
            FORMAT_VERSION,
            ALG_XCHACHA20_POLY1305,
            s0,
            s1,
            g0,
            g1,
            g2,
            g3,
        ]
    }

    fn aad(
        header: &[u8],
        vault_id: &VaultId,
        item_id: &ItemId,
        schema_version: u16,
        generation: KeyGeneration,
    ) -> Aad {
        Aad::with_header(header, label::ITEM)
            .fixed(vault_id.as_bytes())
            .fixed(item_id.as_bytes())
            .u16(schema_version)
            .u32(generation.get())
    }

    /// Encrypts an item's canonical plaintext under its vault key with a
    /// fresh random nonce. `item_id` is the sync `record_id`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_ITEM_PLAINTEXT_LEN`];
    /// [`Error::Randomness`] / [`Error::Allocation`].
    pub fn seal(
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
        schema_version: u16,
        plaintext: &[u8],
    ) -> Result<Self> {
        Self::seal_with_nonce(
            vk,
            vault_id,
            item_id,
            schema_version,
            plaintext,
            &aead::random_nonce()?,
        )
    }

    pub(crate) fn seal_with_nonce(
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
        schema_version: u16,
        plaintext: &[u8],
        nonce: &[u8; NONCE_LEN],
    ) -> Result<Self> {
        if plaintext.len() > MAX_ITEM_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        let generation = vk.generation();
        let header = Self::header(schema_version, generation);
        let aad = Self::aad(&header, vault_id, item_id, schema_version, generation);
        let bytes = seal_padded(
            vk.key.expose_secret(),
            nonce,
            &header,
            aad.as_bytes(),
            plaintext,
        )?;
        Ok(Self {
            bytes,
            schema_version,
            generation,
        })
    }

    /// Strictly decodes an item record.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`], [`Error::UnsupportedAlgorithm`],
    /// [`Error::InvalidKeyGeneration`], [`Error::SizeLimit`] or
    /// [`Error::Malformed`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        check_version_alg(bytes)?;
        let (header, _, body) = split_nonce::<ITEM_HEADER_LEN>(bytes)?;
        let [_, _, s0, s1, g0, g1, g2, g3] = *header;
        let schema_version = u16::from_be_bytes([s0, s1]);
        let generation = KeyGeneration::new(u32::from_be_bytes([g0, g1, g2, g3]))?;
        check_padded_body(body.len(), MAX_ITEM_PADDED_LEN)?;
        let mut v = Vec::new();
        v.try_reserve_exact(bytes.len())
            .map_err(|_| Error::Allocation)?;
        v.extend_from_slice(bytes);
        Ok(Self {
            bytes: v,
            schema_version,
            generation,
        })
    }

    /// The encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The plaintext schema version in the header.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// The vault key generation in the header.
    #[must_use]
    pub const fn key_generation(&self) -> KeyGeneration {
        self.generation
    }

    /// Decrypts the item. `vault_id` comes from the record's vault binding
    /// and `item_id` is the sync `record_id`; a mismatch fails
    /// authentication.
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`] if `vk` belongs to another
    /// generation; [`Error::Decryption`] for every other failure (tag or
    /// padding, indistinguishably).
    pub fn open(&self, vk: &VaultKey, vault_id: &VaultId, item_id: &ItemId) -> Result<SecretVec> {
        check_generation(self.generation, vk)?;
        let (header, nonce, body) = split_nonce::<ITEM_HEADER_LEN>(&self.bytes)?;
        let aad = Self::aad(
            header,
            vault_id,
            item_id,
            self.schema_version,
            self.generation,
        );
        open_padded(vk.key.expose_secret(), nonce, aad.as_bytes(), body)
    }
}

// ---------------------------------------------------------------------------
// Tombstones (§7.2)
// ---------------------------------------------------------------------------

/// An encrypted deletion record (§7.2):
/// `version || alg || key_generation:u32 || nonce[24] || AEAD(VK, nonce, tombstone, aad)`.
/// The plaintext (final version vector and deletion time) is not padded.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TombstoneCiphertext {
    bytes: Vec<u8>,
    generation: KeyGeneration,
}

impl TombstoneCiphertext {
    fn aad(header: &[u8], vault_id: &VaultId, item_id: &ItemId, generation: KeyGeneration) -> Aad {
        Aad::with_header(header, label::TOMBSTONE)
            .fixed(vault_id.as_bytes())
            .fixed(item_id.as_bytes())
            .u32(generation.get())
    }

    /// Encrypts a tombstone with a fresh random nonce.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_TOMBSTONE_PLAINTEXT_LEN`];
    /// [`Error::Randomness`] / [`Error::Allocation`].
    pub fn seal(
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
        tombstone: &[u8],
    ) -> Result<Self> {
        Self::seal_with_nonce(vk, vault_id, item_id, tombstone, &aead::random_nonce()?)
    }

    pub(crate) fn seal_with_nonce(
        vk: &VaultKey,
        vault_id: &VaultId,
        item_id: &ItemId,
        tombstone: &[u8],
        nonce: &[u8; NONCE_LEN],
    ) -> Result<Self> {
        if tombstone.len() > MAX_TOMBSTONE_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        let generation = vk.generation();
        let header = gen_header(generation);
        let aad = Self::aad(&header, vault_id, item_id, generation);
        let bytes = aead::seal_with_prefix(
            vk.key.expose_secret(),
            nonce,
            aad.as_bytes(),
            &[&header, nonce],
            tombstone,
        )?;
        Ok(Self { bytes, generation })
    }

    /// Strictly decodes a tombstone.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`], [`Error::UnsupportedAlgorithm`],
    /// [`Error::InvalidKeyGeneration`], [`Error::SizeLimit`] or
    /// [`Error::Malformed`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        check_version_alg(bytes)?;
        let (header, _, body) = split_nonce::<GEN_HEADER_LEN>(bytes)?;
        let generation = parse_gen_header(*header)?;
        let pt_len = body.len().checked_sub(TAG_LEN).ok_or(Error::Malformed)?;
        if pt_len > MAX_TOMBSTONE_PLAINTEXT_LEN {
            return Err(Error::SizeLimit);
        }
        let mut v = Vec::new();
        v.try_reserve_exact(bytes.len())
            .map_err(|_| Error::Allocation)?;
        v.extend_from_slice(bytes);
        Ok(Self {
            bytes: v,
            generation,
        })
    }

    /// The encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The vault key generation in the header.
    #[must_use]
    pub const fn key_generation(&self) -> KeyGeneration {
        self.generation
    }

    /// Decrypts the tombstone. Both IDs come from context.
    ///
    /// # Errors
    /// [`Error::KeyGenerationMismatch`] if `vk` belongs to another
    /// generation; [`Error::Decryption`] for every other failure.
    pub fn open(&self, vk: &VaultKey, vault_id: &VaultId, item_id: &ItemId) -> Result<SecretVec> {
        check_generation(self.generation, vk)?;
        let (header, nonce, body) = split_nonce::<GEN_HEADER_LEN>(&self.bytes)?;
        let aad = Self::aad(header, vault_id, item_id, self.generation);
        aead::open(vk.key.expose_secret(), nonce, aad.as_bytes(), body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: [u8; NONCE_LEN] = [0x42; NONCE_LEN];

    fn vk() -> VaultKey {
        VaultKey {
            key: SecretBytes::from_array(&mut [0x30; 32]),
            generation: KeyGeneration::FIRST,
        }
    }

    fn ids() -> (VaultId, ItemId) {
        (
            VaultId::from_bytes([0x0B; 16]).unwrap(),
            ItemId::from_bytes([0x0C; 16]).unwrap(),
        )
    }

    /// Padded plaintexts that are not the canonical `pad` output of any
    /// message, each with the reason it is rejected.
    fn bad_paddings() -> Vec<(&'static str, Vec<u8>)> {
        let mut excess = vec![0u8; 768];
        excess[0] = 0x80;
        let mut filler = vec![0u8; 512];
        filler[10] = 0x80;
        filler[511] = 0x01;
        let mut marker = vec![0u8; 512];
        marker[10] = 0x81;
        vec![
            ("no marker", vec![0u8; 512]),
            ("excess padding", excess),
            ("non-zero filler", filler),
            ("wrong marker", marker),
            ("all 0xff", vec![0xFF; 512]),
        ]
    }

    /// Seals `padded` as-is (bypassing `pad`) under the real header and AAD,
    /// and checks that the tag really verifies, so a later failure can only
    /// come from the padding check.
    fn seal_raw(header: &[u8], aad: &Aad, padded: &[u8]) -> Vec<u8> {
        let key = vk();
        let bytes = aead::seal_with_prefix(
            key.key.expose_secret(),
            &NONCE,
            aad.as_bytes(),
            &[header, &NONCE],
            padded,
        )
        .unwrap();
        let body = &bytes[header.len() + NONCE_LEN..];
        let pt = aead::open(key.key.expose_secret(), &NONCE, aad.as_bytes(), body).unwrap();
        assert_eq!(pt.expose_secret(), padded, "tag must verify");
        bytes
    }

    /// §7.2: an authentic item body whose padding is not canonical fails
    /// with the same `Decryption` error as a bad tag, never `Malformed`.
    #[test]
    fn item_padding_failure_is_indistinguishable_from_tag_failure() {
        let (vault, item) = ids();
        let key = vk();
        let header = ItemCiphertext::header(1, KeyGeneration::FIRST);
        let aad = ItemCiphertext::aad(&header, &vault, &item, 1, KeyGeneration::FIRST);
        for (why, bad) in bad_paddings() {
            let bytes = seal_raw(&header, &aad, &bad);
            let ct = ItemCiphertext::from_bytes(&bytes).unwrap();
            assert_eq!(
                ct.open(&key, &vault, &item),
                Err(Error::Decryption),
                "{why}"
            );
        }
        // Control: canonical padding sealed the same way opens, and the
        // padding is stripped.
        let good = encoding::pad(b"x").unwrap();
        let bytes = seal_raw(&header, &aad, good.expose_secret());
        let ct = ItemCiphertext::from_bytes(&bytes).unwrap();
        assert_eq!(ct.open(&key, &vault, &item).unwrap().expose_secret(), b"x");
    }

    #[test]
    fn vault_meta_padding_failure_is_indistinguishable_from_tag_failure() {
        let (vault, _) = ids();
        let key = vk();
        let header = gen_header(KeyGeneration::FIRST);
        let aad = VaultMetaCiphertext::aad(&header, &vault, KeyGeneration::FIRST);
        for (why, bad) in bad_paddings() {
            let bytes = seal_raw(&header, &aad, &bad);
            let ct = VaultMetaCiphertext::from_bytes(&bytes).unwrap();
            assert_eq!(ct.open(&key, &vault), Err(Error::Decryption), "{why}");
        }
        let good = encoding::pad(b"meta").unwrap();
        let bytes = seal_raw(&header, &aad, good.expose_secret());
        let ct = VaultMetaCiphertext::from_bytes(&bytes).unwrap();
        assert_eq!(ct.open(&key, &vault).unwrap().expose_secret(), b"meta");
    }

    /// Tombstones are not padded: a 0-byte plaintext, and a plaintext that
    /// would be invalid padding, both open unchanged.
    #[test]
    fn tombstones_are_not_unpadded() {
        let (vault, item) = ids();
        let key = vk();
        for pt in [&[][..], &[0u8; 512][..], &[0x80][..]] {
            let ct = TombstoneCiphertext::seal(&key, &vault, &item, pt).unwrap();
            assert_eq!(
                ct.as_bytes().len(),
                GEN_HEADER_LEN + NONCE_LEN + pt.len() + TAG_LEN
            );
            let back = TombstoneCiphertext::from_bytes(ct.as_bytes()).unwrap();
            assert_eq!(back.open(&key, &vault, &item).unwrap().expose_secret(), pt);
        }
    }
}
