import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CapturedMessage } from "../shared/messages.ts";
import type { RequestMap, ResultMap } from "../shared/protocol.ts";
import type { CaptureRegistry } from "./capture-registry.ts";
import { createMemoryStore } from "./db.ts";
import type { SenderInfo } from "./router.ts";
import { captureSite, neverSaveKey, PENDING_SAVE_TTL_MS, SaveService, type SaveDeps } from "./save.ts";
import { memorySession, pendingSaveKey } from "./session.ts";

const TOP = "https://bank.test";
const TAB = 7;

const captured = (over: Partial<CapturedMessage> = {}): CapturedMessage => ({
  type: "captured",
  username: "ada",
  password: "s3cret-pw",
  newPassword: null,
  ancestorsSameOrigin: true,
  ...over,
});

const sender = (over: Partial<SenderInfo> = {}): SenderInfo => ({
  id: "ext",
  origin: TOP,
  url: `${TOP}/login`,
  frameId: 0,
  tab: { id: TAB, url: `${TOP}/login` },
  ...over,
});

interface Harness {
  service: SaveService;
  deps: SaveDeps;
  requests: { op: string; body: unknown }[];
  badge: { show: ReturnType<typeof vi.fn>; hide: ReturnType<typeof vi.fn> };
  registry: { access: boolean; registered: boolean };
  state: { connection: string; check: ResultMap["checkSave"]; outcome: ResultMap["proposeSave"]["outcome"]; fail: string | null };
  now: { value: number };
}

function harness(): Harness {
  const requests: Harness["requests"] = [];
  const badge = { show: vi.fn(), hide: vi.fn() };
  const registry = { access: true, registered: true };
  const state: Harness["state"] = {
    connection: "unlocked",
    check: { kind: "new", itemTitle: null },
    outcome: "saved",
    fail: null,
  };
  const now = { value: 1_000_000 };
  const capture: CaptureRegistry = {
    hasAccess: () => Promise.resolve(registry.access),
    isRegistered: () => Promise.resolve(registry.registered),
    register: () => {
      registry.registered = true;
      return Promise.resolve();
    },
    unregister: () => {
      registry.registered = false;
      return Promise.resolve();
    },
  };
  const deps: SaveDeps = {
    native: {
      get connection() {
        return state.connection;
      },
      ensureSession: () => Promise.resolve(),
      request: <O extends "checkSave" | "proposeSave">(op: O, body: RequestMap[O]): Promise<ResultMap[O]> => {
        requests.push({ op, body });
        if (state.fail !== null) {
          return Promise.reject(new Error(state.fail));
        }
        const result =
          op === "checkSave"
            ? { ...state.check }
            : { outcome: state.outcome, vaultId: null, itemId: null };
        return Promise.resolve(result as ResultMap[O]);
      },
    },
    session: memorySession(),
    kv: createMemoryStore(),
    badge: { show: (t) => Promise.resolve(badge.show(t)), hide: (t) => Promise.resolve(badge.hide(t)) },
    capture,
    now: () => now.value,
  };
  return { service: new SaveService(deps), deps, requests, badge, registry, state, now };
}

let h: Harness;
beforeEach(() => {
  h = harness();
});

const capture = (message: CapturedMessage, from: SenderInfo = sender()): Promise<void> =>
  h.service.capture(message, from);

const stored = async (): Promise<unknown> => h.deps.session.get(pendingSaveKey(TAB));

describe("where a capture came from", () => {
  it("accepts the top frame, taking origin and tab from the browser", () => {
    expect(captureSite(captured(), sender())).toEqual({ tabId: TAB, origin: TOP, topOrigin: TOP });
  });

  it("accepts a same-origin frame only with same-origin ancestors", () => {
    const frame = sender({ frameId: 3 });
    expect(captureSite(captured({ ancestorsSameOrigin: true }), frame)).not.toBeNull();
    expect(captureSite(captured({ ancestorsSameOrigin: false }), frame)).toBeNull();
  });

  it("drops cross-origin frames, opaque origins and senders without a tab", () => {
    expect(captureSite(captured(), sender({ frameId: 2, origin: "https://evil.test" }))).toBeNull();
    expect(captureSite(captured(), sender({ origin: "null" }))).toBeNull();
    expect(captureSite(captured(), sender({ tab: undefined }))).toBeNull();
    expect(captureSite(captured(), sender({ tab: { id: TAB, url: "about:blank" } }))).toBeNull();
  });

  it("falls back to the sender URL when the browser gives no origin", () => {
    expect(captureSite(captured(), sender({ origin: undefined }))).toEqual({ tabId: TAB, origin: TOP, topOrigin: TOP });
  });
});

