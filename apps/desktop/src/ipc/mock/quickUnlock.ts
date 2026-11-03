/**
 * Mock quick unlock (biometric, security keys) and hardware-protected vaults. The rules
 * follow the backend contract, not real devices: biometric unlock succeeds, a security key
 * needs the PIN `1234`, and `?quickFail=<code>` forces `unlock_quick` to fail.
 */
import type {
  BiometricPlatformKind,
  ErrorCode,
  HardwareKeySummary,
  QuickUnlockKind,
  QuickUnlockRequest,
  QuickUnlockStatus,
} from "../contract/index.ts";
import { sampleHardwareVault, sampleKey } from "./data.ts";
import {
  fail,
  requireUnlocked,
  type MockAsyncHandlers,
  type MockHandlers,
  type MockHardwareVault,
  type MockQuick,
  type MockState,
} from "./state.ts";

const MOCK_KEY_PIN = "1234";
const MAX_KEYS = 8;
const DAY_MS = 86_400_000;

/** Mock options for quick unlock (the URL parameters are in `mock.ts`). */
export interface QuickOptions {
  /** Seeded quick-unlock methods; `hardwareKey` also turns on security-key support. */
  quick?: readonly ("biometric" | "hardwareKey")[];
  /** Biometric platform of the mock device; `null`: none (Linux). Default Windows Hello. */
  biometricPlatform?: BiometricPlatformKind | null;
  /** Forces `unlock_quick` to fail with this code. */
  quickFail?: ErrorCode;
  /** Security keys count as supported without a seeded key. */
  keysSupported?: boolean;
  /** Milliseconds a security-key touch takes (default 0). */
  touchMs?: number;
  /** Seconds a hardware vault stays open (default 300). */
  hwOpenSeconds?: number;
}

/** Quick-unlock state of the mock device, as the options seed it. */
export function quickFromOptions(options: QuickOptions, now: () => number): MockQuick {
  const keys = options.quick?.includes("hardwareKey") === true;
  const biometric = options.quick?.includes("biometric") === true;
  return {
    ...defaultQuick(),
    platform: options.biometricPlatform === undefined ? "windowsHello" : options.biometricPlatform,
    keysSupported: options.keysSupported === true || keys,
    biometricEnrolledAt: biometric ? now() : null,
    keys: keys ? [sampleKey(now())] : [],
    fail: options.quickFail ?? null,
    touchMs: options.touchMs ?? 0,
    hwOpenSeconds: options.hwOpenSeconds ?? 300,
  };
}

/** `?quick=hardwareKey` also seeds one closed hardware vault with two one-time codes. */
export function seedHardwareVault(state: MockState, options: QuickOptions, hasAccount: boolean): void {
  if (hasAccount && options.quick?.includes("hardwareKey") === true) {
    const { vault, items, hardware } = sampleHardwareVault(state.now());
    state.vaults.push(vault);
    state.items.push(...items);
    state.hardwareVaults.push(hardware);
  }
}

/** Quick-unlock state of a mock device without any enrolled method. */
export function defaultQuick(): MockQuick {
  return {
    platform: "windowsHello",
    biometricAvailable: true,
    keysSupported: false,
    biometricEnrolledAt: null,
    keys: [],
    // The sample account counts as unlocked with its password since app start.
    passwordUsed: true,
    fail: null,
    touchMs: 0,
    hwOpenSeconds: 300,
    pendingTouch: null,
  };
}

export function quickStatus(state: MockState): QuickUnlockStatus {
  const { quick, settings } = state;
  const enrolledAt = quick.biometricEnrolledAt;
  return {
    biometric: {
      platform: quick.platform,
      available: quick.platform !== null && quick.biometricAvailable,
      enrolled: enrolledAt !== null,
      enrolledAt,
      expiresAt: enrolledAt === null ? null : enrolledAt + settings.quickUnlockMaxAgeDays * DAY_MS,
    },
    hardwareKeys: { supported: quick.keysSupported, keys: quick.keys, maxKeys: MAX_KEYS },
    maxAgeDays: settings.quickUnlockMaxAgeDays,
  };
}

/** The methods the unlock screen offers (empty until a master-password unlock happened). */
export function quickMethods(state: MockState): QuickUnlockKind[] {
  const { quick } = state;
  if (!quick.passwordUsed) {
    return [];
  }
  return [
    ...(quick.biometricEnrolledAt === null ? [] : (["biometric"] as const)),
    ...(quick.keysSupported && quick.keys.length > 0 ? (["hardwareKey"] as const) : []),
  ];
}

