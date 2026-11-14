//! Vaults and items: listing, search, detail, reveal, save and trash.
//!
//! IDs are canonical lowercase UUID strings and times are Unix milliseconds.
//! Lists and details never contain a concealed value; a secret leaves only
//! through [`Core::reveal`] as a [`SecretValue`] (CRYPTO_SPEC §6.9.5).

use std::sync::Arc;

use mosaic_core::model::{
    CustomFieldValue, Item, ItemType, LoginUrl, SecretField, TextField, UrlMatch,
};
use mosaic_core::{ItemId, VaultId};
use mosaic_vault::session::{
    ItemFilter as SessionFilter, Result as SessionResult, TrashFilter, UnlockedSession,
    VaultProtection,
};
use zeroize::Zeroizing;

use crate::Core;
use crate::error::{MobileError, Result};
use crate::otp::{OtpView, view_of};
use crate::secret::{SecretValue, text_of};

/// The kind of an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// Username, password, URLs, optional OTP.
    Login,
    /// A WebAuthn credential.
    Passkey,
    /// An OTP generator on its own.
    Totp,
    /// Notes only.
    SecureNote,
    /// A payment card.
    CreditCard,
    /// Personal details.
    Identity,
    /// An SSH key pair.
    SshKey,
    /// An API key or token.
    ApiKey,
    /// Wi-Fi credentials.
    Wifi,
    /// Files with notes.
    Document,
    /// A type written by a newer client.
    Other,
}

impl ItemKind {
    fn of(item_type: Option<&ItemType>) -> Self {
        match item_type {
            Some(ItemType::Login) => Self::Login,
            Some(ItemType::Passkey) => Self::Passkey,
            Some(ItemType::Totp) => Self::Totp,
            Some(ItemType::SecureNote) => Self::SecureNote,
            Some(ItemType::CreditCard) => Self::CreditCard,
            Some(ItemType::Identity) => Self::Identity,
            Some(ItemType::SshKey) => Self::SshKey,
            Some(ItemType::ApiKey) => Self::ApiKey,
            Some(ItemType::Wifi) => Self::Wifi,
            Some(ItemType::Document) => Self::Document,
            Some(ItemType::Other(_)) | None => Self::Other,
        }
    }
}

/// Which items a listing includes with respect to the trash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrashMode {
    /// Items not in the trash.
    #[default]
    Exclude,
    /// Only items in the trash.
    Only,
    /// Both.
    Include,
}

/// A listing filter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemFilter {
    /// Only this vault.
    pub vault_id: Option<String>,
    /// Trash handling.
    pub trash: TrashMode,
    /// Only favorites.
    pub favorites_only: bool,
    /// Only this kind.
    pub kind: Option<ItemKind>,
}

/// A vault in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultSummary {
    /// The vault ID.
    pub vault_id: String,
    /// The name, if readable.
    pub name: Option<String>,
    /// Live items.
    pub item_count: u32,
    /// Whether this session holds the vault's key (a hardware-protected vault
    /// does not until it is opened).
    pub available: bool,
}

/// An item's address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemKey {
    /// The vault ID.
    pub vault_id: String,
    /// The item ID.
    pub item_id: String,
}

/// An item in a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSummary {
    /// The vault ID.
    pub vault_id: String,
    /// The item ID.
    pub item_id: String,
    /// The kind.
    pub kind: ItemKind,
    /// The title.
    pub title: String,
    /// A second line (the user name, for a login).
    pub subtitle: Option<String>,
    /// Marked as a favorite.
    pub favorite: bool,
    /// Has a one-time-password generator.
    pub has_otp: bool,
    /// Holds a passkey.
    pub has_passkey: bool,
    /// The host of the first URL (the package of an app URL), for display.
    pub primary_host: Option<String>,
    /// The last edit.
    pub updated_at_ms: i64,
    /// In a hardware-protected vault.
    pub protected: bool,
}

/// A field of the detail. A concealed field has no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldView {
    /// An opaque reference, the argument of [`Core::reveal`].
    pub key: String,
    /// The i18n key of the label (a built-in field).
    pub label_key: Option<String>,
    /// The label (a custom field).
    pub label: Option<String>,
    /// The value is a secret.
    pub concealed: bool,
    /// The value; `None` for every concealed field.
    pub value: Option<String>,
    /// The value can be copied.
    pub copyable: bool,
}

