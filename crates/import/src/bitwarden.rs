//! Bitwarden JSON export, unencrypted ("Export vault → .json").
//!
//! ```json
//! { "encrypted": false,
//!   "folders": [{ "id": "…", "name": "Work" }],
//!   "collections": [{ "id": "…", "name": "Team" }],
//!   "items": [{
//!     "type": 1, "name": "…", "notes": "…", "favorite": true,
//!     "folderId": "…", "collectionIds": ["…"], "deletedDate": null,
//!     "fields": [{ "name": "…", "value": "…", "type": 0 }],
//!     "login": { "username": "…", "password": "…", "totp": "…",
//!                "uris": [{ "uri": "…" }], "fido2Credentials": [] },
//!     "passwordHistory": [{ "password": "…", "lastUsedDate": "…" }]
//!   }] }
//! ```
//!
//! Types: 1 login, 2 secure note, 3 card, 4 identity, 5 SSH key; any other
//! type becomes a secure note. Custom fields keep their kind (text, hidden,
//! boolean as text; linked fields hold no value and are dropped). Folders
//! and collections become tags. Passkeys (`fido2Credentials`) are not
//! imported yet ([`crate::EntryWarning::PasskeysNotImported`]). A
//! password-protected or account-encrypted export fails with
//! [`Error::Encrypted`].

use std::collections::HashMap;

use mosaic_core::model::{ItemType, SecretField, TextField};

use crate::Format;
use crate::draft::{Builder, EntryWarning, Kind};
use crate::enpass::expiry;
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};

/// Reads a Bitwarden JSON export.
///
/// # Errors
/// [`Error::Encrypted`] for an encrypted export; [`Error::Malformed`] if
/// the file is not JSON with an `items` array; [`Error::TooLarge`],
/// [`Error::LimitExceeded`].
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let doc = json::parse(input)?;
    if doc.flag("encrypted") || doc.flag("passwordProtected") {
        return Err(Error::Encrypted);
    }
    let Some(Json::Array(items)) = doc.get("items") else {
        return Err(Error::Malformed);
    };
    let names = |key: &str| -> HashMap<&str, &str> {
        doc.array(key)
            .iter()
            .filter_map(|f| Some((f.str("id")?, f.str("name")?)))
            .collect()
    };
    let folders = names("folders");
    let collections = names("collections");
    let mut c = Collector::new(Format::Bitwarden);
    for item in items {
        match item.get("type").and_then(Json::as_i64) {
            Some(ty) if item.is_object() => c.push(entry(item, ty, &folders, &collections))?,
            _ => c.skip(SkipReason::Malformed)?,
        }
    }
    Ok(c.finish())
}

fn entry(
    item: &Json,
    ty: i64,
    folders: &HashMap<&str, &str>,
    collections: &HashMap<&str, &str>,
) -> Builder {
    let item_type = match ty {
        1 => ItemType::Login,
        3 => ItemType::CreditCard,
        4 => ItemType::Identity,
        5 => ItemType::SshKey,
        _ => ItemType::SecureNote,
    };
    let mut b = Builder::new(item_type);
    if !(1..=5).contains(&ty) {
        b.warn(EntryWarning::UnknownCategory);
    }
    b.title(item.str("name").unwrap_or_default());
    match ty {
        1 => login(&mut b, item.get("login")),
        3 => card(&mut b, item.get("card")),
        4 => identity(&mut b, item.get("identity")),
        5 => {
            if let Some(k) = item.get("sshKey") {
                b.ssh_key(
                    k.str("privateKey").unwrap_or_default(),
                    k.str("publicKey").unwrap_or_default(),
                    k.str("keyFingerprint").unwrap_or_default(),
                );
            }
        }
        _ => {}
    }
    b.notes(item.str("notes").unwrap_or_default());
    for f in item.array("fields") {
        let name = f.str("name").unwrap_or_default();
        let Some(value) = f.text("value") else {
            continue;
        };
        match f.get("type").and_then(Json::as_i64) {
            Some(1) => b.custom(name, Kind::Hidden, &value),
            Some(3) => {}
            _ => b.custom(name, Kind::Text, &value),
        }
    }
    let mut history: Vec<(&str, &str)> = item
        .array("passwordHistory")
        .iter()
        .filter_map(|h| {
            Some((
                h.str("lastUsedDate").unwrap_or_default(),
                h.str("password")?,
            ))
        })
        .collect();
    // ISO 8601 timestamps sort as text; oldest first.
    history.sort_by(|a, b| a.0.cmp(b.0));
    for (_, old) in history {
        b.history(old);
    }
    if let Some(name) = item.str("folderId").and_then(|id| folders.get(id)) {
        b.tag(name);
    }
    for id in item.array("collectionIds").iter().filter_map(Json::as_str) {
        if let Some(name) = collections.get(id) {
            b.tag(name);
        }
    }
    b.favorite(item.flag("favorite"));
    b.trashed(
        item.get("deletedDate")
            .is_some_and(|d| d.as_str().is_some()),
    );
    b
}

