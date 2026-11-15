//! The messages of the agent endpoint after the handshake (`docs/NATIVE_MESSAGING.md`
//! section 9.3): requests, responses and events as JSON.
//!
//! ```text
//! request   {"v":1,"id":<u32>,"op":<name>,"body":{...}}
//! response  {"v":1,"id":<u32>,"ok":true,"body":{...}}
//!           {"v":1,"id":<u32>,"ok":false,"error":{"code":<code>,"candidates":[...]?}}
//! event     {"v":1,"event":"state",...} | {"v":1,"event":"unpaired"} | {"v":1,"event":"grantRevoked","ref":<ref>}
//! ```
//!
//! Every object rejects unknown members. There is no op that returns a vault value: the four ops
//! are `status`, `listSecrets`, `requestAccess` and `httpRequest`; any other name, among them
//! `get`, `reveal`, `copy`, `inject` and `runStart`, is `invalidRequest`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use mosaic_native_messaging::agent_preauth::{AccessState, VaultState};

use crate::binding::InjectKind;
use crate::engine::{AgentFailure, HttpResponseBody, SecretEntry};
use crate::error::AgentErrorCode;
use crate::grants::GrantState;

/// The version of the messages.
pub const PROTOCOL_VERSION: u8 = 1;
/// The most agent pairings.
pub const MAX_PAIRINGS: usize = 16;
/// The most open authenticated connections.
pub const MAX_CONNECTIONS: usize = 8;
/// The most requests in flight on one connection.
pub const MAX_IN_FLIGHT: usize = 4;
/// How long an access dialog waits for the user.
pub const ACCESS_TIMEOUT_SECS: u64 = 120;
/// A connection without a frame for this long is closed.
pub const IDLE_SECS: u64 = 600;

/// What an access request asks for. Phase 1 has `request` only (commands come later).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessUse {
    /// HTTPS requests.
    Request,
}

/// One request of the relay. The wire form is `"op": <name>, "body": {...}`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "op",
    content = "body",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OpRequest {
    /// The vault and access state.
    Status {},
    /// The usable secrets, filtered by a query.
    ListSecrets {
        /// A case-insensitive filter, at most 128 characters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        query: Option<String>,
    },
    /// Asks the user for a grant.
    RequestAccess {
        /// The `mosaic://` reference.
        #[serde(rename = "ref")]
        reference: String,
        /// What for.
        #[serde(rename = "use")]
        access_use: AccessUse,
        /// Why, at most 200 characters (shown to the user as the agent's words).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// Sends one HTTPS request with the secret injected.
    HttpRequest {
        /// The `mosaic://` reference.
        #[serde(rename = "ref")]
        reference: String,
        /// The method.
        method: String,
        /// The URL.
        url: String,
        /// Extra headers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<BTreeMap<String, String>>,
        /// A text body.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        /// The most response body bytes to return.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_bytes: Option<u32>,
    },
}

/// A request with its ID.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RequestEnvelope {
    /// The caller's ID, echoed in the response.
    pub id: u32,
    /// The operation.
    pub op: OpRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    v: u8,
    id: u32,
    op: String,
    body: serde_json::Value,
}

#[derive(Serialize)]
struct RequestOut<'a> {
    v: u8,
    id: u32,
    #[serde(flatten)]
    op: &'a OpRequest,
}

impl RequestEnvelope {
    /// The JSON of this request.
    ///
    /// # Errors
    /// [`AgentErrorCode::Internal`] if serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, AgentErrorCode> {
        serde_json::to_vec(&RequestOut {
            v: PROTOCOL_VERSION,
            id: self.id,
            op: &self.op,
        })
        .map_err(|_| AgentErrorCode::Internal)
    }
}

/// The ID of a request, if the bytes are a JSON object with a `u32` `id`. Used to answer a
/// request that is otherwise refused.
#[must_use]
pub fn request_id(bytes: &[u8]) -> Option<u32> {
    #[derive(Deserialize)]
    struct IdOnly {
        id: u32,
    }
    serde_json::from_slice::<IdOnly>(bytes).ok().map(|i| i.id)
}

