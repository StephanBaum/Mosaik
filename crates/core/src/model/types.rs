//! Typed values stored inside item fields: login URLs, custom fields,
//! attachment references, OTP configurations, passkeys and SSH keys.
//!
//! Each one is a JSON object in the plaintext. Members this build does not
//! know (written by a newer client) are kept in a private `extra` object
//! and written back unchanged, so editing a value here never drops what a
//! newer client added. A value whose known members are missing, mistyped or
//! out of range does not decode; it stays untouched in the item and is
//! listed by [`crate::model::Item::unreadable_fields`].
//!
//! Secret members use the crate's zeroizing secret types.

use core::fmt;

use super::clock::Timestamp;
use super::doc::parse_uuid;
use super::value::{Object, Text, Value};
use crate::attachment::MAX_ATTACHMENT_LEN;
use crate::error::{Error, Result};
use crate::ids::AttachmentId;
use crate::secret::{AttachmentKey, SecretBytes, SecretString, SecretVec};

/// Longest OTP secret in bytes (a 4096-bit key; real ones are 10 to 64).
pub const MAX_OTP_SECRET_LEN: usize = 512;
/// Longest WebAuthn credential ID in bytes (WebAuthn L3 §5.8.3: 1023).
pub const MAX_CREDENTIAL_ID_LEN: usize = 1023;
/// Longest WebAuthn user handle in bytes (WebAuthn L3: 64).
pub const MAX_USER_HANDLE_LEN: usize = 64;

/// Takes typed members out of a copy of an object; what is left over is
/// the `extra` of the typed value.
struct Reader(Object);

impl Reader {
    fn new(v: &Value) -> Result<Self> {
        v.as_object().map(|o| Self(o.dup())).ok_or(Error::Malformed)
    }

    fn opt(&mut self, k: &str) -> Option<Value> {
        self.0.remove(k)
    }

    fn text(&mut self, k: &str) -> Result<Text> {
        match self.0.remove(k) {
            Some(Value::String(t)) => Ok(t),
            _ => Err(Error::Malformed),
        }
    }

    fn opt_text(&mut self, k: &str) -> Result<Option<Text>> {
        match self.0.remove(k) {
            None => Ok(None),
            Some(Value::String(t)) => Ok(Some(t)),
            Some(_) => Err(Error::Malformed),
        }
    }

    fn bool_or(&mut self, k: &str, default: bool) -> Result<bool> {
        match self.0.remove(k) {
            None => Ok(default),
            Some(v) => v.as_bool().ok_or(Error::Malformed),
        }
    }

    fn u64(&mut self, k: &str) -> Result<u64> {
        self.0
            .remove(k)
            .and_then(|v| v.as_u64())
            .ok_or(Error::Malformed)
    }

    fn i64(&mut self, k: &str) -> Result<i64> {
        self.0
            .remove(k)
            .and_then(|v| v.as_i64())
            .ok_or(Error::Malformed)
    }

    fn secret_bytes(&mut self, k: &str) -> Result<SecretVec> {
        self.0.remove(k).ok_or(Error::Malformed)?.to_secret_bytes()
    }

    fn bytes(&mut self, k: &str) -> Result<Vec<u8>> {
        Ok(self.secret_bytes(k)?.expose_secret().to_vec())
    }

    fn secret_text(&mut self, k: &str) -> Result<SecretString> {
        self.text(k)?.to_secret()
    }

    fn done(self) -> Object {
        self.0
    }
}

/// Starts the object of a typed value from its preserved unknown members.
fn writer(extra: &Object) -> Object {
    extra.dup()
}

fn non_empty(t: &Text) -> Result<()> {
    if t.is_empty() {
        Err(Error::Malformed)
    } else {
        Ok(())
    }
}

macro_rules! redacted_debug {
    ($($t:ty),*) => {$(
        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($t), "([REDACTED])"))
            }
        }
    )*};
}

redacted_debug!(
    LoginUrl,
    CustomField,
    CustomFieldValue,
    AttachmentRef,
    OtpConfig,
    PasskeyCredential,
    SshKeyMaterial
);

