import { describe, expect, it, vi } from "vitest";
import { encodeB64url } from "../shared/b64url.ts";
import type { PasskeyAbortMessage, PasskeyMessage } from "../shared/passkey-messages.ts";
import { installBridge } from "./bridge.ts";

type Listener = (event: { source: unknown; data: unknown }) => void;
type Sent = PasskeyMessage | PasskeyAbortMessage;

interface Harness {
  win: Window & typeof globalThis;
  posted: Record<string, unknown>[];
  sent: Sent[];
  /** Answers the nth pending `sendMessage`. */
  answer(n: number, value: unknown): Promise<void>;
  fail(n: number): Promise<void>;
  deliver(data: unknown, source?: unknown): void;
}

const CHALLENGE = encodeB64url(new Uint8Array(16));

function setup(over: { origin?: string; focus?: boolean; visible?: boolean; top?: boolean } = {}): Harness {
  const listeners: Listener[] = [];
  const posted: Record<string, unknown>[] = [];
  const sent: Sent[] = [];
  const settlers: { resolve(value: unknown): void; reject(reason: unknown): void }[] = [];
  const win = {
    origin: over.origin ?? "https://bank.test",
    document: { hasFocus: () => over.focus ?? true, visibilityState: over.visible === false ? "hidden" : "visible" },
    postMessage: (message: Record<string, unknown>) => posted.push(message),
    addEventListener: (_type: string, listener: Listener) => listeners.push(listener),
  } as unknown as Window & typeof globalThis;
  (win as { parent: unknown }).parent = over.top === false ? {} : win;
  const send = (message: Sent): Promise<unknown> => {
    sent.push(message);
    return new Promise((resolve, reject) => settlers.push({ resolve, reject }));
  };
  expect(installBridge({ win, send })).toBe(true);
  return {
    win,
    posted,
    sent,
    answer: async (n, value) => {
      settlers[n]?.resolve(value);
      await new Promise((resolve) => setTimeout(resolve, 0));
    },
    fail: async (n) => {
      settlers[n]?.reject(new Error("no receiver"));
      await new Promise((resolve) => setTimeout(resolve, 0));
    },
    deliver: (data, source = win) => {
      for (const listener of listeners) {
        listener({ source, data });
      }
    },
  };
}

const getRequest = (id: number, options: Record<string, unknown> = {}): Record<string, unknown> => ({
  type: "mosaic-passkey-req",
  id,
  op: "get",
  options: { rpId: null, challenge: CHALLENGE, allowCredentials: [], userVerification: "preferred", ...options },
});

const createRequest = (id: number): Record<string, unknown> => ({
  type: "mosaic-passkey-req",
  id,
  op: "create",
  options: {
    rpId: "bank.test",
    userId: "AQID",
    userName: "ada",
    userDisplayName: "Ada",
    challenge: CHALLENGE,
    algorithms: [-7],
    excludeCredentials: [],
    userVerification: "preferred",
  },
});

const FALLBACK = { type: "mosaic-passkey-res", result: { kind: "fallback" } };

describe("what the bridge accepts", () => {
  it("answers only messages posted by its own window", () => {
    const h = setup();
    h.deliver(getRequest(1), {});
    h.deliver(getRequest(2), null);
    expect(h.sent).toEqual([]);
    expect(h.posted).toEqual([]);
  });

  it.each([
    ["a message of another type", { type: "other", id: 1 }],
    ["text", "mosaic-passkey-req"],
    ["null", null],
    ["a request with an extra member", { ...getRequest(1), origin: "https://bank.test" }],
    ["a request without options", { type: "mosaic-passkey-req", id: 1, op: "get" }],
    ["a request with an unknown op", { ...getRequest(1), op: "delete" }],
    ["a negative id", getRequest(-1)],
    ["a fractional id", getRequest(1.5)],
    ["a challenge that is too short", getRequest(1, { challenge: encodeB64url(new Uint8Array(15)) })],
    ["a challenge that is too long", getRequest(1, { challenge: encodeB64url(new Uint8Array(1025)) })],
    ["a challenge that is not base64url", getRequest(1, { challenge: "not base64url!!!!!!" })],
    ["a list over 64 entries", getRequest(1, { allowCredentials: Array.from({ length: 65 }, () => "AAAAAAAAAAAAAAAAAAAAAA") })],
    ["an unknown user verification", getRequest(1, { userVerification: "always" })],
    ["an extra option", getRequest(1, { extra: 1 })],
    ["an rp id over 253", getRequest(1, { rpId: "a".repeat(254) })],
  ])("drops %s without a word", (_name, data) => {
    const h = setup();
    h.deliver(data);
    expect(h.sent).toEqual([]);
    expect(h.posted).toEqual([]);
  });

  it("forwards a request with the origin it reads itself, and acknowledges it", async () => {
    const h = setup({ origin: "https://bank.test" });
    h.deliver(createRequest(7));
    expect(h.posted).toEqual([{ type: "mosaic-passkey-ack", id: 7 }]);
    expect(h.sent).toEqual([
      {
        kind: "passkey",
        op: "create",
        pageRequestId: 7,
        origin: "https://bank.test",
        options: (createRequest(7)["options"] as Record<string, unknown>),
      },
    ]);
    await h.answer(0, { kind: "created", credentialId: "AAAAAAAAAAAAAAAAAAAAAA", clientDataJson: "AAAA", attestationObject: "AAAA", authenticatorData: "AAAA", publicKeySpki: "AAAA", publicKeyAlgorithm: -7 });
    expect(h.posted.at(-1)).toMatchObject({ type: "mosaic-passkey-res", id: 7, result: { kind: "created" } });
  });

  it("relays an error result and turns anything else the background says into a fallback", async () => {
    const h = setup();
    h.deliver(getRequest(1));
    await h.answer(0, { kind: "error", name: "NotAllowedError" });
    expect(h.posted.at(-1)).toEqual({ type: "mosaic-passkey-res", id: 1, result: { kind: "error", name: "NotAllowedError" } });

    h.deliver(getRequest(2));
    await h.answer(1, { kind: "error", name: "SecurityError" });
    expect(h.posted.at(-1)).toEqual({ ...FALLBACK, id: 2 });

    h.deliver(getRequest(3));
    await h.answer(2, undefined);
    expect(h.posted.at(-1)).toEqual({ ...FALLBACK, id: 3 });

    h.deliver(getRequest(4));
    await h.fail(3);
    expect(h.posted.at(-1)).toEqual({ ...FALLBACK, id: 4 });
  });
});

