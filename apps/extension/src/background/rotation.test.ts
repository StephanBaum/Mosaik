import { describe, expect, it } from "vitest";
import { ExtensionError } from "../shared/errors.ts";
import type { ContentMessage, FillReply, FrameProbe } from "../shared/messages.ts";
import type { RequestMap, ResultMap, RotationMatch } from "../shared/protocol.ts";
import { sameSite } from "../shared/origin.ts";
import { RotationService, ROTATION_TAB_TTL_MS, changePageFor, chooseChangeTarget, type RotationDeps } from "./rotation.ts";
import { rotationTabKey, memorySession } from "./session.ts";
import type { FrameTarget } from "./tabs.ts";

const BANK = "https://bank.test";
const TAB = { id: 4, url: `${BANK}/settings` };
const IDS = { vaultId: "v1", itemId: "i1", attemptId: "a1.dev.0" };

const frame = (frameId: number, over: Partial<FrameProbe> = {}): FrameProbe => ({
  frameId,
  documentId: `doc-${String(frameId)}`,
  origin: BANK,
  ancestorsSameOrigin: true,
  frameVisible: true,
  depth: frameId === 0 ? 0 : 1,
  activeField: false,
  counts: { username: 0, password: 1, newPassword: 1, otp: 0 },
  passwordRules: null,
  visible: true,
  changeShape: "currentAndNew",
  ...over,
});

const attempt = (over: Partial<RotationMatch> = {}): RotationMatch => ({
  ...IDS,
  title: "Bank",
  username: "ada",
  host: "bank.test",
  stage: "prepared",
  startedAt: 1,
  changeUrl: `${BANK}/account/password`,
  ...over,
});

interface Harness {
  service: RotationService;
  deps: RotationDeps;
  requests: { op: string; body: unknown }[];
  sent: { frame: FrameTarget; message: ContentMessage }[];
  navigated: { tabId: number; url: string }[];
  now: { value: number };
  setProbes: (probes: FrameProbe[]) => void;
  setAttempts: (attempts: RotationMatch[]) => void;
  /** What the desktop answers to "related?"; `null` (the default) is the same-site guess. */
  setRelated: (related: boolean | null) => void;
  setReply: (reply: FillReply) => void;
  fail: (code: ExtensionError | null) => void;
}

function harness(): Harness {
  let probes: FrameProbe[] = [frame(0)];
  let attempts: RotationMatch[] = [attempt()];
  let failure: ExtensionError | null = null;
  let related: boolean | null = null;
  let reply: FillReply = { ok: true, filled: { username: 0, password: 1, otp: 0, newPassword: 2 }, usernameValue: null };
  const requests: Harness["requests"] = [];
  const sent: Harness["sent"] = [];
  const navigated: Harness["navigated"] = [];
  const now = { value: 1_000_000 };
  const deps: RotationDeps = {
    native: {
      request: <O extends keyof RequestMap>(op: O, body: RequestMap[O]): Promise<ResultMap[O]> => {
        requests.push({ op, body });
        if (failure !== null) {
          return Promise.reject(failure);
        }
        const results: Record<string, () => unknown> = {
          rotationForOrigin: () => {
            const ask = body as RequestMap["rotationForOrigin"];
            return {
              attempts,
              related: ask.relatedTo !== null && (related ?? sameSite(ask.origin, ask.relatedTo)),
            };
          },
          rotationBegin: () => ({ outcome: "started", attemptId: "a9.dev.0" }),
          fillChange: () => ({ current: "old-pw", new: "new-pw" }),
          fillPending: () => ({ username: "ada", password: "new-pw" }),
          rotationResult: () => ({ outcome: "confirmed" }),
          rotationAddOrigin: () => ({ outcome: "added" }),
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
      navigate: (tabId, url) => {
        navigated.push({ tabId, url });
        return Promise.resolve();
      },
    },
    session: memorySession(),
    now: () => now.value,
  };
  return {
    service: new RotationService(deps),
    deps,
    requests,
    sent,
    navigated,
    now,
    setProbes: (p) => {
      probes = p;
    },
    setAttempts: (a) => {
      attempts = a;
    },
    setRelated: (r) => {
      related = r;
    },
    setReply: (r) => {
      reply = r;
    },
    fail: (code) => {
      failure = code;
    },
  };
}

const ops = (h: Harness): string[] => h.requests.map((r) => r.op);

describe("the form classification decides before the desktop is asked", () => {
  it("answers changeFormNotFound without a request when no frame has a change form", async () => {
    const h = harness();
    h.setProbes([frame(0, { changeShape: "none" })]);
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "changeFormNotFound" });
    expect(h.requests).toEqual([]);
    expect(h.sent).toEqual([]);
  });

  it("answers changeFormAmbiguous without a request when the form cannot be told apart", async () => {
    const h = harness();
    h.setProbes([frame(0, { changeShape: "ambiguous" })]);
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "changeFormAmbiguous" });
    expect(h.requests).toEqual([]);
    expect(h.sent).toEqual([]);
  });

  it("refuses a frame of another origin before the desktop is asked", async () => {
    const h = harness();
    h.setProbes([
      frame(0, { changeShape: "none" }),
      frame(1, { origin: "https://evil.test", ancestorsSameOrigin: false }),
    ]);
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "crossOriginFrame" });
    expect(h.requests).toEqual([]);
  });

  it("refuses a same-origin frame that was not confirmed", async () => {
    const h = harness();
    h.setProbes([frame(0, { changeShape: "none" }), frame(1)]);
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "frameNeedsConfirmation" });
    expect(h.requests).toEqual([]);
  });
});

