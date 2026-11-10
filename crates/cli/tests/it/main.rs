//! Integration tests: the built `mosaic` binary against a fixture store, with the file-based test
//! backends of the `test-util` feature (spec §5.3). Run with `--test-threads=2`: every fixture copy
//! unlocks with the 128 MiB Argon2id cost.

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "integration tests"
)]

mod common;
mod exec;
mod output;
mod reading;
mod session;
mod transfer;
