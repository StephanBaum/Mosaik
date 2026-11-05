/**
 * Harness of the hostile-page suite: one Chromium with the e2e build of the extension loaded,
 * the hostile-site server (https://*.test:4843, http://*.test:4880, `*.test` mapped to
 * 127.0.0.1), and a handle on the extension's `mosaicE2E` hooks in its service worker.
 *
 * The browser is started once per worker; every test resets the fake desktop first.
 */
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  chromium,
  expect,
  test as base,
  type BrowserContext,
  type Page,
  type Worker,
} from "@playwright/test";
import type { E2EConfig, E2EItem, LogEntry } from "../src/background/e2e-desktop.ts";
import type { E2EGesture } from "../src/background/e2e.ts";
import type { FillOutcome } from "../src/background/fill.ts";
import type { PopupState } from "../src/shared/messages.ts";
import { HTTP_PORT, HTTPS_PORT, startServer } from "../test/hostile-site/server.mjs";

const DIST = resolve(import.meta.dirname, "../dist/e2e");
/** A second, hostile extension (test/evil-extension), loaded only where a test opts in. */
const EVIL_EXTENSION = resolve(import.meta.dirname, "../test/evil-extension");

export const BANK = `https://bank.test:${String(HTTPS_PORT)}`;
export const EVIL = `https://evil.test:${String(HTTPS_PORT)}`;
export const BANK_HTTP = `http://bank.test:${String(HTTP_PORT)}`;
export const USERNAME = "ada@bank.test";
export const PASSWORD = "correct horse battery staple";

export const bankItem = (over: Partial<E2EItem> = {}): E2EItem => ({
  vaultId: "v1",
  itemId: "i1",
  title: "Bank",
  username: USERNAME,
  password: PASSWORD,
  origin: BANK,
  allowHttp: false,
  totp: null,
  ...over,
});

interface Hooks {
  reset(config?: Partial<E2EConfig>): Promise<void>;
  log(): LogEntry[];
  gesture(gesture: E2EGesture): Promise<FillOutcome | null>;
  popupState(tabId: number): Promise<PopupState>;
  popupGenerate(tabId: number): Promise<string>;
  pauseAfterProbe(): void;
  isPaused(): boolean;
  release(): void;
}

type WithHooks = { mosaicE2E: Hooks };

export interface Fake {
  reset(config?: Partial<E2EConfig>): Promise<void>;
  log(): Promise<LogEntry[]>;
  fills(): Promise<LogEntry[]>;
  gesture(gesture: E2EGesture): Promise<FillOutcome | null>;
  popupState(tabId: number): Promise<PopupState>;
  popupGenerate(tabId: number): Promise<string>;
  pauseAfterProbe(): Promise<void>;
  isPaused(): Promise<boolean>;
  release(): Promise<void>;
  tabIdOf(urlPrefix: string): Promise<number>;
  frameIdOf(tabId: number, urlPrefix: string): Promise<number>;
}

interface Extension {
  id: string;
  context: BrowserContext;
  worker(): Promise<Worker>;
}

/** The extension's service worker, started again (by opening its popup page) if the browser stopped it. */
async function liveWorker(context: BrowserContext, id: string): Promise<Worker> {
  for (let attempt = 0; attempt < 5; attempt++) {
    const worker = context.serviceWorkers().find((w) => w.url().startsWith(`chrome-extension://${id}/`));
    if (worker !== undefined) {
      try {
        await worker.evaluate(() => 1);
        return worker;
      } catch {
        // stopped between the lookup and the call: wake it below
      }
    }
    const waker = await context.newPage();
    await waker.goto(`chrome-extension://${id}/popup.html`);
    await waker.close();
  }
  throw new Error("the extension's service worker did not start");
}

async function launch(withEvilExtension: boolean): Promise<Extension & { close(): Promise<void> }> {
  if (!existsSync(join(DIST, "manifest.json"))) {
    throw new Error("dist/e2e is missing: run `pnpm --filter @mosaic/extension build:e2e` first");
  }
  const profile = await mkdtemp(join(tmpdir(), "mosaic-e2e-"));
  // The hostile extension has no service worker, so the first one is always Mosaic's.
  const extensions = withEvilExtension ? `${DIST},${EVIL_EXTENSION}` : DIST;
  const context = await chromium.launchPersistentContext(profile, {
    channel: "chromium",
    headless: true,
    args: [
      `--disable-extensions-except=${extensions}`,
      `--load-extension=${extensions}`,
      "--host-resolver-rules=MAP *.test 127.0.0.1",
      "--ignore-certificate-errors",
    ],
  });
  const first = context.serviceWorkers()[0] ?? (await context.waitForEvent("serviceworker"));
  const id = new URL(first.url()).host;
  return {
    id,
    context,
    worker: () => liveWorker(context, id),
    close: async () => {
      await context.close();
      await rm(profile, { recursive: true, force: true });
    },
  };
}

