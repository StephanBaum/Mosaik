//! The JSON bodies of the hosted protocol (M6 spec §6.1, §6.2, ADR-0044).
//!
//! Conventions, enforced on both sides:
//!
//! - UTF-8 JSON with camelCase keys; every key is present (an absent value
//!   is `null`, and a missing key is an error); unknown keys are an error
//!   (`deny_unknown_fields`).
//! - IDs are lowercase hyphenated UUID strings, never nil; bytes are
//!   base64url without padding, strictly canonical (`base64ct`), with a
//!   length bound checked before decoding; times are Unix milliseconds.
//! - A body is parsed only up to a size cap ([`parse`]).
//!
//! Binary bodies (an attachment, a checkpoint upload) are not JSON and are
//! not described here.
#![expect(
    missing_docs,
    reason = "the field names are the JSON keys of the endpoint table in the M6 spec"
)]

use core::fmt;
use core::marker::PhantomData;

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{
    AccountId, Argon2Profile, AttachmentId, InstallId, ItemId, KeyGeneration, SecretString,
    VaultId, WrappedKey,
};
use serde::de::{self, DeserializeOwned, Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// The most a client reads of a JSON response.
pub const MAX_RESPONSE_LEN: usize = 16 * 1024 * 1024;
/// The most a server reads of a JSON request unless its config says less.
pub const DEFAULT_MAX_REQUEST_LEN: usize = 8 * 1024 * 1024;
/// The longest decoded record ciphertext (1 MiB plus the envelope).
pub const MAX_CIPHERTEXT_LEN: usize = 1_048_576 + 64;
/// The longest decoded vault metadata ciphertext.
pub const MAX_META_LEN: usize = 65_600;
/// The longest checkpoint upload in bytes (the raw body of
/// `PUT /v1/checkpoints/{deviceId}`): the longest checkpoint, 4096 entries
/// (CRYPTO_SPEC §10). As base64url inside a JSON body it is
/// [`MAX_CHECKPOINT_JSON_LEN`] characters.
pub const MAX_CHECKPOINT_UPLOAD_LEN: usize = super::checkpoint::MAX_LEN;
/// The longest base64url text of a checkpoint in a JSON body.
pub const MAX_CHECKPOINT_JSON_LEN: usize = encoded_len(MAX_CHECKPOINT_UPLOAD_LEN);
/// The longest OPAQUE message (registration and login) in bytes.
pub const MAX_OPAQUE_LEN: usize = 4096;
/// The protocol version `GET /v1/info` reports.
pub const PROTOCOL_VERSION: u32 = 1;

/// Why a body was refused. No variant carries the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum JsonError {
    /// The body is longer than its cap.
    #[error("body too large")]
    TooLarge,
    /// The body is not the expected JSON (syntax, an unknown or missing
    /// key, a wrong type or an invalid value).
    #[error("invalid body")]
    Invalid,
}

/// Parses a JSON body of at most `max_len` bytes.
///
/// # Errors
/// [`JsonError::TooLarge`], [`JsonError::Invalid`].
pub fn parse<T: DeserializeOwned>(bytes: &[u8], max_len: usize) -> Result<T, JsonError> {
    if bytes.len() > max_len {
        return Err(JsonError::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| JsonError::Invalid)
}

/// Serializes a body.
///
/// # Errors
/// [`JsonError::Invalid`] (cannot happen for the types of this module).
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    serde_json::to_vec(value).map_err(|_| JsonError::Invalid)
}

/// A key that must be present, `null` allowed.
fn required<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

// ---- IDs ---------------------------------------------------------------

/// A UUID type that has a canonical hyphenated string form.
pub trait UuidId: Sized + fmt::Display {
    /// Wraps 16 bytes, `None` for the nil UUID.
    fn from_raw(bytes: [u8; 16]) -> Option<Self>;
}

macro_rules! uuid_id {
    ($($t:ty),*) => {$(
        impl UuidId for $t {
            fn from_raw(bytes: [u8; 16]) -> Option<Self> {
                Self::from_bytes(bytes).ok()
            }
        }
    )*};
}

uuid_id!(AccountId, VaultId, ItemId, AttachmentId, InstallId);

/// `with` module for ID fields: lowercase hyphenated UUID strings.
mod uuid_str {
    use super::{Deserializer, PhantomData, Serializer, UuidId, Visitor, de, fmt};

    pub(super) fn serialize<T: UuidId, S: Serializer>(v: &T, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(v)
    }

    struct V<T>(PhantomData<T>);

