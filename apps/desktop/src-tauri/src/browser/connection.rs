//! One connection from the native messaging host: the pre-auth exchange,
//! the handshake and the AEAD request loop (`docs/NATIVE_MESSAGING.md` §4).
//!
//! ```text
//! first message   hello      -> server_handshake -> session (AEAD request loop)
//!                 pairRequest-> prompt, wait, pairApproved | refused
//!                 status     -> status
//!                 focus      -> focused | refused
//! ```
//!
//! A session has a reader task (frames in, idle timeout, up to
//! `MAX_IN_FLIGHT` requests running at once) and a writer (the one place
//! that seals and writes, so sequence numbers follow the write order). Any
//! failure to open a frame poisons the session (`ChannelSession`) and closes
//! the connection.
//!
//! Connection slots: a connection that has not authenticated yet holds one
//! of `MAX_PREAUTH` pre-auth slots (a new one evicts the oldest, so idle
//! connections cannot lock the real host out; on Windows any local process
//! can open the pipe). Only a verified handshake counts against
//! `MAX_CONNECTIONS`, so authenticated sessions always have room. A session
//! registers only while its pairing is still in the registry, and closes
//! with it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mosaic_core::browser_channel::ChannelSession;
use mosaic_native_messaging::bodies::{BrowserKind, Hint};
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame, write_frame};
use mosaic_native_messaging::handshake::{read_client_message, server_handshake, write_message};
use mosaic_native_messaging::preauth::{ClientMessage, ServerMessage, new_pairing_code};
use mosaic_native_messaging::protocol::{
    ErrorCode as WireError, Event, HANDSHAKE_STEP_TIMEOUT_SECS, LockState, MAX_CONNECTIONS,
    MAX_IN_FLIGHT, PROTOCOL_VERSION, RequestEnvelope, error_json, to_zeroizing,
};
use mosaic_native_messaging::{Hex, HexKey};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::AbortHandle;
use tokio::time::timeout;
use zeroize::Zeroizing;

use super::ratelimit::Bucket;
use super::{Shared, handlers};
use crate::contract::browser::{BrowserCloseReason, BrowserFamily, BrowserHint};

/// What the writer of a session is told.
pub enum Out {
    /// A plaintext to seal and send.
    Frame(Zeroizing<Vec<u8>>),
    /// Close the connection (after everything queued before).
    Close,
}

/// The most connections that have not authenticated yet. A new one beyond
/// this evicts the oldest.
const MAX_PREAUTH: usize = 4;

struct ConnEntry {
    id: u64,
    pairing: [u8; 16],
    tx: UnboundedSender<Out>,
    /// Set once the reader task is spawned.
    reader: Option<AbortHandle>,
}

/// The open connections.
#[derive(Default)]
pub struct Conns {
    next: u64,
    /// Authenticated sessions being served (at most `MAX_CONNECTIONS`).
    open: usize,
    /// Connections that have not authenticated, oldest first.
    pre: Vec<(u64, AbortHandle)>,
    list: Vec<ConnEntry>,
}

/// Holds the slot of a connection while it is served.
struct ConnGuard {
    shared: Arc<Shared>,
    id: u64,
    authenticated: bool,
}

impl ConnGuard {
    /// The connection is not a handshake any more (a pairing prompt): it
    /// can no longer be evicted.
    fn leave_preauth(&self) {
        if let Ok(mut conns) = self.shared.conns.lock() {
            conns.pre.retain(|(id, _)| *id != self.id);
        }
    }

    /// A verified handshake: the connection counts against
    /// `MAX_CONNECTIONS`. `false`: that many sessions are open already.
    fn authenticate(&mut self) -> bool {
        let Ok(mut conns) = self.shared.conns.lock() else {
            return false;
        };
        if conns.open >= MAX_CONNECTIONS {
            return false;
        }
        conns.open += 1;
        conns.pre.retain(|(id, _)| *id != self.id);
        self.authenticated = true;
        true
    }
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        if let Ok(mut conns) = self.shared.conns.lock() {
            conns.pre.retain(|(id, _)| *id != self.id);
            if self.authenticated {
                conns.open = conns.open.saturating_sub(1);
            }
        }
    }
}

