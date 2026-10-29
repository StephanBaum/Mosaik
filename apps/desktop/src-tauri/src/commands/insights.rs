//! Security dashboard command, through the insights adapter
//! (`adapters::InsightsAdapter`, `mosaic-insights`).

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::contract::errors::IpcResult;
use crate::contract::insights::SecurityReport;
use crate::state::{AppState, Backend};

/// `security_report`.
#[tauri::command]
pub async fn security_report(state: State<'_, AppState>) -> IpcResult<SecurityReport> {
    state.run(Backend::security_report).await
}

impl Backend {
    /// The security report over the unlocked vault.
    ///
    /// # Errors
    /// `locked`, the adapter's errors.
    pub fn security_report(&self) -> IpcResult<SecurityReport> {
        self.touch();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| i64::try_from(d.as_millis()).ok())
            .unwrap_or_default();
        let slot = self.slot()?;
        self.adapters.insights.report(slot.unlocked()?, now)
    }
}