describe("a captured login", () => {
  it("is asked about, stored with the answer and shown by the badge", async () => {
    h.state.check = { kind: "update", itemTitle: "Bank" };
    await capture(captured());
    expect(h.requests).toEqual([
      { op: "checkSave", body: { origin: TOP, topOrigin: TOP, username: "ada", password: "s3cret-pw" } },
    ]);
    expect(await stored()).toMatchObject({ origin: TOP, topOrigin: TOP, kind: "update", itemTitle: "Bank", source: "capture" });
    expect(h.badge.show).toHaveBeenCalledWith(TAB);
  });

  it("saves the new password of a change form", async () => {
    await capture(captured({ password: "old-pw", newPassword: "new-pw" }));
    expect(h.requests[0]?.body).toMatchObject({ password: "new-pw" });
    expect(await stored()).toMatchObject({ password: "new-pw" });
  });

  it("is dropped when the desktop says it is saved already", async () => {
    h.state.check = { kind: "unchanged", itemTitle: "Bank" };
    await capture(captured());
    expect(await stored()).toBeUndefined();
    expect(h.badge.show).not.toHaveBeenCalled();
  });

  it("is kept without an answer while the desktop is locked, and asked about when the popup opens", async () => {
    h.state.connection = "locked";
    await capture(captured());
    expect(h.requests).toEqual([]);
    expect(await stored()).toMatchObject({ kind: null });
    h.state.connection = "unlocked";
    h.state.check = { kind: "new", itemTitle: null };
    expect(await h.service.info(TAB, TOP)).toMatchObject({ host: "bank.test", username: "ada", kind: "new", source: "capture", insecure: false });
    expect(h.requests.map((r) => r.op)).toEqual(["checkSave"]);
  });

  it("is dropped when the popup learns it is unchanged", async () => {
    h.state.connection = "locked";
    await capture(captured());
    h.state.connection = "unlocked";
    h.state.check = { kind: "unchanged", itemTitle: null };
    expect(await h.service.info(TAB, TOP)).toBeNull();
    expect(await stored()).toBeUndefined();
  });

  it("is kept with an unknown kind when the desktop cannot answer", async () => {
    h.state.fail = "rateLimited";
    await capture(captured());
    expect(await stored()).toMatchObject({ kind: null });
  });

  it("needs capture to be switched on, and respects Never for this site", async () => {
    h.registry.registered = false;
    await capture(captured());
    expect(await stored()).toBeUndefined();
    h.registry.registered = true;
    await h.deps.kv.set(neverSaveKey(TOP), true);
    await capture(captured());
    expect(await stored()).toBeUndefined();
    expect(h.requests).toEqual([]);
  });

  it("is dropped when the browser took the site access back, and capture switches itself off", async () => {
    h.registry.access = false;
    await capture(captured());
    expect(await stored()).toBeUndefined();
    expect(h.registry.registered).toBe(false);
  });

  it("ignores passwords outside 1 to 1024 characters", async () => {
    await capture(captured({ password: "x".repeat(1025) }));
    expect(await stored()).toBeUndefined();
  });
});

describe("the pending save is bound to its tab, origin and time", () => {
  beforeEach(async () => {
    await capture(captured());
  });

  it("is offered only on the origin it came from; elsewhere it is dropped", async () => {
    expect(await h.service.info(TAB, "https://evil.test")).toBeNull();
    expect(await stored()).toBeUndefined();
    expect(h.badge.hide).toHaveBeenCalledWith(TAB);
  });

  it("expires after five minutes", async () => {
    h.now.value += PENDING_SAVE_TTL_MS;
    expect(await h.service.info(TAB, TOP)).toBeNull();
    expect(await stored()).toBeUndefined();
  });

  it("is dropped when the tab navigates to another site, not within the site", async () => {
    await h.service.onNavigated(TAB, `${TOP}/dashboard`);
    expect(await stored()).toBeDefined();
    await h.service.onNavigated(TAB, "https://other.test/");
    expect(await stored()).toBeUndefined();
  });

  it("holds the password in session storage only, which the popup state never shows", async () => {
    const info = await h.service.info(TAB, TOP);
    expect(JSON.stringify(info)).not.toContain("s3cret-pw");
  });
});

