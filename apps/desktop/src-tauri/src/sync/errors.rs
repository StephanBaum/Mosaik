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
        TransportError::Unauthorized => ErrorCode::SyncSignedOut,
        TransportError::TwoFactorRequired => ErrorCode::SyncTwoFactorRequired,
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
        Rejection::AccountExists => ErrorCode::SyncAccountExists,
        Rejection::RegistrationClosed => ErrorCode::SyncRegistrationClosed,
        Rejection::InvalidInvite => ErrorCode::SyncInviteInvalid,
        Rejection::InvalidEmail | Rejection::BadRequest => ErrorCode::InvalidInput,
        Rejection::LoginFailed => ErrorCode::WrongCredentials,
        Rejection::DeviceRevoked => ErrorCode::SyncSignedOut,
        Rejection::TwoFactorInvalid => ErrorCode::SyncTwoFactorInvalid,
        Rejection::DeviceIsCurrent => ErrorCode::SyncDeviceIsCurrent,
        Rejection::TooLarge => ErrorCode::SyncQuotaExceeded,
        _ => ErrorCode::Internal,
    }
}

impl From<TransportError> for IpcError {
    fn from(error: TransportError) -> Self {
        Self::new(transport_code(error))
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
    fn transport_failures_map_to_the_codes_the_ui_acts_on() {
        let cases = [
            (
                TransportError::Unreachable,
                ErrorCode::SyncServerUnreachable,
            ),
            (TransportError::Timeout, ErrorCode::SyncServerUnreachable),
            (TransportError::Unauthorized, ErrorCode::SyncSignedOut),
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
        ];
        for (error, code) in cases {
            assert_eq!(transport_code(error), code, "{error:?}");
        }
    }
}
