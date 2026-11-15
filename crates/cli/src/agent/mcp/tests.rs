//! Tests of the relay against a fake connector.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::io::{Cursor, Write};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mosaic_agent::error::AgentErrorCode;
use mosaic_agent::wire::{AccessState, OpRequest, OpResponse, StatusBody, VaultState, WireError};
use serde_json::{Value, json};

use super::run_mcp;
use super::text::{INITIALIZE_VERSIONS, sentence};
use crate::agent::conn::{ConnError, Connector, Link, LocalFut, RequestError};

const CANARY: &str = "CANARY-8f3a91c2d7";

type Responder = Box<dyn Fn(&OpRequest) -> Result<OpResponse, RequestError>>;

struct FakeLink {
    responder: Responder,
    delay: Duration,
    closed: Cell<bool>,
    unpaired: Cell<bool>,
    close_after_first: bool,
    seen: RefCell<Vec<OpRequest>>,
}

impl FakeLink {
    fn new(responder: Responder) -> Rc<Self> {
        Rc::new(Self {
            responder,
            delay: Duration::ZERO,
            closed: Cell::new(false),
            unpaired: Cell::new(false),
            close_after_first: false,
            seen: RefCell::new(Vec::new()),
        })
    }

    fn status() -> Rc<Self> {
        Self::new(Box::new(|_| Ok(status_body())))
    }
}

fn status_body() -> OpResponse {
    OpResponse::Status(StatusBody {
        vault: VaultState::Unlocked,
        agent_access: AccessState::On,
    })
}

impl Link for FakeLink {
    fn request(
        &self,
        op: OpRequest,
        _wait: Duration,
    ) -> LocalFut<'_, Result<OpResponse, RequestError>> {
        Box::pin(async move {
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            let answer = (self.responder)(&op);
            self.seen.borrow_mut().push(op);
            if self.close_after_first {
                self.closed.set(true);
            }
            answer
        })
    }

    fn is_closed(&self) -> bool {
        self.closed.get()
    }

    fn unpaired(&self) -> bool {
        self.unpaired.get()
    }
}

struct FakeConnector {
    script: RefCell<VecDeque<Result<Rc<dyn Link>, ConnError>>>,
    connects: Cell<usize>,
    forgot: Cell<bool>,
}

impl FakeConnector {
    fn new(script: Vec<Result<Rc<dyn Link>, ConnError>>) -> Rc<Self> {
        Rc::new(Self {
            script: RefCell::new(script.into()),
            connects: Cell::new(0),
            forgot: Cell::new(false),
        })
    }

    fn always(link: &Rc<FakeLink>) -> Rc<Self> {
        let script = (0..8)
            .map(|_| Ok(Rc::clone(link) as Rc<dyn Link>))
            .collect();
        Self::new(script)
    }
}

impl Connector for FakeConnector {
    fn connect(&self) -> LocalFut<'_, Result<Rc<dyn Link>, ConnError>> {
        Box::pin(async move {
            self.connects.set(self.connects.get() + 1);
            self.script
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(ConnError::DesktopNotRunning))
        })
    }

    fn forget(&self) {
        self.forgot.set(true);
    }
}

#[derive(Clone, Default)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Feeds `lines` and returns every output line (raw text) in order.
fn drive_raw(lines: &[String], connector: &Rc<FakeConnector>) -> Vec<String> {
    let input = lines.iter().fold(String::new(), |mut s, l| {
        s.push_str(l);
        s.push('\n');
        s
    });
    let out = SharedBuf::default();
    let code = run_mcp(
        "Test",
        Cursor::new(input.into_bytes()),
        out.clone(),
        Rc::clone(connector) as Rc<dyn Connector>,
    )
    .unwrap();
    assert_eq!(code, 0);
    let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    text.lines().map(str::to_owned).collect()
}

fn drive(lines: &[String], connector: &Rc<FakeConnector>) -> Vec<Value> {
    drive_raw(lines, connector)
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn request(id: u64, method: &str, params: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

fn init(id: u64, version: &str) -> String {
    request(
        id,
        "initialize",
        &json!({ "protocolVersion": version, "capabilities": {}, "clientInfo": { "name": "t", "version": "1" } }),
    )
}

fn call_line(id: u64, tool: &str, args: &Value) -> String {
    request(
        id,
        "tools/call",
        &json!({ "name": tool, "arguments": args }),
    )
}

const STATELESS: &str = "2026-07-28";

/// The per-request `_meta` of a stateless request naming `version`.
fn meta(version: &str) -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": version,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
    })
}

