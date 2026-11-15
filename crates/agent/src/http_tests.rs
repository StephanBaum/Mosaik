//! The HTTPS sender against the fixture TLS server (ADR-0046, ADR-0159, design §7.2).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use mosaic_core::SecretString;
use mosaic_core::model::{AgentBindingRecord, AgentInject};

use crate::binding::Binding;
use crate::http::{HttpsSender, MAX_RESPONSE_HEADER_BYTES, SendError};
use crate::request::{HttpRequestInput, PreparedRequest, validate};
use crate::testing::{
    API_HOST, CA_DER, EVIL_HOST, OTHER_CA_DER, RecordedRequest, TestResponse, TestRoute,
    TlsTestServer,
};

const VALUE: &str = "SK-TEST-CANARY-0123456789abcdef";

fn prepared(addr: SocketAddr, path: &str, max_bytes: u32) -> PreparedRequest {
    let origin = format!("https://{API_HOST}:{}", addr.port());
    let binding = Binding::from_record(&AgentBindingRecord::new(
        "api_key.secret",
        &[origin.as_str()],
        AgentInject::Bearer,
    ))
    .unwrap();
    let input = HttpRequestInput {
        method: "GET".to_owned(),
        url: format!("{origin}{path}"),
        headers: Vec::new(),
        body: None,
        max_bytes: Some(max_bytes),
    };
    validate(&binding, &input).unwrap().inject(
        &binding.inject,
        &SecretString::from_str_copy(VALUE).unwrap(),
        None,
    )
}

fn sender(server: &TlsTestServer) -> HttpsSender {
    HttpsSender::for_test(
        CA_DER,
        &[(API_HOST, server.addr()), (EVIL_HOST, server.addr())],
    )
    .unwrap()
}

fn ok(_: &RecordedRequest) -> TestResponse {
    TestResponse::new(200, "hello")
}

#[test]
fn a_request_reaches_the_server_with_the_secret_in_one_header() {
    let server = TlsTestServer::start(vec![TestRoute {
        path: "/ok",
        respond: ok,
    }]);
    let response = sender(&server)
        .send(prepared(server.addr(), "/ok", 1000), 0)
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"hello".to_vec());
    assert!(!response.truncated);
    let seen = server.requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].header("authorization"),
        Some(format!("Bearer {VALUE}").as_str())
    );
    assert_eq!(seen[0].header("accept-encoding"), Some("identity"));
    assert!(
        seen[0]
            .headers
            .iter()
            .all(|(n, v)| n.eq_ignore_ascii_case("authorization") || !v.contains(VALUE)),
        "canary found"
    );
    assert!(!seen[0].target.contains(VALUE), "canary found");
}

static EVIL_PORT: AtomicU16 = AtomicU16::new(0);

fn redirect(_: &RecordedRequest) -> TestResponse {
    let port = EVIL_PORT.load(Ordering::SeqCst);
    TestResponse::new(302, "").header("location", &format!("https://{EVIL_HOST}:{port}/x"))
}

#[test]
fn redirects_are_not_followed() {
    let server = TlsTestServer::start(vec![TestRoute {
        path: "/redirect",
        respond: redirect,
    }]);
    let evil = TlsTestServer::start(vec![TestRoute {
        path: "/x",
        respond: ok,
    }]);
    EVIL_PORT.store(evil.addr().port(), Ordering::SeqCst);
    let sender = HttpsSender::for_test(
        CA_DER,
        &[(API_HOST, server.addr()), (EVIL_HOST, evil.addr())],
    )
    .unwrap();
    let response = sender
        .send(prepared(server.addr(), "/redirect", 1000), 0)
        .unwrap();
    assert_eq!(response.status, 302);
    assert!(
        response.headers.iter().any(|(n, _)| n == "location"),
        "the Location header is handed on for masking, not followed"
    );
    assert_eq!(server.requests().len(), 1);
    assert!(
        evil.requests().is_empty(),
        "the redirect target saw nothing"
    );
}

fn big(_: &RecordedRequest) -> TestResponse {
    TestResponse::new(200, vec![b'x'; 10_000])
}

