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
  | {
      state: "unlocked";
      accountId: AccountId;
      email: string | null;
      /** The Emergency Kit check passed; while false the UI shows the kit step again. */
      kitVerified: boolean;
    };

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

/** The kit's texts in the user's language (`shell.emergencyKit.*`). */
export interface KitLabels {
  title: string;
  intro: string;
  email: string;
  accountId: string;
  signInUrl: string;
  signInUrlValue: string;
  secretKey: string;
  masterPassword: string;
  masterPasswordHint: string;
  footer: string;
}

/** `save_emergency_kit`: Rust shows the save dialog and writes the PDF. */
export interface SaveKitRequest {
  labels: KitLabels;
  /** Needed once the kit was verified or the vault locked since account creation. */
  password: string | null;
}

export type KitSaved = { kind: "saved" } | { kind: "cancelled" };

export const shortcutStates = ["registered", "conflict", "unavailable"] as const;
export type ShortcutState = (typeof shortcutStates)[number];

export interface AppStatus {
  shortcut: ShortcutState;
  /** Whether the app is told when the screen locks (CRYPTO_SPEC section 11 gap). */
  screenLockDetection: boolean;
}

export interface QuickUnlockRequest {
  method: QuickUnlockKind;
}
