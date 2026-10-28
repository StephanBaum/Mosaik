//! The agent on a real OS endpoint (a named pipe on Windows, a Unix socket
//! elsewhere): list the keys, sign a challenge after approval, verify the
//! signature with ssh-key, and the endpoint rules.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mosaic_core::model::{SshKeyMaterial, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_ssh_agent::wire::{self, Request, Response, SignRequest, msg};
use mosaic_ssh_agent::{
    AgentConfig, AgentHandle, AgentKey, ApprovalEvent, ApprovalPrompts, CloseReason, Decision,
    Endpoint, Error, KeySet,
};
use signature::Verifier;
use ssh_key::PublicKey;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const ED25519: (&str, &str) = (
    include_str!("keys/ed25519"),
    include_str!("keys/ed25519.pub"),
);
const P256_ENCRYPTED: (&str, &str) = (
    include_str!("keys/ecdsa_p256_encrypted"),
    include_str!("keys/ecdsa_p256_encrypted.pub"),
);

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn agent_key((private, public): (&str, &str), passphrase: Option<&str>) -> AgentKey {
    let material = SshKeyMaterial::new(
        SecretString::from_str_copy(private).unwrap(),
        Text::new(public),
        Text::new(""),
    );
    AgentKey::new(
        VaultId::new_v7().unwrap(),
        ItemId::new_v7().unwrap(),
        Some("Deploy key"),
        None,
        material,
        passphrase.map(|p| SecretString::from_str_copy(p).unwrap()),
    )
    .unwrap()
}

fn keys() -> KeySet {
    let mut set = KeySet::new();
    set.insert(agent_key(ED25519, None));
    set.insert(agent_key(
        P256_ENCRYPTED,
        Some("correct horse battery staple"),
    ));
    set
}

/// A fresh endpoint per test.
struct TestEndpoint {
    endpoint: Endpoint,
    // Keeps the socket directory alive on Unix.
    _dir: Option<tempfile::TempDir>,
}

fn test_endpoint() -> TestEndpoint {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    if cfg!(windows) {
        TestEndpoint {
            endpoint: Endpoint::NamedPipe(format!(
                r"\\.\pipe\mosaic-ssh-agent-test-{}-{n}",
                std::process::id()
            )),
            _dir: None,
        }
    } else {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::platform_default(&dir.path().join("agent"));
        TestEndpoint {
            endpoint,
            _dir: Some(dir),
        }
    }
}

#[cfg(windows)]
#[allow(clippy::unused_async, reason = "same signature as the Unix version")]
async fn connect(
    endpoint: &Endpoint,
) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    let Endpoint::NamedPipe(name) = endpoint else {
        unreachable!()
    };
    tokio::net::windows::named_pipe::ClientOptions::new().open(name)
}

#[cfg(unix)]
async fn connect(endpoint: &Endpoint) -> std::io::Result<tokio::net::UnixStream> {
    let Endpoint::UnixSocket(path) = endpoint else {
        unreachable!()
    };
    tokio::net::UnixStream::connect(path).await
}

async fn roundtrip<S: AsyncRead + AsyncWrite + Unpin>(
    s: &mut S,
    body: &[u8],
) -> std::io::Result<Vec<u8>> {
    s.write_all(&wire::frame(body).unwrap()).await?;
    let mut prefix = [0u8; 4];
    s.read_exact(&mut prefix).await?;
    let mut out = vec![0u8; wire::body_len(prefix).unwrap()];
    s.read_exact(&mut out).await?;
    Ok(out)
}

type Events = Arc<Mutex<Vec<ApprovalEvent>>>;
type Prompts = Arc<ApprovalPrompts<Box<dyn Fn(ApprovalEvent) + Send + Sync>>>;
type Requests = tokio::sync::mpsc::UnboundedReceiver<mosaic_ssh_agent::RequestId>;

/// Prompts that report every event to the log and every request ID to the
/// returned channel; tests answer through `respond`.
fn prompts() -> (Prompts, Events, Requests) {
    let events: Events = Arc::default();
    let sink = Arc::clone(&events);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let notify: Box<dyn Fn(ApprovalEvent) + Send + Sync> = Box::new(move |e: ApprovalEvent| {
        if let ApprovalEvent::Requested(r) = &e {
            let _ = tx.send(r.id);
        }
        sink.lock().unwrap().push(e);
    });
    (Arc::new(ApprovalPrompts::new(notify)), events, rx)
}

