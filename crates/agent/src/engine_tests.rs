//! The policy engine against a fake secret source and the fixture TLS server (agent access
//! design §7.2, §9; review focus 1 and 4).

use std::cell::Cell;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use base64ct::{Base64, Encoding as _};
use mosaic_core::model::{AgentBasicUser, AgentBindingRecord, AgentInject, ElementId, Text};
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_vault::refs::RefCandidate;

use crate::binding::Binding;
use crate::engine::{
    AccessCheck, AccessFacts, AccessOutcome, Engine, GrantChoice, HttpResponseBody, Now, PairingRef,
};
use crate::error::AgentErrorCode;
use crate::grants::GrantState;
use crate::http::{HttpsSender, SendError};
use crate::log::{LogEntry, LogKind};
use crate::mask::{MASK_TEXT, Masker};
use crate::mask_tests::{contains, encodings_of};
use crate::request::HttpRequestInput;
use crate::response::RawResponse;
use crate::source::{ResolveError, SecretSource, SourceBinding};
use crate::testing::{API_HOST, CA_DER, RecordedRequest, TestResponse, TestRoute, TlsTestServer};

const VALUE: &str = "SK-TEST-CANARY-0123456789abcdef";

struct Fake {
    bindings: Vec<SourceBinding>,
    reads: Cell<usize>,
    candidates: Vec<RefCandidate>,
}

impl Fake {
    fn reference(b: &SourceBinding) -> String {
        format!("mosaic://{}/{}/api_key.secret", b.vault_id, b.item_id)
    }
}

impl SecretSource for Fake {
    fn usable_bindings(&self) -> Vec<SourceBinding> {
        self.bindings.clone()
    }

    fn resolve(&self, reference: &str) -> Result<SourceBinding, ResolveError> {
        if reference == "ambiguous" {
            return Err(ResolveError::Ambiguous(self.candidates.clone()));
        }
        if reference == "invalid" {
            return Err(ResolveError::Invalid);
        }
        self.bindings
            .iter()
            .find(|b| Self::reference(b) == reference)
            .cloned()
            .ok_or(ResolveError::NotUsable)
    }

    fn read_value(&self, _: &SourceBinding) -> Result<SecretString, AgentErrorCode> {
        self.reads.set(self.reads.get() + 1);
        SecretString::from_str_copy(VALUE).map_err(|_| AgentErrorCode::Internal)
    }

    fn basic_user(&self, b: &SourceBinding) -> Result<Option<String>, AgentErrorCode> {
        Ok(match b.binding.inject {
            crate::binding::Inject::Basic(crate::binding::BasicUser::Literal(ref u)) => {
                Some(u.clone())
            }
            _ => None,
        })
    }
}

fn source_binding(origins: &[&str], inject: AgentInject, confirmed: bool) -> SourceBinding {
    let record = AgentBindingRecord::new("api_key.secret", origins, inject);
    SourceBinding {
        vault_id: VaultId::new_v7().unwrap(),
        item_id: ItemId::new_v7().unwrap(),
        element_id: ElementId::from_stored("b1").unwrap(),
        vault_name: "Work".to_owned(),
        item_title: "Example API".to_owned(),
        field_label: "api_key.secret".to_owned(),
        binding: Binding::from_record(&record).unwrap(),
        confirmed,
    }
}

struct World {
    server: TlsTestServer,
    sender: HttpsSender,
    src: Fake,
    engine: Engine,
    pairing: PairingRef,
    t0: Instant,
    origin: String,
}

fn echo(req: &RecordedRequest) -> TestResponse {
    let mut body = format!("{} {}\n", req.method, req.target);
    for (name, value) in &req.headers {
        let _ = writeln!(body, "{name}: {value}");
    }
    body.push_str(&String::from_utf8_lossy(&req.body));
    let mut response = TestResponse::new(200, body.clone())
        .header("content-type", "application/json")
        .header("set-cookie", "session=abc")
        .header("x-echo", req.header("authorization").unwrap_or("none"));
    response.split_at = Some(body.len() / 2);
    response
}

fn redirect_same(req: &RecordedRequest) -> TestResponse {
    TestResponse::new(302, "").header(
        "location",
        &format!(
            "https://{API_HOST}/next?token={}",
            req.header("authorization").unwrap_or("none")
        ),
    )
}

fn gzip(_: &RecordedRequest) -> TestResponse {
    TestResponse::new(200, vec![0x1f, 0x8b, 0x08, 0x00, 1, 2, 3])
        .header("content-type", "application/json")
        .header("content-encoding", "gzip")
}

