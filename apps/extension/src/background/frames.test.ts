import { describe, expect, it } from "vitest";
import type { FrameProbe } from "../shared/messages.ts";
import { chooseTarget, classifyFrame, decideFrame, wantFor, type FrameFacts } from "./frames.ts";

const TOP = "https://bank.test";

const facts = (over: Partial<FrameFacts> = {}): FrameFacts => ({
  frameId: 0,
  origin: TOP,
  ancestorsSameOrigin: true,
  frameVisible: true,
  visible: true,
  ...over,
});

describe("decideFrame", () => {
  const cases: [string, Partial<FrameFacts>, boolean, ReturnType<typeof decideFrame>][] = [
    ["top frame", {}, false, { ok: true, frame: "top" }],
    ["top frame, confirmation is irrelevant", {}, true, { ok: true, frame: "top" }],
    ["top frame whose origin is no longer the tab's", { origin: "https://evil.test" }, false, { ok: false, error: "originChanged" }],
    ["opaque origin", { origin: "null", frameId: 3 }, true, { ok: false, error: "frameOpaque" }],
    ["opaque top frame", { origin: "null" }, false, { ok: false, error: "frameOpaque" }],
    ["not an http origin", { origin: "file://", frameId: 2 }, true, { ok: false, error: "frameOpaque" }],
    ["hidden tab", { visible: false }, false, { ok: false, error: "pageHidden" }],
    ["same-origin frame without confirmation", { frameId: 4 }, false, { ok: false, error: "frameNeedsConfirmation" }],
    ["same-origin frame with confirmation", { frameId: 4 }, true, { ok: true, frame: "sameOriginFrame" }],
    ["cross-origin frame", { frameId: 4, origin: "https://evil.test" }, true, { ok: false, error: "crossOriginFrame" }],
    ["same origin but an ancestor differs (A-B-A)", { frameId: 4, ancestorsSameOrigin: false }, true, { ok: false, error: "crossOriginFrame" }],
    ["same-origin frame the user cannot see", { frameId: 4, frameVisible: false }, true, { ok: false, error: "crossOriginFrame" }],
    ["same host, other scheme", { frameId: 4, origin: "http://bank.test" }, true, { ok: false, error: "crossOriginFrame" }],
    ["same host, other port", { frameId: 4, origin: "https://bank.test:8443" }, true, { ok: false, error: "crossOriginFrame" }],
  ];

  it.each(cases)("%s", (_name, over, confirm, expected) => {
    expect(decideFrame(facts(over), TOP, confirm)).toEqual(expected);
  });
});

const counts = (over: Partial<FrameProbe["counts"]> = {}): FrameProbe["counts"] => ({
  username: 0,
  password: 0,
  newPassword: 0,
  otp: 0,
  ...over,
});

const probe = (frameId: number, over: Partial<FrameProbe> = {}): FrameProbe => ({
  ...facts({ frameId }),
  depth: frameId === 0 ? 0 : 1,
  activeField: false,
  counts: counts(),
  passwordRules: null,
  documentId: null,
  ...over,
});

describe("chooseTarget", () => {
  const login = counts({ username: 1, password: 1 });

  it("uses the frame under the context menu, even without fields", () => {
    const probes = [probe(0, { counts: login }), probe(5)];
    expect(chooseTarget(probes, 5)?.frameId).toBe(5);
    expect(chooseTarget(probes, 9)).toBeNull();
  });

  it("uses the deepest frame that has focus", () => {
    const probes = [
      probe(0, { counts: login, activeField: true, depth: 0 }),
      probe(2, { counts: login, activeField: true, depth: 2 }),
      probe(3, { counts: login, depth: 3 }),
    ];
    expect(chooseTarget(probes, null)?.frameId).toBe(2);
  });

  it("falls back to the top frame when it has fields", () => {
    const probes = [probe(0, { counts: login }), probe(1, { counts: login })];
    expect(chooseTarget(probes, null)?.frameId).toBe(0);
  });

  it("falls back to the only frame with fields", () => {
    expect(chooseTarget([probe(0), probe(1, { counts: login })], null)?.frameId).toBe(1);
  });

  it("does not guess between several frames with fields, or when there are none", () => {
    expect(chooseTarget([probe(0), probe(1, { counts: login }), probe(2, { counts: login })], null)).toBeNull();
    expect(chooseTarget([probe(0), probe(1)], null)).toBeNull();
    expect(chooseTarget([], null)).toBeNull();
  });
});

describe("classifyFrame", () => {
  it("names the frame for the popup", () => {
    expect(classifyFrame(null, TOP)).toBe("none");
    expect(classifyFrame(probe(0), TOP)).toBe("top");
    expect(classifyFrame(probe(2), TOP)).toBe("sameOriginFrame");
    expect(classifyFrame(probe(2, { origin: "https://evil.test" }), TOP)).toBe("crossOriginFrame");
    expect(classifyFrame(probe(2, { ancestorsSameOrigin: false }), TOP)).toBe("crossOriginFrame");
    expect(classifyFrame(probe(2, { origin: "null" }), "null")).toBe("crossOriginFrame");
  });
});

describe("wantFor", () => {
  it("asks for what the frame has fields for", () => {
    expect(wantFor(counts({ username: 1, password: 1 }), true, false)).toEqual({ login: true, otp: false });
    expect(wantFor(counts({ username: 1, password: 1, otp: 1 }), true, false)).toEqual({ login: true, otp: true });
    expect(wantFor(counts({ password: 1, otp: 1 }), false, false)).toEqual({ login: true, otp: false });
    expect(wantFor(counts({ otp: 1 }), true, false)).toEqual({ login: false, otp: true });
  });

  it("asks only for the code on request, and lets the desktop decide when hasOtp is unknown", () => {
    expect(wantFor(counts({ password: 1, otp: 1 }), true, true)).toEqual({ login: false, otp: true });
    expect(wantFor(counts({ password: 1, otp: 1 }), null, false)).toEqual({ login: true, otp: true });
  });

  it("asks for nothing when there is nothing to fill", () => {
    expect(wantFor(counts({ newPassword: 2 }), true, false)).toEqual({ login: false, otp: false });
    expect(wantFor(counts({ username: 1, newPassword: 2 }), true, false)).toEqual({ login: false, otp: false });
    expect(wantFor(counts({ username: 1 }), true, false)).toEqual({ login: true, otp: false });
  });
});