export function newKey(state: MockState): HardwareKeySummary {
  const hex = state.nextId().replaceAll("-", "").slice(0, 32).padEnd(32, "0");
  return { methodId: hex, addedAt: state.now() };
}

function checkPassword(state: MockState, password: string | null): void {
  if (password === null) {
    fail("reauthRequired");
  }
  if (password !== state.account?.password) {
    fail("wrongCredentials");
  }
}

export function verifyPin(pin: string | null): void {
  if (pin === null) {
    fail("hardwareKeyPinRequired");
  }
  if (pin !== MOCK_KEY_PIN) {
    fail("hardwareKeyPinInvalid");
  }
}

/** Waits for the "touch"; `hardware_key_cancel` rejects it with `cancelled`. */
function touch(state: MockState): Promise<void> {
  const { quick } = state;
  return new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => {
      quick.pendingTouch = null;
      resolve();
    }, quick.touchMs);
    quick.pendingTouch = {
      cancel: () => {
        clearTimeout(timer);
        quick.pendingTouch = null;
        reject(new MockCancelled());
      },
    };
  });
}

class MockCancelled extends Error {}

export async function verifyTouch(state: MockState): Promise<void> {
  try {
    await touch(state);
  } catch (error) {
    if (error instanceof MockCancelled) {
      fail("cancelled");
    }
    throw error;
  }
}

function dropForFailure(state: MockState, code: ErrorCode, method: QuickUnlockKind): void {
  if (code === "quickUnlockExpired" || code === "quickUnlockInvalidated") {
    if (method === "biometric") {
      state.quick.biometricEnrolledAt = null;
    } else {
      state.quick.keys = [];
    }
  } else if (code === "passwordRequired") {
    state.quick.passwordUsed = false;
  }
}

export function closeHardwareVault(
  state: MockState,
  vault: MockHardwareVault,
  reason: "timer" | "lock",
): void {
  if (!vault.open) {
    return;
  }
  clearTimeout(vault.timer);
  vault.open = false;
  vault.openUntil = null;
  vault.timer = undefined;
  if (reason === "timer") {
    state.emit({ type: "hardwareVaultClosed", vaultId: vault.vaultId });
  }
}

/** The checks of `unlock_quick`; resolves when the device agreed to unlock. */
export async function verifyQuickUnlock(
  state: MockState,
  request: QuickUnlockRequest,
): Promise<void> {
  const { quick } = state;
  if (state.account === null) {
    fail("noAccount");
  }
  if (state.unlocked) {
    fail("alreadyUnlocked");
  }
  if (!quickMethods(state).includes(request.method)) {
    fail("quickUnlockUnavailable");
  }
  if (quick.fail !== null && !(request.method === "hardwareKey" && request.pin === null)) {
    dropForFailure(state, quick.fail, request.method);
    fail(quick.fail);
  }
  if (request.method === "hardwareKey") {
    verifyPin(request.pin);
    await verifyTouch(state);
  }
}

export const quickUnlockHandlers: MockHandlers = {
  quick_unlock_status: (state) => {
    requireUnlocked(state);
    return quickStatus(state);
  },

  enroll_biometric: (state, request) => {
    requireUnlocked(state);
    if (state.quick.platform === null || !state.quick.biometricAvailable) {
      fail("quickUnlockUnavailable");
    }
    checkPassword(state, request.password);
    state.quick.biometricEnrolledAt = state.now();
    return quickStatus(state);
  },

  remove_biometric: (state) => {
    requireUnlocked(state);
    state.quick.biometricEnrolledAt = null;
    return quickStatus(state);
  },

  remove_hardware_key: (state, request) => {
    requireUnlocked(state);
    if (!state.quick.keys.some((key) => key.methodId === request.methodId)) {
      fail("hardwareKeyNotRecognized");
    }
    state.quick.keys = state.quick.keys.filter((key) => key.methodId !== request.methodId);
    return quickStatus(state);
  },

  hardware_key_cancel: (state) => {
    state.quick.pendingTouch?.cancel();
    return null;
  },
};

export const quickUnlockAsyncHandlers: MockAsyncHandlers = {
  enroll_hardware_key: async (state, request) => {
    requireUnlocked(state);
    if (!state.quick.keysSupported) {
      fail("hardwareKeyUnsupported");
    }
    checkPassword(state, request.password);
    if (state.quick.keys.length >= MAX_KEYS) {
      fail("hardwareKeyLimit");
    }
    verifyPin(request.pin);
    await verifyTouch(state);
    state.quick.keys = [...state.quick.keys, newKey(state)];
    return quickStatus(state);
  },
};
