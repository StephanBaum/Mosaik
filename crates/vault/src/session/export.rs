//! Writing an encrypted export (CRYPTO_SPEC §11.2, ADR-0073).
//!
//! [`UnlockedSession::write_export`] streams the items of the selected
//! vaults through `mosaic_core::ExportWriter` one item at a time as the JSON
//! of [`crate::export`]: the whole export is never held in memory, and an
//! attachment is encoded and written in 64 KiB pieces. Items in the trash
//! and deleted items are left out. The file holds no item, vault, device or
//! account ID.
//!
//! Attachments go in only when [`ExportSelection::attachments`] is set
//! (OD-M8-7); otherwise each attachment is listed by name, type and size
//! with `data: null`. [`UnlockedSession::export_preview`] counts what a
//! selection would write, so a client can warn about attachments first.

use std::io::{self, Write};

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::{CustomFieldValue, Item, SecretField, TextField};
use mosaic_core::{Argon2Profile, ExportPassphrase, ExportWriter, ItemId, Timestamp, VaultId};
use serde::Serialize;
use zeroize::Zeroizing;

use super::{Error, Result, UnlockedSession, now};
use crate::export::{
    CUSTOM_PREFIX, ExportField, ExportHistory, ExportItem, ExportPasskey, FORMAT, SecretText,
    VERSION,
};
use crate::otp::otp_auth_from_config;

/// The COSE identifier of ES256, the only passkey algorithm CXF carries.
const COSE_ES256: i64 = -7;
/// Raw attachment bytes encoded per piece (a multiple of 3, so every piece
/// but the last encodes without padding).
const RAW_STEP: usize = 48 * 1024;

/// What to export.
#[derive(Clone, Debug)]
pub struct ExportSelection {
    /// The vaults to export; `None` is every vault this session can open.
    /// A vault the session cannot open (a hardware-protected one) is
    /// skipped and counted when this is `None`, and an error when it is
    /// named.
    pub vaults: Option<Vec<VaultId>>,
    /// Whether attachment contents go into the file.
    pub attachments: bool,
    /// The Argon2id profile that protects the file (at least standard,
    /// §11.2).
    pub profile: Argon2Profile,
}

impl Default for ExportSelection {
    /// Every openable vault, no attachment data, the standard profile.
    fn default() -> Self {
        Self {
            vaults: None,
            attachments: false,
            profile: Argon2Profile::Standard,
        }
    }
}

/// What a selection would export ([`UnlockedSession::export_preview`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportPreview {
    /// Vaults that would be written.
    pub vaults: u32,
    /// Items that would be written.
    pub items: u32,
    /// Items that carry attachments, so a client can name them in a warning.
    pub items_with_attachments: Vec<(VaultId, ItemId)>,
    /// Attachment files on those items.
    pub attachments: u32,
    /// Their total size in bytes.
    pub attachment_bytes: u64,
    /// Vaults left out because this session cannot open them.
    pub unavailable_vaults: u32,
}

/// What [`UnlockedSession::write_export`] wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// Vaults written.
    pub vaults: u32,
    /// Items written.
    pub items: u32,
    /// Attachment files whose contents were written.
    pub attachments: u32,
    /// Attachment files listed without contents: the selection did not ask
    /// for them, or the file is not in this store (not downloaded yet) or
    /// could not be read.
    pub attachments_left_out: u32,
    /// Passkeys of an algorithm CXF cannot carry (not ES256): the item is
    /// written without its passkey.
    pub unsupported_passkeys: u32,
    /// Vaults left out because this session cannot open them.
    pub unavailable_vaults: u32,
}

fn malformed() -> Error {
    Error::Core(mosaic_core::Error::Malformed)
}

/// Counts the bytes a serialization would take.
struct Counter(usize);

impl Write for Counter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The compact JSON of `value` in a zeroizing buffer of exactly its size
/// (a counting pass first), so no unwiped copy is left behind by a growing
/// vector.
fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<Zeroizing<Vec<u8>>> {
    let mut count = Counter(0);
    serde_json::to_writer(&mut count, value).map_err(|_| malformed())?;
    let mut buf = Vec::new();
    buf.try_reserve_exact(count.0)
        .map_err(|_| Error::Core(mosaic_core::Error::Allocation))?;
    let mut buf = Zeroizing::new(buf);
    serde_json::to_writer(&mut *buf, value).map_err(|_| malformed())?;
    Ok(buf)
}

