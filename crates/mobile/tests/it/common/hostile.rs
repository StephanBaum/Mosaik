//! A hostile server for the M10-G5b sync-chaos scenarios: the real router of
//! `mosaic_server::app` (as in `loopback.rs`) behind a middleware that records
//! every exchange and, while the test arms it, rewrites what the server
//! answers (rollback, freeze, fork, replay, reordering, truncation, swapped
//! and oversized answers) or holds one answer until the test releases it (a
//! lock, a store close or a process death between the server's commit and
//! the phone's apply).
//!
//! The server commits what it is asked to commit; only the answers lie. The
//! hold is a channel handshake, not a sleep: the middleware tells the test it
//! holds the answer and waits for the go.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use mosaic_server::{AppState, Config, Db, ManualClock, ServerKeys, app};
use serde_json::Value;

use super::Server;

/// The server clock at the start (Unix ms), as in `loopback.rs`.
pub const SERVER_T0: u64 = 1_800_000_000_000;
/// The most bytes a recorded request or answer may have.
const BODY_CAP: usize = 64 * 1024 * 1024;
/// How long a held answer waits for the test at most (a safety net: a test
/// that forgets to release fails instead of hanging).
const HOLD_LIMIT: Duration = Duration::from_secs(60);

/// One request and the server's honest answer.
#[derive(Clone, Debug)]
pub struct Exchange {
    pub method: String,
    pub path: String,
    pub query: String,
    pub request: Vec<u8>,
    pub status: u16,
    pub body: Vec<u8>,
    /// The test had armed the server (the phone's run).
    pub armed: bool,
}

impl Exchange {
    /// The vault of a `GET /v1/vaults/{v}/changes`.
    pub fn changes_vault(&self) -> Option<&str> {
        if self.method != "GET" {
            return None;
        }
        self.path
            .strip_prefix("/v1/vaults/")?
            .strip_suffix("/changes")
    }

    /// The vault of a `POST /v1/vaults/{v}/records`.
    pub fn push_vault(&self) -> Option<&str> {
        if self.method != "POST" {
            return None;
        }
        self.path
            .strip_prefix("/v1/vaults/")?
            .strip_suffix("/records")
    }

    /// The `since` of a changes request.
    pub fn since(&self) -> Option<u64> {
        self.query
            .split('&')
            .find_map(|kv| kv.strip_prefix("since="))
            .and_then(|v| v.parse().ok())
    }

    /// The answer as JSON (a success only).
    pub fn json(&self) -> Option<Value> {
        if self.status != 200 {
            return None;
        }
        serde_json::from_slice(&self.body).ok()
    }

    /// The request body as JSON.
    pub fn request_json(&self) -> Option<Value> {
        serde_json::from_slice(&self.request).ok()
    }

    /// The records of a changes answer.
    pub fn records(&self) -> Vec<Value> {
        self.json()
            .and_then(|v| v.get("records").and_then(Value::as_array).cloned())
            .unwrap_or_default()
    }
}

/// What the hostile server answers instead.
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Reply {
    /// A 200 with a JSON body.
    pub fn json(value: &Value) -> Self {
        Self {
            status: 200,
            body: serde_json::to_vec(value).unwrap(),
        }
    }
}

pub type Tamper = Box<dyn FnMut(&Exchange) -> Option<Reply> + Send>;
pub type Matcher = Box<dyn Fn(&Exchange) -> bool + Send>;

struct Hold {
    matches: Matcher,
    reached: Sender<()>,
    go: Receiver<()>,
}

/// The switches of the middleware.
#[derive(Default)]
pub struct Hostility {
    armed: AtomicBool,
    tamper: Mutex<Option<Tamper>>,
    hold: Mutex<Option<Hold>>,
    log: Mutex<Vec<Exchange>>,
}

