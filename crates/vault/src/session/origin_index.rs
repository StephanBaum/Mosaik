//! The in-memory origin index of the fill lookups (never stored, dropped at lock, like the
//! search index).
//!
//! A fill asks "which logins may fill this page (or this Android app)?" and "does this page
//! imitate a saved host?". Answering by parsing every saved URL of every item is linear in the
//! vault, per request. The index keeps the parse result instead: for every item a **key** per
//! saved URL, and the set of saved hosts.
//!
//! **The index only narrows the candidate set.** Every check of `mosaic-origin` (`fill_allowed`,
//! the HTTP rules, the app and certificate rules, the shared-vault rule of the mobile core)
//! still runs, unchanged, on the candidates the caller gets back; the index decides nothing.
//! That is safe because a saved URL can only match a page or an app when it has the same key:
//!
//! - A web URL matches a page by an equal origin (so an equal host) or, for `UrlMatch::Etld1`,
//!   by two registrable domains that are both `Some` and equal. The key of a host is its
//!   registrable domain, else the host itself. Equal hosts have equal keys, and equal `Some`
//!   registrable domains are equal keys, so every legitimate match shares the key of the page.
//!   Keys of different hosts may collide (a registrable domain next to a host that happens to
//!   spell the same); that only adds candidates, which the checks then refuse.
//! - An Android app URL (`androidapp://package`, with or without `?cert=`) matches an app
//!   request only for the equal package. Its key is `app:<package>`; a web host never contains a
//!   colon outside an IPv6 literal in brackets, so the two spaces do not mix.
//! - A URL that parses as neither (garbage, `file:`, `winapp:` and other schemes) can never
//!   match any target in `mosaic-origin`, so it has no key.
//!
//! The index covers every item that is not deleted, trashed ones and items of every type
//! included: the caller's [`ItemFilter`] and type check still decide what is listed, exactly
//! as in a full scan. Candidates come back in the order of a full scan (vault, then item ID).
//!
//! The look-alike check needs the saved hosts, not matches. They are kept as de-duplicated
//! sets per vault, for logins and for every item (the desktop warns against logins, the mobile
//! core against every item of a usable vault), both over live items outside the trash.
//!
//! Currency: the index remembers the [`VaultMap`] revision it was built at, which rises on
//! every access that can change a vault, and is rebuilt on the first lookup after a change.
//! Nothing can be forgotten, as there is no hook to skip.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, PoisonError};

use mosaic_core::model::ItemType;
use mosaic_core::{ItemId, VaultId};
use mosaic_origin::app::AndroidAppUrl;
use mosaic_origin::{PageOrigin, lookalike_of, registrable_domain, saved_origin};

use super::{ItemFilter, ItemRef, UnlockedSession, VaultMap};

/// What a fill lookup is for: the key of a page or of an Android app, see the
/// [module documentation](self).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginKey(String);

impl OriginKey {
    /// The key of a page: its registrable domain, else its host.
    #[must_use]
    pub fn web(page: &PageOrigin) -> Self {
        Self(host_key(&page.host_str()))
    }

    /// The key of an Android app.
    #[must_use]
    pub fn app(package: &str) -> Self {
        Self(app_key(package))
    }
}

/// The key of a host as `PageOrigin::host_str` serializes it.
fn host_key(host: &str) -> String {
    registrable_domain(host).unwrap_or_else(|| host.to_owned())
}

fn app_key(package: &str) -> String {
    format!("app:{package}")
}

/// The saved hosts of the live items outside the trash of one vault.
#[derive(Default)]
struct VaultHosts {
    /// Of the logins.
    logins: BTreeSet<String>,
    /// Of every item.
    all: BTreeSet<String>,
}

#[derive(Default)]
struct OriginIndex {
    /// The items with a saved URL of that key, in vault and item ID order.
    by_key: HashMap<String, Vec<(VaultId, ItemId)>>,
    hosts: BTreeMap<VaultId, VaultHosts>,
}

