//! The IPC contract between the webview and the Rust core
//! (`docs/DESKTOP_IPC.md`). Every request, response, event and error type
//! the webview sees is defined here, by area; the TypeScript mirror lives
//! in `apps/desktop/src/ipc/contract/`.
//!
//! Wire rules (ADR-0018):
//! - JSON with camelCase keys; tagged unions carry their tag in `kind`,
//!   `state`, `op` or `type`; string enums are camelCase (`wire_enum!`).
//! - Every field is always present; "no value" is `null`, never a missing
//!   key. Requests reject unknown keys.
//! - IDs are canonical UUID strings, times are Unix milliseconds.
//! - No secret in any type except [`secret::SecretIn`] (requests) and
//!   [`secret::SecretOut`] (explicit reveals, generated passwords, codes).
//! - `fixtures` exports samples of every type to
//!   `src/ipc/contract.fixtures.ts`, which `tsc` checks against the
//!   TypeScript types; a test keeps the file current.

mod macros;

pub mod clipboard;
pub mod errors;
pub mod events;
pub mod generator;
pub mod ids;
pub mod import;
pub mod insights;
pub mod items;
pub mod secret;
pub mod session;
pub mod settings;
pub mod ssh;
pub mod totp;
pub mod vaults;

#[cfg(test)]
pub(crate) mod fixtures;