/// What a request task knows about its connection.
pub(super) struct Ctx {
    pub(super) shared: Arc<Shared>,
    pub(super) pairing: [u8; 16],
    pub(super) conn: u64,
}

const fn family(kind: BrowserKind) -> BrowserFamily {
    match kind {
        BrowserKind::Chromium => BrowserFamily::Chromium,
        BrowserKind::Firefox => BrowserFamily::Firefox,
    }
}

const fn browser_hint(hint: Hint) -> BrowserHint {
    match hint {
        Hint::Chrome => BrowserHint::Chrome,
        Hint::Edge => BrowserHint::Edge,
        Hint::Brave => BrowserHint::Brave,
        Hint::Opera => BrowserHint::Opera,
        Hint::Vivaldi => BrowserHint::Vivaldi,
        Hint::Chromium => BrowserHint::Chromium,
        Hint::Firefox => BrowserHint::Firefox,
        Hint::Other => BrowserHint::Other,
    }
}

impl Shared {
    /// Serves one connection on the async runtime.
    pub(crate) fn serve<S>(self: &Arc<Self>, stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let Some(id) = self.new_conn_id() else {
            return;
        };
        let (go, wait) = oneshot::channel::<()>();
        let shared = Arc::clone(self);
        let task = tokio::spawn(async move {
            // Starts once the connection holds a pre-auth slot.
            if wait.await.is_err() {
                return;
            }
            let mut guard = ConnGuard {
                shared: Arc::clone(&shared),
                id,
                authenticated: false,
            };
            let _ = handle_connection(&shared, stream, &mut guard).await;
        });
        if self.admit_preauth(id, task.abort_handle()) {
            let _ = go.send(());
        } else {
            task.abort();
        }
    }

    fn new_conn_id(&self) -> Option<u64> {
        let mut conns = self.conns.lock().ok()?;
        conns.next += 1;
        Some(conns.next)
    }

    /// Takes a pre-auth slot for connection `id`, evicting the oldest
    /// pre-auth connection when all are taken.
    fn admit_preauth(&self, id: u64, handle: AbortHandle) -> bool {
        let Ok(mut conns) = self.conns.lock() else {
            return false;
        };
        while conns.pre.len() >= MAX_PREAUTH {
            let (_, oldest) = conns.pre.remove(0);
            oldest.abort();
        }
        conns.pre.push((id, handle));
        true
    }

    /// Whether `MAX_CONNECTIONS` sessions are open.
    fn sessions_full(&self) -> bool {
        self.conns
            .lock()
            .map_or(true, |conns| conns.open >= MAX_CONNECTIONS)
    }

    /// Whether the pairing is still saved.
    pub(super) fn is_paired(&self, pairing: &[u8; 16]) -> bool {
        self.registry
            .lock()
            .is_ok_and(|registry| registry.get(pairing).is_some())
    }

    /// Whether session `conn` is registered.
    pub(super) fn conn_is_open(&self, conn: u64) -> bool {
        self.conns
            .lock()
            .is_ok_and(|conns| conns.list.iter().any(|c| c.id == conn))
    }

    /// The lock state, as the lock hooks last said.
    pub(super) fn lock_state(&self) -> LockState {
        if self.is_unlocked() {
            LockState::Unlocked
        } else {
            LockState::Locked
        }
    }

    /// Registers a session, unless its pairing was removed meanwhile (the
    /// registry is checked under the `conns` lock, so `close_pairing`
    /// reaches every session that gets in). Lock order: `conns`, then
    /// `registry`.
    fn register(&self, pairing: [u8; 16], tx: UnboundedSender<Out>) -> Option<u64> {
        let mut conns = self.conns.lock().ok()?;
        if !self.is_paired(&pairing) {
            return None;
        }
        conns.next += 1;
        let id = conns.next;
        conns.list.push(ConnEntry {
            id,
            pairing,
            tx,
            reader: None,
        });
        Some(id)
    }

