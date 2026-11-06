/**
 * Messages of the passkey path (docs/EXTENSION_SECURITY.md, "Passkeys"):
 *
 *   page shim (main world)  <-- window.postMessage -->  bridge (isolated world)
 *   bridge  <-- runtime.sendMessage -->  background
 *
 * The page can see, forge and answer everything on the first hop, by design: it gains nothing
 * it does not already have through `navigator.credentials` for its own origin. The trust
 * boundary is the second hop, where the background reads origin and frame from the browser.
 * Every value is validated with exact keys on every side.
 */
import { isB64urlBytes } from "./b64url.ts";
import {
  ALGORITHMS_MAX,
  CHALLENGE_MAX_BYTES,
  CHALLENGE_MIN_BYTES,
  CREDENTIAL_ID_MAX_BYTES,
  CREDENTIAL_ID_MIN_BYTES,
  CREDENTIAL_LIST_MAX,
  RP_ID_MAX,
  USER_ID_MAX_BYTES,
  USER_ID_MIN_BYTES,
  USER_TEXT_MAX,
} from "./passkey-limits.ts";
import type { UserVerification } from "./protocol.ts";
import { arr, exactly, int, isRecord, lit, nullable, obj, str, tagged, type Guard } from "./validate.ts";

export type PasskeyOp = "create" | "get";

/** The `type` of a message from the shim to the bridge. */
export const PASSKEY_REQUEST_TYPE = "mosaic-passkey-req";
export const PASSKEY_ABORT_TYPE = "mosaic-passkey-abort";
/** The bridge to the shim: the bridge is there and has the request. */
export const PASSKEY_ACK_TYPE = "mosaic-passkey-ack";
export const PASSKEY_RESPONSE_TYPE = "mosaic-passkey-res";

/** `navigator.credentials.create({publicKey})`, reduced to what the desktop needs (binary values are base64url). */
export interface SerializedCreate {
  rpId: string | null;
  userId: string;
  userName: string;
  userDisplayName: string;
  challenge: string;
  algorithms: number[];
  excludeCredentials: string[];
  userVerification: UserVerification;
}

/** `navigator.credentials.get({publicKey})`, reduced the same way. */
export interface SerializedGet {
  rpId: string | null;
  challenge: string;
  allowCredentials: string[];
  userVerification: UserVerification;
}

export type PageRequestId = number;

/** Shim to bridge. */
export type ShimRequest =
  | { type: typeof PASSKEY_REQUEST_TYPE; id: PageRequestId; op: "create"; options: SerializedCreate }
  | { type: typeof PASSKEY_REQUEST_TYPE; id: PageRequestId; op: "get"; options: SerializedGet };

export interface ShimAbort {
  type: typeof PASSKEY_ABORT_TYPE;
  id: PageRequestId;
}

export interface ShimAck {
  type: typeof PASSKEY_ACK_TYPE;
  id: PageRequestId;
}

/** The credential a registration produced, as the shim builds the page's `PublicKeyCredential` from it. */
export interface CreatedResult {
  kind: "created";
  credentialId: string;
  clientDataJson: string;
  attestationObject: string;
  authenticatorData: string;
  publicKeySpki: string;
  publicKeyAlgorithm: number;
}

export interface AssertedResult {
  kind: "asserted";
  credentialId: string;
  clientDataJson: string;
  authenticatorData: string;
  signature: string;
  userHandle: string;
}

/** What the page's promise becomes. `fallback` hands the call to the browser's own WebAuthn. */
export type PasskeyPageResult =
  | { kind: "fallback" }
  | { kind: "error"; name: "NotAllowedError" | "InvalidStateError" }
  | CreatedResult
  | AssertedResult;

export interface ShimResponse {
  type: typeof PASSKEY_RESPONSE_TYPE;
  id: PageRequestId;
  result: PasskeyPageResult;
}

/** Bridge to background. `origin` is the bridge's `window.origin`; the background compares it with the sender. */
export type PasskeyMessage =
  | { kind: "passkey"; op: "create"; pageRequestId: PageRequestId; origin: string; options: SerializedCreate }
  | { kind: "passkey"; op: "get"; pageRequestId: PageRequestId; origin: string; options: SerializedGet };

