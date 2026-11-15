//! The policy engine (agent access design §5.4, §7.2, §9): what an agent may do, decided without
//! any I/O.
//!
//! The engine holds grants and rate limits in memory and is driven by the host with the vault
//! side ([`SecretSource`]) and the clock ([`Now`]) passed in. It never reads a value before every
//! check that needs none has passed, hands the host a [`PreparedRequest`] to send, and masks the
//! answer before anything leaves it. Every decision yields a [`LogEntry`] for the agent log.
//!
//! Nothing here returns a secret: the only value is read in [`Engine::request_begin`], placed in
//! one slot of the request, and dropped before the function returns.

use core::fmt::{self, Write as _};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use mosaic_core::model::ElementId;
use mosaic_core::{ItemId, VaultId};
use mosaic_origin::parse_agent_url;
use mosaic_vault::refs::RefCandidate;
use serde::Serialize;

use crate::binding::{BindingField, Inject, InjectKind, Method, check_value};
use crate::error::AgentErrorCode;
use crate::grants::{Expiry, Found, GrantState, Grants};
use crate::log::{LogEntry, LogKind, hex16};
use crate::mask::Masker;
use crate::request::{HttpRequestInput, MAX_LOG_PATH_CHARS, PreparedRequest, validate};
use crate::response::{RawResponse, SendError};
use crate::source::{ResolveError, SecretSource, SourceBinding};

/// Most entries `listSecrets` returns.
pub const MAX_LIST_ENTRIES: usize = 50;
/// Longest `listSecrets` query, in characters.
pub const MAX_QUERY_CHARS: usize = 128;
/// Longest `reason` of an access request, in characters.
pub const MAX_REASON_CHARS: usize = 200;
/// Access dialogs per pairing per minute.
pub const DIALOGS_PER_MINUTE: usize = 3;
/// Denials within [`DENIAL_WINDOW`] that block a pairing's dialogs.
pub const DENIALS_BEFORE_BLOCK: usize = 3;
/// The window in which denials count.
pub const DENIAL_WINDOW: Duration = Duration::from_mins(10);
/// How long a pairing's dialogs stay blocked.
pub const DIALOG_BLOCK: Duration = Duration::from_mins(10);

/// Refusals of one pairing within a minute that are logged one by one; beyond it the answer is
/// `rateLimited` and one summary row a minute is logged.
pub const REFUSALS_PER_MINUTE: usize = 60;
/// Longest origin written to the log, in characters; a longer one is logged as the marker.
const MAX_LOG_ORIGIN_CHARS: usize = 256;
/// What the log holds instead of an origin that did not pass the agent URL checks.
pub const INVALID_ORIGIN_MARKER: &str = "invalid";

const MINUTE: Duration = Duration::from_mins(1);

/// The time, twice: the monotonic clock and the wall clock.
#[derive(Clone, Copy, Debug)]
pub struct Now {
    /// A monotonic instant (it pauses while the machine sleeps).
    pub mono: Instant,
    /// Milliseconds since the Unix epoch.
    pub wall_ms: u64,
}

/// A pairing as the engine knows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PairingRef {
    /// The pairing ID.
    pub id: [u8; 16],
    /// The name the client gave at pairing (not verified).
    pub name: String,
}

/// Why an agent operation failed, with the candidates of an ambiguous reference.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AgentFailure {
    /// The code.
    pub code: AgentErrorCode,
    /// Titles and IDs of usable items for [`AgentErrorCode::AmbiguousRef`], else empty.
    pub candidates: Vec<RefCandidate>,
}

impl AgentFailure {
    const fn code(code: AgentErrorCode) -> Self {
        Self {
            code,
            candidates: Vec::new(),
        }
    }
}

/// One line of `listSecrets`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretEntry {
    /// `mosaic://<vault uuid>/<item uuid>/<field key or field:<element id>>`.
    pub reference: String,
    /// The item's title.
    pub title: String,
    /// The vault's name.
    pub vault: String,
    /// The field's label.
    pub field: String,
    /// The destinations.
    pub origins: Vec<String>,
    /// How the value is sent.
    pub inject: InjectKind,
    /// The environment variable name for commands.
    pub env: Option<String>,
    /// Whether commands may receive the value.
    pub run_allowed: bool,
    /// This pairing's grant.
    pub grant: GrantState,
    /// The binding is not confirmed on this device.
    pub needs_review: bool,
}

