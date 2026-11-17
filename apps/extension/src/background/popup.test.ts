import { beforeEach, describe, expect, it, vi } from "vitest";
import type { FrameProbe } from "../shared/messages.ts";
import { createMemoryStore } from "./db.ts";
import { GeneratedHold, type FillDeps } from "./fill.ts";
import type { NativeClient } from "./native.ts";
import { PasskeySwitch, type PasskeyRegistry } from "./passkey-registry.ts";
import { PopupService } from "./popup.ts";
import { RotationService, type RotationDeps } from "./rotation.ts";
import { neverSaveKey, SaveService } from "./save.ts";
import { lastErrorKey, memorySession, pendingSaveKey } from "./session.ts";

const TOP = "https://bank.test";
const TAB = { id: 3, url: `${TOP}/signup` };

const probe = (over: Partial<FrameProbe> = {}): FrameProbe => ({
  frameId: 0,
  documentId: "doc-0",
  origin: TOP,
  ancestorsSameOrigin: true,
  frameVisible: true,
  depth: 0,
  activeField: false,
  counts: { username: 1, password: 0, newPassword: 2, otp: 0 },
  passwordRules: "minlength: 12;",
  visible: true,
  changeShape: "none",
  ...over,
});

interface Harness {
  service: PopupService;
  hold: GeneratedHold;
  save: SaveService;
  kv: ReturnType<typeof createMemoryStore>;
  requests: { op: string; body: unknown }[];
  session: ReturnType<typeof memorySession>;
  badge: { clear: ReturnType<typeof vi.fn> };
  setProbes: (probes: FrameProbe[]) => void;
  /** Per-op replacements for the desktop's answer, checked before the defaults. */
  handlers: Record<string, (body: unknown) => Promise<unknown>>;
  fill: FillDeps;
  passkeys: PasskeySwitch;
  rotation: RotationService;
}

function harness(): Harness {
  let probes: FrameProbe[] = [probe()];
  const requests: Harness["requests"] = [];
  const session = memorySession();
  const badge = { clear: vi.fn(() => Promise.resolve()) };
  const handlers: Harness["handlers"] = {};
  const native = {
    connection: "unlocked",
    ensureSession: () => Promise.resolve(),
    request: (op: string, body: unknown) => {
      requests.push({ op, body });
      const handler = handlers[op];
      if (handler !== undefined) {
        return handler(body);
      }
      const results: Record<string, unknown> = {
        listForOrigin: { items: [], warnings: [] },
        rotationForOrigin: { attempts: [], related: false },
        generate: { value: "Gen-1", entropyBits: 100, strength: "strong", rulesSource: "page" },
        checkSave: { kind: "new", itemTitle: null },
        proposeSave: { outcome: "saved", vaultId: null, itemId: null },
      };
      return Promise.resolve(results[op]);
    },
  };
  const fill: FillDeps = {
    native: native as unknown as FillDeps["native"],
    tabs: {
      inject: () => Promise.resolve(),
      probe: () => Promise.resolve(probes),
      send: () => Promise.resolve({ ok: true, filled: { username: 0, password: 0, otp: 0, newPassword: 2 }, usernameValue: "ada" }),
      navigate: () => Promise.resolve(),
    },
    session,
    badge: { set: () => Promise.resolve(), clear: badge.clear, openPopup: () => Promise.resolve() },
    now: () => 1_000_000,
  };
  const kv = createMemoryStore();
  const save = new SaveService({
    native: native as unknown as ConstructorParameters<typeof SaveService>[0]["native"],
    session,
    kv,
    badge: { show: () => Promise.resolve(), hide: () => Promise.resolve() },
    capture: {
      hasAccess: () => Promise.resolve(true),
      isRegistered: () => Promise.resolve(false),
      register: () => Promise.resolve(),
      unregister: () => Promise.resolve(),
    },
    now: () => 1_000_000,
  });
  fill.onGenerated = save.storeGenerated;
  const hold = new GeneratedHold(() => 1_000_000);
  const registry: PasskeyRegistry = {
    hasAccess: () => Promise.resolve(true),
    isRegistered: () => Promise.resolve(false),
    register: () => Promise.resolve(),
    unregister: () => Promise.resolve(),
  };
  const passkeys = new PasskeySwitch(registry);
  const rotation = new RotationService({ native: native as unknown as RotationDeps["native"], tabs: fill.tabs, session, now: () => 1_000_000 });
  const service = new PopupService({ native: native as unknown as NativeClient, fill, hold, save, passkeys, rotation }, () => null);
  return { service, hold, save, kv, requests, session, badge, setProbes: (p) => (probes = p), handlers, fill, passkeys, rotation };
}