/// A login URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlView {
    /// The URL as saved.
    pub url: String,
    /// Match the registrable domain instead of the exact origin.
    pub match_etld1: bool,
    /// Fill over plain HTTP is allowed.
    pub allow_http: bool,
    /// An Android app URL.
    pub is_app: bool,
    /// An Android app URL without a certificate: it never matches.
    pub needs_linking: bool,
}

/// A login URL in a draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlDraft {
    /// The URL.
    pub url: String,
    /// Match the registrable domain.
    pub match_etld1: bool,
    /// Allow fill over plain HTTP.
    pub allow_http: bool,
}

/// A passkey of an item, without key material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyView {
    /// The relying party.
    pub rp_id: String,
    /// The user name.
    pub user_name: String,
    /// The display name.
    pub display_name: String,
    /// When it was created.
    pub created_at_ms: i64,
}

/// An attachment of an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentView {
    /// The attachment ID.
    pub attachment_id: String,
    /// The file name.
    pub file_name: String,
    /// The size in bytes.
    pub size: u64,
}

/// An item in full, without any concealed value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemDetail {
    /// The list entry.
    pub summary: ItemSummary,
    /// The fields.
    pub fields: Vec<FieldView>,
    /// The URLs.
    pub urls: Vec<UrlView>,
    /// The tags.
    pub tags: Vec<String>,
    /// The passkey, if any.
    pub passkeys: Vec<PasskeyView>,
    /// The attachments.
    pub attachments: Vec<AttachmentView>,
    /// The generator, without its secret.
    pub otp: Option<OtpView>,
    /// How many earlier passwords the history holds.
    pub history_count: u32,
}

/// What a login draft does with the generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OtpEdit {
    /// Leave it.
    Keep,
    /// Remove it.
    Remove,
    /// Set it from a pending setup ([`Core::otp_from_text`]).
    SetPending {
        /// The pending ID.
        id: String,
    },
}

/// A login to create or change.
pub struct LoginDraft {
    /// The vault.
    pub vault_id: String,
    /// `None` creates.
    pub item_id: Option<String>,
    /// The title (not empty).
    pub title: String,
    /// The user name (empty clears it).
    pub username: String,
    /// The password, UTF-8. `None` keeps it; empty clears it.
    pub password: Option<Vec<u8>>,
    /// The URLs: they replace the saved ones.
    pub urls: Vec<UrlDraft>,
    /// The notes, UTF-8. `None` keeps them; empty clears them.
    pub notes: Option<Vec<u8>>,
    /// The generator.
    pub otp: OtpEdit,
    /// Marked as a favorite.
    pub favorite: bool,
}

/// A note to create or change.
pub struct NoteDraft {
    /// The vault.
    pub vault_id: String,
    /// `None` creates.
    pub item_id: Option<String>,
    /// The title (not empty).
    pub title: String,
    /// The note, UTF-8. `None` keeps it; empty clears it.
    pub notes: Option<Vec<u8>>,
    /// Marked as a favorite.
    pub favorite: bool,
}

/// A canonical lowercase hyphenated UUID back to its 16 bytes.
pub(crate) fn parse_uuid_bytes(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return None;
    }
    let mut out = [0u8; 16];
    let mut nibbles = bytes.iter().enumerate().filter_map(|(i, b)| match (i, b) {
        (8 | 13 | 18 | 23, b'-') => None,
        (8 | 13 | 18 | 23, _) => Some(None),
        (_, b'0'..=b'9') => Some(Some(b - b'0')),
        (_, b'a'..=b'f') => Some(Some(b - b'a' + 10)),
        _ => Some(None),
    });
    for byte in &mut out {
        let hi = nibbles.next()??;
        let lo = nibbles.next()??;
        *byte = (hi << 4) | lo;
    }
    nibbles.next().is_none().then_some(out)
}

pub(crate) fn parse_vault_id(text: &str) -> Result<VaultId> {
    let bytes = parse_uuid_bytes(text).ok_or(MobileError::InvalidInput)?;
    VaultId::from_bytes(bytes).map_err(|_| MobileError::InvalidInput)
}

pub(crate) fn parse_item_key(vault: &str, item: &str) -> Result<(VaultId, ItemId)> {
    let item_bytes = parse_uuid_bytes(item).ok_or(MobileError::InvalidInput)?;
    Ok((
        parse_vault_id(vault)?,
        ItemId::from_bytes(item_bytes).map_err(|_| MobileError::InvalidInput)?,
    ))
}

fn non_empty(text: Option<&str>) -> Option<String> {
    text.filter(|t| !t.is_empty()).map(str::to_owned)
}

