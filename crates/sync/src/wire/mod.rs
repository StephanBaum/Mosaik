//! The wire formats of the sync layer (CRYPTO_SPEC §9.2, §10, §10.1,
//! ADR-0043, ADR-0044): the folder backend's binary files, the checkpoint
//! that both backends share, the folder cursor, the strict file-name parser,
//! the email normalization shared by client and server, and the JSON types
//! of the hosted protocol.
//!
//! Every decoder of external bytes follows the same order: the total length
//! is checked against its bound first, the MAC is verified in constant time
//! over everything before the trailing 32 bytes (by [`SyncCrypto`], so
//! this crate never holds a key), and only then is the content parsed;
//! trailing bytes, unknown versions, nil IDs and IDs that differ from the
//! path are rejected. A decoder returns [`WireError`], which carries no
//! data.
//!
//! The OPAQUE ciphersuite module (`wire::opaque`, feature `opaque`) is shared
//! by the server and the HTTP client (ADR-0046).

pub mod account_file;
pub mod blob_delete;
pub mod checkpoint;
pub mod cursor;
pub mod device_info;
pub mod email;
pub mod folder_names;
pub mod json;
#[cfg(feature = "opaque")]
pub mod opaque;
pub mod segment;

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{InstallId, SecretVec};
use mosaic_vault::session::UnlockedSession;

/// Why bytes were refused. No variant carries data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WireError {
    /// The length is outside the format's bounds, bytes are missing, or
    /// bytes trail the object.
    #[error("length out of bounds")]
    Length,
    /// The magic bytes are wrong.
    #[error("wrong magic")]
    Magic,
    /// The format version is not supported.
    #[error("unsupported version")]
    Version,
    /// The MAC does not verify.
    #[error("MAC mismatch")]
    Mac,
    /// A field has a value outside its range (a flag, a kind, a count, a
    /// generation, a name, a profile).
    #[error("invalid field")]
    Field,
    /// Entries are not in the required strictly ascending order, or one
    /// repeats.
    #[error("entries out of order")]
    Order,
    /// An ID is the nil UUID.
    #[error("invalid ID")]
    Id,
    /// An ID in the object differs from the one in its path.
    #[error("ID does not match the path")]
    Path,
    /// A ciphertext inside the object is not a well-formed object of its
    /// kind.
    #[error("malformed ciphertext")]
    Ciphertext,
    /// The session could not compute or open something (a key derivation
    /// failed, or sealed bytes do not open).
    #[error("cryptographic operation failed")]
    Crypto,
}

/// The keyed operations of the wire formats. `mosaic-sync` never holds
/// `AK`: the unlocked session implements this trait with the keys derived
/// inside `mosaic-core` (CRYPTO_SPEC §10, §10.1). Verification is
/// constant-time (`subtle`, inside `mosaic-core`).
pub trait SyncCrypto {
    /// This install's stable ID, the device in every checkpoint it writes.
    fn install_id(&self) -> InstallId;

    /// The folder MAC of `msg` for `purpose`.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    fn folder_mac(&self, purpose: FolderMacPurpose, msg: &[u8]) -> Result<[u8; 32], WireError>;

    /// Whether `mac` is the folder MAC of `msg` for `purpose`.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    fn verify_folder_mac(
        &self,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool, WireError>;

    /// The checkpoint MAC of this install.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    fn checkpoint_mac(&self, counter: u64, seen_digest: &[u8; 32]) -> Result<[u8; 32], WireError>;

    /// Whether `mac` is the checkpoint MAC of `device`.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool, WireError>;

    /// Seals this install's device info.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    fn seal_device_info(&self, plaintext: &[u8]) -> Result<Vec<u8>, WireError>;

    /// Opens the device info sealed by install `device`.
    ///
    /// # Errors
    /// [`WireError::Crypto`] for a forged or damaged file.
    fn open_device_info(&self, device: InstallId, sealed: &[u8]) -> Result<SecretVec, WireError>;
}

impl SyncCrypto for UnlockedSession {
    fn install_id(&self) -> InstallId {
        Self::install_id(self)
    }

    fn folder_mac(&self, purpose: FolderMacPurpose, msg: &[u8]) -> Result<[u8; 32], WireError> {
        Self::folder_mac(self, purpose, msg).map_err(|_| WireError::Crypto)
    }

    fn verify_folder_mac(
        &self,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        Self::verify_folder_mac(self, purpose, msg, mac).map_err(|_| WireError::Crypto)
    }

    fn checkpoint_mac(&self, counter: u64, seen_digest: &[u8; 32]) -> Result<[u8; 32], WireError> {
        Self::checkpoint_mac(self, counter, seen_digest).map_err(|_| WireError::Crypto)
    }

    fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        Self::verify_checkpoint_mac(self, device, counter, seen_digest, mac)
            .map_err(|_| WireError::Crypto)
    }

    fn seal_device_info(&self, plaintext: &[u8]) -> Result<Vec<u8>, WireError> {
        Self::seal_device_info(self, plaintext).map_err(|_| WireError::Crypto)
    }

    fn open_device_info(&self, device: InstallId, sealed: &[u8]) -> Result<SecretVec, WireError> {
        Self::open_device_info(self, device, sealed).map_err(|_| WireError::Crypto)
    }
}

