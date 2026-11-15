//! Test-only fixtures (feature `test-util`, debug builds only): a fixture CA and a local TLS 1.3
//! server for the HTTPS sender and the policy engine. Nothing here is trusted by any build:
//! the CA exists only in `testdata/` and is passed to [`crate::http::HttpsSender::for_test`].

mod host;
mod tls_server;

use std::path::Path;

use mosaic_native_messaging::endpoint::{ChannelKind, Endpoint, new_windows_pipe_name_for};

/// A fresh local agent endpoint for a test: a random pipe on Windows, `agent.sock` in `dir`
/// (which must be a private directory) elsewhere.
///
/// # Errors
/// If the OS random generator fails.
pub fn local_endpoint(dir: &Path) -> Result<Endpoint, mosaic_native_messaging::Error> {
    if cfg!(windows) {
        Ok(Endpoint::NamedPipe(new_windows_pipe_name_for(
            ChannelKind::Agent,
        )?))
    } else {
        Ok(Endpoint::UnixSocket(dir.join("agent.sock")))
    }
}

pub use host::{FakeSource, TestHost};
pub use tls_server::{
    API_HOST, CA_DER, EVIL_HOST, OTHER_CA_DER, RecordedRequest, TestResponse, TestRoute,
    TlsTestServer,
};
