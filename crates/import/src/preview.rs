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
use crate::draft::{EntryWarning, ItemDraft};
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
    /// Passkeys were left out: the format holds none of their private keys
    /// (1PUX).
    PasskeysNotImported,
    /// Some passkeys could not be read and are left out.
    PasskeyUnreadable,
    /// Some passkeys use an algorithm other than ES256 and are left out.
    PasskeyUnsupported,
    /// Some passkeys were left out: an entry held more than
    /// [`crate::MAX_PASSKEYS_PER_ENTRY`], or the file more than
    /// [`crate::MAX_IMPORT_PASSKEYS`].
    PasskeysSkipped,
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

fn count(counts: &mut Vec<(ItemType, u32)>, item_type: &ItemType) {
    match counts.iter_mut().find(|(t, _)| t == item_type) {
        Some((_, n)) => *n = n.saturating_add(1),
        None => counts.push((item_type.clone(), 1)),
    }
}

/// The file-level warning an entry warning rolls up into.
const fn summary_warning(w: EntryWarning) -> Option<ImportWarning> {
    match w {
        EntryWarning::UnmappedFields | EntryWarning::UnknownCategory => {
            Some(ImportWarning::UnmappedFields)
        }
        EntryWarning::AttachmentTooLarge
        | EntryWarning::AttachmentMissing
        | EntryWarning::AttachmentQuota => Some(ImportWarning::AttachmentsSkipped),
        EntryWarning::OtpUnreadable => Some(ImportWarning::OtpUnreadable),
        EntryWarning::PasskeysNotImported => Some(ImportWarning::PasskeysNotImported),
        EntryWarning::PasskeyUnreadable => Some(ImportWarning::PasskeyUnreadable),
        EntryWarning::PasskeyUnsupported => Some(ImportWarning::PasskeyUnsupported),
        EntryWarning::PasskeysLimit => Some(ImportWarning::PasskeysSkipped),
        _ => None,
    }
}

/// For an entry of passkeys only: what `known` (existing passkey items, or the
/// earlier entries of the file) holds for the first of its credentials, when
/// it holds every one of them (same relying party and credential ID). Such an
/// entry has no site and user name to compare.
fn known_passkey_entry<T: Copy>(d: &ItemDraft, known: &HashMap<(String, Vec<u8>), T>) -> Option<T> {
    if d.has_item() {
        return None;
    }
    let mut found = d.passkeys.iter().map(|p| {
        known
            .get(&(
                p.credential.rp_id.as_str().to_owned(),
                p.credential.credential_id.clone(),
            ))
            .copied()
    });
    let first = found.next().flatten()?;
    found.all(|f| f.is_some()).then_some(first)
}

/// Records the credentials of `d` as first held by entry `index`.
fn note_passkeys(seen: &mut HashMap<(String, Vec<u8>), u32>, d: &ItemDraft, index: u32) {
    for p in &d.passkeys {
        seen.entry((
            p.credential.rp_id.as_str().to_owned(),
            p.credential.credential_id.clone(),
        ))
        .or_insert(index);
    }
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
        // Passkeys already in the account: relying party and credential ID
        // (neither is secret; the private key is not read).
        let mut known_passkeys: HashMap<(String, Vec<u8>), Duplicate> = HashMap::new();
        for r in existing {
            if let Some(p) = r.item.passkey_public() {
                known_passkeys
                    .entry((p.rp_id.as_str().to_owned(), p.credential_id))
                    .or_insert(Duplicate {
                        vault_id: r.vault_id,
                        item_id: r.item_id,
                    });
            }
            for k in item_keys(r.item) {
                known.entry(k).or_insert(Duplicate {
                    vault_id: r.vault_id,
                    item_id: r.item_id,
                });
            }
        }
        let mut seen: HashMap<Key, u32> = HashMap::new();
        // Passkeys of earlier entries of the file: relying party and
        // credential ID, with the index of the entry that first held them.
        let mut seen_passkeys: HashMap<(String, Vec<u8>), u32> = HashMap::new();
        let mut counts: Vec<(ItemType, u32)> = Vec::new();
        let mut warnings: Vec<ImportWarning> = Vec::new();
        let mut attachments = 0u32;
        let mut attachment_bytes = 0u64;
        let mut entries = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let d = &e.draft;
            // An entry of passkeys only is a duplicate when the account
            // already holds every one of its credentials; it has no site and
            // user name to compare.
            let ks = if d.has_item() {
                keys(
                    &d.item_type,
                    d.title(),
                    d.text(TextField::Username),
                    d.urls.iter().map(Text::as_str),
                )
            } else {
                Vec::new()
            };
            let duplicate_of = ks
                .iter()
                .find_map(|k| known.get(k).copied())
                .or_else(|| known_passkey_entry(d, &known_passkeys));
            let duplicate_in_file = ks
                .iter()
                .find_map(|k| seen.get(k).copied())
                .or_else(|| known_passkey_entry(d, &seen_passkeys));
            for k in ks {
                seen.entry(k).or_insert(e.index);
            }
            note_passkeys(&mut seen_passkeys, d, e.index);
            // One item for the entry (unless it holds only passkeys) and one
            // per passkey.
            if d.has_item() {
                count(&mut counts, &d.item_type);
            }
            for _ in &d.passkeys {
                count(&mut counts, &ItemType::Passkey);
            }
            let n = to_u32(d.attachments.len());
            attachments = attachments.saturating_add(n);
            attachment_bytes = attachment_bytes.saturating_add(d.attachment_bytes());
            for w in &e.warnings {
                let summary = summary_warning(*w);
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
    #![expect(clippy::too_many_lines, reason = "tests")]

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
