//! Integration tests of `mosaic-server`: the real handlers over an in-memory
//! (or file) database, a manual clock and, where time matters, a paused
//! tokio clock.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    reason = "test code: the scenarios are long and a failure should panic"
)]

mod api_auth;
mod api_limits;
mod api_sync;
mod cli;
mod common;
mod no_plaintext;
