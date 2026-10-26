//! The account row and the wrappings of AK per unlock method (CRYPTO_SPEC
//! §5.3, §6).

use mosaic_core::{
    AccountId, AccountSalt, Argon2Profile, DeviceId, PasswordParams, UnlockMethod, WrappedKey,
};
use rusqlite::{Connection, OptionalExtension, params};

use super::error::{Error, Result};
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
    /// (§5.3, §6.4).
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
        if stored.account_id != params.account_id {
            return Err(Error::AccountMismatch);
        }
        if params.profile < stored.profile {
            return Err(Error::ProfileDowngrade);
        }
        tx.execute(
            "UPDATE account SET salt = ?1, profile = ?2, updated_at = ?3 WHERE id = 1",
            params![params.salt.as_bytes(), params.profile.id(), now_ms()],
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
        if matches!(method, UnlockMethod::Password) {
            return Err(Error::PasswordMethodReserved);
        }
        if params.len() > MAX_UNLOCK_PARAMS_LEN {
            return Err(Error::UnlockParamsTooLarge);
        }
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
        Ok(())
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
