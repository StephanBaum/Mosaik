//! The protocol core over in-memory streams, with a scripted approver.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use signature::Verifier;
use ssh_key::PublicKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

use super::*;
use crate::approval::{ApprovalEvent, ApprovalPrompts, RequestExpired};
use crate::keys::KeyAlgorithm;
use crate::keys::SignFailure;
use crate::testkeys::{fixture, ids, key, material, openssh, s, strings};
use crate::wire::{RefusedRequest, msg};

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// What the scripted approver does with the next request.
#[derive(Clone, Copy, Debug)]
enum Script {
    Answer(Decision),
    /// Never answers.
    Hang,
}

#[derive(Default)]
struct Scripted {
    script: Mutex<VecDeque<Script>>,
    requests: Mutex<Vec<ApprovalRequest>>,
    closed: Mutex<Vec<(RequestId, CloseReason)>>,
    failed: Mutex<Vec<(RequestId, SignFailure)>>,
}

impl Scripted {
    fn new(script: &[Script]) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.iter().copied().collect()),
            ..Self::default()
        })
    }

    fn requests(&self) -> Vec<ApprovalRequest> {
        self.requests.lock().unwrap().clone()
    }

    fn closed(&self) -> Vec<CloseReason> {
        self.closed.lock().unwrap().iter().map(|c| c.1).collect()
    }

    fn failed(&self) -> Vec<SignFailure> {
        self.failed.lock().unwrap().iter().map(|c| c.1).collect()
    }
}

impl Approver for Scripted {
    fn approve(&self, request: ApprovalRequest) -> impl Future<Output = Decision> + Send {
        self.requests.lock().unwrap().push(request);
        let next = self.script.lock().unwrap().pop_front();
        async move {
            match next {
                Some(Script::Answer(d)) => d,
                Some(Script::Hang) => std::future::pending().await,
                None => panic!("unexpected approval request"),
            }
        }
    }

    fn closed(&self, id: RequestId, reason: CloseReason) {
        self.closed.lock().unwrap().push((id, reason));
    }

    fn sign_failed(&self, notice: crate::approval::SignFailed) {
        self.failed
            .lock()
            .unwrap()
            .push((notice.id, notice.failure));
    }
}

fn ssh_client(exe: &str) -> ClientInfo {
    ClientInfo {
        pid: Some(4242),
        process_name: Some("ssh".into()),
        executable: Some(PathBuf::from(exe)),
    }
}

fn keyset(names: &[&str]) -> KeySet {
    let mut set = KeySet::new();
    for n in names {
        assert!(set.insert(key(n)));
    }
    set
}

fn core_with(
    names: &[&str],
    approver: &Arc<Scripted>,
    config: AgentConfig,
) -> Arc<AgentCore<Scripted>> {
    Arc::new(AgentCore::new(keyset(names), Arc::clone(approver), config))
}

fn sign_body(blob: &[u8], data: &[u8], flags: u32) -> Vec<u8> {
    Request::Sign(SignRequest {
        key_blob: blob,
        data,
        flags,
    })
    .encode()
    .unwrap()
}

fn verify(blob: &[u8], data: &[u8], response: &[u8]) {
    let Response::Signature(sig) = Response::parse(response).unwrap() else {
        panic!("not a signature: {response:?}");
    };
    let public = PublicKey::from_bytes(blob).unwrap();
    let sig = ssh_key::Signature::try_from(sig).unwrap();
    Verifier::verify(&public, data, &sig).unwrap();
}

/// Starts `serve` on one end of a duplex pipe; returns the client end.
fn connect(
    core: &Arc<AgentCore<Scripted>>,
    client: ClientInfo,
) -> (DuplexStream, tokio::task::JoinHandle<io::Result<()>>) {
    let (a, b) = tokio::io::duplex(1 << 20);
    let core = Arc::clone(core);
    let task = tokio::spawn(async move { core.serve(b, client).await });
    (a, task)
}

async fn roundtrip(s: &mut DuplexStream, body: &[u8]) -> Vec<u8> {
    s.write_all(&wire::frame(body).unwrap()).await.unwrap();
    read_response(s).await.unwrap()
}