/// A stateless request: `params` with `_meta` for `2026-07-28`.
fn modern(id: u64, method: &str, params: &Value) -> String {
    let mut params = params.clone();
    params
        .as_object_mut()
        .unwrap()
        .insert("_meta".to_owned(), meta(STATELESS));
    request(id, method, &params)
}

fn modern_call(id: u64, tool: &str, args: &Value) -> String {
    modern(
        id,
        "tools/call",
        &json!({ "name": tool, "arguments": args }),
    )
}

/// What every stateless result carries.
fn assert_stateless_result(result: &Value) {
    assert_eq!(result["resultType"], "complete", "{result}");
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "mosaic"
    );
}

fn all_versions() -> Vec<&'static str> {
    let mut all = vec![STATELESS];
    all.extend(INITIALIZE_VERSIONS);
    all
}

fn by_id(out: &[Value], id: u64) -> &Value {
    out.iter()
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response for {id}"))
}

#[test]
fn mcp_stdout_holds_only_protocol_messages() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let lines = vec![
        init(1, "2025-06-18"),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
        request(2, "ping", &json!({})),
        request(3, "tools/list", &json!({})),
        call_line(4, "mosaic_status", &json!({})),
        request(5, "resources/list", &json!({})),
        "this is not json".to_owned(),
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 4 } })
            .to_string(),
    ];
    let raw = drive_raw(&lines, &connector);
    let sent: BTreeSet<u64> = (1..=5).collect();
    let mut answered = BTreeSet::new();
    for line in &raw {
        let v: Value = serde_json::from_str(line).expect("every stdout line is JSON");
        assert_eq!(v["jsonrpc"], "2.0");
        assert!(v.get("result").is_some() || v.get("error").is_some());
        assert!(
            v.get("method").is_none(),
            "no request or notification to the client"
        );
        match v["id"].as_u64() {
            Some(id) => {
                assert!(sent.contains(&id));
                answered.insert(id);
            }
            None => assert_eq!(v["error"]["code"], -32700, "only the parse error has no id"),
        }
    }
    assert_eq!(answered, sent);
    let out: Vec<Value> = raw
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(by_id(&out, 5)["error"]["code"], -32601);
    assert_eq!(
        by_id(&out, 3)["result"]["tools"].as_array().unwrap().len(),
        4
    );
}

#[test]
fn mcp_never_sends_sampling_or_elicitation() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let lines = vec![
        init(1, "2025-11-25"),
        call_line(2, "mosaic_list_secrets", &json!({})),
        call_line(
            3,
            "mosaic_request_access",
            &json!({ "ref": "mosaic://v/i/password", "use": "request" }),
        ),
        call_line(
            4,
            "mosaic_http_request",
            &json!({ "ref": "mosaic://v/i/password", "method": "GET", "url": "https://api.mosaic.test/x" }),
        ),
    ];
    for v in drive(&lines, &connector) {
        for key in ["method", "params"] {
            assert!(v.get(key).is_none(), "{key} would make this a request");
        }
        assert!(!v.to_string().contains("sampling"));
        assert!(!v.to_string().contains("elicitation"));
        assert!(!v.to_string().contains("roots/list"));
    }
}

