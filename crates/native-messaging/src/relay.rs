//! The host's relay: stdio on one side, the desktop channel on the other
//! (`docs/NATIVE_MESSAGING.md` §2).
//!
//! The host is a small, dumb pipe. It parses the envelope of each request,
//! forwards the allowlisted ops ([`FORWARDED_OPS`]) without looking into the
//! body, answers `hello` and `pair` itself, and relays the desktop's
//! responses and events back verbatim. It keeps `K_pair` in memory for its
//! lifetime (as a [`PairingKey`]) and reconnects with a fresh handshake when
//! the desktop went away. It never launches the desktop and holds no vault
//! data.
//!
//! Concurrency: one task reads stdin, one writes stdout, one reads the
//! desktop connection, and one main loop owns all state and the session's
//! sending half. They talk over channels; there is no `select!`.

use core::time::Duration;
use std::collections::{HashMap, HashSet};
use std::future::Future;

use mosaic_core::browser_channel::{ChannelSession, PAIRING_ID_LEN, PairingKey};
use serde::Serialize;
use subtle::ConstantTimeEq;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use zeroize::Zeroizing;

use crate::bodies::{
    BrowserKind, DesktopState, Empty, HelloBody, HelloResult, Hint, PairBody, PairResult,
    StatusResult,
};
use crate::endpoint::{Endpoint, default_endpoint};
use crate::error::Error;
use crate::framing::{ByteOrder, read_frame, write_frame};
use crate::handshake::{
    client_handshake, next_frame, preauth_request, read_server_message, send, step, write_message,
};
use crate::hex::{Hex, HexKey};
use crate::ids::Caller;
use crate::preauth::{ClientMessage, ServerMessage, is_pairing_code};
use crate::protocol::{
    DesktopMessage, ErrorCode, Event, LockState, MAX_FRAME_BYTES, Op, PAIR_TIMEOUT_SECS,
    PASSKEY_PROMPT_TIMEOUT_SECS, PROTOCOL_VERSION, REQUEST_TIMEOUT_SECS, RequestEnvelope,
    SAVE_PROMPT_TIMEOUT_SECS, error_json, ok_json, request_json, to_zeroizing,
};
use crate::transport;

/// Seconds a pairing request may wait for the user, with a little slack over
/// the desktop's own expiry so its `pairingExpired` arrives first.
const PAIR_WAIT: Duration = Duration::from_secs(PAIR_TIMEOUT_SECS + 5);

/// How long the host waits for the desktop's answer to a forwarded request. `proposeSave`
/// waits for the user in the desktop's save dialog (up to [`SAVE_PROMPT_TIMEOUT_SECS`]), so it
/// gets that window plus a margin; the host still answers before the extension gives up. The
/// passkey ops wait for the desktop's passkey dialog the same way ([`PASSKEY_PROMPT_TIMEOUT_SECS`]).
fn request_timeout(op: Op) -> Duration {
    match op {
        Op::ProposeSave => Duration::from_secs(SAVE_PROMPT_TIMEOUT_SECS + 2),
        Op::PasskeyCreate | Op::PasskeyGet => Duration::from_secs(PASSKEY_PROMPT_TIMEOUT_SECS + 2),
        _ => Duration::from_secs(REQUEST_TIMEOUT_SECS),
    }
}

/// Request IDs a connection remembers after their request timed out. A
/// connection that would need more is closed.
pub(crate) const MAX_TOMBSTONES: usize = 1024;

/// Bytes an AEAD frame adds to its plaintext: header and tag.
const FRAME_OVERHEAD: usize = mosaic_core::browser_channel::FRAME_HEADER_LEN + 16;

/// Opens connections to the desktop.
pub trait Connect: Clone + Send + Sync + 'static {
    /// The connection type.
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    /// Connects, within the connect timeout.
    fn connect(&self) -> impl Future<Output = Result<Self::Stream, Error>> + Send;
}

