//! `mosaic agent pair | status | mcp`: the client side of the agent endpoint of the running
//! desktop (ADR-0159, `docs/NATIVE_MESSAGING.md` section 9).
//!
//! These commands never open the store, never touch a CLI session and never sweep one: they run
//! before `App::new` (`lib.rs`). The pairing key lives in the OS keystore only.
//!
//! - [`conn`]: connect, handshake and a multiplexed request channel.
//! - `pair`, `status`: the terminal commands.

pub mod conn;
mod pair;
mod status;

use std::future::Future;

use mosaic_native_messaging::agent_preauth::{AgentRefusal, pairing_name_is_valid};

use crate::backends;
use crate::error::{CliError, EXIT_OTHER, EXIT_VAULT_LOCKED, Result};
use crate::i18n::Msg;
use crate::output::Output;
use crate::spec::Parsed;

use conn::ConnError;

/// Runs `fut` on a current-thread runtime with a `LocalSet` (the connection's reader task needs
/// one).
fn block_on<T>(fut: impl Future<Output = T>) -> Result<T> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| CliError::internal())?;
    Ok(tokio::task::LocalSet::new().block_on(&rt, fut))
}

/// The wire name of a refusal (`pairingDenied`, ...).
fn refusal_name(reason: AgentRefusal) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The CLI error of a connection failure.
fn conn_error(e: ConnError, name: &str) -> CliError {
    match e {
        ConnError::DesktopNotRunning => {
            CliError::new("desktopNotRunning", EXIT_OTHER, Msg::ErrDesktopNotRunning)
        }
        ConnError::NotPaired => {
            CliError::new("notPaired", EXIT_OTHER, Msg::ErrNotPaired).with("name", name)
        }
        ConnError::Locked => CliError::new("locked", EXIT_VAULT_LOCKED, Msg::ErrAgentLocked),
        ConnError::AgentAccessOff => {
            CliError::new("agentAccessOff", EXIT_OTHER, Msg::ErrAgentAccessOff)
        }
        ConnError::Refused(reason) => match reason {
            AgentRefusal::PairingDenied => {
                CliError::new("pairingDenied", EXIT_OTHER, Msg::ErrPairingDenied)
            }
            AgentRefusal::PairingExpired => {
                CliError::new("pairingExpired", EXIT_OTHER, Msg::ErrPairingExpired)
            }
            AgentRefusal::PairingLimit => {
                CliError::new("pairingLimit", EXIT_OTHER, Msg::ErrPairingLimit)
            }
            AgentRefusal::PairingBlocked => {
                CliError::new("pairingBlocked", EXIT_OTHER, Msg::ErrPairingBlocked)
            }
            other => CliError::new("agentRefused", EXIT_OTHER, Msg::ErrAgentRefused)
                .with("reason", refusal_name(other)),
        },
        ConnError::Io => CliError::new("agentIo", EXIT_OTHER, Msg::ErrAgentIo),
    }
}

/// A required, valid client name from `--name` or `--client`.
fn client_name<'a>(p: &'a Parsed, flag: &'static str) -> Result<&'a str> {
    let name = p.value(flag).ok_or_else(|| {
        CliError::usage(Msg::ErrMissingArgument).with("name", format!("--{flag}"))
    })?;
    if pairing_name_is_valid(name) {
        Ok(name)
    } else {
        Err(CliError::usage(Msg::ErrInvalidValue).with("name", format!("--{flag}")))
    }
}

/// Dispatches `mosaic agent <sub>`. Returns the exit code.
///
/// # Errors
/// The command's error.
pub fn run(p: &Parsed, out: &Output) -> Result<u8> {
    match p.args.first().map(String::as_str) {
        Some("pair") => pair::run(p, out).map(|()| 0),
        Some("status") => status::run(p, out).map(|()| 0),
        Some("mcp") => Err(CliError::not_supported("agent mcp")),
        _ => Err(CliError::internal()),
    }
}
