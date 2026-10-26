//! Mosaic crypto core: key hierarchy, key derivation and encrypted object
//! formats.
//!
//! Normative specification: `docs/CRYPTO_SPEC.md`. Code in this crate must
//! match it. Mosaic is pre-audit.
//!
//! # Overview
//!
//! - **Secrets** ([`SecretBytes`], [`SecretVec`], [`SecretString`],
//!   [`MasterPassword`] and the typed keys [`AccountKey`], [`VaultKey`],
//!   [`Kek`], [`OpaquePassword`], [`AttachmentKey`], [`SecretKey`]) zeroize
//!   on drop, are not `Clone`, and print as `[REDACTED]`.
//! - **Identifiers** ([`AccountId`], [`VaultId`], [`ItemId`],
//!   [`AttachmentId`], [`DeviceId`]) are UUIDs; the nil UUID is rejected.
//! - **Derivation** ([`kdf`]): NFC preparation, Argon2id profiles and
//!   calibration, and the two-secret HKDF derivation of `KEK` and `OPW`.
//! - **Secret Key** display format `M1-XXXX-...` ([`SecretKey::parse`],
//!   [`SecretKey::to_display`]).
//! - **Objects** ([`envelope`], [`attachment`]): wrapped keys, items, vault
//!   metadata, tombstones and attachments, each bound to its context IDs.
//! - **Flows** ([`create_account`], [`unlock_with_password`],
//!   [`change_master_password`]).
//!
//! Every seal draws a fresh random nonce; the API never takes one. Every
//! open takes the context (IDs, generation) from the caller, and every
//! authentication failure is the single error [`Error::Decryption`].
//!
//! ```
//! use mosaic_core::{ItemCiphertext, ItemId, KeyGeneration, VaultId, VaultKey};
//!
//! # fn main() -> Result<(), mosaic_core::Error> {
//! let vault_id = VaultId::new_v7()?;
//! let item_id = ItemId::new_v7()?;
//! let vk = VaultKey::generate(KeyGeneration::FIRST)?;
//! let ct = ItemCiphertext::seal(&vk, &vault_id, &item_id, 1, b"item plaintext")?;
//!
//! let stored = ItemCiphertext::from_bytes(ct.as_bytes())?;
//! let pt = stored.open(&vk, &vault_id, &item_id)?;
//! assert_eq!(pt.expose_secret(), b"item plaintext");
//! # Ok(())
//! # }
//! ```
#![forbid(unsafe_code)]

mod aead;
pub mod attachment;
pub mod encoding;
pub mod envelope;
mod error;
mod ids;
pub mod kdf;
mod keys;
mod rng;
mod secret;
mod secret_key;

#[cfg(test)]
mod golden;

pub use attachment::{MAX_ATTACHMENT_LEN, open_attachment, seal_attachment};
pub use envelope::{
    ItemCiphertext, TombstoneCiphertext, UnlockMethod, VaultMetaCiphertext, WrappedKey,
};
pub use error::{Error, Result};
pub use ids::{AccountId, AttachmentId, DeviceId, ItemId, KeyGeneration, VaultId};
pub use kdf::{AccountSalt, Argon2Profile, DerivedKeys};
pub use keys::{
    NewAccount, PasswordChange, PasswordParams, change_master_password, create_account,
    derive_password_keys, unlock_with_password,
};
pub use secret::{
    AccountKey, AttachmentKey, Kek, MasterPassword, OpaquePassword, SecretBytes, SecretKey,
    SecretString, SecretVec, VaultKey,
};
