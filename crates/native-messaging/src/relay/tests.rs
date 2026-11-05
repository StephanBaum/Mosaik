use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, DuplexStream, duplex};
use tokio::task::JoinHandle;

use super::*;
use crate::handshake::{read_client_message, server_handshake};
use crate::testutil::{block_on, block_on_paused};

const PAIRING_ID: [u8; 16] = [9; 16];

fn key(byte: u8) -> PairingKey {
    PairingKey::from_bytes(&mut [byte; 32])
}

#[derive(Clone)]
struct MockConnector {
    streams: mpsc::UnboundedSender<DuplexStream>,
    up: Arc<AtomicBool>,
}

impl MockConnector {
    fn open(&self) -> Result<DuplexStream, Error> {
        if !self.up.load(Ordering::SeqCst) {
            return Err(Error::DesktopNotRunning);
        }
        let (host_end, desktop_end) = duplex(1 << 20);
        self.streams
            .send(desktop_end)
            .map_err(|_| Error::DesktopNotRunning)?;
        Ok(host_end)
    }
}

impl Connect for MockConnector {
    type Stream = DuplexStream;

    fn connect(&self) -> impl Future<Output = Result<DuplexStream, Error>> + Send {
        std::future::ready(self.open())
    }
}

/// The test plays the desktop: it accepts what the host connects.
struct Desktop {
    streams: mpsc::UnboundedReceiver<DuplexStream>,
}

impl Desktop {
    async fn accept(&mut self) -> DuplexStream {
        self.streams.recv().await.unwrap()
    }

    /// Accepts a connection and serves the handshake with `server_key`.
    async fn session_with(
        &mut self,
        server_key: Option<PairingKey>,
        state: LockState,
    ) -> Result<(DuplexStream, ChannelSession), Error> {
        let mut stream = self.accept().await;
        let ClientMessage::Hello {
            pairing_id, nonce, ..
        } = read_client_message(&mut stream).await?
        else {
            return Err(Error::Malformed);
        };
        let session = server_handshake(
            &mut stream,
            &pairing_id.0,
            &nonce.0,
            server_key.as_ref(),
            state,
        )
        .await?;
        Ok((stream, session))
    }

    async fn session(&mut self) -> (DuplexStream, ChannelSession) {
        self.session_with(Some(key(7)), LockState::Unlocked)
            .await
            .unwrap()
    }
}

async fn read_request(stream: &mut DuplexStream, session: &mut ChannelSession) -> Value {
    let frame = next_frame(stream).await.unwrap();
    serde_json::from_slice(session.open(&frame).unwrap().expose_secret()).unwrap()
}

async fn reply(stream: &mut DuplexStream, session: &mut ChannelSession, message: &Value) {
    send(stream, session, message.to_string().as_bytes())
        .await
        .unwrap();
}

/// The browser's side of the host, plus the desktop.
struct Rig {
    to_host: DuplexStream,
    from_host: DuplexStream,
    desktop: Desktop,
    up: Arc<AtomicBool>,
    host: JoinHandle<Result<(), Error>>,
}

impl Rig {
    fn new() -> Self {
        let (to_host, stdin) = duplex(1 << 20);
        let (stdout, from_host) = duplex(1 << 20);
        let (streams, rx) = mpsc::unbounded_channel();
        let up = Arc::new(AtomicBool::new(true));
        let connector = MockConnector {
            streams,
            up: Arc::clone(&up),
        };
        let host = tokio::spawn(run_with(Caller::Firefox, stdin, stdout, connector));
        Self {
            to_host,
            from_host,
            desktop: Desktop { streams: rx },
            up,
            host,
        }
    }

    async fn send(&mut self, message: &Value) {
        self.send_raw(message.to_string().as_bytes()).await;
    }

    async fn send_raw(&mut self, bytes: &[u8]) {
        write_frame(&mut self.to_host, ByteOrder::Native, bytes)
            .await
            .unwrap();
    }

    async fn recv(&mut self) -> Value {
        let frame = timeout(
            Duration::from_secs(60),
            read_frame(&mut self.from_host, ByteOrder::Native),
        )
        .await
        .expect("no message from the host")
        .unwrap()
        .expect("the host closed stdout");
        serde_json::from_slice(&frame).unwrap()
    }

