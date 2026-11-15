//! The server loop of the agent endpoint (`docs/NATIVE_MESSAGING.md` section 9, feature
//! `server`). It accepts connections on the agent [`Endpoint`], runs the pre-auth exchange and
//! the handshake of `mosaic_native_messaging::agent_handshake`, and dispatches the requests of
//! an authenticated relay to an [`AgentHost`].
//!
//! ```text
//! first message   hello       -> agent_server_handshake -> session (AEAD request loop)
//!                 pairRequest -> pairPending, host.pair(), pairApproved | refused
//!                 status      -> status
//! ```
//!
//! A session has a reader task (frames in, idle timeout, up to [`MAX_IN_FLIGHT`] requests
//! running at once, each on its own task) and a writer (the one place that seals and writes, so
//! sequence numbers follow the write order). A frame that does not open poisons the session and
//! closes the connection; a frame that is not a request envelope at all closes it too.
//!
//! Connection slots follow the browser channel: a connection that has not authenticated holds
//! one of [`MAX_PREAUTH`] pre-auth slots (a new one evicts the oldest, so idle connections cannot
//! lock the real relay out), and only a verified handshake counts against
//! [`MAX_CONNECTIONS`].

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use mosaic_core::SecretVec;
use mosaic_core::browser_channel::{ChannelSession, PairingKey};
use mosaic_native_messaging::agent_handshake::{agent_read_client_message, agent_server_handshake};
use mosaic_native_messaging::agent_preauth::{
    AgentClientMessage, AgentRefusal, AgentServerMessage,
};
use mosaic_native_messaging::endpoint::Endpoint;
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame, write_frame};
use mosaic_native_messaging::handshake::write_message;
use mosaic_native_messaging::preauth::new_pairing_code;
use mosaic_native_messaging::transport::Listener;
use mosaic_native_messaging::{Hex, HexKey};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::{AbortHandle, JoinSet};
use tokio::time::timeout;
use zeroize::Zeroizing;

use crate::error::AgentErrorCode;
use crate::wire::{
    ACCESS_TIMEOUT_SECS, AccessState, Event, IDLE_SECS, MAX_CONNECTIONS, MAX_IN_FLIGHT, OpRequest,
    OpResponse, ResponseEnvelope, StatusBody, VaultState, WireError, parse_request, request_id,
};

/// The most connections that have not authenticated yet. A new one beyond this evicts the oldest.
pub const MAX_PREAUTH: usize = 4;

/// The step timeout of a frame write.
const WRITE_STEP: Duration = Duration::from_secs(5);

/// What a desktop (or a test) provides to the server loop.
pub trait AgentHost: Send + Sync + 'static {
    /// The vault and access state now.
    fn state(&self) -> (VaultState, AccessState);

    /// The stored key of a pairing.
    fn pairing_key(&self, id: &[u8; 16]) -> Option<Arc<PairingKey>>;

    /// A pairing request: show the dialog with `code`, wait for the user. The future is dropped
    /// when the client hangs up, which must close the dialog.
    fn pair(
        &self,
        name: String,
        code: String,
    ) -> impl Future<Output = Result<([u8; 16], PairingKey), AgentRefusal>> + Send;

    /// Handles one request of an authenticated pairing. `conn` identifies the connection.
    fn handle(
        &self,
        pairing: [u8; 16],
        conn: u64,
        op: OpRequest,
    ) -> impl Future<Output = Result<OpResponse, WireError>> + Send;

    /// A request of this pairing was answered (for "last used").
    fn note_used(&self, pairing: &[u8; 16]);

    /// The key of a new pairing could not be delivered to the client: nobody can use that
    /// pairing, so the host removes it.
    fn pairing_undelivered(&self, _pairing: &[u8; 16]) {}
}

/// What the writer of a session is told.
enum Out {
    /// A plaintext to seal and send.
    Frame(Zeroizing<Vec<u8>>),
    /// Close the connection (after everything queued before).
    Close,
}

struct ConnEntry {
    id: u64,
    pairing: [u8; 16],
    tx: UnboundedSender<Out>,
    /// Set once the reader task is spawned.
    reader: Option<AbortHandle>,
}

