//! Unlock-method key derivations (CRYPTO_SPEC §6.2, §6.3, §7.4, ADR-0028).
//!
//! Restricted API for `mosaic-vault` and the M3 unlock-method setup. No new
//! primitive: every key here is HKDF-SHA-256 or `random(32)`, and every
//! wrapping is the existing [`WrappedKey`] with a unique AAD label.
//!
//! | Function | Formula |
//! |---|---|
//! | [`hello_unlock_key`] | `HKDF(ikm = Sign(challenge), salt = challenge, info = ctx("hello-unlock"), 32)` |
//! | [`prf_unlock_key`] | `HKDF(ikm = PRF(prf_salt), salt = SHA-256(credential_id), info = ctx("prf-unlock"), 32)` |
//! | [`prf_hw_vault_key`] | `HKDF(ikm = PRF(prf_salt_hw), salt = SHA-256(credential_id), info = ctx("prf-hw-vault"), 32)` |
//! | [`prf_salt_input`] | `SHA-256("WebAuthn PRF" \|\| 0x00 \|\| prf_salt)` |

use sha2::{Digest, Sha256};

use crate::aead::{self, NONCE_LEN};
use crate::encoding::{CTX_PREFIX, Label, label};
use crate::envelope::WrappedKey;
use crate::error::{Error, Result};
use crate::ids::{AccountId, KeyGeneration, VaultId};
use crate::kdf::{hkdf_ctx, hkdf_sha256};
use crate::rng;
use crate::secret::{Kek, SecretBytes, SecretVec, VaultKey};

pub use crate::secret::{PrfOutput, TotpVaultKey};

/// Length of the Windows Hello challenge (§6.2).
pub const HELLO_CHALLENGE_LEN: usize = 32;
/// Shortest accepted Windows Hello signature (RSA 1024 bit).
pub const MIN_HELLO_SIGNATURE_LEN: usize = 128;
/// Longest accepted Windows Hello signature (RSA 8192 bit).
pub const MAX_HELLO_SIGNATURE_LEN: usize = 1024;
/// Shortest accepted WebAuthn credential ID (§6.3).
pub const MIN_CREDENTIAL_ID_LEN: usize = 16;
/// Longest accepted WebAuthn credential ID (§6.3).
pub const MAX_CREDENTIAL_ID_LEN: usize = crate::model::MAX_CREDENTIAL_ID_LEN;
/// The WebAuthn PRF salt prefix (WebAuthn L3 §10.1.4), not a Mosaic label.
pub const WEBAUTHN_PRF_PREFIX: &[u8] = b"WebAuthn PRF\x00";

impl TotpVaultKey {
    /// Wraps `TK` under a hardware key's `W_hw` (§7.4):
    /// `aad = header || aad("wrap/hw-vault", account_id, vault_id, method_id[16])`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn wrap(
        &self,
        w_hw: &Kek,
        account_id: &AccountId,
        vault_id: &VaultId,
        method_id: &[u8; 16],
    ) -> Result<WrappedKey> {
        self.wrap_with_nonce(
            w_hw,
            account_id,
            vault_id,
            method_id,
            &aead::random_nonce()?,
        )
    }

    pub(crate) fn wrap_with_nonce(
        &self,
        w_hw: &Kek,
        account_id: &AccountId,
        vault_id: &VaultId,
        method_id: &[u8; 16],
        nonce: &[u8; NONCE_LEN],
    ) -> Result<WrappedKey> {
        WrappedKey::seal(
            w_hw.0.expose_secret(),
            nonce,
            &WrappedKey::tk_aad(account_id, vault_id, method_id),
            self.0.expose_secret(),
        )
    }
}

impl VaultKey {
    /// Wraps `VK` under `TK` (§7.4):
    /// `aad = header || aad("wrap/vault-under-tk", vault_id, key_generation:u32)`.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the CSPRNG fails.
    pub fn wrap_under_tk(&self, tk: &TotpVaultKey, vault_id: &VaultId) -> Result<WrappedKey> {
        self.wrap_under_tk_with_nonce(tk, vault_id, &aead::random_nonce()?)
    }

    pub(crate) fn wrap_under_tk_with_nonce(
        &self,
        tk: &TotpVaultKey,
        vault_id: &VaultId,
        nonce: &[u8; NONCE_LEN],
    ) -> Result<WrappedKey> {
        WrappedKey::seal(
            tk.0.expose_secret(),
            nonce,
            &WrappedKey::vault_under_tk_aad(vault_id, self.generation),
            self.key.expose_secret(),
        )
    }
}

