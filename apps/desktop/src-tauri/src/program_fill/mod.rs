//! The desktop end of Windows program fill (ADR-0164): the block list and the status the
//! settings screen shows. The offer state machine, picks and links follow in the same module.

pub mod blocklist;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::contract::errors::IpcResult;
use crate::contract::program_fill::{
    ProgramFillStatus, ProgramFillUnavailable, ProgramFillUnblockRequest,
};
use crate::shortcut::HotkeyRegistrar;
use crate::state::Backend;

pub use blocklist::{BLOCKLIST_FILE, BlockList, MAX_BLOCKED};

/// Whether Mosaic runs above the normal integrity level (program fill then stays off: Mosaic
/// never asks for elevation, and a process that is elevated could be driven by a lower one's
/// input).
#[cfg(windows)]
fn mosaic_elevated() -> bool {
    /// The medium integrity level's RID.
    const MEDIUM_RID: u32 = 0x2000;
    // A failed read is treated as elevated: nothing starts that cannot be checked.
    mosaic_winshim::current_integrity_rid().map_or(true, |rid| rid > MEDIUM_RID)
}

/// Why program fill cannot run on this device, or `None`.
#[must_use]
pub fn unavailable_reason() -> Option<ProgramFillUnavailable> {
    if !cfg!(windows) {
        return Some(ProgramFillUnavailable::NotWindows);
    }
    #[cfg(windows)]
    if mosaic_elevated() {
        return Some(ProgramFillUnavailable::MosaicElevated);
    }
    None
}

/// The program fill service: the block list now, the offer state machine in the same struct.
pub struct ProgramFillService {
    blocklist: Mutex<BlockList>,
    /// Whether the watcher runs.
    running: AtomicBool,
    /// The watcher failed to start in this run.
    host_failed: AtomicBool,
    /// Registers the program fill hotkey (`None`: no hotkey manager, or a test).
    hotkey: Mutex<Option<HotkeyRegistrar>>,
}

impl ProgramFillService {
    /// A service with this block list.
    #[must_use]
    pub fn new(blocklist: BlockList) -> Self {
        Self {
            blocklist: Mutex::new(blocklist),
            running: AtomicBool::new(false),
            host_failed: AtomicBool::new(false),
            hotkey: Mutex::new(None),
        }
    }

    /// A service that keeps its block list nowhere (tests).
    #[must_use]
    pub fn memory() -> Self {
        Self::new(BlockList::memory())
    }

    pub(crate) fn blocklist(&self) -> MutexGuard<'_, BlockList> {
        self.blocklist
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the registrar of the program fill hotkey.
    pub fn set_hotkey_registrar(&self, registrar: Option<HotkeyRegistrar>) {
        *self.hotkey.lock().unwrap_or_else(PoisonError::into_inner) = registrar;
    }

    pub(crate) fn hotkey_registrar(&self) -> Option<HotkeyRegistrar> {
        self.hotkey
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn status(&self) -> ProgramFillStatus {
        let reason = unavailable_reason().or_else(|| {
            self.host_failed
                .load(Ordering::SeqCst)
                .then_some(ProgramFillUnavailable::HostFailed)
        });
        ProgramFillStatus {
            available: reason.is_none(),
            reason,
            running: self.running.load(Ordering::SeqCst),
            blocked: self.blocklist().entries().to_vec(),
        }
    }
}

impl Backend {
    /// `program_fill_status`.
    #[must_use]
    pub fn program_fill_status(&self) -> ProgramFillStatus {
        self.program_fill.status()
    }

    /// `program_fill_unblock`: offers the program's logins again.
    ///
    /// # Errors
    /// `storeUnavailable` when the list cannot be written (the entry is off the list for this run).
    pub fn program_fill_unblock(
        &self,
        request: &ProgramFillUnblockRequest,
    ) -> IpcResult<ProgramFillStatus> {
        self.touch();
        self.program_fill.blocklist().remove(&request.key)?;
        Ok(self.program_fill.status())
    }

    /// Starts or stops the watcher to match the settings (called by `update_settings` and after
    /// every unlock). The watcher itself follows with the offer state machine.
    pub fn program_fill_apply_settings(&self) {}

    /// The program fill hotkey was pressed.
    pub fn program_fill_hotkey_pressed(&self) {}
}

/// The hotkey's action: asks the backend to capture the focused field (a no-op until then).
pub fn on_program_fill_hotkey<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager as _;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        state.backend().program_fill_hotkey_pressed();
    }
}