#[test]
fn signs_a_challenge_over_the_os_endpoint() {
    rt().block_on(async {
        let ep = test_endpoint();
        let (prompts, events, mut requests) = prompts();
        let agent = AgentHandle::start(
            ep.endpoint.clone(),
            keys(),
            Arc::clone(&prompts),
            AgentConfig::default(),
        )
        .unwrap();
        assert_eq!(agent.endpoint(), &ep.endpoint);
        assert_eq!(agent.endpoint().to_string(), ep.endpoint.auth_sock());
        assert_eq!(agent.core().key_count(), 2);

        let mut client = connect(&ep.endpoint).await.unwrap();
        let body = roundtrip(&mut client, &[msg::REQUEST_IDENTITIES])
            .await
            .unwrap();
        let Response::Identities(ids) = Response::parse(&body).unwrap() else {
            panic!("no identities answer");
        };
        assert_eq!(ids.len(), 2);
        let blobs: Vec<Vec<u8>> = ids.iter().map(|i| i.key_blob.to_vec()).collect();

        // The user approves each request from another task.
        let answering = Arc::clone(&prompts);
        let answers = tokio::spawn(async move {
            for _ in 0..2 {
                let id = requests.recv().await.unwrap();
                answering.respond(id, Decision::AllowOnce).unwrap();
            }
        });
        let challenge = b"a challenge from the server";
        for blob in &blobs {
            let req = Request::Sign(SignRequest {
                key_blob: blob,
                data: challenge,
                flags: 0,
            })
            .encode()
            .unwrap();
            let body = roundtrip(&mut client, &req).await.unwrap();
            let Response::Signature(sig) = Response::parse(&body).unwrap() else {
                panic!("not signed: {body:?}");
            };
            let public = PublicKey::from_bytes(blob).unwrap();
            let sig = ssh_key::Signature::try_from(sig).unwrap();
            Verifier::verify(&public, challenge, &sig).unwrap();
            // A signature over other data does not verify.
            assert!(Verifier::verify(&public, b"other data", &sig).is_err());
        }
        answers.await.unwrap();
        let log = events.lock().unwrap().clone();
        assert_eq!(log.len(), 4);
        assert!(matches!(&log[0], ApprovalEvent::Requested(r) if r.key_title.as_deref() == Some("Deploy key")));
        assert!(matches!(
            log[1],
            ApprovalEvent::Closed {
                reason: CloseReason::Answered,
                ..
            }
        ));

        agent.stop().await;
        // The endpoint is gone and the connection closed.
        assert!(roundtrip(&mut client, &[msg::REQUEST_IDENTITIES]).await.is_err());
        assert!(connect(&ep.endpoint).await.is_err());
        if let Endpoint::UnixSocket(path) = &ep.endpoint {
            assert!(!path.exists(), "socket removed");
        }
    });
}

#[test]
fn stopping_closes_open_approvals_and_connections() {
    rt().block_on(async {
        let ep = test_endpoint();
        let (prompts, events, mut requests) = prompts();
        let agent = AgentHandle::start(
            ep.endpoint.clone(),
            keys(),
            Arc::clone(&prompts),
            AgentConfig::default(),
        )
        .unwrap();
        let mut client = connect(&ep.endpoint).await.unwrap();
        let blob = agent_key(ED25519, None).public_key_blob().to_vec();
        let req = Request::Sign(SignRequest {
            key_blob: &blob,
            data: b"x",
            flags: 0,
        })
        .encode()
        .unwrap();
        let pending = tokio::spawn(async move { roundtrip(&mut client, &req).await });
        let id = requests.recv().await.unwrap();
        agent.stop().await;
        assert!(pending.await.unwrap().is_err(), "closed without a response");
        assert_eq!(
            prompts.respond(id, Decision::AllowOnce),
            Err(mosaic_ssh_agent::RequestExpired)
        );
        assert_eq!(
            events.lock().unwrap().last(),
            Some(&ApprovalEvent::Closed {
                id,
                reason: CloseReason::Locked
            })
        );
    });
}

