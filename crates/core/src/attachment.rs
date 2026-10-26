//! Attachment encryption with STREAM-BE32 over XChaCha20-Poly1305
//! (CRYPTO_SPEC §7.3), using RustCrypto's `aead-stream`.
//!
//! Layout:
//! ```text
//! version:u8 (0x01) || alg:u8 (0x02) || chunk_log2:u8 (16) || nonce_prefix[19] || chunks...
//! ```
//! The plaintext is split into 64 KiB chunks; each ciphertext chunk is the
//! chunk plus a 16-byte tag. Every chunk but the last is exactly
//! 65 536 + 16 bytes; the last one carries STREAM's last-block flag in its
//! nonce, so truncation and extension are detected. An empty file is a
//! single empty last chunk. The header (`version || alg || chunk_log2`) is
//! bound into every chunk's AAD:
//! `aad = header || aad("attachment", vault_id, item_id, attachment_id)`.
//! The nonce prefix is authenticated through the per-chunk nonce.

use aead_stream::{NewStream, StreamBE32, StreamPrimitive};
use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::KeyInit;

use crate::aead::TAG_LEN;
use crate::encoding::{Aad, label, take};
use crate::envelope::FORMAT_VERSION;
use crate::error::{Error, Result};
use crate::ids::{AttachmentId, ItemId, VaultId};
use crate::rng;
use crate::secret::{AttachmentKey, SecretVec};

/// Algorithm ID of STREAM-BE32-XChaCha20-Poly1305.
pub const ALG_STREAM_BE32_XCHACHA20_POLY1305: u8 = 0x02;
/// `log2` of the plaintext chunk size.
pub const CHUNK_LOG2: u8 = 16;
/// Plaintext bytes per chunk (64 KiB).
pub const CHUNK_LEN: usize = 1 << CHUNK_LOG2;
/// Length of the STREAM nonce prefix (24 - 5 bytes of counter and flag).
pub const NONCE_PREFIX_LEN: usize = 19;
/// Default per-attachment size limit: 50 MiB of plaintext (§7.3).
pub const MAX_ATTACHMENT_LEN: usize = 50 * 1024 * 1024;

const HEADER_LEN: usize = 3;
const HEADER: [u8; HEADER_LEN] = [
    FORMAT_VERSION,
    ALG_STREAM_BE32_XCHACHA20_POLY1305,
    CHUNK_LOG2,
];
const PREFIX_LEN: usize = HEADER_LEN + NONCE_PREFIX_LEN;
const CHUNK_CT_LEN: usize = CHUNK_LEN + TAG_LEN;

type Stream = StreamBE32<XChaCha20Poly1305>;

fn stream(fk: &AttachmentKey, prefix: &[u8; NONCE_PREFIX_LEN]) -> Stream {
    let aead = XChaCha20Poly1305::new(fk.expose_secret().into());
    Stream::from_aead(aead, &(*prefix).into())
}

fn chunk_aad(vault_id: &VaultId, item_id: &ItemId, attachment_id: &AttachmentId) -> Aad {
    Aad::with_header(&HEADER, label::ATTACHMENT)
        .fixed(vault_id.as_bytes())
        .fixed(item_id.as_bytes())
        .fixed(attachment_id.as_bytes())
}

/// Number of chunks for a plaintext of `len` bytes (at least one).
fn chunk_count(len: usize) -> usize {
    len.div_ceil(CHUNK_LEN).max(1)
}

/// The encrypted size of an attachment of `plaintext_len` bytes, so clients
/// can check quotas before encrypting.
///
/// # Errors
/// [`Error::SizeLimit`] above [`MAX_ATTACHMENT_LEN`].
pub fn ciphertext_len(plaintext_len: usize) -> Result<usize> {
    if plaintext_len > MAX_ATTACHMENT_LEN {
        return Err(Error::SizeLimit);
    }
    chunk_count(plaintext_len)
        .checked_mul(TAG_LEN)
        .and_then(|tags| tags.checked_add(plaintext_len))
        .and_then(|n| n.checked_add(PREFIX_LEN))
        .ok_or(Error::SizeLimit)
}

