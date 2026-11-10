//! The plaintext of an encrypted export (CRYPTO_SPEC §11.2, ADR-0073).
//!
//! The export file is `mosaic_core::ExportWriter` over this JSON (compact,
//! UTF-8):
//!
//! ```json
//! {"format":"mosaic-export","v":1,"exportedAt":1700000000000,
//!  "vaults":[{"name":"Personal","items":[ITEM,...]}]}
//! ```
//!
//! The writer ([`crate::session::UnlockedSession::write_export`]) and the
//! importer (`mosaic_import::mosaic_export`) share the item types below, so
//! the two cannot drift apart. Every text that is user content is a
//! [`SecretText`]: it zeroizes on drop and prints as `[REDACTED]`.
//!
//! One [`ExportItem`] holds what the item shows to the user, and nothing
//! that identifies it in an account: no item, vault, device or account ID
//! and no version vector. Members:
//!
//! - `type`: the item type identifier (`login`, `passkey`, `totp`, `note`,
//!   `card`, `identity`, `ssh_key`, `api_key`, `wifi`, `document`).
//! - `title`, `notes`, `favorite`, `createdAt` (Unix ms), `tags`.
//! - `urls`: the login URLs, as text. The match mode and the HTTP allowance
//!   of a URL are not exported; an import uses the most conservative one
//!   (exact origin, HTTPS only).
//! - `fields`: every built-in text or secret field except title and notes
//!   (`key` is the field's document key such as `login.username`; `label`
//!   repeats it) and every custom field (`key` is `custom.<kind>` with
//!   `<kind>` one of `text`, `hidden`, `url`, `email`, `phone`, `date`,
//!   `otp`; `label` is the user's label). `secret` is true for secret
//!   built-in fields, hidden custom fields and OTP custom fields. A custom
//!   OTP field holds an `otpauth://` URI.
//! - `otp`: the item's one-time password as an `otpauth://` URI (the HOTP
//!   counter is the URI's `counter`), or null.
//! - `passkey`: one CXF `passkey` credential ([`ExportPasskey`]) or null.
//!   Only ES256 passkeys are exported (the one algorithm CXF and the
//!   importers carry); another algorithm leaves the member null and is
//!   counted in the report.
//! - `sshKey`: the OpenSSH private key, with `sshPublicKey` and
//!   `sshFingerprint` next to it (members this version adds so that an
//!   import restores the whole key material).
//! - `history`: earlier passwords, oldest first, `{"value","changedAt"}`.
//! - `attachments`: `{"name","mime","size","data"}`; `data` is unpadded
//!   base64url, and null unless the export was made with attachments.
//!
//! Items in the trash and deleted items are not exported. Readers ignore
//! members they do not know, so a later version can add some.

use core::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

/// The `format` member of the plaintext.
pub const FORMAT: &str = "mosaic-export";
/// The `v` member of the plaintext.
pub const VERSION: u32 = 1;
/// The key prefix of a custom field in [`ExportField::key`].
pub const CUSTOM_PREFIX: &str = "custom.";

/// A text that zeroizes on drop and never prints. It serializes as a plain
/// JSON string.
#[derive(Default)]
pub struct SecretText(Zeroizing<String>);

impl SecretText {
    /// A copy of `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(Zeroizing::new(text.to_owned()))
    }

    /// Takes over a zeroizing string without copying it.
    pub(crate) const fn from_zeroizing(text: Zeroizing<String>) -> Self {
        Self(text)
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for SecretText {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretText {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = SecretText;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<SecretText, E> {
                Ok(SecretText::new(v))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<SecretText, E> {
                Ok(SecretText(Zeroizing::new(v)))
            }
        }
        d.deserialize_string(V)
    }
}

/// One field of an item (see the [module documentation](self)).
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportField {
    /// The document key of a built-in field, or `custom.<kind>`.
    pub key: String,
    /// The label (the key itself for a built-in field).
    #[serde(default)]
    pub label: SecretText,
    /// The value.
    #[serde(default)]
    pub value: SecretText,
    /// Whether the value is secret.
    #[serde(default)]
    pub secret: bool,
}

/// A passkey in the form of a CXF `passkey` credential, with two members of
/// Mosaic's own (`signCount`, `discoverable`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPasskey {
    /// Always `passkey`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// The credential ID, base64url.
    pub credential_id: SecretText,
    /// The relying party ID.
    pub rp_id: SecretText,
    /// The user name.
    #[serde(default)]
    pub username: SecretText,
    /// The user's display name.
    #[serde(default)]
    pub user_display_name: SecretText,
    /// The user handle, base64url.
    pub user_handle: SecretText,
    /// The private key, PKCS#8 DER, base64url.
    pub key: SecretText,
    /// The signature counter.
    #[serde(default)]
    pub sign_count: u64,
    /// Whether the credential is discoverable.
    #[serde(default = "yes")]
    pub discoverable: bool,
    /// When the credential was created (Unix ms), if known.
    #[serde(default)]
    pub created_at: Option<i64>,
}