/// Unpadded base64url of `bytes` as a zeroizing text.
fn b64_secret(bytes: &[u8]) -> SecretText {
    SecretText::from_zeroizing(Zeroizing::new(Base64UrlUnpadded::encode_string(bytes)))
}

/// The writer with its error mapped.
struct Out<'a> {
    w: ExportWriter<&'a mut dyn Write>,
}

impl Out<'_> {
    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        Ok(self.w.write_chunk(bytes)?)
    }
}

/// The built-in and custom fields of an item.
fn fields_of(item: &Item) -> Result<Vec<ExportField>> {
    let mut fields = Vec::new();
    for field in TextField::ALL {
        if matches!(field, TextField::Title | TextField::Notes) {
            continue;
        }
        if let Some(value) = item.text(*field)
            && !value.is_empty()
        {
            fields.push(ExportField {
                key: field.key().to_owned(),
                label: SecretText::new(field.key()),
                value: SecretText::new(value),
                secret: false,
            });
        }
    }
    for field in SecretField::ALL {
        if let Some(value) = item.secret(*field)? {
            fields.push(ExportField {
                key: field.key().to_owned(),
                label: SecretText::new(field.key()),
                value: SecretText::new(value.expose_secret()),
                secret: true,
            });
        }
    }
    for (_, custom) in item.custom_fields() {
        let (kind, secret, value) = match &custom.value {
            CustomFieldValue::Text(t) => ("text", false, SecretText::new(t.as_str())),
            CustomFieldValue::Hidden(s) => ("hidden", true, SecretText::new(s.expose_secret())),
            CustomFieldValue::Url(t) => ("url", false, SecretText::new(t.as_str())),
            CustomFieldValue::Email(t) => ("email", false, SecretText::new(t.as_str())),
            CustomFieldValue::Phone(t) => ("phone", false, SecretText::new(t.as_str())),
            CustomFieldValue::Date(t) => ("date", false, SecretText::new(t.as_str())),
            CustomFieldValue::Otp(config) => {
                let uri = otp_auth_from_config(config, 0)?.to_uri()?;
                ("otp", true, SecretText::new(uri.expose_secret()))
            }
        };
        fields.push(ExportField {
            key: format!("{CUSTOM_PREFIX}{kind}"),
            label: SecretText::new(custom.label.as_str()),
            value,
            secret,
        });
    }
    Ok(fields)
}

/// The passkey as a CXF credential: ES256 only. Another algorithm is
/// counted in `report` and leaves the member out.
fn passkey_of(item: &Item, report: &mut ExportReport) -> Option<ExportPasskey> {
    let c = item.passkey()?;
    if c.algorithm != COSE_ES256 {
        report.unsupported_passkeys = report.unsupported_passkeys.saturating_add(1);
        return None;
    }
    Some(ExportPasskey {
        kind: "passkey".to_owned(),
        credential_id: b64_secret(&c.credential_id),
        rp_id: SecretText::new(c.rp_id.as_str()),
        username: SecretText::new(c.user_name.as_str()),
        user_display_name: SecretText::new(c.user_display_name.as_str()),
        user_handle: b64_secret(&c.user_handle),
        key: b64_secret(c.private_key.expose_secret()),
        sign_count: item.passkey_sign_count(),
        discoverable: c.discoverable,
        created_at: Some(c.created_at.as_unix_millis()),
    })
}

/// The login password's earlier values, oldest first (the only history in
/// the schema).
fn history_of(item: &Item) -> Vec<ExportHistory> {
    let mut history: Vec<ExportHistory> = item
        .password_history()
        .into_iter()
        .filter(|h| h.field == SecretField::Password)
        .map(|h| ExportHistory {
            value: SecretText::new(h.value.expose_secret()),
            changed_at: h.changed_at.as_unix_millis(),
        })
        .collect();
    // Stored newest first; the schema is oldest first.
    history.reverse();
    history
}

