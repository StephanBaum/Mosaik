//! Tests of hardware-protected TOTP vaults (CRYPTO_SPEC §7.4, ADR-0028).

use mosaic_core::model::{ItemType, OtpConfig};
use mosaic_core::unlock::PrfOutput;
use mosaic_core::{
    Argon2Profile, ItemId, MasterPassword, SecretBytes, SecretKey, SecretVec, VaultId,
};

use super::{
    DEFAULT_RP_ID, Error, LockedSession, MAX_HARDWARE_KEYS, NewHardwareKey, Reauthenticated,
    UnlockedSession, VaultProtection,
};
use crate::storage::{Error as StorageError, Store, VaultKeyWrap};

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    let (s, created) = LockedSession::open(Store::open_in_memory().unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    (s, created.secret_key)
}

/// A fresh proof for `s` without the Argon2 round trip.
fn ra(s: &UnlockedSession) -> Reauthenticated {
    Reauthenticated::for_test(s.proof_id)
}

fn prf(fill: u8) -> PrfOutput {
    PrfOutput::from_secret(SecretBytes::from_array(&mut [fill; 32]))
}

fn key(fill: u8) -> NewHardwareKey {
    NewHardwareKey::new(vec![fill; 32], DEFAULT_RP_ID.to_owned(), [fill ^ 0x33; 32]).unwrap()
}

fn method_id(fill: u8) -> [u8; 16] {
    match mosaic_core::UnlockMethod::hardware_key(&[fill; 32]) {
        mosaic_core::UnlockMethod::HardwareKey(id) => id,
        _ => unreachable!(),
    }
}

fn otp() -> OtpConfig {
    OtpConfig::totp(SecretVec::from_slice(b"12345678901234567890").unwrap())
}

fn add_totp(s: &mut UnlockedSession, vault: VaultId, title: &str) -> ItemId {
    s.add_item(vault, &ItemType::Totp, |e| {
        e.set_title(title)?;
        e.set_otp(&otp())
    })
    .unwrap()
}

/// An account with a hardware-protected vault (key 1, PRF 0xA1).
fn with_hw_vault() -> (UnlockedSession, SecretKey, VaultId) {
    let (mut s, sk) = account();
    let v = s
        .create_hardware_vault("Bank codes", &key(1), &prf(0xA1))
        .unwrap();
    (s, sk, v)
}

#[test]
fn hardware_vault_lifecycle() {
    let (mut s, sk, v) = with_hw_vault();
    assert_eq!(
        s.vault_protection(v).unwrap(),
        VaultProtection::HardwareKey { open: true }
    );
    assert_eq!(
        s.store.vault(&v).unwrap().unwrap().key_wrap,
        VaultKeyWrap::TotpKey
    );
    let item = add_totp(&mut s, v, "Bank 2FA");
    assert!(s.item(v, item).unwrap().otp().unwrap().hardware_protected);
    assert_eq!(s.search("bank").len(), 1);

    s.close_hardware_vault(v).unwrap();
    s.close_hardware_vault(v).unwrap();
    assert_eq!(
        s.vault_protection(v).unwrap(),
        VaultProtection::HardwareKey { open: false }
    );
    assert!(matches!(s.item(v, item), Err(Error::UnknownItem)));
    assert_eq!(s.search("bank").len(), 0);
    let summary = s.vaults().into_iter().find(|x| x.vault_id == v).unwrap();
    assert!(!summary.available);
    assert!(summary.name.is_none());
    assert!(matches!(
        s.add_item(v, &ItemType::Totp, |_| Ok(())),
        Err(Error::VaultUnavailable)
    ));
    assert_eq!(s.hardware_vault_keys(v).unwrap().len(), 1);

    s.open_hardware_vault(v, &[1; 32], &prf(0xA1)).unwrap();
    assert!(s.item(v, item).unwrap().otp().unwrap().hardware_protected);
    assert_eq!(s.search("bank").len(), 1);
    let summary = s.vaults().into_iter().find(|x| x.vault_id == v).unwrap();
    assert!(summary.available);
    assert_eq!(summary.name, Some("Bank codes"));
    // Opening an open vault re-checks the key and is otherwise a no-op.
    s.open_hardware_vault(v, &[1; 32], &prf(0xA1)).unwrap();
    assert!(matches!(
        s.open_hardware_vault(v, &[1; 32], &prf(0xA2)),
        Err(Error::HardwareKeyRejected)
    ));
    assert_eq!(s.search("bank").len(), 1);

    // A code can be read while it is open.
    let code = s.otp_code(v, item, 59).unwrap();
    assert_eq!(code.code.expose_secret().len(), 6);

    // A lock drops everything; the next unlock lists it closed.
    let s = s.lock().unlock(&pw(), &sk).unwrap();
    assert_eq!(
        s.vault_protection(v).unwrap(),
        VaultProtection::HardwareKey { open: false }
    );
    assert!(matches!(s.item(v, item), Err(Error::UnknownItem)));
}