async fn read_response(s: &mut DuplexStream) -> io::Result<Vec<u8>> {
    let mut prefix = [0u8; 4];
    s.read_exact(&mut prefix).await?;
    let mut body = vec![0u8; wire::body_len(prefix).unwrap()];
    s.read_exact(&mut body).await?;
    Ok(body)
}

#[test]
fn identities_list_every_key_without_asking() {
    rt().block_on(async {
        let approver = Scripted::new(&[]);
        let core = core_with(
            &["ed25519", "ecdsa_p256"],
            &approver,
            AgentConfig::default(),
        );
        let (mut c, _task) = connect(&core, ClientInfo::UNKNOWN);
        let body = roundtrip(&mut c, &[msg::REQUEST_IDENTITIES]).await;
        let Response::Identities(ids) = Response::parse(&body).unwrap() else {
            panic!("not an identities answer");
        };
        let ed = key("ed25519");
        let p256 = key("ecdsa_p256");
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].key_blob, ed.public_key_blob());
        assert_eq!(ids[0].comment, b"mosaic-test-ed25519");
        assert_eq!(ids[1].key_blob, p256.public_key_blob());
        assert_eq!(approver.requests(), []);
        assert_eq!(core.key_count(), 2);
    });
}

#[test]
fn an_approved_request_is_signed_and_the_request_describes_it() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Answer(Decision::AllowOnce)]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let client = ssh_client("/usr/bin/ssh");
        let (mut c, _task) = connect(&core, client.clone());
        let k = key("ed25519");
        let before = SystemTime::now();
        let resp = roundtrip(&mut c, &sign_body(k.public_key_blob(), b"challenge", 0)).await;
        verify(k.public_key_blob(), b"challenge", &resp);
        let reqs = approver.requests();
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.fingerprint, k.fingerprint());
        assert_eq!(r.key_title.as_deref(), Some("ed25519"));
        assert_eq!(r.algorithm, KeyAlgorithm::Ed25519);
        assert_eq!(r.client, client);
        assert!(r.can_remember);
        assert!(r.requested_at >= before);
        assert_eq!(
            r.expires_at.duration_since(r.requested_at).unwrap(),
            Duration::from_secs(25)
        );
        assert_eq!(approver.closed(), [CloseReason::Answered]);
        assert_eq!(approver.closed.lock().unwrap()[0].0, r.id);
    });
}

#[test]
fn denied_unknown_and_refused_requests_fail() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Answer(Decision::Deny)]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let (mut c, _task) = connect(&core, ClientInfo::UNKNOWN);
        let k = key("ed25519");
        let fail = vec![msg::FAILURE];
        assert_eq!(
            roundtrip(&mut c, &sign_body(k.public_key_blob(), b"x", 0)).await,
            fail
        );
        assert_eq!(approver.closed(), [CloseReason::Answered]);
        // A key the agent does not hold: refused without asking.
        let other = key("ecdsa_p256");
        assert_eq!(
            roundtrip(&mut c, &sign_body(other.public_key_blob(), b"x", 0)).await,
            fail
        );
        // Key and lock management, extensions, unknown and malformed
        // messages: refused, and the connection stays usable.
        for r in [
            RefusedRequest::AddIdentity,
            RefusedRequest::RemoveIdentity,
            RefusedRequest::RemoveAllIdentities,
            RefusedRequest::AddSmartcardKey,
            RefusedRequest::RemoveSmartcardKey,
            RefusedRequest::Lock,
            RefusedRequest::Unlock,
        ] {
            let body = Request::Refused(r).encode().unwrap();
            assert_eq!(roundtrip(&mut c, &body).await, fail, "{r:?}");
        }
        let ext = Request::Extension {
            name: b"session-bind@openssh.com",
        }
        .encode()
        .unwrap();
        assert_eq!(roundtrip(&mut c, &ext).await, fail);
        assert_eq!(roundtrip(&mut c, &[200]).await, fail);
        assert_eq!(roundtrip(&mut c, &[msg::SIGN_REQUEST, 0, 0]).await, fail);
        assert_eq!(roundtrip(&mut c, &[msg::REQUEST_IDENTITIES, 1]).await, fail);
        assert_eq!(core.key_count(), 1, "nothing removed");
        assert_eq!(approver.requests().len(), 1, "asked once only");
        let body = roundtrip(&mut c, &[msg::REQUEST_IDENTITIES]).await;
        assert!(matches!(Response::parse(&body), Ok(Response::Identities(ids)) if ids.len() == 1));
    });
}

