//! Save and update (ADR-0038): `checkSave` classifies a captured login,
//! `proposeSave` shows it to the user, and the user's answer writes the
//! vault. The extension never writes: every item change below happens after
//! the user pressed Save in the desktop dialog.

use mosaic_core::model::{ItemType, LoginUrl, SecretField, TextField, UrlMatch};
use mosaic_core::{ItemId, VaultId};
use mosaic_native_messaging::bodies::{
    CheckSaveResult, ProposeSaveResult, SaveBody, SaveKind, SaveOutcome,
};
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use mosaic_origin::{PageOrigin, registrable_domain};
use mosaic_vault::session::{UnlockedSession, VaultProtection};
use subtle::ConstantTimeEq as _;

use super::fill::{best_match, logins, parse_origin, wire};
use super::prompts::{PendingSave, SavePlan};
use crate::commands::is_spoofing_char;
use crate::contract::browser::{BrowserSaveCandidate, BrowserSaveDecision, BrowserSaveKind};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::Backend;

/// The longest user name, in bytes.
pub const MAX_USERNAME_BYTES: usize = 256;
/// The longest password, in bytes.
pub const MAX_PASSWORD_BYTES: usize = 1024;
/// The longest origin, in bytes.
pub const MAX_ORIGIN_BYTES: usize = 2048;
/// The longest title, in characters.
pub const MAX_TITLE_CHARS: usize = 256;

/// What the captured login is compared with.
pub(super) struct Analysis {
    pub(super) kind: SaveKind,
    /// Matching logins with the same user name.
    pub(super) candidates: Vec<BrowserSaveCandidate>,
    /// The title of the login an update would change (the first candidate).
    pub(super) title: Option<String>,
}

/// Whether two user names are the same login: trimmed and exact, or
/// case-insensitive when they are e-mail addresses.
fn same_user(saved: &str, captured: &str) -> bool {
    let (saved, captured) = (saved.trim(), captured.trim());
    if captured.contains('@') {
        saved.to_lowercase() == captured.to_lowercase()
    } else {
        saved == captured
    }
}

/// Whether the stored password equals `password` (constant time in the
/// content; the length is not secret here).
fn same_password(item: &mosaic_core::model::Item, password: &str) -> bool {
    item.secret(SecretField::Password)
        .ok()
        .flatten()
        .is_some_and(|saved| {
            saved
                .expose_secret()
                .as_bytes()
                .ct_eq(password.as_bytes())
                .into()
        })
}

/// Classifies a captured login against the logins that match its page
/// (exact or eTLD+1; the HTTP block is ignored).
pub(super) fn analyze(
    session: &UnlockedSession,
    page: &PageOrigin,
    username: Option<&str>,
    password: &str,
) -> Analysis {
    let matching: Vec<_> = logins(session)
        .into_iter()
        .filter(|entry| best_match(entry.item, page).is_some())
        .collect();
    let new = || Analysis {
        kind: SaveKind::New,
        candidates: Vec::new(),
        title: None,
    };
    let Some(username) = username.filter(|u| !u.trim().is_empty()) else {
        // Without a user name: unchanged only if the one match has this password.
        return match matching.as_slice() {
            [only] if same_password(only.item, password) => Analysis {
                kind: SaveKind::Unchanged,
                ..new()
            },
            _ => new(),
        };
    };
    let mut same: Vec<_> = matching
        .iter()
        .filter(|entry| {
            entry
                .item
                .text(TextField::Username)
                .is_some_and(|saved| same_user(saved, username))
        })
        .collect();
    same.sort_by_key(|entry| entry.item_id.to_string());
    if same.iter().any(|entry| same_password(entry.item, password)) {
        return Analysis {
            kind: SaveKind::Unchanged,
            ..new()
        };
    }
    Analysis {
        kind: if same.is_empty() {
            SaveKind::New
        } else {
            SaveKind::Update
        },
        title: same
            .first()
            .and_then(|entry| entry.item.title())
            .filter(|t| !t.trim().is_empty())
            .map(str::to_owned),
        candidates: same
            .iter()
            .map(|entry| BrowserSaveCandidate {
                vault_id: entry.vault_id.into(),
                item_id: entry.item_id.into(),
                title: entry
                    .item
                    .title()
                    .filter(|t| !t.trim().is_empty())
                    .map(str::to_owned),
                username: entry.item.text(TextField::Username).map(str::to_owned),
            })
            .collect(),
    }
}

