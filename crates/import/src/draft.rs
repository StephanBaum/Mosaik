//! What an importer makes of one entry: an [`ItemDraft`] that becomes a
//! `mosaic-core` [`Item`] through [`ItemDraft::apply`], plus the attachment
//! files that go next to it.

use core::fmt;

use mosaic_core::model::{
    CustomField, CustomFieldValue, Item, ItemEdit, ItemType, LoginUrl, MAX_HISTORY, MAX_TAG_LEN,
    OtpConfig, SecretField, SshKeyMaterial, Text, TextField,
};
use mosaic_core::{DeviceId, MAX_ATTACHMENT_LEN, SecretString, SecretVec, Timestamp};

use crate::buf::Str;
use crate::otp;

/// Most fields (of any kind) kept per entry; the rest are dropped with
/// [`EntryWarning::FieldsDropped`].
pub const MAX_FIELDS_PER_ENTRY: usize = 1_000;
/// Most attachments kept per entry.
pub const MAX_ATTACHMENTS_PER_ENTRY: usize = 100;
/// Most attachment bytes one import keeps in memory: the account's
/// attachment quota (1 GiB). An export can reference one stored file from
/// many entries (KDBX binaries, 1PUX documents), so this bounds the copies,
/// not the file. Attachments beyond it are left out with
/// [`EntryWarning::AttachmentQuota`].
pub const MAX_IMPORT_ATTACHMENT_BYTES: u64 = mosaic_vault::session::MAX_ACCOUNT_ATTACHMENT_BYTES;

/// What is left of [`MAX_IMPORT_ATTACHMENT_BYTES`] while one file is read.
pub(crate) struct AttachmentBudget {
    left: u64,
}

impl AttachmentBudget {
    pub(crate) const fn new() -> Self {
        Self {
            left: MAX_IMPORT_ATTACHMENT_BYTES,
        }
    }

    /// Whether `len` more bytes fit.
    pub(crate) fn allows(&self, len: usize) -> bool {
        u64::try_from(len).is_ok_and(|n| n <= self.left)
    }

    fn take(&mut self, len: usize) -> bool {
        match u64::try_from(len)
            .ok()
            .and_then(|n| self.left.checked_sub(n))
        {
            Some(left) => {
                self.left = left;
                true
            }
            None => false,
        }
    }
}

/// Something about one entry the user should know. Carries no value from
/// the file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EntryWarning {
    /// Fields without a matching Mosaic field were kept as custom fields.
    UnmappedFields,
    /// A one-time-password value was not understood; it was kept as a
    /// hidden custom field.
    OtpUnreadable,
    /// An attachment is larger than 50 MiB and was left out.
    AttachmentTooLarge,
    /// The export references an attachment whose data it does not contain.
    AttachmentMissing,
    /// Tags that are empty or too long after cleanup were left out.
    TagsDropped,
    /// The password history was longer than Mosaic keeps; the oldest
    /// entries were left out.
    HistoryTruncated,
    /// Passkeys cannot be imported from this format (M5); they were left
    /// out.
    PasskeysNotImported,
    /// The entry's category is unknown; it was imported as a secure note.
    UnknownCategory,
    /// The entry had more fields or attachments than the import keeps.
    FieldsDropped,
    /// The entry was archived in the source; it got the tag `archived`.
    Archived,
    /// The import's attachments exceed [`MAX_IMPORT_ATTACHMENT_BYTES`];
    /// this entry's attachment was left out.
    AttachmentQuota,
}

/// An attachment file of an entry.
pub struct Attachment {
    /// The original file name.
    pub file_name: Text,
    /// The MIME type (guessed from the file name if the export has none).
    pub mime: Text,
    /// The file contents.
    pub data: SecretVec,
}

impl fmt::Debug for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attachment")
            .field("size", &self.data.len())
            .finish_non_exhaustive()
    }
}