fn image(_: &RecordedRequest) -> TestResponse {
    TestResponse::new(200, vec![0xff, 0xd8, 0xff, 0xe0, 0, 1, 2])
        .header("content-type", "image/jpeg")
}

const CUT_MAX: usize = 20;

/// A body whose value starts so that the cap of `max_bytes + hold-back` cuts it off.
fn straddle(_: &RecordedRequest) -> TestResponse {
    let holdback = Masker::new(&[VALUE.as_bytes()]).holdback();
    let mut body = "x".repeat(CUT_MAX + holdback - 10).into_bytes();
    body.extend_from_slice(VALUE.as_bytes());
    body.extend_from_slice(&[b'y'; 200]);
    TestResponse::new(200, body).header("content-type", "text/plain")
}

fn world(inject: AgentInject, routes: Vec<TestRoute>) -> World {
    let server = TlsTestServer::start(routes);
    let origin = format!("https://{API_HOST}:{}", server.addr().port());
    let sender = HttpsSender::for_test(CA_DER, &[(API_HOST, server.addr())]).unwrap();
    let binding = source_binding(&[origin.as_str()], inject, true);
    World {
        server,
        sender,
        src: Fake {
            bindings: vec![binding],
            reads: Cell::new(0),
            candidates: Vec::new(),
        },
        engine: Engine::new(),
        pairing: PairingRef {
            id: [7; 16],
            name: "Test Client".to_owned(),
        },
        t0: Instant::now(),
        origin,
    }
}

fn bearer_world() -> World {
    world(
        AgentInject::Bearer,
        vec![
            TestRoute {
                path: "/echo",
                respond: echo,
            },
            TestRoute {
                path: "/redirect",
                respond: redirect_same,
            },
            TestRoute {
                path: "/gzip",
                respond: gzip,
            },
            TestRoute {
                path: "/image",
                respond: image,
            },
            TestRoute {
                path: "/straddle",
                respond: straddle,
            },
        ],
    )
}

impl World {
    fn at(&self, secs: u64) -> Now {
        Now {
            mono: self.t0 + Duration::from_secs(secs),
            wall_ms: 1_000_000_000 + secs * 1000,
        }
    }

    fn reference(&self) -> String {
        Fake::reference(&self.src.bindings[0])
    }

    fn facts(&mut self, secs: u64) -> AccessFacts {
        let now = self.at(secs);
        let reference = self.reference();
        match self
            .engine
            .access_check(&self.src, &self.pairing, &reference, None, now)
        {
            Ok(AccessCheck::NeedsDialog(facts)) => facts,
            other => panic!("expected a dialog, got {other:?}"),
        }
    }

    fn grant(&mut self, choice: GrantChoice, secs: u64) -> (AccessOutcome, LogEntry) {
        let facts = self.facts(secs);
        let now = self.at(secs);
        self.engine.dialog_allowed(&self.pairing.id, now).unwrap();
        self.engine.record_decision(&facts, choice, now)
    }

    fn input(&self, path: &str) -> HttpRequestInput {
        HttpRequestInput {
            method: "GET".to_owned(),
            url: format!("{}{path}", self.origin),
            headers: Vec::new(),
            body: None,
            max_bytes: None,
        }
    }

    fn call(
        &mut self,
        input: &HttpRequestInput,
        secs: u64,
    ) -> (Result<HttpResponseBody, AgentErrorCode>, Vec<LogEntry>) {
        let now = self.at(secs);
        let reference = self.reference();
        let begun = self
            .engine
            .request_begin(&self.src, &self.pairing, &reference, input, now);
        match begun {
            Err((failure, entry)) => (Err(failure.code), entry.into_iter().collect()),
            Ok((prepared, ticket)) => {
                let outcome = self.sender.send(prepared, ticket.holdback());
                let (result, entry) = self.engine.request_end(ticket, outcome, now);
                (result, vec![entry])
            }
        }
    }
}

fn json(value: &impl serde::Serialize) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn assert_no_canary(haystack: &[u8], what: &str) {
    for form in encodings_of(VALUE) {
        assert!(
            !contains(haystack, form.as_bytes()),
            "canary found in {what}"
        );
    }
}

