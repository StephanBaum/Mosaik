//! The desktop end of agent access (ADR-0159, ADR-0160): the endpoint AI clients reach through
//! `mosaic agent mcp`, the paired clients, the dialogs and the request path.
//!
//! ```text
//! mosaic agent mcp --local endpoint--> [mosaic-agent server] --> [host] --> Engine --> VaultSource
//!                                                                  |           |
//!                                                            [prompts]    HttpsSender
//! ```
//!
//! - [`AgentService`] binds the endpoint while the setting `agentAccessEnabled` is on and the
//!   app runs (a Unix socket in a private directory, or on Windows a named pipe with a random
//!   name announced under a lease). It answers `status` while the vault is locked and refuses
//!   everything else then. The desktop never listens on TCP.
//! - `registry`: the paired clients (`agent-pairings.json`). `prompts`: the one pairing prompt
//!   and the queue of access prompts, with the 1 s Allow guard and the expiry enforced here.
//!   `host`: the allowlisted ops, wired to the policy engine of `mosaic-agent`.
//!
//! Activity policy: nothing an agent does counts as user activity. The host never calls
//! `Backend::touch`, so a client that polls, or opens dialogs nobody answers, cannot keep the
//! vault from auto-locking.
//!
//! No value reaches the webview. Events and status carry names, destinations and counts; the
//! injected value exists inside `HttpsSender::send` only.

mod host;
mod prompts;
mod registry;

#[cfg(windows)]
use mosaic_native_messaging::endpoint::DiscoveryLease;
use mosaic_native_messaging::endpoint::Endpoint;
use mosaic_native_messaging::error::Error as NmError;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use mosaic_agent::engine::{Engine, Now};
use mosaic_agent::http::HttpsSender;
use mosaic_agent::server::AgentServer;
use mosaic_agent::wire::{AccessState, Event, VaultState};

use crate::contract::agent_access::{
    AgentAccessStatus, AgentCloseReason, AgentEndpointState, AgentPairingView,
};
use crate::contract::events::AppEvent;
use crate::contract::ids::UnixMillis;
use crate::state::Backend;

pub use host::DesktopHost;
pub use prompts::{MIN_ALLOW_DELAY, Prompts};
pub use registry::{AgentRegistry, MAX_PAIRINGS, RegistryError};

/// Reads the time of the prompts and the engine.
pub type Clock = crate::browser::Clock;

/// The timeouts that tests shorten.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// How long a pairing request stays open.
    pub pair_timeout: Duration,
    /// How long an access request stays open, queue time included.
    pub access_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        let secs = Duration::from_secs(mosaic_agent::wire::ACCESS_TIMEOUT_SECS);
        Self {
            pair_timeout: secs,
            access_timeout: secs,
        }
    }
}

/// The endpoint and what keeps it up.
struct Running {
    server: Arc<AgentServer>,
    /// Windows: the hold on `endpoint.json` of the agent channel, dropped (and the file deleted)
    /// with the endpoint.
    #[cfg(windows)]
    _lease: Option<DiscoveryLease>,
}

/// What the host, the prompts and the commands share.
pub struct Shared {
    pub(crate) clock: Clock,
    pub(crate) limits: Limits,
    backend: Mutex<Weak<Backend>>,
    unlocked: AtomicBool,
    enabled: AtomicBool,
    pub(crate) registry: Mutex<AgentRegistry>,
    pub(crate) prompts: Mutex<Prompts>,
    pub(crate) engine: Mutex<Engine>,
    sender: Mutex<Option<Arc<HttpsSender>>>,
    running: Mutex<Option<Running>>,
    endpoint: Mutex<AgentEndpointState>,
    endpoint_override: Mutex<Option<Endpoint>>,
    in_flight: AtomicU32,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Wall-clock milliseconds.
pub(crate) fn wall_ms() -> u64 {
    u64::try_from(crate::browser::unix_millis()).unwrap_or(0)
}

impl Shared {
    fn new(clock: Clock, limits: Limits) -> Self {
        Self {
            clock,
            limits,
            backend: Mutex::new(Weak::new()),
            unlocked: AtomicBool::new(false),
            enabled: AtomicBool::new(false),
            registry: Mutex::new(AgentRegistry::memory()),
            prompts: Mutex::new(Prompts::default()),
            engine: Mutex::new(Engine::new()),
            sender: Mutex::new(None),
            running: Mutex::new(None),
            endpoint: Mutex::new(AgentEndpointState::Stopped),
            endpoint_override: Mutex::new(None),
            in_flight: AtomicU32::new(0),
        }
    }

