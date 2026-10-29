//! One-time code command: the item's current code for display
//! (`UnlockedSession::otp_code`; HOTP advances and saves the counter
//! first). Copying goes through `copy_to_clipboard` instead.

use mosaic_core::model::OtpKind;
use mosaic_core::{ItemId, SecretString, VaultId};
use mosaic_vault::session::UnlockedSession;
use tauri::State;

use super::clipboard::now_unix_seconds;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::items::ItemRequest;
use crate::contract::secret::SecretOut;
use crate::contract::totp::TotpCode;
use crate::state::{AppState, Backend};

/// `totp_code`.
#[tauri::command]
pub async fn totp_code(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<TotpCode> {
    state.run(move |b| b.totp_code(request)).await
}

/// The current code of an item's own one-time password, its seconds
/// remaining and its period (both `None` for HOTP).
///
/// # Errors
/// `itemNotFound`, `itemDeleted`, `noOtp`, `vaultUnavailable` (a
/// hardware-protected secret, CRYPTO_SPEC §7.4: TK unlock is not built yet,
/// M2-R6).
pub(crate) fn current_code(
    session: &mut UnlockedSession,
    vault_id: VaultId,
    item_id: ItemId,
) -> IpcResult<(SecretString, Option<u32>, Option<u32>)> {
    let item = session.item(vault_id, item_id)?;
    if item.is_deleted() {
        return Err(IpcError::new(ErrorCode::ItemDeleted));
    }
    let config = item.otp().ok_or(IpcError::new(ErrorCode::NoOtp))?;
    if config.hardware_protected {
        return Err(IpcError::new(ErrorCode::VaultUnavailable));
    }
    let code = session.otp_code(vault_id, item_id, now_unix_seconds()?)?;
    let time_based = config.kind != OtpKind::Hotp;
    Ok((
        SecretString::from_str_copy(code.code.expose_secret())?,
        code.seconds_remaining.filter(|_| time_based),
        time_based.then_some(config.period),
    ))
}

impl Backend {
    /// The current code of an item's own one-time password. A background
    /// refresh, so it does not count as user activity.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `itemDeleted`, `noOtp`, `vaultUnavailable`.
    pub fn totp_code(&self, request: ItemRequest) -> IpcResult<TotpCode> {
        let mut slot = self.slot()?;
        let (code, seconds_remaining, period) = current_code(
            slot.unlocked_mut()?,
            request.vault_id.get(),
            request.item_id.get(),
        )?;
        Ok(TotpCode {
            code: SecretOut::new(code),
            seconds_remaining,
            period,
        })
    }
}
