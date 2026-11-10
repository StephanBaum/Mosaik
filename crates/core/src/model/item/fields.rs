//! The item type and the field enums, element IDs and history entries.

use core::fmt;

use super::{HISTORY_PREFIX, URL_PREFIX};
use crate::error::{Error, Result};
use crate::model::clock::Timestamp;
use crate::model::doc::check_key;
use crate::model::value::Value;
use crate::secret::SecretString;

/// The kind of an item.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ItemType {
    /// Username, password, URLs, optional OTP.
    Login,
    /// A WebAuthn credential.
    Passkey,
    /// An OTP generator on its own.
    Totp,
    /// Notes only.
    SecureNote,
    /// A payment card.
    CreditCard,
    /// Personal details.
    Identity,
    /// An SSH key pair.
    SshKey,
    /// An API key or token.
    ApiKey,
    /// Wi-Fi credentials.
    Wifi,
    /// Files (attachments) with notes.
    Document,
    /// A type written by a newer client. Shown as unsupported; its fields
    /// are kept and merged as usual.
    Other(String),
}

impl ItemType {
    /// The identifier stored in the `type` field.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Login => "login",
            Self::Passkey => "passkey",
            Self::Totp => "totp",
            Self::SecureNote => "note",
            Self::CreditCard => "card",
            Self::Identity => "identity",
            Self::SshKey => "ssh_key",
            Self::ApiKey => "api_key",
            Self::Wifi => "wifi",
            Self::Document => "document",
            Self::Other(s) => s,
        }
    }

    /// Parses a stored identifier; unknown ones become [`ItemType::Other`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "login" => Self::Login,
            "passkey" => Self::Passkey,
            "totp" => Self::Totp,
            "note" => Self::SecureNote,
            "card" => Self::CreditCard,
            "identity" => Self::Identity,
            "ssh_key" => Self::SshKey,
            "api_key" => Self::ApiKey,
            "wifi" => Self::Wifi,
            "document" => Self::Document,
            other => Self::Other(other.to_owned()),
        }
    }
}

macro_rules! field_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $key:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum $name {
            $($(#[$vmeta])* $variant),*
        }

        impl $name {
            /// Every field, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// The document key of this field.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key),*
                }
            }

            /// The field stored under `key`, if any.
            #[must_use]
            pub fn from_key(key: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|f| f.key() == key)
            }
        }
    };
}

field_enum!(
    /// Plain text fields.
    TextField {
        /// The item's title.
        Title => "title",
        /// Free-form notes (the whole content of a secure note).
        Notes => "notes",
        /// Login user name or email.
        Username => "login.username",
        /// Name on a card.
        CardholderName => "card.cardholder",
        /// Card network or brand.
        CardBrand => "card.brand",
        /// Card expiry, `MM/YYYY`.
        CardExpiry => "card.expiry",
        /// Identity: given name.
        FirstName => "identity.first_name",
        /// Identity: middle name.
        MiddleName => "identity.middle_name",
        /// Identity: family name.
        LastName => "identity.last_name",
        /// Identity: email.
        Email => "identity.email",
        /// Identity: phone.
        Phone => "identity.phone",
        /// Identity: company.
        Company => "identity.company",
        /// Identity: address line 1.
        AddressLine1 => "identity.address1",
        /// Identity: address line 2.
        AddressLine2 => "identity.address2",
        /// Identity: city.
        City => "identity.city",
        /// Identity: state or region.
        Region => "identity.region",
        /// Identity: postal code.
        PostalCode => "identity.postal_code",
        /// Identity: country.
        Country => "identity.country",
        /// Identity: date of birth, `YYYY-MM-DD`.
        BirthDate => "identity.birth_date",
        /// API key: key ID or client ID.
        ApiKeyId => "api_key.key_id",
        /// API key: host or endpoint.
        ApiKeyHost => "api_key.host",
        /// Wi-Fi network name.
        WifiSsid => "wifi.ssid",
        /// Wi-Fi security type (for example `WPA3`).
        WifiSecurity => "wifi.security",
        /// SSH key comment.
        SshComment => "ssh.comment",
        /// Login: the page where the site changes the password (https only,
        /// related to one of the item's URLs; M7b, ADR-0070).
        ChangePasswordUrl => "login.change_password_url",
    }
);

