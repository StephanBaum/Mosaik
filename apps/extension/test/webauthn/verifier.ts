/**
 * Independent WebAuthn registration and assertion verifier for the tests (M5).
 *
 * Written without any Mosaic code and without a CBOR or WebAuthn library: it
 * has its own CBOR reader (maps, byte and text strings, small integers), its
 * own authenticator-data and COSE parsers, and checks the ES256 signature with
 * `node:crypto` only. It is the second implementation behind the known-answer
 * vectors of CRYPTO_SPEC section 16, so a mistake in the Rust encoders cannot
 * hide behind the same mistake in the checker. Test code, never shipped.
 */
import { createHash, createPublicKey, verify as cryptoVerify } from "node:crypto";
import type { JsonWebKey } from "node:crypto";

export type CborValue = number | string | Uint8Array | Map<CborKey, CborValue>;
export type CborKey = number | string;

const FLAG_UP = 0x01;
const FLAG_UV = 0x04;
const FLAG_BE = 0x08;
const FLAG_BS = 0x10;
const FLAG_AT = 0x40;
const FLAG_ED = 0x80;

/** What the verifier read from a CBOR item: the value and the bytes it used. */
interface Read {
  value: CborValue;
  next: number;
}

function fail(message: string): never {
  throw new Error(`webauthn verifier: ${message}`);
}

function readLength(bytes: Uint8Array, at: number, info: number): { len: number; next: number } {
  if (info < 24) {
    return { len: info, next: at };
  }
  const width = info === 24 ? 1 : info === 25 ? 2 : info === 26 ? 4 : 0;
  if (width === 0) {
    return fail(`unsupported CBOR length encoding ${String(info)}`);
  }
  if (at + width > bytes.length) {
    return fail("truncated CBOR length");
  }
  let len = 0;
  for (let i = 0; i < width; i += 1) {
    len = len * 256 + (bytes[at + i] ?? 0);
  }
  // Shortest form only (CTAP2 canonical CBOR).
  const shortest = width === 1 ? len >= 24 : width === 2 ? len >= 256 : len >= 65536;
  if (!shortest) {
    return fail("CBOR length not in shortest form");
  }
  return { len, next: at + width };
}

function readItem(bytes: Uint8Array, at: number): Read {
  const head = bytes[at];
  if (head === undefined) {
    return fail("truncated CBOR");
  }
  const major = head >> 5;
  const info = head & 0x1f;
  const { len, next } = readLength(bytes, at + 1, info);
  switch (major) {
    case 0:
      return { value: len, next };
    case 1:
      return { value: -1 - len, next };
    case 2:
    case 3: {
      if (next + len > bytes.length) {
        return fail("truncated CBOR string");
      }
      const slice = bytes.subarray(next, next + len);
      return {
        value: major === 2 ? new Uint8Array(slice) : new TextDecoder("utf-8", { fatal: true }).decode(slice),
        next: next + len,
      };
    }
    case 5: {
      const map = new Map<CborKey, CborValue>();
      let cursor = next;
      for (let i = 0; i < len; i += 1) {
        const key = readItem(bytes, cursor);
        if (typeof key.value !== "number" && typeof key.value !== "string") {
          return fail("CBOR map key is not an integer or text");
        }
        if (map.has(key.value)) {
          return fail("duplicate CBOR map key");
        }
        const val = readItem(bytes, key.next);
        map.set(key.value, val.value);
        cursor = val.next;
      }
      return { value: map, next: cursor };
    }
    default:
      return fail(`unsupported CBOR major type ${String(major)}`);
  }
}

/** Reads exactly one CBOR item; trailing bytes are an error. */
export function decodeCbor(bytes: Uint8Array): CborValue {
  const { value, next } = readItem(bytes, 0);
  if (next !== bytes.length) {
    fail("trailing bytes after CBOR item");
  }
  return value;
}

function asMap(value: CborValue | undefined, what: string): Map<CborKey, CborValue> {
  if (!(value instanceof Map)) {
    return fail(`${what} is not a map`);
  }
  return value;
}