/// One imported entry, ready to become a vault item.
///
/// Every string is zeroized on drop; `Debug` prints only the type and
/// counts. Build the item with [`ItemDraft::apply`] inside
/// `UnlockedSession::add_item` (or [`ItemDraft::to_item`]), then add the
/// [`ItemDraft::attachments`].
#[non_exhaustive]
pub struct ItemDraft {
    /// The item type.
    pub item_type: ItemType,
    /// Text fields (title and notes included), each at most once.
    pub texts: Vec<(TextField, Text)>,
    /// Secret fields, each at most once.
    pub secrets: Vec<(SecretField, SecretString)>,
    /// Login URLs, in file order.
    pub urls: Vec<Text>,
    /// The OTP configuration and its HOTP counter.
    pub otp: Option<(OtpConfig, u64)>,
    /// Custom fields, in file order.
    pub custom_fields: Vec<CustomField>,
    /// Tags (folders, groups and tags of the source).
    pub tags: Vec<Text>,
    /// Marked as a favorite in the source.
    pub favorite: bool,
    /// In the source's trash; imported into the trash.
    pub trashed: bool,
    /// Earlier passwords, oldest first (at most [`MAX_HISTORY`]).
    pub password_history: Vec<SecretString>,
    /// SSH key material.
    pub ssh_key: Option<SshKeyMaterial>,
    /// Attachment files, added after the item.
    pub attachments: Vec<Attachment>,
}

impl fmt::Debug for ItemDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemDraft")
            .field("item_type", &self.item_type)
            .field("texts", &self.texts.len())
            .field("secrets", &self.secrets.len())
            .field("urls", &self.urls.len())
            .field("custom_fields", &self.custom_fields.len())
            .field("attachments", &self.attachments.len())
            .finish_non_exhaustive()
    }
}

impl ItemDraft {
    fn new(item_type: ItemType) -> Self {
        Self {
            item_type,
            texts: Vec::new(),
            secrets: Vec::new(),
            urls: Vec::new(),
            otp: None,
            custom_fields: Vec::new(),
            tags: Vec::new(),
            favorite: false,
            trashed: false,
            password_history: Vec::new(),
            ssh_key: None,
            attachments: Vec::new(),
        }
    }

