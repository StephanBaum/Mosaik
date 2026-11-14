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
//!   [`AttachmentId`], [`DeviceId`], [`InstallId`]) are UUIDs; the nil UUID is rejected.
//! - **Derivation** ([`kdf`]): NFC preparation, Argon2id profiles and
//!   calibration, and the two-secret HKDF derivation of `KEK` and `OPW`.
//! - **Secret Key** display format `M1-XXXX-...` ([`SecretKey::parse`],
//!   [`SecretKey::to_display`]).
//! - **Objects** ([`envelope`], [`attachment`]): wrapped keys, items, vault
//!   metadata, tombstones and attachments, each bound to its context IDs.
//! - **CLI** ([`cli_session`], [`export`]): the short-lived CLI session file
//!   (CRYPTO_SPEC §6.8) and the passphrase-encrypted export stream (§11.2).
//! - **Flows** ([`create_account`], [`unlock_with_password`],
//!   [`change_master_password`]).
//! - **Model** ([`model`]): item types, their plaintext serialization and
//!   the sync merge (version vectors, Lamport stamps, conflict copies).
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
pub mod browser_channel;
pub mod cli_session;
pub mod encoding;
pub mod envelope;
mod error;
pub mod export;
pub mod identity;
mod ids;
pub mod kdf;
mod keys;
pub mod model;
mod rng;
mod secret;
mod secret_key;
mod settings_mac;
pub mod setup_code;
pub mod share;
mod spoofing;
pub mod sync_auth;
pub mod unlock;

#[cfg(test)]
mod golden;

pub use attachment::{MAX_ATTACHMENT_LEN, open_attachment, seal_attachment};
#[cfg(fuzzing)]
#[doc(hidden)]
pub use cli_session::seal_cli_session_for_fuzzing;
pub use cli_session::{
    CliSessionBinding, CliSessionFile, CliSessionKey, open_cli_session, seal_cli_session,
};
pub use envelope::{
    ContactsCiphertext, ItemCiphertext, TombstoneCiphertext, UnlockMethod, VaultMetaCiphertext,
    VaultRosterCiphertext, WrappedKey,
};
pub use error::{Error, Result};
pub use export::{ExportPassphrase, ExportReader, ExportWriter};
pub use identity::{Fingerprint, IdentityPublicKey, IdentitySecret};
pub use ids::{AccountId, AttachmentId, DeviceId, InstallId, ItemId, KeyGeneration, VaultId};
pub use kdf::{AccountSalt, Argon2Profile, DerivedKeys};
pub use keys::{
    CredentialChange, NewAccount, PasswordChange, PasswordParams, RekeyedAccount,
    change_credentials, change_master_password, create_account, derive_password_keys,
    rekey_account, unlock_with_password,
};
pub use model::{
    Causality, Contact, ContactBook, ContactState, Item, ItemEdit, ItemType, MergeOutcome, Stamp,
    Timestamp, Tombstone, VaultMeta, VersionVector,
};
pub use rng::random_public_bytes;
pub use secret::{
    AccountKey, AttachmentKey, CheckpointKey, ContactsKey, DeviceInfoKey, FolderMacKey, Kek,
    MasterPassword, OpaquePassword, SecretBytes, SecretKey, SecretString, SecretVec, SessionToken,
    SettingsMacKey, VaultKey,
};
pub use settings_mac::{SETTINGS_MAC_LEN, SETTINGS_NONCE_LEN, new_settings_nonce};
pub use share::{
    ShareSeal, open_emergency_vault_key, open_vault_key, seal_emergency_vault_key, seal_vault_key,
};
pub use spoofing::{has_spoofing_char, is_spoofing_char, strip_spoofing};