describe("the user's answer", () => {
  beforeEach(async () => {
    await capture(captured());
    h.requests.length = 0;
  });

  it("Save asks the desktop, which confirms in its own dialog, then forgets the password", async () => {
    const result = await h.service.act(TAB, TOP, "save");
    expect(result).toEqual({ outcome: "saved" });
    expect(h.requests).toEqual([
      { op: "proposeSave", body: { origin: TOP, topOrigin: TOP, username: "ada", password: "s3cret-pw" } },
    ]);
    expect(await stored()).toBeUndefined();
  });

  it("forgets the password when the desktop dialog was declined too", async () => {
    h.state.outcome = "cancelled";
    expect(await h.service.act(TAB, TOP, "save")).toEqual({ outcome: "cancelled" });
    expect(await stored()).toBeUndefined();
  });

  it("keeps it when the desktop could not ask (busy, locked, timeout)", async () => {
    h.state.fail = "busy";
    await expect(h.service.act(TAB, TOP, "save")).rejects.toThrow("busy");
    expect(await stored()).toBeDefined();
    h.state.fail = null;
    expect(await h.service.act(TAB, TOP, "save")).toEqual({ outcome: "saved" });
  });

  it("allows one save prompt at a time", async () => {
    let finish: (() => void) | undefined;
    const slow = h.deps.native.request.bind(h.deps.native);
    h.deps.native.request = ((op: "checkSave" | "proposeSave", body: never) =>
      new Promise((resolve) => {
        finish = () => {
          resolve(slow(op, body));
        };
      })) as unknown as typeof slow;
    const first = h.service.act(TAB, TOP, "save");
    await expect(h.service.act(TAB, TOP, "save")).rejects.toThrow("busy");
    finish?.();
    await first;
  });

  it("Not now forgets it without asking the desktop", async () => {
    expect(await h.service.act(TAB, TOP, "dismiss")).toEqual({ outcome: "dismissed" });
    expect(h.requests).toEqual([]);
    expect(await stored()).toBeUndefined();
  });

  it("Never for this site is remembered for the origin and stops later offers", async () => {
    expect(await h.service.act(TAB, TOP, "never")).toEqual({ outcome: "never" });
    expect(await h.service.isNeverSave(TOP)).toBe(true);
    expect(await h.service.isNeverSave("https://other.test")).toBe(false);
    await capture(captured());
    expect(await stored()).toBeUndefined();
  });

  it("an answer for a tab on another origin, or with nothing pending, does nothing", async () => {
    expect(await h.service.act(TAB, "https://evil.test", "save")).toEqual({ outcome: "dismissed" });
    expect(await h.service.act(TAB + 1, TOP, "save")).toEqual({ outcome: "dismissed" });
    expect(h.requests).toEqual([]);
  });
});

describe("a generated password", () => {
  it("is offered for saving on its tab and origin, with the username from the form", async () => {
    await h.service.storeGenerated({ tabId: TAB, origin: TOP, topOrigin: TOP, username: " ada@bank.test ", password: "Gen-1" });
    expect(await stored()).toMatchObject({ source: "generated", username: "ada@bank.test", password: "Gen-1", kind: null });
    expect(h.badge.show).toHaveBeenCalledWith(TAB);
    expect(await h.service.info(TAB, "https://other.test")).toBeNull();
  });

  it("is not offered for a site marked Never, and capture need not be on", async () => {
    h.registry.registered = false;
    await h.service.storeGenerated({ tabId: TAB, origin: TOP, topOrigin: TOP, username: null, password: "Gen-1" });
    expect(await stored()).toBeDefined();
    await h.deps.session.remove(pendingSaveKey(TAB));
    await h.deps.kv.set(neverSaveKey(TOP), true);
    await h.service.storeGenerated({ tabId: TAB, origin: TOP, topOrigin: TOP, username: null, password: "Gen-2" });
    expect(await stored()).toBeUndefined();
  });
});

describe("the capture setting", () => {
  it("needs the site access the popup asked for", async () => {
    h.registry.registered = false;
    h.registry.access = false;
    await expect(h.service.setOffers(true)).rejects.toThrow("permissionDenied");
    expect(h.registry.registered).toBe(false);
    h.registry.access = true;
    expect(await h.service.setOffers(true)).toBe(true);
    expect(h.registry.registered).toBe(true);
    expect(await h.service.setOffers(true)).toBe(true);
  });

  it("switching it off unregisters the script and drops captured passwords, not generated ones", async () => {
    await capture(captured());
    await h.service.storeGenerated({ tabId: 9, origin: TOP, topOrigin: TOP, username: null, password: "Gen-1" });
    expect(await h.service.setOffers(false)).toBe(false);
    expect(h.registry.registered).toBe(false);
    expect(await stored()).toBeUndefined();
    expect(await h.deps.session.get(pendingSaveKey(9))).toBeDefined();
  });
});
