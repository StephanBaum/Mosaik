/**
 * The mock's sync state and the `?sync=<variant>` seeds (see the header of `mock.ts`). The
 * mock "server" accepts the sample account's master password and the one-time code
 * `123456`; the invite code it accepts is `MOCK-INVITE`. In the `passwordChanged` variant the
 * server holds another master password (`MOCK_NEW_PASSWORD`): signing in with the one this
 * device knows answers `syncPasswordChanged`, and `sync_recover_password` takes the new one.
 */
import type {
  ErrorCode,
  FolderChoice,
  FolderState,
  RegistrationMode,
  RevokedReason,
  SyncConfigView,
  SyncDevice,
  SyncOverview,
  SyncState,
  SyncStatus,
  SyncWarningView,
  WebauthnKeyView,
} from "../contract/index.ts";
import { sampleAccount } from "./data.ts";
import { fail, type MockState } from "./state.ts";

export const MOCK_TOTP_CODE = "123456";
export const MOCK_INVITE_CODE = "MOCK-INVITE";
export const MOCK_NEW_PASSWORD = "new correct horse battery staple";
export const MOCK_SERVER_URL = "https://sync.example.com";
export const MOCK_FOLDER_PATH = "C:\\Users\\alex\\Dropbox\\Mosaic";

export const syncVariants = [
  "none",
  "hosted",
  "hostedNoTotp",
  "folder",
  "syncing",
  "offline",
  "error",
  "signedOut",
  "passwordChanged",
  "rollback",
  "identity",
  "serverInconsistent",
  "serverRollback",
  "noCommonView",
  "freeze",
  "missingFiles",
  "undecryptable",
  "hwLocal",
  "notRepaired",
  "allWarnings",
] as const;
export type SyncVariant = (typeof syncVariants)[number];

export type MockFolderAnswer = FolderState | "cancel";

/** The sync account's security keys (`&syncWebauthn=`): none, next to a code, or instead of one. */
export type MockWebauthnMode = "none" | "both" | "only";

/** How a browser handoff ends (`&syncHandoff=`): by default the user finishes it. */
export type MockHandoffEnd = "done" | "expired" | "failed";

/** `&syncWebauthn=both|only` and `&syncHandoff=expired|failed` of the mock URL. */
export function webauthnOptionsFromParams(params: URLSearchParams): MockSyncOptions {
  const mode = params.get("syncWebauthn");
  const end = params.get("syncHandoff");
  return {
    ...(mode === "both" || mode === "only" ? { webauthn: mode } : {}),
    ...(end === "expired" || end === "failed" ? { handoff: end } : {}),
  };
}

/** The sync account has a second factor: a code, a security key, or both. */
export function hasSecondFactor(sync: { totp: boolean; keys: readonly unknown[] }): boolean {
  return sync.totp || sync.keys.length > 0;
}

/** What the seed gives the account: its keys and whether it has a code. */
function seedFactors(variant: SyncVariant, wanted: MockWebauthnMode | undefined, now: number) {
  const hosted = variant !== "none" && variant !== "folder";
  const mode = wanted ?? "none";
  return {
    mode,
    keys: hosted ? seedKeys(mode, now) : [],
    totp: hosted && variant !== "hostedNoTotp" && mode !== "only",
  };
}

/** The URL options of the sync mock (`&syncServer=`, `&syncFolder=`, `&syncFail=`, `&syncJoin2fa=1`). */
export interface MockSyncOptions {
  variant?: SyncVariant;
  server?: RegistrationMode;
  folder?: MockFolderAnswer;
  fail?: ErrorCode;
  join2fa?: boolean;
  /** Security keys on the sync account (M9). */
  webauthn?: MockWebauthnMode;
  /** How the browser step ends (M9). */
  handoff?: MockHandoffEnd;
  /** Why the server ended the session of a `signedOut` start (M9, `&syncRevoked=`). */
  revoked?: RevokedReason;
}

/** A join or connect that waits for the second-factor code. */
export type PendingTwoFactor =
  | { kind: "connect"; serverUrl: string; deviceName: string }
  | { kind: "join"; serverUrl: string; deviceName: string };

/** A browser handoff that runs (M9): the second poll finds it finished. */
export interface MockHandoff {
  /** What it is for: a login (connect or join), a new key or a confirm. */
  step: "login" | "register" | "confirm";
  polls: number;
  /** The account flow behind a login step. */
  then: "register" | "confirm" | null;
  label: string | null;
}

