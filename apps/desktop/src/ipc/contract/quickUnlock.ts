/** Quick-unlock contract: mirrors `src-tauri/src/contract/quick_unlock.rs`. */
import type { UnixMillis } from "./common.ts";

export const biometricPlatformKinds = ["windowsHello", "touchId"] as const;
export type BiometricPlatformKind = (typeof biometricPlatformKinds)[number];

export interface BiometricStatus {
  /** `null` where the OS has no biometric prompt (Linux). */
  platform: BiometricPlatformKind | null;
  available: boolean;
  enrolled: boolean;
  enrolledAt: UnixMillis | null;
  /** When the master password is asked for again. */
  expiresAt: UnixMillis | null;
}

export interface HardwareKeySummary {
  /** 32 lowercase hex characters; the UI shows the first 8 as a tag. */
  methodId: string;
  addedAt: UnixMillis;
}

export interface HardwareKeyStatus {
  /** `false` on every OS until the FIDO2 transport lands; hide all key UI then. */
  supported: boolean;
  keys: HardwareKeySummary[];
  maxKeys: number;
}

export interface QuickUnlockStatus {
  biometric: BiometricStatus;
  hardwareKeys: HardwareKeyStatus;
  maxAgeDays: number;
}

/** Master password for re-authentication, or `null` while the session was just unlocked. */
export interface EnrollBiometricRequest {
  password: string | null;
}

export interface EnrollHardwareKeyRequest {
  password: string | null;
  pin: string | null;
}

export interface RemoveHardwareKeyRequest {
  methodId: string;
}

export interface HardwareVaultKeys {
  keys: HardwareKeySummary[];
}
