//! `mosaic agent mcp`: a stdio MCP server that forwards four tools to the running desktop and
//! never holds a secret value (ADR-0159).
//!
//! Newline-delimited JSON-RPC 2.0. Stdout holds only protocol responses; the relay never sends a
//! request or a notification to the client (no sampling, no elicitation, no roots, no logging),
//! offers no resources or prompts, and writes nothing to stderr. Up to four tool calls run at
//! once; a fifth is answered `busy`. The connection to the desktop is opened on the first call,
//! re-opened through a fresh `connect` after it closed (the desktop restarted or locked), and
//! never starts a pairing.

mod jsonrpc;
mod text;
mod tools;

#[cfg(test)]
mod tests;

use std::cell::{Cell, RefCell};
use std::io::{BufRead, Read as _, Write};
use std::rc::Rc;
use std::time::Duration;

use mosaic_agent::error::AgentErrorCode;
use mosaic_agent::wire::WireError;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::{Instant, timeout_at};

use super::conn::{ConnError, Connector, Link, RequestError};
use text::SUPPORTED_PROTOCOL_VERSIONS;
use tools::{Tool, err_result, list, ok_result, parse_call};

/// The most tool calls in flight.
const MAX_CALLS: usize = 4;

/// After stdin closed, how long to wait for calls in flight.
const DRAIN: Duration = Duration::from_secs(5);

/// What the main loop is told.
enum Msg {
    /// One line from the client.
    Line(Vec<u8>),
    /// A line longer than the limit: the stream cannot be trusted any more.
    TooLong,
    /// Stdin closed.
    Eof,
    /// A finished tool call: the response line.
    Done(String),
}

/// Reads one line of at most [`jsonrpc::MAX_LINE_BYTES`]. `Ok(None)` at the end of the stream.
fn read_line(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, ()> {
    let mut buf = Vec::new();
    let limit = jsonrpc::MAX_LINE_BYTES as u64 + 1;
    let n = reader
        .by_ref()
        .take(limit)
        .read_until(b'\n', &mut buf)
        .map_err(|_| ())?;
    if n == 0 {
        return Ok(None);
    }
    if buf.len() > jsonrpc::MAX_LINE_BYTES {
        return Err(());
    }
    Ok(Some(buf))
}

/// Serves MCP until stdin closes. Returns the process exit code, or the code of a startup
/// failure (the caller prints the fixed line).
///
/// # Errors
/// `"runtime"` or `"reader"` if the relay cannot start.
pub fn run_mcp(
    _client: &str,
    stdin: impl BufRead + Send + 'static,
    stdout: impl Write,
    connector: Rc<dyn Connector>,
) -> Result<u8, &'static str> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "runtime")?;
    let (tx, rx) = unbounded_channel::<Msg>();
    let reader_tx = tx.clone();
    std::thread::Builder::new()
        .name("mcp-stdin".to_owned())
        .spawn(move || {
            let mut stdin = stdin;
            loop {
                match read_line(&mut stdin) {
                    Ok(Some(line)) => {
                        if reader_tx.send(Msg::Line(line)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => break,
                    Err(()) => {
                        let _ = reader_tx.send(Msg::TooLong);
                        return;
                    }
                }
            }
            let _ = reader_tx.send(Msg::Eof);
        })
        .map_err(|_| "reader")?;
    let relay = Relay {
        connector,
        link: Rc::new(RefCell::new(None)),
        tx,
        version: Cell::new(None),
        in_flight: Cell::new(0),
    };
    Ok(tokio::task::LocalSet::new().block_on(&rt, relay.serve(rx, stdout)))
}

struct Relay {
    connector: Rc<dyn Connector>,
    link: Rc<RefCell<Option<Rc<dyn Link>>>>,
    tx: UnboundedSender<Msg>,
    /// The negotiated protocol revision.
    version: Cell<Option<&'static str>>,
    in_flight: Cell<usize>,
}

fn write_value(out: &mut impl Write, value: &Value) {
    let _ = out.write_all(jsonrpc::line(value).as_bytes());
    let _ = out.flush();
}

impl Relay {
    async fn serve(self, mut rx: UnboundedReceiver<Msg>, mut out: impl Write) -> u8 {
        let mut deadline: Option<Instant> = None;
        loop {
            let msg = match deadline {
                None => rx.recv().await,
                Some(_) if self.in_flight.get() == 0 => break,
                Some(at) => match timeout_at(at, rx.recv()).await {
                    Ok(msg) => msg,
                    Err(_) => break,
                },
            };
            match msg {
                None => break,
                Some(Msg::Eof) => deadline = Some(Instant::now() + DRAIN),
                Some(Msg::TooLong) => {
                    write_value(
                        &mut out,
                        &jsonrpc::error(&Value::Null, jsonrpc::PARSE_ERROR, "Parse error"),
                    );
                    return 1;
                }
                Some(Msg::Done(line)) => {
                    self.in_flight.set(self.in_flight.get().saturating_sub(1));
                    let _ = out.write_all(line.as_bytes());
                    let _ = out.flush();
                }
                Some(Msg::Line(bytes)) => {
                    if let Some(response) = self.handle(&bytes) {
                        write_value(&mut out, &response);
                    }
                }
            }
        }
        0
    }

    fn negotiated(&self) -> &'static str {
        self.version.get().unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0])
    }

    /// Handles one line. `Some` is the response to write now; a tool call answers later.
    fn handle(&self, bytes: &[u8]) -> Option<Value> {
        let Ok(message) = serde_json::from_slice::<Value>(bytes) else {
            return Some(jsonrpc::error(
                &Value::Null,
                jsonrpc::PARSE_ERROR,
                "Parse error",
            ));
        };
        let Some(object) = message.as_object() else {
            return Some(jsonrpc::error(
                &Value::Null,
                jsonrpc::INVALID_REQUEST,
                "Invalid request",
            ));
        };
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            // A response from the client: the relay never asks anything, so there is nothing to
            // match it to.
            return None;
        };
        let Some(id) = object.get("id") else {
            // A notification (`notifications/initialized`, `notifications/cancelled`, ...).
            return None;
        };
        if !jsonrpc::valid_id(id) {
            return Some(jsonrpc::error(
                &Value::Null,
                jsonrpc::INVALID_REQUEST,
                "Invalid request",
            ));
        }
        let params = object.get("params").unwrap_or(&Value::Null);
        match method {
            "initialize" => Some(jsonrpc::result(id, &self.initialize(params))),
            "ping" => Some(jsonrpc::result(id, &json!({}))),
            "tools/list" => Some(jsonrpc::result(id, &list(self.negotiated()))),
            "tools/call" => self.tools_call(id, params),
            _ => Some(jsonrpc::error(
                id,
                jsonrpc::METHOD_NOT_FOUND,
                "Method not found",
            )),
        }
    }

    fn initialize(&self, params: &Value) -> Value {
        let asked = params.get("protocolVersion").and_then(Value::as_str);
        let version = SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|v| Some(*v) == asked)
            .unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0]);
        self.version.set(Some(version));
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "mosaic", "version": env!("CARGO_PKG_VERSION") },
        })
    }

    fn tools_call(&self, id: &Value, params: &Value) -> Option<Value> {
        let invalid = || {
            Some(jsonrpc::error(
                id,
                jsonrpc::INVALID_PARAMS,
                "Invalid params",
            ))
        };
        let Some(params) = params.as_object() else {
            return invalid();
        };
        if params
            .keys()
            .any(|k| !matches!(k.as_str(), "name" | "arguments" | "_meta"))
        {
            return invalid();
        }
        let Some(tool) = params
            .get("name")
            .and_then(Value::as_str)
            .and_then(Tool::by_name)
        else {
            return invalid();
        };
        let Ok(op) = parse_call(tool, params.get("arguments").unwrap_or(&Value::Null)) else {
            return invalid();
        };
        let version = self.negotiated();
        if self.in_flight.get() >= MAX_CALLS {
            return Some(jsonrpc::result(
                id,
                &err_result(version, AgentErrorCode::Busy, None),
            ));
        }
        self.in_flight.set(self.in_flight.get() + 1);
        let (connector, slot, tx, id) = (
            Rc::clone(&self.connector),
            Rc::clone(&self.link),
            self.tx.clone(),
            id.clone(),
        );
        tokio::task::spawn_local(async move {
            let result = call(
                &*connector,
                &slot,
                op,
                Duration::from_secs(tool.wait_secs()),
                version,
            )
            .await;
            let line = jsonrpc::line(&jsonrpc::result(&id, &result));
            let _ = tx.send(Msg::Done(line));
        });
        None
    }
}

