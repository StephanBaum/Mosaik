//! Key generation, wrapping and the password flows (CRYPTO_SPEC §5–§7).
//!
//! ```text
//! unlock:  MP --prepare--> MP' --Argon2id--> P --HKDF(SK, account_id)--> KEK, OPW
//!          KEK --unwrap(wrap/ak, method = password)--> AK --unwrap(wrap/vault)--> VK
//! change:  new MP, new account_salt, same SK, same (or stronger) profile
//!          --> KEK' --wrap--> AK (vault keys and items are not touched, §6.4;
//!          a weaker profile is refused, §5.3)
//! ```

use crate::aead::{self, NONCE_LEN};
use crate::envelope::{UnlockMethod, WrappedKey};
use crate::error::{Error, Result};
use crate::ids::{AccountId, KeyGeneration, VaultId};
use crate::kdf::{self, AccountSalt, Argon2Cost, Argon2Profile, DerivedKeys};
use crate::secret::{
    AccountKey, Kek, MasterPassword, OpaquePassword, SecretBytes, SecretKey, VaultKey,
};

impl AccountKey {
    /// A fresh account key `AK = random(32)` (§6).
    ///
    /// # Errors
    /// [`crate::Error::Randomness`] if the CSPRNG fails.
    pub fn generate() -> Result<Self> {
        Ok(Self(SecretBytes::random()?))
    }

    /// Wraps `AK` under an unlock method's key `W` (§6):
    /// `aad = header || aad("wrap/ak", account_id, method:u8, method_id[16])`.
    ///
    /// # Errors
    /// [`crate::Error::Randomness`] if the CSPRNG fails.
    pub fn wrap(
        &self,
        w: &Kek,
        account_id: &AccountId,
        method: &UnlockMethod,
    ) -> Result<WrappedKey> {
        self.wrap_with_nonce(w, account_id, method, &aead::random_nonce()?)
    }

    pub(crate) fn wrap_with_nonce(
        &self,
        w: &Kek,
        account_id: &AccountId,
        method: &UnlockMethod,
        nonce: &[u8; NONCE_LEN],
    ) -> Result<WrappedKey> {
        WrappedKey::seal(
            w.0.expose_secret(),
            nonce,
            &WrappedKey::ak_aad(account_id, method),
            self.0.expose_secret(),
        )
    }
}

impl VaultKey {
    /// A fresh vault key `VK = random(32)` of the given generation (§7.1).
    /// New vaults use [`KeyGeneration::FIRST`].
    ///
    /// # Errors
    /// [`crate::Error::Randomness`] if the CSPRNG fails.
    pub fn generate(generation: KeyGeneration) -> Result<Self> {
        Ok(Self {
            key: SecretBytes::random()?,
            generation,
        })
    }

    /// Wraps `VK` under `AK` (§7.1):
    /// `aad = header || aad("wrap/vault", account_id, vault_id, key_generation:u32)`.
    ///
    /// # Errors
    /// [`crate::Error::Randomness`] if the CSPRNG fails.
    pub fn wrap(
        &self,
        ak: &AccountKey,
        account_id: &AccountId,
        vault_id: &VaultId,
    ) -> Result<WrappedKey> {
        self.wrap_with_nonce(ak, account_id, vault_id, &aead::random_nonce()?)
    }

    pub(crate) fn wrap_with_nonce(
        &self,
        ak: &AccountKey,
        account_id: &AccountId,
        vault_id: &VaultId,
        nonce: &[u8; NONCE_LEN],
    ) -> Result<WrappedKey> {
        WrappedKey::seal(
            ak.0.expose_secret(),
            nonce,
            &WrappedKey::vault_aad(account_id, vault_id, self.generation),
            self.key.expose_secret(),
        )
    }
}

impl WrappedKey {
    /// Unwraps an account key. `account_id` and `method` come from context.
    ///
    /// # Errors
    /// [`crate::Error::Decryption`] for a wrong key, wrong context or
    /// tampered data.
    pub fn unwrap_account_key(
        &self,
        w: &Kek,
        account_id: &AccountId,
        method: &UnlockMethod,
    ) -> Result<AccountKey> {
        let key = self.open(w.0.expose_secret(), &Self::ak_aad(account_id, method))?;
        Ok(AccountKey(key))
    }

