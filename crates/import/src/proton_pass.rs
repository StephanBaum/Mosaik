//! Proton Pass export: a zip with `Proton Pass/data.json`, or that
//! `data.json` itself. Only the unencrypted export is read; the
//! PGP-encrypted one is refused as [`Error::Encrypted`].
//!
//! Written from public descriptions of the export. The JSON shape and the
//! item types are unverified against a current export (launch gate).

use std::io::Cursor;

use mosaic_core::model::{ItemType, SecretField, TextField};
use zip::ZipArchive;

use crate::Format;
use crate::archive;
use crate::draft::{Builder, EntryWarning, Kind, is_iso_date};
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};

/// Largest `data.json`: 64 MiB.
pub const MAX_DATA_JSON_LEN: usize = 64 * 1024 * 1024;

const PGP_ARMOR: &[u8] = b"-----BEGIN PGP MESSAGE-----";
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const DATA_JSON: &str = "Proton Pass/data.json";

/// What an archive holds of a Proton Pass export.
pub(crate) enum Located {
    /// The member index of `data.json`.
    Json(usize),
    /// An encrypted `data.pgp` and no `data.json`.
    Pgp,
}

/// Finds the export in an archive: `Proton Pass/data.json`, else the first
/// member named `data.json` in any folder; a `data.pgp` without one is the
/// encrypted export.
pub(crate) fn locate(zip: &ZipArchive<Cursor<&[u8]>>) -> Option<Located> {
    archive::find(zip, |n| n == DATA_JSON)
        .or_else(|| archive::find(zip, |n| n == "data.json" || n.ends_with("/data.json")))
        .map(Located::Json)
        .or_else(|| archive::find(zip, |n| n.ends_with("data.pgp")).map(|_| Located::Pgp))
}

/// Reads a Proton Pass export.
///
/// # Errors
/// [`Error::Encrypted`] for the PGP-encrypted export (or `encrypted: true`);
/// [`Error::Malformed`] if the input is neither a zip with `data.json` nor
/// that JSON; [`Error::TooLarge`], [`Error::LimitExceeded`].
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    if input.starts_with(PGP_ARMOR) {
        return Err(Error::Encrypted);
    }
    if input.starts_with(ZIP_MAGIC) {
        let mut zip = archive::open(input, crate::onepux::MAX_ARCHIVE_MEMBERS)?;
        return match locate(&zip) {
            Some(Located::Json(index)) => {
                let data = archive::read_member(&mut zip, index, MAX_DATA_JSON_LEN)?;
                export(&data)
            }
            Some(Located::Pgp) => Err(Error::Encrypted),
            None => Err(Error::Malformed),
        };
    }
    if input.len() > MAX_DATA_JSON_LEN {
        return Err(Error::TooLarge);
    }
    export(input)
}

fn export(data: &[u8]) -> Result<Import> {
    let doc = json::parse(data)?;
    if doc.flag("encrypted") {
        return Err(Error::Encrypted);
    }
    let Some(vaults) = doc.get("vaults").filter(|v| v.is_object()) else {
        return Err(Error::Malformed);
    };
    let tag_vaults = vaults.members().len() > 1;
    let mut c = Collector::new(Format::ProtonPass);
    for (_, vault) in vaults.members() {
        let name = vault.str("name");
        for item in vault.array("items") {
            if !item.is_object() {
                c.skip(SkipReason::Malformed)?;
                continue;
            }
            let mut b = entry(item);
            if tag_vaults && let Some(name) = name {
                b.tag(name);
            }
            c.push(b)?;
        }
    }
    Ok(c.finish())
}

fn text<'a>(j: Option<&'a Json>, key: &str) -> &'a str {
    j.and_then(|j| j.str(key)).unwrap_or_default()
}

fn entry(item: &Json) -> Builder {
    let data = item.get("data");
    let meta = data.and_then(|d| d.get("metadata"));
    let content = data.and_then(|d| d.get("content"));
    let ty = data.and_then(|d| d.str("type")).unwrap_or_default();
    let (item_type, unknown) = match ty {
        "login" | "alias" => (ItemType::Login, false),
        "note" => (ItemType::SecureNote, false),
        "creditCard" => (ItemType::CreditCard, false),
        "identity" => (ItemType::Identity, false),
        _ => (ItemType::SecureNote, true),
    };
    let mut b = Builder::new(item_type);
    if unknown {
        b.warn(EntryWarning::UnknownCategory);
    }
    b.title(text(meta, "name"));
    b.notes(text(meta, "note"));
    b.favorite(item.flag("pinned"));
    b.trashed(item.get("state").and_then(Json::as_i64) == Some(2));
    let known: &[&str] = match ty {
        "login" => login(&mut b, content),
        "alias" => {
            b.text(
                TextField::Username,
                item.str("aliasEmail").unwrap_or_default(),
                "Username",
            );
            &[]
        }
        "creditCard" => card(&mut b, content),
        "identity" => identity(&mut b, content),
        _ => &[],
    };
    for (key, value) in content.map_or(&[][..], Json::members) {
        if let Json::Str(s) = value
            && !known.contains(&key.as_str())
        {
            b.content(key, Kind::for_key(key), s);
        }
    }
    for field in data.map_or(&[][..], |d| d.array("extraFields")) {
        extra_field(&mut b, field);
    }
    let has_files = [item, data.unwrap_or(item)]
        .iter()
        .any(|j| !j.array("files").is_empty() || !j.array("attachments").is_empty());
    if has_files {
        b.warn(EntryWarning::AttachmentMissing);
    }
    b
}