/// The host (or app package) of a saved URL, for display.
fn display_host(url: &str) -> Option<String> {
    if let Ok(app) = mosaic_origin::app::AndroidAppUrl::parse(url) {
        return Some(app.package().to_owned());
    }
    mosaic_origin::saved_origin(url).map(|o| o.host_str())
}

fn subtitle_of(item: &Item, kind: &ItemKind) -> Option<String> {
    let text = |f: TextField| non_empty(item.text(f));
    match kind {
        ItemKind::Login => text(TextField::Username),
        ItemKind::Passkey => item
            .passkey_public()
            .and_then(|p| non_empty(Some(p.user_name.as_str()))),
        ItemKind::CreditCard => text(TextField::CardholderName),
        ItemKind::Identity => text(TextField::Email),
        ItemKind::ApiKey => text(TextField::ApiKeyId),
        ItemKind::Wifi => text(TextField::WifiSsid),
        _ => None,
    }
}

fn summary_of(
    session: &UnlockedSession,
    vault: VaultId,
    item_id: ItemId,
    item: &Item,
) -> ItemSummary {
    let kind = ItemKind::of(item.item_type().as_ref());
    let protected = session
        .vault_protection(vault)
        .is_ok_and(|p| matches!(p, VaultProtection::HardwareKey { .. }));
    ItemSummary {
        vault_id: vault.to_string(),
        item_id: item_id.to_string(),
        subtitle: subtitle_of(item, &kind),
        kind,
        title: item.title().unwrap_or_default().to_owned(),
        favorite: item.favorite(),
        has_otp: item.otp().is_some(),
        has_passkey: item.passkey_public().is_some(),
        primary_host: item
            .urls()
            .iter()
            .find_map(|(_, u)| display_host(u.url.as_str())),
        updated_at_ms: item
            .modified_at()
            .map_or(0, mosaic_core::Timestamp::as_unix_millis),
        protected,
    }
}

fn url_view(url: &LoginUrl) -> UrlView {
    let parsed = mosaic_origin::app::AndroidAppUrl::parse(url.url.as_str());
    UrlView {
        url: url.url.as_str().to_owned(),
        match_etld1: url.match_mode == UrlMatch::Etld1,
        allow_http: url.allow_http,
        is_app: parsed.is_ok(),
        needs_linking: url.url.as_str().starts_with("androidapp://")
            && parsed.as_ref().map_or(true, |a| a.cert_sha256().is_none()),
    }
}

/// The fields of an item: no concealed value, only references.
fn fields_of(item: &Item) -> Vec<FieldView> {
    let mut fields = Vec::new();
    for field in TextField::ALL {
        if *field == TextField::Title {
            continue;
        }
        if let Some(value) = non_empty(item.text(*field)) {
            fields.push(FieldView {
                key: field.key().to_owned(),
                label_key: Some(format!("field.{}", field.key())),
                label: None,
                concealed: false,
                value: Some(value),
                copyable: true,
            });
        }
    }
    for field in SecretField::ALL {
        if item.secret(*field).ok().flatten().is_some() {
            fields.push(FieldView {
                key: field.key().to_owned(),
                label_key: Some(format!("field.{}", field.key())),
                label: None,
                concealed: true,
                value: None,
                copyable: true,
            });
        }
    }
    if let Some(ssh) = item.ssh_key() {
        fields.push(FieldView {
            key: "ssh.public_key".to_owned(),
            label_key: Some("field.ssh.public_key".to_owned()),
            label: None,
            concealed: false,
            value: Some(ssh.public_key.as_str().to_owned()),
            copyable: true,
        });
        fields.push(FieldView {
            key: "ssh.private_key".to_owned(),
            label_key: Some("field.ssh.private_key".to_owned()),
            label: None,
            concealed: true,
            value: None,
            copyable: true,
        });
    }
    for (id, custom) in item.custom_fields() {
        let (concealed, value) = match &custom.value {
            CustomFieldValue::Text(t)
            | CustomFieldValue::Url(t)
            | CustomFieldValue::Email(t)
            | CustomFieldValue::Phone(t)
            | CustomFieldValue::Date(t) => (false, Some(t.as_str().to_owned())),
            CustomFieldValue::Hidden(_) | CustomFieldValue::Otp(_) => (true, None),
        };
        fields.push(FieldView {
            key: format!("custom:{}", id.as_str()),
            label_key: None,
            label: Some(custom.label.as_str().to_owned()),
            concealed,
            value,
            copyable: true,
        });
    }
    fields
}