field_enum!(
    /// Secret text fields. They are read into a [`SecretString`].
    SecretField {
        /// Login password.
        Password => "login.password",
        /// Card number.
        CardNumber => "card.number",
        /// Card verification code.
        CardVerificationCode => "card.cvv",
        /// Card PIN.
        CardPin => "card.pin",
        /// Identity: passport number.
        PassportNumber => "identity.passport_number",
        /// Identity: driver's license number.
        LicenseNumber => "identity.license_number",
        /// Identity: national ID or social security number.
        NationalId => "identity.national_id",
        /// API key: the secret or token.
        ApiKeySecret => "api_key.secret",
        /// Wi-Fi password.
        WifiPassword => "wifi.password",
    }
);

impl SecretField {
    /// Whether old values go into the password history when the value
    /// changes or a conflict is resolved.
    #[must_use]
    pub const fn keeps_history(self) -> bool {
        matches!(
            self,
            Self::Password | Self::CardPin | Self::ApiKeySecret | Self::WifiPassword
        )
    }
}

/// The ID of one element of a collection (URL, custom field). Derived from
/// the creating edit's stamp, so it is unique without randomness.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ElementId(pub(super) String);

impl ElementId {
    /// The ID as stored after the key prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps a stored ID (as returned by [`ElementId::as_str`]).
    ///
    /// # Errors
    /// [`Error::Malformed`] if no field key can hold it: longer than
    /// [`crate::model::MAX_FIELD_KEY_LEN`] minus the `url:` prefix, or with control
    /// characters. (The edit methods check the full key again.)
    pub fn from_stored(id: &str) -> Result<Self> {
        let mut key = String::with_capacity(URL_PREFIX.len().saturating_add(id.len()));
        key.push_str(URL_PREFIX);
        key.push_str(id);
        check_key(&key)?;
        Ok(Self(id.to_owned()))
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The ID of one password history entry: the stored key suffix.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HistoryId(pub(super) String);

impl HistoryId {
    /// The ID as stored after the `history:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps a stored ID (as returned by [`HistoryId::as_str`]).
    ///
    /// # Errors
    /// [`Error::Malformed`] if no field key can hold it.
    pub fn from_stored(id: &str) -> Result<Self> {
        let mut key = String::with_capacity(HISTORY_PREFIX.len().saturating_add(id.len()));
        key.push_str(HISTORY_PREFIX);
        key.push_str(id);
        check_key(&key)?;
        Ok(Self(id.to_owned()))
    }
}

impl fmt::Display for HistoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a value is in the password history (CRYPTO_SPEC §7.4).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HistoryReason {
    /// A confirmed change replaced the value (the stored entry has no
    /// `reason` member).
    Replaced,
    /// The value was a new password of a credential rotation attempt that
    /// was never confirmed: the site may still have applied it.
    Unconfirmed,
}

impl HistoryReason {
    /// The stored `reason` member, `None` for [`HistoryReason::Replaced`].
    pub(super) const fn stored(self) -> Option<&'static str> {
        match self {
            Self::Replaced => None,
            Self::Unconfirmed => Some("unconfirmed"),
        }
    }

    fn parse(v: Option<&Value>) -> Self {
        match v.and_then(Value::as_str) {
            Some("unconfirmed") => Self::Unconfirmed,
            // Absent, or a reason of a newer client: shown as a plain change.
            _ => Self::Replaced,
        }
    }
}

/// One password history entry.
pub struct HistoryEntry {
    /// The entry's ID, for [`ItemEdit::restore_history`](super::ItemEdit::restore_history).
    pub id: HistoryId,
    /// The field the value belonged to.
    pub field: SecretField,
    /// The old value.
    pub value: SecretString,
    /// When it was replaced.
    pub changed_at: Timestamp,
    /// Why it is here.
    pub reason: HistoryReason,
}

impl fmt::Debug for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HistoryEntry")
            .field("id", &self.id)
            .field("field", &self.field)
            .field("changed_at", &self.changed_at)
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl HistoryEntry {
    pub(super) fn from_value(id: String, v: &Value) -> Result<Self> {
        let o = v.as_object().ok_or(Error::Malformed)?;
        let field = o
            .get("field")
            .and_then(Value::as_str)
            .and_then(SecretField::from_key)
            .ok_or(Error::Malformed)?;
        let value = match o.get("value") {
            Some(Value::String(t)) => t.to_secret()?,
            _ => return Err(Error::Malformed),
        };
        let changed_at = o
            .get("changed_at")
            .and_then(Value::as_i64)
            .ok_or(Error::Malformed)?;
        Ok(Self {
            id: HistoryId(id),
            field,
            value,
            changed_at: Timestamp::from_unix_millis(changed_at),
            reason: HistoryReason::parse(o.get("reason")),
        })
    }
}
