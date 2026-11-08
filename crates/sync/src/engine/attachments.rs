//! Attachments (M6 spec §5.6, M2-R10, M3-R25): upload, deletions,
//! re-assertion by the holder and the download of missing files.

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::{AttachmentId, ItemId, VaultId};
use mosaic_vault::session::{AttachmentFileState, AttachmentRef, Error};
use mosaic_vault::storage::Error as StorageError;

use super::consts::ATTACHMENT_DOWNLOAD_BUDGET;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::transport::{Rejection, RemoteAttachment, Transport, TransportError};

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
        // Downloads wait for a rolled-back vault and skip one the server
        // refuses this device; uploads also skip a vault this account may
        // only read (M6b).
        let mut skip = Self::rolled_back_vaults(access)?;
        skip.extend(access.with(|s| {
            s.synced_vaults()
                .into_iter()
                .filter(|v| s.vault_access_lost(*v).unwrap_or(false))
                .collect::<Vec<_>>()
        })?);
        let write_skip = Self::write_blocked_vaults(access)?;
        self.upload_attachments(access, report, &write_skip)?;
        self.send_attachment_deletions(access, &write_skip)?;
        self.reassert_attachments(access, report, &write_skip)?;
        self.fetch_attachments(access, report, &skip)
    }

    fn upload_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        skip: &BTreeSet<VaultId>,
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
            if queued.contains(&(info.vault_id, info.record_id)) || skip.contains(&info.vault_id) {
                continue;
            }
            let id = info.attachment_id;
            let ciphertext = match access.with(|s| s.attachment_ciphertext(id))? {
                Ok(c) => c,
                Err(e) if is_skippable(&e) => continue,
                Err(e) => return Err(e.into()),
            };
            match self
                .transport
                .put_attachment(info.vault_id, id, &ciphertext)
            {
                Ok(()) => {}
                // A role that only reads: the file stays queued.
                Err(TransportError::Rejected(Rejection::RoleForbidden)) => continue,
                Err(e) => return Err(e.into()),
            }
            access.with(|s| s.mark_attachment_uploaded(id))??;
            report.attachments_up += 1;
        }
        Ok(())
    }

    fn send_attachment_deletions(
        &mut self,
        access: &mut impl SessionAccess,
        skip: &BTreeSet<VaultId>,
    ) -> Result<(), SyncError> {
        let deletions = access.with(|s| s.pending_attachment_deletions())??;
        // The files a rotation replaced are deleted only after the records
        // that refer to the new ones were accepted: a vault with records
        // still queued waits (CRYPTO_SPEC §8.3).
        let queued: BTreeSet<VaultId> = access.with(|s| -> Result<_, Error> {
            let mut out = BTreeSet::new();
            for vault in s.synced_vaults() {
                if !s.pending_uploads_in(vault, 1)?.is_empty() {
                    out.insert(vault);
                }
            }
            Ok(out)
        })??;
        for (vault, id) in deletions {
            if skip.contains(&vault) || queued.contains(&vault) {
                continue;
            }
            match self.transport.delete_attachment(vault, id) {
                Ok(()) => {}
                Err(TransportError::Rejected(Rejection::RoleForbidden)) => continue,
                Err(e) => return Err(e.into()),
            }
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
    /// undone (this device's own), a file the backend lost is uploaded again
    /// from the local copy, and a file on the backend that does not
    /// authenticate under the item's key is replaced by the holder's copy.
    fn reassert_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        skip: &BTreeSet<VaultId>,
    ) -> Result<(), SyncError> {
        let refs = access.with(|s| s.attachment_references())?;
        let mut by_vault: BTreeMap<VaultId, Vec<AttachmentRef>> = BTreeMap::new();
        for r in refs.into_iter().filter(|r| !skip.contains(&r.vault_id)) {
            by_vault.entry(r.vault_id).or_default().push(r);
        }
        let mut budget = ATTACHMENT_DOWNLOAD_BUDGET;
        for (vault, refs) in by_vault {
            let remote = self.transport.attachments(vault)?;
            for r in refs {
                self.reassert_one(access, vault, &r, &remote, &mut budget, report)?;
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
        budget: &mut u64,
        report: &mut SyncReport,
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
            Some(e)
                if matches!(
                    local,
                    AttachmentFileState::Present {
                        upload_pending: false
                    }
                ) =>
            {
                self.heal_one(access, vault, r, e.size, budget, report)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// The holder checks the backend's file against its own: a file whose
    /// size differs is opened with the item's key (every chunk), and so is
    /// one of the same size that this engine has not checked yet, within the
    /// run's download budget. A file that does not authenticate (planted by
    /// anyone who can write the folder, valid layout and all) is replaced by
    /// the local copy, which is authentic.
    fn heal_one(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        r: &AttachmentRef,
        remote_size: u64,
        budget: &mut u64,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let id = r.attachment_id;
        let local_len = access.with(|s| s.attachment_ciphertext_len(id))??;
        let differs = local_len != Some(remote_size);
        if !differs && self.verified_blobs.contains(&id) {
            return Ok(());
        }
        if *budget < remote_size {
            return Ok(());
        }
        *budget = budget.saturating_sub(remote_size);
        let authentic = match self.transport.get_attachment(vault, id)? {
            Some(bytes) => access.with(|s| {
                s.remote_attachment_authentic(vault, r.item_id, id, &bytes)
                    .unwrap_or(false)
            })?,
            // Not a valid layout: not an attachment.
            None => false,
        };
        if authentic {
            self.verified_blobs.insert(id);
            return Ok(());
        }
        let local = match access.with(|s| s.attachment_ciphertext(id))? {
            Ok(c) => c,
            Err(e) if is_skippable(&e) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let local_ok = access.with(|s| {
            s.remote_attachment_authentic(vault, r.item_id, id, &local)
                .unwrap_or(false)
        })?;
        if local_ok {
            match self.transport.replace_attachment(vault, id, &local) {
                Ok(()) => {
                    self.verified_blobs.insert(id);
                    report.attachments_healed += 1;
                }
                // A locked session ends the run; any other failure of this
                // optional repair is counted and tried again next time.
                Err(crate::transport::TransportError::Locked) => return Err(SyncError::Locked),
                Err(_) => report.attachments_heal_failed += 1,
            }
        }
        Ok(())
    }

    fn fetch_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        skip: &BTreeSet<VaultId>,
    ) -> Result<(), SyncError> {
        let missing: Vec<AttachmentRef> = access.with(|s| -> Result<_, Error> {
            let mut out = Vec::new();
            for r in s.attachment_references() {
                if skip.contains(&r.vault_id) {
                    continue;
                }
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
                // Valid layout, but it does not open under the item's file
                // key: not the attachment. It stays missing, and the holder
                // heals the backend.
                Err(Error::Core(_)) => report.attachments_rejected += 1,
                // A file that is not a valid attachment, or one already
                // stored: nothing to do now.
                Err(Error::Store(_) | Error::UnknownAttachmentReference) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