#[test]
fn no_agent_op_returns_a_secret() {
    let mut w = bearer_world();
    // listSecrets.
    let listed = w.engine.list(&w.src, &w.pairing.id, None, w.at(0));
    assert_eq!(listed.len(), 1);
    assert_no_canary(&json(&listed), "listSecrets");
    // requestAccess: the facts, the outcome and the log entry.
    let facts = w.facts(0);
    assert_no_canary(format!("{facts:?}").as_bytes(), "access facts");
    let (outcome, entry) = w.grant(GrantChoice::Minutes15, 0);
    assert!(matches!(
        outcome,
        AccessOutcome::Granted(GrantState::Until(_))
    ));
    assert_no_canary(&entry.to_json(), "access log entry");
    assert_eq!(w.src.reads.get(), 0, "no access operation reads the value");
    // A granted request against the echo route.
    let mut input = w.input("/echo?q=1");
    input.method = "POST".to_owned();
    input.body = Some("{\"hello\":1}".to_owned());
    let (result, entries) = w.call(&input, 1);
    let response = result.unwrap();
    assert_eq!(response.status, 200);
    assert!(response.masked > 0, "the echoed value was masked");
    let body = response.body.clone().unwrap();
    assert!(body.contains(MASK_TEXT));
    assert!(
        body.contains("POST /echo?q=1"),
        "the echo is otherwise intact"
    );
    assert_no_canary(&json(&response), "the response");
    for entry in &entries {
        assert_no_canary(&entry.to_json(), "request log entry");
    }
    // The server did get the value, in the Authorization header only.
    let seen = w.server.requests();
    assert_eq!(
        seen[0].header("authorization"),
        Some(format!("Bearer {VALUE}").as_str())
    );
    // Listing again shows the grant, still no value.
    let listed = w.engine.list(&w.src, &w.pairing.id, None, w.at(2));
    assert!(matches!(listed[0].grant, GrantState::Until(_)));
    assert_no_canary(&json(&listed), "listSecrets");
}

#[test]
fn reflected_secret_is_masked_in_body_and_headers() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let (result, _) = w.call(&w.input("/echo"), 1);
    let response = result.unwrap();
    // The body arrived in two TLS writes with the value spanning the split or not; either way
    // nothing is left.
    assert_no_canary(response.body.as_deref().unwrap().as_bytes(), "body");
    let echo_header = response
        .headers
        .iter()
        .find(|(n, _)| n == "x-echo")
        .map(|(_, v)| v.as_str())
        .unwrap();
    assert_eq!(echo_header, format!("Bearer {MASK_TEXT}"));
    assert_no_canary(&json(&response.headers), "headers");
    assert!(
        response
            .headers
            .iter()
            .all(|(n, _)| !n.eq_ignore_ascii_case("set-cookie")),
        "Set-Cookie is dropped"
    );
}

#[test]
fn basic_credentials_are_masked_too() {
    let mut w = world(
        AgentInject::Basic {
            user: AgentBasicUser::Literal(Text::new("svc")),
        },
        vec![TestRoute {
            path: "/echo",
            respond: echo,
        }],
    );
    w.grant(GrantChoice::Minutes15, 0);
    let (result, _) = w.call(&w.input("/echo"), 1);
    let response = result.unwrap();
    let token = Base64::encode_string(format!("svc:{VALUE}").as_bytes());
    let seen = w.server.requests();
    assert_eq!(
        seen[0].header("authorization"),
        Some(format!("Basic {token}").as_str())
    );
    let all = json(&response);
    assert!(!contains(&all, token.as_bytes()), "basic token found");
    assert_no_canary(&all, "the response");
    assert!(response.masked > 0);
}

#[test]
fn redirects_are_not_followed_and_location_is_masked() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let (result, _) = w.call(&w.input("/redirect"), 1);
    let response = result.unwrap();
    assert_eq!(response.status, 302);
    let location = response.redirect.clone().unwrap();
    assert!(location.contains(MASK_TEXT), "{location}");
    assert_no_canary(&json(&response), "the redirect");
    assert_eq!(w.server.requests().len(), 1, "the redirect is not followed");
}

#[test]
fn compressed_and_binary_answers_are_reported_not_returned() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let (result, _) = w.call(&w.input("/gzip"), 1);
    let response = result.unwrap();
    assert!(response.body.is_none());
    let binary = response.binary.unwrap();
    assert_eq!(binary.content_encoding.as_deref(), Some("gzip"));
    assert_eq!(binary.content_type.as_deref(), Some("application/json"));
    assert_eq!(binary.bytes, 7);
    let (result, _) = w.call(&w.input("/image"), 2);
    let binary = result.unwrap().binary.unwrap();
    assert_eq!(binary.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(binary.content_encoding, None);
}

#[test]
fn a_cut_off_value_is_never_released() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let mut input = w.input("/straddle");
    input.max_bytes = Some(u32::try_from(CUT_MAX).unwrap());
    let (result, _) = w.call(&input, 1);
    let response = result.unwrap();
    assert!(response.truncated);
    let body = response.body.unwrap();
    assert!(body.len() <= CUT_MAX);
    assert!(body.bytes().all(|b| b == b'x'), "{body}");
    assert!(!body.contains(&VALUE[..6]), "a prefix of the value leaked");
}

