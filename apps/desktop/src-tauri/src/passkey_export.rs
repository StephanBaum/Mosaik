//! The passkey export file: FIDO Credential Exchange Format v1.0, JSON
//! (ADR-0041).
//!
//! ```json
//! { "version": { "major": 1, "minor": 0 }, "exporterRpId": "mosaic.local",
//!   "exporterDisplayName": "Mosaic", "timestamp": 1714566600,
//!   "accounts": [{ "id": "<random id>", "username": "", "email": "",
//!     "collections": [], "items": [{
//!       "id": "<random id>", "creationAt": 1714566600, "modifiedAt": 1714566600,
//!       "title": "example.com",
//!       "credentials": [{ "type": "passkey", "credentialId": "...",
//!         "rpId": "example.com", "username": "...", "userDisplayName": "...",
//!         "userHandle": "...", "key": "<base64url PKCS#8>" }] }] }] }
//! ```
//!
//! All binary members are unpadded base64url. The account ID and every item
//! ID are fresh random 16-byte values per export: the file carries neither
//! the vault ID nor an item ID of Mosaic. Only ES256 passkeys of live items
//! are written. The whole file is built in one zeroizing buffer sized
//! by a counting pass first, so it never grows (and never leaves a stale
//! copy of a private key behind); the caller writes it and drops it.
//! Nothing here touches the disk.

use std::io::{self, Write};

use mosaic_core::model::ItemType;
use mosaic_core::{SecretBytes, VaultId};
use mosaic_passkey::{COSE_ALG_ES256, b64url};
use mosaic_vault::session::{ItemFilter, UnlockedSession};
use serde::Serialize;
use zeroize::Zeroizing;

use crate::contract::errors::{ErrorCode, IpcError, IpcResult};

/// The `exporterRpId` of the file.
pub const EXPORTER_RP_ID: &str = "mosaic.local";

#[derive(Serialize)]
struct Version {
    major: u8,
    minor: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Document<'a> {
    version: Version,
    exporter_rp_id: &'static str,
    exporter_display_name: &'static str,
    timestamp: u64,
    accounts: [Account<'a>; 1],
}

#[derive(Serialize)]
struct Account<'a> {
    id: String,
    username: &'static str,
    email: &'static str,
    collections: [u8; 0],
    items: Vec<Item<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Item<'a> {
    id: &'a str,
    creation_at: u64,
    modified_at: u64,
    title: &'a str,
    credentials: [Passkey<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Passkey<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    credential_id: &'a str,
    rp_id: &'a str,
    username: &'a str,
    user_display_name: &'a str,
    user_handle: &'a str,
    key: &'a str,
}

/// The built file.
pub struct Exported {
    /// The JSON text, wiped on drop.
    pub json: Zeroizing<Vec<u8>>,
    /// How many passkeys it holds.
    pub count: u32,
}

/// What one passkey item contributes, owned so the document can borrow it.
struct Entry {
    /// The item's real ID: orders the entries, never written.
    order: [u8; 16],
    title: String,
    created: u64,
    credential_id: String,
    rp_id: String,
    user_name: String,
    user_display_name: String,
    user_handle: String,
    key: Zeroizing<String>,
}

/// A writer that only counts.
struct Counter(usize);

impl Write for Counter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn passkey_items(
    session: &UnlockedSession,
    vault: VaultId,
) -> Vec<mosaic_vault::session::ItemRef<'_>> {
    let mut filter = ItemFilter::default();
    filter.vault = Some(vault);
    session
        .items(&filter)
        .into_iter()
        .filter(|r| r.item.item_type() == Some(ItemType::Passkey))
        .collect()
}

