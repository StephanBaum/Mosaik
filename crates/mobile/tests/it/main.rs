//! Integration tests of `mosaic-mobile`: a `Core` over a temporary directory, an
//! in-memory secure storage, and the real server on `127.0.0.1` where a test
//! needs one. They run through `TestHooks`, so they need the `test-hooks`
//! feature (the crate's own dev-dependency turns it on).

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::assert_is_empty,
    clippy::field_reassign_with_default,
    clippy::too_many_lines,
    clippy::format_collect,
    clippy::bool_assert_comparison,
    reason = "integration tests: scenarios are long and read plainly"
)]

mod autofill;
mod common;
mod g1_review;
mod g3_redteam;
mod g3i_redteam;
mod g5_gaps;
mod items;
mod join;
mod lifecycle;
mod passkeys;
mod quick_unlock;
mod settings;
mod setup;
mod sync;
mod unlock;
