//! Helpers for the unit tests.

use core::future::Future;

/// Runs `fut` on a fresh current-thread runtime.
pub(crate) fn block_on<F: Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

/// Runs `fut` with the clock paused: sleeps and timeouts advance instantly
/// once every task is idle.
pub(crate) fn block_on_paused<F: Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(fut)
}