function asBytes(value: CborValue | undefined, what: string): Uint8Array {
  // Compare by tag so a Uint8Array from another realm (jsdom) still passes.
  if (Object.prototype.toString.call(value) !== "[object Uint8Array]") {
    return fail(`${what} is not a byte string`);
  }
  return value as Uint8Array;
}

export interface ParsedAttestationObject {
  fmt: string;
  attStmt: Map<CborKey, CborValue>;
  authData: Uint8Array;
}

/** Parses an attestation object; the key order must be fmt, attStmt, authData. */
export function parseAttestationObject(bytes: Uint8Array): ParsedAttestationObject {
  const map = asMap(decodeCbor(bytes), "attestation object");
  if ([...map.keys()].join(",") !== "fmt,attStmt,authData") {
    fail("attestation object members are not fmt, attStmt, authData in that order");
  }
  const fmt = map.get("fmt");
  if (typeof fmt !== "string") {
    return fail("fmt is not text");
  }
  return {
    fmt,
    attStmt: asMap(map.get("attStmt"), "attStmt"),
    authData: asBytes(map.get("authData"), "authData"),
  };
}

export interface ParsedAuthData {
  rpIdHash: Uint8Array;
  flags: number;
  up: boolean;
  uv: boolean;
  be: boolean;
  bs: boolean;
  at: boolean;
  ed: boolean;
  signCount: number;
  attested: { aaguid: Uint8Array; credentialId: Uint8Array; cose: Uint8Array } | null;
}

/** Parses authenticator data; trailing bytes after the COSE key are an error (ED is never set). */
export function parseAuthData(bytes: Uint8Array): ParsedAuthData {
  if (bytes.length < 37) {
    fail("authenticator data shorter than 37 bytes");
  }
  const flags = bytes[32] ?? 0;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const parsed: ParsedAuthData = {
    rpIdHash: new Uint8Array(bytes.subarray(0, 32)),
    flags,
    up: (flags & FLAG_UP) !== 0,
    uv: (flags & FLAG_UV) !== 0,
    be: (flags & FLAG_BE) !== 0,
    bs: (flags & FLAG_BS) !== 0,
    at: (flags & FLAG_AT) !== 0,
    ed: (flags & FLAG_ED) !== 0,
    signCount: view.getUint32(33, false),
    attested: null,
  };
  if (parsed.ed) {
    fail("extension data flag is set");
  }
  if (!parsed.at) {
    if (bytes.length !== 37) {
      fail("trailing bytes after authenticator data");
    }
    return parsed;
  }
  if (bytes.length < 37 + 16 + 2) {
    fail("attested credential data truncated");
  }
  const idLen = view.getUint16(53, false);
  const idEnd = 55 + idLen;
  if (idEnd > bytes.length) {
    fail("credential id truncated");
  }
  const coseStart = idEnd;
  const cose = readItem(bytes, coseStart);
  if (cose.next !== bytes.length) {
    fail("trailing bytes after the COSE key");
  }
  parsed.attested = {
    aaguid: new Uint8Array(bytes.subarray(37, 53)),
    credentialId: new Uint8Array(bytes.subarray(55, idEnd)),
    cose: new Uint8Array(bytes.subarray(coseStart, cose.next)),
  };
  return parsed;
}

function b64url(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("base64url");
}

/** Converts an ES256 COSE_Key (kty 2, alg -7, crv 1) to a JWK; anything else throws. */
export function coseToJwk(bytes: Uint8Array): JsonWebKey {
  const map = asMap(decodeCbor(bytes), "COSE key");
  const expected: [number, number][] = [
    [1, 2],
    [3, -7],
    [-1, 1],
  ];
  for (const [key, want] of expected) {
    if (map.get(key) !== want) {
      fail(`COSE key parameter ${String(key)} is not ${String(want)}`);
    }
  }
  if (map.size !== 5) {
    fail("COSE key has extra parameters");
  }
  // CTAP2 canonical order: 1, 3, -1, -2, -3.
  if ([...map.keys()].join(",") !== "1,3,-1,-2,-3") {
    fail("COSE key members are not in CTAP2 canonical order");
  }
  const x = asBytes(map.get(-2), "COSE x");
  const y = asBytes(map.get(-3), "COSE y");
  if (x.length !== 32 || y.length !== 32) {
    fail("COSE coordinates are not 32 bytes");
  }
  return { kty: "EC", crv: "P-256", x: b64url(x), y: b64url(y) };
}