/// Checks the sizes of a captured login.
pub(super) fn check_sizes(body: &SaveBody) -> Result<(), WireError> {
    let password = body.password.expose_secret();
    let ok = body.origin.len() <= MAX_ORIGIN_BYTES
        && body.top_origin.len() <= MAX_ORIGIN_BYTES
        && body.username.as_ref().is_none_or(|u| {
            // The user name is shown in the desktop dialog: no control,
            // format or default-ignorable characters (as for titles).
            u.len() <= MAX_USERNAME_BYTES
                && !u.chars().any(|c| c.is_control() || is_spoofing_char(c))
        })
        && !password.is_empty()
        && password.len() <= MAX_PASSWORD_BYTES;
    if ok {
        Ok(())
    } else {
        Err(WireError::InvalidRequest)
    }
}

/// The page a captured login is from. A save is only ever for the top-level page: a frame
/// (`origin != topOrigin`) is refused, as a fill into one is, so a hidden frame cannot make the
/// user save or update a login for an origin they are not looking at.
fn save_page(body: &SaveBody) -> Result<PageOrigin, WireError> {
    let page = parse_origin(&body.origin)?;
    let top = parse_origin(&body.top_origin)?;
    if page != top {
        return Err(WireError::CrossOriginFrame);
    }
    Ok(page)
}

impl Backend {
    /// `checkSave`: would this captured login be new, an update, or
    /// unchanged.
    pub(crate) fn browser_check_save(&self, body: &SaveBody) -> Result<CheckSaveResult, WireError> {
        check_sizes(body)?;
        let page = save_page(body)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        let found = analyze(
            session,
            &page,
            body.username.as_deref(),
            body.password.expose_secret(),
        );
        Ok(CheckSaveResult {
            kind: found.kind,
            item_title: found.title,
        })
    }

    /// What `proposeSave` shows the user, or `None` when nothing would
    /// change.
    pub(crate) fn browser_plan_save(&self, body: &SaveBody) -> Result<Option<SavePlan>, WireError> {
        check_sizes(body)?;
        let page = save_page(body)?;
        let slot = self.slot().map_err(wire)?;
        let session = slot.unlocked().map_err(wire)?;
        let found = analyze(
            session,
            &page,
            body.username.as_deref(),
            body.password.expose_secret(),
        );
        let kind = match found.kind {
            SaveKind::New => BrowserSaveKind::New,
            SaveKind::Update => BrowserSaveKind::Update,
            SaveKind::Unchanged => return Ok(None),
        };
        let host = page.host_str();
        Ok(Some(SavePlan {
            kind,
            origin: page.serialize(),
            suggested_title: registrable_domain(&host).unwrap_or_else(|| host.clone()),
            host,
            insecure: page.is_http() && !page.is_loopback(),
            candidates: found.candidates,
        }))
    }