impl OriginIndex {
    fn build(vaults: &VaultMap) -> Self {
        let mut index = Self::default();
        for (vault_id, vault) in vaults {
            let mut hosts = VaultHosts::default();
            for (item_id, item) in &vault.items {
                if item.is_deleted() {
                    continue;
                }
                let live = item.trashed_at().is_none();
                let login = item.item_type() == Some(ItemType::Login);
                let mut keys: Vec<String> = Vec::new();
                for (_, url) in item.urls() {
                    let text = url.url.as_str();
                    if let Some(origin) = saved_origin(text) {
                        let host = origin.host_str();
                        keys.push(host_key(&host));
                        if live {
                            if login {
                                hosts.logins.insert(host.clone());
                            }
                            hosts.all.insert(host);
                        }
                    }
                    if let Ok(app) = AndroidAppUrl::parse(text) {
                        keys.push(app_key(app.package()));
                    }
                }
                keys.sort_unstable();
                keys.dedup();
                for key in keys {
                    index
                        .by_key
                        .entry(key)
                        .or_default()
                        .push((*vault_id, *item_id));
                }
            }
            if !hosts.all.is_empty() {
                index.hosts.insert(*vault_id, hosts);
            }
        }
        index
    }
}

/// The index of a session, built by the first lookup and again after every change of the vaults.
#[derive(Default)]
pub(super) struct OriginIndexCache {
    built: Mutex<Option<(u64, Arc<OriginIndex>)>>,
}

impl OriginIndexCache {
    /// The index of `vaults` as they are now.
    fn current(&self, vaults: &VaultMap) -> Arc<OriginIndex> {
        let mut slot = self.built.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((revision, index)) = slot.as_ref()
            && *revision == vaults.revision
        {
            return Arc::clone(index);
        }
        let index = Arc::new(OriginIndex::build(vaults));
        *slot = Some((vaults.revision, Arc::clone(&index)));
        index
    }

    /// Whether an index is held.
    #[cfg(test)]
    pub(super) fn is_built(&self) -> bool {
        self.built
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }
}

impl UnlockedSession {
    /// The items that pass `filter` and have a saved URL that can match the page or app of `key`:
    /// a superset of the items a full scan with the same filter would accept, in the same order
    /// (vault, then item ID). The caller still applies every match rule to them; this only
    /// avoids parsing the URLs of every other item. The index is in memory only.
    #[must_use]
    pub fn items_for_origin(&self, key: &OriginKey, filter: &ItemFilter) -> Vec<ItemRef<'_>> {
        let index = self.origin_index.current(&self.vaults);
        index
            .by_key
            .get(&key.0)
            .into_iter()
            .flatten()
            .filter_map(|(vault_id, item_id)| {
                let item = self.vaults.get(vault_id)?.items.get(item_id)?;
                filter.accepts(*vault_id, item).then_some(ItemRef {
                    vault_id: *vault_id,
                    item_id: *item_id,
                    item,
                })
            })
            .collect()
    }

    /// The saved host that `page_host` imitates (see `mosaic_origin::lookalike_of`), among the
    /// hosts of the saved web URLs of the live items outside the trash of the vaults for which
    /// `vault_ok` is true: of the logins only, or of every item. The same answer as collecting
    /// those hosts by a full scan.
    #[must_use]
    pub fn lookalike_of_saved(
        &self,
        page_host: &str,
        logins_only: bool,
        vault_ok: impl Fn(VaultId) -> bool,
    ) -> Option<String> {
        let index = self.origin_index.current(&self.vaults);
        let hosts = index
            .hosts
            .iter()
            .filter(|(vault_id, _)| vault_ok(**vault_id))
            .flat_map(|(_, hosts)| {
                let set = if logins_only {
                    &hosts.logins
                } else {
                    &hosts.all
                };
                set.iter().map(String::as_str)
            });
        lookalike_of(page_host, hosts).map(str::to_owned)
    }

    /// Whether the origin index is built (tests).
    #[cfg(test)]
    pub(super) fn origin_index_is_built(&self) -> bool {
        self.origin_index.is_built()
    }
}