    /// The value of a text field.
    #[must_use]
    pub fn text(&self, field: TextField) -> Option<&str> {
        self.texts
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, v)| v.as_str())
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.text(TextField::Title)
    }

    /// The value of a secret field.
    #[must_use]
    pub fn secret(&self, field: SecretField) -> Option<&SecretString> {
        self.secrets
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, v)| v)
    }

    /// A user-facing second line: user name, email, cardholder, network
    /// name or key ID. Never derived from a secret.
    #[must_use]
    pub fn subtitle(&self) -> Option<&str> {
        [
            TextField::Username,
            TextField::Email,
            TextField::CardholderName,
            TextField::WifiSsid,
            TextField::ApiKeyId,
        ]
        .into_iter()
        .find_map(|f| self.text(f))
    }

    /// Total attachment size in bytes.
    #[must_use]
    pub fn attachment_bytes(&self) -> u64 {
        self.attachments
            .iter()
            .map(|a| u64::try_from(a.data.len()).unwrap_or(u64::MAX))
            .fold(0, u64::saturating_add)
    }

    /// Writes the draft into an item edit (one sync event): text and secret
    /// fields, the password history (replayed oldest first, so each old
    /// password lands in the item's history), URLs, OTP, custom fields,
    /// tags, favorite, trash and SSH key. Attachments are not part of the
    /// edit.
    ///
    /// # Errors
    /// The edit's errors ([`mosaic_core::Error::Malformed`] for an
    /// invalid OTP configuration or tag, [`mosaic_core::Error::ClockOverflow`]).
    pub fn apply(&self, e: &mut ItemEdit<'_>) -> mosaic_core::Result<()> {
        for (field, value) in &self.texts {
            e.set_text(*field, value.as_str())?;
        }
        if !self.password_history.is_empty() {
            for old in &self.password_history {
                e.set_secret(SecretField::Password, old.expose_secret())?;
            }
            if self.secret(SecretField::Password).is_none() {
                e.clear_secret(SecretField::Password)?;
            }
        }
        for (field, value) in &self.secrets {
            e.set_secret(*field, value.expose_secret())?;
        }
        for url in &self.urls {
            e.add_url(&LoginUrl::new(url.as_str()))?;
        }
        if let Some((config, counter)) = &self.otp {
            e.set_otp(config)?;
            if *counter > 0 {
                e.set_otp_counter(*counter)?;
            }
        }
        for field in &self.custom_fields {
            e.add_custom_field(field)?;
        }
        for tag in &self.tags {
            e.add_tag(tag.as_str())?;
        }
        if self.favorite {
            e.set_favorite(true)?;
        }
        if let Some(key) = &self.ssh_key {
            e.set_ssh_key(key)?;
        }
        if self.trashed {
            e.trash()?;
        }
        Ok(())
    }

    /// The item this draft describes, created by `device` at `at` (without
    /// attachments).
    ///
    /// # Errors
    /// As [`ItemDraft::apply`].
    pub fn to_item(&self, device: DeviceId, at: Timestamp) -> mosaic_core::Result<Item> {
        let mut item = Item::new(&self.item_type, device, at)?;
        self.apply(&mut item.edit(device, at))?;
        Ok(item)
    }

    /// Whether the draft holds nothing at all.
    fn is_empty(&self) -> bool {
        self.texts.is_empty()
            && self.secrets.is_empty()
            && self.urls.is_empty()
            && self.otp.is_none()
            && self.custom_fields.is_empty()
            && self.ssh_key.is_none()
            && self.attachments.is_empty()
            && self.password_history.is_empty()
    }
}

/// The kind of a custom field, for [`Builder::custom`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Text,
    Hidden,
    Url,
    Email,
    Phone,
    Date,
}

/// Builds one [`ItemDraft`] from source fields, collecting warnings.
pub(crate) struct Builder {
    draft: ItemDraft,
    warnings: Vec<EntryWarning>,
    fields: usize,
}

impl Builder {
    pub(crate) fn new(item_type: ItemType) -> Self {
        Self {
            draft: ItemDraft::new(item_type),
            warnings: Vec::new(),
            fields: 0,
        }
    }

    pub(crate) fn item_type(&self) -> &ItemType {
        &self.draft.item_type
    }

    pub(crate) fn warn(&mut self, w: EntryWarning) {
        if !self.warnings.contains(&w) {
            self.warnings.push(w);
        }
    }

    /// Counts one field against [`MAX_FIELDS_PER_ENTRY`].
    fn room(&mut self) -> bool {
        if self.fields >= MAX_FIELDS_PER_ENTRY {
            self.warn(EntryWarning::FieldsDropped);
            return false;
        }
        self.fields = self.fields.saturating_add(1);
        true
    }

    pub(crate) fn has_text(&self, field: TextField) -> bool {
        self.draft.text(field).is_some()
    }

    pub(crate) fn has_secret(&self, field: SecretField) -> bool {
        self.draft.secret(field).is_some()
    }

    /// Sets a text field; if it is already set, the value becomes a custom
    /// field named `label`. Blank values are ignored.
    pub(crate) fn text(&mut self, field: TextField, value: &str, label: &str) {
        if value.trim().is_empty() {
            return;
        }
        if self.has_text(field) {
            self.fallback(label, Kind::Text, value);
            return;
        }
        if self.room() {
            self.draft.texts.push((field, Text::new(value)));
        }
    }

    /// Sets the title (a second title becomes a custom field).
    pub(crate) fn title(&mut self, value: &str) {
        self.text(TextField::Title, value.trim(), "Title");
    }