    /// Writes the user's answer to a save request.
    ///
    /// # Errors
    /// `invalidInput` (a bad title), `vaultNotFound`, `vaultUnavailable`,
    /// `itemNotFound` (not one of the candidates, or gone), `locked`.
    pub(crate) fn browser_apply_save(
        &self,
        pending: &PendingSave,
        decision: &BrowserSaveDecision,
    ) -> IpcResult<ProposeSaveResult> {
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let (vault_id, item_id, outcome) = match decision {
            BrowserSaveDecision::SaveNew {
                vault_id,
                title,
                allow_http,
            } => {
                if !title_is_valid(title) {
                    return Err(IpcError::new(ErrorCode::InvalidInput));
                }
                let vault = vault_id.get();
                check_writable(session, vault)?;
                let item = add_login(session, vault, title, *allow_http, pending)?;
                (vault, item, SaveOutcome::Saved)
            }
            BrowserSaveDecision::Update { vault_id, item_id } => {
                let (vault, item) = (vault_id.get(), item_id.get());
                if !pending.candidates.contains(&(vault, item)) {
                    return Err(IpcError::new(ErrorCode::ItemNotFound));
                }
                check_writable(session, vault)?;
                update_password(session, vault, item, pending)?;
                (vault, item, SaveOutcome::Updated)
            }
            BrowserSaveDecision::Cancel => return Err(IpcError::INTERNAL),
        };
        self.items_changed(session);
        Ok(ProposeSaveResult {
            outcome,
            vault_id: Some(vault_id.to_string()),
            item_id: Some(item_id.to_string()),
        })
    }
}

fn title_is_valid(title: &str) -> bool {
    let count = title.chars().count();
    (1..=MAX_TITLE_CHARS).contains(&count)
        && !title.trim().is_empty()
        && !title.chars().any(|c| c.is_control() || is_spoofing_char(c))
}

/// The vault exists, holds logins, and this session has its key.
pub(super) fn check_writable(session: &UnlockedSession, vault: VaultId) -> IpcResult<()> {
    let summary = session
        .vaults()
        .into_iter()
        .find(|v| v.vault_id == vault)
        .ok_or(IpcError::new(ErrorCode::VaultNotFound))?;
    // A hardware-protected vault holds only one-time-code items.
    let hardware = matches!(
        session.vault_protection(vault),
        Ok(VaultProtection::HardwareKey { .. })
    );
    if summary.available && !hardware {
        Ok(())
    } else {
        Err(IpcError::new(ErrorCode::VaultUnavailable))
    }
}

fn add_login(
    session: &mut UnlockedSession,
    vault: VaultId,
    title: &str,
    allow_http: bool,
    pending: &PendingSave,
) -> IpcResult<ItemId> {
    let mut url = LoginUrl::new(&pending.origin);
    url.match_mode = UrlMatch::ExactOrigin;
    url.allow_http = allow_http && pending.insecure;
    Ok(session.add_item(vault, &ItemType::Login, |edit| {
        edit.set_title(title.trim())?;
        if let Some(username) = pending.username.as_deref().filter(|u| !u.trim().is_empty()) {
            edit.set_text(TextField::Username, username.trim())?;
        }
        edit.set_secret(SecretField::Password, pending.password.expose_secret())?;
        edit.add_url(&url)?;
        Ok(())
    })?)
}

/// Replaces the password of a login; the model moves the old one into the
/// history.
fn update_password(
    session: &mut UnlockedSession,
    vault: VaultId,
    item: ItemId,
    pending: &PendingSave,
) -> IpcResult<()> {
    let current = session
        .item(vault, item)
        .map_err(|_| IpcError::new(ErrorCode::ItemNotFound))?;
    if current.is_deleted()
        || current.trashed_at().is_some()
        || current.item_type() != Some(ItemType::Login)
    {
        return Err(IpcError::new(ErrorCode::ItemNotFound));
    }
    session.update_item(vault, item, |edit| {
        edit.set_secret(SecretField::Password, pending.password.expose_secret())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_names_compare_trimmed_and_case_insensitive_for_emails() {
        assert!(same_user("alex", " alex "));
        assert!(!same_user("Alex", "alex"));
        assert!(same_user("Alex@Example.com", "alex@example.com"));
        assert!(!same_user("alex", "alex2"));
    }

    #[test]
    fn titles_are_checked() {
        assert!(title_is_valid("Example"));
        assert!(!title_is_valid(""));
        assert!(!title_is_valid("  "));
        assert!(!title_is_valid(&"x".repeat(257)));
        assert!(!title_is_valid("a\u{202e}b"));
        assert!(!title_is_valid("a\tb"));
    }
}