    /// Closes stdin and returns how the host ended.
    async fn finish(self) -> Result<(), Error> {
        drop(self.to_host);
        self.host.await.unwrap()
    }
}

fn hello(id: u32) -> Value {
    json!({"v": 1, "id": id, "op": "hello", "body": {
        "pairing": {"pairingId": "09".repeat(16), "key": "07".repeat(32)},
        "hint": "chrome",
    }})
}

fn request(id: u32, op: &str, body: &Value) -> Value {
    json!({"v": 1, "id": id, "op": op, "body": body})
}

fn ok(id: u32, body: &Value) -> Value {
    json!({"v": 1, "id": id, "ok": true, "body": body})
}

fn err(id: u32, code: &str) -> Value {
    json!({"v": 1, "id": id, "ok": false, "error": code})
}

fn event(name: &str) -> Value {
    json!({"event": name, "v": 1})
}

#[test]
fn hello_without_a_pairing_reports_the_lock_state() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&request(
            1,
            "hello",
            &json!({"pairing": null, "hint": "chrome"}),
        ))
        .await;
        let mut stream = rig.desktop.accept().await;
        assert!(matches!(
            read_client_message(&mut stream).await,
            Ok(ClientMessage::Status { .. })
        ));
        write_message(
            &mut stream,
            &ServerMessage::Status {
                v: 1,
                state: LockState::Locked,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            rig.recv().await,
            ok(1, &json!({"desktop": "locked", "paired": false}))
        );
        assert_eq!(rig.finish().await, Ok(()));
    });
}

#[test]
fn hello_status_fill_are_relayed_and_stdin_eof_ends_cleanly() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        assert_eq!(
            rig.recv().await,
            ok(1, &json!({"desktop": "unlocked", "paired": true}))
        );

        rig.send(&request(2, "status", &json!({}))).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await,
            request(2, "status", &json!({}))
        );
        reply(
            &mut stream,
            &mut session,
            &ok(2, &json!({"state": "unlocked"})),
        )
        .await;
        assert_eq!(rig.recv().await, ok(2, &json!({"state": "unlocked"})));

        let fill = json!({
            "vaultId": "v", "itemId": "i", "origin": "https://example.com",
            "topOrigin": "https://example.com", "frame": "top",
            "want": {"login": true, "otp": false},
        });
        rig.send(&request(3, "fill", &fill)).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await,
            request(3, "fill", &fill)
        );
        let secret = ok(3, &json!({"username": "u", "password": "pw", "otp": null}));
        reply(&mut stream, &mut session, &secret).await;
        assert_eq!(rig.recv().await, secret);

        assert_eq!(rig.finish().await, Ok(()));
    });
}

#[test]
fn ops_outside_the_allowlist_never_reach_the_desktop() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        for (id, op) in [
            (10, "get_item"),
            (11, "export"),
            (12, "reveal_secret"),
            (13, "Fill"),
            (14, ""),
        ] {
            rig.send(&request(id, op, &json!({}))).await;
            assert_eq!(rig.recv().await, err(id, "invalidRequest"), "{op}");
        }
        // The next thing the desktop sees is the allowed request.
        rig.send(&request(15, "status", &json!({}))).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await,
            request(15, "status", &json!({}))
        );
    });
}

#[test]
fn a_missing_desktop_is_reported_not_crashed_on() {
    block_on(async {
        let mut rig = Rig::new();
        rig.up.store(false, Ordering::SeqCst);
        rig.send(&hello(1)).await;
        assert_eq!(
            rig.recv().await,
            ok(1, &json!({"desktop": "notRunning", "paired": true}))
        );
        rig.send(&request(2, "fill", &json!({}))).await;
        assert_eq!(rig.recv().await, err(2, "desktopNotRunning"));
        rig.send(&request(
            3,
            "hello",
            &json!({"pairing": null, "hint": "other"}),
        ))
        .await;
        assert_eq!(
            rig.recv().await,
            ok(3, &json!({"desktop": "notRunning", "paired": false}))
        );
        rig.send(&request(4, "pair", &json!({"hint": "other"})))
            .await;
        assert_eq!(rig.recv().await, err(4, "desktopNotRunning"));
        assert_eq!(rig.finish().await, Ok(()));
    });
}

