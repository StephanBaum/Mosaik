/** Import contract: mirrors `src-tauri/src/contract/import.rs`. */
import type { ItemId, VaultId } from "./common.ts";
import type { ItemKind } from "./items.ts";

export const importFormats = [
  "enpass",
  "bitwarden",
  "keepass",
  "onePassword",
  "cxf",
  "dashlane",
  "keeper",
  "keeperCsv",
  "protonPass",
  "csv",
] as const;
export type ImportFormat = (typeof importFormats)[number];

/** The product an export file comes from, as far as its content tells (`import_detect`). */
export const importDetectedSources = [
  "chromium",
  "firefox",
  "safari",
  "bitwardenCsv",
  "bitwarden",
  "enpass",
  "keepass",
  "keepassxcCsv",
  "onePassword",
  "cxf",
  "lastpass",
  "dashlane",
  "keeper",
  "nordpass",
  "protonPass",
  "roboform",
  "csv",
] as const;
export type ImportDetectedSource = (typeof importDetectedSources)[number];

export const importWarnings = [
  "unmappedFields",
  "attachmentsSkipped",
  "entriesSkipped",
  "otpUnreadable",
  "otpWeak",
  "passkeysNotImported",
  "passkeyUnreadable",
  "passkeyUnsupported",
  "passkeysSkipped",
] as const;
export type ImportWarning = (typeof importWarnings)[number];

export const importEntryWarnings = [
  "unmappedFields",
  "otpUnreadable",
  "otpWeak",
  "attachmentTooLarge",
  "attachmentMissing",
  "tagsDropped",
  "historyTruncated",
  "passkeysNotImported",
  "unknownCategory",
  "fieldsDropped",
  "archived",
  "attachmentQuota",
  "titleCleaned",
  "passkeyUnreadable",
  "passkeyUnsupported",
  "passkeysLimit",
  "notesTruncated",
  "urlKeptAsText",
  "idnHost",
] as const;
export type ImportEntryWarning = (typeof importEntryWarnings)[number];

export const importSkipReasons = ["malformed", "empty", "tooLarge"] as const;
export type ImportSkipReason = (typeof importSkipReasons)[number];

/** A cloud service whose sync folder holds the file. */
export const importSyncedFolders = ["oneDrive", "dropbox", "googleDrive", "icloud", "box"] as const;
export type ImportSyncedFolder = (typeof importSyncedFolders)[number];

export interface ImportDeleteRequest {
  fileToken: string;
}

export interface ImportDetectRequest {
  fileToken: string;
}

/** What the file's content says it is. Both are `null` for a file Mosaic does not read. */
export interface ImportDetected {
  source: ImportDetectedSource | null;
  format: ImportFormat | null;
  /** The cloud service that syncs the file's folder: deleting it does not remove copies already made. */
  syncedFolder: ImportSyncedFolder | null;
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
  /** Everything came across (bar what was left out on purpose): the file may be deleted. */
  clean: boolean;
}
