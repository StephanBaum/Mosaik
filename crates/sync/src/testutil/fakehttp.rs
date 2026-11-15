//! A small HTTP server on a loopback port for the tests of the remote folder
//! stores: it records every request and answers it with what a handler
//! returns. One thread per connection, `Connection: close`, request bodies
//! with a `Content-Length` of at most 64 MiB, and it stops when dropped.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// The largest request body the server accepts.
const MAX_BODY: usize = 64 * 1024 * 1024;
/// The largest request head the server accepts.
const MAX_HEAD: usize = 64 * 1024;

/// One request as the server saw it.
#[derive(Clone, Debug)]
pub struct FakeRequest {
    /// `GET`, `PROPFIND`, ...
    pub method: String,
    /// The request target: path and query, as sent.
    pub path_and_query: String,
    /// The headers, names in lower case.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl FakeRequest {
    /// The first header called `name` (any case).
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// What the handler answers.
#[derive(Clone, Debug)]
pub struct FakeResponse {
    /// The status code.
    pub status: u16,
    /// The headers. `Transfer-Encoding: chunked` sends the body chunked.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl FakeResponse {
    /// An answer with these headers and body.
    #[must_use]
    pub fn new(status: u16, headers: &[(&str, &str)], body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: body.into(),
        }
    }
}

type Handler = dyn Fn(&FakeRequest) -> FakeResponse + Send + Sync + 'static;

/// The running server.
pub struct FakeHttp {
    origin: String,
    requests: Arc<Mutex<Vec<FakeRequest>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeHttp {
    /// Starts a server on a free loopback port.
    ///
    /// # Panics
    /// If no loopback port can be bound (a test machine without a network
    /// stack).
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test utility: a failure is a failed test"
    )]
    pub fn start(handler: impl Fn(&FakeRequest) -> FakeResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
        listener
            .set_nonblocking(true)
            .expect("non-blocking listener");
        let port = listener.local_addr().expect("local address").port();
        let handler: Arc<Handler> = Arc::new(handler);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (log, stopped) = (Arc::clone(&requests), Arc::clone(&stop));
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (handler, log) = (Arc::clone(&handler), Arc::clone(&log));
                        std::thread::spawn(move || serve(stream, &*handler, &log));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(2)),
                }
            }
        });
        Self {
            origin: format!("http://127.0.0.1:{port}"),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    /// `http://127.0.0.1:<port>`.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Every request so far, in arrival order.
    #[must_use]
    pub fn requests(&self) -> Vec<FakeRequest> {
        self.requests.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl Drop for FakeHttp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Reads one request; `None` for anything that is not a well-formed one.
fn read_request(stream: &mut TcpStream) -> Option<FakeRequest> {
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    let head_end = loop {
        if let Some(i) = find(&data, b"\r\n\r\n") {
            break i;
        }
        if data.len() > MAX_HEAD {
            return None;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => data.extend_from_slice(buf.get(..n)?),
        }
    };
    let head = String::from_utf8_lossy(data.get(..head_end)?).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target) = (first.next()?.to_owned(), first.next()?.to_owned());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let len = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len > MAX_BODY {
        return None;
    }
    let mut body = data.get(head_end + 4..)?.to_vec();
    while body.len() < len {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => body.extend_from_slice(buf.get(..n)?),
        }
    }
    body.truncate(len);
    Some(FakeRequest {
        method,
        path_and_query: target,
        headers,
        body,
    })
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        207 => "Multi-Status",
        302 => "Found",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        412 => "Precondition Failed",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

fn write_response(stream: &mut TcpStream, response: &FakeResponse) {
    let chunked = response.headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
    });
    let mut out = format!(
        "HTTP/1.1 {} {}\r\nConnection: close\r\n",
        response.status,
        reason(response.status)
    )
    .into_bytes();
    for (k, v) in &response.headers {
        out.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    if chunked {
        out.extend_from_slice(b"\r\n");
        for chunk in response.body.chunks(4096) {
            out.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            out.extend_from_slice(chunk);
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"0\r\n\r\n");
    } else {
        out.extend_from_slice(
            format!("Content-Length: {}\r\n\r\n", response.body.len()).as_bytes(),
        );
        out.extend_from_slice(&response.body);
    }
    let _ = stream.write_all(&out);
    let _ = stream.flush();
}

fn serve(mut stream: TcpStream, handler: &Handler, log: &Mutex<Vec<FakeRequest>>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    if let Some(request) = read_request(&mut stream) {
        if let Ok(mut g) = log.lock() {
            g.push(request.clone());
        }
        write_response(&mut stream, &handler(&request));
    }
    let _ = stream.shutdown(Shutdown::Both);
}
