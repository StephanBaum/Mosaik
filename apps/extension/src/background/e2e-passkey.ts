/**
 * E2E builds only (never bundled into production, see `excludeE2e` and `check-dist.mjs`): the
 * passkey half of the fake desktop. It answers `passkeyCreate`, `passkeyGet` and `passkeyCancel`
 * like the real desktop at the points the hostile pages exercise: the origin and top-origin
 * rules, a relying party ID check, a dialog that waits for the test's decision, and a software
 * authenticator (`test/webauthn/authenticator.ts`, WebCrypto) whose output the independent
 * verifier checks.
 *
 * The relying party ID check here is FAKE: host equality or dot-suffix, no public suffix list.
 * The real rule (a domain at or under the origin's registrable domain, not a public suffix) is
 * `mosaic_origin::webauthn_rp_id` and is tested in Rust.
 */
import type { WireErrorCode } from "../shared/errors.ts";
import { decodeB64url, encodeB64url } from "../shared/b64url.ts";
import { hostOf } from "../shared/origin.ts";
import type { PasskeyCreateBody, PasskeyGetBody, ResultMap } from "../shared/protocol.ts";
import { register, sign, type TestCredential } from "../../test/webauthn/authenticator.ts";

export class Failure extends Error {
  readonly code: WireErrorCode;

  constructor(code: WireErrorCode) {
    super(code);
    this.code = code;
  }
}

/** What the test does in the (pretend) desktop dialog. `allow` is "Save passkey" or "Sign in with passkey". */
export type PasskeyDecision =
  | { kind: "allow" }
  | { kind: "useAnotherDevice" }
  | { kind: "cancel" }
  | { kind: "acknowledge" };

/** One passkey request the fake desktop received (ids and origins, never key material). */
export interface PasskeyLogEntry {
  op: "passkeyCreate" | "passkeyGet" | "passkeyCancel";
  requestId: string;
  origin: string | null;
  topOrigin: string | null;
  rpId: string | null;
  /** `opened` while the dialog waits; then the decision, or the error code. */
  result: string;
}

export interface OpenPrompt {
  requestId: string;
  op: "passkeyCreate" | "passkeyGet";
  origin: string;
  rpId: string;
  excluded: boolean;
}

interface Waiting extends OpenPrompt {
  settle: (decision: PasskeyDecision) => void;
  entry: PasskeyLogEntry;
}

const ID_BYTES = 16;

function bytesOf(value: string): Uint8Array {
  const bytes = decodeB64url(value);
  if (bytes === null) {
    throw new Failure("invalidRequest");
  }
  return bytes;
}

/** Fake relying party ID rule. The real one uses the public suffix list. */
function rpIdOf(origin: string, requested: string | null): string {
  const host = hostOf(origin);
  const rpId = requested ?? host;
  if (rpId !== host && !host.endsWith(`.${rpId}`)) {
    throw new Failure("rpIdMismatch");
  }
  return rpId;
}

function checkOrigin(origin: string, topOrigin: string): void {
  if (origin !== topOrigin) {
    throw new Failure("crossOriginFrame");
  }
  if (!origin.startsWith("https://")) {
    throw new Failure("insecureOrigin");
  }
}

export class FakePasskeys {
  readonly log: PasskeyLogEntry[] = [];
  readonly credentials: TestCredential[] = [];
  #waiting: Waiting | null = null;
  readonly #queued: PasskeyDecision[] = [];

  reset(): void {
    this.log.length = 0;
    this.credentials.length = 0;
    this.#queued.length = 0;
    this.#waiting?.settle({ kind: "cancel" });
    this.#waiting = null;
  }

  /** The dialog that is open, if any. */
  pending(): OpenPrompt[] {
    const open = this.#waiting;
    return open === null ? [] : [{ requestId: open.requestId, op: open.op, origin: open.origin, rpId: open.rpId, excluded: open.excluded }];
  }

  /** The user's answer: given to the open dialog, or kept for the next one. */
  decide(decision: PasskeyDecision): void {
    if (this.#waiting === null) {
      this.#queued.push(decision);
    } else {
      this.#waiting.settle(decision);
    }
  }

  /** `passkeyCancel`: closes the dialog of that request (another request's id changes nothing). */
  cancel(requestId: string): void {
    this.log.push({ op: "passkeyCancel", requestId, origin: null, topOrigin: null, rpId: null, result: "cancel" });
    if (this.#waiting?.requestId === requestId) {
      this.#waiting.settle({ kind: "cancel" });
    }
  }

