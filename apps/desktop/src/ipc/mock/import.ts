/** Mock import: a small Enpass preview with one duplicate and one skipped entry. */
import type { ImportEntry, ImportPreview } from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

function sampleEntries(state: MockState): ImportEntry[] {
  const github = state.items[0]?.detail.summary;
  return [
    {
      index: 0,
      kind: "login",
      title: "GitHub",
      subtitle: "alex@example.com",
      duplicateOf: github === undefined ? null : { vaultId: github.vaultId, itemId: github.itemId },
      duplicateInFile: null,
      attachments: 0,
      warnings: [],
    },
    {
      index: 1,
      kind: "login",
      title: "Example Mail",
      subtitle: "alice@example.com",
      duplicateOf: null,
      duplicateInFile: null,
      attachments: 1,
      warnings: ["unmappedFields", "otpUnreadable"],
    },
    {
      index: 2,
      kind: "secureNote",
      title: "Router notes",
      subtitle: null,
      duplicateOf: null,
      duplicateInFile: 1,
      attachments: 0,
      warnings: ["archived"],
    },
  ];
}

export const importHandlers: MockHandlers = {
  import_choose_file: (state) => {
    requireUnlocked(state);
    return { kind: "chosen", fileToken: "file-1", fileName: "enpass-export.json" };
  },

  import_preview: (state, request) => {
    requireUnlocked(state);
    if (request.fileToken !== "file-1") {
      fail("importFileUnreadable");
    }
    if (request.format === "keepass" && request.password !== "mosaic") {
      fail("wrongCredentials");
    }
    const entries = sampleEntries(state);
    const previewId = state.nextId();
    state.heldImport = { previewId, entries: entries.map((e) => e.index) };
    const preview: ImportPreview = {
      previewId,
      format: request.format,
      entries,
      skipped: [{ index: 3, reason: "empty" }],
      counts: [
        { kind: "login", count: 2 },
        { kind: "secureNote", count: 1 },
      ],
      attachments: 1,
      attachmentBytes: 2048,
      warnings: ["unmappedFields", "entriesSkipped"],
    };
    return preview;
  },

  import_commit: (state, request) => {
    requireUnlocked(state);
    const held = state.heldImport;
    if (held?.previewId !== request.previewId) {
      fail("importPreviewExpired");
    }
    const vault = state.vaults.find((v) => v.vaultId === request.targetVaultId) ?? fail("vaultNotFound");
    const imported = held.entries.filter((i) => !request.skip.includes(i)).length;
    vault.itemCount += imported;
    state.heldImport = null;
    return {
      imported,
      skipped: held.entries.length - imported,
      failed: 0,
      attachments: 0,
      attachmentsFailed: 0,
    };
  },

  import_cancel: (state, request) => {
    if (state.heldImport?.previewId === request.previewId) {
      state.heldImport = null;
    }
    return null;
  },
};