/// Where the host finds the desktop.
#[derive(Clone, Debug)]
pub enum EndpointSource {
    /// This endpoint.
    Fixed(Endpoint),
    /// [`default_endpoint`], resolved at every connect (the Windows pipe
    /// name changes when the desktop restarts).
    Default,
}

/// The connector of the real endpoint.
#[derive(Clone, Debug)]
pub struct PlatformConnector(pub EndpointSource);

impl Connect for PlatformConnector {
    type Stream = transport::ClientStream;

    async fn connect(&self) -> Result<Self::Stream, Error> {
        let endpoint = match &self.0 {
            EndpointSource::Fixed(endpoint) => endpoint.clone(),
            EndpointSource::Default => default_endpoint()?,
        };
        transport::connect(&endpoint).await
    }
}

/// Runs the host until stdin ends (`Ok`) or a browser frame is malformed.
///
/// # Errors
/// [`Error::FrameEmpty`], [`Error::FrameTooLarge`], [`Error::FrameTruncated`]
/// or [`Error::Malformed`] for a malformed frame or envelope from the
/// browser, [`Error::Io`] if stdin fails.
pub async fn run_host<R, W>(
    caller: Caller,
    stdin: R,
    stdout: W,
    endpoint: EndpointSource,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    run_with(caller, stdin, stdout, PlatformConnector(endpoint)).await
}

/// [`run_host`] over any [`Connect`].
///
/// # Errors
/// As [`run_host`].
pub async fn run_with<R, W, C>(
    caller: Caller,
    stdin: R,
    stdout: W,
    connector: C,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
    C: Connect,
{
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (out_tx, out_rx) = mpsc::unbounded_channel();
    tokio::spawn(stdin_task(stdin, tx.clone()));
    let writer = tokio::spawn(stdout_task(stdout, out_rx));
    let mut host = Host::new(caller.browser(), connector, tx, out_tx);
    let mut result = Ok(());
    while let Some(msg) = rx.recv().await {
        match host.handle(msg).await {
            Ok(Flow::Continue) => {}
            Ok(Flow::Stop) | Err(Error::Closed) => break,
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }
    // Let queued output reach the browser before the process ends.
    drop(host);
    let _ = writer.await;
    result
}

enum Flow {
    Continue,
    Stop,
}

/// What the tasks tell the main loop.
enum Msg {
    Browser(Zeroizing<Vec<u8>>),
    BrowserEnd(Result<(), Error>),
    Desktop {
        conn: u64,
        frame: Zeroizing<Vec<u8>>,
    },
    DesktopEnd {
        conn: u64,
    },
    Timeout {
        conn: u64,
        id: u32,
    },
    Paired {
        id: [u8; PAIRING_ID_LEN],
        key: PairingKey,
    },
}

async fn stdin_task<R: AsyncRead + Unpin>(mut stdin: R, tx: mpsc::UnboundedSender<Msg>) {
    loop {
        match read_frame(&mut stdin, ByteOrder::Native).await {
            Ok(Some(frame)) => {
                if tx.send(Msg::Browser(frame)).is_err() {
                    return;
                }
            }
            Ok(None) => {
                let _ = tx.send(Msg::BrowserEnd(Ok(())));
                return;
            }
            Err(e) => {
                let _ = tx.send(Msg::BrowserEnd(Err(e)));
                return;
            }
        }
    }
}

async fn stdout_task<W: AsyncWrite + Unpin>(
    mut stdout: W,
    mut messages: mpsc::UnboundedReceiver<Zeroizing<Vec<u8>>>,
) {
    while let Some(message) = messages.recv().await {
        if write_frame(&mut stdout, ByteOrder::Native, &message)
            .await
            .is_err()
        {
            return;
        }
    }
}

fn spawn_reader<S>(conn: u64, mut reader: ReadHalf<S>, tx: mpsc::UnboundedSender<Msg>)
where
    S: AsyncRead + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            let Ok(Some(frame)) = read_frame(&mut reader, ByteOrder::Big).await else {
                let _ = tx.send(Msg::DesktopEnd { conn });
                return;
            };
            if tx.send(Msg::Desktop { conn, frame }).is_err() {
                return;
            }
        }
    });
}

