//! The protocol core: answers one request at a time, independent of the
//! transport.

use std::future::{Future, poll_fn};
use std::io;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::Poll;
use std::time::{Duration, Instant, SystemTime};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::watch;
use zeroize::Zeroizing;

use mosaic_core::{ItemId, VaultId};

use crate::approval::{
    ApprovalRequest, Approver, ClientInfo, CloseReason, DEFAULT_GRANT, Decision, Grants, RequestId,
    SignFailed,
};
use crate::keys::{AgentKey, DEFAULT_MAX_KDF_ROUNDS, KeySet, SignFailure};
use crate::wire::{self, Request, Response, SignRequest};

/// Agent limits. Start from [`AgentConfig::default`] and change fields.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AgentConfig {
    /// How long a sign request waits for the user before it is refused
    /// (default 25 s).
    pub approval_timeout: Duration,
    /// Sign requests waiting for the user at once; more are refused at
    /// once, so a client cannot flood the user with prompts (default 4).
    pub max_pending_approvals: usize,
    /// The longest [`Decision::AllowFor`] the agent remembers (default
    /// [`DEFAULT_GRANT`], 15 min).
    pub max_grant: Duration,
    /// The highest bcrypt-pbkdf round count of an encrypted key (default
    /// [`DEFAULT_MAX_KDF_ROUNDS`]).
    pub max_kdf_rounds: u32,
    /// Open client connections; more are closed at once (default 32).
    pub max_connections: usize,
    /// How long the body of a message may take to arrive after its length
    /// prefix (default 10 s).
    pub read_timeout: Duration,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            approval_timeout: Duration::from_secs(25),
            max_pending_approvals: 4,
            max_grant: DEFAULT_GRANT,
            max_kdf_rounds: DEFAULT_MAX_KDF_ROUNDS,
            max_connections: 32,
            read_timeout: Duration::from_secs(10),
        }
    }
}

/// The agent's state and request handling, without a transport: the key
/// set, the remembered grants and the approver. [`crate::AgentHandle`] runs
/// it on an OS endpoint; tests and fuzzing drive it directly.
pub struct AgentCore<A> {
    keys: Mutex<KeySet>,
    grants: Mutex<Grants>,
    approver: Arc<A>,
    config: AgentConfig,
    pending: AtomicUsize,
    stop: watch::Sender<bool>,
}

impl<A: Approver> AgentCore<A> {
    /// A running core serving `keys`.
    #[must_use]
    pub fn new(keys: KeySet, approver: Arc<A>, config: AgentConfig) -> Self {
        Self {
            keys: Mutex::new(keys),
            grants: Mutex::new(Grants::default()),
            approver,
            config,
            pending: AtomicUsize::new(0),
            stop: watch::Sender::new(false),
        }
    }

    /// The limits in use.
    #[must_use]
    pub const fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Replaces the key set (items added, changed or removed while
    /// unlocked). The old keys are dropped once no sign request uses them.
    /// Grants stay for the public keys the new set still serves; the others
    /// are dropped, so a key that comes back later is asked about again.
    ///
    /// After [`AgentCore::stop`] it drops `keys` at once and serves nothing.
    pub fn set_keys(&self, keys: KeySet) {
        let old = {
            let mut held = self.keys();
            if self.is_stopped() {
                // `stop` sets the flag before it takes the lock, so either we
                // see the flag here or `stop` takes whatever we install.
                keys
            } else {
                self.grants().retain_keys(|blob| keys.find(blob).is_some());
                std::mem::replace(&mut *held, keys)
            }
        };
        drop(old);
    }

    /// Number of keys served.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys().len()
    }

    /// Forgets every "allow for a while" grant.
    pub fn revoke_grants(&self) {
        self.grants().clear();
    }

    /// Stops the core: refuses every request from now on, refuses the
    /// requests waiting for approval (closed as [`CloseReason::Locked`]),
    /// drops the keys and the grants. Irreversible; start a new agent after
    /// the next unlock.
    pub fn stop(&self) {
        self.stop.send_replace(true);
        let old = std::mem::take(&mut *self.keys());
        drop(old);
        self.grants().clear();
    }