describe("a change fill", () => {
  it("asks for the shape the probe found and sends both values in one message to the probed frame and document", async () => {
    const h = harness();
    h.setProbes([frame(0, { changeShape: "newOnly" })]);
    const summary = await h.service.fillChange({ ...IDS, confirmFrame: null }, TAB);
    expect(summary.newPassword).toBe(2);
    expect(h.requests).toEqual([
      {
        op: "fillChange",
        body: { ...IDS, origin: BANK, topOrigin: BANK, frame: "top", shape: "newOnly" },
      },
    ]);
    expect(h.sent).toHaveLength(1);
    expect(h.sent[0]?.frame).toEqual({ frameId: 0, documentId: "doc-0" });
    expect(h.sent[0]?.message).toMatchObject({ type: "fillChange", expectedOrigin: BANK, shape: "newOnly", current: "old-pw", new: "new-pw" });
  });

  it("goes to a confirmed same-origin frame and names its document", async () => {
    const h = harness();
    h.setProbes([frame(0, { changeShape: "none" }), frame(1)]);
    await h.service.fillChange({ ...IDS, confirmFrame: { frameId: 1, documentId: "doc-1" } }, TAB);
    expect(h.sent[0]?.frame).toEqual({ frameId: 1, documentId: "doc-1" });
    expect(h.requests[0]?.body).toMatchObject({ frame: "sameOriginFrame" });
    // A confirmation for another document of that frame does not count.
    const other = harness();
    other.setProbes([frame(0, { changeShape: "none" }), frame(1)]);
    await expect(
      other.service.fillChange({ ...IDS, confirmFrame: { frameId: 1, documentId: "doc-old" } }, TAB),
    ).rejects.toMatchObject({ code: "frameNeedsConfirmation" });
    expect(other.requests).toEqual([]);
  });

  it("reports what the content script answered and drops its references to the values", async () => {
    const h = harness();
    h.setReply({ ok: false, error: "changeFormAmbiguous" });
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "changeFormAmbiguous" });
    expect(h.sent).toHaveLength(1);
  });

  it("passes the desktop's refusals on", async () => {
    const h = harness();
    h.fail(new ExtensionError("originMismatch"));
    await expect(h.service.fillChange({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "originMismatch" });
    expect(h.sent).toEqual([]);
  });

  it("picks the frame with the form: the top one, the only one, the one with focus, else none", () => {
    expect(chooseChangeTarget([frame(0), frame(1)]).frameId).toBe(0);
    expect(chooseChangeTarget([frame(0, { changeShape: "none" }), frame(2)]).frameId).toBe(2);
    expect(chooseChangeTarget([frame(0, { changeShape: "none" }), frame(1), frame(2, { activeField: true })]).frameId).toBe(2);
    expect(() => chooseChangeTarget([frame(0, { changeShape: "none" }), frame(1), frame(2)])).toThrow(
      expect.objectContaining({ code: "changeFormAmbiguous" }),
    );
    expect(() => chooseChangeTarget([frame(0, { changeShape: "none" })])).toThrow(
      expect.objectContaining({ code: "changeFormNotFound" }),
    );
  });
});

describe("filling the new password to sign in", () => {
  it("sends the user name and the new password as a login fill", async () => {
    const h = harness();
    h.setProbes([frame(0, { counts: { username: 1, password: 1, newPassword: 0, otp: 0 }, changeShape: "none" })]);
    await h.service.fillPending({ ...IDS, confirmFrame: null }, TAB);
    expect(ops(h)).toEqual(["fillPending"]);
    expect(h.sent[0]?.message).toMatchObject({ type: "fill", username: "ada", password: "new-pw", otp: null });
  });

  it("asks nothing on a page without a sign-in form", async () => {
    const h = harness();
    h.setProbes([frame(0, { counts: { username: 0, password: 0, newPassword: 0, otp: 0 }, changeShape: "none" })]);
    await expect(h.service.fillPending({ ...IDS, confirmFrame: null }, TAB)).rejects.toMatchObject({ code: "noFields" });
    expect(h.requests).toEqual([]);
  });
});

