//! Adding imported entries to a vault.

use std::collections::HashSet;

use mosaic_core::VaultId;
use mosaic_core::model::ItemType;
use mosaic_vault::session::{Error as SessionError, UnlockedSession};

use crate::draft::{ItemDraft, PasskeyDraft};
use crate::entries::Import;

/// What [`commit`] did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub struct CommitReport {
    /// Items added (passkeys count one each).
    pub imported: u32,
    /// Entries left out because the caller listed them in `skip`.
    pub skipped: u32,
    /// Entries the session refused (the item failed to encode or the store
    /// failed); nothing of them is stored.
    pub failed: u32,
    /// Attachment files added.
    pub attachments: u32,
    /// Attachment files the session refused (over the account quota, or a
    /// store without attachment storage); their items are imported.
    pub attachments_failed: u32,
}

fn bump(n: &mut u32) {
    *n = n.saturating_add(1);
}

/// Adds every entry of `import` whose index is not in `skip` to
/// `vault_id`, each as one item (one sync event), then its attachments, then
/// each of its passkeys as its own `passkey` item (an entry of passkeys only
/// makes no item of its own).
/// Consumes the import, so every value from the file is wiped when this
/// returns. Entries skipped while reading the file are not counted here.
///
/// An entry that fails is counted in [`CommitReport::failed`] and the
/// commit goes on with the next one; a failed attachment is counted in
/// [`CommitReport::attachments_failed`].
///
/// # Errors
/// [`SessionError::UnknownVault`] or [`SessionError::VaultUnavailable`]
/// for a vault that cannot take items; nothing is imported then.
pub fn commit(
    import: Import,
    session: &mut UnlockedSession,
    vault_id: VaultId,
    skip: &[u32],
) -> Result<CommitReport, SessionError> {
    session.vault_meta(vault_id)?;
    let skip: HashSet<u32> = skip.iter().copied().collect();
    let mut report = CommitReport::default();
    for entry in import.entries {
        if skip.contains(&entry.index) {
            bump(&mut report.skipped);
            continue;
        }
        let draft = entry.draft;
        if draft.has_item() {
            add_item(session, vault_id, &draft, &mut report)?;
        }
        for passkey in &draft.passkeys {
            add_passkey(session, vault_id, &draft, passkey, &mut report)?;
        }
    }
    Ok(report)
}

/// Adds the entry's own item and its attachments.
fn add_item(
    session: &mut UnlockedSession,
    vault_id: VaultId,
    draft: &ItemDraft,
    report: &mut CommitReport,
) -> Result<(), SessionError> {
    let item_id = match session.add_item(vault_id, &draft.item_type, |e| draft.apply(e)) {
        Ok(id) => id,
        Err(e @ (SessionError::UnknownVault | SessionError::VaultUnavailable)) => {
            return Err(e);
        }
        Err(_) => {
            bump(&mut report.failed);
            return Ok(());
        }
    };
    bump(&mut report.imported);
    for a in &draft.attachments {
        match session.add_attachment(
            vault_id,
            item_id,
            a.file_name.as_str(),
            a.mime.as_str(),
            a.data.expose_secret(),
        ) {
            Ok(_) => bump(&mut report.attachments),
            Err(_) => bump(&mut report.attachments_failed),
        }
    }
    Ok(())
}

/// Adds one passkey as its own `passkey` item, with the entry's tags and
/// trash state, and its signature counter. The item is one sync event: the
/// key and the counter are written together.
fn add_passkey(
    session: &mut UnlockedSession,
    vault_id: VaultId,
    draft: &ItemDraft,
    passkey: &PasskeyDraft,
    report: &mut CommitReport,
) -> Result<(), SessionError> {
    let added = session.add_item(vault_id, &ItemType::Passkey, |e| {
        e.set_title(passkey.title.as_str())?;
        e.set_passkey(&passkey.credential)?;
        if passkey.sign_count > 0 {
            e.set_passkey_sign_count(passkey.sign_count)?;
        }
        for tag in &draft.tags {
            e.add_tag(tag.as_str())?;
        }
        if draft.trashed {
            e.trash()?;
        }
        Ok(())
    });
    match added {
        Ok(_) => bump(&mut report.imported),
        Err(e @ (SessionError::UnknownVault | SessionError::VaultUnavailable)) => return Err(e),
        Err(_) => bump(&mut report.failed),
    }
    Ok(())
}