#[test]
fn once_grant_is_consumed_by_one_request() {
    let mut w = bearer_world();
    let (outcome, _) = w.grant(GrantChoice::Once, 0);
    assert_eq!(outcome, AccessOutcome::Granted(GrantState::Once));
    let listed = w.engine.list(&w.src, &w.pairing.id, None, w.at(0));
    assert_eq!(listed[0].grant, GrantState::Once);
    assert!(w.call(&w.input("/echo"), 1).0.is_ok());
    assert_eq!(
        w.call(&w.input("/echo"), 2).0.err(),
        Some(AgentErrorCode::NoGrant)
    );
    // Asking again shows the dialog again.
    assert!(matches!(
        w.engine
            .access_check(&w.src, &w.pairing, &w.reference(), None, w.at(3)),
        Ok(AccessCheck::NeedsDialog(_))
    ));
}

#[test]
fn changed_binding_needs_review_and_voids_grants() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    assert!(w.call(&w.input("/echo"), 1).0.is_ok());
    let sends = w.server.requests().len();
    let reads = w.src.reads.get();

    // The binding gains an origin elsewhere and is not confirmed on this device.
    let old = w.src.bindings[0].clone();
    let mut changed = source_binding(
        &[w.origin.as_str(), "https://other.mosaic.test"],
        AgentInject::Bearer,
        false,
    );
    changed.vault_id = old.vault_id;
    changed.item_id = old.item_id;
    w.src.bindings[0] = changed.clone();
    let reference = w.reference();
    let failure = w
        .engine
        .access_check(&w.src, &w.pairing, &reference, None, w.at(2))
        .unwrap_err();
    assert_eq!(failure.code, AgentErrorCode::NeedsReview);
    assert_eq!(
        w.call(&w.input("/echo"), 2).0.err(),
        Some(AgentErrorCode::NeedsReview)
    );
    let listed = w.engine.list(&w.src, &w.pairing.id, None, w.at(2));
    assert!(listed[0].needs_review);
    assert_eq!(listed[0].grant, GrantState::None);

    // Confirmed, but the digest is new: the old grant does not carry over, no send.
    w.src.bindings[0].confirmed = true;
    assert_eq!(
        w.call(&w.input("/echo"), 3).0.err(),
        Some(AgentErrorCode::NeedsReview)
    );
    assert_eq!(w.server.requests().len(), sends);
    assert_eq!(w.src.reads.get(), reads, "no value was read");

    // A new approval for the new digest works.
    w.grant(GrantChoice::Minutes15, 4);
    assert!(w.call(&w.input("/echo"), 5).0.is_ok());
}

#[test]
fn nothing_is_read_before_checks_pass() {
    let mut w = bearer_world();
    // No grant yet.
    assert_eq!(
        w.call(&w.input("/echo"), 0).0.err(),
        Some(AgentErrorCode::NoGrant)
    );
    assert_eq!(w.src.reads.get(), 0);
    w.grant(GrantChoice::Minutes15, 1);
    let mut bad = w.input("/echo");
    bad.url = "https://evil.mosaic.test/echo".to_owned();
    assert_eq!(
        w.call(&bad, 2).0.err(),
        Some(AgentErrorCode::DestinationNotAllowed)
    );
    let mut bad = w.input("/echo");
    bad.method = "TRACE".to_owned();
    assert_eq!(
        w.call(&bad, 2).0.err(),
        Some(AgentErrorCode::MethodNotAllowed)
    );
    let mut bad = w.input("/echo");
    bad.headers
        .push(("Authorization".to_owned(), "x".to_owned()));
    assert_eq!(
        w.call(&bad, 2).0.err(),
        Some(AgentErrorCode::HeaderNotAllowed)
    );
    let mut bad = w.input("/echo");
    bad.body = Some("a".repeat(1024 * 1024 + 1));
    assert_eq!(
        w.call(&bad, 2).0.err(),
        Some(AgentErrorCode::RequestTooLarge)
    );
    let mut bad = w.input("/echo");
    bad.max_bytes = Some(0);
    assert_eq!(
        w.call(&bad, 2).0.err(),
        Some(AgentErrorCode::InvalidRequest)
    );
    assert_eq!(w.call(&w.input("/echo"), 2).0.err(), None);
    assert_eq!(
        w.src.reads.get(),
        1,
        "only the one valid request read the value"
    );
    // Unknown and malformed references.
    let now = w.at(3);
    for (reference, code) in [
        ("invalid", AgentErrorCode::InvalidRef),
        ("mosaic://nothing/here/x", AgentErrorCode::NotAgentUsable),
    ] {
        let failure = w
            .engine
            .request_begin(&w.src, &w.pairing, reference, &w.input("/echo"), now)
            .err()
            .unwrap();
        assert_eq!(failure.0.code, code);
    }
    assert_eq!(w.src.reads.get(), 1);
}

