//! XChaCha20-Poly1305 (draft-irtf-cfrg-xchacha, RFC 8439) as used by every
//! envelope in this crate.
//!
//! This module is crate-private on purpose: the public API never accepts a
//! caller-supplied nonce. Envelopes draw a fresh `random(24)` nonce for every
//! seal; the deterministic entry points exist only for KATs and golden tests.

use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::{Tag, XChaCha20Poly1305, XNonce};
use zeroize::Zeroize;

use crate::error::{Error, Result};
use crate::rng;
use crate::secret::SecretVec;

/// XChaCha20-Poly1305 nonce length.
pub(crate) const NONCE_LEN: usize = 24;
/// Poly1305 tag length.
pub(crate) const TAG_LEN: usize = 16;

/// A fresh random nonce. Never derived, never reused (§7.2).
pub(crate) fn random_nonce() -> Result<[u8; NONCE_LEN]> {
    rng::array()
}

/// Encrypts `out[start..]` in place under `key`/`nonce`/`aad` and appends the
/// tag. On failure the plaintext in `out` is zeroized.
pub(crate) fn encrypt_tail(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    out: &mut Vec<u8>,
    start: usize,
) -> Result<()> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::from(*nonce);
    let tag = match out.get_mut(start..) {
        Some(body) => cipher.encrypt_inout_detached(&nonce, aad, body.into()),
        None => Err(chacha20poly1305::Error),
    };
    if let Ok(tag) = tag {
        out.extend_from_slice(&tag);
        Ok(())
    } else {
        out.zeroize();
        Err(Error::SizeLimit)
    }
}

/// `prefix || AEAD(key, nonce, plaintext, aad)` where `prefix` is typically
/// `header || nonce`. The output buffer is reserved fallibly up front.
pub(crate) fn seal_with_prefix(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    prefix: &[&[u8]],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let prefix_len = prefix
        .iter()
        .try_fold(0usize, |acc, p| acc.checked_add(p.len()))
        .ok_or(Error::SizeLimit)?;
    let cap = prefix_len
        .checked_add(plaintext.len())
        .and_then(|n| n.checked_add(TAG_LEN))
        .ok_or(Error::SizeLimit)?;
    let mut out = Vec::new();
    out.try_reserve_exact(cap).map_err(|_| Error::Allocation)?;
    for p in prefix {
        out.extend_from_slice(p);
    }
    out.extend_from_slice(plaintext);
    encrypt_tail(key, nonce, aad, &mut out, prefix_len)?;
    Ok(out)
}

/// Verifies and decrypts `ciphertext || tag` into a zeroizing buffer.
///
/// # Errors
/// [`Error::Decryption`] for any failure, including a too-short input.
pub(crate) fn open(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    ciphertext_and_tag: &[u8],
) -> Result<SecretVec> {
    let ct_len = ciphertext_and_tag
        .len()
        .checked_sub(TAG_LEN)
        .ok_or(Error::Decryption)?;
    let (ct, tag) = ciphertext_and_tag.split_at(ct_len);
    let tag = Tag::try_from(tag).map_err(|_| Error::Decryption)?;
    let mut buf = SecretVec::from_slice(ct)?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::from(*nonce);
    cipher
        .decrypt_inout_detached(&nonce, aad, buf.as_mut_vec().as_mut_slice().into(), &tag)
        .map_err(|_| Error::Decryption)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    /// draft-irtf-cfrg-xchacha-03, Appendix A.3.1 (`AEAD_XChaCha20_Poly1305`).
    /// Bytes cross-checked against the `chacha20poly1305` crate's
    /// `tests/lib.rs`.
    #[test]
    fn kat_xchacha20poly1305_draft_a31() {
        let key = hex!("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f");
        let nonce = hex!("404142434445464748494a4b4c4d4e4f5051525354555657");
        let aad = hex!("50515253c0c1c2c3c4c5c6c7");
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let expected_ct = hex!(
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb"
            "731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452"
            "2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9"
            "21f9664c97637da9768812f615c68b13b52e"
        );
        let expected_tag = hex!("c0875924c1c7987947deafd8780acf49");

        let sealed = seal_with_prefix(&key, &nonce, &aad, &[], plaintext).unwrap();
        let (ct, tag) = sealed.split_at(sealed.len() - TAG_LEN);
        assert_eq!(ct, expected_ct);
        assert_eq!(tag, expected_tag);

        let opened = open(&key, &nonce, &aad, &sealed).unwrap();
        assert_eq!(opened.expose_secret(), plaintext);

        let mut bad = sealed.clone();
        bad[0] ^= 1;
        assert_eq!(
            open(&key, &nonce, &aad, &bad).unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            open(&key, &nonce, b"", &sealed).unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            open(&key, &nonce, &aad, &sealed[..15]).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn prefix_is_kept_and_not_encrypted() {
        let key = [1u8; 32];
        let nonce = [2u8; NONCE_LEN];
        let out = seal_with_prefix(&key, &nonce, b"ad", &[b"hd", &nonce], b"msg").unwrap();
        assert_eq!(&out[..2], b"hd");
        assert_eq!(&out[2..26], &nonce);
        assert_eq!(out.len(), 2 + 24 + 3 + 16);
        let pt = open(&key, &nonce, b"ad", &out[26..]).unwrap();
        assert_eq!(pt.expose_secret(), b"msg");
    }

    #[test]
    fn encrypt_tail_out_of_range_fails_and_wipes() {
        let mut buf = b"secret".to_vec();
        assert_eq!(
            encrypt_tail(&[0; 32], &[0; NONCE_LEN], b"", &mut buf, 99),
            Err(Error::SizeLimit)
        );
        assert_eq!(buf, Vec::<u8>::new());
    }

    /// The decryption buffer is reserved at exactly the plaintext length and
    /// decrypted in place, so it is never reallocated.
    #[test]
    fn open_buffer_is_never_reallocated() {
        let key = [5u8; 32];
        let nonce = [6u8; NONCE_LEN];
        for len in [0usize, 1, 16, 1000] {
            let sealed = seal_with_prefix(&key, &nonce, b"", &[], &vec![7u8; len]).unwrap();
            assert_eq!(sealed.capacity(), len + TAG_LEN);
            let pt = open(&key, &nonce, b"", &sealed).unwrap();
            assert_eq!(pt.len(), len);
            assert_eq!(pt.capacity(), len, "reallocated for {len} bytes");
        }
    }

    #[test]
    fn nonces_are_fresh() {
        assert_ne!(random_nonce().unwrap(), random_nonce().unwrap());
    }
}
