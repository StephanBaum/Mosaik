//! The host of the agent endpoint: the ops an authenticated client may call, wired to the policy
//! engine of `mosaic-agent` (ADR-0159).
//!
//! Every op first checks that agent access is on and the vault is unlocked (`status` answers
//! while locked). Nothing here calls `Backend::touch`. The session slot is held only while the
//! engine reads bindings (`VaultSource` is read-only) and while a log entry is written, never
//! while a request is being sent. The value is read once, inside `Engine::request_begin`, after
//! every check passed; this module never sees it.

use std::future::Future;
use std::sync::Arc;

use mosaic_agent::binding::BindingField;
use mosaic_agent::engine::{AccessCheck, AccessOutcome, GrantChoice, PairingRef};
use mosaic_agent::error::AgentErrorCode;
use mosaic_agent::grants::GrantState;
use mosaic_agent::log::{LogEntry, LogKind, hex16};
use mosaic_agent::request::HttpRequestInput;
use mosaic_agent::server::{AgentHost, status_response};
use mosaic_agent::source::SecretSource;
use mosaic_agent::vault_source::VaultSource;
use mosaic_agent::wire::{
    AccessBody, AccessDecision, AccessState, AccessUse, Event, HttpBody, OpRequest, OpResponse,
    SecretsBody, VaultState, WireError, WireSecret,
};
use mosaic_core::browser_channel::PairingKey;
use mosaic_native_messaging::agent_preauth::AgentRefusal;

use super::prompts::{CloseOnDrop, PromptKind};
use super::{Shared, lock, unix_millis_of, wall_ms};
use crate::browser::hex;
use crate::contract::agent_access::{AgentGrantState, AgentGrantView};
use crate::state::Backend;

/// The host the endpoint's server loop calls.
pub struct DesktopHost {
    shared: Arc<Shared>,
}

impl DesktopHost {
    pub(super) const fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

/// Counts a request being sent, for the in-use indicator; the count drops with this guard.
struct InFlight(Arc<Shared>);

impl InFlight {
    fn start(shared: &Arc<Shared>) -> Self {
        shared.bump_in_flight(true);
        Self(Arc::clone(shared))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.bump_in_flight(false);
    }
}

impl Shared {
    fn bump_in_flight(&self, up: bool) {
        use std::sync::atomic::Ordering;
        let now = if up {
            self.in_flight
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1)
        } else {
            self.in_flight
                .fetch_sub(1, Ordering::SeqCst)
                .saturating_sub(1)
        };
        self.emit(crate::contract::events::AppEvent::AgentActivity { in_flight: now });
    }

    /// Runs `work` with the unlocked session (the slot is held for the call).
    fn with_session<T>(
        &self,
        work: impl FnOnce(&mut mosaic_vault::session::UnlockedSession) -> T,
    ) -> Result<T, AgentErrorCode> {
        let backend = self.backend().ok_or(AgentErrorCode::Internal)?;
        let mut slot = backend.slot().map_err(|_| AgentErrorCode::Internal)?;
        let session = slot.unlocked_mut().map_err(|_| AgentErrorCode::Locked)?;
        Ok(work(session))
    }

    /// Seals one log entry into the agent log of the open vault.
    fn append(
        session: &mut mosaic_vault::session::UnlockedSession,
        entry: &LogEntry,
    ) -> Result<(), AgentErrorCode> {
        session
            .append_agent_log_class(entry.at_ms, &entry.to_json(), entry.is_noisy())
            .map_err(|_| AgentErrorCode::Internal)
    }

    /// Writes an entry from outside a request (best effort: nothing is logged while locked).
    pub(crate) fn log_entry(backend: &Backend, entry: &LogEntry) {
        if let Ok(mut slot) = backend.slot()
            && let Ok(session) = slot.unlocked_mut()
        {
            let _ = Self::append(session, entry);
        }
    }

    /// The log entry of a new pairing.
    pub(crate) fn log_paired(backend: &Backend, id: &[u8; 16], name: &str) {
        let mut entry = LogEntry::new(LogKind::Paired, wall_ms());
        entry.pairing_id = Some(hex16(id));
        entry.pairing_name = Some(name.to_owned());
        Self::log_entry(backend, &entry);
    }