#[derive(Default)]
struct Registry {
    next: u64,
    /// Authenticated sessions being served (at most [`MAX_CONNECTIONS`]).
    open: usize,
    /// Connections that have not authenticated, oldest first.
    pre: Vec<(u64, AbortHandle)>,
    list: Vec<ConnEntry>,
}

/// The connections of one server.
#[derive(Default)]
struct Conns(Mutex<Registry>);

impl Conns {
    fn lock(&self) -> MutexGuard<'_, Registry> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn new_id(&self) -> u64 {
        let mut reg = self.lock();
        reg.next += 1;
        reg.next
    }

    /// Takes a pre-auth slot for connection `id`, evicting the oldest when all are taken.
    fn admit_preauth(&self, id: u64, handle: AbortHandle) {
        let mut reg = self.lock();
        while reg.pre.len() >= MAX_PREAUTH {
            let (_, oldest) = reg.pre.remove(0);
            oldest.abort();
        }
        reg.pre.push((id, handle));
    }

    fn sessions_full(&self) -> bool {
        self.lock().open >= MAX_CONNECTIONS
    }

    fn register(&self, pairing: [u8; 16], tx: UnboundedSender<Out>) -> u64 {
        let mut reg = self.lock();
        reg.next += 1;
        let id = reg.next;
        reg.list.push(ConnEntry {
            id,
            pairing,
            tx,
            reader: None,
        });
        id
    }

    fn attach_reader(&self, id: u64, reader: AbortHandle) {
        if let Some(entry) = self.lock().list.iter_mut().find(|c| c.id == id) {
            entry.reader = Some(reader);
        }
    }

    fn unregister(&self, id: u64) {
        self.lock().list.retain(|c| c.id != id);
    }

    fn send_all(&self, json: &[u8]) {
        for conn in &self.lock().list {
            let _ = conn.tx.send(Out::Frame(Zeroizing::new(json.to_vec())));
        }
    }
}

/// Holds the slot of a connection while it is served.
struct ConnGuard {
    conns: Arc<Conns>,
    id: u64,
    authenticated: bool,
}

impl ConnGuard {
    /// The connection is not a handshake any more (a pairing dialog): it can no longer be
    /// evicted.
    fn leave_preauth(&self) {
        self.conns.lock().pre.retain(|(id, _)| *id != self.id);
    }

    /// A verified handshake: the connection counts against [`MAX_CONNECTIONS`]. `false`: that
    /// many sessions are open already.
    fn authenticate(&mut self) -> bool {
        let mut reg = self.conns.lock();
        if reg.open >= MAX_CONNECTIONS {
            return false;
        }
        reg.open += 1;
        let id = self.id;
        reg.pre.retain(|(p, _)| *p != id);
        self.authenticated = true;
        true
    }
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut reg = self.conns.lock();
        let id = self.id;
        reg.pre.retain(|(p, _)| *p != id);
        if self.authenticated {
            reg.open = reg.open.saturating_sub(1);
        }
    }
}

/// The running agent endpoint. Dropping it does not stop it: call [`AgentServer::stop`].
pub struct AgentServer {
    conns: Arc<Conns>,
    accept: AbortHandle,
}

impl AgentServer {
    /// Binds `endpoint` and serves it on the current tokio runtime.
    ///
    /// # Errors
    /// The errors of `Listener::bind`: [`NmError::EndpointInUse`],
    /// [`NmError::EndpointInsecure`], [`NmError::EndpointInvalid`], [`NmError::Io`].
    pub fn start<H: AgentHost>(endpoint: &Endpoint, host: Arc<H>) -> Result<Self, NmError> {
        let listener = Listener::bind(endpoint)?;
        let conns = Arc::new(Conns::default());
        let task = tokio::spawn(accept_loop(listener, host, Arc::clone(&conns)));
        Ok(Self {
            conns,
            accept: task.abort_handle(),
        })
    }

    /// Closes the listener and every connection.
    pub fn stop(&self) {
        self.accept.abort();
        let mut reg = self.conns.lock();
        for conn in &reg.list {
            let _ = conn.tx.send(Out::Close);
            if let Some(reader) = &conn.reader {
                reader.abort();
            }
        }
        for (_, handle) in reg.pre.drain(..) {
            handle.abort();
        }
    }

