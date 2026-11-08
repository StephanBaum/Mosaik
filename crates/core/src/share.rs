//! Vault-key shares and emergency seals (CRYPTO_SPEC §8.2, §8.4).
//!
//! Both are one HPKE seal, RFC 9180 mode Auth, with the suite fixed by
//! CRYPTO_SPEC §2: DHKEM(X25519, HKDF-SHA256) (`0x0020`), HKDF-SHA256
//! (`0x0001`), ChaCha20-Poly1305 (`0x0003`). The sender authenticates with
//! its identity key `isk`; the recipient opens with its own `isk` and the
//! sender's `ipk`.
//!
//! ```text
//! ShareSeal (82 bytes) = version:u8 (1) || alg:u8 (0x03) || enc[32] || ct[48]
//! ct = HPKE.SealAuth(pkR = ipk_recipient, info, aad = header || aad(...), pt = VK[32], skS = isk_sender)
//!
//! share:     info = ctx("share/vault") || vault_id || sender || recipient
//!            aad  = header || aad("share/vault", vault_id, key_generation:u32)
//! emergency: info = ctx("emergency") || grantor || grantee
//!            aad  = header || aad("emergency", 0x02, vault_id, key_generation:u32)
//! ```
//!
//! `0x02` in the emergency AAD is the content type "vault key"; `0x01`
//! (account key) is reserved and no decoder in this crate accepts it
//! (ADR-0048). The ephemeral key costs one 32-byte draw from the OS CSPRNG
//! per seal ([`crate::rng::HpkeRng`]). Every failure to open is the single
//! [`Error::Decryption`].

use hpke::aead::ChaCha20Poly1305;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem, OpModeR, OpModeS, Serializable};

use crate::encoding::{Aad, ctx, label};
use crate::error::{Error, Result};
use crate::identity::{IdentityPublicKey, IdentitySecret, hpke_private, hpke_public};
use crate::ids::{AccountId, KeyGeneration, VaultId};
use crate::rng::HpkeRng;
use crate::secret::{SecretBytes, SecretVec, VaultKey};

/// Format version of a seal.
pub const SEAL_VERSION: u8 = 0x01;
/// Algorithm ID: HPKE mode Auth, KEM `0x0020`, KDF `0x0001`, AEAD `0x0003`.
pub const SEAL_ALG_HPKE_AUTH: u8 = 0x03;
/// Content type of an emergency seal that holds a vault key (§8.4).
const CONTENT_VAULT_KEY: u8 = 0x02;

const HEADER: [u8; 2] = [SEAL_VERSION, SEAL_ALG_HPKE_AUTH];
const ENC_LEN: usize = 32;
const CT_LEN: usize = 32 + 16;

/// A vault key sealed to one account's identity key (§8.2, §8.4). The
/// ciphertext is public; only the recipient's `isk` opens it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShareSeal([u8; ShareSeal::LEN]);

impl ShareSeal {
    /// Encoded length: 2 + 32 + 48.
    pub const LEN: usize = 2 + ENC_LEN + CT_LEN;

    /// Strictly decodes a seal.
    ///
    /// # Errors
    /// [`Error::UnsupportedVersion`] unless the version is 1,
    /// [`Error::UnsupportedAlgorithm`] unless the algorithm is `0x03`,
    /// [`Error::Malformed`] for any other length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ([version, alg], _) = crate::encoding::take::<2>(bytes)?;
        if *version != SEAL_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if *alg != SEAL_ALG_HPKE_AUTH {
            return Err(Error::UnsupportedAlgorithm);
        }
        let arr: [u8; Self::LEN] = bytes.try_into().map_err(|_| Error::Malformed)?;
        Ok(Self(arr))
    }

    /// The encoded bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }
}

/// Which of the two constructions a seal belongs to.
#[derive(Clone, Copy)]
enum Kind {
    Share,
    Emergency,
}

impl Kind {
    fn info(self, vault: &VaultId, from: &AccountId, to: &AccountId) -> Vec<u8> {
        match self {
            Self::Share => {
                let mut out = ctx(label::SHARE_VAULT.as_str());
                out.extend_from_slice(vault.as_bytes());
                out.extend_from_slice(from.as_bytes());
                out.extend_from_slice(to.as_bytes());
                out
            }
            Self::Emergency => {
                let mut out = ctx(label::EMERGENCY.as_str());
                out.extend_from_slice(from.as_bytes());
                out.extend_from_slice(to.as_bytes());
                out
            }
        }
    }

