//! The import preview: counts per type, attachments, warnings and
//! duplicate hints.
//!
//! Two items are duplicates when they share a site and a user name (any
//! login URL of one has the same site key as any login URL of the other,
//! `url::site`, and the user names are equal ignoring case and surrounding
//! space), or, for items without URLs, when type, title and user name are
//! equal (ignoring case and surrounding space). Empty titles never match.
//! The preview only suggests skipping; nothing is skipped without the
//! user's choice.

use core::fmt;
use core::hash::{Hash, Hasher};
use std::collections::HashMap;

use mosaic_core::model::{Item, ItemType, Text, TextField};
use mosaic_core::{ItemId, VaultId};
use mosaic_vault::session::ItemRef;
use zeroize::Zeroizing;

use crate::buf::Str;
use crate::draft::EntryWarning;
use crate::entries::{Import, SkippedEntry};
use crate::url::site;

/// A summary warning for the whole file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ImportWarning {
    /// Some entries have fields Mosaic keeps as custom fields (or an
    /// unknown category, imported as a secure note).
    UnmappedFields,
    /// Some attachments were left out (too large, or missing from the
    /// export).
    AttachmentsSkipped,
    /// Some entries could not be read and are left out.
    EntriesSkipped,
    /// Some one-time-password values were not understood (kept as hidden
    /// fields).
    OtpUnreadable,
    /// Passkeys were left out (passkey import is M5).
    PasskeysNotImported,
}

/// An existing item an entry looks like.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Duplicate {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
}

/// One entry of the preview. `title` and `subtitle` are item content
/// (zeroizing [`Text`]); `Debug` does not print them.
pub struct PreviewEntry {
    /// Position in the file, for `skip`.
    pub index: u32,
    /// The type it becomes.
    pub item_type: ItemType,
    /// The title, if any.
    pub title: Option<Text>,
    /// User name, email, cardholder, network name or key ID, if any.
    pub subtitle: Option<Text>,
    /// An existing item it duplicates (suggested for skipping).
    pub duplicate_of: Option<Duplicate>,
    /// An earlier entry of the same file it duplicates.
    pub duplicate_in_file: Option<u32>,
    /// How many attachment files it brings.
    pub attachments: u32,
    /// Its warnings.
    pub warnings: Vec<EntryWarning>,
}

impl fmt::Debug for PreviewEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewEntry")
            .field("index", &self.index)
            .field("item_type", &self.item_type)
            .field("duplicate_of", &self.duplicate_of)
            .field("duplicate_in_file", &self.duplicate_in_file)
            .field("attachments", &self.attachments)
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

/// The preview of an [`Import`].
#[derive(Debug)]
pub struct Preview {
    /// Every entry that can be imported, in file order.
    pub entries: Vec<PreviewEntry>,
    /// Entries that are left out.
    pub skipped: Vec<SkippedEntry>,
    /// Entries per item type, in order of first appearance.
    pub counts: Vec<(ItemType, u32)>,
    /// Attachment files in total.
    pub attachments: u32,
    /// Their total size in bytes.
    pub attachment_bytes: u64,
    /// Summary warnings, sorted.
    pub warnings: Vec<ImportWarning>,
}

/// A duplicate-detection key: normalized text, compared and hashed by
/// value, wiped on drop.
struct Key(Str);

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

fn norm(s: Option<&str>) -> Str {
    Zeroizing::new(s.unwrap_or_default().trim().to_lowercase())
}

/// The keys of an item: one per distinct URL site, or one for type and
/// title if it has no URL with a site.
fn keys<'a>(
    item_type: &ItemType,
    title: Option<&str>,
    user: Option<&str>,
    urls: impl Iterator<Item = &'a str>,
) -> Vec<Key> {
    let user = norm(user);
    let mut out: Vec<Key> = Vec::new();
    for s in urls.filter_map(site) {
        let mut k = Zeroizing::new(String::from("s\u{0}"));
        k.push_str(&s);
        k.push('\u{0}');
        k.push_str(&user);
        let k = Key(k);
        if !out.contains(&k) {
            out.push(k);
        }
    }
    let title = norm(title);
    if out.is_empty() && !title.is_empty() {
        let mut k = Zeroizing::new(String::from("t\u{0}"));
        k.push_str(item_type.as_str());
        k.push('\u{0}');
        k.push_str(&title);
        k.push('\u{0}');
        k.push_str(&user);
        out.push(Key(k));
    }
    out
}

