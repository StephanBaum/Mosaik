//! Program fill (ADR-0164): the status of the feature and its block list. What the picker
//! windows see is in [`super::picker`].

use serde::{Deserialize, Serialize};

use super::macros::wire_enum;

/// A program the user chose not to be offered logins for ("Don't offer for this program").
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockedProgram {
    /// The block key (`mosaic_program_fill::binding::block_key`). Opaque to the webview.
    pub key: String,
    /// The program's name for the list.
    pub label: String,
}

wire_enum!(
    /// Why program fill is not available on this device.
    pub enum ProgramFillUnavailable {
        /// Program fill reads Windows programs; this is another system.
        NotWindows,
        /// Mosaic itself runs above the normal integrity level.
        MosaicElevated,
        /// The watcher could not start.
        HostFailed,
    }
);

/// `program_fill_status`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramFillStatus {
    /// Whether program fill can run on this device.
    pub available: bool,
    /// Why not, when `available` is false.
    pub reason: Option<ProgramFillUnavailable>,
    /// Whether the watcher runs now (it runs only while unlocked and switched on).
    pub running: bool,
    /// The block list.
    pub blocked: Vec<BlockedProgram>,
}

/// `program_fill_unblock`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgramFillUnblockRequest {
    /// The block key to remove.
    pub key: String,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;

    pub(crate) fn statuses() -> Vec<ProgramFillStatus> {
        vec![
            ProgramFillStatus {
                available: true,
                reason: None,
                running: true,
                blocked: vec![BlockedProgram {
                    key: "signed:notepad.exe:AAAA".into(),
                    label: "Notepad".into(),
                }],
            },
            ProgramFillStatus {
                available: false,
                reason: Some(ProgramFillUnavailable::NotWindows),
                running: false,
                blocked: vec![],
            },
        ]
    }

    pub(crate) fn unblock_requests() -> Vec<ProgramFillUnblockRequest> {
        vec![ProgramFillUnblockRequest {
            key: "signed:notepad.exe:AAAA".into(),
        }]
    }
}