describe("when the page is not in use", () => {
  it.each([
    ["without focus", { focus: false }],
    ["hidden", { visible: false }],
    ["not https", { origin: "http://bank.test" }],
    ["not the top window", { top: false }],
  ])("a page %s gets a fallback and no message goes out", (_name, over) => {
    const h = setup(over);
    h.deliver(getRequest(1));
    expect(h.posted).toEqual([{ ...FALLBACK, id: 1 }]);
    expect(h.sent).toEqual([]);
  });
});

describe("requests in flight", () => {
  it("allows two at a time, refuses a third and a repeated id, and frees a slot when one is answered", async () => {
    const h = setup();
    h.deliver(getRequest(1));
    h.deliver(getRequest(2));
    h.deliver(getRequest(2));
    h.deliver(getRequest(3));
    expect(h.sent).toHaveLength(2);
    expect(h.posted.filter((p) => p["result"] !== undefined)).toEqual([{ ...FALLBACK, id: 2 }, { ...FALLBACK, id: 3 }]);
    await h.answer(0, { kind: "fallback" });
    h.deliver(getRequest(4));
    expect(h.sent).toHaveLength(3);
  });
});

describe("abort", () => {
  it("passes an abort of a request in flight on to the background and sends no answer for it", async () => {
    const h = setup();
    h.deliver(getRequest(5));
    h.deliver({ type: "mosaic-passkey-abort", id: 5 });
    expect(h.sent.at(-1)).toEqual({ kind: "passkeyAbort", pageRequestId: 5 });
    await h.answer(0, { kind: "error", name: "NotAllowedError" });
    expect(h.posted.filter((p) => p["type"] === "mosaic-passkey-res")).toEqual([]);
  });

  it("ignores an abort of a request it does not have, and malformed aborts", () => {
    const h = setup();
    h.deliver({ type: "mosaic-passkey-abort", id: 5 });
    h.deliver({ type: "mosaic-passkey-abort", id: 5, extra: 1 });
    h.deliver({ type: "mosaic-passkey-abort" });
    expect(h.sent).toEqual([]);
  });

  it("frees the slot of an aborted request", () => {
    const h = setup();
    h.deliver(getRequest(1));
    h.deliver(getRequest(2));
    h.deliver({ type: "mosaic-passkey-abort", id: 1 });
    h.deliver(getRequest(3));
    expect(h.sent.filter((m) => m.kind === "passkey")).toHaveLength(3);
  });
});

describe("installation", () => {
  it("installs once per world", () => {
    const win = { addEventListener: vi.fn(), origin: "https://bank.test" } as unknown as Window & typeof globalThis;
    const send = (): Promise<unknown> => Promise.resolve();
    expect(installBridge({ win, send })).toBe(true);
    expect(installBridge({ win, send })).toBe(false);
  });

  it("is quiet about a send that throws at once", () => {
    const listeners: Listener[] = [];
    const posted: unknown[] = [];
    const win = {
      origin: "https://bank.test",
      document: { hasFocus: () => true, visibilityState: "visible" },
      postMessage: (m: unknown) => posted.push(m),
      addEventListener: (_t: string, l: Listener) => listeners.push(l),
    } as unknown as Window & typeof globalThis;
    (win as { parent: unknown }).parent = win;
    installBridge({
      win,
      send: () => {
        throw new Error("context invalidated");
      },
    });
    listeners[0]?.({ source: win, data: getRequest(1) });
    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(posted.at(-1)).toEqual({ ...FALLBACK, id: 1 });
        resolve();
      }, 0);
    });
  });
});
