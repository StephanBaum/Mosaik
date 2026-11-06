//! Mosaic sync: the protocol that moves a vault's ciphertext between devices
//! through a backend that is not trusted (CRYPTO_SPEC §9 and §10, ADR-0042 to
//! ADR-0046).
//!
//! There is one protocol and two backends: the Mosaic server (`hosted`) and
//! a directory that a file-sync client replicates (`folder`). This crate
//! holds what both share and what a server needs without a session:
//!
//! - [`wire`]: the byte formats (folder segments, account file, blob delete
//!   marker, device info, the checkpoint, the folder cursor), the strict
//!   folder file-name parser, the email normalization and the JSON types of
//!   the hosted protocol. Every decoder is bounded, verifies its MAC in
//!   constant time before it parses, and rejects trailing bytes.
//! - [`transport`]: the [`Transport`](transport::Transport) trait the sync
//!   engine talks to and its error type.
//!
//! - [`engine`]: [`SyncEngine`](engine::SyncEngine), one run of the protocol
//!   over any transport.
//! - [`folder`]: the folder backend ([`FolderTransport`](folder::FolderTransport)).
//! - `testutil` (feature `test-util`): a simulated sync client, a chaos and
//!   a malicious transport and a manual clock.
//!
//! The HTTP transport follows in a later track of M6 (feature `http`).
//!
//! `mosaic-sync` never holds an account key. The keyed operations (MACs,
//! sealed device info) go through [`wire::SyncCrypto`], which the unlocked
//! vault session implements with keys derived inside `mosaic-core`.
#![forbid(unsafe_code)]

pub mod clock;
pub mod engine;
pub mod folder;
#[cfg(feature = "test-util")]
pub mod testutil;
pub mod transport;
pub mod wire;

pub use transport::{Transport, TransportError};
pub use wire::{SyncCrypto, WireError};
