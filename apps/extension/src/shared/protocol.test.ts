/** The TypeScript validators against the Rust fixtures (crates/native-messaging/fixtures/protocol.json), both directions. */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { WIRE_ERROR_CODES } from "./errors.ts";
import { OPS } from "./protocol.ts";
import { isOp, isRequestBody, isResultBody, parseHostMessage } from "./validate.ts";

interface Sample {
  op: string;
  message: { v: number; id: number; op: string; body: unknown; ok?: boolean };
}

interface Fixtures {
  version: number;
  requests: Sample[];
  responses: Sample[];
  events: { name: string; message: unknown }[];
  errors: { code: string; message: unknown }[];
}

const fixtures = JSON.parse(
  readFileSync(join(import.meta.dirname, "../../../../crates/native-messaging/fixtures/protocol.json"), "utf8"),
) as Fixtures;

describe("requests (extension to host)", () => {
  it("has a sample for every op", () => {
    expect(fixtures.requests.map((r) => r.op).sort()).toEqual([...OPS].sort());
  });

  it.each(fixtures.requests.map((r) => [r.op, r] as const))("%s is a valid request", (op, sample) => {
    const { message } = sample;
    expect(Object.keys(message).sort()).toEqual(["body", "id", "op", "v"]);
    expect(message.v).toBe(1);
    expect(isOp(message.op)).toBe(true);
    expect(isOp(op) && isRequestBody(op, message.body)).toBe(true);
  });

  it("rejects unknown keys, missing keys and wrong types in a request body", () => {
    const fill = fixtures.requests.find((r) => r.op === "fill")?.message.body as Record<string, unknown>;
    expect(isRequestBody("fill", { ...fill, extra: 1 })).toBe(false);
    expect(isRequestBody("fill", { ...fill, want: undefined })).toBe(false);
    expect(isRequestBody("fill", { ...fill, frame: "iframe" })).toBe(false);
    expect(isRequestBody("search", { query: "x" })).toBe(false);
    expect(isRequestBody("status", { x: 1 })).toBe(false);
  });
});

describe("passkey ops", () => {
  const body = (op: string) =>
    fixtures.requests.find((r) => r.op === op)?.message.body as Record<string, unknown>;
  const result = (op: string) =>
    fixtures.responses.find((r) => r.op === op)?.message.body as Record<string, unknown>;

  it("rejects a passkey request with a missing key, an unknown key or a wrong value", () => {
    const create = body("passkeyCreate");
    expect(isRequestBody("passkeyCreate", { ...create, privateKey: "x" })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, rpId: undefined })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, userVerification: "always" })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, requestId: "ABC" })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, challenge: "not base64!" })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, algorithms: new Array(17).fill(-7) })).toBe(false);
    expect(isRequestBody("passkeyCreate", { ...create, excludeCredentials: new Array(65).fill("AQID") })).toBe(false);
    const get = body("passkeyGet");
    expect(isRequestBody("passkeyGet", { ...get, allowCredentials: undefined })).toBe(false);
    expect(isRequestBody("passkeyGet", { ...get, extra: 1 })).toBe(false);
    expect(isRequestBody("passkeyCancel", { requestId: "00112233445566778899aabbccddeeff", x: 1 })).toBe(false);
  });

  it("rejects a passkey result with key material, a missing key or a wrong outcome", () => {
    const created = result("passkeyCreate");
    expect(isResultBody("passkeyCreate", { ...created, outcome: "done" })).toBe(false);
    expect(isResultBody("passkeyCreate", { outcome: "created" })).toBe(false);
    expect(
      isResultBody("passkeyCreate", {
        ...created,
        credential: { ...(created["credential"] as object), privateKey: "k" },
      }),
    ).toBe(false);
    expect(isResultBody("passkeyCreate", { outcome: "useAnotherDevice", credential: null })).toBe(true);
    expect(isResultBody("passkeyGet", { outcome: "useAnotherDevice", assertion: null })).toBe(true);
    expect(isResultBody("passkeyGet", { outcome: "asserted" })).toBe(false);
    expect(isResultBody("passkeyCancel", { x: 1 })).toBe(false);
  });
});

