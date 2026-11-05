import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ContentMessage, FillReply, FrameProbe } from "../shared/messages.ts";
import type { LoginMatch, RequestMap, ResultMap } from "../shared/protocol.ts";
import { runFill, type FillDeps, type FillOutcome, type FillRequest } from "./fill.ts";
import { lastErrorKey, lastFilledKey, memorySession } from "./session.ts";
import type { FrameTarget } from "./tabs.ts";

const TOP = "https://bank.test";
const TAB = 4;

const frame = (frameId: number, over: Partial<FrameProbe> = {}): FrameProbe => ({
  frameId,
  documentId: `doc-${String(frameId)}`,
  origin: TOP,
  ancestorsSameOrigin: true,
  frameVisible: true,
  depth: frameId === 0 ? 0 : 1,
  activeField: false,
  counts: { username: 1, password: 1, newPassword: 0, otp: 0 },
  passwordRules: null,
  visible: true,
  ...over,
});

const match = (over: Partial<LoginMatch> = {}): LoginMatch => ({
  vaultId: "v1",
  itemId: "i1",
  title: "Bank",
  username: "ada",
  host: "bank.test",
  hasOtp: false,
  favorite: false,
  matchedBy: "exactOrigin",
  blocked: null,
  ...over,
});

interface Harness {
  deps: FillDeps;
  requests: { op: string; body: unknown }[];
  sent: { frame: FrameTarget; message: ContentMessage }[];
  badge: { set: ReturnType<typeof vi.fn>; clear: ReturnType<typeof vi.fn>; openPopup: ReturnType<typeof vi.fn> };
  now: { value: number };
  setProbes: (probes: FrameProbe[]) => void;
  setMatches: (items: LoginMatch[]) => void;
  setReply: (reply: FillReply) => void;
}

function harness(): Harness {
  let probes: FrameProbe[] = [frame(0)];
  let items: LoginMatch[] = [match()];
  let reply: FillReply = {
    ok: true,
    filled: { username: 1, password: 1, otp: 0, newPassword: 0 },
    usernameValue: null,
  };
  const requests: Harness["requests"] = [];
  const sent: Harness["sent"] = [];
  const now = { value: 1_000_000 };
  const badge = { set: vi.fn(), clear: vi.fn(), openPopup: vi.fn() };
  const deps: FillDeps = {
    native: {
      request: <O extends "listForOrigin" | "fill" | "generate">(op: O, body: RequestMap[O]): Promise<ResultMap[O]> => {
        requests.push({ op, body });
        const results: Record<string, () => unknown> = {
          listForOrigin: () => ({ items, warnings: [] }),
          fill: () => ({
            username: "ada",
            password: "pw",
            otp: (body as RequestMap["fill"]).want.otp ? "123456" : null,
          }),
          generate: () => ({ value: "generated", entropyBits: 90, strength: "strong", rulesSource: null }),
        };
        return Promise.resolve(results[op]?.() as ResultMap[O]);
      },
    },
    tabs: {
      inject: () => Promise.resolve(),
      probe: () => Promise.resolve(probes),
      send: (_tab, target, message) => {
        sent.push({ frame: target, message: { ...message } });
        return Promise.resolve(reply);
      },
    },
    session: memorySession(),
    badge: { ...badge, set: (...a: [number, string]) => Promise.resolve(badge.set(...a)), clear: (t: number) => Promise.resolve(badge.clear(t)), openPopup: () => Promise.resolve(badge.openPopup()) },
    now: () => now.value,
  };
  return {
    deps,
    requests,
    sent,
    badge,
    now,
    setProbes: (p) => {
      probes = p;
    },
    setMatches: (m) => {
      items = m;
    },
    setReply: (r) => {
      reply = r;
    },
  };
}

const request = (over: Partial<FillRequest> = {}): FillRequest => ({
  gesture: "command",
  tabId: TAB,
  tabUrl: `${TOP}/login`,
  menuFrameId: null,
  item: null,
  otpOnly: false,
  confirmFrame: false,
  ...over,
});