/// What the dialog shows and what a grant will be bound to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AccessFacts {
    /// The pairing asking.
    pub pairing: PairingRef,
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// The binding.
    pub element_id: ElementId,
    /// The vault's name.
    pub vault_name: String,
    /// The item's title.
    pub item_title: String,
    /// The field's label.
    pub field_label: String,
    /// The destinations.
    pub origins: Vec<String>,
    /// How the value is sent.
    pub inject: InjectKind,
    /// The binding's digest: the grant is bound to it.
    pub digest: [u8; 32],
    /// The agent's reason, capped and escaped; shown as "The agent says".
    pub reason: Option<String>,
}

/// The result of [`Engine::access_check`].
#[derive(Clone, PartialEq, Eq, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "one value per access request; the facts are what the dialog shows"
)]
pub enum AccessCheck {
    /// A live grant exists.
    Granted(GrantState),
    /// The user must be asked (after [`Engine::dialog_allowed`]).
    NeedsDialog(AccessFacts),
}

/// What the user chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GrantChoice {
    /// Deny.
    Deny,
    /// Allow one request.
    Once,
    /// Allow for 15 minutes.
    Minutes15,
    /// Allow for 1 hour.
    Hour1,
    /// Allow until the vault locks.
    UntilLock,
    /// The dialog ran out (120 s).
    Timeout,
}

/// The decision handed back to the agent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessOutcome {
    /// Allowed.
    Granted(GrantState),
    /// Denied.
    Denied,
    /// The dialog timed out.
    Timeout,
}

/// A grant as shown in Settings.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GrantView {
    /// The grant, for [`Engine::revoke_grant`].
    pub grant_id: u64,
    /// The pairing.
    pub pairing: [u8; 16],
    /// The item.
    pub item_id: ItemId,
    /// The binding.
    pub element_id: ElementId,
    /// What it allows.
    pub state: GrantState,
}

/// What a request that was sent is waiting for (see [`Engine::request_end`]).
pub struct RequestTicket {
    masker: Masker,
    grant_id: Option<u64>,
    epoch: u64,
    facts: LogFacts,
    max_bytes: usize,
}

impl fmt::Debug for RequestTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl RequestTicket {
    /// The grant this request used (`None` for a once-grant).
    #[must_use]
    pub const fn grant_id(&self) -> Option<u64> {
        self.grant_id
    }

    /// The hold-back to pass to `HttpsSender::send`.
    #[must_use]
    pub fn holdback(&self) -> usize {
        self.masker.holdback()
    }
}

#[derive(Clone)]
struct LogFacts {
    pairing: PairingRef,
    vault_id: Option<VaultId>,
    item_id: Option<ItemId>,
    item_title: Option<String>,
    field_label: Option<String>,
    method: Option<String>,
    origin: Option<String>,
    path: Option<String>,
}

impl LogFacts {
    fn new(pairing: &PairingRef) -> Self {
        Self {
            pairing: pairing.clone(),
            vault_id: None,
            item_id: None,
            item_title: None,
            field_label: None,
            method: None,
            origin: None,
            path: None,
        }
    }

    /// Only an origin that passed the agent URL checks is logged; anything else (and anything
    /// oversized) is one fixed marker, so a hostile `url` cannot fill the log.
    fn set_url(&mut self, raw: &str) {
        let checked = parse_agent_url(raw)
            .ok()
            .map(|url| (url.origin().ascii_serialization(), url));
        match checked {
            Some((origin, url)) if origin.chars().count() <= MAX_LOG_ORIGIN_CHARS => {
                self.origin = Some(origin);
                self.path = Some(url.path().chars().take(MAX_LOG_PATH_CHARS).collect());
            }
            _ => self.origin = Some(INVALID_ORIGIN_MARKER.to_owned()),
        }
    }

    fn entry(&self, kind: LogKind, now: Now) -> LogEntry {
        let mut e = LogEntry::new(kind, now.wall_ms);
        e.pairing_id = Some(hex16(&self.pairing.id));
        e.pairing_name = Some(self.pairing.name.clone());
        e.vault_id = self.vault_id.map(|v| v.to_string());
        e.item_id = self.item_id.map(|i| i.to_string());
        e.item_title.clone_from(&self.item_title);
        e.field_label.clone_from(&self.field_label);
        e.method.clone_from(&self.method);
        e.origin.clone_from(&self.origin);
        e.path.clone_from(&self.path);
        e
    }
}

