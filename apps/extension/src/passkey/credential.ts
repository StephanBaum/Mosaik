/**
 * Builds the objects a page gets back from `navigator.credentials.create/get`, from the
 * desktop's answer. They are made with `Object.create(PublicKeyCredential.prototype)` and carry
 * their data as own properties, so `instanceof PublicKeyCredential` holds and no prototype getter
 * (which would throw "Illegal invocation" on a plain object) is involved.
 */
import { decodeB64url } from "../shared/b64url.ts";
import type { AssertedResult, CreatedResult } from "../shared/passkey-messages.ts";

/** The page's WebAuthn constructors, taken at load time. */
export interface CredentialClasses {
  PublicKeyCredential: { prototype: object };
  AuthenticatorAttestationResponse: { prototype: object } | undefined;
  AuthenticatorAssertionResponse: { prototype: object } | undefined;
}

type Fields = Record<string, unknown>;

const ObjectCreate = Object.create.bind(Object) as (proto: object | null, props: PropertyDescriptorMap) => object;
const Bytes = Uint8Array;

function values(fields: Fields): PropertyDescriptorMap {
  return Object.fromEntries(
    Object.entries(fields).map(([key, value]) => [key, { value, enumerable: true, writable: false, configurable: true }]),
  );
}

/** Fresh, exactly sized ArrayBuffers for every encoded value, or `null` if one is malformed. */
function buffers(encoded: Record<string, string>): Fields | null {
  const out: Fields = {};
  for (const [key, value] of Object.entries(encoded)) {
    const bytes = decodeB64url(value);
    if (bytes === null) {
      return null;
    }
    out[key] = new Bytes(bytes).buffer;
  }
  return out;
}

const extensionResults =
  (credProps: boolean) =>
  (): object =>
    credProps ? { credProps: { rk: true } } : {};

function credentialOf(classes: CredentialClasses, fields: Fields, credProps: boolean): object {
  return ObjectCreate(
    classes.PublicKeyCredential.prototype,
    values({
      type: "public-key",
      authenticatorAttachment: "platform",
      ...fields,
      getClientExtensionResults: extensionResults(credProps),
    }),
  );
}

/** The `toJSON` of both credential kinds: the L3 registration / authentication response JSON. */
function jsonOf(result: { credentialId: string }, response: object, credProps: boolean): () => object {
  return () => ({
    id: result.credentialId,
    rawId: result.credentialId,
    type: "public-key",
    authenticatorAttachment: "platform",
    response,
    clientExtensionResults: extensionResults(credProps)(),
  });
}

/** The `PublicKeyCredential` of a registration (WebAuthn L3: `getTransports`, `getPublicKey`, `toJSON`), or `null` if the data is malformed. */
export function createdCredential(classes: CredentialClasses, result: CreatedResult, credProps: boolean): object | null {
  const raw = buffers({
    rawId: result.credentialId,
    clientDataJSON: result.clientDataJson,
    attestationObject: result.attestationObject,
    authenticatorData: result.authenticatorData,
    publicKey: result.publicKeySpki,
  });
  if (raw === null) {
    return null;
  }
  const response = ObjectCreate(
    classes.AuthenticatorAttestationResponse?.prototype ?? Object.prototype,
    values({
      clientDataJSON: raw["clientDataJSON"],
      attestationObject: raw["attestationObject"],
      getTransports: () => ["internal"],
      getAuthenticatorData: () => raw["authenticatorData"],
      getPublicKey: () => raw["publicKey"],
      getPublicKeyAlgorithm: () => result.publicKeyAlgorithm,
    }),
  );
  const json = jsonOf(
    result,
    {
      clientDataJSON: result.clientDataJson,
      attestationObject: result.attestationObject,
      authenticatorData: result.authenticatorData,
      transports: ["internal"],
      publicKey: result.publicKeySpki,
      publicKeyAlgorithm: result.publicKeyAlgorithm,
    },
    credProps,
  );
  return credentialOf(classes, { id: result.credentialId, rawId: raw["rawId"], response, toJSON: json }, credProps);
}

/** The `PublicKeyCredential` of a sign-in, or `null` if the data is malformed. */
export function assertedCredential(classes: CredentialClasses, result: AssertedResult, credProps: boolean): object | null {
  const raw = buffers({
    rawId: result.credentialId,
    clientDataJSON: result.clientDataJson,
    authenticatorData: result.authenticatorData,
    signature: result.signature,
    userHandle: result.userHandle,
  });
  if (raw === null) {
    return null;
  }
  const response = ObjectCreate(
    classes.AuthenticatorAssertionResponse?.prototype ?? Object.prototype,
    values({
      clientDataJSON: raw["clientDataJSON"],
      authenticatorData: raw["authenticatorData"],
      signature: raw["signature"],
      userHandle: raw["userHandle"],
    }),
  );
  const json = jsonOf(
    result,
    {
      clientDataJSON: result.clientDataJson,
      authenticatorData: result.authenticatorData,
      signature: result.signature,
      userHandle: result.userHandle,
    },
    credProps,
  );
  return credentialOf(classes, { id: result.credentialId, rawId: raw["rawId"], response, toJSON: json }, credProps);
}
