//! KeePass KDBX 4 databases (KeePass 2.35+, `KeePassXC` 2.7+, `KeePassDX`,
//! Strongbox). KDBX 3.1 and KeePass 1 (KDB) files are refused with
//! [`Error::UnsupportedVersion`].
//!
//! The file format, as specified by KeePass (keepass.info, "KDBX 4") and
//! implemented here:
//!
//! 1. **Outer header**: signatures `9AA2D903` `B54BFB67`, version 4.x, then
//!    TLV fields (`u8` ID, `u32` length): cipher UUID, compression flag,
//!    32-byte master seed, IV, and the KDF parameters as a `VariantDictionary`.
//!    The header is followed by its SHA-256 (corruption check) and its
//!    HMAC-SHA-256 (key check).
//! 2. **Keys**: composite key `SHA-256(SHA-256(password))` (password only;
//!    key files are not supported), transformed by the KDF (AES-KDF, or
//!    Argon2d / Argon2id v1.3 or v1.0); cipher key
//!    `SHA-256(seed || transformed)`; HMAC key
//!    `SHA-512(seed || transformed || 0x01)`, per block
//!    `SHA-512(u64_le(index) || hmac_key)` (header: index `u64::MAX`).
//! 3. **Payload**: HMAC-authenticated blocks (`HMAC-SHA-256(block key,
//!    u64_le(index) || u32_le(len) || data)`), ending with an empty block.
//!    Every block is verified before anything is decrypted.
//! 4. **Decryption**: AES-256-CBC with PKCS#7 or `ChaCha20` (RFC 8439, 96-bit
//!    nonce); Twofish is refused with [`Error::Unsupported`]. Then gzip if
//!    flagged.
//! 5. **Inner header** (TLV): the protected-value stream (`ChaCha20`, key and
//!    nonce from `SHA-512(stream key)`, or `Salsa20` with key
//!    `SHA-256(stream key)` and the fixed nonce `E830094B97205D2A`) and the
//!    attachment binaries.
//! 6. **XML**: groups and entries. Values marked `Protected="True"` are
//!    base64, XOR-ed with the stream, in document order.
//!
//! Mapping: every entry becomes a login (an entry with only notes a secure
//! note). `Title`, `UserName`, `Password`, `URL` and `Notes` map to the
//! item; `otp` (`KeePassXC`), `TimeOtp-*` / `HmacOtp-*` (KeePass 2.47+) and
//! `TOTP Seed` / `TOTP Settings` (`KeeTrayTOTP`) to its OTP; `KP2A_URL*` to
//! further URLs; every other string to a custom field (hidden if
//! protected). Attachments, tags and the password history (from the
//! entry's `History`) are kept; the group path (without the root group)
//! becomes a tag, and entries in the recycle bin go to the trash.
//!
//! Limits (THREAT_MODEL §4.1 "D"): Argon2 memory at most
//! [`MAX_ARGON2_MEMORY_KIB`], Argon2 work (memory × passes) at most
//! [`MAX_ARGON2_WORK`], at most [`MAX_AES_KDF_ROUNDS`] AES-KDF rounds, and
//! at most [`MAX_PAYLOAD_LEN`] of decompressed payload. A file above them
//! fails with [`Error::LimitExceeded`] before any key derivation runs.

mod crypto;
mod payload;
#[cfg(test)]
mod tests;

use flate2::read::GzDecoder;
use mosaic_core::SecretString;

use crate::buf::{self, Buf};
use crate::entries::Import;
use crate::error::{Error, Result};

pub(crate) use payload::parse_payload;

/// Most Argon2 memory a file may ask for: 1 GiB (in KiB).
#[cfg(not(fuzzing))]
pub const MAX_ARGON2_MEMORY_KIB: u32 = 1024 * 1024;
/// Most Argon2 memory a file may ask for (fuzzing builds: 64 KiB).
#[cfg(fuzzing)]
pub const MAX_ARGON2_MEMORY_KIB: u32 = 64;
/// Most Argon2 work, memory in KiB times passes: 64 GiB of memory filled
/// in total (about a minute on a current CPU, single-threaded). `KeePassXC`'s
/// defaults (64 MiB, a second's worth of passes) are far below.
#[cfg(not(fuzzing))]
pub const MAX_ARGON2_WORK: u64 = 64 * 1024 * 1024;
/// Most Argon2 work (fuzzing builds).
#[cfg(fuzzing)]
pub const MAX_ARGON2_WORK: u64 = 256;
/// Most AES-KDF rounds.
#[cfg(not(fuzzing))]
pub const MAX_AES_KDF_ROUNDS: u64 = 2_000_000_000;
/// Most AES-KDF rounds (fuzzing builds).
#[cfg(fuzzing)]
pub const MAX_AES_KDF_ROUNDS: u64 = 64;
/// Largest decrypted and decompressed payload: 256 MiB.
pub const MAX_PAYLOAD_LEN: usize = 256 * 1024 * 1024;

