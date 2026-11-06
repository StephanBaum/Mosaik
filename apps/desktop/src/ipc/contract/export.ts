/**
 * Export (docs/DESKTOP_IPC.md): mirrors `src-tauri/src/contract/export.rs`. Rust opens the save
 * dialog and writes the file; the webview sends a vault ID and the master password.
 */
import type { VaultId } from "./common.ts";

export interface ExportPasskeysRequest {
  vaultId: VaultId;
  /** The master password, or `null` within two minutes of a right one. A success spends it. */
  password: string | null;
}

/** `export_passkeys_cxf`: the file was written, or the user cancelled the dialog. */
export type PasskeysExported = { kind: "saved"; count: number } | { kind: "cancelled" };
