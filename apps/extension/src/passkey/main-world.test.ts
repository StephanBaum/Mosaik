import { afterEach, describe, expect, it, vi } from "vitest";
import { decodeB64url, encodeB64url } from "../shared/b64url.ts";
import { BRIDGE_ACK_TIMEOUT_MS } from "../shared/passkey-limits.ts";
import type { PasskeyPageResult } from "../shared/passkey-messages.ts";
import { installShim } from "./main-world.ts";

type Listener = (event: { source: unknown; data: unknown }) => void;

/** Stand-ins for the browser's classes; the shim only needs their prototypes. */
class CredentialsContainer {
  create(options?: CredentialCreationOptions): Promise<unknown> {
    return Promise.resolve(options);
  }
  get(options?: CredentialRequestOptions): Promise<unknown> {
    return Promise.resolve(options);
  }
}
class PublicKeyCredential {
  readonly kind = "PublicKeyCredential";
}
class AuthenticatorAttestationResponse {
  readonly kind = "AuthenticatorAttestationResponse";
}
class AuthenticatorAssertionResponse {
  readonly kind = "AuthenticatorAssertionResponse";
}

interface Page {
  win: Window & typeof globalThis;
  credentials: CredentialsContainer;
  native: { create: ReturnType<typeof vi.fn>; get: ReturnType<typeof vi.fn> };
  posted: { message: Record<string, unknown>; target: string }[];
  /** Delivers a message event; `source` defaults to the window itself. */
  deliver(data: unknown, source?: unknown): void;
  /** The last request the shim posted. */
  last(): Record<string, unknown>;
  reply(id: number, result: PasskeyPageResult): void;
}

function makePage(over: { origin?: string; withWebAuthn?: boolean } = {}): Page {
  const listeners: Listener[] = [];
  const posted: Page["posted"] = [];
  const credentials = new CredentialsContainer();
  const native = {
    create: vi.fn(() => Promise.resolve("native-create")),
    get: vi.fn(() => Promise.resolve("native-get")),
  };
  for (const [name, fn] of Object.entries(native)) {
    Object.defineProperty(CredentialsContainer.prototype, name, { value: fn, writable: true, enumerable: true, configurable: true });
  }
  const win = {
    origin: over.origin ?? "https://bank.test",
    CredentialsContainer,
    navigator: { credentials },
    postMessage: (message: Record<string, unknown>, target: string) => posted.push({ message, target }),
    addEventListener: (_type: string, listener: Listener) => listeners.push(listener),
    setTimeout: globalThis.setTimeout.bind(globalThis),
    clearTimeout: globalThis.clearTimeout.bind(globalThis),
    ...(over.withWebAuthn === false ? {} : { PublicKeyCredential, AuthenticatorAttestationResponse, AuthenticatorAssertionResponse }),
  } as unknown as Window & typeof globalThis;
  const page: Page = {
    win,
    credentials,
    native,
    posted,
    deliver: (data, source = win) => {
      for (const listener of listeners) {
        listener({ source, data });
      }
    },
    last: () => {
      const entry = posted.at(-1);
      if (entry === undefined) {
        throw new Error("nothing posted");
      }
      return entry.message;
    },
    reply: (id, result) => {
      page.deliver({ type: "mosaic-passkey-res", id, result });
    },
  };
  return page;
}

const bytes = (n: number, fill = 7): Uint8Array => new Uint8Array(n).fill(fill);

const createOptions = (over: Record<string, unknown> = {}, publicKey: Record<string, unknown> = {}): CredentialCreationOptions =>
  ({
    publicKey: {
      rp: { name: "Bank", id: "bank.test" },
      user: { id: bytes(8), name: "ada", displayName: "Ada" },
      challenge: bytes(32),
      pubKeyCredParams: [{ type: "public-key", alg: -7 }],
      ...publicKey,
    },
    ...over,
  }) as unknown as CredentialCreationOptions;

const getOptions = (over: Record<string, unknown> = {}, publicKey: Record<string, unknown> = {}): CredentialRequestOptions =>
  ({ publicKey: { challenge: bytes(16), ...publicKey }, ...over }) as unknown as CredentialRequestOptions;

const create = (page: Page, options?: unknown): Promise<unknown> => page.credentials.create(options as CredentialCreationOptions | undefined);
const get = (page: Page, options?: unknown): Promise<unknown> => page.credentials.get(options as CredentialRequestOptions | undefined);