#[test]
fn grant_expires_on_wall_clock_even_if_monotonic_clock_paused() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let reference = w.reference();
    // Fourteen minutes later on both clocks: still granted.
    let early = w.at(14 * 60);
    assert!(
        w.engine
            .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), early)
            .is_ok()
    );
    // The machine slept: the monotonic clock stood still, the wall clock moved 16 minutes.
    let slept = Now {
        mono: w.t0,
        wall_ms: 1_000_000_000 + 16 * 60 * 1000,
    };
    let failure = w
        .engine
        .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), slept)
        .err()
        .unwrap();
    assert_eq!(failure.0.code, AgentErrorCode::NoGrant);
    // And the other way round: the wall clock was set back, the monotonic clock ran on.
    w.grant(GrantChoice::Minutes15, 20 * 60);
    let wound_back = Now {
        mono: w.t0 + Duration::from_mins(36),
        wall_ms: 1_000_000_000,
    };
    let failure = w
        .engine
        .request_begin(
            &w.src,
            &w.pairing,
            &reference,
            &w.input("/echo"),
            wound_back,
        )
        .err()
        .unwrap();
    assert_eq!(failure.0.code, AgentErrorCode::NoGrant);
}

#[test]
fn sixty_requests_a_minute_per_grant() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let reference = w.reference();
    for i in 0..60 {
        let now = w.at(1);
        assert!(
            w.engine
                .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), now)
                .is_ok(),
            "request {i}"
        );
    }
    let failure = w
        .engine
        .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), w.at(2))
        .err()
        .unwrap();
    assert_eq!(failure.0.code, AgentErrorCode::RateLimited);
    assert!(
        w.engine
            .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), w.at(62))
            .is_ok()
    );
}

#[test]
fn revocation_during_flight_drops_the_response() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let reference = w.reference();
    let now = w.at(1);
    let (prepared, ticket) = w
        .engine
        .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), now)
        .unwrap();
    let grant = ticket.grant_id().unwrap();
    assert!(w.engine.revoke_grant(grant));
    let outcome = w.sender.send(prepared, ticket.holdback());
    assert!(outcome.is_ok(), "the request itself was sent");
    let (result, entry) = w.engine.request_end(ticket, outcome, now);
    assert_eq!(result.err(), Some(AgentErrorCode::Denied));
    assert_eq!(entry.kind, LogKind::Refused);
    // Stop all does the same, and a revoked grant is gone.
    w.grant(GrantChoice::Minutes15, 2);
    let (prepared, ticket) = w
        .engine
        .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), w.at(3))
        .unwrap();
    w.engine.end_all();
    let outcome = w.sender.send(prepared, ticket.holdback());
    assert_eq!(
        w.engine.request_end(ticket, outcome, w.at(3)).0.err(),
        Some(AgentErrorCode::Denied)
    );
    assert_eq!(
        w.call(&w.input("/echo"), 4).0.err(),
        Some(AgentErrorCode::NoGrant)
    );
    assert_eq!(w.engine.grants(w.at(4)).len(), 0);
}

#[test]
fn dialog_limits_count_and_block() {
    let mut w = bearer_world();
    let id = w.pairing.id;
    // The fourth dialog in a minute is refused; a minute later the first one has aged out.
    for t in 0..3 {
        assert!(w.engine.dialog_allowed(&id, w.at(t)).is_ok());
    }
    assert_eq!(
        w.engine.dialog_allowed(&id, w.at(3)),
        Err(AgentErrorCode::RateLimited)
    );
    assert!(w.engine.dialog_allowed(&id, w.at(61)).is_ok());

    // Three denials in ten minutes block that pairing's dialogs for ten minutes.
    let mut w = bearer_world();
    for t in 0..3 {
        let facts = w.facts(t);
        w.engine.dialog_allowed(&id, w.at(t)).unwrap();
        let (outcome, entry) = w.engine.record_decision(&facts, GrantChoice::Deny, w.at(t));
        assert_eq!(outcome, AccessOutcome::Denied);
        assert_eq!(entry.kind, LogKind::AccessDenied);
    }
    assert_eq!(
        w.engine.dialog_allowed(&id, w.at(3)),
        Err(AgentErrorCode::RateLimited)
    );
    assert_eq!(
        w.engine.dialog_allowed(&id, w.at(500)),
        Err(AgentErrorCode::RateLimited)
    );
    assert!(w.engine.dialog_allowed(&id, w.at(603)).is_ok());
    // Another pairing is not affected.
    assert!(w.engine.dialog_allowed(&[9; 16], w.at(3)).is_ok());
    // A timeout is not a denial.
    let mut w = bearer_world();
    for t in 0..3 {
        let facts = w.facts(t);
        let (outcome, entry) = w
            .engine
            .record_decision(&facts, GrantChoice::Timeout, w.at(t * 61));
        assert_eq!(outcome, AccessOutcome::Timeout);
        assert_eq!(entry.code, Some(AgentErrorCode::Timeout));
    }
    assert!(w.engine.dialog_allowed(&id, w.at(300)).is_ok());
}