    /// The backend, while the app runs.
    pub(crate) fn backend(&self) -> Option<Arc<Backend>> {
        lock(&self.backend).upgrade()
    }

    /// The current time of the prompts and the engine's monotonic clock.
    pub(crate) fn now(&self) -> Instant {
        (self.clock)()
    }

    /// The time, twice, for the engine: the monotonic clock and the wall clock.
    pub(crate) fn engine_now(&self) -> Now {
        Now {
            mono: self.now(),
            wall_ms: wall_ms(),
        }
    }

    /// Whether the vault is unlocked, as the lock hooks last said.
    pub(crate) fn is_unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst)
    }

    /// Whether agent access is on.
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// The vault and access state the endpoint reports.
    pub(crate) fn wire_state(&self) -> (VaultState, AccessState) {
        (
            if self.is_unlocked() {
                VaultState::Unlocked
            } else {
                VaultState::Locked
            },
            if self.is_enabled() {
                AccessState::On
            } else {
                AccessState::Off
            },
        )
    }

    /// Sends an event to the webview (and flags the window for attention for the dialogs).
    pub(crate) fn emit(&self, event: AppEvent) {
        if let Some(backend) = self.backend() {
            (backend.event_sink())(event);
        }
    }

    /// Tells the webview the pairings, grants or endpoint changed.
    pub(crate) fn state_changed(&self) {
        self.emit(AppEvent::AgentStateChanged);
    }

    /// The running server, if the endpoint is up.
    pub(crate) fn server(&self) -> Option<Arc<AgentServer>> {
        lock(&self.running).as_ref().map(|r| Arc::clone(&r.server))
    }

    /// Sends `event` to every connected client.
    fn broadcast_state(&self) {
        if let Some(server) = self.server() {
            let (vault, agent_access) = self.wire_state();
            server.broadcast(&Event::State {
                vault,
                agent_access,
            });
        }
    }

    /// The HTTPS sender, created on first use.
    pub(crate) fn sender(&self) -> Option<Arc<HttpsSender>> {
        let mut slot = lock(&self.sender);
        if slot.is_none() {
            *slot = HttpsSender::new().ok().map(Arc::new);
        }
        slot.clone()
    }

    /// The vault locked: dialogs close, every grant ends, the clients are told.
    fn on_locked(&self) {
        self.unlocked.store(false, Ordering::SeqCst);
        self.drop_prompts(AgentCloseReason::Locked);
        lock(&self.engine).end_all();
        self.broadcast_state();
        self.state_changed();
    }

    /// The vault unlocked: the endpoint starts if agent access is on, the clients are told.
    fn on_unlocked(self: &Arc<Self>) {
        self.unlocked.store(true, Ordering::SeqCst);
        let on = self.backend().is_some_and(|b| b.agent_access_on());
        self.enabled.store(on, Ordering::SeqCst);
        if on {
            self.ensure_running();
        }
        self.broadcast_state();
        self.state_changed();
    }

    /// Binds the endpoint unless it is up.
    fn ensure_running(self: &Arc<Self>) {
        if lock(&self.running).is_some() {
            return;
        }
        let state = match self.bind() {
            Ok(running) => {
                *lock(&self.running) = Some(running);
                AgentEndpointState::Running
            }
            Err(state) => state,
        };
        *lock(&self.endpoint) = state;
    }

    fn bind(self: &Arc<Self>) -> Result<Running, AgentEndpointState> {
        let forced = lock(&self.endpoint_override).clone();
        let endpoint = match &forced {
            Some(endpoint) => endpoint.clone(),
            None => platform_endpoint().map_err(|_| AgentEndpointState::Unavailable)?,
        };
        // The listener registers with the runtime when it is created.
        let runtime = tauri::async_runtime::handle();
        let _enter = runtime.inner().enter();
        let host = Arc::new(DesktopHost::new(Arc::clone(self)));
        let server =
            AgentServer::start(&endpoint, host).map_err(|_| AgentEndpointState::Unavailable)?;
        #[cfg(windows)]
        let lease = match (&endpoint, forced.is_some()) {
            (Endpoint::NamedPipe(pipe), false) => match announce(pipe) {
                Ok(lease) => Some(lease),
                Err(state) => {
                    server.stop();
                    return Err(state);
                }
            },
            _ => None,
        };
        Ok(Running {
            server: Arc::new(server),
            #[cfg(windows)]
            _lease: lease,
        })
    }

    /// Closes the endpoint (the discovery file goes with the lease) and every connection.
    fn unbind(&self) {
        let running = lock(&self.running).take();
        if let Some(running) = running {
            running.server.stop();
        }
        *lock(&self.endpoint) = AgentEndpointState::Stopped;
    }

    /// Agent access was switched on or off.
    fn apply_enabled(self: &Arc<Self>, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        if enabled {
            self.ensure_running();
            self.broadcast_state();
        } else {
            self.drop_prompts(AgentCloseReason::Off);
            lock(&self.engine).end_all();
            self.unbind();
        }
        self.state_changed();
    }
}

