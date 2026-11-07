//! The real server on `127.0.0.1:0` for the tests that drive it through the
//! blocking HTTP client: the router of `mosaic_server::app` over an in-memory
//! database, a temporary blob directory and a manual server clock, served by
//! a thread of its own (the client blocks, the server must not share its
//! thread). Included by the server's integration tests and by the sync
//! crate's HTTP transport tests (`#[path]`), so it depends only on
//! `mosaic_server`, `axum`, `tokio` and `tempfile`.
//!
//! `LoopbackTransport` is [`mosaic_sync::http::HttpTransport`] connected to
//! such a server.
#![allow(dead_code, reason = "each test module uses a part of the helpers")]

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;

use mosaic_server::{AppState, Config, Db, ManualClock, ServerKeys, app};
use mosaic_sync::wire::json::RegistrationMode;

/// The server clock at the start (Unix ms).
pub const SERVER_T0: u64 = 1_800_000_000_000;

/// A running server. Dropping it stops the server.
pub struct LoopbackServer {
    /// `http://127.0.0.1:<port>`, the server's canonical origin.
    pub base: String,
    /// The application state (database, invites).
    pub state: AppState,
    /// The server's clock: nothing moves it but the test (rate limits,
    /// session expiry and TOTP steps read it).
    pub clock: Arc<ManualClock>,
    _dir: tempfile::TempDir,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl LoopbackServer {
    /// An open-registration server.
    pub fn start() -> Self {
        Self::with_mode(RegistrationMode::Open)
    }

    /// A server with a registration mode.
    pub fn with_mode(mode: RegistrationMode) -> Self {
        Self::with_origin(mode, None)
    }

    /// A server that names `origin_host` (for example `localhost`) as its
    /// identity instead of `127.0.0.1`, to test the identity check.
    pub fn with_origin(mode: RegistrationMode, origin_host: Option<&str>) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let base = format!("http://127.0.0.1:{port}");
        let origin = format!("http://{}:{port}", origin_host.unwrap_or("127.0.0.1"));
        let dir = tempfile::tempdir().expect("tempdir");
        let d = dir.path().display();
        let mode_text = match mode {
            RegistrationMode::Open => "open",
            RegistrationMode::Invite => "invite",
            RegistrationMode::Closed => "closed",
        };
        let config = Config::parse(&format!(
            "listen = \"127.0.0.1:{port}\"\ncanonical_origin = \"{origin}\"\ndata_dir = '{d}'\n\
             secret_file = '{d}/server.key'\nregistration = \"{mode_text}\"\n"
        ))
        .expect("loopback config");
        let clock = Arc::new(ManualClock::new(SERVER_T0));
        let state = AppState::new(
            &config,
            Db::open_in_memory().expect("db"),
            ServerKeys::generate(SERVER_T0 - 1000).expect("keys"),
            clock.clone(),
        );
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let router = app(state.clone());
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
            state,
            clock,
            _dir: dir,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    /// Moves the server clock (and so the rate-limit buckets) forward.
    pub fn advance_ms(&self, ms: u64) {
        self.clock.advance_ms(ms);
    }

    /// Stores an invite code the way the CLI does (only its hash).
    pub fn add_invite(&self, code: &str) {
        use sha2::{Digest, Sha256};
        let hash: [u8; 32] = Sha256::digest(code.as_bytes()).into();
        self.state
            .db()
            .run(|c| {
                c.execute(
                    "INSERT INTO invites (code_hash, created_at) VALUES (?1, 0)",
                    [hash.as_slice()],
                )?;
                Ok(())
            })
            .expect("invite");
    }
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
