//! KDBX 4 key derivation, block authentication and outer decryption.
//!
//! Only standard primitives from RustCrypto, composed exactly as the KDBX 4
//! format prescribes; nothing here protects Mosaic's own data.

use aes::Aes256;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockCipherEncrypt, BlockModeDecrypt, KeyInit, KeyIvInit, StreamCipher};
use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use hmac::{Hmac, Mac};
use mosaic_core::SecretString;
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroizing;

use super::{Cipher, Cursor, MAX_AES_KDF_ROUNDS, MAX_ARGON2_MEMORY_KIB, MAX_ARGON2_WORK};
use crate::buf::Buf;
use crate::error::{Error, Result};

/// `c9d9f39a-628a-4460-bf74-0d08c18a4fea` (AES-KDF in KDBX 4).
const KDF_AES: [u8; 16] = [
    0xc9, 0xd9, 0xf3, 0x9a, 0x62, 0x8a, 0x44, 0x60, 0xbf, 0x74, 0x0d, 0x08, 0xc1, 0x8a, 0x4f, 0xea,
];
/// `7c02bb82-79a7-4ac0-927d-114a00648238` (AES-KDF as KDBX 3.1 names it).
const KDF_AES_LEGACY: [u8; 16] = [
    0x7c, 0x02, 0xbb, 0x82, 0x79, 0xa7, 0x4a, 0xc0, 0x92, 0x7d, 0x11, 0x4a, 0x00, 0x64, 0x82, 0x38,
];
/// `ef636ddf-8c29-444b-91f7-a9a403e30a0c`.
pub(crate) const KDF_ARGON2D: [u8; 16] = [
    0xef, 0x63, 0x6d, 0xdf, 0x8c, 0x29, 0x44, 0x4b, 0x91, 0xf7, 0xa9, 0xa4, 0x03, 0xe3, 0x0a, 0x0c,
];
/// `9e298b19-56db-4773-b23d-fc3ec6f0a1e6`.
pub(crate) const KDF_ARGON2ID: [u8; 16] = [
    0x9e, 0x29, 0x8b, 0x19, 0x56, 0xdb, 0x47, 0x73, 0xb2, 0x3d, 0xfc, 0x3e, 0xc6, 0xf0, 0xa1, 0xe6,
];

/// Most entries of a `VariantDictionary`.
const MAX_VARIANTS: usize = 64;

/// One value of a `VariantDictionary`.
enum Variant<'a> {
    U32(u32),
    U64(u64),
    Bytes(&'a [u8]),
    Other,
}

/// The key derivation of a file, as its header describes it.
pub(crate) enum Kdf<'a> {
    Aes {
        rounds: u64,
        seed: &'a [u8],
    },
    Argon2 {
        algorithm: Algorithm,
        version: Version,
        salt: &'a [u8],
        memory_kib: u32,
        passes: u32,
        lanes: u32,
        secret: Option<&'a [u8]>,
        data: Option<&'a [u8]>,
    },
}

/// Parses a `VariantDictionary` (version 1.x).
fn variants(data: &[u8]) -> Result<Vec<(&[u8], Variant<'_>)>> {
    let mut r = Cursor::new(data);
    if r.u16()? >> 8 != 1 {
        return Err(Error::Unsupported);
    }
    let mut out = Vec::new();
    loop {
        let ty = r.u8()?;
        if ty == 0 {
            return Ok(out);
        }
        if out.len() >= MAX_VARIANTS {
            return Err(Error::Malformed);
        }
        let name = r.sized()?;
        let value = r.sized()?;
        let v = match ty {
            0x04 => Variant::U32(u32::from_le_bytes(
                value.try_into().map_err(|_| Error::Malformed)?,
            )),
            0x05 => Variant::U64(u64::from_le_bytes(
                value.try_into().map_err(|_| Error::Malformed)?,
            )),
            0x42 => Variant::Bytes(value),
            // bool, i32, i64, string: not used by the KDFs read here.
            0x08 | 0x0C | 0x0D | 0x18 => Variant::Other,
            _ => return Err(Error::Malformed),
        };
        if out.iter().any(|(n, _)| *n == name) {
            return Err(Error::Malformed);
        }
        out.push((name, v));
    }
}