/// The item as the schema holds it, without attachments. `None` for an item
/// without a readable type.
fn export_item(item: &Item, report: &mut ExportReport) -> Result<Option<ExportItem>> {
    let Some(item_type) = item.item_type() else {
        return Ok(None);
    };
    let otp = match item.otp() {
        Some(config) => {
            let uri = otp_auth_from_config(&config, item.otp_counter())?.to_uri()?;
            Some(SecretText::new(uri.expose_secret()))
        }
        None => None,
    };
    let (ssh_key, ssh_public_key, ssh_fingerprint) = match item.ssh_key() {
        Some(k) => (
            Some(SecretText::new(k.private_key.expose_secret())),
            Some(SecretText::new(k.public_key.as_str())),
            Some(SecretText::new(k.fingerprint.as_str())),
        ),
        None => (None, None, None),
    };
    Ok(Some(ExportItem {
        item_type: item_type.as_str().to_owned(),
        title: SecretText::new(item.title().unwrap_or_default()),
        favorite: item.favorite(),
        created_at: item.created_at().map_or(0, Timestamp::as_unix_millis),
        tags: item.tags().into_iter().map(SecretText::new).collect(),
        urls: item
            .urls()
            .iter()
            .map(|(_, u)| SecretText::new(u.url.as_str()))
            .collect(),
        fields: fields_of(item)?,
        notes: SecretText::new(item.text(TextField::Notes).unwrap_or_default()),
        otp,
        passkey: passkey_of(item, report),
        ssh_key,
        ssh_public_key,
        ssh_fingerprint,
        history: history_of(item),
        attachments: Vec::new(),
    }))
}

impl UnlockedSession {
    /// The vaults a selection names, and how many were skipped because the
    /// session cannot open them.
    fn export_vaults(&self, sel: &ExportSelection) -> Result<(Vec<VaultId>, u32)> {
        match &sel.vaults {
            None => {
                let mut open = Vec::new();
                let mut unavailable = 0u32;
                for (id, ov) in &self.vaults {
                    if ov.available() {
                        open.push(*id);
                    } else {
                        unavailable = unavailable.saturating_add(1);
                    }
                }
                Ok((open, unavailable))
            }
            Some(named) => {
                let mut open: Vec<VaultId> = Vec::new();
                for id in named {
                    if !self.vault(*id)?.available() {
                        return Err(Error::VaultUnavailable);
                    }
                    if !open.contains(id) {
                        open.push(*id);
                    }
                }
                Ok((open, 0))
            }
        }
    }

    /// The live items of a vault that an export writes, in item ID order.
    fn export_items(&self, vault_id: VaultId) -> Result<Vec<(ItemId, &Item)>> {
        Ok(self
            .vault(vault_id)?
            .items
            .iter()
            .filter(|(_, i)| !i.is_deleted() && i.trashed_at().is_none() && i.item_type().is_some())
            .map(|(id, i)| (*id, i))
            .collect())
    }

    /// Counts what [`UnlockedSession::write_export`] would write for `sel`,
    /// without touching a key or writing anything.
    ///
    /// # Errors
    /// [`Error::UnknownVault`] or [`Error::VaultUnavailable`] for a named
    /// vault.
    pub fn export_preview(&self, sel: &ExportSelection) -> Result<ExportPreview> {
        let (vaults, unavailable) = self.export_vaults(sel)?;
        let mut preview = ExportPreview {
            vaults: u32::try_from(vaults.len()).unwrap_or(u32::MAX),
            unavailable_vaults: unavailable,
            ..ExportPreview::default()
        };
        for vault_id in vaults {
            for (item_id, item) in self.export_items(vault_id)? {
                preview.items = preview.items.saturating_add(1);
                let files = item.attachments();
                if files.is_empty() {
                    continue;
                }
                preview.items_with_attachments.push((vault_id, item_id));
                for f in files {
                    preview.attachments = preview.attachments.saturating_add(1);
                    preview.attachment_bytes = preview.attachment_bytes.saturating_add(f.size);
                }
            }
        }
        Ok(preview)
    }

