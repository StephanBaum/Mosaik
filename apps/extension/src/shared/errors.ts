/**
 * Error codes (docs/NATIVE_MESSAGING.md §5.3). The wire codes come from the desktop and the
 * host; the local codes are decided in the extension and never travel on the wire.
 */
export const WIRE_ERROR_CODES = [
  "invalidRequest",
  "notPaired",
  "protocolVersion",
  "busy",
  "locked",
  "rateLimited",
  "originMismatch",
  "insecureOrigin",
  "crossOriginFrame",
  "itemNotFound",
  "vaultUnavailable",
  "noOtp",
  "otpNotFillable",
  "fieldEmpty",
  "clipboardUnavailable",
  "cancelled",
  "passkeyNotFound",
  "passkeyExcluded",
  "rpIdMismatch",
  "passkeyUnsupported",
  "rotationNotFound",
  "originNotRelated",
  "rotationLimit",
  "pairingDenied",
  "pairingExpired",
  "pairingConflict",
  "pairingBlocked",
  "pairingLimit",
  "internal",
  "desktopNotRunning",
  "desktopNotAuthentic",
  "timeout",
  "frameTooLarge",
] as const;

export const LOCAL_ERROR_CODES = [
  "hostMissing",
  "hostForbidden",
  "hostExited",
  "unsupportedPage",
  "noFields",
  "noMatch",
  "frameNeedsConfirmation",
  "frameOpaque",
  "originChanged",
  "pageHidden",
  "permissionDenied",
  "generatedExpired",
  "changeFormNotFound",
  "changeFormAmbiguous",
] as const;

export type WireErrorCode = (typeof WIRE_ERROR_CODES)[number];
export type LocalErrorCode = (typeof LOCAL_ERROR_CODES)[number];
export type ErrorCode = WireErrorCode | LocalErrorCode;

const wire: ReadonlySet<unknown> = new Set(WIRE_ERROR_CODES);
const all: ReadonlySet<unknown> = new Set([...WIRE_ERROR_CODES, ...LOCAL_ERROR_CODES]);

export function isWireErrorCode(value: unknown): value is WireErrorCode {
  return wire.has(value);
}

export function isErrorCode(value: unknown): value is ErrorCode {
  return all.has(value);
}

/** A failure with a code and nothing else: the message never carries page or vault data. */
export class ExtensionError extends Error {
  readonly code: ErrorCode;

  constructor(code: ErrorCode) {
    super(code);
    this.name = "ExtensionError";
    this.code = code;
  }
}

/** The code of any thrown value (`internal` for anything that is not an `ExtensionError`). */
export function errorCodeOf(error: unknown): ErrorCode {
  return error instanceof ExtensionError ? error.code : "internal";
}