/// The endpoint of this OS: on Windows a fresh random pipe name per start, elsewhere the socket
/// path (`docs/NATIVE_MESSAGING.md` §9).
fn platform_endpoint() -> Result<Endpoint, NmError> {
    use mosaic_native_messaging::endpoint::ChannelKind;
    #[cfg(windows)]
    {
        mosaic_native_messaging::endpoint::new_windows_pipe_name_for(ChannelKind::Agent)
            .map(Endpoint::NamedPipe)
    }
    #[cfg(not(windows))]
    {
        mosaic_native_messaging::endpoint::default_endpoint_for(ChannelKind::Agent)
    }
}

/// The endpoint a real start would bind (tests check it is a pipe or a socket file).
#[cfg(test)]
pub(crate) fn platform_endpoint_for_test() -> Result<Endpoint, NmError> {
    platform_endpoint()
}

/// Windows: publishes the pipe name in the agent channel's `endpoint.json` and keeps the file
/// open, so a client can tell a live desktop from a stale file.
#[cfg(windows)]
fn announce(pipe: &str) -> Result<DiscoveryLease, AgentEndpointState> {
    use mosaic_native_messaging::endpoint::{ChannelKind, discovery_file_for};
    let path =
        discovery_file_for(ChannelKind::Agent).map_err(|_| AgentEndpointState::Unavailable)?;
    DiscoveryLease::create_for(ChannelKind::Agent, &path, pipe)
        .map_err(|_| AgentEndpointState::Unavailable)
}

/// The agent endpoint of this app instance.
pub struct AgentService {
    shared: Arc<Shared>,
}