#[test]
fn an_unanswered_request_expires() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Hang]);
        let config = AgentConfig {
            approval_timeout: Duration::from_millis(50),
            ..AgentConfig::default()
        };
        let core = core_with(&["ed25519"], &approver, config);
        let (mut c, _task) = connect(&core, ClientInfo::UNKNOWN);
        let k = key("ed25519");
        let resp = roundtrip(&mut c, &sign_body(k.public_key_blob(), b"x", 0)).await;
        assert_eq!(resp, [msg::FAILURE]);
        assert_eq!(approver.closed(), [CloseReason::Expired]);
        let r = &approver.requests()[0];
        assert_eq!(
            r.expires_at.duration_since(r.requested_at).unwrap(),
            Duration::from_millis(50)
        );
    });
}

#[test]
fn stopping_refuses_open_approvals_and_drops_the_keys() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Hang]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        let k = key("ed25519");
        c.write_all(&wire::frame(&sign_body(k.public_key_blob(), b"x", 0)).unwrap())
            .await
            .unwrap();
        while approver.requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(!core.is_stopped());
        core.stop();
        assert!(core.is_stopped());
        // The connection closes without a signature.
        assert!(read_response(&mut c).await.is_err());
        task.await.unwrap().unwrap();
        assert_eq!(approver.closed(), [CloseReason::Locked]);
        assert_eq!(core.key_count(), 0);
        // Requests after the stop are refused at once.
        assert_eq!(
            core.handle(&[msg::REQUEST_IDENTITIES], &ClientInfo::UNKNOWN)
                .await,
            [msg::FAILURE]
        );
        // A connection opened after the stop ends at once.
        let (_c2, task2) = connect(&core, ClientInfo::UNKNOWN);
        task2.await.unwrap().unwrap();
    });
}

#[test]
fn grants_cover_the_same_key_and_executable_only() {
    rt().block_on(async {
        let approver = Scripted::new(&[
            Script::Answer(Decision::AllowFor(Duration::from_secs(60))),
            Script::Answer(Decision::AllowOnce),
            Script::Answer(Decision::AllowOnce),
            Script::Answer(Decision::AllowFor(DEFAULT_GRANT)),
            Script::Answer(Decision::AllowOnce),
            Script::Answer(Decision::AllowOnce),
        ]);
        let core = core_with(
            &["ed25519", "ecdsa_p256"],
            &approver,
            AgentConfig::default(),
        );
        let ed = key("ed25519");
        let p256 = key("ecdsa_p256");
        let ssh = ssh_client("/usr/bin/ssh");
        let git = ssh_client("/usr/bin/git");
        let sign = |k: &AgentKey, client: &ClientInfo| {
            let body = sign_body(k.public_key_blob(), b"x", 0);
            let core = Arc::clone(&core);
            let client = client.clone();
            async move { core.handle(&body, &client).await }
        };
        // 1: asked, granted for 60 s.
        verify(ed.public_key_blob(), b"x", &sign(&ed, &ssh).await);
        assert_eq!(approver.requests().len(), 1);
        // Same key, same executable (another PID): not asked.
        let mut again = ssh.clone();
        again.pid = Some(1);
        verify(ed.public_key_blob(), b"x", &sign(&ed, &again).await);
        assert_eq!(approver.requests().len(), 1);
        // 2: another executable is asked.
        verify(ed.public_key_blob(), b"x", &sign(&ed, &git).await);
        // 3: another key is asked.
        verify(p256.public_key_blob(), b"x", &sign(&p256, &ssh).await);
        assert_eq!(approver.requests().len(), 3);
        // Revoked: asked again (4), granted for 15 minutes.
        core.revoke_grants();
        verify(ed.public_key_blob(), b"x", &sign(&ed, &ssh).await);
        verify(ed.public_key_blob(), b"x", &sign(&ed, &ssh).await);
        assert_eq!(approver.requests().len(), 4);
        // A new key set keeps the grants of the keys it still serves...
        core.set_keys(keyset(&["ed25519"]));
        verify(ed.public_key_blob(), b"x", &sign(&ed, &ssh).await);
        assert_eq!(approver.requests().len(), 4);
        // ...and drops the others: a key that comes back is asked again (5).
        core.set_keys(keyset(&["ecdsa_p256"]));
        assert_eq!(
            core.handle(&sign_body(ed.public_key_blob(), b"x", 0), &ssh)
                .await,
            [msg::FAILURE]
        );
        core.set_keys(keyset(&["ed25519"]));
        verify(ed.public_key_blob(), b"x", &sign(&ed, &ssh).await);
        assert_eq!(approver.requests().len(), 5);
        // An unknown client is never remembered (6).
        verify(
            ed.public_key_blob(),
            b"x",
            &sign(&ed, &ClientInfo::UNKNOWN).await,
        );
        assert!(!approver.requests()[5].can_remember);
        assert_eq!(approver.closed().len(), 6);
    });
}

