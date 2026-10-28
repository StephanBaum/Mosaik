//! Per-use approval: every sign request asks the user through an
//! [`Approver`], unless an earlier "allow for N minutes" grant for the same key
//! and client still holds ("allow for N minutes", default 15).

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use mosaic_core::{ItemId, VaultId};
use thiserror::Error;
use tokio::sync::oneshot;

use crate::keys::{KeyAlgorithm, SignFailure};

/// Identifies one approval request. Unique within the process, also across
/// agent restarts, so a late answer to a request from before a lock can never
/// approve a newer one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RequestId(u64);

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

impl RequestId {
    pub(crate) fn next() -> Self {
        Self(NEXT_REQUEST.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ssh-{}", self.0)
    }
}

/// How long "Allow for 15 minutes" (D18) remembers an answer; also the
/// default and recommended ceiling of `AgentConfig::max_grant`.
pub const DEFAULT_GRANT: Duration = Duration::from_mins(15);

/// A string that is not a [`RequestId`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[error("not an SSH approval request ID")]
pub struct InvalidRequestId;

impl FromStr for RequestId {
    type Err = InvalidRequestId;

    /// Parses the [`Display`](fmt::Display) form, `ssh-<n>`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digits = s.strip_prefix("ssh-").ok_or(InvalidRequestId)?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(InvalidRequestId);
        }
        digits.parse().map(Self).map_err(|_| InvalidRequestId)
    }
}

/// The process on the other end of a connection, as far as the OS tells.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ClientInfo {
    /// Process ID (Linux and macOS through the socket's peer credentials).
    pub pid: Option<u32>,
    /// Executable name, for display.
    pub process_name: Option<String>,
    /// Full executable path (Linux, from `/proc`). Grants that outlast one
    /// request are bound to it.
    pub executable: Option<PathBuf>,
}

impl ClientInfo {
    /// Nothing known about the client.
    pub const UNKNOWN: Self = Self {
        pid: None,
        process_name: None,
        executable: None,
    };

    /// What a grant for this client is bound to. `None`: the client cannot
    /// be told apart from any other process, so nothing is remembered for
    /// it. The process ID is not used: SSH clients run once per connection.
    fn grant_scope(&self) -> Option<&Path> {
        self.executable.as_deref()
    }

    /// Whether an "allow for N minutes" answer can be remembered for this
    /// client.
    #[must_use]
    pub fn can_remember(&self) -> bool {
        self.grant_scope().is_some()
    }
}

/// A sign request waiting for the user.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApprovalRequest {
    /// Answer with this ID.
    pub id: RequestId,
    /// The key's vault.
    pub vault: VaultId,
    /// The key's item.
    pub item: ItemId,
    /// The item title, if any.
    pub key_title: Option<String>,
    /// The public key fingerprint, `SHA256:…`.
    pub fingerprint: String,
    /// The key algorithm.
    pub algorithm: KeyAlgorithm,
    /// Who asks.
    pub client: ClientInfo,
    /// When the request arrived.
    pub requested_at: SystemTime,
    /// When it is refused without an answer.
    pub expires_at: SystemTime,
    /// Whether `Decision::AllowFor` is remembered; if `false` it counts as
    /// `AllowOnce`. Hide "Allow for 15 minutes" when `false`.
    pub can_remember: bool,
}

/// The user's answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// Sign this request only.
    AllowOnce,
    /// Sign this request, and requests for the same key from the same client
    /// executable until `now + min(d, AgentConfig::max_grant)`. For a client
    /// whose executable is unknown, the same as `AllowOnce`.
    AllowFor(Duration),
    /// Refuse.
    Deny,
}

/// Why an approval request closed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CloseReason {
    /// [`Approver::approve`] returned.
    Answered,
    /// Nobody answered within [`crate::AgentConfig::approval_timeout`];
    /// refused.
    Expired,
    /// The agent stopped (the vault locked); refused.
    Locked,
}