/// The code the model sees for a failed connection.
const fn conn_code(e: ConnError) -> AgentErrorCode {
    match e {
        ConnError::DesktopNotRunning => AgentErrorCode::DesktopNotRunning,
        ConnError::NotPaired => AgentErrorCode::NotPaired,
        ConnError::Locked => AgentErrorCode::Locked,
        ConnError::AgentAccessOff => AgentErrorCode::AgentAccessOff,
        ConnError::Refused(mosaic_native_messaging::agent_preauth::AgentRefusal::Busy) => {
            AgentErrorCode::Busy
        }
        ConnError::Refused(mosaic_native_messaging::agent_preauth::AgentRefusal::RateLimited) => {
            AgentErrorCode::RateLimited
        }
        ConnError::Refused(_) => AgentErrorCode::Internal,
        ConnError::Io => AgentErrorCode::RequestFailed,
    }
}

/// A live link: the one in `slot`, or a fresh `connect`.
async fn link_for(
    connector: &dyn Connector,
    slot: &RefCell<Option<Rc<dyn Link>>>,
) -> Result<Rc<dyn Link>, ConnError> {
    let current = slot.borrow().clone();
    if let Some(link) = current
        && !link.is_closed()
    {
        return Ok(link);
    }
    let link = connector.connect().await?;
    *slot.borrow_mut() = Some(Rc::clone(&link));
    Ok(link)
}

/// Runs one tool call to its result.
async fn call(
    connector: &dyn Connector,
    slot: &RefCell<Option<Rc<dyn Link>>>,
    op: mosaic_agent::wire::OpRequest,
    wait: Duration,
    version: &str,
) -> Value {
    let link = match link_for(connector, slot).await {
        Ok(link) => link,
        Err(e) => return err_result(version, conn_code(e), None),
    };
    let answer = link.request(op, wait).await;
    if link.unpaired() {
        // The desktop removed this pairing: forget the key; later calls find none.
        connector.forget();
        *slot.borrow_mut() = None;
        return err_result(version, AgentErrorCode::NotPaired, None);
    }
    match answer {
        Ok(body) => ok_result(version, &body),
        Err(RequestError::Wire(WireError { code, candidates })) => {
            err_result(version, code, candidates.as_deref())
        }
        Err(RequestError::Conn(e)) => {
            if link.is_closed() {
                *slot.borrow_mut() = None;
            }
            err_result(version, conn_code(e), None)
        }
        Err(RequestError::Timeout) => err_result(version, AgentErrorCode::Timeout, None),
    }
}
