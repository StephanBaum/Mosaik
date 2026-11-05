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

struct ConnEntry {
    id: u64,
    pairing: [u8; 16],
    tx: UnboundedSender<Out>,
    reader: AbortHandle,
}

/// The open connections.
#[derive(Default)]
pub struct Conns {
    next: u64,
    /// Connections being served, authenticated or not.
    open: usize,
    list: Vec<ConnEntry>,
}

/// Counts a connection while it is served.
struct ConnGuard(Arc<Shared>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        if let Ok(mut conns) = self.0.conns.lock() {
            conns.open = conns.open.saturating_sub(1);
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
        let Some(guard) = self.admit() else {
            // More than MAX_CONNECTIONS: accepted and closed.
            return;
        };
        let shared = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let _guard = guard;
            let _ = handle_connection(&shared, stream).await;
        });
    }

    fn admit(self: &Arc<Self>) -> Option<ConnGuard> {
        let mut conns = self.conns.lock().ok()?;
        if conns.open >= MAX_CONNECTIONS {
            return None;
        }
        conns.open += 1;
        Some(ConnGuard(Arc::clone(self)))
    }

    /// The lock state, as the lock hooks last said.
    pub(super) fn lock_state(&self) -> LockState {
        if self.is_unlocked() {
            LockState::Unlocked
        } else {
            LockState::Locked
        }
    }

    fn register(&self, pairing: [u8; 16], tx: UnboundedSender<Out>, reader: AbortHandle) -> u64 {
        let Ok(mut conns) = self.conns.lock() else {
            return 0;
        };
        conns.next += 1;
        let id = conns.next;
        conns.list.push(ConnEntry {
            id,
            pairing,
            tx,
            reader,
        });
        id
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

    /// Closes every session (the service stops).
    pub(super) fn close_all(&self) {
        if let Ok(conns) = self.conns.lock() {
            for conn in &conns.list {
                let _ = conn.tx.send(Out::Close);
                conn.reader.abort();
            }
        }
    }
}

/// Reads the first message and serves the connection accordingly.
async fn handle_connection<S>(shared: &Arc<Shared>, mut stream: S) -> Result<(), NmError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match read_client_message(&mut stream).await? {
        ClientMessage::Hello {
            pairing_id, nonce, ..
        } => {
            let key = shared
                .registry
                .lock()
                .ok()
                .and_then(|r| r.get(&pairing_id.0).map(|p| Arc::clone(&p.key)));
            let session = server_handshake(
                &mut stream,
                &pairing_id.0,
                &nonce.0,
                key.as_deref(),
                shared.lock_state(),
            )
            .await?;
            run_session(shared, stream, session, pairing_id.0).await;
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
    let reply = match outcome {
        Ok((id, key)) => ServerMessage::PairApproved {
            v: PROTOCOL_VERSION,
            pairing_id: Hex(id),
            key: HexKey(key),
        },
        Err(reason) => ServerMessage::refused(reason),
    };
    write_message(&mut writer, &reply).await?;
    let _ = writer.shutdown().await;
    Ok(())
}

/// The session: reader task, request tasks, and the writer in this task.
async fn run_session<S>(shared: &Arc<Shared>, stream: S, session: ChannelSession, pairing: [u8; 16])
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (reader, writer) = tokio::io::split(stream);
    let session = Arc::new(Mutex::new(session));
    let (tx, rx) = unbounded_channel();
    let conn = Arc::new(Mutex::new(0u64));
    let task = tokio::spawn(reader_loop(
        reader,
        Arc::clone(&session),
        tx.clone(),
        Arc::clone(shared),
        pairing,
        Arc::clone(&conn),
    ));
    let id = shared.register(pairing, tx, task.abort_handle());
    if let Ok(mut slot) = conn.lock() {
        *slot = id;
    }
    shared.pairings_changed();
    writer_loop(writer, rx, session).await;
    task.abort();
    shared.unregister(id);
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
    conn: Arc<Mutex<u64>>,
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
            conn: conn.lock().map_or(0, |c| *c),
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
