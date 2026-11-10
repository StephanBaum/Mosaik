//! Browser extension commands (`docs/DESKTOP_IPC.md` §3): the paired
//! browsers, and the answers to the pairing and save prompts. The prompts
//! arrive as events; the endpoint itself is `crate::browser`.

use tauri::State;

use crate::browser::parse_id;
use crate::contract::browser::{
    BrowserConnectorAudit, BrowserPairingRespondRequest, BrowserPasskeyRespondRequest,
    BrowserRemovePairingRequest, BrowserRotationRespondRequest, BrowserRotationRevealRequest,
    BrowserRotationRevealed, BrowserSaveRespondRequest, BrowserStatus,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::{AppState, Backend};

/// `browser_status`.
#[tauri::command]
pub async fn browser_status(state: State<'_, AppState>) -> IpcResult<BrowserStatus> {
    Ok(state.backend().browser_status())
}

/// `browser_connector_audit`: the native-messaging entries that do not point
/// to the installed host. T8 wires `audit_manifests`; until then there are no
/// findings.
#[tauri::command]
pub async fn browser_connector_audit(
    state: State<'_, AppState>,
) -> IpcResult<BrowserConnectorAudit> {
    Ok(state.backend().browser_connector_audit())
}

/// `browser_connector_repair`: points the per-user entries to the installed
/// host and audits again. T8 wires the repair; until then it changes nothing
/// and answers with no findings.
#[tauri::command]
pub async fn browser_connector_repair(
    state: State<'_, AppState>,
) -> IpcResult<BrowserConnectorAudit> {
    state.run(Backend::browser_connector_repair).await
}

/// `browser_remove_pairing`.
#[tauri::command]
pub async fn browser_remove_pairing(
    state: State<'_, AppState>,
    request: BrowserRemovePairingRequest,
) -> IpcResult<BrowserStatus> {
    state.run(move |b| b.browser_remove_pairing(&request)).await
}

/// `browser_pairing_respond`.
#[tauri::command]
pub async fn browser_pairing_respond(
    state: State<'_, AppState>,
    request: BrowserPairingRespondRequest,
) -> IpcResult<()> {
    state
        .run(move |b| b.browser_pairing_respond(&request))
        .await
}

/// `browser_save_respond`.
#[tauri::command]
pub async fn browser_save_respond(
    state: State<'_, AppState>,
    request: BrowserSaveRespondRequest,
) -> IpcResult<()> {
    state.run(move |b| b.browser_save_respond(&request)).await
}

/// `browser_passkey_respond`.
#[tauri::command]
pub async fn browser_passkey_respond(
    state: State<'_, AppState>,
    request: BrowserPasskeyRespondRequest,
) -> IpcResult<()> {
    state
        .run(move |b| b.browser_passkey_respond(&request))
        .await
}

/// `browser_rotation_respond`.
#[tauri::command]
pub async fn browser_rotation_respond(
    state: State<'_, AppState>,
    request: BrowserRotationRespondRequest,
) -> IpcResult<()> {
    state
        .run(move |b| b.browser_rotation_respond(&request))
        .await
}

/// `browser_rotation_reveal`.
#[tauri::command]
pub async fn browser_rotation_reveal(
    state: State<'_, AppState>,
    request: BrowserRotationRevealRequest,
) -> IpcResult<BrowserRotationRevealed> {
    state
        .run(move |b| b.browser_rotation_reveal(&request))
        .await
}

impl Backend {
    /// The connector audit. M9 T8 wires `audit_manifests` here; the answer
    /// is empty until then.
    #[must_use]
    pub fn browser_connector_audit(&self) -> BrowserConnectorAudit {
        BrowserConnectorAudit { findings: vec![] }
    }

    /// The connector repair. M9 T8 wires the rewrite of the per-user
    /// entries here; nothing changes until then.
    ///
    /// # Errors
    /// None yet; T8 adds the codes of the rewrite.
    pub fn browser_connector_repair(&self) -> IpcResult<BrowserConnectorAudit> {
        self.touch();
        Ok(self.browser_connector_audit())
    }

    /// The endpoint and the paired browsers.
    #[must_use]
    pub fn browser_status(&self) -> BrowserStatus {
        self.browser.status()
    }

    /// Removes a pairing; its live sessions are told and closed.
    ///
    /// # Errors
    /// `browserPairingNotFound`.
    pub fn browser_remove_pairing(
        &self,
        request: &BrowserRemovePairingRequest,
    ) -> IpcResult<BrowserStatus> {
        self.touch();
        let not_found = || IpcError::new(ErrorCode::BrowserPairingNotFound);
        let id = parse_id(&request.pairing_id).ok_or_else(not_found)?;
        let removed = self
            .browser
            .shared()
            .remove_pairing(&id, None)
            .map_err(|_| IpcError::INTERNAL)?;
        if removed {
            Ok(self.browser.status())
        } else {
            Err(not_found())
        }
    }

    /// Answers the pairing prompt.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon` (the request stays
    /// open), `invalidInput` (a bad label; the request stays open).
    pub fn browser_pairing_respond(&self, request: &BrowserPairingRespondRequest) -> IpcResult<()> {
        self.touch();
        self.browser.shared().respond_pair(request)
    }

    /// Answers the save prompt; a save writes the vault.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon`, `invalidInput`,
    /// `vaultNotFound`, `vaultUnavailable`, `itemNotFound` (the request
    /// stays open after all of them except the first), `locked`.
    pub fn browser_save_respond(&self, request: &BrowserSaveRespondRequest) -> IpcResult<()> {
        self.touch();
        self.browser.shared().respond_save(self, request)
    }
}

impl Backend {
    /// Answers the passkey prompt; a create or a sign-in writes the vault
    /// (a new `passkey` item, or a raised sign count) before the answer
    /// reaches the site.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon`, `reauthRequired` (the
    /// dialog asked for the password), `wrongCredentials`, `vaultNotFound`,
    /// `vaultUnavailable`, `itemNotFound`, `invalidInput`, `locked`: the
    /// request stays open after all of them except the first.
    pub fn browser_passkey_respond(&self, request: &BrowserPasskeyRespondRequest) -> IpcResult<()> {
        self.touch();
        self.browser.shared().respond_passkey(self, request)
    }
}

impl Backend {
    /// Answers the rotation prompt: an `allow` stores the generated password
    /// as a new attempt of the login, a `confirm` or `keepOld` closes one, an
    /// `addOrigin` adds the address (all write the vault before the answer
    /// reaches the browser). `regenerate` keeps the prompt open with a new
    /// password.
    ///
    /// # Errors
    /// `browserRequestExpired`, `browserAllowTooSoon` (the request stays
    /// open), `invalidInput` (a decision that does not fit the request),
    /// `itemNotFound`, `vaultUnavailable`, `rotationNotFound`,
    /// `rotationLimit`, `originNotRelated`, `sharingRoleForbidden`, `locked`:
    /// the request stays open after all of them except the first.
    pub fn browser_rotation_respond(
        &self,
        request: &BrowserRotationRespondRequest,
    ) -> IpcResult<()> {
        self.touch();
        self.browser.shared().respond_rotation(self, request)
    }

    /// The generated password of an open `begin` request, to show it in the
    /// dialog.
    ///
    /// # Errors
    /// `browserRequestExpired` (no such open request), `invalidInput` (not a
    /// `begin`).
    pub fn browser_rotation_reveal(
        &self,
        request: &BrowserRotationRevealRequest,
    ) -> IpcResult<BrowserRotationRevealed> {
        self.touch();
        self.browser.shared().reveal_rotation(request)
    }
}
