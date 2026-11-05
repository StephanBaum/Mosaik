//! Runs the real `mosaic-native-host` binary against an in-test desktop built
//! from the library's server-side handshake and session code.
//!
//! The binary reads `MOSAIC_BROWSER_ENDPOINT` in debug builds and accepts the
//! dev extension ID in debug builds, so these tests need a debug profile.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::panic,
    reason = "integration tests"
)]

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use mosaic_core::browser_channel::PairingKey;
use mosaic_native_messaging::endpoint::{Endpoint, new_windows_pipe_name};
use mosaic_native_messaging::framing::{ByteOrder, read_frame};
use mosaic_native_messaging::handshake::{read_client_message, send, server_handshake};
use mosaic_native_messaging::ids::{DEV_CHROMIUM_EXTENSION_ID, FIREFOX_EXTENSION_ID};
use mosaic_native_messaging::preauth::ClientMessage;
use mosaic_native_messaging::protocol::LockState;
use mosaic_native_messaging::transport::Listener;
use serde_json::{Value, json};

const HOST: &str = env!("CARGO_BIN_EXE_mosaic-native-host");
const PAIRING_ID: [u8; 16] = [9; 16];

fn key() -> PairingKey {
    PairingKey::from_bytes(&mut [7; 32])
}

/// An endpoint for this platform, with the directory that holds it.
fn fresh_endpoint() -> (Endpoint, tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = if cfg!(windows) {
        Endpoint::NamedPipe(new_windows_pipe_name().unwrap())
    } else {
        Endpoint::UnixSocket(dir.path().join("mosaic").join("browser.sock"))
    };
    let value = match &endpoint {
        Endpoint::NamedPipe(name) => name.clone(),
        Endpoint::UnixSocket(path) => path.to_string_lossy().into_owned(),
    };
    (endpoint, dir, value)
}

/// The desktop: serves the handshake and a few ops until stopped.
struct Desktop {
    stop: Arc<AtomicBool>,
    accepted: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
}

impl Desktop {
    fn start(endpoint: Endpoint) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = mpsc::channel();
        let (stop2, accepted2) = (Arc::clone(&stop), Arc::clone(&accepted));
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let listener = Listener::bind(&endpoint).unwrap();
                ready_tx.send(()).unwrap();
                accept_loop(listener, stop2, accepted2).await;
            });
        });
        ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        Self {
            stop,
            accepted,
            thread: Some(thread),
        }
    }

    fn accepted(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }
}

