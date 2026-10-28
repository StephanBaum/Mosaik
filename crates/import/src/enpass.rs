//! Enpass JSON export (Enpass 6: "File → Export → .json").
//!
//! ```json
//! { "folders": [{ "uuid": "…", "title": "Work", "parent_uuid": "" }],
//!   "items": [{
//!     "category": "login", "template_type": "login.default",
//!     "title": "…", "note": "…", "favorite": 1, "trashed": 0, "archived": 0,
//!     "folders": ["…"], "tags": ["…"],
//!     "fields": [{ "label": "Username", "type": "username", "value": "…",
//!                  "sensitive": 0, "deleted": 0, "order": 1 }],
//!     "attachments": [{ "name": "…", "kind": "image/png", "data": "<base64>" }]
//!   }] }
//! ```
//!
//! Categories: `login`, `password` and `computer` (servers, databases,
//! email accounts) become logins, `computer.wifi` a Wi-Fi item,
//! `creditcard` a card, `identity` an identity, `note` a secure note; the
//! other Enpass categories (`finance`, `license`, `travel`, `misc`) become
//! secure notes with their fields as custom fields. Fields are mapped by
//! their type (`username`, `email`, `password`, `url`, `totp`, `phone`,
//! `pin`, `cc*`) and, for identities and Wi-Fi items, by English or German
//! label; every other field becomes a custom field (hidden if Enpass marks
//! it sensitive). Deleted fields, sections and empty values are dropped.
//! Folders (with their parent path) and tags become tags; trashed items go
//! to the trash; archived items get the tag `archived`.

use std::collections::HashMap;

use mosaic_core::model::{ItemType, SecretField, TextField};
use zeroize::Zeroizing;

use crate::Format;
use crate::buf::{self, Str};
use crate::draft::{AttachmentBudget, Builder, EntryWarning, Kind};
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};

/// Deepest folder nesting followed when building a folder path.
const MAX_FOLDER_DEPTH: usize = 32;

/// Reads an Enpass JSON export.
///
/// # Errors
/// [`Error::TooLarge`] above [`crate::MAX_FILE_LEN`]; [`Error::Malformed`]
/// if the file is not JSON with an `items` array;
/// [`Error::LimitExceeded`] above the JSON or entry limits.
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let doc = json::parse(input)?;
    let Some(Json::Array(items)) = doc.get("items") else {
        return Err(Error::Malformed);
    };
    let folders = folder_paths(doc.array("folders"));
    let mut c = Collector::new(Format::Enpass);
    let mut budget = AttachmentBudget::new();
    for item in items {
        if item.is_object() {
            c.push(entry(item, &folders, &mut budget))?;
        } else {
            c.skip(SkipReason::Malformed)?;
        }
    }
    Ok(c.finish())
}

/// Folder UUID → path (`Parent/Child`).
fn folder_paths(folders: &[Json]) -> HashMap<String, Str> {
    let mut by_id: HashMap<&str, (&str, &str)> = HashMap::new();
    for f in folders {
        if let Some(id) = f.str("uuid") {
            by_id.insert(
                id,
                (
                    f.str("title").unwrap_or_default(),
                    f.str("parent_uuid").unwrap_or_default(),
                ),
            );
        }
    }
    let mut out = HashMap::new();
    for id in by_id.keys() {
        let mut parts: Vec<&str> = Vec::new();
        let mut cur = *id;
        while let Some((title, parent)) = by_id.get(cur) {
            if parts.len() >= MAX_FOLDER_DEPTH {
                break;
            }
            parts.push(title.trim());
            cur = parent;
        }
        parts.reverse();
        let mut path = Str::default();
        for p in parts.into_iter().filter(|p| !p.is_empty()) {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(p);
        }
        out.insert((*id).to_owned(), path);
    }
    out
}

/// The item type of an Enpass category and template.
fn item_type(category: &str, template: &str) -> (ItemType, bool) {
    match category {
        "computer" if template.contains("wifi") || template.contains("wireless") => {
            (ItemType::Wifi, true)
        }
        "login" | "password" | "computer" => (ItemType::Login, true),
        "creditcard" => (ItemType::CreditCard, true),
        "identity" => (ItemType::Identity, true),
        "note" | "finance" | "license" | "travel" | "misc" => (ItemType::SecureNote, true),
        _ => (ItemType::SecureNote, false),
    }
}