    fn pairing_ref(&self, id: &[u8; 16]) -> Option<PairingRef> {
        lock(&self.registry).get(id).map(|p| PairingRef {
            id: *id,
            name: p.name.clone(),
        })
    }

    // ----- ops -----------------------------------------------------------------------------

    fn op_list(&self, pairing: &PairingRef, query: Option<&str>) -> Result<OpResponse, WireError> {
        let entries = self.with_session(|session| {
            let src = VaultSource::new(session);
            lock(&self.engine).list(&src, &pairing.id, query, self.engine_now())
        })?;
        Ok(OpResponse::Secrets(SecretsBody {
            secrets: entries.into_iter().map(WireSecret::from).collect(),
        }))
    }

    /// `requestAccess`, first half. `Ok(None)`: a live grant exists, `state` says which.
    fn access_check(
        &self,
        pairing: &PairingRef,
        reference: &str,
        reason: Option<&str>,
    ) -> Result<AccessCheck, WireError> {
        let checked = self.with_session(|session| {
            let src = VaultSource::new(session);
            lock(&self.engine).access_check(&src, pairing, reference, reason, self.engine_now())
        })?;
        let check = checked.map_err(WireError::from)?;
        if matches!(check, AccessCheck::NeedsDialog(_)) {
            // A client with a prompt open is refused `busy` without spending the dialog budget.
            if self.access_pending_for(&pairing.id) {
                return Err(WireError::code(AgentErrorCode::Busy));
            }
            lock(&self.engine)
                .dialog_allowed(&pairing.id, self.engine_now())
                .map_err(WireError::code)?;
        }
        Ok(check)
    }

    /// `requestAccess`, second half: records what the user chose. The slot is held, and must be
    /// unlocked, while the grant is made, so a lock that already happened cannot be followed by a
    /// grant (the lock hook ends every grant after the slot locked).
    fn record_decision(
        &self,
        facts: &mosaic_agent::engine::AccessFacts,
        choice: GrantChoice,
    ) -> Result<AccessBody, AgentErrorCode> {
        let (outcome, entry) = self.with_session(|session| {
            let (outcome, entry) =
                lock(&self.engine).record_decision(facts, choice, self.engine_now());
            // A log that cannot be written is not a reason to forget what the user decided.
            let _ = Self::append(session, &entry);
            (outcome, entry)
        })?;
        drop(entry);
        let (decision, grant) = match outcome {
            AccessOutcome::Granted(state) => (AccessDecision::Granted, state),
            AccessOutcome::Denied => (AccessDecision::Denied, GrantState::None),
            AccessOutcome::Timeout => (AccessDecision::Timeout, GrantState::None),
        };
        let expires_at = match grant {
            GrantState::Until(ms) => Some(ms),
            _ => None,
        };
        self.state_changed();
        Ok(AccessBody {
            decision,
            expires_at,
            grant,
        })
    }

    async fn op_access(
        self: &Arc<Self>,
        pairing: PairingRef,
        reference: String,
        reason: Option<String>,
    ) -> Result<OpResponse, WireError> {
        let shared = Arc::clone(self);
        let reference_for_check = reference.clone();
        let pairing_for_check = pairing.clone();
        let check = tauri::async_runtime::spawn_blocking(move || {
            shared.access_check(&pairing_for_check, &reference_for_check, reason.as_deref())
        })
        .await
        .map_err(|_| WireError::code(AgentErrorCode::Internal))??;
        let facts = match check {
            AccessCheck::Granted(grant) => {
                let expires_at = match grant {
                    GrantState::Until(ms) => Some(ms),
                    _ => None,
                };
                return Ok(OpResponse::Access(AccessBody {
                    decision: AccessDecision::Granted,
                    expires_at,
                    grant,
                }));
            }
            AccessCheck::NeedsDialog(facts) => facts,
        };
        let (request_id, rx) = self
            .enqueue_access(facts.clone())
            .map_err(WireError::code)?;
        // The client hanging up (this future dropped) closes the dialog.
        let _close = CloseOnDrop {
            shared: Arc::clone(self),
            request_id: request_id.clone(),
            kind: PromptKind::Access,
        };
        let choice = self
            .await_access(&request_id, rx)
            .await
            .map_err(WireError::code)?;
        let shared = Arc::clone(self);
        let body =
            tauri::async_runtime::spawn_blocking(move || shared.record_decision(&facts, choice))
                .await
                .map_err(|_| WireError::code(AgentErrorCode::Internal))?
                .map_err(WireError::code)?;
        Ok(OpResponse::Access(body))
    }