/// Parses a request frame.
///
/// # Errors
/// [`AgentErrorCode::InvalidRequest`] for anything that is not exactly one of the four ops with
/// the members of its body: an unknown op (`get`, `reveal`, `copy`, `inject`, `runStart`, ...),
/// an extra or missing member, a wrong type, or a version other than 1.
pub fn parse_request(bytes: &[u8]) -> Result<RequestEnvelope, AgentErrorCode> {
    let raw: RawRequest =
        serde_json::from_slice(bytes).map_err(|_| AgentErrorCode::InvalidRequest)?;
    if raw.v != PROTOCOL_VERSION {
        return Err(AgentErrorCode::InvalidRequest);
    }
    let mut tagged = serde_json::Map::with_capacity(2);
    tagged.insert("op".to_owned(), serde_json::Value::String(raw.op));
    tagged.insert("body".to_owned(), raw.body);
    let op: OpRequest = serde_json::from_value(serde_json::Value::Object(tagged))
        .map_err(|_| AgentErrorCode::InvalidRequest)?;
    Ok(RequestEnvelope { id: raw.id, op })
}

/// One secret in `listSecrets`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireSecret {
    /// The reference.
    #[serde(rename = "ref")]
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

impl From<SecretEntry> for WireSecret {
    fn from(e: SecretEntry) -> Self {
        Self {
            reference: e.reference,
            title: e.title,
            vault: e.vault,
            field: e.field,
            origins: e.origins,
            inject: e.inject,
            env: e.env,
            run_allowed: e.run_allowed,
            grant: e.grant,
            needs_review: e.needs_review,
        }
    }
}

/// The answer of `status`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusBody {
    /// Locked or unlocked.
    pub vault: VaultState,
    /// Agent access on or off.
    pub agent_access: AccessState,
}

/// The answer of `listSecrets`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretsBody {
    /// At most 50 entries.
    pub secrets: Vec<WireSecret>,
}

/// What the user decided.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessDecision {
    /// Allowed.
    Granted,
    /// Denied.
    Denied,
    /// The dialog ran out.
    Timeout,
}

/// The answer of `requestAccess`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessBody {
    /// The decision.
    pub decision: AccessDecision,
    /// When a timed grant ends (ms since the Unix epoch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// The grant now.
    pub grant: GrantState,
}

/// A binary body, described and not returned.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireBinary {
    /// The `Content-Type`, masked.
    pub content_type: Option<String>,
    /// How many bytes were read.
    pub bytes: u64,
    /// The `Content-Encoding`, if other than `identity`.
    pub content_encoding: Option<String>,
}

/// The answer of `httpRequest`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HttpBody {
    /// The status code.
    pub status: u16,
    /// Headers without `Set-Cookie`, masked.
    pub headers: Vec<(String, String)>,
    /// The masked text body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// What a binary body was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<WireBinary>,
    /// The body was cut.
    pub truncated: bool,
    /// How many replacements the masker made.
    pub masked: u32,
    /// The masked `Location` of a redirect (not followed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect: Option<String>,
}

impl From<HttpResponseBody> for HttpBody {
    fn from(r: HttpResponseBody) -> Self {
        Self {
            status: r.status,
            headers: r.headers,
            body: r.body,
            binary: r.binary.map(|b| WireBinary {
                content_type: b.content_type,
                bytes: b.bytes,
                content_encoding: b.content_encoding,
            }),
            truncated: r.truncated,
            masked: r.masked,
            redirect: r.redirect,
        }
    }
}

/// The body of an `ok` response. The shapes differ in their members, so the variant follows from
/// the JSON (the client also knows which op it sent).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpResponse {
    /// `status`.
    Status(StatusBody),
    /// `listSecrets`.
    Secrets(SecretsBody),
    /// `requestAccess`.
    Access(AccessBody),
    /// `httpRequest`.
    Http(HttpBody),
}

/// One candidate of an ambiguous reference.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCandidate {
    /// The vault's name.
    pub vault: String,
    /// The item's title.
    pub title: String,
    /// The item's ID.
    pub id: String,
}

/// The error of a failed response: a code and, for `ambiguousRef` only, candidates. Never a
/// value, never an echo of the input.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireError {
    /// The code.
    pub code: AgentErrorCode,
    /// Titles and IDs of the usable items an ambiguous reference could mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<WireCandidate>>,
}

impl WireError {
    /// An error with only a code.
    #[must_use]
    pub const fn code(code: AgentErrorCode) -> Self {
        Self {
            code,
            candidates: None,
        }
    }
}

