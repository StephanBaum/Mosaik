//! The prompts of the agent endpoint: pairing and access (ADR-0159, builder check B3).
//!
//! One pairing prompt is open at a time. Access prompts form a queue across the paired clients:
//! the first is shown, the others wait, and a client with a prompt open or queued gets `busy`
//! for another. Following the SSH approval pattern (ADR-0025), an Allow within a second of the
//! prompt being **shown** is refused (`agentAllowTooSoon`, the request stays open), and the second
//! starts when a queued prompt reaches the front, not when it was queued. The guard is enforced
//! here, not in the webview. An unanswered prompt ends at its timeout (the queue time counts),
//! on lock, when agent access is switched off, when the client hangs up or its pairing is
//! removed; none of these is ever an approval. Deny is always taken.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mosaic_agent::engine::{AccessFacts, GrantChoice};
use mosaic_agent::error::AgentErrorCode;
use mosaic_core::browser_channel::PairingKey;
use mosaic_native_messaging::agent_preauth::AgentRefusal;
use tokio::sync::oneshot;

use super::{Shared, lock, unix_millis_of, wall_ms};
use crate::browser::hex;
use crate::contract::agent::AgentInjectKind;
use crate::contract::agent_access::{
    AgentAccessDecision, AgentAccessRequest, AgentAccessRespondRequest, AgentCloseReason,
    AgentPairingRequest, AgentPairingRespondRequest,
};
use crate::contract::browser::PairingDecision;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::state::Backend;

/// The earliest an Allow is taken after the prompt was shown.
pub const MIN_ALLOW_DELAY: Duration = Duration::from_secs(1);

/// Denials within this long count towards a block.
const DENIAL_WINDOW: Duration = Duration::from_mins(10);
/// Denials within the window that block pairing prompts.
const DENIALS_TO_BLOCK: usize = 3;
/// How long pairing prompts stay blocked.
const BLOCK_FOR: Duration = Duration::from_mins(10);
/// Pairing requests per minute, shown or not (all clients together).
const PAIR_REQUESTS_PER_MINUTE: usize = 6;

/// What a pairing request ends with: the new pairing's ID and key, or why the client is refused.
pub type PairOutcome = Result<([u8; 16], PairingKey), AgentRefusal>;

/// What an access request ends with: the user's choice, or the code that ended it.
pub type AccessOutcome = Result<GrantChoice, AgentErrorCode>;

struct PendingPair {
    request_id: String,
    name: String,
    shown_at: Instant,
    reply: oneshot::Sender<PairOutcome>,
}

struct PendingAccess {
    request_id: String,
    pairing: [u8; 16],
    facts: AccessFacts,
    requested_wall: u64,
    /// Set when the prompt reaches the front of the queue (and again on every promotion).
    shown_at: Option<Instant>,
    reply: oneshot::Sender<AccessOutcome>,
}

/// The open prompts and the pairing denials.
#[derive(Default)]
pub struct Prompts {
    pair: Option<PendingPair>,
    /// The first entry is the one shown.
    access: VecDeque<PendingAccess>,
    denials: VecDeque<Instant>,
    blocked_until: Option<Instant>,
    pair_requests: VecDeque<Instant>,
}

impl Prompts {
    fn note_denial(&mut self, now: Instant) {
        self.denials.push_back(now);
        while self
            .denials
            .front()
            .is_some_and(|&t| now.saturating_duration_since(t) >= DENIAL_WINDOW)
        {
            self.denials.pop_front();
        }
        if self.denials.len() >= DENIALS_TO_BLOCK {
            self.denials.clear();
            self.blocked_until = Some(now + BLOCK_FOR);
        }
    }

    fn is_blocked(&mut self, now: Instant) -> bool {
        match self.blocked_until {
            Some(until) if now < until => true,
            Some(_) => {
                self.blocked_until = None;
                false
            }
            None => false,
        }
    }