#[test]
fn mcp_tool_schemas_reject_unknown_properties() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let out = drive(
        &[init(1, "2025-06-18"), request(2, "tools/list", &json!({}))],
        &connector,
    );
    let tools = by_id(&out, 2)["result"]["tools"]
        .as_array()
        .unwrap()
        .clone();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "mosaic_status",
            "mosaic_list_secrets",
            "mosaic_request_access",
            "mosaic_http_request"
        ]
    );
    for tool in &tools {
        assert_eq!(
            tool["inputSchema"]["additionalProperties"],
            json!(false),
            "{tool}"
        );
        assert!(tool["description"].as_str().unwrap().len() > 20);
    }
    let http = tools
        .iter()
        .find(|t| t["name"] == "mosaic_http_request")
        .unwrap();
    assert!(http["description"].as_str().unwrap().starts_with(
        "Send one HTTPS request with a secret from the user's Mosaic vault added by Mosaic."
    ));

    let bad_calls = vec![
        call_line(10, "mosaic_status", &json!({ "extra": 1 })),
        call_line(
            11,
            "mosaic_list_secrets",
            &json!({ "query": "x", "extra": 1 }),
        ),
        call_line(
            12,
            "mosaic_list_secrets",
            &json!({ "query": "x".repeat(129) }),
        ),
        call_line(
            13,
            "mosaic_request_access",
            &json!({ "ref": "r", "use": "run" }),
        ),
        call_line(14, "mosaic_request_access", &json!({ "use": "request" })),
        call_line(
            15,
            "mosaic_http_request",
            &json!({ "ref": "r", "method": "TRACE", "url": "https://a" }),
        ),
        call_line(
            16,
            "mosaic_http_request",
            &json!({ "ref": "r", "method": "GET", "url": "https://a", "maxBytes": 0 }),
        ),
        call_line(
            17,
            "mosaic_http_request",
            &json!({ "ref": "r", "method": "GET", "url": "https://a", "headers": { "a": 1 } }),
        ),
        call_line(
            18,
            "mosaic_http_request",
            &json!({ "ref": "r", "method": "GET", "url": "https://a", "token": "x" }),
        ),
        call_line(19, "mosaic_reveal", &json!({})),
    ];
    let connector = FakeConnector::always(&link);
    let out = drive(&bad_calls, &connector);
    for id in 10..=19 {
        assert_eq!(by_id(&out, id)["error"]["code"], -32602, "{id}");
    }
    assert_eq!(connector.connects.get(), 0, "nothing was forwarded");
}

#[test]
fn mcp_errors_hold_no_value() {
    for code in AgentErrorCode::ALL.iter().copied() {
        let link = FakeLink::new(Box::new(move |_| {
            Err(RequestError::Wire(WireError::code(code)))
        }));
        let connector = FakeConnector::always(&link);
        let url = format!("https://api.mosaic.test/{CANARY}");
        let args = json!({ "ref": format!("mosaic://v/i/{CANARY}"), "method": "POST", "url": url, "body": CANARY });
        let lines = vec![
            init(1, "2025-06-18"),
            call_line(2, "mosaic_http_request", &args),
            modern_call(3, "mosaic_http_request", &args),
        ];
        let raw = drive_raw(&lines, &connector);
        let all = raw.join("\n");
        assert!(!all.contains(CANARY), "canary found");
        let out: Vec<Value> = raw
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        for id in [2, 3] {
            let result = &by_id(&out, id)["result"];
            assert_eq!(result["isError"], true);
            assert_eq!(
                result["content"][0]["text"],
                format!("{}: {}", code.as_str(), sentence(code))
            );
            assert_eq!(
                result["structuredContent"],
                json!({ "error": code.as_str() })
            );
        }
    }
}

#[test]
fn unpaired_client_answers_not_paired_without_pairing() {
    let connector = FakeConnector::new(vec![Err(ConnError::NotPaired), Err(ConnError::NotPaired)]);
    let out = drive(
        &[
            init(1, "2025-06-18"),
            call_line(2, "mosaic_status", &json!({})),
        ],
        &connector,
    );
    let result = &by_id(&out, 2)["result"];
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["content"][0]["text"],
        "notPaired: Pair this client: run `mosaic agent pair` in a terminal."
    );
    assert_eq!(
        connector.connects.get(),
        1,
        "one connect, no pairing request exists in the relay"
    );
}

#[test]
fn an_unpaired_event_forgets_the_pairing() {
    let link = FakeLink::status();
    link.unpaired.set(true);
    let connector = FakeConnector::always(&link);
    let out = drive(&[call_line(1, "mosaic_status", &json!({}))], &connector);
    assert_eq!(by_id(&out, 1)["result"]["isError"], true);
    assert!(connector.forgot.get());
}