    impl<T: UuidId> Visitor<'_> for V<T> {
        type Value = T;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a lowercase hyphenated UUID")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
            parse_uuid(v)
                .and_then(T::from_raw)
                .ok_or_else(|| E::custom("invalid UUID"))
        }
    }

    /// `8-4-4-4-12` lowercase hex digits.
    fn parse_uuid(s: &str) -> Option<[u8; 16]> {
        if s.len() != 36 {
            return None;
        }
        let mut out = [0u8; 16];
        let mut digits = s.bytes().enumerate().filter_map(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                // The separators are checked below.
                None
            } else {
                Some(b)
            }
        });
        if s.bytes()
            .enumerate()
            .any(|(i, b)| matches!(i, 8 | 13 | 18 | 23) != (b == b'-'))
        {
            return None;
        }
        for byte in &mut out {
            let hi = nibble(digits.next()?)?;
            let lo = nibble(digits.next()?)?;
            *byte = (hi << 4) | lo;
        }
        Some(out)
    }

    const fn nibble(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        }
    }

    pub(super) fn deserialize<'de, T: UuidId, D: Deserializer<'de>>(d: D) -> Result<T, D::Error> {
        d.deserialize_str(V(PhantomData))
    }
}

// ---- bytes --------------------------------------------------------------

/// Base64url (no padding) bytes of at most `MAX` decoded bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct B64<const MAX: usize>(pub Vec<u8>);

impl<const MAX: usize> fmt::Debug for B64<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B64({} bytes)", self.0.len())
    }
}

/// The encoded length of `n` bytes in base64url without padding.
const fn encoded_len(n: usize) -> usize {
    n / 3 * 4
        + match n % 3 {
            0 => 0,
            1 => 2,
            _ => 3,
        }
}

fn decode_b64<E: de::Error>(s: &str, max: usize) -> Result<Vec<u8>, E> {
    if s.len() > encoded_len(max) {
        return Err(E::custom("too long"));
    }
    Base64UrlUnpadded::decode_vec(s).map_err(|_| E::custom("invalid base64url"))
}

impl<const MAX: usize> Serialize for B64<MAX> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&Base64UrlUnpadded::encode_string(&self.0))
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for B64<MAX> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<const MAX: usize>;
        impl<const MAX: usize> Visitor<'_> for V<MAX> {
            type Value = B64<MAX>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("canonical base64url without padding")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<B64<MAX>, E> {
                decode_b64(v, MAX).map(B64)
            }
        }
        d.deserialize_str(V::<MAX>)
    }
}

/// Base64url (no padding) bytes of exactly `N` decoded bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct B64Fixed<const N: usize>(pub [u8; N]);

impl<const N: usize> fmt::Debug for B64Fixed<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B64Fixed({N} bytes)")
    }
}

impl<const N: usize> Serialize for B64Fixed<N> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&Base64UrlUnpadded::encode_string(&self.0))
    }
}

impl<'de, const N: usize> Deserialize<'de> for B64Fixed<N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let B64::<N>(bytes) = B64::deserialize(d)?;
        let array: [u8; N] = bytes
            .try_into()
            .map_err(|_| de::Error::custom("wrong length"))?;
        Ok(Self(array))
    }
}

/// A secret string in a JSON body (the TOTP setup secret): zeroized on drop,
/// redacted in `Debug`, no `Clone`, no `PartialEq`. It is read straight into
/// a [`SecretString`]; only the server serializes it (to send it once).
pub struct SecretText(SecretString);

impl SecretText {
    /// Wraps a secret string.
    #[must_use]
    pub const fn new(secret: SecretString) -> Self {
        Self(secret)
    }

    /// Borrows the secret. Keep the borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl From<&str> for SecretText {
    /// Copies the text into a zeroizing buffer (a failed allocation yields
    /// the empty secret).
    fn from(v: &str) -> Self {
        Self(SecretString::from_str_copy(v).unwrap_or_else(|_| SecretString::from(String::new())))
    }
}

impl From<String> for SecretText {
    /// Takes the allocation over: it is zeroized with the secret.
    fn from(v: String) -> Self {
        Self(SecretString::from(v))
    }
}

/// Equality without an early exit (for tests and the fuzz targets).
impl PartialEq for SecretText {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (
            self.0.expose_secret().as_bytes(),
            other.0.expose_secret().as_bytes(),
        );
        let mut diff = a.len() ^ b.len();
        for (x, y) in a.iter().zip(b) {
            diff |= usize::from(x ^ y);
        }
        diff == 0
    }
}

impl Eq for SecretText {}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for SecretText {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0.expose_secret())
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
                SecretString::from_str_copy(v)
                    .map(SecretText)
                    .map_err(|_| E::custom("allocation failed"))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<SecretText, E> {
                // Takes the allocation over: it is zeroized with the secret.
                Ok(SecretText(SecretString::from(v)))
            }
        }
        d.deserialize_string(V)
    }
}

/// An OPAQUE protocol message.
pub type OpaqueBytes = B64<MAX_OPAQUE_LEN>;
/// A record ciphertext.
pub type CiphertextBytes = B64<MAX_CIPHERTEXT_LEN>;
/// A vault metadata ciphertext.
pub type MetaBytes = B64<MAX_META_LEN>;
/// A checkpoint in its wire format.
pub type CheckpointBytes = B64<MAX_CHECKPOINT_UPLOAD_LEN>;