    /// Unwraps a vault key of generation `generation`. All context comes
    /// from the caller (the record the wrapping is stored in).
    ///
    /// # Errors
    /// [`crate::Error::Decryption`] for a wrong key, wrong context (including
    /// a wrong generation) or tampered data.
    pub fn unwrap_vault_key(
        &self,
        ak: &AccountKey,
        account_id: &AccountId,
        vault_id: &VaultId,
        generation: KeyGeneration,
    ) -> Result<VaultKey> {
        let key = self.open(
            ak.0.expose_secret(),
            &Self::vault_aad(account_id, vault_id, generation),
        )?;
        Ok(VaultKey { key, generation })
    }
}

/// The public, per-account derivation parameters (§5.3, §9.2 pre-login).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PasswordParams {
    /// The account ID (HKDF salt of §5.4).
    pub account_id: AccountId,
    /// The Argon2id salt.
    pub salt: AccountSalt,
    /// The Argon2id profile.
    pub profile: Argon2Profile,
}

fn derive_with_cost(
    password: &MasterPassword,
    secret_key: &SecretKey,
    account_id: &AccountId,
    salt: &AccountSalt,
    cost: Argon2Cost,
) -> Result<DerivedKeys> {
    let prepared = kdf::prepare_password(password)?;
    let stretched = kdf::stretch_with_cost(&prepared, salt, cost)?;
    drop(prepared);
    kdf::derive_keys(stretched, secret_key, account_id)
}

/// `MP -> MP' -> P -> (KEK, OPW)` (§5.1, §5.3, §5.4).
///
/// # Errors
/// [`crate::Error::EmptyPassword`] or [`crate::Error::Allocation`].
pub fn derive_password_keys(
    password: &MasterPassword,
    secret_key: &SecretKey,
    params: &PasswordParams,
) -> Result<DerivedKeys> {
    derive_with_cost(
        password,
        secret_key,
        &params.account_id,
        &params.salt,
        params.profile.cost(),
    )
}

pub(crate) fn unlock_with_cost(
    password: &MasterPassword,
    secret_key: &SecretKey,
    params: &PasswordParams,
    wrapped: &WrappedKey,
    cost: Argon2Cost,
) -> Result<AccountKey> {
    let keys = derive_with_cost(password, secret_key, &params.account_id, &params.salt, cost)?;
    wrapped.unwrap_account_key(&keys.kek, &params.account_id, &UnlockMethod::Password)
}

/// Password unlock (§6.1): derive `KEK` from MP and SK and unwrap AK.
///
/// # Errors
/// [`crate::Error::Decryption`] for a wrong password, Secret Key or
/// parameters (indistinguishable); [`crate::Error::EmptyPassword`],
/// [`crate::Error::Allocation`].
pub fn unlock_with_password(
    password: &MasterPassword,
    secret_key: &SecretKey,
    params: &PasswordParams,
    wrapped_account_key: &WrappedKey,
) -> Result<AccountKey> {
    unlock_with_cost(
        password,
        secret_key,
        params,
        wrapped_account_key,
        params.profile.cost(),
    )
}

/// Everything account creation produces on the device (§5.2, §5.3, §6).
#[derive(Debug)]
pub struct NewAccount {
    /// Public parameters, sent to the server.
    pub params: PasswordParams,
    /// The new Secret Key: shown once (Emergency Kit), then kept in the OS
    /// keystore. Never sent to the server.
    pub secret_key: SecretKey,
    /// The new account key.
    pub account_key: AccountKey,
    /// AK wrapped for password unlock, synced to the server.
    pub wrapped_account_key: WrappedKey,
    /// The OPAQUE registration input.
    pub opaque_password: OpaquePassword,
}

pub(crate) fn create_account_with_cost(
    password: &MasterPassword,
    profile: Argon2Profile,
    cost: Argon2Cost,
) -> Result<NewAccount> {
    let params = PasswordParams {
        account_id: AccountId::new_v7()?,
        salt: AccountSalt::generate()?,
        profile,
    };
    let secret_key = SecretKey::generate()?;
    let keys = derive_with_cost(
        password,
        &secret_key,
        &params.account_id,
        &params.salt,
        cost,
    )?;
    let account_key = AccountKey::generate()?;
    let wrapped_account_key =
        account_key.wrap(&keys.kek, &params.account_id, &UnlockMethod::Password)?;
    Ok(NewAccount {
        params,
        secret_key,
        account_key,
        wrapped_account_key,
        opaque_password: keys.opaque_password,
    })
}

