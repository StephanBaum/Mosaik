//! The IPC request types: the first byte (minus `A`, modulo the number of types) picks the
//! type, the rest is the JSON body. Deserializing never panics, and
//! `draft::prepare` refuses a bad draft with `invalidInput` (or
//! `otpPendingExpired` for a `setPending` without a slot) only.
#![no_main]
#![allow(
    dead_code,
    unused_imports,
    reason = "the target includes the app modules and uses a subset"
)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/contract/mod.rs"]
mod contract;
#[path = "../../src/commands/draft.rs"]
mod draft;
#[path = "../../src/persist.rs"]
mod persist;
/// `persist.rs` checks a stored shortcut with `crate::shortcut`, which needs
/// Tauri. The harness stands in a check that accepts any non-empty text; the
/// real parser has its own unit tests.
mod shortcut {
    pub fn parse_accelerator(text: &str) -> Result<(), ()> {
        if text.is_empty() {
            Err(())
        } else {
            Ok(())
        }
    }
}

use contract::clipboard::CopyRequest;
use contract::credential_rotation::{
    HistoryRestoreRequest, RotationAttemptRequest, RotationBeginRequest, RotationCloseRequest,
    RotationPolicyRequest, RotationSnoozeRequest,
};
use contract::errors::ErrorCode;
use contract::generator::GenerateRequest;
use contract::import::{
    ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportPreviewRequest,
};
use contract::otp::{OtpParseRequest, OtpPendingRequest, OtpScanRequest};
use contract::items::{
    AddItemRequest, ItemRequest, ListItemsRequest, MoveItemRequest, RevealRequest,
    SetFavoriteRequest, UpdateItemRequest,
};
use contract::rotation::{
    DismissFileRequest, ProfileUpgradeRequest, RekeyCommitRequest, RekeyPrepareRequest,
    RotateCommitRequest, RotatePrepareRequest, RotationResumeRequest,
};
use contract::quick_unlock::{
    CreateHardwareVaultRequest, EnrollBiometricRequest, EnrollHardwareKeyRequest,
    HardwareVaultKeyRequest, HardwareVaultRequest, RemoveHardwareKeyRequest,
};
use contract::session::{
    CreateAccountRequest, QuickUnlockRequest, ReauthenticateRequest, SaveKitRequest, UnlockRequest, VerifyKitRequest,
};
use contract::settings::UpdateSettingsRequest;
use contract::sharing::{
    ContactCodeRequest, EmergencyClaimRequest, EmergencyGrantRequest, EmergencyRequestRequest,
    EmergencyRespondRequest, EmergencyRevokeRequest, InviteAcceptRequest, InviteDeclineRequest,
    InviteRequest, LookupRequest, MemberRequest, SetRoleRequest, ShareVaultRequest,
};
use contract::ssh::{SshRespondRequest, SshSetEnabledRequest};
use contract::sync::{
    AcknowledgeRequest, ChangePasswordRequest, CheckServerRequest, CodeRequest,
    DeviceRemoveRequest, DeviceRenameRequest, FolderConnectRequest, HostedConnectRequest,
    JoinFolderRequest, JoinHostedRequest, RecoverPasswordRequest, TotpBeginRequest,
    TotpDisableRequest,
};
use contract::vaults::{CreateVaultRequest, RenameVaultRequest, VaultRequest};

fn parse<'a, T: serde::Deserialize<'a>>(json: &'a [u8]) -> Option<T> {
    serde_json::from_slice(json).ok()
}

