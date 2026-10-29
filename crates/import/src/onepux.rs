//! 1Password 1PUX export (1Password 8: "File → Export → 1PUX").
//!
//! A 1PUX file is a zip archive with `export.data` (JSON) and the
//! attachment files under `files/` (named `files/<documentId>__<fileName>`).
//!
//! ```json
//! { "accounts": [{ "vaults": [{ "attrs": { "name": "Private" },
//!   "items": [{
//!     "categoryUuid": "001", "favIndex": 1, "state": "active",
//!     "overview": { "title": "…", "url": "…", "urls": [{ "url": "…" }],
//!                   "tags": ["…"] },
//!     "details": {
//!       "loginFields": [{ "designation": "username", "value": "…",
//!                         "fieldType": "T", "name": "…" }],
//!       "notesPlain": "…", "password": "…",
//!       "passwordHistory": [{ "value": "…", "time": 1700000000 }],
//!       "sections": [{ "title": "…", "fields": [{ "title": "…", "id": "…",
//!                      "value": { "concealed": "…" } }] }],
//!       "documentAttributes": { "fileName": "…", "documentId": "…" } } }] }] }] }
//! ```
//!
//! Categories: 001 login, 005 password, 110 server, 111 email account and
//! 102 database become logins; 002 card, 003 secure note, 004 identity,
//! 106 passport, 103 driver license and 108 social security number become
//! identities or cards as named; 006 document, 109 wireless router (Wi-Fi),
//! 112 API credential and 114 SSH key keep their type; every other category
//! becomes a secure note. Section fields are mapped by their value kind and
//! ID (`ccnum`, `cvv`, `expiry`, `firstname`, `network_name`, …); the rest
//! become custom fields. With more than one vault in the export, each
//! item gets its vault's name as a tag.

use std::io::{Cursor, Read};

use mosaic_core::model::{ItemType, SecretField, TextField};
use zip::ZipArchive;
use zip::result::ZipError;

use crate::Format;
use crate::buf::{self, Buf};
use crate::draft::{AttachmentBudget, Builder, EntryWarning, Kind};
use crate::enpass::expiry;
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};

/// Largest `export.data`: 64 MiB.
pub const MAX_EXPORT_DATA_LEN: usize = 64 * 1024 * 1024;
/// Most members of the archive.
pub const MAX_ARCHIVE_MEMBERS: usize = 100_000;

/// Reads a 1PUX export.
///
/// # Errors
/// [`Error::Malformed`] if the file is not a zip archive with a JSON
/// `export.data`; [`Error::Encrypted`] for encrypted archive members;
/// [`Error::Unsupported`] for a compression other than stored or deflate;
/// [`Error::TooLarge`], [`Error::LimitExceeded`].
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    // Bound the central directory before the zip crate reads it: every
    // member needs one central directory header (`PK\x01\x02`).
    let headers = input.windows(4).filter(|w| *w == b"PK\x01\x02").count();
    if headers > MAX_ARCHIVE_MEMBERS {
        return Err(Error::LimitExceeded);
    }
    let mut zip = ZipArchive::new(Cursor::new(input)).map_err(|_| Error::Malformed)?;
    let data = member(&mut zip, "export.data", MAX_EXPORT_DATA_LEN)?.ok_or(Error::Malformed)?;
    let doc = json::parse(&data)?;
    drop(data);
    let Some(Json::Array(accounts)) = doc.get("accounts") else {
        return Err(Error::Malformed);
    };
    let vaults: Vec<&Json> = accounts.iter().flat_map(|a| a.array("vaults")).collect();
    let tag_vaults = vaults.len() > 1;
    let mut files = Files {
        zip,
        budget: AttachmentBudget::new(),
    };
    let mut c = Collector::new(Format::OnePassword);
    for vault in vaults {
        let vault_name = vault.get("attrs").and_then(|a| a.str("name"));
        for item in vault.array("items") {
            if !item.is_object() {
                c.skip(SkipReason::Malformed)?;
                continue;
            }
            let mut b = entry(item, &mut files)?;
            if tag_vaults && let Some(name) = vault_name {
                b.tag(name);
            }
            c.push(b)?;
        }
    }
    Ok(c.finish())
}

/// Reads one archive member by exact name into a zeroizing buffer.
fn member(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str, limit: usize) -> Result<Option<Buf>> {
    let Some(index) = zip.index_for_name(name) else {
        return Ok(None);
    };
    read_member(zip, index, limit).map(Some)
}

