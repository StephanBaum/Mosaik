//! `mosaic agent pair | status` and the agent-context guard, against the real server loop of
//! `mosaic-agent` with a fake host on a local endpoint (`MOSAIC_AGENT_ENDPOINT`).

use std::collections::HashMap;
use std::future::Future;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use mosaic_agent::server::{AgentHost, AgentServer};
use mosaic_agent::testing::local_endpoint;
use mosaic_agent::wire::{AccessState, OpRequest, OpResponse, VaultState, WireError};
use mosaic_core::browser_channel::{PairingKey, new_pairing_id};
use mosaic_native_messaging::agent_preauth::AgentRefusal;
use mosaic_native_messaging::endpoint::Endpoint;

use crate::common::Fixture;

type PairOutcome = Result<([u8; 16], PairingKey), AgentRefusal>;

struct Host {
    state: Mutex<(VaultState, AccessState)>,
    keys: Mutex<HashMap<[u8; 16], Arc<PairingKey>>>,
    pair: Mutex<Option<PairOutcome>>,
}

impl AgentHost for Host {
    fn state(&self) -> (VaultState, AccessState) {
        *self.state.lock().unwrap()
    }

    fn pairing_key(&self, id: &[u8; 16]) -> Option<Arc<PairingKey>> {
        self.keys.lock().unwrap().get(id).cloned()
    }

    fn pair(
        &self,
        _name: String,
        _code: String,
    ) -> impl Future<Output = Result<([u8; 16], PairingKey), AgentRefusal>> + Send {
        let outcome = self
            .pair
            .lock()
            .unwrap()
            .take()
            .unwrap_or(Err(AgentRefusal::PairingDenied));
        async move { outcome }
    }

    async fn handle(
        &self,
        _pairing: [u8; 16],
        _conn: u64,
        _op: OpRequest,
    ) -> Result<OpResponse, WireError> {
        Err(WireError::code(
            mosaic_agent::error::AgentErrorCode::Internal,
        ))
    }

    fn note_used(&self, _pairing: &[u8; 16]) {}
}

/// A desktop stand-in: the server loop on its own thread.
struct Desktop {
    endpoint: String,
    host: Arc<Host>,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

fn endpoint_text(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::UnixSocket(path) => path.to_str().unwrap().to_owned(),
        Endpoint::NamedPipe(name) => name.clone(),
    }
}

