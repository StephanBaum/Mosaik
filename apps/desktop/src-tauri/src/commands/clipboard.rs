//! Clipboard commands: Rust reads the value and writes the system
//! clipboard itself (`crate::clipboard`), so the secret never enters
//! JavaScript. The receipt says when the clipboard clears.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mosaic_core::SecretString;
use mosaic_core::model::{CustomFieldValue, SecretField, TextField};
use mosaic_vault::otp::otp_from_config;
use mosaic_vault::session::UnlockedSession;
use tauri::State;

use crate::contract::clipboard::{CopyField, CopyReceipt, CopyRequest, CopySource};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::state::{AppState, Backend, guard};

/// `copy_to_clipboard`.
#[tauri::command]
pub async fn copy_to_clipboard(
    state: State<'_, AppState>,
    request: CopyRequest,
) -> IpcResult<CopyReceipt> {
    state.run(move |b| b.copy_to_clipboard(request)).await
}

/// `clear_clipboard`: clears it now if it still holds what Mosaic copied.
#[tauri::command]
pub async fn clear_clipboard(state: State<'_, AppState>) -> IpcResult<()> {
    state.run(Backend::clear_clipboard).await
}

fn copy_text(text: Option<&str>) -> IpcResult<SecretString> {
    let text = text
        .filter(|t| !t.is_empty())
        .ok_or(IpcError::new(ErrorCode::FieldEmpty))?;
    Ok(SecretString::from_str_copy(text)?)
}

/// The clock for one-time codes.
///
/// # Errors
/// `internal` if the system clock is before 1970.
pub(crate) fn now_unix_seconds() -> IpcResult<u64> {
    mosaic_totp::unix_time_now().map_err(|_| IpcError::INTERNAL)
}

/// The value of one field of an item (HOTP codes advance the counter).
///
/// # Errors
/// `itemNotFound`, `itemDeleted`, `fieldEmpty`, `noOtp`.
pub(crate) fn field_value(
    session: &mut UnlockedSession,
    vault_id: mosaic_core::VaultId,
    item_id: mosaic_core::ItemId,
    field: &CopyField,
) -> IpcResult<SecretString> {
    let item = session.item(vault_id, item_id)?;
    if item.is_deleted() {
        return Err(IpcError::new(ErrorCode::ItemDeleted));
    }
    match field {
        CopyField::Username => copy_text(item.text(TextField::Username)),
        CopyField::Text { field } => copy_text(item.text(field.to_model())),
        CopyField::Password => item
            .secret(SecretField::Password)?
            .ok_or(IpcError::new(ErrorCode::FieldEmpty)),
        CopyField::Secret { field } => item
            .secret(field.to_model())?
            .ok_or(IpcError::new(ErrorCode::FieldEmpty)),
        CopyField::Totp => {
            super::totp::current_code(session, vault_id, item_id).map(|(code, ..)| code)
        }
        CopyField::CustomField { element_id } => {
            let field = item
                .custom_fields()
                .into_iter()
                .find(|(id, _)| id.as_str() == element_id)
                .map(|(_, field)| field)
                .ok_or(IpcError::new(ErrorCode::FieldEmpty))?;
            match field.value {
                CustomFieldValue::Hidden(secret) if !secret.expose_secret().is_empty() => {
                    Ok(secret)
                }
                CustomFieldValue::Hidden(_) => Err(IpcError::new(ErrorCode::FieldEmpty)),
                CustomFieldValue::Text(t)
                | CustomFieldValue::Url(t)
                | CustomFieldValue::Email(t)
                | CustomFieldValue::Phone(t)
                | CustomFieldValue::Date(t) => copy_text(Some(t.as_str())),
                CustomFieldValue::Otp(config) => {
                    // A custom-field generator has no stored counter, so
                    // HOTP ones cannot advance safely; only time-based
                    // codes are copied.
                    if config.kind == mosaic_core::model::OtpKind::Hotp {
                        return Err(IpcError::new(ErrorCode::NoOtp));
                    }
                    let otp =
                        otp_from_config(&config, 0).map_err(|_| IpcError::new(ErrorCode::NoOtp))?;
                    let code = otp
                        .code_at(now_unix_seconds()?)
                        .map_err(|_| IpcError::new(ErrorCode::NoOtp))?;
                    Ok(SecretString::from_str_copy(code.expose_secret())?)
                }
            }
        }
    }
}

impl Backend {
    /// Copies a value and schedules the clear.
    ///
    /// # Errors
    /// `locked` (item values), `fieldEmpty`, `noOtp`, `itemNotFound`,
    /// `clipboardUnavailable`.
    pub fn copy_to_clipboard(&self, request: CopyRequest) -> IpcResult<CopyReceipt> {
        let value = match request.source {
            CopySource::Generated => {
                let generated = guard(&self.last_generated)?;
                let value = generated
                    .as_ref()
                    .ok_or(IpcError::new(ErrorCode::FieldEmpty))?;
                SecretString::from_str_copy(value.expose_secret())?
            }
            CopySource::Item {
                vault_id,
                item_id,
                field,
            } => {
                let mut slot = self.slot()?;
                field_value(slot.unlocked_mut()?, vault_id.get(), item_id.get(), &field)?
            }
        };
        self.touch();
        let seconds = self.settings_snapshot().clipboard_clear_seconds;
        let clear_after = seconds.map(|s| Duration::from_secs(u64::from(s)));
        self.clipboard.copy(value, clear_after)?;
        let clears_at = clear_after
            .and_then(|d| SystemTime::now().checked_add(d))
            .and_then(|t| {
                t.duration_since(UNIX_EPOCH)
                    .ok()
                    .and_then(|d| i64::try_from(d.as_millis()).ok())
            });
        Ok(CopyReceipt {
            clears_at: seconds.and(clears_at),
            clear_after_seconds: seconds,
        })
    }

    /// Clears the clipboard if it still holds what Mosaic copied.
    ///
    /// # Errors
    /// `clipboardUnavailable`.
    pub fn clear_clipboard(&self) -> IpcResult<()> {
        self.touch();
        self.clipboard.clear_if_ours().map(|_| ())
    }
}
