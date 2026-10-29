//! Adding imported entries to a vault.

use std::collections::HashSet;

use mosaic_core::VaultId;
use mosaic_vault::session::{Error as SessionError, UnlockedSession};

use crate::entries::Import;

/// What [`commit`] did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub struct CommitReport {
    /// Items added.
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
/// `vault_id`, each as one item (one sync event), then its attachments.
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
        let item_id = match session.add_item(vault_id, &draft.item_type, |e| draft.apply(e)) {
            Ok(id) => id,
            Err(e @ (SessionError::UnknownVault | SessionError::VaultUnavailable)) => {
                return Err(e);
            }
            Err(_) => {
                bump(&mut report.failed);
                continue;
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
    }
    Ok(report)
}
