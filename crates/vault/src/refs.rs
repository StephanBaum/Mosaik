//! References (CLI spec §3.1, agent spec §4): `mosaic://<vault>/<item>[/<field>]`, where a vault
//! or item part is a name (percent-encoded, NFC, exact match) or a UUID (32 hex digits or
//! hyphenated). The CLI also takes a bare item UUID or a query that matches exactly one item;
//! agents take only the full three-part form (see [`resolve_agent`]).
//!
//! Errors never carry the reference text. Candidates of an ambiguous match carry the vault name,
//! the title and the ID of each item, never a secret.

use std::collections::BTreeMap;

use mosaic_core::{ItemId, VaultId};
use unicode_normalization::UnicodeNormalization as _;

use crate::session::{ItemFilter, TrashFilter, UnlockedSession};

const SCHEME: &str = "mosaic://";

/// A resolved reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The vault.
    pub vault: VaultId,
    /// The item.
    pub item: ItemId,
    /// The field part, if the reference had one.
    pub field: Option<String>,
}

/// One of several items a reference could mean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefCandidate {
    /// The vault's name (its ID when it has none).
    pub vault: String,
    /// The item's title.
    pub title: String,
    /// The item.
    pub id: ItemId,
}

/// Why a reference did not resolve.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RefError {
    /// Not a well-formed reference.
    #[error("invalid reference")]
    Invalid,
    /// Nothing matches.
    #[error("nothing matches the reference")]
    NotFound,
    /// More than one match (an empty list for an ambiguous vault name).
    #[error("the reference is ambiguous")]
    Ambiguous(Vec<RefCandidate>),
    /// The vault exists but its key is not available.
    #[error("the vault is unavailable")]
    VaultUnavailable,
}

fn uuid_bytes(s: &str) -> Option<[u8; 16]> {
    let compact: String = s.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 || !compact.is_ascii() || (s.len() != 32 && s.len() != 36) {
        return None;
    }
    let mut out = [0u8; 16];
    for (byte, pair) in out.iter_mut().zip(compact.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        if b == b'%' {
            let pair = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(pair, 16).ok()?);
            i += 3;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).ok().map(|d| d.nfc().collect())
}

fn vault_names(session: &UnlockedSession) -> BTreeMap<VaultId, String> {
    session
        .vaults()
        .into_iter()
        .map(|v| {
            (
                v.vault_id,
                v.name.map_or_else(|| v.vault_id.to_string(), str::to_owned),
            )
        })
        .collect()
}

fn candidate(session: &UnlockedSession, vault: VaultId, item: ItemId) -> RefCandidate {
    let names = vault_names(session);
    let title = session
        .item(vault, item)
        .ok()
        .and_then(|i| i.title())
        .unwrap_or_default()
        .to_owned();
    RefCandidate {
        vault: names.get(&vault).cloned().unwrap_or_default(),
        title,
        id: item,
    }
}

fn resolve_vault_part(session: &UnlockedSession, part: &str) -> Result<VaultId, RefError> {
    let by_id = uuid_bytes(part).and_then(|b| VaultId::from_bytes(b).ok());
    let found: Vec<_> = session
        .vaults()
        .into_iter()
        .filter(|v| match by_id {
            Some(id) => v.vault_id == id,
            None => v.name.is_some_and(|n| n.nfc().eq(part.nfc())),
        })
        .collect();
    match found.as_slice() {
        [] => Err(RefError::NotFound),
        [one] if !one.available => Err(RefError::VaultUnavailable),
        [one] => Ok(one.vault_id),
        _ => Err(RefError::Ambiguous(Vec::new())),
    }
}

/// Resolves a vault named by a name or a UUID (percent-encoded names are decoded).
///
/// # Errors
/// [`RefError::NotFound`], [`RefError::Ambiguous`], [`RefError::VaultUnavailable`].
pub fn resolve_vault(session: &UnlockedSession, spec: &str) -> Result<VaultId, RefError> {
    let decoded = percent_decode(spec).unwrap_or_else(|| spec.to_owned());
    resolve_vault_part(session, &decoded)
}

type Visible<'a> = &'a dyn Fn(VaultId, ItemId) -> bool;