export interface MockSync {
  config: SyncConfigView | null;
  status: SyncStatus;
  error: ErrorCode | null;
  lastSyncedAt: number | null;
  warnings: SyncWarningView[];
  devices: SyncDevice[];
  /** The mock server knows a sync account for the sample email. */
  registered: boolean;
  /** The sync account has a second factor. */
  totp: boolean;
  /** The master password the server holds when it is not the device's (`null`: the same). */
  serverPassword: string | null;
  /** The device found out (or the backend says) that its master password is old. */
  passwordChanged: boolean;
  /** `sync_totp_begin` ran and waits for `sync_totp_confirm`. */
  totpPending: boolean;
  pendingTwoFactor: PendingTwoFactor | null;
  /** Folder choices by ID (the webview never sends a path). */
  choices: Map<string, FolderChoice>;
  server: RegistrationMode;
  folder: MockFolderAnswer;
  fail: ErrorCode | null;
  join2fa: boolean;
  /** The sync account's security keys. */
  keys: WebauthnKeyView[];
  webauthnMode: MockWebauthnMode;
  handoffEnd: MockHandoffEnd;
  handoff: MockHandoff | null;
  /** A security-key confirm lasts until then (ms). */
  confirmedUntil: number;
  /** Why the server ended the session; shown while the status is `signedOut` (M9). */
  revoked: RevokedReason | null;
}

const HOUR = 3_600_000;

function warningsFor(variant: SyncVariant, now: number): SyncWarningView[] {
  const day = 24 * HOUR;
  const all: Record<string, SyncWarningView> = {
    rollback: { kind: "rollbackDetected" },
    identity: { kind: "identityDuplicated" },
    serverInconsistent: { kind: "serverInconsistent", vaultName: "Personal" },
    serverRollback: { kind: "serverRollback", vaultName: "Work" },
    noCommonView: { kind: "noCommonView", deviceName: "Mosaic on macOS", since: now - 2 * day },
    freeze: { kind: "freezeSuspected", deviceName: "Mosaic on macOS", since: now - 30 * HOUR },
    missingFiles: { kind: "missingFiles", deviceName: "Mosaic on Linux", since: now - 3 * HOUR },
    undecryptable: { kind: "undecryptable", count: 3 },
    hwLocal: { kind: "hardwareVaultsLocal", count: 1 },
    notRepaired: { kind: "attachmentsNotRepaired", count: 2 },
  };
  if (variant === "allWarnings") {
    return Object.values(all);
  }
  const one = all[variant];
  return one === undefined ? [] : [one];
}

function hostedConfig(totpEnabled: boolean, webauthnKeys: number): SyncConfigView {
  return {
    kind: "hosted",
    serverUrl: MOCK_SERVER_URL,
    email: sampleAccount.email,
    deviceName: "Mosaic on Windows",
    totpEnabled,
    webauthnKeys,
  };
}

/** The keys the `&syncWebauthn=` seeds start with. */
export function seedKeys(mode: MockWebauthnMode, now: number): WebauthnKeyView[] {
  return mode === "none"
    ? []
    : [
        {
          credentialRef: "00112233445566778899aabbccddeeff",
          label: "YubiKey 5C",
          createdAt: now - 30 * 24 * HOUR,
          lastUsedAt: now - 2 * HOUR,
        },
      ];
}

/** The devices of the sync account; names are `null` where the backend has none. */
export function seedDevices(now: number, deviceName: string, removable: boolean): SyncDevice[] {
  return [
    {
      deviceId: "019a2b3c-4d5e-7040-8000-000000000001",
      name: deviceName,
      platform: "windows",
      current: true,
      lastSeenAt: now,
      canRemove: false,
    },
    {
      deviceId: "019a2b3c-4d5e-7040-8000-000000000002",
      name: "Mosaic on macOS",
      platform: null,
      current: false,
      lastSeenAt: now - 2 * HOUR,
      canRemove: removable,
    },
    {
      deviceId: "019a2b3c-4d5e-7040-8000-000000000003",
      name: null,
      platform: null,
      current: false,
      lastSeenAt: null,
      canRemove: removable,
    },
  ];
}

/** Variants whose status is not "up to date" (the halts are `error` too). */
const STATUS_OF: Partial<Record<SyncVariant, SyncStatus>> = {
  none: "notConfigured",
  syncing: "syncing",
  offline: "offline",
  signedOut: "signedOut",
  passwordChanged: "signedOut",
  error: "error",
  rollback: "error",
  identity: "error",
  allWarnings: "error",
};