    fn aad(self, vault: &VaultId, generation: KeyGeneration) -> Aad {
        match self {
            Self::Share => Aad::with_header(&HEADER, label::SHARE_VAULT)
                .fixed(vault.as_bytes())
                .u32(generation.get()),
            Self::Emergency => Aad::with_header(&HEADER, label::EMERGENCY)
                .u8(CONTENT_VAULT_KEY)
                .fixed(vault.as_bytes())
                .u32(generation.get()),
        }
    }
}

/// `SealAuth(pkR, info, aad, pt, skS)` with the Mosaic suite. Returns
/// `(enc, ct)`.
fn hpke_seal_raw(
    sender: &IdentitySecret,
    recipient: &IdentityPublicKey,
    info: &[u8],
    aad: &[u8],
    plaintext: &[u8],
    rng: &mut HpkeRng,
) -> Result<([u8; ENC_LEN], Vec<u8>)> {
    let sk_s = hpke_private(sender)?;
    let pk_s = X25519HkdfSha256::sk_to_pk(&sk_s);
    let pk_r = hpke_public(recipient)?;
    let (enc, ct) =
        hpke::single_shot_seal_with_rng::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
            &OpModeS::Auth((sk_s, pk_s)),
            &pk_r,
            info,
            plaintext,
            aad,
            rng,
        )
        .map_err(|_| Error::Malformed)?;
    // A short ephemeral draw yields zero bytes, never a weak key.
    rng.check()?;
    let enc: [u8; ENC_LEN] = enc
        .to_bytes()
        .as_slice()
        .try_into()
        .map_err(|_| Error::Malformed)?;
    Ok((enc, ct))
}

/// `OpenAuth(enc, skR, info, aad, ct, pkS)` with the Mosaic suite.
fn hpke_open_raw(
    recipient: &IdentitySecret,
    sender: &IdentityPublicKey,
    enc: &[u8],
    info: &[u8],
    aad: &[u8],
    ct: &[u8],
) -> Result<SecretVec> {
    let sk_r = hpke_private(recipient).map_err(|_| Error::Decryption)?;
    let pk_s = hpke_public(sender).map_err(|_| Error::Decryption)?;
    let enc =
        <X25519HkdfSha256 as Kem>::EncappedKey::from_bytes(enc).map_err(|_| Error::Decryption)?;
    let pt = hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, X25519HkdfSha256>(
        &OpModeR::Auth(pk_s),
        &sk_r,
        &enc,
        info,
        ct,
        aad,
    )
    .map_err(|_| Error::Decryption)?;
    Ok(SecretVec::from(pt))
}

#[expect(
    clippy::too_many_arguments,
    reason = "the context fields of one seal; a struct would only rename them"
)]
fn seal_inner(
    kind: Kind,
    vk: &VaultKey,
    vault: &VaultId,
    sender: &AccountId,
    sender_isk: &IdentitySecret,
    recipient: &AccountId,
    recipient_ipk: &IdentityPublicKey,
    rng: &mut HpkeRng,
) -> Result<ShareSeal> {
    let info = kind.info(vault, sender, recipient);
    let aad = kind.aad(vault, vk.generation());
    let (enc, ct) = hpke_seal_raw(
        sender_isk,
        recipient_ipk,
        &info,
        aad.as_bytes(),
        vk.key.expose_secret(),
        rng,
    )?;
    let ct: [u8; CT_LEN] = ct.as_slice().try_into().map_err(|_| Error::Malformed)?;
    let mut out = [0u8; ShareSeal::LEN];
    let mut cursor = out.iter_mut();
    for b in HEADER.iter().chain(enc.iter()).chain(ct.iter()) {
        *cursor.next().ok_or(Error::Malformed)? = *b;
    }
    Ok(ShareSeal(out))
}