#[test]
fn a_dropped_desktop_fails_requests_and_the_next_one_reconnects() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;

        rig.send(&request(
            5,
            "search",
            &json!({"query": "x", "origin": null}),
        ))
        .await;
        read_request(&mut stream, &mut session).await;
        drop(stream);
        assert_eq!(rig.recv().await, err(5, "desktopNotRunning"));
        assert_eq!(rig.recv().await, json!({"event": "disconnected", "v": 1}));

        rig.send(&request(6, "status", &json!({}))).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        assert_eq!(
            rig.recv().await,
            json!({"event": "state", "v": 1, "state": "unlocked"}),
            "the new session announces its state"
        );
        assert_eq!(
            read_request(&mut stream, &mut session).await,
            request(6, "status", &json!({}))
        );
        reply(
            &mut stream,
            &mut session,
            &ok(6, &json!({"state": "unlocked"})),
        )
        .await;
        assert_eq!(rig.recv().await, ok(6, &json!({"state": "unlocked"})));
    });
}

#[test]
fn a_silent_desktop_times_out_after_15_seconds() {
    block_on_paused(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        rig.send(&request(
            2,
            "copy",
            &json!({"source": {"kind": "generated"}}),
        ))
        .await;
        read_request(&mut stream, &mut session).await;
        let started = tokio::time::Instant::now();
        assert_eq!(rig.recv().await, err(2, "timeout"));
        assert_eq!(started.elapsed(), Duration::from_secs(REQUEST_TIMEOUT_SECS));
        // A late answer is dropped; the session still works.
        reply(
            &mut stream,
            &mut session,
            &ok(2, &json!({"clearAfterSeconds": null})),
        )
        .await;
        rig.send(&request(3, "status", &json!({}))).await;
        read_request(&mut stream, &mut session).await;
        reply(
            &mut stream,
            &mut session,
            &ok(3, &json!({"state": "locked"})),
        )
        .await;
        assert_eq!(rig.recv().await, ok(3, &json!({"state": "locked"})));
    });
}

#[test]
fn malformed_browser_frames_end_the_host_with_an_error() {
    block_on(async {
        let truncated = [&5u32.to_ne_bytes()[..], b"{"].concat();
        let cases: [(Vec<u8>, Error); 4] = [
            (0u32.to_ne_bytes().to_vec(), Error::FrameEmpty),
            (u32::MAX.to_ne_bytes().to_vec(), Error::FrameTooLarge),
            (truncated, Error::FrameTruncated),
            (vec![1, 2], Error::FrameTruncated),
        ];
        for (bytes, expected) in cases {
            let mut rig = Rig::new();
            rig.to_host.write_all(&bytes).await.unwrap();
            drop(rig.to_host);
            assert_eq!(rig.host.await.unwrap(), Err(expected), "{bytes:?}");
        }
        // No bytes at all is a clean end.
        assert_eq!(Rig::new().finish().await, Ok(()));
        for bad in [
            &b"not json"[..],
            br#"{"v":1,"id":1,"op":"status","body":{},"extra":true}"#,
            br#"{"v":1,"id":1,"op":"status"}"#,
            b"[]",
        ] {
            let mut rig = Rig::new();
            rig.send_raw(bad).await;
            assert_eq!(rig.host.await.unwrap(), Err(Error::Malformed));
        }
    });
}

#[test]
fn another_protocol_version_is_refused() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&json!({"v": 2, "id": 4, "op": "status", "body": {}}))
            .await;
        assert_eq!(rig.recv().await, err(4, "protocolVersion"));
        assert_eq!(rig.finish().await, Ok(()));
    });
}

