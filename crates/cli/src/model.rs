//! Items as the CLI shows them: rows for `list` and `search`, the field view of `get`. Whether a
//! field is secret comes from the core model (`SecretField`, hidden custom fields), never from
//! here.

use std::collections::BTreeMap;

use mosaic_core::model::{CustomFieldValue, Item, SecretField, TextField};
use mosaic_core::{SecretString, VaultId};
use mosaic_vault::session::{ItemRef, UnlockedSession};
use serde::Serialize;

/// The vault of a row.
#[derive(Serialize)]
pub struct VaultOut {
    /// The vault ID (hyphenated UUID).
    pub id: String,
    /// Its name.
    pub name: String,
}

/// An item in a listing (`list`, `search`, and the head of `get`).
#[derive(Serialize)]
pub struct ItemOut {
    /// The item ID.
    pub id: String,
    /// Its vault.
    pub vault: VaultOut,
    /// The item type (`login`, `note`, ...).
    #[serde(rename = "type")]
    pub kind: String,
    /// The title.
    pub title: String,
    /// The user name, if the item has one.
    pub username: Option<String>,
    /// The URLs.
    pub urls: Vec<String>,
    /// Favorite.
    pub favorite: bool,
    /// In the trash.
    pub trashed: bool,
}

/// A value of an item.
pub enum Value {
    /// Not secret.
    Text(String),
    /// Secret: shown only with `--reveal`.
    Secret(SecretString),
}

/// A named field.
pub struct Entry {
    /// The core's key (`login.username`, ...) or `field:<label>`.
    pub key: String,
    /// The value.
    pub value: Value,
}

/// Vault names by ID (the ID's text for a vault without a readable name).
#[must_use]
pub fn vault_names(session: &UnlockedSession) -> BTreeMap<VaultId, String> {
    session
        .vaults()
        .into_iter()
        .map(|v| {
            (
                v.vault_id,
                v.name.map_or_else(|| v.vault_id.to_string(), str::to_owned),
            )
        })
        .collect()
}

/// A listing row for `item`.
#[must_use]
pub fn row(names: &BTreeMap<VaultId, String>, r: &ItemRef<'_>) -> ItemOut {
    let item = r.item;
    ItemOut {
        id: r.item_id.to_string(),
        vault: VaultOut {
            id: r.vault_id.to_string(),
            name: names.get(&r.vault_id).cloned().unwrap_or_default(),
        },
        kind: item
            .item_type()
            .map_or_else(String::new, |t| t.as_str().to_owned()),
        title: item.title().unwrap_or_default().to_owned(),
        username: item.text(TextField::Username).map(str::to_owned),
        urls: item
            .urls()
            .iter()
            .map(|(_, u)| u.url.as_str().to_owned())
            .collect(),
        favorite: item.favorite(),
        trashed: item.trashed_at().is_some(),
    }
}

/// Every non-empty field of `item` except the title.
#[must_use]
pub fn entries(item: &Item) -> Vec<Entry> {
    let mut out = Vec::new();
    for field in TextField::ALL {
        if *field == TextField::Title {
            continue;
        }
        if let Some(text) = item.text(*field).filter(|t| !t.is_empty()) {
            out.push(Entry {
                key: field.key().to_owned(),
                value: Value::Text(text.to_owned()),
            });
        }
    }
    for field in SecretField::ALL {
        if let Ok(Some(secret)) = item.secret(*field)
            && !secret.expose_secret().is_empty()
        {
            out.push(Entry {
                key: field.key().to_owned(),
                value: Value::Secret(secret),
            });
        }
    }
    for (_, custom) in item.custom_fields() {
        let key = format!("field:{}", custom.label.as_str());
        let value = match &custom.value {
            CustomFieldValue::Text(t)
            | CustomFieldValue::Url(t)
            | CustomFieldValue::Email(t)
            | CustomFieldValue::Phone(t)
            | CustomFieldValue::Date(t) => Value::Text(t.as_str().to_owned()),
            CustomFieldValue::Hidden(s) => match SecretString::from_str_copy(s.expose_secret()) {
                Ok(copy) => Value::Secret(copy),
                Err(_) => continue,
            },
            // The seed of an OTP custom field is read through the `otp` field, never listed.
            CustomFieldValue::Otp(_) => continue,
        };
        out.push(Entry { key, value });
    }
    out
}

/// One field by name: the core's key, an alias (`username`, `password`, `url`, `notes`, `title`)
/// or `field:<label>`.
#[must_use]
pub fn lookup(item: &Item, name: &str) -> Option<Entry> {
    let key = match name {
        "username" => "login.username",
        "password" => "login.password",
        other => other,
    };
    if key == "url" {
        let urls = item.urls();
        let (_, first) = urls.first()?;
        return Some(Entry {
            key: "url".to_owned(),
            value: Value::Text(first.url.as_str().to_owned()),
        });
    }
    if key == "title" {
        return Some(Entry {
            key: "title".to_owned(),
            value: Value::Text(item.title()?.to_owned()),
        });
    }
    entries(item).into_iter().find(|e| e.key == key)
}

/// A filter for [`UnlockedSession::items`] (the type is `non_exhaustive`).
#[must_use]
pub fn item_filter(
    vault: Option<VaultId>,
    trash: mosaic_vault::session::TrashFilter,
    favorites_only: bool,
) -> mosaic_vault::session::ItemFilter {
    let mut filter = mosaic_vault::session::ItemFilter::default();
    filter.vault = vault;
    filter.trash = trash;
    filter.favorites_only = favorites_only;
    filter
}