impl Desktop {
    fn start(f: &Fixture, state: (VaultState, AccessState)) -> Self {
        let endpoint = local_endpoint(&f.tmp.path().join("agent-ep")).unwrap();
        if let Endpoint::UnixSocket(path) = &endpoint {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                std::fs::set_permissions(
                    path.parent().unwrap(),
                    std::fs::Permissions::from_mode(0o700),
                )
                .unwrap();
            }
        }
        let host = Arc::new(Host {
            state: Mutex::new(state),
            keys: Mutex::new(HashMap::new()),
            pair: Mutex::new(None),
        });
        let (stop, wait) = channel::<()>();
        let (ready, started) = channel::<()>();
        let (host2, endpoint2) = (Arc::clone(&host), endpoint.clone());
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let server = AgentServer::start(&endpoint2, host2).unwrap();
                ready.send(()).unwrap();
                // Wait for the stop message without blocking the runtime.
                while wait.try_recv().is_err() {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                server.stop();
            });
        });
        started.recv().unwrap();
        Self {
            endpoint: endpoint_text(&endpoint),
            host,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    fn approve_pairing(&self, id: [u8; 16], key_byte: u8) {
        *self.host.pair.lock().unwrap() =
            Some(Ok((id, PairingKey::from_bytes(&mut [key_byte; 32]))));
        self.host
            .keys
            .lock()
            .unwrap()
            .insert(id, Arc::new(PairingKey::from_bytes(&mut [key_byte; 32])));
    }

    fn refuse_pairing(&self, reason: AgentRefusal) {
        *self.host.pair.lock().unwrap() = Some(Err(reason));
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// An endpoint nothing listens on.
fn dead_endpoint(f: &Fixture) -> String {
    let endpoint = local_endpoint(&f.tmp.path().join("nobody")).unwrap();
    endpoint_text(&endpoint)
}

#[test]
fn agent_subcommands_never_open_the_store_or_a_session() {
    let f = Fixture::new();
    let missing_store = f.tmp.path().join("must-not-exist").join("vault.sqlite3");
    let store = missing_store.to_str().unwrap();
    let ep = dead_endpoint(&f);
    for args in [
        vec!["--store", store, "--json", "agent", "status"],
        vec![
            "--store",
            store,
            "--json",
            "agent",
            "pair",
            "--name",
            "Claude Code",
        ],
    ] {
        let out = f
            .cmd(&args)
            .no_store_flag()
            .env("MOSAIC_AGENT_ENDPOINT", &ep)
            .run();
        // `status` answers, `pair` fails because nobody is there; neither needs the store.
        assert!(matches!(out.code, 0 | 1), "{}", out.stderr);
        assert!(!out.stderr.contains("noStore"), "{}", out.stderr);
    }
    assert!(!missing_store.exists());
    assert!(
        !missing_store.parent().unwrap().exists(),
        "no store directory was created"
    );
    assert!(
        !f.test_dir.join("run").exists(),
        "no CLI session was touched"
    );
}

#[test]
fn status_without_desktop_is_not_running() {
    let f = Fixture::new();
    let ep = dead_endpoint(&f);
    let out = f
        .cmd(&["--json", "agent", "status", "--client", "Claude Code"])
        .env("MOSAIC_AGENT_ENDPOINT", &ep)
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let data = out.json();
    assert_eq!(data["data"]["desktop"], "notRunning");
    assert_eq!(data["data"]["paired"], false);
}

#[test]
fn pair_stores_the_key_and_status_reports_paired() {
    let f = Fixture::new();
    let desktop = Desktop::start(&f, (VaultState::Unlocked, AccessState::On));
    let id = new_pairing_id().unwrap();
    desktop.approve_pairing(id, 5);
    let out = f
        .cmd(&["--json", "agent", "pair", "--name", "Claude Code"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run();
    assert_eq!(out.code, 0, "{}", out.stderr);
    let data = out.json();
    assert_eq!(data["data"]["paired"], true);
    assert_eq!(data["data"]["code"].as_str().unwrap().len(), 6);
    let stored = std::fs::read_dir(f.test_dir.join("keystore"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("agent-pairing-")
        })
        .count();
    assert_eq!(stored, 1, "the key is in the keystore");

    let status = f
        .cmd(&["--json", "agent", "status", "--client", "Claude Code"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run()
        .json();
    assert_eq!(status["data"]["desktop"], "running");
    assert_eq!(status["data"]["vault"], "unlocked");
    assert_eq!(status["data"]["agentAccess"], "on");
    assert_eq!(status["data"]["paired"], true);
    let other = f
        .cmd(&["--json", "agent", "status", "--client", "Cursor"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run()
        .json();
    assert_eq!(other["data"]["paired"], false);
    let plain = f
        .cmd(&["agent", "status"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run();
    assert_eq!(plain.code, 0);
    assert!(plain.stdout.contains("Vault: unlocked"), "{}", plain.stdout);
}

#[test]
fn pair_needs_replace_for_an_existing_name() {
    let f = Fixture::new();
    let desktop = Desktop::start(&f, (VaultState::Unlocked, AccessState::On));
    desktop.approve_pairing(new_pairing_id().unwrap(), 5);
    let pair = |replace: bool| {
        let mut args = vec!["--json", "agent", "pair", "--name", "Cursor (work)"];
        if replace {
            args.push("--replace");
        }
        f.cmd(&args)
            .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
            .run()
    };
    assert_eq!(pair(false).code, 0);
    let again = pair(false);
    assert_eq!(again.code, 1);
    assert_eq!(again.error_json()["error"]["code"], "agentAlreadyPaired");
    desktop.approve_pairing(new_pairing_id().unwrap(), 6);
    assert_eq!(pair(true).code, 0);
}

#[test]
fn pair_refusals_have_their_codes() {
    let f = Fixture::new();
    let desktop = Desktop::start(&f, (VaultState::Unlocked, AccessState::On));
    for (reason, code, exit) in [
        (AgentRefusal::PairingDenied, "pairingDenied", 1),
        (AgentRefusal::PairingExpired, "pairingExpired", 1),
        (AgentRefusal::PairingLimit, "pairingLimit", 1),
        (AgentRefusal::PairingBlocked, "pairingBlocked", 1),
    ] {
        desktop.refuse_pairing(reason);
        let out = f
            .cmd(&["--json", "agent", "pair", "--name", "Codex"])
            .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
            .run();
        assert_eq!(out.code, exit, "{code}");
        assert_eq!(out.error_json()["error"]["code"], code);
    }
    *desktop.host.state.lock().unwrap() = (VaultState::Locked, AccessState::On);
    let out = f
        .cmd(&["--json", "agent", "pair", "--name", "Codex"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run();
    assert_eq!(out.code, 8);
    assert_eq!(out.error_json()["error"]["code"], "locked");
    *desktop.host.state.lock().unwrap() = (VaultState::Unlocked, AccessState::Off);
    let out = f
        .cmd(&["--json", "agent", "pair", "--name", "Codex"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run();
    assert_eq!(out.code, 1);
    assert_eq!(out.error_json()["error"]["code"], "agentAccessOff");
    // A bad name never reaches the desktop.
    let out = f
        .cmd(&["agent", "pair", "--name", "bad/name"])
        .env("MOSAIC_AGENT_ENDPOINT", &desktop.endpoint)
        .run();
    assert_eq!(out.code, 2);
}

#[test]
fn reveal_refuses_in_an_agent_context() {
    let f = Fixture::new();
    f.unlock("5m");
    let reference = "mosaic://Personal/GitHub";
    let refused = f
        .cmd(&[
            "--json",
            "--no-input",
            "get",
            reference,
            "--field",
            "password",
            "--reveal",
        ])
        .env("CLAUDECODE", "1")
        .run();
    assert_eq!(refused.code, 9);
    assert_eq!(refused.error_json()["error"]["code"], "agentContext");
    assert!(refused.stdout.is_empty(), "nothing was printed");
    let inject = f
        .cmd(&["--no-input", "inject"])
        .stdin("{{ mosaic://Personal/GitHub/password }}")
        .env("CLAUDECODE", "1")
        .run();
    assert_eq!(inject.code, 9);
    // `--i-know` gives the old behavior.
    let allowed = f
        .cmd(&[
            "--no-input",
            "get",
            reference,
            "--field",
            "password",
            "--reveal",
            "--i-know",
        ])
        .env("CLAUDECODE", "1")
        .run();
    assert_eq!(allowed.code, 0, "{}", allowed.stderr);
    assert!(!allowed.stdout.is_empty());
    // Without the variable nothing changes.
    let plain = f
        .cmd(&[
            "--no-input",
            "get",
            reference,
            "--field",
            "password",
            "--reveal",
        ])
        .run();
    assert_eq!(plain.code, 0);
}
