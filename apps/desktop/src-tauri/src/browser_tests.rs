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
