//! `ItemDraft` → `ItemEdit`: the edits of `add_item` and `update_item`.
//!
//! Everything that can be checked without the item is checked first
//! ([`prepare`]): element IDs, URL and label lengths, `otpauth://` URIs. The
//! closure that runs inside the session ([`Prepared::apply`]) then only
//! calls the model's setters, and the whole draft is one edit and one sync
//! event; a failing setter stores nothing (`UnlockedSession::update_item`).
//! Secret values stay in zeroizing buffers from the IPC body to the model.

use mosaic_core::SecretString;
use mosaic_core::model::{
    CustomField, CustomFieldValue, ElementId, ItemEdit, LoginUrl, OtpConfig, SecretField, Text,
    TextField, UrlMatch,
};
use mosaic_totp::OtpAuth;
use mosaic_vault::otp::config_from_otp_auth;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::{
    CustomFieldChange, CustomFieldKind, ItemDraft, OtpChange, UrlChange, UrlMatchMode,
};

/// Longest accepted URL, in bytes.
pub const MAX_URL_LEN: usize = 4096;
/// Longest accepted custom-field label, in bytes.
pub const MAX_LABEL_LEN: usize = 256;
/// Most entries accepted in each list of a draft.
pub const MAX_DRAFT_ENTRIES: usize = 512;

fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidInput)
}

fn element(id: &str) -> IpcResult<ElementId> {
    ElementId::from_stored(id).map_err(|_| invalid())
}

fn login_url(url: &str, match_mode: UrlMatchMode, allow_http: bool) -> IpcResult<LoginUrl> {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_URL_LEN || url.chars().any(char::is_control) {
        return Err(invalid());
    }
    let mut login = LoginUrl::new(url);
    login.match_mode = match match_mode {
        UrlMatchMode::ExactOrigin => UrlMatch::ExactOrigin,
        UrlMatchMode::Etld1 => UrlMatch::Etld1,
    };
    login.allow_http = allow_http;
    Ok(login)
}

fn otp_config(uri: &str) -> IpcResult<(OtpConfig, u64)> {
    let auth = OtpAuth::parse(uri.trim()).map_err(|_| invalid())?;
    config_from_otp_auth(&auth).map_err(|_| invalid())
}

fn custom_field(label: &str, kind: CustomFieldKind, value: &str) -> IpcResult<CustomField> {
    if label.len() > MAX_LABEL_LEN || label.chars().any(char::is_control) {
        return Err(invalid());
    }
    let value = match kind {
        CustomFieldKind::Text => CustomFieldValue::Text(Text::new(value)),
        CustomFieldKind::Hidden => CustomFieldValue::Hidden(SecretString::from_str_copy(value)?),
        CustomFieldKind::Url => CustomFieldValue::Url(Text::new(value)),
        CustomFieldKind::Email => CustomFieldValue::Email(Text::new(value)),
        CustomFieldKind::Phone => CustomFieldValue::Phone(Text::new(value)),
        CustomFieldKind::Date => CustomFieldValue::Date(Text::new(value)),
        CustomFieldKind::Otp => CustomFieldValue::Otp(otp_config(value)?.0),
    };
    Ok(CustomField::new(Text::new(label), value))
}

enum UrlOp {
    Add(LoginUrl),
    Set(ElementId, LoginUrl),
    Remove(ElementId),
}

enum OtpOp {
    Keep,
    Remove,
    Set(OtpConfig, u64),
}

enum FieldOp {
    Add(CustomField),
    Set(ElementId, CustomField),
    Remove(ElementId),
}

/// A checked draft, ready to apply.
pub struct Prepared {
    text: Vec<(TextField, Option<String>)>,
    secrets: Vec<(SecretField, Option<SecretString>)>,
    urls: Vec<UrlOp>,
    fields: Vec<FieldOp>,
    tags: Option<Vec<String>>,
    otp: OtpOp,
}

