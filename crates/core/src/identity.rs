//! Identity keys, fingerprints and safety codes (CRYPTO_SPEC §8.1, §8.2).
//!
//! Every account has one X25519 identity key pair `(isk, ipk)` for HPKE
//! (DHKEM(X25519, HKDF-SHA256), RFC 9180 §7.1.3). It is derived with
//! `DeriveKeyPair(ikm)` from 32 fresh random bytes, so no RNG adapter is
//! needed. `isk` is wrapped under the account key like any other key
//! ([`WrappedKey`], label `wrap/identity`, bound to the account ID) and lives
//! in memory only while the vault is unlocked. `ipk` is public.
//!
//! The fingerprint `fpr = SHA-256(ctx("fingerprint") || account_id || ipk)`
//! binds a public key to an account. Its first 130 bits, as 26 Crockford
//! Base32 characters, are the safety code two people compare aloud.

use core::fmt;

use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem, Serializable};
use sha2::{Digest, Sha256};
use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::aead;
use crate::encoding::{ctx, label};
use crate::envelope::WrappedKey;
use crate::error::{Error, Result};
use crate::ids::AccountId;
use crate::secret::{AccountKey, SecretBytes};

/// Length of `isk` and `ipk` in bytes.
pub const IDENTITY_KEY_LEN: usize = 32;
/// Length of a safety code in characters (130 bits, 5 bits each).
pub const SAFETY_CODE_LEN: usize = 26;

const REDACTED: &str = "[REDACTED]";
/// RFC 4648-style Crockford Base32 alphabet (no `I`, `L`, `O`, `U`).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Display groups of a safety code: `5-5-5-5-6`.
const GROUPS: [usize; 5] = [5, 5, 5, 5, 6];

/// The identity private key `isk` (§8.1). Zeroizes on drop, is not `Clone`
/// and prints as `[REDACTED]`.
pub struct IdentitySecret(SecretBytes<IDENTITY_KEY_LEN>);

impl fmt::Debug for IdentitySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl ConstantTimeEq for IdentitySecret {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}

impl PartialEq for IdentitySecret {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for IdentitySecret {}

impl ZeroizeOnDrop for IdentitySecret {}

/// The identity public key `ipk` (§8.1): an X25519 public key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct IdentityPublicKey(pub [u8; IDENTITY_KEY_LEN]);

impl IdentityPublicKey {
    /// The 32 key bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; IDENTITY_KEY_LEN] {
        &self.0
    }

    /// Reads a public key from a slice.
    ///
    /// # Errors
    /// [`Error::Malformed`] unless the slice is exactly 32 bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let arr: [u8; IDENTITY_KEY_LEN] = bytes.try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(arr))
    }

    /// The public key of `DeriveKeyPair(ikm)` (RFC 9180 §7.1.3), the same
    /// derivation as [`IdentitySecret::generate`] with the input given. The
    /// hosted server uses it with a keyed hash as `ikm` to make the directory's
    /// fake keys for unknown addresses: they are then valid X25519 keys in the
    /// prime-order subgroup, like every real `ipk` (M6b review M2). The
    /// secret half is dropped (and zeroized) at once.
    ///
    /// # Errors
    /// [`Error::Malformed`] only if the derived key cannot be encoded (cannot
    /// happen).
    pub fn derived_from(ikm: &[u8; 32]) -> Result<Self> {
        Ok(IdentitySecret::derive(ikm)?.1)
    }
}

type PrivateKey = <X25519HkdfSha256 as Kem>::PrivateKey;
type PublicKey = <X25519HkdfSha256 as Kem>::PublicKey;

/// An `isk` as the HPKE crate's private key, for one operation. The crate's
/// type zeroizes on drop.
pub(crate) fn hpke_private(isk: &IdentitySecret) -> Result<PrivateKey> {
    PrivateKey::from_bytes(isk.0.expose_secret()).map_err(|_| Error::Malformed)
}

/// An `ipk` as the HPKE crate's public key.
pub(crate) fn hpke_public(ipk: &IdentityPublicKey) -> Result<PublicKey> {
    PublicKey::from_bytes(&ipk.0).map_err(|_| Error::Malformed)
}

fn public_bytes(pk: &PublicKey) -> Result<IdentityPublicKey> {
    IdentityPublicKey::from_slice(pk.to_bytes().as_slice())
}