#[test]
fn an_oversized_request_is_refused_before_it_is_sealed() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        // A frame at the cap on the stdio link that cannot fit the AEAD
        // overhead on the desktop link.
        let shell = request(2, "search", &json!({"query": "", "origin": null})).to_string();
        let pad = MAX_FRAME_BYTES - shell.len();
        let big = request(
            2,
            "search",
            &json!({"query": "a".repeat(pad), "origin": null}),
        );
        assert_eq!(big.to_string().len(), MAX_FRAME_BYTES);
        rig.send(&big).await;
        assert_eq!(rig.recv().await, err(2, "frameTooLarge"));
        // Nothing was sent: the next request is the one the desktop sees.
        rig.send(&request(3, "status", &json!({}))).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await["id"],
            json!(3)
        );
    });
}

#[test]
fn an_id_in_flight_cannot_be_reused() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        rig.send(&request(2, "status", &json!({}))).await;
        read_request(&mut stream, &mut session).await;
        rig.send(&request(2, "status", &json!({}))).await;
        assert_eq!(rig.recv().await, err(2, "invalidRequest"));
    });
}

#[test]
fn desktop_events_are_forwarded_and_unpaired_forgets_the_pairing() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;

        let locked = json!({"event": "state", "v": 1, "state": "locked"});
        reply(&mut stream, &mut session, &locked).await;
        assert_eq!(rig.recv().await, locked);

        reply(&mut stream, &mut session, &event("unpaired")).await;
        assert_eq!(rig.recv().await, event("unpaired"));
        rig.send(&request(2, "fill", &json!({}))).await;
        assert_eq!(rig.recv().await, err(2, "notPaired"));
    });
}

#[test]
fn an_impostor_desktop_is_refused() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        // The impostor does not know the key.
        let served = rig
            .desktop
            .session_with(Some(key(8)), LockState::Unlocked)
            .await;
        assert!(served.is_err(), "the host hangs up");
        assert_eq!(rig.recv().await, err(1, "desktopNotAuthentic"));
        rig.send(&request(2, "status", &json!({}))).await;
        let served = rig
            .desktop
            .session_with(Some(key(8)), LockState::Unlocked)
            .await;
        assert!(served.is_err());
        assert_eq!(rig.recv().await, err(2, "desktopNotAuthentic"));
    });
}

#[test]
fn a_pairing_the_desktop_does_not_know_is_reported_as_unpaired() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let refused = rig.desktop.session_with(None, LockState::Unlocked).await;
        assert_eq!(refused.unwrap_err(), Error::Refused(ErrorCode::NotPaired));
        // The host then asks for the lock state to answer `hello`.
        let mut stream = rig.desktop.accept().await;
        assert!(matches!(
            read_client_message(&mut stream).await,
            Ok(ClientMessage::Status { .. })
        ));
        write_message(
            &mut stream,
            &ServerMessage::Status {
                v: 1,
                state: LockState::Unlocked,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            rig.recv().await,
            ok(1, &json!({"desktop": "unlocked", "paired": false}))
        );
    });
}

#[test]
fn status_and_focus_work_without_a_pairing() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&request(1, "focus", &json!({}))).await;
        let mut stream = rig.desktop.accept().await;
        assert!(matches!(
            read_client_message(&mut stream).await,
            Ok(ClientMessage::Focus { .. })
        ));
        write_message(&mut stream, &ServerMessage::Focused { v: 1 })
            .await
            .unwrap();
        assert_eq!(rig.recv().await, ok(1, &json!({})));

        rig.send(&request(2, "status", &json!({}))).await;
        let mut stream = rig.desktop.accept().await;
        read_client_message(&mut stream).await.unwrap();
        write_message(
            &mut stream,
            &ServerMessage::Status {
                v: 1,
                state: LockState::Locked,
            },
        )
        .await
        .unwrap();
        assert_eq!(rig.recv().await, ok(2, &json!({"state": "locked"})));

        rig.send(&request(3, "fill", &json!({}))).await;
        assert_eq!(rig.recv().await, err(3, "notPaired"));

        rig.send(&request(4, "focus", &json!({}))).await;
        let mut stream = rig.desktop.accept().await;
        read_client_message(&mut stream).await.unwrap();
        write_message(&mut stream, &ServerMessage::refused(ErrorCode::RateLimited))
            .await
            .unwrap();
        assert_eq!(rig.recv().await, err(4, "rateLimited"));
    });
}