async fn accept_loop(mut listener: Listener, stop: Arc<AtomicBool>, accepted: Arc<AtomicUsize>) {
    while !stop.load(Ordering::SeqCst) {
        let next = tokio::time::timeout(Duration::from_millis(25), listener.accept()).await;
        let Ok(Ok(stream)) = next else { continue };
        accepted.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(serve(stream));
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn serve<S>(mut stream: S)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let Ok(ClientMessage::Hello {
        pairing_id, nonce, ..
    }) = read_client_message(&mut stream).await
    else {
        return;
    };
    assert_eq!(pairing_id.0, PAIRING_ID);
    let Ok(mut session) = server_handshake(
        &mut stream,
        &pairing_id.0,
        &nonce.0,
        Some(&key()),
        LockState::Unlocked,
    )
    .await
    else {
        return;
    };
    while let Ok(Some(frame)) = read_frame(&mut stream, ByteOrder::Big).await {
        let Ok(plaintext) = session.open(&frame) else {
            return;
        };
        let request: Value = serde_json::from_slice(plaintext.expose_secret()).unwrap();
        let id = request["id"].clone();
        let response = match request["op"].as_str() {
            Some("status") => json!({"v": 1, "id": id, "ok": true, "body": {"state": "unlocked"}}),
            Some("fill") => json!({"v": 1, "id": id, "ok": true, "body": {
                "username": "ada", "password": "pw-from-the-desktop", "otp": null}}),
            _ => json!({"v": 1, "id": id, "ok": false, "error": "invalidRequest"}),
        };
        if send(&mut stream, &mut session, response.to_string().as_bytes())
            .await
            .is_err()
        {
            return;
        }
    }
}

/// A running host with framed stdio.
struct Host {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: mpsc::Receiver<Value>,
}

impl Host {
    fn spawn(args: &[&str], endpoint_value: &str) -> Self {
        let mut child = Command::new(HOST)
            .args(args)
            .env("MOSAIC_BROWSER_ENDPOINT", endpoint_value)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let mut stdout = child.stdout.take().unwrap();
        let (tx, frames) = mpsc::channel();
        std::thread::spawn(move || {
            loop {
                let mut prefix = [0u8; 4];
                if stdout.read_exact(&mut prefix).is_err() {
                    return;
                }
                let mut body = vec![0u8; u32::from_ne_bytes(prefix) as usize];
                if stdout.read_exact(&mut body).is_err() {
                    return;
                }
                if tx.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            frames,
        }
    }

    fn chromium(endpoint_value: &str) -> Self {
        let origin = format!("chrome-extension://{DEV_CHROMIUM_EXTENSION_ID}/");
        Self::spawn(&[&origin], endpoint_value)
    }

    fn send_bytes(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    }

    fn send(&mut self, message: &Value) {
        let json = message.to_string();
        let mut framed = u32::try_from(json.len()).unwrap().to_ne_bytes().to_vec();
        framed.extend_from_slice(json.as_bytes());
        self.send_bytes(&framed);
    }

    fn recv(&self) -> Value {
        self.frames
            .recv_timeout(Duration::from_secs(20))
            .expect("no message from the host")
    }

    /// Closes stdin and waits for the exit code.
    fn finish(mut self) -> Option<i32> {
        drop(self.stdin.take());
        self.wait()
    }

    fn wait(&mut self) -> Option<i32> {
        for _ in 0..400 {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code();
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        panic!("the host did not exit");
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn hello() -> Value {
    json!({"v": 1, "id": 1, "op": "hello", "body": {
        "pairing": {"pairingId": "09".repeat(16), "key": "07".repeat(32)},
        "hint": "chrome"}})
}

#[test]
fn hello_status_and_fill_reach_the_desktop_and_stdin_eof_exits_zero() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (endpoint, _dir, value) = fresh_endpoint();
    let desktop = Desktop::start(endpoint);
    let mut host = Host::chromium(&value);

    host.send(&hello());
    assert_eq!(
        host.recv(),
        json!({"v": 1, "id": 1, "ok": true, "body": {"desktop": "unlocked", "paired": true}})
    );
    host.send(&json!({"v": 1, "id": 2, "op": "status", "body": {}}));
    assert_eq!(
        host.recv(),
        json!({"v": 1, "id": 2, "ok": true, "body": {"state": "unlocked"}})
    );
    let fill = json!({"vaultId": "v", "itemId": "i", "origin": "https://example.com",
        "topOrigin": "https://example.com", "frame": "top",
        "want": {"login": true, "otp": false}});
    host.send(&json!({"v": 1, "id": 3, "op": "fill", "body": fill}));
    assert_eq!(
        host.recv(),
        json!({"v": 1, "id": 3, "ok": true, "body": {
            "username": "ada", "password": "pw-from-the-desktop", "otp": null}})
    );
    // A forbidden op is answered by the host and never forwarded.
    for (id, op) in [(4, "get_item"), (5, "export"), (6, "reveal_secret")] {
        host.send(&json!({"v": 1, "id": id, "op": op, "body": {}}));
        assert_eq!(
            host.recv(),
            json!({"v": 1, "id": id, "ok": false, "error": "invalidRequest"})
        );
    }
    assert_eq!(
        desktop.accepted(),
        1,
        "one connection for the whole session"
    );
    assert_eq!(host.finish(), Some(0));
}

#[test]
fn a_caller_that_is_not_an_allowed_extension_exits_2_without_connecting() {
    let (endpoint, _dir, value) = fresh_endpoint();
    let desktop = Desktop::start(endpoint);
    for args in [
        vec!["chrome-extension://abcdefghijklmnopabcdefghijklmnop/"],
        vec!["https://example.com"],
        vec!["manifest.json", "other@example.org"],
        vec![],
    ] {
        let mut host = Host::spawn(&args, &value);
        assert_eq!(host.wait(), Some(2), "{args:?}");
    }
    assert_eq!(desktop.accepted(), 0);
}

#[test]
fn the_firefox_caller_is_accepted() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (endpoint, _dir, value) = fresh_endpoint();
    let _desktop = Desktop::start(endpoint);
    let mut host = Host::spawn(&["manifest.json", FIREFOX_EXTENSION_ID], &value);
    host.send(&hello());
    assert_eq!(host.recv()["body"]["paired"], json!(true));
    assert_eq!(host.finish(), Some(0));
}

#[test]
fn a_missing_desktop_is_reported_to_the_extension() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (_endpoint, _dir, value) = fresh_endpoint();
    let mut host = Host::chromium(&value);
    host.send(&hello());
    assert_eq!(
        host.recv(),
        json!({"v": 1, "id": 1, "ok": true, "body": {"desktop": "notRunning", "paired": true}})
    );
    host.send(&json!({"v": 1, "id": 2, "op": "fill", "body": {}}));
    assert_eq!(
        host.recv(),
        json!({"v": 1, "id": 2, "ok": false, "error": "desktopNotRunning"})
    );
    assert_eq!(host.finish(), Some(0));
}

#[test]
fn a_malformed_browser_frame_exits_3() {
    if !cfg!(debug_assertions) {
        return;
    }
    let (_endpoint, _dir, value) = fresh_endpoint();
    for bytes in [
        0u32.to_ne_bytes().to_vec(),
        u32::MAX.to_ne_bytes().to_vec(),
        [&4u32.to_ne_bytes()[..], b"nope"].concat(),
    ] {
        let mut host = Host::chromium(&value);
        host.send_bytes(&bytes);
        assert_eq!(host.wait(), Some(3), "{bytes:?}");
    }
}
