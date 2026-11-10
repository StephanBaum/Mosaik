//! Sync errors as IPC error codes. No variant of a transport error carries a
//! path, a body or an ID, so the code is all that reaches the webview.

use mosaic_sync::engine::SyncError;
use mosaic_sync::transport::{Rejection, TransportError};

use crate::contract::errors::{ErrorCode, IpcError};

/// The code of a transport failure.
#[must_use]
pub fn transport_code(error: TransportError) -> ErrorCode {
    match error {
        TransportError::Unreachable | TransportError::Timeout => ErrorCode::SyncServerUnreachable,
        // A session the server ended for a known reason is signed out too (the
        // reason-specific text is M9 T6, M7-R3).
        TransportError::Unauthorized | TransportError::Revoked(_) => ErrorCode::SyncSignedOut,
        TransportError::TwoFactorRequired | TransportError::SecondFactorUnsupported => {
            ErrorCode::SyncTwoFactorRequired
        }
        TransportError::FreshLoginRequired => ErrorCode::SyncFreshLoginExpired,
        TransportError::RateLimited { .. } => ErrorCode::SyncRateLimited,
        TransportError::QuotaExceeded => ErrorCode::SyncQuotaExceeded,
        TransportError::ServerMismatch => ErrorCode::SyncServerMismatch,
        TransportError::Incompatible => ErrorCode::SyncServerIncompatible,
        TransportError::Protocol | TransportError::InvalidUrl => ErrorCode::SyncServerInvalid,
        TransportError::InsecureUrl => ErrorCode::SyncServerInsecure,
        TransportError::FolderUnavailable | TransportError::Io => ErrorCode::SyncFolderUnavailable,
        TransportError::Locked => ErrorCode::Locked,
        TransportError::OutcomeUnknown => ErrorCode::SyncPasswordOutcomeUnknown,
        TransportError::Rejected(rejection) => rejection_code(rejection),
        // Forbidden, NotFound, GenerationMismatch, HeadBelowCursor and the
        // variants a later version adds: nothing the user can act on.
        _ => ErrorCode::Internal,
    }
}

fn rejection_code(rejection: Rejection) -> ErrorCode {
    match rejection {
        Rejection::ProfileDowngrade => ErrorCode::ProfileDowngrade,
        Rejection::EpochMismatch { .. } => ErrorCode::SyncRekeyRefused,
        Rejection::RekeyMismatch { .. } => ErrorCode::RekeyBusy,
        Rejection::AccountExists => ErrorCode::SyncAccountExists,
        Rejection::RegistrationClosed => ErrorCode::SyncRegistrationClosed,
        Rejection::InvalidInvite => ErrorCode::SyncInviteInvalid,
        Rejection::InvalidEmail | Rejection::BadRequest => ErrorCode::InvalidInput,
        Rejection::LoginFailed => ErrorCode::WrongCredentials,
        Rejection::DeviceRevoked => ErrorCode::SyncSignedOut,
        Rejection::TwoFactorInvalid => ErrorCode::SyncTwoFactorInvalid,
        Rejection::WebauthnUnavailable => ErrorCode::SyncWebauthnUnavailable,
        Rejection::HandoffExpired => ErrorCode::SyncHandoffExpired,
        Rejection::DeviceIsCurrent => ErrorCode::SyncDeviceIsCurrent,
        Rejection::TooLarge => ErrorCode::SyncQuotaExceeded,
        Rejection::RoleForbidden | Rejection::EmergencyOwnerOnly => ErrorCode::SharingRoleForbidden,
        Rejection::InviteExists => ErrorCode::SharingInviteExists,
        Rejection::InviteStale { .. } => ErrorCode::SharingInviteStale,
        Rejection::InviteExpired => ErrorCode::SharingInviteExpired,
        Rejection::InviteLimit => ErrorCode::SharingInviteLimit,
        Rejection::AlreadyMember => ErrorCode::SharingAlreadyMember,
        Rejection::ReinviteBlocked => ErrorCode::SharingReinviteBlocked,
        Rejection::OwnerCannotLeave => ErrorCode::SharingOwnerCannotLeave,
        Rejection::EmergencyWaiting { .. } => ErrorCode::EmergencyWaiting,
        Rejection::EmergencyCooldown { .. } => ErrorCode::EmergencyCooldown,
        Rejection::EmergencyNotRequested | Rejection::EmergencyReleased | Rejection::NoShare => {
            ErrorCode::EmergencyNotGranted
        }
        _ => ErrorCode::Internal,
    }
}

