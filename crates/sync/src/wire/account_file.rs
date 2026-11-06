//! The folder backend's account file `.macct` (CRYPTO_SPEC §10.1,
//! ADR-0043): the account's public parameters and the password wrapping of
//! `AK`, versioned for master password changes.
//!
//! ```text
//! "MACC" || 1 || account_id[16] || account_version:u64 (>= 1) || device_id[16]
//! || salt[16] || profile:u8 (1..=4) || wrapped_ak[74] || mac[32]      (168 bytes)
//! mac = HMAC(K_fold, ctx("sync/folder-account") || 0x00 || all preceding bytes)
//! ```
//!
//! A joining device cannot check the MAC before it holds `AK`: it uses the
//! file only to attempt the join ([`AccountFile::decode_for_join`]; the AEAD
//! of `wrapped_ak` authenticates the parameters) and checks the MAC right
//! after unlocking ([`AccountFile::verify_mac`]).

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, AccountSalt, Argon2Profile, InstallId, WrappedKey};

use super::{Reader, SyncCrypto, WireError, split_mac};

const MAGIC: &[u8; 4] = b"MACC";
const VERSION: u8 = 1;
/// The exact length of an account file.
pub const LEN: usize = 4 + 1 + 16 + 8 + 16 + 16 + 1 + WrappedKey::LEN + 32;

/// The IDs of an account file as its path names them
/// (`a/<account>/account/<version:016x>-<device>.macct`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountFilePath {
    /// The account directory.
    pub account_id: AccountId,
    /// The version in the file name.
    pub account_version: u64,
    /// The device in the file name.
    pub device_id: InstallId,
}

/// An account file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountFile {
    /// The account.
    pub account_id: AccountId,
    /// Increases with every master password change (at least 1).
    pub account_version: u64,
    /// The install that wrote the file.
    pub device_id: InstallId,
    /// The Argon2id salt.
    pub salt: AccountSalt,
    /// The Argon2id profile.
    pub profile: Argon2Profile,
    /// `AK` wrapped under the password `KEK` (CRYPTO_SPEC §6.1).
    pub wrapped_ak: WrappedKey,
}

impl AccountFile {
    /// Encodes and MACs the file.
    ///
    /// # Errors
    /// [`WireError::Field`] for a version of 0; [`WireError::Crypto`].
    pub fn encode(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        if self.account_version == 0 {
            return Err(WireError::Field);
        }
        let mut out = Vec::with_capacity(LEN);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(self.account_id.as_bytes());
        out.extend_from_slice(&self.account_version.to_be_bytes());
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(self.salt.as_bytes());
        out.push(self.profile.id());
        out.extend_from_slice(self.wrapped_ak.as_bytes());
        let mac = crypto.folder_mac(FolderMacPurpose::Account, &out)?;
        out.extend_from_slice(&mac);
        Ok(out)
    }

    /// Decodes a file whose MAC can be checked: length, MAC, then content.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Mac`], [`WireError::Magic`],
    /// [`WireError::Version`], [`WireError::Field`], [`WireError::Id`],
    /// [`WireError::Path`], [`WireError::Crypto`].
    pub fn decode(
        bytes: &[u8],
        path: &AccountFilePath,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        if !Self::verify_mac(bytes, crypto)? {
            return Err(WireError::Mac);
        }
        Self::decode_for_join(bytes, path)
    }

    /// Decodes the content without the MAC, for a device that joins and has
    /// no key yet. Whoever wrote the file chose its content, so the result
    /// is only good for an unlock attempt: the wrapped key's AEAD
    /// authenticates the parameters, and [`AccountFile::verify_mac`] must
    /// pass right after the unlock.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Magic`], [`WireError::Version`],
    /// [`WireError::Field`], [`WireError::Id`], [`WireError::Path`].
    pub fn decode_for_join(bytes: &[u8], path: &AccountFilePath) -> Result<Self, WireError> {
        if bytes.len() != LEN {
            return Err(WireError::Length);
        }
        let (body, _) = split_mac(bytes)?;
        let mut r = Reader::new(body);
        if &r.array::<4>()? != MAGIC {
            return Err(WireError::Magic);
        }
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let account_id = AccountId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let account_version = r.u64()?;
        let device_id = InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let salt = AccountSalt::from_bytes(r.array()?);
        let profile = Argon2Profile::from_id(r.u8()?).map_err(|_| WireError::Field)?;
        let wrapped_ak =
            WrappedKey::from_bytes(r.bytes(WrappedKey::LEN)?).map_err(|_| WireError::Ciphertext)?;
        r.finish()?;
        if account_version == 0 {
            return Err(WireError::Field);
        }
        if (account_id, account_version, device_id)
            != (path.account_id, path.account_version, path.device_id)
        {
            return Err(WireError::Path);
        }
        Ok(Self {
            account_id,
            account_version,
            device_id,
            salt,
            profile,
            wrapped_ak,
        })
    }