#[test]
fn dropping_the_handle_stops_the_agent() {
    rt().block_on(async {
        let ep = test_endpoint();
        let (prompts, _events, _requests) = prompts();
        let agent =
            AgentHandle::start(ep.endpoint.clone(), keys(), prompts, AgentConfig::default())
                .unwrap();
        let core_keys = agent.core().key_count();
        assert_eq!(core_keys, 2);
        drop(agent);
        // The accept loop exits on its own; the endpoint disappears.
        let mut gone = false;
        for _ in 0..100 {
            if connect(&ep.endpoint).await.is_err() {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(gone);
    });
}

#[test]
fn an_endpoint_serves_one_agent() {
    rt().block_on(async {
        let ep = test_endpoint();
        let (prompts, _events, _requests) = prompts();
        let first = AgentHandle::start(
            ep.endpoint.clone(),
            keys(),
            Arc::clone(&prompts),
            AgentConfig::default(),
        )
        .unwrap();
        let second = AgentHandle::start(
            ep.endpoint.clone(),
            KeySet::new(),
            Arc::clone(&prompts),
            AgentConfig::default(),
        );
        assert_eq!(second.err(), Some(Error::EndpointInUse));
        // The first one still serves.
        let mut client = connect(&ep.endpoint).await.unwrap();
        let body = roundtrip(&mut client, &[msg::REQUEST_IDENTITIES])
            .await
            .unwrap();
        assert!(matches!(Response::parse(&body), Ok(Response::Identities(ids)) if ids.len() == 2));
        first.stop().await;
        // Free again after the stop.
        let again =
            AgentHandle::start(ep.endpoint.clone(), keys(), prompts, AgentConfig::default())
                .unwrap();
        again.stop().await;
    });
}

#[test]
fn connections_above_the_limit_are_closed() {
    rt().block_on(async {
        let ep = test_endpoint();
        let (prompts, _events, _requests) = prompts();
        let mut config = AgentConfig::default();
        config.max_connections = 1;
        let agent = AgentHandle::start(ep.endpoint.clone(), keys(), prompts, config).unwrap();
        let mut first = connect(&ep.endpoint).await.unwrap();
        roundtrip(&mut first, &[msg::REQUEST_IDENTITIES])
            .await
            .unwrap();
        let mut second = connect(&ep.endpoint).await.unwrap();
        assert!(
            roundtrip(&mut second, &[msg::REQUEST_IDENTITIES])
                .await
                .is_err()
        );
        drop(first);
        // Once the first connection is gone, a new one is served.
        let mut served = false;
        for _ in 0..100 {
            let mut c = connect(&ep.endpoint).await.unwrap();
            if roundtrip(&mut c, &[msg::REQUEST_IDENTITIES]).await.is_ok() {
                served = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(served);
        agent.stop().await;
    });
}

#[test]
fn endpoints_of_another_platform_or_shape_are_invalid() {
    rt().block_on(async {
        let (prompts, _events, _requests) = prompts();
        let foreign = if cfg!(windows) {
            vec![
                Endpoint::UnixSocket(std::env::temp_dir().join("agent.sock")),
                Endpoint::NamedPipe("openssh-ssh-agent".into()),
                Endpoint::NamedPipe(r"\\.\pipe\".into()),
                Endpoint::NamedPipe(r"\\.\pipe\a\b".into()),
                Endpoint::NamedPipe(r"\\remote\pipe\x".into()),
            ]
        } else {
            vec![
                Endpoint::NamedPipe(Endpoint::WINDOWS_OPENSSH_PIPE.into()),
                Endpoint::UnixSocket("relative/agent.sock".into()),
                Endpoint::UnixSocket(std::env::temp_dir().join("x".repeat(120))),
            ]
        };
        for ep in foreign {
            let r = AgentHandle::start(
                ep.clone(),
                KeySet::new(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            );
            assert_eq!(r.err(), Some(Error::EndpointInvalid), "{ep:?}");
        }
    });
}

#[test]
fn starting_outside_a_runtime_fails() {
    let (prompts, _events, _requests) = {
        let rt = rt();
        rt.block_on(async { prompts() })
    };
    let ep = test_endpoint();
    let r = AgentHandle::start(ep.endpoint, KeySet::new(), prompts, AgentConfig::default());
    assert_eq!(r.err(), Some(Error::NoRuntime));
}

#[test]
fn the_platform_default_endpoint() {
    let dir = std::path::Path::new("/run/user/1000/mosaic");
    let ep = Endpoint::platform_default(dir);
    if cfg!(windows) {
        assert_eq!(
            ep,
            Endpoint::NamedPipe(Endpoint::WINDOWS_OPENSSH_PIPE.into())
        );
        assert_eq!(ep.auth_sock(), r"\\.\pipe\openssh-ssh-agent");
    } else {
        assert_eq!(ep, Endpoint::UnixSocket(dir.join("agent.sock")));
        assert_eq!(ep.auth_sock(), "/run/user/1000/mosaic/agent.sock");
    }
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn the_socket_directory_must_be_private() {
        rt().block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let shared = dir.path().join("shared");
            fs::create_dir(&shared).unwrap();
            fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
            let (prompts, _events, _requests) = prompts();
            let r = AgentHandle::start(
                Endpoint::UnixSocket(shared.join("agent.sock")),
                KeySet::new(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            );
            assert_eq!(r.err(), Some(Error::EndpointInsecure));
            // A symbolic link to a private directory is refused as well.
            let private = dir.path().join("private");
            fs::create_dir(&private).unwrap();
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&private, &link).unwrap();
            let r = AgentHandle::start(
                Endpoint::UnixSocket(link.join("agent.sock")),
                KeySet::new(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            );
            assert_eq!(r.err(), Some(Error::EndpointInsecure));
            // A missing directory is created with mode 0700.
            let fresh = dir.path().join("fresh");
            let agent = AgentHandle::start(
                Endpoint::UnixSocket(fresh.join("agent.sock")),
                KeySet::new(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            )
            .unwrap();
            let mode = fs::metadata(&fresh).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
            let sock_mode = fs::metadata(fresh.join("agent.sock"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(sock_mode & 0o777, 0o600);
            agent.stop().await;
        });
    }

    #[test]
    fn other_files_are_never_replaced_but_stale_sockets_are() {
        rt().block_on(async {
            let ep = test_endpoint();
            let Endpoint::UnixSocket(path) = &ep.endpoint else {
                unreachable!()
            };
            let dir = path.parent().unwrap();
            fs::create_dir(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(path, b"not a socket").unwrap();
            let (prompts, _events, _requests) = prompts();
            let r = AgentHandle::start(
                ep.endpoint.clone(),
                KeySet::new(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            );
            assert_eq!(r.err(), Some(Error::EndpointInUse));
            assert_eq!(fs::read(path).unwrap(), b"not a socket");
            fs::remove_file(path).unwrap();
            // A socket nobody listens on is replaced.
            let stale = std::os::unix::net::UnixListener::bind(path).unwrap();
            drop(stale);
            assert!(path.exists());
            let agent = AgentHandle::start(
                ep.endpoint.clone(),
                keys(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            )
            .unwrap();
            let mut c = connect(&ep.endpoint).await.unwrap();
            assert!(roundtrip(&mut c, &[msg::REQUEST_IDENTITIES]).await.is_ok());
            agent.stop().await;
            assert!(!path.exists());
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_clients_are_identified_by_executable() {
        rt().block_on(async {
            let ep = test_endpoint();
            let (prompts, events, mut requests) = prompts();
            let agent = AgentHandle::start(
                ep.endpoint.clone(),
                keys(),
                Arc::clone(&prompts),
                AgentConfig::default(),
            )
            .unwrap();
            let mut client = connect(&ep.endpoint).await.unwrap();
            let blob = agent_key(ED25519, None).public_key_blob().to_vec();
            let req = Request::Sign(SignRequest {
                key_blob: &blob,
                data: b"x",
                flags: 0,
            })
            .encode()
            .unwrap();
            let answering = Arc::clone(&prompts);
            tokio::spawn(async move {
                let id = requests.recv().await.unwrap();
                answering
                    .respond(id, Decision::AllowFor(mosaic_ssh_agent::DEFAULT_GRANT))
                    .unwrap();
            });
            roundtrip(&mut client, &req).await.unwrap();
            let ApprovalEvent::Requested(r) = events.lock().unwrap()[0].clone() else {
                panic!()
            };
            assert_eq!(r.client.pid, Some(std::process::id()));
            assert_eq!(
                r.client.executable.as_deref(),
                Some(std::env::current_exe().unwrap().as_path())
            );
            assert!(r.can_remember);
            // Granted until lock: the second request is not asked.
            let body = roundtrip(&mut client, &req).await.unwrap();
            assert_eq!(body.first(), Some(&msg::SIGN_RESPONSE));
            assert_eq!(events.lock().unwrap().len(), 2);
            agent.stop().await;
        });
    }
}
