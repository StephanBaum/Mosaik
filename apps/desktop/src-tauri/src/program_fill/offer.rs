//! The vault reads of program fill (ADR-0164): which logins are linked to a program, the
//! suggestions and the search of the list, the lookups of a pick and of a link. The password is
//! read here and nowhere else, only for a fill, and only into a `SecretString` that goes to the
//! host.

use std::collections::HashMap;

use mosaic_core::model::{Item, ItemType, LoginUrl, Role, SecretField, TextField};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_origin::program::{BindingChange, ProgramUrl, binding_change};
use mosaic_origin::saved_origin;
use mosaic_program_fill::binding::{BindingMatch, match_bindings, superseded};
use mosaic_program_fill::identity::{CapturedIdentity, exe_name_of};
use mosaic_program_fill::suggest::{MAX_SUGGESTIONS, suggestion_terms, suggests};
use mosaic_vault::session::{ItemFilter, ItemRef, UnlockedSession, VaultProtection};

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::ids::{WireItemId, WireVaultId};
use crate::contract::picker::{LinkAgainReason, PickerLogin};

/// The most logins a list shows as linked to one program.
pub const MAX_MATCHES: usize = 50;
/// The most logins one search returns.
pub const MAX_SEARCH: usize = 20;
/// The longest search text, in characters.
pub const MAX_QUERY_CHARS: usize = 128;

/// A login linked to the program at the field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Match {
    /// What the list shows.
    pub login: PickerLogin,
    /// Whether every matching binding presses Enter after the fill.
    pub submit: bool,
}

/// What the picker offers for one program.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Offer {
    /// The linked logins, by title.
    pub matches: Vec<Match>,
    /// A login was linked to this program before its signer or file changed.
    pub link_again: Option<LinkAgainReason>,
    /// Logins that might belong to it, when none is linked.
    pub suggestions: Vec<PickerLogin>,
}

fn live_logins(session: &UnlockedSession) -> Vec<ItemRef<'_>> {
    session
        .items(&ItemFilter::default())
        .into_iter()
        .filter(|entry| {
            !entry.item.is_deleted()
                && entry.item.item_type() == Some(ItemType::Login)
                && !matches!(
                    session.vault_protection(entry.vault_id),
                    Ok(VaultProtection::HardwareKey { open: false })
                )
        })
        .collect()
}

fn vault_names(session: &UnlockedSession) -> HashMap<VaultId, String> {
    session
        .vaults()
        .into_iter()
        .filter_map(|v| v.name.map(|n| (v.vault_id, n.to_owned())))
        .collect()
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty()).map(str::to_owned)
}

fn url_values(item: &Item) -> Vec<LoginUrl> {
    item.urls().into_iter().map(|(_, url)| url).collect()
}

fn web_hosts(urls: &[LoginUrl]) -> Vec<String> {
    urls.iter()
        .filter_map(|url| saved_origin(url.url.as_str()))
        .map(|origin| origin.host_str())
        .collect()
}

fn picker_login(
    entry: &ItemRef<'_>,
    urls: &[LoginUrl],
    names: &HashMap<VaultId, String>,
) -> PickerLogin {
    PickerLogin {
        vault_id: WireVaultId::from(entry.vault_id),
        item_id: WireItemId::from(entry.item_id),
        title: entry.item.title().unwrap_or_default().to_owned(),
        username: non_empty(entry.item.text(TextField::Username)),
        vault_name: names.get(&entry.vault_id).cloned(),
        host: web_hosts(urls).into_iter().next(),
    }
}

fn sort_key(login: &PickerLogin) -> (String, String) {
    (login.title.to_lowercase(), login.item_id.get().to_string())
}

/// The offer for `identity`: linked logins (at most [`MAX_MATCHES`], by title), the link-again
/// reason, and, when nothing is linked, up to [`MAX_SUGGESTIONS`] suggestions.
#[must_use]
pub fn build(session: &UnlockedSession, identity: &CapturedIdentity) -> Offer {
    let names = vault_names(session);
    let terms = suggestion_terms(identity);
    let mut offer = Offer::default();
    let mut suggestions = Vec::new();
    for entry in live_logins(session) {
        let urls = url_values(entry.item);
        match match_bindings(&urls, identity) {
            BindingMatch::Matched(indices) => {
                let submit = indices
                    .iter()
                    .filter_map(|&i| urls.get(i))
                    .all(mosaic_program_fill::binding::submit_enabled);
                offer.matches.push(Match {
                    login: picker_login(&entry, &urls, &names),
                    submit,
                });
            }
            BindingMatch::LinkAgain(change) => {
                offer.link_again.get_or_insert(match change {
                    BindingChange::SignerChanged => LinkAgainReason::SignerChanged,
                    BindingChange::FileChanged => LinkAgainReason::FileChanged,
                });
            }
            BindingMatch::NoBinding => {
                let hosts = web_hosts(&urls);
                let host_refs: Vec<&str> = hosts.iter().map(String::as_str).collect();
                if suggests(&terms, entry.item.title().unwrap_or_default(), &host_refs) {
                    suggestions.push(picker_login(&entry, &urls, &names));
                }
            }
        }
    }
    offer.matches.sort_by_cached_key(|m| sort_key(&m.login));
    offer.matches.truncate(MAX_MATCHES);
    if offer.matches.is_empty() {
        suggestions.sort_by_cached_key(sort_key);
        suggestions.truncate(MAX_SUGGESTIONS);
        offer.suggestions = suggestions;
    } else {
        offer.link_again = None;
    }
    offer
}