/// `with` module for a wrapped key (74 bytes).
mod wrapped_key {
    use super::{B64Fixed, Deserialize, Deserializer, Serialize, Serializer, WrappedKey, de};

    pub(super) fn serialize<S: Serializer>(v: &WrappedKey, s: S) -> Result<S::Ok, S::Error> {
        B64Fixed(*v.as_bytes()).serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<WrappedKey, D::Error> {
        let B64Fixed::<{ WrappedKey::LEN }>(bytes) = B64Fixed::deserialize(d)?;
        WrappedKey::from_bytes(&bytes).map_err(|_| de::Error::custom("invalid wrapped key"))
    }
}

/// `with` module for an Argon2id profile (1 to 4).
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's `with` serializers take the field by reference"
)]
mod profile {
    use super::{Argon2Profile, Deserialize, Deserializer, Serialize, Serializer, de};

    pub(super) fn serialize<S: Serializer>(v: &Argon2Profile, s: S) -> Result<S::Ok, S::Error> {
        v.id().serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Argon2Profile, D::Error> {
        Argon2Profile::from_id(u8::deserialize(d)?)
            .map_err(|_| de::Error::custom("unknown profile"))
    }
}

/// `with` module for a key generation (at least 1).
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's `with` serializers take the field by reference"
)]
mod generation {
    use super::{Deserialize, Deserializer, KeyGeneration, Serialize, Serializer, de};

    pub(super) fn serialize<S: Serializer>(v: &KeyGeneration, s: S) -> Result<S::Ok, S::Error> {
        v.get().serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<KeyGeneration, D::Error> {
        KeyGeneration::new(u32::deserialize(d)?)
            .map_err(|_| de::Error::custom("invalid generation"))
    }
}

// ---- enumerations ---------------------------------------------------------

/// Who may register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationMode {
    Open,
    Invite,
    Closed,
}

/// The kind of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordKind {
    Item,
    Tombstone,
}

/// The state a login ended in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "twoFactorRequired")]
    TwoFactorRequired,
}

// ---- info and accounts ------------------------------------------------------

/// `GET /v1/info`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InfoResponse {
    pub protocol: u32,
    pub server_identity: String,
    pub registration: RegistrationMode,
    pub limits: Limits,
}

/// The server's limits, in `GET /v1/info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    pub max_request_bytes: u64,
    pub max_attachment_bytes: u64,
    pub account_attachment_bytes: u64,
    pub account_record_bytes: u64,
}

/// A response or request without fields (`{}`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// `POST /v1/prelogin` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreloginRequest {
    pub email: String,
}

/// `POST /v1/prelogin` response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreloginResponse {
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub salt: B64Fixed<16>,
    #[serde(with = "profile")]
    pub profile: Argon2Profile,
}

/// `POST /v1/register/start` and `POST /v1/login/start` share this shape of
/// their OPAQUE request: see [`RegisterStartRequest`] and [`LoginStartRequest`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterStartRequest {
    pub email: String,
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub registration_request: OpaqueBytes,
}

/// The response of `register/start` and `account/password/start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistrationStartResponse {
    pub registration_response: OpaqueBytes,
}

/// `POST /v1/register/finish`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterFinishRequest {
    pub email: String,
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub salt: B64Fixed<16>,
    #[serde(with = "profile")]
    pub profile: Argon2Profile,
    pub registration_upload: OpaqueBytes,
    #[serde(with = "wrapped_key")]
    pub wrapped_ak: WrappedKey,
    #[serde(deserialize_with = "required")]
    pub invite_code: Option<String>,
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub device_name: String,
}

/// `POST /v1/login/start` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginStartRequest {
    pub email: String,
    pub credential_request: OpaqueBytes,
}

/// `POST /v1/login/start` response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginStartResponse {
    pub login_id: B64Fixed<16>,
    pub credential_response: OpaqueBytes,
}

/// `POST /v1/login/finish` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginFinishRequest {
    pub login_id: B64Fixed<16>,
    pub credential_finalization: OpaqueBytes,
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub device_name: String,
}

/// `POST /v1/login/finish` and `POST /v1/login/totp` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginResponse {
    pub status: LoginStatus,
    pub expires_at: u64,
}

/// A request that carries one second-factor code: `login/totp`,
/// `account/totp/confirm` and `account/totp/disable`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeRequest {
    /// The code: a zeroizing secret, redacted in `Debug`, not `Clone`.
    pub code: SecretText,
}

/// `DELETE /v1/account`: the code is `null` unless 2FA is on.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteAccountRequest {
    #[serde(deserialize_with = "required")]
    pub code: Option<SecretText>,
}

/// `GET /v1/account`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountResponse {
    #[serde(with = "uuid_str")]
    pub account_id: AccountId,
    pub email: String,
    pub salt: B64Fixed<16>,
    #[serde(with = "profile")]
    pub profile: Argon2Profile,
    #[serde(with = "wrapped_key")]
    pub wrapped_ak: WrappedKey,
    pub account_version: u64,
    pub totp_enabled: bool,
}