#[test]
fn relay_reconnects_after_the_desktop_restarts() {
    let first: Rc<dyn Link> = FakeLink::status();
    let second = FakeLink::status();
    let connector = FakeConnector::new(vec![
        Err(ConnError::DesktopNotRunning),
        Ok(first),
        Ok(Rc::clone(&second) as Rc<dyn Link>),
    ]);
    // Sequential calls: each is sent after the previous answer arrives (one process, one line at
    // a time), so the order of connects is the order of calls.
    let out = drive(&[call_line(1, "mosaic_status", &json!({}))], &connector);
    assert_eq!(
        by_id(&out, 1)["result"]["content"][0]["text"],
        format!(
            "desktopNotRunning: {}",
            sentence(AgentErrorCode::DesktopNotRunning)
        )
    );
    let out = drive(&[call_line(2, "mosaic_status", &json!({}))], &connector);
    assert_eq!(by_id(&out, 2)["result"]["isError"], false);
    assert_eq!(
        connector.connects.get(),
        2,
        "a fresh connect for the second call"
    );

    // A link that closed is replaced through a new connect.
    let dying = Rc::new(FakeLink {
        responder: Box::new(|_| Ok(status_body())),
        delay: Duration::ZERO,
        closed: Cell::new(false),
        unpaired: Cell::new(false),
        close_after_first: true,
        seen: RefCell::new(Vec::new()),
    });
    let revived = FakeLink::status();
    let connector = FakeConnector::new(vec![
        Ok(Rc::clone(&dying) as Rc<dyn Link>),
        Ok(Rc::clone(&revived) as Rc<dyn Link>),
    ]);
    let out = drive(
        &[
            call_line(1, "mosaic_status", &json!({})),
            call_line(2, "mosaic_status", &json!({})),
        ],
        &connector,
    );
    assert_eq!(by_id(&out, 1)["result"]["isError"], false);
    assert_eq!(by_id(&out, 2)["result"]["isError"], false);
    assert_eq!(connector.connects.get(), 2);
    assert_eq!(dying.seen.borrow().len(), 1);
    assert_eq!(revived.seen.borrow().len(), 1);
    let _ = second;
}

#[test]
fn initialize_negotiates_versions() {
    let link = FakeLink::status();
    for version in INITIALIZE_VERSIONS {
        let connector = FakeConnector::always(&link);
        let out = drive(
            &[
                init(1, version),
                call_line(2, "mosaic_status", &json!({})),
                request(3, "tools/list", &json!({})),
            ],
            &connector,
        );
        let init = &by_id(&out, 1)["result"];
        assert_eq!(init["protocolVersion"], version);
        assert_eq!(init["serverInfo"]["name"], "mosaic");
        assert_eq!(
            init["capabilities"],
            json!({ "tools": { "listChanged": false } })
        );
        let call = &by_id(&out, 2)["result"];
        let has_structured = version >= "2025-06-18";
        assert_eq!(
            call.get("structuredContent").is_some(),
            has_structured,
            "{version}"
        );
        let tool = &by_id(&out, 3)["result"]["tools"][0];
        assert_eq!(
            tool.get("annotations").is_some(),
            version >= "2025-03-26",
            "{version}"
        );
    }
    let connector = FakeConnector::always(&link);
    let out = drive(&[init(1, "2099-01-01")], &connector);
    assert_eq!(
        by_id(&out, 1)["result"]["protocolVersion"],
        INITIALIZE_VERSIONS[0]
    );
}

#[test]
fn fifth_parallel_call_is_busy() {
    let link = Rc::new(FakeLink {
        responder: Box::new(|_| Ok(status_body())),
        delay: Duration::from_millis(300),
        closed: Cell::new(false),
        unpaired: Cell::new(false),
        close_after_first: false,
        seen: RefCell::new(Vec::new()),
    });
    let connector = FakeConnector::always(&link);
    let mut lines = vec![init(1, "2025-06-18")];
    for id in 2..=6 {
        lines.push(call_line(id, "mosaic_status", &json!({})));
    }
    let raw = drive_raw(&lines, &connector);
    let out: Vec<Value> = raw
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let busy: Vec<&Value> = out
        .iter()
        .filter(|v| v["result"]["isError"] == true)
        .collect();
    assert_eq!(busy.len(), 1);
    assert_eq!(busy[0]["id"], 6, "the fifth call");
    assert!(
        busy[0]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("busy: ")
    );
    let ok = out
        .iter()
        .filter(|v| v["result"]["isError"] == false)
        .count();
    assert_eq!(ok, 4);
    assert_eq!(
        raw[1],
        serde_json::to_string(&out[1]).unwrap(),
        "the busy answer comes first"
    );
}