function makeFake(ext: Extension): Fake {
  // The callbacks run inside the service worker: they use nothing but their argument and globals.
  const run = async <R>(fn: (arg: never) => R | Promise<R>, arg?: unknown): Promise<R> =>
    (await ext.worker()).evaluate<R, never>(fn as never, arg as never);
  const log = (): Promise<LogEntry[]> => run(() => (globalThis as unknown as WithHooks).mosaicE2E.log());
  return {
    reset: (config) => run((c: Partial<E2EConfig> | undefined) => (globalThis as unknown as WithHooks).mosaicE2E.reset(c), config),
    log,
    fills: async () => (await log()).filter((entry) => entry.op === "fill"),
    gesture: (gesture) => run((g: E2EGesture) => (globalThis as unknown as WithHooks).mosaicE2E.gesture(g), gesture),
    popupState: (tabId) => run((t: number) => (globalThis as unknown as WithHooks).mosaicE2E.popupState(t), tabId),
    popupGenerate: (tabId) => run((t: number) => (globalThis as unknown as WithHooks).mosaicE2E.popupGenerate(t), tabId),
    pauseAfterProbe: () => run(() => { (globalThis as unknown as WithHooks).mosaicE2E.pauseAfterProbe(); }),
    isPaused: () => run(() => (globalThis as unknown as WithHooks).mosaicE2E.isPaused()),
    release: () => run(() => { (globalThis as unknown as WithHooks).mosaicE2E.release(); }),
    tabIdOf: async (urlPrefix) => {
      const id = await run(async (prefix: string) => {
        const tabs = await chrome.tabs.query({});
        return tabs.find((tab) => tab.url?.startsWith(prefix) === true)?.id;
      }, urlPrefix);
      if (id === undefined) {
        throw new Error(`no tab at ${urlPrefix}`);
      }
      return id;
    },
    frameIdOf: async (tabId, urlPrefix) => {
      const id = await run(async (arg: { tabId: number; prefix: string }) => {
        const frames = (await chrome.webNavigation.getAllFrames({ tabId: arg.tabId })) ?? [];
        return frames.find((frame) => frame.url.startsWith(arg.prefix))?.frameId;
      }, { tabId, prefix: urlPrefix });
      if (id === undefined) {
        throw new Error(`no frame at ${urlPrefix}`);
      }
      return id;
    },
  };
}

export const test = base.extend<{ fake: Fake; ext: Extension }, { shared: Extension; evilExtension: boolean }>({
  // `test.use({ evilExtension: true })` starts a browser that also has the hostile extension.
  evilExtension: [false, { scope: "worker", option: true }],
  shared: [
    async ({ evilExtension }, use) => {
      const server = await startServer();
      const ext = await launch(evilExtension);
      await use(ext);
      await ext.close();
      await server.close();
    },
    { scope: "worker" },
  ],
  context: async ({ shared }, use) => {
    await use(shared.context);
  },
  // The browser outlives the test, so the test's page must not.
  page: async ({ context }, use) => {
    const page = await context.newPage();
    await use(page);
    await page.close();
  },
  ext: async ({ shared }, use) => {
    await use(shared);
  },
  fake: async ({ shared }, use) => {
    await use(makeFake(shared));
  },
});

export { expect };

interface FrameState {
  url: string;
  values: Record<string, string>;
  leaks: { id: string; value: string }[];
}

/** What every frame of the page holds in its inputs, and what its scripts saw arrive. */
export async function inspect(page: Page): Promise<FrameState[]> {
  return Promise.all(
    page.frames().map(async (frame) => ({
      url: frame.url(),
      values: await frame.evaluate(() =>
        Object.fromEntries([...document.querySelectorAll("input")].map((el) => [el.id || el.name, el.value])),
      ),
      leaks: await frame.evaluate(
        () => (window as unknown as { __mosaicLeaks?: { id: string; value: string }[] }).__mosaicLeaks ?? [],
      ),
    })),
  );
}

/** No input of any frame holds a value, and no script saw one arrive: no credential reached the page. */
export async function expectNothingFilled(page: Page): Promise<void> {
  for (const frame of await inspect(page)) {
    expect(Object.values(frame.values).filter((v) => v !== ""), `values in ${frame.url}`).toEqual([]);
    expect(frame.leaks, `leaks in ${frame.url}`).toEqual([]);
  }
}

/** The frame's `#user` and `#pass` hold the item's credential. */
export async function expectFilled(page: Page, urlPrefix: string): Promise<void> {
  const frame = (await inspect(page)).find((f) => f.url.startsWith(urlPrefix));
  expect(frame?.values["user"]).toBe(USERNAME);
  expect(frame?.values["pass"]).toBe(PASSWORD);
}

/** Opens a page and waits until it and its frames have loaded. */
export async function open(page: Page, url: string, frames = 1): Promise<number> {
  await page.goto(url);
  await expect.poll(() => page.frames().length, { message: "frames loaded" }).toBe(frames);
  for (const frame of page.frames()) {
    await frame.waitForLoadState("load");
  }
  return frames;
}