/// Identity fields by lowercase label (English and German).
pub(crate) fn identity_field(label: &str) -> Option<TextField> {
    Some(match label {
        "first name" | "firstname" | "given name" | "vorname" => TextField::FirstName,
        "middle name" | "middlename" | "zweiter vorname" => TextField::MiddleName,
        "last name" | "lastname" | "surname" | "family name" | "nachname" => TextField::LastName,
        "company" | "organization" | "organisation" | "firma" | "unternehmen" => TextField::Company,
        "address" | "street" | "address line 1" | "address 1" | "adresse" | "straße"
        | "strasse" => TextField::AddressLine1,
        "address line 2" | "address 2" => TextField::AddressLine2,
        "city" | "town" | "stadt" | "ort" => TextField::City,
        "state" | "province" | "region" | "bundesland" => TextField::Region,
        "zip" | "zip code" | "zipcode" | "postal code" | "postcode" | "plz" | "postleitzahl" => {
            TextField::PostalCode
        }
        "country" | "land" => TextField::Country,
        "email" | "e-mail" => TextField::Email,
        "phone" | "telephone" | "mobile" | "telefon" | "handy" => TextField::Phone,
        "birthday" | "date of birth" | "birth date" | "geburtstag" | "geburtsdatum" => {
            TextField::BirthDate
        }
        _ => return None,
    })
}

/// Wi-Fi fields by lowercase label.
fn wifi_field(label: &str) -> Option<TextField> {
    Some(match label {
        "ssid" | "network name" | "network" | "name" | "netzwerkname" | "netzwerk" => {
            TextField::WifiSsid
        }
        "security" | "security type" | "encryption" | "sicherheit" | "verschlüsselung" => {
            TextField::WifiSecurity
        }
        _ => return None,
    })
}

/// `MM/YYYY` from `MM/YY`, `MM/YYYY`, `MM-YYYY` or `YYYY-MM`.
pub(crate) fn card_expiry(value: &str) -> Option<String> {
    let v = value.trim();
    let (a, b) = v.split_once(['/', '-', '.'])?;
    let (a, b) = (a.trim(), b.trim());
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    if !digits(a) || !digits(b) {
        return None;
    }
    let (month, year) = if a.len() == 4 { (b, a) } else { (a, b) };
    let m: u32 = month.parse().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    let year = match year.len() {
        2 => format!("20{year}"),
        4 => year.to_owned(),
        _ => return None,
    };
    Some(format!("{m:02}/{year}"))
}

/// Sets the card expiry, or keeps an unrecognized value as a custom field.
pub(crate) fn expiry(b: &mut Builder, value: &str, label: &str) {
    match card_expiry(value) {
        Some(e) => b.text(TextField::CardExpiry, &e, label),
        None => b.fallback(label, Kind::Text, value),
    }
}

fn entry(item: &Json, folders: &HashMap<String, Str>, budget: &mut AttachmentBudget) -> Builder {
    let category = item.str("category").unwrap_or_default();
    let template = item.str("template_type").unwrap_or_default();
    let (ty, known) = item_type(category, template);
    let mut b = Builder::new(ty);
    if !known {
        b.warn(EntryWarning::UnknownCategory);
    }
    b.title(item.str("title").unwrap_or_default());
    let mut emails: Vec<(Str, Str)> = Vec::new();
    for field in item.array("fields") {
        if let Some(e) = self::field(&mut b, field) {
            emails.push(e);
        }
    }
    for (label, e) in emails {
        // A login without a user name signs in with its email.
        if b.item_type() == &ItemType::Login && !b.has_text(TextField::Username) {
            b.text(TextField::Username, &e, &label);
        } else {
            b.custom(&label, Kind::Email, &e);
        }
    }
    b.notes(item.str("note").unwrap_or_default());
    for id in item.array("folders").iter().filter_map(Json::as_str) {
        if let Some(path) = folders.get(id) {
            b.tag(path);
        }
    }
    for tag in item.array("tags") {
        match tag {
            Json::Str(t) => b.tag(t),
            other => b.tag(
                other
                    .str("title")
                    .or_else(|| other.str("name"))
                    .unwrap_or_default(),
            ),
        }
    }
    b.favorite(item.flag("favorite"));
    b.trashed(item.flag("trashed"));
    if item.flag("archived") {
        b.archived();
    }
    for a in item.array("attachments") {
        let name = a.str("name").unwrap_or_default();
        match a.str("data").map(buf::base64_decode) {
            Some(Ok(data)) => b.attachment(budget, name, a.str("kind"), &data),
            _ => b.warn(EntryWarning::AttachmentMissing),
        }
    }
    b
}

