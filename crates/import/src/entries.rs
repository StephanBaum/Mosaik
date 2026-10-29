//! The result of reading an export file.

use core::fmt;

use mosaic_core::{DeviceId, Timestamp};

use crate::Format;
use crate::draft::{Builder, EntryWarning, ItemDraft};
use crate::error::{Error, Result};

/// Most entries one file may hold: 100,000.
pub const MAX_ENTRIES: usize = 100_000;

/// One entry of the file that becomes an item.
pub struct ImportEntry {
    /// The entry's position in the file (counting skipped entries too), for
    /// `skip` lists.
    pub index: u32,
    /// The item to create.
    pub draft: ItemDraft,
    /// What the user should know about it.
    pub warnings: Vec<EntryWarning>,
}

impl fmt::Debug for ImportEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportEntry")
            .field("index", &self.index)
            .field("draft", &self.draft)
            .field("warnings", &self.warnings)
            .finish()
    }
}

/// Why an entry of the file is not imported.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum SkipReason {
    /// The entry is not well-formed (missing or mistyped members).
    Malformed,
    /// The entry holds no data.
    Empty,
    /// The entry does not fit Mosaic's item size limit (1 MiB of
    /// plaintext).
    TooLarge,
}

/// An entry of the file that is left out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkippedEntry {
    /// The entry's position in the file.
    pub index: u32,
    /// Why.
    pub reason: SkipReason,
}

/// What an importer read from one export file. Dropping it wipes every
/// value.
pub struct Import {
    /// The file's format.
    pub format: Format,
    /// The entries that become items, in file order.
    pub entries: Vec<ImportEntry>,
    /// The entries that are left out, in file order.
    pub skipped: Vec<SkippedEntry>,
}

impl fmt::Debug for Import {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Import")
            .field("format", &self.format)
            .field("entries", &self.entries.len())
            .field("skipped", &self.skipped)
            .finish()
    }
}

/// Collects the entries of one file in order, assigning indexes.
pub(crate) struct Collector {
    format: Format,
    entries: Vec<ImportEntry>,
    skipped: Vec<SkippedEntry>,
    next: u32,
}

impl Collector {
    pub(crate) const fn new(format: Format) -> Self {
        Self {
            format,
            entries: Vec::new(),
            skipped: Vec::new(),
            next: 0,
        }
    }

    fn index(&mut self) -> Result<u32> {
        let index = self.next;
        if usize::try_from(index).map_err(|_| Error::LimitExceeded)? >= MAX_ENTRIES {
            return Err(Error::LimitExceeded);
        }
        self.next = index.checked_add(1).ok_or(Error::LimitExceeded)?;
        Ok(index)
    }

    /// Records an entry that could not be read.
    pub(crate) fn skip(&mut self, reason: SkipReason) -> Result<()> {
        let index = self.index()?;
        self.skipped.push(SkippedEntry { index, reason });
        Ok(())
    }

    /// Records a built entry; one that is empty or does not fit an item is
    /// skipped.
    ///
    /// # Errors
    /// [`Error::LimitExceeded`] after [`MAX_ENTRIES`] entries.
    pub(crate) fn push(&mut self, builder: Builder) -> Result<()> {
        let index = self.index()?;
        let Some((draft, warnings)) = builder.finish() else {
            self.skipped.push(SkippedEntry {
                index,
                reason: SkipReason::Empty,
            });
            return Ok(());
        };
        match fits(&draft) {
            Ok(()) => self.entries.push(ImportEntry {
                index,
                draft,
                warnings,
            }),
            Err(reason) => self.skipped.push(SkippedEntry { index, reason }),
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Import {
        Import {
            format: self.format,
            entries: self.entries,
            skipped: self.skipped,
        }
    }
}

/// Whether the draft makes a valid item within the size limit.
fn fits(draft: &ItemDraft) -> core::result::Result<(), SkipReason> {
    let device = DeviceId::from_bytes([0x11; 16]).map_err(|_| SkipReason::Malformed)?;
    let item = draft
        .to_item(device, Timestamp::from_unix_millis(0))
        .map_err(|_| SkipReason::Malformed)?;
    match item.to_json() {
        Ok(_) => Ok(()),
        Err(mosaic_core::Error::SizeLimit) => Err(SkipReason::TooLarge),
        Err(_) => Err(SkipReason::Malformed),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use mosaic_core::model::{ItemType, SecretField};

    use super::*;

    #[test]
    fn indexes_skips_and_size() {
        let mut c = Collector::new(Format::Csv);
        let mut b = Builder::new(ItemType::Login);
        b.title("a");
        c.push(b).unwrap();
        c.skip(SkipReason::Malformed).unwrap();
        c.push(Builder::new(ItemType::Login)).unwrap();
        let mut big = Builder::new(ItemType::Login);
        big.secret(SecretField::Password, &"x".repeat(1024 * 1024), "p");
        c.push(big).unwrap();
        let import = c.finish();
        assert_eq!(import.entries.len(), 1);
        assert_eq!(import.entries[0].index, 0);
        assert_eq!(
            import.skipped,
            [
                SkippedEntry {
                    index: 1,
                    reason: SkipReason::Malformed
                },
                SkippedEntry {
                    index: 2,
                    reason: SkipReason::Empty
                },
                SkippedEntry {
                    index: 3,
                    reason: SkipReason::TooLarge
                },
            ]
        );
        let dbg = format!("{import:?} {:?}", import.entries[0]);
        assert!(dbg.contains("Csv") && dbg.contains("index: 0"));
    }

    #[test]
    fn entry_limit() {
        let mut c = Collector::new(Format::Csv);
        c.next = u32::try_from(MAX_ENTRIES).unwrap();
        assert_eq!(c.skip(SkipReason::Empty), Err(Error::LimitExceeded));
        assert_eq!(
            c.push(Builder::new(ItemType::Login)),
            Err(Error::LimitExceeded)
        );
    }
}
