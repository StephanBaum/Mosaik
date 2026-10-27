//! Item operations: add, edit, read, list, search, favorite, trash,
//! purge, move, and the current one-time code.

use mosaic_core::model::{AttachmentRef, Item, ItemEdit, ItemType, OtpKind, Text};
use mosaic_core::{AttachmentId, AttachmentKey, ItemId, VaultId, open_attachment, seal_attachment};
use mosaic_totp::Code;

use super::{Error, Result, UnlockedSession, dup_item, now};
use crate::otp::otp_from_config;
use crate::storage::Origin;

/// Which items a listing includes with respect to the trash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrashFilter {
    /// Items not in the trash (the default).
    #[default]
    Exclude,
    /// Only items in the trash.
    Only,
    /// Both.
    Include,
}

/// A filter for [`UnlockedSession::items`]. The default lists every vault's
/// items outside the trash.
///
/// `#[non_exhaustive]`: start from [`ItemFilter::default`] and set the
/// fields you need, so a later filter is not a breaking change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ItemFilter {
    /// Only this vault.
    pub vault: Option<VaultId>,
    /// Trash handling.
    pub trash: TrashFilter,
    /// Only favorites.
    pub favorites_only: bool,
}

/// A decrypted item and where it lives.
#[derive(Debug, Clone, Copy)]
pub struct ItemRef<'a> {
    /// The vault.
    pub vault_id: VaultId,
    /// The item (record) ID.
    pub item_id: ItemId,
    /// The decrypted item.
    pub item: &'a Item,
}

/// A one-time code and how long it stays valid.
#[derive(Debug)]
pub struct OtpCode {
    /// The code (zeroized on drop).
    pub code: Code,
    /// Seconds until a time-based code changes; `None` for HOTP.
    pub seconds_remaining: Option<u32>,
}

impl ItemFilter {
    fn accepts(&self, vault_id: VaultId, item: &Item) -> bool {
        if item.is_deleted() || self.vault.is_some_and(|v| v != vault_id) {
            return false;
        }
        if self.favorites_only && !item.favorite() {
            return false;
        }
        let trashed = item.trashed_at().is_some();
        match self.trash {
            TrashFilter::Exclude => !trashed,
            TrashFilter::Only => trashed,
            TrashFilter::Include => true,
        }
    }
}

impl UnlockedSession {
    /// A decrypted item, live or deleted ([`Item::is_deleted`]).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::UnknownItem`].
    pub fn item(&self, vault_id: VaultId, item_id: ItemId) -> Result<&Item> {
        self.vault(vault_id)?
            .items
            .get(&item_id)
            .ok_or(Error::UnknownItem)
    }