    fn pair_rate_ok(&mut self, now: Instant) -> bool {
        while self
            .pair_requests
            .front()
            .is_some_and(|&t| now.saturating_duration_since(t) >= Duration::from_mins(1))
        {
            self.pair_requests.pop_front();
        }
        if self.pair_requests.len() >= PAIR_REQUESTS_PER_MINUTE {
            return false;
        }
        self.pair_requests.push_back(now);
        true
    }
}

fn new_request_id() -> Option<String> {
    mosaic_core::browser_channel::new_endpoint_token()
        .map(|t| hex(&t))
        .ok()
}

fn too_soon(shown_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(shown_at) < MIN_ALLOW_DELAY
}

fn expires_millis(from: u64, limit: Duration) -> i64 {
    i64::try_from(from.saturating_add(u64::try_from(limit.as_millis()).unwrap_or(u64::MAX)))
        .unwrap_or(i64::MAX)
}

fn choice_of(decision: AgentAccessDecision) -> GrantChoice {
    match decision {
        AgentAccessDecision::Deny => GrantChoice::Deny,
        AgentAccessDecision::AllowOnce => GrantChoice::Once,
        AgentAccessDecision::Allow15m => GrantChoice::Minutes15,
        AgentAccessDecision::Allow1h => GrantChoice::Hour1,
        AgentAccessDecision::AllowUntilLock => GrantChoice::UntilLock,
    }
}

fn inject_of(kind: mosaic_agent::binding::InjectKind) -> AgentInjectKind {
    use mosaic_agent::binding::InjectKind;
    match kind {
        InjectKind::Bearer => AgentInjectKind::Bearer,
        InjectKind::Header => AgentInjectKind::Header,
        InjectKind::Basic => AgentInjectKind::Basic,
        InjectKind::Query => AgentInjectKind::Query,
    }
}

impl Shared {
    // ----- pairing -------------------------------------------------------------------------

    /// Opens the pairing prompt for a `pairRequest`.
    ///
    /// # Errors
    /// `agentAccessOff`, `locked`, `pairingBlocked` (three denials lately), `rateLimited`,
    /// `pairingLimit`, `busy` (another pairing prompt is open), `internal`.
    pub(crate) fn open_pair(
        &self,
        name: &str,
        code: &str,
    ) -> Result<(String, oneshot::Receiver<PairOutcome>), AgentRefusal> {
        let now = self.now();
        if !self.is_enabled() {
            return Err(AgentRefusal::AgentAccessOff);
        }
        if !self.is_unlocked() {
            return Err(AgentRefusal::Locked);
        }
        {
            let registry = lock(&self.registry);
            if registry.is_unreadable() {
                // Nothing could be saved: do not ask the user.
                return Err(AgentRefusal::Internal);
            }
            if registry.list().len() >= super::MAX_PAIRINGS {
                return Err(AgentRefusal::PairingLimit);
            }
            if registry.name_taken(name) {
                return Err(AgentRefusal::PairingNameTaken);
            }
        }
        let mut prompts = lock(&self.prompts);
        if prompts.is_blocked(now) {
            return Err(AgentRefusal::PairingBlocked);
        }
        if prompts.pair.is_some() {
            return Err(AgentRefusal::Busy);
        }
        if !prompts.pair_rate_ok(now) {
            return Err(AgentRefusal::RateLimited);
        }
        let request_id = new_request_id().ok_or(AgentRefusal::Internal)?;
        let (reply, rx) = oneshot::channel();
        prompts.pair = Some(PendingPair {
            request_id: request_id.clone(),
            name: name.to_owned(),
            shown_at: now,
            reply,
        });
        drop(prompts);
        let requested_at = wall_ms();
        self.emit(AppEvent::AgentPairingRequested {
            request: AgentPairingRequest {
                request_id: request_id.clone(),
                name: name.to_owned(),
                code: code.to_owned(),
                requested_at: unix_millis_of(requested_at),
                expires_at: expires_millis(requested_at, self.limits.pair_timeout),
            },
        });
        // The window comes forward so the dialog cannot sit unseen (B3).
        if let Some(backend) = self.backend() {
            backend.focus_window();
        }
        Ok((request_id, rx))
    }