/// `POST /v1/account/password/start` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordStartRequest {
    pub registration_request: OpaqueBytes,
}

/// `POST /v1/account/password/finish` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordFinishRequest {
    pub registration_upload: OpaqueBytes,
    #[serde(with = "wrapped_key")]
    pub wrapped_ak: WrappedKey,
    pub salt: B64Fixed<16>,
    #[serde(with = "profile")]
    pub profile: Argon2Profile,
    pub expected_account_version: u64,
}

/// `POST /v1/account/password/finish` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PasswordFinishResponse {
    pub account_version: u64,
}

/// `POST /v1/account/totp/setup` response. The secret and the URI (which
/// holds the secret) are secret strings: redacted `Debug`, no `Clone`, no
/// `PartialEq`, zeroized on drop. `Serialize` is for the server, which sends
/// the response once.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TotpSetupResponse {
    pub secret: SecretText,
    pub otpauth_uri: SecretText,
}

// ---- devices -------------------------------------------------------------------

/// One entry of `GET /v1/devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceEntry {
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub name: String,
    pub created_at: u64,
    pub last_seen_at: u64,
    pub current: bool,
}

/// `GET /v1/devices`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DevicesResponse {
    pub devices: Vec<DeviceEntry>,
}

/// `PATCH /v1/devices/{deviceId}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameDeviceRequest {
    pub name: String,
}

// ---- vaults and records ------------------------------------------------------------

/// One entry of `GET /v1/vaults`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEntry {
    #[serde(with = "uuid_str")]
    pub vault_id: VaultId,
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    pub head_seq: u64,
}

/// `GET /v1/vaults`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultsResponse {
    pub vaults: Vec<VaultEntry>,
}

/// `PUT /v1/vaults/{vaultId}` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVaultRequest {
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    #[serde(with = "wrapped_key")]
    pub wrapped_key: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub meta: MetaBytes,
}

/// The response of `PUT /v1/vaults/{vaultId}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeadResponse {
    pub head_seq: u64,
}

/// A wrapped key in a changes page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeKey {
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    #[serde(with = "wrapped_key")]
    pub wrapped_key: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub server_seq: u64,
}

/// The vault metadata in a changes page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeMeta {
    pub ciphertext: MetaBytes,
    pub server_seq: u64,
}

/// A record in a changes page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeRecord {
    #[serde(with = "uuid_str")]
    pub record_id: ItemId,
    pub kind: RecordKind,
    pub ciphertext: CiphertextBytes,
    pub server_seq: u64,
}

/// `GET /v1/vaults/{v}/changes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangesResponse {
    pub head_seq: u64,
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    pub keys: Vec<ChangeKey>,
    #[serde(deserialize_with = "required")]
    pub meta: Option<ChangeMeta>,
    pub records: Vec<ChangeRecord>,
    pub more: bool,
    pub next_since: u64,
}

/// A record in an upload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PushRecord {
    #[serde(with = "uuid_str")]
    pub record_id: ItemId,
    pub kind: RecordKind,
    pub ciphertext: CiphertextBytes,
    #[serde(deserialize_with = "required")]
    pub base_seq: Option<u64>,
}

/// `POST /v1/vaults/{v}/records` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PushRecordsRequest {
    #[serde(with = "generation")]
    pub generation: KeyGeneration,
    pub records: Vec<PushRecord>,
}

/// The result of one record of an upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RecordResult {
    Accepted {
        #[serde(with = "uuid_str")]
        record_id: ItemId,
        server_seq: u64,
    },
    Conflict {
        #[serde(with = "uuid_str")]
        record_id: ItemId,
        current_seq: u64,
    },
}

/// `POST /v1/vaults/{v}/records` response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PushRecordsResponse {
    pub results: Vec<RecordResult>,
}

/// `PUT /v1/vaults/{v}/meta` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PushMetaRequest {
    pub ciphertext: MetaBytes,
    #[serde(deserialize_with = "required")]
    pub base_seq: Option<u64>,
}

/// `PUT /v1/vaults/{v}/meta` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MetaResult {
    Accepted { server_seq: u64 },
    Conflict { current_seq: u64 },
}

/// `PUT /v1/vaults/{v}/keys/{generation}` request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutKeyRequest {
    #[serde(with = "wrapped_key")]
    pub wrapped_key: WrappedKey,
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    #[serde(deserialize_with = "required")]
    pub expected_head_seq: Option<u64>,
}

/// `PUT /v1/vaults/{v}/keys/{generation}` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutKeyResponse {
    pub server_seq: u64,
}

// ---- attachments, checkpoints, poll ----------------------------------------------------

/// One entry of `GET /v1/vaults/{v}/attachments`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentEntry {
    #[serde(with = "uuid_str")]
    pub attachment_id: AttachmentId,
    pub size: u64,
    #[serde(deserialize_with = "required")]
    pub deleted_at: Option<u64>,
}