impl<'a> Kdf<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let vars = variants(data)?;
        let get = |name: &str| {
            vars.iter()
                .find(|(n, _)| *n == name.as_bytes())
                .map(|(_, v)| v)
        };
        let bytes = |name: &str| match get(name) {
            Some(Variant::Bytes(b)) => Ok(Some(*b)),
            None => Ok(None),
            Some(_) => Err(Error::Malformed),
        };
        let u64v = |name: &str| match get(name) {
            Some(Variant::U64(v)) => Ok(*v),
            _ => Err(Error::Malformed),
        };
        let u32v = |name: &str| match get(name) {
            Some(Variant::U32(v)) => Ok(*v),
            _ => Err(Error::Malformed),
        };
        let uuid = bytes("$UUID")?.ok_or(Error::Malformed)?;
        if uuid == KDF_AES || uuid == KDF_AES_LEGACY {
            let seed = bytes("S")?.ok_or(Error::Malformed)?;
            if seed.len() != 32 {
                return Err(Error::Malformed);
            }
            return Ok(Self::Aes {
                rounds: u64v("R")?,
                seed,
            });
        }
        let algorithm = if uuid == KDF_ARGON2D {
            Algorithm::Argon2d
        } else if uuid == KDF_ARGON2ID {
            Algorithm::Argon2id
        } else {
            return Err(Error::Unsupported);
        };
        let version = match u32v("V")? {
            0x10 => Version::V0x10,
            0x13 => Version::V0x13,
            _ => return Err(Error::Unsupported),
        };
        let memory = u64v("M")?;
        let memory_kib = u32::try_from(memory / 1024).map_err(|_| Error::LimitExceeded)?;
        let passes = u32::try_from(u64v("I")?).map_err(|_| Error::LimitExceeded)?;
        let data = bytes("A")?.filter(|a| !a.is_empty());
        if data.is_some_and(|a| a.len() > argon2::Params::MAX_DATA_LEN) {
            return Err(Error::Unsupported);
        }
        Ok(Self::Argon2 {
            algorithm,
            version,
            salt: bytes("S")?.ok_or(Error::Malformed)?,
            memory_kib,
            passes,
            lanes: u32v("P")?,
            secret: bytes("K")?.filter(|k| !k.is_empty()),
            data,
        })
    }

    /// Refuses parameters above the limits before any work is done.
    pub(crate) fn check_limits(&self) -> Result<()> {
        match *self {
            Self::Aes { rounds, .. } => {
                if rounds > MAX_AES_KDF_ROUNDS {
                    return Err(Error::LimitExceeded);
                }
            }
            Self::Argon2 {
                memory_kib,
                passes,
                lanes,
                ..
            } => {
                let work = u64::from(memory_kib).saturating_mul(u64::from(passes));
                if memory_kib > MAX_ARGON2_MEMORY_KIB || work > MAX_ARGON2_WORK || lanes > 256 {
                    return Err(Error::LimitExceeded);
                }
            }
        }
        Ok(())
    }

    /// The transformed key.
    pub(crate) fn transform(&self, composite: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>> {
        self.check_limits()?;
        match *self {
            Self::Aes { rounds, seed } => {
                let cipher = Aes256::new_from_slice(seed).map_err(|_| Error::Malformed)?;
                let mut block = Zeroizing::new(*composite);
                for half in block.as_chunks_mut::<16>().0 {
                    let mut b = Zeroizing::new(aes::Block::default());
                    b.copy_from_slice(half);
                    for _ in 0..rounds {
                        cipher.encrypt_block(&mut b);
                    }
                    half.copy_from_slice(&b);
                }
                Ok(sha256(&[block.as_slice()]))
            }
            Self::Argon2 {
                algorithm,
                version,
                salt,
                memory_kib,
                passes,
                lanes,
                secret,
                data,
            } => {
                let mut builder = ParamsBuilder::new();
                builder
                    .m_cost(memory_kib)
                    .t_cost(passes)
                    .p_cost(lanes)
                    .output_len(32);
                if let Some(a) = data {
                    builder.data(AssociatedData::new(a).map_err(|_| Error::Unsupported)?);
                }
                let params = builder.build().map_err(|_| Error::Malformed)?;
                let argon = match secret {
                    Some(k) => Argon2::new_with_secret(k, algorithm, version, params)
                        .map_err(|_| Error::Unsupported)?,
                    None => Argon2::new(algorithm, version, params),
                };
                let mut out = Zeroizing::new([0u8; 32]);
                argon
                    .hash_password_into(composite, salt, out.as_mut_slice())
                    .map_err(|_| Error::Malformed)?;
                Ok(out)
            }
        }
    }
}

