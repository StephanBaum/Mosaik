import { describe, expect, it } from "vitest";
import type { AttachmentRequest, ItemDetail } from "../contract/index.ts";
import { createMockIpc } from "../mock.ts";
import { MOCK_PREVIEW_HEIGHT, MOCK_PREVIEW_WIDTH } from "./attachments.ts";

const VAULT = "019a2b3c-4d5e-7010-8000-000000000001";
const PASSPORT = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000008" };
const PNG: AttachmentRequest = { ...PASSPORT, attachmentId: "019a2b3c-4d5e-7040-8000-000000000001" };
const PDF: AttachmentRequest = { ...PASSPORT, attachmentId: "019a2b3c-4d5e-7040-8000-000000000002" };
const GITHUB = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000001" };

async function detail(ipc: ReturnType<typeof createMockIpc>): Promise<ItemDetail> {
  const result = await ipc.call("get_item", PASSPORT);
  if (!result.ok) {
    throw new Error("get_item failed");
  }
  return result.value;
}

describe("mock attachments", () => {
  it("seeds the Passport scan with a previewable image and a PDF", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    const files = (await detail(ipc)).attachments;
    expect(files.map((f) => [f.fileName, f.previewable, f.available])).toEqual([
      ["passport.png", true, true],
      ["contract.pdf", false, true],
    ]);
  });

  it("previews a PNG as header plus RGBA and refuses a PDF", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    const result = await ipc.call("attachment_preview", PNG);
    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }
    const view = new DataView(result.value);
    expect([view.getUint32(0, true), view.getUint32(4, true)]).toEqual([MOCK_PREVIEW_WIDTH, MOCK_PREVIEW_HEIGHT]);
    expect(result.value.byteLength).toBe(8 + MOCK_PREVIEW_WIDTH * MOCK_PREVIEW_HEIGHT * 4);
    expect(await ipc.call("attachment_preview", PDF)).toEqual({
      ok: false,
      error: { code: "attachmentPreviewUnsupported" },
    });
  });

  it("returns a copy, so wiping the answer never touches the mock", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    const first = await ipc.call("attachment_preview", PNG);
    if (first.ok) {
      new Uint8Array(first.value).fill(0);
    }
    const second = await ipc.call("attachment_preview", PNG);
    expect(second.ok && new Uint8Array(second.value).some((byte) => byte !== 0)).toBe(true);
  });

  it("adds scan-2.png by default, and honours ?attachAdd", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    const added = await ipc.call("attachment_add", PASSPORT);
    expect(added.ok && added.value.kind === "added" && added.value.item.attachments.at(-1)?.fileName).toBe(
      "scan-2.png",
    );
    expect((await detail(ipc)).attachments).toHaveLength(3);

    const cancelled = createMockIpc({ start: "unlocked", attachAdd: "cancel" });
    expect(await cancelled.call("attachment_add", PASSPORT)).toEqual({ ok: true, value: { kind: "cancelled" } });
    expect((await detail(cancelled)).attachments).toHaveLength(2);

    const failing = createMockIpc({ start: "unlocked", attachAdd: "attachmentQuota" });
    expect(await failing.call("attachment_add", PASSPORT)).toEqual({
      ok: false,
      error: { code: "attachmentQuota" },
    });
  });

  it("saves, or cancels with ?attachSave", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    expect(await ipc.call("attachment_save", PDF)).toEqual({ ok: true, value: { kind: "saved" } });
    const cancelled = createMockIpc({ start: "unlocked", attachSave: "cancel" });
    expect(await cancelled.call("attachment_save", PDF)).toEqual({ ok: true, value: { kind: "cancelled" } });
  });

  it("removes a file, then does not know it any more", async () => {
    const ipc = createMockIpc({ start: "unlocked" });
    const removed = await ipc.call("attachment_remove", PDF);
    expect(removed.ok && removed.value.attachments.map((f) => f.fileName)).toEqual(["passport.png"]);
    for (const command of ["attachment_remove", "attachment_preview", "attachment_save"] as const) {
      expect(await ipc.call(command, PDF)).toEqual({ ok: false, error: { code: "attachmentNotFound" } });
    }
  });

  it("needs an unlocked vault and refuses unknown items", async () => {
    const locked = createMockIpc({ start: "locked" });
    expect(await locked.call("attachment_preview", PNG)).toEqual({ ok: false, error: { code: "locked" } });
    const ipc = createMockIpc({ start: "unlocked" });
    expect(await ipc.call("attachment_add", { vaultId: VAULT, itemId: "019a2b3c-0000-7000-8000-0000000000ff" })).toEqual({
      ok: false,
      error: { code: "itemNotFound" },
    });
    expect(await ipc.call("attachment_remove", { ...GITHUB, attachmentId: PNG.attachmentId })).toEqual({
      ok: false,
      error: { code: "attachmentNotFound" },
    });
  });
});
