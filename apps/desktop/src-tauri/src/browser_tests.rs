//! Tests of the browser endpoint (M4 T2, ADR-0036, ADR-0037): pairing, the
//! allowlist, fill and lookup, save and update, the rate limits and the
//! connection rules. They run over `tokio::io::duplex` streams against the
//! real handshake of `mosaic-native-messaging` (the client side of the host)
//! and a backend on an in-memory store; the clock of the prompts and limits
//! is moved by hand. The OS endpoint itself (Unix socket, Windows pipe and
//! discovery file) is covered by the library's own tests.
// Test helpers take owned values.
#![expect(
    clippy::needless_pass_by_value,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::too_many_lines,
    reason = "test helpers take their arguments by value; short names for the sample origins"
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::browser_channel::{ChannelSession, PairingKey, Role, new_channel_nonce};
use mosaic_core::model::{ItemType, LoginUrl, SecretField, TextField, UrlMatch};
use mosaic_core::{Argon2Profile, ItemId, VaultId};
use mosaic_native_messaging::Hex;
use mosaic_native_messaging::bodies::{BrowserKind, Hint};
use mosaic_native_messaging::error::Error as NmError;
use mosaic_native_messaging::framing::{ByteOrder, read_frame, write_frame};
use mosaic_native_messaging::handshake::{
    client_handshake, read_server_message, send, write_message,
};
use mosaic_native_messaging::preauth::{ClientMessage, ServerMessage};
use mosaic_native_messaging::protocol::{ErrorCode as Wire, PROTOCOL_VERSION};
use mosaic_totp::OtpAuth;
use mosaic_vault::otp::config_from_otp_auth;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, DuplexStream, ReadBuf};

use crate::browser::{BrowserService, Clock, Limits, Registry};
use crate::clipboard::MemoryClipboard;
use crate::contract::browser::{
    BrowserFamily, BrowserPairingRespondRequest, BrowserRegistryState, BrowserRemovePairingRequest,
    BrowserSaveDecision, BrowserSaveRespondRequest, BrowserStatus, PairingDecision,
};
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::secret::SecretIn;
use crate::contract::session::CreateAccountRequest;
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

const PASSWORD: &str = "correct horse battery staple";
/// RFC 4226 / 6238 test secret "12345678901234567890".
const OTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

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
    clipboard: MemoryClipboard,
    /// Argon2id runs once per account; one test at a time.
    kdf: Option<std::sync::MutexGuard<'static, ()>>,
}

impl Env {
    fn new() -> Self {
        Self::with_limits(Limits::default())
    }

    fn with_limits(limits: Limits) -> Self {
        let clock = TestClock::new();
        let clipboard = MemoryClipboard::default();
        let focus = Arc::new(AtomicUsize::new(0));
        let mut backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_clipboard(clipboard.factory())
        .with_focus({
            let focus = Arc::clone(&focus);
            Arc::new(move || {
                focus.fetch_add(1, Ordering::SeqCst);
            })
        });
        backend.browser = BrowserService::with_clock(clock.handle(), limits);
        let (channel, events) = recording_channel();
        backend.subscribe(channel).unwrap();
        let state = AppState::new(backend);
        state.backend().browser.attach(state.downgrade());
        Self {
            state,
            events,
            clock,
            focus,
            clipboard,
            kdf: None,
        }
    }

    /// An environment with a really unlocked account (Argon2id runs).
    fn unlocked() -> Self {
        Self::unlocked_with(Limits::default())
    }

    fn unlocked_with(limits: Limits) -> Self {
        let kdf = crate::test_support::kdf_permit();
        let mut env = Self::with_limits(limits);
        env.account();
        env.kdf = Some(kdf);
        env
    }

    fn backend(&self) -> &Backend {
        self.state.backend()
    }

    fn service(&self) -> &BrowserService {
        &self.backend().browser
    }

    /// Reads and writes the pairings in `dir`.
    fn use_dir(&self, dir: &std::path::Path) {
        *self.service().shared().registry.lock().unwrap() = Registry::in_dir(dir);
    }

    /// Creates the account: the vault is really unlocked (Argon2id runs).
    fn account(&self) -> VaultId {
        self.backend()
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        self.vault()
    }

    fn vault(&self) -> VaultId {
        self.backend().list_vaults().unwrap()[0].vault_id.get()
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

    fn add_pairing(&self, label: &str) -> ([u8; 16], PairingKey) {
        self.service()
            .shared()
            .registry
            .lock()
            .unwrap()
            .add(label.into(), BrowserFamily::Chromium, 1_000)
            .unwrap()
    }

    /// A login with an exact-origin URL.
    fn login(&self, title: &str, user: &str, password: &str, url: &str) -> ItemId {
        self.login_with(title, user, password, |_| {}, url)
    }

    fn login_with(
        &self,
        title: &str,
        user: &str,
        password: &str,
        tweak: impl FnOnce(&mut LoginUrl),
        url: &str,
    ) -> ItemId {
        let vault = self.vault();
        let mut slot = self.backend().slot().unwrap();
        let session = slot.unlocked_mut().unwrap();
        session
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_text(TextField::Username, user)?;
                e.set_secret(SecretField::Password, password)?;
                let mut entry = LoginUrl::new(url);
                tweak(&mut entry);
                e.add_url(&entry)?;
                Ok(())
            })
            .unwrap()
    }

    fn set_otp(&self, item: ItemId, uri: &str) {
        let vault = self.vault();
        let (config, counter) = config_from_otp_auth(&OtpAuth::parse(uri).unwrap()).unwrap();
        let mut slot = self.backend().slot().unwrap();
        slot.unlocked_mut()
            .unwrap()
            .update_item(vault, item, |e| {
                e.set_otp(&config)?;
                e.set_otp_counter(counter)
            })
            .unwrap();
    }

    fn password_of(&self, item: ItemId) -> String {
        let vault = self.vault();
        let slot = self.backend().slot().unwrap();
        slot.unlocked()
            .unwrap()
            .item(vault, item)
            .unwrap()
            .secret(SecretField::Password)
            .unwrap()
            .unwrap()
            .expose_secret()
            .to_owned()
    }

    /// How long since the last user action.
    fn idle(&self) -> Duration {
        self.backend()
            .activity
            .idle_at(Instant::now(), SystemTime::now())
    }
}

/// A host's end of an authenticated connection.
struct Peer {
    stream: DuplexStream,
    session: ChannelSession,
    next: u32,
    events: Vec<Value>,
}

/// The browser status once the first pairing shows as connected (at most 5 s).
async fn connected_status(env: &Env) -> BrowserStatus {
    for _ in 0..500 {
        let status = env.backend().browser_status();
        if status.pairings.first().is_some_and(|p| p.connected) {
            return status;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the session never showed as connected");
}

async fn connect(env: &Env, id: &[u8; 16], key: &PairingKey) -> Result<Peer, NmError> {
    let (mut stream, server) = tokio::io::duplex(1 << 20);
    env.service().serve(server);
    let (session, _state) = client_handshake(&mut stream, key, id).await?;
    Ok(Peer {
        stream,
        session,
        next: 0,
        events: Vec::new(),
    })
}

impl Peer {
    /// The next message from the desktop, `None` when the connection closed.
    async fn frame(&mut self) -> Option<Value> {
        let read = tokio::time::timeout(
            Duration::from_secs(10),
            read_frame(&mut self.stream, ByteOrder::Big),
        )
        .await
        .expect("the desktop answers");
        let Ok(Some(frame)) = read else {
            return None;
        };
        let plain = self.session.open(&frame).unwrap();
        Some(serde_json::from_slice(plain.expose_secret()).unwrap())
    }

    async fn request(&mut self, op: &str, body: Value) -> u32 {
        self.next += 1;
        let id = self.next;
        let json =
            serde_json::to_vec(&json!({ "v": 1, "id": id, "op": op, "body": body })).unwrap();
        send(&mut self.stream, &mut self.session, &json)
            .await
            .unwrap();
        id
    }

    /// The response to `id` (events on the way are kept).
    async fn response(&mut self, id: u32) -> Result<Value, String> {
        loop {
            let message = self.frame().await.expect("the connection stays open");
            if message.get("event").is_some() {
                self.events.push(message);
                continue;
            }
            assert_eq!(message["id"], id);
            return if message["ok"] == true {
                Ok(message["body"].clone())
            } else {
                Err(message["error"].as_str().unwrap().to_owned())
            };
        }
    }

    async fn call(&mut self, op: &str, body: Value) -> Result<Value, String> {
        let id = self.request(op, body).await;
        self.response(id).await
    }

    /// The next event.
    async fn event(&mut self) -> Option<Value> {
        if self.events.is_empty() {
            self.frame().await
        } else {
            Some(self.events.remove(0))
        }
    }
}

/// Starts a pairing request on a new connection and reads the first answer.
async fn pair_request(env: &Env) -> (DuplexStream, ServerMessage) {
    let (mut stream, server) = tokio::io::duplex(1 << 16);
    env.service().serve(server);
    let request = ClientMessage::PairRequest {
        v: PROTOCOL_VERSION,
        browser: BrowserKind::Chromium,
        hint: Hint::Edge,
    };
    write_message(&mut stream, &request).await.unwrap();
    let reply = read_server_message(&mut stream).await.unwrap();
    (stream, reply)
}

fn respond_pair(
    request_id: &str,
    allow: bool,
    label: Option<&str>,
) -> BrowserPairingRespondRequest {
    BrowserPairingRespondRequest {
        request_id: request_id.to_owned(),
        decision: if allow {
            PairingDecision::Allow
        } else {
            PairingDecision::Deny
        },
        label: label.map(str::to_owned),
    }
}

/// Pairs a browser: the whole dance, the user waiting a second to allow.
async fn pair(env: &Env, label: &str) -> ([u8; 16], PairingKey) {
    let (mut stream, reply) = pair_request(env).await;
    assert!(
        matches!(reply, ServerMessage::PairPending { .. }),
        "{reply:?}"
    );
    let request_id =
        env.events_of("browserPairingRequested").pop().unwrap()["request"]["requestId"]
            .as_str()
            .unwrap()
            .to_owned();
    env.clock.advance(Duration::from_millis(1100));
    env.backend()
        .browser_pairing_respond(&respond_pair(&request_id, true, Some(label)))
        .unwrap();
    match read_server_message(&mut stream).await.unwrap() {
        ServerMessage::PairApproved {
            pairing_id, key, ..
        } => (pairing_id.0, key.0),
        other => panic!("not approved: {other:?}"),
    }
}

fn refusal(message: &ServerMessage) -> Wire {
    match message {
        ServerMessage::Refused { reason, .. } => *reason,
        other => panic!("not refused: {other:?}"),
    }
}

/// An unlocked environment with one paired, connected browser.
async fn paired_peer(env: &Env) -> Peer {
    let (id, key) = pair(env, "Chrome").await;
    connect(env, &id, &key).await.unwrap()
}

// ---------------------------------------------------------------------------
// Pairing
// ---------------------------------------------------------------------------

#[test]
fn pairing_while_locked_is_refused_and_shows_nothing() {
    run(async {
        let env = Env::new();
        let (_stream, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::Locked);
        assert!(env.events_of("browserPairingRequested").is_empty());
    });
}

#[test]
fn allow_waits_a_second_then_persists_the_pairing_and_it_loads_again() {
    run(async {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::new();
        env.use_dir(dir.path());
        env.service().on_unlocked();
        let (mut stream, reply) = pair_request(&env).await;
        let ServerMessage::PairPending {
            code,
            expires_in_seconds,
            ..
        } = reply
        else {
            panic!("not pending: {reply:?}");
        };
        assert_eq!(expires_in_seconds, 120);
        let event = env.events_of("browserPairingRequested").pop().unwrap();
        assert_eq!(event["request"]["code"], code);
        assert_eq!(event["request"]["browser"], "chromium");
        assert_eq!(event["request"]["hint"], "edge");
        let request_id = event["request"]["requestId"].as_str().unwrap().to_owned();
        assert_eq!(request_id.len(), 32);

        let too_soon = |allow| env.backend().browser_pairing_respond(&allow);
        assert_eq!(
            code_of(too_soon(respond_pair(&request_id, true, Some("Work")))),
            ErrorCode::BrowserAllowTooSoon
        );
        env.clock.advance(Duration::from_secs(1));
        for bad in ["", "a\u{202e}b", &"x".repeat(65)] {
            assert_eq!(
                code_of(too_soon(respond_pair(&request_id, true, Some(bad)))),
                ErrorCode::InvalidInput,
                "{bad:?}"
            );
        }
        too_soon(respond_pair(&request_id, true, Some("Work"))).unwrap();
        let ServerMessage::PairApproved {
            pairing_id, key, ..
        } = read_server_message(&mut stream).await.unwrap()
        else {
            panic!("not approved");
        };
        assert_eq!(
            code_of(too_soon(respond_pair(&request_id, true, None))),
            ErrorCode::BrowserRequestExpired,
            "answered once"
        );
        assert_eq!(env.events_of("browserPairingClosed").len(), 1);
        let file = std::fs::read_to_string(dir.path().join("browser-pairings.json")).unwrap();
        assert!(file.contains("\"label\":\"Work\""), "{file}");

        // A new start of the app reads the file; the browser's key works.
        let again = Env::new();
        again.use_dir(dir.path());
        again.service().on_unlocked();
        let status = again.backend().browser_status();
        assert_eq!(status.pairings.len(), 1);
        let mut peer = connect(&again, &pairing_id.0, &key.0).await.unwrap();
        assert_eq!(
            peer.call("status", json!({})).await.unwrap()["state"],
            "unlocked"
        );
        let status = serde_json::to_value(again.backend().browser_status()).unwrap();
        assert_eq!(status["pairings"][0]["connected"], true);
        assert_eq!(status["pairings"][0]["label"], "Work");
    });
}

fn code_of<T>(result: Result<T, IpcError>) -> ErrorCode {
    match result {
        Ok(_) => panic!("expected an error"),
        Err(e) => code(e),
    }
}

#[test]
fn two_pairing_requests_at_once_end_in_a_conflict() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (mut first, reply) = pair_request(&env).await;
        assert!(matches!(reply, ServerMessage::PairPending { .. }));
        let (_second, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::PairingConflict);
        assert_eq!(
            refusal(&read_server_message(&mut first).await.unwrap()),
            Wire::PairingConflict
        );
        let closed = env.events_of("browserPairingClosed");
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0]["reason"], "conflict");
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .pair_id()
                .is_none()
        );
    });
}

#[test]
fn three_denials_block_pairing_for_ten_minutes() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        for _ in 0..3 {
            let (mut stream, reply) = pair_request(&env).await;
            assert!(matches!(reply, ServerMessage::PairPending { .. }));
            let id =
                env.events_of("browserPairingRequested").pop().unwrap()["request"]["requestId"]
                    .as_str()
                    .unwrap()
                    .to_owned();
            // A deny is taken at once.
            env.backend()
                .browser_pairing_respond(&respond_pair(&id, false, None))
                .unwrap();
            assert_eq!(
                refusal(&read_server_message(&mut stream).await.unwrap()),
                Wire::PairingDenied
            );
        }
        let (_stream, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::PairingBlocked);
        env.clock.advance(Duration::from_mins(9));
        let (_stream, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::PairingBlocked);
        env.clock.advance(Duration::from_mins(2));
        let (_stream, reply) = pair_request(&env).await;
        assert!(
            matches!(reply, ServerMessage::PairPending { .. }),
            "{reply:?}"
        );
    });
}

#[test]
fn the_seventeenth_pairing_is_refused() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        for n in 0..16 {
            env.add_pairing(&format!("Browser {n}"));
        }
        let (_stream, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::PairingLimit);
    });
}

