//! The Mosaic browser extension channel: the native messaging host, the wire
//! protocol and the transport to the desktop app (ADR-0035, ADR-0036).
//!
//! Normative protocol: `docs/NATIVE_MESSAGING.md`. Crypto of the channel:
//! `mosaic_core::browser_channel` and CRYPTO_SPEC §15. Mosaic is pre-audit.
//!
//! ```text
//! extension --stdio JSON--> mosaic-native-host --local endpoint--> desktop
//!                                  (pre-auth JSON, then AEAD frames)
//! ```
//!
//! - [`ids`]: pinned extension IDs and the caller check.
//! - [`framing`]: length-prefixed frames, 256 KiB cap.
//! - [`protocol`], [`bodies`]: the envelope, ops, error codes and bodies.
//! - [`preauth`], [`handshake`]: the messages before the session, and the
//!   host's and the desktop's side of the handshake.
//! - [`endpoint`], [`transport`]: where the desktop listens and the OS
//!   transports (Unix socket, Windows named pipe).
//! - [`relay`]: the host's main loop.
//! - [`install`]: registering the host manifests with the browsers.
//!
//! The desktop side (`BrowserService`) is built on [`transport::Listener`],
//! [`handshake::read_client_message`] and [`handshake::server_handshake`].
#![forbid(unsafe_code)]

pub mod bodies;
pub mod endpoint;
pub mod error;
#[cfg(test)]
mod fixtures;
pub mod framing;
pub mod handshake;
pub(crate) mod hex;
pub mod ids;
pub mod install;
pub mod preauth;
pub mod protocol;
pub mod relay;
#[cfg(test)]
pub(crate) mod testutil;
pub mod transport;

pub use error::Error;
pub use hex::{Hex, HexKey};
