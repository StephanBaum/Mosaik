//! Item commands. Summaries and details never carry a secret; the only
//! secret that leaves here is the one `reveal_secret` was asked for.

use mosaic_core::model::{CustomFieldValue, Item, OtpKind, SecretField, TextField, UrlMatch};
use mosaic_core::{ItemId, Timestamp, VaultId};
use mosaic_vault::session::{ItemFilter, ItemRef, TrashFilter as CoreTrashFilter, UnlockedSession};
use tauri::State;

use super::draft;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::ids::WireVaultId;
use crate::contract::items::{
    AddItemRequest, AttachmentEntry, CustomFieldEntry, CustomFieldKind, ItemDetail, ItemKind,
    ItemRequest, ItemSummary, ListItemsRequest, MoveItemRequest, MovedItem, OtpScheme, OtpSummary,
    RevealRequest, RevealedSecret, ReviewState, SecretFieldKey, SecretRef, SetFavoriteRequest,
    TextFieldEntry, TextFieldKey, TrashFilter, UpdateItemRequest, UrlEntry, UrlMatchMode,
};
use crate::contract::secret::SecretOut;
use crate::state::{AppState, Backend};

/// How long a revealed secret stays visible (UX_GUIDELINES §8).
pub const REVEAL_SECONDS: u32 = 60;

/// `list_items`.
#[tauri::command]
pub async fn list_items(
    state: State<'_, AppState>,
    request: ListItemsRequest,
) -> IpcResult<Vec<ItemSummary>> {
    state.run(move |b| b.list_items(&request)).await
}