/// The detail of an item: no concealed value, only references.
fn detail_of(
    session: &UnlockedSession,
    vault: VaultId,
    item_id: ItemId,
    item: &Item,
) -> ItemDetail {
    ItemDetail {
        summary: summary_of(session, vault, item_id, item),
        fields: fields_of(item),
        urls: item.urls().iter().map(|(_, u)| url_view(u)).collect(),
        tags: item.tags().into_iter().map(str::to_owned).collect(),
        passkeys: item
            .passkey_public()
            .map(|p| PasskeyView {
                rp_id: p.rp_id.as_str().to_owned(),
                user_name: p.user_name.as_str().to_owned(),
                display_name: p.user_display_name.as_str().to_owned(),
                created_at_ms: p.created_at.as_unix_millis(),
            })
            .into_iter()
            .collect(),
        attachments: item
            .attachments()
            .iter()
            .map(|a| AttachmentView {
                attachment_id: a.attachment_id.to_string(),
                file_name: a.file_name.as_str().to_owned(),
                size: a.size,
            })
            .collect(),
        otp: item.otp().as_ref().map(view_of),
        history_count: u32::try_from(item.password_history().len()).unwrap_or(u32::MAX),
    }
}

/// The value of the hidden custom field `custom_id`.
fn hidden_custom(item: &Item, custom_id: &str) -> Option<Zeroizing<String>> {
    item.custom_fields().iter().find_map(|(id, field)| {
        if id.as_str() != custom_id {
            return None;
        }
        match &field.value {
            CustomFieldValue::Hidden(v) => Some(Zeroizing::new(v.expose_secret().to_owned())),
            _ => None,
        }
    })
}

fn sort_summaries(list: &mut [ItemSummary]) {
    list.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then_with(|| a.item_id.cmp(&b.item_id))
    });
}

/// The login URLs of a draft against the saved ones: what to remove and add.
fn url_changes(
    saved: &[(mosaic_core::model::ElementId, LoginUrl)],
    wanted: &[UrlDraft],
) -> (Vec<mosaic_core::model::ElementId>, Vec<LoginUrl>) {
    let to_url = |d: &UrlDraft| {
        let mut url = LoginUrl::new(d.url.trim());
        url.match_mode = if d.match_etld1 {
            UrlMatch::Etld1
        } else {
            UrlMatch::ExactOrigin
        };
        url.allow_http = d.allow_http;
        url
    };
    let same = |a: &LoginUrl, b: &LoginUrl| {
        a.url.as_str() == b.url.as_str()
            && a.match_mode == b.match_mode
            && a.allow_http == b.allow_http
    };
    let mut used = vec![false; saved.len()];
    let mut add = Vec::new();
    for draft in wanted {
        let url = to_url(draft);
        let hit = saved
            .iter()
            .enumerate()
            .find(|(i, (_, s))| !used.get(*i).copied().unwrap_or(true) && same(s, &url));
        match hit {
            Some((i, _)) => {
                if let Some(flag) = used.get_mut(i) {
                    *flag = true;
                }
            }
            None => add.push(url),
        }
    }
    let remove = saved
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|((id, _), _)| id.clone())
        .collect();
    (remove, add)
}

/// Common draft input after the secret bytes are wrapped.
struct Edits {
    title: String,
    username: Option<String>,
    password: Option<Zeroizing<String>>,
    notes: Option<Zeroizing<String>>,
    urls: Option<Vec<UrlDraft>>,
    favorite: bool,
    otp: Option<(Option<mosaic_core::model::OtpConfig>, u64)>,
}

/// What an item holds now, copied out so the edit can run without a borrow.
struct Current {
    title: Option<String>,
    username: Option<String>,
    notes: Option<String>,
    favorite: bool,
}

impl Current {
    fn of(item: &Item) -> Self {
        Self {
            title: item.title().map(str::to_owned),
            username: item.text(TextField::Username).map(str::to_owned),
            notes: item.text(TextField::Notes).map(str::to_owned),
            favorite: item.favorite(),
        }
    }
}

