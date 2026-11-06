/**
 * E2E builds only. Installs `globalThis.mosaicE2E`, which the Playwright harness drives from
 * the service worker: it swaps the native port for a fake desktop, runs the real gesture
 * handlers on a tab, and can pause a fill between probe and delivery. The production build
 * replaces this file with `e2e.stub.ts` (plugin `excludeE2e`); `check-dist.mjs` fails the
 * build if any of it is left in.
 */
import type { FillOutcome } from "./fill.ts";
import { fillHooks } from "./fill.ts";
import { COMMAND_FILL, onCommand, onMenuClicked, type GestureDeps } from "./gestures.ts";
import type { NativeClient, PortFactory } from "./native.ts";
import type { PasskeySwitch } from "./passkey-registry.ts";
import type { OpenPrompt, PasskeyDecision, PasskeyLogEntry } from "./e2e-passkey.ts";
import type { PairingStore } from "./pairing.ts";
import type { PopupService } from "./popup.ts";
import { FAKE_PAIRING, FakeDesktop, defaultConfig, type E2EConfig, type LogEntry, type SaveProposal } from "./e2e-desktop.ts";
import type { FrameRef, PopupRequest, PopupState } from "../shared/messages.ts";
import type { SaveService } from "./save.ts";
import type { KeyValueStore } from "./db.ts";
import { errorCodeOf } from "../shared/errors.ts";

export interface E2EHost {
  native: NativeClient;
  pairings: PairingStore;
  popup: PopupService;
  gestures: GestureDeps;
  save: SaveService;
  kv: KeyValueStore;
  passkeys: PasskeySwitch;
}

export type E2EGesture =
  | { kind: "shortcut"; tabId: number }
  | { kind: "contextMenu"; tabId: number; frameId?: number; menuItemId?: string }
  | { kind: "popupFill"; tabId: number; itemId: string; confirmFrame?: boolean | FrameRef; otpOnly?: boolean };

const desktop = new FakeDesktop();

export const e2eEnabled = true;

/** The fake native port, used instead of `chrome.runtime.connectNative`. */
export const e2ePortFactory = (): PortFactory | null => () => desktop.createPort();

/** The tab a test popup page is about (`popup.html?e2eTabId=<id>`). */
export function testTabOf(url: string | undefined): number | null {
  const value = url === undefined ? null : new URL(url).searchParams.get("e2eTabId");
  return value === null ? null : Number(value);
}

/** The popup's "generate" on a tab; returns the value, which the background now holds for a fill. */
async function popupGenerate(popup: PopupService, tabId: number): Promise<string> {
  const tab = await chrome.tabs.get(tabId);
  const generated = (await popup.handle(
    { op: "generate", request: { kind: "forSite", host: "", rules: null } },
    { id: tabId, url: tab.url },
  )) as { value: string };
  return generated.value;
}

/** `true` confirms the frame the popup shows right now (what a click on "Fill in frame" does); a `FrameRef` is a stale or forged one. */
async function confirmationOf(popup: PopupService, tabId: number, url: string | undefined, want: boolean | FrameRef | undefined): Promise<FrameRef | null> {
  if (typeof want === "object") {
    return want;
  }
  if (want !== true) {
    return null;
  }
  const state = (await popup.handle({ op: "getState" }, { id: tabId, url })) as PopupState;
  return state.frameTarget;
}

async function resetE2E(host: E2EHost, partial: Partial<E2EConfig>): Promise<void> {
  desktop.config = { ...defaultConfig(), ...partial };
  desktop.log.length = 0;
  desktop.proposals.length = 0;
  host.native.disconnect();
  await host.save.setOffers(false);
  await host.passkeys.set(false);
  desktop.passkeys.reset();
  await chrome.storage.session.clear();
  await host.kv.clear();
  if (desktop.config.paired) {
    await host.pairings.save(FAKE_PAIRING);
  } else {
    await host.pairings.clear();
  }
  Object.assign(host.native.state, {
    checked: false,
    desktop: null,
    paired: false,
    pairing: false,
    error: null,
    hasPairing: desktop.config.paired,
  });
}