/// A response as the agent gets it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseBody {
    /// The status code.
    pub status: u16,
    /// Headers without `Set-Cookie`, masked.
    pub headers: Vec<(String, String)>,
    /// The masked text body; `None` for a binary body.
    pub body: Option<String>,
    /// What a binary body was.
    pub binary: Option<BinaryInfo>,
    /// The body was cut (the server's or at `maxBytes`).
    pub truncated: bool,
    /// How many replacements the masker made.
    pub masked: u32,
    /// The masked `Location` of a redirect (it is not followed).
    pub redirect: Option<String>,
}

/// A binary body, described and not returned.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryInfo {
    /// The `Content-Type`, masked.
    pub content_type: Option<String>,
    /// How many bytes were read.
    pub bytes: u64,
    /// The `Content-Encoding`, if other than `identity`: nothing is decoded.
    pub content_encoding: Option<String>,
}

#[derive(Default)]
struct PairingLimits {
    dialogs: VecDeque<Instant>,
    refusals: VecDeque<Instant>,
    last_refusal_summary: Option<Instant>,
    denials: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

/// The policy engine. No I/O.
#[derive(Default)]
pub struct Engine {
    grants: Grants,
    limits: HashMap<[u8; 16], PairingLimits>,
    epoch: u64,
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Engine")
    }
}

fn field_part(binding: &SourceBinding) -> String {
    match &binding.binding.field {
        BindingField::Secret(field) => field.key().to_owned(),
        BindingField::Custom(id) => format!("field:{id}"),
    }
}

fn reference_of(b: &SourceBinding) -> String {
    format!("mosaic://{}/{}/{}", b.vault_id, b.item_id, field_part(b))
}

/// Controls (`Cc`), format characters (`Cf`: bidirectional controls, zero-width characters, soft
/// hyphen, the byte-order mark, tags) and the line and paragraph separators (`Zl`, `Zp`).
fn is_hidden_char(c: char) -> bool {
    matches!(
        c,
        '\u{0}'..='\u{1f}'
            | '\u{7f}'..='\u{9f}'
            | '\u{ad}'
            | '\u{600}'..='\u{605}'
            | '\u{61c}'
            | '\u{6dd}'
            | '\u{70f}'
            | '\u{890}'..='\u{891}'
            | '\u{8e2}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{110bd}'
            | '\u{110cd}'
            | '\u{13430}'..='\u{1343f}'
            | '\u{1bca0}'..='\u{1bca3}'
            | '\u{1d173}'..='\u{1d17a}'
            | '\u{e0001}'
            | '\u{e0020}'..='\u{e007f}'
    )
}

/// Nonspacing marks (`Mn`) of the common scripts and the combining blocks. Not every `Mn` of
/// Unicode: the point is to stop a stack of marks that smears over the lines around it.
fn is_combining_mark(c: char) -> bool {
    matches!(
        c,
        '\u{300}'..='\u{36f}'
            | '\u{483}'..='\u{487}'
            | '\u{591}'..='\u{5bd}'
            | '\u{5bf}'
            | '\u{5c1}'..='\u{5c2}'
            | '\u{5c4}'..='\u{5c5}'
            | '\u{5c7}'
            | '\u{610}'..='\u{61a}'
            | '\u{64b}'..='\u{65f}'
            | '\u{670}'
            | '\u{6d6}'..='\u{6dc}'
            | '\u{6df}'..='\u{6e4}'
            | '\u{6e7}'..='\u{6e8}'
            | '\u{6ea}'..='\u{6ed}'
            | '\u{900}'..='\u{902}'
            | '\u{93a}'
            | '\u{93c}'
            | '\u{941}'..='\u{948}'
            | '\u{94d}'
            | '\u{951}'..='\u{957}'
            | '\u{962}'..='\u{963}'
            | '\u{e31}'
            | '\u{e34}'..='\u{e3a}'
            | '\u{e47}'..='\u{e4e}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20f0}'
            | '\u{3099}'..='\u{309a}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{fe20}'..='\u{fe2f}'
            | '\u{e0100}'..='\u{e01ef}'
    )
}

/// Most combining marks in a row that are shown as they are.
const MAX_MARK_RUN: usize = 2;