/// Creates a new account: account ID (UUIDv7), salt, Secret Key, AK and its
/// password wrapping, and `OPW` for OPAQUE registration. `profile` normally
/// comes from [`kdf::calibrate`].
///
/// # Errors
/// [`crate::Error::EmptyPassword`], [`crate::Error::Allocation`],
/// [`crate::Error::Randomness`].
pub fn create_account(password: &MasterPassword, profile: Argon2Profile) -> Result<NewAccount> {
    create_account_with_cost(password, profile, profile.cost())
}

/// The result of a master password change (§6.4).
#[derive(Debug)]
pub struct PasswordChange {
    /// The new public parameters (new salt; same account and profile).
    pub params: PasswordParams,
    /// AK re-wrapped under the new `KEK'`; replaces the old password
    /// wrapping atomically on the server.
    pub wrapped_account_key: WrappedKey,
    /// `OPW'` for OPAQUE re-registration.
    pub opaque_password: OpaquePassword,
}

pub(crate) fn change_master_password_with_cost(
    account_key: &AccountKey,
    new_password: &MasterPassword,
    secret_key: &SecretKey,
    current: &PasswordParams,
    profile: Argon2Profile,
    cost: Argon2Cost,
) -> Result<PasswordChange> {
    // §5.3: profiles only ever increase. §6.4 derives under the current
    // parameters; a §5.3 upgrade may pass a stronger profile.
    if profile < current.profile {
        return Err(Error::ProfileDowngrade);
    }
    let account_id = &current.account_id;
    let params = PasswordParams {
        account_id: *account_id,
        salt: AccountSalt::generate()?,
        profile,
    };
    let keys = derive_with_cost(new_password, secret_key, account_id, &params.salt, cost)?;
    let wrapped_account_key = account_key.wrap(&keys.kek, account_id, &UnlockMethod::Password)?;
    Ok(PasswordChange {
        params,
        wrapped_account_key,
        opaque_password: keys.opaque_password,
    })
}

