//! The client side of the agent endpoint: connect, handshake, and a multiplexed request channel
//! (`docs/NATIVE_MESSAGING.md` section 9). The endpoint is re-read on every connect, so a desktop
//! that restarted under a new pipe name is found again.
//!
//! A [`Connection`] runs a reader task on the current `LocalSet`: it routes each response to the
//! request with its ID, notes the `unpaired` event, and marks the connection
//! closed when the stream ends. Several requests can be in flight at once (an access dialog that
//! waits for the user does not block a `listSecrets`).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

use mosaic_agent::error::AgentErrorCode;
use mosaic_agent::wire::{
    DesktopFrame, Event, OpRequest, OpResponse, RequestEnvelope, ResponseEnvelope, WireError,
};
use mosaic_core::browser_channel::ChannelSession;
use mosaic_native_messaging::agent_handshake::agent_client_handshake;
use mosaic_native_messaging::agent_preauth::AgentRefusal;
use mosaic_native_messaging::endpoint::{ChannelKind, Endpoint, default_endpoint_for};
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame, write_frame};
use mosaic_native_messaging::transport::{ClientStream, connect as transport_connect};
use mosaic_platform::keystore::AgentPairingStore;
use tokio::io::{AsyncWriteExt as _, WriteHalf};
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use zeroize::Zeroizing;

/// A boxed future that need not be `Send` (everything runs on one thread).
pub type LocalFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// How long a frame write may take.
const WRITE_STEP: Duration = Duration::from_secs(5);

/// Why there is no connection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnError {
    /// Nothing serves the endpoint: the desktop is not running (or its agent endpoint is off).
    DesktopNotRunning,
    /// No pairing stored under this name, or the desktop does not know it.
    NotPaired,
    /// The vault is locked.
    Locked,
    /// Agent access is off.
    AgentAccessOff,
    /// Another refusal of the desktop.
    Refused(AgentRefusal),
    /// Any other failure of the link.
    Io,
}

impl From<NmError> for ConnError {
    fn from(e: NmError) -> Self {
        match e {
            NmError::DesktopNotRunning => Self::DesktopNotRunning,
            NmError::AgentRefused(AgentRefusal::NotPaired) => Self::NotPaired,
            NmError::AgentRefused(AgentRefusal::Locked) => Self::Locked,
            NmError::AgentRefused(AgentRefusal::AgentAccessOff) => Self::AgentAccessOff,
            NmError::AgentRefused(other) => Self::Refused(other),
            _ => Self::Io,
        }
    }
}

/// Why a request got no answer body.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RequestError {
    /// The desktop answered with an error.
    Wire(WireError),
    /// The link failed or closed.
    Conn(ConnError),
    /// No answer within the time given.
    Timeout,
}

/// The endpoint of the agent channel (re-read on every call).
///
/// # Errors
/// As `default_endpoint_for`.
pub fn endpoint() -> Result<Endpoint, ConnError> {
    default_endpoint_for(ChannelKind::Agent).map_err(ConnError::from)
}

/// Connects to the agent endpoint (no handshake).
///
/// # Errors
/// [`ConnError::DesktopNotRunning`] if nothing serves it.
pub async fn connect() -> Result<ClientStream, ConnError> {
    let endpoint = endpoint()?;
    transport_connect(&endpoint).await.map_err(ConnError::from)
}

/// What the relay needs from a live link to the desktop.
pub trait Link {
    /// Sends one request and waits for its answer.
    fn request(
        &self,
        op: OpRequest,
        wait: Duration,
    ) -> LocalFut<'_, Result<OpResponse, RequestError>>;

    /// Whether the link is gone (closed by either side).
    fn is_closed(&self) -> bool;

    /// Whether the desktop told us the pairing was removed.
    fn unpaired(&self) -> bool;
}

/// Where links come from.
pub trait Connector {
    /// Opens a link (a fresh `connect` and handshake).
    fn connect(&self) -> LocalFut<'_, Result<Rc<dyn Link>, ConnError>>;

    /// Forgets the stored pairing (after `unpaired`).
    fn forget(&self);
}

type Pending = RefCell<HashMap<u32, oneshot::Sender<ResponseEnvelope>>>;

struct Inner {
    writer: Mutex<WriteHalf<ClientStream>>,
    session: RefCell<ChannelSession>,
    pending: Pending,
    next_id: Cell<u32>,
    closed: Cell<bool>,
    unpaired: Cell<bool>,
}

/// An authenticated connection to the desktop.
pub struct Connection {
    inner: Rc<Inner>,
    reader: tokio::task::AbortHandle,
}