/// `GET /v1/vaults/{v}/attachments`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentsResponse {
    pub attachments: Vec<AttachmentEntry>,
}

/// `POST /v1/vaults/{v}/attachments/{a}/restore` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreResponse {
    pub restored: bool,
}

/// One entry of `GET /v1/checkpoints`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckpointEntry {
    #[serde(with = "uuid_str")]
    pub device_id: InstallId,
    pub body: CheckpointBytes,
}

/// `GET /v1/checkpoints`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckpointsResponse {
    pub checkpoints: Vec<CheckpointEntry>,
}

/// `GET /v1/poll` response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PollResponse {
    pub change_counter: u64,
}

// ---- errors -------------------------------------------------------------------------------

/// The body of every error response: `{"error":"<code>","detail":<object|null>}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorBody {
    pub error: String,
    #[serde(deserialize_with = "required")]
    pub detail: Option<serde_json::Value>,
}

impl ErrorBody {
    /// The detail as `T`, if there is one and it fits.
    #[must_use]
    pub fn detail_as<T: DeserializeOwned>(&self) -> Option<T> {
        self.detail
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Detail of `generation_mismatch` and `generation_taken`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationDetail {
    pub current_generation: u32,
}

/// Detail of `head_moved`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeadMovedDetail {
    pub head_seq: u64,
}

/// Detail of `account_version_mismatch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountVersionDetail {
    pub account_version: u64,
}

/// Detail of `quota_exceeded`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotaDetail {
    pub kind: String,
}

/// The error codes of the hosted protocol (the `error` field).
pub mod codes {
    pub const INVALID_EMAIL: &str = "invalid_email";
    pub const RATE_LIMITED: &str = "rate_limited";
    pub const REGISTRATION_CLOSED: &str = "registration_closed";
    pub const INVALID_INVITE: &str = "invalid_invite";
    pub const ACCOUNT_EXISTS: &str = "account_exists";
    pub const BAD_REQUEST: &str = "bad_request";
    pub const LOGIN_FAILED: &str = "login_failed";
    pub const DEVICE_REVOKED: &str = "device_revoked";
    pub const TWO_FACTOR_INVALID: &str = "two_factor_invalid";
    pub const UNAUTHORIZED: &str = "unauthorized";
    pub const FRESH_LOGIN_REQUIRED: &str = "fresh_login_required";
    pub const ACCOUNT_VERSION_MISMATCH: &str = "account_version_mismatch";
    pub const NOT_FOUND: &str = "not_found";
    pub const DEVICE_IS_CURRENT: &str = "device_is_current";
    pub const VAULT_EXISTS: &str = "vault_exists";
    pub const FORBIDDEN: &str = "forbidden";
    pub const GENERATION_MISMATCH: &str = "generation_mismatch";
    pub const GENERATION_TAKEN: &str = "generation_taken";
    pub const HEAD_MOVED: &str = "head_moved";
    pub const QUOTA_EXCEEDED: &str = "quota_exceeded";
    pub const TOO_LARGE: &str = "too_large";
    pub const ATTACHMENT_EXISTS: &str = "attachment_exists";
    pub const CHECKPOINT_STALE: &str = "checkpoint_stale";
    pub const TOO_MANY_POLLS: &str = "too_many_polls";
}

#[cfg(test)]
mod tests {
    use mosaic_core::{AccountKey, Kek, UnlockMethod};

    use super::*;

    fn wrapped() -> WrappedKey {
        AccountKey::generate()
            .unwrap()
            .wrap(
                &Kek::generate().unwrap(),
                &AccountId::new_v7().unwrap(),
                &UnlockMethod::Password,
            )
            .unwrap()
    }

    fn json<T: Serialize>(v: &T) -> serde_json::Value {
        serde_json::to_value(v).unwrap()
    }

    #[test]
    fn the_totp_setup_secret_is_redacted_and_round_trips() {
        let response = TotpSetupResponse {
            secret: SecretText::new(SecretString::from("JBSWY3DPEHPK3PXP".to_owned())),
            otpauth_uri: SecretText::new(SecretString::from(
                "otpauth://totp/Mosaic?secret=JBSWY3DPEHPK3PXP".to_owned(),
            )),
        };
        let debug = format!("{response:?}");
        assert!(!debug.contains("JBSWY3DPEHPK3PXP"), "{debug}");
        assert!(!debug.contains("otpauth://"), "{debug}");
        assert!(debug.contains("[REDACTED]"));
        let bytes = to_vec(&response).unwrap();
        let back: TotpSetupResponse = parse(&bytes, MAX_RESPONSE_LEN).unwrap();
        assert_eq!(back.secret.expose_secret(), "JBSWY3DPEHPK3PXP");
        assert!(back.otpauth_uri.expose_secret().starts_with("otpauth://"));
        // Strict like every body: an unknown key, a non-string, a missing key.
        for bad in [
            &br#"{"secret":"a","otpauthUri":"b","x":1}"#[..],
            br#"{"secret":1,"otpauthUri":"b"}"#,
            br#"{"secret":"a"}"#,
        ] {
            assert!(parse::<TotpSetupResponse>(bad, MAX_RESPONSE_LEN).is_err());
        }
    }