    /// Whether [`AgentCore::stop`] was called.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        *self.stop.borrow()
    }

    /// Resolves when the core stops.
    pub(crate) fn stopped(&self) -> impl Future<Output = ()> + Send + 'static {
        let mut rx = self.stop.subscribe();
        async move {
            // An error means the sender is gone: stopped as well.
            let _ = rx.wait_for(|stopped| *stopped).await;
        }
    }

    /// Answers one message body (without its length prefix) from `client`
    /// and returns the response body. Malformed requests, management
    /// requests (add, remove, lock, unlock) and extensions get
    /// `SSH_AGENT_FAILURE`.
    pub async fn handle(&self, payload: &[u8], client: &ClientInfo) -> Vec<u8> {
        if self.is_stopped() {
            return failure();
        }
        match Request::parse(payload) {
            Ok(Request::RequestIdentities) => {
                let keys = self.keys();
                Response::Identities(keys.identities())
                    .encode()
                    .unwrap_or_else(|_| failure())
            }
            Ok(Request::Sign(req)) => self.sign(req, client).await,
            Ok(_) | Err(_) => failure(),
        }
    }

    async fn sign(&self, req: SignRequest<'_>, client: &ClientInfo) -> Vec<u8> {
        let Some(key) = self.keys().find(req.key_blob) else {
            return failure();
        };
        let id = RequestId::next();
        let granted = self
            .grants()
            .allows(key.public_key_blob(), client, Instant::now());
        if !granted && !self.approve(id, &key, client).await {
            return failure();
        }
        let data = req.data.to_vec();
        let flags = req.flags;
        let rounds = self.config.max_kdf_rounds;
        let key_ids = (key.vault(), key.item());
        let blocking_key = Arc::clone(&key);
        let signed =
            tokio::task::spawn_blocking(move || blocking_key.sign(&data, flags, rounds)).await;
        drop(key);
        if self.is_stopped() {
            // The vault locked while signing: send nothing.
            return failure();
        }
        match signed {
            Ok(Ok(blob)) => Response::Signature(&blob)
                .encode()
                .unwrap_or_else(|_| failure()),
            Ok(Err(e)) => {
                self.report_failure(id, &key_ids, e);
                failure()
            }
            Err(_) => {
                self.report_failure(id, &key_ids, SignFailure::Internal);
                failure()
            }
        }
    }

    fn report_failure(&self, id: RequestId, key: &(VaultId, ItemId), failure: SignFailure) {
        self.approver.sign_failed(SignFailed {
            id,
            vault: key.0,
            item: key.1,
            failure,
        });
    }

    /// Asks the approver; `true` if the request may be signed. Remembers
    /// grants.
    async fn approve(&self, id: RequestId, key: &AgentKey, client: &ClientInfo) -> bool {
        let Some(_slot) = PendingSlot::take(&self.pending, self.config.max_pending_approvals)
        else {
            return false;
        };
        let now = SystemTime::now();
        let can_remember = client.can_remember();
        let request = ApprovalRequest {
            id,
            vault: key.vault(),
            item: key.item(),
            key_title: key.title().map(str::to_owned),
            fingerprint: key.fingerprint().to_owned(),
            algorithm: key.algorithm(),
            client: client.clone(),
            requested_at: now,
            expires_at: now.checked_add(self.config.approval_timeout).unwrap_or(now),
            can_remember,
        };
        // `closed` runs exactly once, also if this future is dropped
        // midway (the connection task aborted when the agent stops).
        let mut close = CloseOnDrop {
            core: self,
            id,
            reason: None,
        };
        let answer = race(
            tokio::time::timeout(self.config.approval_timeout, self.approver.approve(request)),
            self.stopped(),
        )
        .await;
        let (decision, reason) = match answer {
            Either::Left(Ok(decision)) => (decision, CloseReason::Answered),
            Either::Left(Err(_)) => (Decision::Deny, CloseReason::Expired),
            Either::Right(()) => (Decision::Deny, CloseReason::Locked),
        };
        close.reason = Some(reason);
        drop(close);
        if self.is_stopped() {
            return false;
        }
        match decision {
            Decision::Deny => false,
            Decision::AllowOnce => true,
            Decision::AllowFor(d) => {
                let d = d.min(self.config.max_grant);
                if can_remember && !d.is_zero() {
                    // An unrepresentable instant signs this one request only.
                    if let Some(until) = Instant::now().checked_add(d) {
                        self.grants().add(key.public_key_blob(), client, until);
                    }
                }
                true
            }
        }
    }

    /// Serves one client connection until it closes, sends a malformed
    /// frame, stalls in the middle of a message, or the core stops.
    ///
    /// # Errors
    /// I/O errors of the stream other than a clean close.
    pub async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
        &self,
        mut stream: S,
        client: ClientInfo,
    ) -> io::Result<()> {
        loop {
            let mut prefix = [0u8; 4];
            match race(stream.read_exact(&mut prefix), self.stopped()).await {
                Either::Right(()) => return Ok(()),
                Either::Left(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    return Ok(());
                }
                Either::Left(Err(e)) => return Err(e),
                Either::Left(Ok(_)) => {}
            }
            let Ok(len) = wire::body_len(prefix) else {
                return Ok(());
            };
            // Zeroizing: an `SSH_AGENTC_LOCK` body carries a passphrase.
            let mut body = Zeroizing::new(vec![0u8; len]);
            match tokio::time::timeout(self.config.read_timeout, stream.read_exact(&mut body)).await
            {
                Ok(Ok(_)) => {}
                Ok(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Ok(()),
            }
            let response = self.handle(&body, &client).await;
            if self.is_stopped() {
                return Ok(());
            }
            let framed = wire::frame(&response).map_err(|_| io::ErrorKind::InvalidData)?;
            stream.write_all(&framed).await?;
            stream.flush().await?;
        }
    }

    fn keys(&self) -> MutexGuard<'_, KeySet> {
        self.keys.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn grants(&self) -> MutexGuard<'_, Grants> {
        self.grants.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn failure() -> Vec<u8> {
    vec![wire::msg::FAILURE]
}

/// Calls [`Approver::closed`] when dropped: with the recorded reason, or,
/// if the wait was abandoned, [`CloseReason::Locked`] when the core stopped
/// and [`CloseReason::Expired`] otherwise.
struct CloseOnDrop<'a, A: Approver> {
    core: &'a AgentCore<A>,
    id: RequestId,
    reason: Option<CloseReason>,
}