fn items_in(
    session: &UnlockedSession,
    vault: Option<VaultId>,
    visible: Visible<'_>,
) -> Vec<(VaultId, ItemId, String)> {
    let filter = ItemFilter {
        vault,
        trash: TrashFilter::Exclude,
        ..ItemFilter::default()
    };
    session
        .items(&filter)
        .into_iter()
        .filter(|r| visible(r.vault_id, r.item_id))
        .map(|r| {
            (
                r.vault_id,
                r.item_id,
                r.item.title().unwrap_or_default().nfc().collect(),
            )
        })
        .collect()
}

fn pick(
    session: &UnlockedSession,
    matches: &[(VaultId, ItemId)],
) -> Result<(VaultId, ItemId), RefError> {
    match matches {
        [] => Err(RefError::NotFound),
        [one] => Ok(*one),
        many => Err(RefError::Ambiguous(
            many.iter()
                .map(|(v, i)| candidate(session, *v, *i))
                .collect(),
        )),
    }
}

fn find_item(
    session: &UnlockedSession,
    vault: Option<VaultId>,
    part: &str,
    visible: Visible<'_>,
    allow_search: bool,
) -> Result<(VaultId, ItemId), RefError> {
    let all = items_in(session, vault, visible);
    if let Some(id) = uuid_bytes(part).and_then(|b| ItemId::from_bytes(b).ok()) {
        let hits: Vec<_> = all
            .iter()
            .filter(|(_, i, _)| *i == id)
            .map(|(v, i, _)| (*v, *i))
            .collect();
        return pick(session, &hits);
    }
    let exact: Vec<_> = all
        .iter()
        .filter(|(_, _, t)| t.as_str() == part)
        .map(|(v, i, _)| (*v, *i))
        .collect();
    if !exact.is_empty() || vault.is_some() || !allow_search {
        return pick(session, &exact);
    }
    // A bare query: exactly one search hit.
    let hits: Vec<_> = session
        .search(part)
        .into_iter()
        .map(|r| (r.vault_id, r.item_id))
        .collect();
    pick(session, &hits)
}

type Parts = (String, String, Option<String>);

fn split_scheme(text: &str) -> Result<Parts, RefError> {
    let rest = text.strip_prefix(SCHEME).ok_or(RefError::Invalid)?;
    let parts: Vec<&str> = rest.split('/').collect();
    let (vault_part, item_part, field_part) = match parts.as_slice() {
        [v, i] => (*v, *i, None),
        [v, i, f] => (*v, *i, Some(*f)),
        _ => return Err(RefError::Invalid),
    };
    if vault_part.is_empty() || item_part.is_empty() || field_part.is_some_and(str::is_empty) {
        return Err(RefError::Invalid);
    }
    let decode = |p: &str| percent_decode(p).ok_or(RefError::Invalid);
    Ok((
        decode(vault_part)?,
        decode(item_part)?,
        field_part.map(decode).transpose()?,
    ))
}

/// Resolves a reference to an item (and a field, if it names one): the CLI's semantics (a bare
/// query, a bare item UUID, or `mosaic://<vault>/<item>[/<field>]`).
///
/// # Errors
/// [`RefError`].
pub fn resolve(session: &UnlockedSession, text: &str) -> Result<Target, RefError> {
    let all: Visible<'_> = &|_, _| true;
    if !text.starts_with(SCHEME) {
        let part = percent_decode(text).unwrap_or_else(|| text.nfc().collect());
        let (vault, item) = find_item(session, None, &part, all, true)?;
        return Ok(Target {
            vault,
            item,
            field: None,
        });
    }
    let (vault_part, item_part, field) = split_scheme(text)?;
    let vault = resolve_vault_part(session, &vault_part)?;
    let (_, item) = find_item(session, Some(vault), &item_part, all, false)?;
    Ok(Target { vault, item, field })
}

