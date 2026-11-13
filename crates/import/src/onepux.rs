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

use std::collections::HashMap;
use std::io::{Cursor, Read};

use mosaic_core::model::{ItemType, SecretField, TextField};
use zip::ZipArchive;
use zip::result::ZipError;

use crate::Format;
use crate::buf::{self, Buf};
use crate::draft::{AttachmentBudget, Builder, EntryWarning, Kind};
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
    let mut files = Files::new(zip);
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
    /// `files/<name>`: the member index of each exact name after `files/`
    /// (the lowest index for a repeated name).
    exact: HashMap<Box<[u8]>, usize>,
    /// `files/<head>__<name>`: for each member name containing `__`, the
    /// text before its FIRST `__` (one key per member, so O(name) memory)
    /// maps to the lowest member index with that head. An ID that itself
    /// contains `__` therefore matches only by exact name; real 1Password
    /// IDs contain none. A lookup is two hash probes whatever the ID.
    prefixed: HashMap<Box<[u8]>, usize>,
    /// Number of lookups made by `find` (linearity test).
    #[cfg(test)]
    probes: core::cell::Cell<usize>,
}

impl<'a> Files<'a> {
    fn new(zip: ZipArchive<Cursor<&'a [u8]>>) -> Self {
        let mut exact: HashMap<Box<[u8]>, usize> = HashMap::new();
        let mut prefixed: HashMap<Box<[u8]>, usize> = HashMap::new();
        for (i, raw) in zip.file_names_raw().enumerate() {
            let Some(rest) = raw.strip_prefix(b"files/") else {
                continue;
            };
            exact.entry(rest.into()).or_insert(i);
            if let Some(head) = rest
                .windows(2)
                .position(|w| w == b"__")
                .and_then(|p| rest.get(..p))
            {
                prefixed.entry(head.into()).or_insert(i);
            }
        }
        Self {
            zip,
            budget: AttachmentBudget::new(),
            exact,
            prefixed,
            #[cfg(test)]
            probes: core::cell::Cell::new(0),
        }
    }

