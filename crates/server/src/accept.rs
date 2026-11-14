//! The accept loop. `axum::serve` cannot set hyper's HTTP/1 header read
//! timeout, so a client that trickles header bytes (slowloris, R64) would hold
//! a connection for ever. This loop serves the router through hyper's
//! `http1::Builder` with `header_read_timeout`, which hyper arms every time it
//! starts reading a request head: that covers the first request, a trickled
//! head and the idle wait on a keep-alive connection alike.

use std::future::Future;
use std::io;
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::graceful::GracefulShutdown;
use tower::ServiceExt;

/// How long a client may take to send a request head (and to send the next one
/// on an idle keep-alive connection).
pub const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Pause after an accept error that is not about one dead connection (file
/// descriptor exhaustion and the like), so the loop cannot spin the CPU.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(200);

/// Serves `router` on `listener` until `shutdown` completes, then lets the
/// connections in flight finish (idle ones close at once).
///
/// The peer address is available to handlers as `ConnectInfo<SocketAddr>`.
///
/// # Errors
/// None today; the signature keeps room for listener setup errors.
pub async fn serve_listener(
    listener: tokio::net::TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()>,
    header_read_timeout: Duration,
) -> io::Result<()> {
    let graceful = GracefulShutdown::new();
    tokio::pin!(shutdown);
    loop {
        let (stream, peer) = tokio::select! {
            () = &mut shutdown => break,
            accepted = listener.accept() => match accepted {
                Ok(pair) => pair,
                Err(e) => {
                    if !is_connection_error(&e) {
                        // The kind only: no peer data, nothing secret.
                        tracing::warn!(kind = ?e.kind(), "accept failed");
                        tokio::time::sleep(ACCEPT_BACKOFF).await;
                    }
                    continue;
                }
            },
        };
        let router = router.clone();
        let service = service_fn(move |mut request: hyper::Request<Incoming>| {
            request.extensions_mut().insert(ConnectInfo(peer));
            let router = router.clone();
            async move { router.oneshot(request).await }
        });
        let mut builder = http1::Builder::new();
        builder
            .timer(TokioTimer::new())
            .header_read_timeout(header_read_timeout);
        let connection = builder.serve_connection(TokioIo::new(stream), service);
        let connection = graceful.watch(connection);
        tokio::spawn(async move {
            // A connection error (reset, bad request, header timeout) ends
            // only this connection; nothing of it is logged.
            let _ = connection.await;
        });
    }
    drop(listener);
    graceful.shutdown().await;
    Ok(())
}

/// Errors that concern the one connection that was being accepted.
fn is_connection_error(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionRefused
    )
}