#[test]
fn reason_is_capped_and_escaped() {
    let mut w = bearer_world();
    let reference = w.reference();
    let reason = format!("do it\u{202e}\n\u{7}\u{85}\u{2066}{}", "x".repeat(300));
    let facts = match w
        .engine
        .access_check(&w.src, &w.pairing, &reference, Some(&reason), w.at(0))
        .unwrap()
    {
        AccessCheck::NeedsDialog(f) => f,
        AccessCheck::Granted(_) => panic!("no grant yet"),
    };
    let shown = facts.reason.unwrap();
    assert!(
        shown.starts_with("do it\\u{202e}\\u{a}\\u{7}\\u{85}\\u{2066}"),
        "{shown}"
    );
    assert!(
        shown.chars().all(|c| !matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')),
        "a control or bidi character is left"
    );
    assert_eq!(
        shown.matches('x').count(),
        190,
        "200 characters of input in all"
    );
    // Nothing but blanks is no reason at all.
    let blank = w
        .engine
        .access_check(&w.src, &w.pairing, &reference, Some("   "), w.at(0))
        .unwrap();
    assert!(matches!(
        blank,
        AccessCheck::NeedsDialog(AccessFacts { reason: None, .. })
    ));
}

#[test]
fn log_entries_hold_no_query_or_header() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let mut input = w.input("/echo?token=QUERYVAL-ASDF");
    input.method = "POST".to_owned();
    input
        .headers
        .push(("X-Y".to_owned(), "HEADERVAL-QWERTY".to_owned()));
    input.body = Some("BODYVAL-ZXCV".to_owned());
    let (result, mut entries) = w.call(&input, 1);
    assert!(result.is_ok());
    // A refused one too.
    let mut refused = input.clone();
    refused.url = "https://evil.mosaic.test/steal?token=QUERYVAL-ASDF#FRAGVAL-ZZZ".to_owned();
    let (result, more) = w.call(&refused, 2);
    assert_eq!(result.err(), Some(AgentErrorCode::DestinationNotAllowed));
    entries.extend(more);
    assert_eq!(entries.len(), 2);
    for entry in &entries {
        let text = String::from_utf8(entry.to_json()).unwrap();
        for secret in ["QUERYVAL", "HEADERVAL", "BODYVAL", "FRAGVAL", "token="] {
            assert!(!text.contains(secret), "{secret} found in {text}");
        }
        assert_eq!(entry.pairing_name.as_deref(), Some("Test Client"));
        assert_eq!(entry.method.as_deref(), Some("POST"));
    }
    assert_eq!(entries[0].kind, LogKind::Request);
    assert_eq!(entries[0].path.as_deref(), Some("/echo"));
    assert_eq!(entries[0].origin.as_deref(), Some(w.origin.as_str()));
    assert_eq!(entries[0].status, Some(200));
    assert_eq!(entries[0].item_title.as_deref(), Some("Example API"));
    assert_eq!(entries[1].kind, LogKind::Refused);
    assert_eq!(entries[1].path.as_deref(), Some("/steal"));
    assert_eq!(entries[1].code, Some(AgentErrorCode::DestinationNotAllowed));
    // The entries survive a round trip.
    for entry in &entries {
        assert_eq!(LogEntry::from_json(&entry.to_json()).as_ref(), Some(entry));
    }
}