#[test]
fn pairing_shows_the_code_returns_the_key_and_connects() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&request(1, "pair", &json!({"hint": "edge"})))
            .await;
        let mut stream = rig.desktop.accept().await;
        let Ok(ClientMessage::PairRequest { browser, hint, .. }) =
            read_client_message(&mut stream).await
        else {
            panic!("no pair request");
        };
        assert_eq!(
            (browser, hint),
            (BrowserKind::Firefox, Hint::Edge),
            "browser from the caller"
        );
        let pending = ServerMessage::PairPending {
            v: 1,
            code: "4F7K2Q".into(),
            expires_in_seconds: 120,
        };
        write_message(&mut stream, &pending).await.unwrap();
        assert_eq!(
            rig.recv().await,
            json!({"event": "pairCode", "v": 1, "requestId": 1, "code": "4F7K2Q", "expiresInSeconds": 120})
        );

        let approved = ServerMessage::PairApproved {
            v: 1,
            pairing_id: Hex(PAIRING_ID),
            key: HexKey(key(7)),
        };
        write_message(&mut stream, &approved).await.unwrap();
        assert_eq!(
            rig.recv().await,
            ok(
                1,
                &json!({"pairingId": "09".repeat(16), "key": "07".repeat(32)})
            )
        );

        // The host handshakes with the new pairing on a new connection.
        let (mut stream, mut session) = rig.desktop.session().await;
        assert_eq!(
            rig.recv().await,
            json!({"event": "state", "v": 1, "state": "unlocked"})
        );
        rig.send(&request(2, "status", &json!({}))).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await["op"],
            json!("status")
        );
    });
}

#[test]
fn a_refused_pairing_is_reported_with_the_reason() {
    block_on(async {
        for reason in [
            ErrorCode::PairingDenied,
            ErrorCode::Locked,
            ErrorCode::PairingBlocked,
        ] {
            let mut rig = Rig::new();
            rig.send(&request(1, "pair", &json!({"hint": "other"})))
                .await;
            let mut stream = rig.desktop.accept().await;
            read_client_message(&mut stream).await.unwrap();
            write_message(&mut stream, &ServerMessage::refused(reason))
                .await
                .unwrap();
            assert_eq!(
                rig.recv().await,
                err(1, serde_json::to_value(reason).unwrap().as_str().unwrap())
            );
        }
    });
}

#[test]
fn a_denied_pairing_after_the_code_is_reported() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&request(1, "pair", &json!({"hint": "other"})))
            .await;
        let mut stream = rig.desktop.accept().await;
        read_client_message(&mut stream).await.unwrap();
        let pending = ServerMessage::PairPending {
            v: 1,
            code: "000000".into(),
            expires_in_seconds: 120,
        };
        write_message(&mut stream, &pending).await.unwrap();
        assert_eq!(rig.recv().await["event"], json!("pairCode"));
        write_message(
            &mut stream,
            &ServerMessage::refused(ErrorCode::PairingDenied),
        )
        .await
        .unwrap();
        assert_eq!(rig.recv().await, err(1, "pairingDenied"));
        // And a bad code is not shown.
        let mut rig = Rig::new();
        rig.send(&request(1, "pair", &json!({"hint": "other"})))
            .await;
        let mut stream = rig.desktop.accept().await;
        read_client_message(&mut stream).await.unwrap();
        let pending = ServerMessage::PairPending {
            v: 1,
            code: "<script>".into(),
            expires_in_seconds: 120,
        };
        write_message(&mut stream, &pending).await.unwrap();
        assert_eq!(rig.recv().await, err(1, "internal"));
    });
}

#[test]
fn bad_hello_and_pair_bodies_are_invalid_requests() {
    block_on(async {
        let mut rig = Rig::new();
        let bad = [
            ("hello", json!({"pairing": null})),
            (
                "hello",
                json!({"pairing": {"pairingId": "zz", "key": "00"}, "hint": "chrome"}),
            ),
            (
                "hello",
                json!({"pairing": null, "hint": "chrome", "extra": 1}),
            ),
            ("pair", json!({})),
            ("pair", json!({"hint": "chrome", "extra": 1})),
        ];
        for (n, (op, body)) in bad.into_iter().enumerate() {
            let id = u32::try_from(n).unwrap();
            rig.send(&request(id, op, &body)).await;
            assert_eq!(rig.recv().await, err(id, "invalidRequest"), "{op} {body}");
        }
    });
}