fn position(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(|_| Error::SizeLimit)
}

/// Encrypts an attachment under `FK` with a fresh random nonce prefix.
///
/// # Errors
/// [`Error::SizeLimit`] above [`MAX_ATTACHMENT_LEN`];
/// [`Error::Randomness`] / [`Error::Allocation`].
pub fn seal_attachment(
    fk: &AttachmentKey,
    vault_id: &VaultId,
    item_id: &ItemId,
    attachment_id: &AttachmentId,
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let prefix: [u8; NONCE_PREFIX_LEN] = rng::array()?;
    seal_attachment_with_prefix(fk, vault_id, item_id, attachment_id, plaintext, &prefix)
}

pub(crate) fn seal_attachment_with_prefix(
    fk: &AttachmentKey,
    vault_id: &VaultId,
    item_id: &ItemId,
    attachment_id: &AttachmentId,
    plaintext: &[u8],
    prefix: &[u8; NONCE_PREFIX_LEN],
) -> Result<Vec<u8>> {
    let total = ciphertext_len(plaintext.len())?;
    let mut out = Vec::new();
    out.try_reserve_exact(total)
        .map_err(|_| Error::Allocation)?;
    out.extend_from_slice(&HEADER);
    out.extend_from_slice(prefix);

    let s = stream(fk, prefix);
    let aad = chunk_aad(vault_id, item_id, attachment_id);
    // Never larger than one chunk plus tag; small files get a small buffer.
    let mut buf = SecretVec::with_capacity(plaintext.len().min(CHUNK_LEN).saturating_add(TAG_LEN))?;
    let count = chunk_count(plaintext.len());
    let chunks = plaintext.chunks(CHUNK_LEN).chain(
        // `chunks` yields nothing for an empty input; emit one empty chunk.
        plaintext.is_empty().then_some(&[][..]),
    );
    for (i, chunk) in chunks.enumerate() {
        let last = i.checked_add(1) == Some(count);
        let v = buf.as_mut_vec();
        v.clear();
        v.extend_from_slice(chunk);
        s.encrypt_in_place(position(i)?, last, aad.as_bytes(), v)
            .map_err(|_| Error::SizeLimit)?;
        out.extend_from_slice(v);
    }
    Ok(out)
}

/// Decrypts an attachment. All three IDs come from context.
///
/// # Errors
/// [`Error::UnsupportedVersion`] / [`Error::UnsupportedAlgorithm`] for an
/// unknown header, [`Error::SizeLimit`] above the size limit,
/// [`Error::Malformed`] for an impossible length, and [`Error::Decryption`]
/// for every authentication failure (tampering, reordering, truncation,
/// wrong key or IDs).
pub fn open_attachment(
    fk: &AttachmentKey,
    vault_id: &VaultId,
    item_id: &ItemId,
    attachment_id: &AttachmentId,
    ciphertext: &[u8],
) -> Result<SecretVec> {
    let ([version, alg, chunk_log2], rest) = take::<HEADER_LEN>(ciphertext)?;
    if *version != FORMAT_VERSION {
        return Err(Error::UnsupportedVersion);
    }
    if *alg != ALG_STREAM_BE32_XCHACHA20_POLY1305 || *chunk_log2 != CHUNK_LOG2 {
        return Err(Error::UnsupportedAlgorithm);
    }
    let (prefix, body) = take::<NONCE_PREFIX_LEN>(rest)?;
    if ciphertext.len() > ciphertext_len(MAX_ATTACHMENT_LEN)? {
        return Err(Error::SizeLimit);
    }
    if body.len() < TAG_LEN {
        return Err(Error::Malformed);
    }
    let count = body.len().div_ceil(CHUNK_CT_LEN);
    let pt_len = count
        .checked_mul(TAG_LEN)
        .and_then(|tags| body.len().checked_sub(tags))
        .ok_or(Error::Malformed)?;
    // Canonical form: the last chunk is non-empty unless it is the only one,
    // i.e. the chunk count matches what the encoder would produce.
    if chunk_count(pt_len) != count || ciphertext_len(pt_len)? != ciphertext.len() {
        return Err(Error::Malformed);
    }

    let s = stream(fk, prefix);
    let aad = chunk_aad(vault_id, item_id, attachment_id);
    let mut out = SecretVec::with_capacity(pt_len)?;
    let mut buf = SecretVec::with_capacity(body.len().min(CHUNK_CT_LEN))?;
    for (i, chunk) in body.chunks(CHUNK_CT_LEN).enumerate() {
        let last = i.checked_add(1) == Some(count);
        let v = buf.as_mut_vec();
        v.clear();
        v.extend_from_slice(chunk);
        s.decrypt_in_place(position(i)?, last, aad.as_bytes(), v)
            .map_err(|_| Error::Decryption)?;
        out.as_mut_vec().extend_from_slice(v);
    }
    Ok(out)
}

