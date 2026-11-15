//! Keeper exports: the JSON export (`records`, with typed `custom_fields`)
//! and the CSV export (no header row).
//!
//! Written from public descriptions of the exports. The JSON shape, the
//! `$type` values, the `$kind:label` field keys and the CSV column order are
//! unverified against a current export (launch gate).

use mosaic_core::model::{ItemType, SecretField, TextField};
use zeroize::Zeroizing;

use crate::Format;
use crate::buf::Str;
use crate::draft::{Builder, EntryWarning, Kind};
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};

/// Where a member of a typed custom field goes.
enum Target {
    Text(TextField),
    Secret(SecretField),
    Expiry,
}

const PAYMENT_CARD: &[(&str, Target)] = &[
    ("cardNumber", Target::Secret(SecretField::CardNumber)),
    ("cardExpirationDate", Target::Expiry),
    (
        "cardSecurityCode",
        Target::Secret(SecretField::CardVerificationCode),
    ),
];
const NAME: &[(&str, Target)] = &[
    ("first", Target::Text(TextField::FirstName)),
    ("middle", Target::Text(TextField::MiddleName)),
    ("last", Target::Text(TextField::LastName)),
];
const PHONE: &[(&str, Target)] = &[("number", Target::Text(TextField::Phone))];
const ADDRESS: &[(&str, Target)] = &[
    ("street1", Target::Text(TextField::AddressLine1)),
    ("street2", Target::Text(TextField::AddressLine2)),
    ("city", Target::Text(TextField::City)),
    ("state", Target::Text(TextField::Region)),
    ("zip", Target::Text(TextField::PostalCode)),
    ("country", Target::Text(TextField::Country)),
];

/// Reads a Keeper JSON export.
///
/// # Errors
/// [`Error::Malformed`] if the file is not JSON or has no `records` array;
/// [`Error::TooLarge`], [`Error::LimitExceeded`].
pub fn parse_json(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let doc = json::parse(input)?;
    let Some(Json::Array(records)) = doc.get("records") else {
        return Err(Error::Malformed);
    };
    let mut c = Collector::new(Format::Keeper);
    for record in records {
        if record.is_object() {
            c.push(entry(record))?;
        } else {
            c.skip(SkipReason::Malformed)?;
        }
    }
    Ok(c.finish())
}

fn entry(rec: &Json) -> Builder {
    let (item_type, unknown) = match rec.str("$type") {
        None | Some("login" | "general" | "databaseCredentials" | "serverCredentials") => {
            (ItemType::Login, false)
        }
        Some("bankCard") => (ItemType::CreditCard, false),
        Some("encryptedNotes") => (ItemType::SecureNote, false),
        Some("contact" | "address") => (ItemType::Identity, false),
        Some("wifiCredentials") => (ItemType::Wifi, false),
        Some(_) => (ItemType::SecureNote, true),
    };
    let mut b = Builder::new(item_type.clone());
    if unknown {
        b.warn(EntryWarning::UnknownCategory);
    }
    b.title(rec.str("title").unwrap_or_default());
    let login = rec.str("login").unwrap_or_default();
    let password = rec.str("password").unwrap_or_default();
    match item_type {
        ItemType::Login => {
            b.text(TextField::Username, login, "Username");
            b.secret(SecretField::Password, password, "Password");
        }
        ItemType::Wifi => {
            b.text(TextField::WifiSsid, login, "SSID");
            b.secret(SecretField::WifiPassword, password, "Password");
        }
        _ => {
            b.content("Login", Kind::Text, login);
            b.content("Password", Kind::Hidden, password);
        }
    }
    b.url(rec.str("login_url").unwrap_or_default(), "URL");
    b.notes(rec.str("notes").unwrap_or_default());
    if let Some(Json::Object(fields)) = rec.get("custom_fields") {
        for (key, value) in fields {
            custom_field(&mut b, &item_type, key, value);
        }
    }
    for folder in rec.array("folders") {
        let shared = folder.str("shared_folder").filter(|s| !s.trim().is_empty());
        let plain = folder.str("folder").filter(|s| !s.trim().is_empty());
        match (shared, plain) {
            (Some(s), Some(f)) => b.tag(&format!("{s}/{f}")),
            (Some(t), None) | (None, Some(t)) => b.tag(t),
            (None, None) => {}
        }
    }
    b
}

