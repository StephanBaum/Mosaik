//! A local HTTPS server for tests: rustls `ServerConnection` over a `std::net::TcpListener` on
//! `127.0.0.1:0`, a minimal HTTP/1.1 reader (request line, headers, `Content-Length` body) and
//! scripted answers: split writes, slow drips, endless chunked bodies, large headers.
//!
//! One request per connection (`Connection: close`). The leaf certificate names `api.mosaic.test`
//! and `evil.mosaic.test`; clients resolve those names to [`TlsTestServer::addr`] and put the port
//! in the URL.

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use rustls::crypto::ring;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

/// The fixture CA (DER). Pass it to `HttpsSender::for_test`.
pub const CA_DER: &[u8] = include_bytes!("../../testdata/ca.der");
/// An unrelated CA: trusting only it makes the server's certificate untrusted.
pub const OTHER_CA_DER: &[u8] = include_bytes!("../../testdata/other-ca.der");
const LEAF_DER: &[u8] = include_bytes!("../../testdata/leaf.der");
const LEAF_KEY_DER: &[u8] = include_bytes!("../../testdata/leaf.key.der");

/// A name of the leaf certificate: the "saved" destination of tests.
pub const API_HOST: &str = "api.mosaic.test";
/// A second name of the leaf certificate: the destination that must never be reached.
pub const EVIL_HOST: &str = "evil.mosaic.test";

const MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const ENDLESS_LIMIT: usize = 64 * 1024 * 1024;

/// What the server saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedRequest {
    /// The method.
    pub method: String,
    /// The request target as sent (path and query).
    pub target: String,
    /// The headers in order, names as sent.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl RecordedRequest {
    /// The value of the first header called `name` (case-insensitive).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// A scripted answer.
#[derive(Clone, Debug)]
pub struct TestResponse {
    /// The status code.
    pub status: u16,
    /// Extra headers (`Content-Length` and `Connection` are added by the server).
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
    /// Write the body in two TLS records, split at this offset.
    pub split_at: Option<usize>,
    /// Write the body one byte at a time with this pause in between.
    pub drip: Option<Duration>,
    /// Ignore `body` and send an endless chunked body of `a` bytes.
    pub endless: bool,
}

impl TestResponse {
    /// A plain answer.
    #[must_use]
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
            split_at: None,
            drip: None,
            endless: false,
        }
    }

    /// Adds a header.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// A path and the function that answers it.
#[derive(Clone, Copy)]
pub struct TestRoute {
    /// The request path (without the query).
    pub path: &'static str,
    /// The answer.
    pub respond: fn(&RecordedRequest) -> TestResponse,
}

/// A running test server; it stops listening when dropped.
pub struct TlsTestServer {
    addr: SocketAddr,
    seen: Seen,
    stop: Arc<AtomicBool>,
}

impl core::fmt::Debug for TlsTestServer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TlsTestServer")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

fn server_config() -> Result<Arc<ServerConfig>, rustls::Error> {
    let chain = vec![CertificateDer::from(LEAF_DER.to_vec())];
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(LEAF_KEY_DER.to_vec()));
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(chain, key)?;
    Ok(Arc::new(config))
}

impl TlsTestServer {
    /// Starts a server answering `routes` (any other path gets a 404).
    ///
    /// # Errors
    /// The I/O or TLS error that stopped it from starting.
    pub fn try_start(routes: Vec<TestRoute>) -> io::Result<Self> {
        let config = server_config().map_err(io::Error::other)?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let addr = listener.local_addr()?;
        let seen: Seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let routes = Arc::new(routes);
        let (seen_in, stop_in) = (Arc::clone(&seen), Arc::clone(&stop));
        thread::spawn(move || accept_loop(&listener, &stop_in, &config, &routes, &seen_in));
        Ok(Self { addr, seen, stop })
    }

    /// [`Self::try_start`] for tests.
    ///
    /// # Panics
    /// If the server cannot start.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test fixture: a server that cannot start fails the test"
    )]
    pub fn start(routes: Vec<TestRoute>) -> Self {
        Self::try_start(routes).expect("the test TLS server starts")
    }

    /// Where it listens (`127.0.0.1`).
    #[must_use]
    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Every request seen so far, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for TlsTestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so that it sees the flag.
        let _ = TcpStream::connect(self.addr);
    }
}