#[test]
fn ambiguous_candidates_only_name_usable_items() {
    let mut w = bearer_world();
    // The source reports the candidates it considers; the engine forwards exactly those.
    w.src.candidates = vec![
        RefCandidate {
            vault: "Work".to_owned(),
            title: "Key".to_owned(),
            id: ItemId::new_v7().unwrap(),
        },
        RefCandidate {
            vault: "Work".to_owned(),
            title: "Key".to_owned(),
            id: ItemId::new_v7().unwrap(),
        },
    ];
    let failure = w
        .engine
        .access_check(&w.src, &w.pairing, "ambiguous", None, w.at(0))
        .unwrap_err();
    assert_eq!(failure.code, AgentErrorCode::AmbiguousRef);
    assert_eq!(failure.candidates, w.src.candidates);
    let failure = w
        .engine
        .request_begin(&w.src, &w.pairing, "ambiguous", &w.input("/echo"), w.at(0))
        .err()
        .unwrap();
    assert_eq!(failure.0.code, AgentErrorCode::AmbiguousRef);
    assert_eq!(failure.1.unwrap().kind, LogKind::Refused);
    // Other failures carry none.
    let failure = w
        .engine
        .access_check(&w.src, &w.pairing, "mosaic://x/y/z", None, w.at(0))
        .unwrap_err();
    assert_eq!(
        (failure.code, failure.candidates.len()),
        (AgentErrorCode::NotAgentUsable, 0)
    );
}

#[test]
fn list_filters_orders_and_caps() {
    let mut w = bearer_world();
    for i in 0..60 {
        let mut b = source_binding(
            &["https://api.example.com"],
            AgentInject::Bearer,
            i % 2 == 0,
        );
        b.item_title = format!("Item {i:02}");
        w.src.bindings.push(b);
    }
    let all = w.engine.list(&w.src, &w.pairing.id, None, w.at(0));
    assert_eq!(all.len(), 50);
    assert!(
        all.windows(2)
            .all(|p| (&p[0].vault, &p[0].title) <= (&p[1].vault, &p[1].title))
    );
    let some = w
        .engine
        .list(&w.src, &w.pairing.id, Some("item 07"), w.at(0));
    assert_eq!(some.len(), 1);
    assert!(some[0].needs_review, "item 07 is unconfirmed");
    assert!(some[0].reference.starts_with("mosaic://"));
    let by_origin = w
        .engine
        .list(&w.src, &w.pairing.id, Some("API.MOSAIC.TEST"), w.at(0));
    assert_eq!(by_origin.len(), 1);
    assert_eq!(
        w.engine
            .list(&w.src, &w.pairing.id, Some("nothing like this"), w.at(0))
            .len(),
        0
    );
}

#[test]
fn send_errors_map_to_codes() {
    let mut w = bearer_world();
    w.grant(GrantChoice::Minutes15, 0);
    let reference = w.reference();
    for (error, code) in [
        (SendError::Timeout, AgentErrorCode::Timeout),
        (SendError::Tls, AgentErrorCode::TlsFailed),
        (SendError::Connect, AgentErrorCode::RequestFailed),
        (
            SendError::ResponseTooLarge,
            AgentErrorCode::ResponseTooLarge,
        ),
        (SendError::Other, AgentErrorCode::RequestFailed),
    ] {
        let now = w.at(1);
        let (_, ticket) = w
            .engine
            .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), now)
            .unwrap();
        let (result, entry) = w.engine.request_end(ticket, Err(error), now);
        assert_eq!(result.err(), Some(code));
        assert_eq!(entry.code, Some(code));
    }
    // A raw answer built by hand: set-cookie never passes, header values are masked.
    let now = w.at(1);
    let (_, ticket) = w
        .engine
        .request_begin(&w.src, &w.pairing, &reference, &w.input("/echo"), now)
        .unwrap();
    let raw = RawResponse {
        status: 200,
        headers: vec![
            ("Set-Cookie2".to_owned(), b"a=b".to_vec()),
            ("x-note".to_owned(), format!("see {VALUE}").into_bytes()),
            (
                "content-type".to_owned(),
                b"text/plain; charset=utf-8".to_vec(),
            ),
        ],
        body: format!("\u{e4}{VALUE}\u{e4}").into_bytes(),
        truncated: false,
    };
    let (result, _) = w.engine.request_end(ticket, Ok(raw), now);
    let response = result.unwrap();
    assert_eq!(response.headers.len(), 2);
    assert_eq!(response.masked, 2);
    assert_eq!(
        response.body.as_deref(),
        Some(format!("\u{e4}{MASK_TEXT}\u{e4}").as_str())
    );
    assert_no_canary(&json(&response), "the response");
}

