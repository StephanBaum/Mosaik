//! The hosted backend over HTTPS (M6 T4, ADR-0044, ADR-0046).
//!
//! - [`HttpTransport`]: the [`Transport`](crate::Transport) of the Mosaic
//!   server, blocking (`reqwest` over rustls, TLS 1.3 only, the operating
//!   system's trust store);
//! - [`hosted`]: the account functions over OPAQUE (register, login, the
//!   second factor, join from a new device, master password change);
//! - [`pwned`]: the HTTPS client of the opt-in breach check (M7b, ADR-0065);
//! - [`assetlinks`]: the fetch of a site's Digital Asset Links file (M10, ADR-0091);
//! - [`TlsTrust`] and [`set_tls_trust`]: where the TLS trust comes from (the
//!   operating system, or anchors the platform passes; M10, ADR-0091);
//! - [`canonical_origin`]: the server URL rule (`https`, or `http` to a
//!   loopback address only).
//!
//! Every response is parsed strictly and read only up to a size cap. The
//! session token never appears in an error or a log.

pub mod assetlinks;
mod backend;
mod client;
pub mod hosted;
pub mod pwned;
mod sharing;
mod tls;

pub use backend::HttpTransport;
pub use client::{StoredToken, Token, canonical_origin};
pub use rustls::pki_types::CertificateDer;
pub use tls::{MAX_TRUST_ANCHORS, TlsTrust, set_tls_trust};