const SIGNATURE_1: u32 = 0x9AA2_D903;
const SIGNATURE_2: u32 = 0xB54B_FB67;
/// KeePass 1.x (KDB) and the KeePass 2 pre-release format.
const SIGNATURE_2_OLD: [u32; 2] = [0xB54B_FB65, 0xB54B_FB66];
const MAX_HEADER_FIELDS: usize = 64;

/// A little-endian reader over a byte slice.
pub(crate) struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { rest: data }
    }

    pub(crate) const fn remaining(&self) -> &'a [u8] {
        self.rest
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let (head, tail) = self.rest.split_at_checked(n).ok_or(Error::Malformed)?;
        self.rest = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| Error::Malformed)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(u8::from_le_bytes(self.array()?))
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// A `u32` length followed by that many bytes.
    pub(crate) fn sized(&mut self) -> Result<&'a [u8]> {
        let len = usize::try_from(self.u32()?).map_err(|_| Error::Malformed)?;
        self.take(len)
    }
}

/// The outer cipher.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Cipher {
    Aes256Cbc,
    ChaCha20,
}

/// `31c1f2e6-bf71-4350-be58-05216afc5aff`.
pub(crate) const CIPHER_AES256: [u8; 16] = [
    0x31, 0xc1, 0xf2, 0xe6, 0xbf, 0x71, 0x43, 0x50, 0xbe, 0x58, 0x05, 0x21, 0x6a, 0xfc, 0x5a, 0xff,
];
/// `d6038a2b-8b6f-4cb5-a524-339a31dbb59a`.
pub(crate) const CIPHER_CHACHA20: [u8; 16] = [
    0xd6, 0x03, 0x8a, 0x2b, 0x8b, 0x6f, 0x4c, 0xb5, 0xa5, 0x24, 0x33, 0x9a, 0x31, 0xdb, 0xb5, 0x9a,
];

/// The parsed outer header.
pub(crate) struct Header<'a> {
    /// The header bytes (signatures to end-of-header field).
    bytes: &'a [u8],
    cipher: Cipher,
    compressed: bool,
    master_seed: &'a [u8],
    iv: &'a [u8],
    kdf: crypto::Kdf<'a>,
}

