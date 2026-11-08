//! The folder backend's re-key marker `.mrek` (CRYPTO_SPEC §10.1, ADR-0058):
//! written by the device that re-keys the account, so that devices of the
//! old epoch learn it happened and which account file to look for.
//!
//! ```text
//! "MREK" || 1 || account_id[16] || from_epoch:u32 (>= 1) || to_epoch:u32 (= from_epoch + 1)
//! || to_account_version:u64 (>= 1) || device_id[16] || created_at_ms:u64 || mac[32]   (93 bytes)
//! mac = HMAC(K_fold(from_epoch), ctx("sync/folder-rekey") || 0x00 || all preceding bytes)
//! ```
//!
//! It carries no key material. Only a device that still holds the old epoch's
//! `AK` can verify it; a device at the new epoch cannot, and ignores it. The
//! re-keying device builds the bytes inside the vault session (before the
//! commit drops the old `AK`) and keeps them in the rotation journal.

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, InstallId};

use super::{Reader, SyncCrypto, WireError, split_mac};

const MAGIC: &[u8; 4] = b"MREK";
const VERSION: u8 = 1;
/// The exact length of a marker.
pub const LEN: usize = 4 + 1 + 16 + 4 + 4 + 8 + 16 + 8 + 32;

/// The IDs of a marker as its path names them
/// (`a/<account>/account/rekey-<from_epoch:08x>-<device>.mrek`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RekeyMarkerPath {
    /// The account directory.
    pub account_id: AccountId,
    /// The epoch the marker leaves.
    pub from_epoch: u32,
    /// The device in the file name.
    pub device_id: InstallId,
}

/// A re-key marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RekeyMarker {
    /// The account.
    pub account_id: AccountId,
    /// The epoch the re-key leaves.
    pub from_epoch: u32,
    /// The epoch it moves to (always `from_epoch + 1`).
    pub to_epoch: u32,
    /// The `account_version` of the new account file.
    pub to_account_version: u64,
    /// The device that re-keyed (it wrote the new account file).
    pub device_id: InstallId,
    /// The writer's clock (not trusted for anything).
    pub created_at_ms: u64,
}

impl RekeyMarker {
    /// Encodes and MACs the marker under the keys behind `crypto`, which must
    /// be the keys of `from_epoch`. (The re-keying device does not use this:
    /// the vault builds the same bytes before the epoch changes.)
    ///
    /// # Errors
    /// [`WireError::Field`] for a zero epoch or version or a `to_epoch` other
    /// than `from_epoch + 1`; [`WireError::Crypto`].
    pub fn encode(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        self.check_fields()?;
        let mut out = Vec::with_capacity(LEN);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(self.account_id.as_bytes());
        out.extend_from_slice(&self.from_epoch.to_be_bytes());
        out.extend_from_slice(&self.to_epoch.to_be_bytes());
        out.extend_from_slice(&self.to_account_version.to_be_bytes());
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(&self.created_at_ms.to_be_bytes());
        let mac = crypto.folder_mac(FolderMacPurpose::Rekey, &out)?;
        out.extend_from_slice(&mac);
        Ok(out)
    }

    fn check_fields(&self) -> Result<(), WireError> {
        if self.from_epoch == 0
            || self.to_account_version == 0
            || self.from_epoch.checked_add(1) != Some(self.to_epoch)
        {
            return Err(WireError::Field);
        }
        Ok(())
    }

    /// Decodes a marker whose MAC verifies under the keys behind `crypto`
    /// (the keys of the epoch the marker leaves): length, MAC, then content.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Mac`] (including a marker of
    /// another epoch), [`WireError::Magic`], [`WireError::Version`],
    /// [`WireError::Field`], [`WireError::Id`], [`WireError::Path`],
    /// [`WireError::Crypto`].
    pub fn decode(
        bytes: &[u8],
        path: &RekeyMarkerPath,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        if bytes.len() != LEN {
            return Err(WireError::Length);
        }
        let (body, mac) = split_mac(bytes)?;
        if !crypto.verify_folder_mac(FolderMacPurpose::Rekey, body, &mac)? {
            return Err(WireError::Mac);
        }
        Self::parse(body, path)
    }

