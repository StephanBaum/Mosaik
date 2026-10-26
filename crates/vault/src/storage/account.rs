//! The account row and the wrappings of AK per unlock method (CRYPTO_SPEC
//! §5.3, §6).

use std::collections::HashSet;

use mosaic_core::{
    AccountId, AccountSalt, Argon2Profile, DeviceId, KeyGeneration, PasswordParams, UnlockMethod,
    VaultId, WrappedKey,
};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
use super::vaults::{VaultKeyWrap, require_vault, rewrap_key};
use super::{MAX_UNLOCK_PARAMS_LEN, Store, now_ms};

/// A stored wrapping of the account key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredUnlockMethod {
    /// The unlock method; its byte and `method_id` are the row's key.
    pub method: UnlockMethod,
    /// AK wrapped under the method's `W` (§6).
    pub wrapped_account_key: WrappedKey,
    /// The method's public parameters, for example the §6.3 PRF salt or
    /// the §6.2 Windows Hello challenge. Empty for the password method.
    pub params: Vec<u8>,
    /// When the row was last written (ms since the Unix epoch).
    pub updated_at_ms: i64,
}

/// A quick-unlock or hardware-key wrapping of AK to store, for
/// [`AccountRekey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewUnlockMethod {
    /// The unlock method (not [`UnlockMethod::Password`]).
    pub method: UnlockMethod,
    /// The new AK wrapped under the method's `W` (§6).
    pub wrapped_account_key: WrappedKey,
    /// The method's public parameters (at most
    /// [`super::MAX_UNLOCK_PARAMS_LEN`] bytes).
    pub params: Vec<u8>,
}

/// Everything a §6.6 full re-key changes in the store, applied by
/// [`Store::rekey_account`] in one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountRekey {
    /// The new password parameters: the same `account_id`, a new salt, a
    /// profile no weaker than the stored one.
    pub params: PasswordParams,
    /// The new AK wrapped under the new KEK.
    pub password_wrapping: WrappedKey,
    /// The complete new set of the other unlock methods. Every other stored
    /// wrapping of the old AK is deleted (§6.6 step 5).
    pub unlock_methods: Vec<NewUnlockMethod>,
    /// Every vault key the store holds under AK, of every generation,
    /// re-wrapped under the new AK. Keys under a TK (§7.4) are not wrapped
    /// under AK and are left alone.
    pub vault_keys: Vec<(VaultId, KeyGeneration, WrappedKey)>,
}

fn check_params(params: &PasswordParams, stored: &PasswordParams) -> Result<()> {
    if stored.account_id != params.account_id {
        return Err(Error::AccountMismatch);
    }
    if params.profile < stored.profile {
        return Err(Error::ProfileDowngrade);
    }
    Ok(())
}

fn update_account(conn: &Connection, params: &PasswordParams) -> Result<()> {
    conn.execute(
        "UPDATE account SET salt = ?1, profile = ?2, updated_at = ?3 WHERE id = 1",
        params![params.salt.as_bytes(), params.profile.id(), now_ms()],
    )?;
    Ok(())
}

fn check_other_method(method: &UnlockMethod, params: &[u8]) -> Result<()> {
    if matches!(method, UnlockMethod::Password) {
        return Err(Error::PasswordMethodReserved);
    }
    if params.len() > MAX_UNLOCK_PARAMS_LEN {
        return Err(Error::UnlockParamsTooLarge);
    }
    Ok(())
}

fn read_account(conn: &Connection) -> Result<Option<PasswordParams>> {
    let row: Option<(Vec<u8>, Vec<u8>, i64)> = conn
        .query_row(
            "SELECT account_id, salt, profile FROM account WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((account_id, salt, profile)) = row else {
        return Ok(None);
    };
    let account_id = AccountId::from_slice(&account_id).map_err(|_| Error::Corrupt)?;
    let salt: [u8; 16] = salt.try_into().map_err(|_| Error::Corrupt)?;
    let profile = u8::try_from(profile)
        .ok()
        .and_then(|p| Argon2Profile::from_id(p).ok())
        .ok_or(Error::Corrupt)?;
    Ok(Some(PasswordParams {
        account_id,
        salt: AccountSalt::from_bytes(salt),
        profile,
    }))
}

