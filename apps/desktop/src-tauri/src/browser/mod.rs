//! The desktop end of the browser extension channel (M4, ADR-0035 to
//! ADR-0037; protocol: `docs/NATIVE_MESSAGING.md`).
//!
//! ```text
//! mosaic-native-host --local endpoint--> [connection] --> [handlers] --> Backend (session)
//!                                            |               |
//!                                       [registry]   [prompts] [ratelimit]
//! ```
//!
//! - [`BrowserService`] binds the endpoint (Unix socket in a private
//!   directory; Windows named pipe with a random name, announced in the
//!   discovery file that a [`DiscoveryLease`] keeps alive) and accepts the
//!   host's connections on Tauri's async runtime.
//! - `connection`: the pre-auth exchange (`pairRequest`, `status`, `focus`),
//!   the library's server-side handshake, and the AEAD request loop.
//! - `handlers`: the allowlist. Every op the extension may call is one arm of
//!   one `match`; nothing else exists. The ops run on the blocking pool.
//! - `fill`, `save`: the vault side (`Backend::browser_*`), through
//!   `mosaic-origin` for every match decision.
//! - `prompts`: the single pairing prompt and the single save prompt, with
//!   the 1 s Allow guard and the expiry; `registry`: the paired browsers;
//!   `ratelimit`: sliding windows.
//!
//! Activity policy: only `fill` and `copy` count as user activity
//! (`Backend::touch`), so a script that polls the endpoint cannot keep the
//! vault from auto-locking. Secrets leave this module only in a `fill`
//! response, a `generate` response, and the clipboard.

mod connection;
mod fill;
mod handlers;
mod prompts;
mod ratelimit;
mod registry;
mod save;

#[cfg(windows)]
use mosaic_native_messaging::endpoint::DiscoveryLease;
use mosaic_native_messaging::endpoint::Endpoint;
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::protocol::ErrorCode as WireError;
use mosaic_native_messaging::transport::Listener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::contract::browser::{
    BrowserEndpointState, BrowserPairing, BrowserRegistryState, BrowserStatus,
};
use crate::contract::events::AppEvent;
use crate::contract::ids::UnixMillis;
use crate::state::Backend;

pub use connection::Out;
pub use prompts::{PairOutcome, Prompts};
pub use ratelimit::{Bucket, RateLimits};
pub use registry::{Registry, RegistryError, label_is_valid};

/// Reads the time of the prompts and the rate limits.
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

/// The timeouts that tests shorten.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// How long a pairing request stays open.
    pub pair_timeout: Duration,
    /// How long a save request stays open.
    pub save_timeout: Duration,
    /// A session with no request for this long is closed.
    pub idle: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        use mosaic_native_messaging::protocol::{PAIR_TIMEOUT_SECS, SAVE_PROMPT_TIMEOUT_SECS};
        Self {
            pair_timeout: Duration::from_secs(PAIR_TIMEOUT_SECS),
            save_timeout: Duration::from_secs(SAVE_PROMPT_TIMEOUT_SECS),
            idle: Duration::from_mins(10),
        }
    }
}

/// What the connection tasks, the prompts and the webview commands share.
pub struct Shared {
    pub(crate) clock: Clock,
    pub(crate) limits: Limits,
    pub(crate) registry: Mutex<Registry>,
    pub(crate) prompts: Mutex<Prompts>,
    pub(crate) limiter: Mutex<RateLimits>,
    pub(crate) conns: Mutex<connection::Conns>,
    backend: Mutex<Weak<Backend>>,
    unlocked: AtomicBool,
    endpoint: Mutex<BrowserEndpointState>,
}

impl Shared {
    fn new(clock: Clock, limits: Limits) -> Self {
        Self {
            clock,
            limits,
            registry: Mutex::new(Registry::memory()),
            prompts: Mutex::new(Prompts::default()),
            limiter: Mutex::new(RateLimits::default()),
            conns: Mutex::new(connection::Conns::default()),
            backend: Mutex::new(Weak::new()),
            unlocked: AtomicBool::new(false),
            endpoint: Mutex::new(BrowserEndpointState::Unavailable),
        }
    }

    /// The backend, while the app runs.
    pub(crate) fn backend(&self) -> Option<Arc<Backend>> {
        self.backend.lock().ok().and_then(|b| b.upgrade())
    }

