/**
 * Mock attachment commands. No file is read or written: a file is a kind and a size. The
 * preview is a 64 x 48 gradient in the contract's binary layout, so the canvas path of the
 * UI runs for real. URL options: `?attachAdd=cancel|<errorCode>` and
 * `?attachSave=cancel|<errorCode>`.
 */
import type { AttachmentEntry, ErrorCode, ItemDetail } from "../contract/index.ts";
import { findItem } from "./items.ts";
import { hardwareVaultOf } from "./vaults.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

export const MOCK_PREVIEW_WIDTH = 64;
export const MOCK_PREVIEW_HEIGHT = 48;
/** The size of the file the mock "chooses" when it adds one. */
const ADDED_BYTES = 1_200_000;

/** `[width u32 LE][height u32 LE][RGBA]` of a diagonal gradient, as Rust sends it. */
export function mockPreviewBytes(): ArrayBuffer {
  const pixels = MOCK_PREVIEW_WIDTH * MOCK_PREVIEW_HEIGHT * 4;
  const buffer = new ArrayBuffer(8 + pixels);
  const view = new DataView(buffer);
  view.setUint32(0, MOCK_PREVIEW_WIDTH, true);
  view.setUint32(4, MOCK_PREVIEW_HEIGHT, true);
  const bytes = new Uint8Array(buffer, 8);
  for (let y = 0; y < MOCK_PREVIEW_HEIGHT; y += 1) {
    for (let x = 0; x < MOCK_PREVIEW_WIDTH; x += 1) {
      const at = (y * MOCK_PREVIEW_WIDTH + x) * 4;
      bytes[at] = Math.round((x / (MOCK_PREVIEW_WIDTH - 1)) * 255);
      bytes[at + 1] = Math.round((y / (MOCK_PREVIEW_HEIGHT - 1)) * 255);
      bytes[at + 2] = 160;
      bytes[at + 3] = 255;
    }
  }
  return buffer;
}

function entryOf(item: MockItem, attachmentId: string): AttachmentEntry {
  return (
    item.detail.attachments.find((entry) => entry.attachmentId === attachmentId) ??
    fail("attachmentNotFound")
  );
}

/** Applies `?attachAdd` / `?attachSave`: `null` runs the command, `"cancel"` cancels the dialog. */
function forced(option: ErrorCode | "cancel" | null): "cancel" | null {
  if (option !== null && option !== "cancel") {
    fail(option);
  }
  return option;
}

function addFile(state: MockState, item: MockItem): ItemDetail {
  const attachmentId = crypto.randomUUID();
  state.attachments.set(attachmentId, { kind: "png", bytes: ADDED_BYTES });
  item.detail.attachments.push({
    attachmentId,
    fileName: "scan-2.png",
    mime: "image/png",
    size: ADDED_BYTES,
    addedAt: state.now(),
    available: true,
    previewable: true,
  });
  item.detail.summary.modifiedAt = state.now();
  return item.detail;
}

export const attachmentHandlers: MockHandlers = {
  attachment_add: (state, request) => {
    requireUnlocked(state);
    if (hardwareVaultOf(state, request.vaultId) !== undefined) {
      fail("itemTypeNotAllowed");
    }
    const item = findItem(state, request);
    if (forced(state.options.attachAdd) === "cancel") {
      return { kind: "cancelled" };
    }
    return { kind: "added", item: addFile(state, item) };
  },

  attachment_preview: (state, request) => {
    requireUnlocked(state);
    const entry = entryOf(findItem(state, request), request.attachmentId);
    if (!entry.available) {
      fail("attachmentUnavailable");
    }
    if (state.attachments.get(request.attachmentId)?.kind !== "png") {
      fail("attachmentPreviewUnsupported");
    }
    return mockPreviewBytes();
  },

  attachment_save: (state, request) => {
    requireUnlocked(state);
    const entry = entryOf(findItem(state, request), request.attachmentId);
    if (!entry.available) {
      fail("attachmentUnavailable");
    }
    return forced(state.options.attachSave) === "cancel" ? { kind: "cancelled" } : { kind: "saved" };
  },

  attachment_remove: (state, request) => {
    requireUnlocked(state);
    const item = findItem(state, request);
    entryOf(item, request.attachmentId);
    item.detail.attachments = item.detail.attachments.filter(
      (entry) => entry.attachmentId !== request.attachmentId,
    );
    state.attachments.delete(request.attachmentId);
    item.detail.summary.modifiedAt = state.now();
    return item.detail;
  },
};