// ---------------------------------------------------------------------------
// Login URLs
// ---------------------------------------------------------------------------

/// How the extension matches a login URL against a page (brief §4.2).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum UrlMatch {
    /// Scheme, host and port must be equal. The default.
    #[default]
    ExactOrigin,
    /// The registrable domain (eTLD+1) must be equal. Opt-in per URL.
    Etld1,
}

impl UrlMatch {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ExactOrigin => "origin",
            Self::Etld1 => "etld1",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "origin" => Ok(Self::ExactOrigin),
            "etld1" => Ok(Self::Etld1),
            _ => Err(Error::Malformed),
        }
    }
}

/// One URL of a login.
pub struct LoginUrl {
    /// The URL as entered.
    pub url: Text,
    /// How it is matched. Defaults to [`UrlMatch::ExactOrigin`].
    pub match_mode: UrlMatch,
    /// Whether the user allowed filling over plain HTTP for this URL
    /// (brief §4.8). Defaults to `false`.
    pub allow_http: bool,
    extra: Object,
}

impl LoginUrl {
    /// A URL with exact-origin matching and no HTTP override.
    #[must_use]
    pub fn new(url: &str) -> Self {
        Self {
            url: Text::new(url),
            match_mode: UrlMatch::ExactOrigin,
            allow_http: false,
            extra: Object::new(),
        }
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut o = writer(&self.extra);
        o.insert("url", Value::String(self.url.dup()));
        o.insert("match", Value::text(self.match_mode.as_str()));
        o.insert("allow_http", Value::Bool(self.allow_http));
        Value::Object(o)
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let url = r.text("url")?;
        let match_mode = match r.opt_text("match")? {
            None => UrlMatch::ExactOrigin,
            Some(t) => UrlMatch::parse(t.as_str())?,
        };
        let allow_http = r.bool_or("allow_http", false)?;
        Ok(Self {
            url,
            match_mode,
            allow_http,
            extra: r.done(),
        })
    }
}

// ---------------------------------------------------------------------------
// OTP
// ---------------------------------------------------------------------------

/// The HMAC hash of an OTP generator (CRYPTO_SPEC §13).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum OtpAlgorithm {
    /// HMAC-SHA-1, the RFC 6238 default.
    #[default]
    Sha1,
    /// HMAC-SHA-256.
    Sha256,
    /// HMAC-SHA-512.
    Sha512,
}

/// The OTP scheme.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum OtpKind {
    /// Time-based, RFC 6238.
    #[default]
    Totp,
    /// Counter-based, RFC 4226. The counter is a separate field of the
    /// item ([`crate::model::Item::otp_counter`]).
    Hotp,
    /// Steam Guard: TOTP with 5 characters from Steam's alphabet.
    Steam,
}

/// An OTP generator configuration (brief §5, CRYPTO_SPEC §13).
pub struct OtpConfig {
    /// The shared secret (decoded, not base32).
    pub secret: SecretVec,
    /// The HMAC hash.
    pub algorithm: OtpAlgorithm,
    /// Code length: 6 to 8 for TOTP and HOTP, 5 for Steam.
    pub digits: u8,
    /// Time step in seconds, 1 to 300 (ignored for HOTP).
    pub period: u32,
    /// The scheme.
    pub kind: OtpKind,
    /// The issuer, as in an `otpauth://` URI.
    pub issuer: Option<Text>,
    /// The account label, as in an `otpauth://` URI.
    pub label: Option<Text>,
    /// The secret lives in a vault that needs a hardware key to unlock
    /// (CRYPTO_SPEC §7.4). The item holding this configuration is stored
    /// in that vault.
    pub hardware_protected: bool,
    extra: Object,
}

impl OtpConfig {
    /// A TOTP configuration with the RFC 6238 defaults: SHA-1, 6 digits,
    /// 30 s.
    #[must_use]
    pub fn totp(secret: SecretVec) -> Self {
        Self {
            secret,
            algorithm: OtpAlgorithm::Sha1,
            digits: 6,
            period: 30,
            kind: OtpKind::Totp,
            issuer: None,
            label: None,
            hardware_protected: false,
            extra: Object::new(),
        }
    }