#[test]
fn the_same_pairing_keeps_the_session() {
    block_on(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        rig.send(&hello(2)).await;
        assert_eq!(
            rig.recv().await,
            ok(2, &json!({"desktop": "unlocked", "paired": true})),
            "answered from the open session, without a new connection"
        );
        rig.send(&request(3, "status", &json!({}))).await;
        assert_eq!(
            read_request(&mut stream, &mut session).await["id"],
            json!(3)
        );
        // A different pairing starts over.
        let other = json!({"v": 1, "id": 4, "op": "hello", "body": {
            "pairing": {"pairingId": "0a".repeat(16), "key": "07".repeat(32)}, "hint": "chrome"}});
        rig.send(&other).await;
        let served = rig.desktop.session().await;
        assert_eq!(
            rig.recv().await,
            ok(4, &json!({"desktop": "unlocked", "paired": true}))
        );
        drop(served);
    });
}

#[test]
fn a_timed_out_id_cannot_be_reused_and_a_late_answer_is_dropped() {
    block_on_paused(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        rig.send(&request(
            2,
            "copy",
            &json!({"source": {"kind": "generated"}}),
        ))
        .await;
        read_request(&mut stream, &mut session).await;
        assert_eq!(rig.recv().await, err(2, "timeout"));
        // The extension reuses the ID: refused here, never forwarded.
        rig.send(&request(2, "status", &json!({}))).await;
        assert_eq!(rig.recv().await, err(2, "invalidRequest"));
        // The late answer to the first request is dropped, not taken for an
        // answer to anything else.
        reply(
            &mut stream,
            &mut session,
            &ok(2, &json!({"clearAfterSeconds": null})),
        )
        .await;
        rig.send(&request(3, "status", &json!({}))).await;
        let forwarded = read_request(&mut stream, &mut session).await;
        assert_eq!(
            forwarded["id"], 3,
            "the reused ID never reached the desktop"
        );
        reply(
            &mut stream,
            &mut session,
            &ok(3, &json!({"state": "locked"})),
        )
        .await;
        assert_eq!(rig.recv().await, ok(3, &json!({"state": "locked"})));
    });
}

#[test]
fn a_connection_with_too_many_timed_out_ids_is_closed() {
    block_on_paused(async {
        let mut rig = Rig::new();
        rig.send(&hello(1)).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        rig.recv().await;
        let total = u32::try_from(MAX_TOMBSTONES + 1).unwrap();
        for id in 100..100 + total {
            rig.send(&request(id, "status", &json!({}))).await;
        }
        for _ in 0..total {
            read_request(&mut stream, &mut session).await;
        }
        // Every request times out; the last tombstone is one too many.
        for _ in 0..total {
            assert_eq!(rig.recv().await["error"], "timeout");
        }
        assert_eq!(rig.recv().await, event("disconnected"));
        // The next request opens a fresh connection with an empty set.
        rig.send(&request(100, "status", &json!({}))).await;
        let (mut stream, mut session) = rig.desktop.session().await;
        assert_eq!(read_request(&mut stream, &mut session).await["id"], 100);
    });
}

#[test]
fn propose_save_waits_for_the_desktop_dialog_and_the_host_answers_first() {
    // The desktop's save dialog stays open up to SAVE_PROMPT_TIMEOUT_SECS; the host must not
    // time out before that, and must answer before the extension's 305 s wait ends.
    let save = request_timeout(Op::ProposeSave);
    assert!(save > Duration::from_secs(SAVE_PROMPT_TIMEOUT_SECS));
    assert!(save < Duration::from_secs(305));
    for op in [
        Op::Status,
        Op::Fill,
        Op::Copy,
        Op::CheckSave,
        Op::ListForOrigin,
    ] {
        assert_eq!(
            request_timeout(op),
            Duration::from_secs(REQUEST_TIMEOUT_SECS)
        );
    }
}