impl IdentitySecret {
    /// A new identity key pair: `ikm = random(32)`, then
    /// `(isk, ipk) = DeriveKeyPair(ikm)` (RFC 9180 §7.1.3).
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn generate() -> Result<(Self, IdentityPublicKey)> {
        let ikm = SecretBytes::<32>::random()?;
        Self::derive(ikm.expose_secret())
    }

    /// `DeriveKeyPair(ikm)`; the KAT and `generate` share it.
    pub(crate) fn derive(ikm: &[u8; 32]) -> Result<(Self, IdentityPublicKey)> {
        let (sk, pk) = X25519HkdfSha256::derive_keypair(ikm);
        let mut raw = sk.to_bytes();
        let secret = SecretBytes::<IDENTITY_KEY_LEN>::from_slice(raw.as_slice());
        raw.as_mut_slice().zeroize();
        Ok((Self(secret?), public_bytes(&pk)?))
    }

    /// The matching public key.
    ///
    /// # Errors
    /// [`Error::Malformed`] only if the stored bytes are not a key (cannot
    /// happen for a value made by this crate).
    pub fn public_key(&self) -> Result<IdentityPublicKey> {
        let sk = hpke_private(self)?;
        public_bytes(&X25519HkdfSha256::sk_to_pk(&sk))
    }

    /// Wraps `isk` under `AK` (§8.1):
    /// `aad = header || aad("wrap/identity", account_id)`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn wrap(&self, ak: &AccountKey, account: &AccountId) -> Result<WrappedKey> {
        self.wrap_with_nonce(ak, account, &aead::random_nonce()?)
    }

    pub(crate) fn wrap_with_nonce(
        &self,
        ak: &AccountKey,
        account: &AccountId,
        nonce: &[u8; aead::NONCE_LEN],
    ) -> Result<WrappedKey> {
        WrappedKey::seal(
            ak.0.expose_secret(),
            nonce,
            &WrappedKey::identity_aad(account),
            self.0.expose_secret(),
        )
    }

    /// The raw key bytes for an HPKE operation inside this crate.
    #[cfg(test)]
    pub(crate) fn expose_for_tests(&self) -> &[u8; IDENTITY_KEY_LEN] {
        self.0.expose_secret()
    }
}

impl WrappedKey {
    /// Unwraps an identity private key. `account` comes from context.
    ///
    /// # Errors
    /// [`Error::Decryption`] for a wrong key, wrong account or tampered data.
    pub fn unwrap_identity(&self, ak: &AccountKey, account: &AccountId) -> Result<IdentitySecret> {
        let key = self.open(ak.0.expose_secret(), &Self::identity_aad(account))?;
        Ok(IdentitySecret(key))
    }
}

/// The fingerprint of an identity key (§8.2):
/// `SHA-256(ctx("fingerprint") || account_id || ipk)`. Public data.
/// Equality is constant-time.
#[derive(Clone, Copy)]
pub struct Fingerprint([u8; 32]);

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Fingerprint(")?;
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

impl ConstantTimeEq for Fingerprint {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}

impl PartialEq for Fingerprint {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

impl Eq for Fingerprint {}

impl Fingerprint {
    /// The fingerprint of `ipk` for `account`.
    #[must_use]
    pub fn of(account: &AccountId, ipk: &IdentityPublicKey) -> Self {
        let mut hash = Sha256::new();
        hash.update(ctx(label::FINGERPRINT.as_str()));
        hash.update(account.as_bytes());
        hash.update(ipk.0);
        Self(hash.finalize().into())
    }

    /// Reads a stored fingerprint.
    ///
    /// # Errors
    /// [`Error::Malformed`] unless exactly 32 bytes.
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        let arr: [u8; 32] = b.try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(arr))
    }

    /// The 32 fingerprint bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The first 130 bits as 26 Crockford Base32 characters, most
    /// significant bits first, in groups `5-5-5-5-6` separated by a space:
    /// `"XXXXX XXXXX XXXXX XXXXX XXXXXX"`.
    #[must_use]
    pub fn safety_code(&self) -> String {
        let chars = self.code_chars();
        let mut out = String::with_capacity(SAFETY_CODE_LEN + GROUPS.len() - 1);
        let mut rest = chars.as_slice();
        for (i, n) in GROUPS.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            let (group, tail) = rest.split_at(*n);
            out.extend(group.iter().map(|c| char::from(*c)));
            rest = tail;
        }
        out
    }

    /// The 26 code characters without separators.
    fn code_chars(&self) -> [u8; SAFETY_CODE_LEN] {
        let mut out = [b'0'; SAFETY_CODE_LEN];
        let mut slots = out.iter_mut();
        let mut acc: u32 = 0;
        let mut bits: u32 = 0;
        'bytes: for byte in &self.0 {
            acc = (acc << 8) | u32::from(*byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                let idx = usize::try_from((acc >> bits) & 31).unwrap_or(0);
                let Some(slot) = slots.next() else {
                    break 'bytes;
                };
                *slot = CROCKFORD.get(idx).copied().unwrap_or(b'0');
            }
            acc &= (1 << bits) - 1;
        }
        out
    }

    /// Whether `typed` is this fingerprint's safety code. Case, spaces and
    /// hyphens are ignored, and the Crockford aliases are accepted (`I` and
    /// `L` for `1`, `O` for `0`). The comparison is constant-time.
    #[must_use]
    pub fn matches_code(&self, typed: &str) -> bool {
        let mut normalized = Vec::with_capacity(SAFETY_CODE_LEN);
        for c in typed.chars() {
            if c.is_whitespace() || c == '-' {
                continue;
            }
            let u = c.to_ascii_uppercase();
            let mapped = match u {
                'I' | 'L' => '1',
                'O' => '0',
                other => other,
            };
            match u8::try_from(mapped) {
                Ok(b) if mapped.is_ascii() => normalized.push(b),
                _ => return false,
            }
        }
        let expected = self.code_chars();
        let len_ok = Choice::from(u8::from(normalized.len() == SAFETY_CODE_LEN));
        let mut eq = Choice::from(1);
        for (a, b) in normalized.iter().zip(expected.iter()) {
            eq &= a.ct_eq(b);
        }
        (len_ok & eq).into()
    }
}