#[test]
fn unknown_clients_are_asked_every_time() {
    rt().block_on(async {
        let approver = Scripted::new(&[
            Script::Answer(Decision::AllowFor(DEFAULT_GRANT)),
            Script::Answer(Decision::AllowFor(Duration::from_secs(600))),
            Script::Answer(Decision::Deny),
        ]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let anon = ClientInfo::UNKNOWN;
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &anon).await);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &anon).await);
        assert_eq!(core.handle(&body, &anon).await, [msg::FAILURE]);
        assert_eq!(approver.requests().len(), 3);
        assert!(approver.requests().iter().all(|r| !r.can_remember));
    });
}

#[test]
fn grants_are_capped_and_expire() {
    rt().block_on(async {
        let approver = Scripted::new(&[
            Script::Answer(Decision::AllowFor(Duration::from_hours(1))),
            Script::Answer(Decision::AllowFor(Duration::MAX)),
        ]);
        // Long enough for a second request under an instrumented, loaded
        // test run; short enough to wait out.
        let cap = Duration::from_millis(1500);
        let config = AgentConfig {
            max_grant: cap,
            ..AgentConfig::default()
        };
        let core = core_with(&["ed25519"], &approver, config);
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let ssh = ssh_client("/usr/bin/ssh");
        // One hour, capped.
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        assert_eq!(approver.requests().len(), 1);
        tokio::time::sleep(cap + Duration::from_millis(200)).await;
        // Expired: asked again; `Duration::MAX` is capped as well.
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        assert_eq!(approver.requests().len(), 2);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        assert_eq!(approver.requests().len(), 2);
    });
}

#[test]
fn an_unrepresentable_grant_signs_once() {
    rt().block_on(async {
        let approver = Scripted::new(&[
            Script::Answer(Decision::AllowFor(Duration::MAX)),
            Script::Answer(Decision::AllowOnce),
        ]);
        let config = AgentConfig {
            max_grant: Duration::MAX,
            ..AgentConfig::default()
        };
        let core = core_with(&["ed25519"], &approver, config);
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let ssh = ssh_client("/usr/bin/ssh");
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        assert_eq!(approver.requests().len(), 2, "not remembered");
    });
}

#[test]
fn too_many_open_approvals_are_refused_without_asking() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Hang]);
        let config = AgentConfig {
            max_pending_approvals: 1,
            ..AgentConfig::default()
        };
        let core = core_with(&["ed25519"], &approver, config);
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let first = {
            let core = Arc::clone(&core);
            let body = body.clone();
            tokio::spawn(async move { core.handle(&body, &ClientInfo::UNKNOWN).await })
        };
        while approver.requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(
            core.handle(&body, &ClientInfo::UNKNOWN).await,
            [msg::FAILURE]
        );
        assert_eq!(approver.requests().len(), 1);
        core.stop();
        assert_eq!(first.await.unwrap(), [msg::FAILURE]);
        assert_eq!(approver.closed(), [CloseReason::Locked]);
    });
}