describe("opening the change page", () => {
  it("goes to the desktop's address on the same site and remembers the tab", async () => {
    const h = harness();
    await h.service.openPage(IDS, TAB);
    expect(h.navigated).toEqual([{ tabId: 4, url: `${BANK}/account/password` }]);
    expect(await h.deps.session.get(rotationTabKey(4))).toMatchObject({ ...IDS, fromOrigin: BANK, title: "Bank" });
  });

  it("falls back to the well-known address when the desktop's address is on another site or not https", () => {
    expect(changePageFor(BANK, "https://evil.test/password")).toBe(`${BANK}/.well-known/change-password`);
    expect(changePageFor(BANK, "http://bank.test/password")).toBe(`${BANK}/.well-known/change-password`);
    expect(changePageFor(BANK, "https://bank.test.evil.test/password")).toBe(`${BANK}/.well-known/change-password`);
    expect(changePageFor(BANK, "javascript:alert(1)")).toBe(`${BANK}/.well-known/change-password`);
    expect(changePageFor(BANK, null)).toBe(`${BANK}/.well-known/change-password`);
    expect(changePageFor(BANK, "https://accounts.bank.test/password")).toBe("https://accounts.bank.test/password");
  });

  it("refuses an attempt the desktop does not list for this origin, and an http page", async () => {
    const h = harness();
    h.setAttempts([]);
    await expect(h.service.openPage(IDS, TAB)).rejects.toMatchObject({ code: "rotationNotFound" });
    expect(h.navigated).toEqual([]);
    await expect(h.service.openPage(IDS, { id: 4, url: "http://bank.test/" })).rejects.toMatchObject({ code: "insecureOrigin" });
  });
});