/// Resolves an agent's reference: only the `mosaic://<vault>/<item>/<field>` form (the field is
/// required; no bare query, no search fallback). Items for which `visible` is false do not exist
/// for it: neither as a match nor as a candidate of an ambiguous match.
///
/// # Errors
/// [`RefError`].
pub fn resolve_agent(
    session: &UnlockedSession,
    text: &str,
    visible: &dyn Fn(VaultId, ItemId) -> bool,
) -> Result<Target, RefError> {
    let (vault_part, item_part, field) = split_scheme(text)?;
    let field = field.ok_or(RefError::Invalid)?;
    let vault = resolve_vault_part(session, &vault_part)?;
    let (_, item) = find_item(session, Some(vault), &item_part, visible, false)?;
    Ok(Target {
        vault,
        item,
        field: Some(field),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::LockedSession;
    use crate::storage::Store;
    use mosaic_core::model::ItemType;
    use mosaic_core::{Argon2Profile, MasterPassword};

    fn session() -> UnlockedSession {
        let (s, _) = LockedSession::open(Store::open_in_memory().unwrap())
            .create_account(
                &MasterPassword::new("unit test password".to_owned()),
                Argon2Profile::Baseline,
            )
            .unwrap();
        s
    }

    fn add(s: &mut UnlockedSession, v: VaultId, title: &str) -> ItemId {
        s.add_item(v, &ItemType::Login, |e| e.set_title(title))
            .unwrap()
    }

    #[test]
    fn uuids_parse_in_both_forms() {
        let plain = "0123456789abcdef0123456789abcdef";
        let hyphen = "01234567-89ab-cdef-0123-456789abcdef";
        assert_eq!(uuid_bytes(plain), uuid_bytes(hyphen));
        assert!(uuid_bytes(plain).is_some());
        assert!(uuid_bytes("0123456789abcdef0123456789abcde").is_none());
        assert!(uuid_bytes("My Login").is_none());
    }

    #[test]
    fn names_are_percent_decoded_and_normalized() {
        assert_eq!(percent_decode("My%20Bank").as_deref(), Some("My Bank"));
        assert_eq!(percent_decode("e%CC%81").as_deref(), Some("\u{e9}"));
        assert!(percent_decode("bad%zz").is_none());
        assert!(percent_decode("%ff").is_none());
    }

    #[test]
    fn agent_refs_need_a_field() {
        let mut s = session();
        let v = s.create_vault("Work").unwrap();
        add(&mut s, v, "Key");
        assert_eq!(
            resolve_agent(&s, "mosaic://Work/Key", &|_, _| true),
            Err(RefError::Invalid)
        );
        assert_eq!(
            resolve_agent(&s, "mosaic://Work/Key/", &|_, _| true),
            Err(RefError::Invalid)
        );
        assert!(resolve_agent(&s, "mosaic://Work/Key/api_key.secret", &|_, _| true).is_ok());
    }

    #[test]
    fn agent_refs_skip_invisible_items() {
        let mut store = session();
        let vault = store.create_vault("Work").unwrap();
        let first = add(&mut store, vault, "Key");
        let second = add(&mut store, vault, "Key");
        let reference = "mosaic://Work/Key/api_key.secret";
        let only_a = resolve_agent(&store, reference, &|_, i| i == first).unwrap();
        assert_eq!((only_a.vault, only_a.item), (vault, first));
        assert_eq!(
            resolve_agent(&store, reference, &|_, _| false),
            Err(RefError::NotFound)
        );
        match resolve_agent(&store, reference, &|_, _| true) {
            Err(RefError::Ambiguous(c)) => {
                let mut ids: Vec<_> = c.iter().map(|c| c.id.to_string()).collect();
                ids.sort();
                let mut want = vec![first.to_string(), second.to_string()];
                want.sort();
                assert_eq!(ids, want);
            }
            other => panic!("expected ambiguous, got {other:?}"),
        }
        // Only visible items appear as candidates.
        let third = add(&mut store, vault, "Key");
        match resolve_agent(&store, reference, &|_, i| i != third) {
            Err(RefError::Ambiguous(cs)) => assert_eq!(cs.len(), 2),
            other => panic!("expected ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn agent_refs_take_uuids_and_names() {
        let mut s = session();
        let v = s.create_vault("Work").unwrap();
        let i = add(&mut s, v, "My Key");
        let by_name = resolve_agent(&s, "mosaic://Work/My%20Key/f", &|_, _| true).unwrap();
        let by_id = resolve_agent(&s, &format!("mosaic://{v}/{i}/f"), &|_, _| true).unwrap();
        assert_eq!(by_name, by_id);
        assert_eq!(by_id.item, i);
    }

    #[test]
    fn bare_query_is_not_an_agent_ref() {
        let mut s = session();
        let v = s.create_vault("Work").unwrap();
        let i = add(&mut s, v, "Key");
        assert_eq!(
            resolve_agent(&s, "Key", &|_, _| true),
            Err(RefError::Invalid)
        );
        assert_eq!(
            resolve_agent(&s, &i.to_string(), &|_, _| true),
            Err(RefError::Invalid)
        );
        // The CLI form still takes both.
        assert_eq!(resolve(&s, "Key").unwrap().item, i);
        assert_eq!(resolve(&s, &i.to_string()).unwrap().item, i);
    }
}