impl From<AgentErrorCode> for WireError {
    fn from(code: AgentErrorCode) -> Self {
        Self::code(code)
    }
}

impl From<AgentFailure> for WireError {
    fn from(f: AgentFailure) -> Self {
        let candidates =
            (f.code == AgentErrorCode::AmbiguousRef && !f.candidates.is_empty()).then(|| {
                f.candidates
                    .into_iter()
                    .map(|c| WireCandidate {
                        vault: c.vault,
                        title: c.title,
                        id: c.id.to_string(),
                    })
                    .collect()
            });
        Self {
            code: f.code,
            candidates,
        }
    }
}

/// A response with its ID.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    /// Always 1.
    pub v: u8,
    /// The request's ID.
    pub id: u32,
    /// Whether it succeeded.
    pub ok: bool,
    /// The body of a success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<OpResponse>,
    /// The error of a failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WireError>,
}

impl ResponseEnvelope {
    /// A success.
    #[must_use]
    pub const fn ok(id: u32, body: OpResponse) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            ok: true,
            body: Some(body),
            error: None,
        }
    }

    /// A failure.
    #[must_use]
    pub const fn failed(id: u32, error: WireError) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            ok: false,
            body: None,
            error: Some(error),
        }
    }

    /// The JSON of this response.
    ///
    /// # Errors
    /// [`AgentErrorCode::Internal`] if serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, AgentErrorCode> {
        serde_json::to_vec(self).map_err(|_| AgentErrorCode::Internal)
    }

    /// Parses a response frame; the shape must be consistent (`ok` with a body, a failure with
    /// an error).
    ///
    /// # Errors
    /// [`AgentErrorCode::InvalidRequest`] if the frame is not a response of this protocol.
    pub fn parse(bytes: &[u8]) -> Result<Self, AgentErrorCode> {
        let r: Self = serde_json::from_slice(bytes).map_err(|_| AgentErrorCode::InvalidRequest)?;
        let consistent = r.v == PROTOCOL_VERSION
            && if r.ok {
                r.body.is_some() && r.error.is_none()
            } else {
                r.body.is_none() && r.error.is_some()
            };
        if consistent {
            Ok(r)
        } else {
            Err(AgentErrorCode::InvalidRequest)
        }
    }
}

/// A message the desktop sends on its own.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    /// The vault or access state changed (also the first frame).
    State {
        /// Locked or unlocked.
        vault: VaultState,
        /// Agent access on or off.
        agent_access: AccessState,
    },
    /// This pairing was removed on the desktop; the connection closes.
    Unpaired,
    /// A grant of this pairing ended.
    GrantRevoked {
        /// The reference.
        #[serde(rename = "ref")]
        reference: String,
    },
}

#[derive(Serialize)]
struct EventOut<'a> {
    v: u8,
    #[serde(flatten)]
    event: &'a Event,
}

impl Event {
    /// The JSON of this event, with `"v":1`.
    ///
    /// # Errors
    /// [`AgentErrorCode::Internal`] if serialization fails.
    pub fn to_json(&self) -> Result<Vec<u8>, AgentErrorCode> {
        serde_json::to_vec(&EventOut {
            v: PROTOCOL_VERSION,
            event: self,
        })
        .map_err(|_| AgentErrorCode::Internal)
    }

    /// Parses an event frame.
    ///
    /// # Errors
    /// [`AgentErrorCode::InvalidRequest`] if the frame is not an event of this protocol.
    pub fn parse(bytes: &[u8]) -> Result<Self, AgentErrorCode> {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| AgentErrorCode::InvalidRequest)?;
        let object = value
            .as_object_mut()
            .ok_or(AgentErrorCode::InvalidRequest)?;
        match object.remove("v") {
            Some(v) if v == PROTOCOL_VERSION => {}
            _ => return Err(AgentErrorCode::InvalidRequest),
        }
        // A unit variant of an internally tagged enum ignores extra members.
        if object.get("event") == Some(&serde_json::Value::from("unpaired")) && object.len() != 1 {
            return Err(AgentErrorCode::InvalidRequest);
        }
        serde_json::from_value(value).map_err(|_| AgentErrorCode::InvalidRequest)
    }
}

/// What a plaintext frame from the desktop is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DesktopFrame {
    /// An event.
    Event(Event),
    /// A response.
    Response(ResponseEnvelope),
}