    fn parse(body: &[u8], path: &RekeyMarkerPath) -> Result<Self, WireError> {
        let mut r = Reader::new(body);
        if &r.array::<4>()? != MAGIC {
            return Err(WireError::Magic);
        }
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let marker = Self {
            account_id: AccountId::from_bytes(r.array()?).map_err(|_| WireError::Id)?,
            from_epoch: r.u32()?,
            to_epoch: r.u32()?,
            to_account_version: r.u64()?,
            device_id: InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?,
            created_at_ms: r.u64()?,
        };
        r.finish()?;
        marker.check_fields()?;
        if (marker.account_id, marker.from_epoch, marker.device_id)
            != (path.account_id, path.from_epoch, path.device_id)
        {
            return Err(WireError::Path);
        }
        Ok(marker)
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::AccountKey;

    use super::*;
    use crate::wire::KeyedCrypto;

    fn fixture() -> (KeyedCrypto, RekeyMarker, RekeyMarkerPath) {
        let ak = AccountKey::generate().unwrap();
        let account_id = AccountId::new_v7().unwrap();
        let device_id = InstallId::new_v7().unwrap();
        let crypto = KeyedCrypto::new(&ak, account_id, device_id).unwrap();
        let marker = RekeyMarker {
            account_id,
            from_epoch: 4,
            to_epoch: 5,
            to_account_version: 9,
            device_id,
            created_at_ms: 1_700_000_000_000,
        };
        let path = RekeyMarkerPath {
            account_id,
            from_epoch: 4,
            device_id,
        };
        (crypto, marker, path)
    }

    #[test]
    fn round_trip() {
        let (c, marker, path) = fixture();
        let bytes = marker.encode(&c).unwrap();
        assert_eq!(bytes.len(), 93);
        assert_eq!(LEN, 93);
        assert_eq!(&bytes[..5], b"MREK\x01");
        assert_eq!(RekeyMarker::decode(&bytes, &path, &c).unwrap(), marker);
    }

    #[test]
    fn mac_under_from_epoch_key_only() {
        let (c, marker, path) = fixture();
        let bytes = marker.encode(&c).unwrap();
        // The keys of the new epoch (another AK) do not verify it.
        let other = KeyedCrypto::new(
            &AccountKey::generate().unwrap(),
            marker.account_id,
            marker.device_id,
        )
        .unwrap();
        assert_eq!(
            RekeyMarker::decode(&bytes, &path, &other),
            Err(WireError::Mac)
        );
        // An account-file MAC (another purpose) is not a marker MAC.
        let mut forged = bytes[..LEN - 32].to_vec();
        let mac = c.folder_mac(FolderMacPurpose::Account, &forged).unwrap();
        forged.extend_from_slice(&mac);
        assert_eq!(RekeyMarker::decode(&forged, &path, &c), Err(WireError::Mac));
        for i in [0usize, 4, 5, 21, 25, 29, 37, 53, 60, 92] {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(RekeyMarker::decode(&bad, &path, &c), Err(WireError::Mac));
        }
    }

    #[test]
    fn path_device_mismatch_rejected() {
        let (c, marker, path) = fixture();
        let bytes = marker.encode(&c).unwrap();
        for other in [
            RekeyMarkerPath {
                device_id: InstallId::new_v7().unwrap(),
                ..path
            },
            RekeyMarkerPath {
                from_epoch: 3,
                ..path
            },
            RekeyMarkerPath {
                account_id: AccountId::new_v7().unwrap(),
                ..path
            },
        ] {
            assert_eq!(
                RekeyMarker::decode(&bytes, &other, &c),
                Err(WireError::Path)
            );
        }
    }

    #[test]
    fn trailing_bytes_rejected() {
        let (c, marker, path) = fixture();
        let bytes = marker.encode(&c).unwrap();
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            RekeyMarker::decode(&long, &path, &c),
            Err(WireError::Length)
        );
        assert_eq!(
            RekeyMarker::decode(&bytes[..LEN - 1], &path, &c),
            Err(WireError::Length)
        );
        assert_eq!(RekeyMarker::decode(&[], &path, &c), Err(WireError::Length));
    }

    #[test]
    fn fields_are_checked() {
        let (c, marker, _) = fixture();
        for bad in [
            RekeyMarker {
                from_epoch: 0,
                to_epoch: 1,
                ..marker
            },
            RekeyMarker {
                to_epoch: 6,
                ..marker
            },
            RekeyMarker {
                to_account_version: 0,
                ..marker
            },
            RekeyMarker {
                from_epoch: u32::MAX,
                to_epoch: 0,
                ..marker
            },
        ] {
            assert_eq!(bad.encode(&c), Err(WireError::Field));
        }
    }
}
