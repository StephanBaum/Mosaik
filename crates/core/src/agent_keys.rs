//! Device marks and the agent log (CRYPTO_SPEC §11, ADR-0160).
//!
//! ```text
//! K_agent_mark = HKDF(AK, "", ctx("agent-mark"), 32)
//! mark         = HMAC-SHA-256(K_agent_mark, item_id[16] || u32(len(binding_id)) || binding_id || canonical(v))
//! K_agent_log  = HKDF(AK, "", ctx("agent-log-key"), 32)
//! sealed       = 0x01 || 0x01 || nonce[24] || XChaCha20-Poly1305(K_agent_log, nonce = random(24), plaintext,
//!                                    AAD = sealed[0..2] || aad("agent-log", entry_id[16], at_ms:u64))
//! ```
//!
//! A mark confirms, on one device, that the user saw a binding: the keyed
//! HMAC stores nothing about the binding in cleartext. The log key seals one
//! record per agent decision. Both keys are derived on use, held in a
//! zeroizing container and dropped; neither is stored. The canonical form of
//! a binding is produced by the caller (`AgentBindingRecord::canonical_json`);
//! this module only binds bytes to an item and a binding ID.

use core::fmt;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::ZeroizeOnDrop;

use crate::aead;
use crate::encoding::{Aad, label, take};
use crate::error::{Error, Result};
use crate::ids::ItemId;
use crate::kdf::hkdf_ctx;
use crate::rng;
use crate::secret::{AccountKey, SecretBytes, SecretVec};

/// Length of a mark in bytes.
pub const AGENT_MARK_LEN: usize = 32;
/// Length of an agent log entry ID in bytes.
pub const AGENT_LOG_ENTRY_ID_LEN: usize = 16;

/// Version byte of a sealed log record.
const LOG_VERSION: u8 = 0x01;
/// Algorithm byte of a sealed log record: XChaCha20-Poly1305.
const LOG_ALG_XCHACHA20POLY1305: u8 = 0x01;
/// `version || alg`.
const LOG_HEADER: [u8; 2] = [LOG_VERSION, LOG_ALG_XCHACHA20POLY1305];

/// A fresh `random(16)` agent log entry ID. Not secret.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_agent_log_entry_id() -> Result<[u8; AGENT_LOG_ENTRY_ID_LEN]> {
    rng::array()
}

/// `K_agent_mark` (CRYPTO_SPEC §11, ADR-0160). Zeroized on drop, not `Clone`,
/// `Debug` is `[REDACTED]`.
pub struct AgentMarkKey(SecretBytes<32>);

/// `K_agent_log` (CRYPTO_SPEC §11, ADR-0160). Zeroized on drop, not `Clone`,
/// `Debug` is `[REDACTED]`.
pub struct AgentLogKey(SecretBytes<32>);

impl fmt::Debug for AgentMarkKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Debug for AgentLogKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl ZeroizeOnDrop for AgentMarkKey {}
impl ZeroizeOnDrop for AgentLogKey {}

impl AccountKey {
    /// `K_agent_mark = HKDF(AK, "", ctx("agent-mark"), 32)`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn agent_mark_key(&self) -> Result<AgentMarkKey> {
        Ok(AgentMarkKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::AGENT_MARK,
        )?))
    }

    /// `K_agent_log = HKDF(AK, "", ctx("agent-log-key"), 32)`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 32 bytes).
    pub fn agent_log_key(&self) -> Result<AgentLogKey> {
        Ok(AgentLogKey(hkdf_ctx(
            self.0.expose_secret(),
            &[],
            label::AGENT_LOG_KEY,
        )?))
    }
}

impl AgentMarkKey {
    fn hmac(&self, item: &ItemId, binding_id: &str, canonical: &[u8]) -> Result<Hmac<Sha256>> {
        let id_len = u32::try_from(binding_id.len()).map_err(|_| Error::SizeLimit)?;
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(self.0.expose_secret())
            .map_err(|_| Error::SizeLimit)?;
        mac.update(item.as_bytes());
        mac.update(&id_len.to_be_bytes());
        mac.update(binding_id.as_bytes());
        mac.update(canonical);
        Ok(mac)
    }

    /// `HMAC-SHA-256(K_agent_mark, item_id || u32(len(binding_id)) || binding_id || canonical)`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if `binding_id` does not fit a `u32` length or the
    /// MAC cannot be keyed (it cannot for 32 bytes).
    pub fn mark(
        &self,
        item: &ItemId,
        binding_id: &str,
        canonical: &[u8],
    ) -> Result<[u8; AGENT_MARK_LEN]> {
        Ok(self
            .hmac(item, binding_id, canonical)?
            .finalize()
            .into_bytes()
            .into())
    }

