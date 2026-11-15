//! The stateless revision `2026-07-28` (ADR-0159): no `initialize`, every request names its
//! protocol version and the client's capabilities in `params._meta`, `server/discover`, and the
//! fields every result carries (`resultType`, the server's identity, cache hints on lists).
//!
//! The relay is dual-era: a request whose `_meta` names a stateless revision is served on its
//! own, with nothing taken from earlier requests; anything else is served by the `initialize`
//! session as before. Both eras reach the same four tools through the same path.

use serde_json::{Value, json};

use super::jsonrpc;
use super::text::{INITIALIZE_VERSIONS, STATELESS_VERSIONS};

/// The `_meta` key with the request's protocol version (required).
pub const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
/// The `_meta` key with the client's capabilities for this request (required).
pub const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
/// The result `_meta` key with the server's name and version.
pub const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// How long a client may treat `server/discover` and `tools/list` as fresh: `0`, re-fetch each
/// time (a stdio request costs nothing, and nothing goes stale this way).
const TTL_MS: u64 = 0;

/// How one request is served.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Era {
    /// By the `initialize` session (or its default, the newest handshake revision).
    Session,
    /// On its own, under this stateless revision.
    Stateless(&'static str),
}

/// Why a stateless request is refused before its method runs.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A required `_meta` field is missing or of the wrong type (`-32602`).
    Malformed,
    /// The named revision is not one this relay speaks (`-32022`); the name as sent.
    Unsupported(String),
}

/// Every revision the relay speaks, newest first: the stateless ones, then the `initialize` ones.
#[must_use]
pub fn supported_versions() -> Vec<&'static str> {
    STATELESS_VERSIONS
        .iter()
        .chain(INITIALIZE_VERSIONS.iter())
        .copied()
        .collect()
}

/// The era of a request, from its `params`.
///
/// Without a version in `_meta` (or with one of the `initialize` revisions, which have no
/// per-request metadata) the request belongs to the session. Any other version makes it a
/// stateless request, which must then name a supported revision and carry the capabilities.
///
/// # Errors
/// [`Refusal::Malformed`] for a version that is not a string or missing capabilities,
/// [`Refusal::Unsupported`] for a revision the relay does not speak.
pub fn era(params: &Value) -> Result<Era, Refusal> {
    let meta = params.get("_meta");
    let Some(asked) = meta.and_then(|m| m.get(VERSION_KEY)) else {
        return Ok(Era::Session);
    };
    let Some(asked) = asked.as_str() else {
        return Err(Refusal::Malformed);
    };
    if INITIALIZE_VERSIONS.contains(&asked) {
        return Ok(Era::Session);
    }
    let Some(version) = STATELESS_VERSIONS.iter().copied().find(|v| *v == asked) else {
        return Err(Refusal::Unsupported(asked.to_owned()));
    };
    match meta.and_then(|m| m.get(CAPABILITIES_KEY)) {
        Some(Value::Object(_)) => Ok(Era::Stateless(version)),
        _ => Err(Refusal::Malformed),
    }
}

/// The error response for a refused stateless request.
#[must_use]
pub fn refusal(id: &Value, refusal: &Refusal) -> Value {
    match refusal {
        Refusal::Malformed => jsonrpc::error(id, jsonrpc::INVALID_PARAMS, "Invalid params"),
        Refusal::Unsupported(asked) => jsonrpc::error_with_data(
            id,
            jsonrpc::UNSUPPORTED_PROTOCOL_VERSION,
            "Unsupported protocol version",
            &json!({ "supported": supported_versions(), "requested": asked }),
        ),
    }
}

/// The relay's name and version (`serverInfo` of `initialize`, `_meta` of stateless results).
#[must_use]
pub fn server_info() -> Value {
    json!({ "name": "mosaic", "version": env!("CARGO_PKG_VERSION") })
}

/// `result` as a finished stateless result: `resultType: "complete"` and the server's identity.
#[must_use]
pub fn complete(mut result: Value) -> Value {
    if let Some(object) = result.as_object_mut() {
        object.insert("resultType".to_owned(), json!("complete"));
        object.insert(
            "_meta".to_owned(),
            json!({ SERVER_INFO_KEY: server_info() }),
        );
    }
    result
}

/// `result` with the cache hints that list results carry.
#[must_use]
pub fn cacheable(mut result: Value) -> Value {
    if let Some(object) = result.as_object_mut() {
        object.insert("ttlMs".to_owned(), json!(TTL_MS));
        object.insert("cacheScope".to_owned(), json!("public"));
    }
    result
}

/// The `server/discover` result: the revisions, the `tools` capability only (no list changes),
/// the identity and the cache hints. No `instructions`: the tool descriptions say it all.
#[must_use]
pub fn discover() -> Value {
    cacheable(complete(json!({
        "supportedVersions": supported_versions(),
        "capabilities": { "tools": { "listChanged": false } },
    })))
}
