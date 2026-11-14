/**
 * State and helpers shared by the mock IPC handlers (one file per contract area, so the
 * tracks extend their own file). The mock follows the contract, not the real crypto: it
 * stores sample data in memory and never touches a real vault.
 */
import type {
  AppEvent,
  BiometricPlatformKind,
  CommandName,
  ErrorCode,
  HardwareKeySummary,
  ItemDetail,
  OtpPending,
  RequestOf,
  ResponseOf,
  SecretFieldKey,
  Settings,
  ShortcutState,
  VaultMetaField,
  VaultSummary,
  ReauthPurpose,
} from "../contract/index.ts";
import type { MockBrowser } from "./browser.ts";
import type { MockCredRotation } from "./credentialRotation.ts";
import type { MockRotation } from "./rotation.ts";
import type { MockSharing } from "./sharingData.ts";
import type { MockSync } from "./syncData.ts";

export interface MockAccount {
  accountId: string;
  email: string;
  password: string;
  secretKey: string;
}

export interface MockItem {
  detail: ItemDetail;
  secrets: Partial<Record<SecretFieldKey, string>>;
  history: string[];
  /** Hidden custom-field values by element ID (the detail never carries them). */
  customSecrets: Record<string, string>;
  /** The pasted private key of an SSH key item; it never leaves the mock either. */
  sshKey: { privateKey: string } | null;
}

/** A pending security-key touch that `hardware_key_cancel` can reject. */
export interface PendingTouch {
  cancel: () => void;
}

/** Quick-unlock state of the mock (`mock/quickUnlock.ts`). */
export interface MockQuick {
  platform: BiometricPlatformKind | null;
  biometricAvailable: boolean;
  /** Whether security keys count as supported (the real backend says no on every OS). */
  keysSupported: boolean;
  biometricEnrolledAt: number | null;
  keys: HardwareKeySummary[];
  /** Quick unlock is offered only after a master-password unlock since app start. */
  passwordUsed: boolean;
  /** Forces `unlock_quick` to fail with this code (`&quickFail=<code>`). */
  fail: ErrorCode | null;
  /** How long a security-key touch takes. */
  touchMs: number;
  /** Seconds a hardware vault stays open. */
  hwOpenSeconds: number;
  pendingTouch: PendingTouch | null;
  /** The master-password proof of `reauthenticate` is valid until this time (0: none). */
  reauthUntil: number;
  /** What the proof of `reauthenticate` was typed for; it pays for that action only. */
  reauthPurpose: ReauthPurpose | null;
}

/** The mock's hardware-protected vault: closed until opened with a key and the PIN. */
export interface MockHardwareVault {
  vaultId: string;
  name: string;
  open: boolean;
  openUntil: number | null;
  keys: HardwareKeySummary[];
  timer: ReturnType<typeof setTimeout> | undefined;
}

/** A mock attachment: no bytes, only what the mock needs to answer a preview. */
export interface MockAttachment {
  kind: "png" | "pdf" | "other";
  bytes: number;
}

/**
 * URL options of the one-time code setup (`&otpScan=<code>|cancel`, `&screenScan=0`) and of
 * attachments (`&attachAdd=<code>|cancel`, `&attachSave=<code>|cancel`).
 */
export interface MockSetupOptions {
  /** Makes `attachment_add` fail with this code (or cancel the file dialog). */
  attachAdd: ErrorCode | "cancel" | null;
  /** Makes `attachment_save` fail with this code (or cancel the save dialog). */
  attachSave: ErrorCode | "cancel" | null;
  /** Makes `otp_scan` fail with this code (or cancel the file dialog). */
  otpScan: ErrorCode | "cancel" | null;
  /** Whether "Scan screen" is supported. */
  screenScan: boolean;
  /** What `app_status` says of the global shortcut (`&shortcut=conflict|unsupported|unavailable`). */
  shortcut: ShortcutState;
  /** A sync run reports these metadata fields as dropped 500 ms after subscribing (`&metaDropped=name,description`). */
  metaDropped: VaultMetaField[];
}

export interface MockState {
  account: MockAccount | null;
  secretKeyStored: boolean;
  unlocked: boolean;
  /** The Emergency Kit check passed. */
  kitVerified: boolean;
  sshEnabled: boolean;
  /** The browser extension endpoint (`mock/browser.ts`). */
  browser: MockBrowser;
  /** Sync (`mock/sync.ts`): the backend, its status, warnings and devices. */
  sync: MockSync;
  /** Account keys (`mock/rotation.ts`): the rotation job, the staged Secret Key and the review. */
  rotation: MockRotation;
  /** Credential rotation (`mock/credentialRotation.ts`): snooze, dismissal, open attempts and the breach scan. */
  credRotation: MockCredRotation;
  /** Sharing (`mock/sharing.ts`): contacts, members, invites and emergency access. */
  sharing: MockSharing;
  /** The import preview waiting for a commit. */
  heldImport: { previewId: string; entries: number[] } | null;
  vaults: VaultSummary[];
  items: MockItem[];
  settings: Settings;
  quick: MockQuick;
  hardwareVaults: MockHardwareVault[];
  lastGenerated: string | null;
  /** Parsed one-time code setups waiting for confirmation (ADR-0031). */
  otpPending: Map<string, OtpPending>;
  otpPendingCounter: number;
  /** Attachments by ID (the item details hold the metadata the UI sees). */
  attachments: Map<string, MockAttachment>;
  options: MockSetupOptions;
  /** Whether the mock agent asks for an approval after unlock (`?sshRequest=1`). */
  sshRequest: boolean;
  emit: (event: AppEvent) => void;
  now: () => number;
  nextId: () => string;
}

/** A handler's way to fail a command with a contract error code. */
export class MockFailure extends Error {
  constructor(readonly code: ErrorCode) {
    super(code);
    this.name = "MockFailure";
  }
}

export function fail(code: ErrorCode): never {
  throw new MockFailure(code);
}

export function requireUnlocked(state: MockState): void {
  if (state.account === null) {
    fail("noAccount");
  }
  if (!state.unlocked) {
    fail("locked");
  }
}

export type MockHandler<K extends CommandName> = (
  state: MockState,
  request: RequestOf<K>,
) => ResponseOf<K>;

/** For commands that wait on a device (a security-key touch). */
export type MockAsyncHandler<K extends CommandName> = (
  state: MockState,
  request: RequestOf<K>,
) => Promise<ResponseOf<K>>;

export type MockHandlers = { [K in CommandName]?: MockHandler<K> };
export type MockAsyncHandlers = { [K in CommandName]?: MockAsyncHandler<K> };

/** Random values for the mock's Secret Keys and passwords (never `Math.random`). */
export function randomIndex(limit: number): number {
  const bound = Math.floor(0x1_0000_0000 / limit) * limit;
  const buffer = new Uint32Array(1);
  for (;;) {
    crypto.getRandomValues(buffer);
    const value = buffer[0] ?? 0;
    if (value < bound) {
      return value % limit;
    }
  }
}