/// Checks a draft.
///
/// # Errors
/// `invalidInput` for a list with more than [`MAX_DRAFT_ENTRIES`] entries,
/// a malformed element ID, URL, label or `otpauth://` URI.
pub fn prepare(draft: ItemDraft) -> IpcResult<Prepared> {
    if draft.text.len() > MAX_DRAFT_ENTRIES
        || draft.secrets.len() > MAX_DRAFT_ENTRIES
        || draft.urls.len() > MAX_DRAFT_ENTRIES
        || draft.custom_fields.len() > MAX_DRAFT_ENTRIES
        || draft
            .tags
            .as_ref()
            .is_some_and(|t| t.len() > MAX_DRAFT_ENTRIES)
    {
        return Err(invalid());
    }
    let text = draft
        .text
        .into_iter()
        .map(|change| {
            let value = change.value.filter(|v| !v.is_empty());
            (change.field.to_model(), value)
        })
        .collect();
    let mut secrets = Vec::with_capacity(draft.secrets.len());
    for change in draft.secrets {
        let value = change
            .value
            .map(crate::contract::secret::SecretIn::into_secret)
            .filter(|v| !v.expose_secret().is_empty());
        secrets.push((change.field.to_model(), value));
    }
    let mut urls = Vec::with_capacity(draft.urls.len());
    for change in draft.urls {
        urls.push(match change {
            UrlChange::Add {
                url,
                match_mode,
                allow_http,
            } => UrlOp::Add(login_url(&url, match_mode, allow_http)?),
            UrlChange::Update {
                element_id,
                url,
                match_mode,
                allow_http,
            } => UrlOp::Set(
                element(&element_id)?,
                login_url(&url, match_mode, allow_http)?,
            ),
            UrlChange::Remove { element_id } => UrlOp::Remove(element(&element_id)?),
        });
    }
    let mut fields = Vec::with_capacity(draft.custom_fields.len());
    for change in draft.custom_fields {
        fields.push(match change {
            CustomFieldChange::Add { label, kind, value } => {
                FieldOp::Add(custom_field(&label, kind, value.expose_secret())?)
            }
            CustomFieldChange::Update {
                element_id,
                label,
                kind,
                value,
            } => FieldOp::Set(
                element(&element_id)?,
                custom_field(&label, kind, value.expose_secret())?,
            ),
            CustomFieldChange::Remove { element_id } => FieldOp::Remove(element(&element_id)?),
        });
    }
    let otp = match draft.otp {
        None => OtpOp::Keep,
        Some(OtpChange::Remove) => OtpOp::Remove,
        Some(OtpChange::Set { uri }) => {
            let (config, counter) = otp_config(uri.expose_secret())?;
            OtpOp::Set(config, counter)
        }
    };
    Ok(Prepared {
        text,
        secrets,
        urls,
        fields,
        tags: draft.tags,
        otp,
    })
}

impl Prepared {
    /// Applies the draft to an item being edited. `current_tags` are the
    /// item's tags before the edit (empty for a new item).
    ///
    /// # Errors
    /// The model's errors (`Malformed` for a tag or element it refuses).
    pub fn apply(
        self,
        edit: &mut ItemEdit<'_>,
        current_tags: &[String],
    ) -> mosaic_core::Result<()> {
        for (field, value) in self.text {
            match value {
                Some(value) => edit.set_text(field, &value)?,
                None => edit.clear_text(field)?,
            }
        }
        for (field, value) in self.secrets {
            match value {
                Some(value) => edit.set_secret(field, value.expose_secret())?,
                None => edit.clear_secret(field)?,
            }
        }
        for op in self.urls {
            match op {
                UrlOp::Add(url) => {
                    edit.add_url(&url)?;
                }
                UrlOp::Set(id, url) => edit.set_url(&id, &url)?,
                UrlOp::Remove(id) => edit.remove_url(&id)?,
            }
        }
        for op in self.fields {
            match op {
                FieldOp::Add(field) => {
                    edit.add_custom_field(&field)?;
                }
                FieldOp::Set(id, field) => edit.set_custom_field(&id, &field)?,
                FieldOp::Remove(id) => edit.remove_custom_field(&id)?,
            }
        }
        if let Some(tags) = self.tags {
            for old in current_tags.iter().filter(|t| !tags.contains(t)) {
                edit.remove_tag(old)?;
            }
            for new in tags.iter().filter(|t| !current_tags.contains(t)) {
                edit.add_tag(new)?;
            }
        }
        match self.otp {
            OtpOp::Keep => {}
            OtpOp::Remove => edit.clear_otp()?,
            OtpOp::Set(config, counter) => {
                edit.set_otp(&config)?;
                if counter > 0 {
                    edit.set_otp_counter(counter)?;
                }
            }
        }
        Ok(())
    }
}

/// Maps a session error of an edit: what the model refuses is the
/// draft's fault (`invalidInput`); the rest maps as usual.
#[must_use]
pub fn edit_error(error: mosaic_vault::session::Error) -> IpcError {
    match error {
        mosaic_vault::session::Error::Core(
            mosaic_core::Error::Malformed | mosaic_core::Error::SizeLimit,
        ) => invalid(),
        other => other.into(),
    }
}
