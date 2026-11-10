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
 *
 * Sync (M6, `?mock=unlocked` shows Settings > Sync): `&sync=<variant>` seeds the sync state:
 * none (default: not set up), hosted, hostedNoTotp, folder, syncing, offline, error, signedOut, passwordChanged,
 * and the warnings rollback, identity, serverInconsistent, serverRollback, noCommonView, freeze,
 * missingFiles, undecryptable, hwLocal, notRepaired, allWarnings. `&syncServer=open|invite|closed`
 * is what `sync_check_server` answers (the invite code is `MOCK-INVITE`), `&syncFolder=empty|
 * mosaic|other|cancel` is what the folder picker answers (`mosaic` lists the sample account),
 * `&syncFail=<errorCode>` makes `sync_now` and the two connect commands fail with that code.
 * The mock server takes the master password `mosaic` and the one-time code `123456`. Joining
 * from `?mock=noAccount` takes alex@example.com, `mosaic` and the sample Secret Key;
 * `&syncJoin2fa=1` asks for the code first.
 *
 * Account keys (M7, Settings > Security): `&rotation=<variant>` with none, shared (a review with shared
 * vaults, members, grants and security keys), resumePrepared, resumeAccepted, resumeAttachments,
 * resumeSecretKey (a journal row to continue after unlock), resumeStuck (a committed job that cannot finish: the banner offers to give it up), rekeyElsewhere (hosted: signed out after
 * a re-key on another device), rekeyPending (folder: a verified re-key marker, sync paused),
 * ignoredNotices (a folder with an ignored re-key notice to restore), jumpTooLarge (rekeyPending, but adopting answers
 * `rotationJumpTooLarge`: set the device up again), unknownFile and conflict (a folder account file this device cannot verify), slowDevice
 * (`profile_upgrade` answers `profileTooSlow` unless forced). The other device's kit has the Secret
 * Key `M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ3` and the password `new correct horse battery staple`.
 * `&rotationFail=<errorCode>` makes the commits fail, `&rotationStepMs=<ms>` sets the pause between
 * the steps of a run (default 450).
 *
 * Credential rotation (M7b, Security > Rotate now): `&credRotation=queue|empty|pending|breachRunning|
 * breachOffline|consent` (`&credRotationStepMs=<ms>` paces a scan, default 250): queue seeds weak, reused, old
 * and breached items and one item Mosaic cannot guide; pending an open change with a not-confirmed history
 * entry on GitHub; consent a queue with the breach check off. The mock's breach check finds the passwords
 * password, 123456, hunter2, Sommer2024! and letmein.
 *
 * Sharing (M6b): `&sharing=basic|empty|invites|emergency|request|findings|keyChanged|manager|viewer|
 * rotation` seeds contacts, members, invites and emergency grants (see `docs/DESKTOP_IPC.md` §7) and, unless
 * `&sync=` is given, a hosted sync account. `&sync=folder` or `&sync=none` shows why sharing is off.
 */
