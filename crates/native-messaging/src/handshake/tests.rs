use mosaic_core::browser_channel::{ChannelSession, new_channel_nonce, new_pairing_id};
use tokio::io::{DuplexStream, duplex};
use tokio::task::JoinHandle;

use super::*;
use crate::testutil::{block_on, block_on_paused};

fn key(byte: u8) -> PairingKey {
    PairingKey::from_bytes(&mut [byte; 32])
}

/// What a server task reports.
type ServerResult = Result<ChannelSession, Error>;

/// Spawns the desktop's side: reads the first message, expects a `hello` and
/// runs the handshake with `server_key`.
fn spawn_server(
    mut stream: DuplexStream,
    server_key: Option<PairingKey>,
    state: LockState,
) -> JoinHandle<(ServerResult, DuplexStream)> {
    tokio::spawn(async move {
        let result = async {
            let ClientMessage::Hello {
                pairing_id, nonce, ..
            } = read_client_message(&mut stream).await?
            else {
                return Err(Error::Malformed);
            };
            server_handshake(
                &mut stream,
                &pairing_id.0,
                &nonce.0,
                server_key.as_ref(),
                state,
            )
            .await
        }
        .await;
        (result, stream)
    })
}

#[test]
fn success_then_frames_flow_both_ways() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let id = new_pairing_id().unwrap();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        let (mut host_session, state) = client_handshake(&mut host, &key(7), &id).await.unwrap();
        assert_eq!(state, LockState::Unlocked);
        let (server_session, mut desk) = server.await.unwrap();
        let mut desk_session = server_session.unwrap();

        send(&mut host, &mut host_session, b"{\"v\":1}")
            .await
            .unwrap();
        let frame = next_frame(&mut desk).await.unwrap();
        assert_eq!(
            desk_session.open(&frame).unwrap().expose_secret(),
            b"{\"v\":1}"
        );
        send(&mut desk, &mut desk_session, b"reply").await.unwrap();
        let frame = next_frame(&mut host).await.unwrap();
        assert_eq!(host_session.open(&frame).unwrap().expose_secret(), b"reply");
    });
}

#[test]
fn a_locked_desktop_reports_locked() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let id = new_pairing_id().unwrap();
        let server = spawn_server(desk, Some(key(7)), LockState::Locked);
        let (_, state) = client_handshake(&mut host, &key(7), &id).await.unwrap();
        assert_eq!(state, LockState::Locked);
        server.await.unwrap().0.unwrap();
    });
}

#[test]
fn a_wrong_key_fails_on_both_sides() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let id = new_pairing_id().unwrap();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        let err = client_handshake(&mut host, &key(8), &id).await.unwrap_err();
        assert_eq!(
            err,
            Error::NotAuthentic,
            "the desktop's proof does not verify"
        );
        drop(host);
        let (result, _) = server.await.unwrap();
        assert_eq!(result.unwrap_err(), Error::Closed, "the host hung up");
    });
}

#[test]
fn an_unknown_pairing_is_refused_not_paired() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let id = new_pairing_id().unwrap();
        let server = spawn_server(desk, None, LockState::Unlocked);
        let err = client_handshake(&mut host, &key(7), &id).await.unwrap_err();
        assert_eq!(err, Error::Refused(ErrorCode::NotPaired));
        let (result, _) = server.await.unwrap();
        assert_eq!(result.unwrap_err(), Error::Refused(ErrorCode::NotPaired));
    });
}

/// A client that speaks the protocol by hand, so a test can record and
/// replay its frames or send a bad proof.
struct Manual {
    id: [u8; 16],
    nonce_h: [u8; 32],
}

impl Manual {
    fn new() -> Self {
        Self {
            id: new_pairing_id().unwrap(),
            nonce_h: new_channel_nonce().unwrap(),
        }
    }

    fn hello(&self) -> ClientMessage {
        ClientMessage::Hello {
            v: 1,
            pairing_id: Hex(self.id),
            nonce: Hex(self.nonce_h),
        }
    }
}

