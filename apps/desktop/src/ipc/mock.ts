/**
 * The in-memory mock IPC for browser development (`pnpm dev:mock`), component tests and
 * Playwright. Handlers live per contract area in `mock/`; a command without a handler
 * fails with `notImplemented`, like its Rust stub.
 *
 * URL options in mock mode: `?mock=noAccount|locked|unlocked` picks the start state
 * (default `noAccount`; the sample account's master password is `mosaic`), and
 * `&latency=<ms>` delays every response to show loading states, and `&sshRequest=1` makes
 * the mock agent ask for an approval 500 ms after unlock. Quick unlock (docs/DESKTOP_IPC.md
 * §3): `&quick=biometric,hardwareKey` seeds an enrolled biometric and one security key
 * (and makes keys "supported"; the real backend says no on every OS), `&bioPlatform=
 * windowsHello|touchId|none`, `&quickFail=<code>` fails `unlock_quick`, `&touchMs=<ms>`
 * delays a security-key touch, `&hwOpenSeconds=<s>` sets how long a hardware vault stays
 * open (default 300). A security key needs the PIN `1234`. One-time code setup
 * (ADR-0031): `&otpScan=<errorCode>|cancel` makes `otp_scan` fail with that code (or
 * cancels the file dialog), `&screenScan=0` reports screen scanning as unsupported. Browser
 * extension (M4): `&browserPair=1` makes a browser ask for pairing, and `&browserSave=new|update`
 * makes it offer to save a login, both 500 ms after the app subscribes. Passkeys (M5):
 * `&browserPasskey=get|create|excluded|uv|uvExpires` makes it ask to sign in (`uv`: with the master
 * password; `uvExpires`: the verification runs out while the dialog is open) or to create a passkey, or to create one the vault already holds; the sign-in
 * variants seed two passkeys for example.com.
 */
import type { EventListener, MosaicIpc } from "./client.ts";
import {
  biometricPlatformKinds,
  errorCodes,
  type BiometricPlatformKind,
  type CommandName,
  type ErrorCode,
  type IpcResult, type RequestOf,
  type ResponseOf,
} from "./contract/index.ts";
import { appHandlers } from "./mock/app.ts";
import {
  browserHandlers,
  defaultBrowser,
  startBrowser,
  type PasskeyVariant,
} from "./mock/browser.ts";
import { attachmentHandlers } from "./mock/attachments.ts";
import { clipboardHandlers } from "./mock/clipboard.ts";
import {
  bulkItems,
  sampleAccount,
  sampleAttachments,
  sampleItems,
  samplePasskeys,
  sampleVaults,
} from "./mock/data.ts";
import { generatorHandlers } from "./mock/generator.ts";
import { exportHandlers } from "./mock/export.ts";
import { importHandlers } from "./mock/import.ts";
import { insightHandlers } from "./mock/insights.ts";
import { itemHandlers } from "./mock/items.ts";
import { otpHandlers } from "./mock/otp.ts";
import {
  quickFromOptions,
  quickUnlockAsyncHandlers,
  quickUnlockHandlers,
  seedHardwareVault,
  type QuickOptions,
} from "./mock/quickUnlock.ts";
import { sessionAsyncHandlers, sessionHandlers } from "./mock/session.ts";
import { defaultSettings, settingsHandlers } from "./mock/settings.ts";
import { sshHandlers, startAgent } from "./mock/ssh.ts";
import {
  MockFailure,
  type MockAsyncHandlers,
  type MockAttachment,
  type MockHandlers,
  type MockItem,
  type MockSetupOptions,
  type MockState,
} from "./mock/state.ts";
import { totpHandlers } from "./mock/totp.ts";
import { vaultAsyncHandlers, vaultHandlers } from "./mock/vaults.ts";

export type MockStart = "noAccount" | "locked" | "unlocked";