    fn attach_reader(&self, id: u64, reader: AbortHandle) {
        if let Ok(mut conns) = self.conns.lock()
            && let Some(entry) = conns.list.iter_mut().find(|c| c.id == id)
        {
            entry.reader = Some(reader);
        }
    }

    fn unregister(&self, id: u64) {
        if let Ok(mut conns) = self.conns.lock() {
            conns.list.retain(|c| c.id != id);
        }
    }

    /// The pairings with an open session.
    pub(super) fn connected_pairings(&self) -> Vec<[u8; 16]> {
        self.conns
            .lock()
            .map(|c| c.list.iter().map(|e| e.pairing).collect())
            .unwrap_or_default()
    }

    fn send_all(&self, event: &Event) {
        let Ok(json) = to_zeroizing(event) else {
            return;
        };
        if let Ok(conns) = self.conns.lock() {
            for conn in &conns.list {
                let _ = conn.tx.send(Out::Frame(Zeroizing::new(json.to_vec())));
            }
        }
    }

    /// The vault locked: prompts close, the held password is dropped, the
    /// sessions are told (they stay up; every op but `status`, `focus` and
    /// `unpair` answers `locked`).
    pub(super) fn on_locked(&self) {
        self.unlocked.store(false, Ordering::SeqCst);
        self.drop_prompts(BrowserCloseReason::Locked);
        self.send_all(&Event::state(LockState::Locked));
    }

    /// The vault unlocked: the sessions are told.
    pub(super) fn on_unlocked(&self) {
        self.unlocked.store(true, Ordering::SeqCst);
        self.send_all(&Event::state(LockState::Unlocked));
    }

    /// Tells every session of `pairing` (except `except`) that it was
    /// removed, then closes it.
    pub(super) fn close_pairing(&self, pairing: &[u8; 16], except: Option<u64>) {
        let event = to_zeroizing(&Event::Unpaired {
            v: PROTOCOL_VERSION,
        })
        .ok();
        if let Ok(conns) = self.conns.lock() {
            for conn in conns
                .list
                .iter()
                .filter(|c| c.pairing == *pairing && Some(c.id) != except)
            {
                if let Some(json) = &event {
                    let _ = conn.tx.send(Out::Frame(Zeroizing::new(json.to_vec())));
                }
                let _ = conn.tx.send(Out::Close);
            }
        }
    }

    /// Closes every connection (the service stops).
    pub(super) fn close_all(&self) {
        if let Ok(mut conns) = self.conns.lock() {
            for conn in &conns.list {
                let _ = conn.tx.send(Out::Close);
                if let Some(reader) = &conn.reader {
                    reader.abort();
                }
            }
            for (_, handle) in conns.pre.drain(..) {
                handle.abort();
            }
        }
    }
}