fn sha256(parts: &[&[u8]]) -> Zeroizing<[u8; 32]> {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    Zeroizing::new(h.finalize().into())
}

fn sha512(parts: &[&[u8]]) -> Zeroizing<[u8; 64]> {
    let mut h = Sha512::new();
    for p in parts {
        h.update(p);
    }
    Zeroizing::new(h.finalize().into())
}

/// Whether `stored` is the SHA-256 of the header (a corruption check over
/// public bytes; the key check is the header HMAC).
pub(crate) fn header_hash_matches(header: &[u8], stored: &[u8]) -> bool {
    sha256(&[header]).as_slice() == stored
}

/// The composite key of a password-only database:
/// `SHA-256(SHA-256(UTF-8 password))`.
pub(crate) fn composite_key(password: &SecretString) -> Zeroizing<[u8; 32]> {
    let inner = sha256(&[password.expose_secret().as_bytes()]);
    sha256(&[inner.as_slice()])
}

/// The keys of one file.
pub(crate) struct Keys {
    cipher: Zeroizing<[u8; 32]>,
    hmac: Zeroizing<[u8; 64]>,
}

impl Keys {
    pub(crate) fn derive(master_seed: &[u8], transformed: &[u8; 32]) -> Self {
        Self {
            cipher: sha256(&[master_seed, transformed]),
            hmac: sha512(&[master_seed, transformed, &[1]]),
        }
    }

    /// The HMAC key of block `index` (`u64::MAX` for the header).
    fn block_key(&self, index: u64) -> Zeroizing<[u8; 64]> {
        sha512(&[&index.to_le_bytes(), self.hmac.as_slice()])
    }

    /// Verifies `tag` over the concatenation of `parts` in constant time.
    pub(crate) fn verify(&self, index: u64, parts: &[&[u8]], tag: &[u8]) -> bool {
        let key = self.block_key(index);
        let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(key.as_slice()) else {
            return false;
        };
        for p in parts {
            mac.update(p);
        }
        mac.verify_slice(tag).is_ok()
    }

    /// The tag of `parts` (tests write files with it).
    #[cfg(test)]
    pub(crate) fn tag(&self, index: u64, parts: &[&[u8]]) -> [u8; 32] {
        let key = self.block_key(index);
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key.as_slice()).unwrap();
        for p in parts {
            mac.update(p);
        }
        mac.finalize().into_bytes().into()
    }

    #[cfg(test)]
    pub(crate) fn cipher_key(&self) -> &[u8; 32] {
        &self.cipher
    }
}

/// Decrypts the payload in place.
pub(crate) fn decrypt(cipher: Cipher, keys: &Keys, iv: &[u8], mut data: Buf) -> Result<Buf> {
    match cipher {
        Cipher::Aes256Cbc => {
            let dec = cbc::Decryptor::<Aes256>::new_from_slices(keys.cipher.as_slice(), iv)
                .map_err(|_| Error::Malformed)?;
            let len = dec
                .decrypt_padded::<Pkcs7>(data.as_mut_slice())
                .map_err(|_| Error::Malformed)?
                .len();
            data.truncate(len);
        }
        Cipher::ChaCha20 => {
            let mut c = chacha20::ChaCha20::new_from_slices(keys.cipher.as_slice(), iv)
                .map_err(|_| Error::Malformed)?;
            c.try_apply_keystream(data.as_mut_slice())
                .map_err(|_| Error::Malformed)?;
        }
    }
    Ok(data)
}