describe("the tab record and the related-address offer", () => {
  const moved = { id: 4, url: "https://accounts.bank.test/password" };

  async function sent(h: Harness): Promise<void> {
    h.setAttempts([attempt()]);
    await h.service.openPage(IDS, TAB);
    h.setAttempts([]);
  }

  it("offers a related address only in a tab the extension sent", async () => {
    const h = harness();
    expect((await h.service.forTab(moved)).rotationTabOffer).toBeNull();
    await sent(h);
    expect((await h.service.forTab(moved)).rotationTabOffer).toEqual({ host: "accounts.bank.test", unicodeHost: null, title: "Bank" });
    expect((await h.service.forTab({ id: 5, url: moved.url })).rotationTabOffer).toBeNull();
  });

  it("offers nothing on an unrelated address, and forgets the tab once it left the site", async () => {
    const h = harness();
    await sent(h);
    expect((await h.service.forTab({ id: 4, url: "https://evil.test/" })).rotationTabOffer).toBeNull();
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
  });

  it("asks the desktop whether the page is related, naming the address the tab was sent from", async () => {
    const h = harness();
    await sent(h);
    h.requests.length = 0;
    await h.service.forTab(moved);
    expect(h.requests).toEqual([
      { op: "rotationForOrigin", body: { origin: "https://accounts.bank.test", topOrigin: "https://accounts.bank.test", relatedTo: BANK } },
    ]);
    // A tab without a record, and the login's own address, ask no related question.
    h.requests.length = 0;
    await h.service.forTab({ id: 9, url: moved.url });
    await h.service.forTab(TAB);
    expect(h.requests.map((r) => (r.body as { relatedTo: string | null }).relatedTo)).toEqual([null, null]);
  });

  it("offers nothing when the desktop says the page is not related (a private suffix such as github.io)", async () => {
    const h = harness();
    await sent(h);
    h.setRelated(false);
    expect((await h.service.forTab({ id: 4, url: "https://mallory.bank.test/" })).rotationTabOffer).toBeNull();
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
  });

  it("offers nothing for another port of the same host, whatever the desktop says", async () => {
    const h = harness();
    await sent(h);
    h.setRelated(true);
    const other = { id: 4, url: "https://bank.test:4999/password" };
    expect((await h.service.forTab({ id: 4, url: "https://accounts.bank.test:4999/password" })).rotationTabOffer).toBeNull();
    await expect(h.service.addOrigin(other)).rejects.toMatchObject({ code: "originNotRelated" });
    expect(ops(h).filter((op) => op === "rotationAddOrigin")).toEqual([]);
  });

  it("drops the record when the tab navigates off the site, so coming back gets no offer", async () => {
    const h = harness();
    await sent(h);
    await h.service.onNavigated(4, "https://accounts.bank.test/other");
    expect(await h.deps.session.get(rotationTabKey(4))).toBeDefined();
    await h.service.onNavigated(4, "https://evil.test/");
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
    expect((await h.service.forTab(moved)).rotationTabOffer).toBeNull();
    // Not https is off the site too.
    await sent(h);
    await h.service.onNavigated(4, "http://bank.test/");
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
  });

  it("offers nothing on the login's own address or on http", async () => {
    const h = harness();
    await sent(h);
    expect((await h.service.forTab(TAB)).rotationTabOffer).toBeNull();
    expect((await h.service.forTab({ id: 4, url: "http://accounts.bank.test/" })).rotationTabOffer).toBeNull();
  });

  it("forgets the record after 15 minutes and when the tab closes", async () => {
    const h = harness();
    await sent(h);
    h.now.value += ROTATION_TAB_TTL_MS;
    expect((await h.service.forTab(moved)).rotationTabOffer).toBeNull();
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
    await sent(h);
    await h.service.forgetTab(4);
    expect(await h.deps.session.get(rotationTabKey(4))).toBeUndefined();
  });

  it("asks the desktop to add the address only from a recorded tab on the same site", async () => {
    const h = harness();
    await expect(h.service.addOrigin(moved)).rejects.toMatchObject({ code: "originNotRelated" });
    expect(ops(h)).toEqual([]);
    await sent(h);
    h.requests.length = 0;
    await expect(h.service.addOrigin({ id: 4, url: "https://evil.test/" })).rejects.toMatchObject({ code: "originNotRelated" });
    await expect(h.service.addOrigin({ id: 4, url: "http://accounts.bank.test/" })).rejects.toMatchObject({ code: "insecureOrigin" });
    await expect(h.service.addOrigin(TAB)).rejects.toMatchObject({ code: "originNotRelated" });
    expect(h.requests).toEqual([]);
    expect(await h.service.addOrigin(moved)).toEqual({ outcome: "added" });
    expect(h.requests).toEqual([
      {
        op: "rotationAddOrigin",
        body: { ...IDS, fromOrigin: BANK, origin: "https://accounts.bank.test", topOrigin: "https://accounts.bank.test" },
      },
    ]);
  });
});

describe("starting and closing a change", () => {
  it("passes the page's password rules from the top frame and asks the desktop with the tab's origin", async () => {
    const h = harness();
    h.setProbes([frame(0, { passwordRules: "minlength: 12;" }), frame(1, { passwordRules: "minlength: 4;" })]);
    expect(await h.service.begin({ vaultId: "v1", itemId: "i1" }, TAB)).toEqual({ outcome: "started", attemptId: "a9.dev.0" });
    expect(h.requests).toEqual([
      { op: "rotationBegin", body: { vaultId: "v1", itemId: "i1", origin: BANK, topOrigin: BANK, rules: "minlength: 12;" } },
    ]);
  });

  it("never runs on http", async () => {
    const h = harness();
    await expect(h.service.begin({ vaultId: "v1", itemId: "i1" }, { id: 4, url: "http://bank.test/" })).rejects.toMatchObject({ code: "insecureOrigin" });
    await expect(
      h.service.rotationResult({ ...IDS, outcome: "succeeded" }, { id: 4, url: "http://bank.test/" }),
    ).rejects.toMatchObject({ code: "insecureOrigin" });
    expect(h.requests).toEqual([]);
  });

  it("reports the result with the tab's origin only", async () => {
    const h = harness();
    expect(await h.service.rotationResult({ ...IDS, outcome: "failed" }, TAB)).toEqual({ outcome: "confirmed" });
    expect(h.requests[0]).toEqual({
      op: "rotationResult",
      body: { ...IDS, origin: BANK, topOrigin: BANK, outcome: "failed" },
    });
  });
});

describe("the open changes of a tab", () => {
  it("lists them for an https page and hides the controls for an older desktop", async () => {
    const h = harness();
    expect((await h.service.forTab(TAB)).rotations).toEqual([attempt()]);
    h.fail(new ExtensionError("invalidRequest"));
    expect((await h.service.forTab(TAB)).rotations).toEqual([]);
    const plain = harness();
    expect((await plain.service.forTab({ id: 4, url: "http://bank.test/" })).rotations).toEqual([]);
    expect(plain.requests).toEqual([]);
  });
});