const failure = (outcome: FillOutcome): string => (outcome.ok ? "ok" : outcome.error);
const filledOps = (h: Harness): number => h.requests.filter((r) => r.op === "fill").length;

let h: Harness;
beforeEach(() => {
  h = harness();
});

describe("a shortcut fill", () => {
  it("lists matches for the frame's origin, asks for the first usable one and delivers it once", async () => {
    h.setMatches([match({ itemId: "blocked", blocked: "insecure" }), match({ itemId: "good" })]);
    const outcome = await runFill(h.deps, request());
    expect(outcome).toEqual({ ok: true, summary: { username: 1, password: 1, otp: 0, newPassword: 0 } });
    expect(h.requests).toEqual([
      { op: "listForOrigin", body: { origin: TOP, topOrigin: TOP } },
      {
        op: "fill",
        body: { vaultId: "v1", itemId: "good", origin: TOP, topOrigin: TOP, frame: "top", want: { login: true, otp: false } },
      },
    ]);
    expect(h.sent).toHaveLength(1);
    expect(h.sent[0]?.frame).toEqual({ frameId: 0, documentId: "doc-0" });
    expect(h.sent[0]?.message).toMatchObject({ type: "fill", expectedOrigin: TOP, username: "ada", password: "pw", otp: null });
    expect(h.badge.clear).toHaveBeenCalledWith(TAB);
  });

  it("asks for the code too when the frame has a code field and the item has a code", async () => {
    h.setProbes([frame(0, { counts: { username: 1, password: 1, newPassword: 0, otp: 1 } })]);
    h.setMatches([match({ hasOtp: true })]);
    await runFill(h.deps, request());
    expect(h.requests[1]?.body).toMatchObject({ want: { login: true, otp: true } });
    expect(h.sent[0]?.message).toMatchObject({ otp: "123456" });
  });

  it("fails with noMatch, a badge and the popup when only blocked matches exist", async () => {
    h.setMatches([match({ blocked: "insecure" })]);
    const outcome = await runFill(h.deps, request());
    expect(failure(outcome)).toBe("noMatch");
    expect(filledOps(h)).toBe(0);
    expect(h.badge.set).toHaveBeenCalledWith(TAB, "!");
    expect(h.badge.openPopup).toHaveBeenCalledOnce();
    expect(await h.deps.session.get(lastErrorKey(TAB))).toBe("noMatch");
  });

  it("clears the last error and the badge after a success", async () => {
    await h.deps.session.set(lastErrorKey(TAB), "noMatch");
    await runFill(h.deps, request());
    expect(await h.deps.session.get(lastErrorKey(TAB))).toBeUndefined();
  });

  it("uses the code of the item filled in the last five minutes for a code-only page", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 1 } })]);
    h.setMatches([match({ itemId: "a", hasOtp: true }), match({ itemId: "b", hasOtp: true })]);
    await h.deps.session.set(lastFilledKey(TAB), {
      origin: TOP,
      vaultId: "v1",
      itemId: "b",
      at: h.now.value - 60_000,
      title: null,
      username: null,
      hasOtp: true,
      otpFilled: false,
    });
    await runFill(h.deps, request());
    expect(h.requests[1]?.body).toMatchObject({ itemId: "b", want: { login: false, otp: true } });
    h.requests.length = 0;
    h.now.value += 6 * 60_000;
    await runFill(h.deps, request());
    expect(h.requests[1]?.body).toMatchObject({ itemId: "a" });
  });
});