function install(over: Parameters<typeof makePage>[0] = {}): Page {
  const page = makePage(over);
  expect(installShim(page.win)).toBe(true);
  return page;
}

afterEach(() => {
  vi.useRealTimers();
});

describe("installation", () => {
  it("replaces create and get with the same descriptor and installs once", () => {
    const page = makePage();
    const before = Object.getOwnPropertyDescriptor(CredentialsContainer.prototype, "create");
    expect(installShim(page.win)).toBe(true);
    const after = Object.getOwnPropertyDescriptor(CredentialsContainer.prototype, "create");
    expect(after).toMatchObject({ writable: true, enumerable: true, configurable: true });
    expect(after?.value).not.toBe(before?.value);
    expect((after?.value as { name: string }).name).toBe("create");
    expect((Object.getOwnPropertyDescriptor(CredentialsContainer.prototype, "get")?.value as { name: string }).name).toBe("get");
    expect(installShim(page.win)).toBe(false);
  });

  it("does nothing where there is no CredentialsContainer", () => {
    expect(installShim({} as unknown as Window & typeof globalThis)).toBe(false);
  });
});

describe("what is left to the browser", () => {
  const cases: [string, () => Page, unknown][] = [
    ["no publicKey member", () => install(), {}],
    ["no options at all", () => install(), undefined],
    ["conditional mediation", () => install(), createOptions({ mediation: "conditional" })],
    ["silent mediation", () => install(), createOptions({ mediation: "silent" })],
    ["a cross-platform authenticator", () => install(), createOptions({}, { authenticatorSelection: { authenticatorAttachment: "cross-platform" } })],
    ["enterprise attestation", () => install(), createOptions({}, { attestation: "enterprise" })],
    ["the PRF extension", () => install(), createOptions({}, { extensions: { prf: {} } })],
    ["a challenge that is too short", () => install(), createOptions({}, { challenge: bytes(15) })],
    ["a challenge that is too long", () => install(), createOptions({}, { challenge: bytes(1025) })],
    ["an empty user id", () => install(), createOptions({}, { user: { id: bytes(0), name: "a", displayName: "a" } })],
    ["a user id over 64 bytes", () => install(), createOptions({}, { user: { id: bytes(65), name: "a", displayName: "a" } })],
    ["a user name over 256 units", () => install(), createOptions({}, { user: { id: bytes(8), name: "a".repeat(257), displayName: "a" } })],
    ["an rp id over 253", () => install(), createOptions({}, { rp: { name: "x", id: "a".repeat(254) } })],
    ["more than 16 algorithms", () => install(), createOptions({}, { pubKeyCredParams: Array.from({ length: 17 }, () => ({ type: "public-key", alg: -7 })) })],
    ["algorithms of no known type", () => install(), createOptions({}, { pubKeyCredParams: [{ type: "other", alg: -7 }] })],
    ["a challenge of the wrong type", () => install(), createOptions({}, { challenge: "text" })],
    [
      "a getter that throws",
      () => install(),
      {
        publicKey: Object.defineProperty({ rp: { name: "x" }, user: { id: bytes(8), name: "a", displayName: "a" }, pubKeyCredParams: [] }, "challenge", {
          get: () => {
            throw new Error("boom");
          },
          enumerable: true,
        }),
      },
    ],
  ];

  it.each(cases)("create with %s calls the browser with the same options and this", async (_name, build, options) => {
    const page = build();
    expect(await create(page, options)).toBe("native-create");
    expect(page.native.create).toHaveBeenCalledOnce();
    expect(page.native.create.mock.contexts[0]).toBe(page.credentials);
    expect(page.native.create.mock.calls[0]?.[0]).toBe(options);
    expect(page.posted).toEqual([]);
  });

  it("get with conditional mediation, a long allow list or a oversized id calls the browser", async () => {
    const page = install();
    for (const options of [
      getOptions({ mediation: "conditional" }),
      getOptions({}, { allowCredentials: Array.from({ length: 65 }, () => ({ type: "public-key", id: bytes(16) })) }),
      getOptions({}, { allowCredentials: [{ type: "public-key", id: bytes(1024) }] }),
      getOptions({}, { allowCredentials: [{ type: "other", id: bytes(16) }] }),
      getOptions({}, { extensions: { largeBlob: { read: true } } }),
    ]) {
      expect(await get(page, options)).toBe("native-get");
      expect(page.native.get.mock.calls.at(-1)?.[0]).toBe(options);
    }
    expect(page.posted).toEqual([]);
  });

  it("passes every call through on a page that is not https, without PublicKeyCredential, or with another this", async () => {
    const http = install({ origin: "http://bank.test" });
    expect(await create(http, createOptions())).toBe("native-create");
    expect(http.posted).toEqual([]);

    const none = install({ withWebAuthn: false });
    expect(await get(none, getOptions())).toBe("native-get");
    expect(none.posted).toEqual([]);

    const page = install();
    const other = {};
    const create_ = Object.getOwnPropertyDescriptor(CredentialsContainer.prototype, "create")?.value as (this: unknown, o: unknown) => Promise<unknown>;
    expect(await create_.call(other, createOptions())).toBe("native-create");
    expect(page.native.create.mock.contexts.at(-1)).toBe(other);
    expect(page.posted).toEqual([]);
  });
});

