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
  type Frame,
  type Page,
  type Worker,
} from "@playwright/test";
import type { E2EConfig, E2EItem, LogEntry, SaveProposal } from "../src/background/e2e-desktop.ts";
import type { OpenPrompt, PasskeyDecision, PasskeyLogEntry } from "../src/background/e2e-passkey.ts";
import type { E2EGesture } from "../src/background/e2e.ts";
import type { FillOutcome } from "../src/background/fill.ts";
import type { PopupRequest, PopupState } from "../src/shared/messages.ts";
import { HTTP_PORT, HTTPS_PORT, startServer } from "../test/hostile-site/server.mjs";
import { verifyAssertion, verifyRegistration } from "../test/webauthn/verifier.ts";

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
  configure(config: Partial<E2EConfig>): void;
  log(): LogEntry[];
  gesture(gesture: E2EGesture): Promise<FillOutcome | null>;
  popupState(tabId: number): Promise<PopupState>;
  popupGenerate(tabId: number): Promise<string>;
  popupRequest(tabId: number, request: PopupRequest): Promise<unknown>;
  setCapture(enabled: boolean): Promise<boolean>;
  saves(): SaveProposal[];
  setPasskeys(enabled: boolean): Promise<boolean>;
  passkeyDecide(decision: PasskeyDecision): void;
  passkeyLog(): PasskeyLogEntry[];
  passkeyPending(): OpenPrompt[];
  pauseAfterProbe(): void;
  isPaused(): boolean;
  release(): void;
}

type WithHooks = { mosaicE2E: Hooks };

