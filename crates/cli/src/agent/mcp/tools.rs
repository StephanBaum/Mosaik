//! The four tools: their schemas, the validation of a call, and the shape of results.
//!
//! No tool returns a secret value, and the schemas have no place to put one: a secret is named
//! by its `mosaic://` reference only.

use std::collections::BTreeMap;

use mosaic_agent::error::AgentErrorCode;
use mosaic_agent::wire::{AccessUse, OpRequest, OpResponse, WireCandidate};
use serde_json::{Map, Value, json};

use super::text::{
    ACCESS_DESCRIPTION, ANNOTATIONS_FROM, HTTP_DESCRIPTION, LIST_DESCRIPTION, STATUS_DESCRIPTION,
    STRUCTURED_FROM, sentence,
};

/// Which tool a call names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    /// `mosaic_status`.
    Status,
    /// `mosaic_list_secrets`.
    ListSecrets,
    /// `mosaic_request_access`.
    RequestAccess,
    /// `mosaic_http_request`.
    HttpRequest,
}

impl Tool {
    /// The tool with this name.
    #[must_use]
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "mosaic_status" => Some(Self::Status),
            "mosaic_list_secrets" => Some(Self::ListSecrets),
            "mosaic_request_access" => Some(Self::RequestAccess),
            "mosaic_http_request" => Some(Self::HttpRequest),
            _ => None,
        }
    }

    /// How long the relay waits for the desktop's answer.
    #[must_use]
    pub const fn wait_secs(self) -> u64 {
        match self {
            Self::RequestAccess => 130,
            Self::HttpRequest => 70,
            Self::Status | Self::ListSecrets => 15,
        }
    }
}

/// A call whose arguments do not fit the schema (answered as JSON-RPC `-32602`, never forwarded).
#[derive(Debug, PartialEq, Eq)]
pub struct ParamError;

/// Whether `version` is `from` or newer (revisions are dates, so string order is date order).
fn at_least(version: &str, from: &str) -> bool {
    version >= from
}

fn strings(max: Option<usize>, min: usize) -> Value {
    let mut schema = json!({ "type": "string", "minLength": min });
    if let (Some(max), Some(object)) = (max, schema.as_object_mut()) {
        object.insert("maxLength".to_owned(), json!(max));
    }
    schema
}

fn definition(
    name: &str,
    description: &str,
    properties: &Value,
    required: &[&str],
    annotations: &Value,
    version: &str,
) -> Value {
    let mut tool = json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    });
    if at_least(version, ANNOTATIONS_FROM)
        && let Some(object) = tool.as_object_mut()
    {
        object.insert("annotations".to_owned(), annotations.clone());
    }
    tool
}

/// The `tools/list` result for the negotiated `version`.
#[must_use]
pub fn list(version: &str) -> Value {
    let tools = vec![
        definition(
            "mosaic_status",
            STATUS_DESCRIPTION,
            &json!({}),
            &[],
            &json!({ "readOnlyHint": true }),
            version,
        ),
        definition(
            "mosaic_list_secrets",
            LIST_DESCRIPTION,
            &json!({ "query": strings(Some(128), 0) }),
            &[],
            &json!({ "readOnlyHint": true }),
            version,
        ),
        definition(
            "mosaic_request_access",
            ACCESS_DESCRIPTION,
            &json!({
                "ref": strings(Some(2048), 1),
                "use": { "type": "string", "enum": ["request"] },
                "reason": strings(Some(200), 0),
            }),
            &["ref", "use"],
            &json!({ "readOnlyHint": true }),
            version,
        ),
        definition(
            "mosaic_http_request",
            HTTP_DESCRIPTION,
            &json!({
                "ref": strings(Some(2048), 1),
                "method": { "type": "string", "enum": ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"] },
                "url": strings(Some(8192), 1),
                "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                "body": { "type": "string" },
                "maxBytes": { "type": "integer", "minimum": 1, "maximum": 1_048_576 },
            }),
            &["ref", "method", "url"],
            &json!({ "readOnlyHint": false, "openWorldHint": true }),
            version,
        ),
    ];
    json!({ "tools": tools })
}

const METHODS: [&str; 6] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"];

/// The arguments as an object with only the `allowed` keys.
fn object<'a>(
    args: &'a Value,
    allowed: &[&str],
) -> Result<Option<&'a Map<String, Value>>, ParamError> {
    match args {
        Value::Null => Ok(None),
        Value::Object(map) if map.keys().all(|k| allowed.contains(&k.as_str())) => Ok(Some(map)),
        _ => Err(ParamError),
    }
}

fn text_arg(
    map: Option<&Map<String, Value>>,
    key: &str,
    min: usize,
    max: usize,
    required: bool,
) -> Result<Option<String>, ParamError> {
    match map.and_then(|m| m.get(key)) {
        None => {
            if required {
                Err(ParamError)
            } else {
                Ok(None)
            }
        }
        Some(Value::String(s)) => {
            let n = s.chars().count();
            if (min..=max).contains(&n) {
                Ok(Some(s.clone()))
            } else {
                Err(ParamError)
            }
        }
        Some(_) => Err(ParamError),
    }
}