impl DesktopFrame {
    /// Parses a frame from the desktop: an object with `event` is an event, else a response.
    ///
    /// # Errors
    /// [`AgentErrorCode::InvalidRequest`] if it is neither.
    pub fn parse(bytes: &[u8]) -> Result<Self, AgentErrorCode> {
        let probe: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| AgentErrorCode::InvalidRequest)?;
        if probe.get("event").is_some() {
            Event::parse(bytes).map(Self::Event)
        } else {
            ResponseEnvelope::parse(bytes).map(Self::Response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(json: &str) -> OpRequest {
        parse_request(json.as_bytes()).unwrap().op
    }

    #[test]
    fn requests_round_trip_every_body() {
        let samples = [
            OpRequest::Status {},
            OpRequest::ListSecrets { query: None },
            OpRequest::ListSecrets {
                query: Some("github".to_owned()),
            },
            OpRequest::RequestAccess {
                reference: "mosaic://v/i/password".to_owned(),
                access_use: AccessUse::Request,
                reason: Some("deploy".to_owned()),
            },
            OpRequest::HttpRequest {
                reference: "mosaic://v/i/password".to_owned(),
                method: "GET".to_owned(),
                url: "https://api.example.com/v1".to_owned(),
                headers: Some(BTreeMap::from([("accept".to_owned(), "x".to_owned())])),
                body: Some("{}".to_owned()),
                max_bytes: Some(1024),
            },
        ];
        for (i, op) in samples.into_iter().enumerate() {
            let envelope = RequestEnvelope {
                id: u32::try_from(i).unwrap(),
                op,
            };
            let json = envelope.to_json().unwrap();
            assert_eq!(parse_request(&json).unwrap(), envelope);
        }
        assert_eq!(
            rt(r#"{"v":1,"id":1,"op":"status","body":{}}"#),
            OpRequest::Status {}
        );
    }

    #[test]
    fn requests_are_strict() {
        for bad in [
            r#"{"v":1,"id":1,"op":"get","body":{}}"#,
            r#"{"v":1,"id":1,"op":"reveal","body":{}}"#,
            r#"{"v":1,"id":1,"op":"copy","body":{}}"#,
            r#"{"v":1,"id":1,"op":"inject","body":{}}"#,
            r#"{"v":1,"id":1,"op":"runStart","body":{}}"#,
            r#"{"v":1,"id":1,"op":"status","body":{"x":1}}"#,
            r#"{"v":1,"id":1,"op":"status","body":{},"extra":1}"#,
            r#"{"v":1,"id":1,"op":"status"}"#,
            r#"{"v":2,"id":1,"op":"status","body":{}}"#,
            r#"{"v":1,"id":-1,"op":"status","body":{}}"#,
            r#"{"v":1,"id":1,"op":"listSecrets","body":{"query":1}}"#,
            r#"{"v":1,"id":1,"op":"requestAccess","body":{"ref":"x","use":"run"}}"#,
            r#"{"v":1,"id":1,"op":"requestAccess","body":{"ref":"x","use":"request","y":1}}"#,
            r#"{"v":1,"id":1,"op":"httpRequest","body":{"ref":"x","method":"GET"}}"#,
            r#"{"v":1,"id":1,"op":"httpRequest","body":{"ref":"x","method":"GET","url":"u","maxBytes":-1}}"#,
            "[]",
            "",
        ] {
            assert_eq!(
                parse_request(bad.as_bytes()).unwrap_err(),
                AgentErrorCode::InvalidRequest,
                "{bad}"
            );
        }
    }

    #[test]
    fn request_id_is_read_from_a_refused_request() {
        assert_eq!(
            request_id(br#"{"v":1,"id":7,"op":"get","body":{}}"#),
            Some(7)
        );
        assert_eq!(request_id(b"{}"), None);
        assert_eq!(request_id(b"nope"), None);
    }

    #[test]
    fn responses_and_events_round_trip() {
        let responses = [
            ResponseEnvelope::ok(
                1,
                OpResponse::Status(StatusBody {
                    vault: VaultState::Unlocked,
                    agent_access: AccessState::On,
                }),
            ),
            ResponseEnvelope::ok(
                2,
                OpResponse::Secrets(SecretsBody {
                    secrets: vec![WireSecret {
                        reference: "mosaic://v/i/password".to_owned(),
                        title: "GitHub".to_owned(),
                        vault: "Personal".to_owned(),
                        field: "api_key.secret".to_owned(),
                        origins: vec!["https://api.github.com".to_owned()],
                        inject: InjectKind::Bearer,
                        env: None,
                        run_allowed: false,
                        grant: GrantState::Until(5),
                        needs_review: false,
                    }],
                }),
            ),
            ResponseEnvelope::ok(
                3,
                OpResponse::Access(AccessBody {
                    decision: AccessDecision::Granted,
                    expires_at: Some(9),
                    grant: GrantState::UntilLock,
                }),
            ),
            ResponseEnvelope::ok(
                4,
                OpResponse::Http(HttpBody {
                    status: 200,
                    headers: vec![("content-type".to_owned(), "text/plain".to_owned())],
                    body: Some("hi".to_owned()),
                    binary: None,
                    truncated: false,
                    masked: 0,
                    redirect: None,
                }),
            ),
            ResponseEnvelope::ok(
                5,
                OpResponse::Http(HttpBody {
                    status: 200,
                    headers: Vec::new(),
                    body: None,
                    binary: Some(WireBinary {
                        content_type: Some("image/png".to_owned()),
                        bytes: 12,
                        content_encoding: None,
                    }),
                    truncated: true,
                    masked: 2,
                    redirect: Some("https://x".to_owned()),
                }),
            ),
            ResponseEnvelope::failed(
                6,
                WireError {
                    code: AgentErrorCode::AmbiguousRef,
                    candidates: Some(vec![WireCandidate {
                        vault: "V".to_owned(),
                        title: "T".to_owned(),
                        id: "I".to_owned(),
                    }]),
                },
            ),
            ResponseEnvelope::failed(7, WireError::code(AgentErrorCode::Locked)),
        ];
        for r in responses {
            let json = r.to_json().unwrap();
            assert_eq!(ResponseEnvelope::parse(&json).unwrap(), r);
            assert_eq!(
                DesktopFrame::parse(&json).unwrap(),
                DesktopFrame::Response(r)
            );
        }
    }

    #[test]
    fn events_round_trip() {
        let events = [
            Event::State {
                vault: VaultState::Locked,
                agent_access: AccessState::Off,
            },
            Event::Unpaired,
            Event::GrantRevoked {
                reference: "mosaic://v/i/password".to_owned(),
            },
        ];
        for e in events {
            let json = e.to_json().unwrap();
            assert_eq!(Event::parse(&json).unwrap(), e);
            assert_eq!(DesktopFrame::parse(&json).unwrap(), DesktopFrame::Event(e));
        }
        assert_eq!(
            String::from_utf8(Event::Unpaired.to_json().unwrap()).unwrap(),
            r#"{"v":1,"event":"unpaired"}"#
        );
    }

    #[test]
    fn inconsistent_responses_are_refused() {
        for bad in [
            r#"{"v":1,"id":1,"ok":true}"#,
            r#"{"v":1,"id":1,"ok":false}"#,
            r#"{"v":1,"id":1,"ok":true,"error":{"code":"locked"}}"#,
            r#"{"v":2,"id":1,"ok":false,"error":{"code":"locked"}}"#,
            r#"{"v":1,"id":1,"ok":false,"error":{"code":"nope"}}"#,
        ] {
            assert!(ResponseEnvelope::parse(bad.as_bytes()).is_err(), "{bad}");
        }
        assert!(Event::parse(br#"{"v":1,"event":"state"}"#).is_err());
        assert!(Event::parse(br#"{"v":1,"event":"unpaired","x":1}"#).is_err());
    }

    #[test]
    fn ambiguous_failure_keeps_candidates_and_others_drop_them() {
        let id = mosaic_core::ItemId::new_random().unwrap();
        let candidate = mosaic_vault::refs::RefCandidate {
            vault: "V".to_owned(),
            title: "T".to_owned(),
            id,
        };
        let e = WireError::from(AgentFailure {
            code: AgentErrorCode::AmbiguousRef,
            candidates: vec![candidate.clone()],
        });
        assert_eq!(e.candidates.map(|c| c.len()), Some(1));
        let e = WireError::from(AgentFailure {
            code: AgentErrorCode::NoGrant,
            candidates: vec![candidate],
        });
        assert!(e.candidates.is_none());
    }
}