/// Reads the first message and serves the connection accordingly.
async fn handle_connection<S>(
    shared: &Arc<Shared>,
    mut stream: S,
    guard: &mut ConnGuard,
) -> Result<(), NmError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match read_client_message(&mut stream).await? {
        ClientMessage::Hello {
            pairing_id, nonce, ..
        } => {
            if shared.sessions_full() {
                return write_message(&mut stream, &ServerMessage::refused(WireError::Busy)).await;
            }
            let key = shared
                .registry
                .lock()
                .ok()
                .and_then(|r| r.get(&pairing_id.0).map(|p| Arc::clone(&p.key)));
            // The first frame tells this state; `run_session` pushes a newer
            // one if the vault locked or unlocked during the handshake.
            let told = shared.lock_state();
            let session =
                server_handshake(&mut stream, &pairing_id.0, &nonce.0, key.as_deref(), told)
                    .await?;
            if !guard.authenticate() {
                // Another session took the last slot during the handshake.
                return Ok(());
            }
            run_session(shared, stream, session, pairing_id.0, told).await;
            Ok(())
        }
        ClientMessage::Status { .. } => {
            let state = ServerMessage::Status {
                v: PROTOCOL_VERSION,
                state: shared.lock_state(),
            };
            write_message(&mut stream, &state).await
        }
        ClientMessage::Focus { .. } => {
            let limited = shared
                .limiter
                .lock()
                .map_or(true, |mut l| !l.allow(Bucket::Focus, None, shared.now()));
            let reply = if limited {
                ServerMessage::refused(WireError::RateLimited)
            } else {
                if let Some(backend) = shared.backend() {
                    backend.focus_window();
                }
                ServerMessage::Focused {
                    v: PROTOCOL_VERSION,
                }
            };
            write_message(&mut stream, &reply).await
        }
        ClientMessage::PairRequest { browser, hint, .. } => {
            // The prompt may stay open for minutes: not evictable.
            guard.leave_preauth();
            pair_flow(shared, stream, family(browser), browser_hint(hint)).await
        }
        ClientMessage::Finish { .. } => {
            write_message(
                &mut stream,
                &ServerMessage::refused(WireError::InvalidRequest),
            )
            .await
        }
    }
}

/// A `pairRequest`: show the prompt, wait for the user, answer.
async fn pair_flow<S>(
    shared: &Arc<Shared>,
    mut stream: S,
    browser: BrowserFamily,
    hint: BrowserHint,
) -> Result<(), NmError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let code = new_pairing_code()?;
    let (request_id, rx) = match shared.open_pair(browser, hint, &code) {
        Ok(opened) => opened,
        Err(reason) => {
            return write_message(&mut stream, &ServerMessage::refused(reason)).await;
        }
    };
    let pending = ServerMessage::PairPending {
        v: PROTOCOL_VERSION,
        code,
        expires_in_seconds: u32::try_from(shared.limits.pair_timeout.as_secs()).unwrap_or(u32::MAX),
    };
    if let Err(e) = write_message(&mut stream, &pending).await {
        shared.close_pair(&request_id, BrowserCloseReason::Disconnected);
        return Err(e);
    }
    // The host hangs up if the extension gives up: end the prompt then.
    let (mut reader, mut writer) = tokio::io::split(stream);
    let watcher = {
        let (shared, request_id) = (Arc::clone(shared), request_id.clone());
        tokio::spawn(async move {
            let mut byte = [0u8; 1];
            let _ = reader.read(&mut byte).await;
            shared.close_pair(&request_id, BrowserCloseReason::Disconnected);
        })
    };
    let outcome = shared.await_pair(&request_id, rx).await;
    watcher.abort();
    let approved = outcome.as_ref().ok().map(|(id, _)| *id);
    let reply = match outcome {
        Ok((id, key)) => ServerMessage::PairApproved {
            v: PROTOCOL_VERSION,
            pairing_id: Hex(id),
            key: HexKey(key),
        },
        Err(reason) => ServerMessage::refused(reason),
    };
    let written = write_message(&mut writer, &reply).await;
    drop(reply);
    if written.is_err()
        && let Some(id) = approved
    {
        // The browser never got the key: nobody can use this pairing.
        let _ = shared.remove_pairing_blocking(id, None).await;
    }
    written?;
    let _ = writer.shutdown().await;
    Ok(())
}

