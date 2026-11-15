//! Tests of the server loop against a fake host on a real local endpoint.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mosaic_core::browser_channel::{ChannelSession, PairingKey, new_pairing_id};
use mosaic_native_messaging::agent_handshake::{
    agent_client_handshake, agent_preauth_request, agent_read_server_message,
};
use mosaic_native_messaging::agent_preauth::{
    AgentClientMessage, AgentRefusal, AgentServerMessage,
};
use mosaic_native_messaging::endpoint::Endpoint;
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame, write_frame};
use mosaic_native_messaging::handshake::{send, write_message};
use mosaic_native_messaging::transport::{ClientStream, connect};
use tokio::sync::Semaphore;

use crate::error::AgentErrorCode;
use crate::server::{AgentHost, AgentServer, status_response};
use crate::testing::local_endpoint;
use crate::wire::{
    AccessState, DesktopFrame, Event, OpRequest, OpResponse, ResponseEnvelope, VaultState,
    WireError,
};

fn block_on<F: Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn key(byte: u8) -> PairingKey {
    PairingKey::from_bytes(&mut [byte; 32])
}

type PairOutcome = Result<([u8; 16], PairingKey), AgentRefusal>;

struct FakeHost {
    state: Mutex<(VaultState, AccessState)>,
    keys: Mutex<HashMap<[u8; 16], Arc<PairingKey>>>,
    pair_result: Mutex<Option<PairOutcome>>,
    /// `pair` waits for a permit when `pair_blocks` is set.
    pair_blocks: AtomicBool,
    pair_gate: Semaphore,
    pair_dropped: Arc<AtomicBool>,
    handled: Mutex<Vec<OpRequest>>,
    /// `httpRequest` waits for a permit.
    gate: Semaphore,
    used: Mutex<Vec<[u8; 16]>>,
    undelivered: Mutex<Vec<[u8; 16]>>,
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl FakeHost {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new((VaultState::Unlocked, AccessState::On)),
            keys: Mutex::new(HashMap::new()),
            pair_result: Mutex::new(None),
            pair_blocks: AtomicBool::new(false),
            pair_gate: Semaphore::new(0),
            pair_dropped: Arc::new(AtomicBool::new(false)),
            handled: Mutex::new(Vec::new()),
            gate: Semaphore::new(0),
            used: Mutex::new(Vec::new()),
            undelivered: Mutex::new(Vec::new()),
        })
    }

    fn add_pairing(&self, id: [u8; 16], key_byte: u8) {
        self.keys
            .lock()
            .unwrap()
            .insert(id, Arc::new(key(key_byte)));
    }

    fn set_state(&self, vault: VaultState, access: AccessState) {
        *self.state.lock().unwrap() = (vault, access);
    }
}

impl AgentHost for FakeHost {
    fn state(&self) -> (VaultState, AccessState) {
        *self.state.lock().unwrap()
    }

    fn pairing_key(&self, id: &[u8; 16]) -> Option<Arc<PairingKey>> {
        self.keys.lock().unwrap().get(id).cloned()
    }

    fn pair(
        &self,
        _name: String,
        _code: String,
    ) -> impl Future<Output = Result<([u8; 16], PairingKey), AgentRefusal>> + Send {
        let flag = DropFlag(Arc::clone(&self.pair_dropped));
        async move {
            let _flag = flag;
            if self.pair_blocks.load(Ordering::SeqCst) {
                let _ = self.pair_gate.acquire().await;
            }
            self.pair_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Err(AgentRefusal::PairingDenied))
        }
    }

    async fn handle(
        &self,
        _pairing: [u8; 16],
        _conn: u64,
        op: OpRequest,
    ) -> Result<OpResponse, WireError> {
        self.handled.lock().unwrap().push(op.clone());
        if matches!(op, OpRequest::HttpRequest { .. }) {
            let permit = self.gate.acquire().await;
            if let Ok(permit) = permit {
                permit.forget();
            }
        }
        let (vault, access) = self.state();
        Ok(status_response(vault, access))
    }

    fn note_used(&self, pairing: &[u8; 16]) {
        self.used.lock().unwrap().push(*pairing);
    }

    fn pairing_undelivered(&self, pairing: &[u8; 16]) {
        self.undelivered.lock().unwrap().push(*pairing);
    }
}