fn many_headers(_: &RecordedRequest) -> TestResponse {
    let mut response = TestResponse::new(200, "x");
    for i in 0..70 {
        response = response.header(&format!("x-pad-{i}"), &"p".repeat(1000));
    }
    response
}

fn slow(_: &RecordedRequest) -> TestResponse {
    let mut response = TestResponse::new(200, vec![b's'; 100]);
    response.drip = Some(Duration::from_millis(100));
    response
}

fn endless(_: &RecordedRequest) -> TestResponse {
    let mut response = TestResponse::new(200, "");
    response.endless = true;
    response
}

#[test]
fn response_caps_hold() {
    let server = TlsTestServer::start(vec![
        TestRoute {
            path: "/big",
            respond: big,
        },
        TestRoute {
            path: "/headers",
            respond: many_headers,
        },
        TestRoute {
            path: "/slow",
            respond: slow,
        },
        TestRoute {
            path: "/endless",
            respond: endless,
        },
    ]);
    let client = sender(&server);

    // A body over max_bytes + hold-back is cut and flagged.
    let response = client
        .send(prepared(server.addr(), "/big", 100), 10)
        .unwrap();
    assert_eq!(response.body.len(), 110);
    assert!(response.truncated);
    // Exactly at the cap is not truncated.
    let response = client
        .send(prepared(server.addr(), "/big", 10_000), 0)
        .unwrap();
    assert_eq!(response.body.len(), 10_000);
    assert!(!response.truncated);

    // Headers over 64 KiB.
    const { assert!(70 * 1000 > MAX_RESPONSE_HEADER_BYTES) };
    assert_eq!(
        client
            .send(prepared(server.addr(), "/headers", 100), 0)
            .err(),
        Some(SendError::ResponseTooLarge)
    );

    // An endless chunked body stops at the cap.
    let response = client
        .send(prepared(server.addr(), "/endless", 4096), 0)
        .unwrap();
    assert_eq!(response.body.len(), 4096);
    assert!(response.truncated);

    // A slow drip beyond the total time limit.
    let quick = HttpsSender::for_test_with_timeout(
        CA_DER,
        &[(API_HOST, server.addr())],
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(
        quick.send(prepared(server.addr(), "/slow", 1000), 0).err(),
        Some(SendError::Timeout)
    );
}

fn gzip(_: &RecordedRequest) -> TestResponse {
    TestResponse::new(200, vec![0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef])
        .header("content-encoding", "gzip")
        .header("content-type", "application/json")
}

#[test]
fn compressed_response_is_reported_as_binary() {
    let server = TlsTestServer::start(vec![TestRoute {
        path: "/gzip",
        respond: gzip,
    }]);
    let response = sender(&server)
        .send(prepared(server.addr(), "/gzip", 1000), 0)
        .unwrap();
    assert_eq!(response.content_encoding(), Some("gzip"));
    assert_eq!(response.content_type(), Some("application/json"));
    assert_eq!(
        response.body,
        vec![0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef]
    );
    assert_eq!(
        server.requests()[0].header("accept-encoding"),
        Some("identity")
    );
}

#[test]
fn untrusted_certificate_is_tls_failed() {
    let server = TlsTestServer::start(vec![TestRoute {
        path: "/ok",
        respond: ok,
    }]);
    let distrusting = HttpsSender::for_test(OTHER_CA_DER, &[(API_HOST, server.addr())]).unwrap();
    assert_eq!(
        distrusting
            .send(prepared(server.addr(), "/ok", 1000), 0)
            .err(),
        Some(SendError::Tls)
    );
    assert!(
        server.requests().is_empty(),
        "no request is sent over a failed handshake"
    );
}

#[test]
fn a_closed_port_is_a_connect_error() {
    let server = TlsTestServer::start(vec![]);
    let addr = server.addr();
    let client = sender(&server);
    drop(server);
    std::thread::sleep(Duration::from_millis(200));
    let result = client.send(prepared(addr, "/ok", 1000), 0);
    assert!(
        matches!(result, Err(SendError::Connect | SendError::Other)),
        "{result:?}"
    );
}
