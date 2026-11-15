//! The Windows shim of Mosaic's program fill (ADR-0157): the only crate of the
//! workspace that contains `unsafe`.
//!
//! The rules (ADR-0157 D2):
//! - `unsafe` appears only in the modules named `sys_*`, one operation per
//!   block, each with a `// SAFETY:` comment; `tools/check-unsafe-confined.sh`
//!   keeps it there.
//! - The public API is safe and small: no `unsafe fn`, no raw pointer, no
//!   `windows` crate type and no raw handle. Handles are owned by RAII types
//!   and closed on every path.
//! - Nothing here logs. Strings are decoded strictly (an invalid UTF-16 string
//!   is an error, never a lossy one).
//!
//! The crate is empty on targets other than Windows.
#![cfg(windows)]

mod error;
mod sys_input;
mod sys_process;
mod sys_trust;
mod sys_uia;
mod sys_version;
mod sys_window;

pub use error::{Result, ShimError};
pub use sys_input::{Key, MAX_UNITS_PER_CALL, modifiers_down, send_key, send_unicode};
pub use sys_process::{
    ImageFile, ProcessHandle, current_integrity_rid, final_path, parent_pid, process_chain,
};
pub use sys_trust::Signature;
pub use sys_uia::{ElementFacts, FocusSink, FocusSubscription, Uia, UiaElement};
pub use sys_window::{
    WindowFacts, foreground_window, set_foreground, show_no_activate, window_facts,
};

#[cfg(test)]
mod tests;