    /// Waits for the user's answer, the timeout, or the prompt closing.
    pub(crate) async fn await_pair(
        &self,
        request_id: &str,
        mut rx: oneshot::Receiver<PairOutcome>,
    ) -> PairOutcome {
        match tokio::time::timeout(self.limits.pair_timeout, &mut rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(AgentRefusal::Internal),
            Err(_) => {
                if self.close_pair(request_id, AgentCloseReason::Expired) {
                    Err(AgentRefusal::PairingExpired)
                } else {
                    // Answered in the same instant.
                    rx.try_recv().unwrap_or(Err(AgentRefusal::PairingExpired))
                }
            }
        }
    }

    /// Ends the pairing prompt `request_id` without an answer. `false`: it was not open.
    pub(crate) fn close_pair(&self, request_id: &str, reason: AgentCloseReason) -> bool {
        let mut prompts = lock(&self.prompts);
        if prompts
            .pair
            .as_ref()
            .is_none_or(|p| p.request_id != request_id)
        {
            return false;
        }
        let Some(pending) = prompts.pair.take() else {
            return false;
        };
        drop(prompts);
        drop(pending.reply);
        self.emit(AppEvent::AgentPairingClosed {
            request_id: request_id.to_owned(),
            reason,
        });
        true
    }

    /// `agent_pairing_respond`.
    ///
    /// # Errors
    /// `agentRequestExpired`, `agentAllowTooSoon` (the request stays open), `internal`.
    pub(crate) fn respond_pair(
        &self,
        backend: &Backend,
        request: &AgentPairingRespondRequest,
    ) -> IpcResult<()> {
        let now = self.now();
        let mut prompts = lock(&self.prompts);
        let open = prompts
            .pair
            .as_ref()
            .filter(|p| p.request_id == request.request_id)
            .ok_or(IpcError::new(ErrorCode::AgentRequestExpired))?;
        if request.decision == PairingDecision::Deny {
            let Some(pending) = prompts.pair.take() else {
                return Err(IpcError::INTERNAL);
            };
            prompts.note_denial(now);
            drop(prompts);
            let _ = pending.reply.send(Err(AgentRefusal::PairingDenied));
            self.closed_pair_event(&request.request_id, AgentCloseReason::Answered);
            return Ok(());
        }
        // An Allow needs an unlocked, switched-on app (a lock closes the prompt first; this is
        // the belt to that braces) and one second of looking.
        if !self.is_unlocked() || !self.is_enabled() {
            return Err(IpcError::new(ErrorCode::AgentRequestExpired));
        }
        if too_soon(open.shown_at, now) {
            return Err(IpcError::new(ErrorCode::AgentAllowTooSoon));
        }
        let name = open.name.clone();
        let added = lock(&self.registry).add(name.clone(), unix_millis_of(wall_ms()));
        let (id, key) = added.map_err(|e| {
            IpcError::new(match e {
                super::RegistryError::Full
                | super::RegistryError::NameTaken
                | super::RegistryError::Unreadable => ErrorCode::InvalidInput,
                super::RegistryError::Storage => ErrorCode::Internal,
            })
        })?;
        let Some(pending) = prompts.pair.take() else {
            let _ = lock(&self.registry).remove(&id);
            return Err(IpcError::INTERNAL);
        };
        drop(prompts);
        if pending.reply.send(Ok((id, key))).is_err() {
            // The client hung up between the dialog and the answer: nobody holds the key.
            let _ = lock(&self.registry).remove(&id);
        } else {
            Self::log_paired(backend, &id, &name);
        }
        self.closed_pair_event(&request.request_id, AgentCloseReason::Answered);
        self.state_changed();
        Ok(())
    }

    fn closed_pair_event(&self, request_id: &str, reason: AgentCloseReason) {
        self.emit(AppEvent::AgentPairingClosed {
            request_id: request_id.to_owned(),
            reason,
        });
    }

    // ----- access --------------------------------------------------------------------------

