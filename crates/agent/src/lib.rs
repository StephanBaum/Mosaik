//! Agent access (ADR-0159, ADR-0160): AI agents use vault secrets through a
//! local MCP relay without the value entering the model's context.
//!
//! The decision logic lives here and is host-testable; the desktop wires it to
//! the vault session, the approval dialogs and the IPC.
//!
//! - [`error`]: the error codes every layer reports (a code and a fixed
//!   sentence, never a value).
//! - [`mask`]: the masking engine for responses and, later, command output.
//! - [`request`]: validation of an agent request against its binding and the single-slot
//!   injection of the value.
//! - [`engine`]: the policy engine (grants, limits, masking of answers, log entries); [`source`]
//!   is the vault side it talks to, [`vault_source`] the implementation over an unlocked
//!   session, and [`http`] (feature `http`) the HTTPS sender.
//! - [`binding`]: the policy of one agent-usable secret (`agent:<id>` of an
//!   item), validated: field, destinations, injection, environment name.
//!
//! Secrets live in zeroizing types, are never logged and never appear in an
//! error. `#![forbid(unsafe_code)]`.
#![forbid(unsafe_code)]

pub mod binding;
pub mod engine;
pub mod error;
pub mod grants;
#[cfg(feature = "http")]
pub mod http;
pub mod log;
pub mod mask;
pub mod request;
pub mod response;
pub mod source;
#[cfg(feature = "test-util")]
pub mod testing;
pub mod vault_source;

#[cfg(all(feature = "test-util", not(debug_assertions)))]
compile_error!(
    "the `test-util` feature adds a test CA and a test TLS server and must never be in a release build"
);

#[cfg(test)]
mod binding_tests;
#[cfg(test)]
#[cfg(feature = "test-util")]
mod engine_tests;
#[cfg(test)]
#[cfg(feature = "test-util")]
mod http_tests;
#[cfg(test)]
mod mask_tests;
#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod vault_source_tests;