#[cfg(test)]
#[expect(clippy::similar_names, reason = "the same key derived twice")]
mod tests {
    #[test]
    fn derived_from_is_the_public_half_of_derive_keypair() {
        // The server's directory fakes use it: the same derivation as a real
        // identity key, so a fake is a valid prime-order-subgroup key.
        let ikm = [7u8; 32];
        let (sk, pk) = IdentitySecret::derive(&ikm).unwrap();
        assert_eq!(IdentityPublicKey::derived_from(&ikm).unwrap(), pk);
        assert_eq!(sk.public_key().unwrap(), pk);
        assert_ne!(IdentityPublicKey::derived_from(&[8u8; 32]).unwrap(), pk);
    }

    use super::*;
    use hex_literal::hex;

    fn account(b: u8) -> AccountId {
        AccountId::from_bytes([b; 16]).unwrap()
    }

    fn ak() -> AccountKey {
        AccountKey::generate().unwrap()
    }

    #[test]
    fn wrap_round_trip() {
        let (isk, ipk) = IdentitySecret::generate().unwrap();
        let (a, k) = (account(1), ak());
        let wrapped = isk.wrap(&k, &a).unwrap();
        assert_eq!(wrapped.as_bytes().len(), WrappedKey::LEN);
        let back = wrapped.unwrap_identity(&k, &a).unwrap();
        assert_eq!(back, isk);
        assert_eq!(back.public_key().unwrap(), ipk);
        assert_eq!(format!("{isk:?}"), "[REDACTED]");
    }

