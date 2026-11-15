use mosaic_core::browser_channel::{new_channel_nonce, new_pairing_id};
use tokio::io::{DuplexStream, duplex};
use tokio::task::JoinHandle;

use super::*;
use crate::handshake::{self, read_client_message};
use crate::preauth::ClientMessage;
use crate::protocol::{ErrorCode, LockState};
use crate::testutil::block_on;

fn key(byte: u8) -> PairingKey {
    PairingKey::from_bytes(&mut [byte; 32])
}

type ServerResult = Result<ChannelSession, Error>;

fn spawn_server(
    mut stream: DuplexStream,
    server_key: Option<PairingKey>,
    first: &'static [u8],
) -> JoinHandle<(ServerResult, DuplexStream)> {
    tokio::spawn(async move {
        let result = async {
            let AgentClientMessage::Hello {
                pairing_id, nonce, ..
            } = agent_read_client_message(&mut stream).await?
            else {
                return Err(Error::Malformed);
            };
            agent_server_handshake(
                &mut stream,
                &pairing_id.0,
                &nonce.0,
                server_key.as_ref(),
                first,
            )
            .await
        }
        .await;
        (result, stream)
    })
}

#[test]
fn agent_handshake_succeeds_with_the_agent_key() {
    block_on(async {
        let (mut relay, desk) = duplex(1 << 16);
        let id = new_pairing_id().unwrap();
        let server = spawn_server(desk, Some(key(7)), b"first");
        let (mut relay_session, first) = agent_client_handshake(&mut relay, &key(7), &id)
            .await
            .unwrap();
        assert_eq!(first.expose_secret(), b"first");
        let (server_session, mut desk) = server.await.unwrap();
        let mut desk_session = server_session.unwrap();

        handshake::send(&mut relay, &mut relay_session, b"ping")
            .await
            .unwrap();
        let frame = handshake::next_frame(&mut desk).await.unwrap();
        assert_eq!(desk_session.open(&frame).unwrap().expose_secret(), b"ping");
        handshake::send(&mut desk, &mut desk_session, b"pong")
            .await
            .unwrap();
        let frame = handshake::next_frame(&mut relay).await.unwrap();
        assert_eq!(relay_session.open(&frame).unwrap().expose_secret(), b"pong");
    });
}

#[test]
fn browser_pairing_key_fails_the_agent_handshake() {
    block_on(async {
        // The same key bytes paired on the browser channel.
        let id = new_pairing_id().unwrap();

        // Browser client against the agent server.
        let (mut client, desk) = duplex(1 << 16);
        let server = spawn_server(desk, Some(key(7)), b"first");
        let err = handshake::client_handshake(&mut client, &key(7), &id)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotAuthentic | Error::Refused(ErrorCode::NotPaired)
            ),
            "{err:?}"
        );
        drop(client);
        let (result, _) = server.await.unwrap();
        assert!(result.is_err(), "no session on the agent server");

        // Agent client against the browser server.
        let (mut client, mut desk) = duplex(1 << 16);
        let server = tokio::spawn(async move {
            let ClientMessage::Hello {
                pairing_id, nonce, ..
            } = read_client_message(&mut desk).await?
            else {
                return Err(Error::Malformed);
            };
            handshake::server_handshake(
                &mut desk,
                &pairing_id.0,
                &nonce.0,
                Some(&key(7)),
                LockState::Unlocked,
            )
            .await
        });
        let err = agent_client_handshake(&mut client, &key(7), &id)
            .await
            .map(|_| ())
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::NotAuthentic | Error::AgentRefused(AgentRefusal::NotPaired)
            ),
            "{err:?}"
        );
        drop(client);
        assert!(
            server.await.unwrap().is_err(),
            "no session on the browser server"
        );
    });
}

#[test]
fn unknown_pairing_and_wrong_proof_get_the_same_refusal() {
    block_on(async {
        let id = new_pairing_id().unwrap();

        // Unknown ID: refused at `hello`.
        let (mut relay, desk) = duplex(1 << 16);
        let server = spawn_server(desk, None, b"first");
        let err = agent_client_handshake(&mut relay, &key(7), &id)
            .await
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err, Error::AgentRefused(AgentRefusal::NotPaired));
        assert_eq!(
            server.await.unwrap().0.map(|_| ()).unwrap_err(),
            Error::AgentRefused(AgentRefusal::NotPaired)
        );

        // Wrong proof after `finish`: a relay that proves with another key
        // but accepts the desktop's proof cannot exist (the desktop's proof
        // already fails), so send the finish by hand.
        let (mut relay, mut desk) = duplex(1 << 16);
        let nonce_h = new_channel_nonce().unwrap();
        let server = tokio::spawn(async move {
            let AgentClientMessage::Hello {
                pairing_id, nonce, ..
            } = agent_read_client_message(&mut desk).await?
            else {
                return Err(Error::Malformed);
            };
            agent_server_handshake(&mut desk, &pairing_id.0, &nonce.0, Some(&key(7)), b"x").await
        });
        let hello = AgentClientMessage::Hello {
            v: 1,
            pairing_id: Hex(id),
            nonce: Hex(nonce_h),
        };
        write_message(&mut relay, &hello).await.unwrap();
        let AgentServerMessage::Challenge { .. } =
            agent_read_server_message(&mut relay).await.unwrap()
        else {
            panic!("no challenge");
        };
        let finish = AgentClientMessage::Finish {
            v: 1,
            proof: Hex([9; 32]),
        };
        write_message(&mut relay, &finish).await.unwrap();
        assert!(matches!(
            agent_read_server_message(&mut relay).await.unwrap(),
            AgentServerMessage::Refused {
                reason: AgentRefusal::NotPaired,
                ..
            }
        ));
        assert_eq!(
            server.await.unwrap().map(|_| ()).unwrap_err(),
            Error::AgentRefused(AgentRefusal::NotPaired)
        );
    });
}

#[test]
fn preauth_status_gets_one_answer() {
    block_on(async {
        let (mut relay, mut desk) = duplex(1 << 16);
        let server = tokio::spawn(async move {
            let message = agent_read_client_message(&mut desk).await.unwrap();
            assert!(matches!(message, AgentClientMessage::Status { .. }));
            write_message(
                &mut desk,
                &AgentServerMessage::Status {
                    v: 1,
                    vault: crate::agent_preauth::VaultState::Locked,
                    agent_access: crate::agent_preauth::AccessState::On,
                },
            )
            .await
            .unwrap();
        });
        let answer = agent_preauth_request(&mut relay, &AgentClientMessage::Status { v: 1 })
            .await
            .unwrap();
        assert!(matches!(answer, AgentServerMessage::Status { .. }));
        server.await.unwrap();
    });
}

#[test]
fn a_bad_first_message_is_refused_with_its_reason() {
    block_on(async {
        let (mut relay, mut desk) = duplex(1 << 16);
        let server = tokio::spawn(async move { agent_read_client_message(&mut desk).await });
        write_message(&mut relay, &serde_json::json!({"type": "status", "v": 2}))
            .await
            .unwrap();
        assert!(matches!(
            agent_read_server_message(&mut relay).await.unwrap(),
            AgentServerMessage::Refused {
                reason: AgentRefusal::ProtocolVersion,
                ..
            }
        ));
        assert_eq!(
            server.await.unwrap().map(|_| ()).unwrap_err(),
            Error::ProtocolVersion
        );
    });
}
