//! Security insights for the dashboard (brief §5 "Security dashboard",
//! offline parts): weak, reused and old passwords, logins without a second
//! factor, and URLs that allow plain HTTP.
//!
//! Status: scaffold only. Implementation lands in M3 (insights track); see
//! `docs/PROGRESS.md` and `docs/DESKTOP_IPC.md` ("Security insights").
//!
//! Rules for the implementation:
//!
//! - Everything runs on the device over decrypted items the session already
//!   holds. Nothing leaves the device; the breach check (Have I Been Pwned
//!   k-anonymity) is M7 and lives elsewhere.
//! - Reports carry item IDs and finding kinds only, never a password, a
//!   hash of one or any other value derived from a secret, so a report can
//!   cross the IPC boundary and be logged safely.
//! - Reuse detection compares secrets in memory with constant-time
//!   equality and keeps no lookup table keyed by a secret beyond the call.
#![forbid(unsafe_code)]
