//! `mosaic agent status [--client NAME]`: whether the desktop runs, the vault and agent-access
//! state, and with `--client` whether that pairing still works.

use mosaic_native_messaging::agent_handshake::{agent_client_handshake, agent_preauth_request};
use mosaic_native_messaging::agent_preauth::{
    AccessState, AgentClientMessage, AgentServerMessage, VaultState,
};
use mosaic_platform::keystore::AgentPairingStore;
use serde::Serialize;

use super::conn::{self, ConnError};
use super::{block_on, client_name, conn_error};
use crate::backends;
use crate::error::Result;
use crate::i18n::Msg;
use crate::output::Output;
use crate::spec::Parsed;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    desktop: &'static str,
    vault: Option<&'static str>,
    agent_access: Option<&'static str>,
    paired: Option<bool>,
}

async fn ask() -> core::result::Result<(VaultState, AccessState), ConnError> {
    let mut stream = conn::connect().await?;
    let answer = agent_preauth_request(&mut stream, &AgentClientMessage::Status { v: 1 })
        .await
        .map_err(ConnError::from)?;
    let AgentServerMessage::Status {
        vault,
        agent_access,
        ..
    } = answer
    else {
        return Err(ConnError::Io);
    };
    Ok((vault, agent_access))
}

pub(super) fn run(p: &Parsed, out: &Output) -> Result<()> {
    let client = if p.has("client") {
        Some(client_name(p, "client")?)
    } else {
        None
    };
    let store = backends::agent_pairings();
    let status = block_on(async {
        match ask().await {
            Ok((vault, access)) => {
                let paired = match client {
                    None => None,
                    Some(name) => Some(is_paired(name, store.as_ref()).await),
                };
                Ok(Status {
                    desktop: "running",
                    vault: Some(match vault {
                        VaultState::Locked => "locked",
                        VaultState::Unlocked => "unlocked",
                    }),
                    agent_access: Some(match access {
                        AccessState::On => "on",
                        AccessState::Off => "off",
                    }),
                    paired,
                })
            }
            // Nothing serves the endpoint: the desktop is not running or agent access is off.
            Err(ConnError::DesktopNotRunning) => Ok(Status {
                desktop: "notRunning",
                vault: None,
                agent_access: None,
                paired: client.map(|_| false),
            }),
            Err(e) => Err(conn_error(e, client.unwrap_or(""))),
        }
    })??;
    out.data(&status, |o| {
        let text = o.t(
            Msg::AgentStatus,
            &[
                ("desktop", status.desktop.to_owned()),
                ("vault", status.vault.unwrap_or("-").to_owned()),
                ("access", status.agent_access.unwrap_or("-").to_owned()),
                (
                    "paired",
                    status
                        .paired
                        .map_or("-", |p| if p { "yes" } else { "no" })
                        .to_owned(),
                ),
            ],
        );
        o.answer(&text);
    });
    Ok(())
}

/// Whether the stored pairing of `name` passes the handshake (a locked vault still does).
async fn is_paired(name: &str, store: &dyn AgentPairingStore) -> bool {
    let Ok(Some((id, key))) = store.load(name) else {
        return false;
    };
    let Ok(mut stream) = conn::connect().await else {
        return false;
    };
    agent_client_handshake(&mut stream, &key, &id).await.is_ok()
}