    /// The current time of the prompts and limits.
    pub(crate) fn now(&self) -> Instant {
        (self.clock)()
    }

    /// Whether the vault is unlocked, as the lock hooks last said.
    pub(crate) fn is_unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst)
    }

    /// Sends an event to the webview (and flags the window for attention
    /// for the prompts).
    pub(crate) fn emit(&self, event: AppEvent) {
        if let Some(backend) = self.backend() {
            (backend.event_sink())(event);
        }
    }

    /// Tells the webview the pairings or connections changed.
    pub(crate) fn pairings_changed(&self) {
        self.emit(AppEvent::BrowserPairingsChanged);
    }
}

/// Wall-clock milliseconds, as the webview counts them.
pub(crate) fn unix_millis() -> UnixMillis {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| {
        UnixMillis::try_from(d.as_millis()).unwrap_or(UnixMillis::MAX)
    })
}

/// The endpoint and what keeps it up.
struct Running {
    accept: tauri::async_runtime::JoinHandle<()>,
    /// Windows: the hold on `endpoint.json`, dropped (and the file deleted)
    /// with the service.
    #[cfg(windows)]
    _lease: DiscoveryLease,
}

/// The browser extension endpoint of this app instance.
pub struct BrowserService {
    shared: Arc<Shared>,
    running: Mutex<Option<Running>>,
}

impl Default for BrowserService {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserService {
    /// An idle service: no endpoint, an in-memory registry.
    #[must_use]
    pub fn new() -> Self {
        Self::with_clock(Arc::new(Instant::now), Limits::default())
    }

    /// An idle service on this clock and with these timeouts.
    #[must_use]
    pub fn with_clock(clock: Clock, limits: Limits) -> Self {
        Self {
            shared: Arc::new(Shared::new(clock, limits)),
            running: Mutex::new(None),
        }
    }

    pub(crate) const fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Connects the service to the app: the lock hooks, the event sink.
    pub fn attach(&self, backend: Weak<Backend>) {
        let Some(strong) = backend.upgrade() else {
            return;
        };
        self.shared
            .unlocked
            .store(strong.is_unlocked(), Ordering::SeqCst);
        if let Ok(mut slot) = self.shared.backend.lock() {
            *slot = backend;
        }
        let shared = Arc::clone(&self.shared);
        strong.on_lock(Box::new(move |_| shared.on_locked()));
        let shared = Arc::clone(&self.shared);
        strong.on_unlock(Box::new(move || shared.on_unlocked()));
    }

    /// Loads the pairings from `dir`, binds the endpoint and starts
    /// accepting connections. A failure shows up in [`BrowserService::status`].
    pub fn start(&self, dir: &Path, backend: Weak<Backend>) {
        self.attach(backend);
        if let Ok(mut registry) = self.shared.registry.lock() {
            *registry = Registry::in_dir(dir);
        }
        let state = match self.bind() {
            Ok(running) => {
                if let Ok(mut slot) = self.running.lock() {
                    *slot = Some(running);
                }
                BrowserEndpointState::Running
            }
            Err(state) => state,
        };
        if let Ok(mut endpoint) = self.shared.endpoint.lock() {
            *endpoint = state;
        }
    }