    /// Checks the ranges of CRYPTO_SPEC §13.
    ///
    /// # Errors
    /// [`Error::Malformed`] for an empty or oversized secret, a digit count
    /// outside 6 to 8 (exactly 5 for Steam) or a period outside 1 to 300.
    pub fn validate(&self) -> Result<()> {
        let digits_ok = match self.kind {
            OtpKind::Totp | OtpKind::Hotp => (6..=8).contains(&self.digits),
            OtpKind::Steam => self.digits == 5,
        };
        if self.secret.is_empty()
            || self.secret.len() > MAX_OTP_SECRET_LEN
            || !digits_ok
            || !(1..=300).contains(&self.period)
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut o = writer(&self.extra);
        o.insert("secret", Value::bytes(self.secret.expose_secret()));
        o.insert(
            "algorithm",
            Value::text(match self.algorithm {
                OtpAlgorithm::Sha1 => "sha1",
                OtpAlgorithm::Sha256 => "sha256",
                OtpAlgorithm::Sha512 => "sha512",
            }),
        );
        o.insert("digits", Value::uint(u64::from(self.digits)));
        o.insert("period", Value::uint(u64::from(self.period)));
        o.insert(
            "kind",
            Value::text(match self.kind {
                OtpKind::Totp => "totp",
                OtpKind::Hotp => "hotp",
                OtpKind::Steam => "steam",
            }),
        );
        if let Some(i) = &self.issuer {
            o.insert("issuer", Value::String(i.dup()));
        }
        if let Some(l) = &self.label {
            o.insert("label", Value::String(l.dup()));
        }
        o.insert("hardware_protected", Value::Bool(self.hardware_protected));
        Value::Object(o)
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let secret = r.secret_bytes("secret")?;
        let algorithm = match r.text("algorithm")?.as_str() {
            "sha1" => OtpAlgorithm::Sha1,
            "sha256" => OtpAlgorithm::Sha256,
            "sha512" => OtpAlgorithm::Sha512,
            _ => return Err(Error::Malformed),
        };
        let digits = u8::try_from(r.u64("digits")?).map_err(|_| Error::Malformed)?;
        let period = u32::try_from(r.u64("period")?).map_err(|_| Error::Malformed)?;
        let kind = match r.text("kind")?.as_str() {
            "totp" => OtpKind::Totp,
            "hotp" => OtpKind::Hotp,
            "steam" => OtpKind::Steam,
            _ => return Err(Error::Malformed),
        };
        let issuer = r.opt_text("issuer")?;
        let label = r.opt_text("label")?;
        let hardware_protected = r.bool_or("hardware_protected", false)?;
        let c = Self {
            secret,
            algorithm,
            digits,
            period,
            kind,
            issuer,
            label,
            hardware_protected,
            extra: r.done(),
        };
        c.validate()?;
        Ok(c)
    }
}

// ---------------------------------------------------------------------------
// Custom fields
// ---------------------------------------------------------------------------

/// The value of a custom field, by kind.
pub enum CustomFieldValue {
    /// Plain text.
    Text(Text),
    /// A secret, hidden by default in the UI.
    Hidden(SecretString),
    /// A URL (never used for autofill matching).
    Url(Text),
    /// An email address.
    Email(Text),
    /// A phone number.
    Phone(Text),
    /// A date, ISO 8601 `YYYY-MM-DD`.
    Date(Text),
    /// An OTP generator.
    Otp(OtpConfig),
}

/// A user-defined field.
pub struct CustomField {
    /// The label shown next to the value.
    pub label: Text,
    /// The value.
    pub value: CustomFieldValue,
    extra: Object,
}