import type { EventListener, MosaicIpc } from "./client.ts";
import {
  biometricPlatformKinds,
  errorCodes,
  registrationModes,
  type BiometricPlatformKind,
  type CommandName,
  type ErrorCode,
  type IpcResult,
  type RequestOf,
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
import { applyCredRotationSeed } from "./mock/credentialRotationSeed.ts";
import {
  announceCredRotation,
  credRotationVariants,
  credentialRotationHandlers,
  seedCredRotation,
  type MockCredRotationOptions,
} from "./mock/credentialRotation.ts";
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
import { markerHandlers } from "./mock/rotationMarkers.ts";
import { applyRotationToSync } from "./mock/rotationSync.ts";
import {
  rotationAsyncHandlers,
  rotationHandlers,
  rotationVariants,
  seedRotation,
  type MockRotationOptions,
} from "./mock/rotation.ts";
import { sessionAsyncHandlers, sessionHandlers } from "./mock/session.ts";
import { defaultSettings, settingsHandlers } from "./mock/settings.ts";
import { sshHandlers, startAgent } from "./mock/ssh.ts";
import { sharingHandlers } from "./mock/sharing.ts";
import { announceSharing, seedSharing, sharingVariants, type SharingVariant } from "./mock/sharingData.ts";
import { syncHandlers } from "./mock/sync.ts";
import {
  announceSync,
  seedSync,
  syncVariants,
  type MockFolderAnswer,
  type MockSyncOptions,
} from "./mock/syncData.ts";
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
  /** Sync seeds and answers (`&sync=`, `&syncServer=`, `&syncFolder=`, `&syncFail=`, `&syncJoin2fa=1`). */
  sync?: MockSyncOptions;
  /** Account keys seed (`&rotation=<variant>`, `&rotationFail=<code>`, `&rotationStepMs=<ms>`); implies a backend unless `&sync=` says otherwise. */
  rotation?: MockRotationOptions;
  /** Credential rotation seed (`&credRotation=<variant>`, `&credRotationStepMs=<ms>`). */
  credRotation?: MockCredRotationOptions;
  /** Sharing seed (`&sharing=<variant>`); implies a hosted sync account unless `&sync=` says otherwise. */
  sharing?: SharingVariant;
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
  ...syncHandlers,
  ...rotationHandlers,
  ...markerHandlers,
  ...credentialRotationHandlers,
  ...sharingHandlers,
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
  ...rotationAsyncHandlers,
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

function syncOptionsFromParams(params: URLSearchParams): MockOptions {
  const variant = syncVariants.find((name) => name === params.get("sync"));
  const server = registrationModes.find((mode) => mode === params.get("syncServer"));
  const folderParam = params.get("syncFolder");
  const folder: MockFolderAnswer | undefined =
    folderParam === "empty" || folderParam === "mosaic" || folderParam === "other" || folderParam === "cancel"
      ? folderParam
      : undefined;
  const failParam = params.get("syncFail");
  const failCode = errorCodes.find((code) => code === failParam);
  const sync: MockSyncOptions = {
    ...(variant === undefined ? {} : { variant }),
    ...(server === undefined ? {} : { server }),
    ...(folder === undefined ? {} : { folder }),
    ...(failCode === undefined ? {} : { fail: failCode }),
    ...(params.get("syncJoin2fa") === "1" ? { join2fa: true } : {}),
  };
  return Object.keys(sync).length === 0 ? {} : { sync };
}

/** The backend a rotation variant needs when `&sync=` says nothing: a folder or the server. */
function rotationSync(options: MockOptions): MockSyncOptions {
  const variant = options.rotation?.variant;
  if (variant === undefined) {
    return {};
  }
  const folder = variant === "rekeyPending" ||
    variant === "jumpTooLarge" ||
    variant === "ignoredNotices" ||
    variant === "unknownFile" ||
    variant === "conflict";
  return { variant: folder ? "folder" : "hostedNoTotp" };
}

/** The sync seed of a start: nothing before an account, else the options (or what a rotation variant needs). */
function syncOptionsFor(start: MockStart, options: MockOptions): MockSyncOptions {
  return start === "noAccount" ? { ...options.sync, variant: "none" } : (options.sync ?? rotationSync(options));
}

function rotationOptionsFromParams(params: URLSearchParams): MockOptions {
  const variant = rotationVariants.find((name) => name === params.get("rotation"));
  if (variant === undefined) {
    return {};
  }
  const failParam = params.get("rotationFail");
  const failCode = errorCodes.find((code) => code === failParam);
  const stepParam = params.get("rotationStepMs");
  const step = Number(stepParam ?? "");
  const rotation: MockRotationOptions = {
    variant,
    ...(failCode === undefined ? {} : { fail: failCode }),
    ...(stepParam !== null && Number.isFinite(step) && step >= 0 ? { stepMs: step } : {}),
  };
  return { rotation };
}

function credRotationOptionsFromParams(params: URLSearchParams): MockOptions {
  const variant = credRotationVariants.find((name) => name === params.get("credRotation"));
  if (variant === undefined) {
    return {};
  }
  const stepParam = params.get("credRotationStepMs");
  const step = Number(stepParam ?? "");
  return {
    credRotation: {
      variant,
      ...(stepParam !== null && Number.isFinite(step) && step >= 0 ? { stepMs: step } : {}),
    },
  };
}

function sharingOptionsFromParams(params: URLSearchParams): MockOptions {
  const variant = sharingVariants.find((name) => name === params.get("sharing"));
  if (variant === undefined) {
    return {};
  }
  // Sharing needs the Mosaic server: seed a hosted sync account when none was asked for.
  const sync = params.get("sync") === null ? { sync: { variant: "hosted" as const } } : {};
  return { sharing: variant, ...sync };
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
    ...syncOptionsFromParams(params),
    ...sharingOptionsFromParams(params),
    ...rotationOptionsFromParams(params),
    ...credRotationOptionsFromParams(params),
    ...(bulk > 0 && bulk <= 100_000 ? { bulkItems: bulk } : {}),
    ...quickOptionsFromParams(params),
  };
}

function initialItems(start: MockStart, options: MockOptions): MockItem[] {
  if (start === "noAccount" || options.emptyVault === true) {
    return [];
  }
  // A plain unlocked mock holds one passkey (so the Passkeys category shows); "create" starts
  // with none, and the other variants bring both accounts a sign-in can choose from.
  const passkeys =
    options.browserPasskey === "create"
      ? []
      : options.browserPasskey === undefined
        ? samplePasskeys().slice(0, 1)
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

function seededBrowser(options: MockOptions, now: () => number): MockState["browser"] {
  return {
    ...defaultBrowser(now),
    pairOnStart: options.browserPair === true,
    saveOnStart: options.browserSave ?? null,
    passkeyOnStart: options.browserPasskey ?? null,
  };
}

/** What needs the finished state: the hardware vault, the rotation seeds and the SSH agent. */
function seedAfterSetup(state: MockState, options: MockOptions, start: MockStart): void {
  seedHardwareVault(state, options, start !== "noAccount");
  applyCredRotationSeed(state);
  applyRotationToSync(state, start !== "noAccount" ? options.rotation?.variant : undefined);
  if (start === "unlocked") {
    startAgent(state);
  }
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
    browser: seededBrowser(options, now),
    rotation: seedRotation(options.rotation ?? {}),
    sharing: seedSharing(options.sharing),
    credRotation: seedCredRotation(options.credRotation),
    sync: seedSync(syncOptionsFor(start, options), now()),
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

  seedAfterSetup(state, options, start);

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
      announceSync(state);
      announceSharing(state);
      announceCredRotation(state);
      return Promise.resolve(() => {
        listeners.delete(listener);
      });
    },
  };
}
