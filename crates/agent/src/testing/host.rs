//! A host for the server loop in tests: the real policy engine over a fake secret source, the
//! HTTPS sender, an answer policy that stands in for the access dialog, and an in-memory log.
//!
//! The canary value of the source never leaves the engine except as the injected header.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mosaic_core::browser_channel::{PairingKey, new_pairing_id};
use mosaic_core::model::{AgentBindingRecord, AgentInject, ElementId};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_native_messaging::agent_preauth::AgentRefusal;

use crate::binding::Binding;
use crate::engine::{AccessCheck, AccessOutcome, Engine, GrantChoice, Now, PairingRef};
use crate::error::AgentErrorCode;
use crate::grants::GrantState;
use crate::http::HttpsSender;
use crate::log::LogEntry;
use crate::request::HttpRequestInput;
use crate::server::{AgentHost, status_response};
use crate::source::{ResolveError, SecretSource, SourceBinding};
use crate::wire::{
    AccessBody, AccessDecision, AccessState, HttpBody, OpRequest, OpResponse, SecretsBody,
    VaultState, WireError, WireSecret,
};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One bearer-injected binding to one origin, with a known value.
pub struct FakeSource {
    binding: SourceBinding,
    value: String,
}

impl FakeSource {
    /// A source with one binding to `origin` (for example `https://api.mosaic.test:PORT`) whose
    /// value is `value` (at least 12 bytes).
    ///
    /// # Panics
    /// If the origin or the IDs are not valid (a test setup error).
    #[must_use]
    #[expect(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "test fixture: a bad setup must stop the test"
    )]
    pub fn new(origin: &str, value: &str) -> Self {
        let record = AgentBindingRecord::new("api_key.secret", &[origin], AgentInject::Bearer);
        Self {
            binding: SourceBinding {
                vault_id: VaultId::new_v7().unwrap(),
                item_id: ItemId::new_v7().unwrap(),
                element_id: ElementId::from_stored("b1").unwrap(),
                vault_name: "Work".to_owned(),
                item_title: "Example API".to_owned(),
                field_label: "api_key.secret".to_owned(),
                binding: Binding::from_record(&record).expect("a valid binding"),
                confirmed: true,
            },
            value: value.to_owned(),
        }
    }

    /// The reference of the binding.
    #[must_use]
    pub fn reference(&self) -> String {
        format!(
            "mosaic://{}/{}/api_key.secret",
            self.binding.vault_id, self.binding.item_id
        )
    }
}

impl SecretSource for FakeSource {
    fn usable_bindings(&self) -> Vec<SourceBinding> {
        vec![self.binding.clone()]
    }

    fn resolve(&self, reference: &str) -> Result<SourceBinding, ResolveError> {
        if !reference.starts_with("mosaic://") {
            return Err(ResolveError::Invalid);
        }
        if reference == self.reference() {
            Ok(self.binding.clone())
        } else {
            Err(ResolveError::NotUsable)
        }
    }

    fn read_value(&self, _: &SourceBinding) -> Result<SecretString, AgentErrorCode> {
        SecretString::from_str_copy(&self.value).map_err(|_| AgentErrorCode::Internal)
    }

    fn basic_user(&self, _: &SourceBinding) -> Result<Option<String>, AgentErrorCode> {
        Ok(None)
    }
}

type Pairings = HashMap<[u8; 16], (String, Arc<PairingKey>)>;

/// The host: engine, source, sender, answer policy and log.
pub struct TestHost {
    state: Mutex<(VaultState, AccessState)>,
    pairings: Mutex<Pairings>,
    engine: Mutex<Engine>,
    source: FakeSource,
    sender: Arc<HttpsSender>,
    answer: Mutex<GrantChoice>,
    log: Mutex<Vec<LogEntry>>,
    t0: Instant,
}

