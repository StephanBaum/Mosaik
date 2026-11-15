//! `NordPass` CSV rows: one `type` column decides whether a row is a login, a
//! note, a card, an identity or a folder.
//!
//! Written from public descriptions of the export. The header, the `type`
//! values and the JSON in `additional_urls` and `custom_fields` are
//! unverified against a current export (launch gate).

use mosaic_core::model::{ItemType, SecretField, TextField};

use crate::csv::Named;
use crate::draft::{Builder, EntryWarning, Kind};
use crate::json::{self, Json};

/// Columns every type reads.
const BASE: &[&str] = &["name", "note", "folder", "type", "custom fields"];
const LOGIN: &[&str] = &["url", "additional urls", "username", "password"];
const CARD: &[&str] = &[
    "cardholdername",
    "cardnumber",
    "cvc",
    "pin",
    "expirydate",
    "zipcode",
];
const IDENTITY: &[&str] = &[
    "full name",
    "email",
    "phone number",
    "address1",
    "address2",
    "city",
    "state",
    "zipcode",
    "country",
];

/// One row as an entry builder; `None` for a folder row (neither an entry
/// nor a skipped one).
pub(crate) fn entry(n: &Named<'_>) -> Option<Builder> {
    let kind = n.get("type").trim().to_lowercase();
    let (item_type, columns, unknown): (ItemType, &[&str], bool) = match kind.as_str() {
        "folder" => return None,
        "password" => (ItemType::Login, LOGIN, false),
        "note" => (ItemType::SecureNote, &[], false),
        "credit_card" => (ItemType::CreditCard, CARD, false),
        "identity" => (ItemType::Identity, IDENTITY, false),
        _ => (ItemType::SecureNote, &[], true),
    };
    let mut b = Builder::new(item_type);
    if unknown {
        b.warn(EntryWarning::UnknownCategory);
    }
    b.title(n.get("name"));
    b.notes(n.get("note"));
    if !n.get("folder").trim().is_empty() {
        b.tag(n.get("folder"));
    }
    match kind.as_str() {
        "password" => login(&mut b, n),
        "credit_card" => card(&mut b, n),
        "identity" => identity(&mut b, n),
        _ => {}
    }
    custom_fields(&mut b, n.get("custom fields"));
    let known: Vec<&str> = BASE.iter().chain(columns).copied().collect();
    for (label, value) in n.others(&known) {
        b.content(label, Kind::Text, value);
    }
    Some(b)
}

fn login(b: &mut Builder, n: &Named<'_>) {
    b.url(n.get("url"), "URL");
    let extra = n.get("additional urls").trim();
    if extra.starts_with('[') {
        match json::parse(extra.as_bytes()) {
            Ok(doc) => {
                for url in doc.items().iter().filter_map(Json::as_str) {
                    b.url(url, "URL");
                }
            }
            Err(_) => b.url(extra, "URL"),
        }
    } else {
        b.url(extra, "URL");
    }
    b.text(TextField::Username, n.get("username"), "Username");
    b.secret(SecretField::Password, n.get("password"), "Password");
}

fn card(b: &mut Builder, n: &Named<'_>) {
    b.text(
        TextField::CardholderName,
        n.get("cardholdername"),
        "Cardholder name",
    );
    b.secret(SecretField::CardNumber, n.get("cardnumber"), "Card number");
    b.secret(
        SecretField::CardVerificationCode,
        n.get("cvc"),
        "Verification code",
    );
    b.secret(SecretField::CardPin, n.get("pin"), "PIN");
    if !n.get("expirydate").trim().is_empty() {
        b.expiry(n.get("expirydate"), "Expiry date");
    }
    b.fallback("Zip code", Kind::Text, n.get("zipcode"));
}

fn identity(b: &mut Builder, n: &Named<'_>) {
    b.full_name(n.get("full name"));
    for (column, field, label) in [
        ("email", TextField::Email, "Email"),
        ("phone number", TextField::Phone, "Phone"),
        ("address1", TextField::AddressLine1, "Address 1"),
        ("address2", TextField::AddressLine2, "Address 2"),
        ("city", TextField::City, "City"),
        ("state", TextField::Region, "State"),
        ("zipcode", TextField::PostalCode, "Zip code"),
        ("country", TextField::Country, "Country"),
    ] {
        b.text(field, n.get(column), label);
    }
}

/// `custom_fields`: a JSON array of `{label, type, value}`. Anything else
/// is kept whole as one text field.
fn custom_fields(b: &mut Builder, raw: &str) {
    if raw.trim().is_empty() {
        return;
    }
    let doc = json::parse(raw.as_bytes()).ok();
    let Some(Json::Array(items)) = &doc else {
        b.fallback("Custom fields", Kind::Text, raw);
        return;
    };
    for item in items {
        let label = item.str("label").unwrap_or_default();
        let value = item.str("value").unwrap_or_default();
        let kind = match item.str("type") {
            Some("hidden") => Kind::Hidden,
            Some("date") => Kind::Date,
            _ => Kind::Text,
        };
        b.custom(label, kind, value);
    }
}
