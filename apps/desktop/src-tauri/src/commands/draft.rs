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
    CustomField, CustomFieldValue, ElementId, ItemEdit, LoginUrl, OtpConfig, SecretField,
    SshKeyMaterial, Text, TextField, UrlMatch,
};
use mosaic_totp::OtpAuth;
use mosaic_vault::otp::config_from_otp_auth;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::{
    CustomFieldChange, CustomFieldKind, ItemDraft, OtpChange, SshKeyChange, UrlChange, UrlMatchMode,
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

fn check_label(label: &str) -> IpcResult<()> {
    if label.len() > MAX_LABEL_LEN || label.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(())
}

fn kind_of(value: &CustomFieldValue) -> CustomFieldKind {
    match value {
        CustomFieldValue::Text(_) => CustomFieldKind::Text,
        CustomFieldValue::Hidden(_) => CustomFieldKind::Hidden,
        CustomFieldValue::Url(_) => CustomFieldKind::Url,
        CustomFieldValue::Email(_) => CustomFieldKind::Email,
        CustomFieldValue::Phone(_) => CustomFieldKind::Phone,
        CustomFieldValue::Date(_) => CustomFieldKind::Date,
        CustomFieldValue::Otp(_) => CustomFieldKind::Otp,
    }
}

fn custom_field(label: &str, kind: CustomFieldKind, value: &str) -> IpcResult<CustomField> {
    check_label(label)?;
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
    /// Rename a field and keep its stored value (and unknown keys); the
    /// kind must be the stored one.
    Relabel(ElementId, Text, CustomFieldKind),
    Remove(ElementId),
}

fn url_op(change: UrlChange) -> IpcResult<UrlOp> {
    Ok(match change {
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
    })
}

fn field_op(change: CustomFieldChange) -> IpcResult<FieldOp> {
    Ok(match change {
        CustomFieldChange::Add { label, kind, value } => {
            FieldOp::Add(custom_field(&label, kind, value.expose_secret())?)
        }
        CustomFieldChange::Update {
            element_id,
            label,
            kind,
            value: Some(value),
        } => FieldOp::Set(
            element(&element_id)?,
            custom_field(&label, kind, value.expose_secret())?,
        ),
        CustomFieldChange::Update {
            element_id,
            label,
            kind,
            value: None,
        } => {
            check_label(&label)?;
            FieldOp::Relabel(element(&element_id)?, Text::new(&label), kind)
        }
        CustomFieldChange::Remove { element_id } => FieldOp::Remove(element(&element_id)?),
    })
}

fn otp_op(change: Option<OtpChange>, pending: Option<(OtpConfig, u64)>) -> IpcResult<OtpOp> {
    Ok(match change {
        None => OtpOp::Keep,
        Some(OtpChange::Remove) => OtpOp::Remove,
        Some(OtpChange::Set { uri }) => {
            let (config, counter) = otp_config(uri.expose_secret())?;
            OtpOp::Set(config, counter)
        }
        Some(OtpChange::SetPending { .. }) => {
            let (config, counter) = pending.ok_or(IpcError::new(ErrorCode::OtpPendingExpired))?;
            OtpOp::Set(config, counter)
        }
    })
}

/// A checked draft, ready to apply.
pub struct Prepared {
    text: Vec<(TextField, Option<String>)>,
    secrets: Vec<(SecretField, Option<SecretString>)>,
    urls: Vec<UrlOp>,
    fields: Vec<FieldOp>,
    tags: Option<Vec<String>>,
    otp: OtpOp,
    ssh_key: Option<SshKeyMaterial>,
}

/// Checks a draft. `pending` is the configuration (and HOTP counter) of the
/// scanned setup an `OtpChange::SetPending` refers to; the caller took it
/// from the pending slots (`None` for every other caller).
///
/// # Errors
/// `otpPendingExpired` for a `SetPending` without `pending`, `invalidInput` for a list with more than [`MAX_DRAFT_ENTRIES`] entries,
/// a malformed element ID, URL, label or `otpauth://` URI, or a title with
/// invisible or direction-changing characters.
pub fn prepare(draft: ItemDraft, pending: Option<(OtpConfig, u64)>) -> IpcResult<Prepared> {
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
    let mut text = Vec::with_capacity(draft.text.len());
    for change in draft.text {
        let field = change.field.to_model();
        let value = change.value.filter(|v| !v.is_empty());
        // A title that hides or reorders text could pass for another item in
        // a list, the quick-search palette or an approval prompt.
        if field == TextField::Title && value.as_deref().is_some_and(mosaic_core::has_spoofing_char)
        {
            return Err(invalid());
        }
        text.push((field, value));
    }
    let mut secrets = Vec::with_capacity(draft.secrets.len());
    for change in draft.secrets {
        let value = change
            .value
            .map(crate::contract::secret::SecretIn::into_secret)
            .filter(|v| !v.expose_secret().is_empty());
        secrets.push((change.field.to_model(), value));
    }
    let urls = draft
        .urls
        .into_iter()
        .map(url_op)
        .collect::<IpcResult<Vec<_>>>()?;
    let fields = draft
        .custom_fields
        .into_iter()
        .map(field_op)
        .collect::<IpcResult<Vec<_>>>()?;
    let otp = otp_op(draft.otp, pending)?;
    let ssh_key = match draft.ssh_key {
        None => None,
        Some(SshKeyChange::Set { private_key }) => Some(
            mosaic_ssh_agent::material_from_openssh(private_key.expose_secret())
                .map_err(|_| IpcError::new(ErrorCode::SshKeyInvalid))?,
        ),
    };
    Ok(Prepared {
        text,
        secrets,
        urls,
        fields,
        tags: draft.tags,
        otp,
        ssh_key,
    })
}

impl Prepared {
    /// Whether the draft sets SSH key material (only `sshKey` items take it).
    #[must_use]
    pub const fn sets_ssh_key(&self) -> bool {
        self.ssh_key.is_some()
    }

    /// Applies the draft to an item being edited. `current_tags` and
    /// `current_fields` are the item's tags and custom fields before the
    /// edit (empty for a new item).
    ///
    /// # Errors
    /// The model's errors (`Malformed` for a tag or element it refuses, or
    /// for a rename-only custom field change that names no stored field or
    /// another kind than the stored one).
    pub fn apply(
        self,
        edit: &mut ItemEdit<'_>,
        current_tags: &[String],
        mut current_fields: Vec<(ElementId, CustomField)>,
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
                FieldOp::Relabel(id, label, kind) => {
                    let mut field = current_fields
                        .iter()
                        .position(|(stored, _)| *stored == id)
                        .map(|at| current_fields.swap_remove(at).1)
                        .filter(|field| kind_of(&field.value) == kind)
                        .ok_or(mosaic_core::Error::Malformed)?;
                    field.label = label;
                    edit.set_custom_field(&id, &field)?;
                }
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
        if let Some(key) = &self.ssh_key {
            edit.set_ssh_key(key)?;
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

#[cfg(test)]
mod tests {
    use mosaic_core::DeviceId;
    use mosaic_core::model::{Item, ItemType, Timestamp};

    use super::*;
    use crate::contract::secret::SecretIn;

    const ED25519: &str = include_str!("../../../../../crates/ssh-agent/tests/keys/ed25519");
    const ED25519_PUB: &str =
        include_str!("../../../../../crates/ssh-agent/tests/keys/ed25519.pub");

    fn empty_draft() -> ItemDraft {
        ItemDraft {
            text: vec![],
            secrets: vec![],
            urls: vec![],
            custom_fields: vec![],
            tags: None,
            otp: None,
            ssh_key: None,
        }
    }

    fn device() -> DeviceId {
        DeviceId::new_v7().unwrap()
    }

    fn at() -> Timestamp {
        Timestamp::from_unix_millis(1_761_480_000_000)
    }

    /// A login with the hidden custom field "PIN" = "1234".
    fn item_with_pin() -> (Item, String) {
        let mut item = Item::new(&ItemType::Login, device(), at()).unwrap();
        let draft = ItemDraft {
            custom_fields: vec![CustomFieldChange::Add {
                label: "PIN".into(),
                kind: CustomFieldKind::Hidden,
                value: SecretIn::test("1234"),
            }],
            ..empty_draft()
        };
        let prepared = prepare(draft, None).unwrap();
        prepared
            .apply(&mut item.edit(device(), at()), &[], Vec::new())
            .unwrap();
        let id = item.custom_fields()[0].0.as_str().to_owned();
        (item, id)
    }

    fn relabel(
        item: &mut Item,
        element_id: &str,
        label: &str,
        kind: CustomFieldKind,
    ) -> mosaic_core::Result<()> {
        let draft = ItemDraft {
            custom_fields: vec![CustomFieldChange::Update {
                element_id: element_id.into(),
                label: label.into(),
                kind,
                value: None,
            }],
            ..empty_draft()
        };
        let fields = item.custom_fields();
        prepare(draft, None)
            .unwrap()
            .apply(&mut item.edit(device(), at()), &[], fields)
    }

    #[test]
    fn custom_update_without_value_keeps_hidden_value() {
        let (mut item, id) = item_with_pin();
        relabel(&mut item, &id, "PIN code", CustomFieldKind::Hidden).unwrap();
        let fields = item.custom_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].1.label.as_str(), "PIN code");
        assert!(matches!(
            &fields[0].1.value,
            CustomFieldValue::Hidden(s) if s.expose_secret() == "1234"
        ));
    }

    #[test]
    fn custom_update_without_value_refuses_kind_change() {
        let (mut item, id) = item_with_pin();
        let error = relabel(&mut item, &id, "PIN", CustomFieldKind::Text).unwrap_err();
        assert_eq!(error, mosaic_core::Error::Malformed);
        let session_error = mosaic_vault::session::Error::Core(error);
        assert_eq!(edit_error(session_error).code, ErrorCode::InvalidInput);
        assert!(matches!(
            &item.custom_fields()[0].1.value,
            CustomFieldValue::Hidden(s) if s.expose_secret() == "1234"
        ));
    }

    #[test]
    fn custom_update_without_value_unknown_element_is_invalid() {
        let (mut item, _) = item_with_pin();
        let error = relabel(&mut item, "999-0000", "x", CustomFieldKind::Hidden).unwrap_err();
        assert_eq!(error, mosaic_core::Error::Malformed);
        // A bad label is refused before anything runs.
        let draft = ItemDraft {
            custom_fields: vec![CustomFieldChange::Update {
                element_id: "999-0000".into(),
                label: "bad\u{7}".into(),
                kind: CustomFieldKind::Hidden,
                value: None,
            }],
            ..empty_draft()
        };
        assert_eq!(
            prepare(draft, None).err().unwrap().code,
            ErrorCode::InvalidInput
        );
    }

    fn ssh_draft(private_key: &str) -> ItemDraft {
        ItemDraft {
            ssh_key: Some(SshKeyChange::Set {
                private_key: SecretIn::test(private_key),
            }),
            ..empty_draft()
        }
    }

    #[test]
    fn ssh_key_set_derives_public_key_and_fingerprint() {
        let prepared = prepare(ssh_draft(ED25519), None).unwrap();
        assert!(prepared.sets_ssh_key());
        let mut item = Item::new(&ItemType::SshKey, device(), at()).unwrap();
        prepared
            .apply(&mut item.edit(device(), at()), &[], Vec::new())
            .unwrap();
        let key = item.ssh_key().unwrap();
        assert_eq!(key.public_key.as_str(), ED25519_PUB.trim());
        assert_eq!(
            key.fingerprint.as_str(),
            "SHA256:QjbS4EZK032epnEDVJGO7K3jwMFOWJMwAflXxbJSUUM"
        );
        assert_eq!(key.private_key.expose_secret(), ED25519.trim());
    }

    #[test]
    fn ssh_key_garbage_is_ssh_key_invalid() {
        for text in [
            "",
            "hunter2",
            "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----",
        ] {
            let error = prepare(ssh_draft(text), None).err().unwrap();
            assert_eq!(error.code, ErrorCode::SshKeyInvalid, "{text:?}");
        }
    }

    #[test]
    fn ssh_key_over_16k_is_invalid() {
        let long = format!("{}{}", ED25519.trim(), "A".repeat(16 * 1024));
        let error = prepare(ssh_draft(&long), None).err().unwrap();
        assert_eq!(error.code, ErrorCode::SshKeyInvalid);
    }
}