export interface Fake {
  reset(config?: Partial<E2EConfig>): Promise<void>;
  configure(config: Partial<E2EConfig>): Promise<void>;
  log(): Promise<LogEntry[]>;
  fills(): Promise<LogEntry[]>;
  gesture(gesture: E2EGesture): Promise<FillOutcome | null>;
  popupState(tabId: number): Promise<PopupState>;
  popupGenerate(tabId: number): Promise<string>;
  popupRequest(tabId: number, request: PopupRequest): Promise<unknown>;
  setCapture(enabled: boolean): Promise<boolean>;
  saves(): Promise<SaveProposal[]>;
  /** "Use Mosaic for passkeys": registers (or removes) the passkey scripts; open the page after turning it on. */
  setPasskeys(enabled: boolean): Promise<boolean>;
  /** The answer in the fake desktop's passkey dialog; kept for the next dialog if none is open. */
  passkeyDecide(decision: PasskeyDecision): Promise<void>;
  /** Passkey requests the fake desktop received (ids and origins, never key material). */
  passkeyLog(): Promise<PasskeyLogEntry[]>;
  /** The dialog that is open now. */
  passkeyPending(): Promise<OpenPrompt[]>;
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
    configure: (config) =>
      run((c: Partial<E2EConfig>) => { (globalThis as unknown as WithHooks).mosaicE2E.configure(c); }, config),
    log,
    fills: async () => (await log()).filter((entry) => entry.op === "fill"),
    gesture: (gesture) => run((g: E2EGesture) => (globalThis as unknown as WithHooks).mosaicE2E.gesture(g), gesture),
    popupState: (tabId) => run((t: number) => (globalThis as unknown as WithHooks).mosaicE2E.popupState(t), tabId),
    popupGenerate: (tabId) => run((t: number) => (globalThis as unknown as WithHooks).mosaicE2E.popupGenerate(t), tabId),
    popupRequest: (tabId, request) =>
      run((a: { tabId: number; request: PopupRequest }) => (globalThis as unknown as WithHooks).mosaicE2E.popupRequest(a.tabId, a.request), { tabId, request }),
    setCapture: (enabled) => run((e: boolean) => (globalThis as unknown as WithHooks).mosaicE2E.setCapture(e), enabled),
    saves: () => run(() => (globalThis as unknown as WithHooks).mosaicE2E.saves()),
    setPasskeys: (enabled) => run((e: boolean) => (globalThis as unknown as WithHooks).mosaicE2E.setPasskeys(e), enabled),
    passkeyDecide: (decision) =>
      run((d: PasskeyDecision) => { (globalThis as unknown as WithHooks).mosaicE2E.passkeyDecide(d); }, decision),
    passkeyLog: () => run(() => (globalThis as unknown as WithHooks).mosaicE2E.passkeyLog()),
    passkeyPending: () => run(() => (globalThis as unknown as WithHooks).mosaicE2E.passkeyPending()),
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
      const server = await startServer({ passkeyVerifier: { verifyRegistration, verifyAssertion } });
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

export const POPUP_SIZE = { width: 360, height: 600 } as const;

/**
 * The popup page in a tab of its own, about the tab `tabId` (the test popup takes it from the
 * query string), at the size of the real popup. The page is closed by the caller's `page.close`.
 */
export async function openPopup(
  context: BrowserContext,
  extensionId: string,
  tabId: number,
  options: { colorScheme?: "light" | "dark"; languages?: readonly string[] } = {},
): Promise<Page> {
  const popup = await context.newPage();
  await popup.setViewportSize(POPUP_SIZE);
  await popup.emulateMedia({ colorScheme: options.colorScheme ?? "light" });
  if (options.languages !== undefined) {
    const languages = [...options.languages];
    await popup.addInitScript((value) => {
      Object.defineProperty(navigator, "languages", { get: () => value });
    }, languages);
  }
  await popup.goto(`chrome-extension://${extensionId}/popup.html?e2eTabId=${String(tabId)}`);
  return popup;
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

/** A CDP virtual authenticator (CTAP2, internal, resident keys, user verified): what the browser's own WebAuthn answers with, so a fallback is observable. */
export async function addVirtualAuthenticator(context: BrowserContext, page: Page) {
  const cdp = await context.newCDPSession(page);
  await cdp.send("WebAuthn.enable");
  const { authenticatorId } = await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return {
    /** The credentials the browser's own authenticator holds (ids in base64url; CDP reports padded base64). */
    credentials: async (): Promise<{ credentialId: string; rpId: string }[]> => {
      const { credentials } = await cdp.send("WebAuthn.getCredentials", { authenticatorId });
      return credentials.map((c) => ({ credentialId: Buffer.from(c.credentialId, "base64").toString("base64url"), rpId: c.rpId ?? "" }));
    },
  };
}

/** The passkey relying-party page of an origin. */
export const RP_PAGE = "/passkey/rp.html";
export const bankRp = `${BANK}${RP_PAGE}`;
export const evilRp = `${EVIL}${RP_PAGE}`;

/** Waits for the fake desktop's passkey dialog. */
export async function pendingDialog(fake: Fake): Promise<OpenPrompt> {
  let found: OpenPrompt | undefined;
  await expect
    .poll(async () => {
      found = (await fake.passkeyPending())[0];
      return found !== undefined;
    }, { message: "passkey dialog open" })
    .toBe(true);
  return found as unknown as OpenPrompt;
}

export interface RpOptions {
  rpId?: string;
  exclude?: string[];
  allow?: string[];
  noVerify?: boolean;
  challenge?: string;
}

/** What `/passkey/verify` answered (the independent verifier on the relying-party server). */
export interface Verified {
  status: number;
  ok: boolean;
  flags?: number;
  signCount?: number;
  credentialId?: string;
  error?: string;
}

/** What `rp.create` / `rp.get` (test/hostile-site/pages/passkey/rp.js) report: the call's outcome as the page saw it. */
export interface RpResult {
  error?: { name: string; message: string };
  id?: string;
  type?: string;
  attachment?: string;
  isPublicKeyCredential?: boolean;
  extensions?: unknown;
  clientDataJSON?: string;
  attestationObject?: string;
  authenticatorData?: string;
  signature?: string;
  userHandle?: string | null;
  challenge?: string;
  verify?: Verified;
}

interface RpPage {
  rp: {
    create(options: RpOptions): Promise<RpResult>;
    get(options: RpOptions): Promise<RpResult>;
    start(op: "create" | "get", options: RpOptions): void;
    abort(): void;
    result(): Promise<RpResult>;
    verify(body: object): Promise<Verified>;
  };
}

// The callbacks below run in the page: they use nothing but their argument and globals.
export const rpCreate = (target: Page | Frame, options: RpOptions = {}): Promise<RpResult> =>
  target.evaluate((o) => (window as unknown as RpPage).rp.create(o), options);
export const rpGet = (target: Page | Frame, options: RpOptions = {}): Promise<RpResult> =>
  target.evaluate((o) => (window as unknown as RpPage).rp.get(o), options);
/** Starts a call without waiting (its promise stays in the page); `rpResult` waits for it. */
export const rpStart = (target: Page | Frame, op: "create" | "get", options: RpOptions = {}): Promise<void> =>
  target.evaluate((a) => { (window as unknown as RpPage).rp.start(a.op, a.options); }, { op, options });
export const rpAbort = (target: Page | Frame): Promise<void> => target.evaluate(() => { (window as unknown as RpPage).rp.abort(); });
export const rpResult = (target: Page | Frame): Promise<RpResult> => target.evaluate(() => (window as unknown as RpPage).rp.result());
/** Sends an assertion a page got to the server's verifier (again, for a replay). */
export const rpVerifyAssertion = (target: Page | Frame, got: RpResult): Promise<Verified> =>
  target.evaluate(
    (r) =>
      (window as unknown as RpPage).rp.verify({
        kind: "assertion",
        credentialId: r.id,
        challenge: r.challenge,
        clientDataJSON: r.clientDataJSON,
        authenticatorData: r.authenticatorData,
        signature: r.signature,
      }),
    got,
  );

export const FLAGS_CREATE_UV = 0x5d;
export const FLAGS_GET_UV = 0x1d;
