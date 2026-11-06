import { beforeEach, describe, expect, it, vi } from "vitest";
import { ExtensionError, type ErrorCode } from "../shared/errors.ts";
import type { PasskeyMessage } from "../shared/passkey-messages.ts";
import type { RequestMap, ResultMap } from "../shared/protocol.ts";
import { PasskeyService, senderOrigin, type PasskeyDeps } from "./passkey.ts";
import type { SenderInfo } from "./router.ts";

const BANK = "https://bank.test";
const CHALLENGE = "AAAAAAAAAAAAAAAAAAAAAA";
const ID = "AAAAAAAAAAAAAAAAAAAAAA";

type Request = { op: keyof RequestMap; body: unknown };

interface Harness {
  service: PasskeyService;
  requests: Request[];
  deps: { connection: string; enabled: boolean; tabUrl: string | undefined };
  /** Settles the nth native request. */
  settle(n: number, outcome: unknown): Promise<void>;
  reject(n: number, error: ErrorCode | Error): Promise<void>;
  clock: { now: number };
}

function harness(): Harness {
  const requests: Request[] = [];
  const pending: { resolve(v: unknown): void; reject(e: unknown): void }[] = [];
  const state = { connection: "unlocked", enabled: true, tabUrl: `${BANK}/login` as string | undefined };
  const clock = { now: 1_000_000 };
  let counter = 0;
  const deps: PasskeyDeps = {
    native: {
      request: ((op: keyof RequestMap, body: unknown) => {
        requests.push({ op, body });
        if (op === "passkeyCancel") {
          return Promise.resolve({});
        }
        return new Promise((resolve, reject) => pending.push({ resolve, reject }));
      }) as PasskeyDeps["native"]["request"],
      ensureSession: () => Promise.resolve(),
      get connection() {
        return state.connection;
      },
    },
    tabs: { urlOf: () => Promise.resolve(state.tabUrl) },
    now: () => clock.now,
    randomHex: () => (++counter).toString(16).padStart(32, "0"),
    enabled: () => Promise.resolve(state.enabled),
  };
  return {
    service: new PasskeyService(deps),
    requests,
    deps: state,
    clock,
    settle: async (n, outcome) => {
      pending[n]?.resolve(outcome);
      await new Promise((resolve) => setTimeout(resolve, 0));
    },
    reject: async (n, error) => {
      pending[n]?.reject(typeof error === "string" ? new ExtensionError(error) : error);
      await new Promise((resolve) => setTimeout(resolve, 0));
    },
  };
}

const sender = (over: Partial<SenderInfo> = {}): SenderInfo => ({
  id: "ext",
  origin: BANK,
  url: `${BANK}/login`,
  tab: { id: 7, url: `${BANK}/login` },
  frameId: 0,
  documentId: "doc-1",
  ...over,
});

const createMessage = (over: Partial<Extract<PasskeyMessage, { op: "create" }>> = {}): PasskeyMessage => ({
  kind: "passkey",
  op: "create",
  pageRequestId: 1,
  origin: BANK,
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
  ...over,
});

const getMessage = (pageRequestId = 1): PasskeyMessage => ({
  kind: "passkey",
  op: "get",
  pageRequestId,
  origin: BANK,
  options: { rpId: null, challenge: CHALLENGE, allowCredentials: [ID], userVerification: "required" },
});

const created: ResultMap["passkeyCreate"] = {
  outcome: "created",
  credential: {
    credentialId: ID,
    clientDataJson: "AAAA",
    attestationObject: "BBBB",
    authenticatorData: "CCCC",
    publicKeySpki: "DDDD",
    publicKeyAlgorithm: -7,
    vaultId: "v",
    itemId: "i",
  },
};

const asserted: ResultMap["passkeyGet"] = {
  outcome: "asserted",
  assertion: { credentialId: ID, clientDataJson: "AAAA", authenticatorData: "CCCC", signature: "EEEE", userHandle: "AQID" },
};

const FALLBACK = { kind: "fallback" };

let h: Harness;
beforeEach(() => {
  h = harness();
});

describe("senderOrigin", () => {
  it("prefers the origin the browser reports and falls back to the URL's origin", () => {
    expect(senderOrigin({ origin: BANK, url: "https://other.test/x" })).toBe(BANK);
    expect(senderOrigin({ url: `${BANK}:8443/x?y=1` })).toBe(`${BANK}:8443`);
    expect(senderOrigin({ url: "not a url" })).toBeNull();
    expect(senderOrigin({})).toBeNull();
  });
});