const fn yes() -> bool {
    true
}

/// An earlier password.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportHistory {
    /// The old value.
    pub value: SecretText,
    /// When it was replaced (Unix ms).
    #[serde(default)]
    pub changed_at: i64,
}

/// An attachment of an item.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportAttachment {
    /// The file name.
    pub name: SecretText,
    /// The MIME type.
    #[serde(default)]
    pub mime: String,
    /// The size in bytes.
    #[serde(default)]
    pub size: u64,
    /// The contents, unpadded base64url; null if the export has none.
    #[serde(default)]
    pub data: Option<SecretText>,
}

/// One item of the export.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    /// The item type identifier.
    #[serde(rename = "type")]
    pub item_type: String,
    /// The title.
    #[serde(default)]
    pub title: SecretText,
    /// Whether the item is a favorite.
    #[serde(default)]
    pub favorite: bool,
    /// When the item was created (Unix ms).
    #[serde(default)]
    pub created_at: i64,
    /// The tags.
    #[serde(default)]
    pub tags: Vec<SecretText>,
    /// The login URLs.
    #[serde(default)]
    pub urls: Vec<SecretText>,
    /// Built-in and custom fields.
    #[serde(default)]
    pub fields: Vec<ExportField>,
    /// The notes.
    #[serde(default)]
    pub notes: SecretText,
    /// The one-time password as an `otpauth://` URI.
    #[serde(default)]
    pub otp: Option<SecretText>,
    /// The passkey.
    #[serde(default)]
    pub passkey: Option<ExportPasskey>,
    /// The OpenSSH private key.
    #[serde(default)]
    pub ssh_key: Option<SecretText>,
    /// The SSH public key (`authorized_keys` format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_public_key: Option<SecretText>,
    /// The SSH key fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_fingerprint: Option<SecretText>,
    /// Earlier passwords, oldest first.
    #[serde(default)]
    pub history: Vec<ExportHistory>,
    /// The attachments. The writer streams them after the rest of the item,
    /// so this member is never serialized from here; a reader fills it.
    #[serde(default, skip_serializing)]
    pub attachments: Vec<ExportAttachment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_text_is_redacted_and_round_trips() {
        let t = SecretText::new("hunter2");
        assert_eq!(format!("{t:?}"), "[REDACTED]");
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"hunter2\"");
        let back: SecretText = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "hunter2");
    }

    #[test]
    fn unknown_and_missing_members_are_tolerated() {
        let item: ExportItem =
            serde_json::from_str(r#"{"type":"login","title":"x","future":{"a":1}}"#).unwrap();
        assert_eq!(item.item_type, "login");
        assert_eq!(item.title.as_str(), "x");
        assert!(item.fields.is_empty() && item.otp.is_none() && item.passkey.is_none());
        // `attachments` is read but never written from the item.
        let item: ExportItem = serde_json::from_str(
            r#"{"type":"document","attachments":[{"name":"a.txt","mime":"text/plain","size":1,"data":"QQ"}]}"#,
        )
        .unwrap();
        assert_eq!(item.attachments.len(), 1);
        let out = serde_json::to_string(&item).unwrap();
        assert!(!out.contains("attachments"));
        assert!(out.ends_with('}'));
    }
}
