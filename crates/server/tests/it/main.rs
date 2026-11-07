//! Integration tests of `mosaic-server`: the real handlers over an in-memory
//! (or file) database, a manual clock and, where time matters, a paused
//! tokio clock.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test code: the scenarios are long and a failure should panic"
)]

mod api_auth;
mod api_limits;
mod api_sync;
mod cli;
mod common;
mod hosted;
mod loopback;
mod malicious_server;
mod no_plaintext;
mod scenarios;
mod sim_hosted;