/// `parse_method_id` never panics and accepts only the canonical form.
fn check_method_id(text: &str) {
    if let Some(id) = contract::quick_unlock::parse_method_id(text) {
        assert_eq!(contract::quick_unlock::method_id_hex(&id), text);
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((sel, json)) = data.split_first() else {
        return;
    };
    let kind = usize::from(sel.wrapping_sub(b'A')) % 73;
    match kind {
        0 => drop(parse::<CreateAccountRequest>(json)),
        1 => drop(parse::<VerifyKitRequest>(json)),
        2 => drop(parse::<UnlockRequest>(json)),
        3 => drop(parse::<QuickUnlockRequest>(json)),
        4 => drop(parse::<CreateVaultRequest>(json)),
        5 => drop(parse::<RenameVaultRequest>(json)),
        6 => drop(parse::<ListItemsRequest>(json)),
        7 => drop(parse::<ItemRequest>(json)),
        8 => drop(parse::<RevealRequest>(json)),
        9 => {
            if let Some(r) = parse::<AddItemRequest>(json) {
                if let Err(error) = draft::prepare(r.draft, None) {
                    // setPending refers to a slot the fuzzer does not have.
                    assert!(matches!(
                        error.code,
                        ErrorCode::InvalidInput | ErrorCode::OtpPendingExpired
                    ));
                }
            }
        }
        10 => {
            if let Some(r) = parse::<UpdateItemRequest>(json) {
                if let Err(error) = draft::prepare(r.draft, None) {
                    // setPending refers to a slot the fuzzer does not have.
                    assert!(matches!(
                        error.code,
                        ErrorCode::InvalidInput | ErrorCode::OtpPendingExpired
                    ));
                }
            }
        }
        11 => drop(parse::<SetFavoriteRequest>(json)),
        12 => drop(parse::<MoveItemRequest>(json)),
        13 => drop(parse::<CopyRequest>(json)),
        14 => drop(parse::<GenerateRequest>(json)),
        15 => drop(parse::<UpdateSettingsRequest>(json)),
        16 => drop(parse::<ImportChooseRequest>(json)),
        17 => drop(parse::<ImportPreviewRequest>(json)),
        18 => drop(parse::<ImportCommitRequest>(json)),
        19 => drop(parse::<ImportCancelRequest>(json)),
        20 => drop(parse::<SshSetEnabledRequest>(json)),
        21 => drop(parse::<SaveKitRequest>(json)),
        22 => drop(parse::<EnrollBiometricRequest>(json)),
        23 => drop(parse::<EnrollHardwareKeyRequest>(json)),
        24 => {
            if let Some(r) = parse::<RemoveHardwareKeyRequest>(json) {
                check_method_id(&r.method_id);
            }
        }
        25 => drop(parse::<CreateHardwareVaultRequest>(json)),
        26 => drop(parse::<HardwareVaultRequest>(json)),
        27 => {
            if let Some(r) = parse::<HardwareVaultKeyRequest>(json) {
                check_method_id(&r.method_id);
            }
        }
        28 => drop(parse::<VaultRequest>(json)),
        29 => drop(parse::<ReauthenticateRequest>(json)),
        30 => drop(parse::<SshRespondRequest>(json)),
        31 => drop(parse::<OtpScanRequest>(json)),
        32 => drop(parse::<OtpParseRequest>(json)),
        33 => drop(parse::<OtpPendingRequest>(json)),
        34 => drop(parse::<CheckServerRequest>(json)),
        35 => drop(parse::<HostedConnectRequest>(json)),
        36 => drop(parse::<CodeRequest>(json)),
        37 => {
            // The webview never sends a folder path (ADR-0034): a request that
            // names one is not a request.
            if parse::<FolderConnectRequest>(json).is_some() {
                let text = String::from_utf8_lossy(json);
                assert!(!text.contains("\"path\""));
            }
        }
        38 => drop(parse::<DeviceRenameRequest>(json)),
        39 => drop(parse::<DeviceRemoveRequest>(json)),
        40 => drop(parse::<TotpBeginRequest>(json)),
        41 => drop(parse::<TotpDisableRequest>(json)),
        42 => drop(parse::<JoinHostedRequest>(json)),
        43 => drop(parse::<JoinFolderRequest>(json)),
        44 => drop(parse::<ChangePasswordRequest>(json)),
        45 => drop(parse::<RecoverPasswordRequest>(json)),
        46 => drop(parse::<AcknowledgeRequest>(json)),
        // Sharing and emergency access (M6b).
        47 => drop(parse::<LookupRequest>(json)),
        48 => drop(parse::<ContactCodeRequest>(json)),
        49 => drop(parse::<ShareVaultRequest>(json)),
        50 => drop(parse::<MemberRequest>(json)),
        51 => drop(parse::<SetRoleRequest>(json)),
        52 => drop(parse::<InviteRequest>(json)),
        53 => drop(parse::<InviteAcceptRequest>(json)),
        54 => drop(parse::<InviteDeclineRequest>(json)),
        55 => {
            // A grant names its scope as a tagged union; whatever parses must
            // serialize back to the same kind.
            if let Some(r) = parse::<EmergencyGrantRequest>(json) {
                let text = serde_json::to_string(&r.scope).unwrap_or_default();
                assert!(text.contains("\"kind\""));
            }
        }
        56 => drop(parse::<EmergencyRevokeRequest>(json)),
        57 => drop(parse::<EmergencyRequestRequest>(json)),
        58 => drop(parse::<EmergencyRespondRequest>(json)),
        59 => drop(parse::<EmergencyClaimRequest>(json)),
        // Account keys (M7).
        60 => drop(parse::<RotatePrepareRequest>(json)),
        61 => drop(parse::<RotateCommitRequest>(json)),
        62 => drop(parse::<ProfileUpgradeRequest>(json)),
        63 => drop(parse::<RekeyPrepareRequest>(json)),
        64 => drop(parse::<RekeyCommitRequest>(json)),
        65 => drop(parse::<RotationResumeRequest>(json)),
        66 => drop(parse::<DismissFileRequest>(json)),
        // Credential rotation (M7b): none of these carries a URL, a path or a hash.
        67 => drop(parse::<RotationBeginRequest>(json)),
        68 => drop(parse::<RotationAttemptRequest>(json)),
        69 => drop(parse::<RotationCloseRequest>(json)),
        70 => drop(parse::<HistoryRestoreRequest>(json)),
        71 => drop(parse::<RotationSnoozeRequest>(json)),
        _ => drop(parse::<RotationPolicyRequest>(json)),
    }
});
