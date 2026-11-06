//! The folder backend's blob delete marker `.mdel` (CRYPTO_SPEC §10.1,
//! ADR-0043): "this attachment was deleted", written next to the blob and
//! kept for 30 days before the blob is purged.
//!
//! ```text
//! "MDEL" || 1 || attachment_id[16] || device_id[16] || deleted_at_ms:u64 || mac[32]
//! mac = HMAC(K_fold, ctx("sync/folder-blob-delete") || 0x00 || all preceding bytes)
//! ```
//!
//! The layout adds up to 77 bytes; the M6 spec's "93" does not match its own
//! field list, and the field list is what this module implements.

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AttachmentId, InstallId};

use super::{Reader, SyncCrypto, WireError, split_mac};

const MAGIC: &[u8; 4] = b"MDEL";
const VERSION: u8 = 1;
/// The exact length of a delete marker.
pub const LEN: usize = 4 + 1 + 16 + 16 + 8 + 32;

/// A blob delete marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobDelete {
    /// The deleted attachment (the marker's file name).
    pub attachment_id: AttachmentId,
    /// The install that deleted it.
    pub device_id: InstallId,
    /// When (ms since the Unix epoch).
    pub deleted_at_ms: u64,
}

impl BlobDelete {
    /// Encodes and MACs the marker.
    ///
    /// # Errors
    /// [`WireError::Crypto`].
    pub fn encode(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        let mut out = Vec::with_capacity(LEN);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(self.attachment_id.as_bytes());
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(&self.deleted_at_ms.to_be_bytes());
        let mac = crypto.folder_mac(FolderMacPurpose::BlobDelete, &out)?;
        out.extend_from_slice(&mac);
        Ok(out)
    }

    /// Decodes a marker: length, MAC, then content; the attachment ID must
    /// be the one in the file name.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Mac`], [`WireError::Magic`],
    /// [`WireError::Version`], [`WireError::Id`], [`WireError::Path`],
    /// [`WireError::Crypto`].
    pub fn decode(
        bytes: &[u8],
        path_attachment: AttachmentId,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        if bytes.len() != LEN {
            return Err(WireError::Length);
        }
        let (body, mac) = split_mac(bytes)?;
        if !crypto.verify_folder_mac(FolderMacPurpose::BlobDelete, body, &mac)? {
            return Err(WireError::Mac);
        }
        let mut r = Reader::new(body);
        if &r.array::<4>()? != MAGIC {
            return Err(WireError::Magic);
        }
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let attachment_id = AttachmentId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let device_id = InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        let deleted_at_ms = r.u64()?;
        r.finish()?;
        if attachment_id != path_attachment {
            return Err(WireError::Path);
        }
        Ok(Self {
            attachment_id,
            device_id,
            deleted_at_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountId, AccountKey};

    use super::*;
    use crate::wire::KeyedCrypto;

    fn fixture() -> (KeyedCrypto, BlobDelete) {
        let ak = AccountKey::generate().unwrap();
        let device_id = InstallId::new_v7().unwrap();
        let c = KeyedCrypto::new(&ak, AccountId::new_v7().unwrap(), device_id).unwrap();
        (
            c,
            BlobDelete {
                attachment_id: AttachmentId::new_v7().unwrap(),
                device_id,
                deleted_at_ms: 1_700_000_000_000,
            },
        )
    }

    #[test]
    fn round_trip_and_rejections() {
        let (c, m) = fixture();
        let bytes = m.encode(&c).unwrap();
        assert_eq!(bytes.len(), LEN);
        assert_eq!(&bytes[..5], b"MDEL\x01");
        assert_eq!(BlobDelete::decode(&bytes, m.attachment_id, &c).unwrap(), m);
        for i in 0..bytes.len() {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(
                BlobDelete::decode(&bad, m.attachment_id, &c),
                Err(WireError::Mac),
                "byte {i}"
            );
        }
        assert_eq!(
            BlobDelete::decode(&[], m.attachment_id, &c),
            Err(WireError::Length)
        );
        assert_eq!(
            BlobDelete::decode(&bytes[..LEN - 1], m.attachment_id, &c),
            Err(WireError::Length)
        );
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            BlobDelete::decode(&long, m.attachment_id, &c),
            Err(WireError::Length)
        );
        assert_eq!(
            BlobDelete::decode(&bytes, AttachmentId::new_v7().unwrap(), &c),
            Err(WireError::Path)
        );
        let (other, _) = fixture();
        assert_eq!(
            BlobDelete::decode(&bytes, m.attachment_id, &other),
            Err(WireError::Mac)
        );
    }

    #[test]
    fn the_mac_is_bound_to_its_purpose() {
        // A MAC over the same bytes for another purpose (a segment, an
        // account file) is not a delete marker's.
        let (c, m) = fixture();
        let mut bytes = m.encode(&c).unwrap();
        let body = bytes[..LEN - 32].to_vec();
        let mac = c.folder_mac(FolderMacPurpose::Segment, &body).unwrap();
        bytes[LEN - 32..].copy_from_slice(&mac);
        assert_eq!(
            BlobDelete::decode(&bytes, m.attachment_id, &c),
            Err(WireError::Mac)
        );
    }

    #[test]
    fn nil_ids_are_refused() {
        let (c, m) = fixture();
        let bytes = m.encode(&c).unwrap();
        for at in [5usize, 21] {
            let mut body = bytes[..LEN - 32].to_vec();
            body[at..at + 16].fill(0);
            let mac = c.folder_mac(FolderMacPurpose::BlobDelete, &body).unwrap();
            body.extend_from_slice(&mac);
            assert_eq!(
                BlobDelete::decode(&body, m.attachment_id, &c),
                Err(WireError::Id)
            );
        }
    }
}