/// Asks the user about sign requests. Implemented by the desktop app.
pub trait Approver: Send + Sync + 'static {
    /// Shows `request` and waits for the answer. The agent drops the future
    /// when the request expires or the agent stops, then calls
    /// [`Approver::closed`].
    fn approve(&self, request: ApprovalRequest) -> impl Future<Output = Decision> + Send;

    /// The request closed: answered, expired or refused because the agent
    /// stopped. Called once per request passed to [`Approver::approve`].
    fn closed(&self, id: RequestId, reason: CloseReason) {
        let _ = (id, reason);
    }

    /// A request could not be signed after it was approved or covered by a
    /// grant (for example, a wrong passphrase on the item). With a grant,
    /// `notice.id` was never passed to [`Approver::approve`].
    fn sign_failed(&self, notice: SignFailed) {
        let _ = notice;
    }
}

/// A sign request that failed after approval. Carries no key material.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SignFailed {
    /// The request.
    pub id: RequestId,
    /// The key's vault.
    pub vault: VaultId,
    /// The key's item.
    pub item: ItemId,
    /// Why.
    pub failure: SignFailure,
}

/// [`ApprovalPrompts::respond`] found no request with that ID: it expired,
/// was answered already, or the agent stopped.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[error("the SSH approval request is no longer open")]
pub struct RequestExpired;

/// What [`ApprovalPrompts`] reports to the app.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ApprovalEvent {
    /// Show this request (IPC `sshApprovalRequested`).
    Requested(ApprovalRequest),
    /// Close it (IPC `sshApprovalClosed`).
    Closed {
        /// The request.
        id: RequestId,
        /// Why.
        reason: CloseReason,
    },
    /// A request could not be signed after approval.
    SignFailed(SignFailed),
}

/// An [`Approver`] that reports requests through a callback and takes the
/// answers through [`ApprovalPrompts::respond`]: the shape of the desktop
/// IPC (`sshApprovalRequested` event, `ssh_respond` command).
pub struct ApprovalPrompts<F> {
    notify: F,
    open: Mutex<HashMap<RequestId, oneshot::Sender<Decision>>>,
}

impl<F: Fn(ApprovalEvent) + Send + Sync + 'static> ApprovalPrompts<F> {
    /// Reports events to `notify`. It runs on the agent's tasks and must
    /// not block.
    #[must_use]
    pub fn new(notify: F) -> Self {
        Self {
            notify,
            open: Mutex::new(HashMap::new()),
        }
    }

    /// Answers an open request.
    ///
    /// # Errors
    /// [`RequestExpired`] if no request with this ID is open.
    pub fn respond(&self, id: RequestId, decision: Decision) -> Result<(), RequestExpired> {
        let sender = self.lock().remove(&id).ok_or(RequestExpired)?;
        sender.send(decision).map_err(|_| RequestExpired)
    }

    /// Number of open requests.
    #[must_use]
    pub fn open_requests(&self) -> usize {
        self.lock().len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<RequestId, oneshot::Sender<Decision>>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<F: Fn(ApprovalEvent) + Send + Sync + 'static> Approver for ApprovalPrompts<F> {
    fn approve(&self, request: ApprovalRequest) -> impl Future<Output = Decision> + Send {
        let (tx, rx) = oneshot::channel();
        // Registered before the app hears of it, so an immediate answer
        // finds it.
        self.lock().insert(request.id, tx);
        (self.notify)(ApprovalEvent::Requested(request));
        async move { rx.await.unwrap_or(Decision::Deny) }
    }

    fn closed(&self, id: RequestId, reason: CloseReason) {
        self.lock().remove(&id);
        (self.notify)(ApprovalEvent::Closed { id, reason });
    }

    fn sign_failed(&self, notice: SignFailed) {
        (self.notify)(ApprovalEvent::SignFailed(notice));
    }
}

/// Remembered "allow for N minutes" answers, per key and client executable.
#[derive(Default)]
pub(crate) struct Grants {
    entries: Vec<Grant>,
}

struct Grant {
    key_blob: Vec<u8>,
    executable: PathBuf,
    until: Instant,
}

impl Grants {
    /// Whether a grant covers this key and client at `now`. Drops expired
    /// grants.
    pub(crate) fn allows(&mut self, key_blob: &[u8], client: &ClientInfo, now: Instant) -> bool {
        self.entries.retain(|g| now < g.until);
        let Some(exe) = client.grant_scope() else {
            return false;
        };
        self.entries
            .iter()
            .any(|g| g.key_blob == key_blob && g.executable == exe)
    }

    /// Keeps only the grants for keys `keep` accepts.
    pub(crate) fn retain_keys(&mut self, mut keep: impl FnMut(&[u8]) -> bool) {
        self.entries.retain(|g| keep(&g.key_blob));
    }

