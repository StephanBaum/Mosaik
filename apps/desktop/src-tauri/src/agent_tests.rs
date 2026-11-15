//! Tests of the agent endpoint (ADR-0159): pairing, the dialogs, the request path, the log and
//! revocation. They run against a backend on an in-memory store with a really unlocked account,
//! over the real local endpoint (a named pipe or a Unix socket) with the client side of the
//! handshake from `mosaic-native-messaging`; the clock of the prompts is moved by hand, and the
//! HTTPS destination is the fixture TLS server of `mosaic-agent` (debug builds only).
// Test helpers take owned values.
#![expect(
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    reason = "test helpers take their arguments by value; one test per behavior"
)]

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mosaic_agent::testing::local_endpoint;
use mosaic_agent::wire::{DesktopFrame, Event, ResponseEnvelope};
use mosaic_core::Argon2Profile;
use mosaic_core::browser_channel::{ChannelSession, PairingKey};
use mosaic_native_messaging::agent_handshake::{
    agent_client_handshake, agent_preauth_request, agent_read_server_message,
};
use mosaic_native_messaging::agent_preauth::{
    AccessState, AgentClientMessage, AgentRefusal, AgentServerMessage, VaultState,
};
use mosaic_native_messaging::endpoint::Endpoint;
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame};
use mosaic_native_messaging::handshake::send;
use mosaic_native_messaging::transport::{ClientStream, connect};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};

use crate::agent::{AgentRegistry, AgentService, Clock, Limits};
use crate::contract::agent_access::{
    AgentAccessDecision, AgentAccessRespondRequest, AgentEndpointState, AgentPairingRemoveRequest,
    AgentPairingRespondRequest, AgentSetEnabledRequest,
};
use crate::contract::browser::PairingDecision;
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::secret::SecretIn;
use crate::contract::session::CreateAccountRequest;
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

const PASSWORD: &str = "correct horse battery staple";

fn run<F: Future>(future: F) -> F::Output {
    tauri::async_runtime::block_on(future)
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// A clock the tests move by hand.
#[derive(Clone)]
struct TestClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Arc::default(),
        }
    }

    fn handle(&self) -> Clock {
        let clock = self.clone();
        Arc::new(move || clock.base + *clock.offset.lock().unwrap())
    }

    fn advance(&self, by: Duration) {
        *self.offset.lock().unwrap() += by;
    }
}

struct Env {
    state: AppState,
    events: Log,
    clock: TestClock,
    focus: Arc<AtomicUsize>,
    dir: tempfile::TempDir,
    endpoint: Endpoint,
    /// Argon2id runs once per account; one test at a time.
    _kdf: std::sync::MutexGuard<'static, ()>,
}

impl Env {
    /// An environment with a really unlocked account and agent access still off.
    fn new() -> Self {
        Self::with_limits(Limits::default())
    }

    fn with_limits(limits: Limits) -> Self {
        let kdf = crate::test_support::kdf_permit();
        let clock = TestClock::new();
        let focus = Arc::new(AtomicUsize::new(0));
        let mut backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_focus({
            let focus = Arc::clone(&focus);
            Arc::new(move || {
                focus.fetch_add(1, Ordering::SeqCst);
            })
        });
        backend.agent = AgentService::with_clock(clock.handle(), limits);
        let (channel, events) = recording_channel();
        backend.subscribe(channel).unwrap();
        let state = AppState::new(backend);
        let dir = tempfile::tempdir().unwrap();
        let endpoint = local_endpoint(dir.path()).unwrap();
        state.backend().agent.attach(state.downgrade());
        state.backend().agent.set_endpoint(endpoint.clone());
        *crate::agent::lock(&state.backend().agent.shared().registry) =
            AgentRegistry::in_dir(dir.path());
        state
            .backend()
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        Self {
            state,
            events,
            clock,
            focus,
            dir,
            endpoint,
            _kdf: kdf,
        }
    }

    /// An environment with agent access on.
    fn enabled() -> Self {
        let env = Self::new();
        env.enable();
        env
    }

    fn backend(&self) -> &Backend {
        self.state.backend()
    }

    fn enable(&self) {
        self.backend()
            .agent_access_set_enabled(AgentSetEnabledRequest { enabled: true })
            .unwrap();
    }

    fn disable(&self) {
        self.backend()
            .agent_access_set_enabled(AgentSetEnabledRequest { enabled: false })
            .unwrap();
    }