/// Returns the content keys the login mapping read.
fn login(b: &mut Builder, content: Option<&Json>) -> &'static [&'static str] {
    let get = |key: &str| content.and_then(|c| c.str(key)).unwrap_or_default();
    let (user, email) = (get("itemUsername"), get("itemEmail"));
    if user.trim().is_empty() {
        b.text(TextField::Username, email, "Username");
    } else {
        b.text(TextField::Username, user, "Username");
        b.custom("Email", Kind::Email, email);
    }
    b.secret(SecretField::Password, get("password"), "Password");
    for url in content.map_or(&[][..], |c| c.array("urls")) {
        if let Some(u) = url.as_str() {
            b.url(u, "URL");
        }
    }
    b.otp_text(get("totpUri"), "One-time password");
    if !content.is_none_or(|c| c.array("passkeys").is_empty()) {
        b.warn(EntryWarning::PasskeysNotImported);
    }
    &["itemUsername", "itemEmail", "password", "totpUri"]
}

fn card(b: &mut Builder, content: Option<&Json>) -> &'static [&'static str] {
    let get = |key: &str| content.and_then(|c| c.str(key)).unwrap_or_default();
    b.text(
        TextField::CardholderName,
        get("cardholderName"),
        "Cardholder",
    );
    b.text(TextField::CardBrand, get("cardType"), "Card type");
    b.secret(SecretField::CardNumber, get("number"), "Number");
    b.secret(
        SecretField::CardVerificationCode,
        get("verificationNumber"),
        "Verification number",
    );
    if !get("expirationDate").trim().is_empty() {
        b.expiry(get("expirationDate"), "Expiration date");
    }
    b.secret(SecretField::CardPin, get("pin"), "PIN");
    &[
        "cardholderName",
        "cardType",
        "number",
        "verificationNumber",
        "expirationDate",
        "pin",
    ]
}

fn identity(b: &mut Builder, content: Option<&Json>) -> &'static [&'static str] {
    let get = |key: &str| content.and_then(|c| c.str(key)).unwrap_or_default();
    if get("fullName").trim().is_empty() {
        b.text(TextField::FirstName, get("firstName"), "First name");
        b.text(TextField::MiddleName, get("middleName"), "Middle name");
        b.text(TextField::LastName, get("lastName"), "Last name");
    } else {
        b.full_name(get("fullName"));
    }
    for (key, field, label) in [
        ("email", TextField::Email, "Email"),
        ("phoneNumber", TextField::Phone, "Phone"),
        ("organization", TextField::Company, "Organization"),
        ("streetAddress", TextField::AddressLine1, "Street address"),
        ("city", TextField::City, "City"),
        ("stateOrProvince", TextField::Region, "State or province"),
        (
            "zipOrPostalCode",
            TextField::PostalCode,
            "Zip or postal code",
        ),
        ("countryOrRegion", TextField::Country, "Country or region"),
    ] {
        b.text(field, get(key), label);
    }
    let birth = get("birthdate").trim();
    if is_iso_date(birth) {
        b.text(TextField::BirthDate, birth, "Birthdate");
    } else {
        b.fallback("Birthdate", Kind::Text, birth);
    }
    &[
        "fullName",
        "firstName",
        "middleName",
        "lastName",
        "email",
        "phoneNumber",
        "organization",
        "streetAddress",
        "city",
        "stateOrProvince",
        "zipOrPostalCode",
        "countryOrRegion",
        "birthdate",
    ]
}

/// `extraFields`: `{fieldName, type, data: {content}}`.
fn extra_field(b: &mut Builder, field: &Json) {
    let name = field.str("fieldName").unwrap_or_default();
    let value = field
        .get("data")
        .and_then(|d| d.str("content"))
        .unwrap_or_default();
    match field.str("type") {
        Some("hidden") => b.custom(name, Kind::Hidden, value),
        Some("totp") => b.custom_otp(name, value),
        _ => b.custom(name, Kind::Text, value),
    }
}
