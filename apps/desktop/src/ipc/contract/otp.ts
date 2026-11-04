/** One-time-code setup contract: mirrors `src-tauri/src/contract/otp.rs`. */
import type { OtpScheme } from "./items.ts";

export const otpScanSources = ["screen", "clipboard", "file"] as const;
export type OtpScanSource = (typeof otpScanSources)[number];
export const otpAlgorithms = ["sha1", "sha256", "sha512"] as const;
export type OtpAlgorithm = (typeof otpAlgorithms)[number];
export const otpSecretStrengths = ["weak", "belowRfc4226", "rfc4226"] as const;
export type OtpSecretStrength = (typeof otpSecretStrengths)[number];

export interface OtpScanRequest {
  source: OtpScanSource;
}

/** A parsed setup waiting for confirmation; the secret stays in Rust behind `pendingId`. */
export interface OtpPending {
  pendingId: string;
  scheme: OtpScheme;
  issuer: string | null;
  /** `null` when empty. */
  accountName: string | null;
  algorithm: OtpAlgorithm;
  /** 5 for Steam. */
  digits: number;
  /** `null` for HOTP. */
  period: number | null;
  strength: OtpSecretStrength;
  expiresInSeconds: number;
}

export type OtpScanResult =
  | { kind: "found"; pending: OtpPending; clipboardCleared: boolean }
  /** The file dialog was dismissed. */
  | { kind: "cancelled" };

export type OtpParseRequest =
  | { kind: "uri"; uri: string }
  | {
      kind: "key";
      secret: string;
      issuer: string | null;
      accountName: string | null;
      algorithm: OtpAlgorithm;
      digits: number;
      period: number;
    };

export interface OtpPendingRequest {
  pendingId: string;
}
