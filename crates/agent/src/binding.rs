//! Agent bindings: the validated form of the `agent:<id>` policy of an item
//! (agent access design §6.1, §6.2, ADR-0159).
//!
//! `mosaic-core` stores a binding as a shape-checked record
//! ([`AgentBindingRecord`]). [`Binding::from_record`] adds the semantic rules:
//! the field, canonical `https` origins, header and query names, the
//! environment name deny list, methods and paths. A record that breaks a rule
//! is not usable; nothing here reads a secret value, and no error carries an
//! input.

use mosaic_core::model::{
    AgentBasicUser, AgentBindingRecord, AgentInject, ElementId, SecretField, Text, TextField,
};
use mosaic_origin::{AgentOrigin, AgentOriginError};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The shortest value that may be marked agent-usable, in bytes (ADR-0159 Q9).
pub const MIN_VALUE_BYTES: usize = 12;
/// The most origins one binding may list.
pub const MAX_ORIGINS: usize = 16;

const MAX_HEADER_NAME: usize = 64;
const MAX_HEADER_PREFIX: usize = 32;
const MAX_QUERY_NAME: usize = 64;
const MAX_BASIC_LITERAL_CHARS: usize = 256;
const MAX_ENV_NAME: usize = 128;
const MAX_PATHS: usize = 16;
const MAX_PATH_BYTES: usize = 256;

/// Why a binding or a value cannot be used. The text never holds input.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum BindingError {
    /// The field is not a secret field or a custom field of the item.
    #[error("the field cannot be used by agents")]
    Field,
    /// Not 1 to 16 distinct origins.
    #[error("a binding needs between 1 and 16 distinct origins")]
    Origins,
    /// An origin is not a canonical https destination.
    #[error("invalid origin: {0}")]
    Origin(AgentOriginError),
    /// The header name is not a token or is not allowed.
    #[error("invalid header name")]
    HeaderName,
    /// The header prefix is too long or not printable ASCII.
    #[error("invalid header prefix")]
    HeaderPrefix,
    /// The query parameter name is invalid.
    #[error("invalid query parameter name")]
    QueryName,
    /// The user name source of a Basic binding is invalid.
    #[error("invalid user name for basic authentication")]
    BasicUser,
    /// The environment variable name is invalid or not allowed.
    #[error("invalid environment variable name")]
    Env,
    /// The method list is empty, has a duplicate or an unknown method.
    #[error("invalid method list")]
    Methods,
    /// The path list is invalid.
    #[error("invalid path list")]
    Paths,
    /// The value is shorter than 12 bytes.
    #[error("the value is too short for agent use")]
    ValueTooShort,
    /// The value contains CR, LF or NUL.
    #[error("the value contains a control character")]
    ValueHasControl,
}

impl From<AgentOriginError> for BindingError {
    fn from(e: AgentOriginError) -> Self {
        Self::Origin(e)
    }
}

/// The field a binding places in a request.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BindingField {
    /// A secret field of the item type.
    Secret(SecretField),
    /// A hidden custom field, by element ID.
    Custom(ElementId),
}

/// The user name of a Basic binding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BasicUser {
    /// A fixed user name (not a secret).
    Literal(String),
    /// A text field of the item.
    Field(TextField),
    /// The secret value is the user name and the password is empty.
    SecretIsUser,
}

/// How the value is placed in a request.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Inject {
    /// `Authorization: Bearer <v>`.
    Bearer,
    /// `<name>: <prefix><v>`.
    Header {
        /// The header name (a token, not a forbidden header).
        name: String,
        /// The text before the value.
        prefix: String,
    },
    /// `Authorization: Basic base64(user ":" v)`.
    Basic(BasicUser),
    /// `?<name>=<percent-encoded v>`.
    Query {
        /// The parameter name.
        name: String,
    },
}

/// The kind of an [`Inject`], as shown to agents (no header value).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InjectKind {
    /// `bearer`.
    Bearer,
    /// `header`.
    Header,
    /// `basic`.
    Basic,
    /// `query`.
    Query,
}

impl InjectKind {
    /// The wire name: `bearer`, `header`, `basic` or `query`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bearer => "bearer",
            Self::Header => "header",
            Self::Basic => "basic",
            Self::Query => "query",
        }
    }
}