/// Logins whose title, user name or web host contains `query` (case-insensitive), at most
/// [`MAX_SEARCH`], by title.
///
/// # Errors
/// `invalidInput` for a query over [`MAX_QUERY_CHARS`] characters.
pub fn search(session: &UnlockedSession, query: &str) -> IpcResult<Vec<PickerLogin>> {
    let query = query.trim();
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let needle = query.to_lowercase();
    let names = vault_names(session);
    let mut found: Vec<PickerLogin> = live_logins(session)
        .into_iter()
        .filter_map(|entry| {
            let urls = url_values(entry.item);
            let login = picker_login(&entry, &urls, &names);
            let hit = login.title.to_lowercase().contains(&needle)
                || login
                    .username
                    .as_deref()
                    .is_some_and(|u| u.to_lowercase().contains(&needle))
                || web_hosts(&urls)
                    .iter()
                    .any(|h| h.to_lowercase().contains(&needle));
            hit.then_some(login)
        })
        .collect();
    found.sort_by_cached_key(sort_key);
    found.truncate(MAX_SEARCH);
    Ok(found)
}

/// The login `vault`/`item` as the list shows it.
#[must_use]
pub fn login_of(session: &UnlockedSession, vault: VaultId, item: ItemId) -> Option<PickerLogin> {
    let item_ref = ItemRef {
        vault_id: vault,
        item_id: item,
        item: session.item(vault, item).ok()?,
    };
    let urls = url_values(item_ref.item);
    Some(picker_login(&item_ref, &urls, &vault_names(session)))
}

/// What a link needs to know about the login and the user's other bindings.
pub struct LinkContext {
    /// The login's saved URLs.
    pub saved: Vec<LoginUrl>,
    /// The exe names of the user's other `signed/` and `path/` bindings, without those a link
    /// of this program replaces.
    pub bound_exes: Vec<String>,
    /// The hash prefix of the old binding when its file changed.
    pub old_hash: Option<[u8; 32]>,
}

/// The login to link to: a live login in a vault the account may write.
///
/// # Errors
/// `invalidInput`.
pub fn link_context(
    session: &UnlockedSession,
    vault: VaultId,
    item: ItemId,
    identity: &CapturedIdentity,
) -> IpcResult<LinkContext> {
    let entry = session.item(vault, item).map_err(|_| invalid())?;
    if entry.is_deleted()
        || entry.trashed_at().is_some()
        || entry.item_type() != Some(ItemType::Login)
    {
        return Err(invalid());
    }
    if !matches!(
        session.vault_own_role(vault),
        Ok(Role::Owner | Role::Manage | Role::Write)
    ) {
        return Err(invalid());
    }
    let saved = url_values(entry);
    let observed = identity.as_identity();
    let old_hash = saved
        .iter()
        .find(|url| binding_change(url, &observed) == Some(BindingChange::FileChanged))
        .and_then(|url| match ProgramUrl::parse(url.url.as_str()) {
            Ok(ProgramUrl::Path { sha256, .. }) => Some(sha256),
            _ => None,
        });
    let mut bound_exes = Vec::new();
    for other in live_logins(session) {
        let urls = url_values(other.item);
        let skip = superseded(&urls, identity);
        for (index, url) in urls.iter().enumerate() {
            if skip.contains(&index) {
                continue;
            }
            match ProgramUrl::parse(url.url.as_str()) {
                Ok(ProgramUrl::Signed { exe, .. }) => bound_exes.push(exe),
                Ok(ProgramUrl::Path { path, .. }) => bound_exes.extend(exe_name_of(&path)),
                _ => {}
            }
        }
    }
    bound_exes.sort();
    bound_exes.dedup();
    Ok(LinkContext {
        saved,
        bound_exes,
        old_hash,
    })
}

fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidInput)
}

/// What a fill writes.
pub struct Credentials {
    /// The user name.
    pub username: Option<String>,
    /// The password, when the login has one.
    pub password: Option<SecretString>,
    /// Whether every binding that matches the program presses Enter after the fill.
    pub submit: bool,
}

/// The credentials of a login for a fill. With `identity` the login must still be linked to
/// that program (an edit between the list and the pick can remove the link).
///
/// # Errors
/// `itemNotFound` (gone, trashed or not a login), `pickerStale` (no longer linked),
/// `internal`.
pub fn credentials(
    session: &UnlockedSession,
    vault: VaultId,
    item: ItemId,
    identity: Option<&CapturedIdentity>,
) -> IpcResult<Credentials> {
    let entry = session
        .item(vault, item)
        .map_err(|_| IpcError::new(ErrorCode::ItemNotFound))?;
    if entry.is_deleted()
        || entry.trashed_at().is_some()
        || entry.item_type() != Some(ItemType::Login)
        || matches!(
            session.vault_protection(vault),
            Ok(VaultProtection::HardwareKey { open: false })
        )
    {
        return Err(IpcError::new(ErrorCode::ItemNotFound));
    }
    let mut submit = true;
    if let Some(identity) = identity {
        let urls = url_values(entry);
        let BindingMatch::Matched(indices) = match_bindings(&urls, identity) else {
            return Err(IpcError::new(ErrorCode::PickerStale));
        };
        submit = indices
            .iter()
            .filter_map(|&i| urls.get(i))
            .all(mosaic_program_fill::binding::submit_enabled);
    }
    let password = entry
        .secret(SecretField::Password)
        .map_err(|_| IpcError::INTERNAL)?
        .filter(|p| !p.expose_secret().is_empty());
    Ok(Credentials {
        username: non_empty(entry.text(TextField::Username)),
        password,
        submit,
    })
}