impl From<TransportError> for IpcError {
    fn from(error: TransportError) -> Self {
        Self::new(transport_code(error))
    }
}

/// The error of a failed sharing command. A session error keeps its own
/// code (`IpcError::from`); `not_found` is what an unknown invite or grant
/// means for the command; the server refusing a vault to this account is
/// `sharingRoleForbidden`.
#[must_use]
pub fn sharing_error(error: SyncError, not_found: ErrorCode) -> IpcError {
    match error {
        SyncError::Session(session) => IpcError::from(session),
        SyncError::SharingUnavailable => IpcError::new(ErrorCode::SharingNotHosted),
        SyncError::Transport(TransportError::NotFound) => IpcError::new(not_found),
        SyncError::Transport(TransportError::Forbidden) => {
            IpcError::new(ErrorCode::SharingRoleForbidden)
        }
        other => IpcError::new(sync_code(&other)),
    }
}

/// The code of a failed sync run.
#[must_use]
pub fn sync_code(error: &SyncError) -> ErrorCode {
    match error {
        SyncError::Locked => ErrorCode::Locked,
        SyncError::Transport(e) => transport_code(*e),
        SyncError::Session(mosaic_vault::session::Error::Store(_)) => ErrorCode::StoreUnavailable,
        _ => ErrorCode::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharing_errors_keep_their_meaning() {
        let nf = ErrorCode::SharingInviteExpired;
        assert_eq!(
            sharing_error(SyncError::Transport(TransportError::NotFound), nf).code,
            nf
        );
        assert_eq!(
            sharing_error(SyncError::SharingUnavailable, nf).code,
            ErrorCode::SharingNotHosted
        );
        assert_eq!(
            sharing_error(
                SyncError::Session(mosaic_vault::session::Error::ContactUnverified),
                nf
            )
            .code,
            ErrorCode::SharingContactUnverified
        );
        assert_eq!(
            sharing_error(SyncError::Transport(TransportError::Forbidden), nf).code,
            ErrorCode::SharingRoleForbidden
        );
    }

    #[test]
    fn transport_failures_map_to_the_codes_the_ui_acts_on() {
        let cases = [
            (
                TransportError::Unreachable,
                ErrorCode::SyncServerUnreachable,
            ),
            (TransportError::Timeout, ErrorCode::SyncServerUnreachable),
            (TransportError::Unauthorized, ErrorCode::SyncSignedOut),
            (
                TransportError::Revoked(mosaic_sync::transport::RevokedReason::Rekey),
                ErrorCode::SyncSignedOut,
            ),
            (
                TransportError::TwoFactorRequired,
                ErrorCode::SyncTwoFactorRequired,
            ),
            (
                TransportError::RateLimited { retry_after_s: 3 },
                ErrorCode::SyncRateLimited,
            ),
            (TransportError::InsecureUrl, ErrorCode::SyncServerInsecure),
            (TransportError::InvalidUrl, ErrorCode::SyncServerInvalid),
            (
                TransportError::ServerMismatch,
                ErrorCode::SyncServerMismatch,
            ),
            (
                TransportError::Incompatible,
                ErrorCode::SyncServerIncompatible,
            ),
            (
                TransportError::FolderUnavailable,
                ErrorCode::SyncFolderUnavailable,
            ),
            (
                TransportError::Rejected(Rejection::AccountExists),
                ErrorCode::SyncAccountExists,
            ),
            (
                TransportError::Rejected(Rejection::LoginFailed),
                ErrorCode::WrongCredentials,
            ),
            (
                TransportError::Rejected(Rejection::DeviceIsCurrent),
                ErrorCode::SyncDeviceIsCurrent,
            ),
            (TransportError::Locked, ErrorCode::Locked),
            (TransportError::Forbidden, ErrorCode::Internal),
            (
                TransportError::Rejected(Rejection::InviteExists),
                ErrorCode::SharingInviteExists,
            ),
            (
                TransportError::Rejected(Rejection::ReinviteBlocked),
                ErrorCode::SharingReinviteBlocked,
            ),
            (
                TransportError::Rejected(Rejection::EmergencyWaiting { release_at: None }),
                ErrorCode::EmergencyWaiting,
            ),
            (
                TransportError::Rejected(Rejection::EmergencyNotRequested),
                ErrorCode::EmergencyNotGranted,
            ),
        ];
        for (error, code) in cases {
            assert_eq!(transport_code(error), code, "{error:?}");
        }
    }
}