/// The session: reader task, request tasks, and the writer in this task.
/// `told` is the lock state the first frame reported.
async fn run_session<S>(
    shared: &Arc<Shared>,
    stream: S,
    session: ChannelSession,
    pairing: [u8; 16],
    told: LockState,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let session = Arc::new(Mutex::new(session));
    let (tx, rx) = unbounded_channel();
    let Some(id) = shared.register(pairing, tx.clone()) else {
        // The pairing was removed during the handshake: say so and close.
        if let Ok(json) = to_zeroizing(&Event::Unpaired {
            v: PROTOCOL_VERSION,
        }) {
            let _ = tx.send(Out::Frame(json));
        }
        let _ = tx.send(Out::Close);
        writer_loop(writer, rx, session).await;
        return;
    };
    let task = tokio::spawn(reader_loop(
        reader,
        Arc::clone(&session),
        tx.clone(),
        Arc::clone(shared),
        pairing,
        id,
    ));
    shared.attach_reader(id, task.abort_handle());
    let now = shared.lock_state();
    if now != told {
        // The vault locked or unlocked while the handshake ran.
        if let Ok(json) = to_zeroizing(&Event::state(now)) {
            let _ = tx.send(Out::Frame(json));
        }
    }
    drop(tx);
    shared.pairings_changed();
    writer_loop(writer, rx, session).await;
    task.abort();
    shared.unregister(id);
    // A save prompt of this connection has nobody to answer to.
    shared.close_saves_where(BrowserCloseReason::Disconnected, |save| save.conn == id);
    shared.pairings_changed();
}

/// Seals and writes what the session queued, in order.
async fn writer_loop<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut rx: UnboundedReceiver<Out>,
    session: Arc<Mutex<ChannelSession>>,
) {
    let step = Duration::from_secs(HANDSHAKE_STEP_TIMEOUT_SECS);
    while let Some(out) = rx.recv().await {
        let Out::Frame(plain) = out else {
            break;
        };
        let sealed = session.lock().ok().map(|mut s| s.seal(&plain));
        let Some(Ok(frame)) = sealed else {
            break;
        };
        let written = timeout(step, write_frame(&mut writer, ByteOrder::Big, &frame)).await;
        if !matches!(written, Ok(Ok(()))) {
            break;
        }
    }
    let _ = writer.shutdown().await;
}

/// Reads and opens frames and starts the request tasks. Ends the session
/// on a closed connection, the idle timeout, or any frame that does not
/// open.
async fn reader_loop<R: AsyncRead + Unpin>(
    mut reader: R,
    session: Arc<Mutex<ChannelSession>>,
    tx: UnboundedSender<Out>,
    shared: Arc<Shared>,
    pairing: [u8; 16],
    conn: u64,
) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    loop {
        let read = timeout(shared.limits.idle, read_frame(&mut reader, ByteOrder::Big)).await;
        let Ok(Ok(Some(frame))) = read else {
            break;
        };
        let opened = session.lock().ok().map(|mut s| s.open(&frame));
        let Some(Ok(plain)) = opened else {
            break;
        };
        drop(frame);
        let ctx = Ctx {
            shared: Arc::clone(&shared),
            pairing,
            conn,
        };
        if !start_request(plain, ctx, &tx, &in_flight) {
            break;
        }
    }
    let _ = tx.send(Out::Close);
}

/// Answers `busy` or starts a task for one request. `false`: the plaintext
/// is not a request envelope at all, so the peer is not the host.
fn start_request(
    plain: mosaic_core::SecretVec,
    ctx: Ctx,
    tx: &UnboundedSender<Out>,
    in_flight: &Arc<AtomicUsize>,
) -> bool {
    let id = match serde_json::from_slice::<RequestEnvelope<'_>>(plain.expose_secret()) {
        Ok(envelope) => envelope.id,
        Err(_) => return false,
    };
    if in_flight.fetch_add(1, Ordering::SeqCst) >= MAX_IN_FLIGHT {
        in_flight.fetch_sub(1, Ordering::SeqCst);
        if let Ok(json) = error_json(id, WireError::Busy) {
            let _ = tx.send(Out::Frame(json));
        }
        return true;
    }
    let (tx, in_flight) = (tx.clone(), Arc::clone(in_flight));
    tokio::spawn(async move {
        let done = handlers::run(&ctx, plain.expose_secret()).await;
        drop(plain);
        let _ = tx.send(Out::Frame(done.response));
        if done.close {
            let _ = tx.send(Out::Close);
        }
        in_flight.fetch_sub(1, Ordering::SeqCst);
    });
    true
}