fn apply(
    e: &mut mosaic_core::model::ItemEdit<'_>,
    current: Option<&Current>,
    edits: &Edits,
    saved_urls: &[(mosaic_core::model::ElementId, LoginUrl)],
) -> mosaic_core::Result<()> {
    if current.is_none_or(|c| c.title.as_deref() != Some(edits.title.as_str())) {
        e.set_title(&edits.title)?;
    }
    if let Some(username) = &edits.username {
        let old = current.and_then(|c| c.username.as_deref()).unwrap_or("");
        if old != username {
            if username.is_empty() {
                e.clear_text(TextField::Username)?;
            } else {
                e.set_text(TextField::Username, username)?;
            }
        }
    }
    if let Some(password) = &edits.password {
        if password.is_empty() {
            e.clear_secret(SecretField::Password)?;
        } else {
            e.set_secret(SecretField::Password, password)?;
        }
    }
    if let Some(notes) = &edits.notes {
        let old = current.and_then(|c| c.notes.as_deref()).unwrap_or("");
        if old != notes.as_str() {
            if notes.is_empty() {
                e.clear_text(TextField::Notes)?;
            } else {
                e.set_text(TextField::Notes, notes)?;
            }
        }
    }
    if let Some(urls) = &edits.urls {
        let (remove, add) = url_changes(saved_urls, urls);
        for id in &remove {
            e.remove_url(id)?;
        }
        for url in &add {
            e.add_url(url)?;
        }
    }
    match &edits.otp {
        Some((Some(config), counter)) => {
            e.set_otp(config)?;
            if *counter > 0 {
                e.set_otp_counter(*counter)?;
            }
        }
        Some((None, _)) => e.clear_otp()?,
        None => {}
    }
    if current.map_or(edits.favorite, |c| c.favorite != edits.favorite) {
        e.set_favorite(edits.favorite)?;
    }
    Ok(())
}

fn save(
    session: &mut UnlockedSession,
    vault: VaultId,
    item_id: Option<ItemId>,
    item_type: &ItemType,
    edits: &Edits,
) -> SessionResult<ItemId> {
    if let Some(id) = item_id {
        let (current, saved) = {
            let item = session.item(vault, id)?;
            if item.is_deleted() {
                return Err(mosaic_vault::session::Error::ItemDeleted);
            }
            (Current::of(item), item.urls())
        };
        session.update_item(vault, id, |e| apply(e, Some(&current), edits, &saved))?;
        Ok(id)
    } else {
        session.add_item(vault, item_type, |e| apply(e, None, edits, &[]))
    }
}

impl Core {
    /// The vaults.
    ///
    /// # Errors
    /// `locked`.
    pub fn vaults(&self) -> Result<Vec<VaultSummary>> {
        self.with_session(|s| {
            Ok(s.vaults()
                .into_iter()
                .map(|v| VaultSummary {
                    vault_id: v.vault_id.to_string(),
                    name: v.name.map(str::to_owned),
                    item_count: u32::try_from(v.item_count).unwrap_or(u32::MAX),
                    available: v.available,
                })
                .collect())
        })
    }

    /// The items that pass `filter`, by title.
    ///
    /// # Errors
    /// `locked`, `invalidInput` for a malformed vault ID.
    pub fn items(&self, filter: ItemFilter) -> Result<Vec<ItemSummary>> {
        let vault = filter.vault_id.as_deref().map(parse_vault_id).transpose()?;
        self.with_session(|s| {
            let mut f = SessionFilter::default();
            f.vault = vault;
            f.favorites_only = filter.favorites_only;
            f.trash = match filter.trash {
                TrashMode::Exclude => TrashFilter::Exclude,
                TrashMode::Only => TrashFilter::Only,
                TrashMode::Include => TrashFilter::Include,
            };
            let mut list: Vec<ItemSummary> = s
                .items(&f)
                .into_iter()
                .map(|r| summary_of(s, r.vault_id, r.item_id, r.item))
                .filter(|i| filter.kind.as_ref().is_none_or(|k| *k == i.kind))
                .collect();
            sort_summaries(&mut list);
            Ok(list)
        })
    }

    /// Items matching `query` (titles, user names, URLs, tags), by title.
    ///
    /// # Errors
    /// `locked`.
    pub fn search(&self, query: String) -> Result<Vec<ItemSummary>> {
        self.with_session(|s| {
            let mut list: Vec<ItemSummary> = s
                .search(&query)
                .into_iter()
                .map(|r| summary_of(s, r.vault_id, r.item_id, r.item))
                .collect();
            sort_summaries(&mut list);
            Ok(list)
        })
    }