    /// Adds notes; several notes are joined by a blank line.
    pub(crate) fn notes(&mut self, value: &str) {
        if value.trim().is_empty() {
            return;
        }
        let pos = self
            .draft
            .texts
            .iter()
            .position(|(f, _)| *f == TextField::Notes);
        match pos.and_then(|i| self.draft.texts.get_mut(i)) {
            Some((_, existing)) => {
                let mut joined = Str::default();
                joined.push_str(existing.as_str());
                joined.push_str("\n\n");
                joined.push_str(value);
                *existing = Text::new(&joined);
            }
            None => {
                if self.room() {
                    self.draft.texts.push((TextField::Notes, Text::new(value)));
                }
            }
        }
    }

    /// Sets a secret field; if it is already set, the value becomes a
    /// hidden custom field named `label`. Empty values are ignored (secrets
    /// are never trimmed).
    pub(crate) fn secret(&mut self, field: SecretField, value: &str, label: &str) {
        if value.is_empty() {
            return;
        }
        if self.has_secret(field) {
            self.fallback(label, Kind::Hidden, value);
            return;
        }
        if self.room()
            && let Ok(s) = SecretString::from_str_copy(value)
        {
            self.draft.secrets.push((field, s));
        }
    }

    /// Adds a login URL (a URL custom field on other types).
    pub(crate) fn url(&mut self, value: &str, label: &str) {
        let v = value.trim();
        if v.is_empty() {
            return;
        }
        if self.draft.item_type == ItemType::Login {
            if self.draft.urls.iter().any(|u| u.as_str() == v) {
                return;
            }
            if self.room() {
                self.draft.urls.push(Text::new(v));
            }
        } else {
            self.custom(label, Kind::Url, v);
        }
    }

    /// Adds an OTP value: the item's OTP (logins and TOTP items, first one
    /// only) or an OTP custom field. A value that does not parse is kept as
    /// a hidden custom field with [`EntryWarning::OtpUnreadable`].
    pub(crate) fn otp_text(&mut self, value: &str, label: &str) {
        if value.trim().is_empty() {
            return;
        }
        if let Some(parsed) = otp::parse(value) {
            self.otp(parsed, label);
        } else {
            self.warn(EntryWarning::OtpUnreadable);
            self.custom(label, Kind::Hidden, value);
        }
    }

    /// Adds a parsed OTP configuration (see [`Builder::otp_text`]).
    pub(crate) fn otp(&mut self, parsed: otp::Parsed, label: &str) {
        if !self.room() {
            return;
        }
        let item_slot = matches!(self.draft.item_type, ItemType::Login | ItemType::Totp);
        if item_slot && self.draft.otp.is_none() {
            self.draft.otp = Some(parsed);
        } else {
            self.draft.custom_fields.push(CustomField::new(
                Text::new(label_or(label, "One-time password")),
                CustomFieldValue::Otp(parsed.0),
            ));
        }
    }

    /// Adds a custom field the source defined as such. Blank values are
    /// ignored.
    pub(crate) fn custom(&mut self, label: &str, kind: Kind, value: &str) {
        let blank = match kind {
            Kind::Hidden => value.is_empty(),
            _ => value.trim().is_empty(),
        };
        if blank || !self.room() {
            return;
        }
        let label = Text::new(label_or(label, "Field"));
        let value = match kind {
            Kind::Url => CustomFieldValue::Url(Text::new(value.trim())),
            Kind::Email => CustomFieldValue::Email(Text::new(value.trim())),
            Kind::Phone => CustomFieldValue::Phone(Text::new(value.trim())),
            Kind::Date if is_iso_date(value.trim()) => {
                CustomFieldValue::Date(Text::new(value.trim()))
            }
            Kind::Text | Kind::Date => CustomFieldValue::Text(Text::new(value)),
            Kind::Hidden => match SecretString::from_str_copy(value) {
                Ok(s) => CustomFieldValue::Hidden(s),
                Err(_) => return,
            },
        };
        self.draft
            .custom_fields
            .push(CustomField::new(label, value));
    }

