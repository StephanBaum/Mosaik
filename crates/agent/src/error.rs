//! The error codes of agent access (ADR-0159). They cross the agent channel
//! and the MCP relay as camelCase strings; each has a fixed sentence in the
//! relay and never carries a value or an echo of the input.

use serde::{Deserialize, Serialize};

/// Why an agent request was refused or failed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentErrorCode {
    /// The desktop app is not running (reported by the relay).
    DesktopNotRunning,
    /// The vault is locked.
    Locked,
    /// Agent access is switched off in the desktop.
    AgentAccessOff,
    /// This client is not paired.
    NotPaired,
    /// The reference names nothing an agent may use.
    NotAgentUsable,
    /// The binding is not confirmed on this device.
    NeedsReview,
    /// The reference matches more than one item.
    AmbiguousRef,
    /// No live grant for this client and secret.
    NoGrant,
    /// The user denied the request.
    Denied,
    /// The URL's origin is not one of the binding's origins.
    DestinationNotAllowed,
    /// The method is not allowed.
    MethodNotAllowed,
    /// A header is not allowed.
    HeaderNotAllowed,
    /// The request is over a size limit.
    RequestTooLarge,
    /// The response is over a size limit.
    ResponseTooLarge,
    /// The request timed out.
    Timeout,
    /// The TLS handshake or certificate check failed.
    TlsFailed,
    /// Too many requests.
    RateLimited,
    /// Another request or dialog is in progress.
    Busy,
    /// The reference is not well formed.
    InvalidRef,
    /// The request is not well formed.
    InvalidRequest,
    /// The request could not be sent or the answer could not be read.
    RequestFailed,
    /// An internal error.
    Internal,
}

impl AgentErrorCode {
    /// Every code, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::DesktopNotRunning,
        Self::Locked,
        Self::AgentAccessOff,
        Self::NotPaired,
        Self::NotAgentUsable,
        Self::NeedsReview,
        Self::AmbiguousRef,
        Self::NoGrant,
        Self::Denied,
        Self::DestinationNotAllowed,
        Self::MethodNotAllowed,
        Self::HeaderNotAllowed,
        Self::RequestTooLarge,
        Self::ResponseTooLarge,
        Self::Timeout,
        Self::TlsFailed,
        Self::RateLimited,
        Self::Busy,
        Self::InvalidRef,
        Self::InvalidRequest,
        Self::RequestFailed,
        Self::Internal,
    ];

    /// The camelCase wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DesktopNotRunning => "desktopNotRunning",
            Self::Locked => "locked",
            Self::AgentAccessOff => "agentAccessOff",
            Self::NotPaired => "notPaired",
            Self::NotAgentUsable => "notAgentUsable",
            Self::NeedsReview => "needsReview",
            Self::AmbiguousRef => "ambiguousRef",
            Self::NoGrant => "noGrant",
            Self::Denied => "denied",
            Self::DestinationNotAllowed => "destinationNotAllowed",
            Self::MethodNotAllowed => "methodNotAllowed",
            Self::HeaderNotAllowed => "headerNotAllowed",
            Self::RequestTooLarge => "requestTooLarge",
            Self::ResponseTooLarge => "responseTooLarge",
            Self::Timeout => "timeout",
            Self::TlsFailed => "tlsFailed",
            Self::RateLimited => "rateLimited",
            Self::Busy => "busy",
            Self::InvalidRef => "invalidRef",
            Self::InvalidRequest => "invalidRequest",
            Self::RequestFailed => "requestFailed",
            Self::Internal => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_matches_the_serde_name_and_all_is_complete() {
        for code in AgentErrorCode::ALL {
            let json = serde_json::to_string(code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
            let back: AgentErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *code);
        }
        let mut names: Vec<&str> = AgentErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 22, "22 distinct codes (ADR-0159)");
    }
}
