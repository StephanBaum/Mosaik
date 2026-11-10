//! `import` and `export`. Exports are encrypted only (OD-M8-4); imports show a preview first and
//! need a typed confirmation on the terminal (`--no-input` refuses).

use std::io::{BufReader, BufWriter, Read, Write as _};
use std::path::Path;

use mosaic_core::{Argon2Profile, ExportPassphrase, SecretString};
use mosaic_import::{Format, mosaic_export};
use mosaic_vault::session::{ExportSelection, TrashFilter};
use serde::Serialize;
use zeroize::Zeroizing;

use crate::app::{App, guard_rotation};
use crate::error::{CliError, EXIT_AUTH, EXIT_OTHER, Result};
use crate::i18n::Msg;
use crate::model::{item_filter, vault_names};
use crate::output::escape;
use crate::refs;
use crate::spec::Parsed;
use crate::util::{create_private, finish_private};

const MIN_PASSPHRASE_CHARS: usize = 12;

fn secret_string(mut typed: Zeroizing<String>) -> SecretString {
    SecretString::from(std::mem::take(&mut *typed))
}

fn export_unreadable() -> CliError {
    CliError::new("exportUnreadable", EXIT_AUTH, Msg::ErrExportUnreadable)
}

fn import_failed() -> CliError {
    CliError::new("importFailed", EXIT_OTHER, Msg::ErrImportFailed)
}

/// Reads an import file through `take(MAX_FILE_LEN + 1)`: a file that grows, a FIFO or a device
/// cannot make the read unbounded, and the size the file claims is not trusted.
fn read_file(path: &Path) -> Result<Zeroizing<Vec<u8>>> {
    let file = std::fs::File::open(path).map_err(|_| CliError::file_read(path))?;
    let claimed = file.metadata().map_or(0, |m| m.len());
    read_capped(file, claimed, mosaic_import::MAX_FILE_LEN as u64)
        .map_err(|e| e.unwrap_or_else(|| CliError::file_read(path)))
}

/// Reads at most `max` bytes from `reader`; more is `importFailed`. `Err(None)` is a read error.
/// `claimed` (the size the source says it has) only sizes the buffer.
fn read_capped(
    reader: impl Read,
    claimed: u64,
    max: u64,
) -> core::result::Result<Zeroizing<Vec<u8>>, Option<CliError>> {
    let limit = max.saturating_add(1);
    let mut bytes = Zeroizing::new(Vec::with_capacity(
        usize::try_from(claimed.min(limit)).unwrap_or(0),
    ));
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| None)?;
    if bytes.len() as u64 > max {
        return Err(Some(import_failed()));
    }
    Ok(bytes)
}

fn parse_file(
    app: &mut App,
    p: &Parsed,
    format: &str,
    path: &Path,
) -> Result<mosaic_import::Import> {
    let map = |e: mosaic_import::Error| match e {
        mosaic_import::Error::WrongCredentials if format == "mosaicx" => export_unreadable(),
        mosaic_import::Error::Malformed | mosaic_import::Error::UnsupportedVersion
            if format == "mosaicx" =>
        {
            export_unreadable()
        }
        mosaic_import::Error::WrongCredentials => CliError::wrong_credentials(),
        _ => import_failed(),
    };
    let chosen = match format {
        "mosaicx" => {
            let pass = secret_string(app.prompt.secret(Msg::PromptImportPassphrase, &[])?);
            let pass = ExportPassphrase::new(&pass).map_err(|_| export_unreadable())?;
            let file = std::fs::File::open(path).map_err(|_| CliError::file_read(path))?;
            // The same cap as every other import file.
            let capped = file.take(mosaic_import::MAX_FILE_LEN as u64 + 1);
            return mosaic_export::parse_reader(BufReader::new(capped), &pass).map_err(map);
        }
        "enpass" => Format::Enpass,
        "bitwarden" => Format::Bitwarden,
        "keepass" => Format::Keepass,
        "1password" => Format::OnePassword,
        "cxf" => Format::Cxf,
        _ => Format::Csv,
    };
    let password = if chosen == Format::Keepass {
        if !p.has("file-password-prompt") {
            return Err(
                CliError::usage(Msg::ErrMissingArgument).with("name", "--file-password-prompt")
            );
        }
        Some(secret_string(
            app.prompt.secret(Msg::PromptFilePassword, &[])?,
        ))
    } else {
        None
    };
    let bytes = read_file(path)?;
    mosaic_import::parse(chosen, &bytes, password.as_ref()).map_err(map)
}