    /// Remembers a grant; does nothing for a client without a scope.
    pub(crate) fn add(&mut self, key_blob: &[u8], client: &ClientInfo, until: Instant) {
        let Some(exe) = client.grant_scope() else {
            return;
        };
        self.entries
            .retain(|g| !(g.key_blob == key_blob && g.executable == exe));
        self.entries.push(Grant {
            key_blob: key_blob.to_vec(),
            executable: exe.to_path_buf(),
            until,
        });
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(exe: Option<&str>) -> ClientInfo {
        ClientInfo {
            pid: Some(7),
            process_name: Some("ssh".into()),
            executable: exe.map(PathBuf::from),
        }
    }

    #[test]
    fn request_ids_are_unique_and_round_trip() {
        let a = RequestId::next();
        let b = RequestId::next();
        assert_ne!(a, b);
        assert_eq!(a.to_string().parse::<RequestId>(), Ok(a));
        for bad in [
            "",
            "ssh-",
            "ssh-x",
            "ssh-+1",
            "ssh- 1",
            "x-1",
            "ssh-99999999999999999999999",
        ] {
            assert_eq!(bad.parse::<RequestId>(), Err(InvalidRequestId), "{bad}");
        }
    }

    #[test]
    fn grants_are_scoped_to_key_and_executable_and_expire() {
        let mut g = Grants::default();
        let now = Instant::now();
        let ssh = client(Some("/usr/bin/ssh"));
        let other = client(Some("/tmp/evil"));
        g.add(b"k1", &ssh, now + Duration::from_secs(60));
        assert!(g.allows(b"k1", &ssh, now));
        assert!(!g.allows(b"k2", &ssh, now), "other key");
        assert!(!g.allows(b"k1", &other, now), "other executable");
        assert!(
            !g.allows(b"k1", &ClientInfo::UNKNOWN, now),
            "unknown client"
        );
        assert!(
            !g.allows(b"k1", &ssh, now + Duration::from_secs(60)),
            "expired"
        );
        assert_eq!(g.len(), 0, "expired grants are dropped");
        g.add(b"k1", &ssh, now + Duration::from_hours(48));
        g.add(b"k1", &ssh, now + Duration::from_hours(48));
        assert_eq!(g.len(), 1, "one grant per key and client");
        assert!(g.allows(b"k1", &ssh, now + Duration::from_hours(12)));
        g.clear();
        assert!(!g.allows(b"k1", &ssh, now));
    }

    proptest::proptest! {
        #[test]
        fn grants_only_cover_their_scope(
            key in proptest::collection::vec(proptest::prelude::any::<u8>(), 1..8),
            other in proptest::collection::vec(proptest::prelude::any::<u8>(), 1..8),
            e in 0usize..3,
            f in 0usize..3,
            secs in 1u64..3600,
            probe in 0u64..7200,
        ) {
            const EXES: [&str; 3] = ["/usr/bin/ssh", "/usr/bin/git", "/tmp/evil"];
            let c = |i: usize| client(EXES.get(i).copied());
            let t0 = Instant::now();
            let mut g = Grants::default();
            g.add(&key, &c(e), t0 + Duration::from_secs(secs));
            proptest::prop_assert_eq!(g.allows(&other, &c(e), t0), other == key);
            proptest::prop_assert_eq!(g.allows(&key, &c(f), t0), e == f);
            proptest::prop_assert!(!g.allows(&key, &ClientInfo::UNKNOWN, t0));
            proptest::prop_assert_eq!(
                g.allows(&key, &c(e), t0 + Duration::from_secs(probe)),
                probe < secs
            );
        }

        #[test]
        fn request_ids_never_parse_foreign_strings(s in "\\PC{0,24}") {
            if let Ok(id) = s.parse::<RequestId>() {
                proptest::prop_assert_eq!(id.to_string(), s);
            }
        }
    }

    #[test]
    fn unknown_clients_get_no_grant() {
        let mut g = Grants::default();
        let anon = client(None);
        assert!(!anon.can_remember());
        g.add(b"k", &anon, Instant::now() + Duration::from_secs(60));
        assert_eq!(g.len(), 0);
        assert!(!g.allows(b"k", &anon, Instant::now()));
    }
}