export interface PasskeyAbortMessage {
  kind: "passkeyAbort";
  pageRequestId: PageRequestId;
}

const bytes = isB64urlBytes;
const pageId = int(0, 2_147_483_647);
const userVerification = lit("required", "preferred", "discouraged");
const credentialId = bytes(CREDENTIAL_ID_MIN_BYTES, CREDENTIAL_ID_MAX_BYTES);
const challenge = bytes(CHALLENGE_MIN_BYTES, CHALLENGE_MAX_BYTES);
const rpId = nullable(str(RP_ID_MAX));

export const isSerializedCreate: Guard<SerializedCreate> = obj({
  rpId,
  userId: bytes(USER_ID_MIN_BYTES, USER_ID_MAX_BYTES),
  userName: str(USER_TEXT_MAX),
  userDisplayName: str(USER_TEXT_MAX),
  challenge,
  algorithms: arr(int(-65_536, 65_536), ALGORITHMS_MAX),
  excludeCredentials: arr(credentialId, CREDENTIAL_LIST_MAX),
  userVerification,
});

export const isSerializedGet: Guard<SerializedGet> = obj({
  rpId,
  challenge,
  allowCredentials: arr(credentialId, CREDENTIAL_LIST_MAX),
  userVerification,
});

export const isShimRequest: Guard<ShimRequest> = tagged<ShimRequest>("op", {
  create: obj({ type: exactly(PASSKEY_REQUEST_TYPE), id: pageId, op: exactly("create" as const), options: isSerializedCreate }),
  get: obj({ type: exactly(PASSKEY_REQUEST_TYPE), id: pageId, op: exactly("get" as const), options: isSerializedGet }),
});

export const isShimAbort: Guard<ShimAbort> = obj({ type: exactly(PASSKEY_ABORT_TYPE), id: pageId });
export const isShimAck: Guard<ShimAck> = obj({ type: exactly(PASSKEY_ACK_TYPE), id: pageId });

export const isPasskeyPageResult: Guard<PasskeyPageResult> = tagged<PasskeyPageResult>("kind", {
  fallback: obj({ kind: exactly("fallback" as const) }),
  error: obj({ kind: exactly("error" as const), name: lit("NotAllowedError", "InvalidStateError") }),
  created: obj({
    kind: exactly("created" as const),
    credentialId,
    clientDataJson: bytes(1, 6144),
    attestationObject: bytes(1, 3072),
    authenticatorData: bytes(1, 3072),
    publicKeySpki: bytes(1, 384),
    publicKeyAlgorithm: int(-65_536, 65_536),
  }),
  asserted: obj({
    kind: exactly("asserted" as const),
    credentialId,
    clientDataJson: bytes(1, 6144),
    authenticatorData: bytes(1, 3072),
    signature: bytes(1, 384),
    userHandle: bytes(USER_ID_MIN_BYTES, USER_ID_MAX_BYTES),
  }),
});

export const isShimResponse: Guard<ShimResponse> = obj({
  type: exactly(PASSKEY_RESPONSE_TYPE),
  id: pageId,
  result: isPasskeyPageResult,
});

export const isPasskeyMessage: Guard<PasskeyMessage> = tagged<PasskeyMessage>("op", {
  create: obj({
    kind: exactly("passkey" as const),
    op: exactly("create" as const),
    pageRequestId: pageId,
    origin: str(2048),
    options: isSerializedCreate,
  }),
  get: obj({
    kind: exactly("passkey" as const),
    op: exactly("get" as const),
    pageRequestId: pageId,
    origin: str(2048),
    options: isSerializedGet,
  }),
});

export const isPasskeyAbortMessage: Guard<PasskeyAbortMessage> = obj({
  kind: exactly("passkeyAbort" as const),
  pageRequestId: pageId,
});

/** Whether a router message is one of the two passkey messages (the router routes on this). */
export const isPasskeyKind = (message: unknown): boolean =>
  isRecord(message) && (message["kind"] === "passkey" || message["kind"] === "passkeyAbort");
