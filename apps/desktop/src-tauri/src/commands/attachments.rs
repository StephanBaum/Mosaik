//! Attachment commands (ADR-0034). Every byte of file I/O happens here, in
//! Rust, on paths that only a native dialog opened by Rust can produce; the
//! webview sends IDs and receives, at most, downscaled preview pixels.
//!
//! Nothing decrypted outlives a call: a file is decrypted into a zeroizing
//! buffer, used, and dropped before the command returns. The session lock is
//! never held across a dialog, a disk read or a disk write, so after each of
//! those the flow takes the slot again and checks it is still unlocked.

use std::sync::atomic::{AtomicBool, Ordering};

use mosaic_core::{AccountId, ItemId, MAX_ATTACHMENT_LEN, SecretVec, VaultId};
use mosaic_vault::session::{Error as SessionError, UnlockedSession, VaultProtection};
use mosaic_vault::storage::Error as StoreError;
use tauri::State;
use tauri::ipc::Response;
use zeroize::Zeroizing;

use super::items::detail;
use crate::attachment_files::{
    read_bounded, render_preview, safe_file_name, sniff_mime, stored_name, write_file_atomic,
};
use crate::contract::attachments::{AttachmentAdded, AttachmentRequest, AttachmentSaved};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::{ItemDetail, ItemRequest};
use crate::dialogs::ANY_FILE;
use crate::state::{AppState, Backend};

/// `attachment_add`.
#[tauri::command]
pub async fn attachment_add(
    state: State<'_, AppState>,
    request: ItemRequest,
) -> IpcResult<AttachmentAdded> {
    state.run(move |b| b.attachment_add(request)).await
}

/// `attachment_preview`: the response is binary (an `ArrayBuffer` in the
/// webview), not JSON: `[width u32 LE][height u32 LE][RGBA bytes]`.
#[tauri::command]
pub async fn attachment_preview(
    state: State<'_, AppState>,
    request: AttachmentRequest,
) -> IpcResult<Response> {
    state
        .run(move |b| {
            let mut pixels = b.attachment_preview(&request)?;
            // The buffer leaves Rust's zeroizing types here, into Tauri's
            // response (THREAT_MODEL R35).
            Ok(Response::new(std::mem::take(&mut *pixels)))
        })
        .await
}

/// `attachment_save`.
#[tauri::command]
pub async fn attachment_save(
    state: State<'_, AppState>,
    request: AttachmentRequest,
) -> IpcResult<AttachmentSaved> {
    state.run(move |b| b.attachment_save(&request)).await
}

/// `attachment_remove`.
#[tauri::command]
pub async fn attachment_remove(
    state: State<'_, AppState>,
    request: AttachmentRequest,
) -> IpcResult<ItemDetail> {
    state.run(move |b| b.attachment_remove(&request)).await
}

/// Clears a "one at a time" flag on drop.
struct BusyGuard<'a>(&'a AtomicBool);

