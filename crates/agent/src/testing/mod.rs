//! Test-only fixtures (feature `test-util`, debug builds only): a fixture CA and a local TLS 1.3
//! server for the HTTPS sender and the policy engine. Nothing here is trusted by any build:
//! the CA exists only in `testdata/` and is passed to [`crate::http::HttpsSender::for_test`].

mod tls_server;

pub use tls_server::{
    API_HOST, CA_DER, EVIL_HOST, OTHER_CA_DER, RecordedRequest, TestResponse, TestRoute,
    TlsTestServer,
};
