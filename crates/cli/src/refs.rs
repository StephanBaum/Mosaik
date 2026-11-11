//! References (spec §3.1): `mosaic://<vault>/<item>[/<field>]`, where a vault or item part is a
//! name (percent-encoded, NFC, exact match) or a UUID (32 hex digits or hyphenated). `get` and
//! `totp` also take a bare item UUID or a query that matches exactly one item. More than one match
//! is exit 7 with the candidates' vault, title and ID (never a secret).

use mosaic_core::{ItemId, VaultId};
use mosaic_vault::session::{TrashFilter, UnlockedSession};
use unicode_normalization::UnicodeNormalization as _;

use crate::error::{CliError, EXIT_AMBIGUOUS, EXIT_NOT_FOUND, EXIT_VAULT_LOCKED, Result};
use crate::i18n::Msg;
use crate::model::{item_filter, vault_names};

/// A resolved reference.
pub struct Target {
    /// The vault.
    pub vault: VaultId,
    /// The item.
    pub item: ItemId,
    /// The field part, if the reference had one.
    pub field: Option<String>,
}

const SCHEME: &str = "mosaic://";

/// A part of an `inject` template.
#[derive(Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    /// Literal text, copied through.
    Text(&'a str),
    /// The trimmed `mosaic://` reference of a `{{ ... }}` placeholder.
    Reference(&'a str),
}

/// Splits an `inject` template into literal text and `{{ mosaic://... }}` references. Pure and
/// total: an unclosed `{{` and a placeholder that is not a `mosaic://` reference stay literal.
/// The segments always concatenate (with each reference re-wrapped) back to the input's text.
#[must_use]
pub fn scan_template(template: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some((before, after)) = rest.split_once("{{") {
        out.push(Segment::Text(before));
        let Some((inner, tail)) = after.split_once("}}") else {
            out.push(Segment::Text("{{"));
            rest = after;
            continue;
        };
        let reference = inner.trim();
        if reference.starts_with(SCHEME) {
            out.push(Segment::Reference(reference));
        } else {
            out.push(Segment::Text("{{"));
            out.push(Segment::Text(inner));
            out.push(Segment::Text("}}"));
        }
        rest = tail;
    }
    out.push(Segment::Text(rest));
    out
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

fn invalid(text: &str) -> CliError {
    CliError::new("invalidRef", crate::error::EXIT_USAGE, Msg::ErrInvalidRef).with("ref", text)
}

fn ambiguous(text: &str, candidates: &[serde_json::Value]) -> CliError {
    let mut err =
        CliError::new("ambiguousRef", EXIT_AMBIGUOUS, Msg::ErrAmbiguousRef).with("ref", text);
    err.extra = Some(serde_json::json!({ "candidates": candidates }));
    err
}

fn candidate(session: &UnlockedSession, vault: VaultId, item: ItemId) -> serde_json::Value {
    let names = vault_names(session);
    let title = session
        .item(vault, item)
        .ok()
        .and_then(|i| i.title())
        .unwrap_or_default()
        .to_owned();
    serde_json::json!({
        "vault": names.get(&vault).cloned().unwrap_or_default(),
        "title": title,
        "id": item.to_string(),
    })
}

fn resolve_vault(session: &UnlockedSession, part: &str, text: &str) -> Result<VaultId> {
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
        [] => Err(CliError::not_found(text)),
        [one] if !one.available => Err(CliError::new(
            "vaultUnavailable",
            EXIT_VAULT_LOCKED,
            Msg::ErrVaultUnavailable,
        )),
        [one] => Ok(one.vault_id),
        _ => Err(ambiguous(text, &[])),
    }
}

/// Resolves a vault named by `--vault` (a name or a UUID).
///
/// # Errors
/// `notFound`, `ambiguousRef`, `vaultUnavailable`.
pub fn vault(session: &UnlockedSession, spec: &str) -> Result<VaultId> {
    let decoded = percent_decode(spec).unwrap_or_else(|| spec.to_owned());
    resolve_vault(session, &decoded, spec)
}

fn items_in(session: &UnlockedSession, vault: Option<VaultId>) -> Vec<(VaultId, ItemId, String)> {
    session
        .items(&item_filter(vault, TrashFilter::Exclude, false))
        .into_iter()
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
    text: &str,
) -> Result<(VaultId, ItemId)> {
    match matches {
        [] => Err(CliError::not_found(text)),
        [one] => Ok(*one),
        many => {
            let found: Vec<_> = many
                .iter()
                .map(|(v, i)| candidate(session, *v, *i))
                .collect();
            Err(ambiguous(text, &found))
        }
    }
}

fn find_item(
    session: &UnlockedSession,
    vault: Option<VaultId>,
    part: &str,
    text: &str,
) -> Result<(VaultId, ItemId)> {
    let all = items_in(session, vault);
    if let Some(id) = uuid_bytes(part).and_then(|b| ItemId::from_bytes(b).ok()) {
        let hits: Vec<_> = all
            .iter()
            .filter(|(_, i, _)| *i == id)
            .map(|(v, i, _)| (*v, *i))
            .collect();
        return pick(session, &hits, text);
    }
    let exact: Vec<_> = all
        .iter()
        .filter(|(_, _, t)| t.as_str() == part)
        .map(|(v, i, _)| (*v, *i))
        .collect();
    if !exact.is_empty() || vault.is_some() {
        return pick(session, &exact, text);
    }
    // A bare query: exactly one search hit.
    let hits: Vec<_> = session
        .search(part)
        .into_iter()
        .map(|r| (r.vault_id, r.item_id))
        .collect();
    pick(session, &hits, text)
}

/// Resolves a reference to an item (and a field, if it names one).
///
/// # Errors
/// `invalidRef` (exit 2), `notFound` (4), `ambiguousRef` (7), `vaultUnavailable` (8).
pub fn resolve(session: &UnlockedSession, text: &str) -> Result<Target> {
    let Some(rest) = text.strip_prefix(SCHEME) else {
        let part = percent_decode(text).unwrap_or_else(|| text.nfc().collect());
        let (vault, item) = find_item(session, None, &part, text)?;
        return Ok(Target {
            vault,
            item,
            field: None,
        });
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let (vault_part, item_part, field_part) = match parts.as_slice() {
        [v, i] => (*v, *i, None),
        [v, i, f] => (*v, *i, Some(*f)),
        _ => return Err(invalid(text)),
    };
    if vault_part.is_empty() || item_part.is_empty() || field_part.is_some_and(str::is_empty) {
        return Err(invalid(text));
    }
    let decode = |p: &str| percent_decode(p).ok_or_else(|| invalid(text));
    let vault = resolve_vault(session, &decode(vault_part)?, text)?;
    let (_, item) = find_item(session, Some(vault), &decode(item_part)?, text)?;
    Ok(Target {
        vault,
        item,
        field: field_part.map(decode).transpose()?,
    })
}

/// Exit code of a missing field.
#[must_use]
pub fn no_such_field(field: &str) -> CliError {
    CliError::new("noSuchField", EXIT_NOT_FOUND, Msg::ErrNoSuchField).with("field", field)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn scan_template_splits_references_and_keeps_the_rest_literal() {
        use Segment::{Reference, Text};
        assert_eq!(
            scan_template("a {{ mosaic://V/I/f }} b"),
            vec![Text("a "), Reference("mosaic://V/I/f"), Text(" b")]
        );
        assert_eq!(
            scan_template("{{ other }}"),
            vec![Text(""), Text("{{"), Text(" other "), Text("}}"), Text("")]
        );
        assert_eq!(
            scan_template("x {{ mosaic://V/I"),
            vec![Text("x "), Text("{{"), Text(" mosaic://V/I")]
        );
        assert_eq!(scan_template(""), vec![Text("")]);
    }
}
