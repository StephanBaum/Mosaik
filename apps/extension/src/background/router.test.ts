import { describe, expect, it, vi } from "vitest";
import { ExtensionError } from "../shared/errors.ts";
import type { PopupRequest } from "../shared/messages.ts";
import { Router, isPopupSender, type PortInfo, type RouterDeps, type SenderInfo } from "./router.ts";

const EXT = "abcdefghijklmnopabcdefghijklmnop";
const POPUP = `chrome-extension://${EXT}/popup.html`;
const PAGE_TAB = { id: 7 };

function makeDeps(over: Partial<RouterDeps> = {}): RouterDeps {
  return {
    extensionId: EXT,
    popupUrl: POPUP,
    testPopup: false,
    now: () => 0,
    handlePopup: vi.fn(() => Promise.resolve({ done: true })),
    ...over,
  };
}

function makePort(name: string, sender: SenderInfo | undefined) {
  const posted: unknown[] = [];
  let onMessage: ((m: unknown) => void) | undefined;
  let onDisconnect: (() => void) | undefined;
  const port: PortInfo = {
    name,
    sender,
    postMessage: (m) => posted.push(m),
    disconnect: vi.fn(),
    onMessage: {
      addListener: (l) => {
        onMessage = l;
      },
    },
    onDisconnect: {
      addListener: (l) => {
        onDisconnect = l;
      },
    },
  };
  return {
    port,
    posted,
    send: (m: unknown): void => onMessage?.(m),
    close: (): void => onDisconnect?.(),
  };
}

const popupSender: SenderInfo = { id: EXT, url: POPUP };
const flush = async (): Promise<void> => {
  await Promise.resolve();
  await Promise.resolve();
};

describe("isPopupSender", () => {
  const deps = makeDeps();
  it("accepts only our id, the exact popup url and no tab", () => {
    expect(isPopupSender(popupSender, deps)).toBe(true);
    expect(isPopupSender({ id: "other", url: POPUP }, deps)).toBe(false);
    expect(isPopupSender({ id: EXT, url: `${POPUP}?x=1` }, deps)).toBe(false);
    expect(isPopupSender({ id: EXT, url: "https://evil.test/popup.html" }, deps)).toBe(false);
    expect(isPopupSender({ id: EXT, url: POPUP, tab: PAGE_TAB }, deps)).toBe(false);
    expect(isPopupSender({ id: EXT }, deps)).toBe(false);
    expect(isPopupSender(undefined, deps)).toBe(false);
  });

  it("lets the e2e build pass a query string and a tab, and nothing else", () => {
    const test = makeDeps({ testPopup: true });
    expect(isPopupSender({ id: EXT, url: `${POPUP}?tab=1` }, test)).toBe(true);
    expect(isPopupSender({ id: EXT, url: POPUP, tab: PAGE_TAB }, test)).toBe(true);
    expect(isPopupSender({ id: EXT, url: `${POPUP}x` }, test)).toBe(false);
    expect(isPopupSender({ id: EXT, url: `chrome-extension://${EXT}/other.html` }, test)).toBe(false);
  });
});

