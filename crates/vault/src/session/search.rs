//! The in-memory search index (CRYPTO_SPEC §11: it exists only while the
//! session is unlocked and is never written anywhere).
//!
//! Per live item it keeps lowercase copies of the title, the user name,
//! the email, every URL and every tag, each in a zeroizing buffer. A query
//! matches when every whitespace-separated term occurs in at least one of
//! them.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mosaic_core::model::{Item, TextField};
use mosaic_core::{ItemId, VaultId};
use zeroize::Zeroizing;

use super::OpenVault;

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

/// The search index of a session, built on the first search. Unlocking
/// (and a sync that changes items) does not pay for an index nobody asks
/// for; once built, every change updates it as before. Until then the
/// changes are not tracked: the first search indexes the then-current items
/// of every open vault, which is what keeping the index up to date would
/// have produced. Dropping it (lock) drops the built index.
#[derive(Default)]
pub(super) struct LazyIndex {
    built: OnceLock<SearchIndex>,
}

impl LazyIndex {
    /// The items of `vaults` matching `query` (see [`SearchIndex::search`]),
    /// building the index on the first call.
    pub(super) fn search(
        &self,
        vaults: &BTreeMap<VaultId, OpenVault>,
        query: &str,
    ) -> Vec<(VaultId, ItemId)> {
        self.built
            .get_or_init(|| {
                let mut index = SearchIndex::default();
                for (vault_id, vault) in vaults {
                    index.update_vault(*vault_id, vault);
                }
                index
            })
            .search(query)
    }

    /// See [`SearchIndex::update_vault`]; nothing happens before the first search.
    pub(super) fn update_vault(&mut self, vault_id: VaultId, vault: &OpenVault) {
        if let Some(index) = self.built.get_mut() {
            index.update_vault(vault_id, vault);
        }
    }

    /// See [`SearchIndex::update`]; nothing happens before the first search.
    pub(super) fn update(&mut self, vault_id: VaultId, item_id: ItemId, item: &Item) {
        if let Some(index) = self.built.get_mut() {
            index.update(vault_id, item_id, item);
        }
    }

    /// See [`SearchIndex::remove`]; nothing happens before the first search.
    pub(super) fn remove(&mut self, vault_id: VaultId, item_id: ItemId) {
        if let Some(index) = self.built.get_mut() {
            index.remove(vault_id, item_id);
        }
    }

    /// Whether the index has been built.
    #[cfg(test)]
    pub(super) fn is_built(&self) -> bool {
        self.built.get().is_some()
    }
}

/// See the [module documentation](self).
#[derive(Default)]
pub(super) struct SearchIndex {
    entries: BTreeMap<(VaultId, ItemId), Entry>,
}

impl SearchIndex {
    /// Indexes every item of `vault`.
    pub(super) fn update_vault(&mut self, vault_id: VaultId, vault: &OpenVault) {
        for (item_id, item) in &vault.items {
            self.update(vault_id, *item_id, item);
        }
    }

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