impl Connection {
    /// Connects and runs the handshake with the pairing stored under `name`. Must run inside a
    /// `LocalSet` (the reader task is spawned on it).
    ///
    /// # Errors
    /// [`ConnError::NotPaired`] without a stored pairing, otherwise as [`ConnError`].
    pub async fn open(name: &str, store: &dyn AgentPairingStore) -> Result<Self, ConnError> {
        let (id, key) = store
            .load(name)
            .map_err(|_| ConnError::Io)?
            .ok_or(ConnError::NotPaired)?;
        let mut stream = connect().await?;
        let (session, first) = agent_client_handshake(&mut stream, &key, &id).await?;
        // The first frame is the `state` event; the state is not needed here.
        if !matches!(Event::parse(first.expose_secret()), Ok(Event::State { .. })) {
            return Err(ConnError::Io);
        }
        drop(key);
        let (reader, writer) = tokio::io::split(stream);
        let inner = Rc::new(Inner {
            writer: Mutex::new(writer),
            session: RefCell::new(session),
            pending: RefCell::new(HashMap::new()),
            next_id: Cell::new(1),
            closed: Cell::new(false),
            unpaired: Cell::new(false),
        });
        let task = tokio::task::spawn_local(read_loop(reader, Rc::clone(&inner)));
        Ok(Self {
            inner,
            reader: task.abort_handle(),
        })
    }

    async fn send(&self, op: OpRequest) -> Result<oneshot::Receiver<ResponseEnvelope>, ConnError> {
        let inner = &self.inner;
        if inner.closed.get() {
            return Err(ConnError::DesktopNotRunning);
        }
        let id = inner.next_id.get();
        inner.next_id.set(id.wrapping_add(1).max(1));
        let json = Zeroizing::new(
            RequestEnvelope { id, op }
                .to_json()
                .map_err(|_| ConnError::Io)?,
        );
        let (tx, rx) = oneshot::channel();
        inner.pending.borrow_mut().insert(id, tx);
        let mut writer = inner.writer.lock().await;
        // Seal and write under the writer lock, so sequence numbers follow the write order.
        let frame = inner
            .session
            .borrow_mut()
            .seal(&json)
            .map_err(|_| ConnError::Io);
        let written = match frame {
            Ok(frame) => timeout(
                WRITE_STEP,
                write_frame(&mut *writer, ByteOrder::Big, &frame),
            )
            .await
            .map_or(Err(ConnError::Io), |r| r.map_err(|_| ConnError::Io)),
            Err(e) => Err(e),
        };
        if let Err(e) = written {
            inner.pending.borrow_mut().remove(&id);
            inner.closed.set(true);
            return Err(e);
        }
        Ok(rx)
    }
}

impl Link for Connection {
    fn request(
        &self,
        op: OpRequest,
        wait: Duration,
    ) -> LocalFut<'_, Result<OpResponse, RequestError>> {
        Box::pin(async move {
            let rx = self.send(op).await.map_err(RequestError::Conn)?;
            let answer = match timeout(wait, rx).await {
                Err(_) => return Err(RequestError::Timeout),
                Ok(Err(_)) => return Err(RequestError::Conn(ConnError::DesktopNotRunning)),
                Ok(Ok(answer)) => answer,
            };
            match (answer.ok, answer.body, answer.error) {
                (true, Some(body), _) => Ok(body),
                (_, _, Some(error)) => Err(RequestError::Wire(error)),
                _ => Err(RequestError::Wire(WireError::code(
                    AgentErrorCode::Internal,
                ))),
            }
        })
    }

    fn is_closed(&self) -> bool {
        self.inner.closed.get()
    }

    fn unpaired(&self) -> bool {
        self.inner.unpaired.get()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // The reader task holds the other `Rc`; mark closed so it ends at the next frame, and
        // answer nobody.
        self.inner.closed.set(true);
        self.inner.pending.borrow_mut().clear();
        self.reader.abort();
    }
}

async fn read_loop(mut reader: tokio::io::ReadHalf<ClientStream>, inner: Rc<Inner>) {
    loop {
        let frame = match read_frame(&mut reader, ByteOrder::Big).await {
            Ok(Some(frame)) => frame,
            _ => break,
        };
        let plain = match inner.session.borrow_mut().open(&frame) {
            Ok(plain) => plain,
            Err(_) => break,
        };
        drop(frame);
        match DesktopFrame::parse(plain.expose_secret()) {
            Ok(DesktopFrame::Response(response)) => {
                let waiting = inner.pending.borrow_mut().remove(&response.id);
                if let Some(tx) = waiting {
                    let _ = tx.send(response);
                }
            }
            Ok(DesktopFrame::Event(Event::Unpaired)) => {
                inner.unpaired.set(true);
                break;
            }
            Ok(DesktopFrame::Event(Event::State { .. } | Event::GrantRevoked { .. })) => {}
            Err(_) => break,
        }
        if inner.closed.get() {
            break;
        }
    }
    inner.closed.set(true);
    // Dropping the senders wakes every waiter with "closed".
    inner.pending.borrow_mut().clear();
    let _ = inner.writer.lock().await.shutdown().await;
}

/// The real connector: the keystore pairing named `client`.
pub struct RealConnector {
    /// The client name of the pairing.
    pub client: String,
    /// Where the pairing lives.
    pub store: Box<dyn AgentPairingStore>,
}

impl Connector for RealConnector {
    fn connect(&self) -> LocalFut<'_, Result<Rc<dyn Link>, ConnError>> {
        Box::pin(async move {
            let connection = Connection::open(&self.client, self.store.as_ref()).await?;
            Ok(Rc::new(connection) as Rc<dyn Link>)
        })
    }

    fn forget(&self) {
        let _ = self.store.delete(&self.client);
    }
}
