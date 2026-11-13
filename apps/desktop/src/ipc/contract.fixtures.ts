// Generated from the Rust contract (apps/desktop/src-tauri/src/contract) by
// `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.
// Do not edit. `tsc` checks it against the TypeScript contract (ADR-0018).
import type { ContractFixtures } from "./contract/fixtures.ts";

export const fixtures = {
  "binaryResponses": [
    "attachment_preview"
  ],
  "commands": [
    "session_state",
    "create_account",
    "save_emergency_kit",
    "verify_emergency_kit",
    "unlock",
    "lock",
    "reauthenticate",
    "subscribe_events",
    "app_status",
    "unlock_quick",
    "quick_unlock_status",
    "enroll_biometric",
    "remove_biometric",
    "enroll_hardware_key",
    "remove_hardware_key",
    "hardware_key_cancel",
    "list_vaults",
    "create_vault",
    "rename_vault",
    "create_hardware_vault",
    "open_hardware_vault",
    "close_hardware_vault",
    "hardware_vault_keys",
    "add_hardware_vault_key",
    "remove_hardware_vault_key",
    "list_items",
    "get_item",
    "reveal_secret",
    "add_item",
    "update_item",
    "set_favorite",
    "trash_item",
    "restore_item",
    "purge_item",
    "move_item",
    "attachment_add",
    "attachment_preview",
    "attachment_save",
    "attachment_remove",
    "copy_to_clipboard",
    "clear_clipboard",
    "generate_password",
    "totp_code",
    "otp_scan",
    "otp_parse",
    "otp_pending_code",
    "otp_pending_discard",
    "get_settings",
    "update_settings",
    "security_report",
    "import_choose_file",
    "import_preview",
    "import_commit",
    "import_cancel",
    "export_passkeys_cxf",
    "setup_code_show",
    "ssh_agent_status",
    "ssh_agent_set_enabled",
    "ssh_respond",
    "browser_status",
    "browser_connector_audit",
    "browser_connector_repair",
    "browser_remove_pairing",
    "browser_pairing_respond",
    "browser_save_respond",
    "browser_passkey_respond",
    "browser_rotation_respond",
    "browser_rotation_reveal",
    "sync_status",
    "sync_check_server",
    "sync_hosted_connect",
    "sync_hosted_two_factor",
    "sync_pick_folder",
    "sync_folder_connect",
    "sync_now",
    "sync_disconnect",
    "sync_acknowledge",
    "sync_recover_password",
    "sync_devices",
    "sync_device_rename",
    "sync_device_remove",
    "sync_totp_begin",
    "sync_totp_confirm",
    "sync_totp_disable",
    "sync_login_methods",
    "sync_hosted_webauthn_begin",
    "join_hosted_webauthn_begin",
    "sync_webauthn_status",
    "sync_webauthn_reopen",
    "sync_webauthn_list",
    "sync_webauthn_add_begin",
    "sync_webauthn_add_status",
    "sync_webauthn_confirm_begin",
    "sync_webauthn_remove",
    "join_hosted",
    "join_hosted_two_factor",
    "join_folder",
    "change_master_password",
    "sync_dismiss_account_file",
    "rotation_abandon_committed",
    "sync_rekey_markers",
    "sync_dismiss_rekey_marker",
    "sync_ignored_rekey_markers",
    "sync_restore_rekey_marker",
    "rotation_status",
    "secret_key_rotate_prepare",
    "secret_key_rotate_commit",
    "profile_upgrade",
    "rekey_review",
    "rekey_prepare",
    "rekey_commit",
    "rotation_resume",
    "rotation_abandon",
    "vault_rotate_key",
    "rotation_queue",
    "rotation_begin",
    "rotation_close",
    "rotation_reveal",
    "rotation_copy",
    "password_history_restore",
    "rotation_snooze",
    "rotation_dismiss",
    "rotation_undismiss",
    "rotation_set_policy",
    "breach_scan_start",
    "breach_scan_cancel",
    "sharing_identity",
    "sharing_lookup",
    "sharing_verify_contact",
    "sharing_accept_key_change",
    "sharing_contacts",
    "vault_share",
    "vault_members",
    "vault_member_set_role",
    "vault_member_remove",
    "vault_invite_cancel",
    "vault_leave",
    "invites_list",
    "invite_preview",
    "invite_accept",
    "invite_decline",
    "emergency_overview",
    "emergency_grant",
    "emergency_revoke",
    "emergency_request",
    "emergency_respond",
    "emergency_claim"
  ],
  "enums": {
    "attemptStage": [
      "prepared",
      "delivered"
    ],
    "biometricPlatformKind": [
      "windowsHello",
      "touchId"
    ],
    "breachMode": [
      "off",
      "manual",
      "weekly"
    ],
    "breachScanError": [
      "offline",
      "rateLimited",
      "protocol",
      "cancelled"
    ],
    "browserCloseReason": [
      "answered",
      "expired",
      "conflict",
      "locked",
      "disconnected",
      "abortedByPage"
    ],
    "browserEndpointState": [
      "running",
      "inUse",
      "unavailable"
    ],
    "browserFamily": [
      "chromium",
      "firefox"
    ],
    "browserHint": [
      "chrome",
      "edge",
      "brave",
      "opera",
      "vivaldi",
      "chromium",
      "firefox",
      "other"
    ],
    "browserPasskeyKind": [
      "create",
      "get"
    ],
    "browserRegistryState": [
      "ok",
      "unreadable"
    ],
    "browserRotationKind": [
      "begin",
      "result",
      "addOrigin"
    ],
    "browserSaveKind": [
      "new",
      "update"
    ],
    "changeUrlSource": [
      "stored",
      "quirks",
      "wellKnown"
    ],
    "contactStatus": [
      "new",
      "pinned",
      "verified",
      "changed",
      "conflicting"
    ],
    "customFieldKind": [
      "text",
      "hidden",
      "url",
      "email",
      "phone",
      "date",
      "otp"
    ],
    "devicePlatform": [
      "windows",
      "macos",
      "linux"
    ],
    "emergencyDecision": [
      "decline",
      "approve"
    ],
    "emergencyState": [
      "idle",
      "waiting",
      "approved",
      "released"
    ],
    "errorCode": [
      "notImplemented",
      "noAccount",
      "accountExists",
      "locked",
      "alreadyUnlocked",
      "wrongCredentials",
      "secretKeyRequired",
      "secretKeyMalformed",
      "secretKeyMistyped",
      "emptyPassword",
      "quickUnlockUnavailable",
      "cancelled",
      "quickUnlockExpired",
      "quickUnlockInvalidated",
      "passwordRequired",
      "hardwareKeyUnsupported",
      "hardwareKeyNotFound",
      "hardwareKeyTimeout",
      "hardwareKeyPinRequired",
      "hardwareKeyPinInvalid",
      "hardwareKeyPinBlocked",
      "hardwareKeyNoPin",
      "hardwareKeyNotRecognized",
      "hardwareKeyLimit",
      "hardwareKeyLast",
      "itemTypeNotAllowed",
      "otpScanUnsupported",
      "otpQrNotFound",
      "otpQrNotOtp",
      "otpQrMigration",
      "otpQrMultiple",
      "otpQrInvalid",
      "otpUriInvalid",
      "otpKeyInvalid",
      "otpImageUnreadable",
      "otpImageTooLarge",
      "otpClipboardNoImage",
      "otpPendingExpired",
      "storeInUse",
      "storeUnavailable",
      "vaultNotFound",
      "vaultUnavailable",
      "itemNotFound",
      "itemDeleted",
      "sameVault",
      "noOtp",
      "fieldEmpty",
      "invalidInput",
      "sshKeyInvalid",
      "reauthRequired",
      "attachmentTooLarge",
      "attachmentQuota",
      "attachmentNotFound",
      "attachmentUnavailable",
      "attachmentFileUnreadable",
      "attachmentSaveFailed",
      "attachmentPreviewUnsupported",
      "attachmentPreviewTooLarge",
      "clipboardUnavailable",
      "importFileUnreadable",
      "importFormatInvalid",
      "importPreviewExpired",
      "importTooLarge",
      "importUnsupported",
      "importEncrypted",
      "sshRequestExpired",
      "sshAgentUnavailable",
      "sshAgentEndpointInUse",
      "unlockDelayed",
      "tooManyAttempts",
      "sshAllowTooSoon",
      "browserRequestExpired",
      "browserAllowTooSoon",
      "browserPairingNotFound",
      "browserHostMissing",
      "syncNotConfigured",
      "syncAlreadyConfigured",
      "syncServerUnreachable",
      "syncServerInvalid",
      "syncServerInsecure",
      "syncServerMismatch",
      "syncServerIncompatible",
      "syncRegistrationClosed",
      "syncInviteInvalid",
      "syncAccountExists",
      "syncAccountMismatch",
      "syncTwoFactorRequired",
      "syncTwoFactorInvalid",
      "syncSignedOut",
      "syncRateLimited",
      "syncQuotaExceeded",
      "syncFolderUnavailable",
      "syncFolderNotEmpty",
      "syncFolderNoAccount",
      "syncFolderDamaged",
      "syncPasswordNeedsServer",
      "syncDeviceIsCurrent",
      "syncChoiceExpired",
      "syncFreshLoginExpired",
      "syncPasswordChanged",
      "syncPasswordRollback",
      "syncPasswordOutcomeUnknown",
      "syncWebauthnUnavailable",
      "syncWebauthnLimit",
      "syncHandoffExpired",
      "syncHandoffFailed",
      "shortcutUnavailable",
      "shortcutUnsupported",
      "sharingNotHosted",
      "setupCodeNeedsServer",
      "sharingContactUnverified",
      "sharingKeyChanged",
      "sharingRoleForbidden",
      "sharingInviteExists",
      "sharingInviteStale",
      "sharingInviteExpired",
      "sharingInviteUnreadable",
      "sharingInviteNotInRoster",
      "sharingOwnerCannotLeave",
      "sharingAlreadyMember",
      "sharingReinviteBlocked",
      "sharingInviteLimit",
      "sharingVaultKnown",
      "syncHasSharedVaults",
      "emergencyWaiting",
      "emergencyCooldown",
      "emergencyNotGranted",
      "kitNotVerified",
      "kitGroupMismatch",
      "rotationInProgress",
      "rotationNone",
      "rotationTooFar",
      "rotationJumpTooLarge",
      "rotationNotApplied",
      "secretKeyUnchanged",
      "passwordUnchanged",
      "profileDowngrade",
      "profileTooSlow",
      "rekeyNeedsCleanSync",
      "rekeyMemberKeyMismatch",
      "rekeyBusy",
      "rekeyOutcomeUnknown",
      "rekeyNeedsServer",
      "rekeyNoAccountFile",
      "syncAccountRekeyed",
      "syncEpochRollback",
      "syncRekeyRefused",
      "rotationNotFound",
      "rotationLimit",
      "rotationOpen",
      "rotationNotSupported",
      "historyNotFound",
      "originNotRelated",
      "breachConsentRequired",
      "breachRunning",
      "busy",
      "internal"
    ],
    "folderState": [
      "empty",
      "mosaic",
      "other"
    ],
    "handoffState": [
      "waiting",
      "done",
      "failed",
      "expired"
    ],
    "handoffStep": [
      "login",
      "register",
      "confirm"
    ],
    "historyReasonKey": [
      "replaced",
      "unconfirmed"
    ],
    "hostedMode": [
      "register",
      "signIn"
    ],
    "importEntryWarning": [
      "unmappedFields",
      "otpUnreadable",
      "attachmentTooLarge",
      "attachmentMissing",
      "tagsDropped",
      "historyTruncated",
      "passkeysNotImported",
      "unknownCategory",
      "fieldsDropped",
      "archived",
      "attachmentQuota",
      "titleCleaned",
      "passkeyUnreadable",
      "passkeyUnsupported",
      "passkeysLimit"
    ],
    "importFormat": [
      "enpass",
      "bitwarden",
      "keepass",
      "onePassword",
      "cxf",
      "csv"
    ],
    "importSkipReason": [
      "malformed",
      "empty",
      "tooLarge"
    ],
    "importWarning": [
      "unmappedFields",
      "attachmentsSkipped",
      "entriesSkipped",
      "otpUnreadable",
      "passkeysNotImported",
      "passkeyUnreadable",
      "passkeyUnsupported",
      "passkeysSkipped"
    ],
    "insightAction": [
      "changePassword",
      "setUpTotp",
      "reviewUrl"
    ],
    "insightKind": [
      "weak",
      "reused",
      "old",
      "missingTotp",
      "insecureUrl",
      "breached"
    ],
    "insightSeverity": [
      "low",
      "medium",
      "high"
    ],
    "itemKind": [
      "login",
      "passkey",
      "totp",
      "secureNote",
      "creditCard",
      "identity",
      "sshKey",
      "apiKey",
      "wifi",
      "document",
      "unsupported"
    ],
    "language": [
      "system",
      "en",
      "de"
    ],
    "lockReason": [
      "manual",
      "idle",
      "sleep",
      "screenLock"
    ],
    "memberOriginKind": [
      "owner",
      "invite",
      "emergency"
    ],
    "memberRowState": [
      "member",
      "invited",
      "unconfirmed",
      "dropped",
      "removing"
    ],
    "otpAlgorithm": [
      "sha1",
      "sha256",
      "sha512"
    ],
    "otpScanSource": [
      "screen",
      "clipboard",
      "file"
    ],
    "otpScheme": [
      "totp",
      "hotp",
      "steam"
    ],
    "otpSecretStrength": [
      "weak",
      "belowRfc4226",
      "rfc4226"
    ],
    "pairingDecision": [
      "allow",
      "deny"
    ],
    "passkeyAlgorithm": [
      "es256",
      "other"
    ],
    "passkeyVerification": [
      "required",
      "preferred",
      "discouraged"
    ],
    "queueKind": [
      "guided",
      "manual"
    ],
    "queueReason": [
      "weak",
      "reused",
      "old",
      "breached",
      "unfinished"
    ],
    "quickUnlockKind": [
      "biometric",
      "hardwareKey"
    ],
    "registrationMode": [
      "open",
      "invite",
      "closed"
    ],
    "rekeyBackend": [
      "local",
      "hosted",
      "folder"
    ],
    "revokedReason": [
      "rekey",
      "passwordChange",
      "deviceRemoved",
      "operator"
    ],
    "rotationKind": [
      "secretKey",
      "rekey",
      "profile",
      "vault"
    ],
    "rotationNeeds": [
      "none",
      "password",
      "kitGroup"
    ],
    "rotationOutcomeKey": [
      "succeeded",
      "failed"
    ],
    "rotationPhaseName": [
      "prepared",
      "backendStarted",
      "backendAccepted",
      "folderWritten",
      "localCommitted",
      "attachments",
      "uploading"
    ],
    "rotationProfile": [
      "standard",
      "strong",
      "max"
    ],
    "rulesSourceKey": [
      "siteQuirks"
    ],
    "secretFieldKey": [
      "password",
      "cardNumber",
      "cardVerificationCode",
      "cardPin",
      "passportNumber",
      "licenseNumber",
      "nationalId",
      "apiKeySecret",
      "wifiPassword"
    ],
    "shareRole": [
      "owner",
      "manage",
      "write",
      "read"
    ],
    "shortcutState": [
      "registered",
      "conflict",
      "unavailable",
      "unsupported"
    ],
    "sshCloseReason": [
      "answered",
      "expired",
      "locked"
    ],
    "sshDecision": [
      "allowOnce",
      "allowFor15Minutes",
      "deny"
    ],
    "sshSignFailReason": [
      "passphraseRequired",
      "wrongPassphrase",
      "keyMalformed",
      "kdfTooExpensive",
      "publicKeyMismatch",
      "unsupported",
      "internal"
    ],
    "sshSkipReason": [
      "unsupported",
      "malformed",
      "duplicate"
    ],
    "strength": [
      "veryWeak",
      "weak",
      "fair",
      "strong",
      "veryStrong"
    ],
    "syncStatus": [
      "notConfigured",
      "offline",
      "syncing",
      "upToDate",
      "error",
      "signedOut"
    ],
    "textFieldKey": [
      "title",
      "notes",
      "username",
      "cardholderName",
      "cardBrand",
      "cardExpiry",
      "firstName",
      "middleName",
      "lastName",
      "email",
      "phone",
      "company",
      "addressLine1",
      "addressLine2",
      "city",
      "region",
      "postalCode",
      "country",
      "birthDate",
      "apiKeyId",
      "apiKeyHost",
      "wifiSsid",
      "wifiSecurity",
      "sshComment",
      "changePasswordUrl"
    ],
    "theme": [
      "system",
      "light",
      "dark"
    ],
    "trashFilter": [
      "exclude",
      "only",
      "include"
    ],
    "urlMatchMode": [
      "exactOrigin",
      "etld1"
    ],
    "vaultMetaField": [
      "name",
      "icon",
      "description"
    ],
    "vaultProtection": [
      "accountKey",
      "hardwareKey"
    ]
  },
  "errors": [
    {
      "code": "notImplemented"
    },
    {
      "code": "noAccount"
    },
    {
      "code": "accountExists"
    },
    {
      "code": "locked"
    },
    {
      "code": "alreadyUnlocked"
    },
    {
      "code": "wrongCredentials"
    },
    {
      "code": "secretKeyRequired"
    },
    {
      "code": "secretKeyMalformed"
    },
    {
      "code": "secretKeyMistyped"
    },
    {
      "code": "emptyPassword"
    },
    {
      "code": "quickUnlockUnavailable"
    },
    {
      "code": "cancelled"
    },
    {
      "code": "quickUnlockExpired"
    },
    {
      "code": "quickUnlockInvalidated"
    },
    {
      "code": "passwordRequired"
    },
    {
      "code": "hardwareKeyUnsupported"
    },
    {
      "code": "hardwareKeyNotFound"
    },
    {
      "code": "hardwareKeyTimeout"
    },
    {
      "code": "hardwareKeyPinRequired"
    },
    {
      "code": "hardwareKeyPinInvalid"
    },
    {
      "code": "hardwareKeyPinBlocked"
    },
    {
      "code": "hardwareKeyNoPin"
    },
    {
      "code": "hardwareKeyNotRecognized"
    },
    {
      "code": "hardwareKeyLimit"
    },
    {
      "code": "hardwareKeyLast"
    },
    {
      "code": "itemTypeNotAllowed"
    },
    {
      "code": "otpScanUnsupported"
    },
    {
      "code": "otpQrNotFound"
    },
    {
      "code": "otpQrNotOtp"
    },
    {
      "code": "otpQrMigration"
    },
    {
      "code": "otpQrMultiple"
    },
    {
      "code": "otpQrInvalid"
    },
    {
      "code": "otpUriInvalid"
    },
    {
      "code": "otpKeyInvalid"
    },
    {
      "code": "otpImageUnreadable"
    },
    {
      "code": "otpImageTooLarge"
    },
    {
      "code": "otpClipboardNoImage"
    },
    {
      "code": "otpPendingExpired"
    },
    {
      "code": "storeInUse"
    },
    {
      "code": "storeUnavailable"
    },
    {
      "code": "vaultNotFound"
    },
    {
      "code": "vaultUnavailable"
    },
    {
      "code": "itemNotFound"
    },
    {
      "code": "itemDeleted"
    },
    {
      "code": "sameVault"
    },
    {
      "code": "noOtp"
    },
    {
      "code": "fieldEmpty"
    },
    {
      "code": "invalidInput"
    },
    {
      "code": "sshKeyInvalid"
    },
    {
      "code": "reauthRequired"
    },
    {
      "code": "attachmentTooLarge"
    },
    {
      "code": "attachmentQuota"
    },
    {
      "code": "attachmentNotFound"
    },
    {
      "code": "attachmentUnavailable"
    },
    {
      "code": "attachmentFileUnreadable"
    },
    {
      "code": "attachmentSaveFailed"
    },
    {
      "code": "attachmentPreviewUnsupported"
    },
    {
      "code": "attachmentPreviewTooLarge"
    },
    {
      "code": "clipboardUnavailable"
    },
    {
      "code": "importFileUnreadable"
    },
    {
      "code": "importFormatInvalid"
    },
    {
      "code": "importPreviewExpired"
    },
    {
      "code": "importTooLarge"
    },
    {
      "code": "importUnsupported"
    },
    {
      "code": "importEncrypted"
    },
    {
      "code": "sshRequestExpired"
    },
    {
      "code": "sshAgentUnavailable"
    },
    {
      "code": "sshAgentEndpointInUse"
    },
    {
      "code": "unlockDelayed"
    },
    {
      "code": "tooManyAttempts"
    },
    {
      "code": "sshAllowTooSoon"
    },
    {
      "code": "browserRequestExpired"
    },
    {
      "code": "browserAllowTooSoon"
    },
    {
      "code": "browserPairingNotFound"
    },
    {
      "code": "browserHostMissing"
    },
    {
      "code": "syncNotConfigured"
    },
    {
      "code": "syncAlreadyConfigured"
    },
    {
      "code": "syncServerUnreachable"
    },
    {
      "code": "syncServerInvalid"
    },
    {
      "code": "syncServerInsecure"
    },
    {
      "code": "syncServerMismatch"
    },
    {
      "code": "syncServerIncompatible"
    },
    {
      "code": "syncRegistrationClosed"
    },
    {
      "code": "syncInviteInvalid"
    },
    {
      "code": "syncAccountExists"
    },
    {
      "code": "syncAccountMismatch"
    },
    {
      "code": "syncTwoFactorRequired"
    },
    {
      "code": "syncTwoFactorInvalid"
    },
    {
      "code": "syncSignedOut"
    },
    {
      "code": "syncRateLimited"
    },
    {
      "code": "syncQuotaExceeded"
    },
    {
      "code": "syncFolderUnavailable"
    },
    {
      "code": "syncFolderNotEmpty"
    },
    {
      "code": "syncFolderNoAccount"
    },
    {
      "code": "syncFolderDamaged"
    },
    {
      "code": "syncPasswordNeedsServer"
    },
    {
      "code": "syncDeviceIsCurrent"
    },
    {
      "code": "syncChoiceExpired"
    },
    {
      "code": "syncFreshLoginExpired"
    },
    {
      "code": "syncPasswordChanged"
    },
    {
      "code": "syncPasswordRollback"
    },
    {
      "code": "syncPasswordOutcomeUnknown"
    },
    {
      "code": "syncWebauthnUnavailable"
    },
    {
      "code": "syncWebauthnLimit"
    },
    {
      "code": "syncHandoffExpired"
    },
    {
      "code": "syncHandoffFailed"
    },
    {
      "code": "shortcutUnavailable"
    },
    {
      "code": "shortcutUnsupported"
    },
    {
      "code": "sharingNotHosted"
    },
    {
      "code": "setupCodeNeedsServer"
    },
    {
      "code": "sharingContactUnverified"
    },
    {
      "code": "sharingKeyChanged"
    },
    {
      "code": "sharingRoleForbidden"
    },
    {
      "code": "sharingInviteExists"
    },
    {
      "code": "sharingInviteStale"
    },
    {
      "code": "sharingInviteExpired"
    },
    {
      "code": "sharingInviteUnreadable"
    },
    {
      "code": "sharingInviteNotInRoster"
    },
    {
      "code": "sharingOwnerCannotLeave"
    },
    {
      "code": "sharingAlreadyMember"
    },
    {
      "code": "sharingReinviteBlocked"
    },
    {
      "code": "sharingInviteLimit"
    },
    {
      "code": "sharingVaultKnown"
    },
    {
      "code": "syncHasSharedVaults"
    },
    {
      "code": "emergencyWaiting"
    },
    {
      "code": "emergencyCooldown"
    },
    {
      "code": "emergencyNotGranted"
    },
    {
      "code": "kitNotVerified"
    },
    {
      "code": "kitGroupMismatch"
    },
    {
      "code": "rotationInProgress"
    },
    {
      "code": "rotationNone"
    },
    {
      "code": "rotationTooFar"
    },
    {
      "code": "rotationJumpTooLarge"
    },
    {
      "code": "rotationNotApplied"
    },
    {
      "code": "secretKeyUnchanged"
    },
    {
      "code": "passwordUnchanged"
    },
    {
      "code": "profileDowngrade"
    },
    {
      "code": "profileTooSlow"
    },
    {
      "code": "rekeyNeedsCleanSync"
    },
    {
      "code": "rekeyMemberKeyMismatch"
    },
    {
      "code": "rekeyBusy"
    },
    {
      "code": "rekeyOutcomeUnknown"
    },
    {
      "code": "rekeyNeedsServer"
    },
    {
      "code": "rekeyNoAccountFile"
    },
    {
      "code": "syncAccountRekeyed"
    },
    {
      "code": "syncEpochRollback"
    },
    {
      "code": "syncRekeyRefused"
    },
    {
      "code": "rotationNotFound"
    },
    {
      "code": "rotationLimit"
    },
    {
      "code": "rotationOpen"
    },
    {
      "code": "rotationNotSupported"
    },
    {
      "code": "historyNotFound"
    },
    {
      "code": "originNotRelated"
    },
    {
      "code": "breachConsentRequired"
    },
    {
      "code": "breachRunning"
    },
    {
      "code": "busy"
    },
    {
      "code": "internal"
    }
  ],
  "events": [
    {
      "reason": "idle",
      "type": "locked"
    },
    {
      "type": "clipboardCleared"
    },
    {
      "request": {
        "canRemember": true,
        "client": {
          "pid": 4242,
          "processName": "git"
        },
        "expiresAt": 1761480060000,
        "fingerprint": "SHA256:uNiVztksCsDhcc0u9e8BujQXVUpKZIDTMczCvj3tD2s",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000003",
        "keyTitle": "Deploy key",
        "requestId": "ssh-1",
        "requestedAt": 1761480035000,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      "type": "sshApprovalRequested"
    },
    {
      "reason": "expired",
      "requestId": "ssh-1",
      "type": "sshApprovalClosed"
    },
    {
      "itemId": "019a2b3c-4d5e-7020-8000-000000000003",
      "reason": "wrongPassphrase",
      "requestId": "ssh-1",
      "type": "sshSignFailed",
      "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
    },
    {
      "type": "quickSearchRequested"
    },
    {
      "type": "hardwareVaultClosed",
      "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
    },
    {
      "type": "settingsReset"
    },
    {
      "request": {
        "browser": "chromium",
        "code": "4F7K2Q",
        "expiresAt": 1761480155000,
        "hint": "edge",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000
      },
      "type": "browserPairingRequested"
    },
    {
      "reason": "answered",
      "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
      "type": "browserPairingClosed"
    },
    {
      "request": {
        "candidates": [
          {
            "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
            "title": "Example",
            "username": "alex@example.com",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ],
        "expiresAt": 1761480335000,
        "host": "example.com",
        "insecure": false,
        "kind": "update",
        "origin": "https://example.com",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "suggestedTitle": "example.com",
        "username": "alex@example.com"
      },
      "type": "browserSaveRequested"
    },
    {
      "request": {
        "candidates": [],
        "expiresAt": 1761480335000,
        "host": "example.com",
        "insecure": true,
        "kind": "new",
        "origin": "http://example.com",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "suggestedTitle": "example.com",
        "username": null
      },
      "type": "browserSaveRequested"
    },
    {
      "reason": "locked",
      "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
      "type": "browserSaveClosed"
    },
    {
      "request": {
        "candidates": [
          {
            "itemId": "019a2b3c-4d5e-7020-8000-000000000005",
            "userDisplayName": "Alex Example",
            "userName": "alex@example.com",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Personal"
          },
          {
            "itemId": "019a2b3c-4d5e-7020-8000-000000000006",
            "userDisplayName": "",
            "userName": "alex.work",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "excluded": false,
        "expiresAt": 1761480335000,
        "kind": "get",
        "needsPassword": true,
        "origin": "https://example.com",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "rpId": "example.com",
        "userDisplayName": null,
        "userName": null,
        "userVerification": "required"
      },
      "type": "browserPasskeyRequested"
    },
    {
      "request": {
        "candidates": [],
        "excluded": false,
        "expiresAt": 1761480335000,
        "kind": "create",
        "needsPassword": false,
        "origin": "https://example.com",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "rpId": "example.com",
        "userDisplayName": "Alex Example",
        "userName": "alex@example.com",
        "userVerification": "preferred"
      },
      "type": "browserPasskeyRequested"
    },
    {
      "request": {
        "candidates": [],
        "excluded": true,
        "expiresAt": 1761480335000,
        "kind": "create",
        "needsPassword": false,
        "origin": "https://example.com",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "rpId": "example.com",
        "userDisplayName": "Alex Example",
        "userName": "alex@example.com",
        "userVerification": "discouraged"
      },
      "type": "browserPasskeyRequested"
    },
    {
      "reason": "abortedByPage",
      "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
      "type": "browserPasskeyClosed"
    },
    {
      "request": {
        "attempt": null,
        "expiresAt": 1761480335000,
        "host": "example.com",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
        "kind": "begin",
        "newOrigin": null,
        "outcome": null,
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "title": "Example",
        "unicodeHost": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      "type": "browserRotationRequested"
    },
    {
      "request": {
        "attempt": {
          "otherOpen": 1,
          "stage": "delivered",
          "startedAt": 1761480000000
        },
        "expiresAt": 1761480335000,
        "host": "example.com",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
        "kind": "result",
        "newOrigin": null,
        "outcome": "succeeded",
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "title": "Example",
        "unicodeHost": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      "type": "browserRotationRequested"
    },
    {
      "request": {
        "attempt": null,
        "expiresAt": 1761480335000,
        "host": "xn--bcher-kva.example.com",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
        "kind": "addOrigin",
        "newOrigin": "https://xn--bcher-kva.example.com",
        "outcome": null,
        "pairingLabel": "Chrome",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        "requestedAt": 1761480035000,
        "title": "Example",
        "unicodeHost": "bücher.example.com",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      "type": "browserRotationRequested"
    },
    {
      "reason": "expired",
      "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
      "type": "browserRotationClosed"
    },
    {
      "type": "browserPairingsChanged"
    },
    {
      "emergencyRequests": 0,
      "invites": 1,
      "type": "sharingChanged"
    },
    {
      "emergencyRequests": 2,
      "invites": 0,
      "type": "sharingChanged"
    },
    {
      "done": 3,
      "kind": "rekey",
      "phase": "attachments",
      "total": 12,
      "type": "rotationProgress"
    },
    {
      "fields": [
        "name",
        "description"
      ],
      "type": "vaultMetaDropped",
      "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
    },
    {
      "done": 4,
      "total": 9,
      "type": "breachScanProgress"
    },
    {
      "checked": 12,
      "error": null,
      "found": 1,
      "type": "breachScanFinished"
    },
    {
      "checked": 5,
      "error": "cancelled",
      "found": 0,
      "type": "breachScanFinished"
    },
    {
      "state": {
        "error": "internal",
        "lastSyncedAt": 1761480000000,
        "status": "error"
      },
      "type": "syncStateChanged"
    },
    {
      "state": {
        "error": null,
        "lastSyncedAt": null,
        "status": "notConfigured"
      },
      "type": "syncStateChanged"
    },
    {
      "type": "syncDataChanged",
      "vaultIds": [
        "019a2b3c-4d5e-7010-8000-000000000001",
        "019a2b3c-4d5e-7010-8000-000000000002"
      ]
    },
    {
      "conflicts": 2,
      "replacedUnreadable": 0,
      "restored": 1,
      "type": "syncAttention"
    }
  ],
  "requests": {
    "add_hardware_vault_key": [
      {
        "pin": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "add_item": [
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "kind": "hidden",
              "label": "PIN code",
              "op": "update",
              "value": null
            },
            {
              "elementId": "7-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "set",
            "uri": "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP&issuer=GitHub"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "sshKey": {
            "op": "set",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
          },
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "kind": "login",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "app_status": [
      null
    ],
    "attachment_add": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "attachment_preview": [
      {
        "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "attachment_remove": [
      {
        "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "attachment_save": [
      {
        "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "breach_scan_cancel": [
      null
    ],
    "breach_scan_start": [
      null
    ],
    "browser_connector_audit": [
      null
    ],
    "browser_connector_repair": [
      null
    ],
    "browser_pairing_respond": [
      {
        "decision": "allow",
        "label": "Work laptop",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": "deny",
        "label": null,
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      }
    ],
    "browser_passkey_respond": [
      {
        "decision": {
          "kind": "create",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "password": null,
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "itemId": "019a2b3c-4d5e-7020-8000-000000000005",
          "kind": "assert",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "password": "correct horse battery staple",
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "useAnotherDevice"
        },
        "password": null,
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "cancel"
        },
        "password": null,
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "acknowledge"
        },
        "password": null,
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      }
    ],
    "browser_remove_pairing": [
      {
        "pairingId": "0123456789abcdef0123456789abcdef"
      }
    ],
    "browser_rotation_respond": [
      {
        "decision": {
          "kind": "allow"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "regenerate"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "confirm"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "keepOld"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "addOrigin"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "cancel"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      }
    ],
    "browser_rotation_reveal": [
      {
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      }
    ],
    "browser_save_respond": [
      {
        "decision": {
          "allowHttp": false,
          "kind": "saveNew",
          "title": "Example",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
          "kind": "update",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      },
      {
        "decision": {
          "kind": "cancel"
        },
        "requestId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
      }
    ],
    "browser_status": [
      null
    ],
    "change_master_password": [
      {
        "code": null,
        "currentPassword": "correct horse battery staple",
        "newPassword": "a new master password"
      },
      {
        "code": "492039",
        "currentPassword": "correct horse battery staple",
        "newPassword": "a new master password"
      }
    ],
    "clear_clipboard": [
      null
    ],
    "close_hardware_vault": [
      {
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "copy_to_clipboard": [
      {
        "source": {
          "field": {
            "kind": "username"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "kind": "password"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "kind": "totp"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "field": "email",
            "kind": "text"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "field": "cardNumber",
            "kind": "secret"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "customField"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "kind": "sshPublicKey"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "kind": "generated"
        }
      },
      {
        "source": {
          "kind": "pendingOtp",
          "pendingId": "otp-1"
        }
      }
    ],
    "create_account": [
      {
        "defaultVaultName": "Personal",
        "email": "alex@example.com",
        "password": "correct horse battery staple"
      }
    ],
    "create_hardware_vault": [
      {
        "name": "Authenticators",
        "pin": "246810"
      }
    ],
    "create_vault": [
      {
        "name": "Work"
      }
    ],
    "emergency_claim": [
      {
        "grantorAccountId": "019a2b3c-4d5e-7030-8000-000000000004"
      }
    ],
    "emergency_grant": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "code": null,
        "password": "correct horse battery staple",
        "scope": {
          "kind": "all"
        },
        "waitDays": 7
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "code": "492039",
        "password": "correct horse battery staple",
        "scope": {
          "kind": "vaults",
          "vaultIds": [
            "019a2b3c-4d5e-7010-8000-000000000001",
            "019a2b3c-4d5e-7010-8000-000000000002"
          ]
        },
        "waitDays": 30
      }
    ],
    "emergency_overview": [
      null
    ],
    "emergency_request": [
      {
        "grantorAccountId": "019a2b3c-4d5e-7030-8000-000000000004"
      }
    ],
    "emergency_respond": [
      {
        "code": null,
        "decision": "decline",
        "granteeAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "password": null
      },
      {
        "code": "492039",
        "decision": "approve",
        "granteeAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "password": "correct horse battery staple"
      }
    ],
    "emergency_revoke": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002"
      }
    ],
    "enroll_biometric": [
      {
        "password": null
      },
      {
        "password": "correct horse battery staple"
      }
    ],
    "enroll_hardware_key": [
      {
        "password": null,
        "pin": null
      },
      {
        "password": "correct horse battery staple",
        "pin": "246810"
      }
    ],
    "export_passkeys_cxf": [
      {
        "password": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "generate_password": [
      {
        "digits": true,
        "excludeAmbiguous": false,
        "kind": "password",
        "length": 20,
        "lowercase": true,
        "symbols": true,
        "uppercase": true
      },
      {
        "appendDigit": false,
        "capitalize": true,
        "kind": "passphrase",
        "separator": "-",
        "words": 5
      },
      {
        "host": "example.com",
        "kind": "forSite"
      }
    ],
    "get_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "get_settings": [
      null
    ],
    "hardware_key_cancel": [
      null
    ],
    "hardware_vault_keys": [
      {
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "import_cancel": [
      {
        "previewId": "p-1"
      }
    ],
    "import_choose_file": [
      {
        "format": "enpass"
      },
      {
        "format": "bitwarden"
      },
      {
        "format": "keepass"
      },
      {
        "format": "onePassword"
      },
      {
        "format": "cxf"
      },
      {
        "format": "csv"
      }
    ],
    "import_commit": [
      {
        "previewId": "p-1",
        "skip": [
          0
        ],
        "targetVaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "import_preview": [
      {
        "fileToken": "f-1",
        "format": "keepass",
        "password": "kdbx password"
      },
      {
        "fileToken": "f-1",
        "format": "enpass",
        "password": null
      }
    ],
    "invite_accept": [
      {
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000002",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V"
      },
      {
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000002",
        "safetyCode": null
      }
    ],
    "invite_decline": [
      {
        "block": false,
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000002"
      },
      {
        "block": true,
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000002"
      }
    ],
    "invite_preview": [
      {
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000002"
      }
    ],
    "invites_list": [
      null
    ],
    "join_folder": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "choiceId": "choice-1",
        "deviceName": "Mosaic on Windows",
        "password": "correct horse battery staple",
        "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
      }
    ],
    "join_hosted": [
      {
        "deviceName": "Mosaic on Windows",
        "email": "alex@example.com",
        "password": "correct horse battery staple",
        "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2",
        "serverUrl": "https://sync.example.com"
      }
    ],
    "join_hosted_two_factor": [
      {
        "code": "492039"
      }
    ],
    "join_hosted_webauthn_begin": [
      null
    ],
    "list_items": [
      {
        "favoritesOnly": true,
        "query": "git",
        "trash": "exclude",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "favoritesOnly": false,
        "query": null,
        "trash": "only",
        "vaultId": null
      }
    ],
    "list_vaults": [
      null
    ],
    "lock": [
      null
    ],
    "move_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "targetVaultId": "019a2b3c-4d5e-7010-8000-000000000002",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "open_hardware_vault": [
      {
        "pin": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "otp_parse": [
      {
        "kind": "uri",
        "uri": "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP&issuer=GitHub"
      },
      {
        "accountName": null,
        "algorithm": "sha1",
        "digits": 6,
        "issuer": "GitHub",
        "kind": "key",
        "period": 30,
        "secret": "JBSW Y3DP EHPK 3PXP"
      }
    ],
    "otp_pending_code": [
      {
        "pendingId": "otp-1"
      }
    ],
    "otp_pending_discard": [
      {
        "pendingId": "otp-1"
      }
    ],
    "otp_scan": [
      {
        "source": "screen"
      },
      {
        "source": "clipboard"
      },
      {
        "source": "file"
      }
    ],
    "password_history_restore": [
      {
        "historyId": "00000000000000a2.0123456789abcdef.0",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "profile_upgrade": [
      {
        "code": null,
        "force": false,
        "password": "correct horse battery staple",
        "profile": "strong"
      },
      {
        "code": "492039",
        "force": true,
        "password": "correct horse battery staple",
        "profile": "max"
      }
    ],
    "purge_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "quick_unlock_status": [
      null
    ],
    "reauthenticate": [
      {
        "password": "correct horse battery staple"
      }
    ],
    "rekey_commit": [
      {
        "code": null,
        "newPassword": "a new master password",
        "password": "correct horse battery staple"
      },
      {
        "code": "492039",
        "newPassword": "a new master password",
        "password": "correct horse battery staple"
      }
    ],
    "rekey_prepare": [
      {
        "newPassword": "a new master password",
        "password": "correct horse battery staple"
      }
    ],
    "rekey_review": [
      null
    ],
    "remove_biometric": [
      null
    ],
    "remove_hardware_key": [
      {
        "methodId": "0123456789abcdef0123456789abcdef"
      }
    ],
    "remove_hardware_vault_key": [
      {
        "methodId": "0123456789abcdef0123456789abcdef",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "rename_vault": [
      {
        "name": "Family",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "restore_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "reveal_secret": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "field": "password",
          "kind": "field"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "elementId": "5-019a2b3c4d5e7000",
          "kind": "customField"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "index": 0,
          "kind": "history"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_abandon": [
      null
    ],
    "rotation_abandon_committed": [
      null
    ],
    "rotation_begin": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "newPassword": "q7#Vt2-new",
        "rulesSource": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "newPassword": "q7#Vt2-new",
        "rulesSource": "siteQuirks",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_close": [
      {
        "attemptId": "00000000000000a1.0123456789abcdef.0",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "outcome": "succeeded",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "attemptId": "00000000000000a1.0123456789abcdef.0",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "outcome": "failed",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_copy": [
      {
        "attemptId": "00000000000000a3.0123456789abcdef.1",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_dismiss": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_queue": [
      null
    ],
    "rotation_resume": [
      {
        "code": null,
        "kitGroup": null,
        "password": null
      },
      {
        "code": "492039",
        "kitGroup": "8VJ2",
        "password": "a new master password"
      }
    ],
    "rotation_reveal": [
      {
        "attemptId": "00000000000000a1.0123456789abcdef.0",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_set_policy": [
      {
        "everyDays": 30,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "everyDays": 365,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "everyDays": null,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_snooze": [
      {
        "days": 1,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "days": 7,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "days": 30,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rotation_status": [
      null
    ],
    "rotation_undismiss": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "save_emergency_kit": [
      {
        "labels": {
          "accountId": "Account ID",
          "email": "Account email",
          "footer": "Pre-audit software.",
          "intro": "Keep this page somewhere safe.",
          "masterPassword": "Master password",
          "masterPasswordHint": "Write it here by hand.",
          "secretKey": "Secret Key",
          "signInUrl": "Sign-in address",
          "signInUrlValue": "This device only",
          "title": "Mosaic Emergency Kit"
        },
        "password": null,
        "pending": false
      },
      {
        "labels": {
          "accountId": "Account ID",
          "email": "Account email",
          "footer": "Pre-audit software.",
          "intro": "Keep this page somewhere safe.",
          "masterPassword": "Master password",
          "masterPasswordHint": "Write it here by hand.",
          "secretKey": "Secret Key",
          "signInUrl": "Sign-in address",
          "signInUrlValue": "This device only",
          "title": "Mosaic Emergency Kit"
        },
        "password": "correct horse battery staple",
        "pending": false
      }
    ],
    "secret_key_rotate_commit": [
      {
        "code": null,
        "password": "correct horse battery staple"
      },
      {
        "code": "492039",
        "password": "correct horse battery staple"
      }
    ],
    "secret_key_rotate_prepare": [
      {
        "password": "correct horse battery staple"
      }
    ],
    "security_report": [
      null
    ],
    "session_state": [
      null
    ],
    "set_favorite": [
      {
        "favorite": true,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "setup_code_show": [
      {
        "password": "correct horse battery staple"
      }
    ],
    "sharing_accept_key_change": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V"
      }
    ],
    "sharing_contacts": [
      null
    ],
    "sharing_identity": [
      null
    ],
    "sharing_lookup": [
      {
        "email": "sam@example.com"
      }
    ],
    "sharing_verify_contact": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V"
      }
    ],
    "ssh_agent_set_enabled": [
      {
        "enabled": true
      }
    ],
    "ssh_agent_status": [
      null
    ],
    "ssh_respond": [
      {
        "decision": "allowOnce",
        "requestId": "ssh-1"
      },
      {
        "decision": "allowFor15Minutes",
        "requestId": "ssh-1"
      },
      {
        "decision": "deny",
        "requestId": "ssh-1"
      }
    ],
    "sync_acknowledge": [
      {}
    ],
    "sync_check_server": [
      {
        "serverUrl": "https://sync.example.com"
      }
    ],
    "sync_device_remove": [
      {
        "code": null,
        "deviceId": "019a2b3c-4d5e-7050-8000-000000000002",
        "password": "correct horse battery staple"
      },
      {
        "code": "492039",
        "deviceId": "019a2b3c-4d5e-7050-8000-000000000002",
        "password": "correct horse battery staple"
      }
    ],
    "sync_device_rename": [
      {
        "deviceId": "019a2b3c-4d5e-7050-8000-000000000002",
        "name": "Work laptop"
      }
    ],
    "sync_devices": [
      null
    ],
    "sync_disconnect": [
      null
    ],
    "sync_dismiss_account_file": [
      {
        "name": "account-00000002.macct",
        "sha256": "abababababababababababababababababababababababababababababababab"
      }
    ],
    "sync_dismiss_rekey_marker": [
      {
        "name": "rekey-00000002.mrk",
        "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
      }
    ],
    "sync_folder_connect": [
      {
        "choiceId": "choice-1",
        "deviceName": "Mosaic on Windows"
      }
    ],
    "sync_hosted_connect": [
      {
        "deviceName": "Mosaic on Windows",
        "inviteCode": "INVITE-CODE-1234",
        "mode": "register",
        "password": "correct horse battery staple",
        "serverUrl": "https://sync.example.com"
      },
      {
        "deviceName": "Mosaic on Windows",
        "inviteCode": null,
        "mode": "signIn",
        "password": "correct horse battery staple",
        "serverUrl": "https://sync.example.com"
      }
    ],
    "sync_hosted_two_factor": [
      {
        "code": "492039"
      }
    ],
    "sync_hosted_webauthn_begin": [
      null
    ],
    "sync_ignored_rekey_markers": [
      null
    ],
    "sync_login_methods": [
      null
    ],
    "sync_now": [
      null
    ],
    "sync_pick_folder": [
      null
    ],
    "sync_recover_password": [
      {
        "code": null,
        "newPassword": "a new master password",
        "secretKey": null
      },
      {
        "code": "492039",
        "newPassword": "a new master password",
        "secretKey": null
      },
      {
        "code": null,
        "newPassword": "a new master password",
        "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
      }
    ],
    "sync_rekey_markers": [
      null
    ],
    "sync_restore_rekey_marker": [
      {
        "name": "rekey-00000002.mrk",
        "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
      }
    ],
    "sync_status": [
      null
    ],
    "sync_totp_begin": [
      {
        "password": "correct horse battery staple"
      }
    ],
    "sync_totp_confirm": [
      {
        "code": "492039"
      }
    ],
    "sync_totp_disable": [
      {
        "code": "492039",
        "password": "correct horse battery staple"
      }
    ],
    "sync_webauthn_add_begin": [
      {
        "code": null,
        "label": "YubiKey 5C",
        "password": "correct horse battery staple"
      },
      {
        "code": "492039",
        "label": "Backup key",
        "password": "correct horse battery staple"
      }
    ],
    "sync_webauthn_add_status": [
      null
    ],
    "sync_webauthn_confirm_begin": [
      {
        "password": "correct horse battery staple"
      }
    ],
    "sync_webauthn_list": [
      null
    ],
    "sync_webauthn_remove": [
      {
        "code": null,
        "credentialRef": "00112233445566778899aabbccddeeff"
      },
      {
        "code": "492039",
        "credentialRef": "00112233445566778899aabbccddeeff"
      }
    ],
    "sync_webauthn_reopen": [
      null
    ],
    "sync_webauthn_status": [
      null
    ],
    "totp_code": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "trash_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "unlock": [
      {
        "password": "correct horse battery staple",
        "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
      },
      {
        "password": "correct horse battery staple",
        "secretKey": null
      }
    ],
    "unlock_quick": [
      {
        "method": "biometric",
        "pin": null
      },
      {
        "method": "hardwareKey",
        "pin": null
      },
      {
        "method": "hardwareKey",
        "pin": "246810"
      }
    ],
    "update_item": [
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "kind": "hidden",
              "label": "PIN code",
              "op": "update",
              "value": null
            },
            {
              "elementId": "7-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "set",
            "uri": "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "sshKey": {
            "op": "set",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
          },
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "kind": "hidden",
              "label": "PIN code",
              "op": "update",
              "value": null
            },
            {
              "elementId": "7-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "accountName": "alex@example.com",
            "issuer": "GitHub",
            "op": "setPending",
            "pendingId": "otp-1"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "sshKey": {
            "op": "set",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
          },
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "kind": "hidden",
              "label": "PIN code",
              "op": "update",
              "value": null
            },
            {
              "elementId": "7-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "remove"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "sshKey": {
            "op": "set",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
          },
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "update_settings": [
      {
        "password": null,
        "settings": {
          "autoLockMinutes": 10,
          "breachCheck": "off",
          "breachConsentAt": null,
          "clipboardClearSeconds": 30,
          "globalShortcut": "Ctrl+Shift+Space",
          "language": "system",
          "lockOnScreenLock": true,
          "lockOnSleep": true,
          "quickUnlockMaxAgeDays": 14,
          "rotationReminders": true,
          "theme": "system"
        }
      },
      {
        "password": "correct horse battery staple",
        "settings": {
          "autoLockMinutes": null,
          "breachCheck": "weekly",
          "breachConsentAt": 1761000000000,
          "clipboardClearSeconds": null,
          "globalShortcut": "Ctrl+Alt+K",
          "language": "de",
          "lockOnScreenLock": false,
          "lockOnSleep": false,
          "quickUnlockMaxAgeDays": 30,
          "rotationReminders": false,
          "theme": "dark"
        }
      }
    ],
    "vault_invite_cancel": [
      {
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000001"
      }
    ],
    "vault_leave": [
      {
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "vault_member_remove": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "vault_member_set_role": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "role": "read",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "vault_members": [
      {
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "vault_rotate_key": [
      {
        "proof": null,
        "resealAttachments": false,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "proof": "correct horse battery staple",
        "resealAttachments": true,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      }
    ],
    "vault_share": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "role": "manage",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "role": "write",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "role": "read",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "verify_emergency_kit": [
      {
        "lastGroup": "8VJ2",
        "pending": false
      },
      {
        "lastGroup": "8VJ2",
        "pending": true
      }
    ]
  },
  "responses": {
    "add_hardware_vault_key": [
      {
        "keys": []
      },
      {
        "keys": [
          {
            "addedAt": 1761480000000,
            "methodId": "0123456789abcdef0123456789abcdef"
          }
        ]
      }
    ],
    "add_item": [
      {
        "attachments": [
          {
            "addedAt": 1800000000000,
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "available": true,
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "previewable": false,
            "size": 512
          }
        ],
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        },
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "history": [
          {
            "changedAt": 1761480100000,
            "historyId": "00000000000000a2.0123456789abcdef.0",
            "index": 0,
            "reason": "unconfirmed"
          },
          {
            "changedAt": 1761400000000,
            "historyId": "00000000000000a0.0123456789abcdef.0",
            "index": 1,
            "reason": "replaced"
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "passkey": null,
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [
          {
            "attemptId": "00000000000000a1.0123456789abcdef.0",
            "stage": "prepared",
            "startedAt": 1761480000000
          }
        ],
        "rotationPolicy": 90,
        "secrets": [
          "password"
        ],
        "sshKey": null,
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "app_status": [
      {
        "otpScreenScan": true,
        "screenLockDetection": false,
        "shortcut": "conflict"
      }
    ],
    "attachment_add": [
      {
        "item": {
          "attachments": [
            {
              "addedAt": 1800000000000,
              "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
              "available": true,
              "fileName": "recovery-codes.txt",
              "mime": "text/plain",
              "previewable": false,
              "size": 512
            }
          ],
          "changeUrl": {
            "source": "wellKnown",
            "url": "https://example.com/.well-known/change-password"
          },
          "createdAt": 1761400000000,
          "customFields": [
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "kind": "hidden",
              "label": "PIN",
              "value": null
            }
          ],
          "history": [
            {
              "changedAt": 1761480100000,
              "historyId": "00000000000000a2.0123456789abcdef.0",
              "index": 0,
              "reason": "unconfirmed"
            },
            {
              "changedAt": 1761400000000,
              "historyId": "00000000000000a0.0123456789abcdef.0",
              "index": 1,
              "reason": "replaced"
            }
          ],
          "historyCount": 2,
          "otp": {
            "digits": 6,
            "hardwareProtected": false,
            "issuer": "GitHub",
            "period": 30,
            "scheme": "totp"
          },
          "passkey": null,
          "review": {
            "conflictCopyOf": null,
            "conflictingFields": 0,
            "deletedConcurrently": false
          },
          "rotationAttempts": [
            {
              "attemptId": "00000000000000a1.0123456789abcdef.0",
              "stage": "prepared",
              "startedAt": 1761480000000
            }
          ],
          "rotationPolicy": 90,
          "secrets": [
            "password"
          ],
          "sshKey": null,
          "summary": {
            "favorite": true,
            "hasOtp": true,
            "host": "github.com",
            "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
            "kind": "login",
            "modifiedAt": 1761480000000,
            "needsReview": false,
            "subtitle": "alex@example.com",
            "tags": [
              "work"
            ],
            "title": "GitHub",
            "trashedAt": null,
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          },
          "text": [
            {
              "field": "username",
              "value": "alex@example.com"
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "exactOrigin",
              "url": "https://github.com/login"
            }
          ]
        },
        "kind": "added"
      },
      {
        "kind": "cancelled"
      }
    ],
    "attachment_remove": [
      {
        "attachments": [
          {
            "addedAt": 1800000000000,
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "available": true,
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "previewable": false,
            "size": 512
          }
        ],
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        },
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "history": [
          {
            "changedAt": 1761480100000,
            "historyId": "00000000000000a2.0123456789abcdef.0",
            "index": 0,
            "reason": "unconfirmed"
          },
          {
            "changedAt": 1761400000000,
            "historyId": "00000000000000a0.0123456789abcdef.0",
            "index": 1,
            "reason": "replaced"
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "passkey": null,
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [
          {
            "attemptId": "00000000000000a1.0123456789abcdef.0",
            "stage": "prepared",
            "startedAt": 1761480000000
          }
        ],
        "rotationPolicy": 90,
        "secrets": [
          "password"
        ],
        "sshKey": null,
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "attachment_save": [
      {
        "kind": "saved"
      },
      {
        "kind": "cancelled"
      }
    ],
    "breach_scan_cancel": [
      null
    ],
    "breach_scan_start": [
      null
    ],
    "browser_connector_audit": [
      {
        "findings": []
      },
      {
        "findings": [
          {
            "browser": "chromium",
            "location": "HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\app.mosaic.host",
            "path": "C:\\Users\\alex\\Downloads\\other-host.exe"
          }
        ]
      }
    ],
    "browser_connector_repair": [
      {
        "findings": []
      },
      {
        "findings": [
          {
            "browser": "chromium",
            "location": "HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\app.mosaic.host",
            "path": "C:\\Users\\alex\\Downloads\\other-host.exe"
          }
        ]
      }
    ],
    "browser_pairing_respond": [
      null
    ],
    "browser_passkey_respond": [
      null
    ],
    "browser_remove_pairing": [
      {
        "endpoint": "running",
        "pairings": [
          {
            "browser": "chromium",
            "connected": true,
            "createdAt": 1761480000000,
            "label": "Chrome",
            "lastUsedAt": 1761480035000,
            "pairingId": "0123456789abcdef0123456789abcdef"
          },
          {
            "browser": "firefox",
            "connected": false,
            "createdAt": 1761480010000,
            "label": "Firefox",
            "lastUsedAt": null,
            "pairingId": "fedcba9876543210fedcba9876543210"
          }
        ],
        "registry": "ok"
      },
      {
        "endpoint": "inUse",
        "pairings": [],
        "registry": "ok"
      },
      {
        "endpoint": "unavailable",
        "pairings": [],
        "registry": "ok"
      },
      {
        "endpoint": "running",
        "pairings": [],
        "registry": "unreadable"
      }
    ],
    "browser_rotation_respond": [
      null
    ],
    "browser_rotation_reveal": [
      {
        "hideAfterSeconds": 10,
        "value": "q7#Vt2!mZp9@Lx4$Nw6&"
      }
    ],
    "browser_save_respond": [
      null
    ],
    "browser_status": [
      {
        "endpoint": "running",
        "pairings": [
          {
            "browser": "chromium",
            "connected": true,
            "createdAt": 1761480000000,
            "label": "Chrome",
            "lastUsedAt": 1761480035000,
            "pairingId": "0123456789abcdef0123456789abcdef"
          },
          {
            "browser": "firefox",
            "connected": false,
            "createdAt": 1761480010000,
            "label": "Firefox",
            "lastUsedAt": null,
            "pairingId": "fedcba9876543210fedcba9876543210"
          }
        ],
        "registry": "ok"
      },
      {
        "endpoint": "inUse",
        "pairings": [],
        "registry": "ok"
      },
      {
        "endpoint": "unavailable",
        "pairings": [],
        "registry": "ok"
      },
      {
        "endpoint": "running",
        "pairings": [],
        "registry": "unreadable"
      }
    ],
    "change_master_password": [
      null
    ],
    "clear_clipboard": [
      null
    ],
    "close_hardware_vault": [
      null
    ],
    "copy_to_clipboard": [
      {
        "clearAfterSeconds": 30,
        "clearsAt": 1761480030000
      },
      {
        "clearAfterSeconds": null,
        "clearsAt": null
      }
    ],
    "create_account": [
      {
        "emergencyKit": {
          "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
          "email": "alex@example.com",
          "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
        },
        "session": {
          "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
          "email": "alex@example.com",
          "kitVerified": false,
          "state": "unlocked"
        }
      }
    ],
    "create_hardware_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "openUntil": null,
        "protection": "accountKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "openUntil": null,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      },
      {
        "available": true,
        "itemCount": 5,
        "name": "Authenticators",
        "openUntil": 1761480300000,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "create_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "openUntil": null,
        "protection": "accountKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "openUntil": null,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      },
      {
        "available": true,
        "itemCount": 5,
        "name": "Authenticators",
        "openUntil": 1761480300000,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "emergency_claim": [
      {
        "vaults": 2
      }
    ],
    "emergency_grant": [
      null
    ],
    "emergency_overview": [
      {
        "granted": [
          {
            "email": "sam@example.com",
            "granteeAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
            "releaseAt": 1762084800000,
            "requestedAt": 1761480000000,
            "scope": {
              "kind": "all"
            },
            "state": "waiting",
            "waitDays": 7
          },
          {
            "email": "kim@example.com",
            "granteeAccountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "releaseAt": null,
            "requestedAt": null,
            "scope": {
              "kind": "vaults",
              "vaultIds": [
                "019a2b3c-4d5e-7010-8000-000000000001",
                "019a2b3c-4d5e-7010-8000-000000000002"
              ]
            },
            "state": "idle",
            "waitDays": 30
          }
        ],
        "received": [
          {
            "cooldownUntil": null,
            "email": "lee@example.com",
            "grantorAccountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "releaseAt": 1761480000000,
            "requestedAt": 1761480000000,
            "state": "released",
            "waitDays": 14
          },
          {
            "cooldownUntil": 1761566400000,
            "email": "max@example.com",
            "grantorAccountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "releaseAt": null,
            "requestedAt": null,
            "state": "idle",
            "waitDays": 2
          }
        ]
      },
      {
        "granted": [],
        "received": []
      }
    ],
    "emergency_request": [
      {
        "releaseAt": 1762084800000
      }
    ],
    "emergency_respond": [
      null
    ],
    "emergency_revoke": [
      null
    ],
    "enroll_biometric": [
      {
        "biometric": {
          "available": true,
          "enrolled": true,
          "enrolledAt": 1761480000000,
          "expiresAt": 1762689600000,
          "platform": "windowsHello"
        },
        "hardwareKeys": {
          "keys": [
            {
              "addedAt": 1761480000000,
              "methodId": "0123456789abcdef0123456789abcdef"
            }
          ],
          "maxKeys": 8,
          "supported": true
        },
        "maxAgeDays": 14
      },
      {
        "biometric": {
          "available": false,
          "enrolled": false,
          "enrolledAt": null,
          "expiresAt": null,
          "platform": null
        },
        "hardwareKeys": {
          "keys": [],
          "maxKeys": 8,
          "supported": false
        },
        "maxAgeDays": 30
      }
    ],
    "enroll_hardware_key": [
      {
        "biometric": {
          "available": true,
          "enrolled": true,
          "enrolledAt": 1761480000000,
          "expiresAt": 1762689600000,
          "platform": "windowsHello"
        },
        "hardwareKeys": {
          "keys": [
            {
              "addedAt": 1761480000000,
              "methodId": "0123456789abcdef0123456789abcdef"
            }
          ],
          "maxKeys": 8,
          "supported": true
        },
        "maxAgeDays": 14
      },
      {
        "biometric": {
          "available": false,
          "enrolled": false,
          "enrolledAt": null,
          "expiresAt": null,
          "platform": null
        },
        "hardwareKeys": {
          "keys": [],
          "maxKeys": 8,
          "supported": false
        },
        "maxAgeDays": 30
      }
    ],
    "export_passkeys_cxf": [
      {
        "count": 2,
        "kind": "saved"
      },
      {
        "kind": "cancelled"
      }
    ],
    "generate_password": [
      {
        "entropyBits": 131.1,
        "strength": "veryStrong",
        "value": "q7#Vt2!mZp9@Lx4$Nw6&"
      }
    ],
    "get_item": [
      {
        "attachments": [
          {
            "addedAt": 1800000000000,
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "available": true,
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "previewable": false,
            "size": 512
          }
        ],
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        },
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "history": [
          {
            "changedAt": 1761480100000,
            "historyId": "00000000000000a2.0123456789abcdef.0",
            "index": 0,
            "reason": "unconfirmed"
          },
          {
            "changedAt": 1761400000000,
            "historyId": "00000000000000a0.0123456789abcdef.0",
            "index": 1,
            "reason": "replaced"
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "passkey": null,
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [
          {
            "attemptId": "00000000000000a1.0123456789abcdef.0",
            "stage": "prepared",
            "startedAt": 1761480000000
          }
        ],
        "rotationPolicy": 90,
        "secrets": [
          "password"
        ],
        "sshKey": null,
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      },
      {
        "attachments": [],
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        },
        "createdAt": 1761400000000,
        "customFields": [],
        "history": [
          {
            "changedAt": 1761480100000,
            "historyId": "00000000000000a2.0123456789abcdef.0",
            "index": 0,
            "reason": "unconfirmed"
          },
          {
            "changedAt": 1761400000000,
            "historyId": "00000000000000a0.0123456789abcdef.0",
            "index": 1,
            "reason": "replaced"
          }
        ],
        "historyCount": 2,
        "otp": null,
        "passkey": null,
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [
          {
            "attemptId": "00000000000000a1.0123456789abcdef.0",
            "stage": "prepared",
            "startedAt": 1761480000000
          }
        ],
        "rotationPolicy": 90,
        "secrets": [
          "password"
        ],
        "sshKey": {
          "agentSupported": true,
          "algorithm": "ssh-ed25519",
          "encrypted": true,
          "fingerprint": "SHA256:mockmockmockmockmockmockmockmockmockmockmo",
          "publicKey": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockMockMockMockMockMockMockMockMockMock deploy"
        },
        "summary": {
          "favorite": true,
          "hasOtp": false,
          "host": null,
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "sshKey",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "deploy@example.com",
          "tags": [
            "work"
          ],
          "title": "Deploy key",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "sshComment",
            "value": "deploy@example.com"
          }
        ],
        "urls": []
      },
      {
        "attachments": [],
        "changeUrl": null,
        "createdAt": 1761400000000,
        "customFields": [],
        "history": [],
        "historyCount": 0,
        "otp": null,
        "passkey": {
          "algorithm": "es256",
          "createdAt": 1761400000000,
          "credentialId": "AAECAwQFBgcICQoLDA0ODw",
          "rpId": "example.com",
          "signCount": 0,
          "userDisplayName": "Alex Example",
          "userName": "alex@example.com"
        },
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [],
        "rotationPolicy": null,
        "secrets": [],
        "sshKey": null,
        "summary": {
          "favorite": true,
          "hasOtp": false,
          "host": "example.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "passkey",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "example.com",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [],
        "urls": []
      }
    ],
    "get_settings": [
      {
        "autoLockMinutes": 10,
        "breachCheck": "off",
        "breachConsentAt": null,
        "clipboardClearSeconds": 30,
        "globalShortcut": "Ctrl+Shift+Space",
        "language": "system",
        "lockOnScreenLock": true,
        "lockOnSleep": true,
        "quickUnlockMaxAgeDays": 14,
        "rotationReminders": true,
        "theme": "system"
      },
      {
        "autoLockMinutes": null,
        "breachCheck": "weekly",
        "breachConsentAt": 1761000000000,
        "clipboardClearSeconds": null,
        "globalShortcut": "Ctrl+Alt+K",
        "language": "de",
        "lockOnScreenLock": false,
        "lockOnSleep": false,
        "quickUnlockMaxAgeDays": 30,
        "rotationReminders": false,
        "theme": "dark"
      }
    ],
    "hardware_key_cancel": [
      null
    ],
    "hardware_vault_keys": [
      {
        "keys": []
      },
      {
        "keys": [
          {
            "addedAt": 1761480000000,
            "methodId": "0123456789abcdef0123456789abcdef"
          }
        ]
      }
    ],
    "import_cancel": [
      null
    ],
    "import_choose_file": [
      {
        "fileName": "enpass-export.json",
        "fileToken": "f-1",
        "kind": "chosen"
      },
      {
        "kind": "cancelled"
      }
    ],
    "import_commit": [
      {
        "attachments": 1,
        "attachmentsFailed": 0,
        "failed": 0,
        "imported": 1,
        "skipped": 1
      }
    ],
    "import_preview": [
      {
        "attachmentBytes": 2048,
        "attachments": 1,
        "counts": [
          {
            "count": 1,
            "kind": "login"
          },
          {
            "count": 1,
            "kind": "secureNote"
          }
        ],
        "entries": [
          {
            "attachments": 1,
            "duplicateInFile": null,
            "duplicateOf": {
              "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
              "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
            },
            "index": 0,
            "kind": "login",
            "subtitle": "alex@example.com",
            "title": "GitHub",
            "warnings": [
              "unmappedFields",
              "otpUnreadable",
              "attachmentTooLarge",
              "attachmentMissing",
              "tagsDropped",
              "historyTruncated",
              "passkeysNotImported",
              "unknownCategory",
              "fieldsDropped",
              "archived",
              "attachmentQuota",
              "titleCleaned",
              "passkeyUnreadable",
              "passkeyUnsupported",
              "passkeysLimit"
            ]
          },
          {
            "attachments": 0,
            "duplicateInFile": 0,
            "duplicateOf": null,
            "index": 1,
            "kind": "secureNote",
            "subtitle": null,
            "title": null,
            "warnings": []
          }
        ],
        "format": "enpass",
        "previewId": "p-1",
        "skipped": [
          {
            "index": 2,
            "reason": "malformed"
          },
          {
            "index": 3,
            "reason": "empty"
          },
          {
            "index": 4,
            "reason": "tooLarge"
          }
        ],
        "warnings": [
          "unmappedFields",
          "attachmentsSkipped",
          "entriesSkipped",
          "otpUnreadable",
          "passkeysNotImported",
          "passkeyUnreadable",
          "passkeyUnsupported",
          "passkeysSkipped"
        ]
      }
    ],
    "invite_accept": [
      null
    ],
    "invite_decline": [
      null
    ],
    "invite_preview": [
      {
        "ownerAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "ownerCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "ownerEmail": "sam@example.com",
        "ownerStatus": "pinned",
        "role": "write",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "senderAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "senderEmail": "sam@example.com",
        "senderStatus": "pinned",
        "vaultName": "Family"
      },
      {
        "ownerAccountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "ownerCode": "2B9MQ 7XK4D RT6W0 ZNCF8 V3HJ4P",
        "ownerEmail": "sam@example.com",
        "ownerStatus": "new",
        "role": "read",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "senderAccountId": "019a2b3c-4d5e-7030-8000-000000000003",
        "senderEmail": "kim@example.com",
        "senderStatus": "verified",
        "vaultName": null
      }
    ],
    "invites_list": [
      {
        "incoming": [
          {
            "expiresAt": 1761566400000,
            "inviteId": "019a2b3c-4d5e-7060-8000-000000000002",
            "role": "write",
            "senderEmail": "sam@example.com"
          }
        ],
        "outgoing": [
          {
            "expiresAt": 1761566400000,
            "inviteId": "019a2b3c-4d5e-7060-8000-000000000001",
            "recipientEmail": "kim@example.com",
            "role": "read",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          }
        ]
      },
      {
        "incoming": [],
        "outgoing": [
          {
            "expiresAt": 1761480000000,
            "inviteId": "019a2b3c-4d5e-7060-8000-000000000003",
            "recipientEmail": null,
            "role": "manage",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          }
        ]
      }
    ],
    "join_folder": [
      {
        "state": "unlocked"
      },
      {
        "state": "twoFactorRequired"
      }
    ],
    "join_hosted": [
      {
        "state": "unlocked"
      },
      {
        "state": "twoFactorRequired"
      }
    ],
    "join_hosted_two_factor": [
      {
        "state": "unlocked"
      },
      {
        "state": "twoFactorRequired"
      }
    ],
    "join_hosted_webauthn_begin": [
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913"
      }
    ],
    "list_items": [
      [
        {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        {
          "favorite": false,
          "hasOtp": false,
          "host": null,
          "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
          "kind": "secureNote",
          "modifiedAt": null,
          "needsReview": true,
          "subtitle": null,
          "tags": [],
          "title": null,
          "trashedAt": 1761480000000,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      ]
    ],
    "list_vaults": [
      [
        {
          "available": true,
          "itemCount": 42,
          "name": "Personal",
          "openUntil": null,
          "protection": "accountKey",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        {
          "available": false,
          "itemCount": 0,
          "name": null,
          "openUntil": null,
          "protection": "hardwareKey",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
        },
        {
          "available": true,
          "itemCount": 5,
          "name": "Authenticators",
          "openUntil": 1761480300000,
          "protection": "hardwareKey",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
        }
      ]
    ],
    "lock": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "kitVerified": true,
        "state": "unlocked"
      }
    ],
    "move_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000009",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      }
    ],
    "open_hardware_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "openUntil": null,
        "protection": "accountKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "openUntil": null,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      },
      {
        "available": true,
        "itemCount": 5,
        "name": "Authenticators",
        "openUntil": 1761480300000,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "otp_parse": [
      {
        "accountName": "alex@example.com",
        "algorithm": "sha1",
        "digits": 6,
        "expiresInSeconds": 600,
        "issuer": "GitHub",
        "pendingId": "otp-1",
        "period": 30,
        "scheme": "totp",
        "strength": "belowRfc4226"
      },
      {
        "accountName": null,
        "algorithm": "sha256",
        "digits": 8,
        "expiresInSeconds": 600,
        "issuer": null,
        "pendingId": "otp-2",
        "period": null,
        "scheme": "hotp",
        "strength": "rfc4226"
      },
      {
        "accountName": "gabe",
        "algorithm": "sha1",
        "digits": 5,
        "expiresInSeconds": 590,
        "issuer": "Steam",
        "pendingId": "otp-3",
        "period": 30,
        "scheme": "steam",
        "strength": "weak"
      }
    ],
    "otp_pending_code": [
      {
        "code": "492039",
        "period": 30,
        "secondsRemaining": 17
      },
      {
        "code": "755224",
        "period": null,
        "secondsRemaining": null
      }
    ],
    "otp_pending_discard": [
      null
    ],
    "otp_scan": [
      {
        "clipboardCleared": true,
        "kind": "found",
        "pending": {
          "accountName": "alex@example.com",
          "algorithm": "sha1",
          "digits": 6,
          "expiresInSeconds": 600,
          "issuer": "GitHub",
          "pendingId": "otp-1",
          "period": 30,
          "scheme": "totp",
          "strength": "belowRfc4226"
        }
      },
      {
        "kind": "cancelled"
      }
    ],
    "password_history_restore": [
      null
    ],
    "profile_upgrade": [
      {
        "finished": true,
        "hardwareKeysRemoved": 1,
        "quickUnlockRemoved": true,
        "unreachable": [
          {
            "fileName": "statement.pdf",
            "itemTitle": "Bank"
          },
          {
            "fileName": null,
            "itemTitle": null
          }
        ],
        "unreachableAttachments": 2
      },
      {
        "finished": false,
        "hardwareKeysRemoved": 0,
        "quickUnlockRemoved": false,
        "unreachable": [],
        "unreachableAttachments": 0
      }
    ],
    "purge_item": [
      null
    ],
    "quick_unlock_status": [
      {
        "biometric": {
          "available": true,
          "enrolled": true,
          "enrolledAt": 1761480000000,
          "expiresAt": 1762689600000,
          "platform": "windowsHello"
        },
        "hardwareKeys": {
          "keys": [
            {
              "addedAt": 1761480000000,
              "methodId": "0123456789abcdef0123456789abcdef"
            }
          ],
          "maxKeys": 8,
          "supported": true
        },
        "maxAgeDays": 14
      },
      {
        "biometric": {
          "available": false,
          "enrolled": false,
          "enrolledAt": null,
          "expiresAt": null,
          "platform": null
        },
        "hardwareKeys": {
          "keys": [],
          "maxKeys": 8,
          "supported": false
        },
        "maxAgeDays": 30
      }
    ],
    "reauthenticate": [
      null
    ],
    "rekey_commit": [
      {
        "finished": true,
        "hardwareKeysRemoved": 1,
        "quickUnlockRemoved": true,
        "unreachable": [
          {
            "fileName": "statement.pdf",
            "itemTitle": "Bank"
          },
          {
            "fileName": null,
            "itemTitle": null
          }
        ],
        "unreachableAttachments": 2
      },
      {
        "finished": false,
        "hardwareKeysRemoved": 0,
        "quickUnlockRemoved": false,
        "unreachable": [],
        "unreachableAttachments": 0
      }
    ],
    "rekey_prepare": [
      null
    ],
    "rekey_review": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "backend": "hosted",
        "grants": 1,
        "hardwareKeys": 1,
        "hardwareVaults": 1,
        "memberVaults": [
          {
            "name": null,
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000004"
          }
        ],
        "ownedVaults": 2,
        "quickUnlock": true,
        "sharedOwned": [
          {
            "members": 2,
            "name": "Family",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
          }
        ],
        "unreachableAttachments": 3
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "backend": "folder",
        "grants": 0,
        "hardwareKeys": 0,
        "hardwareVaults": 0,
        "memberVaults": [],
        "ownedVaults": 1,
        "quickUnlock": false,
        "sharedOwned": [],
        "unreachableAttachments": 0
      }
    ],
    "remove_biometric": [
      {
        "biometric": {
          "available": true,
          "enrolled": true,
          "enrolledAt": 1761480000000,
          "expiresAt": 1762689600000,
          "platform": "windowsHello"
        },
        "hardwareKeys": {
          "keys": [
            {
              "addedAt": 1761480000000,
              "methodId": "0123456789abcdef0123456789abcdef"
            }
          ],
          "maxKeys": 8,
          "supported": true
        },
        "maxAgeDays": 14
      },
      {
        "biometric": {
          "available": false,
          "enrolled": false,
          "enrolledAt": null,
          "expiresAt": null,
          "platform": null
        },
        "hardwareKeys": {
          "keys": [],
          "maxKeys": 8,
          "supported": false
        },
        "maxAgeDays": 30
      }
    ],
    "remove_hardware_key": [
      {
        "biometric": {
          "available": true,
          "enrolled": true,
          "enrolledAt": 1761480000000,
          "expiresAt": 1762689600000,
          "platform": "windowsHello"
        },
        "hardwareKeys": {
          "keys": [
            {
              "addedAt": 1761480000000,
              "methodId": "0123456789abcdef0123456789abcdef"
            }
          ],
          "maxKeys": 8,
          "supported": true
        },
        "maxAgeDays": 14
      },
      {
        "biometric": {
          "available": false,
          "enrolled": false,
          "enrolledAt": null,
          "expiresAt": null,
          "platform": null
        },
        "hardwareKeys": {
          "keys": [],
          "maxKeys": 8,
          "supported": false
        },
        "maxAgeDays": 30
      }
    ],
    "remove_hardware_vault_key": [
      {
        "keys": []
      },
      {
        "keys": [
          {
            "addedAt": 1761480000000,
            "methodId": "0123456789abcdef0123456789abcdef"
          }
        ]
      }
    ],
    "rename_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "openUntil": null,
        "protection": "accountKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "openUntil": null,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      },
      {
        "available": true,
        "itemCount": 5,
        "name": "Authenticators",
        "openUntil": 1761480300000,
        "protection": "hardwareKey",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000003"
      }
    ],
    "restore_item": [
      null
    ],
    "reveal_secret": [
      {
        "hideAfterSeconds": 60,
        "value": "hunter2"
      }
    ],
    "rotation_abandon": [
      null
    ],
    "rotation_abandon_committed": [
      null
    ],
    "rotation_begin": [
      {
        "attemptId": "00000000000000a1.0123456789abcdef.0",
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        }
      },
      {
        "attemptId": "00000000000000a1.0123456789abcdef.0",
        "changeUrl": null
      }
    ],
    "rotation_close": [
      null
    ],
    "rotation_copy": [
      {
        "clearAfterSeconds": 30,
        "clearsAt": 1761480030000
      },
      {
        "clearAfterSeconds": null,
        "clearsAt": null
      }
    ],
    "rotation_dismiss": [
      null
    ],
    "rotation_queue": [
      {
        "breach": {
          "consentAt": 1761000000000,
          "lastError": "offline",
          "lastFinishedAt": 1761470000000,
          "mode": "weekly",
          "running": true
        },
        "entries": [
          {
            "attempts": [
              {
                "attemptId": "00000000000000a1.0123456789abcdef.0",
                "stage": "prepared",
                "startedAt": 1761480000000
              }
            ],
            "host": "example.com",
            "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
            "kind": "guided",
            "reasons": [
              "weak",
              "unfinished"
            ],
            "severity": "high",
            "snoozedUntil": null,
            "title": "Example",
            "username": "alex@example.com",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          },
          {
            "attempts": [],
            "host": null,
            "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
            "kind": "manual",
            "reasons": [
              "old",
              "breached"
            ],
            "severity": "high",
            "snoozedUntil": 1761990000000,
            "title": null,
            "username": null,
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ]
      },
      {
        "breach": {
          "consentAt": null,
          "lastError": null,
          "lastFinishedAt": null,
          "mode": "off",
          "running": false
        },
        "entries": []
      }
    ],
    "rotation_resume": [
      {
        "finished": true,
        "hardwareKeysRemoved": 1,
        "quickUnlockRemoved": true,
        "unreachable": [
          {
            "fileName": "statement.pdf",
            "itemTitle": "Bank"
          },
          {
            "fileName": null,
            "itemTitle": null
          }
        ],
        "unreachableAttachments": 2
      },
      {
        "finished": false,
        "hardwareKeysRemoved": 0,
        "quickUnlockRemoved": false,
        "unreachable": [],
        "unreachableAttachments": 0
      }
    ],
    "rotation_reveal": [
      {
        "hideAfterSeconds": 60,
        "value": "q7#Vt2-new"
      }
    ],
    "rotation_set_policy": [
      null
    ],
    "rotation_snooze": [
      null
    ],
    "rotation_status": [
      {
        "job": null
      },
      {
        "job": {
          "done": 3,
          "kind": "secretKey",
          "needs": "password",
          "phase": "prepared",
          "total": 12
        }
      },
      {
        "job": {
          "done": 3,
          "kind": "rekey",
          "needs": "kitGroup",
          "phase": "backendAccepted",
          "total": 12
        }
      },
      {
        "job": {
          "done": 3,
          "kind": "rekey",
          "needs": "none",
          "phase": "attachments",
          "total": 12
        }
      },
      {
        "job": {
          "done": 3,
          "kind": "profile",
          "needs": "none",
          "phase": "localCommitted",
          "total": 12
        }
      }
    ],
    "rotation_undismiss": [
      null
    ],
    "save_emergency_kit": [
      {
        "kind": "saved"
      },
      {
        "kind": "cancelled"
      }
    ],
    "secret_key_rotate_commit": [
      {
        "finished": true,
        "hardwareKeysRemoved": 1,
        "quickUnlockRemoved": true,
        "unreachable": [
          {
            "fileName": "statement.pdf",
            "itemTitle": "Bank"
          },
          {
            "fileName": null,
            "itemTitle": null
          }
        ],
        "unreachableAttachments": 2
      },
      {
        "finished": false,
        "hardwareKeysRemoved": 0,
        "quickUnlockRemoved": false,
        "unreachable": [],
        "unreachableAttachments": 0
      }
    ],
    "secret_key_rotate_prepare": [
      null
    ],
    "security_report": [
      {
        "counts": {
          "breached": 1,
          "high": 2,
          "insecureUrl": 1,
          "low": 0,
          "medium": 0,
          "missingTotp": 2,
          "old": 1,
          "reused": 2,
          "reusedGroups": 1,
          "weak": 1
        },
        "findings": [
          {
            "insecureHost": "shop.example.com",
            "issues": [
              {
                "action": "changePassword",
                "kind": "weak",
                "severity": "high"
              },
              {
                "action": "changePassword",
                "kind": "reused",
                "severity": "high"
              },
              {
                "action": "changePassword",
                "kind": "breached",
                "severity": "high"
              },
              {
                "action": "setUpTotp",
                "kind": "missingTotp",
                "severity": "medium"
              },
              {
                "action": "reviewUrl",
                "kind": "insecureUrl",
                "severity": "medium"
              },
              {
                "action": "changePassword",
                "kind": "old",
                "severity": "low"
              }
            ],
            "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
            "reusedWith": [
              {
                "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
                "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
              }
            ],
            "totpSite": {
              "documentation": "https://docs.github.com/en/authentication/securing-your-account-with-two-factor-authentication-2fa",
              "name": "GitHub"
            },
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          },
          {
            "insecureHost": null,
            "issues": [
              {
                "action": "changePassword",
                "kind": "reused",
                "severity": "high"
              },
              {
                "action": "setUpTotp",
                "kind": "missingTotp",
                "severity": "low"
              }
            ],
            "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
            "reusedWith": [
              {
                "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
                "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
              }
            ],
            "totpSite": {
              "documentation": null,
              "name": "Example"
            },
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ],
        "generatedAt": 1761480000000,
        "itemsChecked": 42
      }
    ],
    "session_state": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "kitVerified": true,
        "state": "unlocked"
      }
    ],
    "set_favorite": [
      {
        "favorite": true,
        "hasOtp": true,
        "host": "github.com",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "kind": "login",
        "modifiedAt": 1761480000000,
        "needsReview": false,
        "subtitle": "alex@example.com",
        "tags": [
          "work"
        ],
        "title": "GitHub",
        "trashedAt": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "setup_code_show": [
      {
        "expiresAt": 1762689660000,
        "modules": [
          [
            true,
            false
          ],
          [
            false,
            true
          ]
        ],
        "size": 2
      }
    ],
    "sharing_accept_key_change": [
      null
    ],
    "sharing_contacts": [
      {
        "contacts": [
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
            "changed": false,
            "email": "sam@example.com",
            "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
            "verified": true
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "changed": true,
            "email": "kim@example.com",
            "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
            "verified": false
          }
        ]
      },
      {
        "contacts": []
      }
    ],
    "sharing_identity": [
      {
        "published": true,
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V"
      },
      {
        "published": false,
        "safetyCode": null
      }
    ],
    "sharing_lookup": [
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "email": "sam@example.com",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "status": "new"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "email": "sam@example.com",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "status": "pinned"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "email": "sam@example.com",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "status": "verified"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "email": "sam@example.com",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "status": "changed"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
        "email": "sam@example.com",
        "safetyCode": "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V",
        "status": "conflicting"
      }
    ],
    "sharing_verify_contact": [
      null
    ],
    "ssh_agent_set_enabled": [
      {
        "enabled": true,
        "endpoint": "\\\\.\\pipe\\openssh-ssh-agent",
        "error": null,
        "keyCount": 2,
        "running": true,
        "skipped": [
          {
            "itemId": "019a2b3c-4d5e-7020-8000-000000000004",
            "reason": "unsupported",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ]
      },
      {
        "enabled": false,
        "endpoint": null,
        "error": "sshAgentEndpointInUse",
        "keyCount": 0,
        "running": false,
        "skipped": []
      }
    ],
    "ssh_agent_status": [
      {
        "enabled": true,
        "endpoint": "\\\\.\\pipe\\openssh-ssh-agent",
        "error": null,
        "keyCount": 2,
        "running": true,
        "skipped": [
          {
            "itemId": "019a2b3c-4d5e-7020-8000-000000000004",
            "reason": "unsupported",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ]
      },
      {
        "enabled": false,
        "endpoint": null,
        "error": "sshAgentEndpointInUse",
        "keyCount": 0,
        "running": false,
        "skipped": []
      }
    ],
    "ssh_respond": [
      null
    ],
    "sync_acknowledge": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_check_server": [
      {
        "registration": "open",
        "serverUrl": "https://sync.example.com"
      },
      {
        "registration": "invite",
        "serverUrl": "https://sync.example.com"
      },
      {
        "registration": "closed",
        "serverUrl": "https://sync.example.com"
      }
    ],
    "sync_device_remove": [
      null
    ],
    "sync_device_rename": [
      null
    ],
    "sync_devices": [
      {
        "devices": [
          {
            "canRemove": false,
            "current": true,
            "deviceId": "019a2b3c-4d5e-7050-8000-000000000001",
            "lastSeenAt": 1761480000000,
            "name": "Mosaic on Windows",
            "platform": "windows"
          },
          {
            "canRemove": true,
            "current": false,
            "deviceId": "019a2b3c-4d5e-7050-8000-000000000002",
            "lastSeenAt": 1761476400000,
            "name": "Mosaic on macOS",
            "platform": null
          },
          {
            "canRemove": false,
            "current": false,
            "deviceId": "019a2b3c-4d5e-7050-8000-000000000003",
            "lastSeenAt": null,
            "name": null,
            "platform": null
          }
        ]
      }
    ],
    "sync_disconnect": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_dismiss_account_file": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_dismiss_rekey_marker": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_folder_connect": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_hosted_connect": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_hosted_two_factor": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_hosted_webauthn_begin": [
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913"
      }
    ],
    "sync_ignored_rekey_markers": [
      [],
      [
        {
          "name": "rekey-00000002.mrk",
          "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
          "toEpoch": 2
        }
      ]
    ],
    "sync_login_methods": [
      {
        "totp": true,
        "webauthn": false
      },
      {
        "totp": true,
        "webauthn": true
      },
      {
        "totp": false,
        "webauthn": true
      }
    ],
    "sync_now": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_pick_folder": [
      null,
      {
        "accounts": [
          "019a2b3c-4d5e-7030-8000-000000000001"
        ],
        "choiceId": "choice-1",
        "contents": "mosaic",
        "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic"
      },
      {
        "accounts": [],
        "choiceId": "choice-2",
        "contents": "empty",
        "displayPath": "D:\\Sync"
      },
      {
        "accounts": [],
        "choiceId": "choice-3",
        "contents": "other",
        "displayPath": "D:\\Documents"
      }
    ],
    "sync_recover_password": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_rekey_markers": [
      [],
      [
        {
          "name": "rekey-00000002.mrk",
          "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
          "toEpoch": 2
        }
      ]
    ],
    "sync_restore_rekey_marker": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_status": [
      {
        "config": null,
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": null,
          "status": "notConfigured"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": true,
          "webauthnKeys": 2
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "upToDate"
        },
        "warnings": []
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "displayPath": "C:\\Users\\alex\\Dropbox\\Mosaic",
          "kind": "folder"
        },
        "passwordChanged": false,
        "rekeyPending": false,
        "revoked": null,
        "sharingFindings": [
          {
            "kind": "identityMismatch"
          },
          {
            "kind": "rosterInvalid",
            "vaultName": "Family"
          },
          {
            "kind": "shareUntrusted",
            "senderEmail": "sam@example.com",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "kind": "memberUnverified",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": "lee@example.com",
            "kind": "memberKeyMismatch",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "vaultHeld",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": null
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": null,
            "kind": "memberDropped",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001",
            "vaultName": "Family"
          },
          {
            "kind": "accessLost",
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000002",
            "vaultName": "Work"
          }
        ],
        "state": {
          "error": null,
          "lastSyncedAt": 1761480000000,
          "status": "syncing"
        },
        "warnings": [
          {
            "deviceName": "Mosaic on macOS",
            "kind": "freezeSuspected",
            "since": 1761480000000
          },
          {
            "deviceName": null,
            "kind": "missingFiles",
            "since": 1761480000000
          },
          {
            "deviceName": "Mosaic on Linux",
            "kind": "noCommonView",
            "since": 1761480000000
          },
          {
            "kind": "rollbackDetected"
          },
          {
            "kind": "identityDuplicated"
          },
          {
            "kind": "serverInconsistent",
            "vaultName": "Personal"
          },
          {
            "kind": "serverRollback",
            "vaultName": null
          },
          {
            "count": 3,
            "kind": "undecryptable"
          },
          {
            "count": 1,
            "kind": "hardwareVaultsLocal"
          },
          {
            "count": 2,
            "kind": "attachmentsNotRepaired"
          },
          {
            "kind": "rotationBlocked",
            "vaultName": "Family"
          },
          {
            "conflict": false,
            "kind": "unverifiedAccountFile",
            "name": "account-00000002.macct",
            "sha256": "abababababababababababababababababababababababababababababababab"
          },
          {
            "conflict": true,
            "kind": "unverifiedAccountFile",
            "name": "account-00000001.macct",
            "sha256": "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
          }
        ]
      },
      {
        "config": {
          "deviceName": "Mosaic on Windows",
          "email": "alex@example.com",
          "kind": "hosted",
          "serverUrl": "https://sync.example.com",
          "totpEnabled": false,
          "webauthnKeys": 0
        },
        "passwordChanged": true,
        "rekeyPending": true,
        "revoked": "rekey",
        "sharingFindings": [],
        "state": {
          "error": "syncSignedOut",
          "lastSyncedAt": 1761480000000,
          "status": "signedOut"
        },
        "warnings": []
      }
    ],
    "sync_totp_begin": [
      {
        "otpauthUri": "otpauth://totp/Mosaic%20sync:alex@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Mosaic%20sync",
        "secret": "JBSWY3DPEHPK3PXP"
      }
    ],
    "sync_totp_confirm": [
      null
    ],
    "sync_totp_disable": [
      null
    ],
    "sync_webauthn_add_begin": [
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "login"
      },
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "register"
      },
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "confirm"
      }
    ],
    "sync_webauthn_add_status": [
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "state": "waiting",
        "step": "register"
      },
      {
        "expiresAt": null,
        "matchCode": null,
        "state": "done",
        "step": "confirm"
      }
    ],
    "sync_webauthn_confirm_begin": [
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "login"
      },
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "register"
      },
      {
        "expiresAt": 1761480300000,
        "matchCode": "482913",
        "step": "confirm"
      }
    ],
    "sync_webauthn_list": [
      {
        "keys": [],
        "totp": false
      },
      {
        "keys": [
          {
            "createdAt": 1761480000000,
            "credentialRef": "00112233445566778899aabbccddeeff",
            "label": "YubiKey 5C",
            "lastUsedAt": 1761480060000
          },
          {
            "createdAt": 1761480001000,
            "credentialRef": "ffeeddccbbaa99887766554433221100",
            "label": "Backup key",
            "lastUsedAt": null
          }
        ],
        "totp": true
      }
    ],
    "sync_webauthn_remove": [
      null
    ],
    "sync_webauthn_reopen": [
      null
    ],
    "sync_webauthn_status": [
      {
        "state": "waiting"
      },
      {
        "state": "done"
      },
      {
        "state": "failed"
      },
      {
        "state": "expired"
      }
    ],
    "totp_code": [
      {
        "code": "492039",
        "period": 30,
        "secondsRemaining": 17
      },
      {
        "code": "755224",
        "period": null,
        "secondsRemaining": null
      }
    ],
    "trash_item": [
      null
    ],
    "unlock": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "kitVerified": true,
        "state": "unlocked"
      }
    ],
    "unlock_quick": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "kitVerified": true,
        "state": "unlocked"
      }
    ],
    "update_item": [
      {
        "attachments": [
          {
            "addedAt": 1800000000000,
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "available": true,
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "previewable": false,
            "size": 512
          }
        ],
        "changeUrl": {
          "source": "wellKnown",
          "url": "https://example.com/.well-known/change-password"
        },
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "history": [
          {
            "changedAt": 1761480100000,
            "historyId": "00000000000000a2.0123456789abcdef.0",
            "index": 0,
            "reason": "unconfirmed"
          },
          {
            "changedAt": 1761400000000,
            "historyId": "00000000000000a0.0123456789abcdef.0",
            "index": 1,
            "reason": "replaced"
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "passkey": null,
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "rotationAttempts": [
          {
            "attemptId": "00000000000000a1.0123456789abcdef.0",
            "stage": "prepared",
            "startedAt": 1761480000000
          }
        ],
        "rotationPolicy": 90,
        "secrets": [
          "password"
        ],
        "sshKey": null,
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "update_settings": [
      {
        "autoLockMinutes": 10,
        "breachCheck": "off",
        "breachConsentAt": null,
        "clipboardClearSeconds": 30,
        "globalShortcut": "Ctrl+Shift+Space",
        "language": "system",
        "lockOnScreenLock": true,
        "lockOnSleep": true,
        "quickUnlockMaxAgeDays": 14,
        "rotationReminders": true,
        "theme": "system"
      },
      {
        "autoLockMinutes": null,
        "breachCheck": "weekly",
        "breachConsentAt": 1761000000000,
        "clipboardClearSeconds": null,
        "globalShortcut": "Ctrl+Alt+K",
        "language": "de",
        "lockOnScreenLock": false,
        "lockOnSleep": false,
        "quickUnlockMaxAgeDays": 30,
        "rotationReminders": false,
        "theme": "dark"
      }
    ],
    "vault_invite_cancel": [
      null
    ],
    "vault_leave": [
      null
    ],
    "vault_member_remove": [
      {
        "unreachableAttachments": 0
      },
      {
        "unreachableAttachments": 3
      }
    ],
    "vault_member_set_role": [
      null
    ],
    "vault_members": [
      {
        "members": [
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
            "email": "alex@example.com",
            "origin": "owner",
            "role": "owner",
            "self": true,
            "state": "member",
            "verified": true
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000002",
            "email": "sam@example.com",
            "origin": "invite",
            "role": "write",
            "self": false,
            "state": "member",
            "verified": true
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000003",
            "email": "kim@example.com",
            "origin": null,
            "role": "read",
            "self": false,
            "state": "invited",
            "verified": false
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000004",
            "email": null,
            "origin": "emergency",
            "role": "manage",
            "self": false,
            "state": "unconfirmed",
            "verified": false
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000005",
            "email": "lee@example.com",
            "origin": null,
            "role": "read",
            "self": false,
            "state": "dropped",
            "verified": true
          },
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000006",
            "email": "max@example.com",
            "origin": "invite",
            "role": "read",
            "self": false,
            "state": "removing",
            "verified": true
          }
        ],
        "ownRole": "owner",
        "pendingRotation": true,
        "unprotectedAttachments": 2
      },
      {
        "members": [
          {
            "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
            "email": "alex@example.com",
            "origin": "owner",
            "role": "owner",
            "self": true,
            "state": "member",
            "verified": true
          }
        ],
        "ownRole": "manage",
        "pendingRotation": false,
        "unprotectedAttachments": 0
      }
    ],
    "vault_rotate_key": [
      {
        "finished": true,
        "hardwareKeysRemoved": 1,
        "quickUnlockRemoved": true,
        "unreachable": [
          {
            "fileName": "statement.pdf",
            "itemTitle": "Bank"
          },
          {
            "fileName": null,
            "itemTitle": null
          }
        ],
        "unreachableAttachments": 2
      },
      {
        "finished": false,
        "hardwareKeysRemoved": 0,
        "quickUnlockRemoved": false,
        "unreachable": [],
        "unreachableAttachments": 0
      }
    ],
    "vault_share": [
      {
        "expiresAt": 1762084800000,
        "inviteId": "019a2b3c-4d5e-7060-8000-000000000001"
      }
    ],
    "verify_emergency_kit": [
      {
        "matches": true
      },
      {
        "matches": false
      }
    ]
  }
} satisfies ContractFixtures;
