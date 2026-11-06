//! ES256 software WebAuthn authenticator for Mosaic-managed passkeys
//! (CRYPTO_SPEC §16, ADR-0039).
//!
//! The crate does the cryptography and the byte encodings and nothing else:
//!
//! - [`Es256Key`]: a P-256 signing key (RFC 6979 deterministic ECDSA over
//!   SHA-256), stored as PKCS#8 and exported as a COSE key and an SPKI.
//! - The encoders: `clientDataJSON`, `authenticatorData`, the `none`
//!   attestation object, base64url.
//! - [`create`] and [`assert`]: one registration or one assertion. An assertion
//!   is a [`PendingAssertion`]; it releases its signature only after the new
//!   sign count is stored.
//!
//! It does no origin or relying-party-ID policy: callers pass an rpId already
//! validated by `mosaic_origin::webauthn_rp_id` (ADR-0040, lands with M5 T3), and the `origin`
//! string of the browser. It holds no vault and does no I/O. Private keys live
//! in zeroizing types and appear in no error.
#![forbid(unsafe_code)]

mod ceremony;
mod encode;
mod error;
mod key;
mod name;
#[cfg(test)]
mod tests;

pub use ceremony::{
    AssertionResponse, CountPersisted, CreateInput, Created, GetInput, PendingAssertion, assert,
    create, select_algorithm,
};
pub use encode::{
    attestation_object_none, b64url, ccd_to_string, client_data_json, from_b64url, rp_id_hash,
};
pub use error::Error;
pub use key::{Es256Key, is_pkcs8_private_key, new_credential_id};
pub use mosaic_core::model::{MAX_CREDENTIAL_ID_LEN, MAX_USER_HANDLE_LEN};
pub use name::{MAX_ACCOUNT_NAME_BYTES, sanitize_name};

/// The COSE algorithm identifier of ES256 (ECDSA P-256 with SHA-256).
pub const COSE_ALG_ES256: i64 = -7;

/// The authenticator attestation GUID: 16 zero bytes, so credentials do not
/// identify Mosaic users (ADR-0039).
pub const AAGUID: [u8; 16] = [0; 16];

/// The length of the credential IDs Mosaic creates, in bytes.
pub const CREDENTIAL_ID_LEN: usize = 16;

/// The shortest challenge accepted, in bytes (WebAuthn L3 recommends 16).
pub const MIN_CHALLENGE_LEN: usize = 16;

/// The longest challenge accepted, in bytes.
pub const MAX_CHALLENGE_LEN: usize = 1024;

/// The longest `allowCredentials` / `excludeCredentials` list a caller should
/// accept.
pub const MAX_ALLOW_LIST: usize = 64;

/// The longest relying party ID accepted, in bytes.
pub const MAX_RP_ID_LEN: usize = 253;

/// The size of a `COSE_Key` for an ES256 public key, in bytes.
pub const COSE_KEY_LEN: usize = 77;

/// Authenticator data flag bits (WebAuthn L3 6.1).
pub mod flags {
    /// User present.
    pub const UP: u8 = 0x01;
    /// User verified.
    pub const UV: u8 = 0x04;
    /// Backup eligible.
    pub const BE: u8 = 0x08;
    /// Backup state: the credential is backed up.
    pub const BS: u8 = 0x10;
    /// Attested credential data included.
    pub const AT: u8 = 0x40;
}

/// Which WebAuthn ceremony a `clientDataJSON` belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ceremony {
    /// `navigator.credentials.create()`: `"webauthn.create"`.
    Create,
    /// `navigator.credentials.get()`: `"webauthn.get"`.
    Get,
}
