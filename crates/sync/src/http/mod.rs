//! The hosted backend over HTTPS (M6 T4, ADR-0044, ADR-0046).
//!
//! - [`HttpTransport`]: the [`Transport`](crate::Transport) of the Mosaic
//!   server, blocking (`reqwest` over rustls, TLS 1.3 only, the operating
//!   system's trust store);
//! - [`hosted`]: the account functions over OPAQUE (register, login, the
//!   second factor, join from a new device, master password change);
//! - [`canonical_origin`]: the server URL rule (`https`, or `http` to a
//!   loopback address only).
//!
//! Every response is parsed strictly and read only up to a size cap. The
//! session token never appears in an error or a log.

mod backend;
mod client;
pub mod hosted;

pub use backend::HttpTransport;
pub use client::{StoredToken, Token, canonical_origin};