#[expect(
    clippy::too_many_arguments,
    reason = "the context fields of one seal; a struct would only rename them"
)]
fn open_inner(
    kind: Kind,
    seal: &ShareSeal,
    vault: &VaultId,
    generation: KeyGeneration,
    sender: &AccountId,
    sender_ipk: &IdentityPublicKey,
    recipient: &AccountId,
    recipient_isk: &IdentitySecret,
) -> Result<VaultKey> {
    let (_, rest) = crate::encoding::take::<2>(&seal.0).map_err(|_| Error::Decryption)?;
    let (enc, ct) = crate::encoding::take::<ENC_LEN>(rest).map_err(|_| Error::Decryption)?;
    let info = kind.info(vault, sender, recipient);
    let aad = kind.aad(vault, generation);
    let pt = hpke_open_raw(recipient_isk, sender_ipk, enc, &info, aad.as_bytes(), ct)?;
    let key = SecretBytes::<32>::from_slice(pt.expose_secret()).map_err(|_| Error::Decryption)?;
    Ok(VaultKey { key, generation })
}

/// Seals `vk` to `recipient_ipk` as a vault share (§8.2). The seal is bound
/// to the vault, both accounts and `vk`'s generation.
///
/// # Errors
/// [`Error::Randomness`] if the CSPRNG fails; [`Error::Malformed`] if the
/// recipient key is not a usable X25519 public key.
pub fn seal_vault_key(
    vk: &VaultKey,
    vault: &VaultId,
    sender: &AccountId,
    sender_isk: &IdentitySecret,
    recipient: &AccountId,
    recipient_ipk: &IdentityPublicKey,
) -> Result<ShareSeal> {
    let mut rng = HpkeRng::from_os()?;
    seal_inner(
        Kind::Share,
        vk,
        vault,
        sender,
        sender_isk,
        recipient,
        recipient_ipk,
        &mut rng,
    )
}

/// Opens a vault share. All context comes from the caller: the seal moved
/// to another vault, generation, sender or recipient does not open.
///
/// # Errors
/// [`Error::Decryption`] for every failure.
pub fn open_vault_key(
    seal: &ShareSeal,
    vault: &VaultId,
    generation: KeyGeneration,
    sender: &AccountId,
    sender_ipk: &IdentityPublicKey,
    recipient: &AccountId,
    recipient_isk: &IdentitySecret,
) -> Result<VaultKey> {
    open_inner(
        Kind::Share,
        seal,
        vault,
        generation,
        sender,
        sender_ipk,
        recipient,
        recipient_isk,
    )
}

/// Seals `vk` to a grantee as an emergency seal (§8.4, content type `0x02`).
///
/// # Errors
/// As [`seal_vault_key`].
pub fn seal_emergency_vault_key(
    vk: &VaultKey,
    vault: &VaultId,
    grantor: &AccountId,
    grantor_isk: &IdentitySecret,
    grantee: &AccountId,
    grantee_ipk: &IdentityPublicKey,
) -> Result<ShareSeal> {
    let mut rng = HpkeRng::from_os()?;
    seal_inner(
        Kind::Emergency,
        vk,
        vault,
        grantor,
        grantor_isk,
        grantee,
        grantee_ipk,
        &mut rng,
    )
}