    /// The live items that pass `filter`, by vault and item ID.
    #[must_use]
    pub fn items(&self, filter: &ItemFilter) -> Vec<ItemRef<'_>> {
        self.vaults
            .iter()
            .flat_map(|(v, ov)| {
                ov.items
                    .iter()
                    .filter(|(_, item)| filter.accepts(*v, item))
                    .map(|(i, item)| ItemRef {
                        vault_id: *v,
                        item_id: *i,
                        item,
                    })
            })
            .collect()
    }

    /// Searches titles, user names, emails, URLs and tags of the live items
    /// outside the trash (case-insensitive substring match of every term).
    /// The index is in memory only.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<ItemRef<'_>> {
        self.index
            .search(query)
            .into_iter()
            .filter_map(|(v, i)| {
                let item = self.vaults.get(&v)?.items.get(&i)?;
                (item.trashed_at().is_none() && !item.is_deleted()).then_some(ItemRef {
                    vault_id: v,
                    item_id: i,
                    item,
                })
            })
            .collect()
    }

    /// Adds an item of `item_type` to a vault and applies `edit` to it, as
    /// one local change.
    ///
    /// ```no_run
    /// # use mosaic_core::model::{ItemType, LoginUrl, SecretField, TextField};
    /// # fn f(s: &mut mosaic_vault::session::UnlockedSession, v: mosaic_core::VaultId) -> mosaic_vault::session::Result<()> {
    /// s.add_item(v, &ItemType::Login, |e| {
    ///     e.set_title("Example")?;
    ///     e.set_text(TextField::Username, "alice")?;
    ///     e.set_secret(SecretField::Password, "hunter2")?;
    ///     e.add_url(&LoginUrl::new("https://example.com"))?;
    ///     Ok(())
    /// })?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::VaultUnavailable`], [`Error::Core`]
    /// from `edit` or sealing (nothing is stored then), [`Error::Store`].
    pub fn add_item<F>(
        &mut self,
        vault_id: VaultId,
        item_type: &ItemType,
        edit: F,
    ) -> Result<ItemId>
    where
        F: FnOnce(&mut ItemEdit<'_>) -> mosaic_core::Result<()>,
    {
        self.vault(vault_id)?.current_key()?;
        let at = now();
        let mut item = Item::new(item_type, self.device, at)?;
        edit(&mut item.edit(self.device, at))?;
        let item_id = ItemId::new_v7()?;
        self.put_item(vault_id, item_id, item, Origin::Local)?;
        Ok(item_id)
    }

    /// Edits an item: `edit` runs on a copy, and the copy replaces the item
    /// only if `edit` succeeds and changed something. Changing a password
    /// (or another secret that keeps history) moves the old value into the
    /// password history ([`ItemEdit::set_secret`]).
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::UnknownItem`],
    /// [`Error::ItemDeleted`], [`Error::Core`] from `edit`, [`Error::Store`].
    pub fn update_item<F>(&mut self, vault_id: VaultId, item_id: ItemId, edit: F) -> Result<()>
    where
        F: FnOnce(&mut ItemEdit<'_>) -> mosaic_core::Result<()>,
    {
        let ov = self.vault(vault_id)?;
        ov.current_key()?;
        let current = ov.live_item(item_id)?;
        let mut copy = dup_item(current)?;
        edit(&mut copy.edit(self.device, now()))?;
        if copy.version() == current.version() {
            return Ok(());
        }
        self.put_item(vault_id, item_id, copy, Origin::Local)
    }

    /// Marks or unmarks an item as a favorite.
    ///
    /// # Errors
    /// As [`UnlockedSession::update_item`].
    pub fn set_favorite(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        favorite: bool,
    ) -> Result<()> {
        self.update_item(vault_id, item_id, |e| e.set_favorite(favorite))
    }

    /// Moves an item to the trash (CRYPTO_SPEC §7.4).
    ///
    /// # Errors
    /// As [`UnlockedSession::update_item`].
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method path is not general enough over the edit's lifetime"
    )]
    pub fn trash_item(&mut self, vault_id: VaultId, item_id: ItemId) -> Result<()> {
        self.update_item(vault_id, item_id, |e| e.trash())
    }

    /// Restores an item from the trash.
    ///
    /// # Errors
    /// As [`UnlockedSession::update_item`].
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method path is not general enough over the edit's lifetime"
    )]
    pub fn restore_item(&mut self, vault_id: VaultId, item_id: ItemId) -> Result<()> {
        self.update_item(vault_id, item_id, |e| e.restore_from_trash())
    }

    /// Deletes an item for good: it becomes an encrypted tombstone that
    /// syncs like any edit (CRYPTO_SPEC §7.2) and keeps the record's
    /// rollback anchor. Its attachment files are removed from this store.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::UnknownItem`],
    /// [`Error::ItemDeleted`], [`Error::Store`].
    pub fn purge_item(&mut self, vault_id: VaultId, item_id: ItemId) -> Result<()> {
        let ov = self.vault(vault_id)?;
        ov.current_key()?;
        let mut copy = dup_item(ov.live_item(item_id)?)?;
        copy.delete(self.device, now())?;
        self.put_item(vault_id, item_id, copy, Origin::Local)?;
        self.prune_attachments(vault_id, item_id)
    }

    /// Moves an item to another vault (CRYPTO_SPEC §7.2): the item is
    /// re-encrypted under the target vault's key as a new record, each
    /// attachment is re-sealed under a fresh attachment ID and key, and only
    /// then is the original turned into a tombstone. Returns the new item
    /// ID.
    ///
    /// Every attachment must be present in this store (downloaded) before
    /// the move; otherwise nothing is changed.
    ///
    /// # Errors
    /// [`Error::SameVault`], [`Error::UnknownVault`],
    /// [`Error::VaultUnavailable`], [`Error::UnknownItem`],
    /// [`Error::ItemDeleted`], [`Error::Store`] (for example a missing
    /// attachment file), [`Error::Core`].
    pub fn move_item(&mut self, from: VaultId, item_id: ItemId, to: VaultId) -> Result<ItemId> {
        if from == to {
            return Err(Error::SameVault);
        }
        self.vault(to)?.current_key()?;
        let source = self.vault(from)?.live_item(item_id)?;
        let new_id = ItemId::new_v7()?;

        // Prepare everything that can fail before the first write.
        let mut resealed = Vec::new();
        for old in source.attachments() {
            let ct = self.store.attachment(&old.attachment_id)?;
            let plain = open_attachment(&old.key, &from, &item_id, &old.attachment_id, &ct)?;
            let new_ref = AttachmentRef::new(
                AttachmentId::new_v7()?,
                Text::new(old.file_name.as_str()),
                Text::new(old.mime.as_str()),
                old.size,
                AttachmentKey::generate()?,
            );
            let new_ct = seal_attachment(
                &new_ref.key,
                &to,
                &new_id,
                &new_ref.attachment_id,
                plain.expose_secret(),
            )?;
            resealed.push((old.attachment_id, new_ref, new_ct));
        }
        let mut moved = dup_item(source)?;
        if !resealed.is_empty() {
            let mut e = moved.edit(self.device, now());
            for (old_id, _, _) in &resealed {
                e.remove_attachment(old_id)?;
            }
        }

        // 1. The copy, without attachment references, in the target vault.
        self.put_item(to, new_id, moved, Origin::Local)?;
        if !resealed.is_empty() {
            // 2. The re-sealed files, then 3. the references to them.
            for (_, new_ref, ct) in &resealed {
                self.store
                    .put_attachment(&to, &new_id, &new_ref.attachment_id, ct, true)?;
            }
            self.update_item(to, new_id, |e| {
                for (_, new_ref, _) in &resealed {
                    e.set_attachment(new_ref)?;
                }
                Ok(())
            })?;
        }
        // 4. Only now the original becomes a tombstone.
        self.purge_item(from, item_id)?;
        Ok(new_id)
    }

    /// The current one-time code of a login or TOTP item at `unix_time`
    /// (Unix seconds; [`mosaic_totp::unix_time_now`] reads the clock).
    ///
    /// For HOTP the code is the one for the stored counter, and the counter
    /// is advanced and saved before the code is returned, so this device
    /// never shows the same code twice. Across devices that is not
    /// guaranteed: two devices that are offline at the same counter both
    /// show its code, and after sync the counter is one above it (it merges
    /// by maximum, ADR-0009 item 8), not two. The service accepts the code
    /// once either way.
    ///
    /// # Errors
    /// [`Error::NoOtp`] if the item has no OTP configuration;
    /// [`Error::Otp`] if the stored configuration is outside what
    /// `mosaic-totp` accepts; as [`UnlockedSession::update_item`] for HOTP.
    pub fn otp_code(
        &mut self,
        vault_id: VaultId,
        item_id: ItemId,
        unix_time: u64,
    ) -> Result<OtpCode> {
        let item = self.vault(vault_id)?.live_item(item_id)?;
        let config = item.otp().ok_or(Error::NoOtp)?;
        let counter = item.otp_counter();
        let otp = otp_from_config(&config, counter)?;
        let code = otp.code_at(unix_time)?;
        let seconds_remaining = otp.seconds_remaining(unix_time);
        if config.kind == OtpKind::Hotp {
            let next = counter
                .checked_add(1)
                .ok_or(Error::Otp(mosaic_totp::Error::SizeLimit))?;
            self.update_item(vault_id, item_id, |e| e.set_otp_counter(next))?;
        }
        Ok(OtpCode {
            code,
            seconds_remaining,
        })
    }
}