#[test]
fn failed_signing_is_reported() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Answer(Decision::AllowOnce)]);
        let (private, public) = fixture("ed25519_encrypted");
        let (v, i) = ids();
        let mut set = KeySet::new();
        set.insert(AgentKey::new(v, i, None, None, material(private, public), None).unwrap());
        let core = Arc::new(AgentCore::new(
            set,
            Arc::clone(&approver),
            AgentConfig::default(),
        ));
        let blob = key("ed25519_encrypted").public_key_blob().to_vec();
        let resp = core
            .handle(&sign_body(&blob, b"x", 0), &ClientInfo::UNKNOWN)
            .await;
        assert_eq!(resp, [msg::FAILURE]);
        assert_eq!(approver.failed(), [SignFailure::PassphraseRequired]);
    });
}

#[test]
fn malformed_frames_close_the_connection() {
    rt().block_on(async {
        let approver = Scripted::new(&[]);
        let config = AgentConfig {
            read_timeout: Duration::from_millis(50),
            ..AgentConfig::default()
        };
        let core = core_with(&["ed25519"], &approver, config);
        // Zero length.
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&[0, 0, 0, 0]).await.unwrap();
        task.await.unwrap().unwrap();
        assert!(read_response(&mut c).await.is_err());
        // Above the limit.
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&[0, 4, 0, 1]).await.unwrap();
        task.await.unwrap().unwrap();
        assert!(read_response(&mut c).await.is_err());
        // A body that never arrives.
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&[0, 0, 0, 9, 11]).await.unwrap();
        task.await.unwrap().unwrap();
        assert!(read_response(&mut c).await.is_err());
        // The client hangs up mid-body, or between messages.
        let (mut c, task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&[0, 0, 0, 9, 11]).await.unwrap();
        drop(c);
        task.await.unwrap().unwrap();
        let (c, task) = connect(&core, ClientInfo::UNKNOWN);
        drop(c);
        task.await.unwrap().unwrap();
        // A message split across writes still works.
        let (mut c, _task) = connect(&core, ClientInfo::UNKNOWN);
        c.write_all(&[0, 0]).await.unwrap();
        tokio::task::yield_now().await;
        c.write_all(&[0, 1]).await.unwrap();
        c.write_all(&[msg::REQUEST_IDENTITIES]).await.unwrap();
        let body = read_response(&mut c).await.unwrap();
        assert_eq!(body.first(), Some(&msg::IDENTITIES_ANSWER));
    });
}

#[test]
fn approval_prompts_bridge_events_and_answers() {
    rt().block_on(async {
        let events: Arc<Mutex<Vec<ApprovalEvent>>> = Arc::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let sink = Arc::clone(&events);
        let prompts = Arc::new(ApprovalPrompts::new(move |e: ApprovalEvent| {
            if let ApprovalEvent::Requested(r) = &e {
                let _ = tx.send(r.id);
            }
            sink.lock().unwrap().push(e);
        }));
        let config = AgentConfig {
            approval_timeout: Duration::from_millis(200),
            ..AgentConfig::default()
        };
        let core = Arc::new(AgentCore::new(
            keyset(&["ed25519"]),
            Arc::clone(&prompts),
            config,
        ));
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        // Answered through `respond`.
        let pending = {
            let core = Arc::clone(&core);
            let body = body.clone();
            tokio::spawn(async move { core.handle(&body, &ClientInfo::UNKNOWN).await })
        };
        let id = rx.recv().await.unwrap();
        assert_eq!(prompts.open_requests(), 1);
        prompts.respond(id, Decision::AllowOnce).unwrap();
        verify(ed.public_key_blob(), b"x", &pending.await.unwrap());
        assert_eq!(prompts.open_requests(), 0);
        assert_eq!(
            prompts.respond(id, Decision::AllowOnce),
            Err(RequestExpired),
            "answered already"
        );
        // Expired: a late answer is refused.
        let resp = core.handle(&body, &ClientInfo::UNKNOWN).await;
        assert_eq!(resp, [msg::FAILURE]);
        let late = rx.recv().await.unwrap();
        assert_eq!(
            prompts.respond(late, Decision::AllowOnce),
            Err(RequestExpired)
        );
        assert_eq!(prompts.open_requests(), 0);
        let log = events.lock().unwrap().clone();
        assert!(matches!(log[0], ApprovalEvent::Requested(_)));
        assert_eq!(
            log[1],
            ApprovalEvent::Closed {
                id,
                reason: CloseReason::Answered
            }
        );
        assert!(matches!(log[2], ApprovalEvent::Requested(_)));
        assert_eq!(
            log[3],
            ApprovalEvent::Closed {
                id: late,
                reason: CloseReason::Expired
            }
        );
        // A failure after approval is reported as an event.
        let (vault, item) = ids();
        let notice = crate::approval::SignFailed {
            id: late,
            vault,
            item,
            failure: SignFailure::WrongPassphrase,
        };
        prompts.sign_failed(notice);
        assert_eq!(
            events.lock().unwrap().last(),
            Some(&ApprovalEvent::SignFailed(notice))
        );
    });
}

