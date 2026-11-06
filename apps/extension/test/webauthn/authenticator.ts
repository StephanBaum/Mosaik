/**
 * A WebAuthn authenticator for the extension's tests (M5 T3): ES256 with WebCrypto, `none`
 * attestation, the same encodings as `mosaic-passkey` (CRYPTO_SPEC section 16) but written
 * separately, so the independent verifier (`verifier.ts`) checks it like it checks the Rust
 * code. It is the fake desktop's signer in the e2e build and runs in a service worker, so it
 * uses only WebCrypto. Test code, never part of the production package.
 */
import { encodeB64url } from "../../src/shared/b64url.ts";

export interface TestCredential {
  credentialId: Uint8Array;
  rpId: string;
  userHandle: Uint8Array;
  userName: string;
  privateKey: CryptoKey;
  /** The 77-byte COSE_Key. */
  cose: Uint8Array;
  /** The 91-byte SubjectPublicKeyInfo. */
  spki: Uint8Array;
}

export interface Registration {
  credential: TestCredential;
  clientDataJSON: Uint8Array;
  authenticatorData: Uint8Array;
  attestationObject: Uint8Array;
}

export interface Assertion {
  clientDataJSON: Uint8Array;
  authenticatorData: Uint8Array;
  signature: Uint8Array;
}

const FLAG_UP = 0x01;
const FLAG_UV = 0x04;
const FLAG_BE = 0x08;
const FLAG_BS = 0x10;
const FLAG_AT = 0x40;

const text = (value: string): Uint8Array => new TextEncoder().encode(value);

const concat = (...parts: Uint8Array[]): Uint8Array => {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
};

const hex = (value: string): Uint8Array => Uint8Array.from(value.match(/../g) ?? [], (pair) => Number.parseInt(pair, 16));

async function sha256(data: Uint8Array): Promise<Uint8Array> {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", data as BufferSource));
}

/** CBOR byte string header for a length (the layouts here never exceed 65535). */
function bstrHeader(length: number): Uint8Array {
  if (length < 24) {
    return Uint8Array.of(0x40 | length);
  }
  return length < 256 ? Uint8Array.of(0x58, length) : Uint8Array.of(0x59, length >> 8, length & 255);
}

/** CBOR text string header and bytes (the keys here are shorter than 24 bytes). */
const cborText = (value: string): Uint8Array => concat(Uint8Array.of(0x60 | value.length), text(value));

function coseKey(raw: Uint8Array): Uint8Array {
  // CTAP2 canonical order: kty 2 (EC2), alg -7, crv 1 (P-256), x, y.
  return concat(hex("A5010203262001215820"), raw.slice(1, 33), hex("225820"), raw.slice(33, 65));
}

function derInteger(bytes: Uint8Array): Uint8Array {
  let start = 0;
  while (start < bytes.length - 1 && bytes[start] === 0) {
    start++;
  }
  const body = bytes.slice(start);
  const padded = (body[0] ?? 0) >= 0x80 ? concat(Uint8Array.of(0), body) : body;
  return concat(Uint8Array.of(0x02, padded.length), padded);
}

/** WebCrypto signs ECDSA as r || s (64 bytes); WebAuthn wants the DER sequence. */
function derSignature(raw: Uint8Array): Uint8Array {
  const body = concat(derInteger(raw.slice(0, 32)), derInteger(raw.slice(32)));
  return concat(Uint8Array.of(0x30, body.length), body);
}

export function clientDataJSON(type: "webauthn.create" | "webauthn.get", challenge: Uint8Array, origin: string): Uint8Array {
  return text(`{"type":${JSON.stringify(type)},"challenge":${JSON.stringify(encodeB64url(challenge))},"origin":${JSON.stringify(origin)},"crossOrigin":false}`);
}

async function authenticatorData(rpId: string, flags: number, attested: Uint8Array | null): Promise<Uint8Array> {
  const head = concat(await sha256(text(rpId)), Uint8Array.of(flags, 0, 0, 0, 0));
  return attested === null ? head : concat(head, attested);
}

export interface RegisterInput {
  rpId: string;
  origin: string;
  challenge: Uint8Array;
  userHandle: Uint8Array;
  userName: string;
  userVerified: boolean;
}

/** A new discoverable credential: `none` attestation, flags UP, BE, BS, AT (and UV), sign count 0. */
export async function register(input: RegisterInput): Promise<Registration> {
  const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]);
  const raw = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  const credential: TestCredential = {
    credentialId: crypto.getRandomValues(new Uint8Array(16)),
    rpId: input.rpId,
    userHandle: input.userHandle,
    userName: input.userName,
    privateKey: pair.privateKey,
    cose: coseKey(raw),
    spki: new Uint8Array(await crypto.subtle.exportKey("spki", pair.publicKey)),
  };
  const attested = concat(new Uint8Array(16), Uint8Array.of(0, credential.credentialId.length), credential.credentialId, credential.cose);
  const flags = FLAG_UP | FLAG_BE | FLAG_BS | FLAG_AT | (input.userVerified ? FLAG_UV : 0);
  const authData = await authenticatorData(input.rpId, flags, attested);
  // {"fmt": "none", "attStmt": {}, "authData": h'...'} in the order the verifier demands.
  const attestationObject = concat(
    Uint8Array.of(0xa3),
    cborText("fmt"),
    cborText("none"),
    cborText("attStmt"),
    Uint8Array.of(0xa0),
    cborText("authData"),
    bstrHeader(authData.length),
    authData,
  );
  return {
    credential,
    clientDataJSON: clientDataJSON("webauthn.create", input.challenge, input.origin),
    authenticatorData: authData,
    attestationObject,
  };
}

export interface SignInput {
  origin: string;
  challenge: Uint8Array;
  userVerified: boolean;
}

/** An assertion over `authenticatorData || SHA-256(clientDataJSON)`: flags UP, BE, BS (and UV), sign count 0. */
export async function sign(credential: TestCredential, input: SignInput): Promise<Assertion> {
  const client = clientDataJSON("webauthn.get", input.challenge, input.origin);
  const authData = await authenticatorData(credential.rpId, FLAG_UP | FLAG_BE | FLAG_BS | (input.userVerified ? FLAG_UV : 0), null);
  const signed = await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, credential.privateKey, concat(authData, await sha256(client)) as BufferSource);
  return { clientDataJSON: client, authenticatorData: authData, signature: derSignature(new Uint8Array(signed)) };
}
