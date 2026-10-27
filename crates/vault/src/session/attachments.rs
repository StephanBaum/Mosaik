//! Attachments (CRYPTO_SPEC §7.3): the file key, name, type and size live
//! in the owning item's plaintext; the STREAM ciphertext is a file in the
//! store.

use mosaic_core::model::{AttachmentRef, Text};
use mosaic_core::{
    AttachmentId, AttachmentKey, ItemId, MAX_ATTACHMENT_LEN, SecretVec, VaultId, open_attachment,
    seal_attachment,
};

use super::{Error, Result, UnlockedSession};

/// Default limit for all attachments of an account together (CRYPTO_SPEC
/// §7.3): 1 GiB of plaintext.
pub const MAX_ACCOUNT_ATTACHMENT_BYTES: u64 = 1024 * 1024 * 1024;

impl UnlockedSession {
    /// The plaintext size of every attachment referenced by a live item, in
    /// bytes (what counts against [`MAX_ACCOUNT_ATTACHMENT_BYTES`]).
    #[must_use]
    pub fn attachment_usage(&self) -> u64 {
        self.vaults
            .values()
            .flat_map(|ov| ov.items.values())
            .filter(|item| !item.is_deleted())
            .flat_map(mosaic_core::Item::attachments)
            .fold(0u64, |sum, a| sum.saturating_add(a.size))
    }

    /// Encrypts `data` as a new attachment of an item: a fresh attachment ID
    /// and file key, the ciphertext stored (and queued for upload), then the
    /// reference added to the item. The limits are checked before anything
    /// is encrypted.
    ///
    /// # Errors
    /// [`Error::AttachmentTooLarge`] above
    /// [`mosaic_core::MAX_ATTACHMENT_LEN`], [`Error::AttachmentQuota`] above
    /// [`MAX_ACCOUNT_ATTACHMENT_BYTES`] in total; [`Error::UnknownVault`],
    /// [`Error::UnknownItem`], [`Error::ItemDeleted`]; [`Error::Store`]
    /// (an in-memory store has no attachments).
    pub fn add_attachment(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        file_name: &str,
        mime: &str,
        data: &[u8],
    ) -> Result<AttachmentId> {
        if data.len() > MAX_ATTACHMENT_LEN {
            return Err(Error::AttachmentTooLarge);
        }
        let size = u64::try_from(data.len()).map_err(|_| Error::AttachmentTooLarge)?;
        if self.attachment_usage().saturating_add(size) > MAX_ACCOUNT_ATTACHMENT_BYTES {
            return Err(Error::AttachmentQuota);
        }
        self.vault(vault_id)?.live_item(item_id)?;
        let reference = AttachmentRef::new(
            AttachmentId::new_v7()?,
            Text::new(file_name),
            Text::new(mime),
            size,
            AttachmentKey::generate()?,
        );
        let attachment_id = reference.attachment_id;
        let ct = seal_attachment(&reference.key, &vault_id, &item_id, &attachment_id, data)?;
        self.store
            .put_attachment(&vault_id, &item_id, &attachment_id, &ct, true)?;
        if let Err(e) = self.update_item(vault_id, item_id, |e| e.set_attachment(&reference)) {
            // Do not leave an unreferenced file behind.
            self.store.delete_attachment(&attachment_id)?;
            return Err(e);
        }
        Ok(attachment_id)
    }

    /// Decrypts an attachment of an item.
    ///
    /// # Errors
    /// [`Error::UnknownAttachment`] if the item has no such attachment;
    /// [`Error::Store`] if its file is not in this store (not downloaded
    /// yet) or damaged; [`Error::Core`] if it fails to authenticate.
    pub fn attachment(
        &self,
        vault_id: VaultId,
        item_id: ItemId,
        attachment_id: AttachmentId,
    ) -> Result<SecretVec> {
        let item = self.vault(vault_id)?.live_item(item_id)?;
        let reference = item
            .attachments()
            .into_iter()
            .find(|a| a.attachment_id == attachment_id)
            .ok_or(Error::UnknownAttachment)?;
        let ct = self.store.attachment(&attachment_id)?;
        Ok(open_attachment(
            &reference.key,
            &vault_id,
            &item_id,
            &attachment_id,
            &ct,
        )?)
    }

    /// Removes an attachment: the reference leaves the item (a synced edit)
    /// and the file leaves this store.
    ///
    /// # Errors
    /// [`Error::UnknownAttachment`] if the item has no such attachment; as
    /// [`UnlockedSession::update_item`].
    pub fn remove_attachment(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        attachment_id: AttachmentId,
    ) -> Result<()> {
        let item = self.vault(vault_id)?.live_item(item_id)?;
        if !item
            .attachments()
            .iter()
            .any(|a| a.attachment_id == attachment_id)
        {
            return Err(Error::UnknownAttachment);
        }
        self.update_item(vault_id, item_id, |e| e.remove_attachment(&attachment_id))?;
        self.store.delete_attachment(&attachment_id)?;
        Ok(())
    }

    /// Deletes the stored files of a record that its current state no
    /// longer references (after a purge, a removal or a merge).
    pub(super) fn prune_attachments(&mut self, vault_id: VaultId, item_id: ItemId) -> Result<()> {
        let referenced: Vec<AttachmentId> = match self.vault(vault_id)?.items.get(&item_id) {
            Some(item) if !item.is_deleted() => {
                item.attachments().iter().map(|a| a.attachment_id).collect()
            }
            _ => Vec::new(),
        };
        for info in self.store.attachments_for_record(&vault_id, &item_id)? {
            if !referenced.contains(&info.attachment_id) {
                self.store.delete_attachment(&info.attachment_id)?;
            }
        }
        Ok(())
    }
}
