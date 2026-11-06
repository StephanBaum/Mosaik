/**
 * Reads the options of `navigator.credentials.create/get({publicKey})` into the plain,
 * size-limited form that travels to the bridge. Page-controlled input: any getter may throw
 * and any value may be of the wrong type. `null` means "not ours to handle": the shim then
 * hands the call to the browser's own WebAuthn with the page's original arguments.
 *
 * Handled: ES256 registrations and sign-ins on this origin's relying party. Left to the
 * browser: conditional and silent mediation, a cross-platform authenticator, enterprise
 * attestation, the PRF and large-blob extensions, and anything beyond the size limits.
 */
import { encodeB64url } from "../shared/b64url.ts";
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
} from "../shared/passkey-limits.ts";
import type { SerializedCreate, SerializedGet } from "../shared/passkey-messages.ts";
import type { UserVerification } from "../shared/protocol.ts";

/** Taken at load time (the script runs before any page script), so a page cannot swap them later. */
const isView = ArrayBuffer.isView.bind(ArrayBuffer);
const isArray = Array.isArray.bind(Array);
const isInteger = Number.isInteger.bind(Number);
const applyFn = Reflect.apply.bind(Reflect) as (target: object, thisArg: unknown, args: unknown[]) => unknown;
// eslint-disable-next-line @typescript-eslint/unbound-method -- applied with an explicit `this` (a page cannot swap what was taken here)
const tagOf = Object.prototype.toString;
const Bytes = Uint8Array;

class Unsupported extends Error {}

const unsupported = (): never => {
  throw new Unsupported();
};

type Dict = Record<string, unknown>;

const isDict = (value: unknown): value is Dict => typeof value === "object" && value !== null;

const dict = (value: unknown): Dict => (isDict(value) ? value : unsupported());

/** The bytes of a BufferSource (an ArrayBuffer or any view, honouring offset and length), base64url, size-checked. */
function bufferSource(value: unknown, min: number, max: number): string {
  let view: Uint8Array;
  if (isView(value)) {
    view = new Bytes(value.buffer, value.byteOffset, value.byteLength);
  } else if (applyFn(tagOf, value, []) === "[object ArrayBuffer]") {
    view = new Bytes(value as ArrayBuffer);
  } else {
    return unsupported();
  }
  if (view.length < min || view.length > max || applyFn(tagOf, view.buffer, []) === "[object SharedArrayBuffer]") {
    return unsupported();
  }
  return encodeB64url(view);
}

function text(value: unknown, max: number): string {
  return typeof value === "string" && value.length <= max ? value : unsupported();
}

/** `undefined` is absent; anything else must be text within the limit. */
const optionalRpId = (value: unknown): string | null => (value === undefined ? null : text(value, RP_ID_MAX));

function list(value: unknown, max: number): unknown[] {
  if (value === undefined) {
    return [];
  }
  if (!isArray(value) || value.length > max) {
    return unsupported();
  }
  return value;
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  if (value === undefined) {
    return fallback;
  }
  return allowed.find((candidate) => candidate === value) ?? unsupported();
}

function checkMediation(options: Dict): void {
  oneOf(options["mediation"], ["optional", "required"], "optional");
}

/** `{type, id}` entries of an exclude or allow list; entries of another type are ignored (WebAuthn). */
function credentialIds(value: unknown): string[] {
  const entries = list(value, CREDENTIAL_LIST_MAX);
  const ids = entries
    .map(dict)
    .filter((entry) => entry["type"] === "public-key")
    .map((entry) => bufferSource(entry["id"], CREDENTIAL_ID_MIN_BYTES, CREDENTIAL_ID_MAX_BYTES));
  // A list that named only credentials of other types would turn into "any credential".
  return entries.length > 0 && ids.length === 0 ? unsupported() : ids;
}

function algorithms(value: unknown): number[] {
  const entries = list(value, ALGORITHMS_MAX).map(dict);
  const algs = entries
    .filter((entry) => entry["type"] === "public-key")
    .map((entry) => (isInteger(entry["alg"]) ? (entry["alg"] as number) : unsupported()));
  return entries.length > 0 && algs.length === 0 ? unsupported() : algs;
}

function checkExtensions(value: unknown): boolean {
  if (value === undefined) {
    return false;
  }
  const extensions = dict(value);
  if (extensions["prf"] !== undefined || extensions["largeBlob"] !== undefined) {
    return unsupported();
  }
  return extensions["credProps"] === true;
}

const VERIFICATION: readonly UserVerification[] = ["required", "preferred", "discouraged"];

/** What `create` needs, and whether the page asked for the `credProps` result. */
export interface PreparedCreate {
  wire: SerializedCreate;
  credProps: boolean;
}

function createWire(pk: Dict): SerializedCreate {
  const rp = dict(pk["rp"]);
  const user = dict(pk["user"]);
  const selection = pk["authenticatorSelection"] === undefined ? {} : dict(pk["authenticatorSelection"]);
  if (selection["authenticatorAttachment"] === "cross-platform" || pk["attestation"] === "enterprise") {
    return unsupported();
  }
  return {
    rpId: optionalRpId(rp["id"]),
    userId: bufferSource(user["id"], USER_ID_MIN_BYTES, USER_ID_MAX_BYTES),
    userName: text(user["name"], USER_TEXT_MAX),
    userDisplayName: text(user["displayName"], USER_TEXT_MAX),
    challenge: bufferSource(pk["challenge"], CHALLENGE_MIN_BYTES, CHALLENGE_MAX_BYTES),
    algorithms: algorithms(pk["pubKeyCredParams"]),
    excludeCredentials: credentialIds(pk["excludeCredentials"]),
    userVerification: oneOf(selection["userVerification"], VERIFICATION, "preferred"),
  };
}

export function prepareCreate(options: unknown): PreparedCreate | null {
  try {
    const top = dict(options);
    checkMediation(top);
    const pk = dict(top["publicKey"]);
    return { wire: createWire(pk), credProps: checkExtensions(pk["extensions"]) };
  } catch {
    return null;
  }
}

export function prepareGet(options: unknown): SerializedGet | null {
  try {
    const top = dict(options);
    checkMediation(top);
    const pk = dict(top["publicKey"]);
    checkExtensions(pk["extensions"]);
    return {
      rpId: optionalRpId(pk["rpId"]),
      challenge: bufferSource(pk["challenge"], CHALLENGE_MIN_BYTES, CHALLENGE_MAX_BYTES),
      allowCredentials: credentialIds(pk["allowCredentials"]),
      userVerification: oneOf(pk["userVerification"], VERIFICATION, "preferred"),
    };
  } catch {
    return null;
  }
}

/** The page's `AbortSignal` (anything with a boolean `aborted`), or `null`. Never throws. */
export function signalOf(options: unknown): AbortSignal | null {
  try {
    const signal = isDict(options) ? options["signal"] : undefined;
    return isDict(signal) && typeof signal["aborted"] === "boolean" ? (signal as unknown as AbortSignal) : null;
  } catch {
    return null;
  }
}
