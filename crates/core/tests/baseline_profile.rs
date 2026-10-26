//! The only tests that run a real Argon2id profile (`baseline`, 128 MiB,
//! t = 3, p = 4). Everything else uses small test-only parameters.

use mosaic_core::{
    AccountId, AccountSalt, Argon2Profile, Error, ItemCiphertext, ItemId, Kek, KeyGeneration,
    MasterPassword, PasswordParams, SecretBytes, SecretKey, VaultId, VaultKey,
    change_master_password, create_account, derive_password_keys, kdf, unlock_with_password,
};

/// `KEK || OPW` under the real `baseline` profile, computed independently
/// by `tests/vectors/derive_keys_baseline.py` (argon2-cffi, i.e. the
/// reference C Argon2, plus RFC 5869 HKDF from the Python standard library).
/// This pins how `Argon2Profile::cost()` feeds the Argon2 parameters (m, t
/// and p) and the whole §5.4 chain against code we did not write.
const BASELINE_KEK_OPW: [u8; 64] = hex_literal::hex!(
    "3e4e4f5fe4306dfdda4319fbb3f4e05fc0b5432b5415ac87e6db4220f7dc7652"
    "8f8c2f5ca16bcd3d1a761a5e09116f49af349f3b2cc606d681f1a198e5cc07ad"
);

#[test]
fn baseline_derivation_matches_independent_reference() {
    let params = PasswordParams {
        account_id: AccountId::from_bytes([0x0A; 16]).unwrap(),
        salt: AccountSalt::from_bytes([0x5A; 16]),
        profile: Argon2Profile::Baseline,
    };
    let sk = SecretKey::from_secret(SecretBytes::from_array(&mut [0x0F; 16]));
    let password = MasterPassword::new("correct horse battery staple".to_string());
    let keys = derive_password_keys(&password, &sk, &params).unwrap();

    let (kek, opw) = BASELINE_KEK_OPW.split_at(32);
    let expected_kek = Kek::from_secret(SecretBytes::from_slice(kek).unwrap());
    assert_eq!(keys.kek, expected_kek, "KEK differs from the reference");
    assert_eq!(
        keys.opaque_password.expose_secret().as_slice(),
        opw,
        "OPW differs from the reference"
    );
}

#[test]
fn create_unlock_with_baseline_profile() {
    let password = MasterPassword::new("correct horse battery staple".to_string());
    let acct = create_account(&password, Argon2Profile::Baseline).unwrap();
    assert_eq!(acct.params.profile, Argon2Profile::Baseline);

    // Display string survives a round trip through the Emergency Kit.
    let shown = acct.secret_key.to_display().unwrap();
    let sk = SecretKey::parse(shown.expose_secret()).unwrap();
    assert_eq!(sk, acct.secret_key);

    let ak = unlock_with_password(&password, &sk, &acct.params, &acct.wrapped_account_key).unwrap();
    assert_eq!(ak, acct.account_key);

    // OPW is reproducible on another device from the same inputs, both
    // through the composed flow and step by step.
    let keys = derive_password_keys(&password, &sk, &acct.params).unwrap();
    assert_eq!(keys.opaque_password, acct.opaque_password);
    let prepared = kdf::prepare_password(&password).unwrap();
    let p = kdf::stretch(&prepared, &acct.params.salt, Argon2Profile::Baseline).unwrap();
    let step_keys = kdf::derive_keys(p, &sk, &acct.params.account_id).unwrap();
    assert_eq!(step_keys.kek, keys.kek);
    assert_eq!(step_keys.opaque_password, keys.opaque_password);

    // The real-clock calibration adapter measures a real derivation.
    let elapsed = kdf::measure_profile(Argon2Profile::Baseline).unwrap();
    assert!(elapsed > std::time::Duration::ZERO);

    // Master password change under the real profile (sec. 6.4).
    let new_password = MasterPassword::new("a different passphrase".to_string());
    let change = change_master_password(
        &acct.account_key,
        &new_password,
        &sk,
        &acct.params,
        acct.params.profile,
    )
    .unwrap();
    assert_eq!(
        unlock_with_password(
            &new_password,
            &sk,
            &change.params,
            &change.wrapped_account_key
        )
        .unwrap(),
        acct.account_key
    );

    // The unlocked AK opens the vault and its items.
    let vault_id = VaultId::new_v7().unwrap();
    let item_id = ItemId::new_v7().unwrap();
    let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
    let wrapped_vk = vk
        .wrap(&acct.account_key, &acct.params.account_id, &vault_id)
        .unwrap();
    let item = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, b"secret").unwrap();
    let vk2 = wrapped_vk
        .unwrap_vault_key(
            &ak,
            &acct.params.account_id,
            &vault_id,
            KeyGeneration::FIRST,
        )
        .unwrap();
    assert_eq!(
        item.open(&vk2, &vault_id, &item_id)
            .unwrap()
            .expose_secret(),
        b"secret"
    );

    // A wrong password is indistinguishable from any other failure.
    let wrong = MasterPassword::new("correct horse battery stapler".to_string());
    assert_eq!(
        unlock_with_password(&wrong, &sk, &acct.params, &acct.wrapped_account_key).unwrap_err(),
        Error::Decryption
    );
}