#[test]
fn hardware_vault_wrong_key() {
    let (mut s, _, v) = with_hw_vault();
    s.close_hardware_vault(v).unwrap();
    assert!(matches!(
        s.open_hardware_vault(v, &[2; 32], &prf(0xA1)),
        Err(Error::UnknownHardwareKey)
    ));
    assert!(matches!(
        s.open_hardware_vault(v, &[1; 32], &prf(0xA2)),
        Err(Error::HardwareKeyRejected)
    ));
    assert!(matches!(
        s.open_hardware_vault(v, &[1; 8], &prf(0xA1)),
        Err(Error::InvalidHardwareKey)
    ));
    assert_eq!(
        s.vault_protection(v).unwrap(),
        VaultProtection::HardwareKey { open: false }
    );
    // Registering the same key for unlock (own salt and label) leaves the
    // vault wrapping alone.
    let unlock_key =
        NewHardwareKey::new(vec![1; 32], DEFAULT_RP_ID.to_owned(), [0x77; 32]).unwrap();
    s.enroll_hardware_key(&unlock_key, &prf(0xA1), &ra(&s))
        .unwrap();
    s.open_hardware_vault(v, &[1; 32], &prf(0xA1)).unwrap();

    // Operations on a vault with the other protection.
    let normal = s.create_vault("Personal").unwrap();
    assert_eq!(
        s.vault_protection(normal).unwrap(),
        VaultProtection::AccountKey
    );
    assert!(matches!(
        s.open_hardware_vault(normal, &[1; 32], &prf(0xA1)),
        Err(Error::WrongVaultProtection)
    ));
    assert!(matches!(
        s.close_hardware_vault(normal),
        Err(Error::WrongVaultProtection)
    ));
    assert!(matches!(
        s.hardware_vault_keys(normal),
        Err(Error::WrongVaultProtection)
    ));
    assert!(matches!(
        s.add_hardware_vault_key(normal, &key(5), &prf(5)),
        Err(Error::WrongVaultProtection)
    ));
    let unknown = VaultId::new_v7().unwrap();
    assert!(matches!(
        s.vault_protection(unknown),
        Err(Error::UnknownVault)
    ));
    assert!(matches!(
        s.close_hardware_vault(unknown),
        Err(Error::UnknownVault)
    ));
}