    /// Checks `mark` for this item, binding ID and canonical form, in
    /// constant time. A mark of the wrong length is not valid.
    #[must_use]
    pub fn verify(&self, item: &ItemId, binding_id: &str, canonical: &[u8], mark: &[u8]) -> bool {
        self.mark(item, binding_id, canonical)
            .is_ok_and(|expected| bool::from(expected.as_slice().ct_eq(mark)))
    }
}

impl AgentLogKey {
    fn aad(entry_id: &[u8; AGENT_LOG_ENTRY_ID_LEN], at_ms: u64) -> Aad {
        Aad::with_header(&LOG_HEADER, label::AGENT_LOG)
            .fixed(entry_id)
            .fixed(&at_ms.to_be_bytes())
    }

    /// Seals one log record: `0x01 || 0x01 || nonce[24] || ct || tag`, under a
    /// fresh random nonce, bound to `entry_id` and `at_ms`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the OS CSPRNG fails; [`Error::SizeLimit`] or
    /// [`Error::Allocation`] if the record cannot be built.
    pub fn seal(
        &self,
        entry_id: &[u8; AGENT_LOG_ENTRY_ID_LEN],
        at_ms: u64,
        plaintext: &[u8],
    ) -> Result<Vec<u8>> {
        let nonce = aead::random_nonce()?;
        let aad = Self::aad(entry_id, at_ms);
        aead::seal_with_prefix(
            self.0.expose_secret(),
            &nonce,
            aad.as_bytes(),
            &[&LOG_HEADER, &nonce],
            plaintext,
        )
    }

