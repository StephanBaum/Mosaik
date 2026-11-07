//! Constants of the sync engine (ADR-0045) and the folder backend.

use core::time::Duration;

/// Objects per pull page.
pub const PULL_LIMIT: usize = 500;
/// Records per push request.
pub const PUSH_MAX_RECORDS: usize = 256;
/// Bytes per push request.
pub const PUSH_MAX_BYTES: usize = 8 * 1024 * 1024;
/// Rounds of pull-then-push after a conflict before the records stay queued.
pub const MAX_CONFLICT_ROUNDS: u32 = 3;
/// The first retry delay after a transient failure.
pub const BACKOFF_INITIAL_MS: u64 = 5_000;
/// The longest retry delay.
pub const BACKOFF_MAX_MS: u64 = 900_000;
/// How long a hosted long-poll waits.
pub const LONG_POLL_MS: u64 = 25_000;
/// How often the folder backend looks for changes while it waits.
pub const FOLDER_POLL_INTERVAL: Duration = Duration::from_secs(30);
/// How long a local change waits for more before a sync starts.
pub const LOCAL_CHANGE_DEBOUNCE_MS: u64 = 2_000;
/// How long a hosted device may lag before a freeze is suspected.
pub const FREEZE_GRACE_HOSTED_MS: u64 = 600_000;
/// How long a folder device may lag before files count as missing.
pub const FREEZE_GRACE_FOLDER_MS: u64 = 86_400_000;
/// Attachment bytes downloaded per run.
pub const ATTACHMENT_DOWNLOAD_BUDGET: u64 = 64 * 1024 * 1024;
/// How long a deleted attachment stays before it is purged (30 days).
pub const BLOB_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1000;
/// Segments in the own log (since the last snapshot) that trigger compaction.
pub const COMPACT_SEGMENTS: usize = 64;
/// Bytes in the own log (since the last snapshot) that trigger compaction.
pub const COMPACT_BYTES: u64 = 32 * 1024 * 1024;
/// How old a temp file in an own directory must be to be removed.
pub const TEMP_MAX_AGE_MS: u64 = 24 * 60 * 60 * 1000;
/// The most records one push pass fetches per vault.
pub(crate) const PUSH_FETCH: usize = 4096;
/// The most conflict-driven key rotations of one vault in
/// [`CONFLICT_ROTATION_WINDOW_MS`] on the hosted backend (a server that
/// keeps contradicting itself must not make devices rotate without end).
pub const MAX_CONFLICT_ROTATIONS: u32 = 3;
/// The window of [`MAX_CONFLICT_ROTATIONS`] (24 hours).
pub const CONFLICT_ROTATION_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;