async fn hostile(State(h): State<Arc<Hostility>>, request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    let request_bytes = axum::body::to_bytes(body, BODY_CAP)
        .await
        .unwrap_or_default();
    let method = parts.method.as_str().to_owned();
    let path = parts.uri.path().to_owned();
    let query = parts.uri.query().unwrap_or("").to_owned();
    let response = next
        .run(Request::from_parts(
            parts,
            Body::from(request_bytes.clone()),
        ))
        .await;
    let (mut parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, BODY_CAP)
        .await
        .unwrap_or_default();
    let armed = h.armed.load(Ordering::SeqCst);
    let mut exchange = Exchange {
        method,
        path,
        query,
        request: request_bytes.to_vec(),
        status: parts.status.as_u16(),
        body: bytes.to_vec(),
        armed,
    };
    h.log.lock().unwrap().push(exchange.clone());
    if armed {
        let reply = h
            .tamper
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|tamper| tamper(&exchange));
        if let Some(reply) = reply {
            exchange.status = reply.status;
            exchange.body = reply.body;
        }
    }
    let hold = {
        let mut slot = h.hold.lock().unwrap();
        if slot.as_ref().is_some_and(|hold| (hold.matches)(&exchange)) {
            slot.take()
        } else {
            None
        }
    };
    if let Some(hold) = hold {
        let _ = hold.reached.send(());
        let _ = hold.go.recv_timeout(HOLD_LIMIT);
    }
    parts.status = StatusCode::from_u16(exchange.status).unwrap();
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(exchange.body))
}

/// A running hostile server. Dropping it stops the server.
pub struct HostileServer {
    pub base: String,
    pub clock: Arc<ManualClock>,
    pub hostility: Arc<Hostility>,
    _dir: tempfile::TempDir,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl HostileServer {
    /// An open-registration server, honest until armed with a tamper.
    pub fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let base = format!("http://127.0.0.1:{port}");
        let dir = tempfile::tempdir().expect("tempdir");
        let d = dir.path().display();
        let config = Config::parse(&format!(
            "listen = \"127.0.0.1:{port}\"\ncanonical_origin = \"{base}\"\ndata_dir = '{d}'\n\
             secret_file = '{d}/server.key'\nregistration = \"open\"\n"
        ))
        .expect("hostile config");
        let clock = Arc::new(ManualClock::new(SERVER_T0));
        let state = AppState::new(
            &config,
            Db::open_in_memory().expect("db"),
            ServerKeys::generate(SERVER_T0 - 1000).expect("keys"),
            clock.clone(),
        );
        let hostility = Arc::new(Hostility::default());
        let router = app(state).layer(middleware::from_fn_with_state(
            Arc::clone(&hostility),
            hostile,
        ));
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
                let _ = axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await;
            });
        });
        Self {
            base,
            clock,
            hostility,
            _dir: dir,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    /// Arms (the phone's run) or disarms the tamper.
    pub fn arm(&self, on: bool) {
        self.hostility.armed.store(on, Ordering::SeqCst);
    }

    /// Rewrites the answers of armed runs; `None` keeps the honest answer.
    pub fn set_tamper(&self, tamper: impl FnMut(&Exchange) -> Option<Reply> + Send + 'static) {
        *self.hostility.tamper.lock().unwrap() = Some(Box::new(tamper));
    }

    /// The server answers honestly again.
    pub fn clear_tamper(&self) {
        *self.hostility.tamper.lock().unwrap() = None;
    }

    /// Holds the next answer that `matches` (after the server committed):
    /// `reached` fires when it is held, a send on `go` releases it.
    pub fn hold(
        &self,
        matches: impl Fn(&Exchange) -> bool + Send + 'static,
    ) -> (Receiver<()>, Sender<()>) {
        let (reached_tx, reached_rx) = mpsc::channel();
        let (go_tx, go_rx) = mpsc::channel();
        *self.hostility.hold.lock().unwrap() = Some(Hold {
            matches: Box::new(matches),
            reached: reached_tx,
            go: go_rx,
        });
        (reached_rx, go_tx)
    }

    /// Every exchange so far, with the server's honest answers.
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.hostility.log.lock().unwrap().clone()
    }

    /// How many requests reached the server.
    pub fn request_count(&self) -> usize {
        self.hostility.log.lock().unwrap().len()
    }

    /// The first uploaded version of a record: its `kind` and `ciphertext`
    /// as the client sent them.
    pub fn first_upload(&self, record_id: &str) -> Option<(Value, Value)> {
        self.exchanges().iter().find_map(|ex| {
            ex.push_vault()?;
            let request = ex.request_json()?;
            request
                .get("records")?
                .as_array()?
                .iter()
                .find(|r| r.get("recordId").and_then(Value::as_str) == Some(record_id))
                .map(|r| (r["kind"].clone(), r["ciphertext"].clone()))
        })
    }
}

impl Server for HostileServer {
    fn base(&self) -> &str {
        &self.base
    }

    fn advance_ms(&self, ms: u64) {
        self.clock.advance_ms(ms);
    }
}

impl Drop for HostileServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