/// Master password change (§6.4): derive `KEK'` from the new password with a
/// new `account_salt`, the same SK and the account's profile, and re-wrap AK.
/// Vault keys and items are not touched. Passing a different `secret_key`
/// is a Secret Key rotation (§6.5). Neither revokes holders of old secrets
/// plus an old wrapping; see §6.6.
///
/// `current` are the account's current public parameters (the ones the last
/// successful unlock used). `profile` is normally `current.profile`; a
/// stronger one performs the §5.3 upgrade at the same time.
///
/// # Errors
/// [`crate::Error::ProfileDowngrade`] if `profile` is weaker than
/// `current.profile` (§5.3: profiles only ever increase);
/// [`crate::Error::EmptyPassword`], [`crate::Error::Allocation`],
/// [`crate::Error::Randomness`].
pub fn change_master_password(
    account_key: &AccountKey,
    new_password: &MasterPassword,
    secret_key: &SecretKey,
    current: &PasswordParams,
    profile: Argon2Profile,
) -> Result<PasswordChange> {
    change_master_password_with_cost(
        account_key,
        new_password,
        secret_key,
        current,
        profile,
        profile.cost(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ItemCiphertext;
    use crate::ids::{DeviceId, ItemId};
    use crate::kdf::TEST_COST;

    fn pw(s: &str) -> MasterPassword {
        MasterPassword::new(s.to_string())
    }

    #[test]
    fn create_and_unlock() {
        let acct =
            create_account_with_cost(&pw("old password"), Argon2Profile::Standard, TEST_COST)
                .unwrap();
        let ak = unlock_with_cost(
            &pw("old password"),
            &acct.secret_key,
            &acct.params,
            &acct.wrapped_account_key,
            TEST_COST,
        )
        .unwrap();
        assert_eq!(ak, acct.account_key);

        let wrong_pw = unlock_with_cost(
            &pw("old passw0rd"),
            &acct.secret_key,
            &acct.params,
            &acct.wrapped_account_key,
            TEST_COST,
        );
        assert_eq!(wrong_pw.unwrap_err(), Error::Decryption);
        let wrong_sk = unlock_with_cost(
            &pw("old password"),
            &SecretKey::generate().unwrap(),
            &acct.params,
            &acct.wrapped_account_key,
            TEST_COST,
        );
        assert_eq!(wrong_sk.unwrap_err(), Error::Decryption);
        // Wrong public parameters fail exactly like a wrong password: another
        // account's salt changes P, and another account_id changes both the
        // HKDF salt and the wrapping's AAD.
        let other_salt = PasswordParams {
            salt: AccountSalt::from_bytes([0x77; 16]),
            ..acct.params
        };
        let other_account = PasswordParams {
            account_id: AccountId::new_v7().unwrap(),
            ..acct.params
        };
        for params in [other_salt, other_account] {
            assert_eq!(
                unlock_with_cost(
                    &pw("old password"),
                    &acct.secret_key,
                    &params,
                    &acct.wrapped_account_key,
                    TEST_COST,
                ),
                Err(Error::Decryption)
            );
        }
        // The account ID is a fresh UUIDv7 (RFC 9562 version and variant).
        let id = acct.params.account_id.as_bytes();
        assert_eq!(id[6] >> 4, 7, "UUID version 7");
        assert_eq!(id[8] >> 6, 0b10, "RFC 9562 variant");
        let empty = unlock_with_cost(
            &pw(""),
            &acct.secret_key,
            &acct.params,
            &acct.wrapped_account_key,
            TEST_COST,
        );
        assert_eq!(empty.unwrap_err(), Error::EmptyPassword);
    }

    #[test]
    fn opw_matches_derivation_and_differs_from_kek() {
        let acct = create_account_with_cost(&pw("pw"), Argon2Profile::Baseline, TEST_COST).unwrap();
        let keys = derive_with_cost(
            &pw("pw"),
            &acct.secret_key,
            &acct.params.account_id,
            &acct.params.salt,
            TEST_COST,
        )
        .unwrap();
        assert_eq!(keys.opaque_password, acct.opaque_password);
        assert_ne!(
            keys.kek.0.expose_secret(),
            keys.opaque_password.expose_secret()
        );
    }

    /// §6.4: a password change re-wraps AK only. Item ciphertexts (and the
    /// wrapped vault key) are byte-for-byte unchanged and still open through
    /// the new unlock path.
    #[test]
    fn password_change_leaves_items_untouched() {
        let acct =
            create_account_with_cost(&pw("old password"), Argon2Profile::Standard, TEST_COST)
                .unwrap();
        let account_id = acct.params.account_id;
        let vault_id = VaultId::new_v7().unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        let wrapped_vk = vk.wrap(&acct.account_key, &account_id, &vault_id).unwrap();
        let items: Vec<(ItemId, ItemCiphertext)> = (0..5u8)
            .map(|n| {
                let id = ItemId::new_v7().unwrap();
                let ct = ItemCiphertext::seal(&vk, &vault_id, &id, 1, &[n; 40]).unwrap();
                (id, ct)
            })
            .collect();
        let items_before: Vec<Vec<u8>> = items.iter().map(|(_, c)| c.as_bytes().to_vec()).collect();
        let wrapped_vk_before = wrapped_vk.clone();

        let change = change_master_password_with_cost(
            &acct.account_key,
            &pw("new password"),
            &acct.secret_key,
            &acct.params,
            acct.params.profile,
            TEST_COST,
        )
        .unwrap();
        assert_eq!(change.params.account_id, account_id);
        assert_eq!(change.params.profile, acct.params.profile);
        assert_ne!(change.params.salt, acct.params.salt, "new account_salt");
        assert_ne!(change.wrapped_account_key, acct.wrapped_account_key);
        assert_ne!(change.opaque_password, acct.opaque_password);

        // Nothing item-related changed.
        let items_after: Vec<Vec<u8>> = items.iter().map(|(_, c)| c.as_bytes().to_vec()).collect();
        assert_eq!(items_before, items_after);
        assert_eq!(wrapped_vk, wrapped_vk_before);

        // New password unlocks the same AK; the old one fails on the new
        // wrapping.
        let ak = unlock_with_cost(
            &pw("new password"),
            &acct.secret_key,
            &change.params,
            &change.wrapped_account_key,
            TEST_COST,
        )
        .unwrap();
        assert_eq!(ak, acct.account_key);
        assert_eq!(
            unlock_with_cost(
                &pw("old password"),
                &acct.secret_key,
                &change.params,
                &change.wrapped_account_key,
                TEST_COST,
            )
            .unwrap_err(),
            Error::Decryption
        );

        let vk2 = wrapped_vk
            .unwrap_vault_key(&ak, &account_id, &vault_id, KeyGeneration::FIRST)
            .unwrap();
        for ((id, ct), n) in items.iter().zip(0u8..) {
            let reparsed = ItemCiphertext::from_bytes(ct.as_bytes()).unwrap();
            assert_eq!(
                reparsed.open(&vk2, &vault_id, id).unwrap().expose_secret(),
                &[n; 40]
            );
        }
    }

    #[test]
    fn wrap_context_binding() {
        let account_id = AccountId::new_v7().unwrap();
        let other_account = AccountId::new_v7().unwrap();
        let w = Kek(SecretBytes::random().unwrap());
        let ak = AccountKey::generate().unwrap();
        let device = DeviceId::new_v7().unwrap();
        for method in [
            UnlockMethod::Password,
            UnlockMethod::QuickUnlock(device),
            UnlockMethod::hardware_key(b"credential"),
        ] {
            let wrapped = ak.wrap(&w, &account_id, &method).unwrap();
            assert_eq!(
                wrapped
                    .unwrap_account_key(&w, &account_id, &method)
                    .unwrap(),
                ak
            );
            assert_eq!(
                wrapped.unwrap_account_key(&w, &other_account, &method),
                Err(Error::Decryption)
            );
            let other_w = Kek(SecretBytes::random().unwrap());
            assert_eq!(
                wrapped.unwrap_account_key(&other_w, &account_id, &method),
                Err(Error::Decryption)
            );
        }
        // Method byte and method_id are both bound.
        let quick = ak
            .wrap(&w, &account_id, &UnlockMethod::QuickUnlock(device))
            .unwrap();
        let other_device = DeviceId::new_v7().unwrap();
        assert_eq!(
            quick.unwrap_account_key(&w, &account_id, &UnlockMethod::QuickUnlock(other_device)),
            Err(Error::Decryption)
        );
        // Same 16 method_id bytes, different method byte.
        let hw_same_id = UnlockMethod::HardwareKey(*device.as_bytes());
        assert_eq!(
            quick.unwrap_account_key(&w, &account_id, &hw_same_id),
            Err(Error::Decryption)
        );
        let pw_wrap = ak.wrap(&w, &account_id, &UnlockMethod::Password).unwrap();
        let hw_acct_id = UnlockMethod::HardwareKey(*account_id.as_bytes());
        assert_eq!(
            pw_wrap.unwrap_account_key(&w, &account_id, &hw_acct_id),
            Err(Error::Decryption)
        );

        // Vault key wrapping binds account, vault and generation.
        let vault_id = VaultId::new_v7().unwrap();
        let g2 = KeyGeneration::new(2).unwrap();
        let vk = VaultKey::generate(g2).unwrap();
        let wvk = vk.wrap(&ak, &account_id, &vault_id).unwrap();
        assert_eq!(
            wvk.unwrap_vault_key(&ak, &account_id, &vault_id, g2)
                .unwrap(),
            vk
        );
        assert_eq!(
            wvk.unwrap_vault_key(&ak, &account_id, &vault_id, KeyGeneration::FIRST),
            Err(Error::Decryption)
        );
        assert_eq!(
            wvk.unwrap_vault_key(&ak, &other_account, &vault_id, g2),
            Err(Error::Decryption)
        );
        assert_eq!(
            wvk.unwrap_vault_key(&ak, &account_id, &VaultId::new_v7().unwrap(), g2),
            Err(Error::Decryption)
        );
        assert_eq!(
            wvk.unwrap_vault_key(&AccountKey::generate().unwrap(), &account_id, &vault_id, g2),
            Err(Error::Decryption)
        );
        // A vault-key wrapping is not an account-key wrapping (labels differ).
        let as_ak = Kek(SecretBytes::from_slice(ak.0.expose_secret()).unwrap());
        assert_eq!(
            wvk.unwrap_account_key(&as_ak, &account_id, &UnlockMethod::Password),
            Err(Error::Decryption)
        );
    }

    #[test]
    fn method_ids() {
        let account_id = AccountId::from_bytes([1; 16]).unwrap();
        let device = DeviceId::from_bytes([2; 16]).unwrap();
        assert_eq!(UnlockMethod::Password.id(), 0x01);
        assert_eq!(UnlockMethod::Password.method_id(&account_id), [1; 16]);
        assert_eq!(UnlockMethod::QuickUnlock(device).id(), 0x02);
        assert_eq!(
            UnlockMethod::QuickUnlock(device).method_id(&account_id),
            [2; 16]
        );
        let hw = UnlockMethod::hardware_key(b"abc");
        assert_eq!(hw.id(), 0x03);
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223 b00361a3...
        assert_eq!(
            hw.method_id(&account_id),
            hex_literal::hex!("ba7816bf8f01cfea414140de5dae2223")
        );
    }

    #[test]
    fn secret_key_rotation_via_change() {
        let acct = create_account_with_cost(&pw("pw"), Argon2Profile::Standard, TEST_COST).unwrap();
        let new_sk = SecretKey::generate().unwrap();
        let change = change_master_password_with_cost(
            &acct.account_key,
            &pw("pw"),
            &new_sk,
            &acct.params,
            acct.params.profile,
            TEST_COST,
        )
        .unwrap();
        let ak = unlock_with_cost(
            &pw("pw"),
            &new_sk,
            &change.params,
            &change.wrapped_account_key,
            TEST_COST,
        )
        .unwrap();
        assert_eq!(ak, acct.account_key);
        assert_eq!(
            unlock_with_cost(
                &pw("pw"),
                &acct.secret_key,
                &change.params,
                &change.wrapped_account_key,
                TEST_COST
            ),
            Err(Error::Decryption)
        );
    }

    /// §5.3 "profiles only ever increase": neither a password change nor a
    /// Secret Key rotation may re-wrap AK under a weaker profile. The same
    /// or a stronger profile is accepted.
    #[test]
    fn password_change_refuses_profile_downgrade() {
        let acct = create_account_with_cost(&pw("pw"), Argon2Profile::Strong, TEST_COST).unwrap();
        for weaker in [Argon2Profile::Baseline, Argon2Profile::Standard] {
            for sk in [&acct.secret_key, &SecretKey::generate().unwrap()] {
                assert_eq!(
                    change_master_password_with_cost(
                        &acct.account_key,
                        &pw("new"),
                        sk,
                        &acct.params,
                        weaker,
                        TEST_COST,
                    )
                    .unwrap_err(),
                    Error::ProfileDowngrade
                );
            }
        }
        // The public entry point checks before any Argon2id work.
        assert_eq!(
            change_master_password(
                &acct.account_key,
                &pw("new"),
                &acct.secret_key,
                &acct.params,
                Argon2Profile::Baseline,
            )
            .unwrap_err(),
            Error::ProfileDowngrade
        );
        // The check runs before derivation: an empty new password (which
        // derivation would reject with EmptyPassword) still reports the
        // downgrade, on both entry points.
        assert_eq!(
            change_master_password_with_cost(
                &acct.account_key,
                &pw(""),
                &acct.secret_key,
                &acct.params,
                Argon2Profile::Standard,
                TEST_COST,
            )
            .unwrap_err(),
            Error::ProfileDowngrade
        );
        assert_eq!(
            change_master_password(
                &acct.account_key,
                &pw(""),
                &acct.secret_key,
                &acct.params,
                Argon2Profile::Baseline,
            )
            .unwrap_err(),
            Error::ProfileDowngrade
        );
        // Without a downgrade the same empty password reaches derivation.
        assert_eq!(
            change_master_password_with_cost(
                &acct.account_key,
                &pw(""),
                &acct.secret_key,
                &acct.params,
                Argon2Profile::Strong,
                TEST_COST,
            )
            .unwrap_err(),
            Error::EmptyPassword
        );
        for same_or_stronger in [Argon2Profile::Strong, Argon2Profile::Max] {
            let change = change_master_password_with_cost(
                &acct.account_key,
                &pw("new"),
                &acct.secret_key,
                &acct.params,
                same_or_stronger,
                TEST_COST,
            )
            .unwrap();
            assert_eq!(change.params.profile, same_or_stronger);
            assert_eq!(change.params.account_id, acct.params.account_id);
        }
    }
}