    /// The webview events of one type, oldest first.
    fn events_of(&self, kind: &str) -> Vec<Value> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|e| serde_json::from_str::<Value>(e).unwrap())
            .filter(|e| e["type"] == kind)
            .collect()
    }

    /// The next event of `kind` after `seen` earlier ones.
    async fn event(&self, kind: &str, seen: usize) -> Value {
        for _ in 0..500 {
            let all = self.events_of(kind);
            if let Some(event) = all.get(seen) {
                return event.clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("no {kind} event within 5 s");
    }

    /// A pairing added straight to the registry: its ID and the client's copy of the key.
    fn add_pairing(&self, name: &str) -> ([u8; 16], PairingKey) {
        crate::agent::lock(&self.backend().agent.shared().registry)
            .add(name.into(), 1_000)
            .unwrap()
    }

    fn pairing_respond(&self, request_id: &str, decision: PairingDecision) -> Result<(), IpcError> {
        self.backend()
            .agent_pairing_respond(&AgentPairingRespondRequest {
                request_id: request_id.into(),
                decision,
            })
    }

    fn access_respond(
        &self,
        request_id: &str,
        decision: AgentAccessDecision,
    ) -> Result<(), IpcError> {
        self.backend()
            .agent_access_respond(&AgentAccessRespondRequest {
                request_id: request_id.into(),
                decision,
            })
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

    async fn request(&mut self, id: u32, op: &str, body: Value) {
        let json = json!({ "v": 1, "id": id, "op": op, "body": body }).to_string();
        send(&mut self.stream, &mut self.session, json.as_bytes())
            .await
            .unwrap();
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

    /// The next response (events in between are skipped).
    async fn response(&mut self) -> ResponseEnvelope {
        loop {
            match self.recv().await {
                Some(DesktopFrame::Response(r)) => return r,
                Some(DesktopFrame::Event(_)) => {}
                None => panic!("the connection closed"),
            }
        }
    }
}

/// Sends a `pairRequest`; the answer is `pairPending`, or the endpoint's refusal before any
/// dialog (locked, off).
async fn try_pair_request(
    endpoint: &Endpoint,
    name: &str,
) -> Result<(ClientStream, AgentServerMessage), NmError> {
    let mut stream = connect(endpoint).await?;
    let first = agent_preauth_request(
        &mut stream,
        &AgentClientMessage::PairRequest {
            v: 1,
            name: name.into(),
        },
    )
    .await?;
    Ok((stream, first))
}

async fn pair_request(endpoint: &Endpoint, name: &str) -> (ClientStream, AgentServerMessage) {
    try_pair_request(endpoint, name).await.unwrap()
}

/// The reason a pairing ended in a refusal, from the next message or the error.
fn refusal(result: &Result<AgentServerMessage, NmError>) -> Option<AgentRefusal> {
    match result {
        Err(NmError::AgentRefused(reason)) | Ok(AgentServerMessage::Refused { reason, .. }) => {
            Some(*reason)
        }
        Ok(_) | Err(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Pairing
// ---------------------------------------------------------------------------

#[test]
fn pairing_needs_the_user_and_the_one_second_guard() {
    let env = Env::enabled();
    run(async {
        let (mut stream, first) = pair_request(&env.endpoint, "Claude Code").await;
        let AgentServerMessage::PairPending { code, .. } = first else {
            panic!("pairPending");
        };
        let event = env.event("agentPairingRequested", 0).await;
        assert_eq!(event["request"]["name"], "Claude Code");
        assert_eq!(
            event["request"]["code"],
            code.as_str(),
            "the dialog shows the code"
        );
        assert!(
            env.focus.load(Ordering::SeqCst) >= 1,
            "the window comes forward"
        );
        let id = event["request"]["requestId"].as_str().unwrap().to_owned();
        assert!(
            crate::agent::lock(&env.backend().agent.shared().registry)
                .list()
                .is_empty(),
            "nothing is paired before the user allows it"
        );

        env.clock.advance(Duration::from_millis(999));
        assert_eq!(
            code_of(env.pairing_respond(&id, PairingDecision::Allow)),
            ErrorCode::AgentAllowTooSoon
        );
        assert!(
            crate::agent::lock(&env.backend().agent.shared().registry)
                .list()
                .is_empty()
        );
        env.clock.advance(Duration::from_millis(1));
        env.pairing_respond(&id, PairingDecision::Allow).unwrap();

        let approved = agent_read_server_message(&mut stream).await.unwrap();
        let AgentServerMessage::PairApproved {
            pairing_id, key, ..
        } = approved
        else {
            panic!("pairApproved");
        };
        let registry = crate::agent::lock(&env.backend().agent.shared().registry);
        let stored = registry.get(&pairing_id.0).expect("the pairing is saved");
        assert_eq!(stored.name, "Claude Code");
        assert_eq!(stored.key.expose_secret(), key.0.expose_secret());
        drop(registry);
        // The saved file holds it too.
        let again = AgentRegistry::in_dir(env.dir.path());
        assert!(again.get(&pairing_id.0).is_some());
        // And the new pairing authenticates.
        let (_client, state) = Client::open(&env.endpoint, &pairing_id.0, &key.0)
            .await
            .unwrap();
        assert_eq!(
            state,
            Event::State {
                vault: VaultState::Unlocked,
                agent_access: AccessState::On
            }
        );
        // The dialog closed as answered.
        let closed = env.event("agentPairingClosed", 0).await;
        assert_eq!(closed["reason"], "answered");
    });
}

fn code_of(result: Result<(), IpcError>) -> ErrorCode {
    code(result.unwrap_err())
}

#[test]
fn pairing_refused_while_locked() {
    let env = Env::enabled();
    run(async {
        env.backend().lock(LockReason::Manual).unwrap();
        let refused = try_pair_request(&env.endpoint, "Cursor")
            .await
            .map(|(_, message)| message);
        assert_eq!(refusal(&refused), Some(AgentRefusal::Locked));
        assert!(
            env.events_of("agentPairingRequested").is_empty(),
            "no dialog"
        );
    });
}

#[test]
fn nothing_listens_while_agent_access_is_off() {
    let off = Env::new();
    run(async {
        assert!(connect(&off.endpoint).await.is_err());
    });
}

#[test]
fn three_denials_block_pairing_prompts() {
    let env = Env::enabled();
    run(async {
        for n in 0..3 {
            let (mut stream, first) = pair_request(&env.endpoint, "Claude Code").await;
            assert!(matches!(first, AgentServerMessage::PairPending { .. }));
            let event = env.event("agentPairingRequested", n).await;
            let id = event["request"]["requestId"].as_str().unwrap().to_owned();
            // A deny is always taken, at once.
            env.pairing_respond(&id, PairingDecision::Deny).unwrap();
            let reply = agent_read_server_message(&mut stream).await;
            assert_eq!(refusal(&reply), Some(AgentRefusal::PairingDenied));
        }
        let (mut stream, _) = pair_request(&env.endpoint, "Claude Code").await;
        let reply = agent_read_server_message(&mut stream).await;
        assert_eq!(refusal(&reply), Some(AgentRefusal::PairingBlocked));
        assert_eq!(
            env.events_of("agentPairingRequested").len(),
            3,
            "no fourth dialog"
        );
        // Ten minutes later the block is over.
        env.clock.advance(Duration::from_mins(11));
        let (_stream, again) = pair_request(&env.endpoint, "Claude Code").await;
        assert!(matches!(again, AgentServerMessage::PairPending { .. }));
    });
}

#[test]
fn a_second_pairing_request_while_one_is_open_is_refused() {
    let env = Env::enabled();
    run(async {
        let (_first, pending) = pair_request(&env.endpoint, "Claude Code").await;
        assert!(matches!(pending, AgentServerMessage::PairPending { .. }));
        let (mut second, _) = pair_request(&env.endpoint, "Cursor").await;
        let busy = agent_read_server_message(&mut second).await;
        assert_eq!(refusal(&busy), Some(AgentRefusal::Busy));
        assert_eq!(env.events_of("agentPairingRequested").len(), 1);
    });
}

#[test]
fn a_client_that_hangs_up_closes_its_pairing_dialog() {
    let env = Env::enabled();
    run(async {
        let (stream, _) = pair_request(&env.endpoint, "Claude Code").await;
        env.event("agentPairingRequested", 0).await;
        drop(stream);
        let closed = env.event("agentPairingClosed", 0).await;
        assert_eq!(closed["reason"], "disconnected");
        assert!(
            crate::agent::lock(&env.backend().agent.shared().registry)
                .list()
                .is_empty()
        );
    });
}

#[test]
fn an_unanswered_pairing_ends_at_its_timeout_and_a_lock_closes_it() {
    let env = Env::with_limits(Limits {
        pair_timeout: Duration::from_millis(300),
        ..Limits::default()
    });
    env.enable();
    run(async {
        let (mut stream, _) = pair_request(&env.endpoint, "Claude Code").await;
        let reply = agent_read_server_message(&mut stream).await;
        assert_eq!(refusal(&reply), Some(AgentRefusal::PairingExpired));
        assert_eq!(
            env.event("agentPairingClosed", 0).await["reason"],
            "expired"
        );
        // A lock closes the next one, and nothing is paired.
        let (mut stream, _) = pair_request(&env.endpoint, "Claude Code").await;
        env.event("agentPairingRequested", 1).await;
        env.backend().lock(LockReason::Idle).unwrap();
        let reply = agent_read_server_message(&mut stream).await;
        assert!(matches!(
            refusal(&reply),
            Some(AgentRefusal::Locked | AgentRefusal::PairingExpired)
        ));
    });
    assert!(
        crate::agent::lock(&env.backend().agent.shared().registry)
            .list()
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// Connections, removal, the switch
// ---------------------------------------------------------------------------

#[test]
fn removing_a_pairing_sends_unpaired_and_closes_the_connection() {
    let env = Env::enabled();
    let (id, key) = env.add_pairing("Claude Code");
    run(async {
        let (mut client, _) = Client::open(&env.endpoint, &id, &key).await.unwrap();
        let status = env
            .backend()
            .agent_pairing_remove(&AgentPairingRemoveRequest {
                pairing_id: crate::browser::hex(&id),
            })
            .unwrap();
        assert!(status.pairings.is_empty());
        assert_eq!(
            client.recv().await,
            Some(DesktopFrame::Event(Event::Unpaired))
        );
        assert_eq!(client.recv().await, None, "then the connection closes");
        // The key no longer authenticates.
        assert!(Client::open(&env.endpoint, &id, &key).await.is_err());
    });
    let e = env
        .backend()
        .agent_pairing_remove(&AgentPairingRemoveRequest {
            pairing_id: crate::browser::hex(&id),
        })
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentPairingNotFound);
    let e = env
        .backend()
        .agent_pairing_remove(&AgentPairingRemoveRequest {
            pairing_id: "nothex".into(),
        })
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::AgentPairingNotFound);
}

#[test]
fn turning_agent_access_off_closes_the_endpoint() {
    let env = Env::enabled();
    let (id, key) = env.add_pairing("Claude Code");
    run(async {
        let (mut client, _) = Client::open(&env.endpoint, &id, &key).await.unwrap();
        assert_eq!(
            env.backend().agent_access_status().endpoint,
            AgentEndpointState::Running
        );
        env.disable();
        assert_eq!(client.recv().await, None, "the connection closes");
        assert!(connect(&env.endpoint).await.is_err(), "nothing listens");
        let status = env.backend().agent_access_status();
        assert!(!status.enabled);
        assert_eq!(status.endpoint, AgentEndpointState::Stopped);
        assert_eq!(status.pairings.len(), 1, "the pairings stay");
    });
    // Turning it on needs an unlocked vault.
    env.backend().lock(LockReason::Manual).unwrap();
    let e = env
        .backend()
        .agent_access_set_enabled(AgentSetEnabledRequest { enabled: true })
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Locked);
}

#[test]
fn status_answers_while_locked_and_every_other_op_refuses() {
    let env = Env::enabled();
    let (id, key) = env.add_pairing("Claude Code");
    run(async {
        let (mut client, _) = Client::open(&env.endpoint, &id, &key).await.unwrap();
        env.backend().lock(LockReason::Manual).unwrap();
        // The state event reaches the client.
        assert_eq!(
            client.recv().await,
            Some(DesktopFrame::Event(Event::State {
                vault: VaultState::Locked,
                agent_access: AccessState::On
            }))
        );
        client.request(1, "status", json!({})).await;
        let r = client.response().await;
        assert!(r.ok, "status works while locked");
        client.request(2, "listSecrets", json!({})).await;
        let r = client.response().await;
        assert_eq!(r.error.map(|e| e.code.as_str()), Some("locked"));
        let mut raw = connect(&env.endpoint).await.unwrap();
        let answer = agent_preauth_request(&mut raw, &AgentClientMessage::Status { v: 1 })
            .await
            .unwrap();
        assert!(matches!(
            answer,
            AgentServerMessage::Status {
                vault: VaultState::Locked,
                agent_access: AccessState::On,
                ..
            }
        ));
    });
}

#[test]
fn endpoint_is_not_a_tcp_port() {
    // The endpoint of this OS is a pipe or a socket file, never a network address.
    let endpoint = crate::agent::platform_endpoint_for_test().unwrap();
    match endpoint {
        Endpoint::NamedPipe(name) => assert!(name.starts_with(r"\\.\pipe\mosaic-agent-")),
        Endpoint::UnixSocket(path) => assert!(path.ends_with("agent.sock")),
    }
    // No source file of the module opens a TCP listener.
    for source in [
        include_str!("agent/mod.rs"),
        include_str!("agent/host.rs"),
        include_str!("agent/prompts.rs"),
    ] {
        assert!(!source.contains("TcpListener"));
        assert!(!source.contains("127.0.0.1"));
    }
}
