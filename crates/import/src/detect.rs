//! Telling what an export file is from its content, never from its name.
//!
//! [`detect`] reads at most the structure needed to decide: the first bytes,
//! the member names of a zip archive, the top-level keys of a JSON document
//! or the header row of a CSV file. It is bounded like the parsers
//! ([`crate::MAX_FILE_LEN`], zip member count, JSON depth and node limits)
//! and returns `None` for anything it does not recognize.

use crate::Format;
use crate::archive;
use crate::csv::{self, CsvMapping, CsvPreset};
use crate::json::{self, Json};
use crate::onepux::MAX_ARCHIVE_MEMBERS;
use crate::proton_pass;

/// The KDBX signature: both base signatures of KeePass 2 files.
const KDBX_SIGNATURE: [u8; 8] = [0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5];
const PGP_ARMOR: &[u8] = b"-----BEGIN PGP MESSAGE-----";

/// The product an export file comes from, as far as its content tells.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum Source {
    /// Chrome, Edge, Brave and other Chromium browsers (CSV).
    Chromium,
    /// Firefox (CSV).
    Firefox,
    /// Safari and the macOS Passwords app (CSV).
    Safari,
    /// Bitwarden CSV.
    BitwardenCsv,
    /// Bitwarden JSON.
    Bitwarden,
    /// Enpass JSON.
    Enpass,
    /// A KeePass KDBX file.
    KeePass,
    /// `KeePassXC` CSV.
    KeePassXcCsv,
    /// 1Password 1PUX.
    OnePassword,
    /// FIDO Credential Exchange Format JSON.
    Cxf,
    /// `LastPass` CSV.
    LastPass,
    /// Dashlane CSV (the zip or an extracted `credentials.csv`).
    Dashlane,
    /// Keeper JSON.
    Keeper,
    /// `NordPass` CSV.
    NordPass,
    /// Proton Pass (zip, JSON or PGP-encrypted).
    ProtonPass,
    /// RoboForm CSV.
    RoboForm,
    /// A CSV file with a header row that matches no known product.
    Csv,
}

/// What a file is: the format to parse it with and the product it is from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Detected {
    /// The format to pass to [`crate::parse`].
    pub format: Format,
    /// The product the file comes from.
    pub source: Source,
}

/// What the file is, from its content only (never its name). `None`: not an
/// export Mosaic reads.
#[must_use]
pub fn detect(input: &[u8]) -> Option<Detected> {
    if input.is_empty() || input.len() > crate::MAX_FILE_LEN {
        return None;
    }
    let found = |format, source| Some(Detected { format, source });
    if input.starts_with(&KDBX_SIGNATURE) {
        return found(Format::Keepass, Source::KeePass);
    }
    if input.starts_with(PGP_ARMOR) {
        return found(Format::ProtonPass, Source::ProtonPass);
    }
    if input.starts_with(b"PK\x03\x04") {
        return zip(input);
    }
    let body = input.strip_prefix(b"\xef\xbb\xbf").unwrap_or(input);
    if body
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{')
    {
        return object(input);
    }
    table(input)
}

fn zip(input: &[u8]) -> Option<Detected> {
    let zip = archive::open(input, MAX_ARCHIVE_MEMBERS).ok()?;
    if archive::find(&zip, |n| n == "export.data").is_some() {
        return Some(Detected {
            format: Format::OnePassword,
            source: Source::OnePassword,
        });
    }
    if proton_pass::locate(&zip).is_some() {
        return Some(Detected {
            format: Format::ProtonPass,
            source: Source::ProtonPass,
        });
    }
    if archive::find(&zip, |n| n.to_lowercase().ends_with("credentials.csv")).is_some() {
        return Some(Detected {
            format: Format::Dashlane,
            source: Source::Dashlane,
        });
    }
    None
}

fn object(input: &[u8]) -> Option<Detected> {
    let doc = json::parse(input).ok()?;
    let is_array = |key: &str| matches!(doc.get(key), Some(Json::Array(_)));
    let (format, source) = if (doc.get("encrypted").is_some() || doc.get("passwordProtected").is_some())
        && is_array("items")
    {
        (Format::Bitwarden, Source::Bitwarden)
    } else if is_array("records") {
        (Format::Keeper, Source::Keeper)
    } else if doc.get("vaults").is_some_and(Json::is_object) {
        (Format::ProtonPass, Source::ProtonPass)
    } else if is_array("accounts") {
        (Format::Cxf, Source::Cxf)
    } else if is_array("items") {
        (Format::Enpass, Source::Enpass)
    } else {
        return None;
    };
    Some(Detected { format, source })
}

fn table(input: &[u8]) -> Option<Detected> {
    if input.contains(&0) {
        return None;
    }
    let mut rows = csv::rows(input).ok()?;
    let header = rows.next_row().ok()??;
    if header.len() < 2 {
        return None;
    }
    let names: Vec<&str> = header.iter().map(|s| s.as_str()).collect();
    let source = match CsvMapping::detect(&names).1 {
        Some(CsvPreset::Chrome) => Source::Chromium,
        Some(CsvPreset::Firefox) => Source::Firefox,
        Some(CsvPreset::Safari) => Source::Safari,
        Some(CsvPreset::Bitwarden) => Source::BitwardenCsv,
        Some(CsvPreset::LastPass) => Source::LastPass,
        Some(CsvPreset::NordPass) => Source::NordPass,
        Some(CsvPreset::RoboForm) => Source::RoboForm,
        Some(CsvPreset::KeePassXc) => Source::KeePassXcCsv,
        Some(CsvPreset::Dashlane) => Source::Dashlane,
        _ => Source::Csv,
    };
    Some(Detected {
        format: Format::Csv,
        source,
    })
}