    /// Writes an encrypted export (§11.2) of `sel` to `w`, protected by
    /// `pass`.
    ///
    /// The output is a stream: write it to a temporary file and move it into
    /// place only after this returns `Ok`, flushing it first. After any
    /// error the output is a truncated stream that no reader accepts; delete
    /// it.
    ///
    /// # Errors
    /// [`Error::UnknownVault`] / [`Error::VaultUnavailable`] for a named
    /// vault; [`Error::Core`] for the writer (a profile below standard, a
    /// failed output, memory); [`Error::Otp`] for an OTP configuration that
    /// has no `otpauth://` form.
    pub fn write_export(
        &self,
        sel: &ExportSelection,
        w: &mut dyn Write,
        pass: &ExportPassphrase,
    ) -> Result<ExportReport> {
        let (vaults, unavailable) = self.export_vaults(sel)?;
        let mut report = ExportReport {
            unavailable_vaults: unavailable,
            ..ExportReport::default()
        };
        let mut out = Out {
            w: ExportWriter::new(w, pass, sel.profile)?,
        };
        out.put(
            format!(
                "{{\"format\":\"{FORMAT}\",\"v\":{VERSION},\"exportedAt\":{},\"vaults\":[",
                now().as_unix_millis()
            )
            .as_bytes(),
        )?;
        for (n, vault_id) in vaults.iter().enumerate() {
            if n > 0 {
                out.put(b",")?;
            }
            self.write_vault(&mut out, *vault_id, sel, &mut report)?;
            report.vaults = report.vaults.saturating_add(1);
        }
        out.put(b"]}")?;
        out.w.finish()?;
        Ok(report)
    }

    fn write_vault(
        &self,
        out: &mut Out<'_>,
        vault_id: VaultId,
        sel: &ExportSelection,
        report: &mut ExportReport,
    ) -> Result<()> {
        let name = self
            .vault(vault_id)?
            .meta
            .as_ref()
            .and_then(|m| m.name())
            .unwrap_or_default();
        out.put(b"{\"name\":")?;
        out.put(&to_json(&SecretText::new(name))?)?;
        out.put(b",\"items\":[")?;
        let mut first = true;
        for (item_id, item) in self.export_items(vault_id)? {
            let Some(exported) = export_item(item, report)? else {
                continue;
            };
            if !first {
                out.put(b",")?;
            }
            first = false;
            self.write_item(out, vault_id, item_id, item, &exported, sel, report)?;
            report.items = report.items.saturating_add(1);
        }
        out.put(b"]}")
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one streaming step of the export"
    )]
    fn write_item(
        &self,
        out: &mut Out<'_>,
        vault_id: VaultId,
        item_id: ItemId,
        item: &Item,
        exported: &ExportItem,
        sel: &ExportSelection,
        report: &mut ExportReport,
    ) -> Result<()> {
        let mut json = to_json(exported)?;
        // The item is a compact object: reopen it to append the attachments,
        // which are streamed.
        if json.pop() != Some(b'}') {
            return Err(malformed());
        }
        out.put(&json)?;
        drop(json);
        out.put(b",\"attachments\":[")?;
        for (n, file) in item.attachments().iter().enumerate() {
            if n > 0 {
                out.put(b",")?;
            }
            out.put(b"{\"name\":")?;
            out.put(&to_json(&SecretText::new(file.file_name.as_str()))?)?;
            out.put(b",\"mime\":")?;
            out.put(&to_json(file.mime.as_str())?)?;
            out.put(format!(",\"size\":{},\"data\":", file.size).as_bytes())?;
            let data = if sel.attachments {
                self.attachment(vault_id, item_id, file.attachment_id).ok()
            } else {
                None
            };
            if let Some(data) = data {
                out.put(b"\"")?;
                let mut text = Zeroizing::new(vec![0u8; RAW_STEP / 3 * 4]);
                for part in data.expose_secret().chunks(RAW_STEP) {
                    let n = Base64UrlUnpadded::encode(part, &mut text)
                        .map_err(|_| malformed())?
                        .len();
                    out.put(text.get(..n).ok_or_else(malformed)?)?;
                }
                out.put(b"\"")?;
                report.attachments = report.attachments.saturating_add(1);
            } else {
                out.put(b"null")?;
                report.attachments_left_out = report.attachments_left_out.saturating_add(1);
            }
            out.put(b"}")?;
        }
        out.put(b"]}")
    }
}