describe("a request Mosaic can answer", () => {
  it("posts the options with every BufferSource copied by its own offset and length", () => {
    const page = install();
    const backing = new Uint8Array(64).map((_, i) => i);
    const challenge = backing.subarray(8, 8 + 20);
    const userId = new DataView(backing.buffer, 4, 6);
    const exclude = backing.slice(10, 26).buffer;
    void create(page, createOptions({}, { challenge, user: { id: userId, name: "ada", displayName: "Ada" }, excludeCredentials: [{ type: "public-key", id: exclude }, { type: "other", id: exclude }] }));
    const message = page.last();
    expect(message).toMatchObject({ type: "mosaic-passkey-req", id: 1, op: "create" });
    expect(page.posted[0]?.target).toBe("https://bank.test");
    const options = message["options"] as Record<string, unknown>;
    expect(decodeB64url(options["challenge"] as string)).toEqual(Uint8Array.from({ length: 20 }, (_, i) => 8 + i));
    expect(decodeB64url(options["userId"] as string)).toEqual(Uint8Array.from({ length: 6 }, (_, i) => 4 + i));
    expect((options["excludeCredentials"] as string[]).map((id) => decodeB64url(id)?.length)).toEqual([16]);
    expect(options).toMatchObject({ rpId: "bank.test", userName: "ada", userDisplayName: "Ada", algorithms: [-7], userVerification: "preferred" });
    expect(Object.keys(options).sort()).toEqual(["algorithms", "challenge", "excludeCredentials", "rpId", "userDisplayName", "userId", "userName", "userVerification"]);
  });

  it("sends a null rp id when the page names none, and the get fields", () => {
    const page = install();
    void get(page, getOptions({}, { userVerification: "required", allowCredentials: [{ type: "public-key", id: bytes(16) }] }));
    expect(page.last()["options"]).toEqual({
      rpId: null,
      challenge: encodeB64url(bytes(16)),
      allowCredentials: [encodeB64url(bytes(16))],
      userVerification: "required",
    });
  });

  it("numbers requests and does not call the browser", () => {
    const page = install();
    void get(page, getOptions());
    void get(page, getOptions());
    expect(page.posted.map((p) => p.message["id"])).toEqual([1, 2]);
    expect(page.native.get).not.toHaveBeenCalled();
  });

  it("falls back to the browser, with the identical options, when the bridge says so", async () => {
    const page = install();
    const options = getOptions();
    const promise = get(page, options);
    page.reply(1, { kind: "fallback" });
    expect(await promise).toBe("native-get");
    expect(page.native.get.mock.calls[0]?.[0]).toBe(options);
    expect(page.native.get.mock.contexts[0]).toBe(page.credentials);
  });

  it("falls back when the bridge does not acknowledge in time, and not after it did", async () => {
    vi.useFakeTimers();
    const page = install();
    const silent = get(page, getOptions());
    await vi.advanceTimersByTimeAsync(BRIDGE_ACK_TIMEOUT_MS + 1);
    expect(await silent).toBe("native-get");

    const acked = get(page, getOptions());
    page.deliver({ type: "mosaic-passkey-ack", id: 2 });
    await vi.advanceTimersByTimeAsync(BRIDGE_ACK_TIMEOUT_MS * 5);
    expect(page.native.get).toHaveBeenCalledOnce();
    page.reply(2, { kind: "error", name: "NotAllowedError" });
    await expect(acked).rejects.toMatchObject({ name: "NotAllowedError" });
  });

  it("tells the bridge to drop a request it hands to the browser for want of an acknowledgement", async () => {
    vi.useFakeTimers();
    const page = install();
    const late = create(page, createOptions());
    await vi.advanceTimersByTimeAsync(BRIDGE_ACK_TIMEOUT_MS + 1);
    expect(await late).toBe("native-create");
    expect(page.posted.map((p) => p.message)).toEqual([
      expect.objectContaining({ type: "mosaic-passkey-req", id: 1 }),
      { type: "mosaic-passkey-abort", id: 1 },
    ]);
    // The late answer of a bridge that still forwarded it changes nothing.
    page.deliver({ type: "mosaic-passkey-ack", id: 1 });
    page.reply(1, { kind: "error", name: "NotAllowedError" });
    expect(page.native.create).toHaveBeenCalledOnce();
  });

  it.each(["NotAllowedError", "InvalidStateError"] as const)("turns the error %s into that DOMException", async (name) => {
    const page = install();
    const promise = create(page, createOptions());
    page.reply(1, { kind: "error", name });
    const error: unknown = await promise.catch((e: unknown) => e);
    expect(error).toBeInstanceOf(DOMException);
    expect(error).toMatchObject({ name });
  });

  it("answers a registration with a PublicKeyCredential that has own data properties", async () => {
    const page = install();
    const promise = create(page, createOptions({}, { extensions: { credProps: true } }));
    const id = encodeB64url(bytes(16, 1));
    page.reply(1, {
      kind: "created",
      credentialId: id,
      clientDataJson: encodeB64url(bytes(10, 2)),
      attestationObject: encodeB64url(bytes(12, 3)),
      authenticatorData: encodeB64url(bytes(14, 4)),
      publicKeySpki: encodeB64url(bytes(91, 5)),
      publicKeyAlgorithm: -7,
    });
    const credential = (await promise) as Record<string, unknown> & { response: Record<string, unknown> };
    expect(credential).toBeInstanceOf(PublicKeyCredential);
    expect(credential).toMatchObject({ id, type: "public-key", authenticatorAttachment: "platform" });
    expect(new Uint8Array(credential["rawId"] as ArrayBuffer)).toEqual(bytes(16, 1));
    expect(credential.response).toBeInstanceOf(AuthenticatorAttestationResponse);
    expect(new Uint8Array(credential.response["clientDataJSON"] as ArrayBuffer)).toEqual(bytes(10, 2));
    expect(new Uint8Array(credential.response["attestationObject"] as ArrayBuffer)).toEqual(bytes(12, 3));
    const response = credential.response as { getTransports(): string[]; getAuthenticatorData(): ArrayBuffer; getPublicKey(): ArrayBuffer; getPublicKeyAlgorithm(): number };
    expect(response.getTransports()).toEqual(["internal"]);
    expect(new Uint8Array(response.getAuthenticatorData())).toEqual(bytes(14, 4));
    expect(new Uint8Array(response.getPublicKey())).toEqual(bytes(91, 5));
    expect(response.getPublicKeyAlgorithm()).toBe(-7);
    expect((credential["getClientExtensionResults"] as () => unknown)()).toEqual({ credProps: { rk: true } });
    expect((credential["toJSON"] as () => unknown)()).toEqual({
      id,
      rawId: id,
      type: "public-key",
      authenticatorAttachment: "platform",
      response: {
        clientDataJSON: encodeB64url(bytes(10, 2)),
        attestationObject: encodeB64url(bytes(12, 3)),
        authenticatorData: encodeB64url(bytes(14, 4)),
        transports: ["internal"],
        publicKey: encodeB64url(bytes(91, 5)),
        publicKeyAlgorithm: -7,
      },
      clientExtensionResults: { credProps: { rk: true } },
    });
  });

  it("reports no extension results unless credProps was asked for", async () => {
    const page = install();
    const promise = create(page, createOptions());
    const field = encodeB64url(bytes(16, 1));
    page.reply(1, { kind: "created", credentialId: field, clientDataJson: field, attestationObject: field, authenticatorData: field, publicKeySpki: field, publicKeyAlgorithm: -7 });
    const credential = (await promise) as { getClientExtensionResults(): unknown };
    expect(credential.getClientExtensionResults()).toEqual({});
  });

  it("answers a sign-in with an assertion response", async () => {
    const page = install();
    const promise = get(page, getOptions());
    const id = encodeB64url(bytes(16, 1));
    page.reply(1, {
      kind: "asserted",
      credentialId: id,
      clientDataJson: encodeB64url(bytes(10, 2)),
      authenticatorData: encodeB64url(bytes(37, 3)),
      signature: encodeB64url(bytes(70, 4)),
      userHandle: encodeB64url(bytes(8, 5)),
    });
    const credential = (await promise) as { id: string; response: Record<string, ArrayBuffer>; toJSON(): { response: Record<string, string> } };
    expect(credential).toBeInstanceOf(PublicKeyCredential);
    expect(credential.id).toBe(id);
    expect(credential.response).toBeInstanceOf(AuthenticatorAssertionResponse);
    expect(new Uint8Array(credential.response["signature"] as ArrayBuffer)).toEqual(bytes(70, 4));
    expect(new Uint8Array(credential.response["userHandle"] as ArrayBuffer)).toEqual(bytes(8, 5));
    expect(credential.toJSON().response["signature"]).toBe(encodeB64url(bytes(70, 4)));
  });

  it("rejects an answer of the wrong kind for the call", async () => {
    const page = install();
    const promise = get(page, getOptions());
    const field = encodeB64url(bytes(16, 1));
    page.reply(1, { kind: "created", credentialId: field, clientDataJson: field, attestationObject: field, authenticatorData: field, publicKeySpki: field, publicKeyAlgorithm: -7 });
    await expect(promise).rejects.toMatchObject({ name: "NotAllowedError" });
  });

  it("ignores answers from any other source, malformed answers and answers for unknown calls", async () => {
    const page = install();
    const promise = get(page, getOptions());
    let settled = false;
    void promise.then(() => (settled = true), () => (settled = true));
    page.deliver({ type: "mosaic-passkey-res", id: 1, result: { kind: "error", name: "NotAllowedError" } }, {});
    page.deliver({ type: "mosaic-passkey-res", id: 1, result: { kind: "error", name: "Other" } });
    page.deliver({ type: "mosaic-passkey-res", id: 9, result: { kind: "error", name: "NotAllowedError" } });
    page.deliver({ type: "mosaic-passkey-res", id: 1, result: { kind: "error", name: "NotAllowedError" }, extra: true });
    page.deliver("text");
    page.deliver(null);
    await Promise.resolve();
    expect(settled).toBe(false);
    page.reply(1, { kind: "fallback" });
    expect(await promise).toBe("native-get");
  });
});