    #[test]
    fn the_longest_checkpoint_fits_the_json_cap() {
        use crate::wire::checkpoint::MAX_LEN;
        assert_eq!(MAX_CHECKPOINT_UPLOAD_LEN, MAX_LEN);
        assert_eq!(MAX_LEN, 114_761);
        assert_eq!(MAX_CHECKPOINT_JSON_LEN, 153_015);
        let device = InstallId::new_v7().unwrap();
        let entry = CheckpointEntry {
            device_id: device,
            body: B64(vec![7; MAX_LEN]),
        };
        round_trip(&entry);
        let text = json(&entry);
        assert_eq!(
            text["body"].as_str().unwrap().len(),
            MAX_CHECKPOINT_JSON_LEN
        );
        let over = CheckpointEntry {
            device_id: device,
            body: B64(vec![7; MAX_LEN + 1]),
        };
        assert!(parse::<CheckpointEntry>(&to_vec(&over).unwrap(), MAX_RESPONSE_LEN).is_err());
    }

    fn round_trip<T: Serialize + DeserializeOwned + PartialEq + fmt::Debug>(v: &T) {
        let bytes = to_vec(v).unwrap();
        assert_eq!(&parse::<T>(&bytes, MAX_RESPONSE_LEN).unwrap(), v);
    }

    #[test]
    fn requests_and_responses_round_trip_with_every_key_present() {
        let device = InstallId::new_v7().unwrap();
        let account = AccountId::new_v7().unwrap();
        let register = RegisterFinishRequest {
            email: "a@b.example".to_owned(),
            account_id: account,
            salt: B64Fixed([5; 16]),
            profile: Argon2Profile::Baseline,
            registration_upload: B64(vec![1, 2, 3]),
            wrapped_ak: wrapped(),
            invite_code: None,
            device_id: device,
            device_name: "Laptop".to_owned(),
        };
        round_trip(&register);
        let v = json(&register);
        assert_eq!(v["inviteCode"], serde_json::Value::Null, "null, not absent");
        assert_eq!(v["profile"], 1);
        assert_eq!(v["accountId"], account.to_string());
        assert_eq!(v.as_object().unwrap().len(), 9);

        let changes = ChangesResponse {
            head_seq: 9,
            generation: KeyGeneration::FIRST,
            keys: vec![ChangeKey {
                generation: KeyGeneration::FIRST,
                wrapped_key: wrapped(),
                device_id: device,
                server_seq: 1,
            }],
            meta: None,
            records: vec![ChangeRecord {
                record_id: ItemId::new_v7().unwrap(),
                kind: RecordKind::Tombstone,
                ciphertext: B64(vec![9; 40]),
                server_seq: 8,
            }],
            more: false,
            next_since: 9,
        };
        round_trip(&changes);
        assert_eq!(json(&changes)["meta"], serde_json::Value::Null);
        round_trip(&InfoResponse {
            protocol: PROTOCOL_VERSION,
            server_identity: "https://sync.example".to_owned(),
            registration: RegistrationMode::Invite,
            limits: Limits {
                max_request_bytes: 1,
                max_attachment_bytes: 2,
                account_attachment_bytes: 3,
                account_record_bytes: 4,
            },
        });
        round_trip(&LoginResponse {
            status: LoginStatus::TwoFactorRequired,
            expires_at: 5,
        });
        assert_eq!(
            json(&LoginResponse {
                status: LoginStatus::TwoFactorRequired,
                expires_at: 5
            })["status"],
            "twoFactorRequired"
        );
        round_trip(&PushMetaRequest {
            ciphertext: B64(vec![1]),
            base_seq: Some(4),
        });
        round_trip(&PutKeyRequest {
            wrapped_key: wrapped(),
            device_id: device,
            expected_head_seq: None,
        });
        round_trip(&AttachmentsResponse {
            attachments: vec![AttachmentEntry {
                attachment_id: AttachmentId::new_v7().unwrap(),
                size: 10,
                deleted_at: None,
            }],
        });
        round_trip(&Empty {});
        round_trip(&ErrorBody {
            error: codes::HEAD_MOVED.to_owned(),
            detail: Some(serde_json::json!({"headSeq": 3})),
        });
    }

