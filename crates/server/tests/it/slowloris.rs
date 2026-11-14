//! R64: the accept loop disconnects slow-header clients and idle keep-alive
//! connections after the header read timeout, serves normal requests, hands
//! handlers the peer address and drains on shutdown.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::ConnectInfo;
use axum::routing::get;
use mosaic_server::accept::serve_listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TIMEOUT: Duration = Duration::from_millis(300);

fn router() -> Router {
    Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route(
            "/peer",
            get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.to_string() }),
        )
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(700)).await;
                "done"
            }),
        )
}

struct Server {
    addr: SocketAddr,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

async fn start() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        serve_listener(
            listener,
            router(),
            async {
                let _ = stopped.await;
            },
            TIMEOUT,
        )
        .await
        .unwrap();
    });
    Server {
        addr,
        stop: Some(stop),
        task,
    }
}

/// Reads until the peer closes; panics if that takes longer than `limit`.
async fn read_to_close(stream: &mut TcpStream, limit: Duration) -> String {
    let mut out = Vec::new();
    let mut buf = [0_u8; 1024];
    let deadline = Instant::now() + limit;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, stream.read(&mut buf)).await {
            Err(elapsed) => panic!("connection still open after {limit:?}: {elapsed}"),
            Ok(Ok(0) | Err(_)) => return String::from_utf8_lossy(&out).into_owned(),
            Ok(Ok(n)) => out.extend_from_slice(&buf[..n]),
        }
    }
}

#[tokio::test]
async fn trickled_headers_are_cut_off_at_the_timeout() {
    let server = start().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    stream
        .write_all(b"GET /ping HTTP/1.1\r\nHost: x\r\n")
        .await
        .unwrap();
    let started = Instant::now();
    // One byte per 100 ms, never finishing the head, until the server hangs up.
    let mut buf = [0_u8; 256];
    let mut closed = false;
    while started.elapsed() < Duration::from_secs(3) {
        tokio::select! {
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                if stream.write_all(b"X").await.is_err() {
                    closed = true;
                    break;
                }
            }
            read = stream.read(&mut buf) => {
                if matches!(read, Ok(0) | Err(_)) {
                    closed = true;
                    break;
                }
            }
        }
    }
    let elapsed = started.elapsed();
    assert!(closed, "the slow client was never disconnected");
    assert!(
        elapsed >= Duration::from_millis(250),
        "cut off early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1500),
        "cut off late: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_silent_connection_is_cut_off() {
    let server = start().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let started = Instant::now();
    read_to_close(&mut stream, Duration::from_millis(1500)).await;
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[tokio::test]
async fn an_idle_keep_alive_connection_is_cut_off() {
    let server = start().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    stream
        .write_all(b"GET /ping HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut buf = [0_u8; 512];
    let n = stream.read(&mut buf).await.unwrap();
    assert!(String::from_utf8_lossy(&buf[..n]).starts_with("HTTP/1.1 200"));
    // The connection stays open (keep-alive) and sends nothing more.
    let started = Instant::now();
    read_to_close(&mut stream, Duration::from_millis(1500)).await;
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[tokio::test]
async fn a_normal_request_works_and_sees_the_peer_address() {
    let server = start().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let local = stream.local_addr().unwrap();
    stream
        .write_all(b"GET /peer HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let reply = read_to_close(&mut stream, Duration::from_secs(3)).await;
    assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
    assert!(reply.ends_with(&local.to_string()), "{reply}");
}

#[tokio::test]
async fn a_head_sent_in_pieces_within_the_timeout_works() {
    let server = start().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    stream.write_all(b"GET /ping HTTP/1.1\r\n").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    stream
        .write_all(b"Host: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let reply = read_to_close(&mut stream, Duration::from_secs(3)).await;
    assert!(
        reply.starts_with("HTTP/1.1 200") && reply.ends_with("pong"),
        "{reply}"
    );
}

#[tokio::test]
async fn shutdown_drains_the_request_in_flight_and_closes_idle_connections() {
    let mut server = start().await;
    let mut busy = TcpStream::connect(server.addr).await.unwrap();
    busy.write_all(b"GET /slow HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut idle = TcpStream::connect(server.addr).await.unwrap();
    idle.write_all(b"GET /ping HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut buf = [0_u8; 512];
    let n = idle.read(&mut buf).await.unwrap();
    assert!(n > 0);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _ = server.stop.take().unwrap().send(());
    // The slow request still completes, then its connection is closed.
    let reply = read_to_close(&mut busy, Duration::from_secs(3)).await;
    assert!(
        reply.starts_with("HTTP/1.1 200") && reply.ends_with("done"),
        "{reply}"
    );
    // The server finishes once the connections are gone.
    tokio::time::timeout(Duration::from_secs(3), server.task)
        .await
        .expect("serve_listener returned after the drain")
        .unwrap();
    read_to_close(&mut idle, Duration::from_secs(1)).await;
    // And it accepts no new connections.
    assert!(TcpStream::connect(server.addr).await.is_err());
}