#[derive(Serialize)]
struct PreviewOut {
    count: usize,
    duplicates: usize,
    skipped: usize,
    counts: std::collections::BTreeMap<String, u32>,
    attachments: u32,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ImportOut {
    imported: u32,
    skipped: u32,
    failed: u32,
    attachments: u32,
}

fn yes(answer: &str) -> bool {
    matches!(answer.to_lowercase().as_str(), "y" | "yes" | "j" | "ja")
}

/// `mosaic import`.
pub fn import(app: &mut App, p: &Parsed) -> Result<()> {
    let format = p.args.first().map_or("", String::as_str);
    let path = Path::new(p.args.get(1).map_or("", String::as_str));
    let parsed = parse_file(app, p, format, path)?;
    let mut u = app.unlock()?;
    guard_rotation(&u.session)?;
    let vault = if let Some(v) = p.value("vault") {
        refs::vault(&u.session, v)?
    } else {
        let open: Vec<_> = u
            .session
            .vaults()
            .into_iter()
            .filter(|v| v.available)
            .collect();
        match open.as_slice() {
            [one] => one.vault_id,
            _ => return Err(CliError::usage(Msg::ErrNoVaultChosen)),
        }
    };
    let name = vault_names(&u.session)
        .get(&vault)
        .cloned()
        .unwrap_or_default();
    let existing = u
        .session
        .items(&item_filter(None, TrashFilter::Exclude, false));
    let preview = parsed.preview(&existing);
    drop(existing);
    let skip: Vec<u32> = preview
        .entries
        .iter()
        .filter(|e| e.duplicate_of.is_some() || e.duplicate_in_file.is_some())
        .map(|e| e.index)
        .collect();
    let importing = preview.entries.len() - skip.len();
    let mut counts = std::collections::BTreeMap::new();
    for (kind, n) in &preview.counts {
        counts.insert(kind.as_str().to_owned(), *n);
    }
    let doc = PreviewOut {
        count: preview.entries.len(),
        duplicates: skip.len(),
        skipped: preview.skipped.len(),
        counts,
        attachments: preview.attachments,
        warnings: preview.warnings.iter().map(|w| format!("{w:?}")).collect(),
    };
    if p.has("dry-run") {
        app.out
            .data(&doc, |o| print_preview(o, &doc, &name, importing));
        app.out.say(&app.out.t(Msg::ImportDryRun, &[]));
        return Ok(());
    }
    if !app.out.json {
        print_preview(&app.out, &doc, &name, importing);
    }
    let answer = app.prompt.line(
        Msg::PromptConfirmImport,
        &[("count", importing.to_string()), ("vault", escape(&name))],
    )?;
    if !yes(&answer) {
        app.out.say(&app.out.t(Msg::ImportCancelled, &[]));
        return Ok(());
    }
    let report = mosaic_import::commit(parsed, &mut u.session, vault, &skip)?;
    let result = ImportOut {
        imported: report.imported,
        skipped: report.skipped,
        failed: report.failed,
        attachments: report.attachments,
    };
    app.out.data(&result, |o| {
        o.say(&o.t(
            Msg::ImportDone,
            &[
                ("imported", report.imported.to_string()),
                ("skipped", report.skipped.to_string()),
                ("failed", report.failed.to_string()),
            ],
        ));
    });
    Ok(())
}

fn print_preview(o: &crate::output::Output, doc: &PreviewOut, vault: &str, importing: usize) {
    let summary = doc
        .counts
        .iter()
        .map(|(k, n)| format!("{k} {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    o.say(&o.t(
        Msg::ImportPreview,
        &[
            ("count", importing.to_string()),
            ("vault", escape(vault)),
            ("summary", summary),
        ],
    ));
    if doc.duplicates > 0 {
        o.say(&o.t(
            Msg::ImportDuplicates,
            &[("count", doc.duplicates.to_string())],
        ));
    }
    if doc.skipped > 0 {
        o.say(&o.t(Msg::ImportWarnings, &[("count", doc.skipped.to_string())]));
    }
}

#[derive(Serialize)]
struct ExportOut {
    vaults: u32,
    items: u32,
    attachments: u32,
    #[serde(rename = "attachmentsLeftOut")]
    attachments_left_out: u32,
}

/// `mosaic export`.
#[expect(
    clippy::too_many_lines,
    reason = "prompt, unlock, stream, atomic move, report"
)]
pub fn export(app: &mut App, p: &Parsed) -> Result<()> {
    let dest = Path::new(p.args.first().map_or("", String::as_str));
    if dest.exists() {
        return Err(CliError::new("fileExists", EXIT_OTHER, Msg::ErrFileExists)
            .with("path", dest.display().to_string()));
    }
    let first = app.prompt.secret(Msg::PromptExportPassphrase, &[])?;
    let again = app.prompt.secret(Msg::PromptExportPassphraseAgain, &[])?;
    if *first != *again {
        return Err(CliError::new(
            "passphraseMismatch",
            EXIT_AUTH,
            Msg::ErrPassphraseMismatch,
        ));
    }
    if first.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err(
            CliError::new("passphraseWeak", EXIT_AUTH, Msg::ErrPassphraseWeak)
                .with("min", MIN_PASSPHRASE_CHARS.to_string()),
        );
    }
    drop(again);
    let pass = ExportPassphrase::new(&secret_string(first)).map_err(|_| CliError::internal())?;
    let u = app.unlock()?;
    let vaults = {
        let names = p.values("vault");
        if names.is_empty() {
            None
        } else {
            Some(
                names
                    .into_iter()
                    .map(|n| refs::vault(&u.session, n))
                    .collect::<Result<Vec<_>>>()?,
            )
        }
    };
    let selection = ExportSelection {
        vaults,
        attachments: p.has("include-attachments"),
        profile: Argon2Profile::Standard,
    };
    let preview = u.session.export_preview(&selection)?;
    if selection.attachments && !preview.items_with_attachments.is_empty() {
        app.out.warn(
            "attachmentsIncluded",
            Msg::ExportAttachmentWarn,
            &[("count", preview.items_with_attachments.len().to_string())],
        );
    }
    let (file, tmp) = create_private(dest)?;
    let mut writer = BufWriter::new(file);
    let written = u.session.write_export(&selection, &mut writer, &pass);
    let flushed = written.is_ok() && writer.flush().is_ok();
    let Ok(file) = writer.into_inner() else {
        let _ = std::fs::remove_file(&tmp);
        return Err(CliError::file_write(dest));
    };
    let report = match written {
        Ok(report) if flushed => report,
        other => {
            drop(file);
            let _ = std::fs::remove_file(&tmp);
            return Err(other
                .err()
                .map_or_else(|| CliError::file_write(dest), CliError::from));
        }
    };
    finish_private(file, &tmp, dest, true)?;
    let doc = ExportOut {
        vaults: report.vaults,
        items: report.items,
        attachments: report.attachments,
        attachments_left_out: report.attachments_left_out,
    };
    app.out.data(&doc, |o| {
        o.say(&o.t(
            Msg::ExportDone,
            &[
                ("items", report.items.to_string()),
                ("vaults", report.vaults.to_string()),
                ("path", dest.display().to_string()),
            ],
        ));
        if report.attachments_left_out > 0 {
            o.note(
                Msg::ExportAttachmentsLeftOut,
                &[("count", report.attachments_left_out.to_string())],
            );
        }
        if report.unavailable_vaults > 0 {
            o.note(
                Msg::ExportUnavailableVaults,
                &[("count", report.unavailable_vaults.to_string())],
            );
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cap_accepts_exactly_the_maximum() {
        let data = [7u8; 100];
        let got = read_capped(&data[..], 100, 100).ok().unwrap();
        assert_eq!(got.len(), 100);
    }

    #[test]
    fn read_cap_refuses_one_byte_over_even_if_the_claim_is_small() {
        let data = [7u8; 101];
        // The claimed size is not trusted: a file that grows or lies is still capped.
        let err = read_capped(&data[..], 3, 100).err().flatten().unwrap();
        assert_eq!(err.code, "importFailed");
        assert_eq!(err.exit, EXIT_OTHER);
    }

    #[test]
    fn read_cap_stops_an_endless_stream() {
        let err = read_capped(std::io::repeat(0), 0, 4096)
            .err()
            .flatten()
            .unwrap();
        assert_eq!(err.code, "importFailed");
    }

    #[test]
    fn read_cap_huge_claim_does_not_reserve_it() {
        let got = read_capped(&b"abc"[..], u64::MAX, 100).ok().unwrap();
        assert_eq!(&got[..], b"abc");
        assert!(got.capacity() <= 101);
    }
}
