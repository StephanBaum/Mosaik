/**
 * The places a user can bring passwords from: which export to make, which file Mosaic expects
 * and what the export leaves behind. Names are plain text (no third-party logos).
 */
import type { ImportDetected, ImportDetectedSource, ImportFormat } from "../../ipc/contract/index.ts";

export type SourceId =
  | "chrome"
  | "edge"
  | "brave"
  | "opera"
  | "vivaldi"
  | "arc"
  | "firefox"
  | "safari"
  | "applePasswords"
  | "onePassword"
  | "bitwarden"
  | "keepass"
  | "keepassxcCsv"
  | "enpass"
  | "lastpass"
  | "dashlane"
  | "keeper"
  | "nordpass"
  | "protonPass"
  | "roboform"
  | "otherFile";
export type LostKey = "passkeys" | "attachments" | "history" | "roboformExtras";
export type ChecklistKey = "browserSaving" | "firefoxSaving" | "safariSaving" | "oldExtension" | "deleteOld";

export interface ImportSourceInfo {
  id: SourceId;
  group: "browser" | "manager" | "other";
  /** The guide has the i18n keys `import.guide.<id>.1` to `.<steps>`. */
  steps: number;
  /** An address the user pastes into the product (shown with a copy button). */
  internalUrl: string | null;
  /** The detected sources this product exports. Empty: anything. */
  detects: readonly ImportDetectedSource[];
  lost: readonly LostKey[];
  checklist: readonly ChecklistKey[];
}

const chromium = (id: SourceId, scheme: string): ImportSourceInfo => ({
  id,
  group: "browser",
  steps: 3,
  internalUrl: `${scheme}://password-manager/settings`,
  detects: ["chromium"],
  lost: ["passkeys"],
  checklist: ["browserSaving", "deleteOld"],
});

const manager = (
  id: SourceId,
  steps: number,
  detects: readonly ImportDetectedSource[],
  lost: readonly LostKey[],
): ImportSourceInfo => ({
  id,
  group: "manager",
  steps,
  internalUrl: null,
  detects,
  lost,
  checklist: ["oldExtension", "deleteOld"],
});

// The menu paths in the guides are unverified against the current product (launch gate).
export const importSources: readonly ImportSourceInfo[] = [
  chromium("chrome", "chrome"),
  { ...chromium("edge", "edge"), internalUrl: "edge://settings/passwords" },
  chromium("brave", "brave"),
  chromium("opera", "opera"),
  chromium("vivaldi", "vivaldi"),
  chromium("arc", "arc"),
  {
    id: "firefox",
    group: "browser",
    steps: 3,
    internalUrl: "about:logins",
    detects: ["firefox"],
    lost: ["passkeys"],
    checklist: ["firefoxSaving", "deleteOld"],
  },
  {
    id: "safari",
    group: "browser",
    steps: 3,
    internalUrl: null,
    detects: ["safari"],
    lost: ["passkeys"],
    checklist: ["safariSaving", "deleteOld"],
  },
  {
    id: "applePasswords",
    group: "browser",
    steps: 3,
    internalUrl: null,
    detects: ["safari"],
    lost: ["passkeys"],
    checklist: ["safariSaving", "deleteOld"],
  },
  manager("onePassword", 3, ["onePassword"], ["passkeys"]),
  manager("bitwarden", 3, ["bitwarden", "bitwardenCsv"], ["attachments"]),
  { ...manager("keepass", 2, ["keepass"], []), checklist: ["deleteOld"] },
  { ...manager("keepassxcCsv", 2, ["keepassxcCsv"], ["attachments", "history"]), checklist: ["deleteOld"] },
  manager("enpass", 3, ["enpass"], []),
  manager("lastpass", 3, ["lastpass"], ["attachments", "passkeys", "history"]),
  manager("dashlane", 3, ["dashlane"], ["attachments", "passkeys", "history"]),
  manager("keeper", 2, ["keeper", "csv"], ["attachments", "passkeys"]),
  manager("nordpass", 2, ["nordpass"], ["attachments", "passkeys"]),
  manager("protonPass", 3, ["protonPass"], ["attachments"]),
  manager("roboform", 2, ["roboform"], ["roboformExtras", "passkeys"]),
  {
    id: "otherFile",
    group: "other",
    steps: 2,
    internalUrl: null,
    detects: [],
    lost: [],
    checklist: ["deleteOld"],
  },
];

export function sourceById(id: SourceId): ImportSourceInfo {
  return importSources.find((source) => source.id === id) ?? importSources[0] ?? fallbackSource;
}

const fallbackSource: ImportSourceInfo = {
  id: "otherFile",
  group: "other",
  steps: 2,
  internalUrl: null,
  detects: [],
  lost: [],
  checklist: ["deleteOld"],
};

/** The format to preview with: the detected one, except a headerless Keeper CSV. `null` when nothing was detected. */
export function formatFor(source: ImportSourceInfo, detected: ImportDetected): ImportFormat | null {
  if (source.id === "keeper" && detected.source === "csv") {
    return "keeperCsv";
  }
  return detected.format;
}

/** True when something was detected and it is not what this source exports. */
export function isMismatch(source: ImportSourceInfo, detected: ImportDetected): boolean {
  if (detected.source === null || source.detects.length === 0) {
    return false;
  }
  return !source.detects.includes(detected.source);
}