fn decode_method(account_id: &AccountId, method: i64, method_id: &[u8]) -> Result<UnlockMethod> {
    let id: [u8; 16] = method_id.try_into().map_err(|_| Error::Corrupt)?;
    match method {
        0x01 if id == *account_id.as_bytes() => Ok(UnlockMethod::Password),
        0x02 => Ok(UnlockMethod::QuickUnlock(
            DeviceId::from_bytes(id).map_err(|_| Error::Corrupt)?,
        )),
        0x03 => Ok(UnlockMethod::HardwareKey(id)),
        _ => Err(Error::Corrupt),
    }
}

type UnlockRow = (i64, Vec<u8>, Vec<u8>, Vec<u8>, i64);

fn decode_unlock(account_id: &AccountId, row: UnlockRow) -> Result<StoredUnlockMethod> {
    let (method, method_id, wrapped, params, updated_at_ms) = row;
    Ok(StoredUnlockMethod {
        method: decode_method(account_id, method, &method_id)?,
        wrapped_account_key: WrappedKey::from_bytes(&wrapped).map_err(|_| Error::Corrupt)?,
        params,
        updated_at_ms,
    })
}

fn upsert_unlock(
    conn: &Connection,
    account_id: &AccountId,
    method: &UnlockMethod,
    wrapped: &WrappedKey,
    params: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT INTO unlock_methods (method, method_id, wrapped_ak, params, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (method, method_id) DO UPDATE SET
             wrapped_ak = excluded.wrapped_ak,
             params = excluded.params,
             updated_at = excluded.updated_at",
        params![
            method.id(),
            method.method_id(account_id),
            wrapped.as_bytes(),
            params,
            now_ms()
        ],
    )?;
    Ok(())
}

impl Store {
    /// The stored account parameters, or `None` before
    /// [`Store::init_account`].
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn account(&self) -> Result<Option<PasswordParams>> {
        read_account(&self.conn)
    }