    /// Adds a custom field for a source field that has no Mosaic field
    /// ([`EntryWarning::UnmappedFields`]).
    pub(crate) fn fallback(&mut self, label: &str, kind: Kind, value: &str) {
        let before = self.draft.custom_fields.len();
        self.custom(label, kind, value);
        if self.draft.custom_fields.len() > before {
            self.warn(EntryWarning::UnmappedFields);
        }
    }

    /// Adds a tag after cleanup: control characters become spaces, the
    /// result is trimmed and cut to [`MAX_TAG_LEN`] bytes at a character
    /// boundary. Blank tags are dropped ([`EntryWarning::TagsDropped`]).
    pub(crate) fn tag(&mut self, name: &str) {
        let mut clean = Str::default();
        clean.extend(name.chars().map(|c| if c.is_control() { ' ' } else { c }));
        let mut t = clean.trim();
        if t.len() > MAX_TAG_LEN {
            let mut cut = MAX_TAG_LEN;
            while !t.is_char_boundary(cut) {
                cut = cut.saturating_sub(1);
            }
            t = t.get(..cut).unwrap_or_default().trim_end();
        }
        if t.is_empty() {
            self.warn(EntryWarning::TagsDropped);
            return;
        }
        if !self.draft.tags.iter().any(|x| x.as_str() == t) && self.room() {
            self.draft.tags.push(Text::new(t));
        }
    }

    pub(crate) fn favorite(&mut self, favorite: bool) {
        self.draft.favorite |= favorite;
    }

    pub(crate) fn trashed(&mut self, trashed: bool) {
        self.draft.trashed |= trashed;
    }

    /// Marks an archived source entry with the tag `archived`.
    pub(crate) fn archived(&mut self) {
        self.warn(EntryWarning::Archived);
        self.tag("archived");
    }

    /// Adds an old password (call oldest first).
    pub(crate) fn history(&mut self, old: &str) {
        if old.is_empty() {
            return;
        }
        if self.draft.password_history.len() >= MAX_HISTORY {
            self.draft.password_history.remove(0);
            self.warn(EntryWarning::HistoryTruncated);
        }
        if let Ok(s) = SecretString::from_str_copy(old) {
            self.draft.password_history.push(s);
        }
    }

    pub(crate) fn ssh_key(&mut self, private_key: &str, public_key: &str, fingerprint: &str) {
        if private_key.is_empty() || self.draft.ssh_key.is_some() {
            if !private_key.is_empty() {
                self.fallback("Private key", Kind::Hidden, private_key);
            }
            return;
        }
        if let Ok(key) = SecretString::from_str_copy(private_key) {
            self.draft.ssh_key = Some(SshKeyMaterial::new(
                key,
                Text::new(public_key.trim()),
                Text::new(fingerprint.trim()),
            ));
        }
    }

    /// Adds an attachment; one above [`MAX_ATTACHMENT_LEN`] is left out
    /// with [`EntryWarning::AttachmentTooLarge`], one past the import's
    /// `budget` with [`EntryWarning::AttachmentQuota`].
    pub(crate) fn attachment(
        &mut self,
        budget: &mut AttachmentBudget,
        file_name: &str,
        mime: Option<&str>,
        data: &[u8],
    ) {
        if data.len() > MAX_ATTACHMENT_LEN {
            self.warn(EntryWarning::AttachmentTooLarge);
            return;
        }
        if self.draft.attachments.len() >= MAX_ATTACHMENTS_PER_ENTRY {
            self.warn(EntryWarning::FieldsDropped);
            return;
        }
        if !budget.take(data.len()) {
            self.warn(EntryWarning::AttachmentQuota);
            return;
        }
        let Ok(data) = SecretVec::from_slice(data) else {
            return;
        };
        let name = label_or(file_name.trim(), "attachment");
        let mime = mime
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map_or_else(|| guess_mime(name), str::to_owned);
        self.draft.attachments.push(Attachment {
            file_name: Text::new(name),
            mime: Text::new(&mime),
            data,
        });
    }