#[test]
fn an_empty_key_set_lists_no_identities() {
    rt().block_on(async {
        let approver = Scripted::new(&[]);
        let core = core_with(&[], &approver, AgentConfig::default());
        let body = core
            .handle(&[msg::REQUEST_IDENTITIES], &ClientInfo::UNKNOWN)
            .await;
        assert_eq!(body, [msg::IDENTITIES_ANSWER, 0, 0, 0, 0]);
        assert_eq!(core.config().approval_timeout, Duration::from_secs(25));
    });
}

#[test]
fn an_abandoned_approval_is_closed() {
    rt().block_on(async {
        let approver = Scripted::new(&[Script::Hang]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let task = {
            let core = Arc::clone(&core);
            tokio::spawn(async move { core.handle(&body, &ClientInfo::UNKNOWN).await })
        };
        while approver.requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(approver.closed(), [CloseReason::Expired]);
        // The pending budget is released.
        assert_eq!(core.pending.load(Ordering::Acquire), 0);
    });
}

#[test]
fn set_keys_after_stop_keeps_nothing() {
    let approver = Scripted::new(&[]);
    let core = core_with(&["ed25519"], &approver, AgentConfig::default());
    core.stop();
    core.set_keys(keyset(&["ed25519", "ecdsa_p256"]));
    assert_eq!(core.key_count(), 0);
    rt().block_on(async {
        assert_eq!(
            core.handle(&[msg::REQUEST_IDENTITIES], &ClientInfo::UNKNOWN)
                .await,
            [msg::FAILURE]
        );
    });
}

#[test]
fn a_zero_or_unknown_client_grant_signs_once() {
    rt().block_on(async {
        let approver = Scripted::new(&[
            Script::Answer(Decision::AllowFor(Duration::ZERO)),
            Script::Answer(Decision::AllowOnce),
        ]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let ed = key("ed25519");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        let ssh = ssh_client("/usr/bin/ssh");
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        assert_eq!(approver.requests().len(), 2);
    });
}

#[test]
fn the_default_grant_is_15_minutes() {
    assert_eq!(AgentConfig::default().max_grant, DEFAULT_GRANT);
    assert_eq!(DEFAULT_GRANT, Duration::from_mins(15));
    rt().block_on(async {
        let approver =
            Scripted::new(&[Script::Answer(Decision::AllowFor(Duration::from_hours(8)))]);
        let core = core_with(&["ed25519"], &approver, AgentConfig::default());
        let ed = key("ed25519");
        let ssh = ssh_client("/usr/bin/ssh");
        let body = sign_body(ed.public_key_blob(), b"x", 0);
        verify(ed.public_key_blob(), b"x", &core.handle(&body, &ssh).await);
        let blob = ed.public_key_blob();
        let now = Instant::now();
        // Probe 899 s before 901 s: `allows` drops expired grants.
        assert!(
            core.grants()
                .allows(blob, &ssh, now + Duration::from_secs(899))
        );
        assert!(
            !core
                .grants()
                .allows(blob, &ssh, now + Duration::from_secs(901))
        );
    });
}

#[test]
fn rfc_vectors_through_the_agent() {
    use hex_literal::hex;
    use p256::elliptic_curve::sec1::ToSec1Point as _;
    use ssh_key::private::{EcdsaKeypair, Ed25519Keypair};

    /// Writes a framed request and reads the complete framed response.
    async fn exchange(c: &mut DuplexStream, body: &[u8]) -> Vec<u8> {
        c.write_all(&wire::frame(body).unwrap()).await.unwrap();
        let mut prefix = [0u8; 4];
        c.read_exact(&mut prefix).await.unwrap();
        let mut rest = vec![0u8; wire::body_len(prefix).unwrap()];
        c.read_exact(&mut rest).await.unwrap();
        [prefix.as_slice(), &rest].concat()
    }
    fn framed(body: &[u8]) -> Vec<u8> {
        wire::frame(body).unwrap()
    }

    // Ed25519: RFC 8032 section 7.1, test 2.
    let seed = hex!("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb");
    let ed_public = hex!("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
    let ed_sig = hex!(
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
    );
    let ed_pem = openssh(Ed25519Keypair::from_seed(&seed));
    // ECDSA P-256: RFC 6979 section A.2.5, message "sample".
    let x = hex!("C9AFA9D845BA75166B5C215767B1D6934E50C3DB36E89B127B8A622B120F6721");
    let secret_key = p256::SecretKey::from_slice(&x).unwrap();
    let p256_public = secret_key.public_key().to_sec1_point(false);
    let p256_pem = openssh(EcdsaKeypair::NistP256 {
        public: p256_public,
        private: secret_key.into(),
    });
    let r = [
        &[0u8][..],
        &hex!("EFD48B2AACB6A8FD1140DD9CD45E81D69D2C877B56AAF991C34D0EA84EAF3716"),
    ]
    .concat();
    let sv = [
        &[0u8][..],
        &hex!("F7CB1C942D657C41D436C7A1B6E29F65F3E900DBB9AFF4064DC4AB2F843ACDA8"),
    ]
    .concat();

    let (v1, i1) = ids();
    let (v2, i2) = ids();
    let mut set = KeySet::new();
    assert!(set.insert(AgentKey::new(v1, i1, None, None, material(&ed_pem, ""), None).unwrap()));
    assert!(set.insert(AgentKey::new(v2, i2, None, None, material(&p256_pem, ""), None).unwrap()));
    let ed_blob = strings(&[b"ssh-ed25519", &ed_public]);
    let p256_blob = key_blob_of(&set, 1);
    assert_eq!(key_blob_of(&set, 0), ed_blob);

    let approver = Scripted::new(&[
        Script::Answer(Decision::AllowOnce),
        Script::Answer(Decision::AllowOnce),
        Script::Answer(Decision::AllowOnce),
    ]);
    let core = Arc::new(AgentCore::new(
        set,
        Arc::clone(&approver),
        AgentConfig::default(),
    ));
    rt().block_on(async {
        let (mut c, _task) = connect(&core, ClientInfo::UNKNOWN);
        let ids_body = [
            [12u8].as_slice(),
            &[0, 0, 0, 2],
            &s(&ed_blob),
            &s(b"kat"),
            &s(&p256_blob),
            &s(b"kat"),
        ]
        .concat();
        assert_eq!(
            exchange(&mut c, &[msg::REQUEST_IDENTITIES]).await,
            framed(&ids_body)
        );
        let want = framed(&[[14u8].as_slice(), &s(&strings(&[b"ssh-ed25519", &ed_sig]))].concat());
        assert_eq!(
            exchange(&mut c, &sign_body(&ed_blob, &[0x72], 0)).await,
            want
        );
        let inner = strings(&[&r, &sv]);
        let want = framed(
            &[
                [14u8].as_slice(),
                &s(&strings(&[b"ecdsa-sha2-nistp256", &inner])),
            ]
            .concat(),
        );
        assert_eq!(
            exchange(&mut c, &sign_body(&p256_blob, b"sample", 0)).await,
            want
        );
        // RSA flags are ignored for non-RSA keys.
        assert_eq!(
            exchange(&mut c, &sign_body(&p256_blob, b"sample", 0x06)).await,
            want
        );
    });
}

fn key_blob_of(set: &KeySet, index: usize) -> Vec<u8> {
    set.iter().nth(index).unwrap().public_key_blob().to_vec()
}