    /// `httpRequest`: every check and the one read of the value happen in `request_begin`, the
    /// slot is released before the request is sent, and the entry is written before the answer
    /// is handed back (a log that cannot be written drops the answer).
    fn op_http(&self, pairing: &PairingRef, op: OpRequest) -> Result<OpResponse, WireError> {
        let OpRequest::HttpRequest {
            reference,
            method,
            url,
            headers,
            body,
            max_bytes,
        } = op
        else {
            return Err(WireError::code(AgentErrorCode::Internal));
        };
        let input = HttpRequestInput {
            method,
            url,
            headers: headers.unwrap_or_default().into_iter().collect(),
            body,
            max_bytes,
        };
        let begun = self.with_session(|session| {
            let begun = {
                let src = VaultSource::new(session);
                lock(&self.engine).request_begin(
                    &src,
                    pairing,
                    &reference,
                    &input,
                    self.engine_now(),
                )
            };
            match begun {
                Ok(pair) => Ok(pair),
                Err((failure, entry)) => {
                    if let Some(entry) = entry {
                        let _ = Self::append(session, &entry);
                    }
                    Err(failure)
                }
            }
        })?;
        let (prepared, ticket) = begun.map_err(WireError::from)?;
        let Some(sender) = self.sender() else {
            return Err(WireError::code(AgentErrorCode::Internal));
        };
        let holdback = ticket.holdback();
        let sent = sender.send(prepared, holdback);
        let (result, entry) = lock(&self.engine).request_end(ticket, sent, self.engine_now());
        self.with_session(|session| Self::append(session, &entry))??;
        result
            .map(|body| OpResponse::Http(HttpBody::from(body)))
            .map_err(WireError::code)
    }

    /// The live grants, named from the open vault (an empty list while locked).
    pub(crate) fn grant_views(&self, backend: &Backend) -> Vec<AgentGrantView> {
        let grants = lock(&self.engine).grants(self.engine_now());
        if grants.is_empty() {
            return Vec::new();
        }
        let Ok(slot) = backend.slot() else {
            return Vec::new();
        };
        let Ok(session) = slot.unlocked() else {
            return Vec::new();
        };
        let bindings = VaultSource::new(session).usable_bindings();
        let registry = lock(&self.registry);
        grants
            .into_iter()
            .filter_map(|g| {
                let b = bindings
                    .iter()
                    .find(|b| b.item_id == g.item_id && b.element_id == g.element_id)?;
                let pairing = registry.get(&g.pairing)?;
                let (state, expires_at) = match g.state {
                    GrantState::Once => (AgentGrantState::Once, None),
                    GrantState::Until(ms) => (AgentGrantState::Until, Some(unix_millis_of(ms))),
                    GrantState::UntilLock => (AgentGrantState::UntilLock, None),
                    GrantState::None => return None,
                };
                Some(AgentGrantView {
                    grant_id: g.grant_id,
                    pairing_id: hex(&g.pairing),
                    pairing_name: pairing.name.clone(),
                    item_title: b.item_title.clone(),
                    vault_name: b.vault_name.clone(),
                    field_label: b.field_label.clone(),
                    state,
                    expires_at,
                })
            })
            .collect()
    }

    /// The pairing, the `mosaic://` reference and a log entry (without its kind) for a grant,
    /// for `grantRevoked` and the log. `None`: the grant ended already.
    pub(crate) fn grant_info(
        &self,
        backend: &Backend,
        grant_id: u64,
    ) -> Option<([u8; 16], String, LogEntry)> {
        let grants = lock(&self.engine).grants(self.engine_now());
        let grant = grants.into_iter().find(|g| g.grant_id == grant_id)?;
        let name = lock(&self.registry)
            .get(&grant.pairing)
            .map(|p| p.name.clone())?;
        let slot = backend.slot().ok()?;
        let session = slot.unlocked().ok()?;
        let b = VaultSource::new(session)
            .usable_bindings()
            .into_iter()
            .find(|b| b.item_id == grant.item_id && b.element_id == grant.element_id)?;
        let field = match &b.binding.field {
            BindingField::Secret(field) => field.key().to_owned(),
            BindingField::Custom(id) => format!("field:{id}"),
        };
        let mut entry = LogEntry::new(LogKind::Revoked, wall_ms());
        entry.pairing_id = Some(hex16(&grant.pairing));
        entry.pairing_name = Some(name);
        entry.vault_id = Some(b.vault_id.to_string());
        entry.item_id = Some(b.item_id.to_string());
        entry.item_title = Some(b.item_title.clone());
        entry.field_label = Some(b.field_label.clone());
        entry.grant = Some(grant.state);
        Some((
            grant.pairing,
            format!("mosaic://{}/{}/{field}", b.vault_id, b.item_id),
            entry,
        ))
    }

