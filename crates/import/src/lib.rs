//! Importers for Enpass JSON, Bitwarden JSON, KeePass KDBX 4, 1Password 1PUX
//! and CSV (brief §5 "Import/export").
//!
//! Each importer reads an export file into an [`Import`]: one
//! [`ImportEntry`] per item, each an [`ItemDraft`] that becomes a
//! `mosaic-core` [`mosaic_core::Item`] through [`ItemDraft::apply`], with
//! per-entry [`EntryWarning`]s, plus the entries that could not be read
//! ([`SkippedEntry`]). [`Import::preview`] counts types and attachments and
//! marks duplicates of items already in the vault (same site and user name,
//! or same type, title and user name); [`commit`] adds the entries the user
//! kept to a vault of an [`mosaic_vault::session::UnlockedSession`].
//!
//! ```no_run
//! # fn f(bytes: &[u8], session: &mut mosaic_vault::session::UnlockedSession,
//! #      vault: mosaic_core::VaultId) -> Result<(), Box<dyn std::error::Error>> {
//! use mosaic_import::{Format, commit};
//! use mosaic_vault::session::ItemFilter;
//!
//! let import = mosaic_import::parse(Format::Enpass, bytes, None)?;
//! let preview = import.preview(&session.items(&ItemFilter::default()));
//! // Show the preview; the user unticks duplicates.
//! let skip: Vec<u32> = preview
//!     .entries
//!     .iter()
//!     .filter(|e| e.duplicate_of.is_some())
//!     .map(|e| e.index)
//!     .collect();
//! let report = commit(import, session, vault, &skip)?;
//! # let _ = report; Ok(()) }
//! ```
//!
//! # Security
//!
//! - Export files are untrusted input (THREAT_MODEL §4.1 "D"): every
//!   parser is bounded ([`MAX_FILE_LEN`], [`MAX_ENTRIES`], nesting and
//!   decompression limits, KDBX key-derivation limits in [`kdbx`]) and
//!   fuzzed (`crates/import/fuzz`).
//! - Plaintext from the file lives in zeroizing buffers: the decrypted and
//!   decompressed KDBX payload, archive members, decoded base64, the JSON
//!   tree, every draft field. Residual: `serde_json`'s scratch copy of
//!   escaped strings (THREAT_MODEL R14), `quick-xml`'s copies of text that
//!   needs entity or line-end handling, and the `zip` crate's internal
//!   inflate buffers.
//! - Errors and warnings carry no value from the file.
//! - KDBX reading uses AES-256-CBC, `ChaCha20`, `Salsa20`, AES-KDF and
//!   Argon2d/Argon2id only to *read* the user's KeePass file; none of them
//!   protects Mosaic data (ADR draft "KDBX import primitives").
//!
//! Mosaic is pre-audit.
#![forbid(unsafe_code)]

mod buf;
mod commit;
mod draft;
mod entries;
mod error;
mod json;
mod otp;
mod preview;
mod url;

pub mod bitwarden;
pub mod csv;
pub mod enpass;
pub mod kdbx;
pub mod onepux;

use mosaic_core::SecretString;

pub use commit::{CommitReport, commit};
pub use draft::{
    Attachment, EntryWarning, ItemDraft, MAX_ATTACHMENTS_PER_ENTRY, MAX_FIELDS_PER_ENTRY,
    MAX_IMPORT_ATTACHMENT_BYTES,
};
pub use entries::{Import, ImportEntry, MAX_ENTRIES, SkipReason, SkippedEntry};
pub use error::{Error, Result};
pub use preview::{Duplicate, ImportWarning, Preview, PreviewEntry};

/// Largest export file any importer reads: 256 MiB.
pub const MAX_FILE_LEN: usize = 256 * 1024 * 1024;

/// The export formats (brief §5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Format {
    /// Enpass JSON.
    Enpass,
    /// Bitwarden JSON (unencrypted export).
    Bitwarden,
    /// KeePass KDBX 4.
    Keepass,
    /// 1Password 1PUX.
    OnePassword,
    /// CSV with a header row; columns are mapped automatically
    /// ([`csv::CsvMapping::detect`]).
    Csv,
}

/// Reads an export file of `format`. `password` opens a KDBX file and is
/// ignored by the other formats.
///
/// # Errors
/// [`Error::TooLarge`] above [`MAX_FILE_LEN`]; [`Error::PasswordRequired`]
/// for KDBX without a password; otherwise the format's errors.
pub fn parse(format: Format, input: &[u8], password: Option<&SecretString>) -> Result<Import> {
    if input.len() > MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    match format {
        Format::Enpass => enpass::parse(input),
        Format::Bitwarden => bitwarden::parse(input),
        Format::Keepass => kdbx::parse(input, password.ok_or(Error::PasswordRequired)?),
        Format::OnePassword => onepux::parse(input),
        Format::Csv => csv::parse(input, None),
    }
}

/// Entry points for the fuzz targets only. Compiled only under
/// `cfg(fuzzing)` (set by cargo-fuzz).
#[cfg(fuzzing)]
#[doc(hidden)]
pub mod fuzzing {
    /// Parses a decrypted, decompressed KDBX 4 payload (inner header and
    /// XML).
    ///
    /// # Errors
    /// As [`crate::kdbx::parse`] after decryption.
    pub fn kdbx_payload(payload: &[u8]) -> crate::Result<crate::Import> {
        crate::kdbx::parse_payload(payload)
    }
}
