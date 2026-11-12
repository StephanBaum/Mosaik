//! `setup_code_show` (ADR-0096): the QR code a phone scans to join this
//! hosted account without typing the Secret Key. It re-authenticates with the
//! master password like every view of the Secret Key, but earns no grant for
//! other commands. Rust builds the payload in zeroizing buffers
//! (`mosaic_sync::setup_code`) and renders the modules; the webview draws
//! them and hides them after 60 s, on blur and on lock.

use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_sync::setup_code::{SetupCode, build};
use mosaic_sync::wire::{email, origin};
use mosaic_vault::session::SyncKind;
use qrcodegen::{QrCode, QrCodeEcc};
use tauri::State;

use crate::commands::session::master_password;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::setup_code::{SETUP_CODE_VISIBLE_MS, SetupCodeShowRequest, SetupCodeShown};
use crate::state::{AppState, Backend};

/// `setup_code_show`.
#[tauri::command]
pub async fn setup_code_show(
    state: State<'_, AppState>,
    request: SetupCodeShowRequest,
) -> IpcResult<SetupCodeShown> {
    state.run(move |b| b.setup_code_show(request)).await
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

impl Backend {
    /// Builds the setup code of the hosted account and renders it.
    ///
    /// The sync kind is checked first (it holds nothing secret), then the
    /// master password (a wrong one counts towards the five-failure lock).
    ///
    /// # Errors
    /// `locked`, `setupCodeNeedsServer` (no sync, or a folder), `emptyPassword`,
    /// `wrongCredentials`, `secretKeyRequired`, `internal`.
    pub fn setup_code_show(&self, request: SetupCodeShowRequest) -> IpcResult<SetupCodeShown> {
        self.touch();
        let (account, server, address) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let config = session
                .sync_config()?
                .filter(|c| c.kind == SyncKind::Hosted)
                .ok_or(IpcError::new(ErrorCode::SetupCodeNeedsServer))?;
            let (Some(server), Some(address)) = (config.server_url, config.email) else {
                return Err(IpcError::new(ErrorCode::SetupCodeNeedsServer));
            };
            (session.account_id(), server, address)
        };
        self.verify_master_only(&master_password(request.password))?;
        let secret_key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let code = SetupCode {
            server: origin::canonical_origin(&server).map_err(|_| IpcError::INTERNAL)?,
            email: email::normalize(&address).map_err(|_| IpcError::INTERNAL)?,
            secret_key,
        };
        let text = build(&code)?;
        // The QR library keeps its own copy of the text while it encodes; the
        // matrix it returns is the same information the screen shows.
        let qr = QrCode::encode_text(&text, QrCodeEcc::Medium).map_err(|_| IpcError::INTERNAL)?;
        drop(text);
        let size = qr.size();
        let modules = (0..size)
            .map(|y| (0..size).map(|x| qr.get_module(x, y)).collect())
            .collect();
        Ok(SetupCodeShown {
            modules,
            size: u32::try_from(size).map_err(|_| IpcError::INTERNAL)?,
            expires_at: now_millis() + SETUP_CODE_VISIBLE_MS,
        })
    }
}
