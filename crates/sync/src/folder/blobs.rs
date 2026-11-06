//! Attachment blobs and their delete markers (ADR-0043): a blob is
//! write-once; every install writes and removes only its own `.mdel`.

use std::collections::BTreeMap;

use mosaic_core::{AttachmentId, InstallId, VaultId};
use mosaic_vault::storage::validate_attachment_ciphertext;

use super::{FolderTransport, fsutil};
use crate::transport::{RemoteAttachment, TransportError};
use crate::wire::SyncCrypto;
use crate::wire::blob_delete::{self, BlobDelete, BlobDeletePath};
use crate::wire::folder_names::{self, FolderName};

/// The most a blob file may hold (the largest attachment ciphertext plus the
/// STREAM overhead, rounded up).
const MAX_BLOB_LEN: u64 = 52 * 1024 * 1024;

impl<C: SyncCrypto + Send> FolderTransport<C> {
    pub(super) fn folder_put_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
        ciphertext: &[u8],
    ) -> Result<(), TransportError> {
        validate_attachment_ciphertext(ciphertext).map_err(|_| TransportError::Protocol)?;
        let path = self.layout.blob_path(&vault, &id);
        if fsutil::exists(&path) {
            // Identical content for an ID everywhere: nothing to write.
            return Ok(());
        }
        fsutil::write_atomic(
            &self.layout.blobs_dir(&vault),
            &folder_names::blob_name(id.as_bytes()),
            ciphertext,
            false,
        )
    }

    pub(super) fn folder_get_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<Option<Vec<u8>>, TransportError> {
        let Some(bytes) = fsutil::read_capped(&self.layout.blob_path(&vault, &id), MAX_BLOB_LEN)?
        else {
            return Ok(None);
        };
        // A partial upload does not validate: not there yet.
        Ok(validate_attachment_ciphertext(&bytes).ok().map(|()| bytes))
    }

    pub(super) fn folder_delete_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), TransportError> {
        let marker = BlobDelete {
            attachment_id: id,
            vault_id: vault,
            device_id: self.install,
            deleted_at_ms: self.now_ms(),
        };
        let bytes = marker
            .encode(&self.crypto)
            .map_err(|_| TransportError::Protocol)?;
        fsutil::write_atomic(
            &self.layout.blobs_dir(&vault),
            &folder_names::blob_delete_name(id.as_bytes(), self.install.as_bytes()),
            &bytes,
            true,
        )
    }

    /// Removes this install's own marker. `true` if the blob is there.
    pub(super) fn folder_restore_attachment(
        &mut self,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<bool, TransportError> {
        fsutil::remove(&self.layout.marker_path(&vault, &id, &self.install))?;
        Ok(fsutil::exists(&self.layout.blob_path(&vault, &id)))
    }

    /// The verified markers of a vault: attachment to the deleting devices
    /// and when.
    pub(crate) fn verified_markers(
        &self,
        vault: &VaultId,
    ) -> Result<BTreeMap<AttachmentId, Vec<(InstallId, u64)>>, TransportError> {
        let mut out: BTreeMap<AttachmentId, Vec<(InstallId, u64)>> = BTreeMap::new();
        for entry in fsutil::list(&self.layout.blobs_dir(vault))? {
            let Some(FolderName::BlobDelete { attachment, device }) =
                folder_names::parse(&entry.name)
            else {
                continue;
            };
            let (Ok(attachment_id), Ok(device_id)) = (
                AttachmentId::from_bytes(attachment),
                InstallId::from_bytes(device),
            ) else {
                continue;
            };
            let path = self
                .layout
                .blobs_dir(vault)
                .join(folder_names::blob_delete_name(&attachment, &device));
            let Ok(Some(bytes)) = fsutil::read_capped(&path, blob_delete::LEN as u64) else {
                continue;
            };
            let expected = BlobDeletePath {
                vault_id: *vault,
                attachment_id,
                device_id,
            };
            if let Ok(marker) = BlobDelete::decode(&bytes, &expected, &self.crypto) {
                out.entry(attachment_id)
                    .or_default()
                    .push((device_id, marker.deleted_at_ms));
            }
        }
        Ok(out)
    }

    pub(super) fn folder_attachments(
        &mut self,
        vault: VaultId,
    ) -> Result<Vec<RemoteAttachment>, TransportError> {
        let markers = self.verified_markers(&vault)?;
        let mut out = Vec::new();
        for entry in fsutil::list(&self.layout.blobs_dir(&vault))? {
            if entry.is_dir {
                continue;
            }
            let Some(FolderName::Blob(id)) = folder_names::parse(&entry.name) else {
                continue;
            };
            let Ok(attachment_id) = AttachmentId::from_bytes(id) else {
                continue;
            };
            let deleted_at_ms = markers
                .get(&attachment_id)
                .and_then(|m| m.iter().map(|(_, t)| *t).max());
            out.push(RemoteAttachment {
                attachment_id,
                size: entry.len,
                deleted_at_ms,
            });
        }
        Ok(out)
    }
}