impl Default for AgentService {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentService {
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
        }
    }

    pub(crate) const fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Connects the service to the app: the lock hooks and the event sink.
    pub fn attach(&self, backend: Weak<Backend>) {
        let Some(strong) = backend.upgrade() else {
            return;
        };
        self.shared
            .unlocked
            .store(strong.is_unlocked(), Ordering::SeqCst);
        *lock(&self.shared.backend) = backend;
        let shared = Arc::clone(&self.shared);
        strong.on_lock(Box::new(move |_| shared.on_locked()));
        let shared = Arc::clone(&self.shared);
        strong.on_unlock(Box::new(move || shared.on_unlocked()));
    }

    /// Loads the pairings from `dir` and connects to the app. The endpoint starts at the first
    /// unlock that finds the setting on (the setting reads off while the vault is locked).
    pub fn start(&self, dir: &Path, backend: Weak<Backend>) {
        self.attach(backend);
        *lock(&self.shared.registry) = AgentRegistry::in_dir(dir);
    }

    /// Stops the endpoint and drops the dialogs (the app is closing).
    pub fn stop(&self) {
        self.shared.drop_prompts(AgentCloseReason::Off);
        lock(&self.shared.engine).end_all();
        self.shared.unbind();
    }

    /// The vault locked (also called by the lock hook).
    pub fn on_lock(&self) {
        self.shared.on_locked();
    }

    /// Agent access was switched on or off: the endpoint follows and every grant ends when off.
    pub fn set_enabled(&self, enabled: bool) {
        self.shared.apply_enabled(enabled);
    }

    /// Closes every connection and ends every grant; the endpoint comes back up at once (the
    /// clients keep their pairings).
    pub(crate) fn restart(&self) {
        let shared = &self.shared;
        shared.drop_prompts(AgentCloseReason::Off);
        lock(&shared.engine).end_all();
        shared.unbind();
        if shared.is_enabled() {
            shared.ensure_running();
        }
        shared.state_changed();
    }

    /// Whether the endpoint is up.
    #[must_use]
    pub fn endpoint_state(&self) -> AgentEndpointState {
        *lock(&self.shared.endpoint)
    }

    /// The paired clients, for the settings screen.
    pub(crate) fn pairing_views(&self) -> Vec<AgentPairingView> {
        lock(&self.shared.registry)
            .list()
            .iter()
            .map(|p| AgentPairingView {
                pairing_id: crate::browser::hex(&p.id),
                name: p.name.clone(),
                paired_at: p.created_at,
                last_used_at: p.last_used_at,
            })
            .collect()
    }

    /// Requests being sent now.
    #[must_use]
    pub fn in_flight(&self) -> u32 {
        self.shared.in_flight.load(Ordering::SeqCst)
    }

    /// Where the `mosaic` program is: next to the app's executable, if it is there.
    pub(crate) fn cli_path() -> Option<String> {
        let exe = std::env::current_exe().ok()?;
        let name = if cfg!(windows) {
            "mosaic.exe"
        } else {
            "mosaic"
        };
        let path = exe.parent()?.join(name);
        path.is_file().then(|| path.to_string_lossy().into_owned())
    }

    /// The status for the settings screen; `grants` come from the engine, named from the vault.
    pub(crate) fn status(&self, backend: &Backend) -> AgentAccessStatus {
        AgentAccessStatus {
            enabled: backend.agent_access_on(),
            endpoint: self.endpoint_state(),
            pairings: self.pairing_views(),
            grants: self.shared.grant_views(backend),
            in_flight: self.in_flight(),
            cli_path: Self::cli_path(),
        }
    }

    /// A fixed endpoint for a test.
    #[cfg(test)]
    pub(crate) fn set_endpoint(&self, endpoint: Endpoint) {
        *lock(&self.shared.endpoint_override) = Some(endpoint);
    }

    /// A fixed sender for a test (a fixture CA and a local server).
    #[cfg(test)]
    pub(crate) fn set_sender(&self, sender: HttpsSender) {
        *lock(&self.shared.sender) = Some(Arc::new(sender));
    }
}

/// An hex string of a pairing ID is parsed by [`crate::browser::parse_id`].
pub(crate) fn unix_millis_of(ms: u64) -> UnixMillis {
    UnixMillis::try_from(ms).unwrap_or(UnixMillis::MAX)
}