    /// Tells every connection of `pairing` that it was removed, then closes it.
    pub fn close_pairing(&self, pairing: &[u8; 16]) {
        let event = Event::Unpaired.to_json().ok();
        for conn in self
            .conns
            .lock()
            .list
            .iter()
            .filter(|c| c.pairing == *pairing)
        {
            if let Some(json) = &event {
                let _ = conn.tx.send(Out::Frame(Zeroizing::new(json.clone())));
            }
            let _ = conn.tx.send(Out::Close);
        }
    }

    /// Sends `event` to every authenticated connection.
    pub fn broadcast(&self, event: &Event) {
        if let Ok(json) = event.to_json() {
            self.conns.send_all(&json);
        }
    }

    /// Sends `event` to the connections of one pairing.
    pub fn send_to_pairing(&self, pairing: &[u8; 16], event: &Event) {
        let Ok(json) = event.to_json() else {
            return;
        };
        for conn in self
            .conns
            .lock()
            .list
            .iter()
            .filter(|c| c.pairing == *pairing)
        {
            let _ = conn.tx.send(Out::Frame(Zeroizing::new(json.clone())));
        }
    }

    /// The number of authenticated connections.
    #[must_use]
    pub fn connections(&self) -> usize {
        self.conns.lock().list.len()
    }
}