    #[test]
    fn tagged_results() {
        let id = ItemId::new_v7().unwrap();
        let results = PushRecordsResponse {
            results: vec![
                RecordResult::Accepted {
                    record_id: id,
                    server_seq: 5,
                },
                RecordResult::Conflict {
                    record_id: id,
                    current_seq: 9,
                },
            ],
        };
        round_trip(&results);
        assert_eq!(
            json(&results)["results"][1],
            serde_json::json!({"status": "conflict", "recordId": id.to_string(), "currentSeq": 9})
        );
        round_trip(&MetaResult::Accepted { server_seq: 2 });
        assert_eq!(
            json(&MetaResult::Conflict { current_seq: 3 }),
            serde_json::json!({"status": "conflict", "currentSeq": 3})
        );
        // A variant with the other variant's key, an unknown status and
        // an extra key are all refused.
        for bad in [
            r#"{"status":"accepted","currentSeq":3}"#,
            r#"{"status":"conflict","serverSeq":3}"#,
            r#"{"status":"accepted","serverSeq":3,"extra":1}"#,
            r#"{"status":"nope","serverSeq":3}"#,
            r#"{"serverSeq":3}"#,
            r#"{"status":"accepted"}"#,
        ] {
            assert!(parse::<MetaResult>(bad.as_bytes(), 1000).is_err(), "{bad}");
        }
    }

