//! Vault commands: list, create, rename.

use mosaic_core::VaultId;
use mosaic_vault::session::UnlockedSession;
use tauri::State;

use super::session::MAX_VAULT_NAME_LEN;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::vaults::{CreateVaultRequest, RenameVaultRequest, VaultSummary};
use crate::state::{AppState, Backend};

/// `list_vaults`.
#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> IpcResult<Vec<VaultSummary>> {
    state.run(Backend::list_vaults).await
}

/// `create_vault`.
#[tauri::command]
pub async fn create_vault(
    state: State<'_, AppState>,
    request: CreateVaultRequest,
) -> IpcResult<VaultSummary> {
    state.run(move |b| b.create_vault(&request)).await
}

/// `rename_vault`.
#[tauri::command]
pub async fn rename_vault(
    state: State<'_, AppState>,
    request: RenameVaultRequest,
) -> IpcResult<VaultSummary> {
    state.run(move |b| b.rename_vault(&request)).await
}

fn vault_name(name: &str) -> IpcResult<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_VAULT_NAME_LEN || name.chars().any(char::is_control) {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    Ok(name)
}

fn to_wire(vault: &mosaic_vault::session::VaultSummary<'_>) -> VaultSummary {
    VaultSummary {
        vault_id: vault.vault_id.into(),
        name: vault.name.map(str::to_owned),
        item_count: u32::try_from(vault.item_count).unwrap_or(u32::MAX),
        available: vault.available,
    }
}

fn summary(session: &UnlockedSession, vault_id: VaultId) -> IpcResult<VaultSummary> {
    session
        .vaults()
        .iter()
        .find(|v| v.vault_id == vault_id)
        .map(to_wire)
        .ok_or(IpcError::new(ErrorCode::VaultNotFound))
}

impl Backend {
    /// Every vault.
    ///
    /// # Errors
    /// `locked`.
    pub fn list_vaults(&self) -> IpcResult<Vec<VaultSummary>> {
        let slot = self.slot()?;
        Ok(slot.unlocked()?.vaults().iter().map(to_wire).collect())
    }

    /// Creates a vault.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, or a store error.
    pub fn create_vault(&self, request: &CreateVaultRequest) -> IpcResult<VaultSummary> {
        self.touch();
        let name = vault_name(&request.name)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let vault_id = session.create_vault(name)?;
        summary(session, vault_id)
    }

    /// Renames a vault.
    ///
    /// # Errors
    /// `locked`, `invalidInput`, `vaultNotFound`, `vaultUnavailable`.
    pub fn rename_vault(&self, request: &RenameVaultRequest) -> IpcResult<VaultSummary> {
        self.touch();
        let name = vault_name(&request.name)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.rename_vault(request.vault_id.get(), name)?;
        summary(session, request.vault_id.get())
    }
}
