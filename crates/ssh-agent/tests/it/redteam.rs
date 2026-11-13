//! Red team of the agent protocol (RT-parsers, 2026-11-13), over in-memory
//! streams against the public core. Tests marked `ignore` demonstrate an
//! open finding; the others are regression tests of attacks that held.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mosaic_core::model::{SshKeyMaterial, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_ssh_agent::wire::{self, MAX_MESSAGE_LEN, Request, Response, SignRequest, msg};
use mosaic_ssh_agent::{
    AgentConfig, AgentCore, AgentKey, ApprovalRequest, Approver, ClientInfo, CloseReason, Decision,
    KeySet,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const ED25519: (&str, &str) = (
    include_str!("../keys/ed25519"),
    include_str!("../keys/ed25519.pub"),
);

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn agent_key() -> AgentKey {
    let material = SshKeyMaterial::new(
        SecretString::from_str_copy(ED25519.0).unwrap(),
        Text::new(ED25519.1),
        Text::new(""),
    );
    AgentKey::new(
        VaultId::new_v7().unwrap(),
        ItemId::new_v7().unwrap(),
        Some("Deploy key"),
        None,
        material,
        None,
    )
    .unwrap()
}

/// Answers from a script; an empty script denies (and records the request).
#[derive(Default)]
struct Scripted {
    script: Mutex<VecDeque<Decision>>,
    requests: Mutex<Vec<ApprovalRequest>>,
    closed: Mutex<Vec<CloseReason>>,
}

impl Scripted {
    fn new(script: &[Decision]) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.iter().copied().collect()),
            ..Self::default()
        })
    }

    fn asked(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Approver for Scripted {
    fn approve(&self, request: ApprovalRequest) -> impl Future<Output = Decision> + Send {
        self.requests.lock().unwrap().push(request);
        let next = self.script.lock().unwrap().pop_front();
        async move { next.unwrap_or(Decision::Deny) }
    }

    fn closed(&self, _id: mosaic_ssh_agent::RequestId, reason: CloseReason) {
        self.closed.lock().unwrap().push(reason);
    }
}

fn ssh_client(exe: &str) -> ClientInfo {
    ClientInfo {
        pid: Some(4242),
        process_name: Some("ssh".into()),
        executable: Some(PathBuf::from(exe)),
    }
}

fn core(approver: &Arc<Scripted>, config: AgentConfig) -> Arc<AgentCore<Scripted>> {
    let mut keys = KeySet::new();
    assert!(keys.insert(agent_key()));
    Arc::new(AgentCore::new(keys, Arc::clone(approver), config))
}

fn connect(
    core: &Arc<AgentCore<Scripted>>,
    client: ClientInfo,
) -> (DuplexStream, tokio::task::JoinHandle<std::io::Result<()>>) {
    let (a, b) = tokio::io::duplex(1 << 20);
    let core = Arc::clone(core);
    let task = tokio::spawn(async move { core.serve(b, client).await });
    (a, task)
}

async fn roundtrip(s: &mut DuplexStream, body: &[u8]) -> Vec<u8> {
    s.write_all(&wire::frame(body).unwrap()).await.unwrap();
    let mut prefix = [0u8; 4];
    s.read_exact(&mut prefix).await.unwrap();
    let mut out = vec![0u8; wire::body_len(prefix).unwrap()];
    s.read_exact(&mut out).await.unwrap();
    out
}

fn sign_body(blob: &[u8], data: &[u8]) -> Vec<u8> {
    Request::Sign(SignRequest {
        key_blob: blob,
        data,
        flags: 0,
    })
    .encode()
    .unwrap()
}

fn is_signature(body: &[u8]) -> bool {
    matches!(Response::parse(body), Ok(Response::Signature(_)))
}

fn string(out: &mut Vec<u8>, s: &[u8]) {
    out.extend_from_slice(&u32::try_from(s.len()).unwrap().to_be_bytes());
    out.extend_from_slice(s);
}

/// `session-bind@openssh.com` (OpenSSH PROTOCOL.agent §3.3): the client tells
/// the agent which host it is talking to and whether this connection is a
/// forwarded one. Mosaic refuses every extension, so this one too.
fn session_bind(forwarding: bool) -> Vec<u8> {
    let mut body = vec![msg::EXTENSION];
    string(&mut body, b"session-bind@openssh.com");
    string(
        &mut body,
        b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20hostkey-of-the-remote-host!!!!!!",
    );
    string(&mut body, &[0x5a; 32]);
    string(&mut body, b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x40sig");
    body.push(u8::from(forwarding));
    body
}

// ---------------------------------------------------------------------------
// RT-parsers-4: a 15-minute grant is bound to the public key and the client
// executable only. With `ForwardAgent yes` the requests a compromised remote
// host sends arrive through the same local `/usr/bin/ssh` process, so the
// grant the user gave for their own login covers them too: the remote host
// signs (authenticates as the user to any other server, signs a certificate
// with a CA key, a git commit) without a prompt for 15 minutes. OpenSSH's
// agent gets the hop from `session-bind@openssh.com` (and refuses forwarded
// use of a key with destination constraints); Mosaic answers the extension
// with `SSH_AGENT_FAILURE` and cannot tell a forwarded request apart.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "RT-parsers-4: a 15-minute grant to /usr/bin/ssh also covers requests forwarded through that ssh from a remote host (session-bind is refused, so the hop is invisible)"]
fn rt_parsers_4_a_forwarded_connection_does_not_reuse_a_local_grant() {
    rt().block_on(async {
        let approver = Scripted::new(&[Decision::AllowFor(Duration::from_mins(15))]);
        let core = core(&approver, AgentConfig::default());
        let blob = agent_key().public_key_blob().to_vec();
        let client = ssh_client("/usr/bin/ssh");

        // The user's own login: approved, remembered for 15 minutes.
        let (mut local, _t1) = connect(&core, client.clone());
        let _ = roundtrip(&mut local, &session_bind(false)).await;
        assert!(is_signature(
            &roundtrip(&mut local, &sign_body(&blob, b"userauth to my-server")).await
        ));
        assert_eq!(approver.asked(), 1);

        // A second connection through the same ssh, bound as a forwarded hop
        // (what sshd on the remote host drives): must not ride on the grant.
        let (mut forwarded, _t2) = connect(&core, client);
        let _ = roundtrip(&mut forwarded, &session_bind(true)).await;
        let resp = roundtrip(
            &mut forwarded,
            &sign_body(&blob, b"userauth to the victim's other server"),
        )
        .await;
        assert!(
            !is_signature(&resp) || approver.asked() == 2,
            "the forwarded request was signed under the local grant without asking"
        );
    });
}

// ---------------------------------------------------------------------------
// Attacks that held.
// ---------------------------------------------------------------------------

/// A grant for `/usr/bin/ssh` does not cover `ssh-keygen` (certificate
/// signing, `ssh-keygen -Y sign`) or an unknown client.
#[test]
fn a_grant_is_bound_to_the_client_executable() {
    rt().block_on(async {
        let approver = Scripted::new(&[Decision::AllowFor(Duration::from_mins(15))]);
        let core = core(&approver, AgentConfig::default());
        let blob = agent_key().public_key_blob().to_vec();
        let (mut ssh, _t1) = connect(&core, ssh_client("/usr/bin/ssh"));
        assert!(is_signature(
            &roundtrip(&mut ssh, &sign_body(&blob, b"login")).await
        ));
        for client in [ssh_client("/usr/bin/ssh-keygen"), ClientInfo::UNKNOWN] {
            let (mut other, _t) = connect(&core, client);
            let resp = roundtrip(&mut other, &sign_body(&blob, b"a certificate")).await;
            assert_eq!(resp, [msg::FAILURE], "denied: the script is spent");
        }
        assert_eq!(approver.asked(), 3);
    });
}

/// Framing: a length prefix above 256 KiB, or zero, ends the connection
/// before any body is read; a body that never arrives times out.
#[test]
fn oversized_and_stalled_frames_end_the_connection() {
    rt().block_on(async {
        let approver = Scripted::new(&[]);
        let mut config = AgentConfig::default();
        config.read_timeout = Duration::from_millis(100);
        let core = core(&approver, config);
        for prefix in [
            (u32::try_from(MAX_MESSAGE_LEN).unwrap() + 1).to_be_bytes(),
            u32::MAX.to_be_bytes(),
            0u32.to_be_bytes(),
        ] {
            let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
            c.write_all(&prefix).await.unwrap();
            assert!(task.await.unwrap().is_ok(), "closed cleanly for {prefix:?}");
            let mut buf = [0u8; 1];
            assert_eq!(c.read(&mut buf).await.unwrap(), 0, "nothing answered");
        }
        // A declared body that never completes.
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&16u32.to_be_bytes()).await.unwrap();
        c.write_all(&[msg::REQUEST_IDENTITIES]).await.unwrap();
        assert!(task.await.unwrap().is_ok());
        assert_eq!(approver.asked(), 0);
    });
}

/// Every request a client can send is answered with the failure byte or a
/// well-formed response; nothing panics and nothing is signed without an
/// approval (the script is empty: every ask is a deny).
#[test]
fn arbitrary_messages_never_sign_without_approval() {
    use proptest::prelude::*;
    use proptest::test_runner::{Config, TestRunner};

    // The cases are drawn first (proptest's runner is synchronous), then
    // driven over one connection: the raw bytes as a message, and the same
    // bytes as the data of a sign request for the real key.
    let bodies: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
    let mut runner = TestRunner::new(Config {
        cases: 128,
        ..Config::default()
    });
    runner
        .run(&proptest::collection::vec(any::<u8>(), 0..300), |bytes| {
            bodies.lock().unwrap().push(bytes);
            Ok(())
        })
        .unwrap();
    let bodies = bodies.into_inner().unwrap();
    let blob = agent_key().public_key_blob().to_vec();
    rt().block_on(async {
        let approver = Scripted::new(&[]);
        let core = core(&approver, AgentConfig::default());
        let (mut c, _t) = connect(&core, ssh_client("/usr/bin/ssh"));
        for bytes in bodies {
            for body in [sign_body(&blob, &bytes), bytes] {
                if body.is_empty() {
                    continue;
                }
                let resp = roundtrip(&mut c, &body).await;
                assert!(!is_signature(&resp), "signed {body:?} without approval");
                assert!(Response::parse(&resp).is_ok() || resp == [msg::FAILURE]);
            }
        }
        assert!(
            approver.asked() > 0,
            "the sign requests reached the approver"
        );
        assert_eq!(core.key_count(), 1, "nothing added or removed");
    });
}