    /// One item in full, without any concealed value.
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput`.
    pub fn item(&self, vault_id: &str, item_id: &str) -> Result<ItemDetail> {
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        self.with_session(|s| {
            let item = s.item(vault, id)?;
            if item.is_deleted() {
                return Err(MobileError::NotFound);
            }
            Ok(detail_of(s, vault, id, item))
        })
    }

    /// A concealed field's value. `field` is the key of a [`FieldView`].
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput`.
    pub fn reveal(&self, vault_id: &str, item_id: &str, field: &str) -> Result<Arc<SecretValue>> {
        let generation = self.generation();
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        let value = self.with_session(|s| {
            let item = s.item(vault, id)?;
            if item.is_deleted() {
                return Err(MobileError::NotFound);
            }
            let text: Option<Zeroizing<String>> = if let Some(f) = SecretField::from_key(field) {
                item.secret(f)
                    .map_err(MobileError::from)?
                    .map(|v| Zeroizing::new(v.expose_secret().to_owned()))
            } else if field == "ssh.private_key" {
                item.ssh_key()
                    .map(|k| Zeroizing::new(k.private_key.expose_secret().to_owned()))
            } else if let Some(custom_id) = field.strip_prefix("custom:") {
                hidden_custom(item, custom_id)
            } else {
                return Err(MobileError::InvalidInput);
            };
            text.ok_or(MobileError::NotFound)
        })?;
        self.work_pause();
        self.issue_gated(SecretValue::from_text(&value), generation)
    }

    /// Creates or changes a login.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (an empty title, a bad ID, a draft URL that is
    /// too long), `notFound`, `vaultUnavailable`, `itemTypeNotAllowed`.
    pub fn save_login(&self, draft: LoginDraft) -> Result<ItemKey> {
        let LoginDraft {
            vault_id,
            item_id,
            title,
            username,
            password,
            urls,
            notes,
            otp,
            favorite,
        } = draft;
        if title.trim().is_empty() {
            return Err(MobileError::InvalidInput);
        }
        let vault = parse_vault_id(&vault_id)?;
        let id = item_id
            .as_deref()
            .map(|i| parse_item_key(&vault_id, i).map(|(_, id)| id))
            .transpose()?;
        let password = password.map(text_of).transpose()?;
        let notes = notes.map(text_of).transpose()?;
        let otp = match otp {
            OtpEdit::Keep => None,
            OtpEdit::Remove => Some((None, 0)),
            OtpEdit::SetPending { id } => {
                if !self.pending_otp_exists(&id)? {
                    return Err(MobileError::NotFound);
                }
                let (config, counter) = self.take_pending_otp(&id)?;
                Some((Some(config), counter))
            }
        };
        let edits = Edits {
            title: title.trim().to_owned(),
            username: Some(username),
            password,
            notes,
            urls: Some(urls),
            favorite,
            otp,
        };
        let saved = self.with_session(|s| {
            save(s, vault, id, &ItemType::Login, &edits).map_err(MobileError::from)
        })?;
        Ok(ItemKey {
            vault_id,
            item_id: saved.to_string(),
        })
    }

    /// Creates or changes a secure note.
    ///
    /// # Errors
    /// As [`Core::save_login`].
    pub fn save_note(&self, draft: NoteDraft) -> Result<ItemKey> {
        let NoteDraft {
            vault_id,
            item_id,
            title,
            notes,
            favorite,
        } = draft;
        if title.trim().is_empty() {
            return Err(MobileError::InvalidInput);
        }
        let vault = parse_vault_id(&vault_id)?;
        let id = item_id
            .as_deref()
            .map(|i| parse_item_key(&vault_id, i).map(|(_, id)| id))
            .transpose()?;
        let edits = Edits {
            title: title.trim().to_owned(),
            username: None,
            password: None,
            notes: notes.map(text_of).transpose()?,
            urls: None,
            favorite,
            otp: None,
        };
        let saved = self.with_session(|s| {
            save(s, vault, id, &ItemType::SecureNote, &edits).map_err(MobileError::from)
        })?;
        Ok(ItemKey {
            vault_id,
            item_id: saved.to_string(),
        })
    }

    /// Moves an item to the trash.
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput`.
    pub fn trash_item(&self, vault_id: &str, item_id: &str) -> Result<()> {
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        self.with_session(|s| Ok(s.trash_item(vault, id)?))
    }

    /// Marks or unmarks an item as a favorite.
    ///
    /// # Errors
    /// `locked`, `notFound`, `invalidInput`.
    pub fn set_favorite(&self, vault_id: &str, item_id: &str, favorite: bool) -> Result<()> {
        let (vault, id) = parse_item_key(vault_id, item_id)?;
        self.with_session(|s| Ok(s.set_favorite(vault, id, favorite)?))
    }
}
