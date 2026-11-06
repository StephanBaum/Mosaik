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
//! and collections become tags. Passkeys (`fido2Credentials`) become
//! `passkey` items:
//!
//! ```json
//! { "credentialId": "<uuid>" | "b64.<base64url>", "keyType": "public-key",
//!   "keyAlgorithm": "ECDSA", "keyCurve": "P-256", "keyValue": "<base64url PKCS#8>",
//!   "rpId": "example.com", "userHandle": "<base64url>", "userName": "…",
//!   "userDisplayName": "…", "counter": "0", "discoverable": "true",
//!   "creationDate": "2024-05-01T12:30:00Z" }
//! ```
//!
//! Only ES256 (ECDSA on P-256) is imported; another key type, algorithm or
//! curve gives [`crate::EntryWarning::PasskeyUnsupported`], a member that is
//! missing or does not decode (a null `userHandle` included) gives
//! [`crate::EntryWarning::PasskeyUnreadable`]. A password-protected or account-encrypted export fails with
//! [`Error::Encrypted`].

use std::collections::HashMap;

use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_passkey::{MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN};
use zeroize::Zeroizing;

use crate::draft::{Builder, EntryWarning, Kind};
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};
use crate::passkey::{self, Built, MAX_PKCS8_LEN, Parts, Problem};
use crate::{Format, buf};

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
    for credential in l.array("fido2Credentials") {
        b.passkey(fido2_passkey(credential));
    }
}

/// One `fido2Credentials` member as a passkey draft.
fn fido2_passkey(c: &Json) -> Built {
    // Metadata first: a credential of another algorithm is unsupported even
    // if its other members would not decode; one without the metadata is
    // unreadable.
    let meta = |key: &str| c.str(key).ok_or(Problem::Unreadable);
    let (ty, alg, curve) = (meta("keyType")?, meta("keyAlgorithm")?, meta("keyCurve")?);
    if (ty, alg, curve) != ("public-key", "ECDSA", "P-256") {
        return Err(Problem::Unsupported);
    }
    let unreadable = |_| Problem::Unreadable;
    let id = c.str("credentialId").ok_or(Problem::Unreadable)?;
    let credential_id = if let Some(encoded) = id.strip_prefix("b64.") {
        buf::base64url_decode(encoded, MAX_CREDENTIAL_ID_LEN).map_err(unreadable)?
    } else {
        let bytes = passkey::uuid_bytes(id).ok_or(Problem::Unreadable)?;
        Zeroizing::new(bytes.to_vec())
    };
    let user_handle = buf::base64url_decode(
        c.str("userHandle").ok_or(Problem::Unreadable)?,
        MAX_USER_HANDLE_LEN,
    )
    .map_err(unreadable)?;
    let private_key =
        buf::base64url_decode(c.str("keyValue").ok_or(Problem::Unreadable)?, MAX_PKCS8_LEN)
            .map_err(unreadable)?;
    let sign_count = match c.text("counter") {
        None => 0,
        Some(n) => n.trim().parse::<u64>().map_err(|_| Problem::Unreadable)?,
    };
    let discoverable = match c.get("discoverable") {
        None => true,
        Some(d) => d.as_flag().ok_or(Problem::Unreadable)?,
    };
    passkey::build(&Parts {
        credential_id,
        rp_id: c.str("rpId").ok_or(Problem::Unreadable)?,
        user_handle,
        user_name: c.str("userName").unwrap_or_default(),
        user_display_name: c.str("userDisplayName").unwrap_or_default(),
        private_key,
        sign_count,
        discoverable,
        created_at: c.str("creationDate").and_then(passkey::parse_rfc3339),
    })
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
        (Some(m), Some(y)) => b.expiry(&format!("{}/{}", m.as_str(), y.as_str()), "Expiry"),
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