#[test]
fn ambiguous_candidates_are_passed_on() {
    let link = FakeLink::new(Box::new(|_| {
        Err(RequestError::Wire(WireError {
            code: AgentErrorCode::AmbiguousRef,
            candidates: Some(vec![mosaic_agent::wire::WireCandidate {
                vault: "Personal".to_owned(),
                title: "GitHub".to_owned(),
                id: "i1".to_owned(),
            }]),
        }))
    }));
    let connector = FakeConnector::always(&link);
    let out = drive(
        &[
            init(1, "2025-06-18"),
            call_line(
                2,
                "mosaic_request_access",
                &json!({ "ref": "mosaic://v/GitHub/password", "use": "request" }),
            ),
        ],
        &connector,
    );
    let result = &by_id(&out, 2)["result"];
    assert_eq!(
        result["structuredContent"]["candidates"][0]["title"],
        "GitHub"
    );
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Candidates")
    );
}

#[test]
fn stateless_discover_needs_no_initialize() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let out = drive(&[modern(1, "server/discover", &json!({}))], &connector);
    let result = &by_id(&out, 1)["result"];
    assert_stateless_result(result);
    assert_eq!(result["supportedVersions"], json!(all_versions()));
    assert_eq!(
        result["capabilities"],
        json!({ "tools": { "listChanged": false } }),
        "tools only, nothing else is offered"
    );
    assert_eq!(result["ttlMs"], 0);
    assert_eq!(result["cacheScope"], "public");
    assert!(result.get("instructions").is_none());
    assert_eq!(
        connector.connects.get(),
        0,
        "discovery never reaches the desktop"
    );
}

#[test]
fn stateless_tools_list_and_call_take_the_same_path() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let http = json!({ "ref": "mosaic://v/i/password", "method": "GET", "url": "https://api.mosaic.test/x" });
    let out = drive(
        &[
            modern(1, "tools/list", &json!({})),
            modern_call(2, "mosaic_status", &json!({})),
            modern_call(3, "mosaic_http_request", &http),
            modern_call(4, "mosaic_status", &json!({ "extra": 1 })),
            modern_call(5, "mosaic_reveal", &json!({})),
        ],
        &connector,
    );
    let list = &by_id(&out, 1)["result"];
    assert_stateless_result(list);
    assert_eq!(list["ttlMs"], 0);
    assert_eq!(list["cacheScope"], "public");
    // The same four tools as a `2025-11-25` session, with annotations.
    let session = drive(
        &[init(1, "2025-11-25"), request(2, "tools/list", &json!({}))],
        &FakeConnector::always(&link),
    );
    assert_eq!(list["tools"], by_id(&session, 2)["result"]["tools"]);
    assert!(list["tools"][0].get("annotations").is_some());

    let call = &by_id(&out, 2)["result"];
    assert_stateless_result(call);
    assert_eq!(call["isError"], false);
    assert_eq!(call["structuredContent"]["vault"], "unlocked");
    assert_eq!(by_id(&out, 3)["result"]["isError"], false);
    for id in [4, 5] {
        assert_eq!(by_id(&out, id)["error"]["code"], -32602, "{id}");
    }
    // Exactly the ops a session call forwards, and nothing for the refused calls.
    let seen = link.seen.borrow();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], OpRequest::Status {});
    assert!(matches!(
        &seen[1],
        OpRequest::HttpRequest { reference, method, url, headers: None, body: None, max_bytes: None }
            if reference == "mosaic://v/i/password" && method == "GET" && url == "https://api.mosaic.test/x"
    ));
}