#[test]
fn hardware_vault_backup_key() {
    let (mut s, _, v) = with_hw_vault();
    let second = s.add_hardware_vault_key(v, &key(2), &prf(0xA2)).unwrap();
    assert_eq!(second.method_id, method_id(2));
    assert_eq!(second.rp_id, DEFAULT_RP_ID);
    assert_eq!(s.hardware_vault_keys(v).unwrap().len(), 2);

    s.close_hardware_vault(v).unwrap();
    s.open_hardware_vault(v, &[2; 32], &prf(0xA2)).unwrap();

    // Remove the first; it no longer opens the vault, TK is unchanged.
    assert!(s.remove_hardware_vault_key(v, &method_id(1)).unwrap());
    assert!(!s.remove_hardware_vault_key(v, &method_id(1)).unwrap());
    // The last key cannot be removed.
    assert!(matches!(
        s.remove_hardware_vault_key(v, &method_id(2)),
        Err(Error::LastHardwareKey)
    ));
    assert_eq!(s.hardware_vault_keys(v).unwrap().len(), 1);
    // Removing while closed needs a key first.
    s.close_hardware_vault(v).unwrap();
    assert!(matches!(
        s.remove_hardware_vault_key(v, &method_id(2)),
        Err(Error::VaultUnavailable)
    ));
    assert!(matches!(
        s.add_hardware_vault_key(v, &key(3), &prf(3)),
        Err(Error::VaultUnavailable)
    ));
    assert!(matches!(
        s.open_hardware_vault(v, &[1; 32], &prf(0xA1)),
        Err(Error::UnknownHardwareKey)
    ));
    s.open_hardware_vault(v, &[2; 32], &prf(0xA2)).unwrap();

    // At most eight keys; replacing one does not count.
    for fill in 3..=u8::try_from(MAX_HARDWARE_KEYS).unwrap() {
        s.add_hardware_vault_key(v, &key(fill), &prf(fill)).unwrap();
    }
    assert_eq!(
        s.hardware_vault_keys(v).unwrap().len(),
        MAX_HARDWARE_KEYS - 1
    );
    s.add_hardware_vault_key(v, &key(9), &prf(9)).unwrap();
    assert_eq!(s.hardware_vault_keys(v).unwrap().len(), MAX_HARDWARE_KEYS);
    assert!(matches!(
        s.add_hardware_vault_key(v, &key(10), &prf(10)),
        Err(Error::HardwareKeyLimit)
    ));
    s.add_hardware_vault_key(v, &key(3), &prf(3)).unwrap();
    // A key edited after construction is checked again.
    let mut bad = key(11);
    bad.rp_id = "has space".to_owned();
    assert!(matches!(
        s.add_hardware_vault_key(v, &bad, &prf(11)),
        Err(Error::InvalidHardwareKey)
    ));
}

#[test]
fn hardware_vault_rotation() {
    let (mut s, _, v) = with_hw_vault();
    let item = add_totp(&mut s, v, "Before rotation");
    let g2 = s.rotate_vault_key(v).unwrap();
    assert_eq!(g2.get(), 2);

    // The new key is wrapped under TK, never under AK.
    let tk = s.vaults.get(&v).unwrap().tk.as_ref().unwrap();
    let keys = s.store.vault_keys(&v).unwrap();
    assert_eq!(keys.len(), 2);
    let (_, wrapped) = keys.iter().find(|(g, _)| *g == g2).unwrap();
    wrapped.unwrap_vault_key_under_tk(tk, &v, g2).unwrap();
    assert!(
        wrapped
            .unwrap_vault_key(&s.account_key, &s.account_id, &v, g2)
            .is_err()
    );

    // Records were re-sealed: they survive a close and reopen.
    s.close_hardware_vault(v).unwrap();
    assert!(matches!(
        s.rotate_vault_key(v),
        Err(Error::VaultUnavailable)
    ));
    s.open_hardware_vault(v, &[1; 32], &prf(0xA1)).unwrap();
    assert!(s.item(v, item).is_ok());
    assert_eq!(
        s.vaults()
            .iter()
            .find(|x| x.vault_id == v)
            .unwrap()
            .key_generation,
        g2
    );
}

#[test]
fn hardware_vault_only_totp() {
    let (mut s, _, hw) = with_hw_vault();
    let normal = s.create_vault("Personal").unwrap();

    // A login is refused before the closure runs.
    let mut ran = false;
    let r = s.add_item(hw, &ItemType::Login, |e| {
        ran = true;
        e.set_title("nope")
    });
    assert!(matches!(r, Err(Error::ItemTypeNotAllowed)));
    assert!(!ran);
    let login = s
        .add_item(normal, &ItemType::Login, |e| e.set_title("login"))
        .unwrap();
    assert!(matches!(
        s.move_item(normal, login, hw),
        Err(Error::ItemTypeNotAllowed)
    ));
    assert!(s.item(normal, login).is_ok_and(|i| !i.is_deleted()));

    // A TOTP item keeps the flag in step with its vault.
    let totp = add_totp(&mut s, normal, "codes");
    assert!(
        !s.item(normal, totp)
            .unwrap()
            .otp()
            .unwrap()
            .hardware_protected
    );
    let moved = s.move_item(normal, totp, hw).unwrap();
    assert!(s.item(hw, moved).unwrap().otp().unwrap().hardware_protected);

    // An edit that sets the flag to false inside the vault is corrected.
    s.update_item(hw, moved, |e| {
        let mut c = otp();
        c.hardware_protected = false;
        e.set_otp(&c)
    })
    .unwrap();
    assert!(s.item(hw, moved).unwrap().otp().unwrap().hardware_protected);

    // And one that sets it outside is corrected too.
    let plain = add_totp(&mut s, normal, "plain");
    s.update_item(normal, plain, |e| {
        let mut c = otp();
        c.hardware_protected = true;
        e.set_otp(&c)
    })
    .unwrap();
    assert!(
        !s.item(normal, plain)
            .unwrap()
            .otp()
            .unwrap()
            .hardware_protected
    );

    let back = s.move_item(hw, moved, normal).unwrap();
    assert!(
        !s.item(normal, back)
            .unwrap()
            .otp()
            .unwrap()
            .hardware_protected
    );

    // A closed vault takes nothing and gives nothing.
    let again = s.move_item(normal, back, hw).unwrap();
    s.close_hardware_vault(hw).unwrap();
    assert!(matches!(
        s.move_item(normal, plain, hw),
        Err(Error::VaultUnavailable)
    ));
    assert!(matches!(
        s.move_item(hw, again, normal),
        Err(Error::VaultUnavailable)
    ));
}