describe("frames", () => {
  it("never fills a cross-origin frame, and never asks the desktop", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 } }), frame(2, { origin: "https://evil.test" })]);
    const outcome = await runFill(h.deps, request());
    expect(failure(outcome)).toBe("crossOriginFrame");
    expect(h.requests).toEqual([]);
    expect(h.sent).toEqual([]);
  });

  it("never fills the frame under the context menu when it is cross-origin", async () => {
    h.setProbes([frame(0), frame(2, { origin: "https://evil.test" })]);
    const outcome = await runFill(h.deps, request({ gesture: "menu", menuFrameId: 2 }));
    expect(failure(outcome)).toBe("crossOriginFrame");
    expect(h.requests).toEqual([]);
  });

  it("never fills an A-B-A frame, even when the user confirms", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 } }), frame(3, { ancestorsSameOrigin: false })]);
    const outcome = await runFill(h.deps, request({ gesture: "popup", confirmFrame: true, item: { vaultId: "v1", itemId: "i1", title: null, username: null, hasOtp: null } }));
    expect(failure(outcome)).toBe("crossOriginFrame");
    expect(filledOps(h)).toBe(0);
  });

  it("asks for a confirmation before a same-origin frame, then fills it as one", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 } }), frame(5)]);
    expect(failure(await runFill(h.deps, request()))).toBe("frameNeedsConfirmation");
    expect(h.requests).toEqual([]);
    const item = { vaultId: "v1", itemId: "i1", title: null, username: null, hasOtp: null };
    const outcome = await runFill(h.deps, request({ gesture: "popup", confirmFrame: true, item }));
    expect(outcome.ok).toBe(true);
    expect(h.requests[0]).toMatchObject({ op: "fill", body: { frame: "sameOriginFrame", origin: TOP, topOrigin: TOP } });
    expect(h.sent[0]?.frame).toEqual({ frameId: 5, documentId: "doc-5" });
  });

  it("fills the top frame and not an iframe that also has fields", async () => {
    h.setProbes([frame(0), frame(2, { origin: "https://evil.test" })]);
    const outcome = await runFill(h.deps, request());
    expect(outcome.ok).toBe(true);
    expect(h.sent.map((s) => s.frame.frameId)).toEqual([0]);
  });

  it("refuses a sandboxed frame with an opaque origin", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 } }), frame(2, { origin: "null" })]);
    expect(failure(await runFill(h.deps, request()))).toBe("frameOpaque");
    expect(h.requests).toEqual([]);
  });

  it("refuses a hidden tab", async () => {
    h.setProbes([frame(0, { visible: false })]);
    expect(failure(await runFill(h.deps, request()))).toBe("pageHidden");
  });
});

describe("pages that cannot be filled", () => {
  it.each([undefined, "about:blank", "chrome://extensions", "file:///a.html", "data:text/html,x"])("%s is an unsupported page", async (tabUrl) => {
    expect(failure(await runFill(h.deps, request({ tabUrl })))).toBe("unsupportedPage");
    expect(h.requests).toEqual([]);
  });

  it("reports noFields when no frame has any", async () => {
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 } })]);
    expect(failure(await runFill(h.deps, request()))).toBe("noFields");
    expect(h.requests).toEqual([]);
  });

  it("does not ask the desktop for a login on a sign-up page", async () => {
    h.setProbes([frame(0, { counts: { username: 1, password: 0, newPassword: 2, otp: 0 } })]);
    expect(failure(await runFill(h.deps, request({ gesture: "popup", item: { vaultId: "v", itemId: "i", title: null, username: null, hasOtp: false } })))).toBe("noFields");
  });
});

describe("delivery", () => {
  it("passes on the content script's refusal and remembers nothing", async () => {
    h.setReply({ ok: false, error: "originChanged" });
    const outcome = await runFill(h.deps, request());
    expect(failure(outcome)).toBe("originChanged");
    expect(await h.deps.session.get(lastFilledKey(TAB))).toBeUndefined();
  });

  it("remembers the item and whether its code was filled, without any secret", async () => {
    h.setMatches([match({ hasOtp: true })]);
    await runFill(h.deps, request());
    const stored = (await h.deps.session.get(lastFilledKey(TAB))) as Record<string, unknown>;
    expect(stored).toMatchObject({ origin: TOP, itemId: "i1", hasOtp: true, otpFilled: false });
    expect(JSON.stringify(stored)).not.toContain("pw");
  });

  it("does not show a badge for a failed popup fill, but still records the error", async () => {
    h.setProbes([]);
    const outcome = await runFill(h.deps, request({ gesture: "popup" }));
    expect(failure(outcome)).toBe("noFields");
    expect(h.badge.set).not.toHaveBeenCalled();
    expect(await h.deps.session.get(lastErrorKey(TAB))).toBe("noFields");
  });
});