fn read_member(zip: &mut ZipArchive<Cursor<&[u8]>>, index: usize, limit: usize) -> Result<Buf> {
    let file = zip.by_index(index).map_err(|e| match e {
        ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED) => Error::Encrypted,
        ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
            Error::Unsupported
        }
        _ => Error::Malformed,
    })?;
    let hint = usize::try_from(file.size()).unwrap_or(limit);
    // Read one byte past the limit so an oversized member is detected
    // whatever size its header declares.
    let reader = file.take(u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1));
    buf::read_limited(reader, limit, hint.min(limit))
}

/// The attachment files of the archive, looked up by document ID.
struct Files<'a> {
    zip: ZipArchive<Cursor<&'a [u8]>>,
    budget: AttachmentBudget,
}

impl Files<'_> {
    /// The member `files/<documentId>` or `files/<documentId>__<name>`.
    fn find(&self, document_id: &str) -> Option<usize> {
        if document_id.is_empty() {
            return None;
        }
        self.zip.file_names_raw().enumerate().find_map(|(i, raw)| {
            let rest = raw
                .strip_prefix(b"files/")?
                .strip_prefix(document_id.as_bytes())?;
            (rest.is_empty() || rest.starts_with(b"__")).then_some(i)
        })
    }

    fn read(&mut self, index: usize) -> Result<Buf> {
        read_member(&mut self.zip, index, mosaic_core::MAX_ATTACHMENT_LEN)
    }
}

fn category(uuid: &str) -> (ItemType, bool) {
    match uuid {
        "001" | "005" | "102" | "110" | "111" => (ItemType::Login, true),
        "002" => (ItemType::CreditCard, true),
        "004" | "103" | "106" | "108" => (ItemType::Identity, true),
        "006" => (ItemType::Document, true),
        "109" => (ItemType::Wifi, true),
        "112" => (ItemType::ApiKey, true),
        "114" => (ItemType::SshKey, true),
        "003" | "100" | "101" | "104" | "105" | "107" | "113" | "115" => {
            (ItemType::SecureNote, true)
        }
        _ => (ItemType::SecureNote, false),
    }
}

fn entry(item: &Json, files: &mut Files<'_>) -> Result<Builder> {
    let cat = item.str("categoryUuid").unwrap_or_default();
    let (ty, known) = category(cat);
    let mut b = Builder::new(ty);
    if !known {
        b.warn(EntryWarning::UnknownCategory);
    }
    let empty = Json::Null;
    let overview = item.get("overview").unwrap_or(&empty);
    let details = item.get("details").unwrap_or(&empty);
    b.title(overview.str("title").unwrap_or_default());
    for f in details.array("loginFields") {
        login_field(&mut b, f);
    }
    b.secret(
        SecretField::Password,
        details.str("password").unwrap_or_default(),
        "Password",
    );
    b.url(overview.str("url").unwrap_or_default(), "Website");
    for u in overview.array("urls") {
        b.url(
            u.str("url").unwrap_or_default(),
            u.str("label").unwrap_or("Website"),
        );
    }
    for section in details.array("sections") {
        for f in section.array("fields") {
            section_field(&mut b, cat, f, files)?;
        }
    }
    b.notes(details.str("notesPlain").unwrap_or_default());
    let mut history: Vec<(i64, &str)> = details
        .array("passwordHistory")
        .iter()
        .filter_map(|h| {
            Some((
                h.get("time").and_then(Json::as_i64).unwrap_or(0),
                h.str("value")?,
            ))
        })
        .collect();
    history.sort_by_key(|(t, _)| *t);
    for (_, old) in history {
        b.history(old);
    }
    if let Some(doc) = details.get("documentAttributes") {
        attachment(&mut b, doc, files)?;
    }
    for t in overview.array("tags").iter().filter_map(Json::as_str) {
        b.tag(t);
    }
    b.favorite(item.get("favIndex").and_then(Json::as_i64).unwrap_or(0) > 0);
    match item.str("state") {
        Some("archived") => b.archived(),
        Some("deleted" | "trashed") => b.trashed(true),
        _ => {}
    }
    if details.get("passkey").is_some() || overview.get("passkey").is_some() {
        b.warn(EntryWarning::PasskeysNotImported);
    }
    Ok(b)
}