    /// The draft and its warnings; `None` if the entry holds nothing.
    pub(crate) fn finish(mut self) -> Option<(ItemDraft, Vec<EntryWarning>)> {
        if self.draft.is_empty() {
            return None;
        }
        self.warnings.sort_unstable();
        Some((self.draft, self.warnings))
    }
}

fn label_or<'a>(label: &'a str, default: &'a str) -> &'a str {
    if label.trim().is_empty() {
        default
    } else {
        label
    }
}

/// `YYYY-MM-DD` with a plausible month and day.
pub(crate) fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return false;
    }
    let num = |r: core::ops::Range<usize>| {
        s.get(r)
            .filter(|p| p.bytes().all(|c| c.is_ascii_digit()))
            .and_then(|p| p.parse::<u32>().ok())
    };
    matches!((num(0..4), num(5..7), num(8..10)), (Some(_), Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d))
}

/// A MIME type from a file name's extension.
pub(crate) fn guess_mime(name: &str) -> String {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "txt" | "log" => "text/plain",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "pem" | "key" | "pub" => "application/x-pem-file",
        _ => "application/octet-stream",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use mosaic_core::model::OtpKind;

    use super::*;

    fn device() -> DeviceId {
        DeviceId::from_bytes([1; 16]).unwrap()
    }

    #[test]
    fn builder_maps_and_falls_back() {
        let mut b = Builder::new(ItemType::Login);
        assert_eq!(b.item_type(), &ItemType::Login);
        b.title("  GitHub ");
        b.title("Second title");
        b.text(TextField::Username, "alice", "Username");
        b.text(TextField::Username, "bob", "Alt user");
        b.text(TextField::Email, "   ", "Email");
        b.secret(SecretField::Password, "pw 1 ", "Password");
        b.secret(SecretField::Password, "pw2", "Password 2");
        b.secret(SecretField::Password, "", "Empty");
        b.url("https://github.com/login", "URL");
        b.url("https://github.com/login", "URL");
        b.url(" ", "URL");
        b.notes("first");
        b.notes("second");
        b.notes("  ");
        b.otp_text("GEZDGNBVGY3TQOJQ", "TOTP");
        b.otp_text("GEZDGNBVGY3TQOJQ", "Backup TOTP");
        b.otp_text("not an otp!", "Broken");
        b.otp_text("", "Blank");
        b.custom("", Kind::Text, "plain");
        b.custom("When", Kind::Date, "2024-02-29");
        b.custom("When2", Kind::Date, "29.02.2024");
        b.custom("Mail", Kind::Email, " a@b.c ");
        b.custom("Tel", Kind::Phone, "+1");
        b.custom("Site", Kind::Url, " https://x ");
        b.custom("Blank", Kind::Text, "  ");
        b.custom("Space secret", Kind::Hidden, " ");
        b.tag(" Work\u{7}Stuff ");
        b.tag("Work Stuff");
        b.tag("\u{1}\u{2}");
        b.tag(&"é".repeat(100));
        b.favorite(true);
        b.trashed(false);
        b.archived();
        let (d, w) = b.finish().unwrap();
        assert_eq!(d.title(), Some("GitHub"));
        assert_eq!(d.subtitle(), Some("alice"));
        assert_eq!(
            d.secret(SecretField::Password).unwrap().expose_secret(),
            "pw 1 "
        );
        assert_eq!(d.urls.len(), 1);
        assert_eq!(d.text(TextField::Notes), Some("first\n\nsecond"));
        assert!(d.otp.is_some());
        assert_eq!(d.tags.len(), 3);
        assert_eq!(d.tags[1].as_str().len(), 128);
        assert!(d.favorite && !d.trashed);
        let labels: Vec<&str> = d.custom_fields.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Title",
                "Alt user",
                "Password 2",
                "Backup TOTP",
                "Broken",
                "Field",
                "When",
                "When2",
                "Mail",
                "Tel",
                "Site",
                "Space secret"
            ]
        );
        assert!(matches!(
            d.custom_fields[6].value,
            CustomFieldValue::Date(_)
        ));
        assert!(matches!(
            d.custom_fields[7].value,
            CustomFieldValue::Text(_)
        ));
        assert!(matches!(d.custom_fields[3].value, CustomFieldValue::Otp(_)));
        assert!(matches!(
            d.custom_fields[4].value,
            CustomFieldValue::Hidden(_)
        ));
        assert_eq!(
            w,
            [
                EntryWarning::UnmappedFields,
                EntryWarning::OtpUnreadable,
                EntryWarning::TagsDropped,
                EntryWarning::Archived
            ]
        );
        let dbg = format!("{d:?}");
        assert!(!dbg.contains("alice") && !dbg.contains("pw"));

        let item = d.to_item(device(), Timestamp::from_unix_millis(1)).unwrap();
        assert_eq!(item.title(), Some("GitHub"));
        assert!(item.favorite());
        assert_eq!(item.urls().len(), 1);
        assert_eq!(item.custom_fields().len(), 12);
        assert_eq!(item.tags().len(), 3);
        assert_eq!(item.otp().unwrap().kind, OtpKind::Totp);
    }

    #[test]
    fn urls_on_other_types_and_otp_slot() {
        let mut b = Builder::new(ItemType::SecureNote);
        b.url("https://x", "Website");
        b.otp_text("GEZDGNBVGY3TQOJQ", "");
        let (d, _) = b.finish().unwrap();
        assert!(d.urls.is_empty() && d.otp.is_none());
        assert_eq!(d.custom_fields.len(), 2);
        assert_eq!(d.custom_fields[1].label.as_str(), "One-time password");
    }

    #[test]
    fn history_is_replayed_into_the_item() {
        let mut b = Builder::new(ItemType::Login);
        b.title("t");
        for i in 0..=MAX_HISTORY {
            b.history(&format!("old{i}"));
        }
        b.history("");
        b.secret(SecretField::Password, "current", "Password");
        let (d, w) = b.finish().unwrap();
        assert_eq!(d.password_history.len(), MAX_HISTORY);
        assert_eq!(d.password_history[0].expose_secret(), "old1");
        assert_eq!(w, [EntryWarning::HistoryTruncated]);
        let item = d.to_item(device(), Timestamp::from_unix_millis(1)).unwrap();
        assert_eq!(
            item.secret(SecretField::Password)
                .unwrap()
                .unwrap()
                .expose_secret(),
            "current"
        );
        let h = item.password_history();
        assert_eq!(h.len(), MAX_HISTORY);

        // History without a current password leaves no password set.
        let mut b = Builder::new(ItemType::Login);
        b.history("a");
        b.history("b");
        let (d, _) = b.finish().unwrap();
        let item = d.to_item(device(), Timestamp::from_unix_millis(1)).unwrap();
        assert!(item.secret(SecretField::Password).unwrap().is_none());
        assert_eq!(item.password_history().len(), 2);
    }

    #[test]
    fn ssh_key_attachments_trash_and_empty() {
        let mut budget = AttachmentBudget::new();
        let mut b = Builder::new(ItemType::SshKey);
        b.ssh_key("", "pub", "fp");
        b.ssh_key("-----BEGIN-----", " ssh-ed25519 AAAA ", "SHA256:x");
        b.ssh_key("second", "", "");
        b.attachment(&mut budget, "a.txt", None, b"hello");
        b.attachment(&mut budget, " ", Some(" "), b"x");
        b.attachment(&mut budget, "b.bin", Some("application/x-y"), b"y");
        b.trashed(true);
        let (d, w) = b.finish().unwrap();
        assert_eq!(w, [EntryWarning::UnmappedFields]);
        let key = d.ssh_key.as_ref().unwrap();
        assert_eq!(key.public_key.as_str(), "ssh-ed25519 AAAA");
        assert_eq!(d.attachments.len(), 3);
        assert_eq!(d.attachments[0].mime.as_str(), "text/plain");
        assert_eq!(d.attachments[1].file_name.as_str(), "attachment");
        assert_eq!(d.attachments[1].mime.as_str(), "application/octet-stream");
        assert_eq!(d.attachments[2].mime.as_str(), "application/x-y");
        assert_eq!(d.attachment_bytes(), 7);
        assert!(format!("{:?}", d.attachments[0]).contains("size"));
        let item = d.to_item(device(), Timestamp::from_unix_millis(1)).unwrap();
        assert!(item.trashed_at().is_some());
        assert!(item.ssh_key().is_some());

        assert!(Builder::new(ItemType::Login).finish().is_none());
        let mut b = Builder::new(ItemType::Login);
        b.tag("only a tag");
        assert!(b.finish().is_none());
    }

    #[test]
    fn limits() {
        let mut budget = AttachmentBudget::new();
        let mut b = Builder::new(ItemType::SecureNote);
        for i in 0..=MAX_FIELDS_PER_ENTRY {
            b.custom("f", Kind::Text, &i.to_string());
        }
        b.text(TextField::Title, "late", "Title");
        b.notes("late");
        b.secret(SecretField::Password, "late", "p");
        b.url("x", "u");
        b.otp_text("GEZDGNBVGY3TQOJQ", "o");
        b.tag("late");
        for _ in 0..=MAX_ATTACHMENTS_PER_ENTRY {
            b.attachment(&mut budget, "a", None, b"1");
        }
        let (d, w) = b.finish().unwrap();
        assert_eq!(d.custom_fields.len(), MAX_FIELDS_PER_ENTRY);
        assert_eq!(d.attachments.len(), MAX_ATTACHMENTS_PER_ENTRY);
        assert_eq!(w, [EntryWarning::FieldsDropped]);

        let mut b = Builder::new(ItemType::Document);
        b.attachment(&mut budget, "big", None, &vec![0; MAX_ATTACHMENT_LEN + 1]);
        b.title("x");
        let (d, w) = b.finish().unwrap();
        assert!(d.attachments.is_empty());
        assert_eq!(w, [EntryWarning::AttachmentTooLarge]);

        // The import-wide budget.
        let mut small = AttachmentBudget { left: 3 };
        assert!(small.allows(3) && !small.allows(4));
        let mut b = Builder::new(ItemType::Document);
        b.title("x");
        b.attachment(&mut small, "a", None, b"12");
        b.attachment(&mut small, "b", None, b"34");
        b.attachment(&mut small, "c", None, b"5");
        let (d, w) = b.finish().unwrap();
        assert_eq!(d.attachments.len(), 2);
        assert_eq!(w, [EntryWarning::AttachmentQuota]);
        assert!(!small.allows(1));
    }

    #[test]
    fn dates_and_mime() {
        for (s, ok) in [
            ("2024-01-31", true),
            ("2024-13-01", false),
            ("2024-00-10", false),
            ("2024-01-32", false),
            ("2024/01/01", false),
            ("24-01-01", false),
            ("abcd-01-01", false),
            ("2024-+1-01", false),
        ] {
            assert_eq!(is_iso_date(s), ok, "{s}");
        }
        for (n, m) in [
            ("a.PDF", "application/pdf"),
            ("a.jpeg", "image/jpeg"),
            ("a.png", "image/png"),
            ("a.gif", "image/gif"),
            ("a.webp", "image/webp"),
            ("a.svg", "image/svg+xml"),
            ("a.json", "application/json"),
            ("a.xml", "application/xml"),
            ("a.zip", "application/zip"),
            ("id.pub", "application/x-pem-file"),
            ("a.csv", "text/csv"),
            ("a.log", "text/plain"),
            ("noext", "application/octet-stream"),
        ] {
            assert_eq!(guess_mime(n), m);
        }
    }
}
