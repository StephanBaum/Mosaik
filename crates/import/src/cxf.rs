//! FIDO Credential Exchange Format v1.0 (CXF), JSON serialization.
//!
//! The member names follow the CDDL of the published CXF v1.0 Proposed
//! Standard (FIDO Alliance, 2025-08-14); the standard defines the data model
//! and this importer reads its JSON form (ADR-0041):
//!
//! ```json
//! { "version": { "major": 1, "minor": 0 },
//!   "exporterRpId": "…", "exporterDisplayName": "…", "timestamp": 1714566600,
//!   "accounts": [{
//!     "id": "<base64url>", "username": "…", "email": "…", "collections": [],
//!     "items": [{
//!       "id": "<base64url>", "creationAt": 1714566600, "modifiedAt": 1714566600,
//!       "title": "…", "favorite": false, "tags": ["…"],
//!       "scope": { "urls": ["https://example.com"], "androidApps": [] },
//!       "credentials": [
//!         { "type": "passkey", "credentialId": "<base64url>", "rpId": "example.com",
//!           "username": "…", "userDisplayName": "…", "userHandle": "<base64url>",
//!           "key": "<base64url PKCS#8>" },
//!         { "type": "basic-auth",
//!           "username": { "fieldType": "string", "value": "…" },
//!           "password": { "fieldType": "concealed-string", "value": "…" } },
//!         { "type": "note", "content": { "fieldType": "string", "value": "…" } }
//!       ] }] }] }
//! ```
//!
//! - `version.major` must be 1 ([`Error::UnsupportedVersion`] otherwise).
//! - A `passkey` credential becomes a `passkey` item (ES256 only; the key is
//!   validated and re-encoded, see [`crate::PasskeyDraft`]). CXF carries no
//!   signature counter, so it starts at 0; the creation time is the item's
//!   `creationAt` (seconds), else the import time.
//! - A `basic-auth` credential becomes a login: `username.value`,
//!   `password.value`, and the item's `scope.urls` as the login URLs.
//! - A `note` credential becomes the notes.
//! - Any other credential type (totp, credit-card, address, ...) is not
//!   imported; the entry gets [`crate::EntryWarning::UnmappedFields`].
//! - Item `tags` and `favorite` are kept. Accounts, collections and
//!   extensions are ignored.

use mosaic_core::model::{ItemType, SecretField, TextField};
use mosaic_passkey::{MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN};

use crate::draft::{Builder, EntryWarning};
use crate::entries::{Collector, Import, SkipReason};
use crate::error::{Error, Result};
use crate::json::{self, Json};
use crate::passkey::{self, Built, MAX_PKCS8_LEN, Parts, Problem};
use crate::{Format, buf};

/// Reads a CXF v1.0 JSON file.
///
/// # Errors
/// [`Error::Malformed`] if the file is not JSON with a `version` and an
/// `accounts` array; [`Error::UnsupportedVersion`] for a major version other
/// than 1; [`Error::TooLarge`], [`Error::LimitExceeded`].
pub fn parse(input: &[u8]) -> Result<Import> {
    if input.len() > crate::MAX_FILE_LEN {
        return Err(Error::TooLarge);
    }
    let doc = json::parse(input)?;
    let major = doc
        .get("version")
        .and_then(|v| v.get("major"))
        .and_then(Json::as_i64)
        .ok_or(Error::Malformed)?;
    if major != 1 {
        return Err(Error::UnsupportedVersion);
    }
    let Some(Json::Array(accounts)) = doc.get("accounts") else {
        return Err(Error::Malformed);
    };
    let mut c = Collector::new(Format::Cxf);
    for account in accounts {
        for item in account.array("items") {
            if item.is_object() && matches!(item.get("credentials"), Some(Json::Array(_))) {
                c.push(entry(item))?;
            } else {
                c.skip(SkipReason::Malformed)?;
            }
        }
    }
    Ok(c.finish())
}

fn entry(item: &Json) -> Builder {
    let creds = item.array("credentials");
    let kind = |t: &str| creds.iter().any(|c| c.str("type") == Some(t));
    let item_type = if !kind("basic-auth") && !kind("passkey") && kind("note") {
        ItemType::SecureNote
    } else {
        ItemType::Login
    };
    let mut b = Builder::new(item_type);
    b.title(item.str("title").unwrap_or_default());
    let created = item
        .get("creationAt")
        .and_then(Json::as_i64)
        .and_then(passkey::from_unix_seconds);
    for c in creds {
        match c.str("type") {
            Some("passkey") => b.passkey(passkey_of(c, created)),
            Some("basic-auth") => {
                b.text(TextField::Username, field(c, "username"), "Username");
                b.secret(SecretField::Password, field(c, "password"), "Password");
                for u in item.get("scope").map_or(&[][..], |s| s.array("urls")) {
                    b.url(u.as_str().unwrap_or_default(), "URL");
                }
            }
            Some("note") => b.notes(field(c, "content")),
            _ => b.warn(EntryWarning::UnmappedFields),
        }
    }
    for t in item.array("tags").iter().filter_map(Json::as_str) {
        b.tag(t);
    }
    b.favorite(item.flag("favorite"));
    b
}

/// The `value` of an `EditableField` member (`""` if it is absent).
fn field<'a>(credential: &'a Json, name: &str) -> &'a str {
    credential
        .get(name)
        .and_then(|f| f.str("value"))
        .unwrap_or_default()
}

fn passkey_of(c: &Json, created: Option<mosaic_core::Timestamp>) -> Built {
    let unreadable = |_| Problem::Unreadable;
    let credential_id = buf::base64url_decode(
        c.str("credentialId").ok_or(Problem::Unreadable)?,
        MAX_CREDENTIAL_ID_LEN,
    )
    .map_err(unreadable)?;
    let user_handle = buf::base64url_decode(
        c.str("userHandle").ok_or(Problem::Unreadable)?,
        MAX_USER_HANDLE_LEN,
    )
    .map_err(unreadable)?;
    let private_key =
        buf::base64url_decode(c.str("key").ok_or(Problem::Unreadable)?, MAX_PKCS8_LEN)
            .map_err(unreadable)?;
    passkey::build(&Parts {
        credential_id,
        rp_id: c.str("rpId").ok_or(Problem::Unreadable)?,
        user_handle,
        user_name: c.str("username").unwrap_or_default(),
        user_display_name: c.str("userDisplayName").unwrap_or_default(),
        private_key,
        sign_count: 0,
        discoverable: true,
        created_at: created,
    })
}
