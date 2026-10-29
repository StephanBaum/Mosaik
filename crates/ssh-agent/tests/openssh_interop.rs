//! Interoperability with the real OpenSSH tools: `ssh-add -L` lists the
//! agent's keys and `ssh-keygen -Y sign` signs through the agent; the
//! signature is checked by `ssh-keygen -Y check-novalidate` and by ssh-key.
//!
//! Ignored by default: it needs OpenSSH 8.2 or newer on the machine. Run
//! with `cargo test -p mosaic-ssh-agent --test openssh_interop -- --ignored`.
//! The tools are taken from `MOSAIC_OPENSSH_BIN` if set, else from
//! `C:\Windows\System32\OpenSSH` on Windows and from `PATH` elsewhere.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use mosaic_core::model::{SshKeyMaterial, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_ssh_agent::{
    AgentConfig, AgentHandle, AgentKey, ApprovalEvent, ApprovalPrompts, Decision, Endpoint, KeySet,
};
use ssh_key::{PublicKey, SshSig};

const KEYS: [(&str, &str, &str, Option<&str>); 2] = [
    (
        "ed25519",
        include_str!("keys/ed25519"),
        include_str!("keys/ed25519.pub"),
        None,
    ),
    (
        "ecdsa_p256_encrypted",
        include_str!("keys/ecdsa_p256_encrypted"),
        include_str!("keys/ecdsa_p256_encrypted.pub"),
        Some("correct horse battery staple"),
    ),
];

fn tool(name: &str) -> PathBuf {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    match std::env::var_os("MOSAIC_OPENSSH_BIN") {
        Some(dir) => Path::new(&dir).join(exe),
        None if cfg!(windows) => Path::new(r"C:\Windows\System32\OpenSSH").join(exe),
        None => PathBuf::from(exe),
    }
}

fn run(cmd: &mut Command, sock: &str) -> Output {
    let out = cmd.env("SSH_AUTH_SOCK", sock).output().unwrap();
    assert!(
        out.status.success(),
        "{cmd:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

#[test]
#[ignore = "needs OpenSSH 8.2+ on the machine"]
fn openssh_lists_and_signs_through_the_agent() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let endpoint = if cfg!(windows) {
        Endpoint::NamedPipe(format!(
            r"\\.\pipe\mosaic-ssh-agent-interop-{}",
            std::process::id()
        ))
    } else {
        Endpoint::UnixSocket(dir.path().join("agent").join("agent.sock"))
    };
    let mut set = KeySet::new();
    for (_, private, public, pass) in KEYS {
        let material = SshKeyMaterial::new(
            SecretString::from_str_copy(private).unwrap(),
            Text::new(public),
            Text::new(""),
        );
        set.insert(
            AgentKey::new(
                VaultId::new_v7().unwrap(),
                ItemId::new_v7().unwrap(),
                None,
                None,
                material,
                pass.map(|p| SecretString::from_str_copy(p).unwrap()),
            )
            .unwrap(),
        );
    }
    // Approve everything (the user clicking "Allow once").
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let prompts = Arc::new(ApprovalPrompts::new(move |e: ApprovalEvent| {
        if let ApprovalEvent::Requested(r) = e {
            let _ = tx.send(r.id);
        }
    }));
    let _guard = rt.enter();
    let agent = AgentHandle::start(
        endpoint.clone(),
        set,
        Arc::clone(&prompts),
        AgentConfig::default(),
    )
    .unwrap();
    let answering = Arc::clone(&prompts);
    rt.spawn(async move {
        while let Some(id) = rx.recv().await {
            let _ = answering.respond(id, Decision::AllowOnce);
        }
    });
    let sock = endpoint.auth_sock();

    let listed = run(Command::new(tool("ssh-add")).arg("-L"), &sock);
    let listed = String::from_utf8(listed.stdout).unwrap();
    for (name, _, public, _) in KEYS {
        assert!(
            listed.contains(public.trim()),
            "{name} not listed: {listed}"
        );
    }

    let message = dir.path().join("message.txt");
    std::fs::write(&message, b"signed through the Mosaic agent\n").unwrap();
    for (name, _, public, _) in KEYS {
        let pubfile = dir.path().join(format!("{name}.pub"));
        std::fs::write(&pubfile, public).unwrap();
        let sigfile = dir.path().join("message.txt.sig");
        let _ = std::fs::remove_file(&sigfile);
        run(
            Command::new(tool("ssh-keygen"))
                .args(["-Y", "sign", "-n", "file", "-f"])
                .arg(&pubfile)
                .arg(&message),
            &sock,
        );
        let check = Command::new(tool("ssh-keygen"))
            .args(["-Y", "check-novalidate", "-n", "file", "-s"])
            .arg(&sigfile)
            .stdin(std::fs::File::open(&message).unwrap())
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        // And independently with ssh-key.
        let sig = SshSig::from_pem(std::fs::read(&sigfile).unwrap()).unwrap();
        PublicKey::from_openssh(public)
            .unwrap()
            .verify("file", &std::fs::read(&message).unwrap(), &sig)
            .unwrap();
    }
    rt.block_on(agent.stop());
}