describe("abort", () => {
  it("rejects with the signal's reason, without a request, when already aborted", async () => {
    const page = install();
    const controller = new AbortController();
    controller.abort(new Error("stop"));
    await expect(get(page, getOptions({ signal: controller.signal }))).rejects.toThrow("stop");
    expect(page.posted).toEqual([]);
    expect(page.native.get).not.toHaveBeenCalled();
  });

  it("rejects with an AbortError when the signal carries no reason", async () => {
    const page = install();
    const fake = { aborted: true, reason: undefined } as unknown as AbortSignal;
    const error: unknown = await get(page, getOptions({ signal: fake })).catch((e: unknown) => e);
    expect(error).toMatchObject({ name: "AbortError" });
  });

  it("tells the bridge when the page aborts later, rejects, and ignores the late answer", async () => {
    const page = install();
    const controller = new AbortController();
    const promise = get(page, getOptions({ signal: controller.signal }));
    controller.abort();
    expect(page.last()).toEqual({ type: "mosaic-passkey-abort", id: 1 });
    await expect(promise).rejects.toMatchObject({ name: "AbortError" });
    page.reply(1, { kind: "fallback" });
    expect(page.native.get).not.toHaveBeenCalled();
  });

  it("stops listening to the signal once the call is answered", async () => {
    const page = install();
    const controller = new AbortController();
    const promise = get(page, getOptions({ signal: controller.signal }));
    page.reply(1, { kind: "fallback" });
    await promise;
    const postedBefore = page.posted.length;
    controller.abort();
    expect(page.posted).toHaveLength(postedBefore);
  });
});
