//! Sync-chaos scenarios for the folder engine (M6 T2 red team): adversarial,
//! deterministic schedules over the simulated file-sync client, with manual
//! clocks and seeds, no sleeps. Each asserts the sync invariants: no silent
//! loss, convergence, no warning after full delivery, and a warning when
//! files stay withheld.
//!
//! A scenario that breaks an invariant is kept as a regression test marked
//! `#[ignore = "M6-T2 chaos finding N: ..."]` until the engine is fixed.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

mod compaction;
mod hostile;
mod races;
mod skew;
mod util;