impl<'a> BusyGuard<'a> {
    /// Takes the flag.
    ///
    /// # Errors
    /// `busy` if it is taken.
    fn acquire(flag: &'a AtomicBool) -> IpcResult<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| IpcError::new(ErrorCode::Busy))?;
        Ok(Self(flag))
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// An item can take a file only in a software-protected vault (a hardware-key
/// vault has no attachment key) and while it is not in the trash.
fn check_attachable(
    session: &UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
) -> IpcResult<()> {
    if matches!(
        session.vault_protection(vault_id)?,
        VaultProtection::HardwareKey { .. }
    ) {
        return Err(IpcError::new(ErrorCode::ItemTypeNotAllowed));
    }
    if session.item(vault_id, item_id)?.is_deleted() {
        return Err(IpcError::new(ErrorCode::ItemDeleted));
    }
    Ok(())
}

/// A session error of reading an attachment, as the code the UI acts on:
/// a file that is not here, damaged or fails authentication is
/// `attachmentUnavailable`; the rest as everywhere.
fn attachment_error(error: SessionError) -> IpcError {
    match error {
        SessionError::Store(
            StoreError::UnknownAttachment
            | StoreError::Corrupt
            | StoreError::AttachmentsUnavailable,
        )
        | SessionError::Core(
            mosaic_core::Error::Decryption
            | mosaic_core::Error::Malformed
            | mosaic_core::Error::UnsupportedVersion
            | mosaic_core::Error::UnsupportedAlgorithm
            | mosaic_core::Error::SizeLimit,
        ) => IpcError::new(ErrorCode::AttachmentUnavailable),
        other => other.into(),
    }
}

impl Backend {
    /// Asks for a file in a native dialog, reads it (bounded), encrypts it as
    /// an attachment of the item and returns the item. `after_lock` needs no
    /// new state for this: the file is read and dropped inside the call.
    ///
    /// # Errors
    /// `locked`, `vaultNotFound`, `vaultUnavailable`, `itemNotFound`,
    /// `itemDeleted`, `itemTypeNotAllowed` (a hardware-protected vault),
    /// `attachmentTooLarge`, `attachmentQuota`, `attachmentFileUnreadable`,
    /// `storeUnavailable`, `internal`.
    pub fn attachment_add(&self, request: ItemRequest) -> IpcResult<AttachmentAdded> {
        self.touch();
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let account = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            check_attachable(session, vault_id, item_id)?;
            session.account_id()
        };
        let Some(path) = self.dialogs.open_file(ANY_FILE)? else {
            return Ok(AttachmentAdded::Cancelled);
        };
        let bytes = read_bounded(&path, MAX_ATTACHMENT_LEN)?;
        let name = stored_name(path.file_name());
        drop(path);
        let head = bytes.get(..16).unwrap_or(bytes.as_slice());
        let mime = sniff_mime(&name, head);
        // If the vault locked meanwhile this is `locked`, and the bytes are
        // wiped when they drop.
        // A lock and unlock during the dialog or the read can have brought in
        // another account, a hardware-protected vault or a trashed item: the
        // checks run again under this guard, against the account captured above.
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        if session.account_id() != account {
            return Err(IpcError::LOCKED);
        }
        check_attachable(session, vault_id, item_id)?;
        session.add_attachment(vault_id, item_id, &name, mime, bytes.as_slice())?;
        drop(bytes);
        self.items_changed(session);
        Ok(AttachmentAdded::Added {
            item: Box::new(detail(
                session.store(),
                vault_id,
                item_id,
                session.item(vault_id, item_id)?,
            )?),
        })
    }

    /// Decrypts an attachment of the item and decodes it as an image, one at
    /// a time. The result is `[width u32 LE][height u32 LE][RGBA]`; the
    /// plaintext and the decoded pixels are wiped before this returns.
    ///
    /// # Errors
    /// `locked`, `busy`, `itemNotFound`, `itemDeleted`, `attachmentNotFound`,
    /// `attachmentUnavailable`, `attachmentPreviewUnsupported`,
    /// `attachmentPreviewTooLarge`, `internal`.
    pub fn attachment_preview(&self, request: &AttachmentRequest) -> IpcResult<Zeroizing<Vec<u8>>> {
        self.touch();
        let _busy = BusyGuard::acquire(&self.preview_busy)?;
        let (plaintext, account) = self.decrypt_attachment(request, None)?;
        let pixels = render_preview(plaintext.expose_secret());
        drop(plaintext);
        // Decoding takes a while: if the vault locked meanwhile (or another
        // account unlocked), the pixels are dropped (wiped) here instead of
        // returned.
        if self.slot()?.unlocked()?.account_id() != account {
            return Err(IpcError::LOCKED);
        }
        pixels
    }

    /// Asks where to save an attachment in a native dialog, then decrypts it
    /// and writes it there atomically. Nothing is decrypted before the user
    /// has chosen a path, and a lock during the dialog saves nothing.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `itemDeleted`, `attachmentNotFound`,
    /// `attachmentUnavailable`, `attachmentSaveFailed`, `internal`.
    pub fn attachment_save(&self, request: &AttachmentRequest) -> IpcResult<AttachmentSaved> {
        self.touch();
        let (account, name) = self.attachment_name(request)?;
        let Some(path) = self.dialogs.save_file(&safe_file_name(&name), ANY_FILE)? else {
            return Ok(AttachmentSaved::Cancelled);
        };
        let (plaintext, _) = self.decrypt_attachment(request, Some(account))?;
        write_file_atomic(&path, plaintext.expose_secret())?;
        Ok(AttachmentSaved::Saved)
    }

    /// Removes an attachment from the item (a synced edit) and deletes its
    /// file from this device's store.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `itemDeleted`, `attachmentNotFound`,
    /// `storeUnavailable`.
    pub fn attachment_remove(&self, request: &AttachmentRequest) -> IpcResult<ItemDetail> {
        self.touch();
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.remove_attachment(vault_id, item_id, request.attachment_id.get())?;
        self.items_changed(session);
        detail(
            session.store(),
            vault_id,
            item_id,
            session.item(vault_id, item_id)?,
        )
    }

    /// The account and the stored name of an attachment that is on this
    /// device (the save dialog suggests the name).
    fn attachment_name(&self, request: &AttachmentRequest) -> IpcResult<(AccountId, String)> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let item = session.item(request.vault_id.get(), request.item_id.get())?;
        if item.is_deleted() {
            return Err(IpcError::new(ErrorCode::ItemDeleted));
        }
        let id = request.attachment_id.get();
        let reference = item
            .attachments()
            .into_iter()
            .find(|a| a.attachment_id == id)
            .ok_or(IpcError::new(ErrorCode::AttachmentNotFound))?;
        if !matches!(session.store().attachment_info(&id), Ok(Some(_))) {
            return Err(IpcError::new(ErrorCode::AttachmentUnavailable));
        }
        Ok((
            session.account_id(),
            reference.file_name.as_str().to_owned(),
        ))
    }

    /// Decrypts an attachment under the session lock, which is released
    /// before the caller uses the plaintext. With `account`, the session must
    /// still be that account's (it can have locked and unlocked during a
    /// dialog); a different one is `locked`. Returns the plaintext with the
    /// account it was decrypted for.
    fn decrypt_attachment(
        &self,
        request: &AttachmentRequest,
        account: Option<AccountId>,
    ) -> IpcResult<(SecretVec, AccountId)> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        if account.is_some_and(|expected| session.account_id() != expected) {
            return Err(IpcError::LOCKED);
        }
        let plaintext = session
            .attachment(
                request.vault_id.get(),
                request.item_id.get(),
                request.attachment_id.get(),
            )
            .map_err(attachment_error)?;
        Ok((plaintext, session.account_id()))
    }
}