let h: Harness;
beforeEach(() => {
  h = harness();
});

describe("generate", () => {
  it("takes the host and the page's rules from the tab, whatever the popup sent", async () => {
    await h.service.handle({ op: "generate", request: { kind: "forSite", host: "evil.test", rules: "x" } }, TAB);
    expect(h.requests.find((r) => r.op === "generate")?.body).toEqual({
      request: { kind: "forSite", host: "bank.test", rules: "minlength: 12;" },
    });
  });

  it("passes explicit options through, and holds the value for this tab and origin only", async () => {
    const options = { kind: "password", length: 20 };
    await h.service.handle({ op: "generate", request: options }, TAB);
    expect(h.requests.find((r) => r.op === "generate")?.body).toEqual({ request: options });
    expect(h.hold.has({ tabId: TAB.id, origin: TOP })).toBe(true);
    expect(h.hold.has({ tabId: TAB.id, origin: "https://evil.test" })).toBe(false);
  });

  it("refuses on a page that is not http(s)", async () => {
    await expect(h.service.handle({ op: "generate", request: { kind: "forSite" } }, { id: 1, url: "about:blank" })).rejects.toThrow("unsupportedPage");
  });
});

describe("fillGenerated from the popup", () => {
  it("fills the held password, clears the hold and offers the save", async () => {
    await h.service.handle({ op: "generate", request: { kind: "password" } }, TAB);
    await h.service.handle({ op: "fillGenerated", confirmFrame: null }, TAB);
    expect(h.hold.has({ tabId: TAB.id, origin: TOP })).toBe(false);
    expect(await h.session.get(pendingSaveKey(TAB.id))).toMatchObject({
      origin: TOP,
      topOrigin: TOP,
      username: "ada",
      password: "Gen-1",
      source: "generated",
    });
    const state = await h.service.handle({ op: "getState" }, TAB);
    expect(state).toMatchObject({ pendingSave: { host: "bank.test", username: "ada", source: "generated" } });
    expect(JSON.stringify(state)).not.toContain("Gen-1");
  });

  it("refuses when nothing is held, instead of making up another password", async () => {
    await expect(h.service.handle({ op: "fillGenerated", confirmFrame: null }, TAB)).rejects.toThrow("generatedExpired");
    expect(h.requests.filter((r) => r.op === "generate")).toEqual([]);
  });
});

describe("getState", () => {
  it("names the frame the popup shows, for a confirmation", async () => {
    h.setProbes([probe({ frameId: 4, documentId: "doc-4", activeField: true, depth: 1 })]);
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({
      frame: "sameOriginFrame",
      frameTarget: { frameId: 4, documentId: "doc-4" },
      generatorFill: true,
    });
  });

  it("has no frame target without a frame", async () => {
    h.setProbes([]);
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({ frame: "none", frameTarget: null });
  });

  it("shows a failed shortcut fill once", async () => {
    await h.session.set(lastErrorKey(TAB.id), "noFields");
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({ lastError: "noFields" });
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({ lastError: null });
    expect(h.badge.clear).toHaveBeenCalledWith(TAB.id);
  });

  it("reports whether save capture is on", async () => {
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({ saveOffers: false });
  });

  it("reports the passkey setting, which is off until the scripts are registered", async () => {
    expect(await h.service.handle({ op: "getState" }, TAB)).toMatchObject({ passkeys: false });
    expect(await h.service.handle({ op: "setPasskeys", enabled: true }, TAB)).toEqual({ passkeys: true });
    expect(await h.service.handle({ op: "setPasskeys", enabled: false }, TAB)).toEqual({ passkeys: false });
  });
});

