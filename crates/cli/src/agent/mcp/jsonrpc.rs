//! JSON-RPC 2.0 framing for the relay: one JSON object per line.

use serde_json::{Value, json};

/// Invalid JSON.
pub const PARSE_ERROR: i64 = -32700;
/// Not a valid request object.
pub const INVALID_REQUEST: i64 = -32600;
/// Unknown method.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// Params that fail the schema.
pub const INVALID_PARAMS: i64 = -32602;
/// A stateless request names a protocol revision the relay does not speak (MCP `2026-07-28`).
pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// The longest line accepted from the client.
pub const MAX_LINE_BYTES: usize = 2 * 1024 * 1024;

/// A success response.
#[must_use]
pub fn result(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// An error response with a fixed message.
#[must_use]
pub fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// An error response with a fixed message and a `data` member.
#[must_use]
pub fn error_with_data(id: &Value, code: i64, message: &str, data: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message, "data": data } })
}

/// A message id is a string or a number.
#[must_use]
pub const fn valid_id(id: &Value) -> bool {
    matches!(id, Value::String(_) | Value::Number(_))
}

/// One line of output: compact JSON and a newline.
#[must_use]
pub fn line(value: &Value) -> String {
    let mut text = serde_json::to_string(value).unwrap_or_default();
    text.push('\n');
    text
}
