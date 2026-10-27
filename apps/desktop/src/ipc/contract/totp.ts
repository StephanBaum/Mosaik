/** One-time code contract: mirrors `src-tauri/src/contract/totp.rs`. */

export interface TotpCode {
  code: string;
  /** `null` for HOTP. */
  secondsRemaining: number | null;
  period: number | null;
}