/// Red team, audit log: the origin of a `url` that failed the agent URL checks is never logged
/// (an oversized host cannot fill a row), and a flood of refusals is answered `rateLimited`
/// with one summary row a minute.
#[test]
fn refusal_flood_is_limited_and_logs_no_hostile_origin() {
    let mut w = bearer_world();
    let huge = HttpRequestInput {
        url: format!("https://{}.example/x", "a".repeat(4000)),
        ..w.input("/echo")
    };
    let (failure, entry) = w
        .engine
        .request_begin(&w.src, &w.pairing, "invalid", &huge, w.at(0))
        .err()
        .unwrap();
    assert_eq!(failure.code, AgentErrorCode::InvalidRef);
    let entry = entry.unwrap();
    assert_eq!(entry.origin.as_deref(), Some("invalid"));
    assert_eq!(entry.path, None);
    let plain = HttpRequestInput {
        url: "http://insecure.example/x".to_owned(),
        ..w.input("/echo")
    };
    let (_, entry) = w
        .engine
        .request_begin(&w.src, &w.pairing, "invalid", &plain, w.at(0))
        .err()
        .unwrap();
    assert_eq!(entry.unwrap().origin.as_deref(), Some("invalid"));
    // A good URL keeps its origin and path in the log.
    let (_, entry) = w
        .engine
        .request_begin(&w.src, &w.pairing, "invalid", &w.input("/echo"), w.at(0))
        .err()
        .unwrap();
    let entry = entry.unwrap();
    assert_eq!(entry.origin.as_deref(), Some(w.origin.as_str()));
    assert_eq!(entry.path.as_deref(), Some("/echo"));

    // Three refusals are counted so far; 57 more still log, then the limit trips.
    let mut logged = 3;
    let mut limited = 0;
    let mut summaries = 0;
    for _ in 0..200 {
        let (failure, entry) = w
            .engine
            .request_begin(&w.src, &w.pairing, "invalid", &w.input("/echo"), w.at(1))
            .err()
            .unwrap();
        if failure.code == AgentErrorCode::RateLimited {
            limited += 1;
            if let Some(e) = entry {
                assert_eq!(e.code, Some(AgentErrorCode::RateLimited));
                summaries += 1;
            }
        } else {
            assert_eq!(failure.code, AgentErrorCode::InvalidRef);
            assert!(entry.is_some());
            logged += 1;
        }
    }
    assert_eq!(logged, 60);
    assert_eq!(limited, 143);
    assert_eq!(summaries, 1, "one summary row a minute");
    // A minute on, the pairing is served again and the next flood logs one more summary.
    let (failure, entry) = w
        .engine
        .request_begin(&w.src, &w.pairing, "invalid", &w.input("/echo"), w.at(70))
        .err()
        .unwrap();
    assert_eq!(failure.code, AgentErrorCode::InvalidRef);
    assert!(entry.is_some());
    // Another pairing is not limited by this one.
    let other = PairingRef {
        id: [9; 16],
        name: "other".to_owned(),
    };
    let (failure, _) = w
        .engine
        .request_begin(&w.src, &other, "invalid", &w.input("/echo"), w.at(71))
        .err()
        .unwrap();
    assert_eq!(failure.code, AgentErrorCode::InvalidRef);
}

/// Red team, reason rendering: format characters, line separators and stacks of combining marks
/// are written out; a normal accent and plain text stay as they are.
#[test]
fn reason_escapes_format_characters_separators_and_mark_stacks() {
    let mut w = bearer_world();
    let reference = w.reference();
    let mut shown = |reason: &str| match w
        .engine
        .access_check(&w.src, &w.pairing, &reference, Some(reason), w.at(0))
        .unwrap()
    {
        AccessCheck::NeedsDialog(f) => f.reason.unwrap(),
        AccessCheck::Granted(_) => panic!("no grant yet"),
    };
    let hidden = "a\u{200b}b\u{2060}c\u{feff}d\u{ad}e\u{2028}f\u{2029}g\u{200d}h";
    assert_eq!(
        shown(hidden),
        r"a\u{200b}b\u{2060}c\u{feff}d\u{ad}e\u{2028}f\u{2029}g\u{200d}h"
    );
    // Two marks stay (a letter with two accents), the third and later are written out.
    let stack = format!("e{}", "\u{301}".repeat(5));
    assert_eq!(
        shown(&stack),
        r"e\u{301}\u{301}\u{301}\u{301}\u{301}".replacen(r"\u{301}", "\u{301}", 2)
    );
    // A base letter between marks starts a new run.
    assert_eq!(
        shown("e\u{301}\u{302}x\u{301}\u{302}"),
        "e\u{301}\u{302}x\u{301}\u{302}"
    );
    // Ordinary text, accents and CJK are untouched.
    assert_eq!(
        shown("Gr\u{fc}\u{df}e, \u{65e5}\u{672c}"),
        "Gr\u{fc}\u{df}e, \u{65e5}\u{672c}"
    );
}
