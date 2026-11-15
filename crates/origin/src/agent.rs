//! Destinations for agent requests (agent access design §6.3, ADR-0159).
//!
//! Stricter than [`crate::fill_allowed`]: exact origin only, `https` only,
//! no IP literal, no `localhost`, no userinfo and no trailing dot. DNS
//! answers are not checked; TLS with the platform verifier is the binding.
//!
//! - [`AgentOrigin`]: a saved destination in its canonical serialization.
//! - [`parse_agent_url`]: the request URL, parsed once with the WHATWG
//!   parser. The caller sends **this** [`Url`], never a second parse of the
//!   string.
//! - [`agent_destination_allowed`]: the parsed URL's origin must equal the
//!   saved one byte for byte.

use thiserror::Error;
use url::{Host, Url};

/// The longest request URL [`parse_agent_url`] accepts, in bytes.
pub const MAX_AGENT_URL_LEN: usize = 8192;

/// Why a string is not a valid saved agent destination.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum AgentOriginError {
    /// Longer than [`MAX_AGENT_URL_LEN`].
    #[error("destination too long")]
    TooLong,
    /// Not parseable as a URL with a host.
    #[error("not a valid destination")]
    Malformed,
    /// The scheme is not `https`.
    #[error("destinations must use https")]
    NotHttps,
    /// The host is an IPv4 or IPv6 address.
    #[error("an IP address is not allowed as a destination")]
    IpLiteral,
    /// `localhost` or a `*.localhost` name.
    #[error("localhost is not allowed as a destination")]
    Localhost,
    /// The URL carries a user name or password.
    #[error("a destination must not contain a user name or password")]
    Userinfo,
    /// A path other than `/`, a query or a fragment.
    #[error("a destination is an origin without a path")]
    HasPath,
    /// The host ends with a dot.
    #[error("a destination host must not end with a dot")]
    TrailingDot,
    /// A stored value that is not its own canonical serialization.
    #[error("not a canonical destination")]
    NotCanonical,
}

/// Why a request URL is refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[non_exhaustive]
pub enum AgentRefusal {
    /// Longer than [`MAX_AGENT_URL_LEN`].
    #[error("url too long")]
    TooLong,
    /// Not an absolute URL with a host.
    #[error("not a valid url")]
    Malformed,
    /// The scheme is not `https`.
    #[error("url must use https")]
    NotHttps,
    /// The URL carries a user name or password.
    #[error("url must not contain a user name or password")]
    Userinfo,
    /// The origin is not the saved one.
    #[error("destination not allowed")]
    NotAllowed,
}

/// A saved destination: `https://host[:port]` in canonical serialization
/// (lowercase ASCII host, IDN as punycode, no default port, no path).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AgentOrigin {
    serialized: String,
    host: String,
}

impl AgentOrigin {
    /// A destination as typed in the editor: trimmed; no `://` means
    /// `https://`.
    ///
    /// # Errors
    /// An [`AgentOriginError`] naming the first rule the input breaks.
    pub fn from_input(s: &str) -> Result<Self, AgentOriginError> {
        let s = s.trim();
        if s.len() > MAX_AGENT_URL_LEN {
            return Err(AgentOriginError::TooLong);
        }
        if s.contains("://") {
            Self::parse(s)
        } else {
            let mut with_scheme = String::with_capacity(s.len().saturating_add(8));
            with_scheme.push_str("https://");
            with_scheme.push_str(s);
            Self::parse(&with_scheme)
        }
    }

    /// A stored destination: it must equal its own serialization.
    ///
    /// # Errors
    /// [`AgentOriginError::NotCanonical`] if it parses but is not canonical,
    /// otherwise the first rule it breaks.
    pub fn parse_canonical(s: &str) -> Result<Self, AgentOriginError> {
        if s.len() > MAX_AGENT_URL_LEN {
            return Err(AgentOriginError::TooLong);
        }
        let origin = Self::parse(s)?;
        if origin.serialized == s {
            Ok(origin)
        } else {
            Err(AgentOriginError::NotCanonical)
        }
    }

    fn parse(s: &str) -> Result<Self, AgentOriginError> {
        let url = Url::parse(s).map_err(|_| AgentOriginError::Malformed)?;
        if url.scheme() != "https" {
            return Err(AgentOriginError::NotHttps);
        }
        if has_userinfo(&url) {
            return Err(AgentOriginError::Userinfo);
        }
        let host = match url.host() {
            Some(Host::Domain(d)) => d,
            Some(Host::Ipv4(_) | Host::Ipv6(_)) => return Err(AgentOriginError::IpLiteral),
            None => return Err(AgentOriginError::Malformed),
        };
        if host.ends_with('.') {
            return Err(AgentOriginError::TrailingDot);
        }
        if host == "localhost" || host.ends_with(".localhost") {
            return Err(AgentOriginError::Localhost);
        }
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err(AgentOriginError::HasPath);
        }
        Ok(Self {
            serialized: url.origin().ascii_serialization(),
            host: host.to_owned(),
        })
    }

    /// The canonical `https://host[:port]`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.serialized
    }

    /// The host, lowercase ASCII (punycode for an IDN).
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }
}

fn has_userinfo(url: &Url) -> bool {
    !url.username().is_empty() || url.password().is_some()
}

/// Parses a request URL once with the WHATWG parser.
///
/// The caller must send the returned [`Url`] (and check it with
/// [`agent_destination_allowed`]), never re-parse the string: two parsers
/// can disagree about the host.
///
/// # Errors
/// [`AgentRefusal::TooLong`] above [`MAX_AGENT_URL_LEN`],
/// [`AgentRefusal::Malformed`] for a relative URL or one without a host,
/// [`AgentRefusal::NotHttps`] for any other scheme,
/// [`AgentRefusal::Userinfo`] for a user name or password.
pub fn parse_agent_url(s: &str) -> Result<Url, AgentRefusal> {
    if s.len() > MAX_AGENT_URL_LEN {
        return Err(AgentRefusal::TooLong);
    }
    let url = Url::parse(s).map_err(|_| AgentRefusal::Malformed)?;
    if url.scheme() != "https" {
        return Err(AgentRefusal::NotHttps);
    }
    if has_userinfo(&url) {
        return Err(AgentRefusal::Userinfo);
    }
    if url.host().is_none() {
        return Err(AgentRefusal::Malformed);
    }
    Ok(url)
}

/// Whether `url` goes to exactly the saved origin: `https`, no userinfo,
/// and the origin's ASCII serialization equals the saved one.
///
/// # Errors
/// [`AgentRefusal::NotHttps`], [`AgentRefusal::Userinfo`] or
/// [`AgentRefusal::NotAllowed`].
pub fn agent_destination_allowed(saved: &AgentOrigin, url: &Url) -> Result<(), AgentRefusal> {
    if url.scheme() != "https" {
        return Err(AgentRefusal::NotHttps);
    }
    if has_userinfo(url) {
        return Err(AgentRefusal::Userinfo);
    }
    if url.origin().ascii_serialization() == saved.as_str() {
        Ok(())
    } else {
        Err(AgentRefusal::NotAllowed)
    }
}