impl CustomField {
    /// A custom field.
    #[must_use]
    pub const fn new(label: Text, value: CustomFieldValue) -> Self {
        Self {
            label,
            value,
            extra: Object::new(),
        }
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut o = writer(&self.extra);
        o.insert("label", Value::String(self.label.dup()));
        let (kind, value) = match &self.value {
            CustomFieldValue::Text(t) => ("text", Value::String(t.dup())),
            CustomFieldValue::Hidden(s) => ("hidden", Value::text(s.expose_secret())),
            CustomFieldValue::Url(t) => ("url", Value::String(t.dup())),
            CustomFieldValue::Email(t) => ("email", Value::String(t.dup())),
            CustomFieldValue::Phone(t) => ("phone", Value::String(t.dup())),
            CustomFieldValue::Date(t) => ("date", Value::String(t.dup())),
            CustomFieldValue::Otp(c) => ("otp", c.to_value()),
        };
        o.insert("kind", Value::text(kind));
        o.insert("value", value);
        Value::Object(o)
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let label = r.text("label")?;
        let kind = r.text("kind")?;
        let value = if kind.as_str() == "otp" {
            let raw = r.opt("value").ok_or(Error::Malformed)?;
            CustomFieldValue::Otp(OtpConfig::from_value(&raw)?)
        } else {
            let t = r.text("value")?;
            match kind.as_str() {
                "text" => CustomFieldValue::Text(t),
                "hidden" => CustomFieldValue::Hidden(t.to_secret()?),
                "url" => CustomFieldValue::Url(t),
                "email" => CustomFieldValue::Email(t),
                "phone" => CustomFieldValue::Phone(t),
                "date" => CustomFieldValue::Date(t),
                _ => return Err(Error::Malformed),
            }
        };
        Ok(Self {
            label,
            value,
            extra: r.done(),
        })
    }
}

// ---------------------------------------------------------------------------
// Attachments
// ---------------------------------------------------------------------------

/// A reference to an encrypted attachment (CRYPTO_SPEC §7.3). The file key,
/// name, type and size live only here, inside the item plaintext.
pub struct AttachmentRef {
    /// The attachment's ID (its AAD context).
    pub attachment_id: AttachmentId,
    /// The original file name.
    pub file_name: Text,
    /// The MIME type.
    pub mime: Text,
    /// The plaintext size in bytes, at most 50 MiB.
    pub size: u64,
    /// The file key `FK`.
    pub key: AttachmentKey,
    extra: Object,
}

impl AttachmentRef {
    /// An attachment reference.
    #[must_use]
    pub const fn new(
        attachment_id: AttachmentId,
        file_name: Text,
        mime: Text,
        size: u64,
        key: AttachmentKey,
    ) -> Self {
        Self {
            attachment_id,
            file_name,
            mime,
            size,
            key,
            extra: Object::new(),
        }
    }

    fn check_size(size: u64) -> Result<()> {
        let max = u64::try_from(MAX_ATTACHMENT_LEN).map_err(|_| Error::SizeLimit)?;
        if size > max {
            return Err(Error::SizeLimit);
        }
        Ok(())
    }

    pub(crate) fn to_value(&self) -> Result<Value> {
        Self::check_size(self.size)?;
        let mut o = writer(&self.extra);
        o.insert("id", Value::text(&self.attachment_id.to_string()));
        o.insert("name", Value::String(self.file_name.dup()));
        o.insert("mime", Value::String(self.mime.dup()));
        o.insert("size", Value::uint(self.size));
        o.insert("key", Value::bytes(self.key.expose_secret()));
        Ok(Value::Object(o))
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let attachment_id = AttachmentId::from_bytes(parse_uuid(r.text("id")?.as_str())?)
            .map_err(|_| Error::Malformed)?;
        let file_name = r.text("name")?;
        let mime = r.text("mime")?;
        let size = r.u64("size")?;
        Self::check_size(size).map_err(|_| Error::Malformed)?;
        let key_bytes = r.secret_bytes("key")?;
        let key = AttachmentKey::from_secret(
            SecretBytes::from_slice(key_bytes.expose_secret()).map_err(|_| Error::Malformed)?,
        );
        Ok(Self {
            attachment_id,
            file_name,
            mime,
            size,
            key,
            extra: r.done(),
        })
    }
}