/// `get_item`.
#[tauri::command]
pub async fn get_item(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<ItemDetail> {
    state.run(move |b| b.get_item(request)).await
}

/// `reveal_secret`.
#[tauri::command]
pub async fn reveal_secret(
    state: State<'_, AppState>,
    request: RevealRequest,
) -> IpcResult<RevealedSecret> {
    state.run(move |b| b.reveal_secret(&request)).await
}

/// `add_item`.
#[tauri::command]
pub async fn add_item(
    state: State<'_, AppState>,
    request: AddItemRequest,
) -> IpcResult<ItemDetail> {
    state.run(move |b| b.add_item(request)).await
}

/// `update_item`.
#[tauri::command]
pub async fn update_item(
    state: State<'_, AppState>,
    request: UpdateItemRequest,
) -> IpcResult<ItemDetail> {
    state.run(move |b| b.update_item(request)).await
}

/// `set_favorite`.
#[tauri::command]
pub async fn set_favorite(
    state: State<'_, AppState>,
    request: SetFavoriteRequest,
) -> IpcResult<ItemSummary> {
    state.run(move |b| b.set_favorite(request)).await
}

/// `trash_item`.
#[tauri::command]
pub async fn trash_item(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<()> {
    state
        .run(move |b| b.edit_item(request, UnlockedSession::trash_item))
        .await
}

/// `restore_item`.
#[tauri::command]
pub async fn restore_item(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<()> {
    state
        .run(move |b| b.edit_item(request, UnlockedSession::restore_item))
        .await
}

/// `purge_item`: delete forever (a tombstone).
#[tauri::command]
pub async fn purge_item(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<()> {
    state
        .run(move |b| b.edit_item(request, UnlockedSession::purge_item))
        .await
}

/// `move_item`.
#[tauri::command]
pub async fn move_item(
    state: State<'_, AppState>,
    request: MoveItemRequest,
) -> IpcResult<MovedItem> {
    state.run(move |b| b.move_item(request)).await
}

const fn core_trash(filter: TrashFilter) -> CoreTrashFilter {
    match filter {
        TrashFilter::Exclude => CoreTrashFilter::Exclude,
        TrashFilter::Only => CoreTrashFilter::Only,
        TrashFilter::Include => CoreTrashFilter::Include,
    }
}

fn accepts(request: &ListItemsRequest, entry: &ItemRef<'_>) -> bool {
    let item = entry.item;
    if item.is_deleted()
        || request.vault_id.is_some_and(|v| v.get() != entry.vault_id)
        || (request.favorites_only && !item.favorite())
    {
        return false;
    }
    let trashed = item.trashed_at().is_some();
    match request.trash {
        TrashFilter::Exclude => !trashed,
        TrashFilter::Only => trashed,
        TrashFilter::Include => true,
    }
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty()).map(str::to_owned)
}

/// The host of a URL as entered, without scheme, user info, port, path,
/// query or fragment. Display only; autofill matching is the extension's
/// (brief §4) and never uses this.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = if host_port.starts_with('[') {
        host_port
            .split_once(']')
            .map_or(host_port, |(h, _)| h)
            .trim_start_matches('[')
    } else {
        host_port.split(':').next().unwrap_or_default()
    };
    non_empty(Some(host)).map(|h| h.to_ascii_lowercase())
}

fn subtitle(kind: ItemKind, item: &Item) -> Option<String> {
    let field = match kind {
        ItemKind::Login => TextField::Username,
        ItemKind::CreditCard => TextField::CardholderName,
        ItemKind::Identity => TextField::Email,
        ItemKind::Wifi => TextField::WifiSsid,
        ItemKind::ApiKey => TextField::ApiKeyHost,
        ItemKind::SshKey => TextField::SshComment,
        ItemKind::Passkey
        | ItemKind::Totp
        | ItemKind::SecureNote
        | ItemKind::Document
        | ItemKind::Unsupported => return None,
    };
    non_empty(item.text(field))
}

fn has_otp(item: &Item) -> bool {
    item.otp().is_some()
        || item
            .custom_fields()
            .iter()
            .any(|(_, field)| matches!(field.value, CustomFieldValue::Otp(_)))
}

fn summarize(vault_id: VaultId, item_id: ItemId, item: &Item) -> ItemSummary {
    let kind = item
        .item_type()
        .as_ref()
        .map_or(ItemKind::Unsupported, ItemKind::from_model);
    ItemSummary {
        vault_id: vault_id.into(),
        item_id: item_id.into(),
        kind,
        title: non_empty(item.title()),
        subtitle: subtitle(kind, item),
        host: item
            .urls()
            .first()
            .and_then(|(_, url)| host_of(url.url.as_str())),
        favorite: item.favorite(),
        has_otp: has_otp(item),
        tags: item.tags().into_iter().map(str::to_owned).collect(),
        modified_at: item.modified_at().map(Timestamp::as_unix_millis),
        trashed_at: item.trashed_at().map(Timestamp::as_unix_millis),
        needs_review: !item.conflicts().is_empty()
            || item.deleted_concurrently().is_some()
            || item.conflict_copy_of().is_some(),
    }
}

fn custom_entry(element_id: String, label: String, value: &CustomFieldValue) -> CustomFieldEntry {
    let (kind, value) = match value {
        CustomFieldValue::Text(t) => (CustomFieldKind::Text, Some(t.as_str().to_owned())),
        CustomFieldValue::Hidden(_) => (CustomFieldKind::Hidden, None),
        CustomFieldValue::Url(t) => (CustomFieldKind::Url, Some(t.as_str().to_owned())),
        CustomFieldValue::Email(t) => (CustomFieldKind::Email, Some(t.as_str().to_owned())),
        CustomFieldValue::Phone(t) => (CustomFieldKind::Phone, Some(t.as_str().to_owned())),
        CustomFieldValue::Date(t) => (CustomFieldKind::Date, Some(t.as_str().to_owned())),
        CustomFieldValue::Otp(_) => (CustomFieldKind::Otp, None),
    };
    CustomFieldEntry {
        element_id,
        label,
        kind,
        value,
    }
}

fn detail(vault_id: VaultId, item_id: ItemId, item: &Item) -> IpcResult<ItemDetail> {
    let text = TextField::ALL
        .iter()
        .filter(|&&field| field != TextField::Title)
        .filter_map(|&field| {
            non_empty(item.text(field)).map(|value| TextFieldEntry {
                field: TextFieldKey::from_model(field),
                value,
            })
        })
        .collect();
    let mut secrets = Vec::new();
    for &field in SecretField::ALL {
        // Decrypted only to test for presence; the value zeroizes at once.
        if item.secret(field)?.is_some() {
            secrets.push(SecretFieldKey::from_model(field));
        }
    }
    let otp = item.otp().map(|config| OtpSummary {
        scheme: match config.kind {
            OtpKind::Totp => OtpScheme::Totp,
            OtpKind::Hotp => OtpScheme::Hotp,
            OtpKind::Steam => OtpScheme::Steam,
        },
        issuer: config.issuer.as_ref().map(|t| t.as_str().to_owned()),
        digits: config.digits,
        period: (config.kind != OtpKind::Hotp).then_some(config.period),
        hardware_protected: config.hardware_protected,
    });
    Ok(ItemDetail {
        summary: summarize(vault_id, item_id, item),
        created_at: item.created_at().map(Timestamp::as_unix_millis),
        text,
        secrets,
        urls: item
            .urls()
            .into_iter()
            .map(|(id, url)| UrlEntry {
                element_id: id.as_str().to_owned(),
                url: url.url.as_str().to_owned(),
                match_mode: match url.match_mode {
                    UrlMatch::ExactOrigin => UrlMatchMode::ExactOrigin,
                    UrlMatch::Etld1 => UrlMatchMode::Etld1,
                },
                allow_http: url.allow_http,
            })
            .collect(),
        custom_fields: item
            .custom_fields()
            .into_iter()
            .map(|(id, field)| {
                custom_entry(
                    id.as_str().to_owned(),
                    field.label.as_str().to_owned(),
                    &field.value,
                )
            })
            .collect(),
        attachments: item
            .attachments()
            .into_iter()
            .map(|a| AttachmentEntry {
                attachment_id: a.attachment_id.into(),
                file_name: a.file_name.as_str().to_owned(),
                mime: a.mime.as_str().to_owned(),
                size: a.size,
            })
            .collect(),
        otp,
        history_count: u32::try_from(item.password_history().len()).unwrap_or(u32::MAX),
        review: ReviewState {
            conflicting_fields: u32::try_from(item.conflicts().len()).unwrap_or(u32::MAX),
            deleted_concurrently: item.deleted_concurrently().is_some(),
            conflict_copy_of: item.conflict_copy_of().map(Into::into),
        },
    })
}

impl Backend {
    /// Lists or searches items.
    ///
    /// # Errors
    /// `locked`.
    pub fn list_items(&self, request: &ListItemsRequest) -> IpcResult<Vec<ItemSummary>> {
        self.touch();
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let query = request.query.as_deref().map(str::trim).unwrap_or_default();
        let entries = if query.is_empty() {
            let mut filter = ItemFilter::default();
            filter.vault = request.vault_id.map(WireVaultId::get);
            filter.trash = core_trash(request.trash);
            filter.favorites_only = request.favorites_only;
            session.items(&filter)
        } else {
            session.search(query)
        };
        Ok(entries
            .iter()
            .filter(|entry| accepts(request, entry))
            .map(|entry| summarize(entry.vault_id, entry.item_id, entry.item))
            .collect())
    }

    /// One item, secrets masked.
    ///
    /// # Errors
    /// `locked`, `vaultNotFound`, `itemNotFound`.
    pub fn get_item(&self, request: ItemRequest) -> IpcResult<ItemDetail> {
        self.touch();
        let slot = self.slot()?;
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let item = slot.unlocked()?.item(vault_id, item_id)?;
        if item.is_deleted() {
            return Err(IpcError::new(ErrorCode::ItemDeleted));
        }
        detail(vault_id, item_id, item)
    }

    /// One secret of an item.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `fieldEmpty`.
    pub fn reveal_secret(&self, request: &RevealRequest) -> IpcResult<RevealedSecret> {
        self.touch();
        let slot = self.slot()?;
        let item = slot
            .unlocked()?
            .item(request.vault_id.get(), request.item_id.get())?;
        let secret = match &request.secret {
            SecretRef::Field { field } => item.secret(field.to_model())?,
            SecretRef::CustomField { element_id } => item
                .custom_fields()
                .into_iter()
                .find(|(id, _)| id.as_str() == element_id)
                .and_then(|(_, field)| match field.value {
                    CustomFieldValue::Hidden(secret) => Some(secret),
                    _ => None,
                }),
            SecretRef::History { index } => usize::try_from(*index).ok().and_then(|index| {
                item.password_history()
                    .into_iter()
                    .nth(index)
                    .map(|entry| entry.value)
            }),
        };
        let value = secret.ok_or(IpcError::new(ErrorCode::FieldEmpty))?;
        Ok(RevealedSecret {
            value: SecretOut::new(value),
            hide_after_seconds: REVEAL_SECONDS,
        })
    }

    /// Stars or unstars an item.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, or a store error.
    pub fn set_favorite(&self, request: SetFavoriteRequest) -> IpcResult<ItemSummary> {
        self.touch();
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.set_favorite(vault_id, item_id, request.favorite)?;
        Ok(summarize(
            vault_id,
            item_id,
            session.item(vault_id, item_id)?,
        ))
    }

    /// Runs one item operation of the session.
    ///
    /// # Errors
    /// `locked`, or the operation's error.
    pub fn edit_item<F>(&self, request: ItemRequest, operation: F) -> IpcResult<()>
    where
        F: FnOnce(&mut UnlockedSession, VaultId, ItemId) -> mosaic_vault::session::Result<()>,
    {
        self.touch();
        let mut slot = self.slot()?;
        operation(
            slot.unlocked_mut()?,
            request.vault_id.get(),
            request.item_id.get(),
        )?;
        Ok(())
    }

    /// Adds an item from a draft and returns its detail.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (an `unsupported` kind or a draft the model
    /// refuses), `vaultNotFound`, `vaultUnavailable`.
    pub fn add_item(&self, request: AddItemRequest) -> IpcResult<ItemDetail> {
        self.touch();
        let item_type = request
            .kind
            .to_model()
            .ok_or(IpcError::new(ErrorCode::InvalidInput))?;
        let prepared = draft::prepare(request.draft)?;
        let vault_id = request.vault_id.get();
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let item_id = session
            .add_item(vault_id, &item_type, |edit| prepared.apply(edit, &[]))
            .map_err(draft::edit_error)?;
        detail(vault_id, item_id, session.item(vault_id, item_id)?)
    }

    /// Applies a draft to an item and returns its detail.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `vaultNotFound`, `itemNotFound`,
    /// `itemDeleted`, `vaultUnavailable`.
    pub fn update_item(&self, request: UpdateItemRequest) -> IpcResult<ItemDetail> {
        self.touch();
        let prepared = draft::prepare(request.draft)?;
        let (vault_id, item_id) = (request.vault_id.get(), request.item_id.get());
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let tags: Vec<String> = session
            .item(vault_id, item_id)?
            .tags()
            .into_iter()
            .map(str::to_owned)
            .collect();
        session
            .update_item(vault_id, item_id, |edit| prepared.apply(edit, &tags))
            .map_err(draft::edit_error)?;
        detail(vault_id, item_id, session.item(vault_id, item_id)?)
    }

    /// Moves an item to another vault (new item ID).
    ///
    /// # Errors
    /// `locked`, `sameVault`, `itemNotFound`, `vaultNotFound`.
    pub fn move_item(&self, request: MoveItemRequest) -> IpcResult<MovedItem> {
        self.touch();
        let mut slot = self.slot()?;
        let target = request.target_vault_id.get();
        let item_id = slot.unlocked_mut()?.move_item(
            request.vault_id.get(),
            request.item_id.get(),
            target,
        )?;
        Ok(MovedItem {
            vault_id: target.into(),
            item_id: item_id.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::host_of;

    #[test]
    fn host_of_strips_everything_but_the_host() {
        assert_eq!(
            host_of("https://GitHub.com/login").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            host_of("https://u:p@example.com:8443/x?y#z").as_deref(),
            Some("example.com")
        );
        assert_eq!(host_of("example.org").as_deref(), Some("example.org"));
        assert_eq!(host_of("http://[::1]:80/").as_deref(), Some("::1"));
        assert_eq!(host_of("https://").as_deref(), None);
    }
}
