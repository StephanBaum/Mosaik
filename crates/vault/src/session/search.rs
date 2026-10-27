//! The in-memory search index (CRYPTO_SPEC §11: it exists only while the
//! session is unlocked and is never written anywhere).
//!
//! Per live item it keeps lowercase copies of the title, the user name,
//! the email, every URL and every tag, each in a zeroizing buffer. A query
//! matches when every whitespace-separated term occurs in at least one of
//! them.

use std::collections::BTreeMap;

use mosaic_core::model::{Item, TextField};
use mosaic_core::{ItemId, VaultId};
use zeroize::Zeroizing;

/// The searchable text of one item. The title comes first.
struct Entry {
    title: Zeroizing<String>,
    others: Vec<Zeroizing<String>>,
}

impl Entry {
    fn matches(&self, term: &str) -> bool {
        self.title.contains(term) || self.others.iter().any(|s| s.contains(term))
    }
}

/// A lowercase copy of `s` in a zeroizing buffer of exactly the final size.
/// `str::to_lowercase` would allocate `s.len()` bytes and reallocate when a
/// mapping grows in UTF-8 (U+023A, 2 bytes, lowercases to U+2C65, 3 bytes),
/// leaving the first buffer unwiped; here nothing reallocates. Characters
/// are mapped one by one (no final-sigma rule), the same for entries and
/// queries.
fn lower(s: &str) -> Zeroizing<String> {
    let len: usize = s
        .chars()
        .flat_map(char::to_lowercase)
        .map(char::len_utf8)
        .sum();
    let mut out = Zeroizing::new(String::with_capacity(len));
    out.extend(s.chars().flat_map(char::to_lowercase));
    out
}

/// See the [module documentation](self).
#[derive(Default)]
pub(super) struct SearchIndex {
    entries: BTreeMap<(VaultId, ItemId), Entry>,
}

impl SearchIndex {
    /// Indexes `item` (or drops it from the index if it is deleted).
    pub(super) fn update(&mut self, vault_id: VaultId, item_id: ItemId, item: &Item) {
        if item.is_deleted() {
            self.remove(vault_id, item_id);
            return;
        }
        let mut others = Vec::new();
        for field in [TextField::Username, TextField::Email] {
            if let Some(v) = item.text(field) {
                others.push(lower(v));
            }
        }
        for (_, url) in item.urls() {
            others.push(lower(url.url.as_str()));
        }
        for tag in item.tags() {
            others.push(lower(tag));
        }
        let entry = Entry {
            title: lower(item.title().unwrap_or("")),
            others,
        };
        self.entries.insert((vault_id, item_id), entry);
    }

    /// Drops an item from the index.
    pub(super) fn remove(&mut self, vault_id: VaultId, item_id: ItemId) {
        self.entries.remove(&(vault_id, item_id));
    }

    /// The items matching every term of `query`, best first: items whose
    /// title starts with the whole query, then the rest, each group in
    /// index order. An empty query matches nothing.
    pub(super) fn search(&self, query: &str) -> Vec<(VaultId, ItemId)> {
        let query = lower(query.trim());
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }
        let mut prefix = Vec::new();
        let mut rest = Vec::new();
        for (key, entry) in &self.entries {
            if terms.iter().all(|t| entry.matches(t)) {
                if entry.title.starts_with(query.as_str()) {
                    prefix.push(*key);
                } else {
                    rest.push(*key);
                }
            }
        }
        prefix.extend(rest);
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::lower;

    /// Regression: the buffer is reserved at its final size, so a mapping
    /// that grows in UTF-8 never leaves an unwiped smaller buffer behind.
    #[test]
    fn lowercase_copies_never_reallocate() {
        for (input, expected) in [
            ("\u{23A}", "\u{2C65}"),
            ("A\u{23A}b\u{130}", "a\u{2C65}bi\u{307}"),
            ("Mail", "mail"),
            ("\u{212A}elvin", "kelvin"),
            ("", ""),
        ] {
            let out = lower(input);
            assert_eq!(out.as_str(), expected);
            assert_eq!(out.capacity(), expected.len(), "{input:?}");
        }
    }
}