/// Opens an emergency seal.
///
/// # Errors
/// [`Error::Decryption`] for every failure.
pub fn open_emergency_vault_key(
    seal: &ShareSeal,
    vault: &VaultId,
    generation: KeyGeneration,
    grantor: &AccountId,
    grantor_ipk: &IdentityPublicKey,
    grantee: &AccountId,
    grantee_isk: &IdentitySecret,
) -> Result<VaultKey> {
    open_inner(
        Kind::Emergency,
        seal,
        vault,
        generation,
        grantor,
        grantor_ipk,
        grantee,
        grantee_isk,
    )
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A share seal with an injected ephemeral `ikm` (for the KATs).
    pub(crate) fn seal_vault_key_with_ephemeral(
        vk: &VaultKey,
        vault: &VaultId,
        sender: &AccountId,
        sender_isk: &IdentitySecret,
        recipient: &AccountId,
        recipient_ipk: &IdentityPublicKey,
        ephemeral_ikm: [u8; 32],
    ) -> Result<ShareSeal> {
        seal_inner(
            Kind::Share,
            vk,
            vault,
            sender,
            sender_isk,
            recipient,
            recipient_ipk,
            &mut HpkeRng::fixed(ephemeral_ikm),
        )
    }

    /// An emergency seal with an injected ephemeral `ikm` (for the KATs).
    pub(crate) fn seal_emergency_with_ephemeral(
        vk: &VaultKey,
        vault: &VaultId,
        grantor: &AccountId,
        grantor_isk: &IdentitySecret,
        grantee: &AccountId,
        grantee_ipk: &IdentityPublicKey,
        ephemeral_ikm: [u8; 32],
    ) -> Result<ShareSeal> {
        seal_inner(
            Kind::Emergency,
            vk,
            vault,
            grantor,
            grantor_isk,
            grantee,
            grantee_ipk,
            &mut HpkeRng::fixed(ephemeral_ikm),
        )
    }

    /// One value of `crates/core/tests/vectors/share_kat.txt`: hex-decoded,
    /// or the raw text for the `*_safety_code` lines.
    pub(crate) fn vector(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("vectors")
            .join("share_kat.txt");
        let text = std::fs::read_to_string(path).unwrap();
        let value = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .find_map(|l| l.strip_prefix(name).and_then(|r| r.strip_prefix('=')))
            .unwrap();
        if name.ends_with("safety_code") {
            return value.as_bytes().to_vec();
        }
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }

    pub(crate) fn raw_seal(
        sender: &IdentitySecret,
        recipient: &IdentityPublicKey,
        info: &[u8],
        aad: &[u8],
        plaintext: &[u8],
        ephemeral_ikm: [u8; 32],
    ) -> Result<([u8; 32], Vec<u8>)> {
        hpke_seal_raw(
            sender,
            recipient,
            info,
            aad,
            plaintext,
            &mut HpkeRng::fixed(ephemeral_ikm),
        )
    }

    pub(crate) fn raw_open(
        recipient: &IdentitySecret,
        sender: &IdentityPublicKey,
        enc: &[u8],
        info: &[u8],
        aad: &[u8],
        ct: &[u8],
    ) -> Result<SecretVec> {
        hpke_open_raw(recipient, sender, enc, info, aad, ct)
    }
}

#[cfg(test)]
#[expect(
    clippy::many_single_char_names,
    reason = "parties a, b, c and a vault v in short tests"
)]
mod tests {
    use super::*;

    struct Party {
        account: AccountId,
        isk: IdentitySecret,
        ipk: IdentityPublicKey,
    }

    fn party(b: u8) -> Party {
        let (isk, ipk) = IdentitySecret::generate().unwrap();
        Party {
            account: AccountId::from_bytes([b; 16]).unwrap(),
            isk,
            ipk,
        }
    }

    fn vault(b: u8) -> VaultId {
        VaultId::from_bytes([b; 16]).unwrap()
    }

    fn kg(n: u32) -> KeyGeneration {
        KeyGeneration::new(n).unwrap()
    }

    fn sealed(a: &Party, b: &Party, v: &VaultId, vk: &VaultKey) -> ShareSeal {
        seal_vault_key(vk, v, &a.account, &a.isk, &b.account, &b.ipk).unwrap()
    }

    fn open(s: &ShareSeal, a: &Party, b: &Party, v: &VaultId, g: u32) -> Result<VaultKey> {
        open_vault_key(s, v, kg(g), &a.account, &a.ipk, &b.account, &b.isk)
    }