#[test]
fn a_pairing_request_expires_and_a_hang_up_ends_it() {
    run(async {
        let env = Env::with_limits(Limits {
            pair_timeout: Duration::from_millis(300),
            ..Limits::default()
        });
        env.service().on_unlocked();
        let (mut stream, reply) = pair_request(&env).await;
        assert!(matches!(reply, ServerMessage::PairPending { .. }));
        assert_eq!(
            refusal(&read_server_message(&mut stream).await.unwrap()),
            Wire::PairingExpired
        );
        assert_eq!(
            env.events_of("browserPairingClosed")[0]["reason"],
            "expired"
        );

        // The browser gives up: the prompt closes as disconnected.
        let (stream, reply) = pair_request(&env).await;
        assert!(matches!(reply, ServerMessage::PairPending { .. }));
        drop(stream);
        for _ in 0..50 {
            if env.events_of("browserPairingClosed").len() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            env.events_of("browserPairingClosed")[1]["reason"],
            "disconnected"
        );
    });
}

#[test]
fn locking_closes_a_pending_pairing() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (mut stream, reply) = pair_request(&env).await;
        assert!(matches!(reply, ServerMessage::PairPending { .. }));
        env.service().on_locked();
        assert_eq!(
            refusal(&read_server_message(&mut stream).await.unwrap()),
            Wire::Locked
        );
        assert_eq!(env.events_of("browserPairingClosed")[0]["reason"], "locked");
    });
}

// ---------------------------------------------------------------------------
// Connections
// ---------------------------------------------------------------------------

#[test]
fn removing_a_pairing_tells_and_closes_its_sessions() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = pair(&env, "Chrome").await;
        let mut peer = connect(&env, &id, &key).await.unwrap();
        // The session registers right after the handshake's first frame, so wait for it.
        let status = connected_status(&env).await;
        let removed = env
            .backend()
            .browser_remove_pairing(&BrowserRemovePairingRequest {
                pairing_id: status.pairings[0].pairing_id.clone(),
            })
            .unwrap();
        assert!(removed.pairings.is_empty());
        assert_eq!(peer.event().await.unwrap()["event"], "unpaired");
        assert!(peer.event().await.is_none(), "closed");
        assert_eq!(
            code_of(
                env.backend()
                    .browser_remove_pairing(&BrowserRemovePairingRequest {
                        pairing_id: "00".repeat(16)
                    })
            ),
            ErrorCode::BrowserPairingNotFound
        );
        assert_eq!(
            code_of(
                env.backend()
                    .browser_remove_pairing(&BrowserRemovePairingRequest {
                        pairing_id: "not an id".into()
                    })
            ),
            ErrorCode::BrowserPairingNotFound
        );
        // The removed pairing no longer authenticates.
        assert!(matches!(
            connect(&env, &id, &key).await.err(),
            Some(NmError::Refused(Wire::NotPaired))
        ));
    });
}

#[test]
fn unpair_removes_the_pairing_and_closes_the_session() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut peer = paired_peer(&env).await;
        peer.call("unpair", json!({})).await.unwrap();
        assert!(peer.event().await.is_none());
        assert!(env.backend().browser_status().pairings.is_empty());
    });
}

#[test]
fn the_handshake_needs_the_right_key_and_a_known_pairing() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, _key) = env.add_pairing("Chrome");
        let wrong = PairingKey::generate().unwrap();
        // A host with the wrong key does not authenticate the desktop... the
        // desktop proves first with the right key, so the host rejects it.
        assert!(matches!(
            connect(&env, &id, &wrong).await.err(),
            Some(NmError::NotAuthentic)
        ));
        assert!(matches!(
            connect(&env, &[7; 16], &wrong).await.err(),
            Some(NmError::Refused(Wire::NotPaired))
        ));
    });
}

#[test]
fn a_request_before_the_handshake_closes_the_connection() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        env.add_pairing("Chrome");
        let (mut stream, server) = tokio::io::duplex(1 << 16);
        env.service().serve(server);
        let request = br#"{"v":1,"id":1,"op":"fill","body":{}}"#;
        write_frame(&mut stream, ByteOrder::Big, request)
            .await
            .unwrap();
        let reply = read_server_message(&mut stream).await.unwrap();
        assert_eq!(refusal(&reply), Wire::InvalidRequest);
        let mut byte = [0u8; 1];
        assert_eq!(stream.read(&mut byte).await.unwrap(), 0, "closed");
    });
}

#[test]
fn pre_auth_status_and_focus_work_and_focus_is_limited() {
    run(async {
        let env = Env::new();
        let (mut stream, server) = tokio::io::duplex(1 << 16);
        env.service().serve(server);
        write_message(&mut stream, &ClientMessage::Status { v: 1 })
            .await
            .unwrap();
        assert!(matches!(
            read_server_message(&mut stream).await.unwrap(),
            ServerMessage::Status {
                state: mosaic_native_messaging::protocol::LockState::Locked,
                ..
            }
        ));
        for expected_ok in [true, false] {
            let (mut stream, server) = tokio::io::duplex(1 << 16);
            env.service().serve(server);
            write_message(&mut stream, &ClientMessage::Focus { v: 1 })
                .await
                .unwrap();
            let reply = read_server_message(&mut stream).await.unwrap();
            assert_eq!(
                matches!(reply, ServerMessage::Focused { .. }),
                expected_ok,
                "{reply:?}"
            );
        }
        assert_eq!(env.focus.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn idle_pre_auth_connections_cannot_lock_the_host_out() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = env.add_pairing("Chrome");
        let mut idle = Vec::new();
        for _ in 0..8 {
            let (client, server) = tokio::io::duplex(1024);
            env.service().serve(server);
            idle.push(client);
        }
        // Only four pre-auth connections wait: each new one evicts the oldest.
        for client in &mut idle[..4] {
            let mut byte = [0u8; 1];
            let read = tokio::time::timeout(Duration::from_secs(2), client.read(&mut byte))
                .await
                .expect("closed at once");
            assert_eq!(read.unwrap(), 0);
        }
        let mut byte = [0u8; 1];
        assert!(
            tokio::time::timeout(Duration::from_millis(150), idle[7].read(&mut byte))
                .await
                .is_err(),
            "the newest still waits"
        );
        // The real host gets in anyway.
        let mut peer = connect(&env, &id, &key).await.unwrap();
        assert_eq!(
            peer.call("status", json!({})).await.unwrap()["state"],
            "unlocked"
        );
    });
}

#[test]
fn the_ninth_session_is_refused_and_pre_auth_messages_still_work() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = env.add_pairing("Chrome");
        let mut sessions = Vec::new();
        for _ in 0..8 {
            sessions.push(connect(&env, &id, &key).await.unwrap());
        }
        assert!(matches!(
            connect(&env, &id, &key).await.err(),
            Some(NmError::Refused(Wire::Busy))
        ));
        // Sessions do not use up the pre-auth slots.
        let (mut stream, server) = tokio::io::duplex(1 << 16);
        env.service().serve(server);
        write_message(&mut stream, &ClientMessage::Status { v: 1 })
            .await
            .unwrap();
        assert!(matches!(
            read_server_message(&mut stream).await.unwrap(),
            ServerMessage::Status { .. }
        ));
        drop(sessions);
    });
}

#[test]
fn the_lock_state_reaches_the_sessions_and_gates_the_ops() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut peer = paired_peer(&env).await;
        env.service().on_locked();
        let event = peer.event().await.unwrap();
        assert_eq!(
            (event["event"].as_str(), event["state"].as_str()),
            (Some("state"), Some("locked"))
        );
        let origin = json!({ "origin": "https://a.example", "topOrigin": "https://a.example" });
        for op in ["listForOrigin"] {
            assert_eq!(peer.call(op, origin.clone()).await.unwrap_err(), "locked");
        }
        assert_eq!(
            peer.call("search", json!({"query": "x", "origin": null}))
                .await
                .unwrap_err(),
            "locked"
        );
        assert_eq!(peer.call("generate", json!({"request": {"kind":"password","length":20,"lowercase":true,"uppercase":true,"digits":true,"symbols":false,"excludeAmbiguous":false}})).await.unwrap_err(), "locked");
        assert_eq!(
            peer.call("status", json!({})).await.unwrap()["state"],
            "locked"
        );
        peer.call("focus", json!({})).await.unwrap();
        env.service().on_unlocked();
        assert_eq!(peer.event().await.unwrap()["state"], "unlocked");
    });
}

// ---------------------------------------------------------------------------
// The allowlist and the limits
// ---------------------------------------------------------------------------

#[test]
fn every_op_outside_the_allowlist_is_an_invalid_request() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut peer = paired_peer(&env).await;
        for op in [
            "create_account",
            "unlock",
            "unlock_quick",
            "lock",
            "reauthenticate",
            "get_item",
            "getItem",
            "reveal_secret",
            "revealSecret",
            "add_item",
            "update_item",
            "list_items",
            "set_favorite",
            "trash_item",
            "restore_item",
            "purge_item",
            "move_item",
            "create_vault",
            "rename_vault",
            "clear_clipboard",
            "copy_to_clipboard",
            "get_settings",
            "update_settings",
            "security_report",
            "import_commit",
            "attachment_save",
            "ssh_agent_set_enabled",
            "subscribe_events",
            "browser_pairing_respond",
            "browser_save_respond",
            "browser_remove_pairing",
            "export",
            "dump",
            "Fill",
            "fill ",
            "",
            "hello",
            "pair",
        ] {
            assert_eq!(
                peer.call(op, json!({})).await.unwrap_err(),
                "invalidRequest",
                "{op:?}"
            );
        }
        // A wrong version, a body of the wrong shape and extra keys.
        let id = peer.next + 1;
        peer.next += 1;
        let bytes =
            serde_json::to_vec(&json!({"v": 2, "id": id, "op": "status", "body": {}})).unwrap();
        send(&mut peer.stream, &mut peer.session, &bytes)
            .await
            .unwrap();
        assert_eq!(peer.response(id).await.unwrap_err(), "protocolVersion");
        assert!(
            peer.call("status", json!({"extra": 1})).await.is_ok(),
            "status reads no body"
        );
        assert_eq!(
            peer.call("focus", json!({"extra": 1})).await.unwrap_err(),
            "invalidRequest"
        );
        assert_eq!(
            peer.call("fill", json!({"vaultId": 1})).await.unwrap_err(),
            "invalidRequest"
        );
    });
}

#[test]
fn the_eleventh_fill_in_ten_seconds_is_rate_limited() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut peer = paired_peer(&env).await;
        for _ in 0..10 {
            assert_eq!(
                peer.call("fill", json!({})).await.unwrap_err(),
                "invalidRequest"
            );
        }
        assert_eq!(
            peer.call("fill", json!({})).await.unwrap_err(),
            "rateLimited"
        );
        env.clock.advance(Duration::from_secs(10));
        assert_eq!(
            peer.call("fill", json!({})).await.unwrap_err(),
            "invalidRequest"
        );
    });
}

// ---------------------------------------------------------------------------
// Fill, lookup, copy and generate
// ---------------------------------------------------------------------------

/// A fill, a second after the last one (the limit is 10 in 10 seconds).
async fn fill(peer: &mut Peer, env: &Env, body: Value) -> Result<Value, String> {
    env.clock.advance(Duration::from_millis(1100));
    peer.call("fill", body).await
}

/// A `checkSave`, 7 seconds after the last one (the limit is 10 a minute).
async fn check_save(peer: &mut Peer, env: &Env, body: Value) -> Result<Value, String> {
    env.clock.advance(Duration::from_secs(7));
    peer.call("checkSave", body).await
}

fn fill_body(
    vault: VaultId,
    item: ItemId,
    origin: &str,
    top: &str,
    frame: &str,
    login: bool,
    otp: bool,
) -> Value {
    json!({
        "vaultId": vault.to_string(), "itemId": item.to_string(), "origin": origin,
        "topOrigin": top, "frame": frame, "want": { "login": login, "otp": otp }
    })
}

struct Fixture {
    bank: ItemId,
    shop: ItemId,
    plain: ItemId,
    trashed: ItemId,
    hotp: ItemId,
    totp: ItemId,
}

fn fixture(env: &Env) -> Fixture {
    let vault = env.vault();
    let bank = env.login("Bank", "alice", "bank-pw-1", "https://bank.example");
    let shop = env.login_with(
        "Shop",
        "bob",
        "shop-pw-1",
        |u| u.match_mode = UrlMatch::Etld1,
        "https://shop.example.com",
    );
    let plain = env.login_with(
        "Plain",
        "carol",
        "plain-pw-1",
        |u| u.allow_http = true,
        "http://plain.example",
    );
    let trashed = env.login("Trashed", "dan", "trashed-pw-1", "https://trashed.example");
    let hotp = env.login("Hotp", "erin", "hotp-pw-1", "https://hotp.example");
    let totp = env.login("Totp", "frank", "totp-pw-1", "https://totp.example");
    env.set_otp(
        hotp,
        &format!("otpauth://hotp/H:erin?secret={OTP_SECRET}&counter=0"),
    );
    env.set_otp(totp, &format!("otpauth://totp/T:frank?secret={OTP_SECRET}"));
    env.backend()
        .slot()
        .unwrap()
        .unlocked_mut()
        .unwrap()
        .trash_item(vault, trashed)
        .unwrap();
    Fixture {
        bank,
        shop,
        plain,
        trashed,
        hotp,
        totp,
    }
}

#[test]
fn fill_returns_a_credential_only_for_a_matching_origin() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let f = fixture(&env);
        let mut peer = paired_peer(&env).await;
        let b = "https://bank.example";

        let ok = fill(
            &mut peer,
            &env,
            fill_body(vault, f.bank, b, b, "top", true, false),
        )
        .await
        .unwrap();
        assert_eq!(
            ok,
            json!({"username": "alice", "password": "bank-pw-1", "otp": null})
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "https://evil.example",
                    "https://evil.example",
                    "top",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "originMismatch"
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "https://bank.example.evil.example",
                    "https://bank.example.evil.example",
                    "top",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "originMismatch"
        );
        // Plain HTTP only with the URL's override; a saved https URL never
        // matches an http page.
        let p = "http://plain.example";
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.plain, p, p, "top", true, false)
            )
            .await
            .unwrap()["password"],
            "plain-pw-1"
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "http://bank.example",
                    "http://bank.example",
                    "top",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "originMismatch"
        );
        let item = env.login_with(
            "Plain no override",
            "gina",
            "gina-pw",
            |_| {},
            "http://nooverride.example",
        );
        let n = "http://nooverride.example";
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, item, n, n, "top", true, false)
            )
            .await
            .unwrap_err(),
            "insecureOrigin"
        );
        // eTLD+1 only for a URL that asks for it.
        let sib = "https://login.example.com";
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.shop, sib, sib, "top", true, false)
            )
            .await
            .unwrap()["username"],
            "bob"
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "https://other.bank.example",
                    "https://other.bank.example",
                    "top",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "originMismatch"
        );
        // Not found: the trash, an unknown ID, a malformed ID.
        let t = "https://trashed.example";
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.trashed, t, t, "top", true, false)
            )
            .await
            .unwrap_err(),
            "itemNotFound"
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, ItemId::new_v7().unwrap(), b, b, "top", true, false)
            )
            .await
            .unwrap_err(),
            "itemNotFound"
        );
        let mut bad = fill_body(vault, f.bank, b, b, "top", true, false);
        bad["itemId"] = json!("nope");
        assert_eq!(
            fill(&mut peer, &env, bad).await.unwrap_err(),
            "invalidRequest"
        );
        // Frames and wants.
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "https://frame.bank.example",
                    b,
                    "sameOriginFrame",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "crossOriginFrame"
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(
                    vault,
                    f.bank,
                    "https://frame.bank.example",
                    b,
                    "top",
                    true,
                    false
                )
            )
            .await
            .unwrap_err(),
            "invalidRequest"
        );
        assert!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.bank, b, b, "sameOriginFrame", true, false)
            )
            .await
            .is_ok()
        );
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.bank, b, b, "top", false, false)
            )
            .await
            .unwrap_err(),
            "invalidRequest"
        );
    });
}