impl TestHost {
    /// A host that answers every access dialog with `answer`.
    #[must_use]
    pub fn new(source: FakeSource, sender: HttpsSender, answer: GrantChoice) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new((VaultState::Unlocked, AccessState::On)),
            pairings: Mutex::new(HashMap::new()),
            engine: Mutex::new(Engine::new()),
            source,
            sender: Arc::new(sender),
            answer: Mutex::new(answer),
            log: Mutex::new(Vec::new()),
            t0: Instant::now(),
        })
    }

    /// The log entries so far.
    #[must_use]
    pub fn log(&self) -> Vec<LogEntry> {
        lock(&self.log).clone()
    }

    /// The reference of the one binding.
    #[must_use]
    pub fn reference(&self) -> String {
        self.source.reference()
    }

    /// Pre-seeds a pairing named `name`: returns its ID and a key equal to the stored one (for
    /// the client's keystore).
    ///
    /// # Panics
    /// If the OS random generator fails.
    #[must_use]
    #[expect(clippy::unwrap_used, reason = "test fixture")]
    pub fn pair_with(&self, name: &str) -> ([u8; 16], PairingKey) {
        let id = new_pairing_id().unwrap();
        let key = PairingKey::generate().unwrap();
        let copy = PairingKey::from_bytes(&mut { *key.expose_secret() });
        lock(&self.pairings).insert(id, (name.to_owned(), Arc::new(copy)));
        (id, key)
    }

    fn now(&self) -> Now {
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Now {
            mono: Instant::now().max(self.t0),
            wall_ms: wall,
        }
    }

    fn push(&self, entry: LogEntry) {
        lock(&self.log).push(entry);
    }

    fn pairing(&self, id: &[u8; 16]) -> PairingRef {
        PairingRef {
            id: *id,
            name: lock(&self.pairings)
                .get(id)
                .map(|(n, _)| n.clone())
                .unwrap_or_default(),
        }
    }

    fn access(
        &self,
        pairing: &PairingRef,
        reference: &str,
        reason: Option<&str>,
    ) -> Result<OpResponse, WireError> {
        let now = self.now();
        let mut engine = lock(&self.engine);
        let outcome = match engine
            .access_check(&self.source, pairing, reference, reason, now)
            .map_err(WireError::from)?
        {
            AccessCheck::Granted(state) => AccessOutcome::Granted(state),
            AccessCheck::NeedsDialog(facts) => {
                engine
                    .dialog_allowed(&pairing.id, now)
                    .map_err(WireError::from)?;
                let choice = *lock(&self.answer);
                let (outcome, entry) = engine.record_decision(&facts, choice, now);
                self.push(entry);
                outcome
            }
        };
        let (decision, grant) = match outcome {
            AccessOutcome::Granted(state) => (AccessDecision::Granted, state),
            AccessOutcome::Denied => (AccessDecision::Denied, GrantState::None),
            AccessOutcome::Timeout => (AccessDecision::Timeout, GrantState::None),
        };
        let expires_at = match grant {
            GrantState::Until(ms) => Some(ms),
            _ => None,
        };
        Ok(OpResponse::Access(AccessBody {
            decision,
            expires_at,
            grant,
        }))
    }

    async fn http(&self, pairing: &PairingRef, op: OpRequest) -> Result<OpResponse, WireError> {
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
        let begun =
            lock(&self.engine).request_begin(&self.source, pairing, &reference, &input, self.now());
        let (prepared, ticket) = match begun {
            Ok(pair) => pair,
            Err((failure, entry)) => {
                if let Some(entry) = entry {
                    self.push(entry);
                }
                return Err(failure.into());
            }
        };
        let holdback = ticket.holdback();
        let sender = Arc::clone(&self.sender);
        let sent = tokio::task::spawn_blocking(move || sender.send(prepared, holdback))
            .await
            .unwrap_or(Err(crate::response::SendError::Other));
        let (result, entry) = lock(&self.engine).request_end(ticket, sent, self.now());
        self.push(entry);
        result
            .map(|body| OpResponse::Http(HttpBody::from(body)))
            .map_err(WireError::code)
    }
}

impl AgentHost for TestHost {
    fn state(&self) -> (VaultState, AccessState) {
        *lock(&self.state)
    }

    fn pairing_key(&self, id: &[u8; 16]) -> Option<Arc<PairingKey>> {
        lock(&self.pairings).get(id).map(|(_, k)| Arc::clone(k))
    }

    fn pair(
        &self,
        _name: String,
        _code: String,
    ) -> impl Future<Output = Result<([u8; 16], PairingKey), AgentRefusal>> + Send {
        std::future::ready(Err(AgentRefusal::PairingDenied))
    }

    async fn handle(
        &self,
        pairing: [u8; 16],
        _conn: u64,
        op: OpRequest,
    ) -> Result<OpResponse, WireError> {
        let (vault, access) = self.state();
        if matches!(op, OpRequest::Status {}) {
            return Ok(status_response(vault, access));
        }
        if vault == VaultState::Locked {
            return Err(WireError::code(AgentErrorCode::Locked));
        }
        let pairing = self.pairing(&pairing);
        match op {
            OpRequest::Status {} => Err(WireError::code(AgentErrorCode::Internal)),
            OpRequest::ListSecrets { query } => {
                let entries = lock(&self.engine).list(
                    &self.source,
                    &pairing.id,
                    query.as_deref(),
                    self.now(),
                );
                Ok(OpResponse::Secrets(SecretsBody {
                    secrets: entries.into_iter().map(WireSecret::from).collect(),
                }))
            }
            OpRequest::RequestAccess {
                reference, reason, ..
            } => self.access(&pairing, &reference, reason.as_deref()),
            op @ OpRequest::HttpRequest { .. } => self.http(&pairing, op).await,
        }
    }

    fn note_used(&self, _pairing: &[u8; 16]) {}
}