fn item_keys(item: &Item) -> Vec<Key> {
    let Some(item_type) = item.item_type() else {
        return Vec::new();
    };
    let urls = item.urls();
    keys(
        &item_type,
        item.title(),
        item.text(TextField::Username),
        urls.iter().map(|(_, u)| u.url.as_str()),
    )
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Import {
    /// The preview against the items already in the account (typically
    /// `session.items(&ItemFilter::default())`).
    #[must_use]
    pub fn preview(&self, existing: &[ItemRef<'_>]) -> Preview {
        let mut known: HashMap<Key, Duplicate> = HashMap::new();
        for r in existing {
            for k in item_keys(r.item) {
                known.entry(k).or_insert(Duplicate {
                    vault_id: r.vault_id,
                    item_id: r.item_id,
                });
            }
        }
        let mut seen: HashMap<Key, u32> = HashMap::new();
        let mut counts: Vec<(ItemType, u32)> = Vec::new();
        let mut warnings: Vec<ImportWarning> = Vec::new();
        let mut attachments = 0u32;
        let mut attachment_bytes = 0u64;
        let mut entries = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let d = &e.draft;
            let ks = keys(
                &d.item_type,
                d.title(),
                d.text(TextField::Username),
                d.urls.iter().map(Text::as_str),
            );
            let duplicate_of = ks.iter().find_map(|k| known.get(k).copied());
            let duplicate_in_file = ks.iter().find_map(|k| seen.get(k).copied());
            for k in ks {
                seen.entry(k).or_insert(e.index);
            }
            match counts.iter_mut().find(|(t, _)| *t == d.item_type) {
                Some((_, n)) => *n = n.saturating_add(1),
                None => counts.push((d.item_type.clone(), 1)),
            }
            let n = to_u32(d.attachments.len());
            attachments = attachments.saturating_add(n);
            attachment_bytes = attachment_bytes.saturating_add(d.attachment_bytes());
            for w in &e.warnings {
                let summary = match w {
                    EntryWarning::UnmappedFields | EntryWarning::UnknownCategory => {
                        Some(ImportWarning::UnmappedFields)
                    }
                    EntryWarning::AttachmentTooLarge
                    | EntryWarning::AttachmentMissing
                    | EntryWarning::AttachmentQuota => Some(ImportWarning::AttachmentsSkipped),
                    EntryWarning::OtpUnreadable => Some(ImportWarning::OtpUnreadable),
                    EntryWarning::PasskeysNotImported => Some(ImportWarning::PasskeysNotImported),
                    _ => None,
                };
                if let Some(s) = summary
                    && !warnings.contains(&s)
                {
                    warnings.push(s);
                }
            }
            entries.push(PreviewEntry {
                index: e.index,
                item_type: d.item_type.clone(),
                title: d.title().map(Text::new),
                subtitle: d.subtitle().map(Text::new),
                duplicate_of,
                duplicate_in_file,
                attachments: n,
                warnings: e.warnings.clone(),
            });
        }
        if !self.skipped.is_empty() {
            warnings.push(ImportWarning::EntriesSkipped);
        }
        warnings.sort_unstable();
        Preview {
            entries,
            skipped: self.skipped.clone(),
            counts,
            attachments,
            attachment_bytes,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::too_many_lines, reason = "tests")]

    use mosaic_core::model::LoginUrl;
    use mosaic_core::{DeviceId, Timestamp};

    use super::*;
    use crate::Format;
    use crate::draft::Builder;
    use crate::entries::{Collector, SkipReason};

    fn login(title: &str, user: &str, url: Option<&str>) -> Builder {
        let mut b = Builder::new(ItemType::Login);
        b.title(title);
        b.text(TextField::Username, user, "Username");
        if let Some(u) = url {
            b.url(u, "URL");
        }
        b
    }

    #[test]
    fn duplicates_counts_and_warnings() {
        let device = DeviceId::from_bytes([3; 16]).unwrap();
        let at = Timestamp::from_unix_millis(0);
        let mut existing_login = Item::new(&ItemType::Login, device, at).unwrap();
        {
            let mut e = existing_login.edit(device, at);
            e.set_title("GitHub").unwrap();
            e.set_text(TextField::Username, "Alice").unwrap();
            e.add_url(&LoginUrl::new("https://www.github.com/login"))
                .unwrap();
        }
        let mut existing_note = Item::new(&ItemType::SecureNote, device, at).unwrap();
        existing_note.edit(device, at).set_title("Recipe").unwrap();
        let deleted = {
            let mut i = Item::new(&ItemType::SecureNote, device, at).unwrap();
            i.delete(device, at).unwrap();
            i
        };
        let vault = VaultId::new_v7().unwrap();
        let (id1, id2, id3) = (
            ItemId::new_v7().unwrap(),
            ItemId::new_v7().unwrap(),
            ItemId::new_v7().unwrap(),
        );
        let refs = [
            ItemRef {
                vault_id: vault,
                item_id: id1,
                item: &existing_login,
            },
            ItemRef {
                vault_id: vault,
                item_id: id2,
                item: &existing_note,
            },
            ItemRef {
                vault_id: vault,
                item_id: id3,
                item: &deleted,
            },
        ];

        let mut c = Collector::new(Format::Bitwarden);
        // 0: same site and user (case, www and path differ): duplicate.
        c.push(login("Other title", " alice ", Some("github.com")))
            .unwrap();
        // 1: same site, other user: no duplicate.
        c.push(login("GitHub", "bob", Some("https://github.com")))
            .unwrap();
        // 2: same as 1 within the file.
        c.push(login("GitHub 2", "BOB", Some("http://github.com:80/x")))
            .unwrap();
        // 3: note with the same title.
        let mut note = Builder::new(ItemType::SecureNote);
        note.title(" recipe ");
        note.fallback("x", crate::draft::Kind::Text, "y");
        note.attachment(
            &mut crate::draft::AttachmentBudget::new(),
            "a.txt",
            None,
            b"abc",
        );
        c.push(note).unwrap();
        // 4: login without URL and without title: no key.
        let mut bare = Builder::new(ItemType::Login);
        bare.text(TextField::Username, "x", "u");
        bare.warn(EntryWarning::OtpUnreadable);
        bare.warn(EntryWarning::PasskeysNotImported);
        bare.warn(EntryWarning::AttachmentMissing);
        bare.warn(EntryWarning::HistoryTruncated);
        c.push(bare).unwrap();
        c.skip(SkipReason::Malformed).unwrap();
        let import = c.finish();

        let p = import.preview(&refs);
        let dup: Vec<_> = p.entries.iter().map(|e| e.duplicate_of).collect();
        assert_eq!(
            dup,
            [
                Some(Duplicate {
                    vault_id: vault,
                    item_id: id1
                }),
                None,
                None,
                Some(Duplicate {
                    vault_id: vault,
                    item_id: id2
                }),
                None
            ]
        );
        let in_file: Vec<_> = p.entries.iter().map(|e| e.duplicate_in_file).collect();
        assert_eq!(in_file, [None, None, Some(1), None, None]);
        assert_eq!(p.counts, [(ItemType::Login, 4), (ItemType::SecureNote, 1)]);
        assert_eq!((p.attachments, p.attachment_bytes), (1, 3));
        assert_eq!(
            p.warnings,
            [
                ImportWarning::UnmappedFields,
                ImportWarning::AttachmentsSkipped,
                ImportWarning::EntriesSkipped,
                ImportWarning::OtpUnreadable,
                ImportWarning::PasskeysNotImported
            ]
        );
        assert_eq!(p.entries[0].title.as_ref().unwrap().as_str(), "Other title");
        assert_eq!(p.entries[0].subtitle.as_ref().unwrap().as_str(), " alice ");
        assert_eq!(p.skipped.len(), 1);
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("alice") && !dbg.contains("GitHub"));

        let empty = Collector::new(Format::Csv).finish().preview(&[]);
        assert!(empty.entries.is_empty() && empty.warnings.is_empty());
    }
}
