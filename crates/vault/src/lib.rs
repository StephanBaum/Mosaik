//! Mosaic vault layer on top of `mosaic-core`.
//!
//! - [`storage`]: the offline-first local SQLite store (CRYPTO_SPEC §11). It
//!   holds only `mosaic-core` ciphertext objects (wrapped keys, items,
//!   tombstones, vault metadata, attachments) and the minimal cleartext
//!   metadata needed to route and sync them. It never sees a key or a
//!   plaintext.
//! - [`session`]: account creation, unlock and lock, vaults, items,
//!   attachments, the in-memory search index and applying received
//!   records, on top of the store and the `mosaic-core` model.
//! - [`otp`]: conversions between the item model's OTP configuration and
//!   `mosaic-totp`.
//!
//! The frozen API for M3 to M6 is described in `docs/CORE_API.md`.
//!
//! Mosaic is pre-audit.
#![forbid(unsafe_code)]

pub mod otp;
pub mod session;
pub mod storage;