describe("who may ask", () => {
  const cases: [string, Partial<SenderInfo>, Partial<PasskeyMessage>][] = [
    ["a subframe", { frameId: 1 }, {}],
    ["a sender with no frame id", { frameId: undefined }, {}],
    ["a sender with no tab", { tab: undefined }, {}],
    ["a sender whose origin differs from the claim", {}, { origin: "https://evil.test" }],
    ["a bridge that claims a different origin than the browser reports", { origin: "https://evil.test" }, {}],
    ["an http page", { origin: "http://bank.test", url: "http://bank.test/" }, { origin: "http://bank.test" }],
    ["an opaque origin", { origin: "null" }, { origin: "null" }],
    ["a tab that shows another site", { tab: { id: 7 } }, {}],
  ];

  it.each(cases)("%s gets a fallback and the desktop is not asked", async (_name, senderOver, messageOver) => {
    if (_name === "a tab that shows another site") {
      h.deps.tabUrl = "https://evil.test/";
    }
    const answer = await h.service.handle({ ...createMessage(), ...messageOver } as PasskeyMessage, sender(senderOver));
    expect(answer).toEqual(FALLBACK);
    expect(h.requests).toEqual([]);
  });

  it("takes the origin from the url where the browser gives no sender origin (Firefox)", async () => {
    const pending = h.service.handle(getMessage(), sender({ origin: undefined, url: `${BANK}/login` }));
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    await h.settle(0, asserted);
    expect((await pending).kind).toBe("asserted");
  });
});