/// The agent's reason as the dialog may show it: at most 200 characters, every control, format,
/// bidirectional and line-separator character written out as `\u{..}` text, and every combining
/// mark beyond the second of a run too.
fn clean_reason(reason: &str) -> Option<String> {
    let mut out = String::new();
    let mut run = 0usize;
    for c in reason.chars().take(MAX_REASON_CHARS) {
        let mark = is_combining_mark(c);
        run = if mark { run + 1 } else { 0 };
        if is_hidden_char(c) || (mark && run > MAX_MARK_RUN) {
            let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
        } else {
            out.push(c);
        }
    }
    (!out.trim().is_empty()).then_some(out)
}

fn resolve_failure(error: ResolveError) -> AgentFailure {
    match error {
        ResolveError::Invalid => AgentFailure::code(AgentErrorCode::InvalidRef),
        ResolveError::NotUsable => AgentFailure::code(AgentErrorCode::NotAgentUsable),
        ResolveError::Ambiguous(candidates) => AgentFailure {
            code: AgentErrorCode::AmbiguousRef,
            candidates,
        },
    }
}

impl Engine {
    /// A new engine with no grants.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes whenever a grant is revoked or everything is stopped: a response whose request
    /// began under an older epoch is dropped.
    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    fn grant_state(&self, pairing: &[u8; 16], b: &SourceBinding, now: Now) -> GrantState {
        match self.grants.find(
            pairing,
            b.vault_id,
            b.item_id,
            &b.element_id,
            &b.binding.digest,
            now,
        ) {
            Found::Live(i) => self
                .grants
                .get(i)
                .map_or(GrantState::None, crate::grants::Grant::state),
            Found::Stale | Found::Missing => GrantState::None,
        }
    }