type Seen = Arc<Mutex<Vec<RecordedRequest>>>;

fn accept_loop(
    listener: &TcpListener,
    stop: &AtomicBool,
    config: &Arc<ServerConfig>,
    routes: &Arc<Vec<TestRoute>>,
    seen: &Seen,
) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(tcp) = stream else { continue };
        let (config, routes, seen) = (Arc::clone(config), Arc::clone(routes), Arc::clone(seen));
        // A failed exchange is the client's doing (it hung up); nothing to report.
        thread::spawn(move || handle(tcp, config, &routes, &seen).ok());
    }
}

type Tls = StreamOwned<ServerConnection, TcpStream>;

fn handle(
    tcp: TcpStream,
    config: Arc<ServerConfig>,
    routes: &[TestRoute],
    seen: &Mutex<Vec<RecordedRequest>>,
) -> io::Result<()> {
    tcp.set_read_timeout(Some(Duration::from_secs(10)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(10)))?;
    let conn = ServerConnection::new(config).map_err(io::Error::other)?;
    let mut tls = StreamOwned::new(conn, tcp);
    let request = read_request(&mut tls)?;
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(request.clone());
    let path = request.target.split('?').next().unwrap_or_default();
    let response = routes.iter().find(|r| r.path == path).map_or_else(
        || TestResponse::new(404, "not found"),
        |r| (r.respond)(&request),
    );
    write_response(&mut tls, &response)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn read_request(tls: &mut Tls) -> io::Result<RecordedRequest> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = find(&buf, b"\r\n\r\n") {
            break i;
        }
        let n = tls.read(&mut chunk)?;
        if n == 0 || buf.len() > MAX_REQUEST_BYTES {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf.extend_from_slice(chunk.get(..n).unwrap_or_default());
    };
    let head = String::from_utf8_lossy(buf.get(..head_end).unwrap_or_default()).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split(' ');
    let method = first.next().unwrap_or_default().to_owned();
    let target = first.next().unwrap_or_default().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.to_owned(), v.trim().to_owned()))
        .collect();
    let length = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buf.get(head_end + 4..).unwrap_or_default().to_vec();
    while body.len() < length {
        let n = tls.read(&mut chunk)?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        body.extend_from_slice(chunk.get(..n).unwrap_or_default());
    }
    body.truncate(length);
    Ok(RecordedRequest {
        method,
        target,
        headers,
        body,
    })
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

fn write_response(tls: &mut Tls, response: &TestResponse) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason(response.status)
    );
    for (name, value) in &response.headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("connection: close\r\n");
    if response.endless {
        head.push_str("transfer-encoding: chunked\r\n\r\n");
    } else {
        head.push_str("content-length: ");
        head.push_str(&response.body.len().to_string());
        head.push_str("\r\n\r\n");
    }
    tls.write_all(head.as_bytes())?;
    tls.flush()?;
    if response.endless {
        let piece = vec![b'a'; 1024];
        let mut sent = 0;
        while sent < ENDLESS_LIMIT {
            tls.write_all(b"400\r\n")?;
            tls.write_all(&piece)?;
            tls.write_all(b"\r\n")?;
            tls.flush()?;
            sent += piece.len();
        }
    } else if let Some(pause) = response.drip {
        for byte in &response.body {
            tls.write_all(std::slice::from_ref(byte))?;
            tls.flush()?;
            thread::sleep(pause);
        }
    } else if let Some(at) = response.split_at {
        let (first, second) = response.body.split_at(at.min(response.body.len()));
        tls.write_all(first)?;
        tls.flush()?;
        thread::sleep(Duration::from_millis(100));
        tls.write_all(second)?;
        tls.flush()?;
    } else {
        tls.write_all(&response.body)?;
        tls.flush()?;
    }
    tls.conn.send_close_notify();
    tls.flush()
}
