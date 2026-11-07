/**
 * Shared contract types (docs/DESKTOP_IPC.md). Mirrors
 * `src-tauri/src/contract/{ids,errors}.rs`; `contract.fixtures.ts` keeps the two in sync
 * (ADR-0018).
 *
 * Wire rules: camelCase keys, every key always present (`null` for "no value"), IDs are
 * canonical UUID strings, times are Unix milliseconds.
 */

/** Milliseconds since the Unix epoch. */
export type UnixMillis = number;

/** Canonical lowercase hyphenated UUIDs. */
export type AccountId = string;
export type VaultId = string;
export type ItemId = string;
export type AttachmentId = string;

/** Why a command failed. The UI shows the i18n message `error.<code>`. */
export const errorCodes = [
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
  "busy",
  "internal",
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
] as const;
export type ErrorCode = (typeof errorCodes)[number];

/** A failed command: a code and nothing else, so it can never carry a secret. */
export interface IpcError {
  code: ErrorCode;
}

/** The result of every command. */
export type IpcResult<T> = { ok: true; value: T } | { ok: false; error: IpcError };

export function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === "string" && (errorCodes as readonly string[]).includes(value);
}