#[test]
fn tk_never_under_ak() {
    let (mut s, _, v) = with_hw_vault();
    s.rotate_vault_key(v).unwrap();
    for (generation, wrapped) in s.store.vault_keys(&v).unwrap() {
        assert!(
            wrapped
                .unwrap_vault_key(&s.account_key, &s.account_id, &v, generation)
                .is_err()
        );
    }
    let normal = s.create_vault("Personal").unwrap();
    assert_eq!(
        s.store.vault(&normal).unwrap().unwrap().key_wrap,
        VaultKeyWrap::AccountKey
    );
    // Sync of TK vault keys is M6: an AK-wrapped key is refused.
    let vk = mosaic_core::VaultKey::generate(mosaic_core::KeyGeneration::new(3).unwrap()).unwrap();
    let wrapped = vk.wrap(&s.account_key, &s.account_id, &v).unwrap();
    assert!(matches!(
        s.apply_remote_vault_key(v, mosaic_core::KeyGeneration::new(3).unwrap(), &wrapped),
        Err(Error::VaultUnavailable)
    ));
}

#[test]
fn create_tk_vault_is_atomic() {
    let (mut s, _) = account();
    let v = VaultId::new_v7().unwrap();
    let tk = mosaic_core::unlock::TotpVaultKey::generate().unwrap();
    let vk = mosaic_core::VaultKey::generate(mosaic_core::KeyGeneration::FIRST).unwrap();
    let vk_wrapping = vk.wrap_under_tk(&tk, &v).unwrap();
    let w = mosaic_core::unlock::prf_hw_vault_key(&prf(1), &[1; 32]).unwrap();
    let tk_wrapping_ct = tk.wrap(&w, &s.account_id, &v, &method_id(1)).unwrap();
    let meta = mosaic_core::model::VaultMeta::new("n", s.device, super::now()).unwrap();
    let meta_ct = meta.seal(&vk, &v).unwrap();
    let too_big = vec![0u8; crate::storage::MAX_UNLOCK_PARAMS_LEN + 1];
    assert!(matches!(
        s.store.create_tk_vault(
            &v,
            mosaic_core::KeyGeneration::FIRST,
            &vk_wrapping,
            &method_id(1),
            &tk_wrapping_ct,
            &too_big,
            &meta_ct,
        ),
        Err(StorageError::UnlockParamsTooLarge)
    ));
    assert!(s.store.vault(&v).unwrap().is_none());
    s.store
        .create_tk_vault(
            &v,
            mosaic_core::KeyGeneration::FIRST,
            &vk_wrapping,
            &method_id(1),
            &tk_wrapping_ct,
            &[1],
            &meta_ct,
        )
        .unwrap();
    assert_eq!(s.store.tk_wrappings(&v).unwrap().len(), 1);
    // The metadata is stored by the same transaction.
    assert!(s.store.vault_meta(&v).unwrap().is_some());
    assert!(matches!(
        s.store.create_tk_vault(
            &v,
            mosaic_core::KeyGeneration::FIRST,
            &vk_wrapping,
            &method_id(1),
            &tk_wrapping_ct,
            &[1],
            &meta_ct,
        ),
        Err(StorageError::VaultExists)
    ));
}
