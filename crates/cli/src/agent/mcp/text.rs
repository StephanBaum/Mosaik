//! The model-facing text of the MCP relay: tool descriptions and the fixed sentence of every error
//! code. It is English protocol text in constants, not part of the i18n catalog (ADR-0159, Q13).
//! No sentence echoes input, and none says anything about a value.

use mosaic_agent::error::AgentErrorCode;

/// The MCP protocol revisions with the `initialize` handshake this relay speaks, newest first.
pub const INITIALIZE_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// The stateless MCP protocol revisions this relay speaks, newest first: no handshake, the
/// version and the client's capabilities in every request's `_meta`, `server/discover`.
pub const STATELESS_VERSIONS: [&str; 1] = ["2026-07-28"];

/// The first revision with `structuredContent` in tool results.
pub const STRUCTURED_FROM: &str = "2025-06-18";

/// The first revision with tool `annotations`.
pub const ANNOTATIONS_FROM: &str = "2025-03-26";

pub const STATUS_DESCRIPTION: &str = "Report whether the Mosaic desktop app is running, whether the vault is locked and whether agent access is on. No tool returns a secret value.";

pub const LIST_DESCRIPTION: &str = "List the secrets the user has made available to agents: a mosaic:// reference, a title, the destinations each may be sent to and whether you hold a grant. Use the reference with the other tools. No tool returns a secret value.";

pub const ACCESS_DESCRIPTION: &str = "Ask the user to allow you to use one secret for a while. The user decides in the Mosaic app; the answer says whether access was granted and until when. No tool returns a secret value.";

pub const HTTP_DESCRIPTION: &str = "Send one HTTPS request with a secret from the user's Mosaic vault added by Mosaic. Name the secret by its mosaic:// reference from mosaic_list_secrets. Mosaic adds the value only for the destinations the user set for it and returns the response with the value masked. The value is never returned to you. Do not try to print, echo or transform secrets. Call mosaic_request_access first if you have no grant.";

/// The fixed sentence of an error code.
#[must_use]
pub const fn sentence(code: AgentErrorCode) -> &'static str {
    match code {
        AgentErrorCode::DesktopNotRunning => {
            "The Mosaic desktop app is not running. Ask the user to start it."
        }
        AgentErrorCode::Locked => "The vault is locked. Ask the user to unlock Mosaic.",
        AgentErrorCode::AgentAccessOff => {
            "Agent access is off in Mosaic. Ask the user to turn it on in Settings."
        }
        AgentErrorCode::NotPaired => "Pair this client: run `mosaic agent pair` in a terminal.",
        AgentErrorCode::NotAgentUsable => {
            "That reference is not a secret an agent may use. Call mosaic_list_secrets for the usable ones."
        }
        AgentErrorCode::NeedsReview => {
            "The user has not confirmed this secret's settings on this device yet."
        }
        AgentErrorCode::AmbiguousRef => {
            "More than one secret matches. Use the reference of one of the candidates."
        }
        AgentErrorCode::NoGrant => {
            "No access is granted for this secret. Call mosaic_request_access first."
        }
        AgentErrorCode::Denied => "The user denied access.",
        AgentErrorCode::DestinationNotAllowed => {
            "The URL's origin is not one of the destinations the user set for this secret."
        }
        AgentErrorCode::MethodNotAllowed => "That method is not allowed for this secret.",
        AgentErrorCode::HeaderNotAllowed => "One of the headers is not allowed.",
        AgentErrorCode::RequestTooLarge => "The request is over a size limit.",
        AgentErrorCode::ResponseTooLarge => "The response is over a size limit.",
        AgentErrorCode::Timeout => "The request timed out.",
        AgentErrorCode::TlsFailed => "The TLS connection could not be verified.",
        AgentErrorCode::RateLimited => "Too many requests. Wait and try again.",
        AgentErrorCode::Busy => "Mosaic is busy with another request. Try again shortly.",
        AgentErrorCode::InvalidRef => "The reference is not well formed.",
        AgentErrorCode::InvalidRequest => "The request is not valid.",
        AgentErrorCode::RequestFailed => "The request could not be sent.",
        AgentErrorCode::Internal => "An internal error occurred.",
    }
}