    fn bind(&self) -> Result<Running, BrowserEndpointState> {
        let endpoint = platform_endpoint().map_err(|_| BrowserEndpointState::Unavailable)?;
        // The listener registers with the runtime when it is created.
        let runtime = tauri::async_runtime::handle();
        let _enter = runtime.inner().enter();
        let mut listener = Listener::bind(&endpoint).map_err(|e| {
            if e == NmError::EndpointInUse {
                BrowserEndpointState::InUse
            } else {
                BrowserEndpointState::Unavailable
            }
        })?;
        #[cfg(windows)]
        let lease = match &endpoint {
            Endpoint::NamedPipe(pipe) => announce(pipe)?,
            Endpoint::UnixSocket(_) => return Err(BrowserEndpointState::Unavailable),
        };
        let shared = Arc::clone(&self.shared);
        let accept = tauri::async_runtime::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok(stream) => shared.serve(stream),
                    Err(_) => tokio::time::sleep(Duration::from_millis(250)).await,
                }
            }
        });
        Ok(Running {
            accept,
            #[cfg(windows)]
            _lease: lease,
        })
    }

    /// Stops accepting, closes every session and removes the endpoint (on
    /// Windows the discovery file goes with the lease). Pending prompts are
    /// dropped.
    pub fn stop(&self) {
        let running = self.running.lock().ok().and_then(|mut r| r.take());
        if let Some(running) = running {
            running.accept.abort();
        }
        self.shared.close_all();
        self.shared
            .drop_prompts(crate::contract::browser::BrowserCloseReason::Disconnected);
        if let Ok(mut endpoint) = self.shared.endpoint.lock() {
            *endpoint = BrowserEndpointState::Unavailable;
        }
    }

    /// The vault locked: prompts close, the held password is dropped, and the
    /// sessions are told.
    pub fn on_locked(&self) {
        self.shared.on_locked();
    }

    /// The vault unlocked: the sessions are told.
    pub fn on_unlocked(&self) {
        self.shared.on_unlocked();
    }

    /// Removes a pairing's live sessions (`unpaired`, then close).
    pub fn close_pairing(&self, pairing_id: &[u8; 16]) {
        self.shared.close_pairing(pairing_id, None);
    }

    /// The endpoint and the pairings, for the settings screen.
    #[must_use]
    pub fn status(&self) -> BrowserStatus {
        let endpoint = self
            .shared
            .endpoint
            .lock()
            .map_or(BrowserEndpointState::Unavailable, |e| *e);
        let connected: Vec<[u8; 16]> = self.shared.connected_pairings();
        let (registry, pairings) = self.shared.registry.lock().map_or_else(
            // A poisoned lock: nothing can be changed, say so.
            |_| (BrowserRegistryState::Unreadable, Vec::new()),
            |registry| {
                let state = if registry.is_unreadable() {
                    BrowserRegistryState::Unreadable
                } else {
                    BrowserRegistryState::Ok
                };
                let pairings = registry
                    .list()
                    .iter()
                    .map(|p| BrowserPairing {
                        pairing_id: hex(&p.id),
                        label: p.label.clone(),
                        browser: p.browser,
                        created_at: p.created_at,
                        last_used_at: p.last_used_at,
                        connected: connected.contains(&p.id),
                    })
                    .collect();
                (state, pairings)
            },
        );
        BrowserStatus {
            endpoint,
            registry,
            pairings,
        }
    }

    /// Serves one connection (tests hand in a duplex stream).
    #[cfg(test)]
    pub(crate) fn serve<S>(&self, stream: S)
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        self.shared.serve(stream);
    }
}

/// The endpoint of this OS: on Windows a fresh random pipe name per start,
/// elsewhere the socket path (`docs/NATIVE_MESSAGING.md` §1).
fn platform_endpoint() -> Result<Endpoint, NmError> {
    #[cfg(windows)]
    {
        mosaic_native_messaging::endpoint::new_windows_pipe_name().map(Endpoint::NamedPipe)
    }
    #[cfg(not(windows))]
    {
        mosaic_native_messaging::endpoint::default_endpoint()
    }
}

/// Windows: publishes the pipe name in `endpoint.json` and keeps the file
/// open, so a host can tell a live desktop from a stale file.
#[cfg(windows)]
fn announce(pipe: &str) -> Result<DiscoveryLease, BrowserEndpointState> {
    let path = mosaic_native_messaging::endpoint::discovery_file()
        .map_err(|_| BrowserEndpointState::Unavailable)?;
    DiscoveryLease::create(&path, pipe).map_err(|e| {
        if e == NmError::EndpointInUse {
            BrowserEndpointState::InUse
        } else {
            BrowserEndpointState::Unavailable
        }
    })
}

/// Lowercase hex of `bytes` (IDs, not secrets).
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Parses 32 lowercase hex digits.
pub(crate) fn parse_id(text: &str) -> Option<[u8; 16]> {
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if pairs.len() != 16 || !rest.is_empty() {
        return None;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 16];
    for (byte, [hi, lo]) in out.iter_mut().zip(pairs) {
        *byte = (digit(*hi)? << 4) | digit(*lo)?;
    }
    Some(out)
}

/// Locks a shared mutex, mapping poisoning to `internal`.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, WireError> {
    mutex.lock().map_err(|_| WireError::Internal)
}