struct Client {
    stream: ClientStream,
    session: ChannelSession,
}

impl Client {
    async fn open(
        endpoint: &Endpoint,
        id: &[u8; 16],
        key: &PairingKey,
    ) -> Result<(Self, Event), NmError> {
        let mut stream = connect(endpoint).await?;
        let (session, first) = agent_client_handshake(&mut stream, key, id).await?;
        let event = Event::parse(first.expose_secret()).unwrap();
        Ok((Self { stream, session }, event))
    }

    async fn send_raw(&mut self, json: &[u8]) {
        send(&mut self.stream, &mut self.session, json)
            .await
            .unwrap();
    }

    async fn request(&mut self, id: u32, op: &str, body: &str) {
        let json = format!(r#"{{"v":1,"id":{id},"op":"{op}","body":{body}}}"#);
        self.send_raw(json.as_bytes()).await;
    }

    /// The next plaintext frame, or `None` when the connection closed.
    async fn recv(&mut self) -> Option<DesktopFrame> {
        let read = tokio::time::timeout(
            Duration::from_secs(10),
            read_frame(&mut self.stream, ByteOrder::Big),
        )
        .await
        .expect("a frame or a close within 10 s");
        match read {
            Ok(Some(frame)) => {
                let plain = self.session.open(&frame).unwrap();
                Some(DesktopFrame::parse(plain.expose_secret()).unwrap())
            }
            _ => None,
        }
    }

    async fn response(&mut self) -> ResponseEnvelope {
        match self.recv().await {
            Some(DesktopFrame::Response(r)) => r,
            other => panic!("expected a response, got {other:?}"),
        }
    }
}

fn setup(host: &Arc<FakeHost>) -> (tempfile::TempDir, Endpoint, AgentServer) {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = local_endpoint(dir.path()).unwrap();
    let server = AgentServer::start(&endpoint, Arc::clone(host)).unwrap();
    (dir, endpoint, server)
}

async fn wait_for(mut done: impl FnMut() -> bool) {
    for _ in 0..500 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("condition not reached within 5 s");
}

#[test]
fn status_and_state_events() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let mut stream = connect(&endpoint).await.unwrap();
        let answer = agent_preauth_request(&mut stream, &AgentClientMessage::Status { v: 1 })
            .await
            .unwrap();
        assert!(matches!(
            answer,
            AgentServerMessage::Status {
                vault: VaultState::Unlocked,
                agent_access: AccessState::On,
                ..
            }
        ));

        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let (mut client, first) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        assert_eq!(
            first,
            Event::State {
                vault: VaultState::Unlocked,
                agent_access: AccessState::On
            }
        );
        wait_for(|| server.connections() == 1).await;
        let event = Event::State {
            vault: VaultState::Locked,
            agent_access: AccessState::On,
        };
        server.broadcast(&event);
        assert_eq!(client.recv().await, Some(DesktopFrame::Event(event)));
        server.stop();
    });
}

#[test]
fn relay_forwards_only_the_allowlisted_ops() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let (mut client, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        client.request(1, "get", "{}").await;
        client.request(2, "reveal", "{}").await;
        client.request(3, "runStart", "{}").await;
        client.request(4, "status", r#"{"extra":1}"#).await;
        client
            .send_raw(br#"{"v":1,"id":5,"op":"status","body":{},"x":1}"#)
            .await;
        for expected in 1..=5 {
            let r = client.response().await;
            assert_eq!(r.id, expected);
            assert!(!r.ok);
            assert_eq!(
                r.error.map(|e| e.code),
                Some(AgentErrorCode::InvalidRequest)
            );
        }
        assert!(
            host.handled.lock().unwrap().is_empty(),
            "nothing reached the host"
        );

        client.request(6, "status", "{}").await;
        let r = client.response().await;
        assert!(r.ok && r.id == 6);
        assert_eq!(host.handled.lock().unwrap().len(), 1);
        assert_eq!(host.used.lock().unwrap().as_slice(), &[id]);
        server.stop();
    });
}

