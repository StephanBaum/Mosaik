//! Adding imported entries to a vault.

use std::collections::HashSet;

use mosaic_core::VaultId;
use mosaic_core::model::ItemType;
use mosaic_vault::session::{Error as SessionError, ItemFilter, UnlockedSession};

use crate::draft::{EntryWarning, ItemDraft, PasskeyDraft};
use crate::entries::{Import, ImportEntry};

/// What [`commit`] did.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[non_exhaustive]
pub struct CommitReport {
    /// Items added (passkeys count one each).
    pub imported: u32,
    /// Entries left out because the caller listed them in `skip`, plus
    /// passkeys left out because the vault (or this commit) already holds the
    /// same credential (same relying party and credential ID).
    pub skipped: u32,
    /// Entries the session refused (the item failed to encode or the store
    /// failed); nothing of them is stored.
    pub failed: u32,
    /// Attachment files added.
    pub attachments: u32,
    /// Attachment files the session refused (over the account quota, or a
    /// store without attachment storage); their items are imported.
    pub attachments_failed: u32,
    /// Something of the file did not come across completely: an entry was
    /// skipped while reading the file, or a kept entry carries a warning
    /// that data was left out (see [`EntryWarning::loses_data`]), or an
    /// entry or attachment failed. Entries the caller listed in `skip`
    /// do not count (left out on purpose).
    pub incomplete: bool,
}

impl CommitReport {
    /// Whether everything the file held, bar what the caller skipped on
    /// purpose, is in the vault. Only then is the file safe to delete.
    #[must_use]
    pub const fn clean(&self) -> bool {
        !self.incomplete && self.failed == 0 && self.attachments_failed == 0
    }
}

/// Entries written per store transaction. One transaction costs one fsync,
/// so this bounds the writes lost to a crash (the chunk in flight) while
/// keeping a 10,000-entry import at tens of commits instead of tens of
/// thousands (ADR-0170).
const CHUNK: usize = 500;

fn bump(n: &mut u32) {
    *n = n.saturating_add(1);
}

/// Adds every entry of `import` whose index is not in `skip` to
/// `vault_id`, each as one item (one sync event), written in batched
/// transactions of 500 entries, then its attachments, then
/// each of its passkeys as its own `passkey` item (an entry of passkeys only
/// makes no item of its own). A passkey whose relying party and credential
/// ID are already in the account, or were written earlier in this commit,
/// is not added again (counted in [`CommitReport::skipped`]).
/// Consumes the import, so every value from the file is wiped when this
/// returns. Entries skipped while reading the file are not counted in
/// [`CommitReport::skipped`], but make the report [`incomplete`](CommitReport::incomplete).
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
    let mut report = CommitReport {
        incomplete: !import.skipped.is_empty(),
        ..CommitReport::default()
    };
    // Credentials already in the account (neither value is secret; no key is
    // read), then those written by this commit.
    let mut known: HashSet<(String, Vec<u8>)> = session
        .items(&ItemFilter::default())
        .iter()
        .filter_map(|r| r.item.passkey_public())
        .map(|p| (p.rp_id.as_str().to_owned(), p.credential_id))
        .collect();
    // Each chunk is one `session.batch`. The session lock is NOT released
    // between chunks: the caller (the desktop import command, the CLI) holds
    // the `&mut UnlockedSession` for the whole call, so letting other work in
    // between would need an API that takes a lock handle instead (a redesign
    // left for later). The batching alone removes the per-entry fsync.
    let mut entries = import.entries.into_iter();
    loop {
        let chunk: Vec<ImportEntry> = entries.by_ref().take(CHUNK).collect();
        if chunk.is_empty() {
            break;
        }
        session.batch(|s| commit_chunk(chunk, s, vault_id, &skip, &mut known, &mut report))?;
    }
    Ok(report)
}

/// Adds one chunk of entries (inside the caller's batch).
fn commit_chunk(
    chunk: Vec<ImportEntry>,
    session: &mut UnlockedSession,
    vault_id: VaultId,
    skip: &HashSet<u32>,
    known: &mut HashSet<(String, Vec<u8>)>,
    report: &mut CommitReport,
) -> Result<(), SessionError> {
    for entry in chunk {
        if skip.contains(&entry.index) {
            bump(&mut report.skipped);
            continue;
        }
        if entry.warnings.iter().any(EntryWarning::loses_data) {
            report.incomplete = true;
        }
        let draft = entry.draft;
        if draft.has_item() {
            add_item(session, vault_id, &draft, report)?;
        }
        for passkey in &draft.passkeys {
            let id = (
                passkey.credential.rp_id.as_str().to_owned(),
                passkey.credential.credential_id.clone(),
            );
            if known.contains(&id) {
                bump(&mut report.skipped);
            } else if add_passkey(session, vault_id, &draft, passkey, report)? {
                known.insert(id);
            }
        }
    }
    Ok(())
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
/// trash state, favorite mark and its signature counter. The item is one sync
/// event: the key and the counter are written together. Whether it was added.
fn add_passkey(
    session: &mut UnlockedSession,
    vault_id: VaultId,
    draft: &ItemDraft,
    passkey: &PasskeyDraft,
    report: &mut CommitReport,
) -> Result<bool, SessionError> {
    let added = session.add_item(vault_id, &ItemType::Passkey, |e| {
        e.set_title(passkey.title.as_str())?;
        e.set_passkey(&passkey.credential)?;
        if passkey.sign_count > 0 {
            e.set_passkey_sign_count(passkey.sign_count)?;
        }
        for tag in &draft.tags {
            e.add_tag(tag.as_str())?;
        }
        if draft.favorite {
            e.set_favorite(true)?;
        }
        if draft.trashed {
            e.trash()?;
        }
        Ok(())
    });
    match added {
        Ok(_) => {
            bump(&mut report.imported);
            Ok(true)
        }
        Err(e @ (SessionError::UnknownVault | SessionError::VaultUnavailable)) => Err(e),
        Err(_) => {
            bump(&mut report.failed);
            Ok(false)
        }
    }
}
