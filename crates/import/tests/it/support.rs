//! Helpers shared by the integration tests.

#![expect(clippy::unwrap_used, reason = "tests")]

use std::io::Write;

use mosaic_core::model::{ItemType, SecretField, Text};
use mosaic_import::{Import, ImportEntry, ItemDraft};

pub fn entry(import: &Import, index: u32) -> &ImportEntry {
    import.entries.iter().find(|e| e.index == index).unwrap()
}

pub fn draft(import: &Import, index: u32) -> &ItemDraft {
    &entry(import, index).draft
}

pub fn secret(d: &ItemDraft, f: SecretField) -> &str {
    d.secret(f).unwrap().expose_secret()
}

pub fn labels(d: &ItemDraft) -> Vec<&str> {
    d.custom_fields.iter().map(|c| c.label.as_str()).collect()
}

pub fn tags(d: &ItemDraft) -> Vec<&str> {
    d.tags.iter().map(Text::as_str).collect()
}

pub fn urls(d: &ItemDraft) -> Vec<&str> {
    d.urls.iter().map(Text::as_str).collect()
}

pub fn kinds(import: &Import) -> Vec<ItemType> {
    import
        .entries
        .iter()
        .map(|e| e.draft.item_type.clone())
        .collect()
}

/// A zip archive (deflate) of `members` in order.
pub fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in members {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