interface ClientDataExpectations {
  clientDataJSON: Uint8Array;
  expectedOrigin: string;
  expectedRpId: string;
  expectedChallenge: Uint8Array;
}

function sha256(bytes: Uint8Array): Buffer {
  return createHash("sha256").update(bytes).digest();
}

function checkClientData(type: string, e: ClientDataExpectations): void {
  const parsed: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(e.clientDataJSON));
  if (typeof parsed !== "object" || parsed === null) {
    fail("clientDataJSON is not an object");
  }
  const data = parsed as Record<string, unknown>;
  if (data["type"] !== type) {
    fail(`clientData type is not ${type}`);
  }
  if (data["challenge"] !== b64url(e.expectedChallenge)) {
    fail("clientData challenge differs");
  }
  if (data["origin"] !== e.expectedOrigin) {
    fail("clientData origin differs");
  }
  if (data["crossOrigin"] !== false) {
    fail("clientData crossOrigin is not false");
  }
  if ("topOrigin" in data) {
    fail("clientData has a topOrigin");
  }
}

function checkRpIdHash(authData: ParsedAuthData, rpId: string): void {
  if (!sha256(new TextEncoder().encode(rpId)).equals(Buffer.from(authData.rpIdHash))) {
    fail("rpIdHash differs");
  }
}

export interface RegistrationResult {
  jwk: JsonWebKey;
  credentialId: Uint8Array;
  flags: number;
  signCount: number;
}

/** Verifies a `none` registration: client data, rpIdHash, flags, COSE key, no statement. */
export function verifyRegistration(input: {
  attestationObject: Uint8Array;
  clientDataJSON: Uint8Array;
  expectedOrigin: string;
  expectedRpId: string;
  expectedChallenge: Uint8Array;
}): RegistrationResult {
  checkClientData("webauthn.create", input);
  const object = parseAttestationObject(input.attestationObject);
  if (object.fmt !== "none" || object.attStmt.size !== 0) {
    fail("attestation is not format none with an empty statement");
  }
  const authData = parseAuthData(object.authData);
  checkRpIdHash(authData, input.expectedRpId);
  if (!authData.up || !authData.at) {
    fail("UP and AT must be set in a registration");
  }
  if (!authData.attested) {
    return fail("no attested credential data");
  }
  if (authData.attested.credentialId.length < 16) {
    fail("credential id shorter than 16 bytes");
  }
  return {
    jwk: coseToJwk(authData.attested.cose),
    credentialId: authData.attested.credentialId,
    flags: authData.flags,
    signCount: authData.signCount,
  };
}

export interface AssertionResult {
  flags: number;
  signCount: number;
}

/** Verifies an assertion: client data, rpIdHash, UP, then the ES256 signature with `node:crypto`. */
export function verifyAssertion(input: {
  authenticatorData: Uint8Array;
  clientDataJSON: Uint8Array;
  signature: Uint8Array;
  jwk: JsonWebKey;
  expectedOrigin: string;
  expectedRpId: string;
  expectedChallenge: Uint8Array;
}): AssertionResult {
  checkClientData("webauthn.get", input);
  const authData = parseAuthData(input.authenticatorData);
  checkRpIdHash(authData, input.expectedRpId);
  if (!authData.up || authData.at) {
    fail("UP must be set and AT clear in an assertion");
  }
  const key = createPublicKey({ key: input.jwk, format: "jwk" });
  const message = Buffer.concat([Buffer.from(input.authenticatorData), sha256(input.clientDataJSON)]);
  if (!cryptoVerify("sha256", message, key, Buffer.from(input.signature))) {
    fail("signature does not verify");
  }
  return { flags: authData.flags, signCount: authData.signCount };
}

/** Verifies a DER ES256 signature over a plain message (the RFC 6979 vectors). */
export function verifyEs256(jwk: JsonWebKey, message: Uint8Array, signature: Uint8Array): boolean {
  const key = createPublicKey({ key: jwk, format: "jwk" });
  return cryptoVerify("sha256", Buffer.from(message), key, Buffer.from(signature));
}