describe("rotation ops", () => {
  const body = (op: string) =>
    fixtures.requests.find((r) => r.op === op)?.message.body as Record<string, unknown>;
  const result = (op: string) =>
    fixtures.responses.find((r) => r.op === op)?.message.body as Record<string, unknown>;

  it("rejects a rotation request with a missing key, an unknown key or a wrong value", () => {
    const change = body("fillChange");
    expect(isRequestBody("fillChange", { ...change, extra: 1 })).toBe(false);
    expect(isRequestBody("fillChange", { ...change, shape: undefined })).toBe(false);
    expect(isRequestBody("fillChange", { ...change, shape: "none" })).toBe(false);
    expect(isRequestBody("fillChange", { ...change, attemptId: "a b" })).toBe(false);
    expect(isRequestBody("fillChange", { ...change, attemptId: "a".repeat(65) })).toBe(false);
    expect(isRequestBody("fillChange", { ...change, frame: "iframe" })).toBe(false);
    const begin = body("rotationBegin");
    expect(isRequestBody("rotationBegin", { ...begin, rules: undefined })).toBe(false);
    expect(isRequestBody("rotationBegin", { ...begin, rules: "x".repeat(4097) })).toBe(false);
    expect(isRequestBody("rotationBegin", { ...begin, attemptId: "a1" })).toBe(false);
    expect(isRequestBody("rotationResult", { ...body("rotationResult"), outcome: "maybe" })).toBe(false);
    expect(isRequestBody("rotationAddOrigin", { ...body("rotationAddOrigin"), fromOrigin: undefined })).toBe(false);
    expect(isRequestBody("fillPending", { ...body("fillPending"), shape: "newOnly" })).toBe(false);
  });

  it("rejects a rotation result with a missing key, an extra key or a wrong outcome", () => {
    expect(isResultBody("fillChange", { ...result("fillChange"), extra: 1 })).toBe(false);
    expect(isResultBody("fillChange", { new: "pw" })).toBe(false);
    expect(isResultBody("fillChange", { current: null, new: "pw" })).toBe(true);
    expect(isResultBody("fillPending", { password: "pw" })).toBe(false);
    expect(isResultBody("rotationBegin", { outcome: "started" })).toBe(false);
    expect(isResultBody("rotationResult", { outcome: "done" })).toBe(false);
    expect(isResultBody("rotationAddOrigin", { outcome: "added", x: 1 })).toBe(false);
    const list = result("rotationForOrigin") as { attempts: Record<string, unknown>[] };
    expect(isResultBody("rotationForOrigin", { attempts: list.attempts.map((a) => ({ ...a, value: "pw" })) })).toBe(false);
    expect(isResultBody("rotationForOrigin", { attempts: new Array(6).fill(list.attempts[0]) })).toBe(false);
  });
});

describe("responses (host to extension)", () => {
  it.each(fixtures.responses.map((r) => [r.op, r] as const))("%s parses and its body validates", (op, sample) => {
    const parsed = parseHostMessage(sample.message);
    expect(parsed).toMatchObject({ kind: "response", id: 7, ok: true });
    const body = parsed?.kind === "response" && parsed.ok ? parsed.body : undefined;
    expect(isOp(op) && isResultBody(op, body)).toBe(true);
  });

  it("rejects bodies with unknown keys, missing keys or the wrong shape", () => {
    const fill = fixtures.responses.find((r) => r.op === "fill")?.message.body as Record<string, unknown>;
    expect(isResultBody("fill", { ...fill, extra: 1 })).toBe(false);
    expect(isResultBody("fill", { ...fill, otp: undefined })).toBe(false);
    expect(isResultBody("fill", { ...fill, password: 5 })).toBe(false);
    expect(isResultBody("listForOrigin", { items: [{}], warnings: [] })).toBe(false);
    expect(isResultBody("pair", { pairingId: "zz", key: "zz" })).toBe(false);
  });

  it("rejects a response with the wrong version, extra keys, or both a body and an error", () => {
    const ok = { v: 1, id: 7, ok: true, body: {} };
    expect(parseHostMessage(ok)).not.toBeNull();
    expect(parseHostMessage({ ...ok, v: 2 })).toBeNull();
    expect(parseHostMessage({ ...ok, extra: 1 })).toBeNull();
    expect(parseHostMessage({ ...ok, error: "internal" })).toBeNull();
    expect(parseHostMessage({ v: 1, id: 7, ok: false })).toBeNull();
    expect(parseHostMessage({ v: 1, id: -1, ok: true, body: {} })).toBeNull();
    expect(parseHostMessage("nope")).toBeNull();
    expect(parseHostMessage(null)).toBeNull();
  });
});

describe("events", () => {
  it.each(fixtures.events.map((e) => [e.name, e] as const))("%s parses as an event", (name, sample) => {
    const parsed = parseHostMessage(sample.message);
    expect(parsed).toMatchObject({ kind: "event", event: { event: name } });
  });

  it("rejects unknown events and extra keys", () => {
    expect(parseHostMessage({ v: 1, event: "reboot" })).toBeNull();
    expect(parseHostMessage({ v: 1, event: "unpaired", x: 1 })).toBeNull();
    expect(parseHostMessage({ v: 1, event: "state", state: "open" })).toBeNull();
    expect(parseHostMessage({ v: 2, event: "unpaired" })).toBeNull();
  });
});

describe("error codes", () => {
  it("TypeScript knows exactly the wire codes of the Rust enum", () => {
    expect(fixtures.errors.map((e) => e.code).sort()).toEqual([...WIRE_ERROR_CODES].sort());
  });

  it.each(fixtures.errors.map((e) => [e.code, e] as const))("%s parses as a failed response", (code, sample) => {
    expect(parseHostMessage(sample.message)).toEqual({ kind: "response", id: 7, ok: false, error: code });
  });

  it("rejects a code that is not on the wire", () => {
    expect(parseHostMessage({ v: 1, id: 7, ok: false, error: "noMatch" })).toBeNull();
  });
});