    /// Queues the access prompt for `facts`; the first of the queue is shown at once.
    ///
    /// # Errors
    /// Whether this client has an access prompt open or queued already (a new one would be
    /// refused `busy`, so it must not count against the dialog budget).
    pub(crate) fn access_pending_for(&self, pairing: &[u8; 16]) -> bool {
        lock(&self.prompts)
            .access
            .iter()
            .any(|p| p.pairing == *pairing)
    }

    /// `busy` (this client has a prompt open or queued), `internal`.
    pub(crate) fn enqueue_access(
        &self,
        facts: AccessFacts,
    ) -> Result<(String, oneshot::Receiver<AccessOutcome>), AgentErrorCode> {
        let now = self.now();
        let request_id = new_request_id().ok_or(AgentErrorCode::Internal)?;
        let (reply, rx) = oneshot::channel();
        let mut prompts = lock(&self.prompts);
        if prompts.access.iter().any(|p| p.pairing == facts.pairing.id) {
            return Err(AgentErrorCode::Busy);
        }
        let first = prompts.access.is_empty();
        prompts.access.push_back(PendingAccess {
            request_id: request_id.clone(),
            pairing: facts.pairing.id,
            facts,
            requested_wall: wall_ms(),
            shown_at: first.then_some(now),
            reply,
        });
        let shown = Self::front_event(&prompts, self.limits.access_timeout);
        drop(prompts);
        if let Some(event) = shown {
            self.emit(event);
            if first && let Some(backend) = self.backend() {
                backend.focus_window();
            }
        }
        Ok((request_id, rx))
    }

    /// The event that shows the front of the queue (with the number of requests behind it).
    fn front_event(prompts: &Prompts, limit: Duration) -> Option<AppEvent> {
        let front = prompts.access.front()?;
        let facts = &front.facts;
        Some(AppEvent::AgentAccessRequested {
            request: AgentAccessRequest {
                request_id: front.request_id.clone(),
                pairing_name: facts.pairing.name.clone(),
                item_title: facts.item_title.clone(),
                vault_name: facts.vault_name.clone(),
                field_label: facts.field_label.clone(),
                origins: facts.origins.clone(),
                inject: inject_of(facts.inject),
                reason: facts.reason.clone(),
                waiting: u32::try_from(prompts.access.len().saturating_sub(1)).unwrap_or(u32::MAX),
                requested_at: unix_millis_of(front.requested_wall),
                expires_at: expires_millis(front.requested_wall, limit),
            },
        })
    }

