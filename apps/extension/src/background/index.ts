/**
 * The background service worker (Chromium) or event page (Firefox). Every listener is
 * registered synchronously at the top level, so the browser can wake the worker for the event;
 * the work behind them starts lazily. It listens to no other extension and no web page.
 */
import { detectHint } from "../shared/ids.ts";
import type { PopupEvent } from "../shared/messages.ts";
import { chromeBadge } from "./badge.ts";
import { createIndexedDbStore } from "./db.ts";
import { e2eEnabled, e2ePortFactory, installE2E, testTabOf } from "./e2e.ts";
import { GeneratedHold, type FillDeps } from "./fill.ts";
import { createMenus, onCommand, onMenuClicked, type GestureDeps } from "./gestures.ts";
import { NativeClient } from "./native.ts";
import { PairingStore } from "./pairing.ts";
import { PopupService, type TabRef } from "./popup.ts";
import { Router, type SenderInfo } from "./router.ts";
import { chromeSession, lastErrorKey, lastFilledKey } from "./session.ts";
import { activeTab, chromeTabs } from "./tabs.ts";

let pairCode: { code: string; expiresInSeconds: number } | null = null;

const router: Router = new Router({
  extensionId: chrome.runtime.id,
  popupUrl: chrome.runtime.getURL("popup.html"),
  testPopup: e2eEnabled,
  now: Date.now,
  handlePopup: async (request, sender) => popup.handle(request, await tabFor(sender)),
});

const pairings = new PairingStore(createIndexedDbStore());
const native = new NativeClient({
  connect: e2ePortFactory() ?? ((name) => chrome.runtime.connectNative(name)),
  lastError: () => chrome.runtime.lastError?.message,
  pairings,
  hint: detectHint(navigator.userAgent),
  onEvent: (event) => {
    let popupEvent: PopupEvent;
    if (event.event === "pairCode") {
      pairCode = { code: event.code, expiresInSeconds: event.expiresInSeconds };
      popupEvent = { event: "pairCode", code: event.code, expiresInSeconds: event.expiresInSeconds };
    } else {
      popupEvent = { event: "connection", connection: native.connection };
    }
    router.broadcast(popupEvent);
  },
});

const fillDeps: FillDeps = {
  native,
  tabs: chromeTabs(),
  session: chromeSession(),
  badge: chromeBadge(),
  now: Date.now,
};
const hold = new GeneratedHold(Date.now);
const gestures: GestureDeps = { fill: fillDeps, hold, activeTab };
const popup = new PopupService({ native, fill: fillDeps, hold }, () => (native.state.pairing ? pairCode : null));

/** The tab a popup request is about: the front tab (a test popup page names its own). */
async function tabFor(sender: SenderInfo): Promise<TabRef | null> {
  const testTab = testTabOf(sender.url);
  if (testTab === null) {
    return activeTab();
  }
  const tab = await chrome.tabs.get(testTab);
  return { id: testTab, url: tab.url };
}

chrome.runtime.onConnect.addListener((port) => {
  router.onConnect(port);
});
chrome.runtime.onMessage.addListener((message: unknown, sender) => {
  router.onMessage(message, sender);
});
chrome.commands.onCommand.addListener((command, tab) => {
  void onCommand(gestures, command, tab);
});
chrome.contextMenus.onClicked.addListener((info, tab) => {
  void onMenuClicked(gestures, info, tab);
});
chrome.runtime.onInstalled.addListener(() => {
  void createMenus();
});
chrome.tabs.onRemoved.addListener((tabId) => {
  router.forgetTab(tabId);
  void fillDeps.session.remove(lastFilledKey(tabId), lastErrorKey(tabId));
});

installE2E({ native, pairings, popup, gestures });