    #[test]
    fn wrong_account_fails() {
        let (isk, _) = IdentitySecret::generate().unwrap();
        let (a, k) = (account(1), ak());
        let wrapped = isk.wrap(&k, &a).unwrap();
        assert_eq!(
            wrapped.unwrap_identity(&k, &account(2)).unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            wrapped.unwrap_identity(&ak(), &a).unwrap_err(),
            Error::Decryption
        );
        // A vault-key wrapping is not an identity wrapping.
        let vk = crate::VaultKey::generate(crate::KeyGeneration::FIRST).unwrap();
        let vault = crate::VaultId::from_bytes([3; 16]).unwrap();
        let w = vk.wrap(&k, &a, &vault).unwrap();
        assert_eq!(w.unwrap_identity(&k, &a).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn public_key_derivation_matches_hpke() {
        // RFC 9180 A.2.3 (X25519 / ChaCha20Poly1305, Auth): DeriveKeyPair of
        // `ikmS` and `ikmR` gives the published public keys.
        let ikm_s = hex!("9d8f94537d5a3ddef71234c0baedfad4ca6861634d0b94c3007fed557ad17df6");
        let pk_s = hex!("f0f4f9e96c54aeed3f323de8534fffd7e0577e4ce269896716bcb95643c8712b");
        let ikm_r = hex!("64835d5ee64aa7aad57c6f2e4f758f7696617f8829e70bc9ac7a5ef95d1c756c");
        let pk_r = hex!("1a478716d63cb2e16786ee93004486dc151e988b34b475043d3e0175bdb01c44");
        let (isk, ipk) = IdentitySecret::derive(&ikm_s).unwrap();
        assert_eq!(ipk.0, pk_s);
        assert_eq!(isk.public_key().unwrap(), ipk);
        let (_, ipk_r) = IdentitySecret::derive(&ikm_r).unwrap();
        assert_eq!(ipk_r.0, pk_r);
        let (isk2, ipk2) = IdentitySecret::derive(&ikm_s).unwrap();
        assert_eq!(isk, isk2, "derivation is deterministic");
        assert_eq!(ipk, ipk2);
        assert_ne!(
            isk.expose_for_tests(),
            &ikm_s,
            "isk is derived, not the ikm"
        );
    }

    #[test]
    fn groups_5_5_5_5_6() {
        let fpr = Fingerprint::of(&account(1), &IdentityPublicKey([7; 32]));
        let code = fpr.safety_code();
        let parts: Vec<&str> = code.split(' ').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            [5, 5, 5, 5, 6]
        );
        assert!(code.bytes().all(|b| b == b' ' || CROCKFORD.contains(&b)));
    }

    #[test]
    fn safety_code_known_answer() {
        // fpr = 0x00..0x1f: the first 130 bits of 00 01 02 03 04 ... read as
        // 5-bit groups, most significant first (computed independently).
        let mut b = [0u8; 32];
        for (i, v) in b.iter_mut().enumerate() {
            *v = u8::try_from(i).unwrap();
        }
        let fpr = Fingerprint::from_bytes(&b).unwrap();
        assert_eq!(fpr.safety_code(), "000G4 0R40M 30E20 9185G R38E1W");
        // All-ones: 130 bits of 1, 26 x 'Z'.
        let ones = Fingerprint::from_bytes(&[0xFF; 32]).unwrap();
        assert_eq!(ones.safety_code(), "ZZZZZ ZZZZZ ZZZZZ ZZZZZ ZZZZZZ");
    }

    #[test]
    fn matches_code_aliases_and_case() {
        let fpr = Fingerprint::of(&account(1), &IdentityPublicKey([9; 32]));
        let code = fpr.safety_code();
        assert!(fpr.matches_code(&code));
        assert!(fpr.matches_code(&code.to_lowercase()));
        assert!(fpr.matches_code(&code.replace(' ', "-")));
        assert!(fpr.matches_code(&code.replace(' ', "")));
        assert!(!fpr.matches_code(""));
        assert!(!fpr.matches_code(&code[..code.len() - 1]));
        assert!(!fpr.matches_code(&format!("{code}0")));
        let mut other = code.clone().into_bytes();
        let last = other.len() - 1;
        other[last] = if other[last] == b'0' { b'1' } else { b'0' };
        assert!(!fpr.matches_code(core::str::from_utf8(&other).unwrap()));
        assert!(!fpr.matches_code("é"));
        // Aliases: I, L and O stand for 1, 1 and 0.
        let ones = Fingerprint::from_bytes(&{
            let mut b = [0u8; 32];
            b[0] = 0b0000_1000; // 00001 000.. -> "1" then zeros
            b
        })
        .unwrap();
        assert_eq!(ones.safety_code(), "10000 00000 00000 00000 000000");
        assert!(ones.matches_code("iOOOO OOOOO OOOOO OOOOO OOOOOO"));
        assert!(ones.matches_code("LoooO 00000 00000 00000 000000"));
    }

    #[test]
    fn fingerprint_binds_account_and_key() {
        let ipk = IdentityPublicKey([5; 32]);
        let f = Fingerprint::of(&account(1), &ipk);
        assert_ne!(f, Fingerprint::of(&account(2), &ipk));
        assert_ne!(f, Fingerprint::of(&account(1), &IdentityPublicKey([6; 32])));
        assert_eq!(Fingerprint::from_bytes(f.as_bytes()).unwrap(), f);
        assert_eq!(
            Fingerprint::from_bytes(&[0; 31]).unwrap_err(),
            Error::Malformed
        );
        assert!(format!("{f:?}").starts_with("Fingerprint("));
    }
}