    /// The member `files/<documentId>` or `files/<documentId>__<name>`; the
    /// lowest member index wins.
    fn find(&self, document_id: &str) -> Option<usize> {
        let id = document_id.as_bytes();
        if id.is_empty() {
            return None;
        }
        #[cfg(test)]
        self.probes.set(self.probes.get() + 1);
        match (self.exact.get(id), self.prefixed.get(id)) {
            (Some(a), Some(b)) => Some(*a.min(b)),
            (a, b) => a.or(b).copied(),
        }
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

fn section_field(b: &mut Builder, cat: &str, f: &Json, files: &mut Files<'_>) -> Result<()> {
    let label = f.str("title").unwrap_or_default();
    let id = f.str("id").unwrap_or_default();
    let Some((kind, v)) = f.get("value").and_then(|v| v.members().first()) else {
        return Ok(());
    };
    let ty = b.item_type().clone();
    let text = v.as_str().unwrap_or_default();
    match kind.as_str() {
        "totp" => b.otp_text(text, label),
        "concealed" => concealed(b, &ty, id, label, text),
        "creditCardNumber" if ty == ItemType::CreditCard => {
            b.secret(SecretField::CardNumber, text, label);
        }
        "creditCardType" if ty == ItemType::CreditCard => {
            b.text(TextField::CardBrand, text, label);
        }
        "monthYear" => {
            let my = v.as_i64().unwrap_or(0);
            let text = format!("{:02}/{:04}", my % 100, my / 100);
            if id == "expiry" && ty == ItemType::CreditCard {
                b.expiry(&text, label);
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
            let e = v.str("email_address").unwrap_or(text);
            identity_or(b, &ty, TextField::Email, Kind::Email, label, e);
        }
        "phone" => identity_or(b, &ty, TextField::Phone, Kind::Phone, label, text),
        "url" => b.fallback(label, Kind::Url, text),
        "address" => address(b, &ty, label, v),
        "sshKey" => ssh_key(b, &ty, label, v),
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

/// A concealed section field, by ID and item type.
fn concealed(b: &mut Builder, ty: &ItemType, id: &str, label: &str, s: &str) {
    match (id, ty) {
        ("password", ItemType::Login) => b.secret(SecretField::Password, s, label),
        ("wireless_password" | "password", ItemType::Wifi) => {
            b.secret(SecretField::WifiPassword, s, label);
        }
        ("credential", ItemType::ApiKey) => b.secret(SecretField::ApiKeySecret, s, label),
        ("cvv", ItemType::CreditCard) => b.secret(SecretField::CardVerificationCode, s, label),
        ("pin", ItemType::CreditCard) => b.secret(SecretField::CardPin, s, label),
        _ => b.fallback(label, Kind::Hidden, s),
    }
}

/// A field that has a text field on identities and is a custom field of
/// `kind` on every other item.
fn identity_or(
    b: &mut Builder,
    ty: &ItemType,
    field: TextField,
    kind: Kind,
    label: &str,
    value: &str,
) {
    if *ty == ItemType::Identity {
        b.text(field, value, label);
    } else {
        b.fallback(label, kind, value);
    }
}

/// An address: the parts are identity fields, or one joined custom field on
/// other items.
fn address(b: &mut Builder, ty: &ItemType, label: &str, v: &Json) {
    if *ty == ItemType::Identity {
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

/// An SSH key: the key material of an SSH key item, a hidden custom field
/// (the private key) on other items.
fn ssh_key(b: &mut Builder, ty: &ItemType, label: &str, v: &Json) {
    let meta = v.get("metadata");
    let private = v.str("privateKey").unwrap_or_default();
    let public = meta.and_then(|m| m.str("publicKey")).unwrap_or_default();
    let fp = meta.and_then(|m| m.str("fingerprint")).unwrap_or_default();
    if *ty == ItemType::SshKey {
        b.ssh_key(private, public, fp);
    } else {
        b.fallback(label, Kind::Hidden, private);
    }
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
        _ => b.content(label, Kind::Text, s),
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

    fn archive(names: &[String]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for n in names {
            zip.start_file(n.as_str(), stored).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn find_matches_document_id_and_double_underscore_names() {
        let bytes = archive(&["files/a__b__c".into(), "files/ab".into(), "files/a".into()]);
        let files = Files::new(ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap());
        // Member 0 is keyed by the text before its FIRST `__`: "a".
        assert_eq!(files.find("a"), Some(0));
        // An ID with a `__` of its own matches only an exact name.
        assert_eq!(files.find("a__b"), None);
        assert_eq!(files.find("a__b__c"), Some(0));
        assert_eq!(files.find("ab"), Some(1));
        assert_eq!(files.find(""), None);
        assert_eq!(files.find("zz"), None);
    }

    #[test]
    fn find_keys_a_member_by_the_text_before_its_first_double_underscore() {
        let bytes = archive(&[
            "files/a___x".into(), // 0: key "a" (first `__` at 1)
            "files/___y".into(),  // 1: key ""  (never looked up: IDs are not empty)
            "files/__z".into(),   // 2: key ""; exact match for ID "__z" only
            "files/a_".into(),    // 3: no `__`; exact match for ID "a_" only
        ]);
        let files = Files::new(ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap());
        assert_eq!(files.find("a_"), Some(3));
        assert_eq!(files.find("_"), None);
        assert_eq!(files.find("__z"), Some(2));
        assert_eq!(files.find("a"), Some(0));
        assert_eq!(files.find("b_"), None);
    }

    /// RT-parsers-2 (red team 2026-11-13): the bucket of a document key is
    /// walked to its end whenever the ID does not match a member exactly or
    /// as `<id>__`. An ID with a `__` of its own (`doc__zz`, key `doc`) or
    /// one ending in `_` lands in a bucket it never matches, and a file with
    /// 100,000 `files/doc__<n>` members and 100,000 entries (or `file`
    /// section fields) naming such an ID costs 10^10 probes: the sibling of
    /// the M3 P2 quadratic lookup that the bucket map did not cover.
    #[test]
    fn find_is_bounded_for_ids_that_match_no_member() {
        let names: Vec<String> = (0..20_000).map(|i| format!("files/doc__{i}")).collect();
        let bytes = archive(&names);
        let files = Files::new(ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap());
        assert_eq!(files.find("doc__zz"), None);
        assert!(
            files.probes.get() <= 2,
            "one miss probed {} members",
            files.probes.get()
        );
    }

    #[test]
    fn find_is_linear_in_lookups() {
        let names: Vec<String> = (0..10_000).map(|i| format!("files/m{i}__x.bin")).collect();
        let bytes = archive(&names);
        let files = Files::new(ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap());
        for i in 0..10_000 {
            assert!(files.find(&format!("m{i}")).is_some());
        }
        assert_eq!(files.probes.get(), 10_000);
    }

    /// RT-parsers-verify-1 (red team verify 2026-11-13): `Files::new` keeps
    /// one `prefixed` key per `__` in a member name, each a copy of the name
    /// up to that point. A name of n underscores costs about n^2/2 bytes of
    /// keys: one 64 KiB member name (the zip maximum) is about 2 GiB, and a
    /// 256 MiB file holds thousands of them. The `document_key` it replaced
    /// cut at the first `__` and was linear.
    #[test]
    fn prefix_map_is_linear_in_the_member_name_length() {
        let name = format!("files/{}", "_".repeat(20_000));
        let bytes = archive(std::slice::from_ref(&name));
        let files = Files::new(ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap());
        let key_bytes: usize = files.prefixed.keys().map(|k| k.len()).sum();
        assert!(
            key_bytes <= 64 * name.len(),
            "{key_bytes} bytes of prefix keys for one {}-byte member name",
            name.len()
        );
    }
}
