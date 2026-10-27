/** Import contract: mirrors `src-tauri/src/contract/import.rs`. */
import type { ItemId, VaultId } from "./common.ts";
import type { ItemKind } from "./items.ts";

export const importFormats = ["enpass", "bitwarden", "keepass", "onePassword", "csv"] as const;
export type ImportFormat = (typeof importFormats)[number];

export const importWarnings = ["unmappedFields", "attachmentsSkipped", "entriesSkipped"] as const;
export type ImportWarning = (typeof importWarnings)[number];

export interface ImportChooseRequest {
  format: ImportFormat;
}

export type ImportFileChoice =
  | { kind: "chosen"; fileToken: string; fileName: string }
  | { kind: "cancelled" };

export interface ImportPreviewRequest {
  fileToken: string;
  format: ImportFormat;
  password: string | null;
}

export interface ImportDuplicate {
  vaultId: VaultId;
  itemId: ItemId;
}

export interface ImportEntry {
  index: number;
  kind: ItemKind;
  title: string | null;
  subtitle: string | null;
  duplicateOf: ImportDuplicate | null;
}

export interface ImportPreview {
  previewId: string;
  entries: ImportEntry[];
  warnings: ImportWarning[];
}

export interface ImportCommitRequest {
  previewId: string;
  targetVaultId: VaultId;
  skip: number[];
}

export interface ImportCancelRequest {
  previewId: string;
}

export interface ImportResult {
  imported: number;
  skipped: number;
}
