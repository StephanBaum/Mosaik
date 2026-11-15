//! Agent access commands (ADR-0159): the setting, the paired clients, the answers to the
//! pairing and access dialogs, grants, the kill switch and the activity log. Requests from the
//! agents themselves never come through here: they arrive on the agent endpoint
//! (`agent::DesktopHost`). The webview cannot send, spawn or reveal anything with these commands.

use mosaic_agent::grants::GrantState;
use mosaic_agent::log::{LogEntry, LogKind};
use tauri::State;

use crate::contract::agent::AgentInjectKind;
use crate::contract::agent_access::{
    AgentAccessRespondRequest, AgentAccessStatus, AgentGrantRevokeRequest, AgentGrantState,
    AgentLogKind, AgentLogPage, AgentLogRequest, AgentLogView, AgentPairingRemoveRequest,
    AgentPairingRespondRequest, AgentRefusalCode, AgentSetEnabledRequest,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::{AppState, Backend};

/// The most log entries one `agent_log` call returns.
const MAX_LOG_LIMIT: usize = 200;
/// The most log pages one `agent_log` call reads while it looks for entries that match.
const MAX_LOG_PAGES: usize = 25;

/// `agent_access_status`.
#[tauri::command]
pub async fn agent_access_status(state: State<'_, AppState>) -> IpcResult<AgentAccessStatus> {
    state.run(|b| Ok(b.agent_access_status())).await
}

/// `agent_access_set_enabled`.
#[tauri::command]
pub async fn agent_access_set_enabled(
    state: State<'_, AppState>,
    request: AgentSetEnabledRequest,
) -> IpcResult<AgentAccessStatus> {
    state
        .run(move |b| b.agent_access_set_enabled(request))
        .await
}

/// `agent_pairing_respond`.
#[tauri::command]
pub async fn agent_pairing_respond(
    state: State<'_, AppState>,
    request: AgentPairingRespondRequest,
) -> IpcResult<()> {
    state.run(move |b| b.agent_pairing_respond(&request)).await
}

/// `agent_pairing_remove`.
#[tauri::command]
pub async fn agent_pairing_remove(
    state: State<'_, AppState>,
    request: AgentPairingRemoveRequest,
) -> IpcResult<AgentAccessStatus> {
    state.run(move |b| b.agent_pairing_remove(&request)).await
}

/// `agent_access_respond`.
#[tauri::command]
pub async fn agent_access_respond(
    state: State<'_, AppState>,
    request: AgentAccessRespondRequest,
) -> IpcResult<()> {
    state.run(move |b| b.agent_access_respond(&request)).await
}

/// `agent_grant_revoke`.
#[tauri::command]
pub async fn agent_grant_revoke(
    state: State<'_, AppState>,
    request: AgentGrantRevokeRequest,
) -> IpcResult<AgentAccessStatus> {
    state.run(move |b| b.agent_grant_revoke(request)).await
}

/// `agent_stop_all`.
#[tauri::command]
pub async fn agent_stop_all(state: State<'_, AppState>) -> IpcResult<AgentAccessStatus> {
    state.run(Backend::agent_stop_all).await
}

/// `agent_log`.
#[tauri::command]
pub async fn agent_log(
    state: State<'_, AppState>,
    request: AgentLogRequest,
) -> IpcResult<AgentLogPage> {
    state.run(move |b| b.agent_log(&request)).await
}

fn serde_name<T: serde::Serialize, U: serde::de::DeserializeOwned>(value: &T) -> Option<U> {
    serde_json::from_value(serde_json::to_value(value).ok()?).ok()
}

fn wire_ms(ms: u64) -> i64 {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

/// The activity view of a log entry: facts only.
fn view_of(entry: LogEntry) -> Option<AgentLogView> {
    let (grant, _) = match entry.grant {
        Some(GrantState::Once) => (Some(AgentGrantState::Once), None),
        Some(GrantState::Until(ms)) => (Some(AgentGrantState::Until), Some(ms)),
        Some(GrantState::UntilLock) => (Some(AgentGrantState::UntilLock), None),
        Some(GrantState::None) | None => (None, None),
    };
    Some(AgentLogView {
        at_ms: wire_ms(entry.at_ms),
        kind: serde_name::<_, AgentLogKind>(&entry.kind)?,
        pairing_id: entry.pairing_id,
        pairing_name: entry.pairing_name,
        item_id: entry.item_id,
        item_title: entry.item_title,
        field_label: entry.field_label,
        method: entry.method,
        origin: entry.origin,
        path: entry.path,
        status: entry.status,
        bytes: entry.bytes,
        masked: entry.masked,
        code: entry
            .code
            .and_then(|c| serde_name::<_, AgentRefusalCode>(&c)),
        grant,
        origins: entry.origins,
        inject: entry
            .inject
            .and_then(|i| serde_name::<_, AgentInjectKind>(&i)),
    })
}

impl Backend {
    /// The setting, the clients, the grants and the endpoint.
    #[must_use]
    pub fn agent_access_status(&self) -> AgentAccessStatus {
        self.agent.status(self)
    }

    /// Turns agent access on or off and stores the choice under the settings MAC. Turning it on
    /// needs an unlocked vault; turning it off also works while locked. Off ends every grant,
    /// closes the dialogs and the endpoint.
    ///
    /// # Errors
    /// `locked` (on), or a storage error.
    pub fn agent_access_set_enabled(
        &self,
        request: AgentSetEnabledRequest,
    ) -> IpcResult<AgentAccessStatus> {
        self.touch();
        {
            let mut slot = self.slot()?;
            if request.enabled {
                slot.unlocked()?;
            } else {
                self.note_agent_off();
                if let Ok(session) = slot.unlocked_mut() {
                    let entry = LogEntry::new(LogKind::StoppedAll, crate::agent::wall_ms());
                    let _ = session.append_agent_log(entry.at_ms, &entry.to_json());
                }
            }
            self.update_stored_in(&mut slot, |s| s.agent_access_enabled = request.enabled)?;
        }
        self.agent.set_enabled(request.enabled);
        Ok(self.agent_access_status())
    }

    /// Answers the pairing dialog.
    ///
    /// # Errors
    /// `agentRequestExpired`, `agentAllowTooSoon` (the request stays open).
    pub fn agent_pairing_respond(&self, request: &AgentPairingRespondRequest) -> IpcResult<()> {
        self.touch();
        self.agent.shared().respond_pair(self, request)
    }

    /// Removes a paired client: its connections close, its grants and dialogs end.
    ///
    /// # Errors
    /// `agentPairingNotFound` (also for an ID that is not 32 hex digits), `internal`.
    pub fn agent_pairing_remove(
        &self,
        request: &AgentPairingRemoveRequest,
    ) -> IpcResult<AgentAccessStatus> {
        self.touch();
        let id = crate::browser::parse_id(&request.pairing_id)
            .ok_or(IpcError::new(ErrorCode::AgentPairingNotFound))?;
        let shared = self.agent.shared();
        let name = {
            let mut registry = crate::agent::lock(&shared.registry);
            let name = registry.get(&id).map(|p| p.name.clone());
            match registry.remove(&id) {
                Ok(true) => name,
                Ok(false) => return Err(IpcError::new(ErrorCode::AgentPairingNotFound)),
                Err(_) => return Err(IpcError::INTERNAL),
            }
        };
        shared.end_pairing(&id);
        let mut entry = LogEntry::new(LogKind::PairingRemoved, crate::agent::wall_ms());
        entry.pairing_id = Some(mosaic_agent::log::hex16(&id));
        entry.pairing_name = name;
        crate::agent::Shared::log_entry(self, &entry);
        shared.state_changed();
        Ok(self.agent_access_status())
    }

    /// Answers the access dialog.
    ///
    /// # Errors
    /// `agentRequestExpired`, `agentAllowTooSoon` (the request stays open).
    pub fn agent_access_respond(&self, request: &AgentAccessRespondRequest) -> IpcResult<()> {
        self.touch();
        self.agent.shared().respond_access(request)
    }

    /// Ends one grant. A request that already began loses its answer.
    ///
    /// # Errors
    /// None for an unknown grant (it ended already).
    pub fn agent_grant_revoke(
        &self,
        request: AgentGrantRevokeRequest,
    ) -> IpcResult<AgentAccessStatus> {
        self.touch();
        let shared = self.agent.shared();
        if let Some((pairing, reference, mut entry)) = shared.grant_info(self, request.grant_id)
            && crate::agent::lock(&shared.engine).revoke_grant(request.grant_id)
        {
            entry.kind = LogKind::Revoked;
            crate::agent::Shared::log_entry(self, &entry);
            shared.tell_revoked(&pairing, reference);
        }
        shared.state_changed();
        Ok(self.agent_access_status())
    }

    /// "Stop all agent access now": every grant ends, every connection closes, the pairings stay.
    ///
    /// # Errors
    /// None.
    pub fn agent_stop_all(&self) -> IpcResult<AgentAccessStatus> {
        self.touch();
        let entry = LogEntry::new(LogKind::StoppedAll, crate::agent::wall_ms());
        crate::agent::Shared::log_entry(self, &entry);
        self.agent.restart();
        Ok(self.agent_access_status())
    }

    /// One page of the activity log, newest first, for a client and an item if asked.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (a pairing ID that is not 32 hex digits), `storeUnavailable`.
    pub fn agent_log(&self, request: &AgentLogRequest) -> IpcResult<AgentLogPage> {
        self.touch();
        let pairing = match request.pairing_id.as_deref() {
            Some(id) => {
                crate::browser::parse_id(id).ok_or(IpcError::new(ErrorCode::InvalidInput))?;
                Some(id.to_owned())
            }
            None => None,
        };
        let item = request.item_id.map(|i| i.get().to_string());
        let limit = usize::try_from(request.limit)
            .unwrap_or(MAX_LOG_LIMIT)
            .clamp(1, MAX_LOG_LIMIT);
        let mut before = request.before_ms.and_then(|ms| u64::try_from(ms).ok());
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let mut entries = Vec::new();
        let mut skipped = 0u32;
        'pages: for _ in 0..MAX_LOG_PAGES {
            let page = session.agent_log_page(before, MAX_LOG_LIMIT)?;
            skipped = skipped.saturating_add(u32::try_from(page.unreadable).unwrap_or(u32::MAX));
            let rows = page.entries.len() + page.unreadable;
            for (at_ms, plain) in page.entries {
                before = Some(at_ms);
                let Some(entry) = LogEntry::from_json(plain.expose_secret()) else {
                    skipped = skipped.saturating_add(1);
                    continue;
                };
                if pairing.is_some() && entry.pairing_id != pairing {
                    continue;
                }
                if item.is_some() && entry.item_id != item {
                    continue;
                }
                if let Some(view) = view_of(entry) {
                    entries.push(view);
                }
                if entries.len() >= limit {
                    break 'pages;
                }
            }
            if rows < MAX_LOG_LIMIT {
                break;
            }
        }
        Ok(AgentLogPage { entries, skipped })
    }
}