export interface MockOptions extends QuickOptions {
  start?: MockStart;
  latencyMs?: number;
  /** Emit an SSH approval request 500 ms after unlock. */
  sshRequest?: boolean;
  /** A browser asks for pairing 500 ms after subscribing (`&browserPair=1`). */
  browserPair?: boolean;
  /** A browser offers to save a login 500 ms after subscribing (`&browserSave=new|update`). */
  browserSave?: "new" | "update";
  /** A browser asks for a passkey 500 ms after subscribing (`&browserPasskey=`). */
  browserPasskey?: PasskeyVariant;
  /** Start with an unconfirmed Emergency Kit (`?kit=unverified`). */
  kitUnverified?: boolean;
  /** Start with vaults but no items (first run without an import). */
  emptyVault?: boolean;
  /** Fail `otp_scan` with this code, or cancel the file dialog (`?otpScan=cancel`). */
  otpScan?: ErrorCode | "cancel";
  /** Report screen scanning as unsupported when `false` (`&screenScan=0`). */
  screenScan?: boolean;
  /** Fail `attachment_add` with this code, or cancel the file dialog (`?attachAdd=cancel`). */
  attachAdd?: ErrorCode | "cancel";
  /** Fail `attachment_save` with this code, or cancel the save dialog (`?attachSave=cancel`). */
  attachSave?: ErrorCode | "cancel";
  /** Add this many generated logins (`?items=5000`), for long-list checks. */
  bulkItems?: number;
  now?: () => number;
}

/** One handler set per contract area. */
const handlers: MockHandlers = {
  ...sessionHandlers,
  ...quickUnlockHandlers,
  ...appHandlers,
  ...importHandlers,
  ...exportHandlers,
  ...vaultHandlers,
  ...itemHandlers,
  ...attachmentHandlers,
  ...clipboardHandlers,
  ...generatorHandlers,
  ...totpHandlers,
  ...otpHandlers,
  ...settingsHandlers,
  ...insightHandlers,
  ...sshHandlers,
  ...browserHandlers,
};

function quickOptionsFromParams(params: URLSearchParams): MockOptions {
  const quick = (params.get("quick") ?? "")
    .split(",")
    .filter((method): method is "biometric" | "hardwareKey" => method === "biometric" || method === "hardwareKey");
  const platform = params.get("bioPlatform");
  const fail = params.get("quickFail");
  const touchMs = Number(params.get("touchMs") ?? "");
  const hwOpenSeconds = Number(params.get("hwOpenSeconds") ?? "");
  return {
    ...(quick.length > 0 ? { quick } : {}),
    ...(platform === "none"
      ? { biometricPlatform: null }
      : biometricPlatformKinds.some((kind) => kind === platform)
        ? { biometricPlatform: platform as BiometricPlatformKind }
        : {}),
    ...(errorCodes.some((code) => code === fail) ? { quickFail: fail as ErrorCode } : {}),
    ...(Number.isFinite(touchMs) && touchMs > 0 ? { touchMs } : {}),
    ...(Number.isFinite(hwOpenSeconds) && hwOpenSeconds > 0 ? { hwOpenSeconds } : {}),
  };
}

/** Handlers that wait on a device; they run after the sync ones are ruled out. */
const asyncHandlers: MockAsyncHandlers = {
  ...sessionAsyncHandlers,
  ...quickUnlockAsyncHandlers,
  ...vaultAsyncHandlers,
};

function browserOptionsFromParams(params: URLSearchParams): MockOptions {
  const save = params.get("browserSave");
  const passkey = params.get("browserPasskey");
  return {
    ...(params.get("browserPair") === "1" ? { browserPair: true } : {}),
    ...(save === "new" || save === "update" ? { browserSave: save } : {}),
    ...(passkey === "get" || passkey === "create" || passkey === "excluded" || passkey === "uv" || passkey === "uvExpires"
      ? { browserPasskey: passkey }
      : {}),
  };
}

function otpOptionsFromParams(params: URLSearchParams): MockOptions {
  const scan = params.get("otpScan");
  return {
    ...(scan === "cancel" || errorCodes.some((code) => code === scan)
      ? { otpScan: scan as ErrorCode | "cancel" }
      : {}),
    ...(params.get("screenScan") === "0" ? { screenScan: false } : {}),
  };
}

function attachOptionsFromParams(params: URLSearchParams): MockOptions {
  const pick = (name: string): ErrorCode | "cancel" | undefined => {
    const value = params.get(name);
    return value === "cancel" || errorCodes.some((code) => code === value)
      ? (value as ErrorCode | "cancel")
      : undefined;
  };
  const add = pick("attachAdd");
  const save = pick("attachSave");
  return {
    ...(add === undefined ? {} : { attachAdd: add }),
    ...(save === undefined ? {} : { attachSave: save }),
  };
}