    /// Stores a new account: its public parameters and the password
    /// wrapping of AK (§6.1), in one transaction.
    ///
    /// # Errors
    /// [`Error::AccountExists`] if the store already has an account.
    pub fn init_account(
        &mut self,
        params: &PasswordParams,
        password_wrapping: &WrappedKey,
    ) -> Result<()> {
        let tx = self.write()?;
        if read_account(&tx)?.is_some() {
            return Err(Error::AccountExists);
        }
        tx.execute(
            "INSERT INTO account (id, account_id, salt, profile, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                params.account_id.as_bytes(),
                params.salt.as_bytes(),
                params.profile.id(),
                now_ms()
            ],
        )?;
        upsert_unlock(
            &tx,
            &params.account_id,
            &UnlockMethod::Password,
            password_wrapping,
            &[],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Replaces the account parameters and the password wrapping of AK
    /// together, as a master password change or profile upgrade needs
    /// (§5.3, §6.4). The old wrapping is scrubbed from the WAL before this
    /// returns.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::AccountMismatch`] for another
    /// account's parameters, [`Error::ProfileDowngrade`] for a weaker
    /// profile.
    pub fn update_password(
        &mut self,
        params: &PasswordParams,
        password_wrapping: &WrappedKey,
    ) -> Result<()> {
        let tx = self.write()?;
        let stored = read_account(&tx)?.ok_or(Error::NoAccount)?;
        check_params(params, &stored)?;
        update_account(&tx, params)?;
        upsert_unlock(
            &tx,
            &params.account_id,
            &UnlockMethod::Password,
            password_wrapping,
            &[],
        )?;
        tx.commit()?;
        self.scrub_wal()
    }

    /// Applies a §6.6 full re-key in one transaction: the new password
    /// parameters and wrapping, exactly the given other unlock methods
    /// (every other old wrapping is deleted), and the re-wrap of every
    /// vault key held under AK, of every generation. Either all of it is
    /// stored or nothing is, so a crash can never leave vault keys under
    /// one AK and the unlock methods under another. Afterwards the old
    /// wrappings are scrubbed from the WAL.
    ///
    /// New vault-key generations (§6.6 step 2) are accepted afterwards
    /// with [`Store::accept_vault_key`], wrapped under the new AK.
    ///
    /// # Errors
    /// [`Error::NoAccount`], [`Error::AccountMismatch`],
    /// [`Error::ProfileDowngrade`], [`Error::PasswordMethodReserved`] if
    /// `unlock_methods` holds the password method,
    /// [`Error::UnlockParamsTooLarge`], [`Error::UnknownVault`],
    /// [`Error::UnknownVaultKey`] for a key that is not held,
    /// [`Error::KeyWrapMismatch`] for a key of a vault under TK,
    /// [`Error::RekeyIncomplete`] if a vault key held under AK is missing
    /// from `vault_keys`.
    pub fn rekey_account(&mut self, rekey: &AccountRekey) -> Result<()> {
        for m in &rekey.unlock_methods {
            check_other_method(&m.method, &m.params)?;
        }
        let tx = self.write()?;
        let stored = read_account(&tx)?.ok_or(Error::NoAccount)?;
        check_params(&rekey.params, &stored)?;
        update_account(&tx, &rekey.params)?;
        tx.execute("DELETE FROM unlock_methods", [])?;
        let account_id = &rekey.params.account_id;
        upsert_unlock(
            &tx,
            account_id,
            &UnlockMethod::Password,
            &rekey.password_wrapping,
            &[],
        )?;
        for m in &rekey.unlock_methods {
            upsert_unlock(
                &tx,
                account_id,
                &m.method,
                &m.wrapped_account_key,
                &m.params,
            )?;
        }
        let mut covered = HashSet::new();
        for (vault_id, generation, wrapped) in &rekey.vault_keys {
            if require_vault(&tx, vault_id)?.key_wrap != VaultKeyWrap::AccountKey {
                return Err(Error::KeyWrapMismatch);
            }
            if !rewrap_key(&tx, vault_id, *generation, wrapped)? {
                return Err(Error::UnknownVaultKey);
            }
            covered.insert((*vault_id, *generation));
        }
        let held: i64 = tx.query_row(
            "SELECT count(*) FROM vault_keys JOIN vaults USING (vault_id) WHERE key_wrap = 1",
            [],
            |r| r.get(0),
        )?;
        if usize::try_from(held).ok() != Some(covered.len()) {
            return Err(Error::RekeyIncomplete);
        }
        tx.commit()?;
        self.scrub_wal()
    }

    /// Stores (or replaces) the wrapping of AK for a quick-unlock or
    /// hardware-key method, with that method's public parameters.
    ///
    /// # Errors
    /// [`Error::PasswordMethodReserved`] for [`UnlockMethod::Password`],
    /// [`Error::UnlockParamsTooLarge`], [`Error::NoAccount`].
    pub fn put_unlock_method(
        &mut self,
        method: &UnlockMethod,
        wrapped_account_key: &WrappedKey,
        params: &[u8],
    ) -> Result<()> {
        check_other_method(method, params)?;
        let tx = self.write()?;
        let account = read_account(&tx)?.ok_or(Error::NoAccount)?;
        upsert_unlock(
            &tx,
            &account.account_id,
            method,
            wrapped_account_key,
            params,
        )?;
        tx.commit()?;
        // A replaced wrapping must not linger in the WAL.
        self.scrub_wal()
    }

    /// The stored wrapping for `method`, if any.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn unlock_method(&self, method: &UnlockMethod) -> Result<Option<StoredUnlockMethod>> {
        let Some(account) = read_account(&self.conn)? else {
            return Ok(None);
        };
        let row: Option<UnlockRow> = self
            .conn
            .query_row(
                "SELECT method, method_id, wrapped_ak, params, updated_at FROM unlock_methods
                 WHERE method = ?1 AND method_id = ?2",
                params![method.id(), method.method_id(&account.account_id)],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        row.map(|row| decode_unlock(&account.account_id, row))
            .transpose()
    }

    /// Every stored wrapping of AK, ordered by method and `method_id`.
    ///
    /// # Errors
    /// [`Error::Corrupt`], [`Error::Database`].
    pub fn unlock_methods(&self) -> Result<Vec<StoredUnlockMethod>> {
        let Some(account) = read_account(&self.conn)? else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(
            "SELECT method, method_id, wrapped_ak, params, updated_at FROM unlock_methods
             ORDER BY method, method_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.map(|row| decode_unlock(&account.account_id, row?))
            .collect()
    }

    /// Deletes the wrapping for a quick-unlock or hardware-key method, for
    /// example when quick unlock expires (§6.2). Returns whether a row was
    /// deleted.
    ///
    /// # Errors
    /// [`Error::PasswordMethodReserved`] for [`UnlockMethod::Password`].
    pub fn remove_unlock_method(&mut self, method: &UnlockMethod) -> Result<bool> {
        if matches!(method, UnlockMethod::Password) {
            return Err(Error::PasswordMethodReserved);
        }
        let tx = self.write()?;
        let Some(account) = read_account(&tx)? else {
            return Ok(false);
        };
        let n = tx.execute(
            "DELETE FROM unlock_methods WHERE method = ?1 AND method_id = ?2",
            params![method.id(), method.method_id(&account.account_id)],
        )?;
        tx.commit()?;
        // §6.2: expiry deletes the wrapping, including its WAL frames.
        self.scrub_wal()?;
        Ok(n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_core::{AccountKey, Kek, SecretBytes};

    fn params(profile: Argon2Profile) -> PasswordParams {
        PasswordParams {
            account_id: AccountId::new_v7().unwrap(),
            salt: AccountSalt::generate().unwrap(),
            profile,
        }
    }

    fn wrapping(p: &PasswordParams, method: &UnlockMethod) -> (AccountKey, Kek, WrappedKey) {
        let ak = AccountKey::generate().unwrap();
        let w = Kek::from_secret(SecretBytes::random().unwrap());
        let wrapped = ak.wrap(&w, &p.account_id, method).unwrap();
        (ak, w, wrapped)
    }

    #[test]
    fn account_lifecycle() {
        let mut s = Store::open_in_memory().unwrap();
        assert_eq!(s.account().unwrap(), None);
        assert_eq!(s.unlock_methods().unwrap().len(), 0);
        assert_eq!(s.unlock_method(&UnlockMethod::Password).unwrap(), None);
        let p = params(Argon2Profile::Standard);
        let (ak, w, wrapped) = wrapping(&p, &UnlockMethod::Password);

        assert!(matches!(
            s.update_password(&p, &wrapped),
            Err(Error::NoAccount)
        ));
        s.init_account(&p, &wrapped).unwrap();
        assert!(matches!(
            s.init_account(&p, &wrapped),
            Err(Error::AccountExists)
        ));
        assert_eq!(s.account().unwrap(), Some(p));

        let stored = s.unlock_method(&UnlockMethod::Password).unwrap().unwrap();
        assert_eq!(stored.method, UnlockMethod::Password);
        assert_eq!(stored.params.len(), 0);
        let back = stored
            .wrapped_account_key
            .unwrap_account_key(&w, &p.account_id, &UnlockMethod::Password)
            .unwrap();
        assert_eq!(back, ak);

        // Password change: new salt, same profile; a stronger profile is
        // an upgrade; weaker is refused; another account is refused.
        let mut changed = p;
        changed.salt = AccountSalt::generate().unwrap();
        let (_, _, wrapped2) = wrapping(&changed, &UnlockMethod::Password);
        s.update_password(&changed, &wrapped2).unwrap();
        assert_eq!(s.account().unwrap(), Some(changed));
        let stored = s.unlock_method(&UnlockMethod::Password).unwrap().unwrap();
        assert_eq!(stored.wrapped_account_key, wrapped2);

        changed.profile = Argon2Profile::Strong;
        s.update_password(&changed, &wrapped2).unwrap();
        let mut weaker = changed;
        weaker.profile = Argon2Profile::Baseline;
        assert!(matches!(
            s.update_password(&weaker, &wrapped2),
            Err(Error::ProfileDowngrade)
        ));
        let other = params(Argon2Profile::Max);
        assert!(matches!(
            s.update_password(&other, &wrapped2),
            Err(Error::AccountMismatch)
        ));
        assert_eq!(s.account().unwrap(), Some(changed));
    }

    #[test]
    fn other_unlock_methods() {
        let mut s = Store::open_in_memory().unwrap();
        let p = params(Argon2Profile::Baseline);
        let device = UnlockMethod::QuickUnlock(DeviceId::new_v7().unwrap());
        let hw = UnlockMethod::hardware_key(b"credential");
        let (_, _, wrapped_q) = wrapping(&p, &device);
        assert!(matches!(
            s.put_unlock_method(&device, &wrapped_q, &[]),
            Err(Error::NoAccount)
        ));
        assert!(!s.remove_unlock_method(&device).unwrap());

        let (_, _, wrapped_p) = wrapping(&p, &UnlockMethod::Password);
        s.init_account(&p, &wrapped_p).unwrap();
        s.put_unlock_method(&device, &wrapped_q, &[]).unwrap();
        let (_, _, wrapped_h) = wrapping(&p, &hw);
        s.put_unlock_method(&hw, &wrapped_h, &[0xAB; 32]).unwrap();
        assert!(matches!(
            s.put_unlock_method(&hw, &wrapped_h, &[0; MAX_UNLOCK_PARAMS_LEN + 1]),
            Err(Error::UnlockParamsTooLarge)
        ));
        assert!(matches!(
            s.put_unlock_method(&UnlockMethod::Password, &wrapped_h, &[]),
            Err(Error::PasswordMethodReserved)
        ));
        assert!(matches!(
            s.remove_unlock_method(&UnlockMethod::Password),
            Err(Error::PasswordMethodReserved)
        ));

        let all = s.unlock_methods().unwrap();
        let methods: Vec<_> = all.iter().map(|m| m.method).collect();
        assert_eq!(methods, [UnlockMethod::Password, device, hw]);
        let h = s.unlock_method(&hw).unwrap().unwrap();
        assert_eq!(h.params, [0xAB; 32]);
        assert_eq!(h.wrapped_account_key, wrapped_h);

        // Replacing keeps one row per method.
        s.put_unlock_method(&hw, &wrapped_q, &[1]).unwrap();
        assert_eq!(s.unlock_methods().unwrap().len(), 3);
        assert_eq!(s.unlock_method(&hw).unwrap().unwrap().params, [1]);

        assert!(s.remove_unlock_method(&device).unwrap());
        assert!(!s.remove_unlock_method(&device).unwrap());
        assert_eq!(s.unlock_method(&device).unwrap(), None);
    }

    /// §6.6: a full re-key swaps every AK wrapping and re-wraps every held
    /// vault key, of every generation, in one transaction. Any problem
    /// leaves the store exactly as it was, so a failure (or crash) half-way
    /// can never strand vault keys under the old AK.
    #[test]
    #[allow(clippy::too_many_lines, reason = "one scenario, step by step")]
    fn rekey_is_complete_and_atomic() {
        use super::super::{PendingPolicy, VaultKeyWrap};
        use mosaic_core::{KeyGeneration, VaultId, VaultKey};

        let g = |n| KeyGeneration::new(n).unwrap();
        let mut s = Store::open_in_memory().unwrap();
        let p = params(Argon2Profile::Standard);
        let (ak, _, password) = wrapping(&p, &UnlockMethod::Password);
        let rekey = |s: &mut Store, r: &AccountRekey| s.rekey_account(r);
        // No account yet.
        let empty = AccountRekey {
            params: p,
            password_wrapping: password.clone(),
            unlock_methods: vec![],
            vault_keys: vec![],
        };
        assert!(matches!(rekey(&mut s, &empty), Err(Error::NoAccount)));
        s.init_account(&p, &password).unwrap();
        let quick = UnlockMethod::QuickUnlock(DeviceId::new_v7().unwrap());
        let hw = UnlockMethod::hardware_key(b"key");
        let (_, _, wq) = wrapping(&p, &quick);
        let (_, _, wh) = wrapping(&p, &hw);
        s.put_unlock_method(&quick, &wq, &[]).unwrap();
        s.put_unlock_method(&hw, &wh, &[7; 32]).unwrap();

        // A vault holding generations 1 and 2, and a TOTP vault under TK.
        let vault = VaultId::new_v7().unwrap();
        let vk1 = VaultKey::generate(g(1)).unwrap();
        let vk2 = VaultKey::generate(g(2)).unwrap();
        s.create_vault(&vault, g(1), &vk1.wrap(&ak, &p.account_id, &vault).unwrap())
            .unwrap();
        s.accept_vault_key(&vault, g(2), &vk2.wrap(&ak, &p.account_id, &vault).unwrap())
            .unwrap();
        let totp = VaultId::new_v7().unwrap();
        let under_tk = vk1.wrap(&ak, &p.account_id, &totp).unwrap();
        s.create_vault_with(&totp, g(1), VaultKeyWrap::TotpKey, &under_tk)
            .unwrap();
        // A vault that was deleted holds no keys and needs none.
        let gone = VaultId::new_v7().unwrap();
        s.create_vault(&gone, g(1), &vk1.wrap(&ak, &p.account_id, &gone).unwrap())
            .unwrap();
        s.delete_vault(&gone, PendingPolicy::Refuse).unwrap();

        let snapshot = |s: &Store| {
            (
                s.account().unwrap(),
                s.unlock_methods().unwrap(),
                s.vault_keys(&vault).unwrap(),
                s.vault_keys(&totp).unwrap(),
            )
        };
        let before = snapshot(&s);

        // The new AK and its wrappings.
        let ak2 = AccountKey::generate().unwrap();
        let kek2 = Kek::from_secret(SecretBytes::random().unwrap());
        let mut p2 = p;
        p2.salt = AccountSalt::generate().unwrap();
        let hw_params = vec![8; 32];
        let full = AccountRekey {
            params: p2,
            password_wrapping: ak2
                .wrap(&kek2, &p.account_id, &UnlockMethod::Password)
                .unwrap(),
            unlock_methods: vec![NewUnlockMethod {
                method: hw,
                wrapped_account_key: ak2.wrap(&kek2, &p.account_id, &hw).unwrap(),
                params: hw_params.clone(),
            }],
            vault_keys: vec![
                (vault, g(1), vk1.wrap(&ak2, &p.account_id, &vault).unwrap()),
                (vault, g(2), vk2.wrap(&ak2, &p.account_id, &vault).unwrap()),
            ],
        };

        // Every failure leaves the store untouched, even when it is found
        // after earlier keys were already re-wrapped in the transaction.
        let mut missing_old = full.clone();
        missing_old.vault_keys.remove(0);
        let mut not_held = full.clone();
        not_held
            .vault_keys
            .push((vault, g(3), vk2.wrap(&ak2, &p.account_id, &vault).unwrap()));
        let mut tk_key = full.clone();
        tk_key.vault_keys.push((totp, g(1), under_tk.clone()));
        let mut unknown_vault = full.clone();
        unknown_vault
            .vault_keys
            .push((gone, g(1), vk1.wrap(&ak2, &p.account_id, &gone).unwrap()));
        let mut with_password = full.clone();
        with_password.unlock_methods.push(NewUnlockMethod {
            method: UnlockMethod::Password,
            wrapped_account_key: full.password_wrapping.clone(),
            params: vec![],
        });
        let mut big_params = full.clone();
        big_params.unlock_methods[0].params = vec![0; MAX_UNLOCK_PARAMS_LEN + 1];
        let mut other_account = full.clone();
        other_account.params = params(Argon2Profile::Max);
        let mut weaker = full.clone();
        weaker.params.profile = Argon2Profile::Baseline;
        for (r, check) in [
            (
                missing_old,
                (|e| matches!(e, Error::RekeyIncomplete)) as fn(&Error) -> bool,
            ),
            (not_held, |e| matches!(e, Error::UnknownVaultKey)),
            (tk_key, |e| matches!(e, Error::KeyWrapMismatch)),
            (unknown_vault, |e| matches!(e, Error::UnknownVault)),
            (with_password, |e| {
                matches!(e, Error::PasswordMethodReserved)
            }),
            (big_params, |e| matches!(e, Error::UnlockParamsTooLarge)),
            (other_account, |e| matches!(e, Error::AccountMismatch)),
            (weaker, |e| matches!(e, Error::ProfileDowngrade)),
        ] {
            let e = rekey(&mut s, &r).unwrap_err();
            assert!(check(&e), "{e:?}");
            assert_eq!(snapshot(&s), before);
        }

        rekey(&mut s, &full).unwrap();
        assert_eq!(s.account().unwrap(), Some(p2));
        // Exactly the new methods remain: quick unlock is gone (§6.6 step 5).
        let methods = s.unlock_methods().unwrap();
        assert_eq!(
            methods.iter().map(|m| m.method).collect::<Vec<_>>(),
            [UnlockMethod::Password, hw]
        );
        assert_eq!(methods[1].params, hw_params);
        let back = methods[0]
            .wrapped_account_key
            .unwrap_account_key(&kek2, &p.account_id, &UnlockMethod::Password)
            .unwrap();
        assert_eq!(back, ak2);
        for (generation, vk) in [(g(1), &vk1), (g(2), &vk2)] {
            let unwrapped = s
                .vault_key(&vault, generation)
                .unwrap()
                .unwrap()
                .unwrap_vault_key(&ak2, &p.account_id, &vault, generation)
                .unwrap();
            assert_eq!(&unwrapped, vk);
        }
        // The TK-wrapped key is not touched.
        assert_eq!(s.vault_key(&totp, g(1)).unwrap(), Some(under_tk));
    }

    #[test]
    fn corrupt_rows_are_reported() {
        let mut s = Store::open_in_memory().unwrap();
        let p = params(Argon2Profile::Baseline);
        let (_, _, wrapped) = wrapping(&p, &UnlockMethod::Password);
        s.init_account(&p, &wrapped).unwrap();
        // A password row whose method_id is not the account ID, and a quick
        // unlock row with the nil device ID.
        s.conn
            .execute(
                "UPDATE unlock_methods SET method_id = zeroblob(16) WHERE method = 1",
                [],
            )
            .unwrap();
        assert!(matches!(s.unlock_methods(), Err(Error::Corrupt)));
        s.conn
            .execute("UPDATE unlock_methods SET method = 2 WHERE method = 1", [])
            .unwrap();
        assert!(matches!(s.unlock_methods(), Err(Error::Corrupt)));
        s.conn
            .execute(
                "UPDATE unlock_methods SET method = 3, wrapped_ak = zeroblob(74)",
                [],
            )
            .unwrap();
        assert!(matches!(s.unlock_methods(), Err(Error::Corrupt)));

        assert!(matches!(
            decode_method(&p.account_id, 9, &[1; 16]),
            Err(Error::Corrupt)
        ));
        assert!(matches!(
            decode_method(&p.account_id, 3, &[1; 15]),
            Err(Error::Corrupt)
        ));
    }
}