// ---------------------------------------------------------------------------
// Passkeys
// ---------------------------------------------------------------------------

/// A WebAuthn credential (brief §5 "Passkeys"). The signature counter is a
/// separate field of the item ([`crate::model::Item::passkey_sign_count`]),
/// merged by taking the maximum.
pub struct PasskeyCredential {
    /// The credential ID, 1 to 1023 bytes.
    pub credential_id: Vec<u8>,
    /// The relying party ID (a domain).
    pub rp_id: Text,
    /// The user handle, 1 to 64 bytes.
    pub user_handle: Vec<u8>,
    /// The user name shown by the relying party.
    pub user_name: Text,
    /// The user's display name.
    pub user_display_name: Text,
    /// The private key (PKCS#8 DER for `ES256`, the 32-byte seed for `EdDSA`).
    pub private_key: SecretVec,
    /// The COSE algorithm identifier (-7 `ES256`, -8 `EdDSA`).
    pub algorithm: i64,
    /// Whether this is a discoverable (resident) credential.
    pub discoverable: bool,
    /// When the credential was created.
    pub created_at: Timestamp,
    extra: Object,
}

impl PasskeyCredential {
    /// A credential; `user_name` and `user_display_name` start empty and
    /// `discoverable` starts `true`.
    #[must_use]
    pub fn new(
        credential_id: Vec<u8>,
        rp_id: Text,
        user_handle: Vec<u8>,
        private_key: SecretVec,
        algorithm: i64,
        created_at: Timestamp,
    ) -> Self {
        Self {
            credential_id,
            rp_id,
            user_handle,
            user_name: Text::new(""),
            user_display_name: Text::new(""),
            private_key,
            algorithm,
            discoverable: true,
            created_at,
            extra: Object::new(),
        }
    }

    /// Checks the WebAuthn length limits.
    ///
    /// # Errors
    /// [`Error::Malformed`] for an empty or oversized credential ID or user
    /// handle, an empty relying party ID or an empty private key.
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_CREDENTIAL_ID_LEN).contains(&self.credential_id.len())
            || !(1..=MAX_USER_HANDLE_LEN).contains(&self.user_handle.len())
            || self.private_key.is_empty()
        {
            return Err(Error::Malformed);
        }
        non_empty(&self.rp_id)
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut o = writer(&self.extra);
        o.insert("credential_id", Value::bytes(&self.credential_id));
        o.insert("rp_id", Value::String(self.rp_id.dup()));
        o.insert("user_handle", Value::bytes(&self.user_handle));
        o.insert("user_name", Value::String(self.user_name.dup()));
        o.insert(
            "user_display_name",
            Value::String(self.user_display_name.dup()),
        );
        o.insert(
            "private_key",
            Value::bytes(self.private_key.expose_secret()),
        );
        o.insert("alg", Value::int(self.algorithm));
        o.insert("discoverable", Value::Bool(self.discoverable));
        o.insert("created_at", Value::int(self.created_at.as_unix_millis()));
        Value::Object(o)
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let c = Self {
            credential_id: r.bytes("credential_id")?,
            rp_id: r.text("rp_id")?,
            user_handle: r.bytes("user_handle")?,
            user_name: r.text("user_name")?,
            user_display_name: r.text("user_display_name")?,
            private_key: r.secret_bytes("private_key")?,
            algorithm: r.i64("alg")?,
            discoverable: r.bool_or("discoverable", true)?,
            created_at: Timestamp::from_unix_millis(r.i64("created_at")?),
            extra: Object::new(),
        };
        c.validate()?;
        Ok(Self {
            extra: r.done(),
            ..c
        })
    }
}

// ---------------------------------------------------------------------------
// SSH keys
// ---------------------------------------------------------------------------

/// SSH key material. Private key, public key and fingerprint change
/// together, so they are one field.
pub struct SshKeyMaterial {
    /// The private key, in OpenSSH format.
    pub private_key: SecretString,
    /// The public key, in `authorized_keys` format.
    pub public_key: Text,
    /// The fingerprint, for example `SHA256:...`.
    pub fingerprint: Text,
    extra: Object,
}