    /// Whether the file carries a valid MAC (constant time).
    ///
    /// # Errors
    /// [`WireError::Length`] for a file that is not [`LEN`] bytes;
    /// [`WireError::Crypto`].
    pub fn verify_mac(bytes: &[u8], crypto: &impl SyncCrypto) -> Result<bool, WireError> {
        if bytes.len() != LEN {
            return Err(WireError::Length);
        }
        let (body, mac) = split_mac(bytes)?;
        crypto.verify_folder_mac(FolderMacPurpose::Account, body, &mac)
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountKey, Kek, UnlockMethod};

    use super::*;
    use crate::wire::KeyedCrypto;

    fn fixture() -> (KeyedCrypto, AccountFile, AccountFilePath) {
        let ak = AccountKey::generate().unwrap();
        let account_id = AccountId::new_v7().unwrap();
        let device_id = InstallId::new_v7().unwrap();
        let crypto = KeyedCrypto::new(&ak, account_id, device_id).unwrap();
        let wrapped_ak = ak
            .wrap(
                &Kek::generate().unwrap(),
                &account_id,
                &UnlockMethod::Password,
            )
            .unwrap();
        let file = AccountFile {
            account_id,
            account_version: 3,
            device_id,
            salt: AccountSalt::from_bytes([7; 16]),
            profile: Argon2Profile::Baseline,
            wrapped_ak,
        };
        let path = AccountFilePath {
            account_id,
            account_version: 3,
            device_id,
        };
        (crypto, file, path)
    }

    #[test]
    fn round_trip_and_length() {
        let (c, file, path) = fixture();
        let bytes = file.encode(&c).unwrap();
        assert_eq!(bytes.len(), 168);
        assert_eq!(LEN, 168);
        assert_eq!(&bytes[..5], b"MACC\x01");
        assert_eq!(AccountFile::decode(&bytes, &path, &c).unwrap(), file);
        assert_eq!(AccountFile::decode_for_join(&bytes, &path).unwrap(), file);
        assert!(AccountFile::verify_mac(&bytes, &c).unwrap());
    }

    #[test]
    fn rejections() {
        let (c, file, path) = fixture();
        let bytes = file.encode(&c).unwrap();
        for i in [0usize, 4, 5, 21, 29, 45, 61, 62, 100, 167] {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(
                AccountFile::decode(&bad, &path, &c),
                Err(WireError::Mac),
                "{i}"
            );
            assert!(!AccountFile::verify_mac(&bad, &c).unwrap());
        }
        for len in [0usize, 167] {
            assert_eq!(
                AccountFile::decode(&bytes[..len], &path, &c),
                Err(WireError::Length)
            );
            assert_eq!(
                AccountFile::decode_for_join(&bytes[..len], &path),
                Err(WireError::Length)
            );
        }
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            AccountFile::decode_for_join(&long, &path),
            Err(WireError::Length)
        );
        // The shape checks of a join (no MAC): magic, version, profile, nil
        // IDs, version 0, path.
        let join = |b: &[u8]| AccountFile::decode_for_join(b, &path);
        let mut x = bytes.clone();
        x[0] = b'X';
        assert_eq!(join(&x), Err(WireError::Magic));
        let mut x = bytes.clone();
        x[4] = 2;
        assert_eq!(join(&x), Err(WireError::Version));
        let mut x = bytes.clone();
        x[61] = 0;
        assert_eq!(join(&x), Err(WireError::Field));
        x[61] = 5;
        assert_eq!(join(&x), Err(WireError::Field));
        let mut x = bytes.clone();
        x[5..21].fill(0);
        assert_eq!(join(&x), Err(WireError::Id));
        let mut x = bytes.clone();
        x[29..45].fill(0);
        assert_eq!(join(&x), Err(WireError::Id));
        let mut x = bytes.clone();
        x[21..29].fill(0);
        assert_eq!(join(&x), Err(WireError::Field));
        for other in [
            AccountFilePath {
                account_version: 4,
                ..path
            },
            AccountFilePath {
                device_id: InstallId::new_v7().unwrap(),
                ..path
            },
            AccountFilePath {
                account_id: AccountId::new_v7().unwrap(),
                ..path
            },
        ] {
            assert_eq!(
                AccountFile::decode_for_join(&bytes, &other),
                Err(WireError::Path)
            );
        }
        // Version 0 cannot be written either.
        let mut zero = file.clone();
        zero.account_version = 0;
        assert_eq!(zero.encode(&c), Err(WireError::Field));
    }
}
