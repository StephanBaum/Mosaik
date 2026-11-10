//! The one error type every command returns (`docs/DESKTOP_IPC.md`,
//! "Errors"). It carries a code and nothing else: no message, no ID, no
//! value from the vault, so it can never leak a secret. The webview maps
//! every code to the i18n key `error.<code>`, which says what happened and
//! what to do next (UX_GUIDELINES §5).

use mosaic_vault::session::Error as SessionError;
use mosaic_vault::storage::Error as StoreError;
use serde::Serialize;

use super::macros::wire_enum;

wire_enum!(
    /// Why a command failed.
    pub enum ErrorCode {
        /// The command is part of the contract but not built yet.
        NotImplemented,
        /// There is no account on this device yet.
        NoAccount,
        /// This device already has an account.
        AccountExists,
        /// The vault is locked; unlock first.
        Locked,
        /// The vault is already unlocked.
        AlreadyUnlocked,
        /// The master password or the Secret Key is wrong (never which one,
        /// CRYPTO_SPEC §5.4).
        WrongCredentials,
        /// This device does not hold the Secret Key; the user enters it from
        /// the Emergency Kit.
        SecretKeyRequired,
        /// The Secret Key is not in the `M1-` format.
        SecretKeyMalformed,
        /// The Secret Key's checksum does not match: a typo.
        SecretKeyMistyped,
        /// The master password is empty.
        EmptyPassword,
        /// The quick-unlock method is not set up or not available.
        QuickUnlockUnavailable,
        /// The user dismissed the OS prompt or the security-key touch was
        /// cancelled. Nothing changed.
        Cancelled,
        /// The quick-unlock enrollment was past its maximum age and was
        /// removed; unlock with the master password and set it up again.
        QuickUnlockExpired,
        /// The platform key no longer opens the enrollment (a changed
        /// fingerprint, a reset Hello); it was removed.
        QuickUnlockInvalidated,
        /// The master password is required first (after an app start, or
        /// after five rejected quick unlocks in a row).
        PasswordRequired,
        /// This build cannot talk to security keys.
        HardwareKeyUnsupported,
        /// No security key is plugged in.
        HardwareKeyNotFound,
        /// The security key was not touched in time.
        HardwareKeyTimeout,
        /// The security key needs its PIN.
        HardwareKeyPinRequired,
        /// The security key's PIN is wrong.
        HardwareKeyPinInvalid,
        /// The security key's PIN is blocked.
        HardwareKeyPinBlocked,
        /// The security key has no PIN set.
        HardwareKeyNoPin,
        /// The plugged-in security key is not registered here, or its answer
        /// did not open the wrapping.
        HardwareKeyNotRecognized,
        /// The most security keys are registered.
        HardwareKeyLimit,
        /// The last security key of a hardware-protected vault cannot be
        /// removed.
        HardwareKeyLast,
        /// A hardware-protected vault holds one-time-code items only.
        ItemTypeNotAllowed,
        /// Screen scanning is not available on this system.
        OtpScanUnsupported,
        /// The scan found no QR code.
        OtpQrNotFound,
        /// The QR code is not an `otpauth://` setup code.
        OtpQrNotOtp,
        /// The QR code is a Google Authenticator export.
        OtpQrMigration,
        /// Several different setup codes were visible.
        OtpQrMultiple,
        /// The setup code uses settings Mosaic does not support.
        OtpQrInvalid,
        /// The text is not a valid `otpauth://` link.
        OtpUriInvalid,
        /// The setup key is not valid base32.
        OtpKeyInvalid,
        /// The image cannot be read (not PNG or JPEG, or damaged).
        OtpImageUnreadable,
        /// The image is over a size limit.
        OtpImageTooLarge,
        /// The clipboard holds no image.
        OtpClipboardNoImage,
        /// The scanned setup expired, was used or was never created.
        OtpPendingExpired,
        /// Another Mosaic process holds the local vault.
        StoreInUse,
        /// The local vault file could not be opened or read.
        StoreUnavailable,
        /// The vault does not exist.
        VaultNotFound,
        /// The vault's key is not available (hardware-protected or damaged).
        VaultUnavailable,
        /// The item does not exist.
        ItemNotFound,
        /// The item was deleted.
        ItemDeleted,
        /// Source and target vault of a move are the same.
        SameVault,
        /// The item has no one-time password.
        NoOtp,
        /// The field is empty, so there is nothing to copy or reveal.
        FieldEmpty,
        /// A request value is out of range or malformed.
        InvalidInput,
        /// The pasted text is not an OpenSSH private key (or is too long).
        SshKeyInvalid,
        /// The change needs the master password again (UX_GUIDELINES §8).
        ReauthRequired,
        /// The file is larger than 50 MiB.
        AttachmentTooLarge,
        /// The account's attachments would exceed 1 GiB.
        AttachmentQuota,
        /// The item has no such file (removed or never synced here).
        AttachmentNotFound,
        /// The file is not on this device (not synced yet), damaged or failed
        /// authentication.
        AttachmentUnavailable,
        /// The chosen file could not be read.
        AttachmentFileUnreadable,
        /// The file could not be written where the user chose.
        AttachmentSaveFailed,
        /// The file is not a PNG or JPEG image the preview can decode.
        AttachmentPreviewUnsupported,
        /// The image is over a preview size limit.
        AttachmentPreviewTooLarge,
        /// The system clipboard could not be written.
        ClipboardUnavailable,
        /// The import file could not be read.
        ImportFileUnreadable,
        /// The import file is not in the chosen format.
        ImportFormatInvalid,
        /// The import preview is gone (cancelled, locked or too old).
        ImportPreviewExpired,
        /// The import file is too large or too complex.
        ImportTooLarge,
        /// The export's format version or cipher is not supported.
        ImportUnsupported,
        /// The export is password-protected in a way Mosaic cannot read.
        ImportEncrypted,
        /// The SSH approval request was already answered or expired.
        SshRequestExpired,
        /// The SSH agent could not start.
        SshAgentUnavailable,
        /// Another SSH agent already uses the endpoint.
        SshAgentEndpointInUse,
        /// Too many wrong master passwords in a row: wait a moment, then try
        /// again. Nothing was checked.
        UnlockDelayed,
        /// Too many wrong answers to the Emergency Kit check; the vault
        /// locked and the next unlock needs the master password.
        TooManyAttempts,
        /// An SSH approval was allowed less than a second after the prompt
        /// appeared; the request is still open.
        SshAllowTooSoon,
        /// The browser pairing or save request was already answered or
        /// expired.
        BrowserRequestExpired,
        /// A browser pairing or save was allowed less than a second after
        /// the prompt appeared; the request is still open.
        BrowserAllowTooSoon,
        /// No browser pairing with that ID.
        BrowserPairingNotFound,
        /// Sync is not set up.
        SyncNotConfigured,
        /// Sync is already set up; disconnect first.
        SyncAlreadyConfigured,
        /// The sync server cannot be reached.
        SyncServerUnreachable,
        /// The address is not a Mosaic server.
        SyncServerInvalid,
        /// The server address is not https.
        SyncServerInsecure,
        /// The server is not the one this account was set up with.
        SyncServerMismatch,
        /// The server speaks another protocol version.
        SyncServerIncompatible,
        /// The server does not accept new accounts.
        SyncRegistrationClosed,
        /// The invite code is missing or wrong.
        SyncInviteInvalid,
        /// A sync account with this email exists already.
        SyncAccountExists,
        /// The sync account is another account than the one on this device.
        SyncAccountMismatch,
        /// The sync account needs a second-factor code.
        SyncTwoFactorRequired,
        /// The second-factor code is wrong.
        SyncTwoFactorInvalid,
        /// This device is signed out of the sync server.
        SyncSignedOut,
        /// Too many requests; wait a moment.
        SyncRateLimited,
        /// The sync account's storage limit is reached.
        SyncQuotaExceeded,
        /// The sync folder cannot be read or written.
        SyncFolderUnavailable,
        /// The chosen folder has other files in it.
        SyncFolderNotEmpty,
        /// The folder has no Mosaic account to join.
        SyncFolderNoAccount,
        /// The folder's account file does not verify.
        SyncFolderDamaged,
        /// The master password can be changed only while the sync server is reachable.
        SyncPasswordNeedsServer,
        /// A device cannot remove itself.
        SyncDeviceIsCurrent,
        /// The chosen folder is no longer remembered; choose it again.
        SyncChoiceExpired,
        /// The password check for this change ran out; start again.
        SyncFreshLoginExpired,
        /// The master password was changed on the server or in the folder.
        SyncPasswordChanged,
        /// The backend offered an older master password than this device had.
        SyncPasswordRollback,
        /// The master password change was sent but its outcome could not be confirmed.
        SyncPasswordOutcomeUnknown,
        /// Sharing and emergency access need the Mosaic server; a folder or
        /// no sync has neither.
        SharingNotHosted,
        /// The contact's safety code has not been compared yet.
        SharingContactUnverified,
        /// The contact's key changed since it was pinned; compare the new code first.
        SharingKeyChanged,
        /// This account's role does not allow that.
        SharingRoleForbidden,
        /// An invite for this person and vault is already waiting.
        SharingInviteExists,
        /// The invite changed after the preview (a new vault key); open it again.
        SharingInviteStale,
        /// The invite ran out or was withdrawn.
        SharingInviteExpired,
        /// The invite cannot be read: it is damaged or not meant for this account.
        SharingInviteUnreadable,
        /// The vault's member list does not match the invite.
        SharingInviteNotInRoster,
        /// The owner cannot leave a vault.
        SharingOwnerCannotLeave,
        /// This person is a member already.
        SharingAlreadyMember,
        /// A removed member can be invited again only after the vault key changed.
        SharingReinviteBlocked,
        /// Too many invites are waiting.
        SharingInviteLimit,
        /// This device already knows that vault; an invite cannot give it a key.
        SharingVaultKnown,
        /// Sync cannot change servers while shared vaults are on this device.
        SyncHasSharedVaults,
        /// An emergency request is waiting; the grant cannot change now.
        EmergencyWaiting,
        /// A refused request can be repeated only after the cooldown.
        EmergencyCooldown,
        /// There is nothing to claim: no request, or no grant.
        EmergencyNotGranted,
        /// The new Emergency Kit was not saved and verified yet.
        KitNotVerified,
        /// The group typed from the new Emergency Kit is not the new Secret Key's.
        KitGroupMismatch,
        /// Another key rotation is open; finish or discard it first.
        RotationInProgress,
        /// There is no key rotation to continue.
        RotationNone,
        /// The change already reached the server: it cannot be discarded.
        RotationTooFar,
        /// This device is too far behind the account's other devices to adopt
        /// a re-key: it has to be set up again (join).
        RotationJumpTooLarge,
        /// The change never reached the backend: nothing was changed. Start
        /// again or discard it.
        RotationNotApplied,
        /// That Secret Key is the current one, or one this device has used before.
        SecretKeyUnchanged,
        /// The new master password is the current one.
        PasswordUnchanged,
        /// The new protection level is weaker than the current one.
        ProfileDowngrade,
        /// This device needs more than two seconds per unlock with that level.
        ProfileTooSlow,
        /// Sync must finish cleanly before the account's keys can be renewed.
        RekeyNeedsCleanSync,
        /// A person's safety code in a shared vault has not been checked, so
        /// the vault cannot be renewed yet.
        RekeyMemberKeyMismatch,
        /// The vaults kept changing; try again.
        RekeyBusy,
        /// The re-key was sent and its outcome is not known yet.
        RekeyOutcomeUnknown,
        /// A re-key needs the sync folder or server to be reachable.
        RekeyNeedsServer,
        /// No account file in the sync folder opens with these keys.
        RekeyNoAccountFile,
        /// The account's keys were renewed on another device; use the new Secret Key.
        SyncAccountRekeyed,
        /// The backend offered older account keys than this device has.
        SyncEpochRollback,
        /// The backend's account keys are not the next ones after this device's.
        SyncRekeyRefused,
        /// The item has no such open password-change attempt (it was closed
        /// on another device or in the browser).
        RotationNotFound,
        /// Four password changes of this item are open already; finish or
        /// close one first.
        RotationLimit,
        /// Mosaic cannot guide the change of this item: it is not a login or
        /// has no `https` URL.
        RotationNotSupported,
        /// The password history has no such entry any more.
        HistoryNotFound,
        /// The page the browser is on does not belong to this login's site.
        OriginNotRelated,
        /// The breach check needs the user's consent first.
        BreachConsentRequired,
        /// A breach check is running already.
        BreachRunning,
        /// Another operation that needs the vault is still running.
        Busy,
        /// An unexpected internal failure; details are not shown.
        Internal,
    }
);

