//! `setup_code_show` (ADR-0096, ADR-0161): the QR code a phone scans to join
//! this hosted or folder account without typing the Secret Key. It re-authenticates with the
//! master password like every view of the Secret Key, but earns no grant for
//! other commands. Rust builds the payload in zeroizing buffers
//! (`mosaic_sync::setup_code`) and renders the modules; the webview draws
//! them and hides them after 60 s, on blur and on lock.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use mosaic_sync::folder::hint::detect_hint;
use mosaic_sync::setup_code::{FolderSetupCode, SetupCode, build, build_folder};
use mosaic_sync::wire::{email, origin};
use mosaic_vault::session::SyncKind;
use qrcodegen::{QrCode, QrCodeEcc};
use tauri::State;

use crate::commands::session::master_password;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::setup_code::{SETUP_CODE_VISIBLE_MS, SetupCodeShowRequest, SetupCodeShown};
use crate::state::{AppState, Backend};
use crate::sync::gather_hint_sources;

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

/// What the account's sync configuration gives the code.
enum Source {
    Hosted { server: String, address: String },
    Folder { path: Option<PathBuf> },
}

impl Backend {
    /// Builds the setup code of the hosted or folder account and renders it.
    ///
    /// The sync kind is checked first (it holds nothing secret), then the
    /// master password (a wrong one counts towards the five-failure lock). A
    /// folder account gets the v2 code with a provider and path hint read from
    /// the provider clients' configuration (ADR-0161).
    ///
    /// # Errors
    /// `locked`, `setupCodeNeedsServer` (no sync), `emptyPassword`,
    /// `wrongCredentials`, `secretKeyRequired`, `internal`.
    pub fn setup_code_show(&self, request: SetupCodeShowRequest) -> IpcResult<SetupCodeShown> {
        self.touch();
        let (account, source) = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            let config = session
                .sync_config()?
                .ok_or(IpcError::new(ErrorCode::SetupCodeNeedsServer))?;
            let source = match config.kind {
                SyncKind::Hosted => {
                    let (Some(server), Some(address)) = (config.server_url, config.email) else {
                        return Err(IpcError::new(ErrorCode::SetupCodeNeedsServer));
                    };
                    Source::Hosted { server, address }
                }
                SyncKind::Folder => Source::Folder {
                    path: config.folder_path,
                },
            };
            (session.account_id(), source)
        };
        self.verify_master_only(&master_password(request.password))?;
        let secret_key = self
            .secret_key_of(account)
            .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?;
        let text = match source {
            Source::Hosted { server, address } => build(&SetupCode {
                server: origin::canonical_origin(&server).map_err(|_| IpcError::INTERNAL)?,
                email: email::normalize(&address).map_err(|_| IpcError::INTERNAL)?,
                secret_key,
            })?,
            Source::Folder { path } => {
                // No folder path (never the case for a connected folder) gives `other`.
                let hint = detect_hint(&path.unwrap_or_default(), &gather_hint_sources());
                build_folder(&FolderSetupCode {
                    account: *account.as_bytes(),
                    provider: hint.provider,
                    server: hint.server,
                    path: hint.path,
                    secret_key,
                })?
            }
        };
        // `text` is a `Zeroizing<String>`, wiped on drop. The QR library's own
        // segment, codeword and module buffers are not wiped (CRYPTO_SPEC
        // 6.9.7); the matrix it returns is what the screen shows.
        let qr = QrCode::encode_text(text.as_str(), QrCodeEcc::Medium)
            .map_err(|_| IpcError::INTERNAL)?;
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