/// A stored pairing.
struct Pairing {
    id: [u8; PAIRING_ID_LEN],
    key: PairingKey,
}

/// An authenticated connection.
struct Active<S> {
    conn: u64,
    writer: WriteHalf<S>,
    session: ChannelSession,
    state: LockState,
    /// IDs of requests that timed out on this connection. They stay
    /// unusable for its lifetime, so a late answer from the desktop can
    /// never be taken for the answer to a request that reused the ID.
    tombstones: HashSet<u32>,
}

struct Host<C: Connect> {
    browser: BrowserKind,
    connector: C,
    tx: mpsc::UnboundedSender<Msg>,
    out: mpsc::UnboundedSender<Zeroizing<Vec<u8>>>,
    hint: Hint,
    pairing: Option<Pairing>,
    active: Option<Active<C::Stream>>,
    /// Request ID to the connection it was sent on.
    pending: HashMap<u32, u64>,
    next_conn: u64,
}

impl<C: Connect> Host<C> {
    fn new(
        browser: BrowserKind,
        connector: C,
        tx: mpsc::UnboundedSender<Msg>,
        out: mpsc::UnboundedSender<Zeroizing<Vec<u8>>>,
    ) -> Self {
        Self {
            browser,
            connector,
            tx,
            out,
            hint: Hint::Other,
            pairing: None,
            active: None,
            pending: HashMap::new(),
            next_conn: 0,
        }
    }

    async fn handle(&mut self, msg: Msg) -> Result<Flow, Error> {
        match msg {
            Msg::Browser(frame) => self.on_browser(&frame).await?,
            Msg::BrowserEnd(Ok(())) => return Ok(Flow::Stop),
            Msg::BrowserEnd(Err(e)) => return Err(e),
            Msg::Desktop { conn, frame } => self.on_desktop(conn, &frame)?,
            Msg::DesktopEnd { conn } => {
                if self.active.as_ref().is_some_and(|a| a.conn == conn) {
                    self.desktop_lost(ErrorCode::DesktopNotRunning)?;
                }
            }
            Msg::Timeout { conn, id } => {
                if self.pending.get(&id) == Some(&conn) {
                    self.pending.remove(&id);
                    self.respond_err(id, ErrorCode::Timeout)?;
                    self.tombstone(conn, id)?;
                }
            }
            Msg::Paired { id, key } => self.on_paired(id, key).await?,
        }
        Ok(Flow::Continue)
    }

    // ----- output ---------------------------------------------------------

    fn emit(&self, message: Zeroizing<Vec<u8>>) -> Result<(), Error> {
        self.out.send(message).map_err(|_| Error::Closed)
    }

    fn respond_ok<T: Serialize>(&self, id: u32, body: &T) -> Result<(), Error> {
        self.emit(ok_json(id, body)?)
    }

    fn respond_err(&self, id: u32, code: ErrorCode) -> Result<(), Error> {
        self.emit(error_json(id, code)?)
    }

    fn emit_event(&self, event: &Event) -> Result<(), Error> {
        self.emit(to_zeroizing(event)?)
    }

    /// Remembers `id` as timed out on `conn`. A connection with more than
    /// [`MAX_TOMBSTONES`] of them is closed.
    fn tombstone(&mut self, conn: u64, id: u32) -> Result<(), Error> {
        let Some(active) = self.active.as_mut().filter(|a| a.conn == conn) else {
            return Ok(());
        };
        active.tombstones.insert(id);
        if active.tombstones.len() > MAX_TOMBSTONES {
            self.desktop_lost(ErrorCode::DesktopNotRunning)?;
        }
        Ok(())
    }

    // ----- requests from the browser -------------------------------------