fn framed(message: &impl serde::Serialize) -> Vec<u8> {
    let json = serde_json::to_vec(message).unwrap();
    let mut out = u32::try_from(json.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(&json);
    out
}

#[test]
fn a_replayed_hello_and_finish_fail_against_a_new_server() {
    block_on(async {
        // Connection 1: a hand-driven client, whose bytes are recorded.
        let (mut host, desk) = duplex(1 << 16);
        let manual = Manual::new();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        let hello = framed(&manual.hello());
        write_message(&mut host, &manual.hello()).await.unwrap();
        let ServerMessage::Challenge { nonce, .. } = read_server_message(&mut host).await.unwrap()
        else {
            panic!("no challenge");
        };
        let proof = key(7)
            .proof(Role::Host, &manual.id, &manual.nonce_h, &nonce.0)
            .unwrap();
        let finish_msg = ClientMessage::Finish {
            v: 1,
            proof: Hex(proof),
        };
        let finish = framed(&finish_msg);
        write_message(&mut host, &finish_msg).await.unwrap();
        server.await.unwrap().0.unwrap();

        // Connection 2: the same two frames to a fresh server.
        let (mut attacker, desk) = duplex(1 << 16);
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        tokio::io::AsyncWriteExt::write_all(&mut attacker, &[hello, finish].concat())
            .await
            .unwrap();
        let (result, _) = server.await.unwrap();
        assert_eq!(result.unwrap_err(), Error::Refused(ErrorCode::NotPaired));
        // The attacker is told the same thing as for an unknown pairing.
        let ServerMessage::Challenge { .. } = read_server_message(&mut attacker).await.unwrap()
        else {
            panic!("no challenge");
        };
        assert!(matches!(
            read_server_message(&mut attacker).await,
            Ok(ServerMessage::Refused {
                reason: ErrorCode::NotPaired,
                ..
            })
        ));
    });
}

#[test]
fn a_bad_finish_proof_is_refused() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let manual = Manual::new();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        write_message(&mut host, &manual.hello()).await.unwrap();
        read_server_message(&mut host).await.unwrap();
        write_message(
            &mut host,
            &ClientMessage::Finish {
                v: 1,
                proof: Hex([0; 32]),
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            read_server_message(&mut host).await,
            Ok(ServerMessage::Refused {
                reason: ErrorCode::NotPaired,
                ..
            })
        ));
        assert_eq!(
            server.await.unwrap().0.unwrap_err(),
            Error::Refused(ErrorCode::NotPaired)
        );
    });
}

#[test]
fn a_wrong_message_after_the_challenge_is_refused() {
    block_on(async {
        let (mut host, desk) = duplex(1 << 16);
        let manual = Manual::new();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        write_message(&mut host, &manual.hello()).await.unwrap();
        read_server_message(&mut host).await.unwrap();
        write_message(&mut host, &manual.hello()).await.unwrap();
        assert!(matches!(
            read_server_message(&mut host).await,
            Ok(ServerMessage::Refused {
                reason: ErrorCode::InvalidRequest,
                ..
            })
        ));
        assert_eq!(server.await.unwrap().0.unwrap_err(), Error::Malformed);
    });
}