/// Maps one field. Returns the label and value of a login's email field,
/// which the caller places once the user name is known.
fn field(b: &mut Builder, f: &Json) -> Option<(Str, Str)> {
    if f.flag("deleted") {
        return None;
    }
    let ty = f.str("type").unwrap_or_default();
    let label = f.str("label").unwrap_or_default();
    let value = f.text("value")?;
    let value = value.as_str();
    // Blank text is dropped by the builder; a secret of spaces is kept.
    if ty == "section" || value.is_empty() {
        return None;
    }
    let sensitive = f.flag("sensitive");
    let item = b.item_type().clone();
    let lower = label.trim().to_lowercase();
    match (ty, &item) {
        ("username", ItemType::Login) => b.text(TextField::Username, value, label),
        ("email", ItemType::Login) => {
            return Some((
                Zeroizing::new(label.to_owned()),
                Zeroizing::new(value.to_owned()),
            ));
        }
        ("email", ItemType::Identity) => b.text(TextField::Email, value, label),
        ("email", _) => b.custom(label, Kind::Email, value),
        ("password", ItemType::Login) => b.secret(SecretField::Password, value, label),
        ("password", ItemType::Wifi) => b.secret(SecretField::WifiPassword, value, label),
        ("url", _) => b.url(value, label),
        ("totp", _) => b.otp_text(value, label),
        ("phone", ItemType::Identity) => b.text(TextField::Phone, value, label),
        ("phone", _) => b.custom(label, Kind::Phone, value),
        (_, ItemType::Identity) if identity_field(&lower) == Some(TextField::BirthDate) => {
            if crate::draft::is_iso_date(value.trim()) {
                b.text(TextField::BirthDate, value.trim(), label);
            } else {
                b.custom(label, Kind::Date, value);
            }
        }
        ("date", _) => b.custom(label, Kind::Date, value),
        ("ccName", ItemType::CreditCard) => b.text(TextField::CardholderName, value, label),
        ("ccNumber", ItemType::CreditCard) => b.secret(SecretField::CardNumber, value, label),
        ("ccCvc", ItemType::CreditCard) => {
            b.secret(SecretField::CardVerificationCode, value, label);
        }
        ("ccPin" | "pin", ItemType::CreditCard) => b.secret(SecretField::CardPin, value, label),
        ("ccExpiry", ItemType::CreditCard) => expiry(b, value, label),
        ("ccType", ItemType::CreditCard) => b.text(TextField::CardBrand, value, label),
        (_, ItemType::Identity) if !sensitive && identity_field(&lower).is_some() => {
            if let Some(tf) = identity_field(&lower) {
                b.text(tf, value, label);
            }
        }
        (_, ItemType::Wifi) if !sensitive && wifi_field(&lower).is_some() => {
            if let Some(tf) = wifi_field(&lower) {
                b.text(tf, value, label);
            }
        }
        _ => {
            let kind = if sensitive
                || matches!(
                    ty,
                    "password" | "pin" | "ccCvc" | "ccPin" | "ccNumber" | "ccTxnpassword"
                ) {
                Kind::Hidden
            } else {
                Kind::Text
            };
            if b.item_type() == &ItemType::SecureNote {
                // The fields of a note-like category are its content.
                b.custom(label, kind, value);
            } else {
                b.fallback(label, kind, value);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::*;

    #[test]
    fn expiry_formats() {
        for (v, want) in [
            ("12/25", Some("12/2025")),
            ("1/2030", Some("01/2030")),
            ("2031-07", Some("07/2031")),
            (" 03 - 27 ", Some("03/2027")),
            ("13/25", None),
            ("12/123", None),
            ("ab/cd", None),
            ("1225", None),
            ("/25", None),
        ] {
            assert_eq!(card_expiry(v).as_deref(), want, "{v}");
        }
    }

    #[test]
    fn folder_paths_follow_parents_and_stop_on_cycles() {
        let doc = json::parse(
            br#"[{"uuid":"a","title":"Work","parent_uuid":""},
                 {"uuid":"b","title":"Projects","parent_uuid":"a"},
                 {"uuid":"c","title":"Loop","parent_uuid":"d"},
                 {"uuid":"d","title":"Back","parent_uuid":"c"},
                 {"title":"no id"}]"#,
        )
        .unwrap();
        let paths = folder_paths(doc.items());
        assert_eq!(paths["a"].as_str(), "Work");
        assert_eq!(paths["b"].as_str(), "Work/Projects");
        assert!(paths["c"].ends_with("Back/Loop"));
        assert_eq!(paths.len(), 4);
    }

    #[test]
    fn labels() {
        assert_eq!(identity_field("plz"), Some(TextField::PostalCode));
        assert_eq!(identity_field("nickname"), None);
        assert_eq!(wifi_field("ssid"), Some(TextField::WifiSsid));
        assert_eq!(wifi_field("security type"), Some(TextField::WifiSecurity));
        assert_eq!(wifi_field("band"), None);
    }

    #[test]
    fn categories() {
        assert_eq!(
            item_type("computer", "computer.wifi"),
            (ItemType::Wifi, true)
        );
        assert_eq!(
            item_type("computer", "computer.wireless"),
            (ItemType::Wifi, true)
        );
        assert_eq!(
            item_type("computer", "computer.server"),
            (ItemType::Login, true)
        );
        assert_eq!(item_type("travel", ""), (ItemType::SecureNote, true));
        assert_eq!(item_type("crypto", ""), (ItemType::SecureNote, false));
    }
}