    /// The paired client's `grantRevoked` event for a revoked grant.
    pub(crate) fn tell_revoked(&self, pairing: &[u8; 16], reference: String) {
        if let Some(server) = self.server() {
            server.send_to_pairing(pairing, &Event::GrantRevoked { reference });
        }
    }

    /// Ends a pairing's connections, grants and dialogs (it was removed).
    pub(crate) fn end_pairing(&self, pairing: &[u8; 16]) {
        if let Some(server) = self.server() {
            server.close_pairing(pairing);
        }
        lock(&self.engine).revoke_pairing(pairing);
        self.drop_access_of(pairing);
    }
}

impl AgentHost for DesktopHost {
    fn state(&self) -> (VaultState, AccessState) {
        self.shared.wire_state()
    }

    fn pairing_key(&self, id: &[u8; 16]) -> Option<Arc<PairingKey>> {
        lock(&self.shared.registry)
            .get(id)
            .map(|p| Arc::clone(&p.key))
    }

    async fn pair(
        &self,
        name: String,
        code: String,
    ) -> Result<([u8; 16], PairingKey), AgentRefusal> {
        let (request_id, rx) = self.shared.open_pair(&name, &code)?;
        // The client hanging up (this future dropped) closes the dialog.
        let _close = CloseOnDrop {
            shared: Arc::clone(&self.shared),
            request_id: request_id.clone(),
            kind: PromptKind::Pair,
        };
        self.shared.await_pair(&request_id, rx).await
    }

    fn handle(
        &self,
        pairing: [u8; 16],
        _conn: u64,
        op: OpRequest,
    ) -> impl Future<Output = Result<OpResponse, WireError>> + Send {
        let shared = Arc::clone(&self.shared);
        async move {
            let (vault, access) = shared.wire_state();
            if matches!(op, OpRequest::Status {}) {
                return Ok(status_response(vault, access));
            }
            if access == AccessState::Off {
                return Err(WireError::code(AgentErrorCode::AgentAccessOff));
            }
            if vault == VaultState::Locked {
                return Err(WireError::code(AgentErrorCode::Locked));
            }
            let Some(pairing) = shared.pairing_ref(&pairing) else {
                return Err(WireError::code(AgentErrorCode::NotPaired));
            };
            match op {
                OpRequest::Status {} => Ok(status_response(vault, access)),
                OpRequest::ListSecrets { query } => {
                    tauri::async_runtime::spawn_blocking(move || {
                        shared.op_list(&pairing, query.as_deref())
                    })
                    .await
                    .map_err(|_| WireError::code(AgentErrorCode::Internal))?
                }
                OpRequest::RequestAccess {
                    reference,
                    access_use: AccessUse::Request,
                    reason,
                } => shared.op_access(pairing, reference, reason).await,
                op @ OpRequest::HttpRequest { .. } => {
                    let blocking = Arc::clone(&shared);
                    let _active = InFlight::start(&shared);
                    tauri::async_runtime::spawn_blocking(move || blocking.op_http(&pairing, op))
                        .await
                        .map_err(|_| WireError::code(AgentErrorCode::Internal))?
                }
            }
        }
    }

    fn note_used(&self, pairing: &[u8; 16]) {
        lock(&self.shared.registry).note_used(pairing, unix_millis_of(wall_ms()));
    }

    fn pairing_undelivered(&self, pairing: &[u8; 16]) {
        // Nobody holds the key: forget the pairing.
        let _ = lock(&self.shared.registry).remove(pairing);
        self.shared.state_changed();
    }
}