#[test]
fn fill_serves_a_totp_code_and_refuses_hotp() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let f = fixture(&env);
        let mut peer = paired_peer(&env).await;
        let (t, h, b) = (
            "https://totp.example",
            "https://hotp.example",
            "https://bank.example",
        );
        let code = fill(
            &mut peer,
            &env,
            fill_body(vault, f.totp, t, t, "top", false, true),
        )
        .await
        .unwrap();
        let digits = code["otp"].as_str().unwrap();
        assert!(
            digits.len() == 6 && digits.bytes().all(|c| c.is_ascii_digit()),
            "{digits}"
        );
        assert!(code["password"].is_null() && code["username"].is_null());
        let both = fill(
            &mut peer,
            &env,
            fill_body(vault, f.totp, t, t, "top", true, true),
        )
        .await
        .unwrap();
        assert_eq!(
            (both["password"].as_str(), both["otp"].is_string()),
            (Some("totp-pw-1"), true)
        );
        // HOTP would advance its counter: refused alone, left out with a login.
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.hotp, h, h, "top", false, true)
            )
            .await
            .unwrap_err(),
            "otpNotFillable"
        );
        let login_only = fill(
            &mut peer,
            &env,
            fill_body(vault, f.hotp, h, h, "top", true, true),
        )
        .await
        .unwrap();
        assert!(login_only["otp"].is_null() && login_only["password"] == "hotp-pw-1");
        // No OTP at all.
        assert_eq!(
            fill(
                &mut peer,
                &env,
                fill_body(vault, f.bank, b, b, "top", false, true)
            )
            .await
            .unwrap_err(),
            "noOtp"
        );
        let no_otp_login = fill(
            &mut peer,
            &env,
            fill_body(vault, f.bank, b, b, "top", true, true),
        )
        .await
        .unwrap();
        assert!(no_otp_login["otp"].is_null());
    });
}

#[test]
fn a_closed_hardware_vault_is_unavailable() {
    use crate::quick_unlock::QuickUnlockService;
    use crate::quick_unlock::fakes::FakeHardwareKey;
    let _kdf = crate::test_support::kdf_permit();
    let key = FakeHardwareKey::new(1);
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    )
    .with_quick_unlock(QuickUnlockService::new(None, Box::new(Arc::clone(&key))));
    backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    let summary = backend
        .create_hardware_vault(
            &mut serde_json::from_value(json!({ "name": "Codes", "pin": null })).unwrap(),
        )
        .unwrap();
    backend
        .close_hardware_vault(
            &serde_json::from_value(json!({ "vaultId": summary.vault_id.0.to_string() })).unwrap(),
        )
        .unwrap();
    let body: mosaic_native_messaging::bodies::FillBody = serde_json::from_value(fill_body(
        summary.vault_id.get(),
        ItemId::new_v7().unwrap(),
        "https://a.example",
        "https://a.example",
        "top",
        true,
        false,
    ))
    .unwrap();
    assert_eq!(
        backend.browser_fill(&body).unwrap_err(),
        Wire::VaultUnavailable
    );
}

#[test]
fn list_for_origin_sorts_caps_and_warns() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        // 55 matches; two favorites; titles that sort differently in
        // lower case.
        let mut ids = Vec::new();
        for n in 0..55 {
            ids.push(env.login(&format!("Site {n:02}"), "u", "pw-1", "https://many.example"));
        }
        let zed = env.login("zed", "u", "pw-1", "https://many.example");
        env.login("Apple", "u", "pw-1", "https://many.example");
        for fav in [zed, ids[7]] {
            env.backend()
                .slot()
                .unwrap()
                .unlocked_mut()
                .unwrap()
                .set_favorite(vault, fav, true)
                .unwrap();
        }
        env.login("GitHub", "u", "pw-1", "https://github.com");
        env.login("Http only", "u", "pw-1", "http://insecure.example");
        let mut peer = paired_peer(&env).await;
        let m = "https://many.example";
        let listed = peer
            .call("listForOrigin", json!({"origin": m, "topOrigin": m}))
            .await
            .unwrap();
        let items = listed["items"].as_array().unwrap();
        assert_eq!(items.len(), 50);
        assert_eq!(
            items[0]["title"], "Site 07",
            "favorites first, then lower-case title"
        );
        assert_eq!(items[1]["title"], "zed");
        assert_eq!(items[2]["title"], "Apple");
        assert_eq!(items[0]["matchedBy"], "exactOrigin");
        assert!(listed["warnings"].as_array().unwrap().is_empty());
        for item in items {
            assert!(item.get("password").is_none() && item.get("secret").is_none());
        }

        // Plain HTTP: listed but blocked, with the warning.
        let i = "http://insecure.example";
        let listed = peer
            .call("listForOrigin", json!({"origin": i, "topOrigin": i}))
            .await
            .unwrap();
        assert_eq!(listed["items"][0]["blocked"], "insecure");
        assert_eq!(listed["warnings"], json!([{"kind": "insecure"}]));
        // Loopback http is fine.
        let l = "http://localhost:3000";
        let listed = peer
            .call("listForOrigin", json!({"origin": l, "topOrigin": l}))
            .await
            .unwrap();
        assert!(listed["warnings"].as_array().unwrap().is_empty());
        // IDN and lookalike (only with no match).
        let x = "https://xn--bcher-kva.example";
        let listed = peer
            .call("listForOrigin", json!({"origin": x, "topOrigin": x}))
            .await
            .unwrap();
        assert_eq!(
            listed["warnings"][0],
            json!({"kind": "idn", "unicodeHost": "b\u{fc}cher.example"})
        );
        for look in ["https://githuh.com", "https://github.co"] {
            let listed = peer
                .call("listForOrigin", json!({"origin": look, "topOrigin": look}))
                .await
                .unwrap();
            assert_eq!(listed["items"], json!([]));
            assert_eq!(
                listed["warnings"],
                json!([{"kind": "lookalike", "savedHost": "github.com"}]),
                "{look}"
            );
        }
        let bad = peer
            .call(
                "listForOrigin",
                json!({"origin": "https://a.example/", "topOrigin": "https://a.example"}),
            )
            .await;
        assert_eq!(bad.unwrap_err(), "invalidRequest");
    });
}

#[test]
fn search_returns_at_most_twenty_logins_without_secrets() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        for n in 0..25 {
            env.login(
                &format!("Match {n:02}"),
                "u",
                "secret-pw-search",
                "https://match.example",
            );
        }
        let gone = env.login("Match trashed", "u", "pw", "https://match.example");
        env.backend()
            .slot()
            .unwrap()
            .unlocked_mut()
            .unwrap()
            .trash_item(vault, gone)
            .unwrap();
        {
            let mut slot = env.backend().slot().unwrap();
            slot.unlocked_mut()
                .unwrap()
                .add_item(vault, &ItemType::SecureNote, |e| e.set_title("Match note"))
                .unwrap();
        }
        let mut peer = paired_peer(&env).await;
        let found = peer
            .call(
                "search",
                json!({"query": "  Match ", "origin": "https://match.example"}),
            )
            .await
            .unwrap();
        let items = found["items"].as_array().unwrap();
        assert_eq!(items.len(), 20);
        assert!(
            items.iter().all(
                |i| i["fillable"] == true && i["title"].as_str().unwrap().starts_with("Match ")
            )
        );
        assert!(
            !items
                .iter()
                .any(|i| i["title"] == "Match note" || i["title"] == "Match trashed")
        );
        let elsewhere = peer
            .call(
                "search",
                json!({"query": "Match 01", "origin": "https://other.example"}),
            )
            .await
            .unwrap();
        assert_eq!(elsewhere["items"][0]["fillable"], false);
        let none = peer
            .call("search", json!({"query": "Match 01", "origin": null}))
            .await
            .unwrap();
        assert_eq!(none["items"][0]["fillable"], false);
        assert!(!found.to_string().contains("secret-pw-search"));
        for bad in ["", "   ", &"x".repeat(129)] {
            assert_eq!(
                peer.call("search", json!({"query": bad, "origin": null}))
                    .await
                    .unwrap_err(),
                "invalidRequest"
            );
        }
    });
}

#[test]
fn only_fill_and_copy_count_as_activity_and_only_fill_generate_and_otp_carry_secrets() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let f = fixture(&env);
        let mut peer = paired_peer(&env).await;
        let hour = Duration::from_hours(1);
        env.backend().activity.backdate(hour);
        assert!(env.idle() >= hour);
        let b = "https://bank.example";
        let mut quiet = Vec::new();
        quiet.push(peer.call("status", json!({})).await.unwrap());
        quiet.push(
            peer.call("listForOrigin", json!({"origin": b, "topOrigin": b}))
                .await
                .unwrap(),
        );
        quiet.push(
            peer.call("search", json!({"query": "bank", "origin": b}))
                .await
                .unwrap(),
        );
        quiet.push(
            peer.call(
                "checkSave",
                json!({"origin": b, "topOrigin": b, "username": "alice", "password": "bank-pw-1"}),
            )
            .await
            .unwrap(),
        );
        let generated = peer.call("generate", json!({"request": {"kind": "password", "length": 20, "lowercase": true, "uppercase": true, "digits": true, "symbols": false, "excludeAmbiguous": false}})).await.unwrap();
        peer.call("focus", json!({})).await.unwrap();
        assert!(env.idle() >= hour, "none of these is user activity");
        assert_eq!(generated["value"].as_str().unwrap().len(), 20);
        assert!(generated["rulesSource"].is_null());

        // The password of a login appears in no response but a fill.
        for response in &quiet {
            let text = response.to_string();
            for secret in [
                "bank-pw-1",
                "shop-pw-1",
                "plain-pw-1",
                "hotp-pw-1",
                "totp-pw-1",
                OTP_SECRET,
            ] {
                assert!(!text.contains(secret), "{text}");
            }
        }
        fill(
            &mut peer,
            &env,
            fill_body(vault, f.bank, b, b, "top", true, false),
        )
        .await
        .unwrap();
        assert!(
            env.idle() < Duration::from_secs(60),
            "a fill is user activity"
        );

        env.backend().activity.backdate(hour);
        let copied = peer.call("copy", json!({"source": {"kind": "item", "vaultId": vault.to_string(), "itemId": f.bank.to_string(), "field": "password"}})).await.unwrap();
        assert_eq!(copied["clearAfterSeconds"], 30);
        assert!(
            env.idle() < Duration::from_secs(60),
            "a copy is user activity"
        );
        assert!(!copied.to_string().contains("bank-pw-1"));
        for _ in 0..100 {
            if env.clipboard.content().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(env.clipboard.content().as_deref(), Some("bank-pw-1"));
    });
}

#[test]
fn copy_handles_generated_values_and_hides_non_logins() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let note = {
            let mut slot = env.backend().slot().unwrap();
            slot.unlocked_mut()
                .unwrap()
                .add_item(vault, &ItemType::SecureNote, |e| e.set_title("Note"))
                .unwrap()
        };
        let mut peer = paired_peer(&env).await;
        assert_eq!(
            peer.call("copy", json!({"source": {"kind": "generated"}}))
                .await
                .unwrap_err(),
            "fieldEmpty"
        );
        let generated = peer.call("generate", json!({"request": {"kind": "passphrase", "words": 5, "separator": "-", "capitalize": false, "appendDigit": false}})).await.unwrap();
        let value = generated["value"].as_str().unwrap().to_owned();
        peer.call("copy", json!({"source": {"kind": "generated"}}))
            .await
            .unwrap();
        for _ in 0..100 {
            if env.clipboard.content().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(env.clipboard.content(), Some(value));
        let item = json!({"source": {"kind": "item", "vaultId": vault.to_string(), "itemId": note.to_string(), "field": "password"}});
        assert_eq!(peer.call("copy", item).await.unwrap_err(), "itemNotFound");
    });
}

#[test]
fn generate_follows_page_rules_then_site_quirks_then_defaults() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut peer = paired_peer(&env).await;
        let site = |host: &str, rules: Value| json!({"request": {"kind": "forSite", "host": host, "rules": rules}});
        let page = peer
            .call("generate", site("example.test", json!("minlength: 24; required: lower; required: upper; required: digit; allowed: lower, upper, digit;")))
            .await
            .unwrap();
        assert_eq!(page["rulesSource"], "page");
        assert!(page["value"].as_str().unwrap().len() >= 24);
        assert!(page["entropyBits"].as_f64().unwrap() > 60.0);
        let defaults = peer
            .call("generate", site("example.test", Value::Null))
            .await
            .unwrap();
        assert!(defaults["rulesSource"].is_null());
        let quirks = peer
            .call("generate", site("www.163.com", Value::Null))
            .await
            .unwrap();
        assert_eq!(quirks["rulesSource"], "siteQuirks");
        assert_eq!(
            peer.call(
                "generate",
                json!({"request": {"kind": "forSite", "host": "example.test"}})
            )
            .await
            .unwrap_err(),
            "invalidRequest",
            "every key must be present"
        );
        assert_eq!(
            peer.call("generate", site("example.test", json!("x".repeat(4097))))
                .await
                .unwrap_err(),
            "invalidRequest"
        );
    });
}

// ---------------------------------------------------------------------------
// Save and update
// ---------------------------------------------------------------------------

fn save_body(origin: &str, user: Option<&str>, password: &str) -> Value {
    json!({"origin": origin, "topOrigin": origin, "username": user, "password": password})
}

#[test]
fn check_save_tells_new_update_and_unchanged() {
    run(async {
        let env = Env::unlocked();
        env.login(
            "Example",
            "alex@example.com",
            "old-pw",
            "https://example.com",
        );
        env.login_with(
            "Sibling",
            "sam",
            "sam-pw",
            |u| u.match_mode = UrlMatch::Etld1,
            "https://www.example.org",
        );
        let mut peer = paired_peer(&env).await;
        let o = "https://example.com";
        let check = |user: Option<&str>, pw: &str| save_body(o, user, pw);
        let kind = |v: Value| {
            (
                v["kind"].as_str().unwrap().to_owned(),
                v["itemTitle"].as_str().map(str::to_owned),
            )
        };
        assert_eq!(
            kind(
                check_save(&mut peer, &env, check(Some("alex@example.com"), "old-pw"))
                    .await
                    .unwrap()
            ),
            ("unchanged".into(), None)
        );
        assert_eq!(
            kind(
                check_save(&mut peer, &env, check(Some("ALEX@Example.com"), "new-pw"))
                    .await
                    .unwrap()
            ),
            ("update".into(), Some("Example".into()))
        );
        assert_eq!(
            kind(
                check_save(&mut peer, &env, check(Some("other"), "new-pw"))
                    .await
                    .unwrap()
            )
            .0,
            "new"
        );
        assert_eq!(
            kind(
                check_save(&mut peer, &env, check(None, "old-pw"))
                    .await
                    .unwrap()
            )
            .0,
            "unchanged"
        );
        assert_eq!(
            kind(
                check_save(&mut peer, &env, check(None, "new-pw"))
                    .await
                    .unwrap()
            )
            .0,
            "new"
        );
        // eTLD+1 matches count; other sites do not.
        let sibling = "https://login.example.org";
        assert_eq!(
            kind(
                check_save(&mut peer, &env, save_body(sibling, Some("sam"), "changed"))
                    .await
                    .unwrap()
            )
            .0,
            "update"
        );
        assert_eq!(
            kind(
                check_save(
                    &mut peer,
                    &env,
                    save_body("https://elsewhere.example", Some("sam"), "changed")
                )
                .await
                .unwrap()
            )
            .0,
            "new"
        );
        // Sizes.
        for bad in [
            save_body(o, None, ""),
            save_body(o, Some(&"u".repeat(257)), "x"),
            save_body(o, None, &"p".repeat(1025)),
        ] {
            assert_eq!(
                check_save(&mut peer, &env, bad).await.unwrap_err(),
                "invalidRequest"
            );
        }
    });
}

