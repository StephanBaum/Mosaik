//! Export commands (ADR-0041). `export_passkeys_cxf` writes the ES256
//! passkeys of one vault to a FIDO CXF JSON file, **unencrypted**, so it
//! needs the master password again and spends the re-authentication.
//!
//! Rust opens the native save dialog and writes the file; the webview sends a
//! vault ID and the password and gets a count back. The file is built from a
//! zeroizing buffer after the dialog returns a path, and written atomically
//! (mode 0600, Mark of the Web on Windows) like an attachment's Save as. The
//! session lock is never held across the dialog or the write.

use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_core::VaultId;
use tauri::State;

use crate::attachment_files::write_file_atomic;
use crate::browser::check_vault;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::export::{ExportPasskeysRequest, PasskeysExported};
use crate::contract::session::ReauthPurpose;
use crate::dialogs::JSON;
use crate::passkey_export;
use crate::state::{AppState, Backend};

/// `export_passkeys_cxf`.
#[tauri::command]
pub async fn export_passkeys_cxf(
    state: State<'_, AppState>,
    request: ExportPasskeysRequest,
) -> IpcResult<PasskeysExported> {
    state.run(move |b| b.export_passkeys_cxf(request)).await
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Backend {
    /// Exports the ES256 passkeys of a vault to a CXF file.
    ///
    /// A non-empty `password` is checked first (`wrongCredentials` counts
    /// towards the five-failure lock); then the two-minute re-authentication
    /// is **taken**, so one password pays for one export. A cancelled dialog
    /// or a failed write hands it back.
    ///
    /// # Errors
    /// `locked`, `vaultNotFound`, `vaultUnavailable` (closed or
    /// hardware-protected), `itemNotFound` (no ES256 passkey in the vault),
    /// `reauthRequired`, `wrongCredentials`, `attachmentSaveFailed` (the file
    /// could not be written there), `internal`.
    pub fn export_passkeys_cxf(
        &self,
        request: ExportPasskeysRequest,
    ) -> IpcResult<PasskeysExported> {
        self.touch();
        let vault = request.vault_id.get();
        {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            check_vault(session, vault)?;
            if passkey_export::count(session, vault) == 0 {
                return Err(IpcError::new(ErrorCode::ItemNotFound));
            }
        }
        if let Some(password) = request.password
            && !password.expose_secret().is_empty()
        {
            self.reauthenticate(password, ReauthPurpose::PasskeyExport)?;
        }
        let grant = self.take_reauth_grant(ReauthPurpose::PasskeyExport)?;
        let exported = self.export_passkeys_now(vault);
        if !matches!(exported, Ok(PasskeysExported::Saved { .. })) {
            self.quick_unlock.restore_reauth(grant);
        }
        exported
    }

    fn export_passkeys_now(&self, vault: VaultId) -> IpcResult<PasskeysExported> {
        let now = now_seconds();
        let Some(path) = self
            .dialogs
            .save_file(&passkey_export::suggested_file_name(now), JSON)?
        else {
            return Ok(PasskeysExported::Cancelled);
        };
        // The vault may have locked while the dialog was open: nothing is
        // built then.
        let exported = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            check_vault(session, vault)?;
            passkey_export::build(session, vault, now)?
        };
        write_file_atomic(&path, &exported.json)?;
        Ok(PasskeysExported::Saved {
            count: exported.count,
        })
    }
}