/// The ES256 passkeys of the live items of `vault`, in a stable order.
fn entries(session: &UnlockedSession, vault: VaultId) -> Vec<Entry> {
    let mut out: Vec<Entry> = passkey_items(session, vault)
        .into_iter()
        .filter_map(|r| {
            // The one place a private key is copied: this credential is being
            // exported. The credential zeroizes at the end of the closure.
            let credential = r.item.passkey()?;
            if credential.algorithm != COSE_ALG_ES256 {
                return None;
            }
            Some(Entry {
                order: *r.item_id.as_bytes(),
                title: r.item.title().unwrap_or_default().to_owned(),
                created: u64::try_from(credential.created_at.as_unix_millis() / 1000).unwrap_or(0),
                credential_id: b64url(&credential.credential_id),
                rp_id: credential.rp_id.as_str().to_owned(),
                user_name: credential.user_name.as_str().to_owned(),
                user_display_name: credential.user_display_name.as_str().to_owned(),
                user_handle: b64url(&credential.user_handle),
                key: Zeroizing::new(b64url(credential.private_key.expose_secret())),
            })
        })
        .collect();
    out.sort_by(|a, b| (&a.title, a.order).cmp(&(&b.title, b.order)));
    out
}

/// A fresh random 16-byte ID, base64url.
fn random_id() -> IpcResult<String> {
    let id = SecretBytes::<16>::random().map_err(|_| IpcError::INTERNAL)?;
    Ok(b64url(id.expose_secret()))
}

/// How many ES256 passkeys `vault` would export (no key is read).
#[must_use]
pub fn count(session: &UnlockedSession, vault: VaultId) -> usize {
    passkey_items(session, vault)
        .into_iter()
        .filter(|r| {
            r.item
                .passkey_public()
                .is_some_and(|p| p.algorithm == COSE_ALG_ES256)
        })
        .count()
}

/// Builds the CXF file for the ES256 passkeys of `vault`; `now` is the Unix
/// time in seconds.
///
/// # Errors
/// `itemNotFound` if the vault holds no ES256 passkey, `internal` if the
/// buffer cannot be allocated or serialized.
pub fn build(session: &UnlockedSession, vault: VaultId, now: u64) -> IpcResult<Exported> {
    let entries = entries(session, vault);
    if entries.is_empty() {
        return Err(IpcError::new(ErrorCode::ItemNotFound));
    }
    let ids = entries
        .iter()
        .map(|_| random_id())
        .collect::<IpcResult<Vec<_>>>()?;
    let items: Vec<Item<'_>> = entries
        .iter()
        .zip(&ids)
        .map(|(e, id)| Item {
            id,
            creation_at: e.created,
            modified_at: e.created,
            title: &e.title,
            credentials: [Passkey {
                kind: "passkey",
                credential_id: &e.credential_id,
                rp_id: &e.rp_id,
                username: &e.user_name,
                user_display_name: &e.user_display_name,
                user_handle: &e.user_handle,
                key: &e.key,
            }],
        })
        .collect();
    let document = Document {
        version: Version { major: 1, minor: 0 },
        exporter_rp_id: EXPORTER_RP_ID,
        exporter_display_name: "Mosaic",
        timestamp: now,
        accounts: [Account {
            id: random_id()?,
            username: "",
            email: "",
            collections: [],
            items,
        }],
    };
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, &document).map_err(|_| IpcError::INTERNAL)?;
    let mut json = Vec::new();
    json.try_reserve_exact(counter.0)
        .map_err(|_| IpcError::INTERNAL)?;
    let mut json = Zeroizing::new(json);
    serde_json::to_writer(&mut *json, &document).map_err(|_| IpcError::INTERNAL)?;
    Ok(Exported {
        json,
        count: u32::try_from(entries.len()).unwrap_or(u32::MAX),
    })
}

/// The suggested file name: `mosaic-passkeys-YYYY-MM-DD.json` (UTC).
#[must_use]
pub fn suggested_file_name(now: u64) -> String {
    let (y, m, d) = civil_from_days(i64::try_from(now / 86_400).unwrap_or(0));
    format!("mosaic-passkeys-{y:04}-{m:02}-{d:02}.json")
}

/// The proleptic Gregorian date of a day count since 1970-01-01 (Hinnant's
/// `civil_from_days`).
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_the_utc_date() {
        assert_eq!(suggested_file_name(0), "mosaic-passkeys-1970-01-01.json");
        assert_eq!(
            suggested_file_name(1_714_566_600),
            "mosaic-passkeys-2024-05-01.json"
        );
        assert_eq!(
            suggested_file_name(1_709_164_800),
            "mosaic-passkeys-2024-02-29.json"
        );
    }
}