describe("the request to the desktop", () => {
  it("builds the body from the sender and its own request id", async () => {
    const pending = h.service.handle(createMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    expect(h.requests[0]).toEqual({
      op: "passkeyCreate",
      body: {
        requestId: "00000000000000000000000000000001",
        origin: BANK,
        topOrigin: BANK,
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
    await h.settle(0, created);
    expect(await pending).toEqual({
      kind: "created",
      credentialId: ID,
      clientDataJson: "AAAA",
      attestationObject: "BBBB",
      authenticatorData: "CCCC",
      publicKeySpki: "DDDD",
      publicKeyAlgorithm: -7,
    });
  });

  it("maps a sign-in answer to the assertion", async () => {
    const pending = h.service.handle(getMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    expect(h.requests[0]?.op).toBe("passkeyGet");
    expect(h.requests[0]?.body).toMatchObject({ origin: BANK, topOrigin: BANK, rpId: null, allowCredentials: [ID], userVerification: "required" });
    await h.settle(0, asserted);
    expect(await pending).toEqual({ kind: "asserted", ...asserted.assertion });
  });

  it("makes a new request id for every call, whatever the page numbered it", async () => {
    const a = h.service.handle(getMessage(5), sender());
    const b = h.service.handle(getMessage(5), sender({ documentId: "doc-2", tab: { id: 8, url: `${BANK}/login` } }));
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(2);
    });
    const ids = h.requests.map((r) => (r.body as { requestId: string }).requestId);
    expect(new Set(ids).size).toBe(2);
    await h.settle(0, asserted);
    await h.settle(1, asserted);
    await Promise.all([a, b]);
  });

  it("lets a page have two requests open and refuses a third", async () => {
    const answers = [1, 2, 3].map((n) => h.service.handle(getMessage(n), sender()));
    expect(await answers[2]).toEqual(FALLBACK);
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(2);
    });
    await h.settle(0, asserted);
    await h.settle(1, asserted);
    await Promise.all(answers);
  });
});

describe("fallback without asking the desktop", () => {
  it.each(["locked", "notPaired", "desktopNotRunning", "hostMissing", "checking"])("a link in state %s", async (connection) => {
    h.deps.connection = connection;
    expect(await h.service.handle(getMessage(), sender())).toEqual(FALLBACK);
    expect(h.requests).toEqual([]);
  });

  it("the setting off", async () => {
    h.deps.enabled = false;
    expect(await h.service.handle(getMessage(), sender())).toEqual(FALLBACK);
    expect(h.requests).toEqual([]);
  });
});

describe("what the desktop answers", () => {
  it("uses another device: a fallback, also when the answer is incomplete", async () => {
    let pending = h.service.handle(createMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    await h.settle(0, { outcome: "useAnotherDevice", credential: null });
    expect(await pending).toEqual(FALLBACK);

    pending = h.service.handle(getMessage(2), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(2);
    });
    await h.settle(1, { outcome: "asserted", assertion: null });
    expect(await pending).toEqual(FALLBACK);
  });

  const errors: [ErrorCode, unknown][] = [
    ["cancelled", { kind: "error", name: "NotAllowedError" }],
    ["busy", { kind: "error", name: "NotAllowedError" }],
    ["timeout", { kind: "error", name: "NotAllowedError" }],
    ["passkeyExcluded", { kind: "error", name: "InvalidStateError" }],
    // The desktop's rate windows are the pairing's, shared by every site: one site spending them must not fail another's call.
    ["rateLimited", FALLBACK],
    ["passkeyNotFound", FALLBACK],
    ["passkeyUnsupported", FALLBACK],
    ["rpIdMismatch", FALLBACK],
    ["insecureOrigin", FALLBACK],
    ["crossOriginFrame", FALLBACK],
    ["locked", FALLBACK],
    ["notPaired", FALLBACK],
    ["desktopNotRunning", FALLBACK],
    ["invalidRequest", FALLBACK],
    ["internal", FALLBACK],
    ["hostExited", FALLBACK],
  ];
  it.each(errors)("maps the error %s", async (code, expected) => {
    const pending = h.service.handle(getMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    await h.reject(0, code);
    expect(await pending).toEqual(expected);
  });

  it("treats a failure that is not an ExtensionError as a fallback", async () => {
    const pending = h.service.handle(getMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    await h.reject(0, new Error("boom"));
    expect(await pending).toEqual(FALLBACK);
  });
});

describe("cancelling", () => {
  const open = async (): Promise<string> => {
    void h.service.handle(getMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests).toHaveLength(1);
    });
    return (h.requests[0]?.body as { requestId: string }).requestId;
  };
  const cancels = (): Request[] => h.requests.filter((r) => r.op === "passkeyCancel");

  it("cancels the request of a page that aborts, once", async () => {
    const requestId = await open();
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender());
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender());
    expect(cancels()).toEqual([{ op: "passkeyCancel", body: { requestId } }]);
  });

  it("does not let another tab, another document, another frame or another request id abort it", async () => {
    await open();
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender({ tab: { id: 8, url: BANK } }));
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender({ documentId: "doc-9" }));
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender({ frameId: 2 }));
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 2 }, sender());
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender({ tab: undefined }));
    expect(cancels()).toEqual([]);
  });

  it("cancels when the tab is closed, and only that tab's requests", async () => {
    const requestId = await open();
    h.service.onTabGone(99);
    expect(cancels()).toEqual([]);
    h.service.onTabGone(7);
    expect(cancels()).toEqual([{ op: "passkeyCancel", body: { requestId } }]);
  });

  it("cancels when the tab loads another document or moves to another site, not on a same-site move", async () => {
    const requestId = await open();
    h.service.onNavigated(7, `${BANK}/other`);
    expect(cancels()).toEqual([]);
    h.service.onNavigated(7, "https://evil.test/");
    expect(cancels()).toEqual([{ op: "passkeyCancel", body: { requestId } }]);
    h.service.onTabGone(7);
    expect(cancels()).toHaveLength(1);
  });

  it("cancels on a navigation that names no URL (a reload)", async () => {
    await open();
    h.service.onNavigated(7);
    expect(cancels()).toHaveLength(1);
  });

  it("never asks the desktop for a request the page aborted before it was sent", async () => {
    const pending = h.service.handle(getMessage(), sender());
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender());
    expect(await pending).toEqual({ kind: "error", name: "NotAllowedError" });
    expect(h.requests).toEqual([]);
  });

  it("never asks the desktop for a request whose tab moved on before it was sent", async () => {
    const moves = [
      () => {
        h.service.onNavigated(7);
      },
      () => {
        h.service.onNavigated(7, "https://evil.test/");
      },
      () => {
        h.service.onTabGone(7);
      },
    ];
    for (const [n, move] of moves.entries()) {
      const pending = h.service.handle(getMessage(n + 1), sender());
      move();
      expect(await pending).toEqual({ kind: "error", name: "NotAllowedError" });
    }
    expect(h.requests).toEqual([]);
  });

  it("keeps a request that moved within its own site before it was sent", async () => {
    void h.service.handle(getMessage(), sender());
    h.service.onNavigated(7, `${BANK}/other`);
    await vi.waitFor(() => {
      expect(h.requests.map((r) => r.op)).toEqual(["passkeyGet"]);
    });
  });

  it("frees the page's slot when an unsent request is aborted", async () => {
    const first = h.service.handle(getMessage(1), sender());
    h.service.abort({ kind: "passkeyAbort", pageRequestId: 1 }, sender());
    await first;
    void h.service.handle(getMessage(2), sender());
    void h.service.handle(getMessage(3), sender());
    await vi.waitFor(() => {
      expect(h.requests.filter((r) => r.op === "passkeyGet")).toHaveLength(2);
    });
  });

  it("forgets requests that outlived the desktop's own timeout", async () => {
    await open();
    h.clock.now += 400_000;
    void h.service.handle(getMessage(), sender());
    await vi.waitFor(() => {
      expect(h.requests.filter((r) => r.op === "passkeyGet")).toHaveLength(2);
    });
  });
});