async function runE2EGesture(host: E2EHost, gesture: E2EGesture): Promise<FillOutcome | null> {
  const tab = await chrome.tabs.get(gesture.tabId);
  if (gesture.kind === "shortcut") {
    return onCommand(host.gestures, COMMAND_FILL, tab);
  }
  if (gesture.kind === "contextMenu") {
    return onMenuClicked(host.gestures, { menuItemId: gesture.menuItemId ?? "fill", frameId: gesture.frameId ?? 0 }, tab);
  }
  const item = desktop.config.items.find((i) => i.itemId === gesture.itemId);
  try {
    const summary = await host.popup.handle(
      {
        op: gesture.otpOnly === true ? "fillOtp" : "fill",
        vaultId: item?.vaultId ?? "",
        itemId: gesture.itemId,
        confirmFrame: await confirmationOf(host.popup, gesture.tabId, tab.url, gesture.confirmFrame),
      },
      { id: gesture.tabId, url: tab.url },
    );
    return { ok: true, summary: summary as Extract<FillOutcome, { ok: true }>["summary"] };
  } catch (error) {
    return { ok: false, error: errorCodeOf(error) };
  }
}

export function installE2E(host: E2EHost): void {
  let paused = false;
  let release: (() => void) | null = null;

  const api = {
    reset: (partial: Partial<E2EConfig> = {}): Promise<void> => resetE2E(host, partial),

    /** Changes the fake desktop (lock it, unlock it) without forgetting anything else, and starts a new session. */
    configure(partial: Partial<E2EConfig>): void {
      Object.assign(desktop.config, partial);
      host.native.disconnect();
      Object.assign(host.native.state, { checked: false, desktop: null, paired: false, pairing: false, error: null });
    },

    log(): LogEntry[] {
      return desktop.log.map((entry) => ({ ...entry }));
    },

    gesture: (gesture: E2EGesture): Promise<FillOutcome | null> => runE2EGesture(host, gesture),

    popupGenerate: (tabId: number): Promise<string> => popupGenerate(host.popup, tabId),

    /** Any popup operation on a tab, as the popup page would send it. */
    async popupRequest(tabId: number, request: PopupRequest): Promise<unknown> {
      const tab = await chrome.tabs.get(tabId);
      return host.popup.handle(request, { id: tabId, url: tab.url });
    },

    /** Save capture on or off (the site access is part of the e2e manifest). */
    setCapture: (enabled: boolean): Promise<boolean> => host.save.setOffers(enabled),

    /** "Use Mosaic for passkeys" on or off (the site access is part of the e2e manifest). */
    setPasskeys: (enabled: boolean): Promise<boolean> => host.passkeys.set(enabled),

    /** The answer in the pretend desktop dialog; kept for the next dialog if none is open. */
    passkeyDecide(decision: PasskeyDecision): void {
      desktop.passkeys.decide(decision);
    },

    /** The passkey requests the fake desktop received, in order (ids and origins only). */
    passkeyLog: (): PasskeyLogEntry[] => desktop.passkeys.log.map((entry) => ({ ...entry })),

    /** The dialog that is open now, if any. */
    passkeyPending: (): OpenPrompt[] => desktop.passkeys.pending(),

    /** What the fake desktop was asked to save. */
    saves: (): SaveProposal[] => desktop.proposals.map((p) => ({ ...p })),

    async popupState(tabId: number): Promise<PopupState> {
      const tab = await chrome.tabs.get(tabId);
      return (await host.popup.handle({ op: "getState" }, { id: tabId, url: tab.url })) as PopupState;
    },

    /** The next fill stops after its probe until `release()`, so a test can change the page in between. */
    pauseAfterProbe(): void {
      fillHooks.afterProbe = () =>
        new Promise<void>((resolve) => {
          delete fillHooks.afterProbe;
          paused = true;
          release = resolve;
        });
    },

    isPaused: (): boolean => paused,

    release(): void {
      paused = false;
      release?.();
      release = null;
    },
  };

  Object.assign(globalThis, { mosaicE2E: api });
}