const LOGIN = { vaultId: "v1", itemId: "i1", title: "Bank", username: "ada", hasOtp: false };
const FRAME = "https://pay.bank.test";
const listFor = (origin: string) => ({ items: [{ ...LOGIN, itemId: `item-of-${origin}` }], warnings: [] });
const listRequests = (h: Harness) => h.requests.filter((r) => r.op === "listForOrigin").map((r) => r.body);

describe("getState loads its lookups together", () => {
  it("lists and offers rotations for a normal tab, with one list request", async () => {
    h.handlers.listForOrigin = (body) => Promise.resolve(listFor((body as { origin: string }).origin));
    h.handlers.rotationForOrigin = () => Promise.resolve({ attempts: [{ attemptId: "a1" }], related: false });
    h.setProbes([probe({ changeShape: "currentAndNew" })]);
    const state = await h.service.handle({ op: "getState" }, TAB);
    expect(state).toMatchObject({
      frame: "top",
      matches: listFor(TOP).items,
      warnings: [],
      rotations: [{ attemptId: "a1" }],
      lastError: null,
      nextOtp: null,
      saveOffers: false,
      passkeys: false,
      pendingSave: null,
    });
    expect(listRequests(h)).toEqual([{ origin: TOP, topOrigin: TOP }]);
  });

  it("drops the speculative list and asks again when the chosen frame has another origin", async () => {
    h.handlers.listForOrigin = (body) => Promise.resolve(listFor((body as { origin: string }).origin));
    h.setProbes([probe({ frameId: 4, documentId: "doc-4", origin: FRAME, activeField: true, depth: 1, ancestorsSameOrigin: false })]);
    const state = await h.service.handle({ op: "getState" }, TAB);
    expect(state.frame).toBe("crossOriginFrame");
    expect(listRequests(h)).toEqual([
      { origin: TOP, topOrigin: TOP },
      { origin: FRAME, topOrigin: TOP },
    ]);
    // Only the list for the frame's origin reached the state.
    expect(state.matches).toEqual(listFor(FRAME).items);
    expect(JSON.stringify(state)).not.toContain(`item-of-${TOP}`);
  });

  it("keeps the other lookups when one optional lookup fails", async () => {
    h.handlers.listForOrigin = () => Promise.reject(new Error("desktop gone"));
    h.handlers.rotationForOrigin = () => Promise.resolve({ attempts: [{ attemptId: "a1" }], related: false });
    vi.spyOn(h.passkeys, "enabled").mockResolvedValue(true);
    let state = await h.service.handle({ op: "getState" }, TAB);
    expect(state).toMatchObject({ matches: [], warnings: [], rotations: [{ attemptId: "a1" }], passkeys: true });

    h.handlers.listForOrigin = () => Promise.resolve(listFor(TOP));
    h.handlers.rotationForOrigin = () => Promise.reject(new Error("old desktop"));
    state = await h.service.handle({ op: "getState" }, TAB);
    expect(state).toMatchObject({ matches: listFor(TOP).items, rotations: [], rotationTabOffer: null, passkeys: true });

    vi.spyOn(h.passkeys, "enabled").mockRejectedValue(new Error("x"));
    vi.spyOn(h.save, "offersEnabled").mockRejectedValue(new Error("x"));
    vi.spyOn(h.save, "info").mockRejectedValue(new Error("x"));
    state = await h.service.handle({ op: "getState" }, TAB);
    expect(state).toMatchObject({ matches: listFor(TOP).items, passkeys: false, saveOffers: false, pendingSave: null });
  });

  it("still aborts when reading the last error or the pending code fails", async () => {
    vi.spyOn(h.session, "get").mockRejectedValue(new Error("session gone"));
    await expect(h.service.handle({ op: "getState" }, TAB)).rejects.toThrow();
  });

  it("overlaps the probe, the list and the rotation lookup", async () => {
    vi.useFakeTimers();
    try {
      const later = <T>(value: T) => new Promise<T>((resolve) => setTimeout(() => resolve(value), 100));
      h.handlers.listForOrigin = () => later({ items: [], warnings: [] });
      h.handlers.rotationForOrigin = () => later({ attempts: [], related: false });
      h.fill.tabs.probe = () => later([probe()]);
      let done = false;
      const pending = h.service.handle({ op: "getState" }, TAB).then(() => (done = true));
      await vi.advanceTimersByTimeAsync(100);
      // Run one after the other these would take 300 ms.
      expect(done).toBe(true);
      await pending;
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("savePending", () => {
  it("answers for the tab's own origin only", async () => {
    await h.save.storeGenerated({ tabId: TAB.id, origin: TOP, topOrigin: TOP, username: null, password: "Gen-2" });
    expect(await h.service.handle({ op: "savePending", action: "save" }, { id: TAB.id, url: "https://evil.test/" })).toEqual({ outcome: "dismissed" });
    expect(h.requests.some((r) => r.op === "proposeSave")).toBe(false);
  });

  it("sends the pending login, not what the popup says, to the desktop", async () => {
    await h.save.storeGenerated({ tabId: TAB.id, origin: TOP, topOrigin: TOP, username: "ada", password: "Gen-2" });
    const result = await h.service.handle({ op: "savePending", action: "save" }, TAB);
    expect(result).toEqual({ outcome: "saved" });
    expect(h.requests.find((r) => r.op === "proposeSave")?.body).toEqual({
      origin: TOP,
      topOrigin: TOP,
      username: "ada",
      password: "Gen-2",
    });
  });

  it("refuses to turn capture on without the site access", async () => {
    const spy = vi.spyOn(h.save, "setOffers").mockRejectedValue(new Error("permissionDenied"));
    await expect(h.service.handle({ op: "saveOffersChanged", enabled: true }, TAB)).rejects.toThrow("permissionDenied");
    expect(spy).toHaveBeenCalledWith(true);
  });
});

describe("the never-saved list (M4-R10)", () => {
  it("lists the exact origins and removes one, leaving the others", async () => {
    expect(await h.service.handle({ op: "neverSaveList" }, TAB)).toEqual({ origins: [] });
    await h.kv.set(neverSaveKey(TOP), true);
    await h.kv.set(neverSaveKey("https://other.test:8443"), true);
    await h.kv.set("pairing", { not: "a site" });
    expect(await h.service.handle({ op: "neverSaveList" }, TAB)).toEqual({ origins: [TOP, "https://other.test:8443"] });
    expect(await h.service.handle({ op: "neverSaveRemove", origin: TOP }, TAB)).toEqual({ origins: ["https://other.test:8443"] });
    expect(await h.save.isNeverSave(TOP)).toBe(false);
    expect(await h.kv.get("pairing")).toEqual({ not: "a site" });
  });

  it("removes only the neverSave entry of that exact origin", async () => {
    await h.kv.set(neverSaveKey(TOP), true);
    expect(await h.service.handle({ op: "neverSaveRemove", origin: "https://bank.test." }, TAB)).toEqual({ origins: [TOP] });
    expect(await h.service.handle({ op: "neverSaveRemove", origin: "pairing" }, TAB)).toEqual({ origins: [TOP] });
  });
});