/// A failed command, as the webview receives it: `{ "code": "locked" }`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, thiserror::Error)]
#[error("IPC command failed: {code:?}")]
pub struct IpcError {
    /// What happened.
    pub code: ErrorCode,
}

impl IpcError {
    /// An error with this code.
    #[must_use]
    pub const fn new(code: ErrorCode) -> Self {
        Self { code }
    }

    /// The command is not built yet.
    pub const NOT_IMPLEMENTED: Self = Self::new(ErrorCode::NotImplemented);
    /// The vault is locked.
    pub const LOCKED: Self = Self::new(ErrorCode::Locked);
    /// An unexpected internal failure.
    pub const INTERNAL: Self = Self::new(ErrorCode::Internal);
}

impl From<ErrorCode> for IpcError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

/// The result of every command.
pub type IpcResult<T> = Result<T, IpcError>;

impl From<SessionError> for IpcError {
    fn from(error: SessionError) -> Self {
        let code = match error {
            SessionError::UnlockFailed => ErrorCode::WrongCredentials,
            SessionError::NoAccount => ErrorCode::NoAccount,
            SessionError::UnknownVault => ErrorCode::VaultNotFound,
            SessionError::VaultUnavailable => ErrorCode::VaultUnavailable,
            SessionError::UnknownItem => ErrorCode::ItemNotFound,
            SessionError::ItemDeleted => ErrorCode::ItemDeleted,
            SessionError::SameVault => ErrorCode::SameVault,
            SessionError::NoOtp => ErrorCode::NoOtp,
            SessionError::AttachmentTooLarge => ErrorCode::AttachmentTooLarge,
            SessionError::AttachmentQuota => ErrorCode::AttachmentQuota,
            SessionError::UnknownAttachment => ErrorCode::AttachmentNotFound,
            SessionError::QuickUnlockExpired => ErrorCode::QuickUnlockExpired,
            SessionError::QuickUnlockInvalidated => ErrorCode::QuickUnlockInvalidated,
            SessionError::NoQuickUnlock => ErrorCode::QuickUnlockUnavailable,
            SessionError::PasswordRequired => ErrorCode::PasswordRequired,
            SessionError::ReauthenticationRequired => ErrorCode::ReauthRequired,
            SessionError::UnknownHardwareKey | SessionError::HardwareKeyRejected => {
                ErrorCode::HardwareKeyNotRecognized
            }
            SessionError::HardwareKeyLimit => ErrorCode::HardwareKeyLimit,
            SessionError::LastHardwareKey => ErrorCode::HardwareKeyLast,
            SessionError::InvalidHardwareKey => ErrorCode::HardwareKeyUnsupported,
            SessionError::WrongVaultProtection => ErrorCode::InvalidInput,
            SessionError::ItemTypeNotAllowed => ErrorCode::ItemTypeNotAllowed,
            SessionError::PasswordRollback | SessionError::AccountVersionRollback => {
                ErrorCode::SyncPasswordRollback
            }
            SessionError::PasswordUnchanged => ErrorCode::PasswordUnchanged,
            SessionError::SecretKeyUnchanged => ErrorCode::SecretKeyUnchanged,
            SessionError::RotationInProgress => ErrorCode::RotationInProgress,
            SessionError::RekeyNotClean => ErrorCode::RekeyNeedsCleanSync,
            SessionError::PlanMismatch => ErrorCode::RekeyMemberKeyMismatch,
            SessionError::EpochRollback { .. } => ErrorCode::SyncEpochRollback,
            SessionError::EpochNotNext { .. } => ErrorCode::SyncRekeyRefused,
            SessionError::EpochJumpTooLarge { .. } => ErrorCode::RotationJumpTooLarge,
            SessionError::PreparedStale => ErrorCode::Busy,
            SessionError::ContactUnverified => ErrorCode::SharingContactUnverified,
            SessionError::ContactKeyChanged | SessionError::MemberKeyMismatch => {
                ErrorCode::SharingKeyChanged
            }
            SessionError::RoleForbidden
            | SessionError::NotOwner
            | SessionError::OwnerCannotBeRevoked
            // A viewer of a shared vault cannot change an item either.
            | SessionError::ReadOnly => ErrorCode::SharingRoleForbidden,
            SessionError::InviteUnreadable => ErrorCode::SharingInviteUnreadable,
            SessionError::InviteNotInRoster | SessionError::OwnerMismatch => {
                ErrorCode::SharingInviteNotInRoster
            }
            SessionError::InviteNotOpened => ErrorCode::SharingInviteStale,
            SessionError::AlreadyMember => ErrorCode::SharingAlreadyMember,
            SessionError::ReinviteBlocked => ErrorCode::SharingReinviteBlocked,
            SessionError::VaultKnown => ErrorCode::SharingVaultKnown,
            SessionError::NoPassword => ErrorCode::FieldEmpty,
            SessionError::Store(store) => return store.into(),
            SessionError::Core(core) => return core.into(),
            _ => ErrorCode::Internal,
        };
        Self::new(code)
    }
}