export function mockOptionsFromUrl(search: string): MockOptions {
  const params = new URLSearchParams(search);
  const start = params.get("mock");
  const latency = Number(params.get("latency") ?? "0");
  const bulk = Math.trunc(Number(params.get("items") ?? "0"));
  return {
    ...(start === "locked" || start === "unlocked" || start === "noAccount" ? { start } : {}),
    ...(Number.isFinite(latency) && latency > 0 ? { latencyMs: latency } : {}),
    ...(params.get("sshRequest") === "1" ? { sshRequest: true } : {}),
    ...browserOptionsFromParams(params),
    ...(params.get("kit") === "unverified" ? { kitUnverified: true } : {}),
    ...otpOptionsFromParams(params),
    ...attachOptionsFromParams(params),
    ...(bulk > 0 && bulk <= 100_000 ? { bulkItems: bulk } : {}),
    ...quickOptionsFromParams(params),
  };
}

function initialItems(start: MockStart, options: MockOptions): MockItem[] {
  if (start === "noAccount" || options.emptyVault === true) {
    return [];
  }
  const passkeys =
    options.browserPasskey === undefined || options.browserPasskey === "create"
      ? []
      : samplePasskeys();
  return [...sampleItems(), ...passkeys, ...bulkItems(options.bulkItems ?? 0)];
}

function initialAttachments(start: MockStart, options: MockOptions): Map<string, MockAttachment> {
  return start === "noAccount" || options.emptyVault === true
    ? new Map<string, MockAttachment>()
    : sampleAttachments();
}

/** The URL options the handlers read, with their defaults. */
function setupOptions(options: MockOptions): MockSetupOptions {
  return {
    otpScan: options.otpScan ?? null,
    screenScan: options.screenScan ?? true,
    attachAdd: options.attachAdd ?? null,
    attachSave: options.attachSave ?? null,
  };
}

export function createMockIpc(options: MockOptions = {}): MosaicIpc {
  const listeners = new Set<EventListener>();
  let counter = 0;
  const start = options.start ?? "noAccount";
  const now = options.now ?? (() => Date.now());
  const state: MockState = {
    account: start === "noAccount" ? null : { ...sampleAccount },
    secretKeyStored: start !== "noAccount",
    unlocked: start === "unlocked",
    kitVerified: start !== "noAccount" && options.kitUnverified !== true,
    sshEnabled: false,
    browser: {
      ...defaultBrowser(now),
      pairOnStart: options.browserPair === true,
      saveOnStart: options.browserSave ?? null,
      passkeyOnStart: options.browserPasskey ?? null,
    },
    heldImport: null,
    vaults: start === "noAccount" ? [] : sampleVaults(),
    items: initialItems(start, options),
    settings: { ...defaultSettings },
    quick: quickFromOptions(options, now),
    hardwareVaults: [],
    lastGenerated: null,
    otpPending: new Map(),
    otpPendingCounter: 0,
    attachments: initialAttachments(start, options),
    options: setupOptions(options),
    emit: (event) => {
      for (const listener of listeners) {
        listener(event);
      }
    },
    sshRequest: options.sshRequest === true,
    now,
    nextId: () => {
      counter += 1;
      return `019a2b3c-0000-7000-8000-${counter.toString(16).padStart(12, "0")}`;
    },
  };

  seedHardwareVault(state, options, start !== "noAccount");

  if (start === "unlocked") {
    startAgent(state);
  }

  async function call<K extends CommandName>(
    command: K,
    request: RequestOf<K>,
  ): Promise<IpcResult<ResponseOf<K>>> {
    if (options.latencyMs !== undefined) {
      await new Promise((resolve) => setTimeout(resolve, options.latencyMs));
    } else {
      await Promise.resolve();
    }
    const handler = handlers[command] ?? asyncHandlers[command];
    if (handler === undefined) {
      return { ok: false, error: { code: "notImplemented" } };
    }
    try {
      // Responses are copied so the UI cannot mutate mock state by reference.
      return { ok: true, value: structuredClone(await handler(state, request)) };
    } catch (error) {
      if (error instanceof MockFailure) {
        return { ok: false, error: { code: error.code } };
      }
      return { ok: false, error: { code: "internal" } };
    }
  }

  return {
    call,
    subscribe(listener) {
      listeners.add(listener);
      startBrowser(state);
      return Promise.resolve(() => {
        listeners.delete(listener);
      });
    },
  };
}
