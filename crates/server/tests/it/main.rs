//! Integration tests of `mosaic-server`: the real handlers over an in-memory
//! (or file) database, a manual clock and, where time matters, a paused
//! tokio clock.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    reason = "test code: the scenarios are long and a failure should panic"
)]

mod api_auth;
mod api_limits;
mod api_sync;
mod chaos_hosted;
mod chaos_m9;
mod chaos_net;
mod cli;
mod common;
mod contest_seed;
mod emergency;
mod hosted;
mod hosted_client;
mod loopback;
mod malicious_rekey;
mod malicious_server;
mod malicious_sharing;
mod no_plaintext;
mod redteam_server;
mod rekey;
mod scenarios;
mod sharing;
mod sharing_world;
mod sim_hosted;
mod sim_rekey;
mod sim_sharing;
mod webauthn;
mod webauthn_m9;
