/**
 * Size limits of a passkey request (docs/EXTENSION_SECURITY.md, "Passkeys"), shared by the
 * page shim, the bridge and the background so all three refuse the same requests. They equal
 * the limits the desktop applies to `passkeyCreate` and `passkeyGet`
 * (docs/NATIVE_MESSAGING.md §5.2). Byte limits are for decoded values; the wire form is
 * base64url without padding.
 */
export const CHALLENGE_MIN_BYTES = 16;
export const CHALLENGE_MAX_BYTES = 1024;
export const USER_ID_MIN_BYTES = 1;
export const USER_ID_MAX_BYTES = 64;
/** UTF-16 code units of `user.name` and `user.displayName`. */
export const USER_TEXT_MAX = 256;
export const RP_ID_MAX = 253;
export const ALGORITHMS_MAX = 16;
/** `excludeCredentials` and `allowCredentials`. */
export const CREDENTIAL_LIST_MAX = 64;
export const CREDENTIAL_ID_MIN_BYTES = 1;
export const CREDENTIAL_ID_MAX_BYTES = 1023;
/** Requests one page may have open at once (the bridge refuses more). */
export const PAGE_MAX_IN_FLIGHT = 2;
/** The shim falls back to the browser when the bridge has not acknowledged a request this fast. */
export const BRIDGE_ACK_TIMEOUT_MS = 1000;