#[test]
fn stateless_version_errors() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let call = json!({ "name": "mosaic_status", "arguments": {} });
    let with_meta = |id: u64, method: &str, meta: Value| {
        let mut params = call.clone();
        params
            .as_object_mut()
            .unwrap()
            .insert("_meta".to_owned(), meta);
        request(id, method, &params)
    };
    let out = drive(
        &[
            with_meta(1, "tools/call", meta("2099-01-01")),
            with_meta(2, "server/discover", meta("1900-01-01")),
            with_meta(
                3,
                "tools/call",
                json!({ "io.modelcontextprotocol/protocolVersion": STATELESS }),
            ),
            with_meta(
                4,
                "tools/call",
                json!({ "io.modelcontextprotocol/protocolVersion": 20_260_728, "io.modelcontextprotocol/clientCapabilities": {} }),
            ),
            modern(5, "initialize", &json!({ "protocolVersion": STATELESS })),
            modern(6, "ping", &json!({})),
            modern(
                7,
                "subscriptions/listen",
                &json!({ "notifications": { "toolsListChanged": true } }),
            ),
            modern(8, "resources/list", &json!({})),
        ],
        &connector,
    );
    for (id, requested) in [(1, "2099-01-01"), (2, "1900-01-01")] {
        let error = &by_id(&out, id)["error"];
        assert_eq!(error["code"], -32022, "{id}");
        assert_eq!(error["message"], "Unsupported protocol version");
        assert_eq!(error["data"]["supported"], json!(all_versions()));
        assert_eq!(error["data"]["requested"], requested);
    }
    for id in [3, 4] {
        assert_eq!(by_id(&out, id)["error"]["code"], -32602, "{id}");
    }
    for id in 5..=8 {
        assert_eq!(by_id(&out, id)["error"]["code"], -32601, "{id}");
    }
    assert_eq!(connector.connects.get(), 0, "nothing was forwarded");
}

#[test]
fn initialize_session_keeps_working_beside_stateless_requests() {
    let link = FakeLink::status();
    let connector = FakeConnector::always(&link);
    let out = drive(
        &[
            init(1, "2024-11-05"),
            request(2, "tools/list", &json!({})),
            modern(3, "tools/list", &json!({})),
            modern_call(4, "mosaic_status", &json!({})),
            request(5, "tools/list", &json!({})),
            call_line(6, "mosaic_status", &json!({})),
            request(7, "ping", &json!({})),
            // An `initialize` revision named in `_meta` belongs to the session.
            request(8, "tools/list", &json!({ "_meta": meta("2025-11-25") })),
        ],
        &connector,
    );
    assert_eq!(by_id(&out, 1)["result"]["protocolVersion"], "2024-11-05");
    for id in [2, 5, 8] {
        let list = &by_id(&out, id)["result"];
        assert!(list["tools"][0].get("annotations").is_none(), "{id}");
        assert!(list.get("resultType").is_none(), "{id}");
        assert!(list.get("ttlMs").is_none(), "{id}");
    }
    let modern_list = &by_id(&out, 3)["result"];
    assert_stateless_result(modern_list);
    assert!(modern_list["tools"][0].get("annotations").is_some());
    let modern_call = &by_id(&out, 4)["result"];
    assert_stateless_result(modern_call);
    assert!(modern_call.get("structuredContent").is_some());
    let session_call = &by_id(&out, 6)["result"];
    assert_eq!(session_call["isError"], false);
    assert!(session_call.get("structuredContent").is_none());
    assert!(session_call.get("resultType").is_none());
    assert!(session_call.get("_meta").is_none());
    assert_eq!(by_id(&out, 7)["result"], json!({}));
    assert_eq!(link.seen.borrow().len(), 2);
}

#[test]
fn cancelled_stateless_call_gets_no_response() {
    let link = Rc::new(FakeLink {
        responder: Box::new(|_| Ok(status_body())),
        delay: Duration::from_millis(300),
        closed: Cell::new(false),
        unpaired: Cell::new(false),
        close_after_first: false,
        seen: RefCell::new(Vec::new()),
    });
    let connector = FakeConnector::always(&link);
    let cancel = |id: u64| {
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": id } })
            .to_string()
    };
    let out = drive(
        &[
            modern_call(1, "mosaic_status", &json!({})),
            modern_call(2, "mosaic_status", &json!({})),
            call_line(3, "mosaic_status", &json!({})),
            cancel(1),
            cancel(3),
            cancel(99),
        ],
        &connector,
    );
    let ids: BTreeSet<u64> = out.iter().filter_map(|v| v["id"].as_u64()).collect();
    assert_eq!(
        ids,
        BTreeSet::from([2, 3]),
        "the cancelled stateless call is silent; the session keeps its answer"
    );
    assert_eq!(link.seen.borrow().len(), 3, "the desktop saw every call");
}