    #[test]
    fn seal_open_round_trip() {
        let (a, b, v) = (party(1), party(2), vault(9));
        let vk = VaultKey::generate(kg(3)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        assert_eq!(s.as_bytes().len(), 82);
        assert_eq!(&s.as_bytes()[..2], &[1, 3]);
        let back = open(&s, &a, &b, &v, 3).unwrap();
        assert_eq!(back, vk);
        assert_eq!(back.generation(), kg(3));
        // Two seals of one key differ (fresh ephemeral key).
        assert_ne!(sealed(&a, &b, &v, &vk), s);
        assert_eq!(ShareSeal::from_bytes(s.as_bytes()).unwrap(), s);
    }

    #[test]
    fn wrong_recipient_fails() {
        let (a, b, c, v) = (party(1), party(2), party(3), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        // Another recipient key, and the right key under another account ID.
        assert_eq!(open(&s, &a, &c, &v, 1).unwrap_err(), Error::Decryption);
        let ak = crate::AccountKey::generate().unwrap();
        let same_key = b
            .isk
            .wrap(&ak, &b.account)
            .unwrap()
            .unwrap_identity(&ak, &b.account)
            .unwrap();
        assert!(open(&s, &a, &b, &v, 1).is_ok());
        let b2 = Party {
            account: c.account,
            isk: same_key,
            ipk: b.ipk,
        };
        assert_eq!(open(&s, &a, &b2, &v, 1).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn wrong_sender_ipk_fails() {
        let (a, b, c, v) = (party(1), party(2), party(3), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        // A different sender key (the "Auth" in HPKE Auth), and a different
        // sender account ID with the right key.
        assert_eq!(open(&s, &c, &b, &v, 1).unwrap_err(), Error::Decryption);
        let a2 = Party {
            account: c.account,
            isk: IdentitySecret::generate().unwrap().0,
            ipk: a.ipk,
        };
        assert_eq!(open(&s, &a2, &b, &v, 1).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn wrong_vault_fails() {
        let (a, b) = (party(1), party(2));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let s = sealed(&a, &b, &vault(9), &vk);
        assert_eq!(
            open(&s, &a, &b, &vault(8), 1).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn wrong_generation_fails() {
        let (a, b, v) = (party(1), party(2), vault(9));
        let vk = VaultKey::generate(kg(2)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        assert_eq!(open(&s, &a, &b, &v, 1).unwrap_err(), Error::Decryption);
        assert_eq!(open(&s, &a, &b, &v, 3).unwrap_err(), Error::Decryption);
        assert!(open(&s, &a, &b, &v, 2).is_ok());
    }

    #[test]
    fn emergency_and_share_not_interchangeable() {
        let (a, b, v) = (party(1), party(2), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let share = sealed(&a, &b, &v, &vk);
        let emergency =
            seal_emergency_vault_key(&vk, &v, &a.account, &a.isk, &b.account, &b.ipk).unwrap();
        // Each opens only through its own function.
        let opened = open_emergency_vault_key(
            &emergency,
            &v,
            kg(1),
            &a.account,
            &a.ipk,
            &b.account,
            &b.isk,
        )
        .unwrap();
        assert_eq!(opened, vk);
        assert_eq!(
            open_emergency_vault_key(&share, &v, kg(1), &a.account, &a.ipk, &b.account, &b.isk)
                .unwrap_err(),
            Error::Decryption
        );
        assert_eq!(
            open_vault_key(
                &emergency,
                &v,
                kg(1),
                &a.account,
                &a.ipk,
                &b.account,
                &b.isk
            )
            .unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn tampered_header_fails() {
        let (a, b, v) = (party(1), party(2), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        // Flip one bit in every byte of enc and ct: none opens.
        for i in 2..ShareSeal::LEN {
            let mut bytes = *s.as_bytes();
            bytes[i] ^= 0x01;
            let t = ShareSeal::from_bytes(&bytes).unwrap();
            assert_eq!(
                open(&t, &a, &b, &v, 1).unwrap_err(),
                Error::Decryption,
                "byte {i}"
            );
        }
    }

    #[test]
    fn bad_length_and_alg_rejected() {
        let (a, b, v) = (party(1), party(2), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let s = sealed(&a, &b, &v, &vk);
        let good = s.as_bytes().to_vec();
        for len in [0, 1, 2, 81, 83, 164] {
            let mut bytes = good.clone();
            bytes.resize(len, 0);
            assert!(ShareSeal::from_bytes(&bytes).is_err(), "length {len}");
        }
        let mut bad_version = good.clone();
        bad_version[0] = 2;
        assert_eq!(
            ShareSeal::from_bytes(&bad_version).unwrap_err(),
            Error::UnsupportedVersion
        );
        for alg in [0u8, 1, 2, 4, 0xFF] {
            let mut bad_alg = good.clone();
            bad_alg[1] = alg;
            assert_eq!(
                ShareSeal::from_bytes(&bad_alg).unwrap_err(),
                Error::UnsupportedAlgorithm
            );
        }
    }

    #[test]
    fn low_order_recipient_key_is_refused() {
        let (a, v) = (party(1), vault(9));
        let vk = VaultKey::generate(kg(1)).unwrap();
        let recipient = AccountId::from_bytes([2; 16]).unwrap();
        let zero = IdentityPublicKey([0; 32]);
        assert!(seal_vault_key(&vk, &v, &a.account, &a.isk, &recipient, &zero).is_err());
    }
}

#[cfg(test)]
#[expect(clippy::similar_names, reason = "sender and recipient key pairs")]
mod kat {
    use hex_literal::hex;

    use super::test_support::{
        raw_open, raw_seal, seal_emergency_with_ephemeral, seal_vault_key_with_ephemeral, vector,
    };
    use super::*;
    use crate::envelope::{ContactsCiphertext, VaultRosterCiphertext};
    use crate::identity::Fingerprint;
    use crate::secret::AccountKey;

    fn ids() -> (AccountId, AccountId, VaultId, KeyGeneration) {
        (
            AccountId::from_bytes([0xA1; 16]).unwrap(),
            AccountId::from_bytes([0xB2; 16]).unwrap(),
            VaultId::from_bytes([0x0B; 16]).unwrap(),
            KeyGeneration::new(3).unwrap(),
        )
    }

    fn arr32(name: &str) -> [u8; 32] {
        vector(name).try_into().unwrap()
    }

    fn vault_key() -> VaultKey {
        let (_, _, _, generation) = ids();
        VaultKey {
            key: SecretBytes::from_slice(&vector("vk")).unwrap(),
            generation,
        }
    }

    /// RFC 9180 appendix A.2.3: DHKEM(X25519, HKDF-SHA256), HKDF-SHA256,
    /// ChaCha20-Poly1305, mode Auth. Pins the `hpke` crate to the published
    /// vector for key derivation, auth encapsulation, key schedule and AEAD.
    #[test]
    fn rfc9180_a23_auth() {
        let ikm_e = hex!("938d3daa5a8904540bc24f48ae90eed3f4f7f11839560597b55e7c9598c996c0");
        let ikm_r = hex!("64835d5ee64aa7aad57c6f2e4f758f7696617f8829e70bc9ac7a5ef95d1c756c");
        let ikm_s = hex!("9d8f94537d5a3ddef71234c0baedfad4ca6861634d0b94c3007fed557ad17df6");
        let info = hex!("4f6465206f6e2061204772656369616e2055726e");
        let pt = hex!("4265617574792069732074727574682c20747275746820626561757479");
        let aad = hex!("436f756e742d30");
        let (isk_s, ipk_s) = IdentitySecret::derive(&ikm_s).unwrap();
        let (isk_r, ipk_r) = IdentitySecret::derive(&ikm_r).unwrap();
        let (enc, ct) = raw_seal(&isk_s, &ipk_r, &info, &aad, &pt, ikm_e).unwrap();
        assert_eq!(
            enc,
            hex!("f7674cc8cd7baa5872d1f33dbaffe3314239f6197ddf5ded1746760bfc847e0e")
        );
        assert_eq!(
            ct,
            hex!(
                "ab1a13c9d4f01a87ec3440dbd756e2677bd2ecf9df0ce7ed73869b98e00c09be"
                "111cb9fdf077347aeb88e61bdf"
            )
        );
        let opened = raw_open(&isk_r, &ipk_s, &enc, &info, &aad, &ct).unwrap();
        assert_eq!(opened.expose_secret(), pt);
        // A flipped bit in the public sender key (mode Auth) does not open.
        let mut wrong = ipk_s;
        wrong.0[0] ^= 1;
        assert!(raw_open(&isk_r, &wrong, &enc, &info, &aad, &ct).is_err());
    }

    /// `crates/core/tests/vectors/share_kat.txt`, written by the independent
    /// script `share_kat.py`: the share and emergency seals, identity keys,
    /// fingerprints and safety codes, and the identity wrap, roster and
    /// contacts objects with fixed nonces.
    #[test]
    fn mosaic_vectors_match_python() {
        let (sender, recipient, vault, generation) = ids();
        let (isk_s, ipk_s) = IdentitySecret::derive(&arr32("sender_ikm")).unwrap();
        let (isk_r, ipk_r) = IdentitySecret::derive(&arr32("recipient_ikm")).unwrap();
        assert_eq!(isk_s.expose_for_tests().as_slice(), vector("sender_isk"));
        assert_eq!(ipk_s.0.as_slice(), vector("sender_ipk"));
        assert_eq!(ipk_r.0.as_slice(), vector("recipient_ipk"));

        // Fingerprints and safety codes.
        let fpr_s = Fingerprint::of(&sender, &ipk_s);
        let fpr_r = Fingerprint::of(&recipient, &ipk_r);
        assert_eq!(fpr_s.as_bytes().as_slice(), vector("sender_fpr"));
        assert_eq!(fpr_r.as_bytes().as_slice(), vector("recipient_fpr"));
        assert_eq!(
            fpr_s.safety_code().as_bytes(),
            vector("sender_safety_code").as_slice()
        );
        assert_eq!(
            fpr_r.safety_code().as_bytes(),
            vector("recipient_safety_code").as_slice()
        );

        // Share and emergency seals.
        let vk = vault_key();
        let share = seal_vault_key_with_ephemeral(
            &vk,
            &vault,
            &sender,
            &isk_s,
            &recipient,
            &ipk_r,
            arr32("eph_share_ikm"),
        )
        .unwrap();
        assert_eq!(share.as_bytes().as_slice(), vector("share_seal"));
        let emergency = seal_emergency_with_ephemeral(
            &vk,
            &vault,
            &sender,
            &isk_s,
            &recipient,
            &ipk_r,
            arr32("eph_emergency_ikm"),
        )
        .unwrap();
        assert_eq!(emergency.as_bytes().as_slice(), vector("emergency_seal"));
        let opened = open_vault_key(
            &share, &vault, generation, &sender, &ipk_s, &recipient, &isk_r,
        )
        .unwrap();
        assert_eq!(opened, vk);
        let opened = open_emergency_vault_key(
            &emergency, &vault, generation, &sender, &ipk_s, &recipient, &isk_r,
        )
        .unwrap();
        assert_eq!(opened, vk);

        // The documented info and aad strings are the ones the code builds.
        assert_eq!(
            Kind::Share.info(&vault, &sender, &recipient),
            vector("share_info")
        );
        assert_eq!(
            Kind::Share.aad(&vault, generation).as_bytes(),
            vector("share_aad").as_slice()
        );
        assert_eq!(
            Kind::Emergency.info(&vault, &sender, &recipient),
            vector("emergency_info")
        );
        assert_eq!(
            Kind::Emergency.aad(&vault, generation).as_bytes(),
            vector("emergency_aad").as_slice()
        );
    }

    /// The identity wrap, roster and contact book of the same file.
    #[test]
    fn mosaic_objects_match_python() {
        let (sender, _, vault, _) = ids();
        let (isk_s, _) = IdentitySecret::derive(&arr32("sender_ikm")).unwrap();
        let vk = vault_key();

        // Identity wrap under AK, bound to the sender account.
        let ak = AccountKey(SecretBytes::from_slice(&vector("ak")).unwrap());
        let nonce: [u8; 24] = vector("identity_wrap_nonce").try_into().unwrap();
        let wrapped = isk_s.wrap_with_nonce(&ak, &sender, &nonce).unwrap();
        assert_eq!(wrapped.as_bytes().as_slice(), vector("identity_wrap"));
        assert_eq!(wrapped.unwrap_identity(&ak, &sender).unwrap(), isk_s);

        // Roster under VK at generation 3.
        let nonce: [u8; 24] = vector("roster_nonce").try_into().unwrap();
        let roster =
            VaultRosterCiphertext::seal_with_nonce(&vk, &vault, br#"{"roster":"kat"}"#, &nonce)
                .unwrap();
        assert_eq!(roster.as_bytes(), vector("roster_ct").as_slice());
        assert_eq!(
            roster.open(&vk, &vault).unwrap().expose_secret(),
            br#"{"roster":"kat"}"#
        );

        // Contact book under K_contacts.
        let key = ak.contacts_key().unwrap();
        assert_eq!(key.0.expose_secret().as_slice(), vector("contacts_key"));
        let nonce: [u8; 24] = vector("contacts_nonce").try_into().unwrap();
        let contacts =
            ContactsCiphertext::seal_with_nonce(&key, &sender, br#"{"contacts":"kat"}"#, &nonce)
                .unwrap();
        assert_eq!(contacts.as_bytes(), vector("contacts_ct").as_slice());
        assert_eq!(
            contacts.open(&key, &sender).unwrap().expose_secret(),
            br#"{"contacts":"kat"}"#
        );
    }
}