fn login_field(b: &mut Builder, f: &Json) {
    let value = f.str("value").unwrap_or_default();
    let name = f.str("name").unwrap_or_default();
    match (
        f.str("designation").unwrap_or_default(),
        f.str("fieldType").unwrap_or_default(),
    ) {
        ("username", _) => b.text(TextField::Username, value, "Username"),
        ("password", _) => b.secret(SecretField::Password, value, "Password"),
        // Checkboxes, radio buttons, buttons and menus are form state.
        (_, "C" | "R" | "B" | "I" | "S") => {}
        (_, "P") => b.fallback(name, Kind::Hidden, value),
        (_, "E") => b.fallback(name, Kind::Email, value),
        (_, "U") => b.fallback(name, Kind::Url, value),
        _ => b.fallback(name, Kind::Text, value),
    }
}

/// `YYYY-MM-DD` of a Unix time in seconds (UTC).
fn date(unix: i64) -> String {
    // Days to civil date, H. Hinnant's algorithm.
    let z = unix.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[expect(
    clippy::too_many_lines,
    reason = "one table of 1Password's field kinds and IDs"
)]
fn section_field(b: &mut Builder, cat: &str, f: &Json, files: &mut Files<'_>) -> Result<()> {
    let label = f.str("title").unwrap_or_default();
    let id = f.str("id").unwrap_or_default();
    let Some((kind, v)) = f.get("value").and_then(|v| v.members().first()) else {
        return Ok(());
    };
    let ty = b.item_type().clone();
    match kind.as_str() {
        "totp" => b.otp_text(v.as_str().unwrap_or_default(), label),
        "concealed" => {
            let s = v.as_str().unwrap_or_default();
            match (id, &ty) {
                ("password", ItemType::Login) => b.secret(SecretField::Password, s, label),
                ("wireless_password" | "password", ItemType::Wifi) => {
                    b.secret(SecretField::WifiPassword, s, label);
                }
                ("credential", ItemType::ApiKey) => b.secret(SecretField::ApiKeySecret, s, label),
                ("cvv", ItemType::CreditCard) => {
                    b.secret(SecretField::CardVerificationCode, s, label);
                }
                ("pin", ItemType::CreditCard) => b.secret(SecretField::CardPin, s, label),
                _ => b.fallback(label, Kind::Hidden, s),
            }
        }
        "creditCardNumber" if ty == ItemType::CreditCard => {
            b.secret(
                SecretField::CardNumber,
                v.as_str().unwrap_or_default(),
                label,
            );
        }
        "creditCardType" if ty == ItemType::CreditCard => {
            b.text(TextField::CardBrand, v.as_str().unwrap_or_default(), label);
        }
        "monthYear" => {
            let my = v.as_i64().unwrap_or(0);
            let text = format!("{:02}/{:04}", my % 100, my / 100);
            if id == "expiry" && ty == ItemType::CreditCard {
                expiry(b, &text, label);
            } else {
                b.fallback(label, Kind::Text, &text);
            }
        }
        "date" => {
            let d = date(v.as_i64().unwrap_or(0));
            if id == "birthdate" && ty == ItemType::Identity {
                b.text(TextField::BirthDate, &d, label);
            } else {
                b.fallback(label, Kind::Date, &d);
            }
        }
        "email" => {
            let e = v
                .str("email_address")
                .or_else(|| v.as_str())
                .unwrap_or_default();
            if ty == ItemType::Identity {
                b.text(TextField::Email, e, label);
            } else {
                b.fallback(label, Kind::Email, e);
            }
        }
        "phone" => {
            let p = v.as_str().unwrap_or_default();
            if ty == ItemType::Identity {
                b.text(TextField::Phone, p, label);
            } else {
                b.fallback(label, Kind::Phone, p);
            }
        }
        "url" => b.fallback(label, Kind::Url, v.as_str().unwrap_or_default()),
        "address" => {
            if ty == ItemType::Identity {
                for (key, field) in [
                    ("street", TextField::AddressLine1),
                    ("city", TextField::City),
                    ("state", TextField::Region),
                    ("zip", TextField::PostalCode),
                    ("country", TextField::Country),
                ] {
                    b.text(field, v.str(key).unwrap_or_default(), key);
                }
            } else {
                let parts: Vec<&str> = ["street", "zip", "city", "state", "country"]
                    .iter()
                    .filter_map(|k| v.str(k))
                    .filter(|s| !s.trim().is_empty())
                    .collect();
                b.fallback(label, Kind::Text, &parts.join(", "));
            }
        }
        "sshKey" => {
            let meta = v.get("metadata");
            let private = v.str("privateKey").unwrap_or_default();
            let public = meta.and_then(|m| m.str("publicKey")).unwrap_or_default();
            let fp = meta.and_then(|m| m.str("fingerprint")).unwrap_or_default();
            if ty == ItemType::SshKey {
                b.ssh_key(private, public, fp);
            } else {
                b.fallback(label, Kind::Hidden, private);
            }
        }
        "file" => attachment(b, v, files)?,
        "reference" => {}
        _ => {
            let s = v.as_str().map_or_else(
                || v.as_i64().map(|i| i.to_string()).unwrap_or_default(),
                str::to_owned,
            );
            string_field(b, cat, &ty, id, label, &s);
        }
    }
    Ok(())
}

/// A plain string section field, by ID.
fn string_field(b: &mut Builder, cat: &str, ty: &ItemType, id: &str, label: &str, s: &str) {
    match (id, ty) {
        ("username", ItemType::Login | ItemType::ApiKey) => {
            let field = if *ty == ItemType::ApiKey {
                TextField::ApiKeyId
            } else {
                TextField::Username
            };
            b.text(field, s, label);
        }
        ("hostname" | "server" | "url", ItemType::Login) => b.url(s, label),
        ("hostname", ItemType::ApiKey) => b.text(TextField::ApiKeyHost, s, label),
        ("cardholder", ItemType::CreditCard) => b.text(TextField::CardholderName, s, label),
        ("network_name", ItemType::Wifi) => b.text(TextField::WifiSsid, s, label),
        ("wireless_security", ItemType::Wifi) => b.text(TextField::WifiSecurity, s, label),
        ("firstname" | "fullname" | "name", ItemType::Identity) => {
            b.text(TextField::FirstName, s, label);
        }
        ("initial", ItemType::Identity) => b.text(TextField::MiddleName, s, label),
        ("lastname", ItemType::Identity) => b.text(TextField::LastName, s, label),
        ("company", ItemType::Identity) => b.text(TextField::Company, s, label),
        ("number", ItemType::Identity) => {
            let field = match cat {
                "106" => SecretField::PassportNumber,
                "103" => SecretField::LicenseNumber,
                _ => SecretField::NationalId,
            };
            b.secret(field, s, label);
        }
        _ if *ty == ItemType::SecureNote => b.custom(label, Kind::Text, s),
        _ => b.fallback(label, Kind::Text, s),
    }
}

fn attachment(b: &mut Builder, doc: &Json, files: &mut Files<'_>) -> Result<()> {
    let name = doc.str("fileName").unwrap_or_default();
    let declared = doc
        .get("decryptedSize")
        .and_then(Json::as_i64)
        .and_then(|s| usize::try_from(s).ok());
    if declared.is_some_and(|s| s > mosaic_core::MAX_ATTACHMENT_LEN) {
        b.warn(EntryWarning::AttachmentTooLarge);
        return Ok(());
    }
    let Some(index) = files.find(doc.str("documentId").unwrap_or_default()) else {
        b.warn(EntryWarning::AttachmentMissing);
        return Ok(());
    };
    // Spent budget stops further decompression, so many entries that point
    // to one large member cannot make the import inflate it again and again.
    if !files.budget.allows(declared.unwrap_or(1).max(1)) {
        b.warn(EntryWarning::AttachmentQuota);
        return Ok(());
    }
    match files.read(index) {
        Ok(data) => b.attachment(&mut files.budget, name, None, &data),
        Err(Error::TooLarge) => b.warn(EntryWarning::AttachmentTooLarge),
        Err(Error::Allocation) => return Err(Error::Allocation),
        Err(_) => b.warn(EntryWarning::AttachmentMissing),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(date(1_700_000_000), "2023-11-14");
        assert_eq!(date(-86_400), "1969-12-31");
    }

    #[test]
    fn categories() {
        assert_eq!(category("001"), (ItemType::Login, true));
        assert_eq!(category("109"), (ItemType::Wifi, true));
        assert_eq!(category("999"), (ItemType::SecureNote, false));
    }
}