impl From<StoreError> for IpcError {
    fn from(error: StoreError) -> Self {
        Self::new(match error {
            StoreError::StoreInUse => ErrorCode::StoreInUse,
            StoreError::AccountExists => ErrorCode::AccountExists,
            StoreError::NoAccount => ErrorCode::NoAccount,
            StoreError::UnknownVault => ErrorCode::VaultNotFound,
            StoreError::AttachmentTooLarge => ErrorCode::AttachmentTooLarge,
            StoreError::HasSharedVaults => ErrorCode::SyncHasSharedVaults,
            StoreError::RotationInProgress => ErrorCode::RotationInProgress,
            StoreError::NoRotationJob => ErrorCode::RotationNone,
            StoreError::RotationTooFar => ErrorCode::RotationTooFar,
            StoreError::Database(_)
            | StoreError::Io(_)
            | StoreError::InvalidPath
            | StoreError::NotAMosaicStore
            | StoreError::SchemaTooNew { .. }
            | StoreError::Corrupt => ErrorCode::StoreUnavailable,
            _ => ErrorCode::Internal,
        })
    }
}

impl From<mosaic_core::Error> for IpcError {
    fn from(error: mosaic_core::Error) -> Self {
        Self::new(match error {
            mosaic_core::Error::SecretKeyMalformed => ErrorCode::SecretKeyMalformed,
            mosaic_core::Error::SecretKeyMistyped => ErrorCode::SecretKeyMistyped,
            mosaic_core::Error::EmptyPassword => ErrorCode::EmptyPassword,
            mosaic_core::Error::ProfileDowngrade => ErrorCode::ProfileDowngrade,
            mosaic_core::Error::NotALogin => ErrorCode::RotationNotSupported,
            mosaic_core::Error::TooManyAttempts => ErrorCode::RotationLimit,
            mosaic_core::Error::RotationNotFound => ErrorCode::RotationNotFound,
            mosaic_core::Error::HistoryNotFound => ErrorCode::HistoryNotFound,
            _ => ErrorCode::Internal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_serialize_as_a_bare_code() {
        let json = serde_json::to_string(&IpcError::LOCKED).unwrap();
        assert_eq!(json, r#"{"code":"locked"}"#);
        let json = serde_json::to_string(&IpcError::new(ErrorCode::SecretKeyMistyped)).unwrap();
        assert_eq!(json, r#"{"code":"secretKeyMistyped"}"#);
    }

    #[test]
    fn every_code_round_trips_through_its_wire_name() {
        let mut seen = std::collections::HashSet::new();
        for &code in ErrorCode::ALL {
            let json = serde_json::to_string(&IpcError::new(code)).unwrap();
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            let name = value["code"].as_str().unwrap().to_owned();
            assert_eq!(value.as_object().unwrap().len(), 1, "only a code: {json}");
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric()), "{name}");
            assert!(name.starts_with(|c: char| c.is_ascii_lowercase()), "{name}");
            let back: ErrorCode = serde_json::from_value(value["code"].clone()).unwrap();
            assert_eq!(back, code);
            assert!(seen.insert(name), "duplicate wire name for {code:?}");
        }
        assert!(serde_json::from_str::<ErrorCode>("\"nonsense\"").is_err());
    }

    #[test]
    fn display_names_the_code_and_nothing_secret() {
        assert_eq!(IpcError::LOCKED.to_string(), "IPC command failed: Locked");
        assert_eq!(IpcError::from(ErrorCode::Busy).code, ErrorCode::Busy);
        assert_eq!(IpcError::NOT_IMPLEMENTED.code, ErrorCode::NotImplemented);
        assert_eq!(IpcError::INTERNAL.code, ErrorCode::Internal);
    }

    #[test]
    fn session_errors_map_to_the_codes_the_ui_acts_on() {
        let cases = [
            (SessionError::NoAccount, ErrorCode::NoAccount),
            (SessionError::UnknownVault, ErrorCode::VaultNotFound),
            (SessionError::VaultUnavailable, ErrorCode::VaultUnavailable),
            (SessionError::UnknownItem, ErrorCode::ItemNotFound),
            (SessionError::ItemDeleted, ErrorCode::ItemDeleted),
            (SessionError::SameVault, ErrorCode::SameVault),
            (
                SessionError::EpochJumpTooLarge {
                    stored: 1,
                    offered: 40,
                },
                ErrorCode::RotationJumpTooLarge,
            ),
            (SessionError::NoOtp, ErrorCode::NoOtp),
            (
                SessionError::AttachmentTooLarge,
                ErrorCode::AttachmentTooLarge,
            ),
            (SessionError::AttachmentQuota, ErrorCode::AttachmentQuota),
            (
                SessionError::UnknownAttachment,
                ErrorCode::AttachmentNotFound,
            ),
            (
                SessionError::QuickUnlockExpired,
                ErrorCode::QuickUnlockExpired,
            ),
            (
                SessionError::QuickUnlockInvalidated,
                ErrorCode::QuickUnlockInvalidated,
            ),
            (
                SessionError::NoQuickUnlock,
                ErrorCode::QuickUnlockUnavailable,
            ),
            (SessionError::PasswordRequired, ErrorCode::PasswordRequired),
            (
                SessionError::ReauthenticationRequired,
                ErrorCode::ReauthRequired,
            ),
            (
                SessionError::UnknownHardwareKey,
                ErrorCode::HardwareKeyNotRecognized,
            ),
            (
                SessionError::HardwareKeyRejected,
                ErrorCode::HardwareKeyNotRecognized,
            ),
            (SessionError::HardwareKeyLimit, ErrorCode::HardwareKeyLimit),
            (SessionError::LastHardwareKey, ErrorCode::HardwareKeyLast),
            (
                SessionError::InvalidHardwareKey,
                ErrorCode::HardwareKeyUnsupported,
            ),
            (SessionError::WrongVaultProtection, ErrorCode::InvalidInput),
            (
                SessionError::ItemTypeNotAllowed,
                ErrorCode::ItemTypeNotAllowed,
            ),
            (
                SessionError::Core(mosaic_core::Error::Decryption),
                ErrorCode::Internal,
            ),
            (
                SessionError::Core(mosaic_core::Error::EmptyPassword),
                ErrorCode::EmptyPassword,
            ),
            (
                SessionError::Store(StoreError::StoreInUse),
                ErrorCode::StoreInUse,
            ),
        ];
        for (error, code) in cases {
            let shown = format!("{error:?}");
            assert_eq!(IpcError::from(error).code, code, "{shown}");
        }
    }

    #[test]
    fn store_errors_map_and_unreadable_files_all_look_alike() {
        let cases = [
            (StoreError::StoreInUse, ErrorCode::StoreInUse),
            (StoreError::AccountExists, ErrorCode::AccountExists),
            (StoreError::NoAccount, ErrorCode::NoAccount),
            (StoreError::UnknownVault, ErrorCode::VaultNotFound),
            (
                StoreError::AttachmentTooLarge,
                ErrorCode::AttachmentTooLarge,
            ),
            (StoreError::InvalidPath, ErrorCode::StoreUnavailable),
            (StoreError::NotAMosaicStore, ErrorCode::StoreUnavailable),
            (StoreError::Corrupt, ErrorCode::StoreUnavailable),
            (
                StoreError::SchemaTooNew {
                    found: 99,
                    supported: 1,
                },
                ErrorCode::StoreUnavailable,
            ),
            (
                StoreError::Io(std::io::Error::other("secret path")),
                ErrorCode::StoreUnavailable,
            ),
            (StoreError::AccountMismatch, ErrorCode::Internal),
        ];
        for (error, code) in cases {
            let shown = format!("{error:?}");
            assert_eq!(IpcError::from(error).code, code, "{shown}");
        }
    }

    #[test]
    fn core_errors_only_surface_what_the_user_can_fix() {
        let cases = [
            (
                mosaic_core::Error::SecretKeyMalformed,
                ErrorCode::SecretKeyMalformed,
            ),
            (
                mosaic_core::Error::SecretKeyMistyped,
                ErrorCode::SecretKeyMistyped,
            ),
            (mosaic_core::Error::EmptyPassword, ErrorCode::EmptyPassword),
            (mosaic_core::Error::Decryption, ErrorCode::Internal),
            (mosaic_core::Error::Malformed, ErrorCode::Internal),
        ];
        for (error, code) in cases {
            assert_eq!(IpcError::from(error).code, code, "{error}");
        }
    }

    #[test]
    fn wrong_password_and_wrong_secret_key_look_the_same() {
        assert_eq!(
            IpcError::from(SessionError::UnlockFailed).code,
            ErrorCode::WrongCredentials
        );
    }
}