/// [`SyncCrypto`] over keys derived from an account key, for tests, the fuzz
/// targets and the simulations that have no unlocked session.
#[cfg(any(test, feature = "test-util"))]
#[derive(Debug)]
pub struct KeyedCrypto {
    install: InstallId,
    account: mosaic_core::AccountId,
    checkpoint: mosaic_core::CheckpointKey,
    folder: mosaic_core::FolderMacKey,
    device: mosaic_core::DeviceInfoKey,
}

#[cfg(any(test, feature = "test-util"))]
impl KeyedCrypto {
    /// The keys of `ak` for the install `install` of `account`.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    pub fn new(
        ak: &mosaic_core::AccountKey,
        account: mosaic_core::AccountId,
        install: InstallId,
    ) -> Result<Self, WireError> {
        Ok(Self {
            install,
            account,
            checkpoint: ak.checkpoint_key().map_err(|_| WireError::Crypto)?,
            folder: ak.folder_mac_key().map_err(|_| WireError::Crypto)?,
            device: ak.device_info_key().map_err(|_| WireError::Crypto)?,
        })
    }
}

#[cfg(any(test, feature = "test-util"))]
impl SyncCrypto for KeyedCrypto {
    fn install_id(&self) -> InstallId {
        self.install
    }

    fn folder_mac(&self, purpose: FolderMacPurpose, msg: &[u8]) -> Result<[u8; 32], WireError> {
        mosaic_core::sync_auth::folder_mac(&self.folder, purpose, msg)
            .map_err(|_| WireError::Crypto)
    }

    fn verify_folder_mac(
        &self,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        mosaic_core::sync_auth::verify_folder_mac(&self.folder, purpose, msg, mac)
            .map_err(|_| WireError::Crypto)
    }

    fn checkpoint_mac(&self, counter: u64, seen_digest: &[u8; 32]) -> Result<[u8; 32], WireError> {
        mosaic_core::sync_auth::checkpoint_mac(
            &self.checkpoint,
            &self.install,
            counter,
            seen_digest,
        )
        .map_err(|_| WireError::Crypto)
    }

    fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        mosaic_core::sync_auth::verify_checkpoint_mac(
            &self.checkpoint,
            &device,
            counter,
            seen_digest,
            mac,
        )
        .map_err(|_| WireError::Crypto)
    }

    fn seal_device_info(&self, plaintext: &[u8]) -> Result<Vec<u8>, WireError> {
        mosaic_core::sync_auth::seal_device_info(
            &self.device,
            &self.account,
            &self.install,
            plaintext,
        )
        .map_err(|_| WireError::Crypto)
    }

    fn open_device_info(&self, device: InstallId, sealed: &[u8]) -> Result<SecretVec, WireError> {
        mosaic_core::sync_auth::open_device_info(&self.device, &self.account, &device, sealed)
            .map_err(|_| WireError::Crypto)
    }
}

/// A cursor over input bytes that never indexes: every read is bounded and a
/// short read is [`WireError::Length`].
pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let (head, tail) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or(WireError::Length)?;
        self.rest = tail;
        Ok(*head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, WireError> {
        let [b] = self.array::<1>()?;
        Ok(b)
    }

    pub(crate) fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        let (head, tail) = self.rest.split_at_checked(n).ok_or(WireError::Length)?;
        self.rest = tail;
        Ok(head)
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Fails with [`WireError::Length`] if bytes remain.
    pub(crate) const fn finish(self) -> Result<(), WireError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(WireError::Length)
        }
    }
}

/// Splits the trailing 32-byte MAC off `bytes`.
pub(crate) fn split_mac(bytes: &[u8]) -> Result<(&[u8], [u8; 32]), WireError> {
    let (body, mac) = bytes.split_last_chunk::<32>().ok_or(WireError::Length)?;
    Ok((body, *mac))
}