/// One value of a custom field: a string, or an object whose members are
/// strings.
enum Value<'a> {
    Scalar(Str),
    Object(&'a Json),
}

fn scalar(v: &Json) -> Option<Str> {
    match v {
        Json::Str(s) => Some(Zeroizing::new(s.as_str().to_owned())),
        Json::Int(i) => Some(Zeroizing::new(i.to_string())),
        Json::UInt(u) => Some(Zeroizing::new(u.to_string())),
        _ => None,
    }
}

/// The values of a custom field: arrays and objects are walked one level
/// each, anything nested deeper is not read.
fn values<'a>(b: &mut Builder, v: &'a Json) -> Vec<Value<'a>> {
    let one = |b: &mut Builder, e: &'a Json| match e {
        Json::Object(_) => Some(Value::Object(e)),
        Json::Array(_) => {
            b.warn(EntryWarning::FieldsDropped);
            None
        }
        other => scalar(other).map(Value::Scalar),
    };
    match v {
        Json::Array(items) => items.iter().filter_map(|e| one(b, e)).collect(),
        other => one(b, other).into_iter().collect(),
    }
}

fn custom_field(b: &mut Builder, item_type: &ItemType, key: &str, value: &Json) {
    let (kind, label) = match key.strip_prefix('$') {
        Some(rest) => match rest.split_once(':') {
            Some((kind, label)) => (Some(kind), Some(label)),
            None => (Some(rest), None),
        },
        None => (None, None),
    };
    for v in values(b, value) {
        match (kind, v) {
            (Some("oneTimeCode"), Value::Scalar(s)) => {
                b.otp_text(&s, label.unwrap_or("One-time password"));
            }
            (Some("paymentCard"), Value::Object(o)) => {
                object(b, o, PAYMENT_CARD, *item_type == ItemType::CreditCard);
            }
            (Some("name"), Value::Object(o)) => {
                object(b, o, NAME, *item_type == ItemType::Identity);
            }
            (Some("name"), Value::Scalar(s)) if *item_type == ItemType::Identity => {
                b.full_name(&s);
            }
            (Some("phone"), Value::Object(o)) => {
                object(b, o, PHONE, *item_type == ItemType::Identity);
            }
            (Some("address"), Value::Object(o)) => {
                object(b, o, ADDRESS, *item_type == ItemType::Identity);
            }
            (Some("email"), Value::Scalar(s)) if *item_type == ItemType::Identity => {
                b.text(TextField::Email, &s, "Email");
            }
            (Some("pinCode"), Value::Scalar(s)) if *item_type == ItemType::CreditCard => {
                b.secret(SecretField::CardPin, &s, "PIN");
            }
            (Some("keyPair" | "secret" | "password" | "pinCode"), Value::Scalar(s)) => {
                b.custom(label.or(kind).unwrap_or(key), Kind::Hidden, &s);
            }
            (Some("keyPair"), Value::Object(o)) => hidden_members(b, o),
            (_, Value::Scalar(s)) => {
                let name = label.unwrap_or(key);
                b.custom(name, Kind::for_key(name), &s);
            }
            (_, Value::Object(o)) => object(b, o, &[], false),
        }
    }
}

/// An object's string members as fields: through `table` if `usable`, else
/// as custom fields (hidden if the member name suggests a secret).
fn object(b: &mut Builder, o: &Json, table: &[(&str, Target)], usable: bool) {
    for (member, value) in o.members() {
        let Some(v) = scalar(value) else {
            if matches!(value, Json::Array(_) | Json::Object(_)) {
                b.warn(EntryWarning::FieldsDropped);
            }
            continue;
        };
        let target = table.iter().find(|(m, _)| *m == member.as_str());
        match target {
            Some((_, Target::Text(f))) if usable => b.text(*f, &v, member),
            Some((_, Target::Secret(f))) if usable => b.secret(*f, &v, member),
            Some((_, Target::Expiry)) if usable => b.expiry(&v, member),
            _ => b.content(member, Kind::for_key(member), &v),
        }
    }
}

fn hidden_members(b: &mut Builder, o: &Json) {
    for (member, value) in o.members() {
        match scalar(value) {
            Some(v) => b.custom(member, Kind::Hidden, &v),
            None => {
                if matches!(value, Json::Array(_) | Json::Object(_)) {
                    b.warn(EntryWarning::FieldsDropped);
                }
            }
        }
    }
}