#[test]
fn a_frame_that_is_not_an_envelope_closes_the_connection() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let (mut client, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        client.send_raw(br#"{"hello":"world"}"#).await;
        assert_eq!(client.recv().await, None);
        server.stop();
    });
}

#[test]
fn in_flight_cap_answers_busy() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let (mut client, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        let body = r#"{"ref":"mosaic://v/i/password","method":"GET","url":"https://x"}"#;
        for n in 1..=5 {
            client.request(n, "httpRequest", body).await;
        }
        // The fifth is answered at once; the four others wait for the gate.
        let busy = client.response().await;
        assert_eq!(busy.id, 5);
        assert_eq!(busy.error.map(|e| e.code), Some(AgentErrorCode::Busy));
        host.gate.add_permits(4);
        let mut ids = Vec::new();
        for _ in 0..4 {
            let r = client.response().await;
            assert!(r.ok);
            ids.push(r.id);
        }
        ids.sort_unstable();
        assert_eq!(ids, [1, 2, 3, 4]);
        server.stop();
    });
}

#[test]
fn connection_cap_holds() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let mut clients = Vec::new();
        for _ in 0..8 {
            clients.push(Client::open(&endpoint, &id, &key(7)).await.unwrap().0);
        }
        wait_for(|| server.connections() == 8).await;
        let mut stream = connect(&endpoint).await.unwrap();
        let err = agent_client_handshake(&mut stream, &key(7), &id)
            .await
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err, NmError::AgentRefused(AgentRefusal::Busy));
        assert_eq!(server.connections(), 8);
        drop(clients);
        server.stop();
    });
}

#[test]
fn unpaired_event_reaches_the_client_on_close_pairing() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let (a, b) = (new_pairing_id().unwrap(), new_pairing_id().unwrap());
        host.add_pairing(a, 7);
        host.add_pairing(b, 8);
        let (mut ca, _) = Client::open(&endpoint, &a, &key(7)).await.unwrap();
        let (mut cb, _) = Client::open(&endpoint, &b, &key(8)).await.unwrap();
        wait_for(|| server.connections() == 2).await;
        server.close_pairing(&a);
        assert_eq!(ca.recv().await, Some(DesktopFrame::Event(Event::Unpaired)));
        assert_eq!(ca.recv().await, None, "the connection closes");
        // The other pairing is untouched.
        cb.request(1, "status", "{}").await;
        assert!(cb.response().await.ok);
        server.stop();
    });
}

#[test]
fn a_pairing_removed_during_the_handshake_is_unpaired() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        // The key is known at `hello`, then removed before the session registers: the next
        // check finds it gone. Simulated by removing it right after the handshake returns.
        host.add_pairing(id, 7);
        let (mut client, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        host.keys.lock().unwrap().remove(&id);
        server.close_pairing(&id);
        assert_eq!(
            client.recv().await,
            Some(DesktopFrame::Event(Event::Unpaired))
        );
        assert_eq!(client.recv().await, None);
        server.stop();
    });
}

#[test]
fn stop_closes_every_connection() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let (mut c1, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        let (mut c2, _) = Client::open(&endpoint, &id, &key(7)).await.unwrap();
        // A connection that has not said anything yet.
        let mut silent = connect(&endpoint).await.unwrap();
        wait_for(|| server.connections() == 2).await;
        // Let the accept loop take the silent connection into a pre-auth slot.
        tokio::time::sleep(Duration::from_millis(200)).await;
        server.stop();
        assert_eq!(c1.recv().await, None);
        assert_eq!(c2.recv().await, None);
        let read = tokio::time::timeout(
            Duration::from_secs(10),
            read_frame(&mut silent, ByteOrder::Big),
        )
        .await
        .expect("closed within 10 s");
        assert!(!matches!(read, Ok(Some(_))));
    });
}