#[test]
fn another_protocol_version_is_refused() {
    block_on(async {
        let (mut host, mut desk) = duplex(1 << 16);
        let id = "00".repeat(16);
        let nonce = "11".repeat(32);
        let json = format!(r#"{{"type":"hello","v":2,"pairingId":"{id}","nonce":"{nonce}"}}"#);
        write_frame(&mut host, ByteOrder::Big, json.as_bytes())
            .await
            .unwrap();
        assert_eq!(
            read_client_message(&mut desk).await.unwrap_err(),
            Error::ProtocolVersion
        );
        assert!(matches!(
            read_server_message(&mut host).await,
            Ok(ServerMessage::Refused {
                reason: ErrorCode::ProtocolVersion,
                ..
            })
        ));
        // And garbage is `invalidRequest`.
        write_frame(&mut host, ByteOrder::Big, b"nonsense")
            .await
            .unwrap();
        assert_eq!(
            read_client_message(&mut desk).await.unwrap_err(),
            Error::Malformed
        );
        assert!(matches!(
            read_server_message(&mut host).await,
            Ok(ServerMessage::Refused {
                reason: ErrorCode::InvalidRequest,
                ..
            })
        ));
    });
}

#[test]
fn a_tampered_first_frame_fails_the_host() {
    block_on(async {
        let (mut host, mut desk) = duplex(1 << 16);
        let manual = Manual::new();
        let client = tokio::spawn(async move {
            let result = client_handshake(&mut host, &key(7), &manual.id).await;
            result.map(|(_, state)| state)
        });
        let ClientMessage::Hello {
            pairing_id, nonce, ..
        } = read_client_message(&mut desk).await.unwrap()
        else {
            panic!("no hello");
        };
        // Serve the handshake by hand and corrupt the AEAD frame.
        let nonce_d = new_channel_nonce().unwrap();
        let k = key(7);
        let proof = k
            .proof(Role::Desktop, &pairing_id.0, &nonce.0, &nonce_d)
            .unwrap();
        write_message(
            &mut desk,
            &ServerMessage::Challenge {
                v: 1,
                nonce: Hex(nonce_d),
                proof: Hex(proof),
            },
        )
        .await
        .unwrap();
        read_client_message(&mut desk).await.unwrap();
        let mut session = k
            .session(Role::Desktop, &pairing_id.0, &nonce.0, &nonce_d)
            .unwrap();
        let mut frame = session
            .seal(br#"{"event":"state","v":1,"state":"locked"}"#)
            .unwrap();
        let last = frame.len() - 1;
        frame[last] ^= 1;
        write_frame(&mut desk, ByteOrder::Big, &frame)
            .await
            .unwrap();
        assert_eq!(
            client.await.unwrap().unwrap_err(),
            Error::Channel(mosaic_core::Error::Decryption)
        );
    });
}

#[test]
fn the_first_frame_must_be_a_state_event() {
    block_on(async {
        let (mut host, mut desk) = duplex(1 << 16);
        let manual = Manual::new();
        let client = tokio::spawn(async move {
            client_handshake(&mut host, &key(7), &manual.id)
                .await
                .map(|_| ())
        });
        let ClientMessage::Hello {
            pairing_id, nonce, ..
        } = read_client_message(&mut desk).await.unwrap()
        else {
            panic!("no hello");
        };
        let nonce_d = new_channel_nonce().unwrap();
        let k = key(7);
        let proof = k
            .proof(Role::Desktop, &pairing_id.0, &nonce.0, &nonce_d)
            .unwrap();
        let challenge = ServerMessage::Challenge {
            v: 1,
            nonce: Hex(nonce_d),
            proof: Hex(proof),
        };
        write_message(&mut desk, &challenge).await.unwrap();
        read_client_message(&mut desk).await.unwrap();
        let mut session = k
            .session(Role::Desktop, &pairing_id.0, &nonce.0, &nonce_d)
            .unwrap();
        send(&mut desk, &mut session, br#"{"event":"unpaired","v":1}"#)
            .await
            .unwrap();
        assert_eq!(client.await.unwrap().unwrap_err(), Error::Malformed);
    });
}

#[test]
fn every_step_times_out() {
    // The desktop never answers `hello`.
    block_on_paused(async {
        let (mut host, _desk) = duplex(1 << 16);
        let err = client_handshake(&mut host, &key(7), &[1; 16])
            .await
            .unwrap_err();
        assert_eq!(err, Error::Timeout);
    });
    // The desktop challenges, the host never finishes.
    block_on_paused(async {
        let (mut host, desk) = duplex(1 << 16);
        let manual = Manual::new();
        let server = spawn_server(desk, Some(key(7)), LockState::Unlocked);
        write_message(&mut host, &manual.hello()).await.unwrap();
        read_server_message(&mut host).await.unwrap();
        let (result, _) = server.await.unwrap();
        assert_eq!(result.unwrap_err(), Error::Timeout);
    });
    // The host finishes, the desktop never sends the first frame.
    block_on_paused(async {
        let (mut host, mut desk) = duplex(1 << 16);
        let manual = Manual::new();
        let id = manual.id;
        let client =
            tokio::spawn(
                async move { client_handshake(&mut host, &key(7), &id).await.map(|_| ()) },
            );
        let ClientMessage::Hello {
            pairing_id, nonce, ..
        } = read_client_message(&mut desk).await.unwrap()
        else {
            panic!("no hello");
        };
        let nonce_d = new_channel_nonce().unwrap();
        let proof = key(7)
            .proof(Role::Desktop, &pairing_id.0, &nonce.0, &nonce_d)
            .unwrap();
        let challenge = ServerMessage::Challenge {
            v: 1,
            nonce: Hex(nonce_d),
            proof: Hex(proof),
        };
        write_message(&mut desk, &challenge).await.unwrap();
        read_client_message(&mut desk).await.unwrap();
        assert_eq!(client.await.unwrap().unwrap_err(), Error::Timeout);
    });
    // The host never sends `hello`.
    block_on_paused(async {
        let (_host, mut desk) = duplex(1 << 16);
        assert_eq!(
            read_client_message(&mut desk).await.unwrap_err(),
            Error::Timeout
        );
    });
}

#[test]
fn preauth_requests_get_one_answer() {
    block_on(async {
        let (mut host, mut desk) = duplex(1 << 16);
        let desktop = tokio::spawn(async move {
            let msg = read_client_message(&mut desk).await.unwrap();
            assert!(matches!(msg, ClientMessage::Status { .. }));
            write_message(
                &mut desk,
                &ServerMessage::Status {
                    v: 1,
                    state: LockState::Locked,
                },
            )
            .await
            .unwrap();
            let msg = read_client_message(&mut desk).await.unwrap();
            assert!(matches!(msg, ClientMessage::Focus { .. }));
            write_message(&mut desk, &ServerMessage::refused(ErrorCode::RateLimited))
                .await
                .unwrap();
        });
        let status = preauth_request(&mut host, &ClientMessage::Status { v: 1 }).await;
        assert!(matches!(
            status,
            Ok(ServerMessage::Status {
                state: LockState::Locked,
                ..
            })
        ));
        let focus = preauth_request(&mut host, &ClientMessage::Focus { v: 1 }).await;
        assert_eq!(focus.unwrap_err(), Error::Refused(ErrorCode::RateLimited));
        desktop.await.unwrap();
    });
}
