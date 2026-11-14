/**
 * The setup code for a phone (docs/DESKTOP_IPC.md, ADR-0096): mirrors
 * `src-tauri/src/contract/setup_code.rs`. Rust builds and renders the QR code; the webview only draws
 * the module matrix, and hides it again after 60 seconds, on blur and on lock.
 */

/** `setup_code_show`. */
export interface SetupCodeShowRequest {
  /** The master password, checked now. */
  password: string;
}

export interface SetupCodeShown {
  /** `modules[y][x]` is `true` for a dark module. No quiet zone: the webview adds one. */
  modules: boolean[][];
  /** The side length in modules. */
  size: number;
  /** When the code must be hidden (Unix milliseconds, 60 s after it was made). */
  expiresAt: number;
}