    /// Waits for the user's answer, the timeout (queue time included), or the prompt closing.
    pub(crate) async fn await_access(
        &self,
        request_id: &str,
        mut rx: oneshot::Receiver<AccessOutcome>,
    ) -> AccessOutcome {
        match tokio::time::timeout(self.limits.access_timeout, &mut rx).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(AgentErrorCode::Internal),
            Err(_) => {
                if self.close_access(request_id, AgentCloseReason::Expired, None) {
                    Ok(GrantChoice::Timeout)
                } else {
                    // Answered in the same instant.
                    rx.try_recv().unwrap_or(Ok(GrantChoice::Timeout))
                }
            }
        }
    }

    /// Ends the access prompt `request_id` without an answer, wherever it is in the queue.
    /// `told` is the code the waiting request is answered with (`None`: nobody is waiting).
    /// `false`: it was not open.
    pub(crate) fn close_access(
        &self,
        request_id: &str,
        reason: AgentCloseReason,
        told: Option<AgentErrorCode>,
    ) -> bool {
        let mut prompts = lock(&self.prompts);
        let Some(at) = prompts
            .access
            .iter()
            .position(|p| p.request_id == request_id)
        else {
            return false;
        };
        let was_front = at == 0;
        let Some(removed) = prompts.access.remove(at) else {
            return false;
        };
        let promoted = self.promote(&mut prompts, was_front);
        drop(prompts);
        if let Some(code) = told {
            let _ = removed.reply.send(Err(code));
        }
        self.emit(AppEvent::AgentAccessClosed {
            request_id: request_id.to_owned(),
            reason,
        });
        if let Some(event) = promoted {
            self.emit(event);
        }
        true
    }

    /// After a removal: a new front starts its second now; a changed queue length is shown.
    fn promote(&self, prompts: &mut Prompts, was_front: bool) -> Option<AppEvent> {
        if was_front && let Some(next) = prompts.access.front_mut() {
            next.shown_at = Some(self.now());
        }
        Self::front_event(prompts, self.limits.access_timeout)
    }

    /// `agent_access_respond`.
    ///
    /// # Errors
    /// `agentRequestExpired` (also for a request that is queued but not shown), and
    /// `agentAllowTooSoon` (the request stays open).
    pub(crate) fn respond_access(&self, request: &AgentAccessRespondRequest) -> IpcResult<()> {
        let now = self.now();
        let mut prompts = lock(&self.prompts);
        let front = prompts
            .access
            .front()
            .filter(|p| p.request_id == request.request_id)
            .ok_or(IpcError::new(ErrorCode::AgentRequestExpired))?;
        let allow = request.decision != AgentAccessDecision::Deny;
        if allow {
            if !self.is_unlocked() || !self.is_enabled() {
                return Err(IpcError::new(ErrorCode::AgentRequestExpired));
            }
            if front.shown_at.is_none_or(|shown| too_soon(shown, now)) {
                return Err(IpcError::new(ErrorCode::AgentAllowTooSoon));
            }
        }
        let Some(answered) = prompts.access.pop_front() else {
            return Err(IpcError::INTERNAL);
        };
        let promoted = self.promote(&mut prompts, true);
        drop(prompts);
        let _ = answered.reply.send(Ok(choice_of(request.decision)));
        self.emit(AppEvent::AgentAccessClosed {
            request_id: request.request_id.clone(),
            reason: AgentCloseReason::Answered,
        });
        if let Some(event) = promoted {
            self.emit(event);
        }
        Ok(())
    }

    /// Ends the queued prompts of one client (its pairing was removed).
    pub(crate) fn drop_access_of(&self, pairing: &[u8; 16]) {
        let ids: Vec<String> = lock(&self.prompts)
            .access
            .iter()
            .filter(|p| p.pairing == *pairing)
            .map(|p| p.request_id.clone())
            .collect();
        for id in ids {
            self.close_access(&id, AgentCloseReason::Off, Some(AgentErrorCode::NotPaired));
        }
    }

    /// Ends every prompt (lock, agent access switched off, the app closing). Nobody is told
    /// "allowed"; the waiting requests are refused with the matching code.
    pub(crate) fn drop_prompts(&self, reason: AgentCloseReason) {
        let (pair, access) = {
            let mut prompts = lock(&self.prompts);
            (
                prompts.pair.take(),
                std::mem::take(&mut prompts.access)
                    .into_iter()
                    .collect::<Vec<_>>(),
            )
        };
        let (refusal, code) = if reason == AgentCloseReason::Locked {
            (AgentRefusal::Locked, AgentErrorCode::Locked)
        } else {
            (AgentRefusal::AgentAccessOff, AgentErrorCode::AgentAccessOff)
        };
        if let Some(pending) = pair {
            let _ = pending.reply.send(Err(refusal));
            self.closed_pair_event(&pending.request_id, reason);
        }
        for pending in access {
            let _ = pending.reply.send(Err(code));
            self.emit(AppEvent::AgentAccessClosed {
                request_id: pending.request_id,
                reason,
            });
        }
    }
}

/// Closes a prompt when the waiting request goes away (the client hung up): the dialog must not
/// stay open for a request nobody waits for.
pub(super) struct CloseOnDrop {
    pub(super) shared: std::sync::Arc<Shared>,
    pub(super) request_id: String,
    pub(super) kind: PromptKind,
}

pub(super) enum PromptKind {
    Pair,
    Access,
}

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        match self.kind {
            PromptKind::Pair => {
                self.shared
                    .close_pair(&self.request_id, AgentCloseReason::Disconnected);
            }
            PromptKind::Access => {
                self.shared
                    .close_access(&self.request_id, AgentCloseReason::Disconnected, None);
            }
        }
    }
}