/// An independent construction of the §7.3 format for tests: plain
/// single-shot XChaCha20-Poly1305 ([`crate::aead`], pinned by the draft KAT)
/// per chunk, with the nonce `prefix || BE32(i) || last` and the AAD written
/// out byte by byte. It shares no code with the `aead-stream` path, so an
/// upstream change to STREAM's nonce layout cannot hide behind re-blessed
/// golden files.
#[cfg(test)]
pub(crate) mod reference {
    use super::{CHUNK_LEN, NONCE_PREFIX_LEN, TAG_LEN};
    use crate::aead::{self, NONCE_LEN};

    pub(crate) fn aad(vault: &[u8; 16], item: &[u8; 16], attachment: &[u8; 16]) -> Vec<u8> {
        let mut aad = vec![0x01, 0x02, 0x10];
        aad.extend_from_slice(b"mosaic/v1/attachment");
        aad.push(0x00);
        aad.extend_from_slice(vault);
        aad.extend_from_slice(item);
        aad.extend_from_slice(attachment);
        aad
    }

    pub(crate) fn nonce(prefix: &[u8; NONCE_PREFIX_LEN], i: u32, last: bool) -> [u8; NONCE_LEN] {
        let mut n = [0u8; NONCE_LEN];
        n[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
        n[NONCE_PREFIX_LEN..NONCE_LEN - 1].copy_from_slice(&i.to_be_bytes());
        n[NONCE_LEN - 1] = u8::from(last);
        n
    }

    fn plaintext_chunks(pt: &[u8]) -> Vec<&[u8]> {
        if pt.is_empty() {
            vec![&[]]
        } else {
            pt.chunks(CHUNK_LEN).collect()
        }
    }

    pub(crate) fn seal(
        key: &[u8; 32],
        aad: &[u8],
        prefix: &[u8; NONCE_PREFIX_LEN],
        pt: &[u8],
    ) -> Vec<u8> {
        let mut out = vec![0x01, 0x02, 0x10];
        out.extend_from_slice(prefix);
        let chunks = plaintext_chunks(pt);
        let n = chunks.len();
        for (i, chunk) in chunks.into_iter().enumerate() {
            let nonce = nonce(prefix, u32::try_from(i).unwrap(), i + 1 == n);
            out.extend(aead::seal_with_prefix(key, &nonce, aad, &[], chunk).unwrap());
        }
        out
    }

    pub(crate) fn open(key: &[u8; 32], aad: &[u8], ct: &[u8]) -> Vec<u8> {
        assert_eq!(&ct[..3], &[0x01, 0x02, 0x10]);
        let prefix: [u8; NONCE_PREFIX_LEN] = ct[3..3 + NONCE_PREFIX_LEN].try_into().unwrap();
        let body = &ct[3 + NONCE_PREFIX_LEN..];
        let chunks: Vec<&[u8]> = body.chunks(CHUNK_LEN + TAG_LEN).collect();
        let n = chunks.len();
        let mut out = Vec::new();
        for (i, chunk) in chunks.into_iter().enumerate() {
            let nonce = nonce(&prefix, u32::try_from(i).unwrap(), i + 1 == n);
            out.extend_from_slice(aead::open(key, &nonce, aad, chunk).unwrap().expose_secret());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::SecretBytes;

    fn ids() -> (VaultId, ItemId, AttachmentId) {
        (
            VaultId::from_bytes([1; 16]).unwrap(),
            ItemId::from_bytes([2; 16]).unwrap(),
            AttachmentId::from_bytes([3; 16]).unwrap(),
        )
    }

    fn fk() -> AttachmentKey {
        AttachmentKey::from_secret(SecretBytes::from_array(&mut [9; 32]))
    }

    #[test]
    fn lengths() {
        assert_eq!(ciphertext_len(0).unwrap(), 22 + 16);
        assert_eq!(ciphertext_len(1).unwrap(), 22 + 1 + 16);
        assert_eq!(ciphertext_len(CHUNK_LEN).unwrap(), 22 + CHUNK_LEN + 16);
        assert_eq!(
            ciphertext_len(CHUNK_LEN + 1).unwrap(),
            22 + CHUNK_LEN + 1 + 32
        );
        assert_eq!(
            ciphertext_len(MAX_ATTACHMENT_LEN).unwrap(),
            22 + MAX_ATTACHMENT_LEN + 800 * 16
        );
        assert_eq!(
            ciphertext_len(MAX_ATTACHMENT_LEN + 1),
            Err(Error::SizeLimit)
        );
    }

    #[test]
    fn boundaries_round_trip() {
        let (v, i, a) = ids();
        let key = fk();
        for len in [0, 1, CHUNK_LEN - 1, CHUNK_LEN, CHUNK_LEN + 1, 3 * CHUNK_LEN] {
            let pt: Vec<u8> = (0..len).map(|x| u8::try_from(x % 251).unwrap()).collect();
            let ct = seal_attachment(&key, &v, &i, &a, &pt).unwrap();
            assert_eq!(ct.len(), ciphertext_len(len).unwrap());
            assert_eq!(&ct[..3], &[0x01, 0x02, 16]);
            let back = open_attachment(&key, &v, &i, &a, &ct).unwrap();
            assert_eq!(back.expose_secret(), pt.as_slice());
        }
    }

    /// The plaintext buffer is reserved at exactly the final length, so
    /// appending the chunks never reallocates (which would leave an
    /// unzeroized copy of decrypted data behind).
    #[test]
    fn open_output_is_never_reallocated() {
        let (v, i, a) = ids();
        let key = fk();
        for len in [0, 1, CHUNK_LEN, CHUNK_LEN + 1] {
            let ct = seal_attachment(&key, &v, &i, &a, &vec![0x11; len]).unwrap();
            let back = open_attachment(&key, &v, &i, &a, &ct).unwrap();
            assert_eq!(back.len(), len);
            assert_eq!(back.capacity(), len, "reallocated for {len} bytes");
        }
    }

    #[test]
    fn truncation_and_extension_are_detected() {
        let (v, i, a) = ids();
        let key = fk();
        let pt = vec![7u8; 2 * CHUNK_LEN + 10];
        let ct = seal_attachment(&key, &v, &i, &a, &pt).unwrap();
        // Drop the last chunk: the new last chunk lacks the last-block flag.
        let truncated = &ct[..PREFIX_LEN + 2 * CHUNK_CT_LEN];
        assert_eq!(
            open_attachment(&key, &v, &i, &a, truncated).unwrap_err(),
            Error::Decryption
        );
        // Swap the first two chunks.
        let mut swapped = ct.clone();
        let (c0, c1) = (PREFIX_LEN, PREFIX_LEN + CHUNK_CT_LEN);
        let first = ct[c0..c1].to_vec();
        swapped.copy_within(c1..c1 + CHUNK_CT_LEN, c0);
        swapped[c1..c1 + CHUNK_CT_LEN].copy_from_slice(&first);
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &swapped).unwrap_err(),
            Error::Decryption
        );
        // Append 16 bytes after a partial last chunk: the length is still a
        // canonical layout (the bytes land in a third chunk that grows from
        // 26 to 42 bytes), so this reaches the crypto and fails the tag.
        let mut extended = ct.clone();
        extended.extend_from_slice(&[0u8; TAG_LEN]);
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &extended),
            Err(Error::Decryption)
        );
        // Cut mid-tag: also a canonical length (a 9-byte last chunk), so the
        // tag check rejects it.
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &ct[..ct.len() - 1]),
            Err(Error::Decryption)
        );
    }

    /// Two seals of the same plaintext under the same FK and IDs differ only
    /// in their random nonce prefix. Chunks of one stream cannot be spliced
    /// into the other: the prefix is part of every chunk's nonce.
    #[test]
    fn cross_stream_splicing_is_detected() {
        let (v, i, a) = ids();
        let key = fk();
        let pt: Vec<u8> = (0..2 * CHUNK_LEN + 10)
            .map(|x| u8::try_from(x % 241).unwrap())
            .collect();
        let ct_a = seal_attachment(&key, &v, &i, &a, &pt).unwrap();
        let ct_b = seal_attachment(&key, &v, &i, &a, &pt).unwrap();
        assert_eq!(ct_a.len(), ct_b.len());
        assert_ne!(
            ct_a[HEADER_LEN..PREFIX_LEN],
            ct_b[HEADER_LEN..PREFIX_LEN],
            "fresh nonce prefixes"
        );
        // The originals open.
        for ct in [&ct_a, &ct_b] {
            let back = open_attachment(&key, &v, &i, &a, ct).unwrap();
            assert_eq!(back.expose_secret(), pt.as_slice());
        }

        let chunk = |n: usize| {
            let start = PREFIX_LEN + n * CHUNK_CT_LEN;
            start..(start + CHUNK_CT_LEN).min(ct_a.len())
        };
        let splice = |range: core::ops::Range<usize>| {
            let mut out = ct_a.clone();
            out[range.clone()].copy_from_slice(&ct_b[range]);
            out
        };
        let cases = [
            ("prefix swap", splice(0..PREFIX_LEN)),
            ("first-chunk swap", splice(chunk(0))),
            ("middle-chunk swap", splice(chunk(1))),
            ("last-chunk swap", splice(chunk(2))),
        ];
        for (name, spliced) in cases {
            assert_ne!(spliced, ct_a, "{name}");
            assert_ne!(spliced, ct_b, "{name}");
            assert_eq!(
                open_attachment(&key, &v, &i, &a, &spliced),
                Err(Error::Decryption),
                "{name}"
            );
        }
    }

    /// A trailing piece shorter than a tag after an exact-multiple file is
    /// not any canonical layout and is rejected before any crypto.
    #[test]
    fn short_trailing_piece_is_malformed() {
        let (v, i, a) = ids();
        let key = fk();
        let ct = seal_attachment(&key, &v, &i, &a, &vec![3u8; 2 * CHUNK_LEN]).unwrap();
        for extra in [1usize, 8, TAG_LEN - 1] {
            let mut odd = ct.clone();
            odd.extend(core::iter::repeat_n(0xA5, extra));
            assert_eq!(
                open_attachment(&key, &v, &i, &a, &odd),
                Err(Error::Malformed),
                "{extra} trailing bytes"
            );
        }
    }

    /// Every chunk equals an independent per-chunk XChaCha20-Poly1305
    /// encryption with nonce `prefix || BE32(i) || last` (§7.3), so the
    /// counter encoding, counter values above 0 and the last-block flag
    /// (off on every chunk but the last) are pinned. The reference output
    /// also opens through the real decoder.
    #[test]
    fn stream_matches_independent_construction() {
        let (v, i, a) = ids();
        let key = fk();
        let prefix: [u8; NONCE_PREFIX_LEN] =
            core::array::from_fn(|n| 0x90 ^ u8::try_from(n).unwrap());
        let aad = reference::aad(v.as_bytes(), i.as_bytes(), a.as_bytes());
        assert_eq!(aad, chunk_aad(&v, &i, &a).as_bytes());
        for len in [
            0,
            1,
            CHUNK_LEN,
            CHUNK_LEN + 1,
            2 * CHUNK_LEN,
            2 * CHUNK_LEN + 10,
        ] {
            let pt: Vec<u8> = (0..len).map(|x| u8::try_from(x % 253).unwrap()).collect();
            let ct = seal_attachment_with_prefix(&key, &v, &i, &a, &pt, &prefix).unwrap();
            let independent = reference::seal(key.expose_secret(), &aad, &prefix, &pt);
            assert!(ct == independent, "STREAM layout differs for {len} bytes");
            assert_eq!(reference::open(key.expose_secret(), &aad, &ct), pt);
            let back = open_attachment(&key, &v, &i, &a, &independent).unwrap();
            assert_eq!(back.expose_secret(), pt.as_slice());
        }
        // Spot-check the nonce layout itself.
        let n = reference::nonce(&prefix, 0x0102_0304, true);
        assert_eq!(&n[..19], &prefix);
        assert_eq!(&n[19..], &[0x01, 0x02, 0x03, 0x04, 0x01]);
    }

    #[test]
    fn exact_multiple_has_no_empty_tail() {
        let (v, i, a) = ids();
        let key = fk();
        let ct = seal_attachment(&key, &v, &i, &a, &vec![1u8; 2 * CHUNK_LEN]).unwrap();
        assert_eq!(ct.len(), PREFIX_LEN + 2 * CHUNK_CT_LEN);
        // A full chunk followed by an empty last chunk is not canonical.
        let mut odd = ct[..PREFIX_LEN + CHUNK_CT_LEN].to_vec();
        odd.extend_from_slice(&[0u8; TAG_LEN]);
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &odd).unwrap_err(),
            Error::Malformed
        );
    }

    #[test]
    fn header_and_limits() {
        let (v, i, a) = ids();
        let key = fk();
        let ct = seal_attachment(&key, &v, &i, &a, b"hi").unwrap();
        let mut bad = ct.clone();
        bad[0] = 2;
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &bad),
            Err(Error::UnsupportedVersion)
        );
        let mut bad = ct.clone();
        bad[1] = 1;
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &bad),
            Err(Error::UnsupportedAlgorithm)
        );
        let mut bad = ct.clone();
        bad[2] = 12;
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &bad),
            Err(Error::UnsupportedAlgorithm)
        );
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &ct[..2]),
            Err(Error::Malformed)
        );
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &ct[..PREFIX_LEN]),
            Err(Error::Malformed)
        );
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &ct[..PREFIX_LEN + 15]),
            Err(Error::Malformed)
        );
        let too_big = vec![0u8; MAX_ATTACHMENT_LEN + 1];
        assert_eq!(
            seal_attachment(&key, &v, &i, &a, &too_big),
            Err(Error::SizeLimit)
        );
        let mut huge = ct.clone();
        huge.resize(ciphertext_len(MAX_ATTACHMENT_LEN).unwrap() + 1, 0);
        assert_eq!(
            open_attachment(&key, &v, &i, &a, &huge),
            Err(Error::SizeLimit)
        );
    }

    /// A ciphertext of exactly the maximum size passes every length check
    /// and reaches the AEAD, where the zero first chunk fails its tag at
    /// once. Cheap, so it runs in every build; the full round trip below
    /// is `#[ignore]`d.
    #[test]
    fn max_size_ciphertext_reaches_the_aead() {
        let (v, i, a) = ids();
        let mut ct = vec![0u8; ciphertext_len(MAX_ATTACHMENT_LEN).unwrap()];
        ct[..HEADER_LEN].copy_from_slice(&HEADER);
        assert_eq!(
            open_attachment(&fk(), &v, &i, &a, &ct),
            Err(Error::Decryption)
        );
    }

    /// Full 50 MiB round trip. Slow in unoptimized builds because the generic
    /// AEAD code is instantiated in this crate; run with
    /// `cargo test --release -- --ignored`.
    #[test]
    #[ignore = "slow in debug builds; run with --release --ignored"]
    fn max_size_round_trips() {
        let (v, i, a) = ids();
        let key = fk();
        let pt = vec![0x5Au8; MAX_ATTACHMENT_LEN];
        let ct = seal_attachment(&key, &v, &i, &a, &pt).unwrap();
        assert_eq!(ct.len(), ciphertext_len(MAX_ATTACHMENT_LEN).unwrap());
        let back = open_attachment(&key, &v, &i, &a, &ct).unwrap();
        assert_eq!(back.len(), MAX_ATTACHMENT_LEN);
        assert_eq!(back.expose_secret(), pt.as_slice());
    }
}
