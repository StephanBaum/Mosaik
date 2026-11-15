//! References (spec §3.1): `mosaic://<vault>/<item>[/<field>]`, where a vault or item part is a
//! name (percent-encoded, NFC, exact match) or a UUID (32 hex digits or hyphenated). `get` and
//! `totp` also take a bare item UUID or a query that matches exactly one item. More than one match
//! is exit 7 with the candidates' vault, title and ID (never a secret).

use mosaic_core::VaultId;
use mosaic_vault::refs::RefError;
pub use mosaic_vault::refs::Target;
use mosaic_vault::session::UnlockedSession;

use crate::error::{CliError, EXIT_AMBIGUOUS, EXIT_NOT_FOUND, EXIT_VAULT_LOCKED, Result};
use crate::i18n::Msg;

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

fn invalid(text: &str) -> CliError {
    CliError::new("invalidRef", crate::error::EXIT_USAGE, Msg::ErrInvalidRef).with("ref", text)
}

/// Maps a shared resolution error to the CLI's error codes (`invalidRef` 2, `notFound` 4,
/// `ambiguousRef` 7 with candidates, `vaultUnavailable` 8); `text` is the reference as typed.
fn cli_error(err: RefError, text: &str) -> CliError {
    match err {
        RefError::Invalid => invalid(text),
        RefError::NotFound => CliError::not_found(text),
        RefError::VaultUnavailable => CliError::new(
            "vaultUnavailable",
            EXIT_VAULT_LOCKED,
            Msg::ErrVaultUnavailable,
        ),
        RefError::Ambiguous(candidates) => {
            let found: Vec<_> = candidates
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "vault": c.vault,
                        "title": c.title,
                        "id": c.id.to_string(),
                    })
                })
                .collect();
            let mut err = CliError::new("ambiguousRef", EXIT_AMBIGUOUS, Msg::ErrAmbiguousRef)
                .with("ref", text);
            err.extra = Some(serde_json::json!({ "candidates": found }));
            err
        }
    }
}

/// Resolves a vault named by `--vault` (a name or a UUID).
///
/// # Errors
/// `notFound`, `ambiguousRef`, `vaultUnavailable`.
pub fn vault(session: &UnlockedSession, spec: &str) -> Result<VaultId> {
    mosaic_vault::refs::resolve_vault(session, spec).map_err(|e| cli_error(e, spec))
}

/// Resolves a reference to an item (and a field, if it names one).
///
/// # Errors
/// `invalidRef` (exit 2), `notFound` (4), `ambiguousRef` (7), `vaultUnavailable` (8).
pub fn resolve(session: &UnlockedSession, text: &str) -> Result<Target> {
    mosaic_vault::refs::resolve(session, text).map_err(|e| cli_error(e, text))
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