/// Validates the `arguments` of a call against the tool's schema and builds the request.
///
/// # Errors
/// [`ParamError`] for an unknown property, a missing required one, a wrong type or a length or
/// range outside the schema.
pub fn parse_call(tool: Tool, args: &Value) -> Result<OpRequest, ParamError> {
    match tool {
        Tool::Status => {
            let map = object(args, &[])?;
            let empty = map.is_none_or(Map::is_empty);
            if empty {
                Ok(OpRequest::Status {})
            } else {
                Err(ParamError)
            }
        }
        Tool::ListSecrets => {
            let map = object(args, &["query"])?;
            Ok(OpRequest::ListSecrets {
                query: text_arg(map, "query", 0, 128, false)?,
            })
        }
        Tool::RequestAccess => {
            let map = object(args, &["ref", "use", "reason"])?;
            let reference = text_arg(map, "ref", 1, 2048, true)?.ok_or(ParamError)?;
            match map.and_then(|m| m.get("use")) {
                Some(Value::String(s)) if s == "request" => {}
                _ => return Err(ParamError),
            }
            Ok(OpRequest::RequestAccess {
                reference,
                access_use: AccessUse::Request,
                reason: text_arg(map, "reason", 0, 200, false)?,
            })
        }
        Tool::HttpRequest => {
            let map = object(
                args,
                &["ref", "method", "url", "headers", "body", "maxBytes"],
            )?;
            let reference = text_arg(map, "ref", 1, 2048, true)?.ok_or(ParamError)?;
            let method = text_arg(map, "method", 1, 16, true)?.ok_or(ParamError)?;
            if !METHODS.contains(&method.as_str()) {
                return Err(ParamError);
            }
            let url = text_arg(map, "url", 1, 8192, true)?.ok_or(ParamError)?;
            let headers = match map.and_then(|m| m.get("headers")) {
                None => None,
                Some(Value::Object(h)) => {
                    let mut out = BTreeMap::new();
                    for (k, v) in h {
                        let Value::String(v) = v else {
                            return Err(ParamError);
                        };
                        out.insert(k.clone(), v.clone());
                    }
                    Some(out)
                }
                Some(_) => return Err(ParamError),
            };
            let body = match map.and_then(|m| m.get("body")) {
                None => None,
                Some(Value::String(s)) => Some(s.clone()),
                Some(_) => return Err(ParamError),
            };
            let max_bytes = match map.and_then(|m| m.get("maxBytes")) {
                None => None,
                Some(v) => {
                    let n = v.as_u64().ok_or(ParamError)?;
                    if !(1..=1_048_576).contains(&n) {
                        return Err(ParamError);
                    }
                    Some(u32::try_from(n).map_err(|_| ParamError)?)
                }
            };
            Ok(OpRequest::HttpRequest {
                reference,
                method,
                url,
                headers,
                body,
                max_bytes,
            })
        }
    }
}

/// The result of a successful call: the desktop's answer as compact JSON text (and as
/// `structuredContent` from `2025-06-18`).
#[must_use]
pub fn ok_result(version: &str, body: &OpResponse) -> Value {
    let structured = serde_json::to_value(body).unwrap_or(Value::Null);
    let text = serde_json::to_string(&structured).unwrap_or_default();
    let mut result = json!({ "content": [{ "type": "text", "text": text }], "isError": false });
    if at_least(version, STRUCTURED_FROM)
        && let Some(object) = result.as_object_mut()
    {
        object.insert("structuredContent".to_owned(), structured);
    }
    result
}

/// The result of a failed call: `"<code>: <fixed sentence>"`, the code and, for `ambiguousRef`,
/// the candidates (titles and IDs, which hold no value).
#[must_use]
pub fn err_result(
    version: &str,
    code: AgentErrorCode,
    candidates: Option<&[WireCandidate]>,
) -> Value {
    let mut text = format!("{}: {}", code.as_str(), sentence(code));
    let listed = candidates.filter(|c| !c.is_empty());
    if let Some(list) = listed {
        text.push_str(" Candidates: ");
        text.push_str(&serde_json::to_string(list).unwrap_or_default());
    }
    let mut result = json!({ "content": [{ "type": "text", "text": text }], "isError": true });
    if at_least(version, STRUCTURED_FROM)
        && let Some(object) = result.as_object_mut()
    {
        let mut structured = json!({ "error": code.as_str() });
        if let (Some(list), Some(s)) = (listed, structured.as_object_mut()) {
            s.insert(
                "candidates".to_owned(),
                serde_json::to_value(list).unwrap_or(Value::Null),
            );
        }
        object.insert("structuredContent".to_owned(), structured);
    }
    result
}
