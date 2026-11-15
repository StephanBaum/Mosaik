//! `mosaic agent pair --name NAME [--replace]`: asks the desktop for a pairing, shows the code
//! the user compares with the desktop's dialog, and stores the key in the OS keystore.

use std::time::Duration;

use mosaic_native_messaging::agent_handshake::{
    agent_preauth_request, agent_read_server_message_unbounded,
};
use mosaic_native_messaging::agent_preauth::{AgentClientMessage, AgentServerMessage};
use serde::Serialize;
use tokio::time::timeout;

use super::conn::{self, ConnError};
use super::{block_on, client_name, conn_error};
use crate::backends;
use crate::error::{CliError, EXIT_OTHER, Result};
use crate::i18n::Msg;
use crate::output::Output;
use crate::spec::Parsed;

/// The desktop waits 120 s for the user; a little longer for the answer to arrive.
const WAIT: Duration = Duration::from_secs(130);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Done<'a> {
    paired: bool,
    name: &'a str,
    code: &'a str,
}

pub(super) fn run(p: &Parsed, out: &Output) -> Result<()> {
    let name = client_name(p, "name")?;
    let store = backends::agent_pairings();
    let existing = store.load(name).map_err(|_| CliError::internal())?;
    if existing.is_some() && !p.has("replace") {
        return Err(
            CliError::new("agentAlreadyPaired", EXIT_OTHER, Msg::ErrAgentAlreadyPaired)
                .with("name", name),
        );
    }
    drop(existing);
    let (id, key, code) = block_on(async {
        let mut stream = conn::connect().await.map_err(|e| conn_error(e, name))?;
        let request = AgentClientMessage::PairRequest {
            v: 1,
            name: name.to_owned(),
        };
        let pending = agent_preauth_request(&mut stream, &request)
            .await
            .map_err(|e| conn_error(ConnError::from(e), name))?;
        let AgentServerMessage::PairPending { code, .. } = pending else {
            return Err(conn_error(ConnError::Io, name));
        };
        out.answer(&out.t(Msg::AgentPairCode, &[("code", code.clone())]));
        let approved = timeout(WAIT, agent_read_server_message_unbounded(&mut stream))
            .await
            .map_err(|_| conn_error(ConnError::Io, name))?
            .map_err(|e| conn_error(ConnError::from(e), name))?;
        match approved {
            AgentServerMessage::PairApproved {
                pairing_id, key, ..
            } => Ok((pairing_id.0, key.0, code)),
            _ => Err(conn_error(ConnError::Io, name)),
        }
    })??;
    store
        .save(name, &id, &key)
        .map_err(|_| CliError::new("agentIo", EXIT_OTHER, Msg::ErrAgentIo))?;
    drop(key);
    out.data(
        &Done {
            paired: true,
            name,
            code: &code,
        },
        |o| o.say(&o.t(Msg::AgentPaired, &[("name", name.to_owned())])),
    );
    Ok(())
}