impl<A: Approver> Drop for CloseOnDrop<'_, A> {
    fn drop(&mut self) {
        let reason = self.reason.unwrap_or(if self.core.is_stopped() {
            CloseReason::Locked
        } else {
            CloseReason::Expired
        });
        self.core.approver.closed(self.id, reason);
    }
}

/// A slot in the pending-approval budget, released on drop.
struct PendingSlot<'a>(&'a AtomicUsize);

impl<'a> PendingSlot<'a> {
    fn take(counter: &'a AtomicUsize, max: usize) -> Option<Self> {
        counter
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < max).then_some(n.saturating_add(1))
            })
            .ok()
            .map(|_| Self(counter))
    }
}

impl Drop for PendingSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) enum Either<L, R> {
    Left(L),
    Right(R),
}

/// Polls both futures; the first to finish wins, the other is dropped.
/// `a` is polled first.
pub(crate) async fn race<A: Future, B: Future>(a: A, b: B) -> Either<A::Output, B::Output> {
    let mut a = pin!(a);
    let mut b = pin!(b);
    poll_fn(|cx| {
        if let Poll::Ready(x) = a.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(x));
        }
        if let Poll::Ready(y) = b.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(y));
        }
        Poll::Pending
    })
    .await
}

#[cfg(test)]
mod tests;