impl Inject {
    /// The kind of this injection.
    #[must_use]
    pub const fn kind(&self) -> InjectKind {
        match self {
            Self::Bearer => InjectKind::Bearer,
            Self::Header { .. } => InjectKind::Header,
            Self::Basic(_) => InjectKind::Basic,
            Self::Query { .. } => InjectKind::Query,
        }
    }
}

/// An HTTP method an agent may use.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Method {
    /// `GET`.
    Get,
    /// `HEAD`.
    Head,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
}

impl Method {
    /// Parses an exact upper-case method name.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Self::Get),
            "HEAD" => Some(Self::Head),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }

    /// The upper-case name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

/// A validated agent binding. It holds no secret value: a literal Basic user
/// name is not a secret, and a header prefix is policy.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Binding {
    /// The field the value comes from.
    pub field: BindingField,
    /// The origins the value may be sent to (1 to 16, distinct).
    pub origins: Vec<AgentOrigin>,
    /// How the value is placed.
    pub inject: Inject,
    /// The environment variable name for commands and the proxy.
    pub env: Option<String>,
    /// Whether commands may receive the value.
    pub run: bool,
    /// Allowed methods; `None` means any.
    pub methods: Option<Vec<Method>>,
    /// Allowed path prefixes; `None` means any.
    pub paths: Option<Vec<String>>,
    /// SHA-256 of the record's canonical JSON: a grant is bound to it.
    pub digest: [u8; 32],
}

impl Binding {
    /// Validates `rec`.
    ///
    /// # Errors
    /// The first [`BindingError`] the record breaks. A record whose canonical
    /// form cannot be built (an unknown member with a non-integer number) is
    /// [`BindingError::Field`]: it is not usable.
    pub fn from_record(rec: &AgentBindingRecord) -> Result<Self, BindingError> {
        let field = parse_field(rec.field.as_str())?;

        if rec.origins.is_empty() || rec.origins.len() > MAX_ORIGINS {
            return Err(BindingError::Origins);
        }
        let mut origins: Vec<AgentOrigin> = Vec::with_capacity(rec.origins.len());
        for o in &rec.origins {
            let origin = AgentOrigin::parse_canonical(o.as_str())?;
            if origins.contains(&origin) {
                return Err(BindingError::Origins);
            }
            origins.push(origin);
        }

        let inject = parse_inject(&rec.inject)?;

        let env = match &rec.env {
            None => None,
            Some(e) if env_name_allowed(e.as_str()) => Some(e.as_str().to_owned()),
            Some(_) => return Err(BindingError::Env),
        };

        let methods = match &rec.methods {
            None => None,
            Some(list) => Some(parse_methods(list.iter().map(Text::as_str))?),
        };
        let paths = match &rec.paths {
            None => None,
            Some(list) => Some(parse_paths(list.iter().map(Text::as_str))?),
        };

        let canonical = rec.canonical_json().map_err(|_| BindingError::Field)?;
        let digest: [u8; 32] = Sha256::digest(canonical.expose_secret()).into();

        Ok(Self {
            field,
            origins,
            inject,
            env,
            run: rec.run,
            methods,
            paths,
            digest,
        })
    }
}

fn parse_field(s: &str) -> Result<BindingField, BindingError> {
    if let Some(id) = s.strip_prefix("field:") {
        if id.is_empty() {
            return Err(BindingError::Field);
        }
        return ElementId::from_stored(id)
            .map(BindingField::Custom)
            .map_err(|_| BindingError::Field);
    }
    SecretField::from_key(s)
        .map(BindingField::Secret)
        .ok_or(BindingError::Field)
}

fn parse_inject(inject: &AgentInject) -> Result<Inject, BindingError> {
    match inject {
        AgentInject::Bearer => Ok(Inject::Bearer),
        AgentInject::Header { name, prefix } => {
            if !is_token(name.as_str()) || is_forbidden_header(name.as_str()) {
                return Err(BindingError::HeaderName);
            }
            let prefix = prefix.as_str();
            if prefix.len() > MAX_HEADER_PREFIX
                || !prefix.bytes().all(|b| (0x20..=0x7e).contains(&b))
            {
                return Err(BindingError::HeaderPrefix);
            }
            Ok(Inject::Header {
                name: name.as_str().to_owned(),
                prefix: prefix.to_owned(),
            })
        }
        AgentInject::Basic { user } => Ok(Inject::Basic(match user {
            AgentBasicUser::SecretIsUser => BasicUser::SecretIsUser,
            AgentBasicUser::Field(key) => {
                BasicUser::Field(TextField::from_key(key.as_str()).ok_or(BindingError::BasicUser)?)
            }
            AgentBasicUser::Literal(text) => {
                let s = text.as_str();
                if s.is_empty()
                    || s.chars().count() > MAX_BASIC_LITERAL_CHARS
                    || s.contains(':')
                    || s.chars().any(char::is_control)
                {
                    return Err(BindingError::BasicUser);
                }
                BasicUser::Literal(s.to_owned())
            }
        })),
        AgentInject::Query { name } => {
            let n = name.as_str();
            if n.is_empty()
                || n.len() > MAX_QUERY_NAME
                || !n
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
            {
                return Err(BindingError::QueryName);
            }
            Ok(Inject::Query { name: n.to_owned() })
        }
    }
}