async fn accept_loop<H: AgentHost>(mut listener: Listener, host: Arc<H>, conns: Arc<Conns>) {
    loop {
        match listener.accept().await {
            Ok(stream) => serve(&host, &conns, stream),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

/// Serves one connection on the runtime once it holds a pre-auth slot.
fn serve<H, S>(host: &Arc<H>, conns: &Arc<Conns>, stream: S)
where
    H: AgentHost,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let id = conns.new_id();
    let (go, wait) = oneshot::channel::<()>();
    let (host, conns2) = (Arc::clone(host), Arc::clone(conns));
    let task = tokio::spawn(async move {
        if wait.await.is_err() {
            return;
        }
        let mut guard = ConnGuard {
            conns: Arc::clone(&conns2),
            id,
            authenticated: false,
        };
        let _ = handle_connection(&host, &conns2, stream, &mut guard).await;
    });
    conns.admit_preauth(id, task.abort_handle());
    let _ = go.send(());
}

/// Reads the first message and serves the connection accordingly.
async fn handle_connection<H, S>(
    host: &Arc<H>,
    conns: &Arc<Conns>,
    mut stream: S,
    guard: &mut ConnGuard,
) -> Result<(), NmError>
where
    H: AgentHost,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match agent_read_client_message(&mut stream).await? {
        AgentClientMessage::Hello {
            pairing_id, nonce, ..
        } => {
            if conns.sessions_full() {
                return refuse(&mut stream, AgentRefusal::Busy).await;
            }
            let (vault, access) = host.state();
            if access == AccessState::Off {
                return refuse(&mut stream, AgentRefusal::AgentAccessOff).await;
            }
            let key = host.pairing_key(&pairing_id.0);
            let first = Event::State {
                vault,
                agent_access: access,
            }
            .to_json()
            .map_err(|_| NmError::Internal)?;
            let session = agent_server_handshake(
                &mut stream,
                &pairing_id.0,
                &nonce.0,
                key.as_deref(),
                &first,
            )
            .await?;
            if !guard.authenticate() {
                // Another session took the last slot during the handshake.
                return Ok(());
            }
            run_session(host, conns, stream, session, pairing_id.0, (vault, access)).await;
            Ok(())
        }
        AgentClientMessage::Status { .. } => {
            let (vault, agent_access) = host.state();
            let reply = AgentServerMessage::Status {
                v: 1,
                vault,
                agent_access,
            };
            write_message(&mut stream, &reply).await
        }
        AgentClientMessage::PairRequest { name, .. } => {
            let (vault, access) = host.state();
            if access == AccessState::Off {
                return refuse(&mut stream, AgentRefusal::AgentAccessOff).await;
            }
            if vault == VaultState::Locked {
                return refuse(&mut stream, AgentRefusal::Locked).await;
            }
            // The dialog may stay open for minutes: the connection is not evictable.
            guard.leave_preauth();
            pair_flow(host, stream, name).await
        }
        AgentClientMessage::Finish { .. } => {
            refuse(&mut stream, AgentRefusal::InvalidRequest).await
        }
    }
}

async fn refuse<S: AsyncWrite + Unpin>(
    stream: &mut S,
    reason: AgentRefusal,
) -> Result<(), NmError> {
    write_message(stream, &AgentServerMessage::refused(reason)).await
}

/// A `pairRequest`: show the dialog, wait for the user, answer.
async fn pair_flow<H, S>(host: &Arc<H>, mut stream: S, name: String) -> Result<(), NmError>
where
    H: AgentHost,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let code = new_pairing_code()?;
    let pending = AgentServerMessage::PairPending {
        v: 1,
        code: code.clone(),
        expires_in_seconds: u32::try_from(ACCESS_TIMEOUT_SECS).unwrap_or(u32::MAX),
    };
    write_message(&mut stream, &pending).await?;
    let pair = {
        let host = Arc::clone(host);
        tokio::spawn(async move { host.pair(name, code).await })
    };
    let abort = pair.abort_handle();
    // The relay hangs up if the user gives up: end the dialog then.
    let (mut reader, mut writer) = tokio::io::split(stream);
    let watcher = {
        let abort = abort.clone();
        tokio::spawn(async move {
            let mut byte = [0u8; 1];
            let _ = reader.read(&mut byte).await;
            abort.abort();
        })
    };
    let wait = Duration::from_secs(ACCESS_TIMEOUT_SECS + 10);
    let outcome = timeout(wait, pair).await;
    watcher.abort();
    let reply = match outcome {
        Ok(Ok(Ok((id, key)))) => Some((
            Some(id),
            AgentServerMessage::PairApproved {
                v: 1,
                pairing_id: Hex(id),
                key: HexKey(key),
            },
        )),
        Ok(Ok(Err(reason))) => Some((None, AgentServerMessage::refused(reason))),
        // The task was cancelled (the client hung up) or panicked: nobody to answer.
        Ok(Err(_)) => None,
        Err(_) => {
            abort.abort();
            Some((
                None,
                AgentServerMessage::refused(AgentRefusal::PairingExpired),
            ))
        }
    };
    let Some((approved, reply)) = reply else {
        return Ok(());
    };
    let written = write_message(&mut writer, &reply).await;
    drop(reply);
    if written.is_err()
        && let Some(id) = approved
    {
        host.pairing_undelivered(&id);
    }
    written?;
    let _ = writer.shutdown().await;
    Ok(())
}

/// The session: reader task, request tasks, and the writer in this task. `told` is the state the
/// first frame reported.
async fn run_session<H, S>(
    host: &Arc<H>,
    conns: &Arc<Conns>,
    stream: S,
    session: ChannelSession,
    pairing: [u8; 16],
    told: (VaultState, AccessState),
) where
    H: AgentHost,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let session = Arc::new(Mutex::new(session));
    let (tx, rx) = unbounded_channel();
    let id = conns.register(pairing, tx.clone());
    if host.pairing_key(&pairing).is_none() {
        // The pairing was removed during the handshake (`close_pairing` ran before this session
        // was registered): say so and close.
        if let Ok(json) = Event::Unpaired.to_json() {
            let _ = tx.send(Out::Frame(Zeroizing::new(json)));
        }
        let _ = tx.send(Out::Close);
        writer_loop(writer, rx, session).await;
        conns.unregister(id);
        return;
    }
    let task = tokio::spawn(reader_loop(
        reader,
        Arc::clone(&session),
        tx.clone(),
        Arc::clone(host),
        pairing,
        id,
    ));
    conns.attach_reader(id, task.abort_handle());
    let now = host.state();
    if now != told
        && let Ok(json) = (Event::State {
            vault: now.0,
            agent_access: now.1,
        })
        .to_json()
    {
        // The state changed while the handshake ran.
        let _ = tx.send(Out::Frame(Zeroizing::new(json)));
    }
    drop(tx);
    writer_loop(writer, rx, session).await;
    task.abort();
    conns.unregister(id);
}