    /// `frame` is the zeroizing buffer of the browser's frame; the envelope
    /// borrows its body from it and never copies it.
    async fn on_browser(&mut self, frame: &[u8]) -> Result<(), Error> {
        let request: RequestEnvelope<'_> =
            serde_json::from_slice(frame).map_err(|_| Error::Malformed)?;
        let id = request.id;
        if request.v != PROTOCOL_VERSION {
            return self.respond_err(id, ErrorCode::ProtocolVersion);
        }
        match Op::from_wire(&request.op) {
            Some(Op::Hello) => self.on_hello(id, request.body.get()).await,
            Some(Op::Pair) => self.on_pair(id, request.body.get()),
            Some(op) if op.is_forwarded() => self.forward(id, op, request.body).await,
            _ => self.respond_err(id, ErrorCode::InvalidRequest),
        }
    }

    async fn on_hello(&mut self, id: u32, body: &str) -> Result<(), Error> {
        let Ok(body) = serde_json::from_str::<HelloBody>(body) else {
            return self.respond_err(id, ErrorCode::InvalidRequest);
        };
        self.hint = body.hint;
        self.set_pairing(body.pairing.map(|p| Pairing {
            id: p.pairing_id.0,
            key: p.key.0,
        }));
        match self.hello_result().await {
            Ok(result) => self.respond_ok(id, &result),
            Err(code) => self.respond_err(id, code),
        }
    }

    /// Replaces the stored pairing. A different pairing ends the session; the
    /// same one keeps it.
    fn set_pairing(&mut self, new: Option<Pairing>) {
        let same = match (&self.pairing, &new) {
            (Some(a), Some(b)) => a.id == b.id && bool::from(a.key.ct_eq(&b.key)),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.active = None;
            self.pending.clear();
        }
        if !(same && self.pairing.is_some()) {
            self.pairing = new;
        }
    }

    async fn hello_result(&mut self) -> Result<HelloResult, ErrorCode> {
        if self.pairing.is_none() {
            let desktop = self.preauth_status().await;
            return Ok(HelloResult {
                desktop,
                paired: false,
            });
        }
        match self.ensure_session(false).await {
            Ok(()) => Ok(HelloResult {
                desktop: self
                    .active
                    .as_ref()
                    .map_or(DesktopState::NotRunning, |a| a.state.into()),
                paired: true,
            }),
            Err(Error::Refused(ErrorCode::NotPaired)) => Ok(HelloResult {
                desktop: self.preauth_status().await,
                paired: false,
            }),
            Err(e @ (Error::NotAuthentic | Error::Channel(_) | Error::Refused(_))) => {
                Err(e.wire_code())
            }
            Err(_) => Ok(HelloResult {
                desktop: DesktopState::NotRunning,
                paired: true,
            }),
        }
    }

    /// The lock state through the unauthenticated `status` message.
    async fn preauth_status(&self) -> DesktopState {
        match self
            .preauth(ClientMessage::Status {
                v: PROTOCOL_VERSION,
            })
            .await
        {
            Ok(ServerMessage::Status { state, .. }) => state.into(),
            _ => DesktopState::NotRunning,
        }
    }

    /// One unauthenticated exchange on its own connection.
    async fn preauth(&self, request: ClientMessage) -> Result<ServerMessage, Error> {
        let mut stream = self.connector.connect().await?;
        preauth_request(&mut stream, &request).await
    }