fn parse_methods<'a>(items: impl Iterator<Item = &'a str>) -> Result<Vec<Method>, BindingError> {
    let mut out: Vec<Method> = Vec::new();
    for s in items {
        let m = Method::parse(s).ok_or(BindingError::Methods)?;
        if out.contains(&m) {
            return Err(BindingError::Methods);
        }
        out.push(m);
    }
    if out.is_empty() {
        return Err(BindingError::Methods);
    }
    Ok(out)
}

fn parse_paths<'a>(items: impl Iterator<Item = &'a str>) -> Result<Vec<String>, BindingError> {
    let mut out: Vec<String> = Vec::new();
    for s in items {
        if !s.starts_with('/')
            || s.len() > MAX_PATH_BYTES
            || s.chars().any(|c| c == '?' || c == '#' || c.is_control())
        {
            return Err(BindingError::Paths);
        }
        out.push(s.to_owned());
    }
    if out.is_empty() || out.len() > MAX_PATHS {
        return Err(BindingError::Paths);
    }
    Ok(out)
}

/// Whether the value may be marked agent-usable: at least
/// [`MIN_VALUE_BYTES`] bytes and no CR, LF or NUL.
///
/// # Errors
/// [`BindingError::ValueTooShort`] or [`BindingError::ValueHasControl`]; the
/// error never holds the value.
pub fn check_value(value: &str) -> Result<(), BindingError> {
    if value.len() < MIN_VALUE_BYTES {
        return Err(BindingError::ValueTooShort);
    }
    if value.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0)) {
        return Err(BindingError::ValueHasControl);
    }
    Ok(())
}

/// Whether `name` is an RFC 9110 token (`tchar`), 1 to 64 bytes.
#[must_use]
pub fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_HEADER_NAME
        && name.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// Header names an agent binding or an agent request may not set
/// (case-insensitive): `Host`, `Content-Length`, `Transfer-Encoding`,
/// `Connection`, `TE`, `Upgrade`, `Cookie`, `Set-Cookie` and any `Proxy-*`.
#[must_use]
pub fn is_forbidden_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "te"
            | "upgrade"
            | "cookie"
            | "set-cookie"
    ) || lower.starts_with("proxy-")
}

/// Names that change how programs load, matched case-insensitively.
const ENV_DENY: &[&str] = &[
    "PATH",
    "PATHEXT",
    "COMSPEC",
    "SYSTEMROOT",
    "WINDIR",
    "HOME",
    "USERPROFILE",
    "SHELL",
    "IFS",
    "ENV",
    "BASH_ENV",
    "PYTHONPATH",
    "PYTHONSTARTUP",
    "NODE_OPTIONS",
    "PERL5LIB",
    "RUBYOPT",
    "JAVA_TOOL_OPTIONS",
    "GIT_SSH_COMMAND",
];

/// Whether `name` may be the environment variable of a binding:
/// `[A-Za-z_][A-Za-z0-9_]{0,127}`, not `MOSAIC_*`, not on the deny list of
/// names that change how programs load, not `LD_*` or `DYLD_*` (all
/// case-insensitive, on every OS).
#[must_use]
pub fn env_name_allowed(name: &str) -> bool {
    let bytes = name.as_bytes();
    let Some(first) = bytes.first() else {
        return false;
    };
    if bytes.len() > MAX_ENV_NAME
        || !(first.is_ascii_alphabetic() || *first == b'_')
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    !(upper.starts_with("MOSAIC_")
        || upper.starts_with("LD_")
        || upper.starts_with("DYLD_")
        || ENV_DENY.contains(&upper.as_str()))
}