impl WrappedKey {
    /// Unwraps the key `TK` of a hardware-protected vault (§7.4).
    ///
    /// # Errors
    /// [`Error::Decryption`] for a wrong key, wrong context or tampered data
    /// (indistinguishable).
    pub fn unwrap_totp_vault_key(
        &self,
        w_hw: &Kek,
        account_id: &AccountId,
        vault_id: &VaultId,
        method_id: &[u8; 16],
    ) -> Result<TotpVaultKey> {
        let key = self.open(
            w_hw.0.expose_secret(),
            &Self::tk_aad(account_id, vault_id, method_id),
        )?;
        Ok(TotpVaultKey(key))
    }

    /// Unwraps a vault key of generation `generation` held under `TK` (§7.4).
    ///
    /// # Errors
    /// [`Error::Decryption`] for a wrong key, wrong context (including a
    /// wrong generation) or tampered data.
    pub fn unwrap_vault_key_under_tk(
        &self,
        tk: &TotpVaultKey,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<VaultKey> {
        let key = self.open(
            tk.0.expose_secret(),
            &Self::vault_under_tk_aad(vault_id, generation),
        )?;
        Ok(VaultKey { key, generation })
    }
}

/// `W = HKDF(ikm = signature, salt = challenge, info = ctx("hello-unlock"), 32)`
/// for Windows Hello quick unlock (§6.2).
///
/// # Errors
/// [`Error::Malformed`] if the signature is not 128 to 1024 bytes.
pub fn hello_unlock_key(
    signature: &SecretVec,
    challenge: &[u8; HELLO_CHALLENGE_LEN],
) -> Result<Kek> {
    if !(MIN_HELLO_SIGNATURE_LEN..=MAX_HELLO_SIGNATURE_LEN).contains(&signature.len()) {
        return Err(Error::Malformed);
    }
    Ok(Kek(hkdf_ctx(
        signature.expose_secret(),
        challenge,
        label::HELLO_UNLOCK,
    )?))
}

/// `SHA-256("WebAuthn PRF" || 0x00 || prf_salt)`: the CTAP `hmac-secret` salt
/// for a PRF input (§3). Public data.
#[must_use]
pub fn prf_salt_input(prf_salt: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(WEBAUTHN_PRF_PREFIX);
    h.update(prf_salt);
    h.finalize().into()
}

/// `SHA-256(credential_id)` (§3).
///
/// # Errors
/// [`Error::Malformed`] if the credential ID is not 16 to 1023 bytes.
pub fn credential_id_hash(credential_id: &[u8]) -> Result<[u8; 32]> {
    if !(MIN_CREDENTIAL_ID_LEN..=MAX_CREDENTIAL_ID_LEN).contains(&credential_id.len()) {
        return Err(Error::Malformed);
    }
    Ok(Sha256::digest(credential_id).into())
}

/// `W = HKDF(ikm = PRF(prf_salt), salt = credential_id_hash, info = ctx("prf-unlock"), 32)` (§6.3).
///
/// # Errors
/// [`Error::Malformed`] for a credential ID outside 16 to 1023 bytes.
pub fn prf_unlock_key(prf: &PrfOutput, credential_id: &[u8]) -> Result<Kek> {
    prf_key(prf, credential_id, label::PRF_UNLOCK)
}

/// `W_hw = HKDF(ikm = PRF(prf_salt_hw), salt = credential_id_hash, info = ctx("prf-hw-vault"), 32)` (§7.4).
///
/// # Errors
/// [`Error::Malformed`] for a credential ID outside 16 to 1023 bytes.
pub fn prf_hw_vault_key(prf: &PrfOutput, credential_id: &[u8]) -> Result<Kek> {
    prf_key(prf, credential_id, label::PRF_HW_VAULT)
}

fn prf_key(prf: &PrfOutput, credential_id: &[u8], label: Label) -> Result<Kek> {
    let salt = credential_id_hash(credential_id)?;
    let mut out = SecretBytes::<32>::zero();
    hkdf_sha256(
        prf.0.expose_secret(),
        &salt,
        &[CTX_PREFIX.as_bytes(), label.as_bytes()],
        out.expose_secret_mut(),
    )?;
    Ok(Kek(out))
}

/// `random(32)` for `prf_salt` (§6.3) or `prf_salt_hw` (§7.4). Public data.
///
/// # Errors
/// [`Error::Randomness`] if the CSPRNG fails.
pub fn new_prf_salt() -> Result<[u8; 32]> {
    rng::array()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::envelope::UnlockMethod;
    use crate::ids::DeviceId;
    use crate::secret::AccountKey;

    fn account() -> AccountId {
        AccountId::from_bytes([0x0A; 16]).unwrap()
    }
    fn vault() -> VaultId {
        VaultId::from_bytes([0x0B; 16]).unwrap()
    }
    fn sig(len: usize, fill: u8) -> SecretVec {
        SecretVec::from_slice(&vec![fill; len]).unwrap()
    }
    fn prf(fill: u8) -> PrfOutput {
        PrfOutput::from_secret(SecretBytes::from_array(&mut [fill; 32]))
    }
    fn wrap_ak(ak: &AccountKey, w: &Kek, m: &UnlockMethod) -> WrappedKey {
        ak.wrap(w, &account(), m).unwrap()
    }
    fn quick() -> UnlockMethod {
        UnlockMethod::QuickUnlock(DeviceId::from_bytes([7; 16]).unwrap())
    }

    #[test]
    fn hello_key_round_trip() {
        let m = quick();
        let ak = AccountKey::generate().unwrap();
        let w1 = hello_unlock_key(&sig(256, 0x5A), &[0x11; 32]).unwrap();
        let w2 = hello_unlock_key(&sig(256, 0x5A), &[0x11; 32]).unwrap();
        let wrapped = wrap_ak(&ak, &w1, &m);
        assert_eq!(wrapped.unwrap_account_key(&w2, &account(), &m).unwrap(), ak);
        // One bit of the signature or the challenge changes W.
        let mut s = vec![0x5A; 256];
        s[100] ^= 1;
        let w3 = hello_unlock_key(&SecretVec::from_slice(&s).unwrap(), &[0x11; 32]).unwrap();
        let mut c = [0x11; 32];
        c[31] ^= 0x80;
        let w4 = hello_unlock_key(&sig(256, 0x5A), &c).unwrap();
        for w in [w3, w4] {
            assert_eq!(
                wrapped.unwrap_account_key(&w, &account(), &m).unwrap_err(),
                Error::Decryption
            );
        }
    }

    #[test]
    fn hello_signature_bounds() {
        for len in [0, 127, 1025] {
            assert_eq!(
                hello_unlock_key(&sig(len, 1), &[0; 32]).unwrap_err(),
                Error::Malformed,
                "{len}"
            );
        }
        for len in [128, 1024] {
            assert!(hello_unlock_key(&sig(len, 1), &[0; 32]).is_ok(), "{len}");
        }
    }

    #[test]
    fn credential_id_bounds() {
        for len in [15, 1024] {
            let id = vec![9u8; len];
            assert_eq!(credential_id_hash(&id).unwrap_err(), Error::Malformed);
            assert_eq!(prf_unlock_key(&prf(1), &id).unwrap_err(), Error::Malformed);
            assert_eq!(
                prf_hw_vault_key(&prf(1), &id).unwrap_err(),
                Error::Malformed
            );
        }
        for len in [16, 1023] {
            let id = vec![9u8; len];
            assert_eq!(
                credential_id_hash(&id).unwrap(),
                <[u8; 32]>::from(Sha256::digest(&id))
            );
            assert!(prf_unlock_key(&prf(1), &id).is_ok());
            assert!(prf_hw_vault_key(&prf(1), &id).is_ok());
        }
    }

    #[test]
    fn prf_labels_are_separated() {
        let id = [0x44; 32];
        let a = prf_unlock_key(&prf(0x33), &id).unwrap();
        let b = prf_hw_vault_key(&prf(0x33), &id).unwrap();
        assert_ne!(a, b);
        let m = UnlockMethod::hardware_key(&id);
        let ak = AccountKey::generate().unwrap();
        let wrapped = wrap_ak(&ak, &a, &m);
        assert_eq!(
            wrapped.unwrap_account_key(&b, &account(), &m).unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn prf_salt_input_is_prefixed_hash() {
        let salt = [0x22; 32];
        let mut m = b"WebAuthn PRF\x00".to_vec();
        m.extend_from_slice(&salt);
        assert_eq!(prf_salt_input(&salt), <[u8; 32]>::from(Sha256::digest(&m)));
        assert_ne!(
            prf_salt_input(&salt),
            <[u8; 32]>::from(Sha256::digest(salt))
        );
    }

    #[test]
    fn tk_wrap_binds_context() {
        let id = [0x44; 32];
        let method_id: [u8; 16] = Sha256::digest(id)[..16].try_into().unwrap();
        let w = prf_hw_vault_key(&prf(0x33), &id).unwrap();
        let tk = TotpVaultKey::generate().unwrap();
        let wrapped = tk.wrap(&w, &account(), &vault(), &method_id).unwrap();
        let back = wrapped
            .unwrap_totp_vault_key(&w, &account(), &vault(), &method_id)
            .unwrap();
        assert_eq!(back, tk);

        let other_account = AccountId::from_bytes([0x0C; 16]).unwrap();
        let other_vault = VaultId::from_bytes([0x0D; 16]).unwrap();
        let mut other_method = method_id;
        other_method[0] ^= 1;
        let other_w = prf_hw_vault_key(&prf(0x34), &id).unwrap();
        for r in [
            wrapped.unwrap_totp_vault_key(&w, &other_account, &vault(), &method_id),
            wrapped.unwrap_totp_vault_key(&w, &account(), &other_vault, &method_id),
            wrapped.unwrap_totp_vault_key(&w, &account(), &vault(), &other_method),
            wrapped.unwrap_totp_vault_key(&other_w, &account(), &vault(), &method_id),
        ] {
            assert_eq!(r.unwrap_err(), Error::Decryption);
        }

        // Labels differ: a TK wrapping is not an AK wrapping and vice versa.
        let m = UnlockMethod::HardwareKey(method_id);
        assert_eq!(
            wrapped.unwrap_account_key(&w, &account(), &m).unwrap_err(),
            Error::Decryption
        );
        let ak = AccountKey::generate().unwrap();
        let ak_wrapped = wrap_ak(&ak, &w, &m);
        assert_eq!(
            ak_wrapped
                .unwrap_totp_vault_key(&w, &account(), &vault(), &method_id)
                .unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn vk_under_tk_binds_context() {
        let tk = TotpVaultKey::generate().unwrap();
        let gen2 = KeyGeneration::new(2).unwrap();
        let vk = VaultKey::generate(gen2).unwrap();
        let wrapped = vk.wrap_under_tk(&tk, &vault()).unwrap();
        assert_eq!(
            wrapped
                .unwrap_vault_key_under_tk(&tk, &vault(), gen2)
                .unwrap(),
            vk
        );
        let other_vault = VaultId::from_bytes([0x0D; 16]).unwrap();
        let other_tk = TotpVaultKey::generate().unwrap();
        for r in [
            wrapped.unwrap_vault_key_under_tk(&tk, &vault(), KeyGeneration::FIRST),
            wrapped.unwrap_vault_key_under_tk(&tk, &other_vault, gen2),
            wrapped.unwrap_vault_key_under_tk(&other_tk, &vault(), gen2),
        ] {
            assert_eq!(r.unwrap_err(), Error::Decryption);
        }
        // Not openable as an ordinary vault key with TK's bytes as AK.
        let as_ak = AccountKey(SecretBytes::from_slice(tk.0.expose_secret()).unwrap());
        assert_eq!(
            wrapped
                .unwrap_vault_key(&as_ak, &account(), &vault(), gen2)
                .unwrap_err(),
            Error::Decryption
        );
    }

    #[test]
    fn kek_generate_is_random() {
        let a = Kek::generate().unwrap();
        let b = Kek::generate().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.expose_secret().len(), 32);
        assert_ne!(a.expose_secret(), &[0u8; 32]);
        assert_ne!(new_prf_salt().unwrap(), new_prf_salt().unwrap());
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

        #[test]
        fn derived_keys_wrap_and_reject_tampering(
            signature in proptest::collection::vec(any::<u8>(), 128..=1024),
            challenge in any::<[u8; 32]>(),
            prf_bytes in any::<[u8; 32]>(),
            credential_id in proptest::collection::vec(any::<u8>(), 16..=1023),
            flip in any::<usize>(),
            bit in 0u8..8,
        ) {
            let ak = AccountKey::generate().unwrap();
            let hello = hello_unlock_key(&SecretVec::from_slice(&signature).unwrap(), &challenge).unwrap();
            let prf_out = PrfOutput::from_secret(SecretBytes::from_slice(&prf_bytes).unwrap());
            let hw = prf_unlock_key(&prf_out, &credential_id).unwrap();
            let methods = [
                (hello, quick()),
                (hw, UnlockMethod::hardware_key(&credential_id)),
            ];
            for (w, m) in &methods {
                let wrapped = wrap_ak(&ak, w, m);
                prop_assert!(wrapped.unwrap_account_key(w, &account(), m).unwrap() == ak);
                let mut bytes = *wrapped.as_bytes();
                let i = flip % bytes.len();
                bytes[i] ^= 1 << bit;
                // A flipped header byte is a different (unsupported) object;
                // everything else must fail authentication.
                if let Ok(tampered) = WrappedKey::from_bytes(&bytes) {
                    prop_assert_eq!(
                        tampered.unwrap_account_key(w, &account(), m).unwrap_err(),
                        Error::Decryption
                    );
                }
            }
        }
    }
}