impl<'a> Header<'a> {
    pub(crate) fn parse(input: &'a [u8]) -> Result<Self> {
        let mut r = Cursor::new(input);
        let sig1 = r.u32().map_err(|_| Error::Malformed)?;
        let sig2 = r.u32()?;
        if sig1 == SIGNATURE_1 && SIGNATURE_2_OLD.contains(&sig2) {
            return Err(Error::UnsupportedVersion);
        }
        if sig1 != SIGNATURE_1 || sig2 != SIGNATURE_2 {
            return Err(Error::Malformed);
        }
        let version = r.u32()?;
        if version >> 16 != 4 {
            return Err(Error::UnsupportedVersion);
        }
        let mut cipher = None;
        let mut compressed = None;
        let mut master_seed = None;
        let mut iv = None;
        let mut kdf = None;
        for _ in 0..MAX_HEADER_FIELDS {
            let id = r.u8()?;
            let data = r.sized()?;
            match id {
                0 => {
                    let len = input.len().saturating_sub(r.remaining().len());
                    let bytes = input.get(..len).ok_or(Error::Malformed)?;
                    // Anything else (Twofish) is not read.
                    let cipher = match cipher.ok_or(Error::Malformed)? {
                        CIPHER_AES256 => Cipher::Aes256Cbc,
                        CIPHER_CHACHA20 => Cipher::ChaCha20,
                        _ => return Err(Error::Unsupported),
                    };
                    let iv: &[u8] = iv.ok_or(Error::Malformed)?;
                    let iv_ok = match cipher {
                        Cipher::Aes256Cbc => iv.len() == 16,
                        Cipher::ChaCha20 => iv.len() == 12,
                    };
                    let master_seed: &[u8] = master_seed.ok_or(Error::Malformed)?;
                    if !iv_ok || master_seed.len() != 32 {
                        return Err(Error::Malformed);
                    }
                    return Ok(Self {
                        bytes,
                        cipher,
                        compressed: compressed.ok_or(Error::Malformed)?,
                        master_seed,
                        iv,
                        kdf: kdf.ok_or(Error::Malformed)?,
                    });
                }
                2 => {
                    let uuid: [u8; 16] = data.try_into().map_err(|_| Error::Malformed)?;
                    set(&mut cipher, uuid)?;
                }
                3 => {
                    let flag = u32::from_le_bytes(data.try_into().map_err(|_| Error::Malformed)?);
                    let gzip = match flag {
                        0 => false,
                        1 => true,
                        _ => return Err(Error::Unsupported),
                    };
                    set(&mut compressed, gzip)?;
                }
                4 => set(&mut master_seed, data)?,
                7 => set(&mut iv, data)?,
                11 => set(&mut kdf, crypto::Kdf::parse(data)?)?,
                // Comment, public custom data, and KDBX 3 fields: ignored.
                _ => {}
            }
        }
        Err(Error::Malformed)
    }
}

/// Sets a header field that must appear once.
fn set<T>(slot: &mut Option<T>, value: T) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(Error::Malformed);
    }
    Ok(())
}

/// Reads a KDBX 4 database opened with `password`.
///
/// # Errors
/// [`Error::Malformed`] for a file that is not KDBX or is damaged;
/// [`Error::UnsupportedVersion`] for KDBX 3.1 and older;
/// [`Error::Unsupported`] for Twofish, an unknown KDF or compression;
/// [`Error::LimitExceeded`] for key-derivation parameters above the limits;
/// [`Error::WrongCredentials`] if the password does not open the file;
/// [`Error::TooLarge`].
pub fn parse(input: &[u8], password: &SecretString) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let header = Header::parse(input)?;
    header.kdf.check_limits()?;
    let mut r = Cursor::new(input.get(header.bytes.len()..).ok_or(Error::Malformed)?);
    let stored_sha = r.take(32)?;
    let stored_hmac = r.take(32)?;
    if !crypto::header_hash_matches(header.bytes, stored_sha) {
        return Err(Error::Malformed);
    }
    let composite = crypto::composite_key(password);
    let transformed = header.kdf.transform(&composite)?;
    let keys = crypto::Keys::derive(header.master_seed, &transformed);
    if !keys.verify(u64::MAX, &[header.bytes], stored_hmac) {
        return Err(Error::WrongCredentials);
    }
    let ciphertext = read_blocks(&mut r, &keys)?;
    let plaintext = crypto::decrypt(header.cipher, &keys, header.iv, ciphertext)?;
    let payload = if header.compressed {
        let hint = plaintext.len().saturating_mul(4);
        buf::read_limited(GzDecoder::new(plaintext.as_slice()), MAX_PAYLOAD_LEN, hint)?
    } else {
        plaintext
    };
    parse_payload(&payload)
}

/// Verifies and concatenates the HMAC blocks.
fn read_blocks(r: &mut Cursor<'_>, keys: &crypto::Keys) -> Result<Buf> {
    let mut out = buf::with_capacity(r.remaining().len())?;
    let mut index = 0u64;
    loop {
        let mac = r.take(32)?;
        let len_bytes = r.take(4)?;
        let len = usize::try_from(u32::from_le_bytes(
            len_bytes.try_into().map_err(|_| Error::Malformed)?,
        ))
        .map_err(|_| Error::Malformed)?;
        let data = r.take(len)?;
        if !keys.verify(index, &[&index.to_le_bytes(), len_bytes, data], mac) {
            return Err(Error::Malformed);
        }
        if len == 0 {
            return Ok(out);
        }
        buf::append(&mut out, data, crate::MAX_FILE_LEN)?;
        index = index.checked_add(1).ok_or(Error::Malformed)?;
    }
}