    async fn forward(
        &mut self,
        id: u32,
        op: Op,
        body: &serde_json::value::RawValue,
    ) -> Result<(), Error> {
        let timed_out = self
            .active
            .as_ref()
            .is_some_and(|a| a.tombstones.contains(&id));
        if self.pending.contains_key(&id) || timed_out {
            return self.respond_err(id, ErrorCode::InvalidRequest);
        }
        if self.pairing.is_none() {
            return self.forward_unpaired(id, op).await;
        }
        if let Err(e) = self.ensure_session(true).await {
            if e == Error::Refused(ErrorCode::NotPaired) {
                return self.forward_unpaired(id, op).await;
            }
            return self.respond_err(id, e.wire_code());
        }
        let plaintext = request_json(id, op, body)?;
        if plaintext.len() + FRAME_OVERHEAD > MAX_FRAME_BYTES {
            return self.respond_err(id, ErrorCode::FrameTooLarge);
        }
        let Some(active) = self.active.as_mut() else {
            return self.respond_err(id, ErrorCode::DesktopNotRunning);
        };
        let conn = active.conn;
        if send(&mut active.writer, &mut active.session, &plaintext)
            .await
            .is_err()
        {
            self.desktop_lost(ErrorCode::DesktopNotRunning)?;
            return self.respond_err(id, ErrorCode::DesktopNotRunning);
        }
        self.pending.insert(id, conn);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            sleep(request_timeout(op)).await;
            let _ = tx.send(Msg::Timeout { conn, id });
        });
        Ok(())
    }

    /// `status` and `focus` work without a pairing, through the pre-auth
    /// messages; everything else needs one.
    async fn forward_unpaired(&mut self, id: u32, op: Op) -> Result<(), Error> {
        let request = match op {
            Op::Status => ClientMessage::Status {
                v: PROTOCOL_VERSION,
            },
            Op::Focus => ClientMessage::Focus {
                v: PROTOCOL_VERSION,
            },
            _ => return self.respond_err(id, ErrorCode::NotPaired),
        };
        match self.preauth(request).await {
            Ok(ServerMessage::Status { state, .. }) if op == Op::Status => {
                self.respond_ok(id, &StatusResult { state })
            }
            Ok(ServerMessage::Focused { .. }) if op == Op::Focus => self.respond_ok(id, &Empty {}),
            Ok(_) => self.respond_err(id, ErrorCode::DesktopNotAuthentic),
            Err(e) => self.respond_err(id, e.wire_code()),
        }
    }

    // ----- the session ----------------------------------------------------

    /// Makes sure an authenticated connection exists, connecting and
    /// handshaking if needed. `announce` pushes a `state` event for it.
    async fn ensure_session(&mut self, announce: bool) -> Result<(), Error> {
        if self.active.is_some() {
            return Ok(());
        }
        let Some(pairing) = self.pairing.as_ref() else {
            return Err(Error::Refused(ErrorCode::NotPaired));
        };
        let mut stream = self.connector.connect().await?;
        let (session, state) = client_handshake(&mut stream, &pairing.key, &pairing.id).await?;
        let (reader, writer) = tokio::io::split(stream);
        self.next_conn += 1;
        let conn = self.next_conn;
        spawn_reader(conn, reader, self.tx.clone());
        self.active = Some(Active {
            conn,
            writer,
            session,
            state,
            tombstones: HashSet::new(),
        });
        if announce {
            self.emit_event(&Event::state(state))?;
        }
        Ok(())
    }

    /// The desktop went away or failed: close the session, fail every request
    /// in flight and tell the extension.
    fn desktop_lost(&mut self, code: ErrorCode) -> Result<(), Error> {
        self.active = None;
        let mut ids: Vec<u32> = self.pending.drain().map(|(id, _)| id).collect();
        ids.sort_unstable();
        for id in ids {
            self.respond_err(id, code)?;
        }
        self.emit_event(&Event::Disconnected {
            v: PROTOCOL_VERSION,
        })
    }

    fn on_desktop(&mut self, conn: u64, frame: &[u8]) -> Result<(), Error> {
        let Some(active) = self.active.as_mut().filter(|a| a.conn == conn) else {
            return Ok(());
        };
        let Ok(plaintext) = active.session.open(frame) else {
            return self.desktop_lost(ErrorCode::DesktopNotAuthentic);
        };
        let Ok(message) = DesktopMessage::parse(plaintext.expose_secret()) else {
            return self.desktop_lost(ErrorCode::DesktopNotAuthentic);
        };
        let verbatim = Zeroizing::new(plaintext.expose_secret().to_vec());
        match message {
            DesktopMessage::Response(response) => {
                // A late answer to a request that timed out is dropped.
                if !active.tombstones.contains(&response.id)
                    && self.pending.remove(&response.id).is_some()
                {
                    self.emit(verbatim)?;
                }
            }
            DesktopMessage::Event(Event::State { state, .. }) => {
                active.state = state;
                self.emit(verbatim)?;
            }
            DesktopMessage::Event(Event::Unpaired { .. }) => {
                self.emit(verbatim)?;
                self.pairing = None;
                self.active = None;
                self.pending.clear();
            }
            // The host's own events never come from the desktop.
            DesktopMessage::Event(_) => {}
        }
        Ok(())
    }

    // ----- pairing --------------------------------------------------------

    fn on_pair(&self, id: u32, body: &str) -> Result<(), Error> {
        let Ok(body) = serde_json::from_str::<PairBody>(body) else {
            return self.respond_err(id, ErrorCode::InvalidRequest);
        };
        tokio::spawn(pair_task(
            self.connector.clone(),
            self.browser,
            body.hint,
            id,
            self.out.clone(),
            self.tx.clone(),
        ));
        Ok(())
    }

    /// A pairing was approved: remember it and connect with it.
    async fn on_paired(&mut self, id: [u8; PAIRING_ID_LEN], key: PairingKey) -> Result<(), Error> {
        self.set_pairing(Some(Pairing { id, key }));
        // Best effort: the next request connects again if this fails.
        match self.ensure_session(true).await {
            Err(Error::Closed) => Err(Error::Closed),
            _ => Ok(()),
        }
    }
}

