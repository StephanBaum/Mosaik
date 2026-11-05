/**
 * Attachments (docs/DESKTOP_IPC.md §3). Mirrors `src-tauri/src/contract/attachments.rs`.
 * Requests carry IDs only: files are chosen and written through native dialogs that Rust
 * opens, so a path never crosses the boundary. The preview answer is binary.
 */
import type { AttachmentId, ItemId, VaultId } from "./common.ts";
import type { ItemDetail } from "./items.ts";

export interface AttachmentRequest {
  vaultId: VaultId;
  itemId: ItemId;
  attachmentId: AttachmentId;
}

/** `attachment_add`: the file is part of the item, or the user cancelled the dialog. */
export type AttachmentAdded = { kind: "added"; item: ItemDetail } | { kind: "cancelled" };

/** `attachment_save`: the file was written, or the user cancelled the dialog. */
export type AttachmentSaved = { kind: "saved" } | { kind: "cancelled" };
