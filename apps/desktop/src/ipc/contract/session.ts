/** Session contract: mirrors `src-tauri/src/contract/session.rs`. */
import type { AccountId } from "./common.ts";

export const quickUnlockKinds = ["biometric", "hardwareKey"] as const;
export type QuickUnlockKind = (typeof quickUnlockKinds)[number];

/** Where the session is; every session command returns the new state. */
export type SessionState =
  | { state: "noAccount" }
  | {
      state: "locked";
      email: string | null;
      /** Whether this device holds the Secret Key; if not, the unlock screen asks for it. */
      secretKeyStored: boolean;
      quickUnlock: QuickUnlockKind[];
    }
  | { state: "unlocked"; accountId: AccountId; email: string | null };

export interface CreateAccountRequest {
  email: string;
  password: string;
  defaultVaultName: string;
}

/** Shown once (CRYPTO_SPEC §5.2). `secretKey` is a secret: never log or persist it. */
export interface EmergencyKit {
  accountId: AccountId;
  email: string;
  secretKey: string;
}

export interface CreatedAccount {
  emergencyKit: EmergencyKit;
  session: SessionState;
}

export interface VerifyKitRequest {
  lastGroup: string;
}

export interface KitVerification {
  matches: boolean;
}

export interface UnlockRequest {
  password: string;
  /** The Secret Key from the Emergency Kit, or `null` to use the one stored on this device. */
  secretKey: string | null;
}

export interface QuickUnlockRequest {
  method: QuickUnlockKind;
}