/// Lowercase hex of `bytes`, for file names.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        // Writing to a `String` cannot fail.
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use mosaic_core::{Argon2Profile, MasterPassword, VaultKey, VaultMetaCiphertext};
    use mosaic_core::{AttachmentId, ItemCiphertext, ItemId, KeyGeneration, VaultId};
    use mosaic_vault::session::LockedSession;
    use mosaic_vault::storage::Store;

    use super::account_file::{AccountFile, AccountFilePath};
    use super::blob_delete::{BlobDelete, BlobDeletePath};
    use super::checkpoint::{Checkpoint, CheckpointEntry};
    use super::device_info::{DeviceInfo, Platform};
    use super::segment::{Segment, SegmentObject, SegmentPath};
    use super::*;

    /// Two accounts' unlocked sessions: the unlocked session is the
    /// `SyncCrypto` of the real client, and a second account's session
    /// verifies none of what the first one wrote.
    fn sessions() -> (UnlockedSession, UnlockedSession) {
        let account = |password: &str| {
            LockedSession::open(Store::open_in_memory().unwrap())
                .create_account(
                    &MasterPassword::new(password.to_owned()),
                    Argon2Profile::Baseline,
                )
                .unwrap()
                .0
        };
        (
            account("a password for the wire tests"),
            account("another password"),
        )
    }

    #[test]
    fn a_session_writes_segments_and_checkpoints() {
        let (session, other) = sessions();
        let install = SyncCrypto::install_id(&session);
        assert_eq!(install, session.install_id());

        // A segment.
        let account = session.account_id();
        let vault = VaultId::new_v7().unwrap();
        let vk = VaultKey::generate(KeyGeneration::FIRST).unwrap();
        let item = ItemId::new_v7().unwrap();
        let segment = Segment {
            snapshot: None,
            account_id: account,
            vault_id: vault,
            device_id: install,
            seq: 1,
            created_at_ms: 5,
            objects: vec![
                SegmentObject::Meta(VaultMetaCiphertext::seal(&vk, &vault, b"{}").unwrap()),
                SegmentObject::Item {
                    record_id: item,
                    ciphertext: ItemCiphertext::seal(&vk, &vault, &item, 1, b"x").unwrap(),
                },
            ],
        };
        let path = SegmentPath {
            account_id: account,
            vault_id: vault,
            device_id: install,
            seq: 1,
        };
        let bytes = segment.encode(&session).unwrap();
        assert_eq!(Segment::decode(&bytes, &path, &session).unwrap(), segment);
        assert_eq!(Segment::decode(&bytes, &path, &other), Err(WireError::Mac));

        // A checkpoint.
        let checkpoint = Checkpoint {
            device_id: install,
            counter: 3,
            issued_at_ms: 9,
            entries: vec![CheckpointEntry {
                vault_id: vault,
                generation: KeyGeneration::FIRST,
                high_seq: 4,
            }],
        };
        let bytes = checkpoint.encode(&session).unwrap();
        assert_eq!(
            Checkpoint::decode(&bytes, install, &session).unwrap(),
            checkpoint
        );
        assert!(checkpoint.verify(&bytes, &session).unwrap());
        assert!(!checkpoint.verify(&bytes, &other).unwrap());
    }

    #[test]
    fn a_session_writes_account_files_markers_and_device_info() {
        let (session, other) = sessions();
        let install = SyncCrypto::install_id(&session);
        let account = session.account_id();

        // An account file and a delete marker.
        let boot = session.account_bootstrap().unwrap();
        let file = AccountFile {
            account_id: account,
            account_version: 1,
            device_id: install,
            salt: boot.params.salt,
            profile: boot.params.profile,
            wrapped_ak: boot.wrapped_ak,
        };
        let file_path = AccountFilePath {
            account_id: account,
            account_version: 1,
            device_id: install,
        };
        let bytes = file.encode(&session).unwrap();
        assert_eq!(
            AccountFile::decode(&bytes, &file_path, &session).unwrap(),
            file
        );
        assert!(!AccountFile::verify_mac(&bytes, &other).unwrap());
        let marker = BlobDelete {
            attachment_id: AttachmentId::new_v7().unwrap(),
            vault_id: VaultId::new_v7().unwrap(),
            device_id: install,
            deleted_at_ms: 1,
        };
        let marker_path = BlobDeletePath {
            vault_id: marker.vault_id,
            attachment_id: marker.attachment_id,
            device_id: install,
        };
        let bytes = marker.encode(&session).unwrap();
        assert_eq!(
            BlobDelete::decode(&bytes, &marker_path, &session).unwrap(),
            marker
        );

        // Device info.
        let info = DeviceInfo {
            created_at_ms: 1,
            name: "Laptop".to_owned(),
            platform: Platform::Windows,
        };
        let sealed = info.seal(&session).unwrap();
        assert_eq!(DeviceInfo::open(&sealed, install, &session).unwrap(), info);
        assert_eq!(
            DeviceInfo::open(&sealed, install, &other),
            Err(WireError::Crypto)
        );
    }

    #[test]
    fn the_reader_is_bounded() {
        let mut r = Reader::new(&[1, 2, 3]);
        assert_eq!(r.u8(), Ok(1));
        assert_eq!(r.u16(), Ok(0x0203));
        assert_eq!(r.u8(), Err(WireError::Length));
        assert_eq!(r.remaining(), 0);
        assert_eq!(r.finish(), Ok(()));
        let mut r = Reader::new(&[0; 7]);
        assert_eq!(r.u64(), Err(WireError::Length));
        assert_eq!(r.bytes(8), Err(WireError::Length));
        assert_eq!(r.bytes(7).map(<[u8]>::len), Ok(7));
        assert_eq!(Reader::new(&[1]).finish(), Err(WireError::Length));
        assert_eq!(split_mac(&[0; 31]), Err(WireError::Length));
        assert_eq!(hex(&[0xAB, 0x01]), "ab01");
    }
}

#[cfg(test)]
mod kat_tests;