describe("Router.onConnect", () => {
  it("serves the popup's port", async () => {
    const deps = makeDeps();
    const router = new Router(deps);
    const { port, posted, send } = makePort("popup", popupSender);
    router.onConnect(port);
    expect(port.disconnect).not.toHaveBeenCalled();
    send({ id: 3, request: { op: "getState" } });
    await flush();
    expect(deps.handlePopup).toHaveBeenCalledWith({ op: "getState" }, popupSender);
    expect(posted).toEqual([{ id: 3, ok: true, result: { done: true } }]);
  });

  it.each([
    ["a port with another name", "other", popupSender],
    ["a content script", "popup", { id: EXT, url: "https://evil.test/", tab: PAGE_TAB }],
    ["a web page (no sender id)", "popup", { url: "https://evil.test/" }],
    ["another extension", "popup", { id: "zzzz", url: POPUP }],
    ["the popup url with a query", "popup", { id: EXT, url: `${POPUP}?x` }],
    ["no sender at all", "popup", undefined],
  ])("disconnects %s without a word", async (_name, name, sender) => {
    const deps = makeDeps();
    const router = new Router(deps);
    const { port, posted, send } = makePort(name, sender);
    router.onConnect(port);
    expect(port.disconnect).toHaveBeenCalledOnce();
    send({ id: 1, request: { op: "getState" } });
    await flush();
    expect(deps.handlePopup).not.toHaveBeenCalled();
    expect(posted).toEqual([]);
  });

  it("answers a failing operation with its error code only", async () => {
    const handlePopup = vi
      .fn<RouterDeps["handlePopup"]>()
      .mockRejectedValueOnce(new ExtensionError("locked"))
      .mockRejectedValueOnce(new Error("secret detail"));
    const router = new Router(makeDeps({ handlePopup }));
    const { port, posted, send } = makePort("popup", popupSender);
    router.onConnect(port);
    send({ id: 9, request: { op: "getState" } });
    send({ id: 10, request: { op: "getState" } });
    await flush();
    expect(posted).toEqual([
      { id: 9, ok: false, error: "locked" },
      { id: 10, ok: false, error: "internal" },
    ]);
  });

  it.each([
    ["an unknown op", { id: 1, request: { op: "dumpVault" } }],
    ["an extra key", { id: 1, request: { op: "getState", extra: 1 } }],
    ["no id", { request: { op: "getState" } }],
    ["a string id", { id: "1", request: { op: "getState" } }],
    ["a fill without confirmFrame", { id: 1, request: { op: "fill", vaultId: "v", itemId: "i" } }],
    ["a save op (not in this build)", { id: 1, request: { op: "savePending", action: "save" } }],
    ["a bare string", "getState"],
  ])("drops %s without a reply", async (_name, message) => {
    const deps = makeDeps();
    const router = new Router(deps);
    const { port, posted, send } = makePort("popup", popupSender);
    router.onConnect(port);
    send(message);
    await flush();
    expect(deps.handlePopup).not.toHaveBeenCalled();
    expect(posted).toEqual([]);
  });

  it("passes every valid popup operation through", async () => {
    const deps = makeDeps();
    const router = new Router(deps);
    const { port, send } = makePort("popup", popupSender);
    router.onConnect(port);
    const requests: PopupRequest[] = [
      { op: "pair" },
      { op: "focusDesktop" },
      { op: "search", query: "git" },
      { op: "fill", vaultId: "v", itemId: "i", confirmFrame: false },
      { op: "fillOtp", vaultId: "v", itemId: "i", confirmFrame: true },
      { op: "copy", source: { kind: "generated" } },
      { op: "copy", source: { kind: "item", vaultId: "v", itemId: "i", field: "totp" } },
      { op: "generate", request: { kind: "forSite", host: "a.test", rules: null } },
      { op: "fillGenerated", confirmFrame: false },
      { op: "unpair" },
    ];
    requests.forEach((request, index) => {
      send({ id: index, request });
    });
    await flush();
    expect(vi.mocked(deps.handlePopup).mock.calls.map(([r]) => r)).toEqual(requests);
  });

  it("broadcasts events to open popups only", () => {
    const router = new Router(makeDeps());
    const open = makePort("popup", popupSender);
    const closed = makePort("popup", popupSender);
    router.onConnect(open.port);
    router.onConnect(closed.port);
    closed.close();
    router.broadcast({ event: "connection", connection: "locked" });
    expect(open.posted).toEqual([{ event: "connection", connection: "locked" }]);
    expect(closed.posted).toEqual([]);
  });
});

describe("Router.onMessage", () => {
  const content: SenderInfo = { id: EXT, url: "https://bank.test/", tab: PAGE_TAB };

  it("hands a captured message from our content script to the capture handler", () => {
    const onCaptured = vi.fn();
    const router = new Router(makeDeps({ onCaptured }));
    router.onMessage({ type: "captured" }, content);
    expect(onCaptured).toHaveBeenCalledOnce();
  });

  it("drops popup operations sent as runtime messages, from anyone", () => {
    const onCaptured = vi.fn();
    const deps = makeDeps({ onCaptured });
    const router = new Router(deps);
    for (const sender of [content, popupSender, { id: EXT }, { url: "https://evil.test/" }]) {
      router.onMessage({ op: "fill", vaultId: "v", itemId: "i", confirmFrame: true }, sender);
      router.onMessage({ id: 1, request: { op: "getState" } }, sender);
      router.onMessage({ type: "fill" }, sender);
    }
    expect(onCaptured).not.toHaveBeenCalled();
    expect(deps.handlePopup).not.toHaveBeenCalled();
  });

  it("drops captured messages that do not come from our content script", () => {
    const onCaptured = vi.fn();
    const router = new Router(makeDeps({ onCaptured }));
    router.onMessage({ type: "captured" }, { id: "other", url: "https://bank.test/", tab: PAGE_TAB });
    router.onMessage({ type: "captured" }, { id: EXT, url: POPUP });
    router.onMessage({ type: "captured" }, {});
    expect(onCaptured).not.toHaveBeenCalled();
  });

  it("allows five captures a minute per tab", () => {
    let now = 0;
    const onCaptured = vi.fn();
    const router = new Router(makeDeps({ onCaptured, now: () => now }));
    for (let i = 0; i < 8; i++) {
      now += 1_500;
      router.onMessage({ type: "captured" }, content);
    }
    expect(onCaptured).toHaveBeenCalledTimes(5);
    router.onMessage({ type: "captured" }, { ...content, tab: { id: 8 } });
    expect(onCaptured).toHaveBeenCalledTimes(6);
    now += 60_000;
    router.onMessage({ type: "captured" }, content);
    expect(onCaptured).toHaveBeenCalledTimes(7);
  });

  it("caps a flood from one tab at ten messages a second, whatever they are", () => {
    const onCaptured = vi.fn();
    const router = new Router(makeDeps({ onCaptured }));
    for (let i = 0; i < 10; i++) {
      router.onMessage({ type: "noise" }, content);
    }
    router.onMessage({ type: "captured" }, content);
    expect(onCaptured).not.toHaveBeenCalled();
  });
});