    #[test]
    fn json_rejects_unknown_and_missing_keys() {
        let ok = r#"{"protocol":1,"serverIdentity":"https://s.example","registration":"open","limits":{"maxRequestBytes":1,"maxAttachmentBytes":2,"accountAttachmentBytes":3,"accountRecordBytes":4}}"#;
        assert!(parse::<InfoResponse>(ok.as_bytes(), MAX_RESPONSE_LEN).is_ok());
        // An unknown key, at the top and nested.
        let unknown = ok.replace(r#""protocol":1,"#, r#""protocol":1,"extra":true,"#);
        assert_eq!(
            parse::<InfoResponse>(unknown.as_bytes(), MAX_RESPONSE_LEN),
            Err(JsonError::Invalid)
        );
        let nested = ok.replace(r#""maxRequestBytes":1,"#, r#""maxRequestBytes":1,"x":1,"#);
        assert!(parse::<InfoResponse>(nested.as_bytes(), MAX_RESPONSE_LEN).is_err());
        // A missing key.
        let missing = ok.replace(r#""protocol":1,"#, "");
        assert!(parse::<InfoResponse>(missing.as_bytes(), MAX_RESPONSE_LEN).is_err());
        // A nullable key must be present: absent is an error, null is not.
        let with_null = r#"{"error":"not_found","detail":null}"#;
        assert!(parse::<ErrorBody>(with_null.as_bytes(), 1000).is_ok());
        assert!(parse::<ErrorBody>(br#"{"error":"not_found"}"#, 1000).is_err());
        assert!(parse::<DeleteAccountRequest>(b"{}", 1000).is_err());
        assert!(parse::<DeleteAccountRequest>(br#"{"code":null}"#, 1000).is_ok());
        // snake_case keys are not accepted.
        assert!(parse::<HeadResponse>(br#"{"head_seq":1}"#, 1000).is_err());
        assert!(parse::<HeadResponse>(br#"{"headSeq":1}"#, 1000).is_ok());
        // Types, trailing data, syntax, and the size cap.
        assert!(parse::<HeadResponse>(br#"{"headSeq":"1"}"#, 1000).is_err());
        assert!(parse::<HeadResponse>(br#"{"headSeq":-1}"#, 1000).is_err());
        assert!(parse::<HeadResponse>(br#"{"headSeq":1} x"#, 1000).is_err());
        assert!(parse::<HeadResponse>(br#"{"headSeq":1,"headSeq":2}"#, 1000).is_err());
        assert!(parse::<HeadResponse>(b"\xff", 1000).is_err());
        assert_eq!(
            parse::<HeadResponse>(br#"{"headSeq":1}"#, 5),
            Err(JsonError::TooLarge)
        );
        assert!(parse::<PreloginRequest>(b"[]", 1000).is_err());
        assert!(parse::<PreloginRequest>(b"null", 1000).is_err());
        // Wrong enum values and profile numbers.
        assert!(parse::<LoginResponse>(br#"{"status":"OK","expiresAt":1}"#, 1000).is_err());
        let prelogin = |profile: &str| {
            format!(
                r#"{{"accountId":"{}","salt":"{}","profile":{profile}}}"#,
                AccountId::new_v7().unwrap(),
                Base64UrlUnpadded::encode_string(&[0u8; 16])
            )
        };
        assert!(parse::<PreloginResponse>(prelogin("1").as_bytes(), 1000).is_ok());
        for bad in ["0", "5", "256", "-1", "1.5", "\"1\"", "null"] {
            assert!(
                parse::<PreloginResponse>(prelogin(bad).as_bytes(), 1000).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn ids_are_strict_uuids() {
        let id = VaultId::new_v7().unwrap().to_string();
        let ok = |s: &str| {
            parse::<VaultEntry>(
                format!(r#"{{"vaultId":"{s}","generation":1,"headSeq":0}}"#).as_bytes(),
                1000,
            )
        };
        assert!(ok(&id).is_ok());
        for bad in [
            id.to_uppercase(),
            id.replace('-', ""),
            format!("{{{id}}}"),
            format!("{id} "),
            id[..35].to_owned(),
            "00000000-0000-0000-0000-000000000000".to_owned(),
            id.replace('-', "_"),
            id.replacen('-', "x", 1),
            format!("{}g", &id[..35]),
            String::new(),
        ] {
            assert!(ok(&bad).is_err(), "{bad}");
        }
        let generation = |g: &str| {
            parse::<VaultEntry>(
                format!(r#"{{"vaultId":"{id}","generation":{g},"headSeq":0}}"#).as_bytes(),
                1000,
            )
        };
        assert!(generation("4294967295").is_ok());
        for bad in ["0", "4294967296", "-1"] {
            assert!(generation(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn base64url_strict() {
        let get = |s: &str| parse::<B64Fixed<4>>(format!("\"{s}\"").as_bytes(), 1000).map(|b| b.0);
        assert_eq!(get("AQIDBA").unwrap(), [1, 2, 3, 4]);
        for bad in [
            "AQIDBA==", // padding
            "AQIDBA=",  // padding
            "AQID+A",   // standard alphabet
            "AQID/A",   // standard alphabet
            "AQIDBB",   // non-canonical trailing bits
            "AQID BA",  // whitespace
            "AQID\nBA",
            "AQIDB",    // impossible length
            "AQID",     // wrong length (3 bytes)
            "AQIDBAUG", // wrong length (6 bytes)
            "",
            "\u{ff21}QIDBA", // full-width letter
        ] {
            assert!(get(bad).is_err(), "{bad:?}");
        }
        assert!(parse::<B64Fixed<4>>(b"1234", 1000).is_err(), "not a string");
        // The length bound is checked before decoding.
        let long = "A".repeat(encoded_len(10) + 1);
        assert!(parse::<B64<10>>(format!("\"{long}\"").as_bytes(), 1000).is_err());
        let exact = "A".repeat(encoded_len(10));
        let decoded = parse::<B64<10>>(format!("\"{exact}\"").as_bytes(), 1000);
        // 14 characters is not a canonical 10-byte encoding ("AA" ends in
        // non-zero bits only for other digits; all-A is canonical).
        assert_eq!(decoded.unwrap().0, vec![0; 10]);
        assert_eq!(encoded_len(0), 0);
        assert_eq!(encoded_len(1), 2);
        assert_eq!(encoded_len(2), 3);
        assert_eq!(encoded_len(3), 4);
        assert_eq!(
            encoded_len(74),
            Base64UrlUnpadded::encode_string(&[0u8; 74]).len()
        );
        // Round trip of every length near the boundaries.
        for n in 0..=12usize {
            let bytes: Vec<u8> = (0..n)
                .map(|i| u8::try_from(i * 37 % 251).unwrap())
                .collect();
            let s = serde_json::to_string(&B64::<64>(bytes.clone())).unwrap();
            assert_eq!(serde_json::from_str::<B64<64>>(&s).unwrap().0, bytes);
            assert_eq!(s.len(), encoded_len(n) + 2);
        }
    }

    #[test]
    fn wrapped_keys_must_be_74_valid_bytes() {
        let w = wrapped();
        let good = serde_json::to_string(&PasswordFinishResponse { account_version: 1 }).unwrap();
        assert!(good.contains("accountVersion"));
        let body = |key: &str| {
            format!(
                r#"{{"wrappedKey":"{key}","deviceId":"{}","expectedHeadSeq":null}}"#,
                InstallId::new_v7().unwrap()
            )
        };
        let enc = Base64UrlUnpadded::encode_string(w.as_bytes());
        assert!(parse::<PutKeyRequest>(body(&enc).as_bytes(), 10_000).is_ok());
        let short = Base64UrlUnpadded::encode_string(&w.as_bytes()[..73]);
        assert!(parse::<PutKeyRequest>(body(&short).as_bytes(), 10_000).is_err());
        let mut bad_header = *w.as_bytes();
        bad_header[0] = 0xEE;
        let bad = Base64UrlUnpadded::encode_string(&bad_header);
        assert!(parse::<PutKeyRequest>(body(&bad).as_bytes(), 10_000).is_err());
    }

    #[test]
    fn error_details_decode_as_their_type() {
        let body: ErrorBody = parse(
            br#"{"error":"generation_taken","detail":{"currentGeneration":4}}"#,
            1000,
        )
        .unwrap();
        assert_eq!(body.error, codes::GENERATION_TAKEN);
        assert_eq!(
            body.detail_as::<GenerationDetail>(),
            Some(GenerationDetail {
                current_generation: 4
            })
        );
        assert_eq!(body.detail_as::<HeadMovedDetail>(), None);
        let none: ErrorBody = parse(br#"{"error":"x","detail":null}"#, 1000).unwrap();
        assert_eq!(none.detail_as::<QuotaDetail>(), None);
    }
}