  #open(entry: PasskeyLogEntry, prompt: Omit<OpenPrompt, "requestId">): Promise<PasskeyDecision> {
    if (this.#waiting !== null) {
      throw new Failure("busy");
    }
    return new Promise((resolve) => {
      const settle = (decision: PasskeyDecision): void => {
        this.#waiting = null;
        entry.result = decision.kind;
        resolve(decision);
      };
      this.#waiting = { ...prompt, requestId: entry.requestId, settle, entry };
      const queued = this.#queued.shift();
      if (queued !== undefined) {
        settle(queued);
      }
    });
  }

  #record(op: "passkeyCreate" | "passkeyGet", body: PasskeyCreateBody | PasskeyGetBody): PasskeyLogEntry {
    const entry: PasskeyLogEntry = {
      op,
      requestId: body.requestId,
      origin: body.origin,
      topOrigin: body.topOrigin,
      rpId: body.rpId,
      result: "opened",
    };
    this.log.push(entry);
    return entry;
  }

  async create(body: PasskeyCreateBody): Promise<ResultMap["passkeyCreate"]> {
    const entry = this.#record("passkeyCreate", body);
    try {
      return await this.#create(body, entry);
    } catch (error) {
      if (error instanceof Failure) {
        entry.result = error.code;
      }
      throw error;
    }
  }

  async #create(body: PasskeyCreateBody, entry: PasskeyLogEntry): Promise<ResultMap["passkeyCreate"]> {
    checkOrigin(body.origin, body.topOrigin);
    const rpId = rpIdOf(body.origin, body.rpId);
    if (body.algorithms.length > 0 && !body.algorithms.includes(-7)) {
      throw new Failure("passkeyUnsupported");
    }
    const challenge = bytesOf(body.challenge);
    const excluded = body.excludeCredentials.some((id) =>
      this.credentials.some((credential) => credential.rpId === rpId && encodeB64url(credential.credentialId) === id),
    );
    const decision = await this.#open(entry, { op: "passkeyCreate", excluded, rpId, origin: body.origin });
    if (decision.kind === "useAnotherDevice") {
      return { outcome: "useAnotherDevice", credential: null };
    }
    if (excluded && decision.kind === "acknowledge") {
      throw new Failure("passkeyExcluded");
    }
    if (decision.kind !== "allow" || excluded) {
      throw new Failure("cancelled");
    }
    const made = await register({
      rpId,
      origin: body.origin,
      challenge,
      userHandle: bytesOf(body.userId),
      userName: body.userName,
      userVerified: body.userVerification !== "discouraged",
    });
    this.credentials.push(made.credential);
    return {
      outcome: "created",
      credential: {
        credentialId: encodeB64url(made.credential.credentialId),
        clientDataJson: encodeB64url(made.clientDataJSON),
        attestationObject: encodeB64url(made.attestationObject),
        authenticatorData: encodeB64url(made.authenticatorData),
        publicKeySpki: encodeB64url(made.credential.spki),
        publicKeyAlgorithm: -7,
        vaultId: "v1",
        itemId: encodeB64url(crypto.getRandomValues(new Uint8Array(ID_BYTES))),
      },
    };
  }

  async get(body: PasskeyGetBody): Promise<ResultMap["passkeyGet"]> {
    const entry = this.#record("passkeyGet", body);
    try {
      return await this.#get(body, entry);
    } catch (error) {
      if (error instanceof Failure) {
        entry.result = error.code;
      }
      throw error;
    }
  }

  async #get(body: PasskeyGetBody, entry: PasskeyLogEntry): Promise<ResultMap["passkeyGet"]> {
    checkOrigin(body.origin, body.topOrigin);
    const rpId = rpIdOf(body.origin, body.rpId);
    const challenge = bytesOf(body.challenge);
    const candidate = this.credentials.find(
      (credential) =>
        credential.rpId === rpId &&
        (body.allowCredentials.length === 0 || body.allowCredentials.includes(encodeB64url(credential.credentialId))),
    );
    // No matching passkey: no dialog (the real desktop answers the same way).
    if (candidate === undefined) {
      throw new Failure("passkeyNotFound");
    }
    const decision = await this.#open(entry, { op: "passkeyGet", excluded: false, rpId, origin: body.origin });
    if (decision.kind === "useAnotherDevice") {
      return { outcome: "useAnotherDevice", assertion: null };
    }
    if (decision.kind !== "allow") {
      throw new Failure("cancelled");
    }
    const signed = await sign(candidate, { origin: body.origin, challenge, userVerified: body.userVerification !== "discouraged" });
    return {
      outcome: "asserted",
      assertion: {
        credentialId: encodeB64url(candidate.credentialId),
        clientDataJson: encodeB64url(signed.clientDataJSON),
        authenticatorData: encodeB64url(signed.authenticatorData),
        signature: encodeB64url(signed.signature),
        userHandle: encodeB64url(candidate.userHandle),
      },
    };
  }
}