const ERROR_OF: Partial<Record<SyncVariant, ErrorCode>> = {
  error: "syncServerUnreachable",
  offline: "syncServerUnreachable",
  signedOut: "syncSignedOut",
  passwordChanged: "syncSignedOut",
};

function configFor(variant: SyncVariant, totp: boolean, keys: number): SyncConfigView | null {
  if (variant === "none") {
    return null;
  }
  return variant === "folder"
    ? { kind: "folder", displayPath: MOCK_FOLDER_PATH, deviceName: "Mosaic on Windows" }
    : hostedConfig(totp, keys);
}

const revokedOf = (options: MockSyncOptions): RevokedReason | null => options.revoked ?? null;

export function seedSync(options: MockSyncOptions, now: number): MockSync {
  const variant = options.variant ?? "none";
  const folder = variant === "folder";
  const connected = variant !== "none";
  const { mode: webauthnMode, keys, totp } = seedFactors(variant, options.webauthn, now);
  return {
    config: configFor(variant, totp, keys.length),
    status: STATUS_OF[variant] ?? "upToDate",
    error: ERROR_OF[variant] ?? null,
    lastSyncedAt: connected ? now - 5 * 60_000 : null,
    warnings: warningsFor(variant, now),
    devices: seedDevices(now, "Mosaic on Windows", !folder),
    registered: connected && !folder,
    totp,
    serverPassword: variant === "passwordChanged" ? MOCK_NEW_PASSWORD : null,
    passwordChanged: variant === "passwordChanged",
    totpPending: false,
    pendingTwoFactor: null,
    choices: new Map(),
    server: options.server ?? "open",
    folder: options.folder ?? "empty",
    fail: options.fail ?? null,
    join2fa: options.join2fa === true || webauthnMode !== "none",
    keys,
    webauthnMode,
    handoffEnd: options.handoff ?? "done",
    handoff: null,
    confirmedUntil: 0,
    revoked: revokedOf(options),
  };
}

export function syncState(sync: MockSync): SyncState {
  return { status: sync.status, lastSyncedAt: sync.lastSyncedAt, error: sync.error };
}

export function overview(state: MockState): SyncOverview {
  const { sync } = state;
  return {
    config: sync.config,
    state: syncState(sync),
    warnings: sync.warnings,
    sharingFindings: state.sharing.findings,
    passwordChanged: sync.passwordChanged,
    rekeyPending: state.rotation.rekeyPending,
    revoked: sync.status === "signedOut" ? sync.revoked : null,
  };
}

/** Tells the app the current sync state (after an unlock or a subscription). */
export function announceSync(state: MockState): void {
  if (!state.unlocked || state.sync.config === null) {
    return;
  }
  setTimeout(() => {
    state.emit({ type: "syncStateChanged", state: syncState(state.sync) });
  }, 0);
}

/** Emits the state now (after a command changed it) and returns the overview. */
export function publish(state: MockState): SyncOverview {
  state.emit({ type: "syncStateChanged", state: syncState(state.sync) });
  return overview(state);
}

/** `https://host` (or `http` on this computer), without a trailing slash. */
export function normalizeServer(raw: string): string {
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return fail("syncServerInvalid");
  }
  const local = url.hostname === "localhost" || url.hostname === "127.0.0.1";
  if (url.protocol === "http:" && !local) {
    return fail("syncServerInsecure");
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") {
    return fail("syncServerInvalid");
  }
  return url.origin;
}


/** The backend a connect or a join sets up. */
export type Backend =
  | { kind: "hosted"; serverUrl: string; deviceName: string }
  | { kind: "folder"; path: string; deviceName: string };

/** Connects the mock to a server or folder: the state after a connect or a join. */
export function settle(state: MockState, backend: Backend): SyncOverview {
  const { sync } = state;
  sync.config =
    backend.kind === "hosted"
      ? {
          kind: "hosted",
          serverUrl: backend.serverUrl,
          email: state.account?.email ?? sampleAccount.email,
          deviceName: backend.deviceName,
          totpEnabled: sync.totp,
          webauthnKeys: sync.keys.length,
        }
      : { kind: "folder", displayPath: backend.path, deviceName: backend.deviceName };
  sync.status = "upToDate";
  sync.error = null;
  sync.warnings = [];
  sync.lastSyncedAt = state.now();
  sync.pendingTwoFactor = null;
  sync.serverPassword = null;
  sync.passwordChanged = false;
  sync.devices = seedDevices(state.now(), backend.deviceName, backend.kind === "hosted");
  return publish(state);
}