#[test]
fn pair_request_is_refused_while_off_or_locked() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let request = AgentClientMessage::PairRequest {
            v: 1,
            name: "Claude Code".to_owned(),
        };
        host.set_state(VaultState::Locked, AccessState::On);
        let mut stream = connect(&endpoint).await.unwrap();
        let err = agent_preauth_request(&mut stream, &request)
            .await
            .unwrap_err();
        assert_eq!(err, NmError::AgentRefused(AgentRefusal::Locked));
        host.set_state(VaultState::Unlocked, AccessState::Off);
        let mut stream = connect(&endpoint).await.unwrap();
        let err = agent_preauth_request(&mut stream, &request)
            .await
            .unwrap_err();
        assert_eq!(err, NmError::AgentRefused(AgentRefusal::AgentAccessOff));
        // A hello is refused while off, too.
        let id = new_pairing_id().unwrap();
        host.add_pairing(id, 7);
        let err = Client::open(&endpoint, &id, &key(7))
            .await
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err, NmError::AgentRefused(AgentRefusal::AgentAccessOff));
        server.stop();
    });
}

#[test]
fn pair_request_is_approved_or_denied_by_the_host() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let request = AgentClientMessage::PairRequest {
            v: 1,
            name: "Cursor (work)".to_owned(),
        };
        let new_id = new_pairing_id().unwrap();
        *host.pair_result.lock().unwrap() = Some(Ok((new_id, key(9))));
        let mut stream = connect(&endpoint).await.unwrap();
        let pending = agent_preauth_request(&mut stream, &request).await.unwrap();
        let AgentServerMessage::PairPending {
            code,
            expires_in_seconds,
            ..
        } = pending
        else {
            panic!("no pairPending");
        };
        assert_eq!(code.len(), 6);
        assert_eq!(expires_in_seconds, 120);
        let approved = agent_read_server_message(&mut stream).await.unwrap();
        let AgentServerMessage::PairApproved {
            pairing_id, key, ..
        } = approved
        else {
            panic!("not approved");
        };
        assert_eq!(pairing_id.0, new_id);
        assert_eq!(key.0.expose_secret(), &[9u8; 32]);

        let mut stream = connect(&endpoint).await.unwrap();
        agent_preauth_request(&mut stream, &request).await.unwrap();
        let denied = agent_read_server_message(&mut stream).await.unwrap();
        assert!(matches!(
            denied,
            AgentServerMessage::Refused {
                reason: AgentRefusal::PairingDenied,
                ..
            }
        ));
        server.stop();
    });
}

#[test]
fn a_client_that_hangs_up_ends_the_pairing_dialog() {
    block_on(async {
        let host = FakeHost::new();
        host.pair_blocks.store(true, Ordering::SeqCst);
        let (_dir, endpoint, server) = setup(&host);
        let mut stream = connect(&endpoint).await.unwrap();
        let request = AgentClientMessage::PairRequest {
            v: 1,
            name: "x".to_owned(),
        };
        write_message(&mut stream, &request).await.unwrap();
        let pending = agent_read_server_message(&mut stream).await.unwrap();
        assert!(matches!(pending, AgentServerMessage::PairPending { .. }));
        assert!(!host.pair_dropped.load(Ordering::SeqCst));
        drop(stream);
        wait_for(|| host.pair_dropped.load(Ordering::SeqCst)).await;
        assert!(host.undelivered.lock().unwrap().is_empty());
        server.stop();
    });
}

#[test]
fn an_unknown_pairing_fails_like_a_wrong_proof() {
    block_on(async {
        let host = FakeHost::new();
        let (_dir, endpoint, server) = setup(&host);
        let id = new_pairing_id().unwrap();
        let err = Client::open(&endpoint, &id, &key(7))
            .await
            .map(|_| ())
            .unwrap_err();
        // The relay holds a key the desktop does not know: its challenge cannot be verified.
        assert_eq!(err, NmError::NotAuthentic);
        assert_eq!(server.connections(), 0);
        // Raw garbage is refused and closed.
        let mut stream = connect(&endpoint).await.unwrap();
        write_frame(&mut stream, ByteOrder::Big, b"not json")
            .await
            .unwrap();
        let answer = agent_read_server_message(&mut stream).await.unwrap();
        assert!(matches!(
            answer,
            AgentServerMessage::Refused {
                reason: AgentRefusal::InvalidRequest,
                ..
            }
        ));
        server.stop();
    });
}