fn save_request_id(env: &Env) -> String {
    env.events_of("browserSaveRequested").pop().unwrap()["request"]["requestId"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn respond_save(request_id: &str, decision: BrowserSaveDecision) -> BrowserSaveRespondRequest {
    BrowserSaveRespondRequest {
        request_id: request_id.to_owned(),
        decision,
    }
}

#[test]
fn propose_save_adds_a_login_after_the_user_confirms() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let mut peer = paired_peer(&env).await;
        let id = peer
            .request(
                "proposeSave",
                save_body("http://insecure.example", Some("bob"), "captured-pw-1"),
            )
            .await;
        wait_for_event(&env, "browserSaveRequested").await;
        let event = env.events_of("browserSaveRequested").pop().unwrap();
        assert_eq!(event["request"]["kind"], "new");
        assert_eq!(event["request"]["insecure"], true);
        assert_eq!(event["request"]["pairingLabel"], "Chrome");
        assert_eq!(event["request"]["host"], "insecure.example");
        assert_eq!(event["request"]["suggestedTitle"], "insecure.example");
        assert!(
            !event.to_string().contains("captured-pw-1"),
            "the password never reaches the webview"
        );
        let request = save_request_id(&env);

        let new = |title: &str, allow_http| BrowserSaveDecision::SaveNew {
            vault_id: vault.into(),
            title: title.into(),
            allow_http,
        };
        assert_eq!(
            code_of(
                env.backend()
                    .browser_save_respond(&respond_save(&request, new("Insecure", true)))
            ),
            ErrorCode::BrowserAllowTooSoon
        );
        env.clock.advance(Duration::from_secs(1));
        assert_eq!(
            code_of(
                env.backend()
                    .browser_save_respond(&respond_save(&request, new("", true)))
            ),
            ErrorCode::InvalidInput
        );
        assert_eq!(
            code_of(env.backend().browser_save_respond(&respond_save(
                &request,
                BrowserSaveDecision::SaveNew {
                    vault_id: VaultId::new_v7().unwrap().into(),
                    title: "x".into(),
                    allow_http: false
                }
            ))),
            ErrorCode::VaultNotFound
        );
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_some(),
            "still open"
        );
        env.backend()
            .browser_save_respond(&respond_save(&request, new("Insecure", true)))
            .unwrap();
        let saved = peer.response(id).await.unwrap();
        assert_eq!(saved["outcome"], "saved");
        assert_eq!(saved["vaultId"], vault.to_string());
        let item: ItemId =
            serde_json::from_value::<crate::contract::ids::WireItemId>(saved["itemId"].clone())
                .unwrap()
                .get();

        {
            let slot = env.backend().slot().unwrap();
            let stored = slot.unlocked().unwrap().item(vault, item).unwrap();
            assert_eq!(stored.title(), Some("Insecure"));
            assert_eq!(stored.text(TextField::Username), Some("bob"));
            assert_eq!(
                stored
                    .secret(SecretField::Password)
                    .unwrap()
                    .unwrap()
                    .expose_secret(),
                "captured-pw-1"
            );
            let urls = stored.urls();
            assert_eq!(urls.len(), 1);
            assert_eq!(urls[0].1.url.as_str(), "http://insecure.example");
            assert_eq!(urls[0].1.match_mode, UrlMatch::ExactOrigin);
            assert!(urls[0].1.allow_http);
        }
        assert_eq!(env.events_of("browserSaveClosed")[0]["reason"], "answered");
        // Asking again about the same login: nothing changes, no dialog.
        let again = peer
            .call(
                "proposeSave",
                save_body("http://insecure.example", Some("bob"), "captured-pw-1"),
            )
            .await
            .unwrap();
        assert_eq!(again["outcome"], "cancelled");
        assert_eq!(env.events_of("browserSaveRequested").len(), 1);
    });
}

async fn wait_for_event(env: &Env, kind: &str) {
    for _ in 0..200 {
        if !env.events_of(kind).is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no {kind} event");
}

#[test]
fn propose_save_updates_cancels_and_expires() {
    run(async {
        let env = Env::unlocked_with(Limits {
            save_timeout: Duration::from_millis(1500),
            ..Limits::default()
        });
        let vault = env.vault();
        let item = env.login(
            "Example",
            "alex@example.com",
            "old-pw",
            "https://example.com",
        );
        let mut peer = paired_peer(&env).await;
        let o = "https://example.com";

        // Update: the old password moves into the history.
        let id = peer
            .request(
                "proposeSave",
                save_body(o, Some("alex@example.com"), "new-pw"),
            )
            .await;
        wait_for_event(&env, "browserSaveRequested").await;
        let event = env.events_of("browserSaveRequested").pop().unwrap();
        assert_eq!(event["request"]["kind"], "update");
        assert_eq!(
            event["request"]["candidates"][0]["itemId"],
            item.to_string()
        );
        // A second request while this one is open.
        assert_eq!(
            peer.call("proposeSave", save_body(o, Some("zed"), "zed-pw"))
                .await
                .unwrap_err(),
            "busy"
        );
        let request = save_request_id(&env);
        env.clock.advance(Duration::from_secs(2));
        // Not a candidate: refused, and still open.
        let stranger = env.login("Other", "alex@example.com", "x", "https://other.example");
        assert_eq!(
            code_of(env.backend().browser_save_respond(&respond_save(
                &request,
                BrowserSaveDecision::Update {
                    vault_id: vault.into(),
                    item_id: stranger.into()
                }
            ))),
            ErrorCode::ItemNotFound
        );
        env.backend()
            .browser_save_respond(&respond_save(
                &request,
                BrowserSaveDecision::Update {
                    vault_id: vault.into(),
                    item_id: item.into(),
                },
            ))
            .unwrap();
        let done = peer.response(id).await.unwrap();
        assert_eq!(
            (
                done["outcome"].as_str(),
                done["itemId"].as_str().map(str::to_owned)
            ),
            (Some("updated"), Some(item.to_string()))
        );
        assert_eq!(env.password_of(item), "new-pw");
        let history = env
            .backend()
            .slot()
            .unwrap()
            .unlocked()
            .unwrap()
            .item(vault, item)
            .unwrap()
            .password_history()
            .len();
        assert_eq!(history, 1);

        // Cancel.
        env.clock.advance(Duration::from_secs(61));
        let id = peer
            .request(
                "proposeSave",
                save_body(o, Some("alex@example.com"), "cancelled-pw"),
            )
            .await;
        wait_for_event_n(&env, "browserSaveRequested", 2).await;
        let request = save_request_id(&env);
        env.backend()
            .browser_save_respond(&respond_save(&request, BrowserSaveDecision::Cancel))
            .unwrap();
        assert_eq!(peer.response(id).await.unwrap()["outcome"], "cancelled");
        assert_eq!(env.password_of(item), "new-pw");

        // Expiry: nobody answers; the password is dropped.
        env.clock.advance(Duration::from_secs(61));
        let result = peer
            .call(
                "proposeSave",
                save_body(o, Some("alex@example.com"), "expired-pw"),
            )
            .await
            .unwrap();
        assert_eq!(result["outcome"], "cancelled");
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_none()
        );
        let closed = env.events_of("browserSaveClosed");
        assert_eq!(closed.last().unwrap()["reason"], "expired");
        assert_eq!(env.password_of(item), "new-pw");
        // An answer after the end.
        assert_eq!(
            code_of(env.backend().browser_save_respond(&respond_save(
                &save_request_id(&env),
                BrowserSaveDecision::Cancel
            ))),
            ErrorCode::BrowserRequestExpired
        );
    });
}

async fn wait_for_event_n(env: &Env, kind: &str, n: usize) {
    for _ in 0..200 {
        if env.events_of(kind).len() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no {kind} event number {n}");
}

#[test]
fn locking_while_a_save_is_pending_refuses_it_and_drops_the_password() {
    run(async {
        let env = Env::unlocked();
        let mut peer = paired_peer(&env).await;
        let id = peer
            .request(
                "proposeSave",
                save_body("https://example.com", Some("bob"), "pending-pw"),
            )
            .await;
        wait_for_event(&env, "browserSaveRequested").await;
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_some()
        );
        env.backend().lock(LockReason::Manual).unwrap();
        assert_eq!(peer.response(id).await.unwrap_err(), "locked");
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_none()
        );
        assert_eq!(env.events_of("browserSaveClosed")[0]["reason"], "locked");
        // The session stays up and is told.
        assert_eq!(peer.event().await.unwrap()["state"], "locked");
    });
}

// ---------------------------------------------------------------------------
// Review follow-ups: revocation, prompt owners, HOTP copy, the registry
// ---------------------------------------------------------------------------

/// The host's side of the handshake up to the challenge: the stream, `nonce_h`
/// and `nonce_d`. The test then does what it wants before `finish`.
async fn hello(env: &Env, id: &[u8; 16]) -> (DuplexStream, [u8; 32], [u8; 32]) {
    let (mut stream, server) = tokio::io::duplex(1 << 20);
    env.service().serve(server);
    let nonce_h = new_channel_nonce().unwrap();
    let hello = ClientMessage::Hello {
        v: PROTOCOL_VERSION,
        pairing_id: Hex(*id),
        nonce: Hex(nonce_h),
    };
    write_message(&mut stream, &hello).await.unwrap();
    let ServerMessage::Challenge { nonce, .. } = read_server_message(&mut stream).await.unwrap()
    else {
        panic!("no challenge");
    };
    (stream, nonce_h, nonce.0)
}

/// The rest of the handshake: the peer and the first frame (the state).
async fn finish(
    mut stream: DuplexStream,
    key: &PairingKey,
    id: &[u8; 16],
    nonce_h: [u8; 32],
    nonce_d: [u8; 32],
) -> (Peer, Value) {
    let proof = key.proof(Role::Host, id, &nonce_h, &nonce_d).unwrap();
    let finish = ClientMessage::Finish {
        v: PROTOCOL_VERSION,
        proof: Hex(proof),
    };
    write_message(&mut stream, &finish).await.unwrap();
    let mut session = key.session(Role::Host, id, &nonce_h, &nonce_d).unwrap();
    let frame = read_frame(&mut stream, ByteOrder::Big)
        .await
        .unwrap()
        .expect("the first frame");
    let first: Value =
        serde_json::from_slice(session.open(&frame).unwrap().expose_secret()).unwrap();
    let peer = Peer {
        stream,
        session,
        next: 0,
        events: Vec::new(),
    };
    (peer, first)
}

#[test]
fn a_pairing_removed_during_the_handshake_never_gets_a_session() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = env.add_pairing("Chrome");
        let (stream, nonce_h, nonce_d) = hello(&env, &id).await;
        // The user disconnects the browser between `hello` and `finish`.
        env.backend()
            .browser_remove_pairing(&BrowserRemovePairingRequest {
                pairing_id: crate::browser::hex(&id),
            })
            .unwrap();
        let (mut peer, first) = finish(stream, &key, &id, nonce_h, nonce_d).await;
        assert_eq!(first["event"], "state");
        // It is told and closed, and no request is ever served.
        assert_eq!(peer.event().await.unwrap()["event"], "unpaired");
        assert!(peer.event().await.is_none(), "closed");
        assert!(env.backend().browser_status().pairings.is_empty());
    });
}

#[test]
fn a_request_after_the_pairing_is_gone_is_not_paired_and_closes() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = env.add_pairing("Chrome");
        let mut peer = connect(&env, &id, &key).await.unwrap();
        // The session registers right after the handshake's first frame; wait for it, so this
        // test reaches `dispatch` (a removal before registering is refused by `register`).
        connected_status(&env).await;
        // Removed behind the sessions' back, as a race would do.
        env.service()
            .shared()
            .registry
            .lock()
            .unwrap()
            .remove(&id)
            .unwrap();
        assert_eq!(
            peer.call("status", json!({})).await.unwrap_err(),
            "notPaired"
        );
        assert!(peer.event().await.is_none(), "closed");
    });
}

#[test]
fn a_lock_during_the_handshake_is_pushed_to_the_new_session() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let (id, key) = env.add_pairing("Chrome");
        let (stream, nonce_h, nonce_d) = hello(&env, &id).await;
        env.service().on_locked();
        let (mut peer, first) = finish(stream, &key, &id, nonce_h, nonce_d).await;
        assert_eq!(
            first["state"], "unlocked",
            "what the handshake started with"
        );
        let event = peer.event().await.unwrap();
        assert_eq!(
            (event["event"].as_str(), event["state"].as_str()),
            (Some("state"), Some("locked"))
        );
        assert_eq!(peer.call("fill", json!({})).await.unwrap_err(), "locked");
    });
}

#[test]
fn a_save_prompt_ends_when_its_browser_disconnects() {
    run(async {
        let env = Env::unlocked();
        let mut peer = paired_peer(&env).await;
        peer.request(
            "proposeSave",
            save_body("https://example.com", Some("bob"), "pending-pw"),
        )
        .await;
        wait_for_event(&env, "browserSaveRequested").await;
        let request = save_request_id(&env);
        drop(peer);
        wait_for_event(&env, "browserSaveClosed").await;
        assert_eq!(
            env.events_of("browserSaveClosed")[0]["reason"],
            "disconnected"
        );
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_none(),
            "the password is dropped"
        );
        assert_eq!(
            code_of(
                env.backend()
                    .browser_save_respond(&respond_save(&request, BrowserSaveDecision::Cancel))
            ),
            ErrorCode::BrowserRequestExpired
        );
    });
}

#[test]
fn a_save_prompt_ends_when_its_pairing_is_removed() {
    run(async {
        let env = Env::unlocked();
        let mut peer = paired_peer(&env).await;
        peer.request(
            "proposeSave",
            save_body("https://example.com", Some("bob"), "pending-pw"),
        )
        .await;
        wait_for_event(&env, "browserSaveRequested").await;
        let request = save_request_id(&env);
        let pairing_id = env.backend().browser_status().pairings[0]
            .pairing_id
            .clone();
        env.backend()
            .browser_remove_pairing(&BrowserRemovePairingRequest { pairing_id })
            .unwrap();
        assert_eq!(
            env.events_of("browserSaveClosed")[0]["reason"],
            "disconnected"
        );
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_none()
        );
        assert_eq!(
            code_of(
                env.backend()
                    .browser_save_respond(&respond_save(&request, BrowserSaveDecision::Cancel))
            ),
            ErrorCode::BrowserRequestExpired
        );
    });
}

