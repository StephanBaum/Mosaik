/** Import contract: mirrors `src-tauri/src/contract/import.rs`. */
import type { ItemId, VaultId } from "./common.ts";
import type { ItemKind } from "./items.ts";

export const importFormats = ["enpass", "bitwarden", "keepass", "onePassword", "csv"] as const;
export type ImportFormat = (typeof importFormats)[number];

export const importWarnings = [
  "unmappedFields",
  "attachmentsSkipped",
  "entriesSkipped",
  "otpUnreadable",
  "passkeysNotImported",
] as const;
export type ImportWarning = (typeof importWarnings)[number];

export const importEntryWarnings = [
  "unmappedFields",
  "otpUnreadable",
  "attachmentTooLarge",
  "attachmentMissing",
  "tagsDropped",
  "historyTruncated",
  "passkeysNotImported",
  "unknownCategory",
  "fieldsDropped",
  "archived",
  "attachmentQuota",
] as const;
export type ImportEntryWarning = (typeof importEntryWarnings)[number];

export const importSkipReasons = ["malformed", "empty", "tooLarge"] as const;
export type ImportSkipReason = (typeof importSkipReasons)[number];

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
  /** An earlier entry of the same file that looks the same. */
  duplicateInFile: number | null;
  attachments: number;
  warnings: ImportEntryWarning[];
}

export interface ImportSkipped {
  index: number;
  reason: ImportSkipReason;
}

export interface ImportCount {
  kind: ItemKind;
  count: number;
}

export interface ImportPreview {
  previewId: string;
  format: ImportFormat;
  entries: ImportEntry[];
  skipped: ImportSkipped[];
  counts: ImportCount[];
  attachments: number;
  attachmentBytes: number;
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
  failed: number;
  attachments: number;
  attachmentsFailed: number;
}