/// Seals and writes what the session queued, in order.
async fn writer_loop<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut rx: UnboundedReceiver<Out>,
    session: Arc<Mutex<ChannelSession>>,
) {
    while let Some(out) = rx.recv().await {
        let Out::Frame(plain) = out else {
            break;
        };
        let sealed = session.lock().ok().map(|mut s| s.seal(&plain));
        let Some(Ok(frame)) = sealed else {
            break;
        };
        let written = timeout(WRITE_STEP, write_frame(&mut writer, ByteOrder::Big, &frame)).await;
        if !matches!(written, Ok(Ok(()))) {
            break;
        }
    }
    let _ = writer.shutdown().await;
}

/// Reads and opens frames and starts the request tasks. Ends the session on a closed
/// connection, the idle timeout, or any frame that does not open. The request tasks end with it.
async fn reader_loop<R: AsyncRead + Unpin, H: AgentHost>(
    mut reader: R,
    session: Arc<Mutex<ChannelSession>>,
    tx: UnboundedSender<Out>,
    host: Arc<H>,
    pairing: [u8; 16],
    conn: u64,
) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let mut tasks = JoinSet::new();
    let idle = Duration::from_secs(IDLE_SECS);
    loop {
        let read = timeout(idle, read_frame(&mut reader, ByteOrder::Big)).await;
        let Ok(Ok(Some(frame))) = read else {
            break;
        };
        let opened = session.lock().ok().map(|mut s| s.open(&frame));
        let Some(Ok(plain)) = opened else {
            break;
        };
        drop(frame);
        while tasks.try_join_next().is_some() {}
        if !start_request(plain, &host, pairing, conn, &tx, &in_flight, &mut tasks) {
            break;
        }
    }
    let _ = tx.send(Out::Close);
}

fn queue(tx: &UnboundedSender<Out>, envelope: &ResponseEnvelope) {
    if let Ok(json) = envelope.to_json() {
        let _ = tx.send(Out::Frame(Zeroizing::new(json)));
    }
}

/// Answers an invalid or `busy` request, or starts a task for a valid one. `false`: the
/// plaintext is not a request envelope at all, so the peer is not the relay.
fn start_request<H: AgentHost>(
    plain: SecretVec,
    host: &Arc<H>,
    pairing: [u8; 16],
    conn: u64,
    tx: &UnboundedSender<Out>,
    in_flight: &Arc<AtomicUsize>,
    tasks: &mut JoinSet<()>,
) -> bool {
    let Some(id) = request_id(plain.expose_secret()) else {
        return false;
    };
    let envelope = match parse_request(plain.expose_secret()) {
        Ok(envelope) => envelope,
        Err(code) => {
            queue(tx, &ResponseEnvelope::failed(id, WireError::code(code)));
            return true;
        }
    };
    drop(plain);
    if in_flight.fetch_add(1, Ordering::SeqCst) >= MAX_IN_FLIGHT {
        in_flight.fetch_sub(1, Ordering::SeqCst);
        queue(
            tx,
            &ResponseEnvelope::failed(id, WireError::code(AgentErrorCode::Busy)),
        );
        return true;
    }
    let (host, tx, in_flight) = (Arc::clone(host), tx.clone(), Arc::clone(in_flight));
    tasks.spawn(async move {
        let answer = host.handle(pairing, conn, envelope.op).await;
        let response = match answer {
            Ok(body) => {
                host.note_used(&pairing);
                ResponseEnvelope::ok(id, body)
            }
            Err(error) => ResponseEnvelope::failed(id, error),
        };
        queue(&tx, &response);
        in_flight.fetch_sub(1, Ordering::SeqCst);
    });
    true
}

/// The `status` answer a host gives without looking at anything else.
#[must_use]
pub const fn status_response(vault: VaultState, agent_access: AccessState) -> OpResponse {
    OpResponse::Status(StatusBody {
        vault,
        agent_access,
    })
}
