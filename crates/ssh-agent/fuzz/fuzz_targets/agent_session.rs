//! The protocol core on a fuzzed client byte stream.
//!
//! Input: `[decisions_len, decisions..., client_flag, stream...]`. The
//! stream is written to a connection served by `AgentCore::serve` (two
//! keys: the Ed25519 and P-256 test fixtures); each decision byte answers
//! one approval request (`% 3`: allow once, allow for a minute, deny; deny
//! once they run out). Bit 0 of `client_flag` gives the
//! client an executable, so grants apply.
//!
//! Oracles: `serve` ends cleanly; the agent answers exactly the complete
//! frames before the first bad length prefix, in order; every answer is a
//! valid response; an identities request gets both keys; a signature
//! answers only a well-formed sign request for a held key and verifies
//! over the requested data; key management, extensions, unknown and
//! malformed messages get `SSH_AGENT_FAILURE`; a denied request is never
//! signed unless a grant covered it.
#![no_main]

use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use libfuzzer_sys::fuzz_target;
use mosaic_core::model::{SshKeyMaterial, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_ssh_agent::wire::{self, Request, Response};
use mosaic_ssh_agent::{
    AgentConfig, AgentCore, AgentKey, ApprovalRequest, Approver, ClientInfo, Decision, KeySet,
};
use signature::Verifier;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const KEYS: [(&str, &str); 2] = [
    (
        include_str!("../../tests/keys/ed25519"),
        include_str!("../../tests/keys/ed25519.pub"),
    ),
    (
        include_str!("../../tests/keys/ecdsa_p256"),
        include_str!("../../tests/keys/ecdsa_p256.pub"),
    ),
];

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    })
}

fn keys() -> KeySet {
    let mut set = KeySet::new();
    for (private, public) in KEYS {
        let material = SshKeyMaterial::new(
            SecretString::from_str_copy(private).expect("secret"),
            Text::new(public),
            Text::new(""),
        );
        let key = AgentKey::new(
            VaultId::from_bytes([1; 16]).expect("id"),
            ItemId::from_bytes([2; 16]).expect("id"),
            None,
            None,
            material,
            None,
        )
        .expect("fixture key");
        assert!(set.insert(key));
    }
    set
}

#[derive(Default)]
struct Decisions {
    queue: Mutex<VecDeque<u8>>,
    asked: Mutex<Vec<Decision>>,
}

impl Approver for Decisions {
    fn approve(&self, _request: ApprovalRequest) -> impl Future<Output = Decision> + Send {
        let d = match self.queue.lock().unwrap().pop_front().map(|b| b % 3) {
            Some(0) => Decision::AllowOnce,
            Some(1) => Decision::AllowFor(Duration::from_secs(60)),
            _ => Decision::Deny,
        };
        self.asked.lock().unwrap().push(d);
        async move { d }
    }
}

/// The complete frames of `stream` up to the first bad length prefix.
fn frames(mut stream: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    while let Some((prefix, rest)) = stream.split_first_chunk::<4>() {
        let Ok(len) = wire::body_len(*prefix) else {
            break;
        };
        let Some((body, rest)) = rest.split_at_checked(len) else {
            break;
        };
        out.push(body);
        stream = rest;
    }
    out
}

fuzz_target!(|data: &[u8]| {
    let Some((&n, rest)) = data.split_first() else {
        return;
    };
    let Some((decisions, rest)) = rest.split_at_checked(usize::from(n)) else {
        return;
    };
    let Some((&client_flag, stream)) = rest.split_first() else {
        return;
    };
    let approver = Arc::new(Decisions {
        queue: Mutex::new(decisions.iter().copied().collect()),
        ..Decisions::default()
    });
    let client = ClientInfo {
        pid: Some(1),
        process_name: Some("ssh".into()),
        executable: (client_flag & 1 == 1).then(|| PathBuf::from("/usr/bin/ssh")),
    };
    let held: Vec<Vec<u8>> = keys().iter().map(|k| k.public_key_blob().to_vec()).collect();
    let core = Arc::new(AgentCore::new(
        keys(),
        Arc::clone(&approver),
        AgentConfig::default(),
    ));

    let responses = runtime().block_on(async {
        let (mut client_end, server_end) = tokio::io::duplex(1 << 22);
        let serving = {
            let core = Arc::clone(&core);
            tokio::spawn(async move { core.serve(server_end, client).await })
        };
        client_end.write_all(stream).await.expect("write");
        client_end.shutdown().await.expect("shutdown");
        let mut out = Vec::new();
        client_end.read_to_end(&mut out).await.expect("read");
        serving.await.expect("join").expect("serve");
        out
    });

    let requests = frames(stream);
    let answers = frames(&responses);
    assert_eq!(
        answers.len(),
        requests.len(),
        "one answer per complete frame"
    );
    let mut denials_unexplained = 0usize;
    for (req, ans) in requests.iter().zip(&answers) {
        let resp = Response::parse(ans).expect("valid response");
        match (Request::parse(req), resp) {
            (Ok(Request::RequestIdentities), Response::Identities(ids)) => {
                let blobs: Vec<&[u8]> = ids.iter().map(|i| i.key_blob).collect();
                assert_eq!(blobs, held.iter().map(Vec::as_slice).collect::<Vec<_>>());
            }
            (Ok(Request::Sign(s)), Response::Signature(sig)) => {
                assert!(held.iter().any(|b| b == s.key_blob), "signed with an unknown key");
                let public = ssh_key::PublicKey::from_bytes(s.key_blob).expect("key");
                let sig = ssh_key::Signature::try_from(sig).expect("signature blob");
                Verifier::verify(&public, s.data, &sig).expect("signature verifies");
            }
            (Ok(Request::Sign(s)), Response::Failure) => {
                // Only an unknown key or a denial may fail.
                if held.iter().any(|b| b == s.key_blob) {
                    denials_unexplained += 1;
                }
            }
            (Ok(Request::RequestIdentities) | Ok(Request::Sign(_)), other) => {
                panic!("unexpected answer {other:?}")
            }
            (_, Response::Failure) => {}
            (_, other) => panic!("non-failure answer {other:?} to a refused request"),
        }
    }
    let denied = approver
        .asked
        .lock()
        .unwrap()
        .iter()
        .filter(|d| **d == Decision::Deny)
        .count();
    assert_eq!(denials_unexplained, denied, "every failed sign was a denial");
});