    /// `listSecrets`: up to 50 usable bindings, metadata only. Returns no value.
    #[must_use]
    pub fn list(
        &self,
        src: &dyn SecretSource,
        pairing: &[u8; 16],
        query: Option<&str>,
        now: Now,
    ) -> Vec<SecretEntry> {
        let query: Option<String> = query
            .map(|q| {
                q.chars()
                    .take(MAX_QUERY_CHARS)
                    .collect::<String>()
                    .to_lowercase()
            })
            .filter(|q| !q.trim().is_empty());
        let mut entries: Vec<SecretEntry> = src
            .usable_bindings()
            .iter()
            .filter(|b| {
                query.as_ref().is_none_or(|q| {
                    let hay = format!(
                        "{}\n{}\n{}\n{}",
                        b.item_title,
                        b.vault_name,
                        b.field_label,
                        b.binding
                            .origins
                            .iter()
                            .map(mosaic_origin::AgentOrigin::as_str)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                    .to_lowercase();
                    hay.contains(q.as_str())
                })
            })
            .map(|b| SecretEntry {
                reference: reference_of(b),
                title: b.item_title.clone(),
                vault: b.vault_name.clone(),
                field: b.field_label.clone(),
                origins: b
                    .binding
                    .origins
                    .iter()
                    .map(|o| o.as_str().to_owned())
                    .collect(),
                inject: b.binding.inject.kind(),
                env: b.binding.env.clone(),
                run_allowed: b.binding.run,
                grant: self.grant_state(pairing, b, now),
                needs_review: !b.confirmed,
            })
            .collect();
        entries.sort_by(|a, b| {
            (&a.vault, &a.title, &a.field, &a.reference).cmp(&(
                &b.vault,
                &b.title,
                &b.field,
                &b.reference,
            ))
        });
        entries.truncate(MAX_LIST_ENTRIES);
        entries
    }

    /// `requestAccess`, first half: resolve, check the device mark and look for a live grant.
    ///
    /// # Errors
    /// [`AgentErrorCode::InvalidRef`], [`AgentErrorCode::NotAgentUsable`],
    /// [`AgentErrorCode::AmbiguousRef`] (with candidates), [`AgentErrorCode::NeedsReview`].
    pub fn access_check(
        &mut self,
        src: &dyn SecretSource,
        pairing: &PairingRef,
        reference: &str,
        reason: Option<&str>,
        now: Now,
    ) -> Result<AccessCheck, AgentFailure> {
        self.grants.sweep(now);
        let b = src.resolve(reference).map_err(resolve_failure)?;
        if !b.confirmed {
            return Err(AgentFailure::code(AgentErrorCode::NeedsReview));
        }
        if let Found::Live(i) = self.grants.find(
            &pairing.id,
            b.vault_id,
            b.item_id,
            &b.element_id,
            &b.binding.digest,
            now,
        ) && let Some(g) = self.grants.get(i)
        {
            return Ok(AccessCheck::Granted(g.state()));
        }
        Ok(AccessCheck::NeedsDialog(AccessFacts {
            pairing: pairing.clone(),
            vault_id: b.vault_id,
            item_id: b.item_id,
            element_id: b.element_id.clone(),
            vault_name: b.vault_name.clone(),
            item_title: b.item_title.clone(),
            field_label: b.field_label.clone(),
            origins: b
                .binding
                .origins
                .iter()
                .map(|o| o.as_str().to_owned())
                .collect(),
            inject: b.binding.inject.kind(),
            digest: b.binding.digest,
            reason: reason.and_then(clean_reason),
        }))
    }

    /// Whether this pairing may be shown another access dialog now. A call that returns `Ok`
    /// counts as a shown dialog (limits count shown dialogs, not requests): at most 3 a minute,
    /// and none for 10 minutes after 3 denials in 10 minutes.
    ///
    /// # Errors
    /// [`AgentErrorCode::RateLimited`].
    pub fn dialog_allowed(&mut self, pairing: &[u8; 16], now: Now) -> Result<(), AgentErrorCode> {
        let limits = self.limits.entry(*pairing).or_default();
        if limits.blocked_until.is_some_and(|until| now.mono < until) {
            return Err(AgentErrorCode::RateLimited);
        }
        limits.blocked_until = None;
        while limits
            .dialogs
            .front()
            .is_some_and(|t| now.mono.saturating_duration_since(*t) >= MINUTE)
        {
            limits.dialogs.pop_front();
        }
        if limits.dialogs.len() >= DIALOGS_PER_MINUTE {
            return Err(AgentErrorCode::RateLimited);
        }
        limits.dialogs.push_back(now.mono);
        Ok(())
    }

    /// `requestAccess`, second half: records what the user chose and creates the grant.
    pub fn record_decision(
        &mut self,
        facts: &AccessFacts,
        choice: GrantChoice,
        now: Now,
    ) -> (AccessOutcome, LogEntry) {
        let expiry = match choice {
            GrantChoice::Deny | GrantChoice::Timeout => None,
            GrantChoice::Once => Some(Expiry::Once),
            GrantChoice::Minutes15 => Some(Expiry::after(Duration::from_mins(15), now)),
            GrantChoice::Hour1 => Some(Expiry::after(Duration::from_hours(1), now)),
            GrantChoice::UntilLock => Some(Expiry::UntilLock),
        };
        let mut entry = LogEntry::new(LogKind::AccessGranted, now.wall_ms);
        entry.pairing_id = Some(hex16(&facts.pairing.id));
        entry.pairing_name = Some(facts.pairing.name.clone());
        entry.vault_id = Some(facts.vault_id.to_string());
        entry.item_id = Some(facts.item_id.to_string());
        entry.item_title = Some(facts.item_title.clone());
        entry.field_label = Some(facts.field_label.clone());
        entry.origins = Some(facts.origins.clone());
        entry.inject = Some(facts.inject);
        let Some(expiry) = expiry else {
            entry.kind = LogKind::AccessDenied;
            let timed_out = choice == GrantChoice::Timeout;
            entry.code = Some(if timed_out {
                AgentErrorCode::Timeout
            } else {
                AgentErrorCode::Denied
            });
            if !timed_out {
                self.note_denial(&facts.pairing.id, now);
            }
            let outcome = if timed_out {
                AccessOutcome::Timeout
            } else {
                AccessOutcome::Denied
            };
            return (outcome, entry);
        };
        self.grants.insert(
            facts.pairing.id,
            facts.vault_id,
            facts.item_id,
            facts.element_id.clone(),
            facts.digest,
            expiry,
        );
        let state = match expiry {
            Expiry::Once => GrantState::Once,
            Expiry::Until { wall_ms, .. } => GrantState::Until(wall_ms),
            Expiry::UntilLock => GrantState::UntilLock,
        };
        entry.grant = Some(state);
        (AccessOutcome::Granted(state), entry)
    }

    fn note_denial(&mut self, pairing: &[u8; 16], now: Now) {
        let limits = self.limits.entry(*pairing).or_default();
        limits.denials.push_back(now.mono);
        while limits
            .denials
            .front()
            .is_some_and(|t| now.mono.saturating_duration_since(*t) >= DENIAL_WINDOW)
        {
            limits.denials.pop_front();
        }
        if limits.denials.len() >= DENIALS_BEFORE_BLOCK {
            limits.denials.clear();
            limits.blocked_until = Some(now.mono + DIALOG_BLOCK);
        }
    }

    /// `httpRequest`, first half: every check, in the order of design §7.2, then the one place
    /// the value is read. Any failure stops before the value is read.
    ///
    /// # Errors
    /// An [`AgentFailure`] and the log entry of the refusal.
    #[expect(
        clippy::result_large_err,
        reason = "a refusal is rare and carries its log entry, as the host needs both"
    )]
    pub fn request_begin(
        &mut self,
        src: &dyn SecretSource,
        pairing: &PairingRef,
        reference: &str,
        input: &HttpRequestInput,
        now: Now,
    ) -> Result<(PreparedRequest, RequestTicket), (AgentFailure, Option<LogEntry>)> {
        match self.begin_checked(src, pairing, reference, input, now) {
            Ok(pair) => Ok(pair),
            Err((failure, entry)) => Err(self.limit_refusals(pairing, failure, entry, now)),
        }
    }

    /// Counts a refusal toward the pairing's limit. Up to [`REFUSALS_PER_MINUTE`] a minute they
    /// pass unchanged; beyond it the answer is `rateLimited` and only one summary row a minute is
    /// logged (`None` otherwise), so a flood of bad references cannot fill the log.
    fn limit_refusals(
        &mut self,
        pairing: &PairingRef,
        failure: AgentFailure,
        entry: LogEntry,
        now: Now,
    ) -> (AgentFailure, Option<LogEntry>) {
        let limits = self.limits.entry(pairing.id).or_default();
        while limits
            .refusals
            .front()
            .is_some_and(|t| now.mono.saturating_duration_since(*t) >= MINUTE)
        {
            limits.refusals.pop_front();
        }
        limits.refusals.push_back(now.mono);
        if limits.refusals.len() <= REFUSALS_PER_MINUTE {
            return (failure, Some(entry));
        }
        // Keep the queue bounded while the flood goes on.
        limits.refusals.pop_front();
        let summary_due = limits
            .last_refusal_summary
            .is_none_or(|t| now.mono.saturating_duration_since(t) >= MINUTE);
        let summary = summary_due.then(|| {
            limits.last_refusal_summary = Some(now.mono);
            let mut e = LogEntry::new(LogKind::Refused, now.wall_ms);
            e.pairing_id = Some(hex16(&pairing.id));
            e.pairing_name = Some(pairing.name.clone());
            e.code = Some(AgentErrorCode::RateLimited);
            e
        });
        (AgentFailure::code(AgentErrorCode::RateLimited), summary)
    }

    #[expect(
        clippy::result_large_err,
        reason = "a refusal is rare and carries its log entry, as the host needs both"
    )]
    fn begin_checked(
        &mut self,
        src: &dyn SecretSource,
        pairing: &PairingRef,
        reference: &str,
        input: &HttpRequestInput,
        now: Now,
    ) -> Result<(PreparedRequest, RequestTicket), (AgentFailure, LogEntry)> {
        self.grants.sweep(now);
        let mut facts = LogFacts::new(pairing);
        facts.method = Method::parse(&input.method).map(|m| m.as_str().to_owned());
        facts.set_url(&input.url);
        let refuse = |facts: &LogFacts, failure: AgentFailure| {
            let mut entry = facts.entry(LogKind::Refused, now);
            entry.code = Some(failure.code);
            (failure, entry)
        };
        let b = match src.resolve(reference) {
            Ok(b) => b,
            Err(e) => return Err(refuse(&facts, resolve_failure(e))),
        };
        facts.vault_id = Some(b.vault_id);
        facts.item_id = Some(b.item_id);
        facts.item_title = Some(b.item_title.clone());
        facts.field_label = Some(b.field_label.clone());
        if !b.confirmed {
            return Err(refuse(
                &facts,
                AgentFailure::code(AgentErrorCode::NeedsReview),
            ));
        }
        let found = self.grants.find(
            &pairing.id,
            b.vault_id,
            b.item_id,
            &b.element_id,
            &b.binding.digest,
            now,
        );
        let grant_id = match found {
            Found::Missing => {
                return Err(refuse(&facts, AgentFailure::code(AgentErrorCode::NoGrant)));
            }
            Found::Stale => {
                return Err(refuse(
                    &facts,
                    AgentFailure::code(AgentErrorCode::NeedsReview),
                ));
            }
            Found::Live(i) => self.take_grant(i, now),
        };
        let grant_id = match grant_id {
            Ok(id) => id,
            Err(code) => return Err(refuse(&facts, AgentFailure::code(code))),
        };
        let validated = match validate(&b.binding, input) {
            Ok(v) => v,
            Err(code) => return Err(refuse(&facts, AgentFailure::code(code))),
        };
        facts.method = Some(validated.method.as_str().to_owned());
        facts.origin = Some(validated.origin.clone());
        facts.path = Some(validated.path.clone());
        let max_bytes = validated.max_bytes;

        // The only place a value is read: every check that needs none has passed.
        let value = match src.read_value(&b) {
            Ok(v) => v,
            Err(code) => return Err(refuse(&facts, AgentFailure::code(code))),
        };
        if check_value(value.expose_secret()).is_err() {
            return Err(refuse(
                &facts,
                AgentFailure::code(AgentErrorCode::NotAgentUsable),
            ));
        }
        let user = match src.basic_user(&b) {
            Ok(u) => u,
            Err(code) => return Err(refuse(&facts, AgentFailure::code(code))),
        };
        let mut masker = Masker::new(&[value.expose_secret().as_bytes()]);
        let prepared = validated.inject(&b.binding.inject, &value, user.as_deref());
        drop(value);
        if matches!(b.binding.inject, Inject::Basic(_))
            && let Some((_, header)) = &prepared.secret_header
            && let Some(token) = header.as_bytes().strip_prefix(b"Basic ")
        {
            masker.add_exact(token);
        }
        let ticket = RequestTicket {
            masker,
            grant_id,
            epoch: self.epoch,
            facts,
            max_bytes,
        };
        Ok((prepared, ticket))
    }

    /// Uses the grant at `index`: a once-grant is consumed, any other counts toward the 60 a
    /// minute. Returns the grant's ID (`None` for a once-grant).
    fn take_grant(&mut self, index: usize, now: Now) -> Result<Option<u64>, AgentErrorCode> {
        let once = self
            .grants
            .get(index)
            .is_some_and(|g| matches!(g.state(), GrantState::Once));
        if once {
            return self
                .grants
                .remove_at(index)
                .map(|_| None)
                .ok_or(AgentErrorCode::NoGrant);
        }
        let grant = self.grants.get_mut(index).ok_or(AgentErrorCode::NoGrant)?;
        if grant.take_use(now) {
            Ok(Some(grant.id))
        } else {
            Err(AgentErrorCode::RateLimited)
        }
    }

    /// `httpRequest`, second half: maps the outcome, drops the answer if the grant was revoked in
    /// the meantime, and masks everything that goes back to the agent.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a ticket ends with its answer: the masker is dropped here"
    )]
    pub fn request_end(
        &mut self,
        ticket: RequestTicket,
        outcome: Result<RawResponse, SendError>,
        now: Now,
    ) -> (Result<HttpResponseBody, AgentErrorCode>, LogEntry) {
        let refused = |facts: &LogFacts, code: AgentErrorCode| {
            let mut entry = facts.entry(LogKind::Refused, now);
            entry.code = Some(code);
            (Err(code), entry)
        };
        // Revoked, locked or stopped while the request was out: its answer is dropped.
        if ticket.epoch != self.epoch {
            return refused(&ticket.facts, AgentErrorCode::Denied);
        }
        let raw = match outcome {
            Ok(raw) => raw,
            Err(error) => {
                let code = match error {
                    SendError::Timeout => AgentErrorCode::Timeout,
                    SendError::Tls => AgentErrorCode::TlsFailed,
                    SendError::ResponseTooLarge => AgentErrorCode::ResponseTooLarge,
                    SendError::Connect | SendError::Other => AgentErrorCode::RequestFailed,
                };
                return refused(&ticket.facts, code);
            }
        };
        let body = shape_response(&ticket.masker, &raw, ticket.max_bytes);
        let mut entry = ticket.facts.entry(LogKind::Request, now);
        entry.status = Some(body.status);
        entry.bytes = Some(body.raw_bytes);
        entry.masked = Some(body.response.masked);
        (Ok(body.response), entry)
    }

    /// Revokes one grant. Requests that already began lose their answer.
    pub fn revoke_grant(&mut self, grant_id: u64) -> bool {
        let removed = self.grants.remove(grant_id);
        if removed {
            self.epoch = self.epoch.wrapping_add(1);
        }
        removed
    }

    /// Ends every grant of a pairing and forgets its limits (unpairing).
    pub fn revoke_pairing(&mut self, pairing: &[u8; 16]) {
        self.grants.remove_pairing(pairing);
        self.limits.remove(pairing);
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Ends every grant (lock, "stop all", agent access switched off). The dialog limits stay:
    /// stopping does not reset a block.
    pub fn end_all(&mut self) {
        self.grants.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// The live grants.
    #[must_use]
    pub fn grants(&self, now: Now) -> Vec<GrantView> {
        self.grants
            .iter()
            .filter(|g| g.live(now))
            .map(|g| GrantView {
                grant_id: g.id,
                pairing: g.pairing,
                item_id: g.item_id,
                element_id: g.element_id.clone(),
                state: g.state(),
            })
            .collect()
    }
}

struct Shaped {
    status: u16,
    raw_bytes: u64,
    response: HttpResponseBody,
}

fn is_text_type(content_type: Option<&str>) -> bool {
    let Some(raw) = content_type else {
        return false;
    };
    let essence = raw
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    essence.starts_with("text/")
        || matches!(
            essence.as_str(),
            "application/json" | "application/xml" | "application/x-www-form-urlencoded"
        )
        || (essence.starts_with("application/") && essence.ends_with("+json"))
}

/// The text of `bytes`, if it is UTF-8. A body cut by the size cap may end inside a character;
/// that incomplete tail is not an error.
fn utf8_prefix(bytes: &[u8], cut: bool) -> Option<&[u8]> {
    match core::str::from_utf8(bytes) {
        Ok(_) => Some(bytes),
        Err(e) if cut && e.error_len().is_none() => bytes.get(..e.valid_up_to()),
        Err(_) => None,
    }
}

/// At most `max` bytes of `bytes`, cut at a character boundary.
fn cut_text(bytes: &[u8], max: usize) -> String {
    let head = bytes.get(..max.min(bytes.len())).unwrap_or_default();
    match core::str::from_utf8(head) {
        Ok(s) => s.to_owned(),
        Err(e) => core::str::from_utf8(head.get(..e.valid_up_to()).unwrap_or_default())
            .unwrap_or_default()
            .to_owned(),
    }
}

fn masked_text(masker: &Masker, bytes: &[u8]) -> (String, u32) {
    let (out, count) = masker.mask(bytes);
    (String::from_utf8_lossy(&out).into_owned(), count)
}

fn shape_response(masker: &Masker, raw: &RawResponse, max_bytes: usize) -> Shaped {
    let mut replaced = 0u32;
    let mut headers = Vec::with_capacity(raw.headers.len());
    let mut redirect = None;
    for (name, value) in &raw.headers {
        if name.eq_ignore_ascii_case("set-cookie") || name.eq_ignore_ascii_case("set-cookie2") {
            continue;
        }
        let (text, count) = masked_text(masker, value);
        replaced = replaced.saturating_add(count);
        if name.eq_ignore_ascii_case("location") && (300..400).contains(&raw.status) {
            redirect = Some(text.clone());
        }
        headers.push((name.clone(), text));
    }
    let encoding = raw
        .content_encoding()
        .map(str::trim)
        .filter(|e| !e.is_empty() && !e.eq_ignore_ascii_case("identity"))
        .map(str::to_owned);
    let utf8 = utf8_prefix(&raw.body, raw.truncated);
    let text_body = is_text_type(raw.content_type()) && encoding.is_none();
    let raw_bytes = u64::try_from(raw.body.len()).unwrap_or(u64::MAX);
    let mut response = HttpResponseBody {
        status: raw.status,
        headers,
        body: None,
        binary: None,
        truncated: raw.truncated,
        masked: replaced,
        redirect,
    };
    match utf8.filter(|_| text_body) {
        Some(text) => {
            let (out, count) = if raw.truncated {
                // The last `holdback` bytes may start a value that was cut off: they are
                // withheld, never released unmasked.
                let mut stream = masker.stream();
                let released = stream.push(text);
                (released, stream.count())
            } else {
                masker.mask(text)
            };
            response.masked = response.masked.saturating_add(count);
            response.truncated = raw.truncated || out.len() > max_bytes;
            response.body = Some(cut_text(&out, max_bytes));
        }
        None => {
            response.binary = Some(BinaryInfo {
                content_type: raw
                    .content_type()
                    .map(|t| masked_text(masker, t.as_bytes()).0),
                bytes: raw_bytes,
                content_encoding: encoding,
            });
        }
    }
    Shaped {
        status: raw.status,
        raw_bytes,
        response,
    }
}