impl SshKeyMaterial {
    /// SSH key material.
    #[must_use]
    pub const fn new(private_key: SecretString, public_key: Text, fingerprint: Text) -> Self {
        Self {
            private_key,
            public_key,
            fingerprint,
            extra: Object::new(),
        }
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut o = writer(&self.extra);
        o.insert("private_key", Value::text(self.private_key.expose_secret()));
        o.insert("public_key", Value::String(self.public_key.dup()));
        o.insert("fingerprint", Value::String(self.fingerprint.dup()));
        Value::Object(o)
    }

    pub(crate) fn from_value(v: &Value) -> Result<Self> {
        let mut r = Reader::new(v)?;
        let private_key = r.secret_text("private_key")?;
        let public_key = r.text("public_key")?;
        let fingerprint = r.text("fingerprint")?;
        if private_key.expose_secret().is_empty() {
            return Err(Error::Malformed);
        }
        Ok(Self {
            private_key,
            public_key,
            fingerprint,
            extra: r.done(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::value::parse_json;

    fn sv(b: &[u8]) -> SecretVec {
        SecretVec::from_slice(b).unwrap()
    }

    #[test]
    fn login_url_defaults_and_extra() {
        let u = LoginUrl::new("https://example.com");
        let v = u.to_value();
        let back = LoginUrl::from_value(&v).unwrap();
        assert_eq!(back.url.as_str(), "https://example.com");
        assert_eq!(back.match_mode, UrlMatch::ExactOrigin);
        assert!(!back.allow_http);
        let newer =
            parse_json(br#"{"url":"http://x","match":"etld1","allow_http":true,"label":"Work"}"#)
                .unwrap();
        let mut u = LoginUrl::from_value(&newer).unwrap();
        assert_eq!(u.match_mode, UrlMatch::Etld1);
        u.allow_http = false;
        let out = u.to_value();
        assert_eq!(
            out.as_object().unwrap().get("label").unwrap().as_str(),
            Some("Work"),
            "unknown member preserved after an edit"
        );
        let minimal = parse_json(br#"{"url":"x"}"#).unwrap();
        assert_eq!(
            LoginUrl::from_value(&minimal).unwrap().match_mode,
            UrlMatch::ExactOrigin
        );
        for bad in [
            &br#"{"url":"x","match":"prefix"}"#[..],
            br#"{"url":1}"#,
            br#"{"url":"x","allow_http":1}"#,
            br#"{"url":"x","match":1}"#,
            b"[]",
        ] {
            let v = parse_json(bad).unwrap();
            assert_eq!(LoginUrl::from_value(&v).unwrap_err(), Error::Malformed);
        }
        assert!(format!("{u:?}").contains("REDACTED"));
    }

    #[test]
    fn otp_round_trip_and_validation() {
        let mut c = OtpConfig::totp(sv(b"12345678901234567890"));
        c.issuer = Some(Text::new("Example"));
        c.label = Some(Text::new("alice"));
        c.hardware_protected = true;
        for (alg, kind, digits) in [
            (OtpAlgorithm::Sha1, OtpKind::Totp, 6),
            (OtpAlgorithm::Sha256, OtpKind::Hotp, 8),
            (OtpAlgorithm::Sha512, OtpKind::Steam, 5),
        ] {
            c.algorithm = alg;
            c.kind = kind;
            c.digits = digits;
            let back = OtpConfig::from_value(&c.to_value()).unwrap();
            assert_eq!(back.secret, c.secret);
            assert_eq!(back.algorithm, alg);
            assert_eq!(back.kind, kind);
            assert_eq!(back.digits, digits);
            assert_eq!(back.period, 30);
            assert_eq!(back.issuer.unwrap().as_str(), "Example");
            assert_eq!(back.label.unwrap().as_str(), "alice");
            assert!(back.hardware_protected);
        }
        let check = |f: &dyn Fn(&mut OtpConfig)| {
            let mut c = OtpConfig::totp(sv(b"k"));
            f(&mut c);
            c.validate()
        };
        assert!(check(&|_| {}).is_ok());
        assert!(check(&|c| c.digits = 5).is_err());
        assert!(check(&|c| c.digits = 9).is_err());
        assert!(
            check(&|c| {
                c.kind = OtpKind::Steam;
                c.digits = 6;
            })
            .is_err()
        );
        assert!(check(&|c| c.period = 0).is_err());
        assert!(check(&|c| c.period = 301).is_err());
        assert!(check(&|c| c.period = 300).is_ok());
        assert!(check(&|c| c.secret = sv(b"")).is_err());
        assert!(check(&|c| c.secret = sv(&[1; MAX_OTP_SECRET_LEN + 1])).is_err());
        for bad in [
            &br#"{"secret":"AA","algorithm":"md5","digits":6,"period":30,"kind":"totp"}"#[..],
            br#"{"secret":"AA","algorithm":"sha1","digits":6,"period":30,"kind":"motp"}"#,
            br#"{"secret":"AA","algorithm":"sha1","digits":300,"period":30,"kind":"totp"}"#,
            br#"{"secret":"AA","algorithm":"sha1","digits":6,"period":5000000000,"kind":"totp"}"#,
            br#"{"secret":"A","algorithm":"sha1","digits":6,"period":30,"kind":"totp"}"#,
            br#"{"secret":"AA","algorithm":"sha1","digits":6,"period":30,"kind":"totp","issuer":1}"#,
        ] {
            let v = parse_json(bad).unwrap();
            assert_eq!(OtpConfig::from_value(&v).unwrap_err(), Error::Malformed);
        }
    }

    #[test]
    fn custom_field_kinds() {
        let values = [
            CustomFieldValue::Text(Text::new("t")),
            CustomFieldValue::Hidden(Text::new("h").to_secret().unwrap()),
            CustomFieldValue::Url(Text::new("u")),
            CustomFieldValue::Email(Text::new("e")),
            CustomFieldValue::Phone(Text::new("p")),
            CustomFieldValue::Date(Text::new("2026-10-26")),
            CustomFieldValue::Otp(OtpConfig::totp(sv(b"secret"))),
        ];
        for value in values {
            let f = CustomField::new(Text::new("label"), value);
            let back = CustomField::from_value(&f.to_value()).unwrap();
            assert_eq!(back.label.as_str(), "label");
            let same = match (&f.value, &back.value) {
                (CustomFieldValue::Text(a), CustomFieldValue::Text(b))
                | (CustomFieldValue::Url(a), CustomFieldValue::Url(b))
                | (CustomFieldValue::Email(a), CustomFieldValue::Email(b))
                | (CustomFieldValue::Phone(a), CustomFieldValue::Phone(b))
                | (CustomFieldValue::Date(a), CustomFieldValue::Date(b)) => a == b,
                (CustomFieldValue::Hidden(a), CustomFieldValue::Hidden(b)) => {
                    a.expose_secret() == b.expose_secret()
                }
                (CustomFieldValue::Otp(a), CustomFieldValue::Otp(b)) => a.secret == b.secret,
                _ => false,
            };
            assert!(same);
            assert!(format!("{:?}", back.value).contains("REDACTED"));
        }
        for bad in [
            &br#"{"label":"l","kind":"color","value":"x"}"#[..],
            br#"{"label":"l","kind":"otp"}"#,
            br#"{"label":"l","kind":"text","value":1}"#,
            br#"{"kind":"text","value":"x"}"#,
        ] {
            let v = parse_json(bad).unwrap();
            assert_eq!(CustomField::from_value(&v).unwrap_err(), Error::Malformed);
        }
    }

    #[test]
    fn attachment_round_trip_and_limits() {
        let id = AttachmentId::from_bytes([0x0D; 16]).unwrap();
        let key = AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]));
        let a = AttachmentRef::new(id, Text::new("a.pdf"), Text::new("application/pdf"), 9, key);
        let back = AttachmentRef::from_value(&a.to_value().unwrap()).unwrap();
        assert_eq!(back.attachment_id, id);
        assert_eq!(back.key, a.key);
        assert_eq!(back.size, 9);
        assert_eq!(back.file_name.as_str(), "a.pdf");
        assert_eq!(back.mime.as_str(), "application/pdf");
        let key = AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]));
        let max = u64::try_from(MAX_ATTACHMENT_LEN).unwrap();
        let big = AttachmentRef::new(id, Text::new("b"), Text::new("x"), max + 1, key);
        assert_eq!(big.to_value().unwrap_err(), Error::SizeLimit);
        let mut v = a.to_value().unwrap();
        if let Value::Object(o) = &mut v {
            o.insert("size", Value::uint(max + 1));
        }
        assert_eq!(AttachmentRef::from_value(&v).unwrap_err(), Error::Malformed);
        for (member, bad) in [
            ("key", Value::bytes(&[1; 31])),
            ("id", Value::text("00000000-0000-0000-0000-000000000000")),
            ("id", Value::text("nope")),
        ] {
            let mut v = a.to_value().unwrap();
            if let Value::Object(o) = &mut v {
                o.insert(member, bad);
            }
            assert_eq!(AttachmentRef::from_value(&v).unwrap_err(), Error::Malformed);
        }
    }