/// The pairing flow on its own connection, so the main loop stays free while
/// the user decides.
async fn pair_task<C: Connect>(
    connector: C,
    browser: BrowserKind,
    hint: Hint,
    id: u32,
    out: mpsc::UnboundedSender<Zeroizing<Vec<u8>>>,
    tx: mpsc::UnboundedSender<Msg>,
) {
    let outcome = pair_flow(&connector, browser, hint, id, &out).await;
    let response = match outcome {
        Ok((pairing_id, key)) => {
            let body = PairResult {
                pairing_id: Hex(pairing_id),
                key: HexKey(key),
            };
            let response = ok_json(id, &body);
            // Hand the key to the main loop (it needs its own copy).
            let HexKey(key) = body.key;
            let _ = tx.send(Msg::Paired {
                id: pairing_id,
                key,
            });
            response
        }
        Err(e) => error_json(id, e.wire_code()),
    };
    if let Ok(response) = response {
        let _ = out.send(response);
    }
}

async fn pair_flow(
    connector: &impl Connect,
    browser: BrowserKind,
    hint: Hint,
    request_id: u32,
    out: &mpsc::UnboundedSender<Zeroizing<Vec<u8>>>,
) -> Result<([u8; PAIRING_ID_LEN], PairingKey), Error> {
    let mut stream = connector.connect().await?;
    let request = ClientMessage::PairRequest {
        v: PROTOCOL_VERSION,
        browser,
        hint,
    };
    step(write_message(&mut stream, &request)).await?;
    match read_server_message(&mut stream).await? {
        ServerMessage::PairPending {
            code,
            expires_in_seconds,
            ..
        } => {
            if !is_pairing_code(&code) || u64::from(expires_in_seconds) > PAIR_TIMEOUT_SECS {
                return Err(Error::Malformed);
            }
            let event = Event::PairCode {
                v: PROTOCOL_VERSION,
                request_id,
                code,
                expires_in_seconds,
            };
            out.send(to_zeroizing(&event)?).map_err(|_| Error::Closed)?;
        }
        ServerMessage::Refused { reason, .. } => return Err(Error::Refused(reason)),
        _ => return Err(Error::Malformed),
    }
    let frame = timeout(PAIR_WAIT, next_frame(&mut stream))
        .await
        .map_err(|_| Error::Timeout)??;
    match ServerMessage::parse(&frame)? {
        ServerMessage::PairApproved {
            pairing_id, key, ..
        } => Ok((pairing_id.0, key.0)),
        ServerMessage::Refused { reason, .. } => Err(Error::Refused(reason)),
        _ => Err(Error::Malformed),
    }
}

#[cfg(test)]
mod tests;
