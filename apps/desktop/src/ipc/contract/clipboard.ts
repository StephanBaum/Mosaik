/** Clipboard contract: mirrors `src-tauri/src/contract/clipboard.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";
import type { SecretFieldKey, TextFieldKey } from "./items.ts";

export type CopyField =
  | { kind: "username" }
  | { kind: "password" }
  | { kind: "totp" }
  | { kind: "text"; field: TextFieldKey }
  | { kind: "secret"; field: SecretFieldKey }
  | { kind: "customField"; elementId: string };

export type CopySource =
  | { kind: "item"; vaultId: VaultId; itemId: ItemId; field: CopyField }
  /** The password `generate_password` returned last. */
  | { kind: "generated" };

export interface CopyRequest {
  source: CopySource;
}

/** For the "Copied, clears in 30 seconds" toast; `null`: auto-clear is off. */
export interface CopyReceipt {
  clearsAt: UnixMillis | null;
  clearAfterSeconds: number | null;
}