fn login(b: &mut Builder, login: Option<&Json>) {
    let Some(l) = login else { return };
    b.text(
        TextField::Username,
        l.str("username").unwrap_or_default(),
        "Username",
    );
    b.secret(
        SecretField::Password,
        l.str("password").unwrap_or_default(),
        "Password",
    );
    for u in l.array("uris") {
        b.url(u.str("uri").unwrap_or_default(), "URL");
    }
    b.otp_text(l.str("totp").unwrap_or_default(), "TOTP");
    if !l.array("fido2Credentials").is_empty() {
        b.warn(EntryWarning::PasskeysNotImported);
    }
}

fn card(b: &mut Builder, card: Option<&Json>) {
    let Some(c) = card else { return };
    b.text(
        TextField::CardholderName,
        c.str("cardholderName").unwrap_or_default(),
        "Cardholder",
    );
    b.text(
        TextField::CardBrand,
        c.str("brand").unwrap_or_default(),
        "Brand",
    );
    b.secret(
        SecretField::CardNumber,
        c.str("number").unwrap_or_default(),
        "Number",
    );
    b.secret(
        SecretField::CardVerificationCode,
        c.str("code").unwrap_or_default(),
        "Security code",
    );
    match (c.text("expMonth"), c.text("expYear")) {
        (Some(m), Some(y)) => expiry(b, &format!("{}/{}", m.as_str(), y.as_str()), "Expiry"),
        (Some(m), None) => b.fallback("Expiry month", Kind::Text, &m),
        (None, Some(y)) => b.fallback("Expiry year", Kind::Text, &y),
        (None, None) => {}
    }
}

fn identity(b: &mut Builder, identity: Option<&Json>) {
    let Some(i) = identity else { return };
    let text = |b: &mut Builder, key: &str, field: TextField, label: &str| {
        b.text(field, i.str(key).unwrap_or_default(), label);
    };
    b.fallback("Title", Kind::Text, i.str("title").unwrap_or_default());
    text(b, "firstName", TextField::FirstName, "First name");
    text(b, "middleName", TextField::MiddleName, "Middle name");
    text(b, "lastName", TextField::LastName, "Last name");
    text(b, "company", TextField::Company, "Company");
    text(b, "email", TextField::Email, "Email");
    text(b, "phone", TextField::Phone, "Phone");
    text(b, "address1", TextField::AddressLine1, "Address 1");
    text(b, "address2", TextField::AddressLine2, "Address 2");
    b.fallback(
        "Address 3",
        Kind::Text,
        i.str("address3").unwrap_or_default(),
    );
    text(b, "city", TextField::City, "City");
    text(b, "state", TextField::Region, "State");
    text(b, "postalCode", TextField::PostalCode, "Postal code");
    text(b, "country", TextField::Country, "Country");
    b.fallback(
        "Username",
        Kind::Text,
        i.str("username").unwrap_or_default(),
    );
    let secret = |b: &mut Builder, key: &str, field: SecretField, label: &str| {
        b.secret(field, i.str(key).unwrap_or_default(), label);
    };
    secret(b, "ssn", SecretField::NationalId, "Social security number");
    secret(
        b,
        "passportNumber",
        SecretField::PassportNumber,
        "Passport number",
    );
    secret(
        b,
        "licenseNumber",
        SecretField::LicenseNumber,
        "License number",
    );
}