#[test]
fn copying_the_code_of_an_hotp_login_is_refused_and_leaves_the_counter() {
    run(async {
        let env = Env::unlocked();
        let vault = env.vault();
        let f = fixture(&env);
        let mut peer = paired_peer(&env).await;
        let copy = |item: ItemId| {
            json!({"source": {"kind": "item", "vaultId": vault.to_string(),
                "itemId": item.to_string(), "field": "totp"}})
        };
        let counter = || {
            let slot = env.backend().slot().unwrap();
            slot.unlocked()
                .unwrap()
                .item(vault, f.hotp)
                .unwrap()
                .otp_counter()
        };
        assert_eq!(counter(), 0);
        assert_eq!(
            peer.call("copy", copy(f.hotp)).await.unwrap_err(),
            "otpNotFillable"
        );
        assert_eq!(counter(), 0, "no vault write");
        assert_eq!(env.clipboard.content(), None);
        // A time-based code is copied.
        peer.call("copy", copy(f.totp)).await.unwrap();
        for _ in 0..100 {
            if env.clipboard.content().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let code = env
            .clipboard
            .content()
            .expect("the code is on the clipboard");
        assert!(
            code.len() == 6 && code.bytes().all(|c| c.is_ascii_digit()),
            "{code}"
        );
    });
}

#[test]
fn propose_save_answers_busy_before_it_looks_at_the_vault() {
    run(async {
        let env = Env::unlocked();
        env.login(
            "Example",
            "alex@example.com",
            "old-pw",
            "https://example.com",
        );
        let mut peer = paired_peer(&env).await;
        peer.request(
            "proposeSave",
            save_body("https://example.com", Some("alex@example.com"), "new-pw"),
        )
        .await;
        wait_for_event(&env, "browserSaveRequested").await;
        // The password equals the saved one: with no prompt open this would be
        // `cancelled`. With one open it is `busy`, whatever is saved.
        env.clock.advance(Duration::from_secs(61));
        assert_eq!(
            peer.call(
                "proposeSave",
                save_body("https://example.com", Some("alex@example.com"), "old-pw"),
            )
            .await
            .unwrap_err(),
            "busy"
        );
        assert_eq!(
            peer.call(
                "proposeSave",
                save_body("https://example.com", Some("alex@example.com"), ""),
            )
            .await
            .unwrap_err(),
            "busy",
            "not even the sizes are checked"
        );
    });
}

#[test]
fn propose_save_counts_against_the_check_save_limit_too() {
    run(async {
        let env = Env::unlocked();
        env.login(
            "Example",
            "alex@example.com",
            "old-pw",
            "https://example.com",
        );
        let mut peer = paired_peer(&env).await;
        let same = save_body("https://example.com", Some("alex@example.com"), "old-pw");
        // Nine `checkSave` in a minute, then a `proposeSave` is the tenth.
        for _ in 0..9 {
            peer.call("checkSave", same.clone()).await.unwrap();
        }
        assert_eq!(
            peer.call("proposeSave", same.clone()).await.unwrap()["outcome"],
            "cancelled"
        );
        assert_eq!(
            peer.call("checkSave", same).await.unwrap_err(),
            "rateLimited"
        );
    });
}

#[test]
fn captured_user_names_with_control_or_spoofing_characters_are_refused() {
    run(async {
        let env = Env::unlocked();
        let mut peer = paired_peer(&env).await;
        let o = "https://example.com";
        for name in ["bo\u{202e}b", "a\tb", "a\nb", "a\u{200b}b"] {
            assert_eq!(
                check_save(&mut peer, &env, save_body(o, Some(name), "pw"))
                    .await
                    .unwrap_err(),
                "invalidRequest",
                "{name:?}"
            );
        }
        assert_eq!(
            check_save(&mut peer, &env, save_body(o, Some("bob"), "pw"))
                .await
                .unwrap()["kind"],
            "new"
        );
    });
}

/// A stream the host never closes, which reads one message and whose writes
/// fail once `fail` is set.
struct BreaksOnDemand {
    input: Vec<u8>,
    at: usize,
    fail: Arc<AtomicBool>,
    /// Everything written while the stream still worked.
    written: Arc<Mutex<Vec<u8>>>,
}

impl AsyncRead for BreaksOnDemand {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let left = self.input.len() - self.at;
        if left == 0 {
            return Poll::Pending;
        }
        let n = left.min(buf.remaining());
        let (from, to) = (self.at, self.at + n);
        buf.put_slice(&self.input[from..to]);
        self.at = to;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for BreaksOnDemand {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.fail.load(Ordering::SeqCst) {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        }
        self.written.lock().unwrap().extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.fail.load(Ordering::SeqCst) {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn a_pairing_whose_approval_cannot_be_delivered_is_removed_again() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let mut input = Vec::new();
        let request = ClientMessage::PairRequest {
            v: PROTOCOL_VERSION,
            browser: BrowserKind::Chromium,
            hint: Hint::Edge,
        };
        write_message(&mut input, &request).await.unwrap();
        let fail = Arc::new(AtomicBool::new(false));
        let written = Arc::new(Mutex::new(Vec::new()));
        env.service().serve(BreaksOnDemand {
            input,
            at: 0,
            fail: Arc::clone(&fail),
            written: Arc::clone(&written),
        });
        wait_for_event(&env, "browserPairingRequested").await;
        // Wait for the whole `pairPending` frame.
        for _ in 0..200 {
            let complete = {
                let sent = written.lock().unwrap();
                sent.first_chunk::<4>().is_some_and(|head| {
                    sent.len() >= 4 + usize::try_from(u32::from_be_bytes(*head)).unwrap()
                })
            };
            if complete {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let request_id =
            env.events_of("browserPairingRequested").pop().unwrap()["request"]["requestId"]
                .as_str()
                .unwrap()
                .to_owned();
        // The pending message went out; now the connection breaks.
        fail.store(true, Ordering::SeqCst);
        env.clock.advance(Duration::from_millis(1100));
        env.backend()
            .browser_pairing_respond(&respond_pair(&request_id, true, None))
            .unwrap();
        let mut gone = false;
        for _ in 0..200 {
            if env.backend().browser_status().pairings.is_empty() {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(gone, "nobody got the key, so the pairing is removed");
    });
}

#[test]
fn an_unreadable_registry_is_reported_and_left_alone() {
    run(async {
        let env = Env::new();
        env.service().on_unlocked();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("browser-pairings.json");
        std::fs::write(&file, "not the pairings").unwrap();
        env.use_dir(dir.path());
        let status = env.backend().browser_status();
        assert_eq!(status.registry, BrowserRegistryState::Unreadable);
        assert_eq!(
            serde_json::to_value(&status).unwrap()["registry"],
            "unreadable"
        );
        // No pairing is offered, none is removed, and the file stays.
        let (_stream, reply) = pair_request(&env).await;
        assert_eq!(refusal(&reply), Wire::Internal);
        assert!(env.events_of("browserPairingRequested").is_empty());
        assert!(
            env.backend()
                .browser_remove_pairing(&BrowserRemovePairingRequest {
                    pairing_id: "ab".repeat(16)
                })
                .is_err()
        );
        assert_eq!(std::fs::read(&file).unwrap(), b"not the pairings");

        // A readable (here: missing) file is fine again.
        let fresh = tempfile::tempdir().unwrap();
        env.use_dir(fresh.path());
        assert_eq!(
            env.backend().browser_status().registry,
            BrowserRegistryState::Ok
        );
    });
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

#[test]
fn the_app_identifier_is_the_bundle_identifier() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(
        config["identifier"],
        mosaic_native_messaging::endpoint::APP_IDENTIFIER
    );
}

#[test]
fn a_failed_save_answer_after_the_request_expired_leaves_no_prompt_behind() {
    run(async {
        let env = Env::unlocked_with(Limits {
            save_timeout: Duration::from_millis(700),
            ..Limits::default()
        });
        let mut peer = paired_peer(&env).await;
        let id = peer
            .request(
                "proposeSave",
                save_body("https://new.example", Some("bob"), "captured-pw"),
            )
            .await;
        wait_for_event(&env, "browserSaveRequested").await;
        env.clock.advance(Duration::from_secs(2));
        // A vault that does not exist: the answer fails once it has the vault
        // slot, which the test holds while the request expires.
        let request = respond_save(
            &save_request_id(&env),
            BrowserSaveDecision::SaveNew {
                vault_id: VaultId::new_v7().unwrap().into(),
                title: "Bob".into(),
                allow_http: false,
            },
        );
        let slot = env.backend().slot().unwrap();
        let backend = env.backend();
        let answered = std::thread::scope(|scope| {
            let answered = scope.spawn(move || backend.browser_save_respond(&request));
            std::thread::sleep(Duration::from_millis(1500));
            drop(slot);
            answered.join().unwrap()
        });
        assert_eq!(code(answered.unwrap_err()), ErrorCode::VaultNotFound);
        assert_eq!(peer.response(id).await.unwrap()["outcome"], "cancelled");
        assert!(
            env.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .save_id()
                .is_none(),
            "the prompt is not put back"
        );
        let closed = env.events_of("browserSaveClosed");
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0]["reason"], "expired");
    });
}

// ---------------------------------------------------------------------------
// Passkeys (M5 T2, ADR-0039, ADR-0040)
// ---------------------------------------------------------------------------

mod passkeys {
    use std::collections::HashMap;

    use mosaic_core::Timestamp;
    use mosaic_core::model::{PasskeyCredential, Text};
    use mosaic_passkey::{CreateInput, b64url, from_b64url};
    use p256::ecdsa::signature::Verifier as _;
    use p256::ecdsa::{Signature, VerifyingKey};
    use p256::pkcs8::DecodePublicKey as _;
    use sha2::{Digest as _, Sha256};

    use super::*;
    use crate::contract::browser::{
        BrowserPasskeyDecision as Decision, BrowserPasskeyRespondRequest,
    };

    const CHALLENGE: [u8; 32] = [7; 32];
    const SITE: &str = "https://login.example.com";

    fn rid(n: u8) -> String {
        format!("{n:032x}")
    }

    thread_local! {
        /// The dialog handle (the desktop's own ID) of each prompt the test
        /// opened, by the extension's request ID.
        static HANDLES: std::cell::RefCell<HashMap<String, String>> =
            std::cell::RefCell::new(HashMap::new());
    }

    /// The handle of the dialog opened for the extension's request `n`.
    fn handle_of(n: u8) -> String {
        HANDLES.with(|handles| {
            handles
                .borrow()
                .get(&rid(n))
                .cloned()
                .unwrap_or_else(|| rid(n))
        })
    }

    fn create_body(origin: &str, rp: Option<&str>, uv: &str, request: u8) -> Value {
        json!({
            "requestId": rid(request), "origin": origin, "topOrigin": origin, "rpId": rp,
            "userId": b64url(&[1, 2, 3, 4]), "userName": "alex@example.com",
            "userDisplayName": "Alex Example", "challenge": b64url(&CHALLENGE),
            "algorithms": [-7, -257], "excludeCredentials": [], "userVerification": uv,
        })
    }

    fn get_body(origin: &str, rp: Option<&str>, allow: &[&[u8]], uv: &str, request: u8) -> Value {
        json!({
            "requestId": rid(request), "origin": origin, "topOrigin": origin, "rpId": rp,
            "challenge": b64url(&CHALLENGE),
            "allowCredentials": allow.iter().map(|id| b64url(id)).collect::<Vec<_>>(),
            "userVerification": uv,
        })
    }

    fn answer(
        request: u8,
        decision: Decision,
        password: Option<&str>,
    ) -> BrowserPasskeyRespondRequest {
        BrowserPasskeyRespondRequest {
            request_id: handle_of(request),
            decision,
            password: password.map(SecretIn::test),
        }
    }

    fn respond(env: &Env, request: u8, decision: Decision) -> Result<(), ErrorCode> {
        env.backend()
            .browser_passkey_respond(&answer(request, decision, None))
            .map_err(code)
    }

    fn assert_with(item: ItemId, vault: VaultId) -> Decision {
        Decision::Assert {
            vault_id: vault.into(),
            item_id: item.into(),
        }
    }

    fn create_in(vault: VaultId) -> Decision {
        Decision::Create {
            vault_id: vault.into(),
        }
    }

    /// A stored passkey of the tests, with the public key to check signatures.
    struct Seeded {
        item: ItemId,
        credential_id: Vec<u8>,
        spki: Vec<u8>,
        user_handle: Vec<u8>,
    }

    impl Env {
        fn seed_passkey(&self, rp: &str, user: &str, discoverable: bool, count: u64) -> Seeded {
            let created = mosaic_passkey::create(&CreateInput {
                rp_id: rp,
                origin: &format!("https://{rp}"),
                challenge: &CHALLENGE,
                user_handle: &[9; 16],
                algorithms: &[-7],
                user_verified: false,
            })
            .unwrap();
            let mut credential = PasskeyCredential::new(
                created.credential_id.clone(),
                Text::new(rp),
                vec![9; 16],
                created.private_key_pkcs8,
                -7,
                Timestamp::from_unix_millis(1_000),
            );
            credential.user_name = Text::new(user);
            credential.discoverable = discoverable;
            let vault = self.vault();
            let mut slot = self.backend().slot().unwrap();
            let item = slot
                .unlocked_mut()
                .unwrap()
                .add_item(vault, &ItemType::Passkey, |e| {
                    e.set_title(rp)?;
                    e.set_passkey(&credential)?;
                    if count > 0 {
                        e.set_passkey_sign_count(count)?;
                    }
                    Ok(())
                })
                .unwrap();
            Seeded {
                item,
                credential_id: created.credential_id,
                spki: created.public_key_spki,
                user_handle: vec![9; 16],
            }
        }

        /// Every `passkey` item: its credential and sign count.
        fn passkeys(&self) -> Vec<(ItemId, PasskeyCredential, u64)> {
            let slot = self.backend().slot().unwrap();
            let session = slot.unlocked().unwrap();
            session
                .items(&mosaic_vault::session::ItemFilter::default())
                .into_iter()
                .filter_map(|e| Some((e.item_id, e.item.passkey()?, e.item.passkey_sign_count())))
                .collect()
        }

        fn passkey_open(&self) -> bool {
            self.service()
                .shared()
                .prompts
                .lock()
                .unwrap()
                .passkey_id()
                .is_some()
        }
    }

    /// A call after the rate-limit windows slid (for tests that send many requests).
    async fn ask(env: &Env, peer: &mut Peer, op: &str, body: Value) -> Result<Value, String> {
        env.clock.advance(Duration::from_secs(61));
        peer.call(op, body).await
    }

    /// Sends a passkey request and waits until its dialog is open.
    async fn start(env: &Env, peer: &mut Peer, op: &str, body: Value) -> u32 {
        let before = env.events_of("browserPasskeyRequested").len();
        let extension_id = body["requestId"].as_str().unwrap().to_owned();
        let id = peer.request(op, body).await;
        wait_for_event_n(env, "browserPasskeyRequested", before + 1).await;
        let handle = last_request(env)["requestId"].as_str().unwrap().to_owned();
        HANDLES.with(|handles| handles.borrow_mut().insert(extension_id, handle));
        id
    }

    fn last_request(env: &Env) -> Value {
        env.events_of("browserPasskeyRequested").pop().unwrap()["request"].clone()
    }

    /// The Allow guard of the dialog just shown has passed.
    fn pass_guard(env: &Env) {
        env.clock.advance(Duration::from_millis(1100));
    }

    fn decode(value: &Value) -> Vec<u8> {
        from_b64url(value.as_str().unwrap(), 4096).unwrap()
    }

    fn client_data(value: &Value) -> Value {
        serde_json::from_slice(&decode(value)).unwrap()
    }

    /// Checks `signature` over `authenticatorData || SHA-256(clientDataJSON)` with a SPKI.
    fn verifies(spki: &[u8], assertion: &Value) -> bool {
        let mut message = decode(&assertion["authenticatorData"]);
        message.extend_from_slice(&Sha256::digest(decode(&assertion["clientDataJson"])));
        let key = VerifyingKey::from_public_key_der(spki).unwrap();
        let signature = Signature::from_der(&decode(&assertion["signature"])).unwrap();
        key.verify(&message, &signature).is_ok()
    }

    fn rp_hash(rp: &str) -> Vec<u8> {
        Sha256::digest(rp.as_bytes()).to_vec()
    }

    #[test]
    fn create_writes_the_item_before_it_answers_and_the_new_key_signs() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let mut peer = paired_peer(&env).await;
            let id = start(
                &env,
                &mut peer,
                "passkeyCreate",
                create_body(SITE, Some("example.com"), "preferred", 1),
            )
            .await;
            let request = last_request(&env);
            assert_eq!(request["kind"], "create");
            // The handle is the desktop's own, not the extension's request ID.
            assert_eq!(request["requestId"].as_str().unwrap().len(), 32);
            assert_ne!(request["requestId"], rid(1));
            assert_eq!(request["pairingLabel"], "Chrome");
            assert_eq!(request["origin"], SITE);
            assert_eq!(request["rpId"], "example.com");
            assert_eq!(request["userName"], "alex@example.com");
            assert_eq!(request["userDisplayName"], "Alex Example");
            assert_eq!(request["candidates"], json!([]));
            assert_eq!(request["excluded"], false);
            assert_eq!(request["needsPassword"], false, "unlocked a moment ago");
            assert_eq!(request["userVerification"], "preferred");
            assert!(!request.to_string().contains("challenge"));

            // The Allow guard: nothing is written before it.
            assert_eq!(
                respond(&env, 1, create_in(vault)),
                Err(ErrorCode::BrowserAllowTooSoon)
            );
            assert!(env.passkeys().is_empty());
            pass_guard(&env);
            assert_eq!(
                respond(
                    &env,
                    1,
                    Decision::Create {
                        vault_id: VaultId::new_v7().unwrap().into()
                    }
                ),
                Err(ErrorCode::VaultNotFound)
            );
            assert!(env.passkey_open(), "still open after an error");
            respond(&env, 1, create_in(vault)).unwrap();

            let created = peer.response(id).await.unwrap();
            assert_eq!(created["outcome"], "created");
            let credential = &created["credential"];
            let stored = env.passkeys();
            assert_eq!(stored.len(), 1);
            let (item, passkey, count) = &stored[0];
            assert_eq!(passkey.credential_id, decode(&credential["credentialId"]));
            assert_eq!(passkey.credential_id.len(), 16);
            assert_eq!(passkey.rp_id.as_str(), "example.com");
            assert_eq!(passkey.user_name.as_str(), "alex@example.com");
            assert_eq!(passkey.user_display_name.as_str(), "Alex Example");
            assert_eq!(passkey.user_handle, vec![1, 2, 3, 4]);
            assert_eq!(
                (passkey.algorithm, passkey.discoverable, *count),
                (-7, true, 0)
            );
            assert_eq!(credential["vaultId"], vault.to_string());
            assert_eq!(credential["itemId"], item.to_string());
            assert_eq!(credential["publicKeyAlgorithm"], -7);
            {
                let slot = env.backend().slot().unwrap();
                let stored = slot.unlocked().unwrap().item(vault, *item).unwrap();
                assert_eq!(stored.title(), Some("example.com"));
            }

            // authenticatorData: rpIdHash, UP|UV|BE|BS|AT, count 0, the credential.
            let auth = decode(&credential["authenticatorData"]);
            assert_eq!(auth[..32], rp_hash("example.com")[..]);
            assert_eq!(auth[32], 0x5D);
            assert_eq!(auth[33..37], [0, 0, 0, 0]);
            assert_eq!(auth[53..55], [0, 16]);
            assert_eq!(auth[55..71], passkey.credential_id[..]);
            assert_eq!(auth.len(), 148);
            let spki = decode(&credential["publicKeySpki"]);
            assert_eq!(spki.len(), 91);
            // clientDataJSON is built here from the browser's origin.
            let data = client_data(&credential["clientDataJson"]);
            assert_eq!(
                data,
                json!({"type": "webauthn.create", "challenge": b64url(&CHALLENGE),
                       "origin": SITE, "crossOrigin": false})
            );
            assert_eq!(
                decode(&credential["attestationObject"]).len(),
                178,
                "format none"
            );
            assert_eq!(
                env.events_of("browserPasskeyClosed")[0]["reason"],
                "answered"
            );

            // The new key signs a sign-in that the returned public key verifies.
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "preferred", 2),
            )
            .await;
            assert_eq!(
                last_request(&env)["candidates"].as_array().unwrap().len(),
                1
            );
            pass_guard(&env);
            respond(&env, 2, assert_with(*item, vault)).unwrap();
            let asserted = peer.response(id).await.unwrap();
            assert_eq!(asserted["outcome"], "asserted");
            assert!(verifies(&spki, &asserted["assertion"]));
        });
    }

    #[test]
    fn get_signs_for_allow_lists_and_discoverable_requests() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let found = env.seed_passkey("example.com", "alex", true, 0);
            let hidden = env.seed_passkey("example.com", "work", false, 0);
            env.seed_passkey("other.example", "alex", true, 0);
            let mut peer = paired_peer(&env).await;

            // The rp ID defaults to the host, which holds no passkey.
            assert_eq!(
                peer.call("passkeyGet", get_body(SITE, None, &[], "discouraged", 1))
                    .await
                    .unwrap_err(),
                "passkeyNotFound"
            );
            assert!(env.events_of("browserPasskeyRequested").is_empty());

            // Without a list only discoverable passkeys of this site are offered.
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "discouraged", 1),
            )
            .await;
            let request = last_request(&env);
            assert_eq!(request["kind"], "get");
            let candidates = request["candidates"].as_array().unwrap();
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0]["itemId"], found.item.to_string());
            assert_eq!(candidates[0]["vaultName"], "Personal");
            assert_eq!(candidates[0]["userName"], "alex");
            assert_eq!(request["needsPassword"], false);
            // A passkey that was not offered cannot be used.
            pass_guard(&env);
            assert_eq!(
                respond(&env, 1, assert_with(hidden.item, vault)),
                Err(ErrorCode::ItemNotFound)
            );
            assert_eq!(
                respond(&env, 1, create_in(vault)),
                Err(ErrorCode::InvalidInput)
            );
            respond(&env, 1, assert_with(found.item, vault)).unwrap();
            let asserted = peer.response(id).await.unwrap();
            let assertion = &asserted["assertion"];
            assert!(verifies(&found.spki, assertion));
            assert_eq!(decode(&assertion["credentialId"]), found.credential_id);
            assert_eq!(decode(&assertion["userHandle"]), found.user_handle);
            let auth = decode(&assertion["authenticatorData"]);
            assert_eq!(auth.len(), 37);
            assert_eq!(auth[..32], rp_hash("example.com")[..]);
            assert_eq!(auth[32], 0x19, "UP|BE|BS, no user verification");
            assert_eq!(
                client_data(&assertion["clientDataJson"]),
                json!({"type": "webauthn.get", "challenge": b64url(&CHALLENGE),
                       "origin": SITE, "crossOrigin": false})
            );

            // An allow list names the non-discoverable one.
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(
                    SITE,
                    Some("example.com"),
                    &[b"unknown", &hidden.credential_id],
                    "discouraged",
                    2,
                ),
            )
            .await;
            let request = last_request(&env);
            let candidates = request["candidates"].as_array().unwrap();
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0]["itemId"], hidden.item.to_string());
            pass_guard(&env);
            respond(&env, 2, assert_with(hidden.item, vault)).unwrap();
            let asserted = peer.response(id).await.unwrap();
            assert!(verifies(&hidden.spki, &asserted["assertion"]));
            assert!(!verifies(&found.spki, &asserted["assertion"]));
        });
    }

    #[test]
    fn requests_that_cannot_be_answered_open_no_dialog() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            env.seed_passkey("example.com", "alex", true, 0);
            let trashed = env.seed_passkey("example.net", "alex", true, 0);
            env.backend()
                .slot()
                .unwrap()
                .unlocked_mut()
                .unwrap()
                .trash_item(vault, trashed.item)
                .unwrap();
            let mut peer = paired_peer(&env).await;
            env.backend().activity.backdate(Duration::from_hours(1));

            let get = |origin: &str, rp: Option<&str>| get_body(origin, rp, &[], "preferred", 1);
            let create = |origin: &str, rp: Option<&str>| create_body(origin, rp, "preferred", 1);
            // No passkey for the site, or only one in the trash.
            for origin in ["https://example.org", "https://example.net"] {
                assert_eq!(
                    ask(&env, &mut peer, "passkeyGet", get(origin, None))
                        .await
                        .unwrap_err(),
                    "passkeyNotFound",
                    "{origin}"
                );
            }
            // A passkey of a site is not offered to its look-alike.
            assert_eq!(
                ask(
                    &env,
                    &mut peer,
                    "passkeyGet",
                    get("https://example.com.evil.test", Some("example.com"))
                )
                .await
                .unwrap_err(),
                "rpIdMismatch"
            );
            // A page cannot claim another site's rp ID: only the page's own
            // origin is believed.
            assert_eq!(
                ask(
                    &env,
                    &mut peer,
                    "passkeyGet",
                    get("https://evil.test", Some("bank.test"))
                )
                .await
                .unwrap_err(),
                "rpIdMismatch"
            );
            assert_eq!(
                ask(
                    &env,
                    &mut peer,
                    "passkeyCreate",
                    create("https://evil.test", Some("bank.test"))
                )
                .await
                .unwrap_err(),
                "rpIdMismatch"
            );
            for rp in [
                "com",
                "Example.com",
                "example.com.",
                "sub.login.example.com",
                "",
            ] {
                assert_eq!(
                    ask(&env, &mut peer, "passkeyGet", get(SITE, Some(rp)))
                        .await
                        .unwrap_err(),
                    "rpIdMismatch",
                    "{rp:?}"
                );
            }
            assert_eq!(
                ask(
                    &env,
                    &mut peer,
                    "passkeyGet",
                    get("http://example.com", None)
                )
                .await
                .unwrap_err(),
                "insecureOrigin"
            );
            assert_eq!(
                ask(
                    &env,
                    &mut peer,
                    "passkeyCreate",
                    create("http://example.com", None)
                )
                .await
                .unwrap_err(),
                "insecureOrigin"
            );
            for origin in ["https://127.0.0.1", "https://localhost"] {
                assert_eq!(
                    ask(&env, &mut peer, "passkeyGet", get(origin, None))
                        .await
                        .unwrap_err(),
                    "rpIdMismatch",
                    "{origin}"
                );
            }
            // A frame that is not the top document.
            let mut framed = create(SITE, Some("example.com"));
            framed["topOrigin"] = json!("https://example.com");
            assert_eq!(
                ask(&env, &mut peer, "passkeyCreate", framed)
                    .await
                    .unwrap_err(),
                "crossOriginFrame"
            );
            let mut framed = get(SITE, Some("example.com"));
            framed["topOrigin"] = json!("https://other.example");
            assert_eq!(
                ask(&env, &mut peer, "passkeyGet", framed)
                    .await
                    .unwrap_err(),
                "crossOriginFrame"
            );
            // Only ES256 is made.
            let mut other = create(SITE, Some("example.com"));
            other["algorithms"] = json!([-8, -257]);
            assert_eq!(
                ask(&env, &mut peer, "passkeyCreate", other)
                    .await
                    .unwrap_err(),
                "passkeyUnsupported"
            );
            // Malformed inputs.
            for (key, value) in [
                ("requestId", json!("abc")),
                ("requestId", json!("A".repeat(32))),
                ("challenge", json!("AAEC")),
                ("challenge", json!("not base64!")),
                ("userId", json!("")),
                ("userId", json!(b64url(&[0; 65]))),
                ("userName", json!("x".repeat(257))),
                ("excludeCredentials", json!([""])),
                ("excludeCredentials", json!(vec!["AQID"; 65])),
                ("algorithms", json!(vec![-7; 17])),
            ] {
                let mut bad = create(SITE, Some("example.com"));
                bad[key] = value;
                assert_eq!(
                    ask(&env, &mut peer, "passkeyCreate", bad)
                        .await
                        .unwrap_err(),
                    "invalidRequest",
                    "{key}"
                );
            }
            let mut bad = get(SITE, Some("example.com"));
            bad["allowCredentials"] = json!([b64url(&[0; 1024])]);
            assert_eq!(
                ask(&env, &mut peer, "passkeyGet", bad).await.unwrap_err(),
                "invalidRequest"
            );
            let mut extra = get(SITE, Some("example.com"));
            extra["extra"] = json!(1);
            assert_eq!(
                ask(&env, &mut peer, "passkeyGet", extra).await.unwrap_err(),
                "invalidRequest"
            );
            let mut missing = get(SITE, Some("example.com"));
            missing.as_object_mut().unwrap().remove("rpId");
            assert_eq!(
                ask(&env, &mut peer, "passkeyGet", missing)
                    .await
                    .unwrap_err(),
                "invalidRequest"
            );

            assert!(
                env.events_of("browserPasskeyRequested").is_empty(),
                "none of these shows a dialog"
            );
            assert!(!env.passkey_open());
            assert!(
                env.idle() >= Duration::from_hours(1),
                "an unanswered request is not user activity"
            );
            assert_eq!(env.passkeys().len(), 1, "nothing was written");
        });
    }

    #[test]
    fn an_excluded_create_only_informs_and_answers_passkey_excluded() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let existing = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let mut body = create_body(SITE, Some("example.com"), "required", 1);
            body["excludeCredentials"] = json!([b64url(b"other"), b64url(&existing.credential_id)]);
            let id = start(&env, &mut peer, "passkeyCreate", body).await;
            let request = last_request(&env);
            assert_eq!(request["excluded"], true);
            assert_eq!(request["needsPassword"], false, "nothing is signed");
            // Nothing can be created from this dialog.
            pass_guard(&env);
            assert_eq!(
                respond(&env, 1, create_in(vault)),
                Err(ErrorCode::InvalidInput)
            );
            assert_eq!(
                respond(&env, 1, Decision::UseAnotherDevice),
                Err(ErrorCode::InvalidInput)
            );
            respond(&env, 1, Decision::Acknowledge).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "passkeyExcluded");
            assert_eq!(env.passkeys().len(), 1);

            // Acknowledging is for an exclusion only.
            let id = start(
                &env,
                &mut peer,
                "passkeyCreate",
                create_body(SITE, Some("example.com"), "preferred", 2),
            )
            .await;
            assert_eq!(
                respond(&env, 2, Decision::Acknowledge),
                Err(ErrorCode::InvalidInput)
            );
            respond(&env, 2, Decision::Cancel).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
        });
    }

    #[test]
    fn use_another_device_and_cancel_sign_and_write_nothing() {
        run(async {
            let env = Env::unlocked();
            let found = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let id = start(
                &env,
                &mut peer,
                "passkeyCreate",
                create_body(SITE, Some("example.com"), "preferred", 1),
            )
            .await;
            // These need no Allow guard.
            respond(&env, 1, Decision::UseAnotherDevice).unwrap();
            assert_eq!(
                peer.response(id).await.unwrap(),
                json!({"outcome": "useAnotherDevice", "credential": null})
            );
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "preferred", 2),
            )
            .await;
            respond(&env, 2, Decision::UseAnotherDevice).unwrap();
            assert_eq!(
                peer.response(id).await.unwrap(),
                json!({"outcome": "useAnotherDevice", "assertion": null})
            );
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "preferred", 3),
            )
            .await;
            respond(&env, 3, Decision::Cancel).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
            assert_eq!(env.passkeys().len(), 1);
            assert_eq!(env.passkeys()[0].0, found.item);
            assert_eq!(
                code_of(
                    env.backend()
                        .browser_passkey_respond(&answer(3, Decision::Cancel, None))
                ),
                ErrorCode::BrowserRequestExpired
            );
            assert_eq!(env.events_of("browserPasskeyClosed").len(), 3);
        });
    }

    #[test]
    fn user_verification_follows_the_five_minute_window_and_the_password() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let get = |uv: &str, n: u8| get_body(SITE, Some("example.com"), &[], uv, n);
            let flags = |asserted: &Value| decode(&asserted["assertion"]["authenticatorData"])[32];

            // Within the window of the unlock: UV with a dialog click.
            let id = start(&env, &mut peer, "passkeyGet", get("required", 1)).await;
            assert_eq!(last_request(&env)["needsPassword"], false);
            pass_guard(&env);
            respond(&env, 1, assert_with(key.item, vault)).unwrap();
            assert_eq!(flags(&peer.response(id).await.unwrap()), 0x1D);

            // 301 s later the dialog asks for the master password.
            env.clock.advance(Duration::from_secs(301));
            let id = start(&env, &mut peer, "passkeyGet", get("required", 2)).await;
            assert_eq!(last_request(&env)["needsPassword"], true);
            pass_guard(&env);
            let decision = || assert_with(key.item, vault);
            assert_eq!(respond(&env, 2, decision()), Err(ErrorCode::ReauthRequired));
            let wrong = env.backend().browser_passkey_respond(&answer(
                2,
                decision(),
                Some("not the password"),
            ));
            assert_eq!(code_of(wrong), ErrorCode::WrongCredentials);
            assert!(env.passkey_open(), "the dialog stays open");
            env.backend()
                .browser_passkey_respond(&answer(2, decision(), Some(PASSWORD)))
                .unwrap();
            assert_eq!(flags(&peer.response(id).await.unwrap()), 0x1D);

            // The right password verified the user for five minutes.
            let id = start(&env, &mut peer, "passkeyGet", get("preferred", 3)).await;
            assert_eq!(last_request(&env)["needsPassword"], false);
            pass_guard(&env);
            respond(&env, 3, decision()).unwrap();
            assert_eq!(flags(&peer.response(id).await.unwrap()), 0x1D);

            // `discouraged` never asks and never sets the flag.
            env.clock.advance(Duration::from_secs(301));
            let id = start(&env, &mut peer, "passkeyGet", get("discouraged", 4)).await;
            assert_eq!(last_request(&env)["needsPassword"], false);
            pass_guard(&env);
            respond(&env, 4, decision()).unwrap();
            assert_eq!(flags(&peer.response(id).await.unwrap()), 0x19);

            // A create is the same: 0x59 without, 0x5D with the verification.
            let id = start(
                &env,
                &mut peer,
                "passkeyCreate",
                create_body(SITE, Some("example.com"), "discouraged", 5),
            )
            .await;
            pass_guard(&env);
            respond(&env, 5, create_in(vault)).unwrap();
            let created = peer.response(id).await.unwrap();
            assert_eq!(
                decode(&created["credential"]["authenticatorData"])[32],
                0x59
            );
        });
    }

    #[test]
    fn five_wrong_passwords_in_the_dialog_lock_the_vault() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            env.clock.advance(Duration::from_secs(301));
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "required", 1),
            )
            .await;
            pass_guard(&env);
            for _ in 0..4 {
                let wrong = env.backend().browser_passkey_respond(&answer(
                    1,
                    assert_with(key.item, vault),
                    Some("nope"),
                ));
                assert_eq!(code_of(wrong), ErrorCode::WrongCredentials);
            }
            assert!(env.backend().is_unlocked());
            let wrong = env.backend().browser_passkey_respond(&answer(
                1,
                assert_with(key.item, vault),
                Some("nope"),
            ));
            assert_eq!(code_of(wrong), ErrorCode::WrongCredentials);
            assert!(!env.backend().is_unlocked(), "the fifth failure locks");
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
            assert_eq!(env.events_of("browserPasskeyClosed")[0]["reason"], "locked");
        });
    }

    #[test]
    fn an_imported_counter_is_raised_and_stored_before_the_signature_is_released() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 41);
            let mut peer = paired_peer(&env).await;
            for (n, expected) in [(1u8, 42u32), (2, 43)] {
                let id = start(
                    &env,
                    &mut peer,
                    "passkeyGet",
                    get_body(SITE, Some("example.com"), &[], "discouraged", n),
                )
                .await;
                pass_guard(&env);
                respond(&env, n, assert_with(key.item, vault)).unwrap();
                let asserted = peer.response(id).await.unwrap();
                let auth = decode(&asserted["assertion"]["authenticatorData"]);
                assert_eq!(auth[32], 0x19);
                assert_eq!(auth[33..37], expected.to_be_bytes());
                assert_eq!(env.passkeys()[0].2, u64::from(expected), "stored");
                assert!(verifies(&key.spki, &asserted["assertion"]));
            }
            // The counter of a passkey Mosaic made stays 0 and is never written.
            let fresh = env.seed_passkey("example.org", "alex", true, 0);
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body("https://example.org", None, &[], "discouraged", 3),
            )
            .await;
            pass_guard(&env);
            respond(&env, 3, assert_with(fresh.item, vault)).unwrap();
            let asserted = peer.response(id).await.unwrap();
            assert_eq!(
                decode(&asserted["assertion"]["authenticatorData"])[33..37],
                [0; 4]
            );
            let stored = env.passkeys();
            assert_eq!(stored.iter().find(|p| p.0 == fresh.item).unwrap().2, 0);
        });
    }

    #[test]
    fn a_passkey_that_changed_since_the_dialog_opened_is_not_signed() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "discouraged", 1),
            )
            .await;
            env.backend()
                .slot()
                .unwrap()
                .unlocked_mut()
                .unwrap()
                .trash_item(vault, key.item)
                .unwrap();
            pass_guard(&env);
            assert_eq!(
                respond(&env, 1, assert_with(key.item, vault)),
                Err(ErrorCode::ItemNotFound)
            );
            respond(&env, 1, Decision::Cancel).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
        });
    }

    #[test]
    fn names_are_sanitized_and_a_closed_vault_refuses_a_create() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let mut peer = paired_peer(&env).await;
            let mut body = create_body(SITE, Some("example.com"), "discouraged", 1);
            body["userName"] = json!("al\u{202e}ex\u{7}@example.com");
            body["userDisplayName"] = json!("é".repeat(100));
            let id = start(&env, &mut peer, "passkeyCreate", body).await;
            let request = last_request(&env);
            assert_eq!(request["userName"], "alex@example.com");
            assert_eq!(request["userDisplayName"].as_str().unwrap().len(), 64);
            pass_guard(&env);
            respond(&env, 1, create_in(vault)).unwrap();
            peer.response(id).await.unwrap();
            let stored = env.passkeys();
            assert_eq!(stored[0].1.user_name.as_str(), "alex@example.com");
            assert_eq!(stored[0].1.user_display_name.as_str().len(), 64);
        });
    }

    #[test]
    fn a_hardware_protected_vault_never_takes_a_passkey() {
        use crate::browser::CreatePlan;
        use crate::quick_unlock::QuickUnlockService;
        use crate::quick_unlock::fakes::FakeHardwareKey;
        let _kdf = crate::test_support::kdf_permit();
        let key = FakeHardwareKey::new(1);
        let backend = Backend::new(
            Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
            Box::new(MemoryKeystore::default()),
            || Ok(Argon2Profile::Baseline),
        )
        .with_quick_unlock(QuickUnlockService::new(None, Box::new(Arc::clone(&key))));
        backend
            .create_account(CreateAccountRequest {
                email: "alex@example.com".into(),
                password: SecretIn::test(PASSWORD),
                default_vault_name: "Personal".into(),
            })
            .unwrap();
        let summary = backend
            .create_hardware_vault(
                &mut serde_json::from_value(json!({ "name": "Codes", "pin": null })).unwrap(),
            )
            .unwrap();
        let plan = CreatePlan {
            request_id: rid(1),
            pairing_label: "Chrome".into(),
            origin: SITE.into(),
            rp_id: "example.com".into(),
            challenge: CHALLENGE.to_vec(),
            user_handle: vec![1, 2, 3, 4],
            user_name: "alex".into(),
            user_display_name: "Alex".into(),
            algorithms: vec![-7],
            user_verification: mosaic_native_messaging::bodies::UserVerification::Discouraged,
            excluded: false,
        };
        // Open, and still not a place for passkeys.
        let refused = backend.browser_apply_passkey_create(&plan, summary.vault_id.get(), false);
        assert_eq!(code_of(refused), ErrorCode::VaultUnavailable);
        backend
            .close_hardware_vault(
                &serde_json::from_value(json!({ "vaultId": summary.vault_id.0.to_string() }))
                    .unwrap(),
            )
            .unwrap();
        let refused = backend.browser_apply_passkey_create(&plan, summary.vault_id.get(), false);
        assert_eq!(code_of(refused), ErrorCode::VaultUnavailable);
        let unknown =
            backend.browser_apply_passkey_create(&plan, VaultId::new_v7().unwrap(), false);
        assert_eq!(code_of(unknown), ErrorCode::VaultNotFound);
    }

    #[test]
    fn one_passkey_prompt_at_a_time_and_only_its_own_pairing_can_cancel_it() {
        run(async {
            let env = Env::unlocked();
            let (id_a, key_a) = pair(&env, "Chrome").await;
            let (id_b, key_b) = pair(&env, "Firefox").await;
            let mut first = connect(&env, &id_a, &key_a).await.unwrap();
            let mut second = connect(&env, &id_b, &key_b).await.unwrap();
            env.seed_passkey("example.com", "alex", true, 0);
            let get = |n: u8| get_body(SITE, Some("example.com"), &[], "discouraged", n);

            let id = start(&env, &mut first, "passkeyGet", get(1)).await;
            // A second request, of either kind and of either browser, is busy.
            assert_eq!(first.call("passkeyGet", get(2)).await.unwrap_err(), "busy");
            assert_eq!(second.call("passkeyGet", get(3)).await.unwrap_err(), "busy");
            assert_eq!(
                second
                    .call(
                        "passkeyCreate",
                        create_body(SITE, Some("example.com"), "preferred", 4)
                    )
                    .await
                    .unwrap_err(),
                "busy"
            );
            // Another browser cannot close it, whatever ID it names.
            assert_eq!(
                second
                    .call("passkeyCancel", json!({"requestId": rid(1)}))
                    .await
                    .unwrap(),
                json!({})
            );
            assert!(env.passkey_open());
            assert!(env.events_of("browserPasskeyClosed").is_empty());
            // The page of the first browser gave up.
            let cancel = first
                .request("passkeyCancel", json!({"requestId": rid(1)}))
                .await;
            let mut answers = first.responses(&[id, cancel]).await;
            assert_eq!(answers.remove(&cancel).unwrap().unwrap(), json!({}));
            assert_eq!(answers.remove(&id).unwrap().unwrap_err(), "cancelled");
            assert!(!env.passkey_open());
            assert_eq!(
                env.events_of("browserPasskeyClosed")[0]["reason"],
                "abortedByPage"
            );
            // Cancelling what is not open is quiet.
            assert_eq!(
                first
                    .call("passkeyCancel", json!({"requestId": rid(1)}))
                    .await
                    .unwrap(),
                json!({})
            );
            let mut extra = json!({"requestId": rid(1)});
            extra["extra"] = json!(1);
            assert_eq!(
                first.call("passkeyCancel", extra).await.unwrap_err(),
                "invalidRequest"
            );
        });
    }

    #[test]
    fn a_passkey_prompt_expires_and_the_lock_cancels_it() {
        run(async {
            let env = Env::unlocked_with(Limits {
                passkey_timeout: Duration::from_millis(1500),
                ..Limits::default()
            });
            env.seed_passkey("example.com", "alex", true, 0);
            let get = |n: u8| get_body(SITE, Some("example.com"), &[], "discouraged", n);
            let mut peer = paired_peer(&env).await;

            // Expiry: the dialog says when, and the site is told `cancelled`.
            let id = start(&env, &mut peer, "passkeyGet", get(1)).await;
            let shown = last_request(&env);
            let window =
                shown["expiresAt"].as_i64().unwrap() - shown["requestedAt"].as_i64().unwrap();
            assert!((1500..1600).contains(&window), "{window}");
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
            assert_eq!(
                env.events_of("browserPasskeyClosed")[0]["reason"],
                "expired"
            );
            assert_eq!(
                code_of(
                    env.backend()
                        .browser_passkey_respond(&answer(1, Decision::Cancel, None))
                ),
                ErrorCode::BrowserRequestExpired
            );

            // The vault locks: the site is told `cancelled`, nothing is signed.
            let id = start(&env, &mut peer, "passkeyGet", get(2)).await;
            env.backend().lock(LockReason::Manual).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
            assert_eq!(env.events_of("browserPasskeyClosed")[1]["reason"], "locked");
            assert_eq!(peer.event().await.unwrap()["state"], "locked");
            assert!(!env.passkey_open());
            // While locked: no new request, but an abort is still taken.
            assert_eq!(peer.call("passkeyGet", get(3)).await.unwrap_err(), "locked");
            assert_eq!(
                peer.call("passkeyCreate", create_body(SITE, None, "preferred", 4))
                    .await
                    .unwrap_err(),
                "locked"
            );
            assert_eq!(
                peer.call("passkeyCancel", json!({"requestId": rid(3)}))
                    .await
                    .unwrap(),
                json!({})
            );
        });
    }

    #[test]
    fn a_passkey_prompt_ends_when_its_browser_disconnects() {
        run(async {
            let env = Env::unlocked();
            env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "discouraged", 1),
            )
            .await;
            drop(peer);
            wait_for_event(&env, "browserPasskeyClosed").await;
            assert_eq!(
                env.events_of("browserPasskeyClosed")[0]["reason"],
                "disconnected"
            );
            assert!(!env.passkey_open());
            assert_eq!(
                code_of(
                    env.backend()
                        .browser_passkey_respond(&answer(1, Decision::Cancel, None))
                ),
                ErrorCode::BrowserRequestExpired
            );
        });
    }

    #[test]
    fn a_passkey_prompt_ends_when_its_pairing_is_removed() {
        run(async {
            let env = Env::unlocked();
            env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let status = connected_status(&env).await;
            start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "discouraged", 1),
            )
            .await;
            env.backend()
                .browser_remove_pairing(&BrowserRemovePairingRequest {
                    pairing_id: status.pairings[0].pairing_id.clone(),
                })
                .unwrap();
            wait_for_event(&env, "browserPasskeyClosed").await;
            assert_eq!(
                env.events_of("browserPasskeyClosed")[0]["reason"],
                "disconnected"
            );
            assert!(!env.passkey_open());
        });
    }

    #[test]
    fn passkey_requests_are_rate_limited_per_pairing() {
        run(async {
            let env = Env::unlocked();
            let mut peer = paired_peer(&env).await;
            // Every request comes from another site, so only the pairing's windows count.
            let creates = |n: u8| {
                create_body(
                    &format!("https://evil{n}.test"),
                    Some("bank.test"),
                    "preferred",
                    n,
                )
            };
            for n in 0..3 {
                assert_eq!(
                    peer.call("passkeyCreate", creates(n)).await.unwrap_err(),
                    "rpIdMismatch"
                );
            }
            assert_eq!(
                peer.call("passkeyCreate", creates(9)).await.unwrap_err(),
                "rateLimited"
            );
            for n in 0..10 {
                assert_eq!(
                    peer.call(
                        "passkeyGet",
                        get_body(
                            &format!("https://s{n}.example.com"),
                            Some("example.com"),
                            &[],
                            "preferred",
                            n
                        )
                    )
                    .await
                    .unwrap_err(),
                    "passkeyNotFound"
                );
            }
            assert_eq!(
                peer.call(
                    "passkeyGet",
                    get_body(
                        "https://s9.example.com",
                        Some("example.com"),
                        &[],
                        "preferred",
                        9
                    )
                )
                .await
                .unwrap_err(),
                "rateLimited"
            );
            for _ in 0..20 {
                peer.call("passkeyCancel", json!({"requestId": rid(1)}))
                    .await
                    .unwrap();
            }
            assert_eq!(
                peer.call("passkeyCancel", json!({"requestId": rid(1)}))
                    .await
                    .unwrap_err(),
                "rateLimited"
            );
            // The windows slide.
            env.clock.advance(Duration::from_secs(61));
            assert_eq!(
                peer.call("passkeyCreate", creates(9)).await.unwrap_err(),
                "rpIdMismatch"
            );
        });
    }

    #[test]
    fn one_site_cannot_spend_the_passkey_windows_of_another() {
        run(async {
            let env = Env::unlocked();
            let mut peer = paired_peer(&env).await;
            let evil = |n: u8| create_body("https://evil.test", Some("bank.test"), "preferred", n);
            let other =
                |n: u8| create_body("https://other.test", Some("bank.test"), "preferred", n);
            // A site gets 2 registrations a minute; the third is refused...
            for n in 0..2 {
                assert_eq!(
                    peer.call("passkeyCreate", evil(n)).await.unwrap_err(),
                    "rpIdMismatch"
                );
            }
            for n in 2..6 {
                assert_eq!(
                    peer.call("passkeyCreate", evil(n)).await.unwrap_err(),
                    "rateLimited"
                );
            }
            // ...without costing the pairing anything: its third window is still free for
            // another site, and then the pairing's own limit of 3 applies.
            assert_eq!(
                peer.call("passkeyCreate", other(7)).await.unwrap_err(),
                "rpIdMismatch"
            );
            assert_eq!(
                peer.call("passkeyCreate", other(8)).await.unwrap_err(),
                "rateLimited"
            );
            // The same for sign-ins: 6 a minute per site, 10 per pairing. (An origin that is not
            // in its canonical form is refused `invalidRequest`, so it cannot be a second key.)
            let get =
                |origin: &str, n: u8| get_body(origin, Some("example.com"), &[], "preferred", n);
            for n in 0..6 {
                assert_eq!(
                    peer.call("passkeyGet", get("https://a.example.com", n))
                        .await
                        .unwrap_err(),
                    "passkeyNotFound"
                );
            }
            assert_eq!(
                peer.call("passkeyGet", get("https://a.example.com", 6))
                    .await
                    .unwrap_err(),
                "rateLimited"
            );
            for n in 7..11 {
                assert_eq!(
                    peer.call("passkeyGet", get("https://b.example.com", n))
                        .await
                        .unwrap_err(),
                    "passkeyNotFound"
                );
            }
            assert_eq!(
                peer.call("passkeyGet", get("https://c.example.com", 12))
                    .await
                    .unwrap_err(),
                "rateLimited"
            );
            // The windows slide.
            env.clock.advance(Duration::from_secs(61));
            assert_eq!(
                peer.call("passkeyCreate", evil(9)).await.unwrap_err(),
                "rpIdMismatch"
            );
        });
    }

    #[test]
    fn the_item_detail_of_a_passkey_never_holds_the_private_key() {
        let env = Env::unlocked();
        let vault = env.vault();
        let key = env.seed_passkey("example.com", "alex", true, 7);
        let detail = env
            .backend()
            .get_item(crate::contract::items::ItemRequest {
                vault_id: vault.into(),
                item_id: key.item.into(),
            })
            .unwrap();
        let passkey = detail.passkey.as_ref().unwrap();
        assert_eq!(passkey.rp_id, "example.com");
        assert_eq!(passkey.user_name, "alex");
        assert_eq!(passkey.credential_id, b64url(&key.credential_id));
        assert_eq!(passkey.sign_count, 7);
        assert_eq!(detail.summary.subtitle.as_deref(), Some("alex"));
        assert_eq!(detail.summary.host.as_deref(), Some("example.com"));
        assert!(detail.secrets.is_empty(), "the key is not a secret field");
        let json = serde_json::to_string(&detail).unwrap();
        assert!(!json.to_lowercase().contains("privatekey") && !json.contains("private_key"));
    }

    #[test]
    fn redteam_no_browser_op_returns_a_private_key() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let stored = env.seed_passkey("example.com", "alex", true, 5);
            let der = env.passkeys()[0].1.private_key.expose_secret().to_vec();
            let needles = crate::redteam_tests::key_needles(&der);
            env.login("Example", "alex", "pw-1", "https://example.com");
            let mut peer = paired_peer(&env).await;
            let mut seen: Vec<String> = Vec::new();
            let mut keep = |value: Result<Value, String>| {
                seen.push(match value {
                    Ok(v) => v.to_string(),
                    Err(e) => e,
                });
            };

            // A creation, a sign-in and every other op that could list or dump.
            let id = start(
                &env,
                &mut peer,
                "passkeyCreate",
                create_body(SITE, Some("example.com"), "discouraged", 1),
            )
            .await;
            pass_guard(&env);
            respond(&env, 1, create_in(vault)).unwrap();
            keep(peer.response(id).await);
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(
                    SITE,
                    Some("example.com"),
                    &[stored.credential_id.as_slice()],
                    "discouraged",
                    2,
                ),
            )
            .await;
            pass_guard(&env);
            respond(&env, 2, assert_with(stored.item, vault)).unwrap();
            keep(peer.response(id).await);
            env.clock.advance(Duration::from_secs(61));
            let b = "https://example.com";
            keep(peer.call("status", json!({})).await);
            keep(
                peer.call("listForOrigin", json!({"origin": b, "topOrigin": b}))
                    .await,
            );
            keep(
                peer.call("search", json!({"query": "alex", "origin": b}))
                    .await,
            );
            keep(
                peer.call("search", json!({"query": "example", "origin": null}))
                    .await,
            );
            keep(
                peer.call(
                    "checkSave",
                    json!({"origin": b, "topOrigin": b, "username": "alex", "password": "x"}),
                )
                .await,
            );

            // The ops that do not exist.
            for op in [
                "listPasskeys",
                "getPasskey",
                "exportPasskey",
                "dumpVault",
                "revealSecret",
            ] {
                keep(peer.call(op, json!({})).await);
            }
            keep(
                peer.call(
                    "passkeyGet",
                    json!({"requestId": rid(3), "privateKey": true}),
                )
                .await,
            );

            let new_der = env
                .passkeys()
                .into_iter()
                .find(|p| p.0 != stored.item)
                .unwrap()
                .1
                .private_key
                .expose_secret()
                .to_vec();
            let all_needles: Vec<String> = needles
                .into_iter()
                .chain(crate::redteam_tests::key_needles(&new_der))
                .collect();
            let events: Vec<String> = env.events.lock().unwrap().iter().cloned().collect();
            for text in seen.iter().chain(events.iter()) {
                assert!(
                    !text.contains("privateKey") && !text.contains("private_key"),
                    "{text}"
                );
                for needle in &all_needles {
                    assert!(!text.contains(needle.as_str()), "{text}");
                }
            }
            assert!(seen.len() >= 11);
        });
    }

    #[test]
    fn an_answer_belongs_to_the_dialog_handle_not_to_a_reused_extension_id() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let get = || get_body(SITE, Some("example.com"), &[], "discouraged", 1);

            // A opens and is cancelled by the page.
            let first = start(&env, &mut peer, "passkeyGet", get()).await;
            let handle_a = handle_of(1);
            let cancel = peer
                .request("passkeyCancel", json!({"requestId": rid(1)}))
                .await;
            let mut answers = peer.responses(&[first, cancel]).await;
            assert_eq!(answers.remove(&first).unwrap().unwrap_err(), "cancelled");

            // B reuses the extension's ID.
            let second = start(&env, &mut peer, "passkeyGet", get()).await;
            let handle_b = handle_of(1);
            assert_ne!(handle_a, handle_b, "a handle is never reused");
            assert_ne!(handle_b, rid(1), "and is not the extension's ID");
            pass_guard(&env);

            let stale = |request_id: &str| {
                env.backend()
                    .browser_passkey_respond(&BrowserPasskeyRespondRequest {
                        request_id: request_id.to_owned(),
                        decision: assert_with(key.item, vault),
                        password: None,
                    })
            };
            // A's handle, and the extension's own ID, do not apply to B.
            assert_eq!(code_of(stale(&handle_a)), ErrorCode::BrowserRequestExpired);
            assert_eq!(code_of(stale(&rid(1))), ErrorCode::BrowserRequestExpired);
            assert!(env.passkey_open(), "B is still waiting");
            assert_eq!(env.passkeys()[0].2, 0, "nothing was signed");
            respond(&env, 1, assert_with(key.item, vault)).unwrap();
            assert!(peer.response(second).await.is_ok());
        });
    }

    #[test]
    fn an_acknowledge_waits_out_the_allow_guard_but_a_cancel_does_not() {
        run(async {
            let env = Env::unlocked();
            let existing = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let mut body = create_body(SITE, Some("example.com"), "required", 1);
            body["excludeCredentials"] = json!([b64url(&existing.credential_id)]);
            let id = start(&env, &mut peer, "passkeyCreate", body.clone()).await;
            assert_eq!(
                respond(&env, 1, Decision::Acknowledge),
                Err(ErrorCode::BrowserAllowTooSoon)
            );
            assert!(env.passkey_open(), "still open");
            pass_guard(&env);
            respond(&env, 1, Decision::Acknowledge).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "passkeyExcluded");

            // A cancel is always taken.
            let id = start(&env, &mut peer, "passkeyCreate", body).await;
            respond(&env, 1, Decision::Cancel).unwrap();
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
        });
    }

    #[test]
    fn a_failed_answer_after_the_request_expired_leaves_no_prompt_behind() {
        run(async {
            let env = Env::unlocked_with(Limits {
                passkey_timeout: Duration::from_millis(700),
                ..Limits::default()
            });
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "discouraged", 1),
            )
            .await;
            pass_guard(&env);
            let request = answer(1, assert_with(key.item, vault), None);
            // The answer waits for the vault; meanwhile the request expires,
            // and the passkey is trashed so the answer fails.
            let mut slot = env.backend().slot().unwrap();
            let backend = env.backend();
            let answered = std::thread::scope(|scope| {
                let answered = scope.spawn(move || backend.browser_passkey_respond(&request));
                std::thread::sleep(Duration::from_millis(1500));
                slot.unlocked_mut()
                    .unwrap()
                    .trash_item(vault, key.item)
                    .unwrap();
                drop(slot);
                answered.join().unwrap()
            });
            assert_eq!(code(answered.unwrap_err()), ErrorCode::ItemNotFound);
            assert_eq!(peer.response(id).await.unwrap_err(), "cancelled");
            assert!(!env.passkey_open(), "the prompt is not put back");
            let closed = env.events_of("browserPasskeyClosed");
            assert_eq!(closed.len(), 1);
            assert_eq!(closed[0]["reason"], "expired");
            assert_eq!(closed[0]["requestId"], handle_of(1));
        });
    }

    #[test]
    fn a_passkey_dialog_password_verifies_but_earns_no_reauth_grant() {
        run(async {
            let env = Env::unlocked();
            let vault = env.vault();
            let key = env.seed_passkey("example.com", "alex", true, 0);
            let mut peer = paired_peer(&env).await;
            env.clock.advance(Duration::from_secs(301));
            let id = start(
                &env,
                &mut peer,
                "passkeyGet",
                get_body(SITE, Some("example.com"), &[], "required", 1),
            )
            .await;
            pass_guard(&env);
            // Wrong ones count; a right one answers.
            let wrong = env.backend().browser_passkey_respond(&answer(
                1,
                assert_with(key.item, vault),
                Some("nope"),
            ));
            assert_eq!(code_of(wrong), ErrorCode::WrongCredentials);
            env.backend()
                .browser_passkey_respond(&answer(1, assert_with(key.item, vault), Some(PASSWORD)))
                .unwrap();
            assert!(peer.response(id).await.is_ok());
            assert!(env.backend().uv_recent(), "the user counts as verified");
            // A move out of a hardware vault (and every other change that
            // spends the grant) still asks for the password itself.
            assert_eq!(
                code_of(env.backend().take_reauth_grant().map(|_| ())),
                ErrorCode::ReauthRequired
            );
            env.backend()
                .reauthenticate(SecretIn::test(PASSWORD))
                .unwrap();
            assert!(env.backend().take_reauth_grant().is_ok());
        });
    }

    #[test]
    fn a_passkey_item_changes_only_its_title_notes_and_tags() {
        let env = Env::unlocked();
        let vault = env.vault();
        let key = env.seed_passkey("example.com", "alex", true, 0);
        let update = |draft: Value| {
            let mut full = json!({
                "text": [], "secrets": [], "urls": [], "customFields": [],
                "tags": null, "otp": null, "sshKey": null,
            });
            for (name, value) in draft.as_object().unwrap() {
                full[name] = value.clone();
            }
            env.backend()
                .update_item(
                    serde_json::from_value(json!({
                        "vaultId": vault.to_string(), "itemId": key.item.to_string(),
                        "draft": full,
                    }))
                    .unwrap(),
                )
                .map_err(code)
        };
        let invalid = Err(ErrorCode::InvalidInput);
        assert_eq!(
            update(json!({"text": [{"field": "username", "value": "mallory"}]})).map(|_| ()),
            invalid
        );
        assert_eq!(
            update(json!({"secrets": [{"field": "password", "value": "pw"}]})).map(|_| ()),
            invalid
        );
        assert_eq!(
            update(json!({"urls": [{
                "op": "add", "url": "https://evil.example",
                "matchMode": "exactOrigin", "allowHttp": false
            }]}))
            .map(|_| ()),
            invalid
        );
        assert_eq!(
            update(json!({"customFields": [
                {"op": "add", "label": "x", "kind": "text", "value": "y"}
            ]}))
            .map(|_| ()),
            invalid
        );
        assert_eq!(
            update(json!({"otp": {"op": "remove"}})).map(|_| ()),
            invalid
        );
        // Nothing of the above changed the item.
        assert_eq!(env.passkeys()[0].1.user_name.as_str(), "alex");
        let detail = update(json!({
            "text": [
                {"field": "title", "value": "Example account"},
                {"field": "notes", "value": "from the bank"},
            ],
            "tags": ["bank"],
        }))
        .unwrap();
        assert_eq!(detail.summary.title.as_deref(), Some("Example account"));
        assert_eq!(detail.summary.tags, vec!["bank".to_owned()]);
        assert_eq!(detail.passkey.unwrap().rp_id, "example.com");
        assert_eq!(env.passkeys().len(), 1);
    }

    #[test]
    fn the_names_of_a_passkey_in_the_item_views_lose_control_and_format_characters() {
        let env = Env::unlocked();
        let vault = env.vault();
        let key = env.seed_passkey("example.com", "al\u{202e}ex\u{7}\n", true, 0);
        let detail = env
            .backend()
            .get_item(crate::contract::items::ItemRequest {
                vault_id: vault.into(),
                item_id: key.item.into(),
            })
            .unwrap();
        assert_eq!(detail.passkey.as_ref().unwrap().user_name, "alex");
        assert_eq!(detail.summary.subtitle.as_deref(), Some("alex"));
        let json = serde_json::to_string(&detail).unwrap();
        assert!(!json.contains("202e") && !json.contains('\u{202e}'));
    }

    impl Peer {
        /// The responses to `ids`, in whatever order they arrive.
        async fn responses(&mut self, ids: &[u32]) -> HashMap<u32, Result<Value, String>> {
            let mut out = HashMap::new();
            while out.len() < ids.len() {
                let message = self.frame().await.expect("the connection stays open");
                if message.get("event").is_some() {
                    self.events.push(message);
                    continue;
                }
                let id = u32::try_from(message["id"].as_u64().unwrap()).unwrap();
                assert!(ids.contains(&id));
                out.insert(
                    id,
                    if message["ok"] == true {
                        Ok(message["body"].clone())
                    } else {
                        Err(message["error"].as_str().unwrap().to_owned())
                    },
                );
            }
            out
        }
    }
}
