//! Fake provider servers for the tests of the remote folder stores. Each one
//! serves a plain directory, so [`CloudSim`](super::CloudSim) can move files
//! in and out of it like a file-sync client does, and each one can lag, fail
//! and lie on demand.

mod dav;
mod dropbox;
mod graph;

pub use dav::{FakeDav, Faults};
pub use dropbox::{AUTHORIZATION_CODE, FakeDropbox};
pub use graph::{DRIVE_ID, FakeGraph, GRAPH_AUTHORIZATION_CODE, id_of};