/// §5.3 upgrade during a §6.4 change: the new wrapping must be derived under
/// the requested (stronger) profile's real cost, not the current one's.
/// Unlocking under `change.params` re-derives with `Standard`, so a change
/// that derived with `Baseline` cost would fail here.
#[test]
#[ignore = "real Argon2id (128 + 256 MiB); run in release with --ignored"]
fn change_master_password_upgrades_profile_with_real_cost() {
    let old = MasterPassword::new("correct horse battery staple".to_string());
    let acct = create_account(&old, Argon2Profile::Baseline).unwrap();
    let new_password = MasterPassword::new("a different passphrase".to_string());
    let change = change_master_password(
        &acct.account_key,
        &new_password,
        &acct.secret_key,
        &acct.params,
        Argon2Profile::Standard,
    )
    .unwrap();
    assert_eq!(change.params.profile, Argon2Profile::Standard);
    assert_eq!(change.params.account_id, acct.params.account_id);
    let ak = unlock_with_password(
        &new_password,
        &acct.secret_key,
        &change.params,
        &change.wrapped_account_key,
    )
    .unwrap();
    assert_eq!(ak, acct.account_key);
    // The same salt under the old profile does not open the new wrapping.
    let as_baseline = PasswordParams {
        profile: Argon2Profile::Baseline,
        ..change.params
    };
    assert_eq!(
        unlock_with_password(
            &new_password,
            &acct.secret_key,
            &as_baseline,
            &change.wrapped_account_key,
        )
        .unwrap_err(),
        Error::Decryption
    );
}

/// `create_account` must derive under the requested profile's real cost:
/// another device re-deriving from the public parameters gets the same
/// `OPW`, and the profile actually changes the result.
#[test]
#[ignore = "real Argon2id (256 + 128 MiB); run in release with --ignored"]
fn create_account_uses_requested_profile_cost() {
    let password = MasterPassword::new("correct horse battery staple".to_string());
    let acct = create_account(&password, Argon2Profile::Standard).unwrap();
    assert_eq!(acct.params.profile, Argon2Profile::Standard);
    let keys = derive_password_keys(&password, &acct.secret_key, &acct.params).unwrap();
    assert_eq!(keys.opaque_password, acct.opaque_password);
    let as_baseline = PasswordParams {
        profile: Argon2Profile::Baseline,
        ..acct.params
    };
    let other = derive_password_keys(&password, &acct.secret_key, &as_baseline).unwrap();
    assert_ne!(other.opaque_password, acct.opaque_password);
}
