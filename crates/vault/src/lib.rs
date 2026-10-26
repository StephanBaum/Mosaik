//! Mosaic vault layer on top of `mosaic-core`.
//!
//! - [`storage`]: the offline-first local SQLite store (CRYPTO_SPEC §11). It
//!   holds only `mosaic-core` ciphertext objects (wrapped keys, items,
//!   tombstones, vault metadata, attachments) and the minimal cleartext
//!   metadata needed to route and sync them. It never sees a key or a
//!   plaintext.
//!
//! Mosaic is pre-audit.
#![forbid(unsafe_code)]

pub mod storage;