    /// Opens a record sealed under `entry_id` and `at_ms`.
    ///
    /// # Errors
    /// [`Error::Decryption`] for every failure: a short record, an unknown
    /// version or algorithm, another `entry_id` or `at_ms`, a changed bit, the
    /// wrong key.
    pub fn open(
        &self,
        entry_id: &[u8; AGENT_LOG_ENTRY_ID_LEN],
        at_ms: u64,
        sealed: &[u8],
    ) -> Result<SecretVec> {
        let (header, rest) = take::<2>(sealed).map_err(|_| Error::Decryption)?;
        if *header != LOG_HEADER {
            return Err(Error::Decryption);
        }
        let (nonce, ciphertext) =
            take::<{ aead::NONCE_LEN }>(rest).map_err(|_| Error::Decryption)?;
        let aad = Self::aad(entry_id, at_ms);
        aead::open(self.0.expose_secret(), nonce, aad.as_bytes(), ciphertext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn ak(byte: u8) -> AccountKey {
        AccountKey(SecretBytes::from_array(&mut [byte; 32]))
    }

    fn item(start: u8) -> ItemId {
        let mut b = [0u8; 16];
        for (i, v) in b.iter_mut().enumerate() {
            *v = start.wrapping_add(u8::try_from(i).unwrap());
        }
        ItemId::from_bytes(b).unwrap()
    }

    const CANON: &[u8] = br#"{"env":null,"field":"api_key.secret","inject":{"kind":"bearer"},"methods":null,"origins":["https://api.openai.com"],"paths":null,"run":false}"#;

    /// Known-answer vector for the mark with `AK = 01 * 32`, item
    /// `10..1f`, binding ID `b1` and the canonical form `CANON`, from this
    /// script (Python standard library, HKDF per RFC 5869):
    ///
    /// ```python
    /// import hashlib, hmac
    /// def hkdf(ikm, salt, info, n):
    ///     prk = hmac.new(salt if salt else b"\0" * 32, ikm, hashlib.sha256).digest()
    ///     okm, t, i = b"", b"", 1
    ///     while len(okm) < n:
    ///         t = hmac.new(prk, t + info + bytes([i]), hashlib.sha256).digest()
    ///         okm += t; i += 1
    ///     return okm[:n]
    /// k = hkdf(bytes([1] * 32), b"", b"mosaic/v1/agent-mark", 32)
    /// m = hmac.new(k, bytes(range(0x10, 0x20)) + (2).to_bytes(4, "big") + b"b1" + CANON,
    ///              hashlib.sha256).digest()
    /// ```
    #[test]
    fn mark_known_answer() {
        let key = ak(1).agent_mark_key().unwrap();
        assert_eq!(
            key.mark(&item(0x10), "b1", CANON).unwrap(),
            hex!("2d0c50d08e239b9524cd341ebbd74a667d23283508a214ffb87cd53ec38154a2")
        );
    }

    #[test]
    fn mark_depends_on_item_binding_id_and_canonical_bytes() {
        let key = ak(1).agent_mark_key().unwrap();
        let mark = key.mark(&item(0x10), "b1", CANON).unwrap();
        assert!(key.verify(&item(0x10), "b1", CANON, &mark));
        assert!(!key.verify(&item(0x11), "b1", CANON, &mark), "other item");
        assert!(
            !key.verify(&item(0x10), "b2", CANON, &mark),
            "other binding"
        );
        let mut other = CANON.to_vec();
        if let Some(last) = other.last_mut() {
            *last ^= 1;
        }
        assert!(!key.verify(&item(0x10), "b1", &other, &mark), "one byte");
        assert!(!key.verify(&item(0x10), "b1", CANON, &mark[..31]));
        assert!(!key.verify(&item(0x10), "b1", CANON, &[mark.as_slice(), &[0]].concat()));
        assert!(!key.verify(&item(0x10), "b1", CANON, &[]));
        let other_key = ak(2).agent_mark_key().unwrap();
        assert!(!other_key.verify(&item(0x10), "b1", CANON, &mark));
    }

    /// The length prefix makes the input injective: moving a byte from the
    /// binding ID into the canonical form is another mark.
    #[test]
    fn mark_input_is_injective_across_the_binding_id_boundary() {
        let key = ak(1).agent_mark_key().unwrap();
        let a = key.mark(&item(1), "ab", b"c").unwrap();
        let b = key.mark(&item(1), "a", b"bc").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn mark_and_log_keys_differ() {
        let a = ak(1);
        assert_ne!(
            a.agent_mark_key().unwrap().0.expose_secret(),
            a.agent_log_key().unwrap().0.expose_secret()
        );
        assert_eq!(
            a.agent_log_key().unwrap().0.expose_secret(),
            &hex!("f76f2299ca7c9b8843ac12d4ea3edd0624211d4688f12d124d205db133267a8c")
        );
    }

    #[test]
    fn log_record_round_trips_and_is_bound_to_entry_id_and_time() {
        let key = ak(1).agent_log_key().unwrap();
        let id = [7u8; AGENT_LOG_ENTRY_ID_LEN];
        let sealed = key.seal(&id, 1_700_000_000_000, b"record").unwrap();
        assert_eq!(&sealed[..2], &[0x01, 0x01]);
        assert_eq!(
            key.open(&id, 1_700_000_000_000, &sealed)
                .unwrap()
                .expose_secret(),
            b"record"
        );
        let bad = |r: Result<SecretVec>| assert_eq!(r.err(), Some(Error::Decryption));
        bad(key.open(&[8u8; 16], 1_700_000_000_000, &sealed));
        bad(key.open(&id, 1_700_000_000_001, &sealed));
        bad(key.open(&id, 1_700_000_000_000, &[]));
        bad(key.open(&id, 1_700_000_000_000, &sealed[..25]));
        bad(key.open(&id, 1_700_000_000_000, &sealed[..sealed.len() - 1]));
        let mut version = sealed.clone();
        version[0] = 0x02;
        bad(key.open(&id, 1_700_000_000_000, &version));
        let mut alg = sealed.clone();
        alg[1] = 0x02;
        bad(key.open(&id, 1_700_000_000_000, &alg));
        for i in 0..sealed.len() {
            let mut flipped = sealed.clone();
            flipped[i] ^= 0x01;
            bad(key.open(&id, 1_700_000_000_000, &flipped));
        }
        let other = ak(2).agent_log_key().unwrap();
        bad(other.open(&id, 1_700_000_000_000, &sealed));
        let again = key.seal(&id, 1_700_000_000_000, b"record").unwrap();
        assert_ne!(sealed, again, "a fresh random nonce per record");
    }

    #[test]
    fn agent_keys_debug_is_redacted() {
        let a = ak(1);
        assert_eq!(format!("{:?}", a.agent_mark_key().unwrap()), "[REDACTED]");
        assert_eq!(format!("{:?}", a.agent_log_key().unwrap()), "[REDACTED]");
    }

    #[test]
    fn entry_ids_are_random() {
        assert_ne!(
            new_agent_log_entry_id().unwrap(),
            new_agent_log_entry_id().unwrap()
        );
    }
}
