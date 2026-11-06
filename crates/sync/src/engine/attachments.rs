//! Attachments (M6 spec §5.6, M2-R10, M3-R25): upload, deletions,
//! re-assertion by the holder and the download of missing files.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{AttachmentId, ItemId, VaultId};
use mosaic_vault::session::{AttachmentFileState, AttachmentRef, Error};
use mosaic_vault::storage::Error as StorageError;

use super::consts::ATTACHMENT_DOWNLOAD_BUDGET;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{RemoteAttachment, Transport};

/// Whether the store simply has no file (an in-memory store, a row without
/// its file): the attachment is skipped, not an error.
const fn is_skippable(e: &Error) -> bool {
    matches!(
        e,
        Error::Store(
            StorageError::AttachmentsUnavailable
                | StorageError::UnknownAttachment
                | StorageError::Corrupt
        )
    )
}

impl<T: Transport> SyncEngine<T> {
    pub(super) fn attachments_phase(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        self.upload_attachments(access, report)?;
        self.send_attachment_deletions(access)?;
        self.reassert_attachments(access)?;
        self.fetch_attachments(access, report)
    }

    fn upload_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let (pending, queued) = access.with(|s| -> Result<_, Error> {
            let pending = s.pending_attachment_uploads()?;
            let mut queued: BTreeSet<(VaultId, ItemId)> = BTreeSet::new();
            for vault in s.synced_vaults() {
                for record in s
                    .pending_uploads_in(vault, usize::MAX)?
                    .into_iter()
                    .flatten()
                {
                    queued.insert((vault, record.record_id));
                }
            }
            Ok((pending, queued))
        })??;
        for info in pending {
            // The record that refers to the file has to be on the backend
            // first.
            if queued.contains(&(info.vault_id, info.record_id)) {
                continue;
            }
            let id = info.attachment_id;
            let ciphertext = match access.with(|s| s.attachment_ciphertext(id))? {
                Ok(c) => c,
                Err(e) if is_skippable(&e) => continue,
                Err(e) => return Err(e.into()),
            };
            self.transport
                .put_attachment(info.vault_id, id, &ciphertext)?;
            access.with(|s| s.mark_attachment_uploaded(id))??;
            report.attachments_up += 1;
        }
        Ok(())
    }

    fn send_attachment_deletions(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(), SyncError> {
        let deletions = access.with(|s| s.pending_attachment_deletions())??;
        for (vault, id) in deletions {
            self.transport.delete_attachment(vault, id)?;
            access.with(|s| s.mark_attachment_deletion_sent(id))??;
        }
        Ok(())
    }

    /// Uploads a local file again (the backend lost it).
    fn put_from_local(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        id: AttachmentId,
    ) -> Result<(), SyncError> {
        match access.with(|s| s.attachment_ciphertext(id))? {
            Ok(ciphertext) => {
                self.transport.put_attachment(vault, id, &ciphertext)?;
                Ok(())
            }
            Err(e) if is_skippable(&e) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// For every attachment an item still refers to: a delete marker is
    /// undone (this device's own), and a file the backend lost is
    /// uploaded again from the local copy.
    fn reassert_attachments(&mut self, access: &mut impl SessionAccess) -> Result<(), SyncError> {
        let refs = access.with(|s| s.attachment_references())?;
        let mut by_vault: BTreeMap<VaultId, Vec<AttachmentRef>> = BTreeMap::new();
        for r in refs {
            by_vault.entry(r.vault_id).or_default().push(r);
        }
        for (vault, refs) in by_vault {
            let remote = self.transport.attachments(vault)?;
            for r in refs {
                self.reassert_one(access, vault, &r, &remote)?;
            }
        }
        Ok(())
    }

    /// Re-asserts one referenced attachment against the backend's list.
    fn reassert_one(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        r: &AttachmentRef,
        remote: &[RemoteAttachment],
    ) -> Result<(), SyncError> {
        let id = r.attachment_id;
        let entry = remote.iter().find(|a| a.attachment_id == id);
        let local = access.with(|s| s.attachment_file_state(id))??;
        let has_local = matches!(local, AttachmentFileState::Present { .. });
        match entry {
            Some(e) if e.deleted_at_ms.is_some() => {
                if !self.transport.restore_attachment(vault, id)? && has_local {
                    self.put_from_local(access, vault, id)?;
                }
            }
            None if matches!(
                local,
                AttachmentFileState::Present {
                    upload_pending: false
                }
            ) =>
            {
                self.put_from_local(access, vault, id)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn fetch_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let missing: Vec<AttachmentRef> = access.with(|s| -> Result<_, Error> {
            let mut out = Vec::new();
            for r in s.attachment_references() {
                if s.attachment_file_state(r.attachment_id)? == AttachmentFileState::Missing {
                    out.push(r);
                }
            }
            Ok(out)
        })??;
        let mut budget = ATTACHMENT_DOWNLOAD_BUDGET;
        for r in missing {
            if budget == 0 {
                break;
            }
            let Some(ciphertext) = self.transport.get_attachment(r.vault_id, r.attachment_id)?
            else {
                continue;
            };
            budget = budget.saturating_sub(u64::try_from(ciphertext.len()).unwrap_or(u64::MAX));
            let stored = access.with(|s| {
                s.store_remote_attachment(r.vault_id, r.item_id, r.attachment_id, &ciphertext)
            })?;
            match stored {
                Ok(()) => report.attachments_down += 1,
                Err(Error::Store(StorageError::AttachmentsUnavailable)) => break,
                // A file that is not a valid attachment, or one already
                // stored: nothing to do now.
                Err(Error::Store(_) | Error::UnknownAttachmentReference) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
