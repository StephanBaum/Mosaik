//! The running agent: an [`AgentCore`] behind an OS endpoint.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::{JoinHandle, JoinSet};

use crate::approval::Approver;
use crate::core::{AgentConfig, AgentCore, Either, race};
use crate::error::Error;
use crate::keys::KeySet;
use crate::transport::{Endpoint, Transport};

/// A running agent. Start it after unlock with [`AgentHandle::start`]; stop
/// it on lock with [`AgentHandle::stop`] (dropping the handle stops it too).
pub struct AgentHandle<A: Approver> {
    core: Arc<AgentCore<A>>,
    endpoint: Endpoint,
    task: Option<JoinHandle<()>>,
}

impl<A: Approver> AgentHandle<A> {
    /// Binds `endpoint` and serves `keys` on the current Tokio runtime.
    ///
    /// # Errors
    /// [`Error::NoRuntime`] outside a Tokio runtime, otherwise see
    /// [`Error`]. Nothing is left behind on failure.
    pub fn start(
        endpoint: Endpoint,
        keys: KeySet,
        approver: Arc<A>,
        config: AgentConfig,
    ) -> Result<Self, Error> {
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| Error::NoRuntime)?;
        let transport = {
            let _guard = runtime.enter();
            Transport::bind(&endpoint)?
        };
        let core = Arc::new(AgentCore::new(keys, approver, config));
        let task = runtime.spawn(accept_loop(transport, Arc::clone(&core)));
        Ok(Self {
            core,
            endpoint,
            task: Some(task),
        })
    }

    /// The endpoint, for `SSH_AUTH_SOCK`.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The core: key count, key updates, grant revocation.
    #[must_use]
    pub fn core(&self) -> &AgentCore<A> {
        &self.core
    }

    /// Stops the agent and waits until the endpoint is closed: pending
    /// approvals are refused (closed as locked), connections are closed,
    /// the socket file is removed, and the keys and grants are dropped.
    pub async fn stop(mut self) {
        self.core.stop();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl<A: Approver> Drop for AgentHandle<A> {
    fn drop(&mut self) {
        self.core.stop();
        // The accept loop sees the stop and cleans up on its own; aborting
        // would skip closing the connections in order.
    }
}

async fn accept_loop<A: Approver>(mut transport: Transport, core: Arc<AgentCore<A>>) {
    let mut connections = JoinSet::new();
    loop {
        match race(transport.accept(), core.stopped()).await {
            Either::Right(()) => break,
            Either::Left(Ok((stream, client))) => {
                while connections.try_join_next().is_some() {}
                if connections.len() >= core.config().max_connections {
                    drop(stream);
                    continue;
                }
                let core = Arc::clone(&core);
                connections.spawn(async move {
                    let _ = core.serve(stream, client).await;
                });
            }
            Either::Left(Err(_)) => {
                // A failing accept (for example, out of handles) must not
                // spin.
                if let Either::Right(()) = race(
                    tokio::time::sleep(Duration::from_millis(100)),
                    core.stopped(),
                )
                .await
                {
                    break;
                }
            }
        }
    }
    // `serve` returns soon after the stop; the abort covers a connection
    // stuck in a write to a client that does not read, or in a message
    // body that never completes.
    let drained = tokio::time::timeout(Duration::from_secs(1), async {
        while connections.join_next().await.is_some() {}
    })
    .await;
    let _ = drained;
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    drop(transport);
}
