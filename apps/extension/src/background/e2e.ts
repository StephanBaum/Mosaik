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
import type { PairingStore } from "./pairing.ts";
import type { PopupService } from "./popup.ts";
import { FAKE_PAIRING, FakeDesktop, defaultConfig, type E2EConfig, type LogEntry } from "./e2e-desktop.ts";
import type { PopupState } from "../shared/messages.ts";
import { errorCodeOf } from "../shared/errors.ts";

export interface E2EHost {
  native: NativeClient;
  pairings: PairingStore;
  popup: PopupService;
  gestures: GestureDeps;
}

export type E2EGesture =
  | { kind: "shortcut"; tabId: number }
  | { kind: "contextMenu"; tabId: number; frameId?: number; menuItemId?: string }
  | { kind: "popupFill"; tabId: number; itemId: string; confirmFrame?: boolean; otpOnly?: boolean };

const desktop = new FakeDesktop();

export const e2eEnabled = true;

/** The fake native port, used instead of `chrome.runtime.connectNative`. */
export const e2ePortFactory = (): PortFactory | null => () => desktop.createPort();

/** The tab a test popup page is about (`popup.html?e2eTabId=<id>`). */
export function testTabOf(url: string | undefined): number | null {
  const value = url === undefined ? null : new URL(url).searchParams.get("e2eTabId");
  return value === null ? null : Number(value);
}

export function installE2E(host: E2EHost): void {
  let paused = false;
  let release: (() => void) | null = null;

  const api = {
    async reset(partial: Partial<E2EConfig> = {}): Promise<void> {
      desktop.config = { ...defaultConfig(), ...partial };
      desktop.log.length = 0;
      host.native.disconnect();
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
    },

    log(): LogEntry[] {
      return desktop.log.map((entry) => ({ ...entry }));
    },

    async gesture(gesture: E2EGesture): Promise<FillOutcome | null> {
      const tab = await chrome.tabs.get(gesture.tabId);
      if (gesture.kind === "shortcut") {
        return onCommand(host.gestures, COMMAND_FILL, tab);
      }
      if (gesture.kind === "contextMenu") {
        return onMenuClicked(
          host.gestures,
          { menuItemId: gesture.menuItemId ?? "fill", frameId: gesture.frameId ?? 0 },
          tab,
        );
      }
      const item = desktop.config.items.find((i) => i.itemId === gesture.itemId);
      try {
        const summary = await host.popup.handle(
          {
            op: gesture.otpOnly === true ? "fillOtp" : "fill",
            vaultId: item?.vaultId ?? "",
            itemId: gesture.itemId,
            confirmFrame: gesture.confirmFrame ?? false,
          },
          { id: gesture.tabId, url: tab.url },
        );
        return { ok: true, summary: summary as Extract<FillOutcome, { ok: true }>["summary"] };
      } catch (error) {
        return { ok: false, error: errorCodeOf(error) };
      }
    },

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