    #[test]
    fn passkey_round_trip_and_validation() {
        let mut p = PasskeyCredential::new(
            vec![1, 2, 3],
            Text::new("example.com"),
            vec![9; 16],
            sv(&[7; 32]),
            -7,
            Timestamp::from_unix_millis(5),
        );
        p.user_name = Text::new("alice");
        p.user_display_name = Text::new("Alice");
        p.discoverable = false;
        let back = PasskeyCredential::from_value(&p.to_value()).unwrap();
        assert_eq!(back.credential_id, p.credential_id);
        assert_eq!(back.user_handle, p.user_handle);
        assert_eq!(back.private_key, p.private_key);
        assert_eq!(back.algorithm, -7);
        assert!(!back.discoverable);
        assert_eq!(back.created_at, Timestamp::from_unix_millis(5));
        assert_eq!(back.user_name.as_str(), "alice");
        assert_eq!(back.user_display_name.as_str(), "Alice");
        assert_eq!(back.rp_id.as_str(), "example.com");
        let mut bad = PasskeyCredential::from_value(&p.to_value()).unwrap();
        bad.credential_id = vec![];
        assert!(bad.validate().is_err());
        bad.credential_id = vec![0; MAX_CREDENTIAL_ID_LEN + 1];
        assert!(bad.validate().is_err());
        bad.credential_id = vec![1];
        bad.user_handle = vec![0; MAX_USER_HANDLE_LEN + 1];
        assert!(bad.validate().is_err());
        bad.user_handle = vec![1];
        bad.rp_id = Text::new("");
        assert!(bad.validate().is_err());
        bad.rp_id = Text::new("x");
        bad.private_key = sv(b"");
        assert!(bad.validate().is_err());
        assert!(PasskeyCredential::from_value(&bad.to_value()).is_err());
    }

    #[test]
    fn ssh_round_trip() {
        let k = SshKeyMaterial::new(
            Text::new("-----BEGIN OPENSSH PRIVATE KEY-----")
                .to_secret()
                .unwrap(),
            Text::new("ssh-ed25519 AAAA"),
            Text::new("SHA256:abc"),
        );
        let back = SshKeyMaterial::from_value(&k.to_value()).unwrap();
        assert_eq!(
            back.private_key.expose_secret(),
            k.private_key.expose_secret()
        );
        assert_eq!(back.public_key.as_str(), "ssh-ed25519 AAAA");
        assert_eq!(back.fingerprint.as_str(), "SHA256:abc");
        let empty = parse_json(br#"{"private_key":"","public_key":"","fingerprint":""}"#).unwrap();
        assert_eq!(
            SshKeyMaterial::from_value(&empty).unwrap_err(),
            Error::Malformed
        );
    }
}
